pub mod commands;
pub mod files;

use std::path::Path;
use tauri::Manager;

use commands::AppState;

/// Copies saved projects, connections, credentials, history, and demo assets into the active
/// workspace directory on first run when migrating from %APPDATA% or legacy identifier folders.
fn migrate_legacy_storage(target_dir: &Path, fallback_app_data: &Path) {
    if target_dir.join("connections.json").is_file() || target_dir.join("projects.json").is_file() {
        return;
    }
    let mut candidates = Vec::new();
    if fallback_app_data != target_dir {
        candidates.push(fallback_app_data.to_path_buf());
    }
    if let Some(parent) = fallback_app_data.parent() {
        candidates.push(parent.join("cn.fwpt.pacergrip"));
    }

    let files = [
        "projects.json",
        "connections.json",
        "credentials.dat",
        "history.json",
        "demo_store.sqlite",
        "示例查询手册.md",
    ];
    for src_dir in candidates {
        if !src_dir.is_dir() {
            continue;
        }
        let has_any = files.iter().any(|f| src_dir.join(f).is_file());
        if !has_any {
            continue;
        }
        let _ = std::fs::create_dir_all(target_dir);
        for file in files {
            let src = src_dir.join(file);
            let dst = target_dir.join(file);
            if src.is_file() && !dst.is_file() {
                let _ = std::fs::copy(&src, &dst);
            }
        }
        break;
    }
}

fn is_dir_writable(dir: &Path) -> bool {
    if std::fs::create_dir_all(dir).is_err() {
        return false;
    }
    let probe = dir.join(".pacer_write_probe");
    if std::fs::write(&probe, b"ok").is_ok() {
        let _ = std::fs::remove_file(&probe);
        true
    } else {
        false
    }
}

#[cfg(target_os = "macos")]
fn is_restricted_macos_bundle_parent(bundle_parent: &Path) -> bool {
    let norm = bundle_parent.to_string_lossy().replace('\\', "/");
    norm == "/Applications"
        || norm.starts_with("/Applications/")
        || norm == "/System"
        || norm.starts_with("/System/")
        || norm == "/Library"
        || norm.starts_with("/Library/")
        || norm == "/usr"
        || norm.starts_with("/usr/")
        || norm == "/Volumes"
        || norm.starts_with("/Volumes/")
        || norm.contains("/AppTranslocation/")
}

#[cfg(target_os = "macos")]
fn resolve_macos_bundle_parent(exe_dir: &Path) -> Option<std::path::PathBuf> {
    let is_macos_dir = exe_dir
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.eq_ignore_ascii_case("MacOS"));
    if !is_macos_dir {
        return None;
    }
    let contents_dir = exe_dir.parent()?;
    let is_contents_dir = contents_dir
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.eq_ignore_ascii_case("Contents"));
    if !is_contents_dir {
        return None;
    }
    let app_bundle = contents_dir.parent()?;
    let is_app_bundle = app_bundle
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.to_ascii_lowercase().ends_with(".app"));
    if !is_app_bundle {
        return None;
    }
    app_bundle.parent().map(|p| p.to_path_buf())
}

#[cfg(target_os = "macos")]
fn build_macos_menu<R: tauri::Runtime>(
    app_handle: &tauri::AppHandle<R>,
) -> tauri::Result<tauri::menu::Menu<R>> {
    use tauri::menu::{AboutMetadata, Menu, PredefinedMenuItem, Submenu, WINDOW_SUBMENU_ID};

    let pkg_info = app_handle.package_info();
    let config = app_handle.config();
    let about_metadata = AboutMetadata {
        name: Some(pkg_info.name.clone()),
        version: Some(pkg_info.version.to_string()),
        copyright: config.bundle.copyright.clone(),
        authors: config.bundle.publisher.clone().map(|p| vec![p]),
        ..Default::default()
    };

    let app_menu = Submenu::with_items(
        app_handle,
        pkg_info.name.clone(),
        true,
        &[
            &PredefinedMenuItem::about(app_handle, None, Some(about_metadata))?,
            &PredefinedMenuItem::separator(app_handle)?,
            &PredefinedMenuItem::services(app_handle, None)?,
            &PredefinedMenuItem::separator(app_handle)?,
            &PredefinedMenuItem::hide(app_handle, None)?,
            &PredefinedMenuItem::hide_others(app_handle, None)?,
            &PredefinedMenuItem::separator(app_handle)?,
            &PredefinedMenuItem::quit(app_handle, None)?,
        ],
    )?;

    // Keep standard Edit menu items so Cmd+C / Cmd+V / Cmd+X / Cmd+A / Cmd+Z work natively in WKWebView.
    let edit_menu = Submenu::with_items(
        app_handle,
        "Edit",
        true,
        &[
            &PredefinedMenuItem::undo(app_handle, None)?,
            &PredefinedMenuItem::redo(app_handle, None)?,
            &PredefinedMenuItem::separator(app_handle)?,
            &PredefinedMenuItem::cut(app_handle, None)?,
            &PredefinedMenuItem::copy(app_handle, None)?,
            &PredefinedMenuItem::paste(app_handle, None)?,
            &PredefinedMenuItem::select_all(app_handle, None)?,
        ],
    )?;

    let view_menu = Submenu::with_items(
        app_handle,
        "View",
        true,
        &[&PredefinedMenuItem::fullscreen(app_handle, None)?],
    )?;

    // Omit PredefinedMenuItem::close_window so Cmd+W routes to Pacer's editor tab close action.
    let window_menu = Submenu::with_id_and_items(
        app_handle,
        WINDOW_SUBMENU_ID,
        "Window",
        true,
        &[
            &PredefinedMenuItem::minimize(app_handle, None)?,
            &PredefinedMenuItem::maximize(app_handle, None)?,
        ],
    )?;

    Menu::with_items(app_handle, &[&app_menu, &edit_menu, &view_menu, &window_menu])
}

/// Prefers `<exe_dir>/pacer-workspace` on Windows/Linux (or next to `Pacer.app` in custom writable
/// folders on macOS); falls back to OS AppData / Application Support when inside `/Applications`,
/// DMG volumes, Gatekeeper AppTranslocation, or read-only locations.
fn resolve_workspace_dir(fallback_app_data: &Path) -> (std::path::PathBuf, bool) {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            #[cfg(target_os = "macos")]
            {
                if let Some(bundle_parent) = resolve_macos_bundle_parent(exe_dir) {
                    if !is_restricted_macos_bundle_parent(&bundle_parent) {
                        let portable_dir = bundle_parent.join("pacer-workspace");
                        if is_dir_writable(&portable_dir) {
                            return (portable_dir, true);
                        }
                    }
                    let _ = std::fs::create_dir_all(fallback_app_data);
                    return (fallback_app_data.to_path_buf(), false);
                }
            }
            let portable_dir = exe_dir.join("pacer-workspace");
            if is_dir_writable(&portable_dir) {
                return (portable_dir, true);
            }
        }
    }
    let _ = std::fs::create_dir_all(fallback_app_data);
    (fallback_app_data.to_path_buf(), false)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .enable_macos_default_menu(false)
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            #[cfg(target_os = "macos")]
            {
                if let Ok(menu) = build_macos_menu(app.handle()) {
                    let _ = app.set_menu(menu);
                }
            }
            let fallback_app_data = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::env::temp_dir().join("pacer"));
            let (workspace_dir, is_portable) = resolve_workspace_dir(&fallback_app_data);
            migrate_legacy_storage(&workspace_dir, &fallback_app_data);
            for sub in ["consoles", "plugins", "docs"] {
                let _ = std::fs::create_dir_all(workspace_dir.join(sub));
            }
            app.manage(AppState::new(workspace_dir, is_portable));
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(2));
                interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                let mut ticks: u32 = 0;
                loop {
                    interval.tick().await;
                    ticks = ticks.wrapping_add(1);
                    let st = handle.state::<AppState>();
                    st.flush_dirty_history().await;
                    if ticks.is_multiple_of(30) {
                        let _ = st.evict_idle_backends().await;
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_workspace_info_cmd,
            commands::save_console_scratch_cmd,
            commands::list_local_plugins_cmd,
            commands::create_sample_plugin_cmd,
            commands::list_projects_cmd,
            commands::save_project_cmd,
            commands::delete_project_cmd,
            commands::assign_connection_project_cmd,
            commands::list_connections,
            commands::save_connection,
            commands::delete_connection,
            commands::test_connection_cmd,
            commands::disconnect_connection_cmd,
            commands::list_active_connections_cmd,
            commands::test_connection_draft_cmd,
            commands::create_demo_sqlite_connection,
            commands::discover_dbeaver_cmd,
            commands::import_dbeaver_path_cmd,
            commands::import_dbeaver_text_cmd,
            commands::export_dbeaver_json_cmd,
            commands::export_dbeaver_dbp_cmd,
            commands::export_dbeaver_folder_cmd,
            commands::export_connections_xlsx_cmd,
            commands::export_rows_xlsx_cmd,
            commands::read_xlsx_sheet_cmd,
            commands::list_databases_cmd,
            commands::list_databases_draft_cmd,
            commands::list_tables_cmd,
            commands::list_columns_cmd,
            commands::list_indexes_cmd,
            commands::list_foreign_keys_cmd,
            commands::list_routines_cmd,
            commands::get_table_ddl_cmd,
            commands::execute_sql_cmd,
            commands::run_sql_file_cmd,
            commands::export_query_to_file_cmd,
            commands::cancel_query_cmd,
            commands::get_transaction_status_cmd,
            commands::set_transaction_mode_cmd,
            commands::commit_transaction_cmd,
            commands::rollback_transaction_cmd,
            commands::explain_sql_cmd,
            commands::analyze_sql_cmd,
            commands::format_sql_cmd,
            commands::extract_statement_at_cursor_cmd,
            commands::extract_markdown_sql_blocks_cmd,
            commands::open_text_file_cmd,
            commands::save_text_file_cmd,
            commands::list_workspace_files_cmd,
            commands::create_demo_markdown_file_cmd,
            commands::reveal_in_explorer_cmd,
            commands::list_query_history_cmd,
            commands::toggle_history_pin_cmd,
            commands::delete_history_entry_cmd,
            commands::list_extensions_cmd,
            commands::search_objects_cmd,
            commands::export_sql_dump_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Pacer application");
}
