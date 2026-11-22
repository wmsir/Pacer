use std::collections::BTreeMap;

use pacer_core::DbEngineType;
use serde_json::Value;

const DEFAULT_BATCH_SIZE: usize = 200;

/// Options controlling how a table DDL and data dump are formatted.
#[derive(Debug, Clone)]
pub struct SqlDumpFormatOptions<'a> {
    pub engine: &'a DbEngineType,
    pub include_data: bool,
    pub batch_size: usize,
}

impl<'a> SqlDumpFormatOptions<'a> {
    pub fn new(engine: &'a DbEngineType, include_data: bool) -> Self {
        Self {
            engine,
            include_data,
            batch_size: DEFAULT_BATCH_SIZE,
        }
    }
}

/// Quotes a SQL identifier according to the target database dialect.
pub fn quote_sql_ident(name: &str, engine: &DbEngineType) -> String {
    match engine {
        DbEngineType::MySql
        | DbEngineType::MariaDb
        | DbEngineType::TiDb
        | DbEngineType::OceanBase => format!("`{}`", name.replace('`', "``")),
        DbEngineType::SqlServer => format!("[{}]", name.replace(']', "]]")),
        _ => format!("\"{}\"", name.replace('"', "\"\"")),
    }
}

/// Formats a qualified `[db.]table` target for INSERT / SELECT statements.
pub fn quote_qualified_table(db: &str, table: &str, engine: &DbEngineType) -> String {
    let qt = quote_sql_ident(table, engine);
    let clean_db = db.trim();
    if clean_db.is_empty()
        || clean_db.eq_ignore_ascii_case("default")
        || matches!(engine, DbEngineType::Sqlite) && clean_db.eq_ignore_ascii_case("main")
    {
        qt
    } else {
        format!("{}.{}", quote_sql_ident(clean_db, engine), qt)
    }
}

/// Formats a JSON cell value as a dialect-safe SQL literal.
pub fn format_sql_value(val: &Value) -> String {
    match val {
        Value::Null => "NULL".to_string(),
        Value::Bool(b) => {
            if *b {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            }
        }
        Value::Number(n) => n.to_string(),
        Value::String(s) => quote_sql_string(s),
        Value::Array(_) | Value::Object(_) => quote_sql_string(&val.to_string()),
    }
}

fn quote_sql_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for ch in s.chars() {
        match ch {
            '\'' => out.push_str("''"),
            '\\' => out.push_str("\\\\"),
            '\0' => out.push_str("\\0"),
            '\r' => out.push_str("\\r"),
            '\n' => out.push_str("\\n"),
            _ => out.push(ch),
        }
    }
    out.push('\'');
    out
}

/// Builds the top header comment banner for an exported `.sql` dump file.
pub fn build_sql_dump_header(
    db: &str,
    engine: &DbEngineType,
    include_data: bool,
    table_count: usize,
    generated_at: &str,
) -> String {
    let mode = if include_data {
        "Structure + Data (仅结构 + 数据)"
    } else {
        "Structure Only (仅结构)"
    };
    let mut out = String::new();
    out.push_str("-- ============================================================\n");
    out.push_str("-- Pacer Native SQL Dump\n");
    out.push_str(&format!(
        "-- Engine: {} | Schema: {}\n",
        engine.as_pacer_db_type(),
        db
    ));
    out.push_str(&format!(
        "-- Export Mode: {mode} | Tables: {table_count}\n"
    ));
    out.push_str(&format!("-- Generated At: {generated_at}\n"));
    out.push_str("-- ============================================================\n\n");
    if matches!(
        engine,
        DbEngineType::MySql | DbEngineType::MariaDb | DbEngineType::TiDb | DbEngineType::OceanBase
    ) {
        out.push_str("SET NAMES utf8mb4;\nSET FOREIGN_KEY_CHECKS = 0;\n\n");
    }
    out
}

/// Builds the footer trailer for an exported `.sql` dump file.
pub fn build_sql_dump_footer(engine: &DbEngineType) -> &'static str {
    if matches!(
        engine,
        DbEngineType::MySql | DbEngineType::MariaDb | DbEngineType::TiDb | DbEngineType::OceanBase
    ) {
        "\nSET FOREIGN_KEY_CHECKS = 1;\n"
    } else {
        "\n"
    }
}

/// Formats one table's `CREATE` DDL and optional batched `INSERT INTO` rows.
pub fn format_sql_dump_table(
    db: &str,
    table: &str,
    ddl: &str,
    columns: &[String],
    rows: &[BTreeMap<String, Value>],
    opts: SqlDumpFormatOptions<'_>,
) -> String {
    let mut out = String::new();
    out.push_str("-- ----------------------------\n");
    out.push_str(&format!("-- Table structure for {table}\n"));
    out.push_str("-- ----------------------------\n");

    let clean_ddl = ddl.trim();
    if !clean_ddl.is_empty() {
        out.push_str(clean_ddl);
        if !clean_ddl.ends_with(';') && !clean_ddl.starts_with("--") {
            out.push(';');
        }
        out.push_str("\n\n");
    }

    if !opts.include_data || columns.is_empty() || rows.is_empty() {
        return out;
    }

    out.push_str("-- ----------------------------\n");
    out.push_str(&format!("-- Records of {table} ({} rows)\n", rows.len()));
    out.push_str("-- ----------------------------\n");

    let qualified = quote_qualified_table(db, table, opts.engine);
    let col_list: Vec<String> = columns
        .iter()
        .map(|c| quote_sql_ident(c, opts.engine))
        .collect();
    let cols_joined = col_list.join(", ");
    let batch_size = opts.batch_size.max(1);

    for chunk in rows.chunks(batch_size) {
        out.push_str(&format!("INSERT INTO {qualified} ({cols_joined}) VALUES\n"));
        for (idx, row) in chunk.iter().enumerate() {
            let vals: Vec<String> = columns
                .iter()
                .map(|c| row.get(c).map(format_sql_value).unwrap_or_else(|| "NULL".to_string()))
                .collect();
            let sep = if idx + 1 == chunk.len() { ";" } else { "," };
            out.push_str(&format!("  ({}){sep}\n", vals.join(", ")));
        }
        out.push('\n');
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_quote_sql_ident_dialects() {
        assert_eq!(quote_sql_ident("users", &DbEngineType::MySql), "`users`");
        assert_eq!(
            quote_sql_ident("my`tbl", &DbEngineType::TiDb),
            "`my``tbl`"
        );
        assert_eq!(
            quote_sql_ident("orders", &DbEngineType::PostgreSql),
            "\"orders\""
        );
        assert_eq!(
            quote_sql_ident("items", &DbEngineType::SqlServer),
            "[items]"
        );
    }

    #[test]
    fn test_format_sql_value_types_and_escaping() {
        assert_eq!(format_sql_value(&Value::Null), "NULL");
        assert_eq!(format_sql_value(&json!(true)), "TRUE");
        assert_eq!(format_sql_value(&json!(false)), "FALSE");
        assert_eq!(format_sql_value(&json!(42)), "42");
        assert_eq!(format_sql_value(&json!("O'Brien\nline2")), "'O''Brien\\nline2'");
        // Large integer strings encoded safely by drivers stay cleanly quoted or preserved
        assert_eq!(
            format_sql_value(&json!("1892345678901234567")),
            "'1892345678901234567'"
        );
    }

    #[test]
    fn test_format_sql_dump_table_structure_only_and_batched_data() {
        let cols = vec!["id".to_string(), "name".to_string(), "active".to_string()];
        let mut r1 = BTreeMap::new();
        r1.insert("id".to_string(), json!(1));
        r1.insert("name".to_string(), json!("Alice"));
        r1.insert("active".to_string(), json!(true));
        let mut r2 = BTreeMap::new();
        r2.insert("id".to_string(), json!(2));
        r2.insert("name".to_string(), Value::Null);
        r2.insert("active".to_string(), json!(false));
        let mut r3 = BTreeMap::new();
        r3.insert("id".to_string(), json!(3));
        r3.insert("name".to_string(), json!("C'est"));
        r3.insert("active".to_string(), json!(true));
        let rows = vec![r1, r2, r3];

        let struct_only = format_sql_dump_table(
            "shop",
            "users",
            "CREATE TABLE `users` (`id` INT PRIMARY KEY)",
            &cols,
            &rows,
            SqlDumpFormatOptions::new(&DbEngineType::MySql, false),
        );
        assert!(struct_only.contains("CREATE TABLE `users` (`id` INT PRIMARY KEY);"));
        assert!(!struct_only.contains("INSERT INTO"));

        let mut opts = SqlDumpFormatOptions::new(&DbEngineType::MySql, true);
        opts.batch_size = 2;
        let with_data = format_sql_dump_table(
            "shop",
            "users",
            "CREATE TABLE `users` (`id` INT PRIMARY KEY);",
            &cols,
            &rows,
            opts,
        );
        assert_eq!(with_data.matches("INSERT INTO `shop`.`users`").count(), 2);
        assert!(with_data.contains("(1, 'Alice', TRUE),"));
        assert!(with_data.contains("(2, NULL, FALSE);"));
        assert!(with_data.contains("(3, 'C''est', TRUE);"));
    }

    #[test]
    fn test_postgres_dump_header_and_quoting() {
        let header = build_sql_dump_header(
            "public",
            &DbEngineType::PostgreSql,
            true,
            3,
            "2026-11-23 02:00:00",
        );
        assert!(!header.contains("FOREIGN_KEY_CHECKS"));
        let cols = vec!["id".to_string()];
        let mut r1 = BTreeMap::new();
        r1.insert("id".to_string(), json!(99));
        let dump = format_sql_dump_table(
            "public",
            "orders",
            "CREATE TABLE \"orders\" (id INT);",
            &cols,
            &[r1],
            SqlDumpFormatOptions::new(&DbEngineType::PostgreSql, true),
        );
        assert!(dump.contains("INSERT INTO \"public\".\"orders\" (\"id\") VALUES"));
        assert!(dump.contains("  (99);"));
    }
}
