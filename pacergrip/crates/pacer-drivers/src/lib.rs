pub mod mysql;
pub mod postgres;
pub mod redis;
pub mod sqlite;

use std::sync::Arc;
use std::time::Duration;

use pacer_core::{
    ConnectionOptionsConfig, CoreError, CoreResult, DbBackend, DbEngineType, DirectConfig,
};

pub use mysql::MySqlBackend;
pub use postgres::PostgresBackend;
pub use redis::RedisBackend;
pub use sqlite::{create_demo_sqlite_db, SqliteBackend};

pub(crate) const JS_MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;

/// Encodes signed 64-bit integers losslessly for JavaScript / Excel consumers:
/// numbers exceeding `Number.MAX_SAFE_INTEGER` (2^53 - 1, e.g. 19-digit Snowflake IDs)
/// are emitted as exact decimal strings so WebView JSON.parse never rounds trailing digits.
pub(crate) fn i64_to_json(v: i64) -> serde_json::Value {
    if v.abs() > JS_MAX_SAFE_INTEGER {
        serde_json::Value::String(v.to_string())
    } else {
        serde_json::Value::from(v)
    }
}

/// Encodes unsigned 64-bit integers losslessly for JavaScript / Excel consumers.
pub(crate) fn u64_to_json(v: u64) -> serde_json::Value {
    if v > JS_MAX_SAFE_INTEGER as u64 {
        serde_json::Value::String(v.to_string())
    } else {
        serde_json::Value::from(v)
    }
}

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
        DbEngineType::Redis => Ok(Arc::new(RedisBackend::connect(config, password).await?)),
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

    async fn read_mock_resp_cmd<R: tokio::io::AsyncBufRead + Unpin>(
        reader: &mut R,
    ) -> std::io::Result<Option<Vec<String>>> {
        use tokio::io::{AsyncBufReadExt, AsyncReadExt};
        let mut header = String::new();
        let n = reader.read_line(&mut header).await?;
        if n == 0 {
            return Ok(None);
        }
        let trimmed = header.trim();
        let count: usize = trimmed
            .strip_prefix('*')
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);
        let mut args = Vec::with_capacity(count);
        for _ in 0..count {
            let mut len_line = String::new();
            reader.read_line(&mut len_line).await?;
            let byte_len: usize = len_line
                .trim()
                .strip_prefix('$')
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0);
            let mut buf = vec![0u8; byte_len + 2];
            reader.read_exact(&mut buf).await?;
            buf.truncate(byte_len);
            args.push(String::from_utf8_lossy(&buf).to_string());
        }
        Ok(Some(args))
    }

    fn resp_bulk(s: &str) -> Vec<u8> {
        format!("${}\r\n{}\r\n", s.len(), s).into_bytes()
    }

    fn resp_bulk_array(items: &[&str]) -> Vec<u8> {
        let mut out = format!("*{}\r\n", items.len()).into_bytes();
        for item in items {
            out.extend_from_slice(&resp_bulk(item));
        }
        out
    }

    #[tokio::test]
    async fn test_redis_connect_scan_get_hgetall_and_read_only_guard() {
        use tokio::io::{AsyncWriteExt, BufReader};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_task = tokio::spawn(async move {
            loop {
                let Ok((socket, _)) = listener.accept().await else {
                    break;
                };
                tokio::spawn(async move {
                    let (r, mut w) = socket.into_split();
                    let mut reader = BufReader::new(r);
                    while let Ok(Some(args)) = read_mock_resp_cmd(&mut reader).await {
                        if args.is_empty() {
                            continue;
                        }
                        let cmd = args[0].to_ascii_uppercase();
                        let reply: Vec<u8> = match cmd.as_str() {
                            "AUTH" => {
                                if args.last().map(String::as_str) == Some("secret_pass") {
                                    b"+OK\r\n".to_vec()
                                } else {
                                    b"-WRONGPASS invalid username-password pair\r\n".to_vec()
                                }
                            }
                            "PING" => b"+PONG\r\n".to_vec(),
                            "SELECT" => b"+OK\r\n".to_vec(),
                            "INFO" => {
                                let sec = args.get(1).map(|s| s.to_ascii_lowercase());
                                match sec.as_deref() {
                                    Some("server") => resp_bulk(
                                        "# Server\r\nredis_version:7.2.4\r\nredis_mode:standalone\r\n",
                                    ),
                                    Some("keyspace") => resp_bulk(
                                        "# Keyspace\r\ndb0:keys=2,expires=1,avg_ttl=60000\r\n",
                                    ),
                                    _ => resp_bulk(
                                        "# Server\r\nredis_version:7.2.4\r\n# Keyspace\r\ndb0:keys=2,expires=1,avg_ttl=60000\r\n",
                                    ),
                                }
                            }
                            "CONFIG" => resp_bulk_array(&["databases", "16"]),
                            "SCAN" => {
                                let mut out = b"*2\r\n".to_vec();
                                out.extend_from_slice(&resp_bulk("0"));
                                out.extend_from_slice(&resp_bulk_array(&["user:1001", "rank:scores"]));
                                out
                            }
                            "TYPE" => match args.get(1).map(String::as_str) {
                                Some("user:1001") => b"+hash\r\n".to_vec(),
                                Some("rank:scores") => b"+zset\r\n".to_vec(),
                                _ => b"+string\r\n".to_vec(),
                            },
                            "TTL" => match args.get(1).map(String::as_str) {
                                Some("user:1001") => b":3600\r\n".to_vec(),
                                _ => b":-1\r\n".to_vec(),
                            },
                            "OBJECT" => b"+listpack\r\n".to_vec(),
                            "MEMORY" => b":128\r\n".to_vec(),
                            "HLEN" | "ZCARD" => b":2\r\n".to_vec(),
                            "HSCAN" => {
                                let mut out = b"*2\r\n".to_vec();
                                out.extend_from_slice(&resp_bulk("0"));
                                out.extend_from_slice(&resp_bulk_array(&["name", "Alice", "role", "admin"]));
                                out
                            }
                            "GET" => resp_bulk("hello_redis"),
                            "HGETALL" => resp_bulk_array(&["name", "Alice", "role", "admin"]),
                            "ZRANGE" => resp_bulk_array(&["pacer", "99.5", "redis", "88"]),
                            "SET" => b"+OK\r\n".to_vec(),
                            _ => b"-ERR unknown command\r\n".to_vec(),
                        };
                        if w.write_all(&reply).await.is_err() {
                            break;
                        }
                    }
                });
            }
        });

        let make_cfg = |read_only: bool| DirectConfig {
            db_type: DbEngineType::Redis,
            host: Some("127.0.0.1".to_string()),
            port: Some(addr.port()),
            database: Some("0".to_string()),
            jdbc_url: format!("redis://127.0.0.1:{}/0", addr.port()),
            driver_ref: None,
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
            auth_type: Some("userPassword".to_string()),
            password_save_mode: Some("session".to_string()),
            socket_path: None,
            has_saved_password: false,
        };

        let backend = connect_direct(&make_cfg(false), Some("secret_pass"))
            .await
            .expect("Redis backend should connect with password");

        let ping = backend.test_connection().await.unwrap();
        assert!(ping.success);
        assert_eq!(ping.server_version.as_deref(), Some("Redis 7.2.4 (standalone)"));

        let dbs = backend.list_databases().await.unwrap();
        assert_eq!(dbs.len(), 16);
        assert_eq!(dbs[0].name, "db0");
        assert_eq!(dbs[0].table_count, Some(2));

        let tables = backend.list_tables("db0").await.unwrap();
        assert_eq!(tables.len(), 2);
        assert_eq!(tables[0].name, "rank:scores");
        assert_eq!(tables[0].table_type.as_deref(), Some("zset"));
        assert_eq!(tables[0].engine.as_deref(), Some("永久"));
        assert_eq!(tables[1].name, "user:1001");
        assert_eq!(tables[1].table_type.as_deref(), Some("hash"));

        let ddl = backend.get_table_ddl("db0", "user:1001").await.unwrap();
        assert!(ddl.contains("HGETALL user:1001"));

        let cols = backend.list_columns("db0", "user:1001").await.unwrap();
        assert!(cols.iter().any(|c| c.name == "name" && c.default_value.as_deref() == Some("Alice")));

        let (tx_h, mut rx_h) = mpsc::channel(4);
        let sum_h = backend
            .execute_query(
                QueryReq {
                    database: Some("db0".to_string()),
                    sql: "HGETALL user:1001".to_string(),
                    limit: Some(100),
                },
                tx_h,
            )
            .await
            .unwrap();
        assert_eq!(sum_h.total_rows, 2);
        let chunk_h = rx_h.recv().await.unwrap();
        assert_eq!(chunk_h.columns, vec!["field", "value"]);
        assert_eq!(chunk_h.rows[0]["field"], serde_json::json!("name"));
        assert_eq!(chunk_h.rows[0]["value"], serde_json::json!("Alice"));

        let (tx_z, mut rx_z) = mpsc::channel(4);
        let sum_z = backend
            .execute_query(
                QueryReq {
                    database: Some("db0".to_string()),
                    sql: "ZRANGE rank:scores 0 99 WITHSCORES".to_string(),
                    limit: Some(100),
                },
                tx_z,
            )
            .await
            .unwrap();
        assert_eq!(sum_z.total_rows, 2);
        let chunk_z = rx_z.recv().await.unwrap();
        assert_eq!(chunk_z.columns, vec!["member", "score"]);
        assert_eq!(chunk_z.rows[0]["member"], serde_json::json!("pacer"));
        assert_eq!(chunk_z.rows[0]["score"], serde_json::json!("99.5"));

        let explain = backend.explain("db0", "HGETALL user:1001").await.unwrap();
        assert!(explain.success);
        assert!(explain.rows.iter().any(|r| r["property"] == "memory_usage" && r["value"] == "128 bytes"));

        // Verify read-only connection permits GET/INFO but blocks SET and FLUSHALL
        let ro_backend = connect_direct(&make_cfg(true), Some("secret_pass"))
            .await
            .unwrap();
        let (tx_ro_ok, _) = mpsc::channel(4);
        assert!(ro_backend
            .execute_query(
                QueryReq {
                    database: Some("db0".to_string()),
                    sql: "GET greeting".to_string(),
                    limit: None,
                },
                tx_ro_ok,
            )
            .await
            .is_ok());

        let (tx_ro_err, _) = mpsc::channel(4);
        let err = ro_backend
            .execute_query(
                QueryReq {
                    database: Some("db0".to_string()),
                    sql: "SET greeting blocked".to_string(),
                    limit: None,
                },
                tx_ro_err,
            )
            .await
            .unwrap_err();
        assert!(matches!(err, CoreError::PermissionDenied(_)));

        server_task.abort();
    }

    #[test]
    fn test_long_integer_json_precision_preservation() {
        assert_eq!(i64_to_json(42), serde_json::json!(42));
        assert_eq!(i64_to_json(-9_007_199_254_740_991), serde_json::json!(-9_007_199_254_740_991i64));
        assert_eq!(
            i64_to_json(1_892_345_678_901_234_567),
            serde_json::json!("1892345678901234567")
        );
        assert_eq!(
            i64_to_json(-1_892_345_678_901_234_567),
            serde_json::json!("-1892345678901234567")
        );
        assert_eq!(
            u64_to_json(18_446_744_073_709_551_615),
            serde_json::json!("18446744073709551615")
        );
    }
}
