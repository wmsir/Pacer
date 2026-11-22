use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use aes::Aes128;
use cbc::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use chrono::Local;
use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use tauri::State;
use tokio::sync::{mpsc, RwLock as AsyncRwLock};
use uuid::Uuid;

use pacer_core::{
    AuditEvent, CapabilityFlags, ColumnMeta, ConnectionFingerprint, ConnectionMode,
    ConnectionOptionsConfig, ConnectionProfile, CoreError, DatabaseMeta, DbBackend, DbEngineType,
    DbeaverDiscoveryItem, DirectConfig, EnvLevel, ExplainResult, ExtensionMeta, ExtensionRegistry,
    FileStatsDto, IndexMeta, MarkdownSqlBlock, OpenedFileDto, PingMetrics, PreExecuteDecision,
    QueryChunk, QueryHistoryEntry, QueryReq, QuerySummary, SshTunnelConfig, SslConfig, TableMeta,
    WorkspaceFileEntry, WorkspaceProject,
};
use pacer_dbeaver_io::{
    build_jdbc_url, discover_local_dbeaver_files, export_to_dbeaver_dbp_bytes,
    export_to_dbeaver_folder, export_to_dbeaver_json_with_folders, import_dbeaver_from_path,
    parse_connection_url, parse_dbeaver_text, write_xlsx_file, ImportedDbeaverConnection,
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
const DEFAULT_PROJECT_ID: &str = "default-project";
const LOCAL_VAULT_KEY: [u8; 16] = [
    0x50, 0x61, 0x63, 0x65, 0x72, 0x47, 0x72, 0x69, 0x70, 0x56, 0x61, 0x75, 0x6C, 0x74, 0x32, 0x36,
];

type Aes128CbcEnc = cbc::Encryptor<Aes128>;
type Aes128CbcDec = cbc::Decryptor<Aes128>;

pub struct AppState {
    pub projects: RwLock<Vec<WorkspaceProject>>,
    pub connections: RwLock<Vec<ConnectionProfile>>,
    /// Active session + persisted passwords in memory.
    pub passwords: RwLock<HashMap<String, String>>,
    /// Subset of passwords persisted to disk (`Save: Forever`).
    pub persisted_passwords: RwLock<HashMap<String, String>>,
    pub active_backends: AsyncRwLock<HashMap<String, Arc<dyn DbBackend>>>,
    pub history: RwLock<Vec<QueryHistoryEntry>>,
    pub extensions: ExtensionRegistry,
    pub storage_dir: PathBuf,
    pub is_portable: bool,
}

impl AppState {
    pub fn new(storage_dir: PathBuf, is_portable: bool) -> Self {
        let mut projects = load_projects_from_disk(&storage_dir);
        if projects.is_empty() {
            projects.push(default_workspace_project());
        }
        let valid_project_ids: Vec<String> = projects.iter().map(|p| p.id.clone()).collect();
        let fallback_project_id = valid_project_ids[0].clone();

        let mut connections = load_connections_from_disk(&storage_dir);
        let persisted = load_vault_passwords(&storage_dir);
        for profile in &mut connections {
            let needs_project = match profile.project_id.as_deref() {
                Some(pid) => !valid_project_ids.iter().any(|id| id == pid),
                None => true,
            };
            if needs_project {
                profile.project_id = Some(fallback_project_id.clone());
            }
            if let ConnectionMode::Direct(ref mut d)
            | ConnectionMode::Hybrid {
                direct: ref mut d, ..
            } = profile.mode
            {
                d.has_saved_password = persisted.contains_key(&profile.id);
            }
        }
        let history = load_history_from_disk(&storage_dir);
        let _ = std::fs::create_dir_all(storage_dir.join("consoles"));
        let _ = std::fs::create_dir_all(storage_dir.join("plugins"));
        Self {
            projects: RwLock::new(projects),
            connections: RwLock::new(connections),
            passwords: RwLock::new(persisted.clone()),
            persisted_passwords: RwLock::new(persisted),
            active_backends: AsyncRwLock::new(HashMap::new()),
            history: RwLock::new(history),
            extensions: ExtensionRegistry::new(),
            storage_dir,
            is_portable,
        }
    }

    fn persist_projects(&self) -> Result<(), String> {
        let snapshot = self
            .projects
            .read()
            .map_err(|e| e.to_string())?
            .clone();
        std::fs::create_dir_all(&self.storage_dir).map_err(|e| e.to_string())?;
        let path = self.storage_dir.join("projects.json");
        let json = serde_json::to_string_pretty(&snapshot).map_err(|e| e.to_string())?;
        std::fs::write(path, json).map_err(|e| e.to_string())?;
        Ok(())
    }

    fn default_project_id(&self) -> String {
        self.projects
            .read()
            .ok()
            .and_then(|g| g.first().map(|p| p.id.clone()))
            .unwrap_or_else(|| DEFAULT_PROJECT_ID.to_string())
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

    fn collect_passwords(&self) -> HashMap<String, String> {
        let mut merged = self
            .persisted_passwords
            .read()
            .map(|g| g.clone())
            .unwrap_or_default();
        if let Ok(session) = self.passwords.read() {
            for (k, v) in session.iter() {
                if !v.is_empty() {
                    merged.insert(k.clone(), v.clone());
                }
            }
        }
        merged
    }

    fn folder_map_by_profile(&self) -> HashMap<String, String> {
        let proj_names: HashMap<String, String> = self
            .projects
            .read()
            .map(|list| list.iter().map(|p| (p.id.clone(), p.name.clone())).collect())
            .unwrap_or_default();
        let mut out = HashMap::new();
        if let Ok(conns) = self.connections.read() {
            for p in conns.iter() {
                if let Some(ref pid) = p.project_id {
                    if let Some(pname) = proj_names.get(pid) {
                        out.insert(p.id.clone(), pname.clone());
                    }
                }
            }
        }
        out
    }

    fn filter_profiles(
        &self,
        profile_ids: Option<Vec<String>>,
    ) -> Result<Vec<ConnectionProfile>, String> {
        let all = self
            .connections
            .read()
            .map_err(|e| e.to_string())?
            .clone();
        Ok(match profile_ids {
            Some(ids) if !ids.is_empty() => all
                .into_iter()
                .filter(|p| ids.iter().any(|id| id == &p.id))
                .collect(),
            _ => all,
        })
    }

    async fn merge_imported_dbeaver(
        &self,
        incoming: Vec<ImportedDbeaverConnection>,
        target_project_id: Option<String>,
    ) -> Result<Vec<ConnectionProfile>, String> {
        let fallback_pid = target_project_id
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| self.default_project_id());
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
                item.profile.project_id = Some(fallback_pid.clone());

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
        let snapshot = if let Ok(mut guard) = self.history.write() {
            guard.insert(0, entry);
            if guard.len() > MAX_HISTORY_ENTRIES {
                guard.truncate(MAX_HISTORY_ENTRIES);
            }
            Some(guard.clone())
        } else {
            None
        };
        if let Some(list) = snapshot {
            let _ = std::fs::create_dir_all(&self.storage_dir);
            if let Ok(json) = serde_json::to_string_pretty(&list) {
                let _ = std::fs::write(self.storage_dir.join("history.json"), json);
            }
        }
    }
}

fn load_history_from_disk(storage_dir: &Path) -> Vec<QueryHistoryEntry> {
    let path = storage_dir.join("history.json");
    let Ok(raw) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    serde_json::from_str::<Vec<QueryHistoryEntry>>(&raw).unwrap_or_default()
}

fn default_workspace_project() -> WorkspaceProject {
    WorkspaceProject {
        id: DEFAULT_PROJECT_ID.to_string(),
        name: "默认项目".to_string(),
        description: Some("默认多数据源工作区项目".to_string()),
        color: Some("#3574F0".to_string()),
        root_dir: None,
        created_at: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    }
}

fn load_projects_from_disk(storage_dir: &Path) -> Vec<WorkspaceProject> {
    let path = storage_dir.join("projects.json");
    let Ok(raw) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    serde_json::from_str::<Vec<WorkspaceProject>>(&raw).unwrap_or_default()
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
pub struct ProjectDraft {
    pub id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
    pub root_dir: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionDraft {
    pub id: Option<String>,
    pub project_id: Option<String>,
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
    pub ssl: Option<SslConfig>,
    pub ssh: Option<SshTunnelConfig>,
    pub schema_pattern: Option<String>,
    pub object_filter: Option<String>,
    pub options: Option<ConnectionOptionsConfig>,
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
                // URL overrides settings above (standard URL binding behavior)
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

    let ssl = if let Some(mut s) = draft.ssl.clone() {
        if s.mode.as_deref().map(str::trim).filter(|v| !v.is_empty()).is_none() {
            s.mode = draft
                .ssl_mode
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(ToString::to_string);
        }
        Some(s)
    } else {
        draft
            .ssl_mode
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|mode| SslConfig {
                enabled: !mode.eq_ignore_ascii_case("disabled")
                    && !mode.eq_ignore_ascii_case("disable"),
                mode: Some(mode.to_string()),
                ..Default::default()
            })
    };

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
        object_filter: draft.object_filter.clone().filter(|s| !s.trim().is_empty()),
        options: draft.options.clone(),
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
pub fn list_projects_cmd(state: State<'_, AppState>) -> Result<Vec<WorkspaceProject>, String> {
    state
        .projects
        .read()
        .map(|g| g.clone())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_project_cmd(
    state: State<'_, AppState>,
    draft: ProjectDraft,
) -> Result<WorkspaceProject, String> {
    let name = draft.name.trim().to_string();
    if name.is_empty() {
        return Err("项目名称不能为空".to_string());
    }
    let id = draft
        .id
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| format!("proj-{}", Uuid::new_v4()));

    let project = WorkspaceProject {
        id: id.clone(),
        name,
        description: draft.description.filter(|s| !s.trim().is_empty()),
        color: draft.color.filter(|s| !s.trim().is_empty()),
        root_dir: draft.root_dir.filter(|s| !s.trim().is_empty()),
        created_at: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    };

    {
        let mut guard = state.projects.write().map_err(|e| e.to_string())?;
        if let Some(existing) = guard.iter_mut().find(|p| p.id == id) {
            let preserved_created = existing.created_at.clone();
            *existing = project.clone();
            existing.created_at = preserved_created;
        } else {
            guard.push(project.clone());
        }
    }
    state.persist_projects()?;
    Ok(project)
}

#[tauri::command]
pub fn delete_project_cmd(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<WorkspaceProject>, String> {
    let fallback_id = {
        let mut guard = state.projects.write().map_err(|e| e.to_string())?;
        if guard.len() <= 1 {
            return Err("至少保留一个项目，无法删除最后一个项目".to_string());
        }
        guard.retain(|p| p.id != project_id);
        guard[0].id.clone()
    };

    // Reassign any connections belonging to the deleted project to the remaining default project.
    {
        let mut cguard = state.connections.write().map_err(|e| e.to_string())?;
        for conn in cguard.iter_mut() {
            if conn.project_id.as_deref() == Some(project_id.as_str()) {
                conn.project_id = Some(fallback_id.clone());
            }
        }
    }

    state.persist_projects()?;
    state.persist_connections()?;
    state
        .projects
        .read()
        .map(|g| g.clone())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn assign_connection_project_cmd(
    state: State<'_, AppState>,
    profile_id: String,
    project_id: String,
) -> Result<ConnectionProfile, String> {
    let updated = {
        let mut guard = state.connections.write().map_err(|e| e.to_string())?;
        let conn = guard
            .iter_mut()
            .find(|p| p.id == profile_id)
            .ok_or_else(|| format!("未找到数据源: {profile_id}"))?;
        conn.project_id = Some(project_id);
        conn.clone()
    };
    state.persist_connections()?;
    Ok(updated)
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

    let resolved_project_id = draft
        .project_id
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| state.default_project_id());

    let profile = ConnectionProfile {
        id: id.clone(),
        name: draft.name.trim().to_string(),
        project_id: Some(resolved_project_id),
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

fn resolve_draft_password(
    state: &State<'_, AppState>,
    draft_id: Option<&str>,
    auth_type: Option<&str>,
    input_password: Option<String>,
) -> Result<Option<String>, String> {
    if auth_type == Some("noAuth") {
        return Ok(None);
    }
    if input_password.is_some() {
        return Ok(input_password);
    }
    if let Some(existing_id) = draft_id {
        return Ok(state
            .passwords
            .read()
            .map_err(|e| e.to_string())?
            .get(existing_id)
            .cloned());
    }
    Ok(None)
}

#[tauri::command]
pub async fn test_connection_draft_cmd(
    state: State<'_, AppState>,
    draft: ConnectionDraft,
    password: Option<String>,
) -> Result<PingMetrics, String> {
    let (_, direct, url_password) = resolve_direct_from_draft(&draft);
    let effective_pwd = resolve_draft_password(
        &state,
        draft.id.as_deref(),
        direct.auth_type.as_deref(),
        password.or(url_password),
    )?;

    let backend = connect_direct(&direct, effective_pwd.as_deref())
        .await
        .map_err(map_err)?;
    backend.test_connection().await.map_err(map_err)
}

#[tauri::command]
pub async fn list_databases_draft_cmd(
    state: State<'_, AppState>,
    draft: ConnectionDraft,
    password: Option<String>,
) -> Result<Vec<DatabaseMeta>, String> {
    let (_, direct, url_password) = resolve_direct_from_draft(&draft);
    let effective_pwd = resolve_draft_password(
        &state,
        draft.id.as_deref(),
        direct.auth_type.as_deref(),
        password.or(url_password),
    )?;

    let backend = connect_direct(&direct, effective_pwd.as_deref())
        .await
        .map_err(map_err)?;
    backend.list_databases().await.map_err(map_err)
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
    project_id: Option<String>,
) -> Result<ConnectionProfile, String> {
    let demo_path = state.storage_dir.join("demo_store.sqlite");
    create_demo_sqlite_db(&demo_path).await.map_err(map_err)?;

    let path_str = demo_path.to_string_lossy().replace('\\', "/");
    let demo_id = "demo-sqlite-store".to_string();
    let target_project_id = project_id
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| state.default_project_id());

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
        object_filter: None,
        options: None,
        connection_type: Some("default".to_string()),
        auth_type: Some("noAuth".to_string()),
        password_save_mode: Some("session".to_string()),
        socket_path: None,
        has_saved_password: false,
    };

    let profile = ConnectionProfile {
        id: demo_id.clone(),
        name: "Demo SQLite Store".to_string(),
        project_id: Some(target_project_id),
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
    project_id: Option<String>,
) -> Result<Vec<ConnectionProfile>, String> {
    let parsed = import_dbeaver_from_path(
        Path::new(&path),
        include_passwords.unwrap_or(true),
    )
    .map_err(map_err)?;
    state.merge_imported_dbeaver(parsed, project_id).await
}

#[tauri::command]
pub async fn import_dbeaver_text_cmd(
    state: State<'_, AppState>,
    text: String,
    project_id: Option<String>,
) -> Result<Vec<ConnectionProfile>, String> {
    let parsed = parse_dbeaver_text(&text, None).map_err(map_err)?;
    state.merge_imported_dbeaver(parsed, project_id).await
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResultDto {
    pub path: String,
    pub count: usize,
    pub included_passwords: bool,
    pub files: Vec<String>,
}

#[tauri::command]
pub fn export_dbeaver_json_cmd(
    state: State<'_, AppState>,
    profile_ids: Option<Vec<String>>,
) -> Result<String, String> {
    let selected = state.filter_profiles(profile_ids)?;
    let folders = state.folder_map_by_profile();
    export_to_dbeaver_json_with_folders(&selected, &folders).map_err(map_err)
}

#[tauri::command]
pub fn export_dbeaver_dbp_cmd(
    state: State<'_, AppState>,
    path: String,
    include_passwords: bool,
    profile_ids: Option<Vec<String>>,
) -> Result<ExportResultDto, String> {
    let selected = state.filter_profiles(profile_ids)?;
    let folders = state.folder_map_by_profile();
    let passwords = if include_passwords {
        state.collect_passwords()
    } else {
        HashMap::new()
    };
    let bytes = export_to_dbeaver_dbp_bytes(
        &selected,
        &passwords,
        &folders,
        include_passwords,
        "PacerExport",
    )
    .map_err(map_err)?;

    let out_path = PathBuf::from(&path);
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&out_path, bytes).map_err(|e| e.to_string())?;

    Ok(ExportResultDto {
        path: path.clone(),
        count: selected.len(),
        included_passwords: include_passwords,
        files: vec![path],
    })
}

#[tauri::command]
pub fn export_dbeaver_folder_cmd(
    state: State<'_, AppState>,
    dir_path: String,
    include_passwords: bool,
    profile_ids: Option<Vec<String>>,
) -> Result<ExportResultDto, String> {
    let selected = state.filter_profiles(profile_ids)?;
    let folders = state.folder_map_by_profile();
    let passwords = if include_passwords {
        state.collect_passwords()
    } else {
        HashMap::new()
    };
    let files = export_to_dbeaver_folder(
        Path::new(&dir_path),
        &selected,
        &passwords,
        &folders,
        include_passwords,
    )
    .map_err(map_err)?;

    Ok(ExportResultDto {
        path: dir_path,
        count: selected.len(),
        included_passwords: include_passwords,
        files,
    })
}

#[tauri::command]
pub fn export_connections_xlsx_cmd(
    state: State<'_, AppState>,
    path: String,
    include_passwords: bool,
    profile_ids: Option<Vec<String>>,
) -> Result<ExportResultDto, String> {
    let selected = state.filter_profiles(profile_ids)?;
    let folders = state.folder_map_by_profile();
    let passwords = if include_passwords {
        state.collect_passwords()
    } else {
        HashMap::new()
    };

    let columns = vec![
        "项目".to_string(),
        "数据源名称".to_string(),
        "引擎类型".to_string(),
        "环境".to_string(),
        "主机".to_string(),
        "端口".to_string(),
        "默认库".to_string(),
        "用户名".to_string(),
        "密码".to_string(),
        "连接 URL".to_string(),
        "只读模式".to_string(),
        "备注".to_string(),
    ];

    let mut rows = Vec::with_capacity(selected.len());
    for p in &selected {
        let mut r = BTreeMap::new();
        let direct = match &p.mode {
            ConnectionMode::Direct(d) | ConnectionMode::Hybrid { direct: d, .. } => Some(d),
            ConnectionMode::PacerManaged { .. } => None,
        };
        let proj = folders.get(&p.id).cloned().unwrap_or_default();
        let host = direct
            .and_then(|d| d.host.clone())
            .or_else(|| p.fingerprint.host.clone())
            .unwrap_or_default();
        let port = direct
            .and_then(|d| d.port)
            .or(p.fingerprint.port)
            .map(|n| serde_json::Value::from(n as i64))
            .unwrap_or(serde_json::Value::String(String::new()));
        let db = direct
            .and_then(|d| d.database.clone())
            .or_else(|| p.fingerprint.database.clone())
            .unwrap_or_default();
        let username = direct
            .and_then(|d| d.username.clone())
            .unwrap_or_default();
        let pwd = if include_passwords {
            passwords.get(&p.id).cloned().unwrap_or_default()
        } else {
            String::new()
        };
        let url = direct.map(|d| d.jdbc_url.clone()).unwrap_or_default();
        let read_only = if direct.is_some_and(|d| d.read_only) {
            "是"
        } else {
            "否"
        };
        let env_str = match p.env {
            EnvLevel::Dev => "DEV",
            EnvLevel::Test => "TEST",
            EnvLevel::Prod => "PROD",
            EnvLevel::Unknown => "UNKNOWN",
        };

        r.insert("项目".to_string(), serde_json::Value::String(proj));
        r.insert("数据源名称".to_string(), serde_json::Value::String(p.name.clone()));
        r.insert(
            "引擎类型".to_string(),
            serde_json::Value::String(p.db_type.as_pacer_db_type().to_string()),
        );
        r.insert("环境".to_string(), serde_json::Value::String(env_str.to_string()));
        r.insert("主机".to_string(), serde_json::Value::String(host));
        r.insert("端口".to_string(), port);
        r.insert("默认库".to_string(), serde_json::Value::String(db));
        r.insert("用户名".to_string(), serde_json::Value::String(username));
        r.insert("密码".to_string(), serde_json::Value::String(pwd));
        r.insert("连接 URL".to_string(), serde_json::Value::String(url));
        r.insert("只读模式".to_string(), serde_json::Value::String(read_only.to_string()));
        r.insert(
            "备注".to_string(),
            serde_json::Value::String(p.comment.clone().unwrap_or_default()),
        );
        rows.push(r);
    }

    let count = write_xlsx_file(Path::new(&path), "数据源账号清单", &columns, &rows)
        .map_err(map_err)?;

    Ok(ExportResultDto {
        path: path.clone(),
        count,
        included_passwords: include_passwords,
        files: vec![path],
    })
}

#[tauri::command]
pub fn export_rows_xlsx_cmd(
    path: String,
    sheet_name: String,
    columns: Vec<String>,
    rows: Vec<BTreeMap<String, serde_json::Value>>,
) -> Result<usize, String> {
    write_xlsx_file(Path::new(&path), &sheet_name, &columns, &rows).map_err(map_err)
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
                project_id: profile.project_id.clone(),
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
                project_id: profile.project_id.clone(),
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

#[tauri::command]
pub fn reveal_in_explorer_cmd(path: String) -> Result<(), String> {
    let target = PathBuf::from(&path);
    if !target.exists() {
        return Err(format!("路径不存在: {}", target.display()));
    }

    #[cfg(target_os = "windows")]
    {
        let win_path = target.to_string_lossy().replace('/', "\\");
        let mut cmd = std::process::Command::new("explorer");
        if target.is_file() {
            cmd.arg(format!("/select,{win_path}"));
        } else {
            cmd.arg(&win_path);
        }
        cmd.spawn()
            .map_err(|e| format!("无法打开资源管理器: {e}"))?;
    }

    #[cfg(target_os = "macos")]
    {
        let mut cmd = std::process::Command::new("open");
        if target.is_file() {
            cmd.arg("-R");
        }
        cmd.arg(&target);
        cmd.spawn().map_err(|e| format!("无法打开 Finder: {e}"))?;
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let dir = if target.is_file() {
            target.parent().unwrap_or(&target)
        } else {
            target.as_path()
        };
        std::process::Command::new("xdg-open")
            .arg(dir)
            .spawn()
            .map_err(|e| format!("无法打开文件管理器: {e}"))?;
    }

    Ok(())
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceInfoDto {
    pub workspace_dir: String,
    pub consoles_dir: String,
    pub plugins_dir: String,
    pub is_portable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginSnippetItem {
    pub title: String,
    #[serde(default)]
    pub category: Option<String>,
    pub sql: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginSqlRule {
    pub code: String,
    #[serde(default)]
    pub level: String,
    pub pattern: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalPluginManifest {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub snippets: Vec<PluginSnippetItem>,
    #[serde(default)]
    pub sql_rules: Vec<PluginSqlRule>,
    #[serde(default)]
    pub manifest_path: String,
}

fn sanitize_console_filename(raw: &str) -> String {
    let base = raw
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("")
        .trim();
    let mut cleaned: String = base
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            _ if c.is_control() => '_',
            _ => c,
        })
        .collect();
    while cleaned.contains("..") {
        cleaned = cleaned.replace("..", "_");
    }
    let cleaned = cleaned.trim_matches('.').trim();
    if cleaned.is_empty() {
        return "console_1.sql".to_string();
    }
    if cleaned.to_ascii_lowercase().ends_with(".sql") {
        cleaned.to_string()
    } else {
        format!("{cleaned}.sql")
    }
}

fn normalize_display_path(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

#[tauri::command]
pub fn get_workspace_info_cmd(state: State<'_, AppState>) -> WorkspaceInfoDto {
    let consoles = state.storage_dir.join("consoles");
    let plugins = state.storage_dir.join("plugins");
    WorkspaceInfoDto {
        workspace_dir: normalize_display_path(&state.storage_dir),
        consoles_dir: normalize_display_path(&consoles),
        plugins_dir: normalize_display_path(&plugins),
        is_portable: state.is_portable,
    }
}

#[tauri::command]
pub fn save_console_scratch_cmd(
    state: State<'_, AppState>,
    file_name: String,
    content: String,
) -> Result<String, String> {
    let consoles_dir = state.storage_dir.join("consoles");
    std::fs::create_dir_all(&consoles_dir).map_err(|e| e.to_string())?;
    let safe_name = sanitize_console_filename(&file_name);
    let target = consoles_dir.join(safe_name);
    std::fs::write(&target, content.as_bytes()).map_err(|e| e.to_string())?;
    Ok(normalize_display_path(&target))
}

fn scan_plugins_dir(plugins_dir: &Path) -> Vec<LocalPluginManifest> {
    if !plugins_dir.is_dir() {
        return Vec::new();
    }
    let mut manifests = Vec::new();
    let Ok(entries) = std::fs::read_dir(plugins_dir) else {
        return manifests;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let candidate = if path.is_dir() {
            path.join("pacer-plugin.json")
        } else if path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.ends_with(".pacer-plugin.json"))
        {
            path
        } else {
            continue;
        };
        if !candidate.is_file() {
            continue;
        }
        let Ok(raw) = std::fs::read_to_string(&candidate) else {
            continue;
        };
        let Ok(mut parsed) = serde_json::from_str::<LocalPluginManifest>(&raw) else {
            continue;
        };
        parsed.manifest_path = normalize_display_path(&candidate);
        manifests.push(parsed);
    }
    manifests.sort_by_key(|a| a.name.to_ascii_lowercase());
    manifests
}

#[tauri::command]
pub fn list_local_plugins_cmd(state: State<'_, AppState>) -> Result<Vec<LocalPluginManifest>, String> {
    let plugins_dir = state.storage_dir.join("plugins");
    let _ = std::fs::create_dir_all(&plugins_dir);
    Ok(scan_plugins_dir(&plugins_dir))
}

#[tauri::command]
pub fn create_sample_plugin_cmd(
    state: State<'_, AppState>,
) -> Result<Vec<LocalPluginManifest>, String> {
    let plugin_dir = state.storage_dir.join("plugins").join("team-sql-pack");
    std::fs::create_dir_all(&plugin_dir).map_err(|e| e.to_string())?;
    let manifest_file = plugin_dir.join("pacer-plugin.json");
    if !manifest_file.is_file() {
        let sample = LocalPluginManifest {
            id: "team-sql-pack".to_string(),
            name: "团队规范与高频 SQL 插件包".to_string(),
            version: "1.0.0".to_string(),
            author: Some("Pacer Workspace".to_string()),
            description: "放在 pacer-workspace/plugins 目录下的 JSON 本地插件示例，无需重编译即可热加载 SQL 模板与团队安全审计规则。".to_string(),
            enabled: true,
            snippets: vec![
                PluginSnippetItem {
                    title: "查看当前库表体积排行".to_string(),
                    category: Some("扩展模板".to_string()),
                    sql: "SELECT table_name, table_rows,\n  ROUND((data_length + index_length) / 1024 / 1024, 2) AS size_mb\nFROM information_schema.tables\nWHERE table_schema = DATABASE()\nORDER BY size_mb DESC\nLIMIT 20;".to_string(),
                },
                PluginSnippetItem {
                    title: "按时间范围统计订单汇总".to_string(),
                    category: Some("扩展模板".to_string()),
                    sql: "SELECT DATE(created_at) AS biz_day, COUNT(*) AS cnt, SUM(total_amount) AS total_amt\nFROM orders\nGROUP BY DATE(created_at)\nORDER BY biz_day DESC;".to_string(),
                },
            ],
            sql_rules: vec![PluginSqlRule {
                code: "TEAM_NO_TRUNCATE".to_string(),
                level: "HIGH".to_string(),
                pattern: "TRUNCATE TABLE".to_string(),
                message: "团队规范插件拦截：禁止直接执行 TRUNCATE TABLE，请先备份数据或提交变更审批。".to_string(),
            }],
            manifest_path: String::new(),
        };
        let json = serde_json::to_string_pretty(&sample).map_err(|e| e.to_string())?;
        std::fs::write(&manifest_file, json).map_err(|e| e.to_string())?;
    }
    Ok(scan_plugins_dir(&state.storage_dir.join("plugins")))
}
