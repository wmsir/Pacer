use std::collections::{BTreeMap, HashMap};
use std::io::{Cursor, Read};
use std::path::Path;

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use uuid::Uuid;
use zip::ZipArchive;

use pacer_core::{
    CapabilityFlags, ConnectionFingerprint, ConnectionMode, ConnectionProfile, CoreError,
    CoreResult, DirectConfig, EnvLevel, SshTunnelConfig, SslConfig,
};

use crate::jdbc::parse_jdbc_url;
use crate::ssh::parse_ssh_configs_xml;

#[derive(Debug, Default)]
struct RawDataSource {
    uuid: Option<String>,
    name: Option<String>,
    driver_ref: Option<String>,
    jdbc_driver: Option<String>,
    jdbc_url: Option<String>,
    user_name: Option<String>,
    comment: Option<String>,
    read_only: Option<bool>,
    color: Option<String>,
    schema_pattern: Option<String>,
    driver_properties: BTreeMap<String, String>,
    ssh: Option<RawSshProps>,
    ssl: Option<SslConfig>,
}

#[derive(Debug, Default)]
struct RawSshProps {
    enabled: bool,
    ssh_config_id: Option<String>,
    host: Option<String>,
    port: Option<u16>,
    username: Option<String>,
    auth_type: Option<String>,
    key_file: Option<String>,
}

/// Parses standalone `<data-source>` or `<component name="DataSourceManagerImpl">` DataGrip XML.
pub fn parse_datagrip_xml(xml: &str) -> CoreResult<Vec<ConnectionProfile>> {
    parse_datagrip_bundle(xml, None, None)
}

/// Parses `dataSources.xml` merged with optional `dataSources.local.xml` and `sshConfigs.xml`.
pub fn parse_datagrip_bundle(
    main_xml: &str,
    local_xml: Option<&str>,
    ssh_xml: Option<&str>,
) -> CoreResult<Vec<ConnectionProfile>> {
    let main_sources = parse_raw_data_sources(main_xml)?;
    let local_sources = match local_xml {
        Some(lx) if !lx.trim().is_empty() => parse_raw_data_sources(lx)?,
        _ => Vec::new(),
    };
    let ssh_map = match ssh_xml {
        Some(sx) if !sx.trim().is_empty() => parse_ssh_configs_xml(sx)?,
        _ => HashMap::new(),
    };

    let mut local_by_uuid: HashMap<String, RawDataSource> = HashMap::new();
    let mut local_by_name: HashMap<String, RawDataSource> = HashMap::new();
    for item in local_sources {
        if let Some(uid) = item.uuid.clone().filter(|s| !s.is_empty()) {
            local_by_uuid.insert(uid, item);
        } else if let Some(name) = item.name.clone().filter(|s| !s.is_empty()) {
            local_by_name.insert(name, item);
        }
    }

    let mut profiles = Vec::with_capacity(main_sources.len());
    for mut src in main_sources {
        let local_override = src
            .uuid
            .as_deref()
            .and_then(|u| local_by_uuid.remove(u))
            .or_else(|| src.name.as_deref().and_then(|n| local_by_name.remove(n)));

        if let Some(ov) = local_override {
            merge_local_override(&mut src, ov);
        }

        if let Some(profile) = build_profile(src, &ssh_map)? {
            profiles.push(profile);
        }
    }

    Ok(profiles)
}

/// Imports DataGrip connections from an exported JetBrains Settings `.zip` archive in memory.
pub fn import_datagrip_zip(bytes: &[u8]) -> CoreResult<Vec<ConnectionProfile>> {
    let reader = Cursor::new(bytes);
    let mut archive = ZipArchive::new(reader)
        .map_err(|err| CoreError::ParseError(format!("invalid DataGrip zip archive: {err}")))?;

    let mut main_xml: Option<String> = None;
    let mut local_xml: Option<String> = None;
    let mut ssh_xml: Option<String> = None;

    for idx in 0..archive.len() {
        let mut entry = archive
            .by_index(idx)
            .map_err(|err| CoreError::ParseError(format!("failed reading zip entry: {err}")))?;
        if entry.is_dir() {
            continue;
        }
        let normalized_name = entry.name().replace('\\', "/").to_ascii_lowercase();
        if normalized_name.ends_with("/datasources.xml") || normalized_name == "datasources.xml" {
            let mut content = String::new();
            entry.read_to_string(&mut content)?;
            // Prefer `options/dataSources.xml` over secondary project caches if multiple exist.
            if main_xml.is_none() || normalized_name.contains("options/datasources.xml") {
                main_xml = Some(content);
            }
        } else if normalized_name.ends_with("/datasources.local.xml")
            || normalized_name == "datasources.local.xml"
        {
            let mut content = String::new();
            entry.read_to_string(&mut content)?;
            if local_xml.is_none() || normalized_name.contains("options/datasources.local.xml") {
                local_xml = Some(content);
            }
        } else if normalized_name.ends_with("/sshconfigs.xml") || normalized_name == "sshconfigs.xml"
        {
            let mut content = String::new();
            entry.read_to_string(&mut content)?;
            if ssh_xml.is_none() || normalized_name.contains("options/sshconfigs.xml") {
                ssh_xml = Some(content);
            }
        }
    }

    let main = main_xml.ok_or_else(|| {
        CoreError::ParseError("zip archive does not contain dataSources.xml".to_string())
    })?;

    parse_datagrip_bundle(&main, local_xml.as_deref(), ssh_xml.as_deref())
}

/// Imports DataGrip connections from a `.zip`, `.xml` file, or `.idea`/`options` directory path.
pub fn import_from_path(path: &Path) -> CoreResult<Vec<ConnectionProfile>> {
    if !path.exists() {
        return Err(CoreError::InvalidConfig(format!(
            "path does not exist: {}",
            path.display()
        )));
    }

    if path.is_file() {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if ext == "zip" {
            let bytes = std::fs::read(path)?;
            return import_datagrip_zip(&bytes);
        }

        let main_xml = std::fs::read_to_string(path)?;
        let parent = path.parent();
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        let local_xml = if file_name != "datasources.local.xml" {
            parent
                .map(|p| p.join("dataSources.local.xml"))
                .filter(|p| p.is_file())
                .and_then(|p| std::fs::read_to_string(p).ok())
        } else {
            None
        };

        let ssh_xml = parent
            .map(|p| p.join("sshConfigs.xml"))
            .filter(|p| p.is_file())
            .and_then(|p| std::fs::read_to_string(p).ok());

        return parse_datagrip_bundle(&main_xml, local_xml.as_deref(), ssh_xml.as_deref());
    }

    // Directory search: `<dir>`, `<dir>/.idea`, `<dir>/options`.
    let candidates = [
        path.to_path_buf(),
        path.join(".idea"),
        path.join("options"),
    ];

    for dir in candidates {
        let main_path = dir.join("dataSources.xml");
        if main_path.is_file() {
            let main_xml = std::fs::read_to_string(&main_path)?;
            let local_xml = std::fs::read_to_string(dir.join("dataSources.local.xml")).ok();
            let ssh_xml = std::fs::read_to_string(dir.join("sshConfigs.xml"))
                .or_else(|_| std::fs::read_to_string(path.join("options/sshConfigs.xml")))
                .ok();
            return parse_datagrip_bundle(&main_xml, local_xml.as_deref(), ssh_xml.as_deref());
        }
    }

    Err(CoreError::ParseError(format!(
        "no dataSources.xml found under {}",
        path.display()
    )))
}

fn parse_raw_data_sources(xml: &str) -> CoreResult<Vec<RawDataSource>> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut out = Vec::new();
    let mut current: Option<RawDataSource> = None;
    let mut path_stack: Vec<String> = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let tag = tag_local_name(e.name().as_ref());
                if tag == "data-source" {
                    let mut raw = RawDataSource::default();
                    apply_data_source_attrs(&e, &mut raw)?;
                    current = Some(raw);
                    path_stack.clear();
                } else if let Some(ref mut ds) = current {
                    path_stack.push(tag.clone());
                    handle_element_attrs(&tag, &e, ds, &path_stack)?;
                }
            }
            Ok(Event::Empty(e)) => {
                let tag = tag_local_name(e.name().as_ref());
                if tag == "data-source" {
                    let mut raw = RawDataSource::default();
                    apply_data_source_attrs(&e, &mut raw)?;
                    out.push(raw);
                } else if let Some(ref mut ds) = current {
                    path_stack.push(tag.clone());
                    handle_element_attrs(&tag, &e, ds, &path_stack)?;
                    path_stack.pop();
                }
            }
            Ok(Event::Text(e)) => {
                if let Some(ref mut ds) = current {
                    let unescaped = e
                        .unescape()
                        .map_err(|err| CoreError::ParseError(format!("xml unescape error: {err}")))?;
                    apply_text(ds, &path_stack, unescaped.as_ref());
                }
            }
            Ok(Event::CData(e)) => {
                if let Some(ref mut ds) = current {
                    let text = String::from_utf8_lossy(e.as_ref());
                    apply_text(ds, &path_stack, &text);
                }
            }
            Ok(Event::End(e)) => {
                let tag = tag_local_name(e.name().as_ref());
                if tag == "data-source" {
                    if let Some(ds) = current.take() {
                        out.push(ds);
                    }
                    path_stack.clear();
                } else if current.is_some() {
                    path_stack.pop();
                }
            }
            Ok(Event::Eof) => break,
            Err(err) => {
                return Err(CoreError::ParseError(format!(
                    "failed parsing DataGrip XML: {err}"
                )))
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(out)
}

fn apply_data_source_attrs(e: &BytesStart<'_>, ds: &mut RawDataSource) -> CoreResult<()> {
    for attr in e.attributes() {
        let attr = attr.map_err(|err| CoreError::ParseError(format!("xml attr error: {err}")))?;
        let val = attr
            .unescape_value()
            .map_err(|err| CoreError::ParseError(format!("xml attr value error: {err}")))?
            .into_owned();
        match attr.key.as_ref() {
            b"uuid" => ds.uuid = non_empty(val),
            b"name" => ds.name = non_empty(val),
            _ => {}
        }
    }
    Ok(())
}

fn handle_element_attrs(
    tag: &str,
    e: &BytesStart<'_>,
    ds: &mut RawDataSource,
    stack: &[String],
) -> CoreResult<()> {
    if tag == "ssh-properties" && ds.ssh.is_none() {
        ds.ssh = Some(RawSshProps::default());
    } else if tag == "ssl-config" && ds.ssl.is_none() {
        ds.ssl = Some(SslConfig::default());
    } else if tag == "property" && stack.iter().any(|p| p == "driver-properties") {
        let mut prop_name = None;
        let mut prop_val = String::new();
        for attr in e.attributes() {
            let attr = attr.map_err(|err| CoreError::ParseError(format!("xml attr error: {err}")))?;
            let val = attr
                .unescape_value()
                .map_err(|err| CoreError::ParseError(format!("xml attr value error: {err}")))?
                .into_owned();
            match attr.key.as_ref() {
                b"name" => prop_name = non_empty(val),
                b"value" => prop_val = val,
                _ => {}
            }
        }
        if let Some(k) = prop_name {
            ds.driver_properties.insert(k, prop_val);
        }
    } else if tag == "node" && stack.iter().any(|p| p == "schema-mapping") {
        for attr in e.attributes() {
            let attr = attr.map_err(|err| CoreError::ParseError(format!("xml attr error: {err}")))?;
            if attr.key.as_ref() == b"qname" {
                let val = attr
                    .unescape_value()
                    .map_err(|err| CoreError::ParseError(format!("xml attr value error: {err}")))?
                    .into_owned();
                if !val.is_empty() && ds.schema_pattern.is_none() {
                    ds.schema_pattern = Some(val);
                }
            }
        }
    } else if tag == "read-only" {
        for attr in e.attributes() {
            let attr = attr.map_err(|err| CoreError::ParseError(format!("xml attr error: {err}")))?;
            if attr.key.as_ref() == b"value" {
                let val = attr
                    .unescape_value()
                    .map_err(|err| CoreError::ParseError(format!("xml attr value error: {err}")))?;
                ds.read_only = Some(val.eq_ignore_ascii_case("true"));
            }
        }
    }
    Ok(())
}

fn apply_text(ds: &mut RawDataSource, stack: &[String], text: &str) {
    let trimmed = text.trim();
    if trimmed.is_empty() || stack.is_empty() {
        return;
    }

    let leaf = stack.last().map(String::as_str).unwrap_or("");
    let parent = stack
        .len()
        .checked_sub(2)
        .and_then(|i| stack.get(i))
        .map(String::as_str);

    match (parent, leaf) {
        (Some("ssh-properties"), "enabled") => {
            let ssh = ds.ssh.get_or_insert_with(RawSshProps::default);
            ssh.enabled = trimmed.eq_ignore_ascii_case("true");
        }
        (Some("ssh-properties"), "ssh-config-id") => {
            let ssh = ds.ssh.get_or_insert_with(RawSshProps::default);
            ssh.ssh_config_id = Some(trimmed.to_string());
        }
        (Some("ssh-properties"), "host") => {
            let ssh = ds.ssh.get_or_insert_with(RawSshProps::default);
            ssh.host = Some(trimmed.to_string());
        }
        (Some("ssh-properties"), "port") => {
            if let Ok(p) = trimmed.parse::<u16>() {
                let ssh = ds.ssh.get_or_insert_with(RawSshProps::default);
                ssh.port = Some(p);
            }
        }
        (Some("ssh-properties"), "user") | (Some("ssh-properties"), "username") => {
            let ssh = ds.ssh.get_or_insert_with(RawSshProps::default);
            ssh.username = Some(trimmed.to_string());
        }
        (Some("ssh-properties"), "auth-type") => {
            let ssh = ds.ssh.get_or_insert_with(RawSshProps::default);
            ssh.auth_type = Some(trimmed.to_string());
        }
        (Some("ssh-properties"), "key-file") | (Some("ssh-properties"), "key-path") => {
            let ssh = ds.ssh.get_or_insert_with(RawSshProps::default);
            ssh.key_file = Some(trimmed.to_string());
        }
        (Some("ssl-config"), "enabled") => {
            let ssl = ds.ssl.get_or_insert_with(SslConfig::default);
            ssl.enabled = trimmed.eq_ignore_ascii_case("true");
        }
        (Some("ssl-config"), "ca-cert") => {
            let ssl = ds.ssl.get_or_insert_with(SslConfig::default);
            ssl.ca_cert = Some(trimmed.to_string());
        }
        (Some("ssl-config"), "client-cert") => {
            let ssl = ds.ssl.get_or_insert_with(SslConfig::default);
            ssl.client_cert = Some(trimmed.to_string());
        }
        (Some("ssl-config"), "client-key") => {
            let ssl = ds.ssl.get_or_insert_with(SslConfig::default);
            ssl.client_key = Some(trimmed.to_string());
        }
        (Some("ssl-config"), "mode") => {
            let ssl = ds.ssl.get_or_insert_with(SslConfig::default);
            ssl.mode = Some(trimmed.to_string());
        }
        (None, "driver-ref") => ds.driver_ref = Some(trimmed.to_string()),
        (None, "jdbc-driver") => ds.jdbc_driver = Some(trimmed.to_string()),
        (None, "jdbc-url") => ds.jdbc_url = Some(trimmed.to_string()),
        (None, "user-name") => ds.user_name = Some(trimmed.to_string()),
        (None, "comment") | (None, "remarks") => ds.comment = Some(trimmed.to_string()),
        (None, "read-only") => ds.read_only = Some(trimmed.eq_ignore_ascii_case("true")),
        (None, "color") => ds.color = Some(trimmed.to_string()),
        (None, "introspection-schemas") => ds.schema_pattern = Some(trimmed.to_string()),
        _ => {}
    }
}

fn merge_local_override(dst: &mut RawDataSource, src: RawDataSource) {
    if dst.user_name.is_none() {
        dst.user_name = src.user_name;
    }
    if dst.jdbc_url.is_none() {
        dst.jdbc_url = src.jdbc_url;
    }
    if dst.driver_ref.is_none() {
        dst.driver_ref = src.driver_ref;
    }
    if dst.jdbc_driver.is_none() {
        dst.jdbc_driver = src.jdbc_driver;
    }
    if dst.comment.is_none() {
        dst.comment = src.comment;
    }
    if dst.read_only.is_none() {
        dst.read_only = src.read_only;
    }
    if dst.color.is_none() {
        dst.color = src.color;
    }
    if dst.schema_pattern.is_none() {
        dst.schema_pattern = src.schema_pattern;
    }
    for (k, v) in src.driver_properties {
        dst.driver_properties.entry(k).or_insert(v);
    }
    if let Some(src_ssh) = src.ssh {
        match &mut dst.ssh {
            Some(dst_ssh) => {
                if dst_ssh.ssh_config_id.is_none() {
                    dst_ssh.ssh_config_id = src_ssh.ssh_config_id;
                }
                if dst_ssh.host.is_none() {
                    dst_ssh.host = src_ssh.host;
                }
                if dst_ssh.port.is_none() {
                    dst_ssh.port = src_ssh.port;
                }
                if dst_ssh.username.is_none() {
                    dst_ssh.username = src_ssh.username;
                }
                if dst_ssh.auth_type.is_none() {
                    dst_ssh.auth_type = src_ssh.auth_type;
                }
                if dst_ssh.key_file.is_none() {
                    dst_ssh.key_file = src_ssh.key_file;
                }
                dst_ssh.enabled = dst_ssh.enabled || src_ssh.enabled;
            }
            None => dst.ssh = Some(src_ssh),
        }
    }
    if dst.ssl.is_none() {
        dst.ssl = src.ssl;
    }
}

fn build_profile(
    raw: RawDataSource,
    ssh_map: &HashMap<String, SshTunnelConfig>,
) -> CoreResult<Option<ConnectionProfile>> {
    // `.local.xml` entries without a JDBC URL on their own can be skipped if orphaned.
    let Some(jdbc_url) = raw.jdbc_url.clone().filter(|s| !s.trim().is_empty()) else {
        return Ok(None);
    };

    let parsed = parse_jdbc_url(
        &jdbc_url,
        raw.driver_ref.as_deref(),
        raw.jdbc_driver.as_deref(),
    )?;

    let ssh = resolve_ssh(raw.ssh, ssh_map);
    let read_only = raw.read_only.unwrap_or(false);
    let env = raw
        .color
        .as_deref()
        .map(EnvLevel::from_datagrip_color)
        .unwrap_or(EnvLevel::Unknown);

    let name = raw
        .name
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| match (&parsed.host, &parsed.database) {
            (Some(h), Some(db)) => format!("{h}/{db}"),
            (Some(h), None) => h.clone(),
            _ => parsed.db_type.as_pacer_db_type().to_string(),
        });

    let id = raw
        .uuid
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let fingerprint = ConnectionFingerprint {
        db_type: parsed.db_type.clone(),
        host: parsed.host.clone(),
        port: parsed.port,
        database: parsed.database.clone(),
    };

    let direct = DirectConfig {
        db_type: parsed.db_type.clone(),
        host: parsed.host,
        port: parsed.port,
        database: parsed.database,
        jdbc_url: parsed.raw_url,
        driver_ref: raw.driver_ref,
        driver_class: raw.jdbc_driver,
        username: raw.user_name,
        driver_properties: raw.driver_properties,
        ssh,
        ssl: raw.ssl,
        read_only,
        schema_pattern: raw.schema_pattern,
    };

    let mut tags = Vec::new();
    if read_only {
        tags.push("READ_ONLY".to_string());
    }
    if direct.ssh.as_ref().is_some_and(|s| s.enabled) {
        tags.push("SSH_TUNNEL".to_string());
    }

    Ok(Some(ConnectionProfile {
        id,
        name,
        source_uuid: raw.uuid,
        db_type: parsed.db_type,
        env,
        color: raw.color,
        comment: raw.comment,
        tags,
        resource_groups: Vec::new(),
        capabilities: CapabilityFlags::direct_full(read_only),
        mode: ConnectionMode::Direct(direct),
        fingerprint,
    }))
}

fn resolve_ssh(
    raw: Option<RawSshProps>,
    ssh_map: &HashMap<String, SshTunnelConfig>,
) -> Option<SshTunnelConfig> {
    let raw = raw?;
    if let Some(cfg_id) = raw.ssh_config_id.as_deref() {
        if let Some(mut resolved) = ssh_map.get(cfg_id).cloned() {
            resolved.enabled = raw.enabled || resolved.enabled;
            return Some(resolved);
        }
    }

    if raw.enabled || raw.host.is_some() || raw.ssh_config_id.is_some() {
        return Some(SshTunnelConfig {
            id: raw.ssh_config_id,
            host: raw.host.unwrap_or_default(),
            port: raw.port.unwrap_or(22),
            username: raw.username.unwrap_or_default(),
            auth_type: raw.auth_type.unwrap_or_else(|| "OPEN_SSH".to_string()),
            key_file: raw.key_file,
            enabled: raw.enabled,
        });
    }

    None
}

fn tag_local_name(raw: &[u8]) -> String {
    let stripped = match raw.iter().rposition(|b| *b == b':') {
        Some(idx) => &raw[idx + 1..],
        None => raw,
    };
    String::from_utf8_lossy(stripped).into_owned()
}

fn non_empty(s: String) -> Option<String> {
    if s.trim().is_empty() {
        None
    } else {
        Some(s)
    }
}
