use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde_json::Value as JsonValue;
use sqlx::postgres::{PgConnectOptions, PgPool, PgPoolOptions, PgRow};
use sqlx::{Column, Executor, Row, TypeInfo, ValueRef};
use tokio::sync::mpsc;

use pacer_core::{
    CapabilityFlags, ColumnMeta, CoreError, CoreResult, DatabaseMeta, DbBackend, DirectConfig,
    ExplainResult, PingMetrics, QueryChunk, QueryReq, QuerySummary, TableMeta,
};
use pacer_sql_ast::{analyze_sql, SqlAccessKind};

use crate::sqlite::stream_chunks;

/// Native PostgreSQL / openGauss / KingbaseES backend powered by `sqlx`.
pub struct PostgresBackend {
    pool: PgPool,
    read_only: bool,
    engine_label: String,
    engine_type: pacer_core::DbEngineType,
}

impl PostgresBackend {
    pub async fn connect(config: &DirectConfig, password: Option<&str>) -> CoreResult<Self> {
        let host = config
            .host
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or("localhost");
        let port = config
            .port
            .or_else(|| config.db_type.default_port())
            .unwrap_or(5432);

        let mut opts = PgConnectOptions::new()
            .host(host)
            .port(port)
            .username(config.username.as_deref().unwrap_or("postgres"));

        if let Some(pwd) = password {
            opts = opts.password(pwd);
        }
        if let Some(db) = config.database.as_deref().filter(|s| !s.is_empty()) {
            opts = opts.database(db);
        }

        let pool = PgPoolOptions::new()
            .max_connections(4)
            .acquire_timeout(Duration::from_secs(8))
            .connect_with(opts)
            .await
            .map_err(|e| CoreError::Transport(format!("PostgreSQL connection failed: {e}")))?;

        Ok(Self {
            pool,
            read_only: config.read_only,
            engine_label: config.db_type.as_pacer_db_type().to_string(),
            engine_type: config.db_type.clone(),
        })
    }
}

#[async_trait]
impl DbBackend for PostgresBackend {
    async fn test_connection(&self) -> CoreResult<PingMetrics> {
        let started = Instant::now();
        let version: String = sqlx::query_scalar("SELECT version()")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;

        Ok(PingMetrics {
            success: true,
            latency_ms: started.elapsed().as_millis() as u64,
            server_version: Some(version),
            message: format!("Connected ({})", self.engine_label),
        })
    }

    async fn list_databases(&self) -> CoreResult<Vec<DatabaseMeta>> {
        let rows = sqlx::query(
            "SELECT s.schema_name, COUNT(t.table_name)::bigint AS table_count \
             FROM information_schema.schemata s \
             LEFT JOIN information_schema.tables t ON s.schema_name = t.table_schema \
             GROUP BY s.schema_name \
             ORDER BY s.schema_name",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let name: String = row.try_get("schema_name").unwrap_or_default();
            let is_sys = name.starts_with("pg_") || name == "information_schema";
            let count: i64 = row.try_get("table_count").unwrap_or(0);
            out.push(DatabaseMeta {
                name,
                charset: Some("UTF8".to_string()),
                collation: None,
                table_count: u64::try_from(count).ok(),
                data_size_mb: None,
                is_system: is_sys,
                comment: Some("Schema".to_string()),
            });
        }
        Ok(out)
    }

    async fn list_tables(&self, db: &str) -> CoreResult<Vec<TableMeta>> {
        let schema = if db.is_empty() { "public" } else { db };
        let rows = sqlx::query(
            "SELECT table_name, table_type \
             FROM information_schema.tables \
             WHERE table_schema = $1 \
             ORDER BY table_name",
        )
        .bind(schema)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|row| TableMeta {
                name: row.try_get("table_name").unwrap_or_default(),
                table_type: row.try_get("table_type").ok(),
                engine: Some(self.engine_label.clone()),
                estimated_rows: None,
                data_size_kb: None,
                comment: None,
            })
            .collect())
    }

    async fn list_columns(&self, db: &str, table: &str) -> CoreResult<Vec<ColumnMeta>> {
        let schema = if db.is_empty() { "public" } else { db };
        let rows = sqlx::query(
            "SELECT c.column_name, c.data_type, c.is_nullable, c.column_default, \
                    EXISTS ( \
                      SELECT 1 FROM information_schema.table_constraints tc \
                      JOIN information_schema.key_column_usage kcu \
                        ON tc.constraint_name = kcu.constraint_name AND tc.table_schema = kcu.table_schema \
                      WHERE tc.table_schema = c.table_schema AND tc.table_name = c.table_name \
                        AND kcu.column_name = c.column_name AND tc.constraint_type = 'PRIMARY KEY' \
                    ) AS is_pk \
             FROM information_schema.columns c \
             WHERE c.table_schema = $1 AND c.table_name = $2 \
             ORDER BY c.ordinal_position",
        )
        .bind(schema)
        .bind(table)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|row| ColumnMeta {
                name: row.try_get("column_name").unwrap_or_default(),
                data_type: row
                    .try_get::<String, _>("data_type")
                    .unwrap_or_else(|_| "unknown".to_string()),
                nullable: row
                    .try_get::<String, _>("is_nullable")
                    .map(|v| !v.eq_ignore_ascii_case("NO"))
                    .unwrap_or(true),
                is_primary_key: row.try_get::<bool, _>("is_pk").unwrap_or(false),
                default_value: row.try_get("column_default").ok().flatten(),
                comment: None,
                masking_rule: None,
            })
            .collect())
    }

    async fn get_table_ddl(&self, db: &str, table: &str) -> CoreResult<String> {
        let schema = if db.is_empty() { "public" } else { db };
        let cols = self.list_columns(schema, table).await?;
        if cols.is_empty() {
            return Err(CoreError::ExecutionFailed(format!(
                "Table {schema}.{table} not found"
            )));
        }

        let mut lines = Vec::new();
        let mut pks = Vec::new();
        for c in &cols {
            let mut line = format!("  \"{}\" {}", c.name.replace('"', "\"\""), c.data_type);
            if !c.nullable {
                line.push_str(" NOT NULL");
            }
            if let Some(ref dflt) = c.default_value {
                line.push_str(&format!(" DEFAULT {dflt}"));
            }
            lines.push(line);
            if c.is_primary_key {
                pks.push(format!("\"{}\"", c.name.replace('"', "\"\"")));
            }
        }
        if !pks.is_empty() {
            lines.push(format!("  PRIMARY KEY ({})", pks.join(", ")));
        }

        Ok(format!(
            "CREATE TABLE \"{}\".\"{}\" (\n{}\n);",
            schema.replace('"', "\"\""),
            table.replace('"', "\"\""),
            lines.join(",\n")
        ))
    }

    async fn execute_query(
        &self,
        req: QueryReq,
        tx: mpsc::Sender<QueryChunk>,
    ) -> CoreResult<QuerySummary> {
        let started = Instant::now();
        let analysis = analyze_sql(&req.sql, &self.engine_type);
        let is_read_only = matches!(analysis.overall_kind, SqlAccessKind::ReadOnly);

        if self.read_only && !is_read_only {
            return Err(CoreError::PermissionDenied(
                "Connection is marked read-only; DML/DDL execution blocked".to_string(),
            ));
        }

        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;

        let schema = req
            .database
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "public".to_string());
        let set_path = format!("SET search_path TO \"{}\", public", schema.replace('"', "\"\""));
        let _ = conn.execute(set_path.as_str()).await;

        if !is_read_only {
            let res = conn
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
                database: schema,
                total_rows: 0,
                affected_rows: Some(res.rows_affected()),
                is_dml_or_ddl: true,
                duration_ms: started.elapsed().as_millis() as u64,
                route_node: Some("DIRECT".to_string()),
                route_description: Some(format!("{} native direct execution", self.engine_label)),
                masked_columns: BTreeMap::new(),
                error_message: None,
            });
        }

        let rows = conn
            .fetch_all(req.sql.as_str())
            .await
            .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;

        let max_rows = req.limit.map(|l| l as usize).unwrap_or(usize::MAX);
        let limited: Vec<&PgRow> = rows.iter().take(max_rows).collect();
        let columns: Vec<String> = rows
            .first()
            .map(|r| r.columns().iter().map(|c| c.name().to_string()).collect())
            .unwrap_or_default();

        let mapped: Vec<BTreeMap<String, JsonValue>> =
            limited.iter().map(|r| pg_row_to_map(r, &columns)).collect();
        let total_rows = mapped.len();
        stream_chunks(&columns, mapped, &tx).await;

        Ok(QuerySummary {
            success: true,
            database: schema,
            total_rows,
            affected_rows: None,
            is_dml_or_ddl: false,
            duration_ms: started.elapsed().as_millis() as u64,
            route_node: Some("DIRECT".to_string()),
            route_description: Some(format!("{} native direct execution", self.engine_label)),
            masked_columns: BTreeMap::new(),
            error_message: None,
        })
    }

    async fn explain(&self, db: &str, sql: &str) -> CoreResult<ExplainResult> {
        let started = Instant::now();
        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;
        if !db.is_empty() {
            let set_path = format!("SET search_path TO \"{}\", public", db.replace('"', "\"\""));
            let _ = conn.execute(set_path.as_str()).await;
        }
        let trimmed = sql.trim().trim_end_matches(';');
        let explain_sql = if trimmed.to_ascii_uppercase().starts_with("EXPLAIN") {
            trimmed.to_string()
        } else {
            format!("EXPLAIN {trimmed}")
        };

        let rows = conn
            .fetch_all(explain_sql.as_str())
            .await
            .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;

        let columns: Vec<String> = rows
            .first()
            .map(|r| r.columns().iter().map(|c| c.name().to_string()).collect())
            .unwrap_or_default();
        let mapped = rows.iter().map(|r| pg_row_to_map(r, &columns)).collect();
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

fn pg_row_to_map(row: &PgRow, columns: &[String]) -> BTreeMap<String, JsonValue> {
    let mut map = BTreeMap::new();
    for (idx, col_name) in columns.iter().enumerate() {
        map.insert(col_name.clone(), decode_pg_cell(row, idx));
    }
    map
}

fn decode_pg_cell(row: &PgRow, idx: usize) -> JsonValue {
    let Ok(raw) = row.try_get_raw(idx) else {
        return JsonValue::Null;
    };
    if raw.is_null() {
        return JsonValue::Null;
    }

    let type_name = row.column(idx).type_info().name().to_ascii_uppercase();
    match type_name.as_str() {
        "BOOL" => row
            .try_get::<bool, _>(idx)
            .map(JsonValue::Bool)
            .unwrap_or(JsonValue::Null),
        "INT2" => row
            .try_get::<i16, _>(idx)
            .map(JsonValue::from)
            .unwrap_or(JsonValue::Null),
        "INT4" => row
            .try_get::<i32, _>(idx)
            .map(JsonValue::from)
            .unwrap_or(JsonValue::Null),
        "INT8" => row
            .try_get::<i64, _>(idx)
            .map(JsonValue::from)
            .unwrap_or(JsonValue::Null),
        "FLOAT4" => row
            .try_get::<f32, _>(idx)
            .ok()
            .and_then(|v| serde_json::Number::from_f64(f64::from(v)))
            .map(JsonValue::Number)
            .unwrap_or(JsonValue::Null),
        "FLOAT8" => row
            .try_get::<f64, _>(idx)
            .ok()
            .and_then(serde_json::Number::from_f64)
            .map(JsonValue::Number)
            .unwrap_or(JsonValue::Null),
        "JSON" | "JSONB" => row
            .try_get::<JsonValue, _>(idx)
            .unwrap_or(JsonValue::Null),
        "TIMESTAMP" => row
            .try_get::<chrono::NaiveDateTime, _>(idx)
            .map(|dt| JsonValue::String(dt.format("%Y-%m-%d %H:%M:%S").to_string()))
            .unwrap_or(JsonValue::Null),
        "TIMESTAMPTZ" => row
            .try_get::<chrono::DateTime<chrono::Utc>, _>(idx)
            .map(|dt| JsonValue::String(dt.to_rfc3339()))
            .unwrap_or(JsonValue::Null),
        "DATE" => row
            .try_get::<chrono::NaiveDate, _>(idx)
            .map(|d| JsonValue::String(d.to_string()))
            .unwrap_or(JsonValue::Null),
        "TIME" => row
            .try_get::<chrono::NaiveTime, _>(idx)
            .map(|t| JsonValue::String(t.to_string()))
            .unwrap_or(JsonValue::Null),
        "BYTEA" => row
            .try_get::<Vec<u8>, _>(idx)
            .map(|b| JsonValue::String(format!("<BYTEA {} bytes>", b.len())))
            .unwrap_or(JsonValue::Null),
        _ => row
            .try_get::<String, _>(idx)
            .map(JsonValue::String)
            .or_else(|_| row.try_get::<i64, _>(idx).map(JsonValue::from))
            .or_else(|_| {
                row.try_get::<f64, _>(idx).map(|f| {
                    serde_json::Number::from_f64(f)
                        .map(JsonValue::Number)
                        .unwrap_or(JsonValue::Null)
                })
            })
            .unwrap_or_else(|_| {
                raw.as_str()
                    .map(|s| JsonValue::String(s.to_string()))
                    .unwrap_or(JsonValue::Null)
            }),
    }
}
