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

/// Prefers `<exe_dir>/pacer-workspace` so portable builds keep all projects, connections,
/// temporary console scripts, and local plugins next to Pacer.exe; falls back to AppData if read-only.
fn resolve_workspace_dir(fallback_app_data: &Path) -> (std::path::PathBuf, bool) {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
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
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
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
            commands::test_connection_draft_cmd,
            commands::create_demo_sqlite_connection,
            commands::discover_dbeaver_cmd,
            commands::import_dbeaver_path_cmd,
            commands::import_dbeaver_text_cmd,
            commands::export_dbeaver_json_cmd,
            commands::list_databases_cmd,
            commands::list_tables_cmd,
            commands::list_columns_cmd,
            commands::list_indexes_cmd,
            commands::get_table_ddl_cmd,
            commands::execute_sql_cmd,
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
            commands::list_extensions_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Pacer application");
}
