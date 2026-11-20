use serde::{Deserialize, Serialize};
use sqlparser::ast::{SelectItem, SetExpr, Statement};
use sqlparser::dialect::{
    Dialect, GenericDialect, MsSqlDialect, MySqlDialect, PostgreSqlDialect, SQLiteDialect,
};
use sqlparser::parser::Parser;

use pacer_core::DbEngineType;

/// High-level access category for routing between direct/read-only query and Pacer ticket flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SqlAccessKind {
    ReadOnly,
    WriteDml,
    WriteDdl,
    AdminOrDcl,
    Unknown,
}

/// Risk severity attached to static AST/lexical inspections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RiskLevel {
    Info,
    Medium,
    High,
    Critical,
}

/// Static risk warning emitted for a SQL statement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SqlRiskWarning {
    pub code: String,
    pub level: RiskLevel,
    pub message: String,
    pub statement_index: usize,
}

/// Per-statement analysis summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatementAnalysis {
    pub index: usize,
    pub sql: String,
    pub access_kind: SqlAccessKind,
    pub statement_type: String,
    pub expected_rows_hint: Option<i64>,
    pub warnings: Vec<SqlRiskWarning>,
}

/// Whole-script analysis returned to the editor ribbon and execution router.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptAnalysis {
    pub overall_kind: SqlAccessKind,
    pub requires_ticket: bool,
    pub statements: Vec<StatementAnalysis>,
    pub total_expected_rows_hint: Option<i64>,
}

/// Splits and analyzes a SQL script using `sqlparser` (with lexical fallback for proprietary dialects).
pub fn analyze_sql(sql: &str, db_type: &DbEngineType) -> ScriptAnalysis {
    let raw_chunks = split_sql_statements(sql);
    let dialect = dialect_for_engine(db_type);

    let mut statements = Vec::new();
    for raw_chunk in raw_chunks {
        let trimmed = raw_chunk.trim();
        if trimmed.is_empty() {
            continue;
        }
        let body = strip_sql_comments(trimmed);
        if body.trim().is_empty() {
            continue;
        }

        let stmt_index = statements.len();
        let expected_rows_hint = extract_expected_rows_hint(trimmed);
        let analysis = analyze_single_statement(
            stmt_index,
            trimmed.to_string(),
            &body,
            expected_rows_hint,
            dialect.as_ref(),
        );
        statements.push(analysis);
    }

    let overall_kind = aggregate_access_kind(&statements);
    let requires_ticket = !statements.is_empty() && !matches!(overall_kind, SqlAccessKind::ReadOnly);

    let sum_hints: Vec<i64> = statements
        .iter()
        .filter_map(|s| s.expected_rows_hint)
        .collect();
    let total_expected_rows_hint = if sum_hints.is_empty() {
        None
    } else {
        Some(sum_hints.into_iter().sum())
    };

    ScriptAnalysis {
        overall_kind,
        requires_ticket,
        statements,
        total_expected_rows_hint,
    }
}

fn dialect_for_engine(db_type: &DbEngineType) -> Box<dyn Dialect> {
    match db_type {
        DbEngineType::MySql | DbEngineType::MariaDb | DbEngineType::TiDb | DbEngineType::OceanBase => {
            Box::new(MySqlDialect {})
        }
        DbEngineType::PostgreSql | DbEngineType::OpenGauss | DbEngineType::Kingbase => {
            Box::new(PostgreSqlDialect {})
        }
        DbEngineType::SqlServer => Box::new(MsSqlDialect {}),
        DbEngineType::Sqlite => Box::new(SQLiteDialect {}),
        DbEngineType::Oracle | DbEngineType::Dameng | DbEngineType::ClickHouse | DbEngineType::Unknown(_) => {
            Box::new(GenericDialect {})
        }
    }
}

fn analyze_single_statement(
    index: usize,
    raw_sql: String,
    clean_body: &str,
    expected_rows_hint: Option<i64>,
    dialect: &dyn Dialect,
) -> StatementAnalysis {
    let mut warnings = Vec::new();

    match Parser::parse_sql(dialect, clean_body) {
        Ok(parsed) if !parsed.is_empty() => {
            let stmt = &parsed[0];
            let (access_kind, statement_type) = classify_ast_statement(index, stmt, &mut warnings);
            StatementAnalysis {
                index,
                sql: raw_sql,
                access_kind,
                statement_type,
                expected_rows_hint,
                warnings,
            }
        }
        _ => classify_lexically(index, raw_sql, clean_body, expected_rows_hint),
    }
}

fn classify_ast_statement(
    index: usize,
    stmt: &Statement,
    warnings: &mut Vec<SqlRiskWarning>,
) -> (SqlAccessKind, String) {
    match stmt {
        Statement::Query(query) => {
            if let Some(inner_stmt) = unwrap_write_set_expr(query.body.as_ref()) {
                return classify_ast_statement(index, inner_stmt, warnings);
            }
            let has_limit = query.limit.is_some() || query.fetch.is_some();
            if !has_limit && query_has_wildcard(query.body.as_ref()) {
                warnings.push(SqlRiskWarning {
                    code: "SELECT_NO_LIMIT".to_string(),
                    level: RiskLevel::Info,
                    message: "SELECT * without LIMIT clause; Pacer proxy will cap rows by MaxRows".to_string(),
                    statement_index: index,
                });
            }
            (SqlAccessKind::ReadOnly, "SELECT".to_string())
        }
        Statement::Explain { .. } | Statement::ExplainTable { .. } => {
            (SqlAccessKind::ReadOnly, "EXPLAIN".to_string())
        }
        Statement::ShowTables { .. }
        | Statement::ShowColumns { .. }
        | Statement::ShowCreate { .. }
        | Statement::ShowVariable { .. }
        | Statement::ShowVariables { .. }
        | Statement::ShowStatus { .. }
        | Statement::ShowFunctions { .. } => (SqlAccessKind::ReadOnly, "SHOW".to_string()),
        Statement::Insert(_) => (SqlAccessKind::WriteDml, "INSERT".to_string()),
        Statement::Update { selection, .. } => {
            if selection.is_none() {
                warnings.push(SqlRiskWarning {
                    code: "DML_NO_WHERE".to_string(),
                    level: RiskLevel::High,
                    message: "UPDATE statement lacks a WHERE clause and may modify the entire table".to_string(),
                    statement_index: index,
                });
            }
            (SqlAccessKind::WriteDml, "UPDATE".to_string())
        }
        Statement::Delete(del) => {
            if del.selection.is_none() {
                warnings.push(SqlRiskWarning {
                    code: "DML_NO_WHERE".to_string(),
                    level: RiskLevel::High,
                    message: "DELETE statement lacks a WHERE clause and may wipe the entire table".to_string(),
                    statement_index: index,
                });
            }
            (SqlAccessKind::WriteDml, "DELETE".to_string())
        }
        Statement::Merge { .. } => (SqlAccessKind::WriteDml, "MERGE".to_string()),
        Statement::Drop { .. } => {
            warnings.push(SqlRiskWarning {
                code: "DESTRUCTIVE_DDL".to_string(),
                level: RiskLevel::Critical,
                message: "DROP is destructive and must go through dual-review Pacer approval".to_string(),
                statement_index: index,
            });
            (SqlAccessKind::WriteDdl, "DROP".to_string())
        }
        Statement::Truncate { .. } => {
            warnings.push(SqlRiskWarning {
                code: "DESTRUCTIVE_DDL".to_string(),
                level: RiskLevel::Critical,
                message: "TRUNCATE removes all table rows without transactional undo".to_string(),
                statement_index: index,
            });
            (SqlAccessKind::WriteDdl, "TRUNCATE".to_string())
        }
        Statement::AlterTable { .. } | Statement::AlterIndex { .. } | Statement::AlterView { .. } => {
            (SqlAccessKind::WriteDdl, "ALTER".to_string())
        }
        Statement::CreateTable(_)
        | Statement::CreateIndex(_)
        | Statement::CreateView { .. }
        | Statement::CreateDatabase { .. }
        | Statement::CreateSchema { .. } => (SqlAccessKind::WriteDdl, "CREATE".to_string()),
        Statement::Grant { .. } | Statement::Revoke { .. } | Statement::CreateRole { .. } => {
            (SqlAccessKind::AdminOrDcl, "DCL".to_string())
        }
        other => {
            let debug_head = format!("{other:?}");
            let upper = debug_head.to_ascii_uppercase();
            if upper.starts_with("SHOW") || upper.starts_with("USE") {
                (SqlAccessKind::ReadOnly, "SHOW".to_string())
            } else if upper.starts_with("CREATE") || upper.starts_with("ALTER") {
                (SqlAccessKind::WriteDdl, "DDL".to_string())
            } else {
                (SqlAccessKind::Unknown, "UNKNOWN".to_string())
            }
        }
    }
}

fn unwrap_write_set_expr(body: &SetExpr) -> Option<&Statement> {
    match body {
        SetExpr::Insert(stmt) | SetExpr::Update(stmt) => Some(stmt),
        SetExpr::Query(inner) => unwrap_write_set_expr(inner.body.as_ref()),
        _ => None,
    }
}

fn query_has_wildcard(body: &SetExpr) -> bool {
    match body {
        SetExpr::Select(select) => select.projection.iter().any(|item| {
            matches!(
                item,
                SelectItem::Wildcard(_) | SelectItem::QualifiedWildcard(_, _)
            )
        }),
        SetExpr::Query(inner) => query_has_wildcard(inner.body.as_ref()),
        _ => false,
    }
}

fn classify_lexically(
    index: usize,
    raw_sql: String,
    clean_body: &str,
    expected_rows_hint: Option<i64>,
) -> StatementAnalysis {
    let tokens: Vec<String> = clean_body
        .split_whitespace()
        .take(4)
        .map(|t| t.trim_matches(|c: char| c == '(' || c == ';').to_ascii_uppercase())
        .collect();

    let first = tokens.first().map(String::as_str).unwrap_or("");
    let upper_body = clean_body.to_ascii_uppercase();
    let mut warnings = Vec::new();

    let (access_kind, statement_type) = match first {
        "WITH" => classify_cte_lexically(index, clean_body, &mut warnings),
        "SELECT" | "EXPLAIN" | "SHOW" | "DESC" | "DESCRIBE" | "PRAGMA" => {
            (SqlAccessKind::ReadOnly, first.to_string())
        }
        "INSERT" | "REPLACE" | "MERGE" => (SqlAccessKind::WriteDml, first.to_string()),
        "UPDATE" | "DELETE" => {
            if !contains_word_ci(&upper_body, "WHERE") {
                warnings.push(SqlRiskWarning {
                    code: "DML_NO_WHERE".to_string(),
                    level: RiskLevel::High,
                    message: format!("{first} statement lacks a WHERE clause"),
                    statement_index: index,
                });
            }
            (SqlAccessKind::WriteDml, first.to_string())
        }
        "DROP" | "TRUNCATE" => {
            warnings.push(SqlRiskWarning {
                code: "DESTRUCTIVE_DDL".to_string(),
                level: RiskLevel::Critical,
                message: format!("{first} is a high-risk destructive DDL operation"),
                statement_index: index,
            });
            (SqlAccessKind::WriteDdl, first.to_string())
        }
        "CREATE" | "ALTER" | "RENAME" | "COMMENT" => (SqlAccessKind::WriteDdl, first.to_string()),
        "GRANT" | "REVOKE" => (SqlAccessKind::AdminOrDcl, first.to_string()),
        _ => (
            SqlAccessKind::Unknown,
            if first.is_empty() {
                "UNKNOWN".to_string()
            } else {
                first.to_string()
            },
        ),
    };

    StatementAnalysis {
        index,
        sql: raw_sql,
        access_kind,
        statement_type,
        expected_rows_hint,
        warnings,
    }
}

fn contains_word_ci(upper_haystack: &str, word: &str) -> bool {
    upper_haystack
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .any(|tok| tok == word)
}

fn classify_cte_lexically(
    index: usize,
    clean_body: &str,
    warnings: &mut Vec<SqlRiskWarning>,
) -> (SqlAccessKind, String) {
    // Scan tokens at parenthesis depth 0 (outside string literals) to find the main DML verb after CTE definitions.
    let bytes = clean_body.as_bytes();
    let mut depth = 0usize;
    let mut in_single = false;
    let mut in_double = false;
    let mut i = 0usize;
    let mut top_words = Vec::new();

    while i < bytes.len() {
        let b = bytes[i];
        if in_single {
            if b == b'\'' {
                if bytes.get(i + 1) == Some(&b'\'') {
                    i += 2;
                    continue;
                }
                in_single = false;
            }
            i += 1;
            continue;
        }
        if in_double {
            if b == b'"' {
                in_double = false;
            }
            i += 1;
            continue;
        }
        if b == b'\'' {
            in_single = true;
            i += 1;
            continue;
        }
        if b == b'"' {
            in_double = true;
            i += 1;
            continue;
        }
        if b == b'(' {
            depth += 1;
            i += 1;
            continue;
        }
        if b == b')' {
            depth = depth.saturating_sub(1);
            i += 1;
            continue;
        }
        if depth == 0 && (b.is_ascii_alphabetic() || b == b'_') {
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            top_words.push(clean_body[start..i].to_ascii_uppercase());
            continue;
        }
        i += 1;
    }

    for word in &top_words[1..] {
        match word.as_str() {
            "INSERT" | "REPLACE" | "MERGE" => {
                return (SqlAccessKind::WriteDml, word.clone());
            }
            "UPDATE" | "DELETE" => {
                if !top_words.iter().any(|w| w == "WHERE") {
                    warnings.push(SqlRiskWarning {
                        code: "DML_NO_WHERE".to_string(),
                        level: RiskLevel::High,
                        message: format!("{word} CTE statement lacks a top-level WHERE clause"),
                        statement_index: index,
                    });
                }
                return (SqlAccessKind::WriteDml, word.clone());
            }
            "SELECT" | "VALUES" => {
                return (SqlAccessKind::ReadOnly, "SELECT".to_string());
            }
            _ => {}
        }
    }

    (SqlAccessKind::ReadOnly, "WITH".to_string())
}

fn aggregate_access_kind(statements: &[StatementAnalysis]) -> SqlAccessKind {
    if statements.is_empty() {
        return SqlAccessKind::ReadOnly;
    }
    let mut best = SqlAccessKind::ReadOnly;
    for stmt in statements {
        let rank = kind_rank(stmt.access_kind);
        if rank > kind_rank(best) {
            best = stmt.access_kind;
        }
    }
    best
}

fn kind_rank(kind: SqlAccessKind) -> u8 {
    match kind {
        SqlAccessKind::ReadOnly => 0,
        SqlAccessKind::Unknown => 1,
        SqlAccessKind::WriteDml => 2,
        SqlAccessKind::AdminOrDcl => 3,
        SqlAccessKind::WriteDdl => 4,
    }
}

/// Splits SQL text on top-level `;` while respecting string literals, quoted identifiers, and comments.
pub fn split_sql_statements(sql: &str) -> Vec<String> {
    let bytes = sql.as_bytes();
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut i = 0usize;

    let mut in_single = false;
    let mut in_double = false;
    let mut in_backtick = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;

    while i < bytes.len() {
        let b = bytes[i];
        let next = bytes.get(i + 1).copied();

        if in_line_comment {
            if b == b'\n' {
                in_line_comment = false;
            }
            i += 1;
            continue;
        }
        if in_block_comment {
            if b == b'*' && next == Some(b'/') {
                in_block_comment = false;
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }
        if in_single {
            if b == b'\\' {
                i += 2;
                continue;
            }
            if b == b'\'' {
                if next == Some(b'\'') {
                    i += 2;
                    continue;
                }
                in_single = false;
            }
            i += 1;
            continue;
        }
        if in_double {
            if b == b'"' {
                if next == Some(b'"') {
                    i += 2;
                    continue;
                }
                in_double = false;
            }
            i += 1;
            continue;
        }
        if in_backtick {
            if b == b'`' {
                in_backtick = false;
            }
            i += 1;
            continue;
        }

        if b == b'-' && next == Some(b'-') {
            in_line_comment = true;
            i += 2;
            continue;
        }
        if b == b'/' && next == Some(b'*') {
            in_block_comment = true;
            i += 2;
            continue;
        }
        if b == b'\'' {
            in_single = true;
            i += 1;
            continue;
        }
        if b == b'"' {
            in_double = true;
            i += 1;
            continue;
        }
        if b == b'`' {
            in_backtick = true;
            i += 1;
            continue;
        }

        if b == b';' {
            let chunk = sql[start..i].trim();
            if !chunk.is_empty() {
                out.push(chunk.to_string());
            }
            start = i + 1;
        }
        i += 1;
    }

    if start < sql.len() {
        let tail = sql[start..].trim();
        if !tail.is_empty() {
            out.push(tail.to_string());
        }
    }

    out
}

fn strip_sql_comments(sql: &str) -> String {
    let bytes = sql.as_bytes();
    let mut out = String::with_capacity(sql.len());
    let mut i = 0usize;
    let mut in_single = false;
    let mut in_double = false;
    let mut in_backtick = false;

    while i < bytes.len() {
        let b = bytes[i];
        let next = bytes.get(i + 1).copied();

        if !in_single && !in_double && !in_backtick {
            if b == b'-' && next == Some(b'-') {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                continue;
            }
            if b == b'/' && next == Some(b'*') {
                i += 2;
                while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                i = (i + 2).min(bytes.len());
                out.push(' ');
                continue;
            }
        }

        if b == b'\'' && !in_double && !in_backtick {
            in_single = !in_single;
        } else if b == b'"' && !in_single && !in_backtick {
            in_double = !in_double;
        } else if b == b'`' && !in_single && !in_double {
            in_backtick = !in_backtick;
        }

        // Advance safely by UTF-8 char boundary when copying non-comment content.
        let ch = sql[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }

    out
}

/// Extracts Pacer-compatible expected affected rows hints from SQL `--` comments:
/// `-- 12`, `-- 影响行数: 12`, `-- 影响行数：12`, `-- expected_rows=12`.
pub fn extract_expected_rows_hint(sql: &str) -> Option<i64> {
    for line in sql.lines() {
        if let Some((_, comment_part)) = line.split_once("--") {
            let trimmed = comment_part.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Ok(n) = trimmed.parse::<i64>() {
                return Some(n);
            }
            for prefix in ["影响行数:", "影响行数：", "expected_rows=", "expected_rows:"] {
                let lower = trimmed.to_ascii_lowercase();
                if let Some(rest) = trimmed
                    .strip_prefix(prefix)
                    .or_else(|| lower.strip_prefix(prefix))
                {
                    let digits: String = rest
                        .trim()
                        .chars()
                        .take_while(|c| c.is_ascii_digit())
                        .collect();
                    if let Ok(n) = digits.parse::<i64>() {
                        return Some(n);
                    }
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_only_select_with_and_without_limit() {
        let res = analyze_sql(
            "SELECT * FROM orders; SELECT id, name FROM users LIMIT 20;",
            &DbEngineType::MySql,
        );
        assert_eq!(res.overall_kind, SqlAccessKind::ReadOnly);
        assert!(!res.requires_ticket);
        assert_eq!(res.statements.len(), 2);
        assert_eq!(res.statements[0].warnings.len(), 1);
        assert_eq!(res.statements[0].warnings[0].code, "SELECT_NO_LIMIT");
        assert!(res.statements[1].warnings.is_empty());
    }

    #[test]
    fn test_dml_missing_where_emits_high_risk() {
        let res = analyze_sql(
            "UPDATE orders SET status = 'CLOSED'; DELETE FROM audit_temp;",
            &DbEngineType::MySql,
        );
        assert_eq!(res.overall_kind, SqlAccessKind::WriteDml);
        assert!(res.requires_ticket);
        assert_eq!(res.statements.len(), 2);
        assert_eq!(res.statements[0].warnings[0].level, RiskLevel::High);
        assert_eq!(res.statements[0].warnings[0].code, "DML_NO_WHERE");
        assert_eq!(res.statements[1].warnings[0].level, RiskLevel::High);
    }

    #[test]
    fn test_expected_rows_annotations_summed() {
        let sql = r#"
        -- 3
        UPDATE users SET active = 1 WHERE dept = 'ops';
        -- 影响行数：5
        DELETE FROM temp_logs WHERE id < 10;
        -- expected_rows=2
        INSERT INTO archive(id) VALUES (1), (2);
        "#;
        let res = analyze_sql(sql, &DbEngineType::PostgreSql);
        assert_eq!(res.overall_kind, SqlAccessKind::WriteDml);
        assert_eq!(res.statements.len(), 3);
        assert_eq!(res.statements[0].expected_rows_hint, Some(3));
        assert_eq!(res.statements[1].expected_rows_hint, Some(5));
        assert_eq!(res.statements[2].expected_rows_hint, Some(2));
        assert_eq!(res.total_expected_rows_hint, Some(10));
    }

    #[test]
    fn test_destructive_ddl_overrides_read_only() {
        let sql = "SELECT COUNT(*) FROM t_order; DROP TABLE t_order;";
        let res = analyze_sql(sql, &DbEngineType::MySql);
        assert_eq!(res.overall_kind, SqlAccessKind::WriteDdl);
        assert!(res.requires_ticket);
        assert_eq!(res.statements[1].warnings[0].code, "DESTRUCTIVE_DDL");
        assert_eq!(res.statements[1].warnings[0].level, RiskLevel::Critical);
    }

    #[test]
    fn test_semicolon_inside_string_literal_not_split() {
        let sql = "SELECT 'a;b;c' AS txt; SHOW TABLES;";
        let res = analyze_sql(sql, &DbEngineType::MySql);
        assert_eq!(res.statements.len(), 2);
        assert_eq!(res.overall_kind, SqlAccessKind::ReadOnly);
    }

    #[test]
    fn test_lexical_fallback_for_proprietary_syntax() {
        let sql = "ALTER TABLE \"SYSDBA\".\"ORDERS\" ADD COLUMN (\"REMARK\" VARCHAR2(100)) /* dameng */;";
        let res = analyze_sql(sql, &DbEngineType::Dameng);
        assert_eq!(res.statements.len(), 1);
        assert_eq!(res.overall_kind, SqlAccessKind::WriteDdl);
        assert!(res.requires_ticket);
    }

    #[test]
    fn test_cte_with_insert_classified_as_dml_and_cte_select_as_readonly() {
        let cte_insert = "WITH RECURSIVE cnt(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM cnt WHERE x < 5) INSERT INTO items_big SELECT x FROM cnt;";
        let res_ins = analyze_sql(cte_insert, &DbEngineType::Sqlite);
        assert_eq!(res_ins.overall_kind, SqlAccessKind::WriteDml);
        assert!(res_ins.requires_ticket);

        let cte_select = "WITH recent AS (SELECT id FROM orders) SELECT * FROM recent LIMIT 5;";
        let res_sel = analyze_sql(cte_select, &DbEngineType::Sqlite);
        assert_eq!(res_sel.overall_kind, SqlAccessKind::ReadOnly);
        assert!(!res_sel.requires_ticket);
    }
}
