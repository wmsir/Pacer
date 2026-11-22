use serde::{Deserialize, Serialize};
use sqlparser::ast::{SelectItem, SetExpr, Statement};
use sqlparser::dialect::{
    Dialect, GenericDialect, MsSqlDialect, MySqlDialect, PostgreSqlDialect, SQLiteDialect,
};
use sqlparser::parser::Parser;

use pacer_core::{DbEngineType, MarkdownSqlBlock};

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
    if matches!(db_type, DbEngineType::Redis) {
        return analyze_redis_script(sql);
    }

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
        DbEngineType::Oracle
        | DbEngineType::Dameng
        | DbEngineType::ClickHouse
        | DbEngineType::Redis
        | DbEngineType::Unknown(_) => Box::new(GenericDialect {}),
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

/// Returns the individual SQL statement span containing `cursor_char_offset` (0-based UTF-8 char index).
pub fn extract_statement_at_cursor(sql: &str, cursor_char_offset: usize) -> Option<String> {
    let cursor_byte = sql
        .char_indices()
        .nth(cursor_char_offset)
        .map(|(idx, _)| idx)
        .unwrap_or(sql.len());

    let spans = split_sql_statement_spans(sql);
    if spans.is_empty() {
        return None;
    }

    // Prefer the span directly containing cursor_byte; otherwise pick the nearest preceding non-empty span.
    for (start, end) in &spans {
        if cursor_byte >= *start && cursor_byte <= *end {
            let candidate = sql[*start..*end].trim();
            if !candidate.is_empty() {
                return Some(candidate.to_string());
            }
        }
    }

    for (start, end) in spans.iter().rev() {
        if *start <= cursor_byte {
            let candidate = sql[*start..*end].trim();
            if !candidate.is_empty() {
                return Some(candidate.to_string());
            }
        }
    }

    spans
        .first()
        .map(|(s, e)| sql[*s..*e].trim().to_string())
        .filter(|s| !s.is_empty())
}

fn split_sql_statement_spans(sql: &str) -> Vec<(usize, usize)> {
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
            if !sql[start..i].trim().is_empty() {
                out.push((start, i));
            }
            start = i + 1;
        }
        i += 1;
    }

    if start < sql.len() && !sql[start..].trim().is_empty() {
        out.push((start, sql.len()));
    }
    out
}

/// Extracts all fenced SQL code blocks from a Markdown document with 1-based line numbers.
pub fn extract_markdown_sql_blocks(markdown: &str) -> Vec<MarkdownSqlBlock> {
    let lines: Vec<&str> = markdown.lines().collect();
    let mut blocks = Vec::new();
    let mut last_heading: Option<String> = None;

    let mut i = 0usize;
    while i < lines.len() {
        let trimmed = lines[i].trim_start();
        if let Some(heading_rest) = trimmed.strip_prefix('#') {
            let title = heading_rest.trim_start_matches('#').trim();
            if !title.is_empty() {
                last_heading = Some(title.to_string());
            }
            i += 1;
            continue;
        }

        let fence_char = if trimmed.starts_with("```") {
            Some('`')
        } else if trimmed.starts_with("~~~") {
            Some('~')
        } else {
            None
        };

        if let Some(fc) = fence_char {
            let fence_len = trimmed.chars().take_while(|&c| c == fc).count();
            let info = trimmed[fence_len..].trim();
            let lang_token = info
                .split(|c: char| c.is_whitespace() || c == ',' || c == '{')
                .next()
                .unwrap_or("")
                .to_ascii_lowercase();

            let start_line = i + 1;
            let content_start_line = i + 2;
            let mut j = i + 1;
            let mut body_lines = Vec::new();

            while j < lines.len() {
                let close_trimmed = lines[j].trim();
                let close_len = close_trimmed.chars().take_while(|&c| c == fc).count();
                if close_len >= fence_len && close_trimmed[close_len..].trim().is_empty() {
                    break;
                }
                body_lines.push(lines[j]);
                j += 1;
            }

            let end_line = (j + 1).min(lines.len());
            let content_end_line = if body_lines.is_empty() {
                content_start_line
            } else {
                content_start_line + body_lines.len() - 1
            };

            if is_sql_fence_lang(&lang_token) {
                let sql = body_lines.join("\n").trim().to_string();
                if !sql.is_empty() {
                    blocks.push(MarkdownSqlBlock {
                        index: blocks.len(),
                        language: lang_token,
                        start_line,
                        end_line,
                        content_start_line,
                        content_end_line,
                        sql,
                        title: last_heading.clone(),
                    });
                }
            }

            i = j + 1;
            continue;
        }

        i += 1;
    }

    blocks
}

fn is_sql_fence_lang(lang: &str) -> bool {
    matches!(
        lang,
        "sql"
            | "mysql"
            | "mariadb"
            | "postgresql"
            | "postgres"
            | "pgsql"
            | "plpgsql"
            | "sqlite"
            | "sqlite3"
            | "oracle"
            | "plsql"
            | "tsql"
            | "mssql"
            | "sqlserver"
            | "dm"
            | "dameng"
            | "kingbase"
            | "opengauss"
            | "tidb"
            | "oceanbase"
            | "clickhouse"
            | "ddl"
            | "dml"
    )
}

/// Lightweight, comment- and string-safe SQL formatter inspired by DataGrip's Reformat Code (`Ctrl+Alt+L`).
pub fn format_sql(sql: &str) -> String {
    let stmts = split_sql_statements(sql);
    if stmts.is_empty() {
        return sql.trim().to_string();
    }

    let formatted: Vec<String> = stmts.iter().map(|s| format_single_sql(s)).collect();
    let has_trailing_semi = sql.trim_end().ends_with(';');
    let mut joined = formatted.join(";\n\n");
    if has_trailing_semi || stmts.len() > 1 {
        joined.push(';');
    }
    joined
}

fn format_single_sql(raw: &str) -> String {
    #[derive(Debug)]
    enum Tok {
        Word(String),
        Quoted(String),
        Comment(String),
        Punct(char),
    }

    let chars: Vec<char> = raw.chars().collect();
    let mut toks = Vec::new();
    let mut i = 0usize;

    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c == '-' && chars.get(i + 1) == Some(&'-') {
            let start = i;
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            toks.push(Tok::Comment(chars[start..i].iter().collect()));
            continue;
        }
        if c == '/' && chars.get(i + 1) == Some(&'*') {
            let start = i;
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i = (i + 2).min(chars.len());
            toks.push(Tok::Comment(chars[start..i].iter().collect()));
            continue;
        }
        if c == '\'' || c == '"' || c == '`' {
            let quote = c;
            let start = i;
            i += 1;
            while i < chars.len() {
                if chars[i] == '\\' && quote == '\'' {
                    i += 2;
                    continue;
                }
                if chars[i] == quote {
                    i += 1;
                    if chars.get(i) == Some(&quote) {
                        i += 1;
                        continue;
                    }
                    break;
                }
                i += 1;
            }
            toks.push(Tok::Quoted(chars[start..i].iter().collect()));
            continue;
        }
        if c.is_alphanumeric() || c == '_' || c == '$' {
            let start = i;
            while i < chars.len()
                && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '$')
            {
                i += 1;
            }
            toks.push(Tok::Word(chars[start..i].iter().collect()));
            continue;
        }
        toks.push(Tok::Punct(c));
        i += 1;
    }

    let keywords = [
        "SELECT", "DISTINCT", "FROM", "WHERE", "AND", "OR", "NOT", "IN", "IS", "NULL", "AS",
        "ON", "JOIN", "INNER", "LEFT", "RIGHT", "FULL", "OUTER", "CROSS", "GROUP", "BY",
        "HAVING", "ORDER", "ASC", "DESC", "LIMIT", "OFFSET", "UNION", "ALL", "INSERT", "INTO",
        "VALUES", "UPDATE", "SET", "DELETE", "CREATE", "TABLE", "INDEX", "VIEW", "IF", "EXISTS",
        "PRIMARY", "KEY", "FOREIGN", "REFERENCES", "DEFAULT", "UNIQUE", "ALTER", "ADD", "COLUMN",
        "DROP", "TRUNCATE", "CASE", "WHEN", "THEN", "ELSE", "END", "BETWEEN", "LIKE", "WITH",
        "RECURSIVE", "EXPLAIN", "SHOW", "PRAGMA",
    ];

    let mut out = String::new();
    let mut depth = 0usize;
    let mut idx = 0usize;

    while idx < toks.len() {
        match &toks[idx] {
            Tok::Comment(c) => {
                if !out.is_empty() && !out.ends_with('\n') {
                    out.push('\n');
                }
                out.push_str(c);
                out.push('\n');
                idx += 1;
            }
            Tok::Quoted(q) => {
                push_space_if_needed(&mut out);
                out.push_str(q);
                idx += 1;
            }
            Tok::Punct(p) => {
                match p {
                    '(' => {
                        if out.ends_with(' ')
                            || out.is_empty()
                            || out.ends_with(|ch: char| ch.is_alphanumeric() || ch == '_')
                        {
                            out.push('(');
                        } else {
                            out.push(' ');
                            out.push('(');
                        }
                        depth += 1;
                    }
                    ')' => {
                        depth = depth.saturating_sub(1);
                        out.push(')');
                    }
                    ',' => {
                        out.push(',');
                        out.push(' ');
                    }
                    '.' => {
                        out.push('.');
                    }
                    _ => {
                        push_space_if_needed(&mut out);
                        out.push(*p);
                    }
                }
                idx += 1;
            }
            Tok::Word(w) => {
                let upper = w.to_ascii_uppercase();
                let next_upper = match toks.get(idx + 1) {
                    Some(Tok::Word(nw)) => Some(nw.to_ascii_uppercase()),
                    _ => None,
                };

                if depth == 0 {
                    // Two-word clause headers.
                    if let Some(ref nu) = next_upper {
                        let two = format!("{upper} {nu}");
                        if matches!(
                            two.as_str(),
                            "LEFT JOIN"
                                | "RIGHT JOIN"
                                | "INNER JOIN"
                                | "FULL JOIN"
                                | "CROSS JOIN"
                                | "GROUP BY"
                                | "ORDER BY"
                                | "UNION ALL"
                                | "INSERT INTO"
                                | "DELETE FROM"
                        ) {
                            if !out.is_empty() && !out.ends_with('\n') {
                                out.push('\n');
                            }
                            out.push_str(&two);
                            idx += 2;
                            continue;
                        }
                    }

                    if matches!(
                        upper.as_str(),
                        "SELECT" | "FROM" | "WHERE" | "JOIN" | "HAVING" | "LIMIT" | "OFFSET"
                            | "SET" | "VALUES" | "UNION" | "UPDATE"
                    ) {
                        if !out.is_empty() && !out.ends_with('\n') {
                            out.push('\n');
                        }
                        out.push_str(&upper);
                        idx += 1;
                        continue;
                    }

                    if matches!(upper.as_str(), "AND" | "OR") {
                        if !out.is_empty() && !out.ends_with('\n') {
                            out.push('\n');
                        }
                        out.push_str("  ");
                        out.push_str(&upper);
                        idx += 1;
                        continue;
                    }
                }

                push_space_if_needed(&mut out);
                if keywords.contains(&upper.as_str()) {
                    out.push_str(&upper);
                } else {
                    out.push_str(w);
                }
                idx += 1;
            }
        }
    }

    out.trim().to_string()
}

fn push_space_if_needed(out: &mut String) {
    if out.is_empty()
        || out.ends_with('\n')
        || out.ends_with(' ')
        || out.ends_with('(')
        || out.ends_with('.')
    {
        return;
    }
    out.push(' ');
}

/// Splits a Redis script into non-empty executable command lines, stripping `--`, `#`, and `//` comment lines
/// as well as optional trailing semicolons outside quotes.
pub fn split_redis_commands(script: &str) -> Vec<String> {
    let mut out = Vec::new();
    for raw_line in script.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty()
            || trimmed.starts_with("--")
            || trimmed.starts_with('#')
            || trimmed.starts_with("//")
        {
            continue;
        }
        let cleaned = strip_trailing_semicolon_outside_quotes(trimmed).trim();
        if !cleaned.is_empty() {
            out.push(cleaned.to_string());
        }
    }
    out
}

fn strip_trailing_semicolon_outside_quotes(input: &str) -> &str {
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;
    let mut last_outside = false;
    for ch in input.chars() {
        if escaped {
            escaped = false;
            last_outside = false;
            continue;
        }
        if (in_single || in_double) && ch == '\\' {
            escaped = true;
            last_outside = false;
            continue;
        }
        if ch == '"' && !in_single {
            in_double = !in_double;
            last_outside = false;
            continue;
        }
        if ch == '\'' && !in_double {
            in_single = !in_single;
            last_outside = false;
            continue;
        }
        last_outside = !in_single && !in_double && ch == ';';
    }
    if last_outside {
        input.strip_suffix(';').unwrap_or(input)
    } else {
        input
    }
}

/// Tokenizes a single Redis command line into arguments, supporting double quotes (`"..."`),
/// single quotes (`'...'`), and backslash escapes inside double/single quotes.
pub fn tokenize_redis_command(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut has_token = false;
    let mut quote: Option<char> = None;
    let mut chars = line.trim().chars().peekable();

    while let Some(ch) = chars.next() {
        if let Some(q) = quote {
            if ch == '\\' {
                if let Some(next_ch) = chars.next() {
                    match next_ch {
                        'n' => current.push('\n'),
                        'r' => current.push('\r'),
                        't' => current.push('\t'),
                        '\\' => current.push('\\'),
                        other if other == q => current.push(other),
                        other => {
                            current.push('\\');
                            current.push(other);
                        }
                    }
                }
                continue;
            }
            if ch == q {
                quote = None;
                has_token = true;
                continue;
            }
            current.push(ch);
            continue;
        }

        if ch.is_whitespace() {
            if has_token {
                tokens.push(std::mem::take(&mut current));
                has_token = false;
            }
            continue;
        }
        if ch == '"' || ch == '\'' {
            quote = Some(ch);
            has_token = true;
            continue;
        }
        current.push(ch);
        has_token = true;
    }

    if has_token || !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

/// Analyzes a Redis command script for read-only vs write/admin safety and blocking/destructive risks.
pub fn analyze_redis_script(sql: &str) -> ScriptAnalysis {
    let commands = split_redis_commands(sql);
    let mut statements = Vec::with_capacity(commands.len());

    for (idx, cmd_text) in commands.into_iter().enumerate() {
        let tokens = tokenize_redis_command(&cmd_text);
        let (access_kind, statement_type, warnings) = classify_redis_command(idx, &tokens);
        statements.push(StatementAnalysis {
            index: idx,
            sql: cmd_text,
            access_kind,
            statement_type,
            expected_rows_hint: None,
            warnings,
        });
    }

    let overall_kind = aggregate_access_kind(&statements);
    let requires_ticket = !statements.is_empty() && !matches!(overall_kind, SqlAccessKind::ReadOnly);

    ScriptAnalysis {
        overall_kind,
        requires_ticket,
        statements,
        total_expected_rows_hint: None,
    }
}

fn classify_redis_command(
    statement_index: usize,
    tokens: &[String],
) -> (SqlAccessKind, String, Vec<SqlRiskWarning>) {
    let mut warnings = Vec::new();
    let Some(first) = tokens.first() else {
        return (SqlAccessKind::Unknown, "UNKNOWN".to_string(), warnings);
    };

    let cmd = first.to_ascii_uppercase();
    let sub = tokens
        .get(1)
        .map(|s| s.to_ascii_uppercase())
        .unwrap_or_default();

    // Two-word command disambiguation first.
    match (cmd.as_str(), sub.as_str()) {
        ("CONFIG", "GET") => return (SqlAccessKind::ReadOnly, "CONFIG GET".to_string(), warnings),
        ("CONFIG", "SET" | "REWRITE" | "RESETSTAT") => {
            warnings.push(SqlRiskWarning {
                code: "REDIS_ADMIN_CONFIG".to_string(),
                level: RiskLevel::High,
                message: format!("CONFIG {sub} 会修改 Redis 服务端运行时配置，请谨慎执行。"),
                statement_index,
            });
            return (SqlAccessKind::AdminOrDcl, format!("CONFIG {sub}"), warnings);
        }
        ("ACL", "LIST" | "WHOAMI" | "USERS" | "GETUSER" | "CAT" | "LOG") => {
            return (SqlAccessKind::ReadOnly, format!("ACL {sub}"), warnings)
        }
        ("ACL", _) => {
            warnings.push(SqlRiskWarning {
                code: "REDIS_ACL_MUTATION".to_string(),
                level: RiskLevel::Critical,
                message: "ACL 权限变更命令将修改 Redis 用户权限规则。".to_string(),
                statement_index,
            });
            return (SqlAccessKind::AdminOrDcl, format!("ACL {sub}"), warnings);
        }
        ("CLIENT", "LIST" | "ID" | "GETNAME" | "INFO") => {
            return (SqlAccessKind::ReadOnly, format!("CLIENT {sub}"), warnings)
        }
        ("CLIENT", _) => {
            warnings.push(SqlRiskWarning {
                code: "REDIS_CLIENT_ADMIN".to_string(),
                level: RiskLevel::High,
                message: format!("CLIENT {sub} 可能中断或变更客户端连接状态。"),
                statement_index,
            });
            return (SqlAccessKind::AdminOrDcl, format!("CLIENT {sub}"), warnings);
        }
        ("MEMORY", "USAGE" | "STATS" | "DOCTOR" | "MALLOC-STATS") => {
            return (SqlAccessKind::ReadOnly, format!("MEMORY {sub}"), warnings)
        }
        ("OBJECT", "ENCODING" | "REFCOUNT" | "IDLETIME" | "FREQ") => {
            return (SqlAccessKind::ReadOnly, format!("OBJECT {sub}"), warnings)
        }
        ("SLOWLOG", "GET" | "LEN") => {
            return (SqlAccessKind::ReadOnly, format!("SLOWLOG {sub}"), warnings)
        }
        ("CLUSTER", "INFO" | "NODES" | "SLOTS" | "SHARDS" | "KEYSLOT" | "COUNTKEYSINSLOT") => {
            return (SqlAccessKind::ReadOnly, format!("CLUSTER {sub}"), warnings)
        }
        ("XINFO", _) => return (SqlAccessKind::ReadOnly, "XINFO".to_string(), warnings),
        ("SCRIPT", "EXISTS") => return (SqlAccessKind::ReadOnly, "SCRIPT EXISTS".to_string(), warnings),
        ("SCRIPT", _) => {
            warnings.push(SqlRiskWarning {
                code: "REDIS_ADMIN_SCRIPT".to_string(),
                level: RiskLevel::High,
                message: format!("SCRIPT {sub} 会修改服务端脚本缓存或中止脚本。"),
                statement_index,
            });
            return (SqlAccessKind::AdminOrDcl, format!("SCRIPT {sub}"), warnings);
        }
        _ => {}
    }

    match cmd.as_str() {
        "PING" | "ECHO" | "INFO" | "DBSIZE" | "TIME" | "SCAN" | "TYPE" | "TTL" | "PTTL"
        | "EXPIRETIME" | "PEXPIRETIME" | "EXISTS" | "GET" | "MGET" | "STRLEN" | "GETRANGE"
        | "SUBSTR" | "GETBIT" | "BITCOUNT" | "BITPOS" | "HGET" | "HMGET" | "HGETALL"
        | "HKEYS" | "HVALS" | "HLEN" | "HEXISTS" | "HSCAN" | "HSTRLEN" | "LRANGE" | "LLEN"
        | "LINDEX" | "LPOS" | "SMEMBERS" | "SCARD" | "SISMEMBER" | "SMISMEMBER" | "SSCAN"
        | "SDIFF" | "SINTER" | "SUNION" | "ZRANGE" | "ZREVRANGE" | "ZRANGEBYSCORE"
        | "ZREVRANGEBYSCORE" | "ZRANGEBYLEX" | "ZREVRANGEBYLEX" | "ZCARD" | "ZSCORE"
        | "ZMSCORE" | "ZRANK" | "ZREVRANK" | "ZCOUNT" | "ZLEXCOUNT" | "ZSCAN" | "XLEN"
        | "XRANGE" | "XREVRANGE" | "SELECT" | "ROLE" | "COMMAND" | "LOLWUT" | "PFCOUNT"
        | "GEOPOS" | "GEODIST" | "GEOHASH" | "GEORADIUS_RO" | "GEOSEARCH" | "JSON.GET"
        | "JSON.MGET" | "JSON.TYPE" | "JSON.STRLEN" | "JSON.OBJKEYS" | "JSON.OBJLEN"
        | "JSON.ARRLEN" | "AUTH" | "HELLO" => (SqlAccessKind::ReadOnly, cmd, warnings),

        "KEYS" => {
            warnings.push(SqlRiskWarning {
                code: "REDIS_BLOCKING_KEYS".to_string(),
                level: RiskLevel::High,
                message: "KEYS 命令会阻塞 Redis 主线程，生产环境建议改用非阻塞 SCAN 0 MATCH pattern COUNT 100。".to_string(),
                statement_index,
            });
            (SqlAccessKind::ReadOnly, cmd, warnings)
        }

        "SET" | "MSET" | "MSETNX" | "SETEX" | "PSETEX" | "SETNX" | "GETSET" | "GETDEL"
        | "GETEX" | "APPEND" | "SETRANGE" | "SETBIT" | "INCR" | "INCRBY" | "INCRBYFLOAT"
        | "DECR" | "DECRBY" | "HSET" | "HMSET" | "HSETNX" | "HINCRBY" | "HINCRBYFLOAT"
        | "HDEL" | "LPUSH" | "RPUSH" | "LPUSHX" | "RPUSHX" | "LPOP" | "RPOP" | "LSET"
        | "LREM" | "LTRIM" | "LINSERT" | "SADD" | "SREM" | "SPOP" | "SMOVE" | "ZADD" | "ZREM"
        | "ZINCRBY" | "ZPOPMIN" | "ZPOPMAX" | "ZREMRANGEBYRANK" | "ZREMRANGEBYSCORE"
        | "EXPIRE" | "PEXPIRE" | "EXPIREAT" | "PEXPIREAT" | "PERSIST" | "RENAME" | "RENAMENX"
        | "COPY" | "MOVE" | "XADD" | "XDEL" | "XTRIM" | "PFADD" | "GEOADD" | "JSON.SET"
        | "JSON.DEL" => (SqlAccessKind::WriteDml, cmd, warnings),

        "DEL" | "UNLINK" => {
            warnings.push(SqlRiskWarning {
                code: "REDIS_DELETE_KEY".to_string(),
                level: RiskLevel::High,
                message: format!("{cmd} 将永久删除指定的 Redis Key 及其全部数据。"),
                statement_index,
            });
            (SqlAccessKind::WriteDml, cmd, warnings)
        }

        "FLUSHDB" | "FLUSHALL" | "SWAPDB" => {
            warnings.push(SqlRiskWarning {
                code: "REDIS_FLUSH_ALL".to_string(),
                level: RiskLevel::Critical,
                message: format!("{cmd} 将不可逆清空 Redis 数据库中的所有键值！"),
                statement_index,
            });
            (SqlAccessKind::WriteDdl, cmd, warnings)
        }

        "SHUTDOWN" | "DEBUG" | "SLAVEOF" | "REPLICAOF" | "FAILOVER" | "MODULE" | "MIGRATE"
        | "BGREWRITEAOF" | "BGSAVE" | "SAVE" | "EVAL" | "EVALSHA" => {
            warnings.push(SqlRiskWarning {
                code: "REDIS_ADMIN_COMMAND".to_string(),
                level: RiskLevel::Critical,
                message: format!("{cmd} 属于高危管理或服务端脚本命令，可能影响实例稳定性。"),
                statement_index,
            });
            (SqlAccessKind::AdminOrDcl, cmd, warnings)
        }

        _ => (SqlAccessKind::Unknown, cmd, warnings),
    }
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

    #[test]
    fn test_extract_markdown_sql_blocks() {
        let md = r#"# 订单排查手册

## 1. 查询今日VIP订单

```sql
SELECT id, order_no
FROM orders
WHERE status = 'PAID';
```

普通文本说明：

```json
{"ignore": true}
```

### 2. PostgreSQL 聚合统计

```postgresql
SELECT customer_id, SUM(total_amount)
FROM orders
GROUP BY customer_id;
```
"#;
        let blocks = extract_markdown_sql_blocks(md);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].language, "sql");
        assert_eq!(blocks[0].title.as_deref(), Some("1. 查询今日VIP订单"));
        assert!(blocks[0].sql.contains("SELECT id, order_no"));
        assert_eq!(blocks[1].language, "postgresql");
        assert_eq!(blocks[1].title.as_deref(), Some("2. PostgreSQL 聚合统计"));
        assert!(blocks[1].sql.contains("GROUP BY customer_id"));
    }

    #[test]
    fn test_extract_statement_at_cursor_picks_containing_statement() {
        let sql = "SELECT 1;\nSELECT id, name FROM customers WHERE id = 2;\nSELECT 3;";
        let cursor_in_second = sql.find("customers").unwrap();
        let stmt = extract_statement_at_cursor(sql, cursor_in_second).unwrap();
        assert_eq!(stmt, "SELECT id, name FROM customers WHERE id = 2");
    }

    #[test]
    fn test_format_sql_pretty_clauses_and_preserves_strings() {
        let formatted = format_sql(
            "select id, name, 'where from' as lbl from customers left join orders on customers.id = orders.customer_id where status = 'PAID' and total_amount > 100 order by id desc limit 20;",
        );
        assert!(formatted.contains("SELECT"));
        assert!(formatted.contains("\nFROM customers"));
        assert!(formatted.contains("\nLEFT JOIN orders"));
        assert!(formatted.contains("\nWHERE status = 'PAID'"));
        assert!(formatted.contains("\n  AND total_amount > 100"));
        assert!(formatted.contains("'where from'"));
        assert!(formatted.contains("\nLIMIT 20;"));
    }

    #[test]
    fn test_redis_command_analysis_and_risk_classification() {
        let script = r#"
        -- inspect keys
        # comment line
        GET user:1001
        HGETALL "session:a b"
        KEYS *
        SET temp:flag "hello \"world\"" EX 60;
        FLUSHALL
        "#;
        let res = analyze_sql(script, &DbEngineType::Redis);
        assert_eq!(res.statements.len(), 5);
        assert_eq!(res.statements[0].access_kind, SqlAccessKind::ReadOnly);
        assert!(res.statements[0].warnings.is_empty());
        assert_eq!(res.statements[1].access_kind, SqlAccessKind::ReadOnly);
        assert_eq!(res.statements[2].access_kind, SqlAccessKind::ReadOnly);
        assert_eq!(res.statements[2].warnings[0].code, "REDIS_BLOCKING_KEYS");
        assert_eq!(res.statements[2].warnings[0].level, RiskLevel::High);
        assert_eq!(res.statements[3].access_kind, SqlAccessKind::WriteDml);
        assert_eq!(res.statements[4].access_kind, SqlAccessKind::WriteDdl);
        assert_eq!(res.statements[4].warnings[0].code, "REDIS_FLUSH_ALL");
        assert_eq!(res.statements[4].warnings[0].level, RiskLevel::Critical);
        assert_eq!(res.overall_kind, SqlAccessKind::WriteDdl);
        assert!(res.requires_ticket);

        let toks = tokenize_redis_command(r#"SET my:key "hello \"redis\"" EX 60"#);
        assert_eq!(
            toks,
            vec!["SET", "my:key", "hello \"redis\"", "EX", "60"]
        );
    }
}
