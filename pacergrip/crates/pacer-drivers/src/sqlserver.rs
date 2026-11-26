use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Instant;

use async_trait::async_trait;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use futures_util::TryStreamExt;
use serde_json::Value;
use tiberius::{AuthMethod, Client, ColumnData, Config, EncryptionLevel, QueryItem};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex};
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

use pacer_core::{
    CapabilityFlags, ColumnMeta, CoreError, CoreResult, DatabaseMeta, DbBackend, DbEngineType,
    DirectConfig, ExplainResult, ForeignKeyMeta, IndexMeta, ObjectSearchHit, PingMetrics,
    QueryChunk, QueryReq, QuerySummary, RoutineMeta, TableMeta, TransactionStatus,
};
use pacer_sql_ast::{analyze_sql, SqlAccessKind};

use crate::sqlite::adaptive_chunk_size;
use crate::ssh_tunnel::{open_ssh_tunnel, SshTunnelHandle};
use crate::{classify_tx_statement, i64_to_json, TxStatementKind};

#[derive(Clone)]
struct SqlServerEndpoint {
    connect_host: String,
    connect_port: u16,
    username: String,
    password: String,
    default_db: Option<String>,
    ssl_enabled: bool,
    trust_cert: bool,
    ca_cert: Option<String>,
}

#[derive(Default)]
struct SqlServerTxState {
    manual_mode: bool,
    in_transaction: bool,
    pending_statements: u32,
}

/// Native async SQL Server (MSSQL) backend powered directly by `tiberius` over TDS 7.3+.
pub struct SqlServerBackend {
    endpoint: SqlServerEndpoint,
    client: Arc<Mutex<Client<Compat<TcpStream>>>>,
    session_spid: Option<i32>,
    read_only: bool,
    tx_state: StdMutex<SqlServerTxState>,
    running_spids: StdMutex<HashMap<String, i32>>,
    _ssh_tunnel: Option<SshTunnelHandle>,
}

struct MssqlRunningGuard<'a> {
    map: &'a StdMutex<HashMap<String, i32>>,
    key: Option<String>,
}

impl Drop for MssqlRunningGuard<'_> {
    fn drop(&mut self) {
        if let Some(ref k) = self.key {
            if let Ok(mut g) = self.map.lock() {
                g.remove(k);
            }
        }
    }
}

async fn connect_tiberius(
    ep: &SqlServerEndpoint,
    db_override: Option<&str>,
) -> CoreResult<Client<Compat<TcpStream>>> {
    let mut cfg = Config::new();
    cfg.host(&ep.connect_host);
    cfg.port(ep.connect_port);
    cfg.authentication(AuthMethod::sql_server(&ep.username, &ep.password));
    if let Some(db) = db_override
        .or(ep.default_db.as_deref())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        cfg.database(db);
    }
    if ep.ssl_enabled {
        cfg.encryption(EncryptionLevel::Required);
        if ep.trust_cert {
            cfg.trust_cert();
        } else if let Some(ref ca) = ep.ca_cert {
            cfg.trust_cert_ca(ca);
        }
    } else {
        cfg.encryption(EncryptionLevel::Off);
        cfg.trust_cert();
    }

    let addr = cfg.get_addr();
    let tcp = tokio::time::timeout(
        std::time::Duration::from_secs(8),
        TcpStream::connect(&addr),
    )
    .await
    .map_err(|_| CoreError::Transport(format!("SQL Server 连接超时 ({addr})")))?
    .map_err(|e| CoreError::Transport(format!("SQL Server TCP 连接失败 ({addr}): {e}")))?;
    let _ = tcp.set_nodelay(true);

    Client::connect(cfg, tcp.compat_write())
        .await
        .map_err(|e| CoreError::Transport(format!("SQL Server TDS 握手/认证失败: {e}")))
}

impl SqlServerBackend {
    pub async fn connect(config: &DirectConfig, password: Option<&str>) -> CoreResult<Self> {
        let host = config
            .host
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("localhost");
        let port = config.port.unwrap_or(1433);

        let ssh_tunnel = match config.ssh.as_ref().filter(|s| s.enabled) {
            Some(ssh_cfg) => Some(open_ssh_tunnel(ssh_cfg, host, port).await?),
            None => None,
        };
        let (connect_host, connect_port) = match &ssh_tunnel {
            Some(t) => ("127.0.0.1".to_string(), t.local_port()),
            None => (host.to_string(), port),
        };

        let ssl_enabled = config.ssl.as_ref().is_some_and(|s| s.enabled);
        let trust_cert = should_trust_sqlserver_cert(config);
        let ca_cert = config
            .ssl
            .as_ref()
            .and_then(|s| s.ca_cert.as_deref())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string);

        let endpoint = SqlServerEndpoint {
            connect_host,
            connect_port,
            username: config
                .username
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or("sa")
                .to_string(),
            password: password.unwrap_or_default().to_string(),
            default_db: config
                .database
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string),
            ssl_enabled,
            trust_cert,
            ca_cert,
        };

        let mut client = connect_tiberius(&endpoint, None).await?;
        let session_spid = fetch_scalar_i32(&mut client, "SELECT CAST(@@SPID AS INT) AS spid").await;

        let backend = Self {
            endpoint,
            client: Arc::new(Mutex::new(client)),
            session_spid,
            read_only: config.read_only,
            tx_state: StdMutex::new(SqlServerTxState::default()),
            running_spids: StdMutex::new(HashMap::new()),
            _ssh_tunnel: ssh_tunnel,
        };
        Ok(backend)
    }

    async fn query_all_simple(&self, sql: &str) -> CoreResult<(Vec<String>, Vec<BTreeMap<String, Value>>)> {
        let mut guard = self.client.lock().await;
        exec_tiberius_collect(&mut guard, sql, None).await
    }
}

async fn fetch_scalar_i32(client: &mut Client<Compat<TcpStream>>, sql: &str) -> Option<i32> {
    let (_, rows) = exec_tiberius_collect(client, sql, Some(1)).await.ok()?;
    rows.first()
        .and_then(|r| r.values().next())
        .and_then(|v| v.as_i64())
        .and_then(|n| i32::try_from(n).ok())
}

async fn exec_tiberius_collect(
    client: &mut Client<Compat<TcpStream>>,
    sql: &str,
    limit: Option<usize>,
) -> CoreResult<(Vec<String>, Vec<BTreeMap<String, Value>>)> {
    let mut stream = client
        .query(sql, &[])
        .await
        .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;
    let mut columns = Vec::new();
    let mut rows = Vec::new();
    while let Some(item) = stream
        .try_next()
        .await
        .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?
    {
        match item {
            QueryItem::Metadata(meta) => {
                if columns.is_empty() {
                    columns = meta.columns().iter().map(|c| c.name().to_string()).collect();
                }
            }
            QueryItem::Row(row) => {
                if columns.is_empty() {
                    columns = row.columns().iter().map(|c| c.name().to_string()).collect();
                }
                if limit.is_some_and(|l| rows.len() >= l) {
                    continue;
                }
                rows.push(tiberius_row_to_map(&row, &columns));
            }
        }
    }
    Ok((columns, rows))
}

fn tiberius_row_to_map(row: &tiberius::Row, cols: &[String]) -> BTreeMap<String, Value> {
    let mut map = BTreeMap::new();
    for (idx, col_name) in cols.iter().enumerate() {
        let val = match row.cells().nth(idx) {
            Some((_, col_data)) => match col_data {
                ColumnData::DateTime(_)
                | ColumnData::SmallDateTime(_)
                | ColumnData::DateTime2(_) => row
                    .try_get::<NaiveDateTime, _>(idx)
                    .ok()
                    .flatten()
                    .map(|dt| Value::String(dt.format("%Y-%m-%d %H:%M:%S").to_string()))
                    .unwrap_or_else(|| Value::String(format!("{col_data:?}"))),
                ColumnData::DateTimeOffset(_) => row
                    .try_get::<chrono::DateTime<chrono::FixedOffset>, _>(idx)
                    .ok()
                    .flatten()
                    .map(|dt| Value::String(dt.to_rfc3339()))
                    .unwrap_or_else(|| Value::String(format!("{col_data:?}"))),
                ColumnData::Date(_) => row
                    .try_get::<NaiveDate, _>(idx)
                    .ok()
                    .flatten()
                    .map(|d| Value::String(d.format("%Y-%m-%d").to_string()))
                    .unwrap_or(Value::Null),
                ColumnData::Time(_) => row
                    .try_get::<NaiveTime, _>(idx)
                    .ok()
                    .flatten()
                    .map(|t| Value::String(t.format("%H:%M:%S").to_string()))
                    .unwrap_or(Value::Null),
                other => column_data_to_json(other),
            },
            None => Value::Null,
        };
        map.insert(col_name.clone(), val);
    }
    map
}

fn column_data_to_json(col: &ColumnData<'_>) -> Value {
    match col {
        ColumnData::U8(Some(v)) => Value::from(*v),
        ColumnData::I16(Some(v)) => Value::from(*v),
        ColumnData::I32(Some(v)) => Value::from(*v),
        ColumnData::I64(Some(v)) => i64_to_json(*v),
        ColumnData::F32(Some(v)) => serde_json::Number::from_f64(f64::from(*v))
            .map(Value::Number)
            .unwrap_or(Value::Null),
        ColumnData::F64(Some(v)) => serde_json::Number::from_f64(*v)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        ColumnData::Bit(Some(v)) => Value::from(*v),
        ColumnData::String(Some(s)) => Value::String(s.to_string()),
        ColumnData::Guid(Some(u)) => Value::String(u.to_string()),
        ColumnData::Numeric(Some(n)) => Value::String(n.to_string()),
        ColumnData::Xml(Some(x)) => Value::String(x.to_string()),
        ColumnData::Binary(Some(b)) => {
            if b.is_empty() {
                Value::String(String::new())
            } else {
                let mut hex = String::with_capacity(2 + b.len() * 2);
                hex.push_str("0x");
                for byte in b.iter().take(256) {
                    use std::fmt::Write as _;
                    let _ = write!(hex, "{byte:02X}");
                }
                if b.len() > 256 {
                    hex.push_str("...");
                }
                Value::String(hex)
            }
        }
        _ => Value::Null,
    }
}

fn quote_tsql_ident(name: &str) -> String {
    format!("[{}]", name.replace(']', "]]"))
}

fn escape_tsql_str(s: &str) -> String {
    s.replace('\'', "''")
}

fn split_schema_table(_db: &str, raw_table: &str) -> (String, String) {
    if let Some((sch, tbl)) = raw_table.split_once('.') {
        (sch.trim().to_string(), tbl.trim().to_string())
    } else {
        ("dbo".to_string(), raw_table.trim().to_string())
    }
}

#[async_trait]
impl DbBackend for SqlServerBackend {
    async fn test_connection(&self) -> CoreResult<PingMetrics> {
        let started = Instant::now();
        let (_, rows) = self.query_all_simple("SELECT @@VERSION AS ver").await?;
        let ver = rows
            .first()
            .and_then(|r| r.get("ver"))
            .and_then(|v| v.as_str())
            .and_then(|s| s.lines().next())
            .unwrap_or("SQL Server")
            .to_string();
        Ok(PingMetrics {
            success: true,
            latency_ms: started.elapsed().as_millis() as u64,
            server_version: Some(ver.clone()),
            message: format!("Connected to {ver}"),
        })
    }

    async fn list_databases(&self) -> CoreResult<Vec<DatabaseMeta>> {
        let (_, rows) = self
            .query_all_simple("SELECT name, collation_name FROM sys.databases ORDER BY name")
            .await?;
        Ok(rows
            .into_iter()
            .map(|r| {
                let name = r
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let is_system = matches!(
                    name.as_str(),
                    "master" | "tempdb" | "model" | "msdb"
                );
                DatabaseMeta {
                    name,
                    charset: Some("UTF-8".to_string()),
                    collation: r
                        .get("collation_name")
                        .and_then(|v| v.as_str())
                        .map(str::to_string),
                    table_count: None,
                    data_size_mb: None,
                    is_system,
                    comment: None,
                }
            })
            .collect())
    }

    async fn list_tables(&self, db: &str) -> CoreResult<Vec<TableMeta>> {
        let use_prefix = if db.trim().is_empty() {
            String::new()
        } else {
            format!("USE {}; ", quote_tsql_ident(db.trim()))
        };
        let sql = format!(
            "{use_prefix}\
             SELECT t.TABLE_SCHEMA, t.TABLE_NAME, t.TABLE_TYPE, \
                    CAST(ISNULL(SUM(p.rows), 0) AS BIGINT) AS row_est \
             FROM INFORMATION_SCHEMA.TABLES t \
             LEFT JOIN sys.objects o ON o.name = t.TABLE_NAME AND SCHEMA_NAME(o.schema_id) = t.TABLE_SCHEMA \
             LEFT JOIN sys.partitions p ON p.object_id = o.object_id AND p.index_id IN (0, 1) \
             GROUP BY t.TABLE_SCHEMA, t.TABLE_NAME, t.TABLE_TYPE \
             ORDER BY t.TABLE_SCHEMA, t.TABLE_NAME"
        );
        let (_, rows) = self.query_all_simple(&sql).await?;
        Ok(rows
            .into_iter()
            .map(|r| {
                let sch = r.get("TABLE_SCHEMA").and_then(|v| v.as_str()).unwrap_or("dbo");
                let tbl = r.get("TABLE_NAME").and_then(|v| v.as_str()).unwrap_or_default();
                let display_name = if sch.eq_ignore_ascii_case("dbo") {
                    tbl.to_string()
                } else {
                    format!("{sch}.{tbl}")
                };
                let ttype = r
                    .get("TABLE_TYPE")
                    .and_then(|v| v.as_str())
                    .map(|s| if s.contains("VIEW") { "VIEW" } else { "TABLE" }.to_string());
                let estimated_rows = r.get("row_est").and_then(|v| v.as_i64()).and_then(|n| u64::try_from(n).ok());
                TableMeta {
                    name: display_name,
                    table_type: ttype,
                    engine: Some("SQL Server".to_string()),
                    estimated_rows,
                    data_size_kb: None,
                    comment: None,
                }
            })
            .collect())
    }

    async fn list_columns(&self, db: &str, table: &str) -> CoreResult<Vec<ColumnMeta>> {
        let (sch, tbl) = split_schema_table(db, table);
        let use_prefix = if db.trim().is_empty() {
            String::new()
        } else {
            format!("USE {}; ", quote_tsql_ident(db.trim()))
        };
        let sql = format!(
            "{use_prefix}\
             SELECT c.COLUMN_NAME, c.DATA_TYPE, c.IS_NULLABLE, c.COLUMN_DEFAULT, \
                    CASE WHEN EXISTS ( \
                        SELECT 1 FROM INFORMATION_SCHEMA.TABLE_CONSTRAINTS tc \
                        JOIN INFORMATION_SCHEMA.KEY_COLUMN_USAGE kcu \
                          ON tc.CONSTRAINT_NAME = kcu.CONSTRAINT_NAME AND tc.TABLE_SCHEMA = kcu.TABLE_SCHEMA \
                        WHERE tc.CONSTRAINT_TYPE = 'PRIMARY KEY' \
                          AND tc.TABLE_SCHEMA = c.TABLE_SCHEMA \
                          AND tc.TABLE_NAME = c.TABLE_NAME \
                          AND kcu.COLUMN_NAME = c.COLUMN_NAME \
                    ) THEN 1 ELSE 0 END AS is_pk \
             FROM INFORMATION_SCHEMA.COLUMNS c \
             WHERE c.TABLE_SCHEMA = '{}' AND c.TABLE_NAME = '{}' \
             ORDER BY c.ORDINAL_POSITION",
            escape_tsql_str(&sch),
            escape_tsql_str(&tbl)
        );
        let (_, rows) = self.query_all_simple(&sql).await?;
        Ok(rows
            .into_iter()
            .map(|r| ColumnMeta {
                name: r
                    .get("COLUMN_NAME")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                data_type: r
                    .get("DATA_TYPE")
                    .and_then(|v| v.as_str())
                    .unwrap_or("nvarchar")
                    .to_string(),
                nullable: r
                    .get("IS_NULLABLE")
                    .and_then(|v| v.as_str())
                    .is_some_and(|s| s.eq_ignore_ascii_case("YES")),
                is_primary_key: r
                    .get("is_pk")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0)
                    > 0,
                default_value: r
                    .get("COLUMN_DEFAULT")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                comment: None,
                masking_rule: None,
            })
            .collect())
    }

    async fn list_indexes(&self, db: &str, table: &str) -> CoreResult<Vec<IndexMeta>> {
        let (sch, tbl) = split_schema_table(db, table);
        let use_prefix = if db.trim().is_empty() {
            String::new()
        } else {
            format!("USE {}; ", quote_tsql_ident(db.trim()))
        };
        let sql = format!(
            "{use_prefix}\
             SELECT i.name AS idx_name, i.is_unique, i.type_desc, c.name AS col_name \
             FROM sys.indexes i \
             JOIN sys.index_columns ic ON i.object_id = ic.object_id AND i.index_id = ic.index_id \
             JOIN sys.columns c ON ic.object_id = c.object_id AND ic.column_id = c.column_id \
             JOIN sys.objects o ON i.object_id = o.object_id \
             WHERE SCHEMA_NAME(o.schema_id) = '{}' AND o.name = '{}' AND i.name IS NOT NULL AND ic.is_included_column = 0 \
             ORDER BY i.name, ic.key_ordinal",
            escape_tsql_str(&sch),
            escape_tsql_str(&tbl)
        );
        let (_, rows) = self.query_all_simple(&sql).await?;
        let mut grouped: BTreeMap<String, IndexMeta> = BTreeMap::new();
        for r in rows {
            let name = r.get("idx_name").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            if name.is_empty() {
                continue;
            }
            let entry = grouped.entry(name.clone()).or_insert_with(|| IndexMeta {
                name,
                unique: r.get("is_unique").and_then(|v| v.as_bool()).unwrap_or(false),
                columns: Vec::new(),
                index_type: r.get("type_desc").and_then(|v| v.as_str()).map(str::to_string),
            });
            if let Some(col) = r.get("col_name").and_then(|v| v.as_str()) {
                entry.columns.push(col.to_string());
            }
        }
        Ok(grouped.into_values().collect())
    }

    async fn list_foreign_keys(&self, db: &str, table: &str) -> CoreResult<Vec<ForeignKeyMeta>> {
        let (sch, tbl) = split_schema_table(db, table);
        let use_prefix = if db.trim().is_empty() {
            String::new()
        } else {
            format!("USE {}; ", quote_tsql_ident(db.trim()))
        };
        let sql = format!(
            "{use_prefix}\
             SELECT fk.name AS fk_name, pc.name AS col_name, ro.name AS ref_table, rc.name AS ref_col, \
                    fk.delete_referential_action_desc AS on_delete, fk.update_referential_action_desc AS on_update \
             FROM sys.foreign_keys fk \
             JOIN sys.foreign_key_columns fkc ON fk.object_id = fkc.constraint_object_id \
             JOIN sys.objects po ON po.object_id = fkc.parent_object_id \
             JOIN sys.columns pc ON pc.object_id = fkc.parent_object_id AND pc.column_id = fkc.parent_column_id \
             JOIN sys.objects ro ON ro.object_id = fkc.referenced_object_id \
             JOIN sys.columns rc ON rc.object_id = fkc.referenced_object_id AND rc.column_id = fkc.referenced_column_id \
             WHERE SCHEMA_NAME(po.schema_id) = '{}' AND po.name = '{}' \
             ORDER BY fk.name",
            escape_tsql_str(&sch),
            escape_tsql_str(&tbl)
        );
        let (_, rows) = self.query_all_simple(&sql).await?;
        Ok(rows
            .into_iter()
            .map(|r| ForeignKeyMeta {
                name: r.get("fk_name").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                column_name: r.get("col_name").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                referenced_table: r.get("ref_table").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                referenced_column: r.get("ref_col").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                on_delete: r.get("on_delete").and_then(|v| v.as_str()).map(str::to_string),
                on_update: r.get("on_update").and_then(|v| v.as_str()).map(str::to_string),
            })
            .collect())
    }

    async fn list_routines(&self, db: &str) -> CoreResult<Vec<RoutineMeta>> {
        let use_prefix = if db.trim().is_empty() {
            String::new()
        } else {
            format!("USE {}; ", quote_tsql_ident(db.trim()))
        };
        let sql = format!(
            "{use_prefix}\
             SELECT ROUTINE_NAME, ROUTINE_TYPE, DATA_TYPE \
             FROM INFORMATION_SCHEMA.ROUTINES ORDER BY ROUTINE_NAME"
        );
        let Ok((_, rows)) = self.query_all_simple(&sql).await else {
            return Ok(Vec::new());
        };
        Ok(rows
            .into_iter()
            .map(|r| RoutineMeta {
                name: r.get("ROUTINE_NAME").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                kind: r.get("ROUTINE_TYPE").and_then(|v| v.as_str()).unwrap_or("PROCEDURE").to_string(),
                return_type: r.get("DATA_TYPE").and_then(|v| v.as_str()).map(str::to_string),
                definition: None,
                comment: None,
            })
            .collect())
    }

    async fn search_objects(&self, db: &str, query: &str) -> CoreResult<Vec<ObjectSearchHit>> {
        let needle = query.trim().to_ascii_lowercase();
        if needle.is_empty() {
            return Ok(Vec::new());
        }
        let use_prefix = if db.trim().is_empty() {
            String::new()
        } else {
            format!("USE {}; ", quote_tsql_ident(db.trim()))
        };
        let like_pat = escape_tsql_str(&format!("%{needle}%"));
        let mut hits = Vec::new();
        let mut seen = HashSet::new();

        let sql_tbl = format!(
            "{use_prefix}SELECT TOP 150 TABLE_SCHEMA, TABLE_NAME, TABLE_TYPE FROM INFORMATION_SCHEMA.TABLES \
             WHERE LOWER(TABLE_NAME) LIKE '{like_pat}' ORDER BY TABLE_NAME"
        );
        if let Ok((_, rows)) = self.query_all_simple(&sql_tbl).await {
            for r in rows {
                let sch = r.get("TABLE_SCHEMA").and_then(|v| v.as_str()).unwrap_or("dbo");
                let tbl = r.get("TABLE_NAME").and_then(|v| v.as_str()).unwrap_or_default();
                let name = if sch.eq_ignore_ascii_case("dbo") {
                    tbl.to_string()
                } else {
                    format!("{sch}.{tbl}")
                };
                if seen.insert(format!("t:{name}")) {
                    let ttype = r.get("TABLE_TYPE").and_then(|v| v.as_str()).unwrap_or("TABLE");
                    hits.push(ObjectSearchHit {
                        kind: if ttype.contains("VIEW") { "view" } else { "table" }.to_string(),
                        database: db.to_string(),
                        table_name: name,
                        column_name: None,
                        data_type: Some(ttype.to_string()),
                        comment: None,
                    });
                }
            }
        }
        if hits.len() < 150 {
            let rem = 150 - hits.len();
            let sql_col = format!(
                "{use_prefix}SELECT TOP {rem} TABLE_SCHEMA, TABLE_NAME, COLUMN_NAME, DATA_TYPE \
                 FROM INFORMATION_SCHEMA.COLUMNS WHERE LOWER(COLUMN_NAME) LIKE '{like_pat}' ORDER BY TABLE_NAME, ORDINAL_POSITION"
            );
            if let Ok((_, rows)) = self.query_all_simple(&sql_col).await {
                for r in rows {
                    let sch = r.get("TABLE_SCHEMA").and_then(|v| v.as_str()).unwrap_or("dbo");
                    let tbl = r.get("TABLE_NAME").and_then(|v| v.as_str()).unwrap_or_default();
                    let col = r.get("COLUMN_NAME").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                    let tname = if sch.eq_ignore_ascii_case("dbo") {
                        tbl.to_string()
                    } else {
                        format!("{sch}.{tbl}")
                    };
                    if seen.insert(format!("c:{tname}.{col}")) {
                        hits.push(ObjectSearchHit {
                            kind: "column".to_string(),
                            database: db.to_string(),
                            table_name: tname,
                            column_name: Some(col),
                            data_type: r.get("DATA_TYPE").and_then(|v| v.as_str()).map(str::to_string),
                            comment: None,
                        });
                    }
                }
            }
        }
        Ok(hits)
    }

    async fn get_table_ddl(&self, db: &str, table: &str) -> CoreResult<String> {
        let (sch, tbl) = split_schema_table(db, table);
        let cols = self.list_columns(db, table).await?;
        if cols.is_empty() {
            return Err(CoreError::ExecutionFailed(format!(
                "Table {sch}.{tbl} not found"
            )));
        }
        let mut lines = Vec::new();
        let mut pks = Vec::new();
        for c in &cols {
            let mut line = format!("  {} {}", quote_tsql_ident(&c.name), c.data_type);
            if !c.nullable {
                line.push_str(" NOT NULL");
            }
            if let Some(ref dflt) = c.default_value {
                line.push_str(&format!(" DEFAULT {dflt}"));
            }
            lines.push(line);
            if c.is_primary_key {
                pks.push(quote_tsql_ident(&c.name));
            }
        }
        if !pks.is_empty() {
            lines.push(format!("  PRIMARY KEY ({})", pks.join(", ")));
        }
        Ok(format!(
            "CREATE TABLE {}.{} (\n{}\n);",
            quote_tsql_ident(&sch),
            quote_tsql_ident(&tbl),
            lines.join(",\n")
        ))
    }

    async fn execute_query(
        &self,
        req: QueryReq,
        tx: mpsc::Sender<QueryChunk>,
    ) -> CoreResult<QuerySummary> {
        let started = Instant::now();
        let analysis = analyze_sql(&req.sql, &DbEngineType::SqlServer);
        let is_read_only = matches!(analysis.overall_kind, SqlAccessKind::ReadOnly);
        if self.read_only && !is_read_only {
            return Err(CoreError::PermissionDenied(
                "Connection is marked read-only; DML/DDL blocked".to_string(),
            ));
        }

        let _guard = if let (Some(ref qid), Some(spid)) = (&req.query_id, self.session_spid) {
            if let Ok(mut g) = self.running_spids.lock() {
                g.insert(qid.clone(), spid);
            }
            Some(MssqlRunningGuard {
                map: &self.running_spids,
                key: Some(qid.clone()),
            })
        } else {
            None
        };

        let db_label = req
            .database
            .clone()
            .filter(|s| !s.trim().is_empty())
            .or_else(|| self.endpoint.default_db.clone())
            .unwrap_or_else(|| "master".to_string());

        let mut guard = self.client.lock().await;
        if let Some(ref db) = req.database {
            if !db.trim().is_empty() {
                let use_sql = format!("USE {};", quote_tsql_ident(db.trim()));
                let _ = guard.execute(use_sql.as_str(), &[]).await;
            }
        }

        let stmt_kind = classify_tx_statement(&req.sql);
        if is_read_only && !matches!(stmt_kind, TxStatementKind::Begin | TxStatementKind::Commit | TxStatementKind::Rollback) {
            let max_rows = req.limit.map(|l| l as usize).unwrap_or(usize::MAX);
            let mut stream = guard
                .query(req.sql.as_str(), &[])
                .await
                .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;

            let mut columns: Vec<String> = Vec::new();
            let mut batch: Vec<BTreeMap<String, Value>> = Vec::new();
            let mut chunk_size: usize = 500;
            let mut total_rows: usize = 0;
            let mut batch_index: usize = 0;
            let mut sent_header = false;

            while let Some(item) = stream
                .try_next()
                .await
                .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?
            {
                match item {
                    QueryItem::Metadata(meta) => {
                        if columns.is_empty() {
                            columns = meta.columns().iter().map(|c| c.name().to_string()).collect();
                            chunk_size = adaptive_chunk_size(columns.len());
                        }
                    }
                    QueryItem::Row(row) => {
                        if columns.is_empty() {
                            columns = row.columns().iter().map(|c| c.name().to_string()).collect();
                            chunk_size = adaptive_chunk_size(columns.len());
                        }
                        if total_rows >= max_rows {
                            continue;
                        }
                        batch.push(tiberius_row_to_map(&row, &columns));
                        total_rows += 1;
                        if batch.len() >= chunk_size {
                            let to_send = std::mem::take(&mut batch);
                            let _ = tx
                                .send(QueryChunk {
                                    columns: if !sent_header {
                                        sent_header = true;
                                        columns.clone()
                                    } else {
                                        Vec::new()
                                    },
                                    rows: to_send,
                                    batch_index,
                                    is_last: false,
                                    masked_columns: BTreeMap::new(),
                                })
                                .await;
                            batch_index += 1;
                        }
                    }
                }
            }

            let _ = tx
                .send(QueryChunk {
                    columns: if !sent_header { columns } else { Vec::new() },
                    rows: batch,
                    batch_index,
                    is_last: true,
                    masked_columns: BTreeMap::new(),
                })
                .await;

            return Ok(QuerySummary {
                success: true,
                database: db_label,
                total_rows,
                affected_rows: None,
                is_dml_or_ddl: false,
                duration_ms: started.elapsed().as_millis() as u64,
                route_node: None,
                route_description: Some("SQL Server Native TDS".to_string()),
                masked_columns: BTreeMap::new(),
                error_message: None,
            });
        }

        // Write / transaction statement path
        let exec_res = guard
            .execute(req.sql.as_str(), &[])
            .await
            .map_err(|e| CoreError::ExecutionFailed(e.to_string()))?;
        let affected = exec_res.total();

        if let Ok(mut tx_g) = self.tx_state.lock() {
            match stmt_kind {
                TxStatementKind::Begin => {
                    tx_g.in_transaction = true;
                }
                TxStatementKind::Commit | TxStatementKind::Rollback => {
                    tx_g.in_transaction = false;
                    tx_g.pending_statements = 0;
                }
                TxStatementKind::Other => {
                    if tx_g.manual_mode {
                        if !tx_g.in_transaction {
                            tx_g.in_transaction = true;
                        }
                        tx_g.pending_statements += 1;
                    }
                }
            }
        }

        let _ = tx
            .send(QueryChunk {
                columns: Vec::new(),
                rows: Vec::new(),
                batch_index: 0,
                is_last: true,
                masked_columns: BTreeMap::new(),
            })
            .await;

        Ok(QuerySummary {
            success: true,
            database: db_label,
            total_rows: 0,
            affected_rows: Some(affected),
            is_dml_or_ddl: true,
            duration_ms: started.elapsed().as_millis() as u64,
            route_node: None,
            route_description: Some("SQL Server Native TDS".to_string()),
            masked_columns: BTreeMap::new(),
            error_message: None,
        })
    }

    async fn cancel_running_query(&self, query_id: &str) -> CoreResult<bool> {
        let spid = self
            .running_spids
            .lock()
            .ok()
            .and_then(|g| g.get(query_id).copied());
        let Some(spid_val) = spid else {
            return Ok(false);
        };
        let mut aux = connect_tiberius(&self.endpoint, None).await?;
        let kill_sql = format!("KILL {spid_val}");
        let res = aux.execute(kill_sql.as_str(), &[]).await;
        Ok(res.is_ok())
    }

    async fn explain(&self, db: &str, sql: &str) -> CoreResult<ExplainResult> {
        crate::validate_explain_sql(sql, &DbEngineType::SqlServer, self.read_only)?;
        let started = Instant::now();
        let mut guard = self.client.lock().await;
        if !db.trim().is_empty() {
            let _ = guard
                .execute(format!("USE {};", quote_tsql_ident(db.trim())).as_str(), &[])
                .await;
        }
        guard
            .execute("SET SHOWPLAN_TEXT ON", &[])
            .await
            .map_err(|e| CoreError::ExecutionFailed(format!("启用 SQL Server SHOWPLAN_TEXT 失败: {e}")))?;
        let res = exec_tiberius_collect(&mut guard, sql, None).await;
        let _ = guard.execute("SET SHOWPLAN_TEXT OFF", &[]).await;
        match res {
            Ok((columns, rows)) => Ok(ExplainResult {
                success: true,
                columns,
                rows,
                duration_ms: started.elapsed().as_millis() as u64,
                error_message: None,
            }),
            Err(err) => Ok(ExplainResult {
                success: false,
                columns: Vec::new(),
                rows: Vec::new(),
                duration_ms: started.elapsed().as_millis() as u64,
                error_message: Some(err.to_string()),
            }),
        }
    }

    async fn transaction_status(&self) -> CoreResult<TransactionStatus> {
        let g = self
            .tx_state
            .lock()
            .map_err(|e| CoreError::Transport(e.to_string()))?;
        Ok(TransactionStatus {
            supported: true,
            manual_mode: g.manual_mode,
            in_transaction: g.in_transaction,
            pending_statements: g.pending_statements,
        })
    }

    async fn set_manual_transaction(&self, enabled: bool) -> CoreResult<TransactionStatus> {
        if !enabled {
            return self.commit_transaction().await;
        }
        let mut guard = self.client.lock().await;
        let _ = guard.execute("BEGIN TRANSACTION", &[]).await;
        drop(guard);
        let mut g = self
            .tx_state
            .lock()
            .map_err(|e| CoreError::Transport(e.to_string()))?;
        g.manual_mode = true;
        g.in_transaction = true;
        Ok(TransactionStatus {
            supported: true,
            manual_mode: true,
            in_transaction: true,
            pending_statements: g.pending_statements,
        })
    }

    async fn commit_transaction(&self) -> CoreResult<TransactionStatus> {
        let mut guard = self.client.lock().await;
        let _ = guard.execute("IF @@TRANCOUNT > 0 COMMIT TRANSACTION", &[]).await;
        drop(guard);
        let mut g = self
            .tx_state
            .lock()
            .map_err(|e| CoreError::Transport(e.to_string()))?;
        g.manual_mode = false;
        g.in_transaction = false;
        g.pending_statements = 0;
        Ok(TransactionStatus {
            supported: true,
            manual_mode: false,
            in_transaction: false,
            pending_statements: 0,
        })
    }

    async fn rollback_transaction(&self) -> CoreResult<TransactionStatus> {
        {
            let mut guard = self.client.lock().await;
            let _ = guard.execute("IF @@TRANCOUNT > 0 ROLLBACK TRANSACTION", &[]).await;
        }
        let manual = {
            let mut g = self
                .tx_state
                .lock()
                .map_err(|e| CoreError::Transport(e.to_string()))?;
            g.in_transaction = false;
            g.pending_statements = 0;
            g.manual_mode
        };
        if manual {
            {
                let mut guard2 = self.client.lock().await;
                let _ = guard2.execute("BEGIN TRANSACTION", &[]).await;
            }
            let mut g2 = self
                .tx_state
                .lock()
                .map_err(|e| CoreError::Transport(e.to_string()))?;
            g2.in_transaction = true;
        }
        let g3 = self
            .tx_state
            .lock()
            .map_err(|e| CoreError::Transport(e.to_string()))?;
        Ok(TransactionStatus {
            supported: true,
            manual_mode: g3.manual_mode,
            in_transaction: g3.in_transaction,
            pending_statements: 0,
        })
    }

    fn capabilities(&self) -> CapabilityFlags {
        CapabilityFlags::direct_full(self.read_only)
    }
}

fn should_trust_sqlserver_cert(config: &DirectConfig) -> bool {
    config
        .ssl
        .as_ref()
        .and_then(|s| s.mode.as_deref())
        .is_some_and(|m| {
            matches!(
                m.trim().to_ascii_lowercase().as_str(),
                "insecure" | "skip-verify" | "skip_verify" | "no-verify" | "disable" | "disabled"
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqlserver_tls_verifies_by_default_and_only_skips_on_explicit_insecure() {
        let base_ssl = |mode: Option<&str>| DirectConfig {
            db_type: DbEngineType::SqlServer,
            host: Some("db.internal".to_string()),
            port: Some(1433),
            database: None,
            jdbc_url: "jdbc:sqlserver://db.internal:1433".to_string(),
            driver_ref: None,
            driver_class: None,
            username: Some("sa".to_string()),
            driver_properties: BTreeMap::new(),
            ssh: None,
            ssl: Some(pacer_core::SslConfig {
                enabled: true,
                mode: mode.map(ToString::to_string),
                ..Default::default()
            }),
            read_only: false,
            schema_pattern: None,
            object_filter: None,
            options: None,
            connection_type: None,
            auth_type: None,
            password_save_mode: None,
            socket_path: None,
            has_saved_password: false,
        };

        assert!(!should_trust_sqlserver_cert(&base_ssl(None)));
        assert!(!should_trust_sqlserver_cert(&base_ssl(Some("preferred"))));
        assert!(!should_trust_sqlserver_cert(&base_ssl(Some("required"))));
        assert!(!should_trust_sqlserver_cert(&base_ssl(Some("verify_ca"))));
        assert!(!should_trust_sqlserver_cert(&base_ssl(Some("verify_identity"))));
        assert!(should_trust_sqlserver_cert(&base_ssl(Some("insecure"))));
        assert!(should_trust_sqlserver_cert(&base_ssl(Some("disable"))));
    }
}
