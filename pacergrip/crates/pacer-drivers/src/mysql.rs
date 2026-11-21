use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde_json::Value as JsonValue;
use sqlx::mysql::{MySqlConnectOptions, MySqlPool, MySqlPoolOptions, MySqlRow, MySqlSslMode};
use sqlx::{Column, Executor, Row, TypeInfo, ValueRef};
use tokio::sync::mpsc;

use pacer_core::{
    CapabilityFlags, ColumnMeta, CoreError, CoreResult, DatabaseMeta, DbBackend, DirectConfig,
    ExplainResult, IndexMeta, PingMetrics, QueryChunk, QueryReq, QuerySummary, TableMeta,
};
use pacer_sql_ast::{analyze_sql, SqlAccessKind};

use crate::sqlite::stream_chunks;
use crate::{build_connection_init_stmts, resolve_pool_limits};

/// Native MySQL / MariaDB / TiDB / OceanBase backend powered by `sqlx`.
pub struct MySqlBackend {
    pool: MySqlPool,
    default_db: Option<String>,
    read_only: bool,
    engine_label: String,
    engine_type: pacer_core::DbEngineType,
}

impl MySqlBackend {
    pub async fn connect(config: &DirectConfig, password: Option<&str>) -> CoreResult<Self> {
        let host = config
            .host
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or("localhost");
        let port = config
            .port
            .or_else(|| config.db_type.default_port())
            .unwrap_or(3306);

        let mut opts = MySqlConnectOptions::new()
            .host(host)
            .port(port)
            .username(config.username.as_deref().unwrap_or("root"));

        if config.connection_type.as_deref() == Some("socket") {
            if let Some(sock) = config.socket_path.as_deref().filter(|s| !s.trim().is_empty()) {
                opts = opts.socket(sock);
            }
        }

        if let Some(ssl_mode) = resolve_mysql_ssl_mode(config) {
            opts = opts.ssl_mode(ssl_mode);
        }
        if let Some(ref ssl) = config.ssl {
            if ssl.enabled {
                if let Some(ca) = ssl.ca_cert.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                    opts = opts.ssl_ca(ca);
                }
                if let Some(cert) = ssl.client_cert.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                    opts = opts.ssl_client_cert(cert);
                }
                if let Some(key) = ssl.client_key.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                    opts = opts.ssl_client_key(key);
                }
            }
        }

        if let Some(pwd) = password {
            opts = opts.password(pwd);
        }
        if let Some(db) = config.database.as_deref().filter(|s| !s.is_empty()) {
            opts = opts.database(db);
        }

        let (max_conns, idle_timeout, test_before_acquire) =
            resolve_pool_limits(config.options.as_ref());
        let init_stmts = build_connection_init_stmts(config.options.as_ref(), Some("SET time_zone = "));
        let mut pool_opts = MySqlPoolOptions::new()
            .max_connections(max_conns)
            .acquire_timeout(Duration::from_secs(8))
            .test_before_acquire(test_before_acquire);
        if let Some(idle) = idle_timeout {
            pool_opts = pool_opts.idle_timeout(Some(idle));
        }
        if !init_stmts.is_empty() {
            let stmts = init_stmts.clone();
            pool_opts = pool_opts.after_connect(move |conn, _meta| {
                let stmts = stmts.clone();
                Box::pin(async move {
                    for stmt in &stmts {
                        let _ = conn.execute(stmt.as_str()).await;
                    }
                    Ok(())
                })
            });
        }

        let pool = pool_opts
            .connect_with(opts)
            .await
            .map_err(|e| CoreError::Transport(format!("MySQL connection failed: {e}")))?;

        Ok(Self {
            pool,
            default_db: config.database.clone(),
            read_only: config.read_only,
            engine_label: config.db_type.as_pacer_db_type().to_string(),
            engine_type: config.db_type.clone(),
        })
    }
}

#[async_trait]
impl DbBackend for MySqlBackend {
    async fn test_connection(&self) -> CoreResult<PingMetrics> {
        let started = Instant::now();
        let version: String = sqlx::query_scalar("SELECT VERSION()")
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
        // Universal query compatible with MySQL 5.6/5.7/8.0, MariaDB, OceanBase, and TiDB 6.5.5+
        // (avoids MySQL 8.0-only `CAST(... AS DOUBLE)` and heavy cross-table joins on large clusters).
        let primary = sqlx::query(
            "SELECT SCHEMA_NAME AS schema_name, \
                    DEFAULT_CHARACTER_SET_NAME AS charset, \
                    DEFAULT_COLLATION_NAME AS collation \
             FROM information_schema.SCHEMATA \
             ORDER BY SCHEMA_NAME",
        )
        .fetch_all(&self.pool)
        .await;

        let mut out = Vec::new();
        match primary {
            Ok(rows) => {
                out.reserve(rows.len());
                for row in rows {
                    let name: String = row.try_get("schema_name").unwrap_or_default();
                    let is_sys = is_mysql_system_schema(&name);
                    out.push(DatabaseMeta {
                        name,
                        charset: row.try_get("charset").ok(),
                        collation: row.try_get("collation").ok(),
                        table_count: None,
                        data_size_mb: None,
                        is_system: is_sys,
                        comment: None,
                    });
                }
            }
            Err(_) => {
                // Fallback for highly restricted accounts that can run `SHOW DATABASES`.
                let rows = sqlx::query("SHOW DATABASES")
                    .fetch_all(&self.pool)
                    .await
                    .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;
                for row in rows {
                    let name: String = row.try_get::<String, _>(0).unwrap_or_default();
                    let is_sys = is_mysql_system_schema(&name);
                    out.push(DatabaseMeta {
                        name,
                        charset: None,
                        collation: None,
                        table_count: None,
                        data_size_mb: None,
                        is_system: is_sys,
                        comment: None,
                    });
                }
            }
        }
        Ok(out)
    }

    async fn list_tables(&self, db: &str) -> CoreResult<Vec<TableMeta>> {
        let rows = sqlx::query(
            "SELECT TABLE_NAME AS table_name, TABLE_TYPE AS table_type, ENGINE AS engine, \
                    TABLE_ROWS AS table_rows, DATA_LENGTH AS data_length, TABLE_COMMENT AS table_comment \
             FROM information_schema.TABLES \
             WHERE TABLE_SCHEMA = ? \
             ORDER BY TABLE_NAME",
        )
        .bind(db)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let name: String = row.try_get("table_name").unwrap_or_default();
            let data_len: Option<u64> = row.try_get::<Option<u64>, _>("data_length").ok().flatten();
            let est_rows: Option<u64> = row.try_get::<Option<u64>, _>("table_rows").ok().flatten();
            let comment: Option<String> = row
                .try_get::<Option<String>, _>("table_comment")
                .ok()
                .flatten()
                .filter(|s| !s.is_empty());
            out.push(TableMeta {
                name,
                table_type: row.try_get("table_type").ok(),
                engine: row.try_get("engine").ok(),
                estimated_rows: est_rows,
                data_size_kb: data_len.map(|b| b as f64 / 1024.0),
                comment,
            });
        }
        Ok(out)
    }

    async fn list_columns(&self, db: &str, table: &str) -> CoreResult<Vec<ColumnMeta>> {
        let rows = sqlx::query(
            "SELECT COLUMN_NAME AS col_name, COLUMN_TYPE AS col_type, IS_NULLABLE AS is_nullable, \
                    COLUMN_KEY AS col_key, COLUMN_DEFAULT AS col_default, COLUMN_COMMENT AS col_comment \
             FROM information_schema.COLUMNS \
             WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ? \
             ORDER BY ORDINAL_POSITION",
        )
        .bind(db)
        .bind(table)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let name: String = row.try_get("col_name").unwrap_or_default();
            let data_type: String = row
                .try_get::<String, _>("col_type")
                .unwrap_or_else(|_| "unknown".to_string());
            let nullable = row
                .try_get::<String, _>("is_nullable")
                .map(|v| !v.eq_ignore_ascii_case("NO"))
                .unwrap_or(true);
            let is_primary_key = row
                .try_get::<String, _>("col_key")
                .map(|k| k.eq_ignore_ascii_case("PRI"))
                .unwrap_or(false);
            let default_value: Option<String> = row.try_get("col_default").ok().flatten();
            let comment: Option<String> = row
                .try_get::<Option<String>, _>("col_comment")
                .ok()
                .flatten()
                .filter(|s| !s.is_empty());

            out.push(ColumnMeta {
                name,
                data_type,
                nullable,
                is_primary_key,
                default_value,
                comment,
                masking_rule: None,
            });
        }
        Ok(out)
    }

    async fn list_indexes(&self, db: &str, table: &str) -> CoreResult<Vec<IndexMeta>> {
        let rows = sqlx::query(
            "SELECT INDEX_NAME AS idx_name, NON_UNIQUE AS non_unique, \
                    COLUMN_NAME AS col_name, INDEX_TYPE AS idx_type \
             FROM information_schema.STATISTICS \
             WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ? \
             ORDER BY INDEX_NAME, SEQ_IN_INDEX",
        )
        .bind(db)
        .bind(table)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;

        let mut grouped: BTreeMap<String, IndexMeta> = BTreeMap::new();
        for row in rows {
            let name: String = row.try_get("idx_name").unwrap_or_default();
            let non_unique: i64 = row.try_get("non_unique").unwrap_or(1);
            let col_name: String = row.try_get("col_name").unwrap_or_default();
            let idx_type: Option<String> = row.try_get("idx_type").ok();
            let entry = grouped.entry(name.clone()).or_insert_with(|| IndexMeta {
                name,
                unique: non_unique == 0,
                columns: Vec::new(),
                index_type: idx_type,
            });
            if !col_name.is_empty() {
                entry.columns.push(col_name);
            }
        }
        Ok(grouped.into_values().collect())
    }

    async fn get_table_ddl(&self, db: &str, table: &str) -> CoreResult<String> {
        let sql = format!(
            "SHOW CREATE TABLE `{}`.`{}`",
            db.replace('`', "``"),
            table.replace('`', "``")
        );
        let row = sqlx::query(&sql)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;
        row.try_get::<String, _>(1)
            .map_err(|e| CoreError::ExecutionFailed(e.to_string()))
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

        let active_db = req
            .database
            .clone()
            .or_else(|| self.default_db.clone())
            .filter(|s| !s.is_empty());

        if let Some(ref db) = active_db {
            let use_sql = format!("USE `{}`", db.replace('`', "``"));
            let _ = conn.execute(use_sql.as_str()).await;
        }

        let db_label = active_db.unwrap_or_else(|| "default".to_string());

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
                database: db_label,
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
        let limited: Vec<&MySqlRow> = rows.iter().take(max_rows).collect();
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

        let mapped: Vec<BTreeMap<String, JsonValue>> = limited
            .iter()
            .map(|r| mysql_row_to_map(r, &columns))
            .collect();
        let total_rows = mapped.len();
        stream_chunks(&columns, mapped, &tx).await;

        Ok(QuerySummary {
            success: true,
            database: db_label,
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
        if !db.is_empty() && db != "default" {
            let use_sql = format!("USE `{}`", db.replace('`', "``"));
            let _ = conn.execute(use_sql.as_str()).await;
        }
        let trimmed = sql.trim().trim_end_matches(';');
        let upper = trimmed.to_ascii_uppercase();
        let explain_sql = if upper.starts_with("EXPLAIN")
            || upper.starts_with("ANALYZE ")
            || upper.starts_with("ANALYZE\n")
            || upper.starts_with("ANALYZE\t")
        {
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
        let mapped = rows.iter().map(|r| mysql_row_to_map(r, &columns)).collect();
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

fn is_mysql_system_schema(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "information_schema"
            | "performance_schema"
            | "mysql"
            | "sys"
            | "metrics_schema"
            | "ndbinfo"
            | "oceanbase"
            | "__recycle_bin__"
            | "__cdb_recycle_bin__"
    ) || lower.starts_with("__")
}

fn resolve_mysql_ssl_mode(config: &DirectConfig) -> Option<MySqlSslMode> {
    if let Some(ref ssl) = config.ssl {
        if !ssl.enabled {
            return Some(MySqlSslMode::Disabled);
        }
        if let Some(ref mode) = ssl.mode {
            return parse_mysql_ssl_mode_str(mode);
        }
    }
    for key in ["sslMode", "ssl-mode", "sslmode"] {
        if let Some(val) = config.driver_properties.get(key) {
            if let Some(mode) = parse_mysql_ssl_mode_str(val) {
                return Some(mode);
            }
        }
    }
    if let Some(use_ssl) = config
        .driver_properties
        .get("useSSL")
        .or_else(|| config.driver_properties.get("ssl"))
    {
        if use_ssl.eq_ignore_ascii_case("false") || use_ssl == "0" {
            return Some(MySqlSslMode::Disabled);
        }
        if use_ssl.eq_ignore_ascii_case("true") || use_ssl == "1" {
            return Some(MySqlSslMode::Required);
        }
    }
    None
}

fn parse_mysql_ssl_mode_str(raw: &str) -> Option<MySqlSslMode> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "disabled" | "disable" | "false" | "off" | "none" => Some(MySqlSslMode::Disabled),
        "preferred" | "prefer" => Some(MySqlSslMode::Preferred),
        "required" | "require" | "true" => Some(MySqlSslMode::Required),
        "verify_ca" | "verify-ca" => Some(MySqlSslMode::VerifyCa),
        "verify_identity" | "verify-identity" | "verify_full" | "verify-full" => {
            Some(MySqlSslMode::VerifyIdentity)
        }
        _ => None,
    }
}

fn mysql_row_to_map(row: &MySqlRow, columns: &[String]) -> BTreeMap<String, JsonValue> {
    let mut map = BTreeMap::new();
    for (idx, col_name) in columns.iter().enumerate() {
        map.insert(col_name.clone(), decode_mysql_cell(row, idx));
    }
    map
}

fn decode_mysql_cell(row: &MySqlRow, idx: usize) -> JsonValue {
    let Ok(raw) = row.try_get_raw(idx) else {
        return JsonValue::Null;
    };
    if raw.is_null() {
        return JsonValue::Null;
    }

    let type_name = row.column(idx).type_info().name().to_ascii_uppercase();
    match type_name.as_str() {
        "BOOLEAN" | "BOOL" => row
            .try_get::<bool, _>(idx)
            .map(JsonValue::Bool)
            .unwrap_or(JsonValue::Null),
        "TINYINT" | "SMALLINT" | "INT" | "MEDIUMINT" | "BIGINT" => row
            .try_get::<i64, _>(idx)
            .map(JsonValue::from)
            .unwrap_or(JsonValue::Null),
        "TINYINT UNSIGNED" | "SMALLINT UNSIGNED" | "INT UNSIGNED" | "MEDIUMINT UNSIGNED"
        | "BIGINT UNSIGNED" => row
            .try_get::<u64, _>(idx)
            .map(JsonValue::from)
            .unwrap_or(JsonValue::Null),
        "FLOAT" | "DOUBLE" => row
            .try_get::<f64, _>(idx)
            .ok()
            .and_then(serde_json::Number::from_f64)
            .map(JsonValue::Number)
            .unwrap_or(JsonValue::Null),
        "JSON" => row
            .try_get::<JsonValue, _>(idx)
            .unwrap_or_else(|_| JsonValue::String(String::new())),
        "DATETIME" | "TIMESTAMP" => row
            .try_get::<chrono::NaiveDateTime, _>(idx)
            .map(|dt| JsonValue::String(dt.format("%Y-%m-%d %H:%M:%S").to_string()))
            .or_else(|_| {
                row.try_get::<chrono::DateTime<chrono::Utc>, _>(idx)
                    .map(|dt| JsonValue::String(dt.to_rfc3339()))
            })
            .unwrap_or(JsonValue::Null),
        "DATE" => row
            .try_get::<chrono::NaiveDate, _>(idx)
            .map(|d| JsonValue::String(d.to_string()))
            .unwrap_or(JsonValue::Null),
        "TIME" => row
            .try_get::<chrono::NaiveTime, _>(idx)
            .map(|t| JsonValue::String(t.to_string()))
            .unwrap_or(JsonValue::Null),
        "BLOB" | "TINYBLOB" | "MEDIUMBLOB" | "LONGBLOB" | "BINARY" | "VARBINARY" => row
            .try_get::<Vec<u8>, _>(idx)
            .map(|b| JsonValue::String(format!("<BLOB {} bytes>", b.len())))
            .unwrap_or(JsonValue::Null),
        _ => row
            .try_get::<String, _>(idx)
            .map(JsonValue::String)
            .or_else(|_| {
                row.try_get::<i64, _>(idx)
                    .map(JsonValue::from)
            })
            .or_else(|_| {
                row.try_get::<f64, _>(idx).map(|f| {
                    serde_json::Number::from_f64(f)
                        .map(JsonValue::Number)
                        .unwrap_or(JsonValue::Null)
                })
            })
            .unwrap_or_else(|_| {
                row.try_get::<Vec<u8>, _>(idx)
                    .map(|b| JsonValue::String(String::from_utf8_lossy(&b).into_owned()))
                    .unwrap_or(JsonValue::Null)
            }),
    }
}
