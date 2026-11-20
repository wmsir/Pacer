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

/// SSH jump-host tunnel configuration imported from DataGrip `sshConfigs.xml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SshTunnelConfig {
    pub id: Option<String>,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_type: String,
    pub key_file: Option<String>,
    pub enabled: bool,
}

/// TLS/SSL transport configuration parsed from DataGrip `<ssl-config>`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SslConfig {
    pub enabled: bool,
    pub ca_cert: Option<String>,
    pub client_cert: Option<String>,
    pub client_key: Option<String>,
    pub mode: Option<String>,
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

/// Unified connection profile shared across DataGrip imports and Pacer-managed connections.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionProfile {
    pub id: String,
    pub name: String,
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
