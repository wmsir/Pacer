pub mod commands;

use tauri::Manager;

use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::env::temp_dir().join("pacergrip"));
            app.manage(AppState::new(data_dir));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_connections,
            commands::save_connection,
            commands::delete_connection,
            commands::test_connection_cmd,
            commands::create_demo_sqlite_connection,
            commands::import_datagrip_xml_cmd,
            commands::import_datagrip_path_cmd,
            commands::export_datagrip_xml_cmd,
            commands::list_databases_cmd,
            commands::list_tables_cmd,
            commands::list_columns_cmd,
            commands::get_table_ddl_cmd,
            commands::execute_sql_cmd,
            commands::explain_sql_cmd,
            commands::analyze_sql_cmd,
            commands::list_query_history_cmd,
            commands::list_extensions_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running PacerGrip application");
}
