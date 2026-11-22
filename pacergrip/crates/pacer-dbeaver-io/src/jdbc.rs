use std::collections::BTreeMap;

use pacer_core::{CoreError, CoreResult, DbEngineType};

/// Structured representation extracted from a JDBC or native database connection URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedJdbcUrl {
    pub db_type: DbEngineType,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub database: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub socket_path: Option<String>,
    pub params: BTreeMap<String, String>,
    pub raw_url: String,
}

/// Parses either a `jdbc:...` URL or a native URI (`mysql://`, `postgresql://`, `sqlite://`).
/// Refines engine type using optional driver hints (e.g. `TiDB` / `OceanBase` over `jdbc:mysql://`).
pub fn parse_connection_url(
    raw_url: &str,
    driver_ref: Option<&str>,
    driver_class: Option<&str>,
) -> CoreResult<ParsedJdbcUrl> {
    let trimmed = raw_url.trim();
    if trimmed.is_empty() {
        return Err(CoreError::ParseError("连接 URL 不能为空".to_string()));
    }

    let normalized = if strip_prefix_ci(trimmed, "jdbc:").is_some() {
        trimmed.to_string()
    } else if trimmed.contains("://") || strip_prefix_ci(trimmed, "sqlite:").is_some() {
        format!("jdbc:{trimmed}")
    } else {
        return Err(CoreError::ParseError(format!(
            "无效的连接 URL（需要 jdbc: 或 scheme:// 前缀）: {trimmed}"
        )));
    };

    let mut parsed = parse_jdbc_url(&normalized, driver_ref, driver_class)?;
    parsed.raw_url = trimmed.to_string();
    Ok(parsed)
}

/// Parses a JDBC URL and refines engine type using optional driver hints.
pub fn parse_jdbc_url(
    raw_url: &str,
    driver_ref: Option<&str>,
    driver_class: Option<&str>,
) -> CoreResult<ParsedJdbcUrl> {
    let trimmed = raw_url.trim();
    if trimmed.is_empty() {
        return Err(CoreError::ParseError("empty JDBC URL".to_string()));
    }

    let after_jdbc = strip_prefix_ci(trimmed, "jdbc:").ok_or_else(|| {
        CoreError::ParseError(format!("invalid JDBC URL (missing jdbc:): {trimmed}"))
    })?;

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

    if let Some(refined) = refine_engine_from_hints(&parsed.db_type, driver_ref, driver_class) {
        parsed.db_type = refined;
    }

    if parsed.port.is_none() {
        parsed.port = parsed.db_type.default_port();
    }

    Ok(parsed)
}

/// Builds a canonical JDBC URL matching standard JetBrains/DBeaver driver conventions.
/// Notably, TiDB uses `jdbc:mysql://host:port[/db]` (compatible with MySQL Connector/J).
pub fn build_jdbc_url(
    db_type: &DbEngineType,
    host: Option<&str>,
    port: Option<u16>,
    database: Option<&str>,
    params: &BTreeMap<String, String>,
    socket_path: Option<&str>,
) -> String {
    let h = host.filter(|s| !s.trim().is_empty()).unwrap_or("localhost");
    let p = port.or_else(|| db_type.default_port());
    let port_part = p.map(|n| format!(":{n}")).unwrap_or_default();
    let db = database.map(str::trim).unwrap_or_default();
    let db_slash = if db.is_empty() {
        String::new()
    } else {
        format!("/{db}")
    };

    let mut effective_params = params.clone();
    if let Some(sock) = socket_path.map(str::trim).filter(|s| !s.is_empty()) {
        effective_params
            .entry("socket".to_string())
            .or_insert_with(|| sock.to_string());
    }
    let redis_ssl = matches!(db_type, DbEngineType::Redis)
        && effective_params
            .get("ssl")
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(false);
    if redis_ssl {
        effective_params.remove("ssl");
    }

    let base = match db_type {
        DbEngineType::Sqlite => format!("jdbc:sqlite:{db}"),
        DbEngineType::Oracle => format!("jdbc:oracle:thin:@//{h}{port_part}{db_slash}"),
        DbEngineType::SqlServer => {
            if db.is_empty() {
                format!("jdbc:sqlserver://{h}{port_part}")
            } else {
                format!("jdbc:sqlserver://{h}{port_part};databaseName={db}")
            }
        }
        DbEngineType::Dameng => format!("jdbc:dm://{h}{port_part}{db_slash}"),
        DbEngineType::Kingbase => format!("jdbc:kingbase8://{h}{port_part}{db_slash}"),
        DbEngineType::OpenGauss => format!("jdbc:opengauss://{h}{port_part}{db_slash}"),
        DbEngineType::PostgreSql => format!("jdbc:postgresql://{h}{port_part}{db_slash}"),
        DbEngineType::MariaDb => format!("jdbc:mariadb://{h}{port_part}{db_slash}"),
        DbEngineType::OceanBase => format!("jdbc:oceanbase://{h}{port_part}{db_slash}"),
        DbEngineType::ClickHouse => format!("jdbc:clickhouse://{h}{port_part}{db_slash}"),
        DbEngineType::Redis => {
            let scheme = if redis_ssl { "rediss" } else { "redis" };
            format!("{scheme}://{h}{port_part}{db_slash}")
        }
        // TiDB and MySQL both use standard `jdbc:mysql://host:port[/db]`.
        DbEngineType::TiDb | DbEngineType::MySql | DbEngineType::Unknown(_) => {
            format!("jdbc:mysql://{h}{port_part}{db_slash}")
        }
    };

    if effective_params.is_empty() {
        return base;
    }

    let sep = if matches!(db_type, DbEngineType::SqlServer) {
        ";"
    } else {
        "&"
    };
    let joiner = if matches!(db_type, DbEngineType::SqlServer) {
        ";"
    } else if base.contains('?') {
        "&"
    } else {
        "?"
    };
    let qs = effective_params
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join(sep);

    format!("{base}{joiner}{qs}")
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

    let (username, password, host, port) = parse_authority(authority);

    let mut params = BTreeMap::new();
    if primary_scheme.eq_ignore_ascii_case("rediss") {
        params.insert("ssl".to_string(), "true".to_string());
    }
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

    let socket_path = params
        .get("socket")
        .or_else(|| params.get("socketPath"))
        .or_else(|| params.get("unixSocket"))
        .cloned();

    let effective_user = username.or_else(|| {
        params
            .get("user")
            .or_else(|| params.get("username"))
            .cloned()
    });
    let effective_pass = password.or_else(|| params.get("password").cloned());

    Ok(ParsedJdbcUrl {
        db_type,
        host,
        port,
        database,
        username: effective_user,
        password: effective_pass,
        socket_path,
        params,
        raw_url: raw_url.to_string(),
    })
}

fn parse_oracle_url(raw_url: &str, after_oracle: &str) -> CoreResult<ParsedJdbcUrl> {
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
            username: None,
            password: None,
            socket_path: None,
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
        let (_, _, h, p) = parse_authority(authority);
        host = h;
        port = p;
        let db_trimmed = db_part.trim();
        if !db_trimmed.is_empty() {
            database = Some(db_trimmed.to_string());
        }
    } else {
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
        username: None,
        password: None,
        socket_path: None,
        params,
        raw_url: raw_url.to_string(),
    })
}

fn parse_sqlserver_url(raw_url: &str, after_sqlserver: &str) -> CoreResult<ParsedJdbcUrl> {
    let trimmed = after_sqlserver.strip_prefix("//").unwrap_or(after_sqlserver);
    let mut segments = trimmed.split(';');
    let authority_raw = segments.next().unwrap_or("");
    let authority = authority_raw.split('\\').next().unwrap_or(authority_raw);
    let (username, password, host, port) = parse_authority(authority);

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
        username,
        password,
        socket_path: None,
        params,
        raw_url: raw_url.to_string(),
    })
}

fn parse_sqlite_url(raw_url: &str, after_sqlite: &str) -> ParsedJdbcUrl {
    let stripped = after_sqlite.strip_prefix("//").unwrap_or(after_sqlite);
    let (path_part, query_part) = match stripped.find('?') {
        Some(idx) => (&stripped[..idx], Some(&stripped[idx + 1..])),
        None => (stripped, None),
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
        username: None,
        password: None,
        socket_path: None,
        params,
        raw_url: raw_url.to_string(),
    }
}

fn parse_authority(
    authority: &str,
) -> (Option<String>, Option<String>, Option<String>, Option<u16>) {
    let first_endpoint = authority.split(',').next().unwrap_or("").trim();
    let (userinfo, host_port) = match first_endpoint.rsplit_once('@') {
        Some((u, hp)) => (Some(u), hp.trim()),
        None => (None, first_endpoint.trim()),
    };

    let (username, password) = match userinfo {
        Some(info) => match info.split_once(':') {
            Some((u, p)) => (non_empty(u), non_empty(p)),
            None => (non_empty(info), None),
        },
        None => (None, None),
    };

    if host_port.is_empty() {
        return (username, password, None, None);
    }

    if let Some(rest) = host_port.strip_prefix('[') {
        if let Some((ip, tail)) = rest.split_once(']') {
            let port = tail
                .strip_prefix(':')
                .and_then(|p| p.trim().parse::<u16>().ok());
            return (username, password, non_empty(ip), port);
        }
    }

    if let Some((h, p)) = host_port.rsplit_once(':') {
        if let Ok(parsed_port) = p.trim().parse::<u16>() {
            return (username, password, non_empty(h), Some(parsed_port));
        }
    }

    (username, password, non_empty(host_port), None)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redis_and_rediss_url_parsing_and_building() {
        let parsed = parse_connection_url(
            "rediss://user:secret@redis.example.com:6380/2",
            None,
            None,
        )
        .unwrap();
        assert_eq!(parsed.db_type, DbEngineType::Redis);
        assert_eq!(parsed.host.as_deref(), Some("redis.example.com"));
        assert_eq!(parsed.port, Some(6380));
        assert_eq!(parsed.database.as_deref(), Some("2"));
        assert_eq!(parsed.username.as_deref(), Some("user"));
        assert_eq!(parsed.password.as_deref(), Some("secret"));
        assert_eq!(parsed.params.get("ssl").map(String::as_str), Some("true"));

        let rebuilt_tls = build_jdbc_url(
            &DbEngineType::Redis,
            parsed.host.as_deref(),
            parsed.port,
            parsed.database.as_deref(),
            &parsed.params,
            None,
        );
        assert_eq!(rebuilt_tls, "rediss://redis.example.com:6380/2");

        let legacy_auth =
            parse_connection_url("redis://:onlypass@127.0.0.1/0", None, None).unwrap();
        assert_eq!(legacy_auth.db_type, DbEngineType::Redis);
        assert_eq!(legacy_auth.port, Some(6379));
        assert_eq!(legacy_auth.username, None);
        assert_eq!(legacy_auth.password.as_deref(), Some("onlypass"));
        assert_eq!(legacy_auth.database.as_deref(), Some("0"));
        assert!(!legacy_auth.params.contains_key("ssl"));

        let rebuilt_plain = build_jdbc_url(
            &DbEngineType::Redis,
            legacy_auth.host.as_deref(),
            legacy_auth.port,
            legacy_auth.database.as_deref(),
            &legacy_auth.params,
            None,
        );
        assert_eq!(rebuilt_plain, "redis://127.0.0.1:6379/0");
    }
}
