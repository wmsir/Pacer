use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Mutex as StdMutex;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::mpsc;

use pacer_core::{
    CapabilityFlags, ColumnMeta, CoreError, CoreResult, DatabaseMeta, DbBackend, DbEngineType,
    DirectConfig, ExplainResult, IndexMeta, ObjectSearchHit, PingMetrics, QueryChunk, QueryReq,
    QuerySummary, TableMeta,
};
use pacer_sql_ast::{analyze_sql, SqlAccessKind};

use crate::sqlite::adaptive_chunk_size;
use crate::ssh_tunnel::{open_ssh_tunnel, SshTunnelHandle};
use crate::{i64_to_json, u64_to_json};

#[derive(Debug, Deserialize)]
struct ClickHouseMetaCol {
    name: String,
    #[serde(rename = "type")]
    _col_type: String,
}

#[derive(Debug, Deserialize)]
struct ClickHouseJsonCompact {
    #[serde(default)]
    meta: Vec<ClickHouseMetaCol>,
    #[serde(default)]
    data: Vec<Vec<Value>>,
    #[serde(default)]
    _rows: Option<usize>,
}

/// Native ClickHouse backend using its HTTP(S) JSONCompact protocol.
pub struct ClickHouseBackend {
    client: Client,
    base_url: String,
    default_db: String,
    username: String,
    password: Option<String>,
    read_only: bool,
    running_query_ids: StdMutex<HashMap<String, String>>,
    _ssh_tunnel: Option<SshTunnelHandle>,
}

struct ChRunningGuard<'a> {
    map: &'a StdMutex<HashMap<String, String>>,
    key: Option<String>,
}

impl Drop for ChRunningGuard<'_> {
    fn drop(&mut self) {
        if let Some(ref k) = self.key {
            if let Ok(mut g) = self.map.lock() {
                g.remove(k);
            }
        }
    }
}

impl ClickHouseBackend {
    pub async fn connect(config: &DirectConfig, password: Option<&str>) -> CoreResult<Self> {
        let host = config
            .host
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("localhost");
        let port = config.port.unwrap_or(8123);
        let use_ssl = config.ssl.as_ref().is_some_and(|s| s.enabled)
            || config.jdbc_url.to_ascii_lowercase().contains("ssl=true")
            || port == 8443;

        let ssh_tunnel = match config.ssh.as_ref().filter(|s| s.enabled) {
            Some(ssh_cfg) => Some(open_ssh_tunnel(ssh_cfg, host, port).await?),
            None => None,
        };
        let (connect_host, connect_port) = match &ssh_tunnel {
            Some(t) => ("127.0.0.1", t.local_port()),
            None => (host, port),
        };

        let scheme = if use_ssl { "https" } else { "http" };
        let base_url = format!("{scheme}://{connect_host}:{connect_port}/");
        let ssl_mode_low = config
            .ssl
            .as_ref()
            .and_then(|s| s.mode.as_deref())
            .map(|m| m.trim().to_ascii_lowercase());
        let ca_path = config
            .ssl
            .as_ref()
            .and_then(|s| s.ca_cert.as_deref())
            .map(str::trim)
            .filter(|s| !s.is_empty());

        let mut builder = Client::builder().connect_timeout(Duration::from_secs(8));
        if use_ssl {
            if let Some(ca) = ca_path {
                let pem = std::fs::read(ca).map_err(|e| {
                    CoreError::InvalidConfig(format!("读取 ClickHouse CA 证书失败 ({ca}): {e}"))
                })?;
                let cert = reqwest::Certificate::from_pem(&pem).map_err(|e| {
                    CoreError::InvalidConfig(format!("解析 ClickHouse PEM 证书失败 ({ca}): {e}"))
                })?;
                builder = builder.add_root_certificate(cert);
            } else if matches!(
                ssl_mode_low.as_deref(),
                Some("require" | "required" | "prefer" | "preferred" | "allow" | "insecure" | "skip-verify")
            ) {
                builder = builder.danger_accept_invalid_certs(true);
            }
        }
        let client = builder
            .build()
            .map_err(|e| CoreError::Transport(format!("ClickHouse HTTP 客户端初始化失败: {e}")))?;

        let default_db = config
            .database
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("default")
            .to_string();

        let username = config
            .username
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("default")
            .to_string();

        let backend = Self {
            client,
            base_url,
            default_db,
            username,
            password: password.map(|p| p.to_string()),
            read_only: config.read_only,
            running_query_ids: StdMutex::new(HashMap::new()),
            _ssh_tunnel: ssh_tunnel,
        };

        // Validate connectivity immediately on connect.
        let _ = backend.query_compact("SELECT 1", None, None, None).await?;
        Ok(backend)
    }

    fn build_post_request(
        &self,
        sql: &str,
        database: Option<&str>,
        format_name: Option<&str>,
        ch_query_id: Option<&str>,
        timeout_sec: Option<u32>,
    ) -> reqwest::RequestBuilder {
        let mut params: Vec<(&str, String)> = Vec::new();
        if let Some(fmt) = format_name {
            params.push(("default_format", fmt.to_string()));
        }
        if let Some(ch_qid) = ch_query_id {
            params.push(("query_id", ch_qid.to_string()));
        }
        if let Some(t) = timeout_sec.filter(|&s| s > 0) {
            params.push(("max_execution_time", t.to_string()));
        }

        let mut req = self
            .client
            .post(&self.base_url)
            .query(&params)
            .header("X-ClickHouse-User", &self.username);

        if let Some(ref pwd) = self.password {
            if !pwd.is_empty() {
                req = req.header("X-ClickHouse-Key", pwd);
            }
        }
        let db_hdr = database
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(&self.default_db);
        if !db_hdr.is_empty() {
            req = req.header("X-ClickHouse-Database", db_hdr);
        }

        req.body(sql.to_string())
    }

    async fn post_raw(
        &self,
        sql: &str,
        database: Option<&str>,
        with_json_compact: bool,
        ch_query_id: Option<&str>,
        timeout_sec: Option<u32>,
    ) -> CoreResult<String> {
        const MAX_META_BODY_BYTES: usize = 32 * 1024 * 1024;
        let mut resp = self
            .build_post_request(
                sql,
                database,
                if with_json_compact { Some("JSONCompact") } else { None },
                ch_query_id,
                timeout_sec,
            )
            .send()
            .await
            .map_err(|e| CoreError::Transport(format!("ClickHouse 请求失败: {e}")))?;
        let status = resp.status();
        let mut raw = Vec::new();
        while let Some(chunk) = resp
            .chunk()
            .await
            .map_err(|e| CoreError::ExecutionFailed(format!("读取 ClickHouse 响应失败: {e}")))?
        {
            if raw.len().saturating_add(chunk.len()) > MAX_META_BODY_BYTES {
                return Err(CoreError::ExecutionFailed(
                    "ClickHouse 元数据响应体超过 32MB 安全上限".to_string(),
                ));
            }
            raw.extend_from_slice(&chunk);
        }
        let body = String::from_utf8(raw).map_err(|e| {
            CoreError::ParseError(format!("ClickHouse 响应非有效 UTF-8: {e}"))
        })?;
        if !status.is_success() {
            return Err(CoreError::ExecutionFailed(body.trim().to_string()));
        }
        Ok(body)
    }

    async fn query_compact(
        &self,
        sql: &str,
        database: Option<&str>,
        ch_query_id: Option<&str>,
        timeout_sec: Option<u32>,
    ) -> CoreResult<(Vec<String>, Vec<BTreeMap<String, Value>>)> {
        let body = self
            .post_raw(sql, database, true, ch_query_id, timeout_sec)
            .await?;
        let trimmed = body.trim();
        if trimmed.is_empty() {
            return Ok((Vec::new(), Vec::new()));
        }
        let parsed: ClickHouseJsonCompact = serde_json::from_str(trimmed).map_err(|e| {
            CoreError::ParseError(format!("解析 ClickHouse JSONCompact 响应失败: {e}"))
        })?;
        let cols: Vec<String> = parsed.meta.into_iter().map(|m| m.name).collect();
        let mut rows = Vec::with_capacity(parsed.data.len());
        for raw_row in parsed.data {
            let mut map = BTreeMap::new();
            for (idx, col) in cols.iter().enumerate() {
                let v = raw_row.get(idx).cloned().unwrap_or(Value::Null);
                map.insert(col.clone(), normalize_ch_value(v));
            }
            rows.push(map);
        }
        Ok((cols, rows))
    }
}

fn normalize_ch_value(val: Value) -> Value {
    match val {
        Value::Number(ref n) => {
            if let Some(i) = n.as_i64() {
                i64_to_json(i)
            } else if let Some(u) = n.as_u64() {
                u64_to_json(u)
            } else {
                val
            }
        }
        other => other,
    }
}

fn escape_ch_str(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\'', "\\'")
}

#[async_trait]
impl DbBackend for ClickHouseBackend {
    async fn test_connection(&self) -> CoreResult<PingMetrics> {
        let started = Instant::now();
        let (_, rows) = self.query_compact("SELECT version() AS ver", None, None, Some(10)).await?;
        let ver = rows
            .first()
            .and_then(|r| r.get("ver"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        Ok(PingMetrics {
            success: true,
            latency_ms: started.elapsed().as_millis() as u64,
            server_version: Some(format!("ClickHouse {ver}")),
            message: format!("Connected to ClickHouse {ver}"),
        })
    }

    async fn list_databases(&self) -> CoreResult<Vec<DatabaseMeta>> {
        let (_, rows) = self
            .query_compact(
                "SELECT name, engine, comment FROM system.databases ORDER BY name",
                None,
                None,
                None,
            )
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
                    "system" | "INFORMATION_SCHEMA" | "information_schema"
                );
                let comment = r
                    .get("comment")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .or_else(|| {
                        r.get("engine")
                            .and_then(|v| v.as_str())
                            .filter(|s| !s.is_empty())
                            .map(|e| format!("Engine: {e}"))
                    });
                DatabaseMeta {
                    name,
                    charset: Some("UTF-8".to_string()),
                    collation: None,
                    table_count: None,
                    data_size_mb: None,
                    is_system,
                    comment,
                }
            })
            .collect())
    }

    async fn list_tables(&self, db: &str) -> CoreResult<Vec<TableMeta>> {
        let target_db = if db.trim().is_empty() {
            &self.default_db
        } else {
            db.trim()
        };
        let sql = format!(
            "SELECT name, engine, total_rows, round(total_bytes / 1024.0, 2) AS size_kb, comment \
             FROM system.tables WHERE database = '{}' ORDER BY name",
            escape_ch_str(target_db)
        );
        let (_, rows) = self.query_compact(&sql, Some(target_db), None, None).await?;
        Ok(rows
            .into_iter()
            .map(|r| {
                let name = r
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let engine = r
                    .get("engine")
                    .and_then(|v| v.as_str())
                    .map(str::to_string);
                let table_type = if engine
                    .as_deref()
                    .is_some_and(|e| e.to_ascii_lowercase().contains("view"))
                {
                    Some("VIEW".to_string())
                } else {
                    Some("TABLE".to_string())
                };
                let estimated_rows = r.get("total_rows").and_then(|v| match v {
                    Value::Number(n) => n.as_u64(),
                    Value::String(s) => s.parse::<u64>().ok(),
                    _ => None,
                });
                let data_size_kb = r.get("size_kb").and_then(|v| match v {
                    Value::Number(n) => n.as_f64(),
                    Value::String(s) => s.parse::<f64>().ok(),
                    _ => None,
                });
                let comment = r
                    .get("comment")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(str::to_string);
                TableMeta {
                    name,
                    table_type,
                    engine,
                    estimated_rows,
                    data_size_kb,
                    comment,
                }
            })
            .collect())
    }

    async fn list_columns(&self, db: &str, table: &str) -> CoreResult<Vec<ColumnMeta>> {
        let target_db = if db.trim().is_empty() {
            &self.default_db
        } else {
            db.trim()
        };
        let sql = format!(
            "SELECT name, type, default_expression, comment, is_in_primary_key \
             FROM system.columns WHERE database = '{}' AND table = '{}' ORDER BY position",
            escape_ch_str(target_db),
            escape_ch_str(table)
        );
        let (_, rows) = self.query_compact(&sql, Some(target_db), None, None).await?;
        Ok(rows
            .into_iter()
            .map(|r| {
                let data_type = r
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("String")
                    .to_string();
                let nullable = data_type.starts_with("Nullable(");
                let is_pk = r
                    .get("is_in_primary_key")
                    .and_then(|v| match v {
                        Value::Number(n) => Some(n.as_i64().unwrap_or(0) > 0),
                        Value::Bool(b) => Some(*b),
                        _ => None,
                    })
                    .unwrap_or(false);
                ColumnMeta {
                    name: r
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    data_type,
                    nullable,
                    is_primary_key: is_pk,
                    default_value: r
                        .get("default_expression")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                        .map(str::to_string),
                    comment: r
                        .get("comment")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                        .map(str::to_string),
                    masking_rule: None,
                }
            })
            .collect())
    }

    async fn list_indexes(&self, db: &str, table: &str) -> CoreResult<Vec<IndexMeta>> {
        let target_db = if db.trim().is_empty() {
            &self.default_db
        } else {
            db.trim()
        };
        let sql = format!(
            "SELECT name, expr, type FROM system.data_skipping_indices WHERE database = '{}' AND table = '{}'",
            escape_ch_str(target_db),
            escape_ch_str(table)
        );
        let Ok((_, rows)) = self.query_compact(&sql, Some(target_db), None, None).await else {
            return Ok(Vec::new());
        };
        Ok(rows
            .into_iter()
            .map(|r| IndexMeta {
                name: r
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                unique: false,
                columns: r
                    .get("expr")
                    .and_then(|v| v.as_str())
                    .map(|s| vec![s.to_string()])
                    .unwrap_or_default(),
                index_type: r.get("type").and_then(|v| v.as_str()).map(str::to_string),
            })
            .collect())
    }

    async fn search_objects(&self, db: &str, query: &str) -> CoreResult<Vec<ObjectSearchHit>> {
        let needle = query.trim().to_ascii_lowercase();
        if needle.is_empty() {
            return Ok(Vec::new());
        }
        let target_db = if db.trim().is_empty() {
            &self.default_db
        } else {
            db.trim()
        };
        let pattern = escape_ch_str(&needle);
        let mut hits = Vec::new();
        let mut seen = HashSet::new();

        let tbl_sql = format!(
            "SELECT name, engine, comment FROM system.tables \
             WHERE database = '{}' AND positionCaseInsensitiveUTF8(name, '{}') > 0 \
             ORDER BY name LIMIT 150",
            escape_ch_str(target_db),
            pattern
        );
        if let Ok((_, tbl_rows)) = self.query_compact(&tbl_sql, Some(target_db), None, None).await {
            for r in tbl_rows {
                let name = r
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let engine = r.get("engine").and_then(|v| v.as_str()).unwrap_or("TABLE");
                let kind = if engine.to_ascii_lowercase().contains("view") {
                    "view"
                } else {
                    "table"
                };
                if seen.insert(format!("t:{name}")) {
                    hits.push(ObjectSearchHit {
                        kind: kind.to_string(),
                        database: target_db.to_string(),
                        table_name: name,
                        column_name: None,
                        data_type: Some(engine.to_string()),
                        comment: r
                            .get("comment")
                            .and_then(|v| v.as_str())
                            .filter(|s| !s.is_empty())
                            .map(str::to_string),
                    });
                }
            }
        }

        if hits.len() < 150 {
            let rem = 150 - hits.len();
            let col_sql = format!(
                "SELECT table, name, type, comment FROM system.columns \
                 WHERE database = '{}' AND positionCaseInsensitiveUTF8(name, '{}') > 0 \
                 ORDER BY table, position LIMIT {rem}",
                escape_ch_str(target_db),
                pattern
            );
            if let Ok((_, col_rows)) = self.query_compact(&col_sql, Some(target_db), None, None).await
            {
                for r in col_rows {
                    let tbl = r
                        .get("table")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let col = r
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    if seen.insert(format!("c:{tbl}.{col}")) {
                        hits.push(ObjectSearchHit {
                            kind: "column".to_string(),
                            database: target_db.to_string(),
                            table_name: tbl,
                            column_name: Some(col),
                            data_type: r.get("type").and_then(|v| v.as_str()).map(str::to_string),
                            comment: r
                                .get("comment")
                                .and_then(|v| v.as_str())
                                .filter(|s| !s.is_empty())
                                .map(str::to_string),
                        });
                    }
                }
            }
        }
        Ok(hits)
    }

    async fn get_table_ddl(&self, db: &str, table: &str) -> CoreResult<String> {
        let target_db = if db.trim().is_empty() {
            &self.default_db
        } else {
            db.trim()
        };
        let q_db = target_db.replace('`', "``");
        let q_tbl = table.replace('`', "``");
        let sql = format!("SHOW CREATE TABLE `{q_db}`.`{q_tbl}`");
        let (_, rows) = self.query_compact(&sql, Some(target_db), None, None).await?;
        let ddl = rows
            .first()
            .and_then(|r| r.get("statement").or_else(|| r.values().next()))
            .and_then(|v| v.as_str())
            .ok_or_else(|| CoreError::ExecutionFailed(format!("Table {target_db}.{table} not found")))?;
        Ok(if ddl.trim_end().ends_with(';') {
            ddl.to_string()
        } else {
            format!("{ddl};")
        })
    }

    async fn execute_query(
        &self,
        req: QueryReq,
        tx: mpsc::Sender<QueryChunk>,
    ) -> CoreResult<QuerySummary> {
        let started = Instant::now();
        let analysis = analyze_sql(&req.sql, &DbEngineType::ClickHouse);
        let is_read_only = matches!(analysis.overall_kind, SqlAccessKind::ReadOnly);

        if self.read_only && !is_read_only {
            return Err(CoreError::PermissionDenied(
                "Connection is marked read-only; DDL/DML blocked".to_string(),
            ));
        }

        let target_db = req
            .database
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| self.default_db.clone());
        let ch_qid = req
            .query_id
            .as_ref()
            .map(|id| format!("pacer_{}", id.replace('\'', "")));
        let _guard = if let (Some(ref pacer_qid), Some(ref qid_val)) = (&req.query_id, &ch_qid) {
            if let Ok(mut g) = self.running_query_ids.lock() {
                g.insert(pacer_qid.clone(), qid_val.clone());
            }
            Some(ChRunningGuard {
                map: &self.running_query_ids,
                key: Some(pacer_qid.clone()),
            })
        } else {
            None
        };

        if !is_read_only {
            let _ = self
                .post_raw(
                    &req.sql,
                    Some(&target_db),
                    false,
                    ch_qid.as_deref(),
                    req.timeout_sec,
                )
                .await?;
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
                database: target_db,
                total_rows: 0,
                affected_rows: None,
                is_dml_or_ddl: true,
                duration_ms: started.elapsed().as_millis() as u64,
                route_node: None,
                route_description: Some("ClickHouse Native HTTP".to_string()),
                masked_columns: BTreeMap::new(),
                error_message: None,
            });
        }

        const MAX_CH_STREAM_LINE_BYTES: usize = 32 * 1024 * 1024;
        let mut resp = self
            .build_post_request(
                &req.sql,
                Some(&target_db),
                Some("JSONCompactEachRowWithNames"),
                ch_qid.as_deref(),
                req.timeout_sec,
            )
            .send()
            .await
            .map_err(|e| CoreError::Transport(format!("ClickHouse 流式请求失败: {e}")))?;

        let status = resp.status();
        if !status.is_success() {
            let mut err_buf = Vec::new();
            while let Some(chunk) = resp
                .chunk()
                .await
                .map_err(|e| CoreError::ExecutionFailed(format!("读取 ClickHouse 错误失败: {e}")))?
            {
                let remaining = 64 * 1024usize.saturating_sub(err_buf.len());
                if remaining == 0 {
                    break;
                }
                err_buf.extend_from_slice(&chunk[..chunk.len().min(remaining)]);
            }
            let err_msg = String::from_utf8_lossy(&err_buf).trim().to_string();
            return Err(CoreError::ExecutionFailed(err_msg));
        }

        let max_rows = req.limit.map(|l| l as usize).unwrap_or(usize::MAX);
        let mut columns: Vec<String> = Vec::new();
        let mut batch: Vec<BTreeMap<String, Value>> = Vec::new();
        let mut batch_index: usize = 0;
        let mut total_rows: usize = 0;
        let mut line_buf: Vec<u8> = Vec::new();
        let mut header_parsed = false;
        let mut hit_limit = false;

        'stream: while let Some(chunk) = resp
            .chunk()
            .await
            .map_err(|e| CoreError::ExecutionFailed(format!("读取 ClickHouse 流失败: {e}")))?
        {
            for &b in chunk.iter() {
                if b != b'\n' {
                    if line_buf.len() >= MAX_CH_STREAM_LINE_BYTES {
                        return Err(CoreError::ExecutionFailed(
                            "ClickHouse 单行响应超过 32MB 安全上限".to_string(),
                        ));
                    }
                    line_buf.push(b);
                    continue;
                }
                let line = std::mem::take(&mut line_buf);
                let trimmed = String::from_utf8_lossy(&line).trim().to_string();
                if trimmed.is_empty() {
                    continue;
                }
                if !header_parsed {
                    columns = serde_json::from_str::<Vec<String>>(&trimmed).map_err(|e| {
                        CoreError::ParseError(format!("解析 ClickHouse 列头失败: {e}"))
                    })?;
                    header_parsed = true;
                    continue;
                }
                if total_rows >= max_rows {
                    hit_limit = true;
                    break 'stream;
                }
                let raw_row = serde_json::from_str::<Vec<Value>>(&trimmed).map_err(|e| {
                    CoreError::ParseError(format!("解析 ClickHouse 行数据失败: {e}"))
                })?;
                let mut map = BTreeMap::new();
                for (idx, col) in columns.iter().enumerate() {
                    let v = raw_row.get(idx).cloned().unwrap_or(Value::Null);
                    map.insert(col.clone(), normalize_ch_value(v));
                }
                batch.push(map);
                total_rows += 1;

                let flush_sz = adaptive_chunk_size(columns.len());
                if batch.len() >= flush_sz {
                    let send_rows = std::mem::take(&mut batch);
                    let _ = tx
                        .send(QueryChunk {
                            columns: if batch_index == 0 {
                                columns.clone()
                            } else {
                                Vec::new()
                            },
                            rows: send_rows,
                            batch_index,
                            is_last: false,
                            masked_columns: BTreeMap::new(),
                        })
                        .await;
                    batch_index += 1;
                }
                if total_rows >= max_rows {
                    hit_limit = true;
                    break 'stream;
                }
            }
        }

        if !hit_limit && !line_buf.is_empty() {
            let trimmed = String::from_utf8_lossy(&line_buf).trim().to_string();
            if !trimmed.is_empty() {
                if !header_parsed {
                    columns = serde_json::from_str::<Vec<String>>(&trimmed).map_err(|e| {
                        CoreError::ParseError(format!("解析 ClickHouse 列头失败: {e}"))
                    })?;
                } else if total_rows < max_rows {
                    let raw_row = serde_json::from_str::<Vec<Value>>(&trimmed).map_err(|e| {
                        CoreError::ParseError(format!("解析 ClickHouse 行数据失败: {e}"))
                    })?;
                    let mut map = BTreeMap::new();
                    for (idx, col) in columns.iter().enumerate() {
                        let v = raw_row.get(idx).cloned().unwrap_or(Value::Null);
                        map.insert(col.clone(), normalize_ch_value(v));
                    }
                    batch.push(map);
                    total_rows += 1;
                }
            }
        }

        let _ = tx
            .send(QueryChunk {
                columns: if batch_index == 0 {
                    columns.clone()
                } else {
                    Vec::new()
                },
                rows: batch,
                batch_index,
                is_last: true,
                masked_columns: BTreeMap::new(),
            })
            .await;

        Ok(QuerySummary {
            success: true,
            database: target_db,
            total_rows,
            affected_rows: None,
            is_dml_or_ddl: false,
            duration_ms: started.elapsed().as_millis() as u64,
            route_node: None,
            route_description: Some("ClickHouse Native HTTP Stream".to_string()),
            masked_columns: BTreeMap::new(),
            error_message: None,
        })
    }

    async fn cancel_running_query(&self, query_id: &str) -> CoreResult<bool> {
        let ch_qid = self
            .running_query_ids
            .lock()
            .ok()
            .and_then(|g| g.get(query_id).cloned());
        let Some(qid) = ch_qid else {
            return Ok(false);
        };
        let kill_sql = format!("KILL QUERY WHERE query_id = '{}' SYNC", escape_ch_str(&qid));
        let res = self.post_raw(&kill_sql, None, false, None, Some(5)).await;
        Ok(res.is_ok())
    }

    async fn explain(&self, db: &str, sql: &str) -> CoreResult<ExplainResult> {
        let started = Instant::now();
        let clean = sql.trim().trim_end_matches(';');
        let explain_sql = if clean.to_ascii_uppercase().starts_with("EXPLAIN") {
            clean.to_string()
        } else {
            format!("EXPLAIN {clean}")
        };
        match self
            .query_compact(&explain_sql, Some(db), None, Some(30))
            .await
        {
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

    fn capabilities(&self) -> CapabilityFlags {
        CapabilityFlags::direct_full(self.read_only)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    async fn read_http_request(socket: &mut tokio::net::TcpStream) -> (String, String) {
        let mut buf = Vec::new();
        let mut tmp = [0u8; 2048];
        let mut expected_total: Option<usize> = None;
        let mut header_end: Option<usize> = None;

        loop {
            let n = socket.read(&mut tmp).await.unwrap_or(0);
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&tmp[..n]);
            if header_end.is_none() {
                if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                    let h_end = pos + 4;
                    header_end = Some(h_end);
                    let hdr_str = String::from_utf8_lossy(&buf[..pos]);
                    let mut cl = 0usize;
                    for line in hdr_str.lines() {
                        if let Some(v) = line.strip_prefix("Content-Length:").or_else(|| line.strip_prefix("content-length:")) {
                            cl = v.trim().parse::<usize>().unwrap_or(0);
                        }
                    }
                    expected_total = Some(h_end + cl);
                }
            }
            if let Some(total) = expected_total {
                if buf.len() >= total {
                    break;
                }
            }
        }
        let h_end = header_end.unwrap_or(buf.len());
        let head = String::from_utf8_lossy(&buf[..h_end]).to_string();
        let body = String::from_utf8_lossy(&buf[h_end..]).to_string();
        (head, body)
    }

    #[tokio::test]
    async fn test_clickhouse_connect_catalog_streaming_and_read_only() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            loop {
                let Ok((mut sock, _)) = listener.accept().await else {
                    break;
                };
                tokio::spawn(async move {
                    let (head, body) = read_http_request(&mut sock).await;
                    let json_resp = if head.contains("JSONCompactEachRowWithNames") && body.contains("SELECT id, label") {
                        "[\"id\",\"label\"]\n[9007199254740995,\"big\"]\n[1,\"ok\"]\n[2,\"extra\"]\n".to_string()
                    } else if body.contains("SELECT version()") {
                        serde_json::json!({
                            "meta": [{"name": "ver", "type": "String"}],
                            "data": [["24.8.1.1"]]
                        })
                        .to_string()
                    } else if body.contains("system.databases") {
                        serde_json::json!({
                            "meta": [
                                {"name": "name", "type": "String"},
                                {"name": "engine", "type": "String"},
                                {"name": "comment", "type": "String"}
                            ],
                            "data": [["analytics", "Atomic", "Main warehouse"]]
                        })
                        .to_string()
                    } else if body.contains("system.tables") {
                        serde_json::json!({
                            "meta": [
                                {"name": "name", "type": "String"},
                                {"name": "engine", "type": "String"},
                                {"name": "total_rows", "type": "UInt64"},
                                {"name": "size_kb", "type": "Float64"},
                                {"name": "comment", "type": "String"}
                            ],
                            "data": [["events", "MergeTree", "5000", 128.5, "Event stream"]]
                        })
                        .to_string()
                    } else {
                        serde_json::json!({
                            "meta": [{"name": "1", "type": "UInt8"}],
                            "data": [[1]]
                        })
                        .to_string()
                    };
                    let reply = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        json_resp.len(),
                        json_resp
                    );
                    let _ = sock.write_all(reply.as_bytes()).await;
                });
            }
        });

        let mut cfg = DirectConfig {
            db_type: DbEngineType::ClickHouse,
            host: Some("127.0.0.1".to_string()),
            port: Some(addr.port()),
            database: Some("analytics".to_string()),
            jdbc_url: format!("jdbc:clickhouse://127.0.0.1:{}/analytics", addr.port()),
            driver_ref: None,
            driver_class: None,
            username: Some("default".to_string()),
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

        let ch = ClickHouseBackend::connect(&cfg, Some("pass")).await.unwrap();
        let ping = ch.test_connection().await.unwrap();
        assert!(ping.success);
        assert!(ping.server_version.unwrap().contains("24.8.1.1"));

        let dbs = ch.list_databases().await.unwrap();
        assert_eq!(dbs[0].name, "analytics");
        let tbls = ch.list_tables("analytics").await.unwrap();
        assert_eq!(tbls[0].name, "events");
        assert_eq!(tbls[0].estimated_rows, Some(5000));

        let (tx, mut rx) = mpsc::channel(4);
        let sum = ch
            .execute_query(
                QueryReq {
                    database: Some("analytics".to_string()),
                    sql: "SELECT id, label FROM events".to_string(),
                    limit: Some(2),
                    query_id: Some("ch-q1".to_string()),
                    timeout_sec: None,
                },
                tx,
            )
            .await
            .unwrap();
        assert_eq!(sum.total_rows, 2);
        let chunk = rx.recv().await.unwrap();
        assert_eq!(chunk.rows[0]["id"], serde_json::json!("9007199254740995"));

        cfg.read_only = true;
        let ro = ClickHouseBackend::connect(&cfg, Some("pass")).await.unwrap();
        let (tx_ro, _) = mpsc::channel(2);
        let err = ro
            .execute_query(
                QueryReq {
                    database: Some("analytics".to_string()),
                    sql: "DROP TABLE events".to_string(),
                    limit: None,
                    query_id: None,
                    timeout_sec: None,
                },
                tx_ro,
            )
            .await
            .unwrap_err();
        assert!(matches!(err, CoreError::PermissionDenied(_)));

        server.abort();
    }
}
