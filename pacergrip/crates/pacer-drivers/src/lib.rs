pub mod mysql;
pub mod postgres;
pub mod sqlite;

use std::sync::Arc;
use std::time::Duration;

use pacer_core::{
    ConnectionOptionsConfig, CoreError, CoreResult, DbBackend, DbEngineType, DirectConfig,
};

pub use mysql::MySqlBackend;
pub use postgres::PostgresBackend;
pub use sqlite::{create_demo_sqlite_db, SqliteBackend};

pub(crate) fn resolve_pool_limits(
    options: Option<&ConnectionOptionsConfig>,
) -> (u32, Option<Duration>, bool) {
    let Some(opts) = options else {
        return (4, None, false);
    };
    let max_conns = if opts.single_session_mode { 1 } else { 4 };
    let idle_timeout = if opts.auto_disconnect_enabled {
        Some(Duration::from_secs(u64::from(
            opts.auto_disconnect_sec.unwrap_or(300).max(10),
        )))
    } else {
        None
    };
    (max_conns, idle_timeout, opts.keep_alive_enabled)
}

pub(crate) fn build_connection_init_stmts(
    options: Option<&ConnectionOptionsConfig>,
    tz_prefix: Option<&str>,
) -> Vec<String> {
    let mut out = Vec::new();
    let Some(opts) = options else {
        return out;
    };
    if let (Some(prefix), Some(tz)) = (
        tz_prefix,
        opts.time_zone
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty()),
    ) {
        out.push(format!("{prefix}'{}'", tz.replace('\'', "''")));
    }
    if let Some(script) = opts.startup_script.as_deref() {
        for raw in script.split(';') {
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with("--") {
                continue;
            }
            out.push(trimmed.to_string());
        }
    }
    out
}

/// Connects directly to a supported database engine using native Rust async drivers.
pub async fn connect_direct(
    config: &DirectConfig,
    password: Option<&str>,
) -> CoreResult<Arc<dyn DbBackend>> {
    match &config.db_type {
        DbEngineType::Sqlite => Ok(Arc::new(SqliteBackend::connect(config).await?)),
        DbEngineType::MySql
        | DbEngineType::MariaDb
        | DbEngineType::TiDb
        | DbEngineType::OceanBase => {
            Ok(Arc::new(MySqlBackend::connect(config, password).await?))
        }
        DbEngineType::PostgreSql | DbEngineType::OpenGauss | DbEngineType::Kingbase => {
            Ok(Arc::new(PostgresBackend::connect(config, password).await?))
        }
        other => Err(CoreError::Unsupported(format!(
            "Direct native driver for {:?} is not yet bundled (requires external ODBC/OCI runtime); URL / DBeaver config import & AST parsing remain supported.",
            other
        ))),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use tokio::sync::mpsc;

    use pacer_core::QueryReq;

    use super::*;

    fn sqlite_config(path: &std::path::Path, read_only: bool) -> DirectConfig {
        let p = path.to_string_lossy().replace('\\', "/");
        DirectConfig {
            db_type: DbEngineType::Sqlite,
            host: None,
            port: None,
            database: Some(p.clone()),
            jdbc_url: format!("jdbc:sqlite:{p}"),
            driver_ref: Some("sqlite.xerial".to_string()),
            driver_class: None,
            username: None,
            driver_properties: BTreeMap::new(),
            ssh: None,
            ssl: None,
            read_only,
            schema_pattern: None,
            object_filter: None,
            options: None,
            connection_type: Some("default".to_string()),
            auth_type: Some("noAuth".to_string()),
            password_save_mode: Some("session".to_string()),
            socket_path: None,
            has_saved_password: false,
        }
    }

    #[tokio::test]
    async fn test_sqlite_demo_connect_catalog_and_ddl() {
        let tmp = std::env::temp_dir().join(format!("pacergrip_drv_{}.db", std::process::id()));
        let _ = std::fs::remove_file(&tmp);

        create_demo_sqlite_db(&tmp).await.unwrap();
        let backend = connect_direct(&sqlite_config(&tmp, false), None)
            .await
            .unwrap();

        let ping = backend.test_connection().await.unwrap();
        assert!(ping.success);
        assert!(ping.server_version.unwrap().contains("SQLite"));

        let dbs = backend.list_databases().await.unwrap();
        assert!(dbs.iter().any(|d| d.name == "main"));

        let tables = backend.list_tables("main").await.unwrap();
        let table_names: Vec<&str> = tables.iter().map(|t| t.name.as_str()).collect();
        assert!(table_names.contains(&"customers"));
        assert!(table_names.contains(&"orders"));

        let cols = backend.list_columns("main", "customers").await.unwrap();
        assert!(cols.iter().any(|c| c.name == "id" && c.is_primary_key));
        assert!(cols.iter().any(|c| c.name == "email" && !c.nullable));

        let ddl = backend.get_table_ddl("main", "customers").await.unwrap();
        assert!(ddl.contains("CREATE TABLE"));
        assert!(ddl.contains("customers"));

        let indexes = backend.list_indexes("main", "orders").await.unwrap();
        assert!(indexes.iter().any(|i| i.name == "idx_orders_customer"));
        assert!(indexes.iter().any(|i| i.name == "idx_orders_status"));

        drop(backend);
        let _ = std::fs::remove_file(&tmp);
    }

    #[tokio::test]
    async fn test_sqlite_dml_affected_rows_and_read_only_guard() {
        let tmp = std::env::temp_dir().join(format!("pacergrip_dml_{}.db", std::process::id()));
        let _ = std::fs::remove_file(&tmp);
        create_demo_sqlite_db(&tmp).await.unwrap();

        let rw = connect_direct(&sqlite_config(&tmp, false), None)
            .await
            .unwrap();
        let (tx, _rx) = mpsc::channel(8);
        let summary = rw
            .execute_query(
                QueryReq {
                    database: Some("main".to_string()),
                    sql: "UPDATE customers SET tier = 'GOLD' WHERE country = 'CN';".to_string(),
                    limit: None,
                },
                tx,
            )
            .await
            .unwrap();
        assert!(summary.is_dml_or_ddl);
        assert_eq!(summary.affected_rows, Some(3));
        drop(rw);

        let ro = connect_direct(&sqlite_config(&tmp, true), None)
            .await
            .unwrap();
        let (tx2, _rx2) = mpsc::channel(8);
        let err = ro
            .execute_query(
                QueryReq {
                    database: Some("main".to_string()),
                    sql: "DELETE FROM customers WHERE id = 1;".to_string(),
                    limit: None,
                },
                tx2,
            )
            .await
            .unwrap_err();
        assert!(matches!(err, CoreError::PermissionDenied(_)));
        drop(ro);

        let _ = std::fs::remove_file(&tmp);
    }

    #[tokio::test]
    async fn test_sqlite_streaming_over_500_rows_and_mixed_types() {
        let tmp = std::env::temp_dir().join(format!("pacergrip_stream_{}.db", std::process::id()));
        let _ = std::fs::remove_file(&tmp);
        create_demo_sqlite_db(&tmp).await.unwrap();

        let backend = connect_direct(&sqlite_config(&tmp, false), None)
            .await
            .unwrap();

        let (tx_ddl, _rx_ddl) = mpsc::channel(4);
        backend
            .execute_query(
                QueryReq {
                    database: None,
                    sql: "CREATE TABLE items_big (id INTEGER, score REAL, note TEXT, raw_bin BLOB);"
                        .to_string(),
                    limit: None,
                },
                tx_ddl,
            )
            .await
            .unwrap();

        let (tx_ins, _rx_ins) = mpsc::channel(4);
        let insert_sql = "WITH RECURSIVE cnt(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM cnt WHERE x < 650) \
                          INSERT INTO items_big SELECT x, x * 1.5, CASE WHEN x % 2 = 0 THEN NULL ELSE 'row-' || x END, X'CAFE' FROM cnt;";
        let ins_sum = backend
            .execute_query(
                QueryReq {
                    database: None,
                    sql: insert_sql.to_string(),
                    limit: None,
                },
                tx_ins,
            )
            .await
            .unwrap();
        assert_eq!(ins_sum.affected_rows, Some(650));

        let (tx, mut rx) = mpsc::channel(8);
        let sel_sum = backend
            .execute_query(
                QueryReq {
                    database: None,
                    sql: "SELECT id, score, note, raw_bin FROM items_big ORDER BY id;".to_string(),
                    limit: None,
                },
                tx,
            )
            .await
            .unwrap();
        assert_eq!(sel_sum.total_rows, 650);

        let mut chunks = Vec::new();
        while let Some(c) = rx.recv().await {
            chunks.push(c);
        }
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].rows.len(), 500);
        assert!(!chunks[0].is_last);
        assert_eq!(chunks[1].rows.len(), 150);
        assert!(chunks[1].is_last);
        assert_eq!(chunks[0].rows[0]["id"], 1);
        assert_eq!(chunks[0].rows[0]["note"], "row-1");
        assert_eq!(chunks[0].rows[1]["note"], serde_json::Value::Null);
        assert!(chunks[0].rows[0]["raw_bin"]
            .as_str()
            .unwrap()
            .contains("<BLOB"));

        drop(backend);
        let _ = std::fs::remove_file(&tmp);
    }

    #[tokio::test]
    async fn test_sqlite_explain_query_plan() {
        let tmp = std::env::temp_dir().join(format!("pacergrip_exp_{}.db", std::process::id()));
        let _ = std::fs::remove_file(&tmp);
        create_demo_sqlite_db(&tmp).await.unwrap();

        let backend = connect_direct(&sqlite_config(&tmp, false), None)
            .await
            .unwrap();
        let explain = backend
            .explain("main", "SELECT * FROM orders WHERE customer_id = 1;")
            .await
            .unwrap();
        assert!(explain.success);
        assert!(!explain.columns.is_empty());
        assert!(!explain.rows.is_empty());

        drop(backend);
        let _ = std::fs::remove_file(&tmp);
    }

    #[tokio::test]
    async fn test_unsupported_direct_engine_returns_clear_error() {
        let cfg = DirectConfig {
            db_type: DbEngineType::Oracle,
            host: Some("127.0.0.1".to_string()),
            port: Some(1521),
            database: Some("ORCL".to_string()),
            jdbc_url: "jdbc:oracle:thin:@//127.0.0.1:1521/ORCL".to_string(),
            driver_ref: None,
            driver_class: None,
            username: Some("scott".to_string()),
            driver_properties: BTreeMap::new(),
            ssh: None,
            ssl: None,
            read_only: false,
            schema_pattern: None,
            object_filter: None,
            options: None,
            connection_type: Some("default".to_string()),
            auth_type: Some("userPassword".to_string()),
            password_save_mode: Some("session".to_string()),
            socket_path: None,
            has_saved_password: false,
        };
        let err = match connect_direct(&cfg, Some("tiger")).await {
            Ok(_) => panic!("expected Unsupported error for Oracle"),
            Err(e) => e,
        };
        assert!(matches!(err, CoreError::Unsupported(_)));
    }

    #[tokio::test]
    async fn test_sqlite_missing_file_returns_transport_error() {
        let missing = std::env::temp_dir().join("definitely_missing_pacergrip_file.db");
        let _ = std::fs::remove_file(&missing);
        let err = match connect_direct(&sqlite_config(&missing, false), None).await {
            Ok(_) => panic!("expected Transport error for missing SQLite file"),
            Err(e) => e,
        };
        assert!(matches!(err, CoreError::Transport(_)));
    }

    #[tokio::test]
    async fn test_sqlite_empty_table_returns_columns_with_zero_rows() {
        let tmp = std::env::temp_dir().join(format!("pacergrip_empty_{}.db", std::process::id()));
        let _ = std::fs::remove_file(&tmp);
        create_demo_sqlite_db(&tmp).await.unwrap();

        let backend = connect_direct(&sqlite_config(&tmp, false), None)
            .await
            .unwrap();

        let (tx_ddl, _rx_ddl) = mpsc::channel(4);
        backend
            .execute_query(
                QueryReq {
                    database: None,
                    sql: "CREATE TABLE empty_activities (id INTEGER PRIMARY KEY, title TEXT, status TEXT, created_at TEXT);".to_string(),
                    limit: None,
                },
                tx_ddl,
            )
            .await
            .unwrap();

        let (tx, mut rx) = mpsc::channel(8);
        let summary = backend
            .execute_query(
                QueryReq {
                    database: None,
                    sql: "SELECT * FROM empty_activities LIMIT 100;".to_string(),
                    limit: Some(100),
                },
                tx,
            )
            .await
            .unwrap();
        assert!(summary.success);
        assert!(!summary.is_dml_or_ddl);
        assert_eq!(summary.total_rows, 0);

        let mut chunks = Vec::new();
        while let Some(c) = rx.recv().await {
            chunks.push(c);
        }
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].rows.is_empty());
        assert_eq!(
            chunks[0].columns,
            vec!["id", "title", "status", "created_at"]
        );

        let (tx2, mut rx2) = mpsc::channel(8);
        let summary2 = backend
            .execute_query(
                QueryReq {
                    database: None,
                    sql: "SELECT id AS activity_id, title FROM empty_activities WHERE id > 999;"
                        .to_string(),
                    limit: Some(100),
                },
                tx2,
            )
            .await
            .unwrap();
        assert!(summary2.success);
        assert_eq!(summary2.total_rows, 0);
        let chunk2 = rx2.recv().await.expect("expected metadata chunk for zero rows");
        assert!(chunk2.rows.is_empty());
        assert_eq!(chunk2.columns, vec!["activity_id", "title"]);

        drop(backend);
        let _ = std::fs::remove_file(&tmp);
    }

    #[tokio::test]
    async fn test_sqlite_connection_options_startup_script_and_single_session() {
        let tmp = std::env::temp_dir().join(format!("pacergrip_opts_{}.db", std::process::id()));
        let _ = std::fs::remove_file(&tmp);
        create_demo_sqlite_db(&tmp).await.unwrap();

        let mut cfg = sqlite_config(&tmp, false);
        cfg.options = Some(ConnectionOptionsConfig {
            single_session_mode: true,
            auto_disconnect_enabled: true,
            auto_disconnect_sec: Some(120),
            keep_alive_enabled: true,
            keep_alive_sec: Some(60),
            startup_script: Some("PRAGMA foreign_keys = ON;".to_string()),
            ..Default::default()
        });

        let backend = connect_direct(&cfg, None).await.unwrap();
        let (tx, mut rx) = mpsc::channel(4);
        let sum = backend
            .execute_query(
                QueryReq {
                    database: None,
                    sql: "SELECT * FROM pragma_foreign_keys;".to_string(),
                    limit: None,
                },
                tx,
            )
            .await
            .unwrap();
        assert!(sum.success);
        let chunk = rx.recv().await.expect("expected PRAGMA foreign_keys row");
        assert_eq!(chunk.rows[0]["foreign_keys"], serde_json::json!(1));

        drop(backend);
        let _ = std::fs::remove_file(&tmp);
    }
}
