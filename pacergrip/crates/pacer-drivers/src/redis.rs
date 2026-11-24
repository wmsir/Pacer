use std::collections::BTreeMap;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use native_tls::{Certificate, TlsConnector as NativeTlsConnector};
use serde_json::Value as JsonValue;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufStream, ReadBuf};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex};
use tokio_native_tls::{TlsConnector, TlsStream};

use pacer_core::{
    CapabilityFlags, ColumnMeta, CoreError, CoreResult, DatabaseMeta, DbBackend, DbEngineType,
    DirectConfig, ExplainResult, ObjectSearchHit, PingMetrics, QueryChunk, QueryReq, QuerySummary,
    TableMeta,
};
use pacer_sql_ast::{analyze_sql, split_redis_commands, tokenize_redis_command, SqlAccessKind};

use crate::sqlite::stream_chunks;
use crate::ssh_tunnel::{open_ssh_tunnel, SshTunnelHandle};

const IO_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_RESP_DEPTH: usize = 16;
const SCAN_KEY_CAP: usize = 200;
const SCAN_MAX_ROUNDS: usize = 24;
const DEFAULT_ARRAY_ROW_LIMIT: usize = 1000;

enum RawRedisStream {
    Plain(TcpStream),
    Tls(Box<TlsStream<TcpStream>>),
}

impl AsyncRead for RawRedisStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            Self::Plain(s) => Pin::new(s).poll_read(cx, buf),
            Self::Tls(s) => Pin::new(&mut **s).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for RawRedisStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        match self.get_mut() {
            Self::Plain(s) => Pin::new(s).poll_write(cx, buf),
            Self::Tls(s) => Pin::new(&mut **s).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            Self::Plain(s) => Pin::new(s).poll_flush(cx),
            Self::Tls(s) => Pin::new(&mut **s).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            Self::Plain(s) => Pin::new(s).poll_shutdown(cx),
            Self::Tls(s) => Pin::new(&mut **s).poll_shutdown(cx),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RespValue {
    SimpleString(String),
    Error(String),
    Integer(i64),
    BulkBytes(Vec<u8>),
    Null,
    Array(Vec<RespValue>),
}

impl RespValue {
    fn to_display_string(&self) -> String {
        match self {
            Self::SimpleString(s) | Self::Error(s) => s.clone(),
            Self::Integer(n) => n.to_string(),
            Self::Null => "(nil)".to_string(),
            Self::BulkBytes(bytes) => decode_bytes_display(bytes),
            Self::Array(items) => {
                let mapped: Vec<JsonValue> = items.iter().map(Self::to_json_value).collect();
                serde_json::to_string(&mapped).unwrap_or_else(|_| format!("[array({})]", items.len()))
            }
        }
    }

    fn to_json_value(&self) -> JsonValue {
        match self {
            Self::SimpleString(s) | Self::Error(s) => JsonValue::String(s.clone()),
            Self::Integer(n) => crate::i64_to_json(*n),
            Self::Null => JsonValue::Null,
            Self::BulkBytes(bytes) => JsonValue::String(decode_bytes_display(bytes)),
            Self::Array(items) => JsonValue::Array(items.iter().map(Self::to_json_value).collect()),
        }
    }

    fn as_text(&self) -> Option<String> {
        match self {
            Self::SimpleString(s) => Some(s.clone()),
            Self::Integer(n) => Some(n.to_string()),
            Self::BulkBytes(bytes) => Some(decode_bytes_display(bytes)),
            _ => None,
        }
    }

    fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Integer(n) => Some(*n),
            Self::SimpleString(s) => s.trim().parse::<i64>().ok(),
            Self::BulkBytes(b) => std::str::from_utf8(b).ok()?.trim().parse::<i64>().ok(),
            _ => None,
        }
    }
}

fn decode_bytes_display(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => {
            let preview_len = bytes.len().min(24);
            let mut hex = String::with_capacity(preview_len * 2);
            for b in &bytes[..preview_len] {
                use std::fmt::Write as _;
                let _ = write!(&mut hex, "{b:02x}");
            }
            if bytes.len() > preview_len {
                format!("[binary {} bytes] 0x{hex}...", bytes.len())
            } else {
                format!("[binary {} bytes] 0x{hex}", bytes.len())
            }
        }
    }
}

struct RedisConn {
    stream: BufStream<RawRedisStream>,
    current_db: u32,
}

impl RedisConn {
    async fn connect(endpoint: &RedisEndpoint) -> CoreResult<Self> {
        let addr = (endpoint.connect_host.as_str(), endpoint.connect_port);
        let tcp = tokio::time::timeout(IO_TIMEOUT, TcpStream::connect(addr))
            .await
            .map_err(|_| {
                CoreError::Transport(format!(
                    "Redis TCP connection timed out ({}:{})",
                    endpoint.host, endpoint.port
                ))
            })?
            .map_err(|e| {
                CoreError::Transport(format!(
                    "Redis connection failed ({}:{}): {e}",
                    endpoint.host, endpoint.port
                ))
            })?;
        let _ = tcp.set_nodelay(true);

        let raw = if endpoint.use_tls {
            let mut tls_builder = NativeTlsConnector::builder();
            if let Some(ref ca_path) = endpoint.ca_cert {
                let pem = std::fs::read(ca_path).map_err(|e| {
                    CoreError::InvalidConfig(format!("Failed reading Redis SSL CA file '{ca_path}': {e}"))
                })?;
                let cert = Certificate::from_pem(&pem).map_err(|e| {
                    CoreError::InvalidConfig(format!("Invalid Redis SSL CA PEM certificate: {e}"))
                })?;
                tls_builder.add_root_certificate(cert);
            }
            match endpoint.ssl_mode.as_deref().map(str::to_ascii_lowercase).as_deref() {
                Some("verify-full" | "verify_full") => {}
                Some("verify-ca" | "verify_ca") => {
                    tls_builder.danger_accept_invalid_hostnames(true);
                }
                _ => {
                    tls_builder.danger_accept_invalid_certs(true);
                    tls_builder.danger_accept_invalid_hostnames(true);
                }
            }
            let connector = tls_builder.build().map_err(|e| {
                CoreError::Transport(format!("Redis TLS initialization failed: {e}"))
            })?;
            let async_tls = TlsConnector::from(connector);
            let tls_stream = tokio::time::timeout(
                IO_TIMEOUT,
                async_tls.connect(&endpoint.host, tcp),
            )
            .await
            .map_err(|_| CoreError::Transport("Redis TLS handshake timed out".to_string()))?
            .map_err(|e| CoreError::Transport(format!("Redis TLS handshake failed: {e}")))?;
            RawRedisStream::Tls(Box::new(tls_stream))
        } else {
            RawRedisStream::Plain(tcp)
        };

        let mut conn = Self {
            stream: BufStream::new(raw),
            current_db: 0,
        };

        // Authenticate using Redis 6+ ACL (AUTH username password) or legacy AUTH password.
        if let Some(ref pwd) = endpoint.password {
            let explicit_user = endpoint
                .username
                .as_deref()
                .map(str::trim)
                .filter(|u| !u.is_empty() && !u.eq_ignore_ascii_case("default"));
            if let Some(user) = explicit_user {
                let args = vec!["AUTH".to_string(), user.to_string(), pwd.clone()];
                match conn.send_command_raw(&args).await? {
                    RespValue::Error(msg)
                        if msg.to_ascii_lowercase().contains("wrong number of arguments")
                            || msg.to_ascii_lowercase().contains("syntax error") =>
                    {
                        let fallback = vec!["AUTH".to_string(), pwd.clone()];
                        if let RespValue::Error(e2) = conn.send_command_raw(&fallback).await? {
                            return Err(CoreError::AuthRequired(format!("Redis AUTH failed: {e2}")));
                        }
                    }
                    RespValue::Error(msg) => {
                        return Err(CoreError::AuthRequired(format!("Redis AUTH failed: {msg}")));
                    }
                    _ => {}
                }
            } else {
                let args = vec!["AUTH".to_string(), pwd.clone()];
                if let RespValue::Error(msg) = conn.send_command_raw(&args).await? {
                    return Err(CoreError::AuthRequired(format!("Redis AUTH failed: {msg}")));
                }
            }
        }

        if endpoint.default_db > 0 {
            conn.select_db(endpoint.default_db).await?;
        }

        for cmd_line in &endpoint.startup_commands {
            let tokens = tokenize_redis_command(cmd_line);
            if !tokens.is_empty() {
                let _ = conn.send_command_raw(&tokens).await;
            }
        }

        Ok(conn)
    }

    async fn select_db(&mut self, db_index: u32) -> CoreResult<()> {
        if self.current_db == db_index {
            return Ok(());
        }
        let args = vec!["SELECT".to_string(), db_index.to_string()];
        match self.send_command_checked(&args).await {
            Ok(_) => {
                self.current_db = db_index;
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    async fn send_command_checked(&mut self, args: &[String]) -> CoreResult<RespValue> {
        match self.send_command_raw(args).await? {
            RespValue::Error(msg) => Err(CoreError::ExecutionFailed(format!("Redis error: {msg}"))),
            other => Ok(other),
        }
    }

    async fn send_command_raw(&mut self, args: &[String]) -> CoreResult<RespValue> {
        self.write_command(args).await?;
        tokio::time::timeout(IO_TIMEOUT, self.stream.flush())
            .await
            .map_err(|_| CoreError::Transport("Redis socket write flush timed out".to_string()))?
            .map_err(|e| CoreError::Transport(format!("Redis write flush failed: {e}")))?;
        self.read_value_with_timeout(0).await
    }

    async fn send_pipeline_raw(&mut self, cmds: &[Vec<String>]) -> CoreResult<Vec<RespValue>> {
        if cmds.is_empty() {
            return Ok(Vec::new());
        }
        for cmd in cmds {
            self.write_command(cmd).await?;
        }
        tokio::time::timeout(IO_TIMEOUT, self.stream.flush())
            .await
            .map_err(|_| CoreError::Transport("Redis pipeline flush timed out".to_string()))?
            .map_err(|e| CoreError::Transport(format!("Redis pipeline flush failed: {e}")))?;

        let mut replies = Vec::with_capacity(cmds.len());
        for _ in 0..cmds.len() {
            replies.push(self.read_value_with_timeout(0).await?);
        }
        Ok(replies)
    }

    async fn write_command(&mut self, args: &[String]) -> CoreResult<()> {
        let header = format!("*{}\r\n", args.len());
        self.stream
            .write_all(header.as_bytes())
            .await
            .map_err(|e| CoreError::Transport(format!("Redis write failed: {e}")))?;
        for arg in args {
            let bytes = arg.as_bytes();
            let prefix = format!("${}\r\n", bytes.len());
            self.stream
                .write_all(prefix.as_bytes())
                .await
                .map_err(|e| CoreError::Transport(format!("Redis write failed: {e}")))?;
            self.stream
                .write_all(bytes)
                .await
                .map_err(|e| CoreError::Transport(format!("Redis write failed: {e}")))?;
            self.stream
                .write_all(b"\r\n")
                .await
                .map_err(|e| CoreError::Transport(format!("Redis write failed: {e}")))?;
        }
        Ok(())
    }

    async fn read_value_with_timeout(&mut self, depth: usize) -> CoreResult<RespValue> {
        tokio::time::timeout(IO_TIMEOUT, self.read_value(depth))
            .await
            .map_err(|_| CoreError::Transport("Redis server response timed out".to_string()))?
    }

    fn read_value<'a>(
        &'a mut self,
        depth: usize,
    ) -> Pin<Box<dyn std::future::Future<Output = CoreResult<RespValue>> + Send + 'a>> {
        Box::pin(async move {
            if depth > MAX_RESP_DEPTH {
                return Err(CoreError::ParseError(
                    "RESP nesting depth exceeded maximum limit".to_string(),
                ));
            }
            let line = self.read_crlf_line().await?;
            let (prefix, rest) = line
                .split_first()
                .ok_or_else(|| CoreError::Transport("Redis closed connection unexpectedly".to_string()))?;
            let header = std::str::from_utf8(rest)
                .map_err(|e| CoreError::ParseError(format!("Invalid UTF-8 in RESP header: {e}")))?
                .to_string();

            match *prefix {
                b'+' => Ok(RespValue::SimpleString(header)),
                b'-' => Ok(RespValue::Error(header)),
                b':' => {
                    let n = header.trim().parse::<i64>().map_err(|e| {
                        CoreError::ParseError(format!("Invalid RESP integer '{header}': {e}"))
                    })?;
                    Ok(RespValue::Integer(n))
                }
                b'$' => {
                    let len = header.trim().parse::<isize>().map_err(|e| {
                        CoreError::ParseError(format!("Invalid RESP bulk length '{header}': {e}"))
                    })?;
                    if len < 0 {
                        return Ok(RespValue::Null);
                    }
                    let byte_len = len as usize;
                    let mut buf = vec![0u8; byte_len + 2];
                    self.stream.read_exact(&mut buf).await.map_err(|e| {
                        CoreError::Transport(format!("Redis stream read interrupted: {e}"))
                    })?;
                    buf.truncate(byte_len);
                    Ok(RespValue::BulkBytes(buf))
                }
                b'*' => {
                    let count = header.trim().parse::<isize>().map_err(|e| {
                        CoreError::ParseError(format!("Invalid RESP array length '{header}': {e}"))
                    })?;
                    if count < 0 {
                        return Ok(RespValue::Null);
                    }
                    let cap = (count as usize).min(100_000);
                    let mut items = Vec::with_capacity(cap);
                    for _ in 0..count {
                        items.push(self.read_value(depth + 1).await?);
                    }
                    Ok(RespValue::Array(items))
                }
                other => Err(CoreError::ParseError(format!(
                    "Unsupported RESP type byte '{}' in reply '{header}'",
                    char::from(other)
                ))),
            }
        })
    }

    async fn read_crlf_line(&mut self) -> CoreResult<Vec<u8>> {
        let mut line = Vec::new();
        let n = self
            .stream
            .read_until(b'\n', &mut line)
            .await
            .map_err(|e| CoreError::Transport(format!("Redis socket read failed: {e}")))?;
        if n == 0 {
            return Err(CoreError::Transport(
                "Redis connection closed by remote host".to_string(),
            ));
        }
        if line.ends_with(b"\r\n") {
            line.truncate(line.len() - 2);
        } else if line.ends_with(b"\n") {
            line.truncate(line.len() - 1);
        }
        Ok(line)
    }
}

#[derive(Debug, Clone)]
struct RedisEndpoint {
    host: String,
    port: u16,
    connect_host: String,
    connect_port: u16,
    username: Option<String>,
    password: Option<String>,
    default_db: u32,
    use_tls: bool,
    ssl_mode: Option<String>,
    ca_cert: Option<String>,
    startup_commands: Vec<String>,
}

/// Native async Redis backend implemented directly over RESP2 + optional TLS (`redis://` / `rediss://`).
pub struct RedisBackend {
    endpoint: RedisEndpoint,
    read_only: bool,
    conn: Mutex<Option<RedisConn>>,
    _ssh_tunnel: Option<SshTunnelHandle>,
}

impl RedisBackend {
    pub async fn connect(config: &DirectConfig, password: Option<&str>) -> CoreResult<Self> {
        let host = config
            .host
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("127.0.0.1")
            .to_string();
        let port = config.port.unwrap_or(6379);
        let default_db = parse_redis_db_index(config.database.as_deref()).unwrap_or(0);

        let url_tls = config
            .jdbc_url
            .to_ascii_lowercase()
            .contains("rediss://");
        let prop_tls = config
            .driver_properties
            .get("ssl")
            .or_else(|| config.driver_properties.get("tls"))
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(false);
        let ssl_enabled = config.ssl.as_ref().is_some_and(|s| s.enabled);

        let startup_commands = config
            .options
            .as_ref()
            .and_then(|o| o.startup_script.as_deref())
            .map(split_redis_commands)
            .unwrap_or_default();

        let ssh_tunnel = match config.ssh.as_ref().filter(|s| s.enabled) {
            Some(ssh_cfg) => Some(open_ssh_tunnel(ssh_cfg, &host, port).await?),
            None => None,
        };
        let (connect_host, connect_port) = match &ssh_tunnel {
            Some(t) => ("127.0.0.1".to_string(), t.local_port()),
            None => (host.clone(), port),
        };

        let endpoint = RedisEndpoint {
            host,
            port,
            connect_host,
            connect_port,
            username: config.username.clone().filter(|s| !s.trim().is_empty()),
            password: password
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty()),
            default_db,
            use_tls: ssl_enabled || url_tls || prop_tls,
            ssl_mode: config.ssl.as_ref().and_then(|s| s.mode.clone()),
            ca_cert: config
                .ssl
                .as_ref()
                .and_then(|s| s.ca_cert.as_deref())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToString::to_string),
            startup_commands,
        };

        let initial_conn = RedisConn::connect(&endpoint).await?;

        Ok(Self {
            endpoint,
            read_only: config.read_only,
            conn: Mutex::new(Some(initial_conn)),
            _ssh_tunnel: ssh_tunnel,
        })
    }
}

macro_rules! redis_retry {
    ($self:expr, $conn:ident, $body:block) => {{
        let mut guard = $self.conn.lock().await;
        if guard.is_none() {
            *guard = Some(RedisConn::connect(&$self.endpoint).await?);
        }
        let first_res = {
            let $conn = guard.as_mut().expect("connection populated");
            $body
        };
        match first_res {
            Ok(v) => Ok(v),
            Err(CoreError::Transport(_)) => {
                // Reconnect once automatically if idle Redis connection was dropped by server/proxy.
                *guard = None;
                let mut fresh = RedisConn::connect(&$self.endpoint).await?;
                let retry_res = {
                    let $conn = &mut fresh;
                    $body
                };
                if retry_res.is_ok() {
                    *guard = Some(fresh);
                }
                retry_res
            }
            Err(other) => Err(other),
        }
    }};
}

#[async_trait]
impl DbBackend for RedisBackend {
    async fn test_connection(&self) -> CoreResult<PingMetrics> {
        let started = Instant::now();
        let (pong, info_text) = redis_retry!(self, conn, {
            let pong = conn
                .send_command_checked(&["PING".to_string()])
                .await?
                .to_display_string();
            let info = conn
                .send_command_raw(&["INFO".to_string(), "server".to_string()])
                .await
                .ok()
                .and_then(|v| match v {
                    RespValue::Error(_) => None,
                    other => other.as_text(),
                })
                .unwrap_or_default();
            Ok((pong, info))
        })?;

        let version = parse_info_field(&info_text, "redis_version").map(|ver| {
            if let Some(mode) = parse_info_field(&info_text, "redis_mode") {
                format!("Redis {ver} ({mode})")
            } else {
                format!("Redis {ver}")
            }
        });
        let latency_ms = started.elapsed().as_millis() as u64;
        let proto = if self.endpoint.use_tls { "TLS" } else { "TCP" };
        Ok(PingMetrics {
            success: true,
            latency_ms,
            server_version: version.or_else(|| Some("Redis".to_string())),
            message: format!(
                "Redis ({proto}) {}:{} 连接正常 [{pong}] ({latency_ms} ms)",
                self.endpoint.host, self.endpoint.port
            ),
        })
    }

    async fn list_databases(&self) -> CoreResult<Vec<DatabaseMeta>> {
        let default_db = self.endpoint.default_db;
        let (keyspace_raw, db_count_raw) = redis_retry!(self, conn, {
            let ks = conn
                .send_command_raw(&["INFO".to_string(), "keyspace".to_string()])
                .await
                .ok()
                .and_then(|v| v.as_text())
                .unwrap_or_default();
            let cfg = conn
                .send_command_raw(&[
                    "CONFIG".to_string(),
                    "GET".to_string(),
                    "databases".to_string(),
                ])
                .await
                .ok();
            Ok((ks, cfg))
        })?;

        let mut ks_stats: BTreeMap<u32, (u64, String)> = BTreeMap::new();
        for line in keyspace_raw.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("db") {
                if let Some((idx_str, metrics)) = rest.split_once(':') {
                    if let Ok(idx) = idx_str.trim().parse::<u32>() {
                        let keys = parse_keyspace_metric(metrics, "keys").unwrap_or(0);
                        ks_stats.insert(idx, (keys, metrics.trim().to_string()));
                    }
                }
            }
        }

        let configured_dbs = db_count_raw
            .and_then(|resp| match resp {
                RespValue::Array(items) if items.len() >= 2 => items[1].as_i64(),
                _ => None,
            })
            .and_then(|n| u32::try_from(n).ok())
            .filter(|&n| n > 0 && n <= 256)
            .unwrap_or(16);

        let max_seen_db = ks_stats.keys().copied().max().map(|m| m + 1).unwrap_or(0);
        let total = configured_dbs.max(max_seen_db).max(default_db + 1);

        let mut out = Vec::with_capacity(total as usize);
        for idx in 0..total {
            let stats = ks_stats.get(&idx);
            out.push(DatabaseMeta {
                name: format!("db{idx}"),
                charset: Some("UTF-8".to_string()),
                collation: None,
                table_count: Some(stats.map(|(k, _)| *k).unwrap_or(0)),
                data_size_mb: None,
                is_system: false,
                comment: stats.map(|(_, detail)| detail.clone()),
            });
        }
        Ok(out)
    }

    async fn list_tables(&self, db: &str) -> CoreResult<Vec<TableMeta>> {
        let db_idx = parse_redis_db_index(Some(db)).unwrap_or(self.endpoint.default_db);

        redis_retry!(self, conn, {
            conn.select_db(db_idx).await?;

            let mut keys: Vec<String> = Vec::new();
            let mut cursor = "0".to_string();
            for _ in 0..SCAN_MAX_ROUNDS {
                let reply = conn
                    .send_command_checked(&[
                        "SCAN".to_string(),
                        cursor.clone(),
                        "COUNT".to_string(),
                        "200".to_string(),
                    ])
                    .await?;
                let RespValue::Array(parts) = reply else {
                    break;
                };
                let Some(next_cursor) = parts.first().and_then(RespValue::as_text) else {
                    break;
                };
                if let Some(RespValue::Array(batch_keys)) = parts.get(1) {
                    for k in batch_keys {
                        if let Some(text) = k.as_text() {
                            if !keys.contains(&text) {
                                keys.push(text);
                            }
                        }
                        if keys.len() >= SCAN_KEY_CAP {
                            break;
                        }
                    }
                }
                cursor = next_cursor;
                if cursor == "0" || keys.len() >= SCAN_KEY_CAP {
                    break;
                }
            }

            keys.sort();

            let mut metas = Vec::with_capacity(keys.len());
            for chunk in keys.chunks(80) {
                let mut cmds = Vec::with_capacity(chunk.len() * 2);
                for key in chunk {
                    cmds.push(vec!["TYPE".to_string(), key.clone()]);
                    cmds.push(vec!["TTL".to_string(), key.clone()]);
                }
                let replies = conn.send_pipeline_raw(&cmds).await?;
                for (idx, key) in chunk.iter().enumerate() {
                    let key_type = replies
                        .get(idx * 2)
                        .and_then(RespValue::as_text)
                        .unwrap_or_else(|| "string".to_string());
                    let ttl_val = replies.get(idx * 2 + 1).and_then(RespValue::as_i64);
                    let ttl_label = format_ttl_label(ttl_val);
                    metas.push(TableMeta {
                        name: key.clone(),
                        table_type: Some(key_type.clone()),
                        engine: Some(ttl_label.clone()),
                        estimated_rows: None,
                        data_size_kb: None,
                        comment: Some(format!("type={key_type} | ttl={ttl_label}")),
                    });
                }
            }
            Ok(metas)
        })
    }

    async fn list_columns(&self, db: &str, key: &str) -> CoreResult<Vec<ColumnMeta>> {
        let db_idx = parse_redis_db_index(Some(db)).unwrap_or(self.endpoint.default_db);

        redis_retry!(self, conn, {
            conn.select_db(db_idx).await?;
            let key_inner = key.to_string();
            let pipe = conn
                .send_pipeline_raw(&[
                    vec!["TYPE".to_string(), key_inner.clone()],
                    vec!["TTL".to_string(), key_inner.clone()],
                    vec![
                        "OBJECT".to_string(),
                        "ENCODING".to_string(),
                        key_inner.clone(),
                    ],
                ])
                .await?;

            let key_type = pipe
                .first()
                .and_then(RespValue::as_text)
                .unwrap_or_else(|| "none".to_string());
            let ttl_val = pipe.get(1).and_then(RespValue::as_i64);
            let encoding = pipe
                .get(2)
                .and_then(|v| match v {
                    RespValue::Error(_) => None,
                    other => other.as_text(),
                })
                .unwrap_or_else(|| "-".to_string());

            let len_cmd = match key_type.as_str() {
                "hash" => Some("HLEN"),
                "list" => Some("LLEN"),
                "set" => Some("SCARD"),
                "zset" => Some("ZCARD"),
                "stream" => Some("XLEN"),
                "string" => Some("STRLEN"),
                _ => None,
            };
            let size_str = if let Some(cmd) = len_cmd {
                conn.send_command_raw(&[cmd.to_string(), key_inner.clone()])
                    .await
                    .ok()
                    .and_then(|v| v.as_i64())
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "-".to_string())
            } else {
                "-".to_string()
            };

            let mut cols = vec![
                ColumnMeta {
                    name: "type".to_string(),
                    data_type: key_type.clone(),
                    nullable: false,
                    is_primary_key: true,
                    default_value: None,
                    comment: Some("Redis 数据类型".to_string()),
                    masking_rule: None,
                },
                ColumnMeta {
                    name: "ttl".to_string(),
                    data_type: format_ttl_label(ttl_val),
                    nullable: false,
                    is_primary_key: false,
                    default_value: None,
                    comment: Some("剩余过期时间".to_string()),
                    masking_rule: None,
                },
                ColumnMeta {
                    name: "encoding".to_string(),
                    data_type: encoding,
                    nullable: true,
                    is_primary_key: false,
                    default_value: None,
                    comment: Some("内部紧凑编码".to_string()),
                    masking_rule: None,
                },
                ColumnMeta {
                    name: "length".to_string(),
                    data_type: size_str,
                    nullable: true,
                    is_primary_key: false,
                    default_value: None,
                    comment: Some("元素数量或字节长度".to_string()),
                    masking_rule: None,
                },
            ];

            if key_type == "hash" {
                if let Ok(RespValue::Array(parts)) = conn
                    .send_command_checked(&[
                        "HSCAN".to_string(),
                        key_inner,
                        "0".to_string(),
                        "COUNT".to_string(),
                        "40".to_string(),
                    ])
                    .await
                {
                    if let Some(RespValue::Array(kv)) = parts.get(1) {
                        for pair in kv.chunks(2) {
                            let field = pair.first().and_then(RespValue::as_text).unwrap_or_default();
                            let val = pair.get(1).map(RespValue::to_display_string);
                            if !field.is_empty() {
                                cols.push(ColumnMeta {
                                    name: field,
                                    data_type: "hash-field".to_string(),
                                    nullable: true,
                                    is_primary_key: false,
                                    default_value: val.map(|s| {
                                        if s.chars().count() > 48 {
                                            let head: String = s.chars().take(48).collect();
                                            format!("{head}...")
                                        } else {
                                            s
                                        }
                                    }),
                                    comment: Some("Hash 字段".to_string()),
                                    masking_rule: None,
                                });
                            }
                        }
                    }
                }
            }

            Ok(cols)
        })
    }

    async fn search_objects(&self, db: &str, query: &str) -> CoreResult<Vec<ObjectSearchHit>> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }
        let pattern = if trimmed.contains('*') || trimmed.contains('?') {
            trimmed.to_string()
        } else {
            format!("*{trimmed}*")
        };
        let db_idx = parse_redis_db_index(Some(db)).unwrap_or(self.endpoint.default_db);
        let db_label = format!("db{db_idx}");

        redis_retry!(self, conn, {
            conn.select_db(db_idx).await?;
            let mut keys: Vec<String> = Vec::new();
            let mut cursor = "0".to_string();
            for _ in 0..SCAN_MAX_ROUNDS {
                let reply = conn
                    .send_command_checked(&[
                        "SCAN".to_string(),
                        cursor.clone(),
                        "MATCH".to_string(),
                        pattern.clone(),
                        "COUNT".to_string(),
                        "200".to_string(),
                    ])
                    .await?;
                let RespValue::Array(parts) = reply else {
                    break;
                };
                let Some(next_cursor) = parts.first().and_then(RespValue::as_text) else {
                    break;
                };
                if let Some(RespValue::Array(batch_keys)) = parts.get(1) {
                    for k in batch_keys {
                        if let Some(text) = k.as_text() {
                            if !keys.contains(&text) {
                                keys.push(text);
                            }
                        }
                        if keys.len() >= 150 {
                            break;
                        }
                    }
                }
                cursor = next_cursor;
                if cursor == "0" || keys.len() >= 150 {
                    break;
                }
            }
            keys.sort();

            let mut hits = Vec::with_capacity(keys.len());
            for chunk in keys.chunks(80) {
                let mut cmds = Vec::with_capacity(chunk.len() * 2);
                for key in chunk {
                    cmds.push(vec!["TYPE".to_string(), key.clone()]);
                    cmds.push(vec!["TTL".to_string(), key.clone()]);
                }
                let replies = conn.send_pipeline_raw(&cmds).await?;
                for (idx, key) in chunk.iter().enumerate() {
                    let key_type = replies
                        .get(idx * 2)
                        .and_then(RespValue::as_text)
                        .unwrap_or_else(|| "string".to_string());
                    let ttl_val = replies.get(idx * 2 + 1).and_then(RespValue::as_i64);
                    hits.push(ObjectSearchHit {
                        kind: "key".to_string(),
                        database: db_label.clone(),
                        table_name: key.clone(),
                        column_name: None,
                        data_type: Some(key_type),
                        comment: Some(format_ttl_label(ttl_val)),
                    });
                }
            }
            Ok(hits)
        })
    }

    async fn get_table_ddl(&self, db: &str, key: &str) -> CoreResult<String> {
        let db_idx = parse_redis_db_index(Some(db)).unwrap_or(self.endpoint.default_db);

        redis_retry!(self, conn, {
            conn.select_db(db_idx).await?;
            let key_inner = key.to_string();
            let pipe = conn
                .send_pipeline_raw(&[
                    vec!["TYPE".to_string(), key_inner.clone()],
                    vec!["TTL".to_string(), key_inner.clone()],
                    vec![
                        "OBJECT".to_string(),
                        "ENCODING".to_string(),
                        key_inner.clone(),
                    ],
                ])
                .await?;

            let key_type = pipe
                .first()
                .and_then(RespValue::as_text)
                .unwrap_or_else(|| "string".to_string());
            let ttl = format_ttl_label(pipe.get(1).and_then(RespValue::as_i64));
            let encoding = pipe
                .get(2)
                .and_then(|v| match v {
                    RespValue::Error(_) => None,
                    other => other.as_text(),
                })
                .unwrap_or_else(|| "unknown".to_string());

            let qk = quote_redis_key(&key_inner);
            let inspect_cmd = match key_type.as_str() {
                "hash" => format!("HGETALL {qk}"),
                "list" => format!("LRANGE {qk} 0 99"),
                "set" => format!("SMEMBERS {qk}"),
                "zset" => format!("ZRANGE {qk} 0 99 WITHSCORES"),
                "stream" => format!("XRANGE {qk} - + COUNT 100"),
                _ => format!("GET {qk}"),
            };

            Ok(format!(
                "-- Redis Key 详情与常用操作模板\n\
                 -- Database : db{db_idx}\n\
                 -- Key      : {key_inner}\n\
                 -- Type     : {key_type} (encoding: {encoding})\n\
                 -- TTL      : {ttl}\n\n\
                 {inspect_cmd}\n\
                 TTL {qk}\n\
                 -- 修改过期时间: EXPIRE {qk} 3600\n\
                 -- 删除键: DEL {qk}\n"
            ))
        })
    }

    async fn execute_query(
        &self,
        req: QueryReq,
        tx: mpsc::Sender<QueryChunk>,
    ) -> CoreResult<QuerySummary> {
        let started = Instant::now();
        let analysis = analyze_sql(&req.sql, &DbEngineType::Redis);
        let is_read_only = matches!(analysis.overall_kind, SqlAccessKind::ReadOnly);

        if self.read_only && !is_read_only {
            return Err(CoreError::PermissionDenied(
                "Connection is marked read-only; mutating/admin Redis command blocked".to_string(),
            ));
        }

        let cmds = split_redis_commands(&req.sql);
        if cmds.is_empty() {
            return Err(CoreError::InvalidConfig(
                "未检测到可执行的 Redis 命令（例如: GET user:1001、HGETALL session:1、SCAN 0 COUNT 50）"
                    .to_string(),
            ));
        }

        let target_db_idx = parse_redis_db_index(req.database.as_deref());
        let row_limit = req
            .limit
            .map(|l| l.max(1) as usize)
            .unwrap_or(DEFAULT_ARRAY_ROW_LIMIT);

        let (final_cols, final_rows, final_db, last_integer, cmds_tokens): (
            Vec<String>,
            Vec<BTreeMap<String, JsonValue>>,
            u32,
            Option<i64>,
            Vec<String>,
        ) = redis_retry!(self, conn, {
            if let Some(db_idx) = target_db_idx {
                conn.select_db(db_idx).await?;
            }

            let mut last_tokens: Vec<String> = Vec::new();
            let mut last_resp = RespValue::Null;

            for cmd_line in &cmds {
                let tokens = tokenize_redis_command(cmd_line);
                if tokens.is_empty() {
                    continue;
                }
                let resp = conn.send_command_checked(&tokens).await?;
                if tokens[0].eq_ignore_ascii_case("SELECT") {
                    if let Some(next_db) = tokens.get(1).and_then(|s| parse_redis_db_index(Some(s))) {
                        conn.current_db = next_db;
                    }
                }
                last_tokens = tokens;
                last_resp = resp;
            }

            let int_val = if let RespValue::Integer(n) = last_resp {
                Some(n)
            } else {
                None
            };
            let (cols, rows) = format_resp_table(&last_tokens, &last_resp, row_limit);
            Ok((cols, rows, conn.current_db, int_val, last_tokens))
        })?;

        let total_rows = final_rows.len();
        stream_chunks(&final_cols, final_rows, &tx).await;

        let affected_rows = if !is_read_only {
            last_integer.and_then(|n| u64::try_from(n).ok()).or(Some(1))
        } else {
            None
        };
        let cmd_head = cmds_tokens
            .first()
            .map(|s| s.to_ascii_uppercase())
            .unwrap_or_else(|| "REDIS".to_string());

        Ok(QuerySummary {
            success: true,
            database: format!("db{final_db}"),
            total_rows,
            affected_rows,
            is_dml_or_ddl: !is_read_only,
            duration_ms: started.elapsed().as_millis() as u64,
            route_node: Some(format!("redis://{}:{}", self.endpoint.host, self.endpoint.port)),
            route_description: Some(format!("Redis {cmd_head} @ db{final_db}")),
            masked_columns: BTreeMap::new(),
            error_message: None,
        })
    }

    async fn explain(&self, db: &str, sql: &str) -> CoreResult<ExplainResult> {
        let started = Instant::now();
        let analysis = analyze_sql(sql, &DbEngineType::Redis);
        let cmds = split_redis_commands(sql);
        let first_cmd = cmds.first().map(|c| tokenize_redis_command(c)).unwrap_or_default();
        let cmd_name = first_cmd
            .first()
            .map(|s| s.to_ascii_uppercase())
            .unwrap_or_else(|| "INFO".to_string());
        let key_arg = first_cmd.get(1).cloned();
        let db_idx = parse_redis_db_index(Some(db)).unwrap_or(self.endpoint.default_db);

        let key_diag = redis_retry!(self, conn, {
            conn.select_db(db_idx).await?;
            if let Some(ref k) = key_arg {
                let pipe = conn
                    .send_pipeline_raw(&[
                        vec!["TYPE".to_string(), k.clone()],
                        vec!["TTL".to_string(), k.clone()],
                        vec!["OBJECT".to_string(), "ENCODING".to_string(), k.clone()],
                        vec!["MEMORY".to_string(), "USAGE".to_string(), k.clone()],
                    ])
                    .await?;
                let ktype = pipe.first().and_then(RespValue::as_text).unwrap_or_default();
                let ttl = format_ttl_label(pipe.get(1).and_then(RespValue::as_i64));
                let enc = pipe
                    .get(2)
                    .and_then(|v| match v {
                        RespValue::Error(_) => None,
                        other => other.as_text(),
                    })
                    .unwrap_or_else(|| "N/A".to_string());
                let mem = pipe
                    .get(3)
                    .and_then(RespValue::as_i64)
                    .map(|b| format!("{b} bytes"))
                    .unwrap_or_else(|| "N/A".to_string());
                Ok(Some((k.clone(), ktype, ttl, enc, mem)))
            } else {
                Ok(None)
            }
        })?;

        let columns = vec!["property".to_string(), "value".to_string()];
        let mut rows = Vec::new();
        let push_row = |rows: &mut Vec<BTreeMap<String, JsonValue>>, k: &str, v: String| {
            let mut m = BTreeMap::new();
            m.insert("property".to_string(), JsonValue::String(k.to_string()));
            m.insert("value".to_string(), JsonValue::String(v));
            rows.push(m);
        };

        push_row(&mut rows, "command", cmd_name);
        push_row(
            &mut rows,
            "access_kind",
            format!("{:?}", analysis.overall_kind),
        );
        push_row(&mut rows, "database", format!("db{db_idx}"));

        if let Some((k, ktype, ttl, enc, mem)) = key_diag {
            push_row(&mut rows, "target_key", k);
            push_row(&mut rows, "key_type", ktype);
            push_row(&mut rows, "ttl", ttl);
            push_row(&mut rows, "encoding", enc);
            push_row(&mut rows, "memory_usage", mem);
        }

        let warnings: Vec<String> = analysis
            .statements
            .iter()
            .flat_map(|s| s.warnings.iter().map(|w| format!("[{}] {}", w.code, w.message)))
            .collect();
        if !warnings.is_empty() {
            push_row(&mut rows, "risk_warnings", warnings.join(" | "));
        }

        Ok(ExplainResult {
            success: true,
            columns,
            rows,
            duration_ms: started.elapsed().as_millis() as u64,
            error_message: None,
        })
    }

    fn capabilities(&self) -> CapabilityFlags {
        CapabilityFlags::direct_full(self.read_only)
    }
}

fn parse_redis_db_index(raw: Option<&str>) -> Option<u32> {
    let text = raw?.trim();
    if text.is_empty() {
        return None;
    }
    let norm = text
        .strip_prefix("db")
        .or_else(|| text.strip_prefix("DB"))
        .unwrap_or(text);
    norm.trim().parse::<u32>().ok()
}

fn parse_info_field(info: &str, key: &str) -> Option<String> {
    for line in info.lines() {
        let trimmed = line.trim();
        if let Some((k, v)) = trimmed.split_once(':') {
            if k.trim().eq_ignore_ascii_case(key) {
                let val = v.trim();
                if !val.is_empty() {
                    return Some(val.to_string());
                }
            }
        }
    }
    None
}

fn parse_keyspace_metric(metrics: &str, key: &str) -> Option<u64> {
    for part in metrics.split(',') {
        if let Some((k, v)) = part.trim().split_once('=') {
            if k.trim().eq_ignore_ascii_case(key) {
                if let Ok(n) = v.trim().parse::<u64>() {
                    return Some(n);
                }
            }
        }
    }
    None
}

fn format_ttl_label(ttl: Option<i64>) -> String {
    match ttl {
        Some(-1) => "永久".to_string(),
        Some(-2) => "不存在/已过期".to_string(),
        Some(secs) if secs >= 0 => {
            if secs >= 86_400 {
                format!("{}d {}h", secs / 86_400, (secs % 86_400) / 3600)
            } else {
                format!("{secs}s")
            }
        }
        _ => "-".to_string(),
    }
}

fn quote_redis_key(key: &str) -> String {
    if key.is_empty()
        || key
            .chars()
            .any(|c| c.is_whitespace() || c == '"' || c == '\'' || c.is_control())
    {
        let escaped = key
            .replace('\\', "\\\\")
            .replace('"', "\\\"");
        format!("\"{escaped}\"")
    } else {
        key.to_string()
    }
}

fn format_resp_table(
    tokens: &[String],
    resp: &RespValue,
    row_limit: usize,
) -> (Vec<String>, Vec<BTreeMap<String, JsonValue>>) {
    let cmd = tokens
        .first()
        .map(|s| s.to_ascii_uppercase())
        .unwrap_or_default();
    let args_upper: Vec<String> = tokens.iter().map(|s| s.to_ascii_uppercase()).collect();

    // 1. `INFO [section]` -> section / property / value
    if cmd == "INFO" {
        if let Some(text) = resp.as_text() {
            let cols = vec![
                "section".to_string(),
                "property".to_string(),
                "value".to_string(),
            ];
            let mut rows = Vec::new();
            let mut current_section = "Server".to_string();
            for line in text.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if let Some(sec) = trimmed.strip_prefix('#') {
                    let s = sec.trim();
                    if !s.is_empty() {
                        current_section = s.to_string();
                    }
                    continue;
                }
                if let Some((k, v)) = trimmed.split_once(':') {
                    let mut m = BTreeMap::new();
                    m.insert("section".to_string(), JsonValue::String(current_section.clone()));
                    m.insert("property".to_string(), JsonValue::String(k.trim().to_string()));
                    m.insert("value".to_string(), JsonValue::String(v.trim().to_string()));
                    rows.push(m);
                    if rows.len() >= row_limit {
                        break;
                    }
                }
            }
            if !rows.is_empty() {
                return (cols, rows);
            }
        }
    }

    // 2. `HGETALL` / `CONFIG GET` -> pair field & value
    if cmd == "HGETALL" || (cmd == "CONFIG" && args_upper.get(1).map(String::as_str) == Some("GET"))
    {
        if let RespValue::Array(items) = resp {
            let col0 = if cmd == "CONFIG" { "parameter" } else { "field" };
            let cols = vec![col0.to_string(), "value".to_string()];
            let mut rows = Vec::new();
            for pair in items.chunks(2).take(row_limit) {
                let mut m = BTreeMap::new();
                m.insert(
                    col0.to_string(),
                    pair.first().map(RespValue::to_json_value).unwrap_or(JsonValue::Null),
                );
                m.insert(
                    "value".to_string(),
                    pair.get(1).map(RespValue::to_json_value).unwrap_or(JsonValue::Null),
                );
                rows.push(m);
            }
            return (cols, rows);
        }
    }

    // 3. Sorted Set with WITHSCORES
    if matches!(
        cmd.as_str(),
        "ZRANGE" | "ZREVRANGE" | "ZRANGEBYSCORE" | "ZREVRANGEBYSCORE" | "ZPOPMIN" | "ZPOPMAX"
    ) && (args_upper.iter().any(|a| a == "WITHSCORES") || matches!(cmd.as_str(), "ZPOPMIN" | "ZPOPMAX"))
    {
        if let RespValue::Array(items) = resp {
            let cols = vec!["member".to_string(), "score".to_string()];
            let mut rows = Vec::new();
            for pair in items.chunks(2).take(row_limit) {
                let mut m = BTreeMap::new();
                m.insert(
                    "member".to_string(),
                    pair.first().map(RespValue::to_json_value).unwrap_or(JsonValue::Null),
                );
                m.insert(
                    "score".to_string(),
                    pair.get(1).map(RespValue::to_json_value).unwrap_or(JsonValue::Null),
                );
                rows.push(m);
            }
            return (cols, rows);
        }
    }

    // 4. Cursor-based SCAN commands (`[cursor, [elements...]]`)
    if matches!(cmd.as_str(), "SCAN" | "HSCAN" | "SSCAN" | "ZSCAN") {
        if let RespValue::Array(parts) = resp {
            let next_cursor = parts
                .first()
                .and_then(RespValue::as_text)
                .unwrap_or_else(|| "0".to_string());
            let items = match parts.get(1) {
                Some(RespValue::Array(arr)) => arr.as_slice(),
                _ => &[],
            };
            match cmd.as_str() {
                "HSCAN" => {
                    let cols = vec![
                        "cursor".to_string(),
                        "field".to_string(),
                        "value".to_string(),
                    ];
                    let mut rows = Vec::new();
                    for pair in items.chunks(2).take(row_limit) {
                        let mut m = BTreeMap::new();
                        m.insert("cursor".to_string(), JsonValue::String(next_cursor.clone()));
                        m.insert(
                            "field".to_string(),
                            pair.first().map(RespValue::to_json_value).unwrap_or(JsonValue::Null),
                        );
                        m.insert(
                            "value".to_string(),
                            pair.get(1).map(RespValue::to_json_value).unwrap_or(JsonValue::Null),
                        );
                        rows.push(m);
                    }
                    return (cols, rows);
                }
                "ZSCAN" => {
                    let cols = vec![
                        "cursor".to_string(),
                        "member".to_string(),
                        "score".to_string(),
                    ];
                    let mut rows = Vec::new();
                    for pair in items.chunks(2).take(row_limit) {
                        let mut m = BTreeMap::new();
                        m.insert("cursor".to_string(), JsonValue::String(next_cursor.clone()));
                        m.insert(
                            "member".to_string(),
                            pair.first().map(RespValue::to_json_value).unwrap_or(JsonValue::Null),
                        );
                        m.insert(
                            "score".to_string(),
                            pair.get(1).map(RespValue::to_json_value).unwrap_or(JsonValue::Null),
                        );
                        rows.push(m);
                    }
                    return (cols, rows);
                }
                _ => {
                    let item_col = if cmd == "SSCAN" { "member" } else { "key" };
                    let cols = vec![
                        "cursor".to_string(),
                        "index".to_string(),
                        item_col.to_string(),
                    ];
                    let mut rows = Vec::new();
                    for (idx, item) in items.iter().take(row_limit).enumerate() {
                        let mut m = BTreeMap::new();
                        m.insert("cursor".to_string(), JsonValue::String(next_cursor.clone()));
                        m.insert("index".to_string(), JsonValue::from(idx as i64));
                        m.insert(item_col.to_string(), item.to_json_value());
                        rows.push(m);
                    }
                    if rows.is_empty() {
                        let mut m = BTreeMap::new();
                        m.insert("cursor".to_string(), JsonValue::String(next_cursor));
                        m.insert("index".to_string(), JsonValue::Null);
                        m.insert(item_col.to_string(), JsonValue::Null);
                        rows.push(m);
                    }
                    return (cols, rows);
                }
            }
        }
    }

    // 5. `MGET k1 k2 ...` -> key / value
    if cmd == "MGET" {
        if let RespValue::Array(items) = resp {
            let cols = vec!["key".to_string(), "value".to_string()];
            let mut rows = Vec::new();
            for (idx, val) in items.iter().take(row_limit).enumerate() {
                let k = tokens
                    .get(idx + 1)
                    .cloned()
                    .unwrap_or_else(|| format!("key[{idx}]"));
                let mut m = BTreeMap::new();
                m.insert("key".to_string(), JsonValue::String(k));
                m.insert("value".to_string(), val.to_json_value());
                rows.push(m);
            }
            return (cols, rows);
        }
    }

    // 6. `HMGET key f1 f2 ...` -> field / value
    if cmd == "HMGET" {
        if let RespValue::Array(items) = resp {
            let cols = vec!["field".to_string(), "value".to_string()];
            let mut rows = Vec::new();
            for (idx, val) in items.iter().take(row_limit).enumerate() {
                let f = tokens
                    .get(idx + 2)
                    .cloned()
                    .unwrap_or_else(|| format!("field[{idx}]"));
                let mut m = BTreeMap::new();
                m.insert("field".to_string(), JsonValue::String(f));
                m.insert("value".to_string(), val.to_json_value());
                rows.push(m);
            }
            return (cols, rows);
        }
    }

    // 7. `XRANGE` / `XREVRANGE` -> id / fields
    if matches!(cmd.as_str(), "XRANGE" | "XREVRANGE") {
        if let RespValue::Array(entries) = resp {
            let cols = vec!["id".to_string(), "fields".to_string()];
            let mut rows = Vec::new();
            for entry in entries.iter().take(row_limit) {
                let mut m = BTreeMap::new();
                if let RespValue::Array(parts) = entry {
                    let id_val = parts.first().map(RespValue::to_json_value).unwrap_or(JsonValue::Null);
                    let fields_val = match parts.get(1) {
                        Some(RespValue::Array(kv)) => {
                            let mut obj = serde_json::Map::new();
                            for pair in kv.chunks(2) {
                                if let Some(fk) = pair.first().and_then(RespValue::as_text) {
                                    let fv = pair
                                        .get(1)
                                        .map(RespValue::to_json_value)
                                        .unwrap_or(JsonValue::Null);
                                    obj.insert(fk, fv);
                                }
                            }
                            JsonValue::String(serde_json::to_string(&obj).unwrap_or_default())
                        }
                        Some(other) => JsonValue::String(other.to_display_string()),
                        None => JsonValue::Null,
                    };
                    m.insert("id".to_string(), id_val);
                    m.insert("fields".to_string(), fields_val);
                } else {
                    m.insert("id".to_string(), JsonValue::Null);
                    m.insert("fields".to_string(), entry.to_json_value());
                }
                rows.push(m);
            }
            return (cols, rows);
        }
    }

    // 8. Generic Array -> index / value
    if let RespValue::Array(items) = resp {
        let cols = vec!["index".to_string(), "value".to_string()];
        let mut rows = Vec::new();
        for (idx, item) in items.iter().take(row_limit).enumerate() {
            let mut m = BTreeMap::new();
            m.insert("index".to_string(), JsonValue::from(idx as i64));
            let val = match item {
                RespValue::Array(_) => JsonValue::String(item.to_display_string()),
                other => other.to_json_value(),
            };
            m.insert("value".to_string(), val);
            rows.push(m);
        }
        return (cols, rows);
    }

    // 9. Scalar reply (`SimpleString`, `BulkBytes`, `Integer`, `Null`)
    let cols = vec!["command".to_string(), "result".to_string()];
    let mut m = BTreeMap::new();
    let label = if tokens.len() <= 3 {
        tokens.join(" ")
    } else {
        cmd
    };
    m.insert("command".to_string(), JsonValue::String(label));
    m.insert("result".to_string(), resp.to_json_value());
    (cols, vec![m])
}
