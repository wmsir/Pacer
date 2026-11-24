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

/// Builds a dialect-appropriate `SELECT *` query bounded by `limit` rows.
pub fn build_limited_select_sql(
    db: &str,
    table: &str,
    engine: &DbEngineType,
    limit: u32,
) -> String {
    let qualified = quote_qualified_table(db, table, engine);
    match engine {
        DbEngineType::SqlServer => format!("SELECT TOP {limit} * FROM {qualified}"),
        DbEngineType::Oracle => format!("SELECT * FROM {qualified} FETCH FIRST {limit} ROWS ONLY"),
        _ => format!("SELECT * FROM {qualified} LIMIT {limit}"),
    }
}

/// Formats a JSON cell value as a dialect-safe SQL literal.
pub fn format_sql_value(val: &Value) -> String {
    let mut out = String::new();
    push_sql_value(&mut out, val);
    out
}

fn push_sql_value(out: &mut String, val: &Value) {
    use std::fmt::Write as _;
    match val {
        Value::Null => out.push_str("NULL"),
        Value::Bool(b) => out.push_str(if *b { "TRUE" } else { "FALSE" }),
        Value::Number(n) => {
            let _ = write!(out, "{n}");
        }
        Value::String(s) => push_quoted_sql_string(out, s),
        Value::Array(_) | Value::Object(_) => push_quoted_sql_string(out, &val.to_string()),
    }
}

fn push_quoted_sql_string(out: &mut String, s: &str) {
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

/// Writes one table's comment banner and `CREATE` DDL directly to `writer`.
pub fn write_sql_dump_table_header<W: std::io::Write>(
    writer: &mut W,
    table: &str,
    ddl: &str,
) -> std::io::Result<()> {
    writeln!(writer, "-- ----------------------------")?;
    writeln!(writer, "-- Table structure for {table}")?;
    writeln!(writer, "-- ----------------------------")?;

    let clean_ddl = ddl.trim();
    if !clean_ddl.is_empty() {
        writer.write_all(clean_ddl.as_bytes())?;
        if !clean_ddl.ends_with(';') && !clean_ddl.starts_with("--") {
            writer.write_all(b";")?;
        }
        writer.write_all(b"\n\n")?;
    }
    Ok(())
}

/// Streams a batch of rows as `INSERT INTO` statements directly into `writer` without buffering
/// the entire table in memory.
pub fn write_sql_dump_insert_batch<W: std::io::Write>(
    writer: &mut W,
    db: &str,
    table: &str,
    columns: &[String],
    rows: &[BTreeMap<String, Value>],
    opts: SqlDumpFormatOptions<'_>,
    include_banner: bool,
) -> std::io::Result<usize> {
    if !opts.include_data || columns.is_empty() || rows.is_empty() {
        return Ok(0);
    }

    if include_banner {
        writeln!(writer, "-- ----------------------------")?;
        writeln!(writer, "-- Records of {table}")?;
        writeln!(writer, "-- ----------------------------")?;
    }

    let qualified = quote_qualified_table(db, table, opts.engine);
    let cols_joined: String = columns
        .iter()
        .map(|c| quote_sql_ident(c, opts.engine))
        .collect::<Vec<_>>()
        .join(", ");
    let batch_size = opts.batch_size.max(1);
    let mut written = 0usize;
    let mut line_buf = String::with_capacity(256);

    for chunk in rows.chunks(batch_size) {
        writeln!(writer, "INSERT INTO {qualified} ({cols_joined}) VALUES")?;
        for (idx, row) in chunk.iter().enumerate() {
            line_buf.clear();
            line_buf.push_str("  (");
            for (c_idx, col) in columns.iter().enumerate() {
                if c_idx > 0 {
                    line_buf.push_str(", ");
                }
                match row.get(col) {
                    Some(val) => push_sql_value(&mut line_buf, val),
                    None => line_buf.push_str("NULL"),
                }
            }
            if idx + 1 == chunk.len() {
                line_buf.push_str(");\n");
            } else {
                line_buf.push_str("),\n");
            }
            writer.write_all(line_buf.as_bytes())?;
            written += 1;
        }
        writer.write_all(b"\n")?;
    }

    Ok(written)
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
    use std::io::Write as _;
    let mut buf = Vec::with_capacity(512 + rows.len() * 64);
    let _ = write_sql_dump_table_header(&mut buf, table, ddl);
    if opts.include_data && !columns.is_empty() && !rows.is_empty() {
        let _ = writeln!(&mut buf, "-- ----------------------------");
        let _ = writeln!(&mut buf, "-- Records of {table} ({} rows)", rows.len());
        let _ = writeln!(&mut buf, "-- ----------------------------");
        let _ = write_sql_dump_insert_batch(&mut buf, db, table, columns, rows, opts, false);
    }
    String::from_utf8_lossy(&buf).into_owned()
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

    #[test]
    fn test_streaming_sql_dump_batches_and_dialects() {
        assert_eq!(
            build_limited_select_sql("shop", "orders", &DbEngineType::MySql, 500),
            "SELECT * FROM `shop`.`orders` LIMIT 500"
        );
        assert_eq!(
            build_limited_select_sql("dbo", "orders", &DbEngineType::SqlServer, 500),
            "SELECT TOP 500 * FROM [dbo].[orders]"
        );

        let cols = vec!["id".to_string(), "note".to_string()];
        let mut r1 = BTreeMap::new();
        r1.insert("id".to_string(), json!(1));
        r1.insert("note".to_string(), json!("one"));
        let mut r2 = BTreeMap::new();
        r2.insert("id".to_string(), json!(2));
        r2.insert("note".to_string(), json!("two"));

        let mut sink = Vec::new();
        write_sql_dump_table_header(&mut sink, "orders", "CREATE TABLE `orders` (`id` INT);").unwrap();
        let n1 = write_sql_dump_insert_batch(
            &mut sink,
            "shop",
            "orders",
            &cols,
            &[r1],
            SqlDumpFormatOptions::new(&DbEngineType::MySql, true),
            true,
        )
        .unwrap();
        let n2 = write_sql_dump_insert_batch(
            &mut sink,
            "shop",
            "orders",
            &cols,
            &[r2],
            SqlDumpFormatOptions::new(&DbEngineType::MySql, true),
            false,
        )
        .unwrap();
        assert_eq!(n1 + n2, 2);
        let text = String::from_utf8(sink).unwrap();
        assert_eq!(text.matches("-- Records of orders").count(), 1);
        assert_eq!(text.matches("INSERT INTO `shop`.`orders`").count(), 2);
        assert!(text.contains("(1, 'one');"));
        assert!(text.contains("(2, 'two');"));
    }
}
