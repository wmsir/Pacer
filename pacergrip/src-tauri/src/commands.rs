use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use aes::Aes128;
use cbc::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use chrono::Local;
use serde::Deserialize;
use tauri::ipc::Channel;
use tauri::State;
use tokio::sync::{mpsc, RwLock as AsyncRwLock};
use uuid::Uuid;

use pacer_core::{
    AuditEvent, CapabilityFlags, ColumnMeta, ConnectionFingerprint, ConnectionMode,
    ConnectionProfile, CoreError, DatabaseMeta, DbBackend, DbEngineType, DbeaverDiscoveryItem,
    DirectConfig, EnvLevel, ExplainResult, ExtensionMeta, ExtensionRegistry, FileStatsDto,
    IndexMeta, MarkdownSqlBlock, OpenedFileDto, PingMetrics, PreExecuteDecision, QueryChunk,
    QueryHistoryEntry, QueryReq, QuerySummary, SshTunnelConfig, SslConfig, TableMeta,
    WorkspaceFileEntry,
};
use pacer_dbeaver_io::{
    build_jdbc_url, discover_local_dbeaver_files, export_to_dbeaver_json,
    import_dbeaver_from_path, parse_connection_url, parse_dbeaver_text, ImportedDbeaverConnection,
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
const LOCAL_VAULT_KEY: [u8; 16] = [
    0x50, 0x61, 0x63, 0x65, 0x72, 0x47, 0x72, 0x69, 0x70, 0x56, 0x61, 0x75, 0x6C, 0x74, 0x32, 0x36,
];

type Aes128CbcEnc = cbc::Encryptor<Aes128>;
type Aes128CbcDec = cbc::Decryptor<Aes128>;

pub struct AppState {
    pub connections: RwLock<Vec<ConnectionProfile>>,
    /// Active session + persisted passwords in memory.
    pub passwords: RwLock<HashMap<String, String>>,
    /// Subset of passwords persisted to disk (`Save: Forever`).
    pub persisted_passwords: RwLock<HashMap<String, String>>,
    pub active_backends: AsyncRwLock<HashMap<String, Arc<dyn DbBackend>>>,
    pub history: RwLock<Vec<QueryHistoryEntry>>,
    pub extensions: ExtensionRegistry,
    pub storage_dir: PathBuf,
}

impl AppState {
    pub fn new(storage_dir: PathBuf) -> Self {
        let mut connections = load_connections_from_disk(&storage_dir);
        let persisted = load_vault_passwords(&storage_dir);
        for profile in &mut connections {
            if let ConnectionMode::Direct(ref mut d)
            | ConnectionMode::Hybrid {
                direct: ref mut d, ..
            } = profile.mode
            {
                d.has_saved_password = persisted.contains_key(&profile.id);
            }
        }
        Self {
            connections: RwLock::new(connections),
            passwords: RwLock::new(persisted.clone()),
            persisted_passwords: RwLock::new(persisted),
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

    fn persist_vault(&self) -> Result<(), String> {
        let snapshot = self
            .persisted_passwords
            .read()
            .map_err(|e| e.to_string())?
            .clone();
        save_vault_passwords(&self.storage_dir, &snapshot)
    }

    fn get_profile(&self, profile_id: &str) -> Result<ConnectionProfile, String> {
        let guard = self.connections.read().map_err(|e| e.to_string())?;
        guard
            .iter()
            .find(|p| p.id == profile_id)
            .cloned()
            .ok_or_else(|| format!("connection profile not found: {profile_id}"))
    }

    async fn merge_imported_dbeaver(
        &self,
        incoming: Vec<ImportedDbeaverConnection>,
    ) -> Result<Vec<ConnectionProfile>, String> {
        let mut profiles = Vec::with_capacity(incoming.len());
        let mut changed_vault = false;

        {
            let mut guard = self.connections.write().map_err(|e| e.to_string())?;
            let mut pwd_guard = self.passwords.write().map_err(|e| e.to_string())?;
            let mut vault_guard = self
                .persisted_passwords
                .write()
                .map_err(|e| e.to_string())?;

            for mut item in incoming {
                let target_id = if let Some(existing) = guard.iter().find(|p| {
                    p.id == item.profile.id
                        || (p.source_uuid.is_some()
                            && p.source_uuid == item.profile.source_uuid)
                }) {
                    existing.id.clone()
                } else {
                    item.profile.id.clone()
                };
                item.profile.id = target_id.clone();

                if let Some(pwd) = item.password {
                    pwd_guard.insert(target_id.clone(), pwd.clone());
                    if item.save_password {
                        vault_guard.insert(target_id.clone(), pwd);
                        changed_vault = true;
                    }
                    if let ConnectionMode::Direct(ref mut d)
                    | ConnectionMode::Hybrid {
                        direct: ref mut d, ..
                    } = item.profile.mode
                    {
                        d.has_saved_password = true;
                    }
                } else if pwd_guard.contains_key(&target_id) {
                    if let ConnectionMode::Direct(ref mut d)
                    | ConnectionMode::Hybrid {
                        direct: ref mut d, ..
                    } = item.profile.mode
                    {
                        d.has_saved_password = true;
                    }
                }

                if let Some(existing) = guard.iter_mut().find(|p| p.id == target_id) {
                    *existing = item.profile.clone();
                } else {
                    guard.push(item.profile.clone());
                }
                profiles.push(item.profile);
            }
        }

        {
            let mut backends = self.active_backends.write().await;
            for p in &profiles {
                backends.remove(&p.id);
            }
        }

        self.persist_connections()?;
        if changed_vault {
            self.persist_vault()?;
        }
        Ok(profiles)
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

        let effective_pwd = if direct.auth_type.as_deref() == Some("noAuth") {
            None
        } else {
            stored_pwd.as_deref()
        };

        let backend = connect_direct(&direct, effective_pwd)
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

fn load_vault_passwords(storage_dir: &Path) -> HashMap<String, String> {
    let path = storage_dir.join("credentials.dat");
    let Ok(raw) = std::fs::read(path) else {
        return HashMap::new();
    };
    if raw.len() <= 16 {
        return HashMap::new();
    }
    let (iv, ct) = raw.split_at(16);
    let mut buf = ct.to_vec();
    let Ok(dec) = Aes128CbcDec::new_from_slices(&LOCAL_VAULT_KEY, iv) else {
        return HashMap::new();
    };
    let Ok(pt) = dec.decrypt_padded_mut::<Pkcs7>(&mut buf) else {
        return HashMap::new();
    };
    serde_json::from_slice::<HashMap<String, String>>(pt).unwrap_or_default()
}

fn save_vault_passwords(
    storage_dir: &Path,
    map: &HashMap<String, String>,
) -> Result<(), String> {
    std::fs::create_dir_all(storage_dir).map_err(|e| e.to_string())?;
    let path = storage_dir.join("credentials.dat");
    if map.is_empty() {
        let _ = std::fs::remove_file(path);
        return Ok(());
    }
    let pt = serde_json::to_vec(map).map_err(|e| e.to_string())?;
    let iv = *Uuid::new_v4().as_bytes();
    let mut buf = vec![0u8; pt.len() + 16];
    buf[..pt.len()].copy_from_slice(&pt);
    let enc = Aes128CbcEnc::new_from_slices(&LOCAL_VAULT_KEY, &iv).map_err(|e| e.to_string())?;
    let ct = enc
        .encrypt_padded_mut::<Pkcs7>(&mut buf, pt.len())
        .map_err(|_| "vault encrypt error".to_string())?;

    let mut out = Vec::with_capacity(16 + ct.len());
    out.extend_from_slice(&iv);
    out.extend_from_slice(ct);
    std::fs::write(path, out).map_err(|e| e.to_string())
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
    pub color: Option<String>,
    pub read_only: Option<bool>,
    pub comment: Option<String>,
    pub connection_type: Option<String>,
    pub auth_type: Option<String>,
    pub password_save_mode: Option<String>,
    pub socket_path: Option<String>,
    pub driver_properties: Option<BTreeMap<String, String>>,
    pub ssl_mode: Option<String>,
    pub ssh: Option<SshTunnelConfig>,
    pub schema_pattern: Option<String>,
}

/// Resolves a `DirectConfig` + optional URL-embedded password from a `ConnectionDraft`.
/// Explicit `jdbc_url` takes precedence ("Overrides settings above") while preserving compatible
/// specific drivers (e.g. `TiDB` / `OceanBase` / `MariaDB` over `jdbc:mysql://...`).
fn resolve_direct_from_draft(
    draft: &ConnectionDraft,
) -> (DbEngineType, DirectConfig, Option<String>) {
    let read_only = draft.read_only.unwrap_or(false);
    let mut db_type = draft.db_type.clone();
    let mut host = draft.host.clone().filter(|s| !s.trim().is_empty());
    let mut port = draft.port.or_else(|| db_type.default_port());
    let mut database = draft.database.clone().filter(|s| !s.trim().is_empty());
    let mut username = draft.username.clone().filter(|s| !s.trim().is_empty());
    let mut url_password: Option<String> = None;
    let mut socket_path = draft.socket_path.clone().filter(|s| !s.trim().is_empty());
    let mut driver_properties = draft.driver_properties.clone().unwrap_or_default();

    let jdbc_url = match draft.jdbc_url.clone().filter(|s| !s.trim().is_empty()) {
        Some(url) => {
            let driver_hint = db_type.as_pacer_db_type();
            if let Ok(parsed) = parse_connection_url(&url, Some(driver_hint), None) {
                if !matches!(parsed.db_type, DbEngineType::Unknown(_)) {
                    db_type = parsed.db_type;
                }
                // URL overrides settings above (DataGrip / DBeaver standard behavior)
                if parsed.host.is_some() {
                    host = parsed.host;
                }
                if parsed.port.is_some() {
                    port = parsed.port;
                }
                // Preserve explicit database in URL (or allow empty DB if URL has none)
                database = parsed.database;
                if username.is_none() {
                    username = parsed.username;
                }
                if parsed.password.is_some() {
                    url_password = parsed.password;
                }
                if socket_path.is_none() {
                    socket_path = parsed.socket_path;
                }
                for (k, v) in parsed.params {
                    driver_properties.entry(k).or_insert(v);
                }
            }
            url
        }
        None => build_jdbc_url(
            &db_type,
            host.as_deref(),
            port,
            database.as_deref(),
            &driver_properties,
            socket_path.as_deref(),
        ),
    };

    let ssl = draft
        .ssl_mode
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|mode| SslConfig {
            enabled: !mode.eq_ignore_ascii_case("disabled") && !mode.eq_ignore_ascii_case("disable"),
            mode: Some(mode.to_string()),
            ..Default::default()
        });

    let direct = DirectConfig {
        db_type: db_type.clone(),
        host,
        port,
        database,
        jdbc_url,
        driver_ref: None,
        driver_class: None,
        username,
        driver_properties,
        ssh: draft.ssh.clone(),
        ssl,
        read_only,
        schema_pattern: draft.schema_pattern.clone().filter(|s| !s.trim().is_empty()),
        connection_type: draft
            .connection_type
            .clone()
            .or_else(|| Some("default".to_string())),
        auth_type: draft
            .auth_type
            .clone()
            .or_else(|| Some("userPassword".to_string())),
        password_save_mode: draft
            .password_save_mode
            .clone()
            .or_else(|| Some("forever".to_string())),
        socket_path,
        has_saved_password: false,
    };

    (db_type, direct, url_password)
}

#[tauri::command]
pub fn list_connections(state: State<'_, AppState>) -> Result<Vec<ConnectionProfile>, String> {
    let mut list = state
        .connections
        .read()
        .map(|g| g.clone())
        .map_err(|e| e.to_string())?;
    let pwds = state.passwords.read().map_err(|e| e.to_string())?;
    for profile in &mut list {
        if let ConnectionMode::Direct(ref mut d)
        | ConnectionMode::Hybrid {
            direct: ref mut d, ..
        } = profile.mode
        {
            d.has_saved_password = pwds.contains_key(&profile.id);
        }
    }
    Ok(list)
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
    let (db_type, mut direct, url_password) = resolve_direct_from_draft(&draft);
    let effective_input_pwd = password.or(url_password);
    let save_mode = direct
        .password_save_mode
        .as_deref()
        .unwrap_or("forever")
        .to_string();

    // Update password caches / encrypted vault according to `Save: Forever / Until restart / Never`
    let has_saved_password = {
        let mut pwd_guard = state.passwords.write().map_err(|e| e.to_string())?;
        let mut vault_guard = state
            .persisted_passwords
            .write()
            .map_err(|e| e.to_string())?;

        if direct.auth_type.as_deref() == Some("noAuth") || save_mode == "never" {
            pwd_guard.remove(&id);
            vault_guard.remove(&id);
            false
        } else {
            let final_pwd = effective_input_pwd.or_else(|| pwd_guard.get(&id).cloned());
            match final_pwd {
                Some(p) => {
                    pwd_guard.insert(id.clone(), p.clone());
                    if save_mode == "forever" {
                        vault_guard.insert(id.clone(), p);
                    } else {
                        vault_guard.remove(&id);
                    }
                    true
                }
                None => false,
            }
        }
    };
    direct.has_saved_password = has_saved_password;

    let fingerprint = ConnectionFingerprint {
        db_type: db_type.clone(),
        host: direct.host.clone(),
        port: direct.port,
        database: direct.database.clone(),
    };

    let profile = ConnectionProfile {
        id: id.clone(),
        name: draft.name.trim().to_string(),
        source_uuid: None,
        db_type,
        env,
        color: draft.color.filter(|s| !s.trim().is_empty()),
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

    state.active_backends.write().await.remove(&id);
    state.persist_connections()?;
    state.persist_vault()?;
    Ok(profile)
}

#[tauri::command]
pub async fn test_connection_draft_cmd(
    state: State<'_, AppState>,
    draft: ConnectionDraft,
    password: Option<String>,
) -> Result<PingMetrics, String> {
    let (_, direct, url_password) = resolve_direct_from_draft(&draft);
    let effective_pwd = if direct.auth_type.as_deref() == Some("noAuth") {
        None
    } else if let Some(p) = password.or(url_password) {
        Some(p)
    } else if let Some(ref existing_id) = draft.id {
        state
            .passwords
            .read()
            .map_err(|e| e.to_string())?
            .get(existing_id)
            .cloned()
    } else {
        None
    };

    let backend = connect_direct(&direct, effective_pwd.as_deref())
        .await
        .map_err(map_err)?;
    backend.test_connection().await.map_err(map_err)
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
    if let Ok(mut vguard) = state.persisted_passwords.write() {
        vguard.remove(&profile_id);
    }
    state.active_backends.write().await.remove(&profile_id);
    state.persist_connections()?;
    state.persist_vault()?;
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
        driver_ref: Some("sqlite_jdbc".to_string()),
        driver_class: None,
        username: None,
        driver_properties: BTreeMap::new(),
        ssh: None,
        ssl: None,
        read_only: false,
        schema_pattern: None,
        connection_type: Some("default".to_string()),
        auth_type: Some("noAuth".to_string()),
        password_save_mode: Some("session".to_string()),
        socket_path: None,
        has_saved_password: false,
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
pub fn discover_dbeaver_cmd() -> Vec<DbeaverDiscoveryItem> {
    discover_local_dbeaver_files()
}

#[tauri::command]
pub async fn import_dbeaver_path_cmd(
    state: State<'_, AppState>,
    path: String,
    include_passwords: Option<bool>,
) -> Result<Vec<ConnectionProfile>, String> {
    let parsed = import_dbeaver_from_path(
        Path::new(&path),
        include_passwords.unwrap_or(true),
    )
    .map_err(map_err)?;
    state.merge_imported_dbeaver(parsed).await
}

#[tauri::command]
pub async fn import_dbeaver_text_cmd(
    state: State<'_, AppState>,
    text: String,
) -> Result<Vec<ConnectionProfile>, String> {
    let parsed = parse_dbeaver_text(&text, None).map_err(map_err)?;
    state.merge_imported_dbeaver(parsed).await
}

#[tauri::command]
pub fn export_dbeaver_json_cmd(
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

    export_to_dbeaver_json(&selected).map_err(map_err)
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
