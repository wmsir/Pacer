pub mod dbeaver;
pub mod jdbc;
pub mod matcher;
pub mod sql_dump;
pub mod sql_script;
pub mod xlsx;

pub use dbeaver::{
    decrypt_dbeaver_credentials, discover_local_dbeaver_files, encrypt_dbeaver_credentials,
    export_to_dbeaver_dbp_bytes, export_to_dbeaver_folder, export_to_dbeaver_json,
    export_to_dbeaver_json_with_folders, import_dbeaver_from_path, import_dbeaver_zip,
    parse_dbeaver_json, parse_dbeaver_text, parse_dbeaver_xml, DbeaverCredentialPair,
    ImportedDbeaverConnection,
};
pub use jdbc::{build_jdbc_url, parse_connection_url, parse_jdbc_url, ParsedJdbcUrl};
pub use matcher::{match_profiles_with_pacer, MatchKind, MatchResult};
pub use sql_dump::{
    build_limited_select_sql, build_sql_dump_footer, build_sql_dump_header,
    format_sql_dump_table, format_sql_value, quote_qualified_table, quote_sql_ident,
    write_sql_dump_insert_batch, write_sql_dump_table_header, SqlDumpFormatOptions,
};
pub use sql_script::SqlScriptStreamReader;
pub use xlsx::{
    build_xlsx_bytes, col_index_to_excel_letters, read_xlsx_sheet_preview, write_csv_row,
    write_xlsx_file, StreamingXlsxWriter,
};
