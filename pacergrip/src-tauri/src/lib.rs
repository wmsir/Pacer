pub mod commands;
pub mod files;

use std::path::Path;
use tauri::Manager;

use commands::AppState;

/// One-time migration helper preserving saved connections & encrypted credentials when
/// upgrading from the legacy `cn.fwpt.pacergrip` identifier to `cn.fwpt.pacer`.
fn migrate_legacy_storage(new_dir: &Path) {
    if new_dir.join("connections.json").is_file() {
        return;
    }
    let Some(parent) = new_dir.parent() else {
        return;
    };
    let legacy_dir = parent.join("cn.fwpt.pacergrip");
    if !legacy_dir.is_dir() {
        return;
    }
    let _ = std::fs::create_dir_all(new_dir);
    for file in ["connections.json", "credentials.dat", "demo_store.sqlite", "示例查询手册.md"] {
        let src = legacy_dir.join(file);
        let dst = new_dir.join(file);
        if src.is_file() && !dst.is_file() {
            let _ = std::fs::copy(&src, &dst);
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::env::temp_dir().join("pacer"));
            migrate_legacy_storage(&data_dir);
            app.manage(AppState::new(data_dir));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
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
            commands::list_query_history_cmd,
            commands::list_extensions_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Pacer application");
}
