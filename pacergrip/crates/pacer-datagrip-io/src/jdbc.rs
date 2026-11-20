use std::collections::BTreeMap;

use pacer_core::{CoreError, CoreResult, DbEngineType};

/// Structured representation extracted from a DataGrip JDBC connection string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedJdbcUrl {
    pub db_type: DbEngineType,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub database: Option<String>,
    pub params: BTreeMap<String, String>,
    pub raw_url: String,
}

/// Parses a JDBC URL and refines engine type using optional DataGrip `<driver-ref>` and `<jdbc-driver>`.
pub fn parse_jdbc_url(
    raw_url: &str,
    driver_ref: Option<&str>,
    driver_class: Option<&str>,
) -> CoreResult<ParsedJdbcUrl> {
    let trimmed = raw_url.trim();
    if trimmed.is_empty() {
        return Err(CoreError::ParseError("empty JDBC URL".to_string()));
    }

    let after_jdbc = strip_prefix_ci(trimmed, "jdbc:")
        .ok_or_else(|| CoreError::ParseError(format!("invalid JDBC URL (missing jdbc:): {trimmed}")))?;

    let mut parsed = if let Some(rest) = strip_prefix_ci(after_jdbc, "oracle:") {
        parse_oracle_url(trimmed, rest)?
    } else if let Some(rest) = strip_prefix_ci(after_jdbc, "sqlserver:")
        .or_else(|| strip_prefix_ci(after_jdbc, "microsoft:sqlserver:"))
    {
        parse_sqlserver_url(trimmed, rest)?
    } else if let Some(rest) = strip_prefix_ci(after_jdbc, "sqlite:") {
        parse_sqlite_url(trimmed, rest)
    } else {
        parse_standard_url(trimmed, after_jdbc)?
    };

    // Refine engine type using DataGrip's driver-ref or driver-class (e.g. TiDB/OceanBase over jdbc:mysql://).
    if let Some(refined) = refine_engine_from_hints(&parsed.db_type, driver_ref, driver_class) {
        parsed.db_type = refined;
    }

    if parsed.port.is_none() {
        parsed.port = parsed.db_type.default_port();
    }

    Ok(parsed)
}

fn strip_prefix_ci<'a>(input: &'a str, prefix: &str) -> Option<&'a str> {
    if input.len() >= prefix.len() && input[..prefix.len()].eq_ignore_ascii_case(prefix) {
        Some(&input[prefix.len()..])
    } else {
        None
    }
}

fn refine_engine_from_hints(
    current: &DbEngineType,
    driver_ref: Option<&str>,
    driver_class: Option<&str>,
) -> Option<DbEngineType> {
    for hint in [driver_ref, driver_class].into_iter().flatten() {
        let candidate = DbEngineType::from_str_loose(hint);
        match (&candidate, current) {
            (DbEngineType::TiDb, DbEngineType::MySql)
            | (DbEngineType::OceanBase, DbEngineType::MySql | DbEngineType::Oracle)
            | (DbEngineType::MariaDb, DbEngineType::MySql)
            | (DbEngineType::OpenGauss, DbEngineType::PostgreSql)
            | (DbEngineType::Kingbase, DbEngineType::PostgreSql)
            | (DbEngineType::Dameng, DbEngineType::Unknown(_)) => return Some(candidate),
            (other, DbEngineType::Unknown(_)) if !matches!(other, DbEngineType::Unknown(_)) => {
                return Some(other.clone())
            }
            _ => {}
        }
    }
    None
}

fn parse_standard_url(raw_url: &str, after_jdbc: &str) -> CoreResult<ParsedJdbcUrl> {
    let Some(scheme_end) = after_jdbc.find("://") else {
        return Err(CoreError::ParseError(format!(
            "unsupported JDBC URL syntax: {raw_url}"
        )));
    };

    let scheme_part = &after_jdbc[..scheme_end];
    // Handle sub-protocols like `mysql:replication://` or `mysql:loadbalance://`.
    let primary_scheme = scheme_part.split(':').next().unwrap_or(scheme_part);
    let db_type = DbEngineType::from_str_loose(primary_scheme);

    let remainder = &after_jdbc[scheme_end + 3..];
    let (before_query, query_part) = match remainder.find('?') {
        Some(idx) => (&remainder[..idx], Some(&remainder[idx + 1..])),
        None => (remainder, None),
    };

    let (before_semi, semi_part) = match before_query.find(';') {
        Some(idx) => (&before_query[..idx], Some(&before_query[idx + 1..])),
        None => (before_query, None),
    };

    let (authority, path_part) = match before_semi.find('/') {
        Some(idx) => (&before_semi[..idx], Some(&before_semi[idx + 1..])),
        None => (before_semi, None),
    };

    let (host, port) = parse_authority_host_port(authority);

    let mut params = BTreeMap::new();
    if let Some(q) = query_part {
        parse_kv_pairs(q, '&', &mut params);
    }
    if let Some(s) = semi_part {
        parse_kv_pairs(s, ';', &mut params);
    }

    let database = path_part
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.trim_end_matches('/').to_string())
        .or_else(|| {
            params
                .get("databaseName")
                .or_else(|| params.get("database"))
                .or_else(|| params.get("schema"))
                .cloned()
        });

    Ok(ParsedJdbcUrl {
        db_type,
        host,
        port,
        database,
        params,
        raw_url: raw_url.to_string(),
    })
}

fn parse_oracle_url(raw_url: &str, after_oracle: &str) -> CoreResult<ParsedJdbcUrl> {
    // Examples:
    // thin:@//10.0.0.1:1521/orclpdb1
    // thin:@10.0.0.1:1521:ORCL
    // thin:@10.0.0.1:1521/orclpdb1
    // thin:@(DESCRIPTION=(ADDRESS=(HOST=10.0.0.1)(PORT=1521))(CONNECT_DATA=(SERVICE_NAME=orcl)))
    let target = after_oracle
        .split('@')
        .nth(1)
        .unwrap_or(after_oracle)
        .trim();

    if target.starts_with('(') {
        let host = extract_tns_value(target, "HOST");
        let port = extract_tns_value(target, "PORT").and_then(|p| p.parse::<u16>().ok());
        let database = extract_tns_value(target, "SERVICE_NAME")
            .or_else(|| extract_tns_value(target, "SID"));
        return Ok(ParsedJdbcUrl {
            db_type: DbEngineType::Oracle,
            host,
            port: port.or(Some(1521)),
            database,
            params: BTreeMap::new(),
            raw_url: raw_url.to_string(),
        });
    }

    let (main_part, query_part) = match target.find('?') {
        Some(idx) => (&target[..idx], Some(&target[idx + 1..])),
        None => (target, None),
    };

    let clean = main_part.strip_prefix("//").unwrap_or(main_part);
    let mut host = None;
    let mut port = None;
    let mut database = None;

    if let Some((authority, db_part)) = clean.split_once('/') {
        let (h, p) = parse_authority_host_port(authority);
        host = h;
        port = p;
        let db_trimmed = db_part.trim();
        if !db_trimmed.is_empty() {
            database = Some(db_trimmed.to_string());
        }
    } else {
        // Legacy SID syntax: host:port:SID or host:SID
        let parts: Vec<&str> = clean.split(':').collect();
        match parts.as_slice() {
            [h, p, sid] => {
                host = non_empty(h);
                port = p.trim().parse::<u16>().ok();
                database = non_empty(sid);
            }
            [h, second] => {
                host = non_empty(h);
                if let Ok(p) = second.trim().parse::<u16>() {
                    port = Some(p);
                } else {
                    database = non_empty(second);
                }
            }
            [h] => {
                host = non_empty(h);
            }
            _ => {}
        }
    }

    let mut params = BTreeMap::new();
    if let Some(q) = query_part {
        parse_kv_pairs(q, '&', &mut params);
    }

    Ok(ParsedJdbcUrl {
        db_type: DbEngineType::Oracle,
        host,
        port: port.or(Some(1521)),
        database,
        params,
        raw_url: raw_url.to_string(),
    })
}

fn parse_sqlserver_url(raw_url: &str, after_sqlserver: &str) -> CoreResult<ParsedJdbcUrl> {
    let trimmed = after_sqlserver.strip_prefix("//").unwrap_or(after_sqlserver);
    let mut segments = trimmed.split(';');
    let authority_raw = segments.next().unwrap_or("");
    // Strip optional `\\instanceName` from `host\instance:1433` or `host:1433\instance`.
    let authority = authority_raw.split('\\').next().unwrap_or(authority_raw);
    let (host, port) = parse_authority_host_port(authority);

    let mut params = BTreeMap::new();
    for seg in segments {
        if let Some((k, v)) = seg.split_once('=') {
            let key = k.trim();
            if !key.is_empty() {
                params.insert(key.to_string(), v.trim().to_string());
            }
        }
    }

    let database = params
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("databaseName") || k.eq_ignore_ascii_case("database"))
        .map(|(_, v)| v.clone())
        .filter(|s| !s.is_empty());

    Ok(ParsedJdbcUrl {
        db_type: DbEngineType::SqlServer,
        host,
        port: port.or(Some(1433)),
        database,
        params,
        raw_url: raw_url.to_string(),
    })
}

fn parse_sqlite_url(raw_url: &str, after_sqlite: &str) -> ParsedJdbcUrl {
    let (path_part, query_part) = match after_sqlite.find('?') {
        Some(idx) => (&after_sqlite[..idx], Some(&after_sqlite[idx + 1..])),
        None => (after_sqlite, None),
    };
    let mut params = BTreeMap::new();
    if let Some(q) = query_part {
        parse_kv_pairs(q, '&', &mut params);
    }
    ParsedJdbcUrl {
        db_type: DbEngineType::Sqlite,
        host: None,
        port: None,
        database: non_empty(path_part),
        params,
        raw_url: raw_url.to_string(),
    }
}

fn parse_authority_host_port(authority: &str) -> (Option<String>, Option<u16>) {
    // If multiple hosts are comma-separated (e.g. replica sets), take the first primary endpoint.
    let first_endpoint = authority.split(',').next().unwrap_or("").trim();
    // Strip optional userinfo (`user:pass@host:port`).
    let host_port = first_endpoint
        .rsplit_once('@')
        .map(|(_, hp)| hp)
        .unwrap_or(first_endpoint)
        .trim();

    if host_port.is_empty() {
        return (None, None);
    }

    // Handle bracketed IPv6 literals `[::1]:3306`.
    if let Some(rest) = host_port.strip_prefix('[') {
        if let Some((ip, tail)) = rest.split_once(']') {
            let port = tail
                .strip_prefix(':')
                .and_then(|p| p.trim().parse::<u16>().ok());
            return (non_empty(ip), port);
        }
    }

    if let Some((h, p)) = host_port.rsplit_once(':') {
        if let Ok(parsed_port) = p.trim().parse::<u16>() {
            return (non_empty(h), Some(parsed_port));
        }
    }

    (non_empty(host_port), None)
}

fn parse_kv_pairs(raw: &str, sep: char, out: &mut BTreeMap<String, String>) {
    for pair in raw.split(sep) {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }
        if let Some((k, v)) = pair.split_once('=') {
            let key = k.trim();
            if !key.is_empty() {
                out.insert(key.to_string(), v.trim().to_string());
            }
        }
    }
}

fn extract_tns_value(tns: &str, key: &str) -> Option<String> {
    let needle = format!("{key}=");
    let upper = tns.to_ascii_uppercase();
    let idx = upper.find(&needle.to_ascii_uppercase())?;
    let after = &tns[idx + needle.len()..];
    let end = after.find(')')?;
    non_empty(&after[..end])
}

fn non_empty(s: &str) -> Option<String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
