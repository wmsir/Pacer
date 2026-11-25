use std::io::BufRead;

pub const DEFAULT_MAX_STATEMENT_BYTES: usize = 32 * 1024 * 1024;

fn read_bounded_line<R: BufRead>(
    reader: &mut R,
    max_line_bytes: usize,
) -> std::io::Result<Option<String>> {
    let mut raw: Vec<u8> = Vec::new();
    loop {
        let avail = reader.fill_buf()?;
        if avail.is_empty() {
            if raw.is_empty() {
                return Ok(None);
            }
            let s = String::from_utf8(raw)
                .unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned());
            return Ok(Some(s));
        }
        let nl_pos = avail.iter().position(|&b| b == b'\n');
        let take_len = nl_pos.map_or(avail.len(), |p| p + 1);
        if raw.len().saturating_add(take_len) > max_line_bytes {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("单条 SQL 语句或单行超过最大大小限制（{} bytes）", max_line_bytes),
            ));
        }
        raw.extend_from_slice(&avail[..take_len]);
        reader.consume(take_len);
        if nl_pos.is_some() {
            let s = String::from_utf8(raw)
                .unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned());
            return Ok(Some(s));
        }
    }
}

/// Incrementally splits potentially multi-gigabyte SQL script files into executable statements
/// using bounded memory while preserving string literals, comments, PostgreSQL dollar-quoted
/// blocks (`$$...$$` / `$tag$...$tag$`), and MySQL `DELIMITER` directives.
pub struct SqlScriptStreamReader<R: BufRead> {
    reader: R,
    pending: String,
    delimiter: String,
    bytes_read: u64,
    max_statement_bytes: usize,
    first_line: bool,
    eof: bool,
    in_single_quote: bool,
    in_double_quote: bool,
    in_backtick: bool,
    in_block_comment: bool,
    dollar_tag: Option<String>,
}

impl<R: BufRead> SqlScriptStreamReader<R> {
    pub fn new(reader: R) -> Self {
        Self::with_max_statement_bytes(reader, DEFAULT_MAX_STATEMENT_BYTES)
    }

    pub fn with_max_statement_bytes(reader: R, max_statement_bytes: usize) -> Self {
        Self {
            reader,
            pending: String::new(),
            delimiter: ";".to_string(),
            bytes_read: 0,
            max_statement_bytes: max_statement_bytes.max(1024),
            first_line: true,
            eof: false,
            in_single_quote: false,
            in_double_quote: false,
            in_backtick: false,
            in_block_comment: false,
            dollar_tag: None,
        }
    }

    pub fn bytes_read(&self) -> u64 {
        self.bytes_read
    }

    fn is_top_level(&self) -> bool {
        !self.in_single_quote
            && !self.in_double_quote
            && !self.in_backtick
            && !self.in_block_comment
            && self.dollar_tag.is_none()
    }

    /// Reads the next non-empty, non-comment-only SQL statement from the underlying stream.
    pub fn next_statement(&mut self) -> std::io::Result<Option<String>> {
        let mut stmt = String::new();

        loop {
            while self.pending.is_empty() {
                if self.eof {
                    let trimmed = stmt.trim();
                    if has_executable_sql(trimmed) {
                        return Ok(Some(trimmed.to_string()));
                    }
                    return Ok(None);
                }

                let rem_budget = self.max_statement_bytes.saturating_sub(stmt.len()).max(1);
                let Some(mut line) = read_bounded_line(&mut self.reader, rem_budget)? else {
                    self.eof = true;
                    break;
                };
                self.bytes_read += line.len() as u64;
                if self.first_line {
                    self.first_line = false;
                    if let Some(stripped) = line.strip_prefix('\u{FEFF}') {
                        line = stripped.to_string();
                    }
                }

                // MySQL / MariaDB CLI `DELIMITER <token>` directive at statement boundary
                if self.is_top_level() && !has_executable_sql(stmt.trim()) {
                    if let Some(next_delim) = parse_delimiter_directive(&line) {
                        self.delimiter = next_delim;
                        stmt.clear();
                        continue;
                    }
                }

                self.pending = line;
            }

            if self.pending.is_empty() && self.eof {
                let trimmed = stmt.trim();
                if has_executable_sql(trimmed) {
                    return Ok(Some(trimmed.to_string()));
                }
                return Ok(None);
            }

            if let Some(emitted) = self.scan_pending_into(&mut stmt) {
                if has_executable_sql(&emitted) {
                    return Ok(Some(emitted));
                }
                stmt.clear();
            }
            if stmt.len() > self.max_statement_bytes {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "单条 SQL 语句超过最大大小限制（{} bytes）",
                        self.max_statement_bytes
                    ),
                ));
            }
        }
    }

    fn scan_pending_into(&mut self, stmt: &mut String) -> Option<String> {
        let src = std::mem::take(&mut self.pending);
        let bytes = src.as_bytes();
        let mut idx = 0usize;

        while idx < bytes.len() {
            let rest = &src[idx..];

            if self.in_block_comment {
                if let Some(end_rel) = rest.find("*/") {
                    let end = idx + end_rel + 2;
                    stmt.push_str(&src[idx..end]);
                    idx = end;
                    self.in_block_comment = false;
                } else {
                    stmt.push_str(rest);
                    return None;
                }
                continue;
            }

            if let Some(ref tag) = self.dollar_tag {
                if rest.starts_with(tag.as_str()) {
                    let end = idx + tag.len();
                    stmt.push_str(&src[idx..end]);
                    idx = end;
                    self.dollar_tag = None;
                } else if let Some(ch) = rest.chars().next() {
                    stmt.push(ch);
                    idx += ch.len_utf8();
                } else {
                    break;
                }
                continue;
            }

            if self.in_single_quote {
                if rest.starts_with("''") {
                    stmt.push_str("''");
                    idx += 2;
                } else if rest.starts_with("\\'") || rest.starts_with("\\\\") {
                    stmt.push_str(&src[idx..idx + 2]);
                    idx += 2;
                } else if bytes[idx] == b'\'' {
                    stmt.push('\'');
                    idx += 1;
                    self.in_single_quote = false;
                } else if let Some(ch) = rest.chars().next() {
                    stmt.push(ch);
                    idx += ch.len_utf8();
                }
                continue;
            }

            if self.in_double_quote {
                if rest.starts_with("\"\"") {
                    stmt.push_str("\"\"");
                    idx += 2;
                } else if rest.starts_with("\\\"") || rest.starts_with("\\\\") {
                    stmt.push_str(&src[idx..idx + 2]);
                    idx += 2;
                } else if bytes[idx] == b'"' {
                    stmt.push('"');
                    idx += 1;
                    self.in_double_quote = false;
                } else if let Some(ch) = rest.chars().next() {
                    stmt.push(ch);
                    idx += ch.len_utf8();
                }
                continue;
            }

            if self.in_backtick {
                if rest.starts_with("``") {
                    stmt.push_str("``");
                    idx += 2;
                } else if bytes[idx] == b'`' {
                    stmt.push('`');
                    idx += 1;
                    self.in_backtick = false;
                } else if let Some(ch) = rest.chars().next() {
                    stmt.push(ch);
                    idx += ch.len_utf8();
                }
                continue;
            }

            // Top-level state
            if !self.delimiter.is_empty() && rest.starts_with(&self.delimiter) {
                let after = idx + self.delimiter.len();
                if after < src.len() {
                    self.pending = src[after..].to_string();
                }
                return Some(stmt.trim().to_string());
            }

            if rest.starts_with("--") || rest.starts_with('#') {
                stmt.push_str(rest);
                return None;
            }

            if rest.starts_with("/*") {
                stmt.push_str("/*");
                idx += 2;
                self.in_block_comment = true;
                continue;
            }

            if bytes[idx] == b'$' {
                if let Some(tag) = parse_pg_dollar_tag(rest) {
                    stmt.push_str(&tag);
                    idx += tag.len();
                    self.dollar_tag = Some(tag);
                    continue;
                }
            }

            match bytes[idx] {
                b'\'' => {
                    stmt.push('\'');
                    idx += 1;
                    self.in_single_quote = true;
                }
                b'"' => {
                    stmt.push('"');
                    idx += 1;
                    self.in_double_quote = true;
                }
                b'`' => {
                    stmt.push('`');
                    idx += 1;
                    self.in_backtick = true;
                }
                _ => {
                    if let Some(ch) = rest.chars().next() {
                        stmt.push(ch);
                        idx += ch.len_utf8();
                    } else {
                        break;
                    }
                }
            }
        }

        None
    }
}

fn parse_delimiter_directive(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("delimiter") {
        let orig_rest = &trimmed["delimiter".len()..];
        if rest.chars().next().is_some_and(char::is_whitespace) {
            let tok = orig_rest.trim();
            if !tok.is_empty() {
                return Some(tok.to_string());
            }
        }
    }
    None
}

fn parse_pg_dollar_tag(rest: &str) -> Option<String> {
    let inner = rest.strip_prefix('$')?;
    if inner.starts_with('$') {
        return Some("$$".to_string());
    }
    let mut end = 0usize;
    for (idx, ch) in inner.char_indices() {
        if ch == '$' {
            if idx == 0 {
                return Some("$$".to_string());
            }
            return Some(rest[..idx + 2].to_string());
        }
        if !(ch.is_ascii_alphanumeric() || ch == '_') {
            return None;
        }
        end = idx;
    }
    let _ = end;
    None
}

/// Returns true if `sql` contains at least one non-whitespace token outside SQL comments.
fn has_executable_sql(sql: &str) -> bool {
    let bytes = sql.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let rest = &sql[i..];
        if bytes[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if rest.starts_with("--") || rest.starts_with('#') {
            if let Some(nl) = rest.find('\n') {
                i += nl + 1;
                continue;
            }
            return false;
        }
        if let Some(after) = rest.strip_prefix("/*") {
            if after.starts_with('!') {
                return true;
            }
            if let Some(end) = after.find("*/") {
                i += 2 + end + 2;
                continue;
            }
            return false;
        }
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    fn collect_all(script: &str) -> Vec<String> {
        let mut reader = SqlScriptStreamReader::new(Cursor::new(script.as_bytes()));
        let mut out = Vec::new();
        while let Some(stmt) = reader.next_statement().unwrap() {
            out.push(stmt);
        }
        assert!(reader.bytes_read() >= script.len() as u64);
        out
    }

    #[test]
    fn test_sql_script_stream_handles_quotes_comments_and_multiple_per_line() {
        let script = "\u{FEFF}-- header comment\n\
            SELECT 1; SELECT 'a;b''c' AS txt;\n\
            /* block ; comment */\n\
            INSERT INTO logs(msg) VALUES ('line1;\nline2');\n\
            -- trailing comment only\n";
        let stmts = collect_all(script);
        assert_eq!(stmts.len(), 3);
        assert!(stmts[0].contains("SELECT 1"));
        assert_eq!(stmts[1], "SELECT 'a;b''c' AS txt");
        assert!(stmts[2].contains("VALUES ('line1;\nline2')"));
    }

    #[test]
    fn test_sql_script_stream_handles_pg_dollar_quotes_and_mysql_delimiter() {
        let script = "DELIMITER //\n\
            CREATE PROCEDURE p_test() BEGIN SELECT 'hi//there'; SELECT 42; END//\n\
            DELIMITER ;\n\
            CREATE FUNCTION fn_pg() RETURNS text AS $body$\nBEGIN\n  RETURN 'x;y';\nEND;\n$body$ LANGUAGE plpgsql;\n\
            SELECT 99;\n";
        let stmts = collect_all(script);
        assert_eq!(stmts.len(), 3);
        assert!(stmts[0].starts_with("CREATE PROCEDURE p_test()"));
        assert!(stmts[0].contains("SELECT 42; END"));
        assert!(stmts[1].contains("$body$"));
        assert_eq!(stmts[2], "SELECT 99");
    }

    #[test]
    fn test_sql_script_stream_rejects_oversized_single_statement() {
        let huge = format!("SELECT '{}';", "x".repeat(4096));
        let mut reader = SqlScriptStreamReader::with_max_statement_bytes(
            Cursor::new(huge.as_bytes()),
            1024,
        );
        let err = reader.next_statement().unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }
}
