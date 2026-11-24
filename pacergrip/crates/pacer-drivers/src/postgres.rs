use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde_json::Value as JsonValue;
use sqlx::pool::PoolConnection;
use sqlx::postgres::{PgConnectOptions, PgPool, PgPoolOptions, PgRow, PgSslMode};
use sqlx::{Column, Executor, PgConnection, Postgres, Row, TypeInfo, ValueRef};
use tokio::sync::{mpsc, Mutex};

use pacer_core::{
    CapabilityFlags, ColumnMeta, CoreError, CoreResult, DatabaseMeta, DbBackend, DirectConfig,
    ExplainResult, ForeignKeyMeta, IndexMeta, ObjectSearchHit, PingMetrics, QueryChunk, QueryReq,
    QuerySummary, RoutineMeta, TableMeta, TransactionStatus,
};
use pacer_sql_ast::{analyze_sql, SqlAccessKind};

use crate::sqlite::stream_chunks;
use crate::ssh_tunnel::{open_ssh_tunnel, SshTunnelHandle};
use crate::{
    build_connection_init_stmts, classify_tx_statement, resolve_pool_limits, TxStatementKind,
};

struct PgTxSession {
    manual_mode: bool,
    conn: Option<PoolConnection<Postgres>>,
    in_transaction: bool,
    pending_statements: u32,
}

/// Native PostgreSQL / openGauss / KingbaseES backend powered by `sqlx`.
pub struct PostgresBackend {
    pool: PgPool,
    read_only: bool,
    engine_label: String,
    engine_type: pacer_core::DbEngineType,
    tx: Mutex<PgTxSession>,
    _ssh_tunnel: Option<SshTunnelHandle>,
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

        let ssh_tunnel = match config.ssh.as_ref().filter(|s| s.enabled) {
            Some(ssh_cfg) => Some(open_ssh_tunnel(ssh_cfg, host, port).await?),
            None => None,
        };
        let (connect_host, connect_port) = match &ssh_tunnel {
            Some(t) => ("127.0.0.1", t.local_port()),
            None => (host, port),
        };

        let mut opts = PgConnectOptions::new()
            .host(connect_host)
            .port(connect_port)
            .username(config.username.as_deref().unwrap_or("postgres"));

        if let Some(ssl_mode) = resolve_pg_ssl_mode(config) {
            opts = opts.ssl_mode(ssl_mode);
        }
        if let Some(ref ssl) = config.ssl {
            if ssl.enabled {
                if let Some(ca) = ssl.ca_cert.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                    opts = opts.ssl_root_cert(ca);
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
        let init_stmts =
            build_connection_init_stmts(config.options.as_ref(), Some("SET TIME ZONE "));
        let mut pool_opts = PgPoolOptions::new()
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
            .map_err(|e| CoreError::Transport(format!("PostgreSQL connection failed: {e}")))?;

        let manual_mode = config
            .options
            .as_ref()
            .and_then(|o| o.transaction_control.as_deref())
            .is_some_and(|m| m.eq_ignore_ascii_case("manual"));

        Ok(Self {
            pool,
            read_only: config.read_only,
            engine_label: config.db_type.as_pacer_db_type().to_string(),
            engine_type: config.db_type.clone(),
            tx: Mutex::new(PgTxSession {
                manual_mode,
                conn: None,
                in_transaction: false,
                pending_statements: 0,
            }),
            _ssh_tunnel: ssh_tunnel,
        })
    }
}

fn resolve_pg_ssl_mode(config: &DirectConfig) -> Option<PgSslMode> {
    if let Some(ref ssl) = config.ssl {
        if !ssl.enabled {
            return Some(PgSslMode::Disable);
        }
        if let Some(ref mode) = ssl.mode {
            return parse_pg_ssl_mode_str(mode);
        }
    }
    for key in ["sslmode", "sslMode", "ssl-mode"] {
        if let Some(val) = config.driver_properties.get(key) {
            if let Some(mode) = parse_pg_ssl_mode_str(val) {
                return Some(mode);
            }
        }
    }
    if let Some(ssl) = config.driver_properties.get("ssl") {
        if ssl.eq_ignore_ascii_case("false") || ssl == "0" {
            return Some(PgSslMode::Disable);
        }
        if ssl.eq_ignore_ascii_case("true") || ssl == "1" {
            return Some(PgSslMode::Require);
        }
    }
    None
}

fn parse_pg_ssl_mode_str(raw: &str) -> Option<PgSslMode> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "disable" | "disabled" | "false" | "off" => Some(PgSslMode::Disable),
        "allow" => Some(PgSslMode::Allow),
        "prefer" | "preferred" => Some(PgSslMode::Prefer),
        "require" | "required" | "true" => Some(PgSslMode::Require),
        "verify-ca" | "verify_ca" => Some(PgSslMode::VerifyCa),
        "verify-full" | "verify_full" | "verify_identity" => Some(PgSslMode::VerifyFull),
        _ => None,
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

    async fn list_indexes(&self, db: &str, table: &str) -> CoreResult<Vec<IndexMeta>> {
        let schema = if db.is_empty() { "public" } else { db };
        let rows = sqlx::query(
            "SELECT indexname, indexdef \
             FROM pg_indexes \
             WHERE schemaname = $1 AND tablename = $2 \
             ORDER BY indexname",
        )
        .bind(schema)
        .bind(table)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|row| {
                let name: String = row.try_get("indexname").unwrap_or_default();
                let def: String = row.try_get("indexdef").unwrap_or_default();
                let unique = def.to_ascii_uppercase().contains("UNIQUE");
                let columns = def
                    .rfind('(')
                    .and_then(|start| def[start + 1..].find(')').map(|end| &def[start + 1..start + 1 + end]))
                    .map(|cols| {
                        cols.split(',')
                            .map(|c| c.trim().trim_matches('"').to_string())
                            .filter(|c| !c.is_empty())
                            .collect()
                    })
                    .unwrap_or_default();
                IndexMeta {
                    name,
                    unique,
                    columns,
                    index_type: Some("BTREE".to_string()),
                }
            })
            .collect())
    }

    async fn list_foreign_keys(
        &self,
        db: &str,
        table: &str,
    ) -> CoreResult<Vec<ForeignKeyMeta>> {
        let schema = if db.is_empty() { "public" } else { db };
        let rows = sqlx::query(
            "SELECT tc.constraint_name::text AS fk_name, \
                    kcu.column_name::text AS col_name, \
                    ccu.table_name::text AS ref_table, \
                    ccu.column_name::text AS ref_col, \
                    rc.delete_rule::text AS on_delete, \
                    rc.update_rule::text AS on_update \
             FROM information_schema.table_constraints tc \
             JOIN information_schema.key_column_usage kcu \
               ON tc.constraint_name = kcu.constraint_name \
              AND tc.table_schema = kcu.table_schema \
             JOIN information_schema.constraint_column_usage ccu \
               ON ccu.constraint_name = tc.constraint_name \
              AND ccu.constraint_schema = tc.constraint_schema \
             LEFT JOIN information_schema.referential_constraints rc \
               ON tc.constraint_name = rc.constraint_name \
              AND tc.constraint_schema = rc.constraint_schema \
             WHERE tc.constraint_type = 'FOREIGN KEY' \
               AND tc.table_schema = $1 \
               AND tc.table_name = $2 \
             ORDER BY tc.constraint_name, kcu.ordinal_position",
        )
        .bind(schema)
        .bind(table)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let name: String = row.try_get("fk_name").unwrap_or_default();
            let column_name: String = row.try_get("col_name").unwrap_or_default();
            let referenced_table: String = row.try_get("ref_table").unwrap_or_default();
            let referenced_column: String = row.try_get("ref_col").unwrap_or_default();
            if column_name.is_empty() || referenced_table.is_empty() {
                continue;
            }
            out.push(ForeignKeyMeta {
                name,
                column_name,
                referenced_table,
                referenced_column,
                on_delete: row.try_get::<Option<String>, _>("on_delete").ok().flatten(),
                on_update: row.try_get::<Option<String>, _>("on_update").ok().flatten(),
            });
        }
        Ok(out)
    }

    async fn list_routines(&self, db: &str) -> CoreResult<Vec<RoutineMeta>> {
        let schema = if db.is_empty() { "public" } else { db };
        let mut out = Vec::new();
        if let Ok(rows) = sqlx::query(
            "SELECT routine_name::text AS r_name, \
                    routine_type::text AS r_type, \
                    data_type::text AS r_ret, \
                    routine_definition::text AS r_def \
             FROM information_schema.routines \
             WHERE routine_schema = $1 \
             ORDER BY routine_type, routine_name \
             LIMIT 200",
        )
        .bind(schema)
        .fetch_all(&self.pool)
        .await
        {
            for row in rows {
                let name: String = row.try_get("r_name").unwrap_or_default();
                if name.is_empty() {
                    continue;
                }
                let kind = row
                    .try_get::<Option<String>, _>("r_type")
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| "FUNCTION".to_string())
                    .to_ascii_uppercase();
                out.push(RoutineMeta {
                    name,
                    kind,
                    return_type: row.try_get::<Option<String>, _>("r_ret").ok().flatten(),
                    definition: row.try_get::<Option<String>, _>("r_def").ok().flatten(),
                    comment: None,
                });
            }
        }

        if let Ok(rows) = sqlx::query(
            "SELECT trigger_name::text AS t_name, \
                    action_timing::text AS t_timing, \
                    event_manipulation::text AS t_event, \
                    event_object_table::text AS t_table, \
                    action_statement::text AS t_stmt \
             FROM information_schema.triggers \
             WHERE trigger_schema = $1 \
             ORDER BY trigger_name \
             LIMIT 200",
        )
        .bind(schema)
        .fetch_all(&self.pool)
        .await
        {
            for row in rows {
                let name: String = row.try_get("t_name").unwrap_or_default();
                if name.is_empty() {
                    continue;
                }
                let timing: String = row.try_get("t_timing").unwrap_or_default();
                let event: String = row.try_get("t_event").unwrap_or_default();
                let tbl: String = row.try_get("t_table").unwrap_or_default();
                let comment = format!("{timing} {event} ON {tbl}").trim().to_string();
                out.push(RoutineMeta {
                    name,
                    kind: "TRIGGER".to_string(),
                    return_type: None,
                    definition: row.try_get::<Option<String>, _>("t_stmt").ok().flatten(),
                    comment: if comment.is_empty() {
                        None
                    } else {
                        Some(comment)
                    },
                });
            }
        }
        Ok(out)
    }

    async fn search_objects(&self, db: &str, query: &str) -> CoreResult<Vec<ObjectSearchHit>> {
        let needle = query.trim();
        if needle.is_empty() {
            return Ok(Vec::new());
        }
        let schema = if db.is_empty() { "public" } else { db };
        let pattern = format!("%{needle}%");
        let mut hits = Vec::new();

        let tbl_rows = sqlx::query(
            "SELECT t.table_name, t.table_type, d.description AS table_comment \
             FROM information_schema.tables t \
             LEFT JOIN pg_catalog.pg_namespace n ON n.nspname = t.table_schema \
             LEFT JOIN pg_catalog.pg_class c ON c.relnamespace = n.oid AND c.relname = t.table_name \
             LEFT JOIN pg_catalog.pg_description d ON d.objoid = c.oid AND d.objsubid = 0 \
             WHERE t.table_schema = $1 AND (t.table_name ILIKE $2 OR COALESCE(d.description, '') ILIKE $2) \
             ORDER BY t.table_name LIMIT 80",
        )
        .bind(schema)
        .bind(&pattern)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;

        for row in tbl_rows {
            let table_name: String = row.try_get("table_name").unwrap_or_default();
            let table_type: String = row.try_get("table_type").unwrap_or_default();
            let comment: Option<String> = row
                .try_get::<Option<String>, _>("table_comment")
                .ok()
                .flatten()
                .filter(|s| !s.is_empty());
            let kind = if table_type.to_ascii_uppercase().contains("VIEW") {
                "view"
            } else {
                "table"
            };
            hits.push(ObjectSearchHit {
                kind: kind.to_string(),
                database: schema.to_string(),
                table_name,
                column_name: None,
                data_type: Some(table_type),
                comment,
            });
        }

        let col_rows = sqlx::query(
            "SELECT col.table_name, col.column_name, col.data_type, d.description AS col_comment \
             FROM information_schema.columns col \
             LEFT JOIN pg_catalog.pg_namespace n ON n.nspname = col.table_schema \
             LEFT JOIN pg_catalog.pg_class c ON c.relnamespace = n.oid AND c.relname = col.table_name \
             LEFT JOIN pg_catalog.pg_attribute a ON a.attrelid = c.oid AND a.attname = col.column_name \
             LEFT JOIN pg_catalog.pg_description d ON d.objoid = c.oid AND d.objsubid = a.attnum \
             WHERE col.table_schema = $1 AND (col.column_name ILIKE $2 OR COALESCE(d.description, '') ILIKE $2) \
             ORDER BY col.table_name, col.ordinal_position LIMIT 120",
        )
        .bind(schema)
        .bind(&pattern)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;

        for row in col_rows {
            if hits.len() >= 150 {
                break;
            }
            hits.push(ObjectSearchHit {
                kind: "column".to_string(),
                database: schema.to_string(),
                table_name: row.try_get("table_name").unwrap_or_default(),
                column_name: row.try_get::<String, _>("column_name").ok(),
                data_type: row.try_get::<String, _>("data_type").ok(),
                comment: row
                    .try_get::<Option<String>, _>("col_comment")
                    .ok()
                    .flatten()
                    .filter(|s| !s.is_empty()),
            });
        }

        Ok(hits)
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

        let schema = req
            .database
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "public".to_string());
        let stmt_kind = classify_tx_statement(&req.sql);
        let mut tx_guard = self.tx.lock().await;

        if !tx_guard.manual_mode
            && !tx_guard.in_transaction
            && matches!(stmt_kind, TxStatementKind::Other)
        {
            drop(tx_guard);
            let mut conn = self
                .pool
                .acquire()
                .await
                .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;
            let set_path = format!("SET search_path TO \"{}\", public", schema.replace('"', "\"\""));
            let _ = conn.execute(set_path.as_str()).await;
            return exec_pg_on_conn(
                &mut conn,
                &req.sql,
                &schema,
                req.limit,
                is_read_only,
                &self.engine_label,
                &tx,
                started,
            )
            .await;
        }

        if tx_guard.conn.is_none() {
            let conn = self
                .pool
                .acquire()
                .await
                .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;
            tx_guard.conn = Some(conn);
        }

        if !tx_guard.in_transaction {
            if let Some(conn) = tx_guard.conn.as_deref_mut() {
                let set_path = format!("SET search_path TO \"{}\", public", schema.replace('"', "\"\""));
                let _ = conn.execute(set_path.as_str()).await;
            }
        }

        match stmt_kind {
            TxStatementKind::Begin => {
                if !tx_guard.in_transaction {
                    if let Some(conn) = tx_guard.conn.as_deref_mut() {
                        conn.execute("BEGIN")
                            .await
                            .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;
                    }
                    tx_guard.in_transaction = true;
                    tx_guard.pending_statements = 0;
                }
                send_pg_empty_chunk(&tx).await;
                return Ok(QuerySummary {
                    success: true,
                    database: schema,
                    total_rows: 0,
                    affected_rows: Some(0),
                    is_dml_or_ddl: true,
                    duration_ms: started.elapsed().as_millis() as u64,
                    route_node: Some("DIRECT".to_string()),
                    route_description: Some(format!("{} manual transaction started", self.engine_label)),
                    masked_columns: BTreeMap::new(),
                    error_message: None,
                });
            }
            TxStatementKind::Commit => {
                if tx_guard.in_transaction {
                    if let Some(conn) = tx_guard.conn.as_deref_mut() {
                        conn.execute("COMMIT")
                            .await
                            .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;
                    }
                }
                tx_guard.in_transaction = false;
                tx_guard.pending_statements = 0;
                tx_guard.conn = None;
                send_pg_empty_chunk(&tx).await;
                return Ok(QuerySummary {
                    success: true,
                    database: schema,
                    total_rows: 0,
                    affected_rows: Some(0),
                    is_dml_or_ddl: true,
                    duration_ms: started.elapsed().as_millis() as u64,
                    route_node: Some("DIRECT".to_string()),
                    route_description: Some(format!("{} transaction committed", self.engine_label)),
                    masked_columns: BTreeMap::new(),
                    error_message: None,
                });
            }
            TxStatementKind::Rollback => {
                if tx_guard.in_transaction {
                    if let Some(conn) = tx_guard.conn.as_deref_mut() {
                        conn.execute("ROLLBACK")
                            .await
                            .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;
                    }
                }
                tx_guard.in_transaction = false;
                tx_guard.pending_statements = 0;
                tx_guard.conn = None;
                send_pg_empty_chunk(&tx).await;
                return Ok(QuerySummary {
                    success: true,
                    database: schema,
                    total_rows: 0,
                    affected_rows: Some(0),
                    is_dml_or_ddl: true,
                    duration_ms: started.elapsed().as_millis() as u64,
                    route_node: Some("DIRECT".to_string()),
                    route_description: Some(format!("{} transaction rolled back", self.engine_label)),
                    masked_columns: BTreeMap::new(),
                    error_message: None,
                });
            }
            TxStatementKind::Other => {
                if !tx_guard.in_transaction && tx_guard.manual_mode && !is_read_only {
                    if let Some(conn) = tx_guard.conn.as_deref_mut() {
                        conn.execute("BEGIN")
                            .await
                            .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;
                    }
                    tx_guard.in_transaction = true;
                }
                let conn = tx_guard
                    .conn
                    .as_deref_mut()
                    .ok_or_else(|| CoreError::Transport("Missing PostgreSQL session connection".to_string()))?;
                let summary = exec_pg_on_conn(
                    conn,
                    &req.sql,
                    &schema,
                    req.limit,
                    is_read_only,
                    &self.engine_label,
                    &tx,
                    started,
                )
                .await?;
                if tx_guard.in_transaction && !is_read_only {
                    tx_guard.pending_statements = tx_guard.pending_statements.saturating_add(1);
                } else if !tx_guard.in_transaction && !tx_guard.manual_mode {
                    tx_guard.conn = None;
                }
                return Ok(summary);
            }
        }
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

    async fn transaction_status(&self) -> CoreResult<TransactionStatus> {
        let g = self.tx.lock().await;
        Ok(TransactionStatus {
            supported: true,
            manual_mode: g.manual_mode,
            in_transaction: g.in_transaction,
            pending_statements: g.pending_statements,
        })
    }

    async fn set_manual_transaction(&self, enabled: bool) -> CoreResult<TransactionStatus> {
        let mut g = self.tx.lock().await;
        if !enabled && g.in_transaction {
            if let Some(conn) = g.conn.as_deref_mut() {
                let _ = conn.execute("ROLLBACK").await;
            }
            g.in_transaction = false;
            g.pending_statements = 0;
            g.conn = None;
        }
        g.manual_mode = enabled;
        Ok(TransactionStatus {
            supported: true,
            manual_mode: g.manual_mode,
            in_transaction: g.in_transaction,
            pending_statements: g.pending_statements,
        })
    }

    async fn commit_transaction(&self) -> CoreResult<TransactionStatus> {
        let mut g = self.tx.lock().await;
        if g.in_transaction {
            if let Some(conn) = g.conn.as_deref_mut() {
                conn.execute("COMMIT")
                    .await
                    .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;
            }
            g.in_transaction = false;
            g.pending_statements = 0;
            g.conn = None;
        }
        Ok(TransactionStatus {
            supported: true,
            manual_mode: g.manual_mode,
            in_transaction: false,
            pending_statements: 0,
        })
    }

    async fn rollback_transaction(&self) -> CoreResult<TransactionStatus> {
        let mut g = self.tx.lock().await;
        if g.in_transaction {
            if let Some(conn) = g.conn.as_deref_mut() {
                conn.execute("ROLLBACK")
                    .await
                    .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;
            }
            g.in_transaction = false;
            g.pending_statements = 0;
            g.conn = None;
        }
        Ok(TransactionStatus {
            supported: true,
            manual_mode: g.manual_mode,
            in_transaction: false,
            pending_statements: 0,
        })
    }

    fn capabilities(&self) -> CapabilityFlags {
        CapabilityFlags::direct_full(self.read_only)
    }
}

async fn send_pg_empty_chunk(tx: &mpsc::Sender<QueryChunk>) {
    let _ = tx
        .send(QueryChunk {
            columns: Vec::new(),
            rows: Vec::new(),
            batch_index: 0,
            is_last: true,
            masked_columns: BTreeMap::new(),
        })
        .await;
}

#[allow(clippy::too_many_arguments)]
async fn exec_pg_on_conn(
    conn: &mut PgConnection,
    sql: &str,
    schema: &str,
    limit: Option<u32>,
    is_read_only: bool,
    engine_label: &str,
    tx: &mpsc::Sender<QueryChunk>,
    started: Instant,
) -> CoreResult<QuerySummary> {
    if !is_read_only {
        let res = conn
            .execute(sql)
            .await
            .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;
        send_pg_empty_chunk(tx).await;
        return Ok(QuerySummary {
            success: true,
            database: schema.to_string(),
            total_rows: 0,
            affected_rows: Some(res.rows_affected()),
            is_dml_or_ddl: true,
            duration_ms: started.elapsed().as_millis() as u64,
            route_node: Some("DIRECT".to_string()),
            route_description: Some(format!("{engine_label} native direct execution")),
            masked_columns: BTreeMap::new(),
            error_message: None,
        });
    }

    let rows = conn
        .fetch_all(sql)
        .await
        .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;

    let max_rows = limit.map(|l| l as usize).unwrap_or(usize::MAX);
    let limited: Vec<&PgRow> = rows.iter().take(max_rows).collect();
    let columns: Vec<String> = if let Some(first) = rows.first() {
        first.columns().iter().map(|c| c.name().to_string()).collect()
    } else {
        match conn.describe(sql).await {
            Ok(desc) => desc
                .columns()
                .iter()
                .map(|c| c.name().to_string())
                .collect(),
            Err(_) => Vec::new(),
        }
    };

    let mapped: Vec<BTreeMap<String, JsonValue>> =
        limited.iter().map(|r| pg_row_to_map(r, &columns)).collect();
    let total_rows = mapped.len();
    stream_chunks(&columns, mapped, tx).await;

    Ok(QuerySummary {
        success: true,
        database: schema.to_string(),
        total_rows,
        affected_rows: None,
        is_dml_or_ddl: false,
        duration_ms: started.elapsed().as_millis() as u64,
        route_node: Some("DIRECT".to_string()),
        route_description: Some(format!("{engine_label} native direct execution")),
        masked_columns: BTreeMap::new(),
        error_message: None,
    })
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
            .map(crate::i64_to_json)
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
            .or_else(|_| row.try_get::<i64, _>(idx).map(crate::i64_to_json))
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
