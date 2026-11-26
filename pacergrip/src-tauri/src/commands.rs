use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex, RwLock};
use std::time::{Duration, Instant};

use chrono::Local;
use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use tauri::State;
use tauri_plugin_dialog::DialogExt;
use tokio::sync::{mpsc, oneshot, RwLock as AsyncRwLock};
use uuid::Uuid;
use zeroize::Zeroize;

use pacer_core::files::{
    canonicalize_existing_path, canonicalize_target_path, is_path_within_any_root,
    reject_traversal_components,
};

use pacer_core::{
    load_vault_passwords, save_vault_passwords, AuditEvent, CapabilityFlags, ColumnMeta,
    ConnectionFingerprint, ConnectionMode, ConnectionOptionsConfig, ConnectionProfile, CoreError,
    DatabaseMeta, DbBackend, DbEngineType, DbeaverDiscoveryItem, DirectConfig, EnvLevel,
    ExplainResult, ExtensionMeta, ExtensionRegistry, FileStatsDto, ForeignKeyMeta, IndexMeta,
    MarkdownSqlBlock, ObjectSearchHit, OpenedFileDto, PingMetrics, PreExecuteDecision, QueryChunk,
    QueryExportProgress, QueryExportResult, QueryHistoryEntry, QueryReq, QuerySummary,
    RoutineMeta, SqlScriptProgress, SqlScriptRunResult, SshTunnelConfig, SslConfig, TableMeta,
    TransactionStatus, WorkspaceFileEntry, WorkspaceProject, XlsxSheetPreview,
};
use pacer_dbeaver_io::{
    build_jdbc_url, build_limited_select_sql, build_sql_dump_footer, build_sql_dump_header,
    discover_local_dbeaver_files, export_to_dbeaver_dbp_bytes, export_to_dbeaver_folder,
    export_to_dbeaver_json_with_folders, import_dbeaver_from_path, parse_connection_url,
    parse_dbeaver_text, read_xlsx_sheet_preview, write_csv_row, write_sql_dump_insert_batch,
    write_sql_dump_table_header, write_xlsx_file, ImportedDbeaverConnection, SqlDumpFormatOptions,
    SqlScriptStreamReader, StreamingXlsxWriter,
};
use pacer_drivers::{connect_direct, create_demo_sqlite_db};
use pacer_sql_ast::{
    analyze_sql, extract_markdown_sql_blocks, extract_statement_at_cursor, format_sql,
    ScriptAnalysis, SqlAccessKind,
};

use crate::files::{
    ensure_demo_markdown_file, list_workspace_files, open_text_file, save_text_file,
};

#[cfg(feature = "pacer-extension")]
pub use pacer_client::PacerClient;

const MAX_HISTORY_ENTRIES: usize = 2_000;
const DEFAULT_PROJECT_ID: &str = "default-project";

#[derive(Clone)]
pub struct CachedBackend {
    pub backend: Arc<dyn DbBackend>,
    pub last_used: Arc<StdMutex<Instant>>,
    pub idle_evict_secs: u64,
}

impl CachedBackend {
    fn new(backend: Arc<dyn DbBackend>, idle_evict_secs: u64) -> Self {
        Self {
            backend,
            last_used: Arc::new(StdMutex::new(Instant::now())),
            idle_evict_secs,
        }
    }

    pub fn touch(&self) {
        if let Ok(mut guard) = self.last_used.lock() {
            *guard = Instant::now();
        }
    }

    fn idle_secs(&self) -> u64 {
        self.last_used
            .lock()
            .map(|g| g.elapsed().as_secs())
            .unwrap_or(0)
    }
}

pub struct RunningQueryHandle {
    pub cancel_tx: oneshot::Sender<()>,
    pub backend: Arc<dyn DbBackend>,
}

pub struct AppState {
    pub projects: RwLock<Vec<WorkspaceProject>>,
    pub connections: RwLock<Vec<ConnectionProfile>>,
    /// Active session + persisted passwords in memory.
    pub passwords: RwLock<HashMap<String, String>>,
    /// Subset of passwords persisted to disk (`Save: Forever`).
    pub persisted_passwords: RwLock<HashMap<String, String>>,
    pub active_backends: AsyncRwLock<HashMap<String, CachedBackend>>,
    pub running_queries: AsyncRwLock<HashMap<String, RunningQueryHandle>>,
    pub history: RwLock<Vec<QueryHistoryEntry>>,
    pub history_dirty: Arc<AtomicBool>,
    pub extensions: ExtensionRegistry,
    pub storage_dir: PathBuf,
    pub is_portable: bool,
    pub approved_read_paths: RwLock<HashSet<PathBuf>>,
    pub approved_write_paths: RwLock<HashSet<PathBuf>>,
    pub approved_dirs: RwLock<HashSet<PathBuf>>,
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
        let persisted = load_vault_passwords(&storage_dir, is_portable);
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
        let _ = std::fs::create_dir_all(storage_dir.join("docs"));
        let _ = std::fs::create_dir_all(storage_dir.join("exports"));

        let mut initial_dirs = HashSet::new();
        for proj in &projects {
            if let Some(ref rd) = proj.root_dir {
                if let Ok(canon) = canonicalize_existing_path(Path::new(rd)) {
                    if canon.is_dir() {
                        initial_dirs.insert(canon);
                    }
                }
            }
        }

        Self {
            projects: RwLock::new(projects),
            connections: RwLock::new(connections),
            passwords: RwLock::new(persisted.clone()),
            persisted_passwords: RwLock::new(persisted),
            active_backends: AsyncRwLock::new(HashMap::new()),
            running_queries: AsyncRwLock::new(HashMap::new()),
            history: RwLock::new(history),
            history_dirty: Arc::new(AtomicBool::new(false)),
            extensions: ExtensionRegistry::new(),
            storage_dir,
            is_portable,
            approved_read_paths: RwLock::new(HashSet::new()),
            approved_write_paths: RwLock::new(HashSet::new()),
            approved_dirs: RwLock::new(initial_dirs),
        }
    }

    fn protected_storage_paths(&self) -> Vec<PathBuf> {
        let protected_names = [
            "credentials.dat",
            "vault.key",
            "vault_v3.key",
            "vault.dpapi",
            "vault_v3.dpapi",
            "connections.json",
            "projects.json",
            "history.json",
            "demo_store.sqlite",
            ".pacer-portable",
            "pacer.portable",
        ];
        let base = canonicalize_existing_path(&self.storage_dir)
            .unwrap_or_else(|_| self.storage_dir.clone());
        protected_names
            .into_iter()
            .map(|name| base.join(name))
            .collect()
    }

    fn is_protected_storage_target(&self, canonical_target: &Path) -> bool {
        if let Ok(base) = canonicalize_existing_path(&self.storage_dir) {
            if canonical_target == base {
                return true;
            }
            if canonical_target.parent() == Some(base.as_path()) {
                if let Some(fname) = canonical_target.file_name().and_then(|n| n.to_str()) {
                    if fname.starts_with('.') || fname.ends_with(".tmp") {
                        return true;
                    }
                }
            }
        }
        let protected = self.protected_storage_paths();
        is_path_within_any_root(canonical_target, &protected)
    }

    fn canonical_writable_dirs(&self) -> Vec<PathBuf> {
        let mut dirs = Vec::new();
        for sub in ["consoles", "docs", "exports"] {
            if let Ok(c) = canonicalize_existing_path(&self.storage_dir.join(sub)) {
                dirs.push(c);
            }
        }
        if let Ok(guard) = self.approved_dirs.read() {
            dirs.extend(guard.iter().cloned());
        }
        dirs
    }

    fn canonical_readable_dirs(&self) -> Vec<PathBuf> {
        let mut dirs = self.canonical_writable_dirs();
        if let Ok(c) = canonicalize_existing_path(&self.storage_dir.join("plugins")) {
            dirs.push(c);
        }
        dirs
    }

    fn dbeaver_discovery_roots() -> Vec<PathBuf> {
        let mut roots = Vec::new();
        if let Ok(appdata) = std::env::var("APPDATA") {
            roots.push(PathBuf::from(&appdata).join("DBeaverData"));
        }
        if let Ok(userprofile) = std::env::var("USERPROFILE") {
            roots.push(PathBuf::from(&userprofile).join(".dbeaver4"));
            roots.push(
                PathBuf::from(&userprofile)
                    .join("AppData")
                    .join("Roaming")
                    .join("DBeaverData"),
            );
        }
        if let Ok(home) = std::env::var("HOME") {
            let h = PathBuf::from(&home);
            roots.push(h.join("Library").join("DBeaverData"));
            roots.push(h.join(".local").join("share").join("DBeaverData"));
            roots.push(h.join(".dbeaver4"));
        }
        roots
            .into_iter()
            .filter_map(|p| canonicalize_existing_path(&p).ok())
            .collect()
    }

    fn validate_read_path(&self, raw: &str) -> Result<PathBuf, String> {
        let canon = canonicalize_existing_path(Path::new(raw)).map_err(map_err)?;
        if self.is_protected_storage_target(&canon) {
            return Err("安全限制：禁止直接读取内部状态或凭据库文件".to_string());
        }
        if self
            .approved_read_paths
            .read()
            .is_ok_and(|g| g.contains(&canon))
            || self
                .approved_write_paths
                .read()
                .is_ok_and(|g| g.contains(&canon))
        {
            return Ok(canon);
        }
        if let Ok(handbook) = canonicalize_existing_path(&self.storage_dir.join("示例查询手册.md"))
        {
            if canon == handbook {
                return Ok(canon);
            }
        }
        let dirs = self.canonical_readable_dirs();
        if is_path_within_any_root(&canon, &dirs) {
            return Ok(canon);
        }
        Err(format!(
            "安全限制：拒绝读取未授权工作区或未通过系统对话框选择的文件 ({raw})"
        ))
    }

    fn validate_write_path(&self, raw: &str) -> Result<PathBuf, String> {
        let canon = canonicalize_target_path(Path::new(raw), false).map_err(map_err)?;
        if self.is_protected_storage_target(&canon) {
            return Err("安全限制：禁止写入或覆盖内部状态或凭据库文件".to_string());
        }
        if let Ok(plugins_dir) = canonicalize_existing_path(&self.storage_dir.join("plugins")) {
            if is_path_within_any_root(&canon, &[plugins_dir]) {
                return Err("安全限制：禁止通过通用文件接口修改本地插件目录".to_string());
            }
        }
        if self
            .approved_write_paths
            .read()
            .is_ok_and(|g| g.contains(&canon))
        {
            return Ok(canon);
        }
        let dirs = self.canonical_writable_dirs();
        if is_path_within_any_root(&canon, &dirs) {
            return Ok(canon);
        }
        Err(format!(
            "安全限制：拒绝写入未授权目录或未通过保存对话框选择的路径 ({raw})"
        ))
    }

    fn validate_dir_path(&self, raw: &str, for_write: bool) -> Result<PathBuf, String> {
        let canon = canonicalize_existing_path(Path::new(raw)).map_err(map_err)?;
        if !canon.is_dir() {
            return Err(format!("目录不存在: {raw}"));
        }
        let dirs = if for_write {
            self.canonical_writable_dirs()
        } else {
            self.canonical_readable_dirs()
        };
        if is_path_within_any_root(&canon, &dirs) {
            return Ok(canon);
        }
        Err(format!(
            "安全限制：拒绝访问未授权工作区之外的目录 ({raw})"
        ))
    }

    fn validate_import_dbeaver_path(&self, raw: &str) -> Result<PathBuf, String> {
        let canon = canonicalize_existing_path(Path::new(raw)).map_err(map_err)?;
        if self.is_protected_storage_target(&canon) {
            return Err("安全限制：禁止从内部凭据存储路径导入".to_string());
        }
        if self
            .approved_read_paths
            .read()
            .is_ok_and(|g| g.contains(&canon))
        {
            return Ok(canon);
        }
        let mut dirs = self.canonical_readable_dirs();
        dirs.extend(Self::dbeaver_discovery_roots());
        if is_path_within_any_root(&canon, &dirs) {
            return Ok(canon);
        }
        Err(format!(
            "安全限制：仅允许从已发现的 DBeaver 工作区、已挂载目录或通过文件选择框选定的路径导入 ({raw})"
        ))
    }

    fn validate_reveal_path(&self, raw: &str) -> Result<PathBuf, String> {
        let canon = canonicalize_existing_path(Path::new(raw)).map_err(map_err)?;
        if let Ok(base) = canonicalize_existing_path(&self.storage_dir) {
            if canon == base {
                return Ok(canon);
            }
        }
        if self.is_protected_storage_target(&canon) {
            return Err("安全限制：禁止直接定位内部凭据或状态文件".to_string());
        }
        if self
            .approved_read_paths
            .read()
            .is_ok_and(|g| g.contains(&canon))
            || self
                .approved_write_paths
                .read()
                .is_ok_and(|g| g.contains(&canon))
        {
            return Ok(canon);
        }
        let dirs = self.canonical_readable_dirs();
        if is_path_within_any_root(&canon, &dirs) {
            return Ok(canon);
        }
        Err(format!(
            "安全限制：拒绝在系统文件管理器中打开未授权路径 ({raw})"
        ))
    }

    fn approve_read_path(&self, canon: PathBuf, allow_write: bool) {
        if let Ok(mut g) = self.approved_read_paths.write() {
            g.insert(canon.clone());
        }
        if allow_write {
            if let Ok(mut g) = self.approved_write_paths.write() {
                g.insert(canon);
            }
        }
    }

    fn approve_write_path(&self, canon: PathBuf) {
        if let Ok(mut g) = self.approved_write_paths.write() {
            g.insert(canon);
        }
    }

    fn approve_dir(&self, canon: PathBuf) {
        if let Ok(mut g) = self.approved_dirs.write() {
            g.insert(canon);
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
        save_vault_passwords(&self.storage_dir, self.is_portable, &snapshot)
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

        let mut transient_override: Option<String> = None;
        if let Some(pwd) = password_override {
            let is_never = matches!(
                &profile.mode,
                ConnectionMode::Direct(d) | ConnectionMode::Hybrid { direct: d, .. }
                    if d.password_save_mode.as_deref() == Some("never")
            );
            if is_never {
                transient_override = Some(pwd);
            } else if let Ok(mut guard) = self.passwords.write() {
                guard.insert(profile_id.to_string(), pwd);
            }
            self.active_backends.write().await.remove(profile_id);
        } else if let Some(cached) = self.active_backends.read().await.get(profile_id).cloned() {
            cached.touch();
            return Ok((profile, Arc::clone(&cached.backend)));
        }

        let stored_pwd = if transient_override.is_some() {
            transient_override.clone()
        } else {
            self.passwords
                .read()
                .map_err(|e| e.to_string())?
                .get(profile_id)
                .cloned()
        };
        if let Some(ref mut tmp) = transient_override {
            tmp.zeroize();
        }

        let mut direct = match &profile.mode {
            ConnectionMode::Direct(d) | ConnectionMode::Hybrid { direct: d, .. } => d.clone(),
            ConnectionMode::PacerManaged { .. } => {
                return Err(
                    "Connection is marked as an external managed profile; enable an extension plugin to execute."
                        .to_string(),
                )
            }
        };

        if let Some(ref mut ssh_cfg) = direct.ssh {
            if let Ok(pwds) = self.passwords.read() {
                if ssh_cfg.password.is_none() {
                    ssh_cfg.password = pwds.get(&format!("ssh_pwd:{profile_id}")).cloned();
                }
                if ssh_cfg.passphrase.is_none() {
                    ssh_cfg.passphrase = pwds.get(&format!("ssh_pass:{profile_id}")).cloned();
                }
            }
        }

        let effective_pwd = if direct.auth_type.as_deref() == Some("noAuth") {
            None
        } else {
            stored_pwd.as_deref()
        };

        let idle_evict_secs = direct
            .options
            .as_ref()
            .filter(|o| o.auto_disconnect_enabled)
            .map(|o| u64::from(o.auto_disconnect_sec.unwrap_or(300).clamp(30, 86_400)))
            .unwrap_or(1800);

        let backend = connect_direct(&direct, effective_pwd)
            .await
            .map_err(map_err)?;
        self.active_backends.write().await.insert(
            profile_id.to_string(),
            CachedBackend::new(Arc::clone(&backend), idle_evict_secs),
        );

        Ok((profile, backend))
    }

    pub async fn evict_idle_backends(&self) -> Vec<String> {
        let mut evicted_ids = Vec::new();
        let mut evicted_backends = Vec::new();
        {
            let mut guard = self.active_backends.write().await;
            let stale_keys: Vec<String> = guard
                .iter()
                .filter_map(|(id, entry)| {
                    if Arc::strong_count(&entry.backend) > 1 {
                        entry.touch();
                        None
                    } else if entry.idle_secs() >= entry.idle_evict_secs {
                        Some(id.clone())
                    } else {
                        None
                    }
                })
                .collect();
            for key in stale_keys {
                if let Some(removed) = guard.remove(&key) {
                    evicted_ids.push(key);
                    evicted_backends.push(removed);
                }
            }
        }
        drop(evicted_backends);
        evicted_ids
    }

    fn push_history(&self, mut entry: QueryHistoryEntry) {
        entry.sql = sanitize_sql_for_history(&entry.sql);
        if let Ok(mut guard) = self.history.write() {
            guard.insert(0, entry);
            while guard.len() > MAX_HISTORY_ENTRIES {
                if let Some(pos) = guard.iter().rposition(|item| !item.pinned) {
                    guard.remove(pos);
                } else {
                    guard.truncate(MAX_HISTORY_ENTRIES);
                    break;
                }
            }
            self.history_dirty.store(true, Ordering::Release);
        }
    }

    pub async fn flush_dirty_history(&self) {
        if !self.history_dirty.swap(false, Ordering::AcqRel) {
            return;
        }
        let snapshot = match self.history.read() {
            Ok(g) => g.clone(),
            Err(_) => return,
        };
        let storage_dir = self.storage_dir.clone();
        let dirty_flag = Arc::clone(&self.history_dirty);
        let _ = tokio::task::spawn_blocking(move || {
            if write_history_snapshot_atomic(&storage_dir, &snapshot).is_err() {
                dirty_flag.store(true, Ordering::Release);
            }
        })
        .await;
    }
}

fn write_history_snapshot_atomic(storage_dir: &Path, list: &[QueryHistoryEntry]) -> Result<(), String> {
    std::fs::create_dir_all(storage_dir).map_err(|e| e.to_string())?;
    let json = serde_json::to_string(list).map_err(|e| e.to_string())?;
    let target = storage_dir.join("history.json");
    let tmp = storage_dir.join(format!(".history.json.{}.tmp", Uuid::new_v4()));
    std::fs::write(&tmp, json.as_bytes()).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
    }
    if let Err(err) = std::fs::rename(&tmp, &target) {
        let _ = std::fs::remove_file(&tmp);
        return Err(err.to_string());
    }
    Ok(())
}

fn redact_uri_passwords(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(scheme_pos) = rest.find("://") {
        let prefix_end = scheme_pos + 3;
        out.push_str(&rest[..prefix_end]);
        rest = &rest[prefix_end..];
        let seg_end = rest
            .find(|c: char| c.is_whitespace() || matches!(c, '\'' | '"' | ';' | ')' | ','))
            .unwrap_or(rest.len());
        let segment = &rest[..seg_end];
        if let Some(at_pos) = segment.rfind('@') {
            let auth = &segment[..at_pos];
            if let Some(colon_pos) = auth.find(':') {
                out.push_str(&auth[..colon_pos + 1]);
                out.push_str("***");
                out.push_str(&segment[at_pos..]);
            } else {
                out.push_str(segment);
            }
        } else {
            out.push_str(segment);
        }
        rest = &rest[seg_end..];
    }
    out.push_str(rest);
    out
}

fn redact_quoted_keyword_secret(sql: &str, keyword: &str) -> String {
    let upper = sql.to_ascii_uppercase();
    let mut out = String::with_capacity(sql.len());
    let mut cursor = 0usize;
    while let Some(rel) = upper[cursor..].find(keyword) {
        let kw_start = cursor + rel;
        let kw_end = kw_start + keyword.len();
        out.push_str(&sql[cursor..kw_end]);
        let mut idx = kw_end;
        let bytes = sql.as_bytes();
        while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
            out.push(bytes[idx] as char);
            idx += 1;
        }
        if idx < bytes.len() && bytes[idx] == b'=' {
            out.push('=');
            idx += 1;
            while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
                out.push(bytes[idx] as char);
                idx += 1;
            }
        }
        if idx < bytes.len() && (bytes[idx] == b'\'' || bytes[idx] == b'"') {
            let quote = bytes[idx];
            out.push(quote as char);
            out.push_str("***");
            idx += 1;
            while idx < bytes.len() {
                if bytes[idx] == b'\\' && idx + 1 < bytes.len() {
                    idx += 2;
                    continue;
                }
                if bytes[idx] == quote {
                    if bytes.get(idx + 1) == Some(&quote) {
                        idx += 2;
                        continue;
                    }
                    out.push(quote as char);
                    idx += 1;
                    break;
                }
                idx += 1;
            }
        }
        cursor = idx;
    }
    out.push_str(&sql[cursor..]);
    out
}

pub(crate) fn sanitize_sql_for_history(sql: &str) -> String {
    let trimmed = sql.trim();
    let redacted_uri = redact_uri_passwords(trimmed);
    let redacted_id = redact_quoted_keyword_secret(&redacted_uri, "IDENTIFIED BY");
    let mut cleaned = redact_quoted_keyword_secret(&redacted_id, "PASSWORD");
    let first_tok = cleaned
        .split_whitespace()
        .next()
        .map(str::to_ascii_uppercase)
        .unwrap_or_default();
    if first_tok == "AUTH" {
        cleaned = "AUTH ***".to_string();
    }
    const MAX_HISTORY_SQL_CHARS: usize = 16_384;
    if let Some((byte_idx, _)) = cleaned.char_indices().nth(MAX_HISTORY_SQL_CHARS) {
        cleaned.truncate(byte_idx);
        cleaned.push_str(" ... [truncated]");
    }
    cleaned
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

    let validated_root_dir = match draft.root_dir.filter(|s| !s.trim().is_empty()) {
        Some(raw_dir) => {
            let canon = state.validate_dir_path(&raw_dir, false)?;
            Some(normalize_display_path(&canon))
        }
        None => None,
    };

    let project = WorkspaceProject {
        id: id.clone(),
        name,
        description: draft.description.filter(|s| !s.trim().is_empty()),
        color: draft.color.filter(|s| !s.trim().is_empty()),
        root_dir: validated_root_dir,
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
            if let Some(mut old) = pwd_guard.remove(&id) {
                old.zeroize();
            }
            if let Some(mut old) = vault_guard.remove(&id) {
                old.zeroize();
            }
            false
        } else {
            let final_pwd = effective_input_pwd.or_else(|| pwd_guard.get(&id).cloned());
            match final_pwd {
                Some(p) => {
                    pwd_guard.insert(id.clone(), p.clone());
                    if save_mode == "forever" {
                        vault_guard.insert(id.clone(), p);
                    } else if let Some(mut old) = vault_guard.remove(&id) {
                        old.zeroize();
                    }
                    true
                }
                None => false,
            }
        }
    };
    direct.has_saved_password = has_saved_password;

    {
        let mut pwd_guard = state.passwords.write().map_err(|e| e.to_string())?;
        let mut vault_guard = state
            .persisted_passwords
            .write()
            .map_err(|e| e.to_string())?;
        let ssh_pwd_key = format!("ssh_pwd:{id}");
        let ssh_pass_key = format!("ssh_pass:{id}");
        let ssh_enabled = direct.ssh.as_ref().is_some_and(|s| s.enabled);

        if save_mode == "never" || !ssh_enabled {
            if let Some(mut old) = pwd_guard.remove(&ssh_pwd_key) {
                old.zeroize();
            }
            if let Some(mut old) = pwd_guard.remove(&ssh_pass_key) {
                old.zeroize();
            }
            if let Some(mut old) = vault_guard.remove(&ssh_pwd_key) {
                old.zeroize();
            }
            if let Some(mut old) = vault_guard.remove(&ssh_pass_key) {
                old.zeroize();
            }
        } else if let Some(ref ssh_cfg) = direct.ssh {
            if save_mode != "forever" {
                if let Some(mut old) = vault_guard.remove(&ssh_pwd_key) {
                    old.zeroize();
                }
                if let Some(mut old) = vault_guard.remove(&ssh_pass_key) {
                    old.zeroize();
                }
            }
            if let Some(ref sp) = ssh_cfg.password {
                if !sp.is_empty() {
                    pwd_guard.insert(ssh_pwd_key.clone(), sp.clone());
                    if save_mode == "forever" {
                        vault_guard.insert(ssh_pwd_key, sp.clone());
                    }
                }
            }
            if let Some(ref kp) = ssh_cfg.passphrase {
                if !kp.is_empty() {
                    pwd_guard.insert(ssh_pass_key.clone(), kp.clone());
                    if save_mode == "forever" {
                        vault_guard.insert(ssh_pass_key, kp.clone());
                    }
                }
            }
        }
    }

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
    let (_, mut direct, url_password) = resolve_direct_from_draft(&draft);
    let effective_pwd = resolve_draft_password(
        &state,
        draft.id.as_deref(),
        direct.auth_type.as_deref(),
        password.or(url_password),
    )?;
    if let (Some(ref mut ssh_cfg), Some(ref existing_id)) = (&mut direct.ssh, &draft.id) {
        if let Ok(pwds) = state.passwords.read() {
            if ssh_cfg.password.is_none() {
                ssh_cfg.password = pwds.get(&format!("ssh_pwd:{existing_id}")).cloned();
            }
            if ssh_cfg.passphrase.is_none() {
                ssh_cfg.passphrase = pwds.get(&format!("ssh_pass:{existing_id}")).cloned();
            }
        }
    }

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
    let (_, mut direct, url_password) = resolve_direct_from_draft(&draft);
    let effective_pwd = resolve_draft_password(
        &state,
        draft.id.as_deref(),
        direct.auth_type.as_deref(),
        password.or(url_password),
    )?;
    if let (Some(ref mut ssh_cfg), Some(ref existing_id)) = (&mut direct.ssh, &draft.id) {
        if let Ok(pwds) = state.passwords.read() {
            if ssh_cfg.password.is_none() {
                ssh_cfg.password = pwds.get(&format!("ssh_pwd:{existing_id}")).cloned();
            }
            if ssh_cfg.passphrase.is_none() {
                ssh_cfg.passphrase = pwds.get(&format!("ssh_pass:{existing_id}")).cloned();
            }
        }
    }

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
    let ssh_pwd_key = format!("ssh_pwd:{profile_id}");
    let ssh_pass_key = format!("ssh_pass:{profile_id}");
    if let Ok(mut pguard) = state.passwords.write() {
        for key in [&profile_id, &ssh_pwd_key, &ssh_pass_key] {
            if let Some(mut old) = pguard.remove(key) {
                old.zeroize();
            }
        }
    }
    if let Ok(mut vguard) = state.persisted_passwords.write() {
        for key in [&profile_id, &ssh_pwd_key, &ssh_pass_key] {
            if let Some(mut old) = vguard.remove(key) {
                old.zeroize();
            }
        }
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
pub async fn disconnect_connection_cmd(
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<bool, String> {
    let removed = state.active_backends.write().await.remove(&profile_id);
    Ok(removed.is_some())
}

#[tauri::command]
pub async fn list_active_connections_cmd(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    Ok(state.active_backends.read().await.keys().cloned().collect())
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DialogFilterDto {
    pub name: String,
    pub extensions: Vec<String>,
}

#[tauri::command]
pub async fn pick_dialog_path_cmd(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    kind: String,
    default_path: Option<String>,
    filters: Option<Vec<DialogFilterDto>>,
) -> Result<Option<String>, String> {
    let mut builder = app.dialog().file();
    if let Some(ref flts) = filters {
        for f in flts {
            let ext_refs: Vec<&str> = f.extensions.iter().map(String::as_str).collect();
            if !ext_refs.is_empty() {
                builder = builder.add_filter(&f.name, &ext_refs);
            }
        }
    }
    if kind == "saveFile" {
        if let Some(def) = default_path.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            let safe_name = def.rsplit(['/', '\\']).next().unwrap_or(def);
            builder = builder.set_file_name(safe_name);
        }
    }

    let (tx, rx) = oneshot::channel();
    match kind.as_str() {
        "openDir" => {
            builder.pick_folder(move |picked| {
                let _ = tx.send(picked);
            });
        }
        "saveFile" => {
            builder.save_file(move |picked| {
                let _ = tx.send(picked);
            });
        }
        "openFile" | "openTextFile" => {
            builder.pick_file(move |picked| {
                let _ = tx.send(picked);
            });
        }
        other => return Err(format!("不支持的对话框类型: {other}")),
    }

    let Some(file_path) = rx.await.map_err(|e| format!("等待系统文件对话框失败: {e}"))? else {
        return Ok(None);
    };
    let pb = file_path
        .into_path()
        .map_err(|e| format!("所选路径不是有效本地文件系统路径: {e}"))?;

    match kind.as_str() {
        "openDir" => {
            let canon = canonicalize_existing_path(&pb).map_err(map_err)?;
            state.approve_dir(canon.clone());
            Ok(Some(normalize_display_path(&canon)))
        }
        "saveFile" => {
            let canon = canonicalize_target_path(&pb, false).map_err(map_err)?;
            if state.is_protected_storage_target(&canon) {
                return Err("安全限制：禁止选择内部凭据或状态文件作为保存目标".to_string());
            }
            state.approve_write_path(canon.clone());
            Ok(Some(normalize_display_path(&canon)))
        }
        "openTextFile" => {
            let canon = canonicalize_existing_path(&pb).map_err(map_err)?;
            if state.is_protected_storage_target(&canon) {
                return Err("安全限制：禁止直接打开内部凭据或状态文件".to_string());
            }
            state.approve_read_path(canon.clone(), true);
            Ok(Some(normalize_display_path(&canon)))
        }
        _ => {
            let canon = canonicalize_existing_path(&pb).map_err(map_err)?;
            if state.is_protected_storage_target(&canon) {
                return Err("安全限制：禁止直接打开内部凭据或状态文件".to_string());
            }
            state.approve_read_path(canon.clone(), false);
            Ok(Some(normalize_display_path(&canon)))
        }
    }
}

#[tauri::command]
pub async fn import_dbeaver_path_cmd(
    state: State<'_, AppState>,
    path: String,
    include_passwords: Option<bool>,
    project_id: Option<String>,
) -> Result<Vec<ConnectionProfile>, String> {
    let canon = state.validate_import_dbeaver_path(&path)?;
    let parsed = import_dbeaver_from_path(&canon, include_passwords.unwrap_or(true))
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
    let out_path = state.validate_write_path(&path)?;
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

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&out_path, bytes).map_err(|e| e.to_string())?;

    let display_path = normalize_display_path(&out_path);
    Ok(ExportResultDto {
        path: display_path.clone(),
        count: selected.len(),
        included_passwords: include_passwords,
        files: vec![display_path],
    })
}

#[tauri::command]
pub fn export_dbeaver_folder_cmd(
    state: State<'_, AppState>,
    dir_path: String,
    include_passwords: bool,
    profile_ids: Option<Vec<String>>,
) -> Result<ExportResultDto, String> {
    let out_dir = state.validate_dir_path(&dir_path, true)?;
    let selected = state.filter_profiles(profile_ids)?;
    let folders = state.folder_map_by_profile();
    let passwords = if include_passwords {
        state.collect_passwords()
    } else {
        HashMap::new()
    };
    let files = export_to_dbeaver_folder(
        &out_dir,
        &selected,
        &passwords,
        &folders,
        include_passwords,
    )
    .map_err(map_err)?;

    Ok(ExportResultDto {
        path: normalize_display_path(&out_dir),
        count: selected.len(),
        included_passwords: include_passwords,
        files,
    })
}

#[tauri::command]
pub fn export_connections_xlsx_cmd(
    state: State<'_, AppState>,
    path: String,
    include_passwords: Option<bool>,
    profile_ids: Option<Vec<String>>,
) -> Result<ExportResultDto, String> {
    let include_passwords = include_passwords.unwrap_or(false);
    let out_path = state.validate_write_path(&path)?;
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

    let count = write_xlsx_file(&out_path, "数据源账号清单", &columns, &rows)
        .map_err(map_err)?;

    let display_path = normalize_display_path(&out_path);
    Ok(ExportResultDto {
        path: display_path.clone(),
        count,
        included_passwords: include_passwords,
        files: vec![display_path],
    })
}

#[tauri::command]
pub async fn export_rows_xlsx_cmd(
    state: State<'_, AppState>,
    path: String,
    sheet_name: String,
    columns: Vec<String>,
    rows: Vec<BTreeMap<String, serde_json::Value>>,
) -> Result<usize, String> {
    let out_path = state.validate_write_path(&path)?;
    tokio::task::spawn_blocking(move || {
        write_xlsx_file(&out_path, &sheet_name, &columns, &rows).map_err(map_err)
    })
    .await
    .map_err(|e| e.to_string())?
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
#[allow(clippy::too_many_arguments)]
pub async fn execute_sql_cmd(
    state: State<'_, AppState>,
    profile_id: String,
    db_name: Option<String>,
    sql: String,
    limit: Option<u32>,
    query_id: Option<String>,
    timeout_sec: Option<u32>,
    skip_history: Option<bool>,
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

    let effective_timeout_sec = timeout_sec
        .or_else(|| match &profile.mode {
            ConnectionMode::Direct(d) | ConnectionMode::Hybrid { direct: d, .. } => {
                d.options.as_ref().and_then(|o| o.query_timeout_sec)
            }
            _ => None,
        })
        .filter(|&s| s > 0);

    let (tx, mut rx) = mpsc::channel::<QueryChunk>(16);
    let forwarder = tokio::spawn(async move {
        while let Some(chunk) = rx.recv().await {
            if on_chunk.send(chunk).is_err() {
                break;
            }
        }
    });

    let effective_qid = if query_id.is_some() || effective_timeout_sec.is_some() {
        Some(query_id.unwrap_or_else(|| format!("q_{}", Uuid::new_v4())))
    } else {
        None
    };

    let cancel_rx_opt = if let Some(ref qid) = effective_qid {
        let (ctx, crx) = oneshot::channel::<()>();
        state.running_queries.write().await.insert(
            qid.clone(),
            RunningQueryHandle {
                cancel_tx: ctx,
                backend: Arc::clone(&backend),
            },
        );
        Some(crx)
    } else {
        None
    };

    let req = QueryReq {
        database: db_name.clone(),
        sql: sql.clone(),
        limit,
        query_id: effective_qid.clone(),
        timeout_sec: effective_timeout_sec,
    };

    let exec_result = match (cancel_rx_opt, effective_timeout_sec) {
        (Some(cancel_rx), Some(secs)) => {
            let sleep_fut = tokio::time::sleep(Duration::from_secs(u64::from(secs)));
            tokio::pin!(sleep_fut);
            tokio::select! {
                res = backend.execute_query(req, tx) => res,
                _ = cancel_rx => Err(CoreError::ExecutionFailed(
                    "QUERY_CANCELLED: 用户已手动中止当前查询".to_string(),
                )),
                _ = &mut sleep_fut => {
                    if let Some(ref qid) = effective_qid {
                        let _ = backend.cancel_running_query(qid).await;
                    }
                    Err(CoreError::ExecutionFailed(format!(
                        "QUERY_TIMEOUT: 查询执行超过 {} 秒超时限制，已自动触发终止",
                        secs
                    )))
                }
            }
        }
        (Some(cancel_rx), None) => {
            tokio::select! {
                res = backend.execute_query(req, tx) => res,
                _ = cancel_rx => Err(CoreError::ExecutionFailed(
                    "QUERY_CANCELLED: 用户已手动中止当前查询".to_string(),
                )),
            }
        }
        (None, _) => backend.execute_query(req, tx).await,
    };

    if let Some(ref qid) = effective_qid {
        state.running_queries.write().await.remove(qid);
    }

    let _ = forwarder.await;
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let record_history = !skip_history.unwrap_or(false);
    match exec_result {
        Ok(summary) => {
            if record_history {
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
                    pinned: false,
                });
            }
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
            if record_history {
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
                    pinned: false,
                });
            }
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
    let (profile, backend) = state.resolve_backend(&profile_id, None).await?;
    let analysis = analyze_sql(&sql, &profile.db_type);
    if analysis.statements.is_empty() {
        return Err("EXPLAIN 需要提供有效的 SQL 语句".to_string());
    }
    if analysis.statements.len() > 1 {
        return Err("安全限制：EXPLAIN 仅允许单条 SQL 语句，禁止拼接多条语句".to_string());
    }
    let stmt = &analysis.statements[0];
    let is_read_only_sql = matches!(analysis.overall_kind, SqlAccessKind::ReadOnly);
    let profile_read_only = matches!(
        &profile.mode,
        ConnectionMode::Direct(d) | ConnectionMode::Hybrid { direct: d, .. } if d.read_only
    ) || !profile.capabilities.can_dml;

    if profile_read_only && !is_read_only_sql {
        return Err(
            "安全限制：只读连接禁止对写操作/DDL/DCL 或未知语句执行 EXPLAIN".to_string(),
        );
    }
    if !is_read_only_sql
        && (stmt.statement_type.starts_with("EXPLAIN_ANALYZE_")
            || matches!(
                analysis.overall_kind,
                SqlAccessKind::WriteDdl | SqlAccessKind::AdminOrDcl | SqlAccessKind::Unknown
            ))
    {
        return Err(format!(
            "安全限制：禁止通过 EXPLAIN 执行高危或会实际修改数据的语句 ({})",
            stmt.statement_type
        ));
    }

    match state
        .extensions
        .check_before_execute(&profile, Some(db_name.as_str()), &sql)
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

    backend.explain(&db_name, &sql).await.map_err(map_err)
}

#[tauri::command]
pub async fn analyze_sql_cmd(sql: String, db_type: DbEngineType) -> Result<ScriptAnalysis, String> {
    tokio::task::spawn_blocking(move || analyze_sql(&sql, &db_type))
        .await
        .map_err(|e| e.to_string())
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
pub async fn cancel_query_cmd(
    state: State<'_, AppState>,
    query_id: String,
) -> Result<bool, String> {
    let entry = state.running_queries.write().await.remove(&query_id);
    if let Some(handle) = entry {
        let killed_remote = handle
            .backend
            .cancel_running_query(&query_id)
            .await
            .unwrap_or(false);
        let signaled_local = handle.cancel_tx.send(()).is_ok();
        Ok(killed_remote || signaled_local)
    } else {
        Ok(false)
    }
}

#[tauri::command]
pub async fn list_foreign_keys_cmd(
    state: State<'_, AppState>,
    profile_id: String,
    db_name: String,
    table_name: String,
) -> Result<Vec<ForeignKeyMeta>, String> {
    let (_, backend) = state.resolve_backend(&profile_id, None).await?;
    backend
        .list_foreign_keys(&db_name, &table_name)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn list_routines_cmd(
    state: State<'_, AppState>,
    profile_id: String,
    db_name: String,
) -> Result<Vec<RoutineMeta>, String> {
    let (_, backend) = state.resolve_backend(&profile_id, None).await?;
    backend.list_routines(&db_name).await.map_err(map_err)
}

#[tauri::command]
pub async fn get_transaction_status_cmd(
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<TransactionStatus, String> {
    let (_, backend) = state.resolve_backend(&profile_id, None).await?;
    backend.transaction_status().await.map_err(map_err)
}

#[tauri::command]
pub async fn set_transaction_mode_cmd(
    state: State<'_, AppState>,
    profile_id: String,
    manual: bool,
) -> Result<TransactionStatus, String> {
    let (_, backend) = state.resolve_backend(&profile_id, None).await?;
    backend.set_manual_transaction(manual).await.map_err(map_err)
}

#[tauri::command]
pub async fn commit_transaction_cmd(
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<TransactionStatus, String> {
    let (_, backend) = state.resolve_backend(&profile_id, None).await?;
    backend.commit_transaction().await.map_err(map_err)
}

#[tauri::command]
pub async fn rollback_transaction_cmd(
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<TransactionStatus, String> {
    let (_, backend) = state.resolve_backend(&profile_id, None).await?;
    backend.rollback_transaction().await.map_err(map_err)
}

#[tauri::command]
pub async fn format_sql_cmd(sql: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || format_sql(&sql))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn extract_statement_at_cursor_cmd(
    sql: String,
    cursor_offset: usize,
) -> Result<Option<String>, String> {
    tokio::task::spawn_blocking(move || extract_statement_at_cursor(&sql, cursor_offset))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn extract_markdown_sql_blocks_cmd(
    content: String,
) -> Result<Vec<MarkdownSqlBlock>, String> {
    tokio::task::spawn_blocking(move || extract_markdown_sql_blocks(&content))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn open_text_file_cmd(
    state: State<'_, AppState>,
    path: String,
    encoding_override: Option<String>,
) -> Result<OpenedFileDto, String> {
    let canon = state.validate_read_path(&path)?;
    tokio::task::spawn_blocking(move || {
        open_text_file(&canon, encoding_override.as_deref()).map_err(map_err)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn save_text_file_cmd(
    state: State<'_, AppState>,
    path: String,
    content: String,
    encoding: String,
    line_ending: String,
    with_bom: bool,
) -> Result<FileStatsDto, String> {
    let canon = state.validate_write_path(&path)?;
    let canon_for_approve = canon.clone();
    let stats = tokio::task::spawn_blocking(move || {
        save_text_file(&canon, &content, &encoding, &line_ending, with_bom).map_err(map_err)
    })
    .await
    .map_err(|e| e.to_string())??;
    state.approve_read_path(canon_for_approve, true);
    Ok(stats)
}

#[tauri::command]
pub async fn list_workspace_files_cmd(
    state: State<'_, AppState>,
    dir_path: String,
) -> Result<Vec<WorkspaceFileEntry>, String> {
    let canon = state.validate_dir_path(&dir_path, false)?;
    tokio::task::spawn_blocking(move || list_workspace_files(&canon).map_err(map_err))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn create_demo_markdown_file_cmd(state: State<'_, AppState>) -> Result<OpenedFileDto, String> {
    ensure_demo_markdown_file(&state.storage_dir).map_err(map_err)
}

#[tauri::command]
pub fn reveal_in_explorer_cmd(
    state: State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    let target = state.validate_reveal_path(&path)?;
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
pub async fn save_console_scratch_cmd(
    state: State<'_, AppState>,
    file_name: String,
    content: String,
) -> Result<String, String> {
    if content.len() > 2 * 1024 * 1024 {
        return Ok(String::new());
    }
    let consoles_dir = state.storage_dir.join("consoles");
    tokio::task::spawn_blocking(move || {
        std::fs::create_dir_all(&consoles_dir).map_err(|e| e.to_string())?;
        let canon_dir = canonicalize_existing_path(&consoles_dir).map_err(map_err)?;
        let safe_name = sanitize_console_filename(&file_name);
        reject_traversal_components(Path::new(&safe_name)).map_err(map_err)?;
        let target = canonicalize_target_path(&canon_dir.join(safe_name), false).map_err(map_err)?;
        if !is_path_within_any_root(&target, &[canon_dir]) {
            return Err("安全限制：控制台草稿路径越界".to_string());
        }
        std::fs::write(&target, content.as_bytes()).map_err(|e| e.to_string())?;
        Ok(normalize_display_path(&target))
    })
    .await
    .map_err(|e| e.to_string())?
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SqlDumpResultDto {
    pub path: String,
    pub table_count: usize,
    pub row_count: usize,
    pub truncated_tables: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SqlDumpRequest {
    pub profile_id: String,
    pub db_name: String,
    pub table_names: Option<Vec<String>>,
    pub include_data: bool,
    pub max_rows_per_table: Option<u32>,
    pub path: String,
}

#[tauri::command]
pub async fn search_objects_cmd(
    state: State<'_, AppState>,
    profile_id: String,
    db_name: String,
    query: String,
) -> Result<Vec<ObjectSearchHit>, String> {
    let (_, backend) = state.resolve_backend(&profile_id, None).await?;
    backend
        .search_objects(&db_name, &query)
        .await
        .map_err(map_err)
}

enum DumpWriteMsg {
    Header {
        table_name: String,
        ddl: String,
    },
    DataBatch {
        table_name: String,
        columns: Vec<String>,
        rows: Vec<BTreeMap<String, serde_json::Value>>,
        include_banner: bool,
    },
    Comment(String),
    Footer,
}

#[tauri::command]
pub async fn export_sql_dump_cmd(
    state: State<'_, AppState>,
    req: SqlDumpRequest,
) -> Result<SqlDumpResultDto, String> {
    let out_path = state.validate_write_path(&req.path)?;
    let (profile, backend) = state.resolve_backend(&req.profile_id, None).await?;
    if matches!(profile.db_type, DbEngineType::Redis) {
        return Err("Redis 数据源不支持导出 SQL 转储文件，请使用结果集导出或 Redis 命令。".to_string());
    }

    let listed_tables = backend.list_tables(&req.db_name).await.map_err(map_err)?;
    let target_tables: Vec<TableMeta> = match &req.table_names {
        Some(names) if !names.is_empty() => names
            .iter()
            .map(|name| {
                listed_tables
                    .iter()
                    .find(|t| t.name == *name)
                    .cloned()
                    .unwrap_or(TableMeta {
                        name: name.clone(),
                        table_type: Some("TABLE".to_string()),
                        engine: None,
                        estimated_rows: None,
                        data_size_kb: None,
                        comment: None,
                    })
            })
            .collect(),
        _ => listed_tables,
    };

    let row_cap = req.max_rows_per_table.unwrap_or(20_000).clamp(1, 200_000);
    let generated_at = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let header_str = build_sql_dump_header(
        &req.db_name,
        &profile.db_type,
        req.include_data,
        target_tables.len(),
        &generated_at,
    );
    let footer_str = build_sql_dump_footer(&profile.db_type);

    let approved_out = out_path.clone();
    let display_out = normalize_display_path(&out_path);
    let db_name_writer = req.db_name.clone();
    let engine_writer = profile.db_type.clone();
    let (dump_tx, mut dump_rx) = mpsc::channel::<DumpWriteMsg>(8);

    let writer_handle = tokio::task::spawn_blocking(move || -> Result<(), String> {
        use std::io::Write as _;
        if let Some(parent) = out_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("创建 SQL 转储目录失败: {e}"))?;
            }
        }
        let file = std::fs::File::create(&out_path)
            .map_err(|e| format!("创建 SQL 转储文件失败: {e}"))?;
        let mut writer = std::io::BufWriter::with_capacity(64 * 1024, file);
        writer
            .write_all(header_str.as_bytes())
            .map_err(|e| format!("写入 SQL 转储头部失败: {e}"))?;

        let mut active_cols: Vec<String> = Vec::new();
        while let Some(msg) = dump_rx.blocking_recv() {
            match msg {
                DumpWriteMsg::Header { table_name, ddl } => {
                    active_cols.clear();
                    write_sql_dump_table_header(&mut writer, &table_name, &ddl)
                        .map_err(|e| format!("写入表结构失败: {e}"))?;
                }
                DumpWriteMsg::DataBatch {
                    table_name,
                    columns,
                    rows,
                    include_banner,
                } => {
                    if !columns.is_empty() {
                        active_cols = columns;
                    }
                    if !active_cols.is_empty() && !rows.is_empty() {
                        write_sql_dump_insert_batch(
                            &mut writer,
                            &db_name_writer,
                            &table_name,
                            &active_cols,
                            &rows,
                            SqlDumpFormatOptions::new(&engine_writer, true),
                            include_banner,
                        )
                        .map_err(|e| format!("写入表数据批次失败: {e}"))?;
                    }
                }
                DumpWriteMsg::Comment(line) => {
                    writer
                        .write_all(line.as_bytes())
                        .map_err(|e| format!("写入转储注释失败: {e}"))?;
                }
                DumpWriteMsg::Footer => {
                    writer
                        .write_all(footer_str.as_bytes())
                        .map_err(|e| format!("写入 SQL 转储尾部失败: {e}"))?;
                }
            }
        }
        writer
            .flush()
            .map_err(|e| format!("刷新 SQL 转储文件失败: {e}"))?;
        Ok(())
    });

    let mut total_rows: usize = 0;
    let mut truncated_tables = Vec::new();

    for tbl in &target_tables {
        let ddl = match backend.get_table_ddl(&req.db_name, &tbl.name).await {
            Ok(d) => d,
            Err(e) => format!("-- Failed to fetch DDL for {}: {e}", tbl.name),
        };
        if dump_tx
            .send(DumpWriteMsg::Header {
                table_name: tbl.name.clone(),
                ddl,
            })
            .await
            .is_err()
        {
            break;
        }

        let is_view = tbl
            .table_type
            .as_deref()
            .is_some_and(|tt| tt.to_ascii_uppercase().contains("VIEW"));

        if req.include_data && !is_view {
            let select_sql =
                build_limited_select_sql(&req.db_name, &tbl.name, &profile.db_type, row_cap);
            let (tx, mut rx) = mpsc::channel::<QueryChunk>(4);
            let table_name_fwd = tbl.name.clone();
            let dump_tx_fwd = dump_tx.clone();
            let forwarder = tokio::spawn(async move {
                let mut wrote_banner = false;
                let mut table_rows = 0usize;
                while let Some(chunk) = rx.recv().await {
                    let chunk_len = chunk.rows.len();
                    if chunk.columns.is_empty() && chunk_len == 0 {
                        continue;
                    }
                    let include_banner = if chunk_len > 0 && !wrote_banner {
                        wrote_banner = true;
                        true
                    } else {
                        false
                    };
                    table_rows += chunk_len;
                    if dump_tx_fwd
                        .send(DumpWriteMsg::DataBatch {
                            table_name: table_name_fwd.clone(),
                            columns: chunk.columns,
                            rows: chunk.rows,
                            include_banner,
                        })
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                table_rows
            });

            let exec_res = backend
                .execute_query(
                    QueryReq {
                        database: Some(req.db_name.clone()),
                        sql: select_sql,
                        limit: Some(row_cap),
                        query_id: None,
                        timeout_sec: None,
                    },
                    tx,
                )
                .await;

            let table_rows = forwarder.await.unwrap_or(0);
            if exec_res.is_ok() {
                if table_rows >= row_cap as usize {
                    truncated_tables.push(tbl.name.clone());
                }
                total_rows += table_rows;
            } else if let Err(err) = exec_res {
                let _ = dump_tx
                    .send(DumpWriteMsg::Comment(format!(
                        "-- Warning: Failed to export rows for {}: {err}\n",
                        tbl.name
                    )))
                    .await;
            }
        }
    }

    let _ = dump_tx.send(DumpWriteMsg::Footer).await;
    drop(dump_tx);

    writer_handle
        .await
        .map_err(|e| format!("等待 SQL 转储写入线程失败: {e}"))??;

    state.approve_read_path(approved_out, false);

    Ok(SqlDumpResultDto {
        path: display_out,
        table_count: target_tables.len(),
        row_count: total_rows,
        truncated_tables,
    })
}

#[tauri::command]
pub fn toggle_history_pin_cmd(state: State<'_, AppState>, history_id: String) -> Result<bool, String> {
    let mut new_pinned = false;
    let snapshot = {
        let mut guard = state.history.write().map_err(|e| e.to_string())?;
        if let Some(entry) = guard.iter_mut().find(|h| h.id == history_id) {
            entry.pinned = !entry.pinned;
            new_pinned = entry.pinned;
        }
        guard.clone()
    };
    state.history_dirty.store(false, Ordering::Release);
    let _ = write_history_snapshot_atomic(&state.storage_dir, &snapshot);
    Ok(new_pinned)
}

#[tauri::command]
pub fn delete_history_entry_cmd(
    state: State<'_, AppState>,
    history_id: String,
) -> Result<bool, String> {
    let (removed, snapshot) = {
        let mut guard = state.history.write().map_err(|e| e.to_string())?;
        let before = guard.len();
        guard.retain(|h| h.id != history_id);
        (guard.len() != before, guard.clone())
    };
    if removed {
        state.history_dirty.store(false, Ordering::Release);
        let _ = write_history_snapshot_atomic(&state.storage_dir, &snapshot);
    }
    Ok(removed)
}

#[tauri::command]
pub async fn read_xlsx_sheet_cmd(
    state: State<'_, AppState>,
    path: String,
    sheet_name: Option<String>,
    max_rows: Option<usize>,
) -> Result<XlsxSheetPreview, String> {
    let canon = state.validate_read_path(&path)?;
    tokio::task::spawn_blocking(move || {
        read_xlsx_sheet_preview(
            &canon,
            sheet_name.as_deref(),
            max_rows.unwrap_or(1000),
        )
        .map_err(map_err)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn run_sql_file_cmd(
    state: State<'_, AppState>,
    profile_id: String,
    db_name: Option<String>,
    path: String,
    continue_on_error: bool,
    query_id: Option<String>,
    on_progress: Channel<SqlScriptProgress>,
) -> Result<SqlScriptRunResult, String> {
    let started = Instant::now();
    let file_path = state.validate_read_path(&path)?;
    let (profile, backend) = state.resolve_backend(&profile_id, None).await?;
    let total_bytes = std::fs::metadata(&file_path)
        .map_err(|e| format!("读取 SQL 文件信息失败: {e}"))?
        .len();

    let (stmt_tx, mut stmt_rx) = mpsc::channel::<Result<(String, u64), String>>(8);
    let reader_handle = tokio::task::spawn_blocking(move || {
        let file = match std::fs::File::open(&file_path) {
            Ok(f) => f,
            Err(e) => {
                let _ = stmt_tx.blocking_send(Err(format!("打开 SQL 文件失败: {e}")));
                return;
            }
        };
        let buf = std::io::BufReader::with_capacity(64 * 1024, file);
        let mut reader = SqlScriptStreamReader::new(buf);
        loop {
            match reader.next_statement() {
                Ok(Some(stmt)) => {
                    if stmt_tx.blocking_send(Ok((stmt, reader.bytes_read()))).is_err() {
                        break;
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    let _ = stmt_tx.blocking_send(Err(format!("流式读取 SQL 失败: {e}")));
                    break;
                }
            }
        }
    });

    let effective_qid = query_id.clone();
    let mut cancel_rx_opt = if let Some(ref qid) = effective_qid {
        let (ctx, crx) = oneshot::channel::<()>();
        state.running_queries.write().await.insert(
            qid.clone(),
            RunningQueryHandle {
                cancel_tx: ctx,
                backend: Arc::clone(&backend),
            },
        );
        Some(crx)
    } else {
        None
    };

    let mut total_statements = 0usize;
    let mut succeeded_statements = 0usize;
    let mut failed_statements = 0usize;
    let mut last_bytes_read = 0u64;
    let mut last_error: Option<String> = None;
    let mut errors: Vec<String> = Vec::new();
    let mut cancelled = false;

    while let Some(item) = stmt_rx.recv().await {
        if cancel_rx_opt
            .as_mut()
            .is_some_and(|rx| matches!(rx.try_recv(), Ok(())))
        {
            cancelled = true;
            break;
        }
        let (stmt, bytes_read) = item?;
        last_bytes_read = bytes_read;
        total_statements += 1;

        let ext_res = state
            .extensions
            .check_before_execute(&profile, db_name.as_deref(), &stmt)
            .await;
        match ext_res {
            Ok(PreExecuteDecision::Allow) => {}
            Ok(PreExecuteDecision::RequireApproval { reason }) => {
                if let Some(ref qid) = effective_qid {
                    state.running_queries.write().await.remove(qid);
                }
                return Err(format!("EXTENSION_APPROVAL_REQUIRED: {reason}"));
            }
            Ok(PreExecuteDecision::Block { reason }) => {
                if let Some(ref qid) = effective_qid {
                    state.running_queries.write().await.remove(qid);
                }
                return Err(format!("EXTENSION_BLOCKED: {reason}"));
            }
            Err(err) => {
                if let Some(ref qid) = effective_qid {
                    state.running_queries.write().await.remove(qid);
                }
                return Err(map_err(err));
            }
        }

        let (tx_dummy, mut rx_dummy) = mpsc::channel::<QueryChunk>(4);
        let drain_task = tokio::spawn(async move { while rx_dummy.recv().await.is_some() {} });
        let req = QueryReq {
            database: db_name.clone(),
            sql: stmt.clone(),
            limit: None,
            query_id: effective_qid.clone(),
            timeout_sec: None,
        };

        let res = match cancel_rx_opt.as_mut() {
            Some(crx) => tokio::select! {
                r = backend.execute_query(req, tx_dummy) => r,
                _ = crx => {
                    cancelled = true;
                    Err(CoreError::ExecutionFailed("QUERY_CANCELLED".to_string()))
                }
            },
            None => backend.execute_query(req, tx_dummy).await,
        };
        let _ = drain_task.await;

        if cancelled {
            break;
        }

        match res {
            Ok(_) => succeeded_statements += 1,
            Err(err) => {
                failed_statements += 1;
                let msg = format!("#{total_statements}: {err}");
                last_error = Some(msg.clone());
                if errors.len() < 30 {
                    errors.push(msg);
                }
                if !continue_on_error {
                    let preview: String = stmt.chars().take(120).collect();
                    let _ = on_progress.send(SqlScriptProgress {
                        executed_statements: total_statements,
                        failed_statements,
                        bytes_read: last_bytes_read,
                        total_bytes,
                        current_statement_preview: preview,
                        last_error: last_error.clone(),
                        done: true,
                    });
                    if let Some(ref qid) = effective_qid {
                        state.running_queries.write().await.remove(qid);
                    }
                    drop(stmt_rx);
                    let _ = reader_handle.await;
                    return Ok(SqlScriptRunResult {
                        total_statements,
                        succeeded_statements,
                        failed_statements,
                        duration_ms: started.elapsed().as_millis() as u64,
                        cancelled: false,
                        errors,
                    });
                }
            }
        }

        if total_statements <= 5 || total_statements.is_multiple_of(5) {
            let preview: String = stmt.chars().take(120).collect();
            let _ = on_progress.send(SqlScriptProgress {
                executed_statements: total_statements,
                failed_statements,
                bytes_read: last_bytes_read,
                total_bytes,
                current_statement_preview: preview,
                last_error: last_error.clone(),
                done: false,
            });
        }
    }

    if let Some(ref qid) = effective_qid {
        state.running_queries.write().await.remove(qid);
    }
    drop(stmt_rx);
    let _ = reader_handle.await;

    let _ = on_progress.send(SqlScriptProgress {
        executed_statements: total_statements,
        failed_statements,
        bytes_read: if cancelled { last_bytes_read } else { total_bytes },
        total_bytes,
        current_statement_preview: String::new(),
        last_error,
        done: true,
    });

    Ok(SqlScriptRunResult {
        total_statements,
        succeeded_statements,
        failed_statements,
        duration_ms: started.elapsed().as_millis() as u64,
        cancelled,
        errors,
    })
}

enum QueryExportWriteMsg {
    Chunk {
        columns: Vec<String>,
        rows: Vec<BTreeMap<String, serde_json::Value>>,
    },
    Finish,
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn export_query_to_file_cmd(
    state: State<'_, AppState>,
    profile_id: String,
    db_name: Option<String>,
    sql: String,
    format: String,
    path: String,
    sheet_name: Option<String>,
    query_id: Option<String>,
    on_progress: Channel<QueryExportProgress>,
) -> Result<QueryExportResult, String> {
    let started = Instant::now();
    let (profile, backend) = state.resolve_backend(&profile_id, None).await?;
    let analysis = analyze_sql(&sql, &profile.db_type);
    if !matches!(analysis.overall_kind, SqlAccessKind::ReadOnly) {
        return Err(
            "安全限制：流式结果导出仅允许只读查询语句（已拦截 DML/DDL 或潜在写操作）".to_string(),
        );
    }
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

    let fmt = format.trim().to_ascii_lowercase();
    if !matches!(fmt.as_str(), "csv" | "xlsx" | "json") {
        return Err(format!("不支持的导出格式: {format}"));
    }

    let out_path = state.validate_write_path(&path)?;
    let cleanup_path = out_path.clone();
    let display_out = normalize_display_path(&out_path);
    let sheet_title = sheet_name.unwrap_or_else(|| "QueryResult".to_string());
    let fmt_writer = fmt.clone();
    let (write_tx, mut write_rx) = mpsc::channel::<QueryExportWriteMsg>(8);

    let writer_handle = tokio::task::spawn_blocking(move || -> Result<(usize, u64), String> {
        use std::io::Write as _;
        if let Some(parent) = out_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| format!("创建导出目录失败: {e}"))?;
            }
        }
        let mut total_rows = 0usize;
        let mut active_cols: Vec<String> = Vec::new();

        match fmt_writer.as_str() {
            "xlsx" => {
                let mut writer =
                    StreamingXlsxWriter::create(&out_path, &sheet_title).map_err(map_err)?;
                while let Some(msg) = write_rx.blocking_recv() {
                    match msg {
                        QueryExportWriteMsg::Chunk { columns, rows } => {
                            if !columns.is_empty() {
                                active_cols = columns;
                            }
                            if !active_cols.is_empty() || !rows.is_empty() {
                                let _ = writer
                                    .write_chunk(&active_cols, &rows)
                                    .map_err(map_err)?;
                            }
                        }
                        QueryExportWriteMsg::Finish => break,
                    }
                }
                let count = writer.finish().map_err(map_err)?;
                let sz = std::fs::metadata(&out_path).map(|m| m.len()).unwrap_or(0);
                Ok((count, sz))
            }
            "csv" => {
                let file = std::fs::File::create(&out_path)
                    .map_err(|e| format!("创建 CSV 文件失败: {e}"))?;
                let mut bw = std::io::BufWriter::with_capacity(64 * 1024, file);
                bw.write_all(b"\xEF\xBB\xBF").map_err(|e| e.to_string())?;
                let mut wrote_header = false;
                while let Some(msg) = write_rx.blocking_recv() {
                    match msg {
                        QueryExportWriteMsg::Chunk { columns, rows } => {
                            if !columns.is_empty() {
                                active_cols = columns;
                            }
                            if !wrote_header && !active_cols.is_empty() {
                                let hdr_refs: Vec<&str> =
                                    active_cols.iter().map(String::as_str).collect();
                                write_csv_row(&mut bw, &hdr_refs).map_err(|e| e.to_string())?;
                                wrote_header = true;
                            }
                            let mut cell_bufs: Vec<String> = Vec::with_capacity(active_cols.len());
                            for row in rows {
                                cell_bufs.clear();
                                for col in &active_cols {
                                    let s = match row.get(col) {
                                        None | Some(serde_json::Value::Null) => String::new(),
                                        Some(serde_json::Value::String(s)) => s.clone(),
                                        Some(v) => v.to_string(),
                                    };
                                    cell_bufs.push(s);
                                }
                                let refs: Vec<&str> =
                                    cell_bufs.iter().map(String::as_str).collect();
                                write_csv_row(&mut bw, &refs).map_err(|e| e.to_string())?;
                                total_rows += 1;
                            }
                        }
                        QueryExportWriteMsg::Finish => break,
                    }
                }
                bw.flush().map_err(|e| e.to_string())?;
                let sz = std::fs::metadata(&out_path).map(|m| m.len()).unwrap_or(0);
                Ok((total_rows, sz))
            }
            _ => {
                let file = std::fs::File::create(&out_path)
                    .map_err(|e| format!("创建 JSON 文件失败: {e}"))?;
                let mut bw = std::io::BufWriter::with_capacity(64 * 1024, file);
                bw.write_all(b"[\n").map_err(|e| e.to_string())?;
                let mut first_row = true;
                while let Some(msg) = write_rx.blocking_recv() {
                    match msg {
                        QueryExportWriteMsg::Chunk { rows, .. } => {
                            for row in rows {
                                if !first_row {
                                    bw.write_all(b",\n").map_err(|e| e.to_string())?;
                                }
                                first_row = false;
                                let line = serde_json::to_string(&row).map_err(|e| e.to_string())?;
                                bw.write_all(b"  ").map_err(|e| e.to_string())?;
                                bw.write_all(line.as_bytes()).map_err(|e| e.to_string())?;
                                total_rows += 1;
                            }
                        }
                        QueryExportWriteMsg::Finish => break,
                    }
                }
                bw.write_all(b"\n]\n").map_err(|e| e.to_string())?;
                bw.flush().map_err(|e| e.to_string())?;
                let sz = std::fs::metadata(&out_path).map(|m| m.len()).unwrap_or(0);
                Ok((total_rows, sz))
            }
        }
    });

    let effective_qid = query_id.clone();
    let cancel_rx_opt = if let Some(ref qid) = effective_qid {
        let (ctx, crx) = oneshot::channel::<()>();
        state.running_queries.write().await.insert(
            qid.clone(),
            RunningQueryHandle {
                cancel_tx: ctx,
                backend: Arc::clone(&backend),
            },
        );
        Some(crx)
    } else {
        None
    };

    let (tx, mut rx) = mpsc::channel::<QueryChunk>(8);
    let write_tx_fwd = write_tx.clone();
    let progress_ch = on_progress.clone();
    let forwarder = tokio::spawn(async move {
        let mut streamed_rows = 0usize;
        let mut last_emit_rows = 0usize;
        let mut last_emit_at = Instant::now();
        while let Some(chunk) = rx.recv().await {
            let chunk_rows = chunk.rows.len();
            if chunk.columns.is_empty() && chunk_rows == 0 {
                continue;
            }
            streamed_rows += chunk_rows;
            if write_tx_fwd
                .send(QueryExportWriteMsg::Chunk {
                    columns: chunk.columns,
                    rows: chunk.rows,
                })
                .await
                .is_err()
            {
                break;
            }
            if streamed_rows.saturating_sub(last_emit_rows) >= 500
                || last_emit_at.elapsed() >= Duration::from_millis(200)
            {
                last_emit_rows = streamed_rows;
                last_emit_at = Instant::now();
                let _ = progress_ch.send(QueryExportProgress {
                    exported_rows: streamed_rows,
                    bytes_written: 0,
                    done: false,
                });
            }
        }
    });

    let req = QueryReq {
        database: db_name.clone(),
        sql: sql.clone(),
        limit: None,
        query_id: effective_qid.clone(),
        timeout_sec: None,
    };
    let exec_res = match cancel_rx_opt {
        Some(cancel_rx) => tokio::select! {
            r = backend.execute_query(req, tx) => r,
            _ = cancel_rx => {
                if let Some(ref qid) = effective_qid {
                    let _ = backend.cancel_running_query(qid).await;
                }
                Err(CoreError::ExecutionFailed("QUERY_CANCELLED: 用户已取消流式导出".to_string()))
            }
        },
        None => backend.execute_query(req, tx).await,
    };

    if let Some(ref qid) = effective_qid {
        state.running_queries.write().await.remove(qid);
    }
    let _ = forwarder.await;
    let _ = write_tx.send(QueryExportWriteMsg::Finish).await;
    drop(write_tx);

    let (exported_rows, bytes_written) = writer_handle
        .await
        .map_err(|e| format!("等待流式导出线程失败: {e}"))??;
    if let Err(err) = exec_res {
        let _ = std::fs::remove_file(&cleanup_path);
        return Err(map_err(err));
    }

    let _ = profile;
    let _ = on_progress.send(QueryExportProgress {
        exported_rows,
        bytes_written,
        done: true,
    });

    Ok(QueryExportResult {
        path: display_out,
        format: fmt,
        exported_rows,
        duration_ms: started.elapsed().as_millis() as u64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_appstate_path_guards_block_traversal_and_vault_files() {
        let dir = std::env::temp_dir().join(format!("pacergrip_cmd_path_{}", Uuid::new_v4()));
        let outside = std::env::temp_dir().join(format!("pacergrip_cmd_out_{}", Uuid::new_v4()));
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&outside);
        std::fs::create_dir_all(&outside).unwrap();

        let state = AppState::new(dir.clone(), false);
        std::fs::write(dir.join("vault_v3.key"), b"fake").unwrap();
        std::fs::write(dir.join("credentials.dat"), b"fake").unwrap();
        let outside_file = outside.join("secret.txt");
        std::fs::write(&outside_file, b"outside").unwrap();

        // Reading or overwriting vault files inside storage_dir must be denied.
        assert!(state
            .validate_read_path(&dir.join("vault_v3.key").to_string_lossy())
            .is_err());
        assert!(state
            .validate_write_path(&dir.join("credentials.dat").to_string_lossy())
            .is_err());

        // Reading/writing outside approved workspace or dialog selection must be denied.
        assert!(state
            .validate_read_path(&outside_file.to_string_lossy())
            .is_err());
        assert!(state
            .validate_write_path(&outside.join("new.sql").to_string_lossy())
            .is_err());

        // Traversal inside consoles must be denied, while valid consoles file works.
        assert!(state
            .validate_write_path(&dir.join("consoles/../vault_v3.key").to_string_lossy())
            .is_err());
        assert!(state
            .validate_write_path(&dir.join("consoles/q1.sql").to_string_lossy())
            .is_ok());

        // Explicit dialog approval allows only that approved path.
        let canon_out = canonicalize_existing_path(&outside_file).unwrap();
        state.approve_read_path(canon_out, false);
        assert!(state
            .validate_read_path(&outside_file.to_string_lossy())
            .is_ok());
        assert!(state
            .validate_write_path(&outside_file.to_string_lossy())
            .is_err());

        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&outside);
    }
}
