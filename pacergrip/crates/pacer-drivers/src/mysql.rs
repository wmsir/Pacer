use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex as StdMutex;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use futures_util::TryStreamExt;
use serde_json::Value as JsonValue;
use sqlx::mysql::{MySqlConnectOptions, MySqlPool, MySqlPoolOptions, MySqlRow, MySqlSslMode};
use sqlx::pool::PoolConnection;
use sqlx::{Column, Executor, MySql, MySqlConnection, Row, TypeInfo, ValueRef};
use tokio::sync::{mpsc, Mutex};

use pacer_core::{
    CapabilityFlags, ColumnMeta, CoreError, CoreResult, DatabaseMeta, DbBackend, DirectConfig,
    ExplainResult, ForeignKeyMeta, IndexMeta, ObjectSearchHit, PingMetrics, QueryChunk, QueryReq,
    QuerySummary, RoutineMeta, TableMeta, TransactionStatus,
};
use pacer_sql_ast::{analyze_sql, SqlAccessKind};

use crate::sqlite::adaptive_chunk_size;
use crate::ssh_tunnel::{open_ssh_tunnel, SshTunnelHandle};
use crate::{
    build_connection_init_stmts, classify_tx_statement, resolve_pool_limits, TxStatementKind,
};

struct MySqlTxSession {
    manual_mode: bool,
    conn: Option<PoolConnection<MySql>>,
    in_transaction: bool,
    pending_statements: u32,
}

struct MySqlTxRunGuard<'a> {
    state: &'a StdMutex<MySqlTxSession>,
    restore_in_tx: bool,
    disarmed: bool,
}

impl<'a> Drop for MySqlTxRunGuard<'a> {
    fn drop(&mut self) {
        if !self.disarmed && self.restore_in_tx {
            if let Ok(mut g) = self.state.lock() {
                g.in_transaction = false;
                g.pending_statements = 0;
                g.conn = None;
            }
        }
    }
}

struct RunningQueryIdGuard<'a> {
    map: &'a StdMutex<HashMap<String, u64>>,
    query_id: Option<String>,
}

impl<'a> Drop for RunningQueryIdGuard<'a> {
    fn drop(&mut self) {
        if let Some(ref qid) = self.query_id {
            if let Ok(mut guard) = self.map.lock() {
                guard.remove(qid);
            }
        }
    }
}

/// Native MySQL / MariaDB / TiDB / OceanBase backend powered by `sqlx`.
pub struct MySqlBackend {
    pool: MySqlPool,
    default_db: Option<String>,
    read_only: bool,
    engine_label: String,
    engine_type: pacer_core::DbEngineType,
    tx: StdMutex<MySqlTxSession>,
    tx_op: Mutex<()>,
    running_conn_ids: StdMutex<HashMap<String, u64>>,
    _ssh_tunnel: Option<SshTunnelHandle>,
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

        let is_socket = config.connection_type.as_deref() == Some("socket")
            && config
                .socket_path
                .as_deref()
                .is_some_and(|s| !s.trim().is_empty());
        let ssh_tunnel = if !is_socket {
            match config.ssh.as_ref().filter(|s| s.enabled) {
                Some(ssh_cfg) => Some(open_ssh_tunnel(ssh_cfg, host, port).await?),
                None => None,
            }
        } else {
            None
        };
        let (connect_host, connect_port) = match &ssh_tunnel {
            Some(t) => ("127.0.0.1", t.local_port()),
            None => (host, port),
        };

        let mut opts = MySqlConnectOptions::new()
            .host(connect_host)
            .port(connect_port)
            .username(config.username.as_deref().unwrap_or("root"));

        if is_socket {
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

        let manual_mode = config
            .options
            .as_ref()
            .and_then(|o| o.transaction_control.as_deref())
            .is_some_and(|m| m.eq_ignore_ascii_case("manual"));

        Ok(Self {
            pool,
            default_db: config.database.clone(),
            read_only: config.read_only,
            engine_label: config.db_type.as_pacer_db_type().to_string(),
            engine_type: config.db_type.clone(),
            tx: StdMutex::new(MySqlTxSession {
                manual_mode,
                conn: None,
                in_transaction: false,
                pending_statements: 0,
            }),
            tx_op: Mutex::new(()),
            running_conn_ids: StdMutex::new(HashMap::new()),
            _ssh_tunnel: ssh_tunnel,
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

    async fn list_foreign_keys(
        &self,
        db: &str,
        table: &str,
    ) -> CoreResult<Vec<ForeignKeyMeta>> {
        if db.trim().is_empty() || table.trim().is_empty() {
            return Ok(Vec::new());
        }
        let rows = sqlx::query(
            "SELECT k.CONSTRAINT_NAME AS fk_name, \
                    k.COLUMN_NAME AS col_name, \
                    k.REFERENCED_TABLE_NAME AS ref_table, \
                    k.REFERENCED_COLUMN_NAME AS ref_col, \
                    rc.DELETE_RULE AS on_delete, \
                    rc.UPDATE_RULE AS on_update \
             FROM information_schema.KEY_COLUMN_USAGE k \
             LEFT JOIN information_schema.REFERENTIAL_CONSTRAINTS rc \
               ON k.CONSTRAINT_SCHEMA = rc.CONSTRAINT_SCHEMA \
              AND k.TABLE_NAME = rc.TABLE_NAME \
              AND k.CONSTRAINT_NAME = rc.CONSTRAINT_NAME \
             WHERE k.TABLE_SCHEMA = ? \
               AND k.TABLE_NAME = ? \
               AND k.REFERENCED_TABLE_NAME IS NOT NULL \
             ORDER BY k.CONSTRAINT_NAME, k.ORDINAL_POSITION",
        )
        .bind(db)
        .bind(table)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let name = mysql_get_opt_str(&row, "fk_name").unwrap_or_default();
            let column_name = mysql_get_opt_str(&row, "col_name").unwrap_or_default();
            let referenced_table = mysql_get_opt_str(&row, "ref_table").unwrap_or_default();
            let referenced_column = mysql_get_opt_str(&row, "ref_col").unwrap_or_default();
            if referenced_table.is_empty() || column_name.is_empty() {
                continue;
            }
            out.push(ForeignKeyMeta {
                name,
                column_name,
                referenced_table,
                referenced_column,
                on_delete: mysql_get_opt_str(&row, "on_delete"),
                on_update: mysql_get_opt_str(&row, "on_update"),
            });
        }
        Ok(out)
    }

    async fn list_routines(&self, db: &str) -> CoreResult<Vec<RoutineMeta>> {
        if db.trim().is_empty() {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        if let Ok(rows) = sqlx::query(
            "SELECT ROUTINE_NAME AS r_name, ROUTINE_TYPE AS r_type, \
                    DTD_IDENTIFIER AS r_ret, ROUTINE_DEFINITION AS r_def, \
                    ROUTINE_COMMENT AS r_comment \
             FROM information_schema.ROUTINES \
             WHERE ROUTINE_SCHEMA = ? \
             ORDER BY ROUTINE_TYPE, ROUTINE_NAME \
             LIMIT 200",
        )
        .bind(db)
        .fetch_all(&self.pool)
        .await
        {
            for row in rows {
                let name = mysql_get_opt_str(&row, "r_name").unwrap_or_default();
                if name.is_empty() {
                    continue;
                }
                let kind = mysql_get_opt_str(&row, "r_type")
                    .unwrap_or_else(|| "PROCEDURE".to_string())
                    .to_ascii_uppercase();
                out.push(RoutineMeta {
                    name,
                    kind,
                    return_type: mysql_get_opt_str(&row, "r_ret"),
                    definition: mysql_get_opt_str(&row, "r_def"),
                    comment: mysql_get_opt_str(&row, "r_comment").filter(|s| !s.is_empty()),
                });
            }
        }

        if let Ok(rows) = sqlx::query(
            "SELECT TRIGGER_NAME AS t_name, ACTION_TIMING AS t_timing, \
                    EVENT_MANIPULATION AS t_event, EVENT_OBJECT_TABLE AS t_table, \
                    ACTION_STATEMENT AS t_stmt \
             FROM information_schema.TRIGGERS \
             WHERE TRIGGER_SCHEMA = ? \
             ORDER BY TRIGGER_NAME \
             LIMIT 200",
        )
        .bind(db)
        .fetch_all(&self.pool)
        .await
        {
            for row in rows {
                let name = mysql_get_opt_str(&row, "t_name").unwrap_or_default();
                if name.is_empty() {
                    continue;
                }
                let timing = mysql_get_opt_str(&row, "t_timing").unwrap_or_default();
                let event = mysql_get_opt_str(&row, "t_event").unwrap_or_default();
                let tbl = mysql_get_opt_str(&row, "t_table").unwrap_or_default();
                let comment = format!("{timing} {event} ON {tbl}").trim().to_string();
                out.push(RoutineMeta {
                    name,
                    kind: "TRIGGER".to_string(),
                    return_type: None,
                    definition: mysql_get_opt_str(&row, "t_stmt"),
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
        if needle.is_empty() || db.trim().is_empty() {
            return Ok(Vec::new());
        }
        let pattern = format!("%{needle}%");
        let mut hits = Vec::new();

        let tbl_rows = sqlx::query(
            "SELECT TABLE_NAME AS table_name, TABLE_TYPE AS table_type, TABLE_COMMENT AS table_comment \
             FROM information_schema.TABLES \
             WHERE TABLE_SCHEMA = ? AND (TABLE_NAME LIKE ? OR TABLE_COMMENT LIKE ?) \
             ORDER BY TABLE_NAME LIMIT 80",
        )
        .bind(db)
        .bind(&pattern)
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
                database: db.to_string(),
                table_name,
                column_name: None,
                data_type: Some(table_type),
                comment,
            });
        }

        let col_rows = sqlx::query(
            "SELECT TABLE_NAME AS table_name, COLUMN_NAME AS col_name, COLUMN_TYPE AS col_type, \
                    COLUMN_COMMENT AS col_comment \
             FROM information_schema.COLUMNS \
             WHERE TABLE_SCHEMA = ? AND (COLUMN_NAME LIKE ? OR COLUMN_COMMENT LIKE ?) \
             ORDER BY TABLE_NAME, ORDINAL_POSITION LIMIT 120",
        )
        .bind(db)
        .bind(&pattern)
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
                database: db.to_string(),
                table_name: row.try_get("table_name").unwrap_or_default(),
                column_name: row.try_get::<String, _>("col_name").ok(),
                data_type: row.try_get::<String, _>("col_type").ok(),
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

        let active_db = req
            .database
            .clone()
            .or_else(|| self.default_db.clone())
            .filter(|s| !s.is_empty());
        let db_label = active_db.clone().unwrap_or_else(|| "default".to_string());
        let stmt_kind = classify_tx_statement(&req.sql);
        let (manual_mode, in_tx_snapshot) = {
            let g = self.tx.lock().map_err(|e| CoreError::Transport(e.to_string()))?;
            (g.manual_mode, g.in_transaction)
        };

        if !manual_mode && !in_tx_snapshot && matches!(stmt_kind, TxStatementKind::Other) {
            let mut conn = self
                .pool
                .acquire()
                .await
                .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;
            if let Some(ref db) = active_db {
                let use_sql = format!("USE `{}`", db.replace('`', "``"));
                let _ = conn.execute(use_sql.as_str()).await;
            }
            let registered_qid = register_mysql_running_id(
                &mut conn,
                &self.running_conn_ids,
                req.query_id.as_deref(),
            )
            .await;
            let _id_guard = RunningQueryIdGuard {
                map: &self.running_conn_ids,
                query_id: registered_qid,
            };
            let (summary, stopped_early) = exec_mysql_on_conn(
                &mut conn,
                &req.sql,
                &db_label,
                req.limit,
                is_read_only,
                &self.engine_label,
                &tx,
                started,
            )
            .await?;
            if stopped_early {
                let _ = conn.close().await;
            }
            return Ok(summary);
        }

        let _tx_op_guard = self.tx_op.lock().await;
        let (mut manual_mode, mut in_tx, mut conn_opt) = {
            let mut g = self.tx.lock().map_err(|e| CoreError::Transport(e.to_string()))?;
            (g.manual_mode, g.in_transaction, g.conn.take())
        };
        if conn_opt.is_none() {
            conn_opt = Some(
                self.pool
                    .acquire()
                    .await
                    .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?,
            );
        }
        let mut conn = conn_opt.ok_or_else(|| {
            CoreError::Transport("Missing MySQL session connection".to_string())
        })?;

        if !in_tx {
            if let Some(ref db) = active_db {
                let use_sql = format!("USE `{}`", db.replace('`', "``"));
                let _ = conn.execute(use_sql.as_str()).await;
            }
        }

        match stmt_kind {
            TxStatementKind::Begin => {
                if !in_tx {
                    conn.execute("START TRANSACTION")
                        .await
                        .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;
                    in_tx = true;
                }
                if let Ok(mut g) = self.tx.lock() {
                    g.in_transaction = in_tx;
                    g.pending_statements = 0;
                    g.conn = Some(conn);
                }
                send_mysql_empty_chunk(&tx).await;
                return Ok(QuerySummary {
                    success: true,
                    database: db_label,
                    total_rows: 0,
                    affected_rows: Some(0),
                    is_dml_or_ddl: true,
                    duration_ms: started.elapsed().as_millis() as u64,
                    route_node: Some("DIRECT".to_string()),
                    route_description: Some(format!(
                        "{} manual transaction started",
                        self.engine_label
                    )),
                    masked_columns: BTreeMap::new(),
                    error_message: None,
                });
            }
            TxStatementKind::Commit => {
                if in_tx {
                    conn.execute("COMMIT")
                        .await
                        .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;
                }
                if let Ok(mut g) = self.tx.lock() {
                    g.in_transaction = false;
                    g.pending_statements = 0;
                    g.conn = None;
                }
                send_mysql_empty_chunk(&tx).await;
                return Ok(QuerySummary {
                    success: true,
                    database: db_label,
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
                if in_tx {
                    conn.execute("ROLLBACK")
                        .await
                        .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;
                }
                if let Ok(mut g) = self.tx.lock() {
                    g.in_transaction = false;
                    g.pending_statements = 0;
                    g.conn = None;
                }
                send_mysql_empty_chunk(&tx).await;
                return Ok(QuerySummary {
                    success: true,
                    database: db_label,
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
                if !in_tx && manual_mode && !is_read_only {
                    conn.execute("START TRANSACTION")
                        .await
                        .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;
                    in_tx = true;
                    if let Ok(mut g) = self.tx.lock() {
                        g.in_transaction = true;
                    }
                }
                let registered_qid = register_mysql_running_id(
                    &mut conn,
                    &self.running_conn_ids,
                    req.query_id.as_deref(),
                )
                .await;
                let _id_guard = RunningQueryIdGuard {
                    map: &self.running_conn_ids,
                    query_id: registered_qid,
                };
                let mut run_guard = MySqlTxRunGuard {
                    state: &self.tx,
                    restore_in_tx: in_tx,
                    disarmed: false,
                };
                let (summary, stopped_early) = exec_mysql_on_conn(
                    &mut conn,
                    &req.sql,
                    &db_label,
                    req.limit,
                    is_read_only,
                    &self.engine_label,
                    &tx,
                    started,
                )
                .await?;
                run_guard.disarmed = true;
                if let Ok(mut g) = self.tx.lock() {
                    manual_mode = g.manual_mode;
                    g.in_transaction = in_tx;
                    if in_tx && !is_read_only {
                        g.pending_statements = g.pending_statements.saturating_add(1);
                    }
                    if in_tx || manual_mode {
                        g.conn = Some(conn);
                        return Ok(summary);
                    }
                }
                if stopped_early {
                    let _ = conn.close().await;
                }
                return Ok(summary);
            }
        }
    }

    async fn cancel_running_query(&self, query_id: &str) -> CoreResult<bool> {
        let target_id = {
            let guard = self
                .running_conn_ids
                .lock()
                .map_err(|e| CoreError::Transport(e.to_string()))?;
            guard.get(query_id).copied()
        };
        let Some(conn_id) = target_id else {
            return Ok(false);
        };
        let mut kill_conn = tokio::time::timeout(Duration::from_secs(3), self.pool.acquire())
            .await
            .map_err(|_| CoreError::Transport("Timed out acquiring MySQL kill connection".to_string()))?
            .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;
        let kill_sql = format!("KILL QUERY {conn_id}");
        let res = kill_conn.execute(kill_sql.as_str()).await;
        Ok(res.is_ok())
    }

    async fn explain(&self, db: &str, sql: &str) -> CoreResult<ExplainResult> {
        crate::validate_explain_sql(sql, &self.engine_type, self.read_only)?;
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

    async fn transaction_status(&self) -> CoreResult<TransactionStatus> {
        let g = self.tx.lock().map_err(|e| CoreError::Transport(e.to_string()))?;
        Ok(TransactionStatus {
            supported: true,
            manual_mode: g.manual_mode,
            in_transaction: g.in_transaction,
            pending_statements: g.pending_statements,
        })
    }

    async fn set_manual_transaction(&self, enabled: bool) -> CoreResult<TransactionStatus> {
        let _op = self.tx_op.lock().await;
        let (in_tx, conn_opt) = {
            let mut g = self.tx.lock().map_err(|e| CoreError::Transport(e.to_string()))?;
            g.manual_mode = enabled;
            if !enabled {
                (g.in_transaction, g.conn.take())
            } else {
                (false, None)
            }
        };
        if let Some(mut conn) = conn_opt {
            if in_tx {
                let _ = conn.execute("ROLLBACK").await;
            }
        }
        let mut g = self.tx.lock().map_err(|e| CoreError::Transport(e.to_string()))?;
        if !enabled {
            g.in_transaction = false;
            g.pending_statements = 0;
        }
        Ok(TransactionStatus {
            supported: true,
            manual_mode: g.manual_mode,
            in_transaction: g.in_transaction,
            pending_statements: g.pending_statements,
        })
    }

    async fn commit_transaction(&self) -> CoreResult<TransactionStatus> {
        let _op = self.tx_op.lock().await;
        let (in_tx, conn_opt) = {
            let mut g = self.tx.lock().map_err(|e| CoreError::Transport(e.to_string()))?;
            (g.in_transaction, g.conn.take())
        };
        if in_tx {
            if let Some(mut conn) = conn_opt {
                conn.execute("COMMIT")
                    .await
                    .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;
            }
        }
        let mut g = self.tx.lock().map_err(|e| CoreError::Transport(e.to_string()))?;
        g.in_transaction = false;
        g.pending_statements = 0;
        Ok(TransactionStatus {
            supported: true,
            manual_mode: g.manual_mode,
            in_transaction: false,
            pending_statements: 0,
        })
    }

    async fn rollback_transaction(&self) -> CoreResult<TransactionStatus> {
        let _op = self.tx_op.lock().await;
        let (in_tx, conn_opt) = {
            let mut g = self.tx.lock().map_err(|e| CoreError::Transport(e.to_string()))?;
            (g.in_transaction, g.conn.take())
        };
        if in_tx {
            if let Some(mut conn) = conn_opt {
                conn.execute("ROLLBACK")
                    .await
                    .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;
            }
        }
        let mut g = self.tx.lock().map_err(|e| CoreError::Transport(e.to_string()))?;
        g.in_transaction = false;
        g.pending_statements = 0;
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

async fn register_mysql_running_id(
    conn: &mut MySqlConnection,
    map: &StdMutex<HashMap<String, u64>>,
    query_id: Option<&str>,
) -> Option<String> {
    let qid = query_id?.trim();
    if qid.is_empty() {
        return None;
    }
    let conn_id: Option<u64> = sqlx::query_scalar("SELECT CONNECTION_ID()")
        .fetch_one(&mut *conn)
        .await
        .ok();
    if let Some(id) = conn_id {
        if let Ok(mut guard) = map.lock() {
            guard.insert(qid.to_string(), id);
        }
        Some(qid.to_string())
    } else {
        None
    }
}

fn mysql_get_opt_str(row: &MySqlRow, col: &str) -> Option<String> {
    if let Ok(v) = row.try_get::<Option<String>, _>(col) {
        return v;
    }
    if let Ok(v) = row.try_get::<Option<Vec<u8>>, _>(col) {
        return v.map(|b| String::from_utf8_lossy(&b).into_owned());
    }
    None
}

async fn send_mysql_empty_chunk(tx: &mpsc::Sender<QueryChunk>) {
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
async fn exec_mysql_on_conn(
    conn: &mut MySqlConnection,
    sql: &str,
    db_label: &str,
    limit: Option<u32>,
    is_read_only: bool,
    engine_label: &str,
    tx: &mpsc::Sender<QueryChunk>,
    started: Instant,
) -> CoreResult<(QuerySummary, bool)> {
    if !is_read_only {
        let res = conn
            .execute(sql)
            .await
            .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;
        send_mysql_empty_chunk(tx).await;
        return Ok((
            QuerySummary {
                success: true,
                database: db_label.to_string(),
                total_rows: 0,
                affected_rows: Some(res.rows_affected()),
                is_dml_or_ddl: true,
                duration_ms: started.elapsed().as_millis() as u64,
                route_node: Some("DIRECT".to_string()),
                route_description: Some(format!("{engine_label} native direct execution")),
                masked_columns: BTreeMap::new(),
                error_message: None,
            },
            false,
        ));
    }

    let max_rows = limit.map(|l| l as usize).unwrap_or(usize::MAX);
    let mut columns: Option<Vec<String>> = None;
    let mut chunk_size: usize = 500;
    let mut batch: Vec<BTreeMap<String, JsonValue>> = Vec::new();
    let mut batch_index: usize = 0;
    let mut total_rows: usize = 0;
    let mut stopped_early = false;
    let mut receiver_closed = false;

    {
        let mut stream = conn.fetch(sql);
        while let Some(row) = stream
            .try_next()
            .await
            .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?
        {
            if total_rows >= max_rows {
                stopped_early = true;
                break;
            }
            let cols = columns.get_or_insert_with(|| {
                let c: Vec<String> = row.columns().iter().map(|col| col.name().to_string()).collect();
                chunk_size = adaptive_chunk_size(c.len());
                c
            });
            batch.push(mysql_row_to_map(&row, cols));
            total_rows += 1;

            if batch.len() >= chunk_size && total_rows < max_rows {
                let send_rows = std::mem::take(&mut batch);
                let chunk = QueryChunk {
                    columns: if batch_index == 0 {
                        cols.clone()
                    } else {
                        Vec::new()
                    },
                    rows: send_rows,
                    batch_index,
                    is_last: false,
                    masked_columns: BTreeMap::new(),
                };
                batch_index += 1;
                if tx.send(chunk).await.is_err() {
                    receiver_closed = true;
                    stopped_early = true;
                    break;
                }
            }
        }
    }

    let final_cols = match columns {
        Some(c) => c,
        None => match conn.describe(sql).await {
            Ok(desc) => desc.columns().iter().map(|c| c.name().to_string()).collect(),
            Err(_) => Vec::new(),
        },
    };

    if !receiver_closed {
        let _ = tx
            .send(QueryChunk {
                columns: if batch_index == 0 {
                    final_cols
                } else {
                    Vec::new()
                },
                rows: batch,
                batch_index,
                is_last: true,
                masked_columns: BTreeMap::new(),
            })
            .await;
    }

    Ok((
        QuerySummary {
            success: true,
            database: db_label.to_string(),
            total_rows,
            affected_rows: None,
            is_dml_or_ddl: false,
            duration_ms: started.elapsed().as_millis() as u64,
            route_node: Some("DIRECT".to_string()),
            route_description: Some(format!("{engine_label} native direct execution")),
            masked_columns: BTreeMap::new(),
            error_message: None,
        },
        stopped_early,
    ))
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
            .map(crate::i64_to_json)
            .unwrap_or(JsonValue::Null),
        "TINYINT UNSIGNED" | "SMALLINT UNSIGNED" | "INT UNSIGNED" | "MEDIUMINT UNSIGNED"
        | "BIGINT UNSIGNED" => row
            .try_get::<u64, _>(idx)
            .map(crate::u64_to_json)
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
                    .map(crate::i64_to_json)
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
