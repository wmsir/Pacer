use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Supported database engines across direct DataGrip connections and Pacer-managed instances.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DbEngineType {
    MySql,
    MariaDb,
    PostgreSql,
    Oracle,
    Dameng,
    Kingbase,
    OpenGauss,
    OceanBase,
    TiDb,
    SqlServer,
    Sqlite,
    ClickHouse,
    Redis,
    Unknown(String),
}

impl DbEngineType {
    /// Parses loose engine identifiers from DataGrip driver refs, JDBC schemes, or Pacer `dbType`.
    pub fn from_str_loose(raw: &str) -> Self {
        let norm = raw.trim().to_ascii_lowercase();
        if norm.is_empty() {
            return Self::Unknown(String::new());
        }
        if norm.contains("mariadb") {
            Self::MariaDb
        } else if norm.contains("tidb") {
            Self::TiDb
        } else if norm.contains("oceanbase") {
            Self::OceanBase
        } else if norm.contains("mysql") {
            Self::MySql
        } else if norm.contains("opengauss") || norm.contains("gauss") {
            Self::OpenGauss
        } else if norm.contains("kingbase") {
            Self::Kingbase
        } else if norm == "dm" || norm.contains("dameng") || norm.contains("dmjdbc") {
            Self::Dameng
        } else if norm.contains("postgres") || norm == "pg" || norm.contains("pgsql") {
            Self::PostgreSql
        } else if norm.contains("oracle") {
            Self::Oracle
        } else if norm.contains("sqlserver") || norm.contains("mssql") || norm.contains("jtds") {
            Self::SqlServer
        } else if norm.contains("sqlite") {
            Self::Sqlite
        } else if norm.contains("clickhouse") {
            Self::ClickHouse
        } else if norm.contains("redis") {
            Self::Redis
        } else {
            Self::Unknown(raw.trim().to_string())
        }
    }

    /// Maps engine type to the string identifier expected by `wmsir/Pacer` backend.
    pub fn as_pacer_db_type(&self) -> &str {
        match self {
            Self::MySql | Self::MariaDb => "mysql",
            Self::PostgreSql => "postgresql",
            Self::Oracle => "oracle",
            Self::Dameng => "dameng",
            Self::Kingbase => "kingbase",
            Self::OpenGauss => "opengauss",
            Self::OceanBase => "oceanbase",
            Self::TiDb => "tidb",
            Self::SqlServer => "sqlserver",
            Self::Sqlite => "sqlite",
            Self::ClickHouse => "clickhouse",
            Self::Redis => "redis",
            Self::Unknown(s) => s.as_str(),
        }
    }

    /// Returns the conventional default TCP port for this database engine, if applicable.
    pub fn default_port(&self) -> Option<u16> {
        match self {
            Self::MySql | Self::MariaDb => Some(3306),
            Self::PostgreSql | Self::OpenGauss => Some(5432),
            Self::Oracle => Some(1521),
            Self::Dameng => Some(5236),
            Self::Kingbase => Some(54321),
            Self::OceanBase => Some(2881),
            Self::TiDb => Some(4000),
            Self::SqlServer => Some(1433),
            Self::ClickHouse => Some(8123),
            Self::Redis => Some(6379),
            Self::Sqlite | Self::Unknown(_) => None,
        }
    }

    /// Canonical engine key used when comparing connection fingerprints.
    pub fn canonical_family(&self) -> &str {
        match self {
            Self::MySql | Self::MariaDb => "mysql",
            other => other.as_pacer_db_type(),
        }
    }
}

/// Environment classification shared between DataGrip tab colors and Pacer instances.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EnvLevel {
    Dev,
    Test,
    Prod,
    #[default]
    Unknown,
}

impl EnvLevel {
    pub fn from_str_loose(raw: &str) -> Self {
        match raw.trim().to_ascii_uppercase().as_str() {
            "DEV" | "DEVELOPMENT" | "LOCAL" => Self::Dev,
            "TEST" | "TESTING" | "STAGING" | "UAT" | "QA" => Self::Test,
            "PROD" | "PRODUCTION" | "PRD" => Self::Prod,
            _ => Self::Unknown,
        }
    }

    /// Infers an environment tier from DataGrip's `<color>` tag (named preset or hex RGB).
    pub fn from_datagrip_color(color: &str) -> Self {
        let c = color.trim().to_ascii_lowercase();
        if c.is_empty() {
            return Self::Unknown;
        }
        if c.contains("red") || c.contains("rose") || c.contains("crimson") {
            return Self::Prod;
        }
        if c.contains("yellow") || c.contains("amber") || c.contains("orange") || c.contains("gold") {
            return Self::Test;
        }
        if c.contains("green") || c.contains("mint") || c.contains("emerald") || c.contains("teal") {
            return Self::Dev;
        }
        if let Some((r, g, b)) = parse_hex_rgb(&c) {
            // Red-dominant -> Prod; Yellow/Orange-dominant -> Test; Green-dominant -> Dev.
            if r >= 180 && g <= 110 && b <= 110 {
                return Self::Prod;
            }
            if r >= 180 && g >= 130 && b <= 110 {
                return Self::Test;
            }
            if g >= 140 && r <= 130 {
                return Self::Dev;
            }
        }
        Self::Unknown
    }
}

fn parse_hex_rgb(raw: &str) -> Option<(u8, u8, u8)> {
    let hex = raw.strip_prefix('#').unwrap_or(raw);
    if hex.len() == 6 && hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        return Some((r, g, b));
    }
    None
}

fn default_true() -> bool {
    true
}

/// SSH jump-host tunnel configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SshTunnelConfig {
    pub id: Option<String>,
    #[serde(default)]
    pub config_name: Option<String>,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_type: String,
    pub key_file: Option<String>,
    #[serde(default)]
    pub local_port: Option<u16>,
    pub enabled: bool,
    #[serde(default, skip_serializing)]
    pub password: Option<String>,
    #[serde(default, skip_serializing)]
    pub passphrase: Option<String>,
}

/// TLS/SSL transport configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SslConfig {
    pub enabled: bool,
    pub ca_cert: Option<String>,
    pub client_cert: Option<String>,
    pub client_key: Option<String>,
    pub mode: Option<String>,
    #[serde(default)]
    pub use_ide_truststore: bool,
    #[serde(default)]
    pub use_system_truststore: bool,
    #[serde(default)]
    pub client_key_password_save_mode: Option<String>,
}

/// Advanced session, keep-alive, time zone, startup script & introspection options.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionOptionsConfig {
    #[serde(default)]
    pub transaction_control: Option<String>,
    #[serde(default)]
    pub switch_schema: Option<String>,
    #[serde(default)]
    pub time_zone: Option<String>,
    #[serde(default)]
    pub single_session_mode: bool,
    #[serde(default)]
    pub keep_alive_enabled: bool,
    #[serde(default)]
    pub keep_alive_sec: Option<u32>,
    #[serde(default)]
    pub auto_disconnect_enabled: bool,
    #[serde(default)]
    pub auto_disconnect_sec: Option<u32>,
    #[serde(default)]
    pub startup_script: Option<String>,
    #[serde(default = "default_true")]
    pub auto_sync: bool,
    #[serde(default)]
    pub introspection_level: Option<String>,
}

impl Default for ConnectionOptionsConfig {
    fn default() -> Self {
        Self {
            transaction_control: Some("auto".to_string()),
            switch_schema: Some("automatic".to_string()),
            time_zone: None,
            single_session_mode: false,
            keep_alive_enabled: false,
            keep_alive_sec: Some(60),
            auto_disconnect_enabled: false,
            auto_disconnect_sec: Some(300),
            startup_script: None,
            auto_sync: true,
            introspection_level: Some("auto".to_string()),
        }
    }
}

/// Direct database connection configuration (passwords are never stored in plain JSON profiles).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectConfig {
    pub db_type: DbEngineType,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub database: Option<String>,
    pub jdbc_url: String,
    pub driver_ref: Option<String>,
    pub driver_class: Option<String>,
    pub username: Option<String>,
    pub driver_properties: BTreeMap<String, String>,
    pub ssh: Option<SshTunnelConfig>,
    pub ssl: Option<SslConfig>,
    pub read_only: bool,
    pub schema_pattern: Option<String>,
    #[serde(default)]
    pub object_filter: Option<String>,
    #[serde(default)]
    pub options: Option<ConnectionOptionsConfig>,
    /// Connection mode: `"default"`, `"socket"`, or `"urlOnly"`.
    #[serde(default)]
    pub connection_type: Option<String>,
    /// Authentication mode: `"userPassword"` or `"noAuth"`.
    #[serde(default)]
    pub auth_type: Option<String>,
    /// Password persistence strategy: `"forever"`, `"session"`, or `"never"`.
    #[serde(default)]
    pub password_save_mode: Option<String>,
    /// Optional Unix domain socket path when `connection_type == "socket"`.
    #[serde(default)]
    pub socket_path: Option<String>,
    /// Indicates whether a password is already saved (either in encrypted store or current session).
    #[serde(default)]
    pub has_saved_password: bool,
}

/// Execution mode governing whether queries run locally or through Pacer governance proxy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "config", rename_all = "camelCase")]
pub enum ConnectionMode {
    Direct(DirectConfig),
    Hybrid {
        direct: DirectConfig,
        pacer_instance_id: i64,
    },
    PacerManaged {
        workspace_url: String,
        instance_id: i64,
        env: EnvLevel,
    },
}

/// Normalized identity key used to match DataGrip local connections against Pacer instances.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionFingerprint {
    pub db_type: DbEngineType,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub database: Option<String>,
}

impl ConnectionFingerprint {
    pub fn normalized_host(&self) -> Option<String> {
        self.host
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| match s.to_ascii_lowercase().as_str() {
                "127.0.0.1" | "::1" => "localhost".to_string(),
                other => other.to_string(),
            })
    }

    pub fn effective_port(&self) -> Option<u16> {
        self.port.or_else(|| self.db_type.default_port())
    }

    pub fn normalized_database(&self) -> Option<String> {
        self.database
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_ascii_lowercase)
    }

    pub fn matches_host_port(&self, other: &Self) -> bool {
        if self.db_type.canonical_family() != other.db_type.canonical_family() {
            return false;
        }
        match (self.normalized_host(), other.normalized_host()) {
            (Some(a), Some(b)) if a == b => {
                match (self.effective_port(), other.effective_port()) {
                    (Some(pa), Some(pb)) => pa == pb,
                    (None, None) => true,
                    _ => false,
                }
            }
            _ => false,
        }
    }

    pub fn matches_exact_host_port_db(&self, other: &Self) -> bool {
        if !self.matches_host_port(other) {
            return false;
        }
        match (self.normalized_database(), other.normalized_database()) {
            (Some(a), Some(b)) => a == b,
            _ => false,
        }
    }
}

/// Capability matrix mapped directly from Pacer's `supportedOps` or DataGrip `read-only` state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityFlags {
    pub can_query: bool,
    pub can_dml: bool,
    pub can_ddl: bool,
    pub can_export: bool,
    pub can_dry_run: bool,
    pub has_masking: bool,
    pub can_rollback: bool,
    pub can_release: bool,
}

impl Default for CapabilityFlags {
    fn default() -> Self {
        Self::direct_full(false)
    }
}

impl CapabilityFlags {
    pub fn direct_full(read_only: bool) -> Self {
        Self {
            can_query: true,
            can_dml: !read_only,
            can_ddl: !read_only,
            can_export: true,
            can_dry_run: false,
            has_masking: false,
            can_rollback: false,
            can_release: false,
        }
    }

    /// Maps Pacer's Chinese `supportedOps` tags to capability booleans.
    /// When `ops` is empty (legacy instance without explicit tags), defaults to permissive governance flags.
    pub fn from_pacer_ops(ops: &[String]) -> Self {
        if ops.is_empty() {
            return Self {
                can_query: true,
                can_dml: true,
                can_ddl: true,
                can_export: true,
                can_dry_run: true,
                has_masking: true,
                can_rollback: true,
                can_release: true,
            };
        }
        let has = |needle: &str| ops.iter().any(|op| op.contains(needle));
        Self {
            can_query: has("支持查询"),
            can_dml: has("支持DML变更") || has("DML"),
            can_ddl: has("支持DDL结构变更") || has("DDL"),
            can_export: has("支持数据导出"),
            can_dry_run: has("支持事务预执行"),
            has_masking: has("支持数据脱敏"),
            can_rollback: has("支持历史回滚"),
            can_release: has("支持上线"),
        }
    }
}

/// Workspace project grouping multiple database connections, scripts, and a shared execution console.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceProject {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub root_dir: Option<String>,
    #[serde(default)]
    pub created_at: String,
}

/// Unified connection profile shared across direct, DBeaver-imported, and extension-managed connections.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionProfile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub project_id: Option<String>,
    pub source_uuid: Option<String>,
    pub db_type: DbEngineType,
    pub env: EnvLevel,
    pub color: Option<String>,
    pub comment: Option<String>,
    pub tags: Vec<String>,
    pub resource_groups: Vec<String>,
    pub capabilities: CapabilityFlags,
    pub mode: ConnectionMode,
    pub fingerprint: ConnectionFingerprint,
}

/// Connection health check metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PingMetrics {
    pub success: bool,
    pub latency_ms: u64,
    pub server_version: Option<String>,
    pub message: String,
}

/// Database / Schema catalog metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseMeta {
    pub name: String,
    pub charset: Option<String>,
    pub collation: Option<String>,
    pub table_count: Option<u64>,
    pub data_size_mb: Option<f64>,
    pub is_system: bool,
    pub comment: Option<String>,
}

/// Table / View catalog metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableMeta {
    pub name: String,
    pub table_type: Option<String>,
    pub engine: Option<String>,
    pub estimated_rows: Option<u64>,
    pub data_size_kb: Option<f64>,
    pub comment: Option<String>,
}

/// Column catalog metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnMeta {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub is_primary_key: bool,
    pub default_value: Option<String>,
    pub comment: Option<String>,
    pub masking_rule: Option<String>,
}

/// Request payload for backend query execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryReq {
    pub database: Option<String>,
    pub sql: String,
    pub limit: Option<u32>,
}

/// Streamed batch of query rows sent via `mpsc` / Tauri `Channel`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryChunk {
    pub columns: Vec<String>,
    pub rows: Vec<BTreeMap<String, serde_json::Value>>,
    pub batch_index: usize,
    pub is_last: bool,
    pub masked_columns: BTreeMap<String, String>,
}

/// Final summary returned when query execution finishes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuerySummary {
    pub success: bool,
    pub database: String,
    pub total_rows: usize,
    #[serde(default)]
    pub affected_rows: Option<u64>,
    #[serde(default)]
    pub is_dml_or_ddl: bool,
    pub duration_ms: u64,
    pub route_node: Option<String>,
    pub route_description: Option<String>,
    pub masked_columns: BTreeMap<String, String>,
    pub error_message: Option<String>,
}

/// Execution plan (`EXPLAIN`) result returned by backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExplainResult {
    pub success: bool,
    pub columns: Vec<String>,
    pub rows: Vec<BTreeMap<String, serde_json::Value>>,
    pub duration_ms: u64,
    pub error_message: Option<String>,
}

/// Persisted or session query execution record displayed in the IDE History tab.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryHistoryEntry {
    pub id: String,
    pub timestamp: String,
    #[serde(default)]
    pub project_id: Option<String>,
    pub profile_id: String,
    pub profile_name: String,
    pub database: Option<String>,
    pub sql: String,
    pub duration_ms: u64,
    pub row_count: usize,
    pub affected_rows: Option<u64>,
    pub success: bool,
    pub error_message: Option<String>,
}

/// Table index metadata shown in the Database Explorer & Structure tool window.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexMeta {
    pub name: String,
    pub unique: bool,
    pub columns: Vec<String>,
    pub index_type: Option<String>,
}

/// Fenced SQL code block extracted from a Markdown (`.md`) document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkdownSqlBlock {
    pub index: usize,
    pub language: String,
    /// 1-based inclusive fence opening line number.
    pub start_line: usize,
    /// 1-based inclusive fence closing line number.
    pub end_line: usize,
    /// 1-based inclusive first SQL content line.
    pub content_start_line: usize,
    /// 1-based inclusive last SQL content line.
    pub content_end_line: usize,
    pub sql: String,
    /// Nearest preceding Markdown heading, if any.
    pub title: Option<String>,
}

/// Opened local text/SQL/Markdown file with detected encoding, line count, and line endings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenedFileDto {
    pub path: String,
    pub name: String,
    pub content: String,
    pub encoding: String,
    pub has_bom: bool,
    pub line_ending: String,
    pub line_count: usize,
    pub char_count: usize,
    pub byte_size: u64,
    pub file_kind: String,
    pub sql_blocks: Vec<MarkdownSqlBlock>,
}

/// Saved file statistics returned after writing a file to disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileStatsDto {
    pub path: String,
    pub encoding: String,
    pub has_bom: bool,
    pub line_ending: String,
    pub line_count: usize,
    pub char_count: usize,
    pub byte_size: u64,
}

/// Entry shown in the DataGrip-style local Files / Workspace Explorer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceFileEntry {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub extension: Option<String>,
}

/// Auto-discovered local DBeaver workspace connection file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbeaverDiscoveryItem {
    pub path: String,
    pub workspace_name: String,
    pub format: String,
    pub connection_count: usize,
    pub has_credentials_file: bool,
}

/// Global database object/column/Redis-key search hit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectSearchHit {
    pub kind: String,
    pub database: String,
    pub table_name: String,
    pub column_name: Option<String>,
    pub data_type: Option<String>,
    pub comment: Option<String>,
}

/// Foreign key relationship metadata for table navigation & ER diagrams.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForeignKeyMeta {
    pub name: String,
    pub column_name: String,
    pub referenced_table: String,
    pub referenced_column: String,
    pub on_delete: Option<String>,
    pub on_update: Option<String>,
}

/// Stored procedure, function, or trigger metadata in a SQL database/schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutineMeta {
    pub name: String,
    pub kind: String,
    pub return_type: Option<String>,
    pub definition: Option<String>,
    pub comment: Option<String>,
}

/// Live ACID transaction session status for a database backend.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionStatus {
    pub supported: bool,
    pub manual_mode: bool,
    pub in_transaction: bool,
    pub pending_statements: u32,
}
