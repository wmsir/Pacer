use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use chrono::Local;
use serde::Deserialize;
use tauri::ipc::Channel;
use tauri::State;
use tokio::sync::{mpsc, RwLock as AsyncRwLock};
use uuid::Uuid;

use pacer_core::{
    AuditEvent, CapabilityFlags, ColumnMeta, ConnectionFingerprint, ConnectionMode,
    ConnectionProfile, CoreError, DatabaseMeta, DbBackend, DbEngineType, DirectConfig, EnvLevel,
    ExplainResult, ExtensionMeta, ExtensionRegistry, FileStatsDto, IndexMeta, MarkdownSqlBlock,
    OpenedFileDto, PingMetrics, PreExecuteDecision, QueryChunk, QueryHistoryEntry, QueryReq,
    QuerySummary, TableMeta, WorkspaceFileEntry,
};
use pacer_datagrip_io::{
    export_to_datagrip_xml, import_from_path, parse_datagrip_xml, parse_jdbc_url,
};
use pacer_drivers::{connect_direct, create_demo_sqlite_db};
use pacer_sql_ast::{
    analyze_sql, extract_markdown_sql_blocks, extract_statement_at_cursor, format_sql,
    ScriptAnalysis,
};

use crate::files::{
    ensure_demo_markdown_file, list_workspace_files, open_text_file, save_text_file,
};

#[cfg(feature = "pacer-extension")]
pub use pacer_client::PacerClient;

const MAX_HISTORY_ENTRIES: usize = 200;

pub struct AppState {
    pub connections: RwLock<Vec<ConnectionProfile>>,
    pub passwords: RwLock<HashMap<String, String>>,
    pub active_backends: AsyncRwLock<HashMap<String, Arc<dyn DbBackend>>>,
    pub history: RwLock<Vec<QueryHistoryEntry>>,
    pub extensions: ExtensionRegistry,
    pub storage_dir: PathBuf,
}

impl AppState {
    pub fn new(storage_dir: PathBuf) -> Self {
        let connections = load_connections_from_disk(&storage_dir);
        Self {
            connections: RwLock::new(connections),
            passwords: RwLock::new(HashMap::new()),
            active_backends: AsyncRwLock::new(HashMap::new()),
            history: RwLock::new(Vec::new()),
            extensions: ExtensionRegistry::new(),
            storage_dir,
        }
    }

    fn persist_connections(&self) -> Result<(), String> {
        let snapshot = self
            .connections
            .read()
            .map_err(|e| e.to_string())?
            .clone();
        std::fs::create_dir_all(&self.storage_dir).map_err(|e| e.to_string())?;
        let path = self.storage_dir.join("connections.json");
        let json = serde_json::to_string_pretty(&snapshot).map_err(|e| e.to_string())?;
        std::fs::write(path, json).map_err(|e| e.to_string())?;
        Ok(())
    }

    fn get_profile(&self, profile_id: &str) -> Result<ConnectionProfile, String> {
        let guard = self.connections.read().map_err(|e| e.to_string())?;
        guard
            .iter()
            .find(|p| p.id == profile_id)
            .cloned()
            .ok_or_else(|| format!("connection profile not found: {profile_id}"))
    }

    fn merge_profiles(&self, incoming: Vec<ConnectionProfile>) -> Result<Vec<ConnectionProfile>, String> {
        {
            let mut guard = self.connections.write().map_err(|e| e.to_string())?;
            for item in &incoming {
                if let Some(existing) = guard.iter_mut().find(|p| {
                    p.id == item.id
                        || (p.source_uuid.is_some() && p.source_uuid == item.source_uuid)
                }) {
                    *existing = item.clone();
                } else {
                    guard.push(item.clone());
                }
            }
        }
        for item in &incoming {
            self.active_backends.blocking_write().remove(&item.id);
        }
        self.persist_connections()?;
        Ok(incoming)
    }

    async fn resolve_backend(
        &self,
        profile_id: &str,
        password_override: Option<String>,
    ) -> Result<(ConnectionProfile, Arc<dyn DbBackend>), String> {
        let profile = self.get_profile(profile_id)?;

        if let Some(pwd) = password_override {
            if let Ok(mut guard) = self.passwords.write() {
                guard.insert(profile_id.to_string(), pwd);
            }
            self.active_backends.write().await.remove(profile_id);
        } else if let Some(cached) = self.active_backends.read().await.get(profile_id).cloned() {
            return Ok((profile, cached));
        }

        let stored_pwd = self
            .passwords
            .read()
            .map_err(|e| e.to_string())?
            .get(profile_id)
            .cloned();

        let direct = match &profile.mode {
            ConnectionMode::Direct(d) | ConnectionMode::Hybrid { direct: d, .. } => d.clone(),
            ConnectionMode::PacerManaged { .. } => {
                return Err(
                    "Connection is marked as an external managed profile; enable an extension plugin to execute."
                        .to_string(),
                )
            }
        };

        let backend = connect_direct(&direct, stored_pwd.as_deref())
            .await
            .map_err(map_err)?;
        self.active_backends
            .write()
            .await
            .insert(profile_id.to_string(), Arc::clone(&backend));

        Ok((profile, backend))
    }

    fn push_history(&self, entry: QueryHistoryEntry) {
        if let Ok(mut guard) = self.history.write() {
            guard.insert(0, entry);
            if guard.len() > MAX_HISTORY_ENTRIES {
                guard.truncate(MAX_HISTORY_ENTRIES);
            }
        }
    }
}

fn load_connections_from_disk(storage_dir: &Path) -> Vec<ConnectionProfile> {
    let path = storage_dir.join("connections.json");
    let Ok(raw) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    serde_json::from_str::<Vec<ConnectionProfile>>(&raw).unwrap_or_default()
}

fn map_err(err: CoreError) -> String {
    err.to_string()
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionDraft {
    pub id: Option<String>,
    pub name: String,
    pub db_type: DbEngineType,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub database: Option<String>,
    pub jdbc_url: Option<String>,
    pub username: Option<String>,
    pub env: Option<EnvLevel>,
    pub read_only: Option<bool>,
    pub comment: Option<String>,
}

fn synthesize_jdbc_url(
    db_type: &DbEngineType,
    host: Option<&str>,
    port: Option<u16>,
    database: Option<&str>,
) -> String {
    let h = host.filter(|s| !s.trim().is_empty()).unwrap_or("localhost");
    let p = port.or_else(|| db_type.default_port());
    let port_part = p.map(|n| format!(":{n}")).unwrap_or_default();
    let db = database.map(str::trim).unwrap_or_default();
    let db_slash = if db.is_empty() {
        String::new()
    } else {
        format!("/{db}")
    };

    match db_type {
        DbEngineType::Sqlite => format!("jdbc:sqlite:{db}"),
        DbEngineType::Oracle => format!("jdbc:oracle:thin:@//{h}{port_part}{db_slash}"),
        DbEngineType::SqlServer => {
            if db.is_empty() {
                format!("jdbc:sqlserver://{h}{port_part}")
            } else {
                format!("jdbc:sqlserver://{h}{port_part};databaseName={db}")
            }
        }
        DbEngineType::Dameng => format!("jdbc:dm://{h}{port_part}{db_slash}"),
        DbEngineType::Kingbase => format!("jdbc:kingbase8://{h}{port_part}{db_slash}"),
        DbEngineType::OpenGauss => format!("jdbc:opengauss://{h}{port_part}{db_slash}"),
        DbEngineType::PostgreSql => format!("jdbc:postgresql://{h}{port_part}{db_slash}"),
        DbEngineType::MariaDb => format!("jdbc:mariadb://{h}{port_part}{db_slash}"),
        DbEngineType::TiDb => format!("jdbc:tidb://{h}{port_part}{db_slash}"),
        DbEngineType::OceanBase => format!("jdbc:oceanbase://{h}{port_part}{db_slash}"),
        DbEngineType::ClickHouse => format!("jdbc:clickhouse://{h}{port_part}{db_slash}"),
        _ => format!("jdbc:mysql://{h}{port_part}{db_slash}"),
    }
}

#[tauri::command]
pub fn list_connections(state: State<'_, AppState>) -> Result<Vec<ConnectionProfile>, String> {
    state
        .connections
        .read()
        .map(|g| g.clone())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_connection(
    state: State<'_, AppState>,
    draft: ConnectionDraft,
    password: Option<String>,
) -> Result<ConnectionProfile, String> {
    let id = draft
        .id
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let read_only = draft.read_only.unwrap_or(false);
    let env = draft.env.unwrap_or(EnvLevel::Dev);
    let mut db_type = draft.db_type;
    let mut host = draft.host.filter(|s| !s.trim().is_empty());
    let mut port = draft.port.or_else(|| db_type.default_port());
    let mut database = draft.database.filter(|s| !s.trim().is_empty());

    let jdbc_url = match draft.jdbc_url.filter(|s| !s.trim().is_empty()) {
        Some(url) => {
            if let Ok(parsed) = parse_jdbc_url(&url, None, None) {
                if !matches!(parsed.db_type, DbEngineType::Unknown(_)) {
                    db_type = parsed.db_type;
                }
                if host.is_none() {
                    host = parsed.host;
                }
                if draft.port.is_none() {
                    port = parsed.port;
                }
                if database.is_none() {
                    database = parsed.database;
                }
            }
            url
        }
        None => synthesize_jdbc_url(
            &db_type,
            host.as_deref(),
            port,
            database.as_deref(),
        ),
    };

    let fingerprint = ConnectionFingerprint {
        db_type: db_type.clone(),
        host: host.clone(),
        port,
        database: database.clone(),
    };

    let direct = DirectConfig {
        db_type: db_type.clone(),
        host,
        port,
        database,
        jdbc_url,
        driver_ref: None,
        driver_class: None,
        username: draft.username.filter(|s| !s.trim().is_empty()),
        driver_properties: BTreeMap::new(),
        ssh: None,
        ssl: None,
        read_only,
        schema_pattern: None,
    };

    let profile = ConnectionProfile {
        id: id.clone(),
        name: draft.name.trim().to_string(),
        source_uuid: None,
        db_type,
        env,
        color: None,
        comment: draft.comment.filter(|s| !s.trim().is_empty()),
        tags: if read_only {
            vec!["READ_ONLY".to_string()]
        } else {
            Vec::new()
        },
        resource_groups: Vec::new(),
        capabilities: CapabilityFlags::direct_full(read_only),
        mode: ConnectionMode::Direct(direct),
        fingerprint,
    };

    {
        let mut guard = state.connections.write().map_err(|e| e.to_string())?;
        if let Some(existing) = guard.iter_mut().find(|p| p.id == id) {
            let preserved_uuid = existing.source_uuid.clone();
            *existing = profile.clone();
            existing.source_uuid = preserved_uuid;
        } else {
            guard.push(profile.clone());
        }
    }

    if let Some(pwd) = password {
        if let Ok(mut pguard) = state.passwords.write() {
            pguard.insert(id.clone(), pwd);
        }
    }

    state.active_backends.write().await.remove(&id);
    state.persist_connections()?;
    Ok(profile)
}

#[tauri::command]
pub async fn delete_connection(
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<(), String> {
    {
        let mut guard = state.connections.write().map_err(|e| e.to_string())?;
        guard.retain(|p| p.id != profile_id);
    }
    if let Ok(mut pguard) = state.passwords.write() {
        pguard.remove(&profile_id);
    }
    state.active_backends.write().await.remove(&profile_id);
    state.persist_connections()?;
    Ok(())
}

#[tauri::command]
pub async fn test_connection_cmd(
    state: State<'_, AppState>,
    profile_id: String,
    password: Option<String>,
) -> Result<PingMetrics, String> {
    let (_, backend) = state.resolve_backend(&profile_id, password).await?;
    backend.test_connection().await.map_err(map_err)
}

#[tauri::command]
pub async fn create_demo_sqlite_connection(
    state: State<'_, AppState>,
) -> Result<ConnectionProfile, String> {
    let demo_path = state.storage_dir.join("demo_store.sqlite");
    create_demo_sqlite_db(&demo_path).await.map_err(map_err)?;

    let path_str = demo_path.to_string_lossy().replace('\\', "/");
    let demo_id = "demo-sqlite-store".to_string();

    let direct = DirectConfig {
        db_type: DbEngineType::Sqlite,
        host: None,
        port: None,
        database: Some(path_str.clone()),
        jdbc_url: format!("jdbc:sqlite:{path_str}"),
        driver_ref: Some("sqlite.xerial".to_string()),
        driver_class: None,
        username: None,
        driver_properties: BTreeMap::new(),
        ssh: None,
        ssl: None,
        read_only: false,
        schema_pattern: None,
    };

    let profile = ConnectionProfile {
        id: demo_id.clone(),
        name: "Demo SQLite Store".to_string(),
        source_uuid: Some(demo_id.clone()),
        db_type: DbEngineType::Sqlite,
        env: EnvLevel::Dev,
        color: Some("#22C55E".to_string()),
        comment: Some("Sample local SQLite e-commerce database".to_string()),
        tags: vec!["DEMO".to_string()],
        resource_groups: Vec::new(),
        capabilities: CapabilityFlags::direct_full(false),
        mode: ConnectionMode::Direct(direct),
        fingerprint: ConnectionFingerprint {
            db_type: DbEngineType::Sqlite,
            host: None,
            port: None,
            database: Some(path_str),
        },
    };

    {
        let mut guard = state.connections.write().map_err(|e| e.to_string())?;
        if let Some(existing) = guard.iter_mut().find(|p| p.id == demo_id) {
            *existing = profile.clone();
        } else {
            guard.push(profile.clone());
        }
    }
    state.active_backends.write().await.remove(&demo_id);
    state.persist_connections()?;
    Ok(profile)
}

#[tauri::command]
pub fn import_datagrip_xml_cmd(
    state: State<'_, AppState>,
    xml: String,
) -> Result<Vec<ConnectionProfile>, String> {
    let parsed = parse_datagrip_xml(&xml).map_err(map_err)?;
    state.merge_profiles(parsed)
}

#[tauri::command]
pub fn import_datagrip_path_cmd(
    state: State<'_, AppState>,
    path: String,
) -> Result<Vec<ConnectionProfile>, String> {
    let parsed = import_from_path(&PathBuf::from(path)).map_err(map_err)?;
    state.merge_profiles(parsed)
}

#[tauri::command]
pub fn export_datagrip_xml_cmd(
    state: State<'_, AppState>,
    profile_ids: Option<Vec<String>>,
) -> Result<String, String> {
    let all = state
        .connections
        .read()
        .map_err(|e| e.to_string())?
        .clone();

    let selected: Vec<ConnectionProfile> = match profile_ids {
        Some(ids) if !ids.is_empty() => all
            .into_iter()
            .filter(|p| ids.iter().any(|id| id == &p.id))
            .collect(),
        _ => all,
    };

    export_to_datagrip_xml(&selected).map_err(map_err)
}

#[tauri::command]
pub async fn list_databases_cmd(
    state: State<'_, AppState>,
    profile_id: String,
    password: Option<String>,
) -> Result<Vec<DatabaseMeta>, String> {
    let (_, backend) = state.resolve_backend(&profile_id, password).await?;
    backend.list_databases().await.map_err(map_err)
}

#[tauri::command]
pub async fn list_tables_cmd(
    state: State<'_, AppState>,
    profile_id: String,
    db_name: String,
) -> Result<Vec<TableMeta>, String> {
    let (_, backend) = state.resolve_backend(&profile_id, None).await?;
    backend.list_tables(&db_name).await.map_err(map_err)
}

#[tauri::command]
pub async fn list_columns_cmd(
    state: State<'_, AppState>,
    profile_id: String,
    db_name: String,
    table_name: String,
) -> Result<Vec<ColumnMeta>, String> {
    let (_, backend) = state.resolve_backend(&profile_id, None).await?;
    backend
        .list_columns(&db_name, &table_name)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn get_table_ddl_cmd(
    state: State<'_, AppState>,
    profile_id: String,
    db_name: String,
    table_name: String,
) -> Result<String, String> {
    let (_, backend) = state.resolve_backend(&profile_id, None).await?;
    backend
        .get_table_ddl(&db_name, &table_name)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn execute_sql_cmd(
    state: State<'_, AppState>,
    profile_id: String,
    db_name: Option<String>,
    sql: String,
    limit: Option<u32>,
    on_chunk: Channel<QueryChunk>,
) -> Result<QuerySummary, String> {
    let (profile, backend) = state.resolve_backend(&profile_id, None).await?;

    match state
        .extensions
        .check_before_execute(&profile, db_name.as_deref(), &sql)
        .await
        .map_err(map_err)?
    {
        PreExecuteDecision::Allow => {}
        PreExecuteDecision::RequireApproval { reason } => {
            return Err(format!("EXTENSION_APPROVAL_REQUIRED: {reason}"));
        }
        PreExecuteDecision::Block { reason } => {
            return Err(format!("EXTENSION_BLOCKED: {reason}"));
        }
    }

    let (tx, mut rx) = mpsc::channel::<QueryChunk>(16);
    let forwarder = tokio::spawn(async move {
        while let Some(chunk) = rx.recv().await {
            if on_chunk.send(chunk).is_err() {
                break;
            }
        }
    });

    let exec_result = backend
        .execute_query(
            QueryReq {
                database: db_name.clone(),
                sql: sql.clone(),
                limit,
            },
            tx,
        )
        .await;

    let _ = forwarder.await;
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    match exec_result {
        Ok(summary) => {
            state.push_history(QueryHistoryEntry {
                id: Uuid::new_v4().to_string(),
                timestamp,
                profile_id: profile.id.clone(),
                profile_name: profile.name.clone(),
                database: db_name.clone(),
                sql: sql.clone(),
                duration_ms: summary.duration_ms,
                row_count: summary.total_rows,
                affected_rows: summary.affected_rows,
                success: summary.success,
                error_message: summary.error_message.clone(),
            });
            state
                .extensions
                .emit_audit(&AuditEvent {
                    connection_id: profile.id,
                    connection_name: profile.name,
                    database: db_name,
                    sql,
                    duration_ms: summary.duration_ms,
                    rows: summary.total_rows,
                    affected_rows: summary.affected_rows,
                    success: summary.success,
                    error_message: summary.error_message.clone(),
                })
                .await;
            Ok(summary)
        }
        Err(err) => {
            let msg = err.to_string();
            state.push_history(QueryHistoryEntry {
                id: Uuid::new_v4().to_string(),
                timestamp,
                profile_id: profile.id.clone(),
                profile_name: profile.name.clone(),
                database: db_name.clone(),
                sql: sql.clone(),
                duration_ms: 0,
                row_count: 0,
                affected_rows: None,
                success: false,
                error_message: Some(msg.clone()),
            });
            state
                .extensions
                .emit_audit(&AuditEvent {
                    connection_id: profile.id,
                    connection_name: profile.name,
                    database: db_name,
                    sql,
                    duration_ms: 0,
                    rows: 0,
                    affected_rows: None,
                    success: false,
                    error_message: Some(msg.clone()),
                })
                .await;
            Err(msg)
        }
    }
}

#[tauri::command]
pub async fn explain_sql_cmd(
    state: State<'_, AppState>,
    profile_id: String,
    db_name: String,
    sql: String,
) -> Result<ExplainResult, String> {
    let (_, backend) = state.resolve_backend(&profile_id, None).await?;
    backend.explain(&db_name, &sql).await.map_err(map_err)
}

#[tauri::command]
pub fn analyze_sql_cmd(sql: String, db_type: DbEngineType) -> ScriptAnalysis {
    analyze_sql(&sql, &db_type)
}

#[tauri::command]
pub fn list_query_history_cmd(state: State<'_, AppState>) -> Result<Vec<QueryHistoryEntry>, String> {
    state
        .history
        .read()
        .map(|g| g.clone())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_extensions_cmd(state: State<'_, AppState>) -> Vec<ExtensionMeta> {
    state.extensions.list_extensions()
}

#[tauri::command]
pub async fn list_indexes_cmd(
    state: State<'_, AppState>,
    profile_id: String,
    db_name: String,
    table_name: String,
) -> Result<Vec<IndexMeta>, String> {
    let (_, backend) = state.resolve_backend(&profile_id, None).await?;
    backend
        .list_indexes(&db_name, &table_name)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub fn format_sql_cmd(sql: String) -> String {
    format_sql(&sql)
}

#[tauri::command]
pub fn extract_statement_at_cursor_cmd(sql: String, cursor_offset: usize) -> Option<String> {
    extract_statement_at_cursor(&sql, cursor_offset)
}

#[tauri::command]
pub fn extract_markdown_sql_blocks_cmd(content: String) -> Vec<MarkdownSqlBlock> {
    extract_markdown_sql_blocks(&content)
}

#[tauri::command]
pub fn open_text_file_cmd(
    path: String,
    encoding_override: Option<String>,
) -> Result<OpenedFileDto, String> {
    open_text_file(Path::new(&path), encoding_override.as_deref()).map_err(map_err)
}

#[tauri::command]
pub fn save_text_file_cmd(
    path: String,
    content: String,
    encoding: String,
    line_ending: String,
    with_bom: bool,
) -> Result<FileStatsDto, String> {
    save_text_file(Path::new(&path), &content, &encoding, &line_ending, with_bom).map_err(map_err)
}

#[tauri::command]
pub fn list_workspace_files_cmd(dir_path: String) -> Result<Vec<WorkspaceFileEntry>, String> {
    list_workspace_files(Path::new(&dir_path)).map_err(map_err)
}

#[tauri::command]
pub fn create_demo_markdown_file_cmd(state: State<'_, AppState>) -> Result<OpenedFileDto, String> {
    ensure_demo_markdown_file(&state.storage_dir).map_err(map_err)
}
