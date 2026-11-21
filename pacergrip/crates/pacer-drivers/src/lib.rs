pub mod mysql;
pub mod postgres;
pub mod sqlite;

use std::sync::Arc;

use pacer_core::{CoreError, CoreResult, DbBackend, DbEngineType, DirectConfig};

pub use mysql::MySqlBackend;
pub use postgres::PostgresBackend;
pub use sqlite::{create_demo_sqlite_db, SqliteBackend};

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
            "Direct native driver for {:?} is not yet bundled (requires external ODBC/OCI runtime); DataGrip XML import/export & AST parsing remain supported.",
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
}
