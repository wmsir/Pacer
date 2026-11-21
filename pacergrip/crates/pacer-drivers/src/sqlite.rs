use std::collections::BTreeMap;
use std::path::Path;
use std::str::FromStr;
use std::time::Instant;

use async_trait::async_trait;
use serde_json::Value as JsonValue;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions, SqliteRow};
use sqlx::{Column, Executor, Row, TypeInfo, ValueRef};
use tokio::sync::mpsc;

use pacer_core::{
    CapabilityFlags, ColumnMeta, CoreError, CoreResult, DatabaseMeta, DbBackend, DbEngineType,
    DirectConfig, ExplainResult, IndexMeta, PingMetrics, QueryChunk, QueryReq, QuerySummary,
    TableMeta,
};
use pacer_sql_ast::{analyze_sql, SqlAccessKind};

const STREAM_CHUNK_SIZE: usize = 500;

/// Native SQLite backend powered by `sqlx`.
pub struct SqliteBackend {
    pool: SqlitePool,
    db_path: String,
    read_only: bool,
}

impl SqliteBackend {
    pub async fn connect(config: &DirectConfig) -> CoreResult<Self> {
        let db_path = resolve_sqlite_path(config)?;
        let mut opts = if db_path == ":memory:" {
            SqliteConnectOptions::from_str("sqlite::memory:")
                .map_err(|e| CoreError::InvalidConfig(e.to_string()))?
        } else {
            SqliteConnectOptions::new()
                .filename(&db_path)
                .create_if_missing(false)
        };
        if config.read_only {
            opts = opts.read_only(true);
        }

        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect_with(opts)
            .await
            .map_err(|e| CoreError::Transport(format!("SQLite connection failed ({db_path}): {e}")))?;

        Ok(Self {
            pool,
            db_path,
            read_only: config.read_only,
        })
    }
}

fn resolve_sqlite_path(config: &DirectConfig) -> CoreResult<String> {
    if let Some(db) = config.database.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        return Ok(db.to_string());
    }
    let raw = config.jdbc_url.trim();
    if let Some(rest) = raw
        .strip_prefix("jdbc:sqlite:")
        .or_else(|| raw.strip_prefix("sqlite:"))
    {
        let clean = rest.split('?').next().unwrap_or(rest).trim();
        if !clean.is_empty() {
            return Ok(clean.to_string());
        }
    }
    Err(CoreError::InvalidConfig(
        "SQLite connection requires a database file path".to_string(),
    ))
}

#[async_trait]
impl DbBackend for SqliteBackend {
    async fn test_connection(&self) -> CoreResult<PingMetrics> {
        let started = Instant::now();
        let version: String = sqlx::query_scalar("SELECT sqlite_version()")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;

        Ok(PingMetrics {
            success: true,
            latency_ms: started.elapsed().as_millis() as u64,
            server_version: Some(format!("SQLite {version}")),
            message: format!("Connected to {}", self.db_path),
        })
    }

    async fn list_databases(&self) -> CoreResult<Vec<DatabaseMeta>> {
        let rows = sqlx::query("PRAGMA database_list")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;

        let tables = self.list_tables("main").await.unwrap_or_default();
        let table_count = tables.len() as u64;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let name: String = row.try_get("name").unwrap_or_else(|_| "main".to_string());
            let file: String = row.try_get("file").unwrap_or_default();
            out.push(DatabaseMeta {
                is_system: name != "main",
                table_count: if name == "main" { Some(table_count) } else { None },
                charset: Some("UTF-8".to_string()),
                collation: Some("BINARY".to_string()),
                data_size_mb: None,
                comment: if file.is_empty() { None } else { Some(file) },
                name,
            });
        }
        Ok(out)
    }

    async fn list_tables(&self, _db: &str) -> CoreResult<Vec<TableMeta>> {
        let rows = sqlx::query(
            "SELECT name, type FROM sqlite_master \
             WHERE type IN ('table', 'view') AND name NOT LIKE 'sqlite_%' \
             ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let name: String = row.try_get("name").unwrap_or_default();
            let obj_type: String = row.try_get("type").unwrap_or_else(|_| "table".to_string());
            let est_rows = if obj_type == "table" {
                let count_sql = format!("SELECT COUNT(*) FROM \"{}\"", name.replace('"', "\"\""));
                sqlx::query_scalar::<_, i64>(&count_sql)
                    .fetch_one(&self.pool)
                    .await
                    .ok()
                    .and_then(|c| u64::try_from(c).ok())
            } else {
                None
            };
            out.push(TableMeta {
                name,
                table_type: Some(obj_type.to_ascii_uppercase()),
                engine: Some("SQLite".to_string()),
                estimated_rows: est_rows,
                data_size_kb: None,
                comment: None,
            });
        }
        Ok(out)
    }

    async fn list_columns(&self, _db: &str, table: &str) -> CoreResult<Vec<ColumnMeta>> {
        let sql = format!("PRAGMA table_info(\"{}\")", table.replace('"', "\"\""));
        let rows = sqlx::query(&sql)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let name: String = row.try_get("name").unwrap_or_default();
            let data_type: String = row
                .try_get::<String, _>("type")
                .ok()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "ANY".to_string());
            let notnull: i64 = row.try_get("notnull").unwrap_or(0);
            let pk: i64 = row.try_get("pk").unwrap_or(0);
            let dflt_value: Option<String> = row.try_get("dflt_value").ok().flatten();

            out.push(ColumnMeta {
                name,
                data_type,
                nullable: notnull == 0 && pk == 0,
                is_primary_key: pk > 0,
                default_value: dflt_value,
                comment: None,
                masking_rule: None,
            });
        }
        Ok(out)
    }

    async fn list_indexes(&self, _db: &str, table: &str) -> CoreResult<Vec<IndexMeta>> {
        let sql = format!("PRAGMA index_list(\"{}\")", table.replace('"', "\"\""));
        let rows = sqlx::query(&sql)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let idx_name: String = row.try_get("name").unwrap_or_default();
            let unique: i64 = row.try_get("unique").unwrap_or(0);
            let origin: Option<String> = row.try_get("origin").ok();

            let info_sql = format!("PRAGMA index_info(\"{}\")", idx_name.replace('"', "\"\""));
            let col_rows = sqlx::query(&info_sql)
                .fetch_all(&self.pool)
                .await
                .unwrap_or_default();
            let columns: Vec<String> = col_rows
                .into_iter()
                .filter_map(|r| r.try_get::<Option<String>, _>("name").ok().flatten())
                .collect();

            out.push(IndexMeta {
                name: idx_name,
                unique: unique != 0,
                columns,
                index_type: origin.map(|o| match o.as_str() {
                    "pk" => "PRIMARY".to_string(),
                    "u" => "UNIQUE".to_string(),
                    _ => "BTREE".to_string(),
                }),
            });
        }
        Ok(out)
    }

    async fn get_table_ddl(&self, _db: &str, table: &str) -> CoreResult<String> {
        let ddl: Option<String> =
            sqlx::query_scalar("SELECT sql FROM sqlite_master WHERE name = ? LIMIT 1")
                .bind(table)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?
                .flatten();

        ddl.ok_or_else(|| CoreError::ExecutionFailed(format!("DDL not found for table {table}")))
    }

    async fn execute_query(
        &self,
        req: QueryReq,
        tx: mpsc::Sender<QueryChunk>,
    ) -> CoreResult<QuerySummary> {
        let started = Instant::now();
        let analysis = analyze_sql(&req.sql, &DbEngineType::Sqlite);
        let is_read_only = matches!(analysis.overall_kind, SqlAccessKind::ReadOnly);

        if self.read_only && !is_read_only {
            return Err(CoreError::PermissionDenied(
                "Connection is marked read-only; DML/DDL execution blocked".to_string(),
            ));
        }

        let db_label = req.database.clone().unwrap_or_else(|| "main".to_string());

        if !is_read_only {
            let mut conn = self
                .pool
                .acquire()
                .await
                .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;
            let result = conn
                .execute(req.sql.as_str())
                .await
                .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;

            let _ = tx
                .send(QueryChunk {
                    columns: Vec::new(),
                    rows: Vec::new(),
                    batch_index: 0,
                    is_last: true,
                    masked_columns: BTreeMap::new(),
                })
                .await;

            return Ok(QuerySummary {
                success: true,
                database: db_label,
                total_rows: 0,
                affected_rows: Some(result.rows_affected()),
                is_dml_or_ddl: true,
                duration_ms: started.elapsed().as_millis() as u64,
                route_node: Some("DIRECT".to_string()),
                route_description: Some("SQLite native direct execution".to_string()),
                masked_columns: BTreeMap::new(),
                error_message: None,
            });
        }

        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;

        let rows = conn
            .fetch_all(req.sql.as_str())
            .await
            .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;

        let max_rows = req.limit.map(|l| l as usize).unwrap_or(usize::MAX);
        let limited_rows: Vec<&SqliteRow> = rows.iter().take(max_rows).collect();

        let columns: Vec<String> = if let Some(first) = rows.first() {
            first.columns().iter().map(|c| c.name().to_string()).collect()
        } else {
            match conn.describe(req.sql.as_str()).await {
                Ok(desc) => desc
                    .columns()
                    .iter()
                    .map(|c| c.name().to_string())
                    .collect(),
                Err(_) => Vec::new(),
            }
        };

        let mut converted = Vec::with_capacity(limited_rows.len());
        for row in &limited_rows {
            converted.push(sqlite_row_to_map(row, &columns));
        }

        let total_rows = converted.len();
        stream_chunks(&columns, converted, &tx).await;

        Ok(QuerySummary {
            success: true,
            database: db_label,
            total_rows,
            affected_rows: None,
            is_dml_or_ddl: false,
            duration_ms: started.elapsed().as_millis() as u64,
            route_node: Some("DIRECT".to_string()),
            route_description: Some("SQLite native direct execution".to_string()),
            masked_columns: BTreeMap::new(),
            error_message: None,
        })
    }

    async fn explain(&self, _db: &str, sql: &str) -> CoreResult<ExplainResult> {
        let started = Instant::now();
        let trimmed = sql.trim().trim_end_matches(';');
        let explain_sql = if trimmed.to_ascii_uppercase().starts_with("EXPLAIN") {
            trimmed.to_string()
        } else {
            format!("EXPLAIN QUERY PLAN {trimmed}")
        };

        let rows = sqlx::query(&explain_sql)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;

        let columns: Vec<String> = rows
            .first()
            .map(|r| r.columns().iter().map(|c| c.name().to_string()).collect())
            .unwrap_or_default();
        let mapped = rows
            .iter()
            .map(|r| sqlite_row_to_map(r, &columns))
            .collect();

        Ok(ExplainResult {
            success: true,
            columns,
            rows: mapped,
            duration_ms: started.elapsed().as_millis() as u64,
            error_message: None,
        })
    }

    fn capabilities(&self) -> CapabilityFlags {
        CapabilityFlags::direct_full(self.read_only)
    }
}

fn sqlite_row_to_map(row: &SqliteRow, columns: &[String]) -> BTreeMap<String, JsonValue> {
    let mut map = BTreeMap::new();
    for (idx, col_name) in columns.iter().enumerate() {
        let val = decode_sqlite_cell(row, idx);
        map.insert(col_name.clone(), val);
    }
    map
}

fn decode_sqlite_cell(row: &SqliteRow, idx: usize) -> JsonValue {
    let Ok(raw) = row.try_get_raw(idx) else {
        return JsonValue::Null;
    };
    if raw.is_null() {
        return JsonValue::Null;
    }

    let type_name = raw.type_info().name().to_ascii_uppercase();
    match type_name.as_str() {
        "INTEGER" | "INT" => row
            .try_get::<i64, _>(idx)
            .map(JsonValue::from)
            .unwrap_or(JsonValue::Null),
        "REAL" | "FLOAT" | "DOUBLE" => row
            .try_get::<f64, _>(idx)
            .ok()
            .and_then(serde_json::Number::from_f64)
            .map(JsonValue::Number)
            .unwrap_or(JsonValue::Null),
        "TEXT" => row
            .try_get::<String, _>(idx)
            .map(JsonValue::String)
            .unwrap_or(JsonValue::Null),
        "BLOB" => row
            .try_get::<Vec<u8>, _>(idx)
            .map(|bytes| JsonValue::String(format!("<BLOB {} bytes>", bytes.len())))
            .unwrap_or(JsonValue::Null),
        _ => {
            if let Ok(v) = row.try_get::<i64, _>(idx) {
                JsonValue::from(v)
            } else if let Ok(v) = row.try_get::<f64, _>(idx) {
                serde_json::Number::from_f64(v)
                    .map(JsonValue::Number)
                    .unwrap_or(JsonValue::Null)
            } else if let Ok(v) = row.try_get::<String, _>(idx) {
                JsonValue::String(v)
            } else if let Ok(v) = row.try_get::<Vec<u8>, _>(idx) {
                JsonValue::String(format!("<BLOB {} bytes>", v.len()))
            } else {
                JsonValue::Null
            }
        }
    }
}

pub(crate) async fn stream_chunks(
    columns: &[String],
    rows: Vec<BTreeMap<String, JsonValue>>,
    tx: &mpsc::Sender<QueryChunk>,
) {
    if rows.is_empty() {
        let _ = tx
            .send(QueryChunk {
                columns: columns.to_vec(),
                rows: Vec::new(),
                batch_index: 0,
                is_last: true,
                masked_columns: BTreeMap::new(),
            })
            .await;
        return;
    }

    let total_batches = rows.len().div_ceil(STREAM_CHUNK_SIZE);
    for (batch_index, slice) in rows.chunks(STREAM_CHUNK_SIZE).enumerate() {
        let is_last = batch_index + 1 == total_batches;
        let chunk = QueryChunk {
            columns: columns.to_vec(),
            rows: slice.to_vec(),
            batch_index,
            is_last,
            masked_columns: BTreeMap::new(),
        };
        if tx.send(chunk).await.is_err() {
            break;
        }
    }
}

/// Creates a sample SQLite database with tables & seed rows if it does not yet exist.
pub async fn create_demo_sqlite_db(path: &Path) -> CoreResult<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let opts = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await
        .map_err(|e| CoreError::ExecutionFailed(format!("failed creating demo SQLite DB: {e}")))?;

    let schema_sql = r#"
    CREATE TABLE IF NOT EXISTS customers (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        full_name TEXT NOT NULL,
        email TEXT NOT NULL UNIQUE,
        tier TEXT NOT NULL DEFAULT 'STANDARD',
        country TEXT NOT NULL DEFAULT 'CN',
        created_at TEXT NOT NULL
    );
    CREATE TABLE IF NOT EXISTS products (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        sku TEXT NOT NULL UNIQUE,
        title TEXT NOT NULL,
        category TEXT NOT NULL,
        unit_price REAL NOT NULL,
        stock_qty INTEGER NOT NULL DEFAULT 0
    );
    CREATE TABLE IF NOT EXISTS orders (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        order_no TEXT NOT NULL UNIQUE,
        customer_id INTEGER NOT NULL REFERENCES customers(id),
        status TEXT NOT NULL,
        total_amount REAL NOT NULL,
        ordered_at TEXT NOT NULL
    );
    CREATE TABLE IF NOT EXISTS order_items (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        order_id INTEGER NOT NULL REFERENCES orders(id),
        product_id INTEGER NOT NULL REFERENCES products(id),
        quantity INTEGER NOT NULL,
        unit_price REAL NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_orders_customer ON orders(customer_id);
    CREATE INDEX IF NOT EXISTS idx_orders_status ON orders(status);
    "#;

    pool.execute(schema_sql)
        .await
        .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM customers")
        .fetch_one(&pool)
        .await
        .unwrap_or(0);

    if count == 0 {
        let seed_sql = r#"
        INSERT INTO customers (full_name, email, tier, country, created_at) VALUES
          ('Alice Chen', 'alice.chen@example.com', 'VIP', 'CN', '2026-01-10 09:15:00'),
          ('Bob Lin', 'bob.lin@example.com', 'STANDARD', 'CN', '2026-02-14 14:22:00'),
          ('Charlie Wu', 'charlie.wu@example.com', 'ENTERPRISE', 'SG', '2026-03-01 11:05:00'),
          ('Diana Zhao', 'diana.zhao@example.com', 'VIP', 'CN', '2026-04-18 16:40:00');

        INSERT INTO products (sku, title, category, unit_price, stock_qty) VALUES
          ('SKU-DB-01', 'NVMe SSD 2TB', 'Hardware', 1299.00, 42),
          ('SKU-DB-02', 'Mechanical Keyboard Pro', 'Peripherals', 699.50, 88),
          ('SKU-DB-03', 'USB-C 10GbE Adapter', 'Networking', 249.00, 120),
          ('SKU-DB-04', '27" 4K IPS Monitor', 'Displays', 2499.00, 15);

        INSERT INTO orders (order_no, customer_id, status, total_amount, ordered_at) VALUES
          ('ORD-2026-1001', 1, 'PAID', 1998.50, '2026-05-02 10:12:00'),
          ('ORD-2026-1002', 2, 'SHIPPED', 249.00, '2026-05-03 18:30:00'),
          ('ORD-2026-1003', 3, 'PENDING', 4998.00, '2026-05-05 08:45:00'),
          ('ORD-2026-1004', 4, 'PAID', 1299.00, '2026-05-06 13:20:00');

        INSERT INTO order_items (order_id, product_id, quantity, unit_price) VALUES
          (1, 1, 1, 1299.00),
          (1, 2, 1, 699.50),
          (2, 3, 1, 249.00),
          (3, 4, 2, 2499.00),
          (4, 1, 1, 1299.00);
        "#;
        pool.execute(seed_sql)
            .await
            .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;
    }

    pool.close().await;
    Ok(())
}
