use std::collections::{BTreeMap, HashMap};
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use aes::Aes128;
use cbc::cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyIvInit};
use quick_xml::events::Event;
use quick_xml::Reader;
use serde_json::{json, Map, Value};
use uuid::Uuid;
use zip::ZipArchive;

use pacer_core::{
    CapabilityFlags, ConnectionFingerprint, ConnectionMode, ConnectionProfile, CoreError,
    CoreResult, DbeaverDiscoveryItem, DbEngineType, DirectConfig, EnvLevel, SshTunnelConfig,
    SslConfig,
};

use crate::jdbc::{build_jdbc_url, parse_connection_url};

type Aes128CbcDec = cbc::Decryptor<Aes128>;

/// Standard Apache-2.0 local credential encryption key used by DBeaver Community
/// (`org.jkiss.dbeaver.model.impl.app.LocalSecretController` / `SimpleStringEncrypter`).
const DBEAVER_LOCAL_AES_KEY: [u8; 16] = [
    0xBA, 0xBB, 0x4A, 0x9F, 0x77, 0x4A, 0xB8, 0x53, 0xC9, 0x6C, 0x2D, 0x65, 0x3D, 0xFE, 0x54, 0x4A,
];

/// Imported DBeaver connection profile together with optional decoded password (kept out of JSON profiles).
#[derive(Debug, Clone)]
pub struct ImportedDbeaverConnection {
    pub profile: ConnectionProfile,
    pub password: Option<String>,
    pub save_password: bool,
}

#[derive(Debug, Clone, Default)]
pub struct DbeaverCredentialPair {
    pub user: Option<String>,
    pub password: Option<String>,
}

/// Auto-discovers local DBeaver workspace connection files across Windows, macOS, and Linux.
pub fn discover_local_dbeaver_files() -> Vec<DbeaverDiscoveryItem> {
    let mut candidate_roots: Vec<PathBuf> = Vec::new();

    if let Ok(appdata) = std::env::var("APPDATA") {
        candidate_roots.push(PathBuf::from(&appdata).join("DBeaverData").join("workspace6"));
        candidate_roots.push(PathBuf::from(&appdata).join("DBeaverData"));
    }
    if let Ok(userprofile) = std::env::var("USERPROFILE") {
        candidate_roots.push(PathBuf::from(&userprofile).join(".dbeaver4"));
        candidate_roots.push(
            PathBuf::from(&userprofile)
                .join("AppData")
                .join("Roaming")
                .join("DBeaverData")
                .join("workspace6"),
        );
    }
    if let Ok(home) = std::env::var("HOME") {
        let h = PathBuf::from(&home);
        candidate_roots.push(h.join("Library").join("DBeaverData").join("workspace6"));
        candidate_roots.push(
            h.join(".local")
                .join("share")
                .join("DBeaverData")
                .join("workspace6"),
        );
        candidate_roots.push(h.join(".dbeaver4"));
    }

    let mut seen_paths: Vec<String> = Vec::new();
    let mut discovered: Vec<DbeaverDiscoveryItem> = Vec::new();

    for root in candidate_roots {
        if !root.is_dir() {
            continue;
        }
        scan_dbeaver_root(&root, 0, &mut seen_paths, &mut discovered);
    }

    discovered
}

fn scan_dbeaver_root(
    dir: &Path,
    depth: usize,
    seen: &mut Vec<String>,
    out: &mut Vec<DbeaverDiscoveryItem>,
) {
    if depth > 3 {
        return;
    }

    let modern = dir.join(".dbeaver").join("data-sources.json");
    if modern.is_file() {
        push_discovered_item(&modern, dir, "data-sources.json", seen, out);
    }

    let direct_modern = dir.join("data-sources.json");
    if direct_modern.is_file() {
        push_discovered_item(&direct_modern, dir, "data-sources.json", seen, out);
    }

    let legacy = dir.join(".dbeaver-data-sources.xml");
    if legacy.is_file() {
        push_discovered_item(&legacy, dir, ".dbeaver-data-sources.xml", seen, out);
    }

    if let Ok(rd) = std::fs::read_dir(dir) {
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('.') && name != ".dbeaver" {
                    continue;
                }
                scan_dbeaver_root(&p, depth + 1, seen, out);
            }
        }
    }
}

fn push_discovered_item(
    file_path: &Path,
    workspace_dir: &Path,
    format: &str,
    seen: &mut Vec<String>,
    out: &mut Vec<DbeaverDiscoveryItem>,
) {
    let norm = file_path.to_string_lossy().replace('\\', "/");
    if seen.iter().any(|s| s.eq_ignore_ascii_case(&norm)) {
        return;
    }
    seen.push(norm.clone());

    let connection_count = std::fs::read_to_string(file_path)
        .ok()
        .and_then(|text| parse_dbeaver_text(&text, None).ok())
        .map(|list| list.len())
        .unwrap_or(0);

    let has_credentials_file = file_path
        .parent()
        .map(|p| p.join("credentials-config.json").is_file())
        .unwrap_or(false);

    let workspace_name = workspace_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("DBeaver")
        .to_string();

    out.push(DbeaverDiscoveryItem {
        path: norm,
        workspace_name,
        format: format.to_string(),
        connection_count,
        has_credentials_file,
    });
}

/// Imports DBeaver connections from a `.dbp`/`.zip` archive, `data-sources.json`,
/// `.dbeaver-data-sources.xml`, or a `.dbeaver` / DBeaver workspace directory.
pub fn import_dbeaver_from_path(
    path: &Path,
    include_passwords: bool,
) -> CoreResult<Vec<ImportedDbeaverConnection>> {
    if !path.exists() {
        return Err(CoreError::InvalidConfig(format!(
            "路径不存在: {}",
            path.display()
        )));
    }

    if path.is_file() {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if ext == "dbp" || ext == "zip" {
            let bytes = std::fs::read(path)?;
            return import_dbeaver_zip(&bytes, include_passwords);
        }

        let content = std::fs::read_to_string(path)?;
        let creds = if include_passwords {
            load_sibling_credentials(path.parent())
        } else {
            HashMap::new()
        };
        return parse_dbeaver_text(&content, Some(&creds));
    }

    // Directory path: check `.dbeaver/data-sources.json`, `data-sources.json`, or `.dbeaver-data-sources.xml`.
    let candidates = [
        path.join(".dbeaver").join("data-sources.json"),
        path.join("data-sources.json"),
        path.join(".dbeaver-data-sources.xml"),
        path.join("General").join(".dbeaver").join("data-sources.json"),
    ];

    for cand in candidates {
        if cand.is_file() {
            let content = std::fs::read_to_string(&cand)?;
            let creds = if include_passwords {
                load_sibling_credentials(cand.parent())
            } else {
                HashMap::new()
            };
            return parse_dbeaver_text(&content, Some(&creds));
        }
    }

    Err(CoreError::ParseError(format!(
        "在目录中未找到 DBeaver 连接文件 (data-sources.json 或 .dbeaver-data-sources.xml): {}",
        path.display()
    )))
}

/// Imports DBeaver connections from an in-memory `.dbp` or `.zip` project export archive.
pub fn import_dbeaver_zip(
    bytes: &[u8],
    include_passwords: bool,
) -> CoreResult<Vec<ImportedDbeaverConnection>> {
    let reader = Cursor::new(bytes);
    let mut archive = ZipArchive::new(reader)
        .map_err(|e| CoreError::ParseError(format!("无效的 DBeaver .dbp / .zip 归档: {e}")))?;

    let mut json_contents: Vec<String> = Vec::new();
    let mut xml_contents: Vec<String> = Vec::new();
    let mut creds_map: HashMap<String, DbeaverCredentialPair> = HashMap::new();

    for i in 0..archive.len() {
        let Ok(mut entry) = archive.by_index(i) else {
            continue;
        };
        if entry.is_dir() {
            continue;
        }
        let lower = entry.name().replace('\\', "/").to_ascii_lowercase();
        if lower.ends_with("credentials-config.json") && include_passwords {
            let mut raw = Vec::new();
            if entry.read_to_end(&mut raw).is_ok() {
                if let Ok(parsed) = decrypt_dbeaver_credentials(&raw) {
                    creds_map.extend(parsed);
                }
            }
        } else if lower.ends_with("data-sources.json") {
            let mut s = String::new();
            if entry.read_to_string(&mut s).is_ok() {
                json_contents.push(s);
            }
        } else if lower.ends_with(".dbeaver-data-sources.xml")
            || lower.ends_with("data-sources.xml")
        {
            let mut s = String::new();
            if entry.read_to_string(&mut s).is_ok() {
                xml_contents.push(s);
            }
        }
    }

    let mut all = Vec::new();
    for j in &json_contents {
        all.extend(parse_dbeaver_json(j, Some(&creds_map))?);
    }
    for x in &xml_contents {
        all.extend(parse_dbeaver_xml(x)?);
    }

    if all.is_empty() {
        return Err(CoreError::ParseError(
            "归档中未找到有效的 DBeaver 连接配置 (data-sources.json / .dbeaver-data-sources.xml)"
                .to_string(),
        ));
    }

    Ok(all)
}

/// Auto-detects JSON (`data-sources.json`) vs legacy XML (`.dbeaver-data-sources.xml`) text and parses it.
pub fn parse_dbeaver_text(
    text: &str,
    creds: Option<&HashMap<String, DbeaverCredentialPair>>,
) -> CoreResult<Vec<ImportedDbeaverConnection>> {
    let trimmed = text.trim_start_matches('\u{feff}').trim();
    if trimmed.is_empty() {
        return Err(CoreError::ParseError("DBeaver 配置内容为空".to_string()));
    }
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        parse_dbeaver_json(trimmed, creds)
    } else if trimmed.starts_with('<') {
        parse_dbeaver_xml(trimmed)
    } else {
        Err(CoreError::ParseError(
            "无法识别的 DBeaver 配置格式，请提供 data-sources.json 或 .dbeaver-data-sources.xml"
                .to_string(),
        ))
    }
}

/// Parses modern DBeaver `data-sources.json` (DBeaver 6.1 through 25.x).
pub fn parse_dbeaver_json(
    json_text: &str,
    creds_opt: Option<&HashMap<String, DbeaverCredentialPair>>,
) -> CoreResult<Vec<ImportedDbeaverConnection>> {
    let root: Value = serde_json::from_str(json_text)
        .map_err(|e| CoreError::ParseError(format!("解析 DBeaver data-sources.json 失败: {e}")))?;

    let conn_map = root
        .get("connections")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            CoreError::ParseError("JSON 中缺少 DBeaver `connections` 字段".to_string())
        })?;

    let conn_types = root.get("connection-types").and_then(Value::as_object);
    let mut results = Vec::with_capacity(conn_map.len());

    for (conn_id, entry) in conn_map {
        let Some(obj) = entry.as_object() else {
            continue;
        };
        let name = obj
            .get("name")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(conn_id.as_str())
            .to_string();

        let provider = obj.get("provider").and_then(Value::as_str).unwrap_or("");
        let driver = obj.get("driver").and_then(Value::as_str).unwrap_or("");
        let folder = obj
            .get("folder")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string);
        let read_only = obj
            .get("read-only")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let save_password = obj
            .get("save-password")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let comment = obj
            .get("description")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string);

        let cfg = obj
            .get("configuration")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();

        let mut host = json_str(&cfg, "host");
        let mut port = json_port(&cfg, "port");
        let mut database = json_str(&cfg, "database").or_else(|| json_str(&cfg, "server"));
        let raw_url = json_str(&cfg, "url");
        let config_type = json_str(&cfg, "configurationType");
        let dbeaver_type = json_str(&cfg, "type");
        let mut username = json_str(&cfg, "user").or_else(|| json_str(&cfg, "username"));
        let mut password = json_str(&cfg, "password");

        // Merge credentials from sibling `credentials-config.json` if present.
        if let Some(cred_map) = creds_opt {
            if let Some(pair) = cred_map.get(conn_id) {
                if username.is_none() {
                    username = pair.user.clone();
                }
                if password.is_none() {
                    password = pair.password.clone();
                }
            }
        }

        let mut driver_properties = BTreeMap::new();
        if let Some(props) = cfg.get("properties").and_then(Value::as_object) {
            for (k, v) in props {
                let val_str = match v {
                    Value::String(s) => s.clone(),
                    Value::Bool(b) => b.to_string(),
                    Value::Number(n) => n.to_string(),
                    _ => continue,
                };
                driver_properties.insert(k.clone(), val_str);
            }
        }

        // Parse driver/provider -> DbEngineType
        let mut db_type = resolve_dbeaver_engine(driver, provider, raw_url.as_deref(), &name);

        // Parse URL to fill or override missing host/port/db/params
        if let Some(ref url) = raw_url {
            if let Ok(parsed) =
                parse_connection_url(url, Some(db_type.as_pacer_db_type()), Some(driver))
            {
                if matches!(db_type, DbEngineType::Unknown(_)) {
                    db_type = parsed.db_type;
                }
                let is_url_only = config_type
                    .as_deref()
                    .map(|t| t.eq_ignore_ascii_case("URL"))
                    .unwrap_or(false);
                if is_url_only || host.is_none() {
                    host = parsed.host;
                }
                if is_url_only || port.is_none() {
                    port = parsed.port;
                }
                if is_url_only || database.is_none() {
                    database = parsed.database;
                }
                if username.is_none() {
                    username = parsed.username;
                }
                if password.is_none() {
                    password = parsed.password;
                }
                for (k, v) in parsed.params {
                    driver_properties.entry(k).or_insert(v);
                }
            }
        }

        if port.is_none() {
            port = db_type.default_port();
        }

        // Extract optional SSH tunnel & SSL settings from `configuration.handlers`
        let ssh = extract_dbeaver_ssh(&cfg);
        let ssl = extract_dbeaver_ssl(&cfg, &driver_properties);

        let jdbc_url = raw_url.unwrap_or_else(|| {
            build_jdbc_url(
                &db_type,
                host.as_deref(),
                port,
                database.as_deref(),
                &driver_properties,
                None,
            )
        });

        let color = dbeaver_type
            .as_deref()
            .and_then(|t| conn_types.and_then(|m| m.get(t)))
            .and_then(|ct| ct.get("color"))
            .and_then(Value::as_str)
            .and_then(rgb_csv_to_hex);

        let env = infer_dbeaver_env(dbeaver_type.as_deref(), &name, folder.as_deref());

        let mut tags = vec!["DBeaver".to_string()];
        if let Some(ref f) = folder {
            tags.push(f.clone());
        }
        if read_only {
            tags.push("READ_ONLY".to_string());
        }

        let connection_type = if config_type
            .as_deref()
            .map(|t| t.eq_ignore_ascii_case("URL"))
            .unwrap_or(false)
        {
            Some("urlOnly".to_string())
        } else {
            Some("default".to_string())
        };

        let password_save_mode = if save_password {
            Some("forever".to_string())
        } else {
            Some("session".to_string())
        };

        let fingerprint = ConnectionFingerprint {
            db_type: db_type.clone(),
            host: host.clone(),
            port,
            database: database.clone(),
        };

        let direct = DirectConfig {
            db_type: db_type.clone(),
            host,
            port,
            database,
            jdbc_url,
            driver_ref: if driver.is_empty() {
                None
            } else {
                Some(driver.to_string())
            },
            driver_class: if provider.is_empty() {
                None
            } else {
                Some(provider.to_string())
            },
            username,
            driver_properties,
            ssh,
            ssl,
            read_only,
            schema_pattern: None,
            connection_type,
            auth_type: Some("userPassword".to_string()),
            password_save_mode,
            socket_path: None,
            has_saved_password: password.is_some(),
        };

        let profile = ConnectionProfile {
            id: format!("dbeaver-{conn_id}"),
            name,
            project_id: None,
            source_uuid: Some(conn_id.clone()),
            db_type,
            env,
            color,
            comment,
            tags,
            resource_groups: Vec::new(),
            capabilities: CapabilityFlags::direct_full(read_only),
            mode: ConnectionMode::Direct(direct),
            fingerprint,
        };

        results.push(ImportedDbeaverConnection {
            profile,
            password,
            save_password,
        });
    }

    Ok(results)
}

/// Parses legacy DBeaver `.dbeaver-data-sources.xml` files.
pub fn parse_dbeaver_xml(xml_text: &str) -> CoreResult<Vec<ImportedDbeaverConnection>> {
    let mut reader = Reader::from_str(xml_text);
    reader.config_mut().trim_text(true);

    let mut out = Vec::new();
    let mut cur_id = String::new();
    let mut cur_name = String::new();
    let mut cur_provider = String::new();
    let mut cur_driver = String::new();
    let mut cur_folder: Option<String> = None;
    let mut cur_ro = false;
    let mut cur_save_pwd = true;
    let mut cur_host: Option<String> = None;
    let mut cur_port: Option<u16> = None;
    let mut cur_db: Option<String> = None;
    let mut cur_url: Option<String> = None;
    let mut cur_user: Option<String> = None;
    let mut cur_type: Option<String> = None;
    let mut cur_props: BTreeMap<String, String> = BTreeMap::new();
    let mut in_ds = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();

                if tag == "data-source" {
                    in_ds = true;
                    cur_id = attr_val(e, "id").unwrap_or_else(|| Uuid::new_v4().to_string());
                    cur_name = attr_val(e, "name").unwrap_or_else(|| cur_id.clone());
                    cur_provider = attr_val(e, "provider").unwrap_or_default();
                    cur_driver = attr_val(e, "driver").unwrap_or_default();
                    cur_folder = attr_val(e, "folder");
                    cur_ro = attr_val(e, "read-only")
                        .map(|v| v.eq_ignore_ascii_case("true"))
                        .unwrap_or(false);
                    cur_save_pwd = attr_val(e, "save-password")
                        .map(|v| !v.eq_ignore_ascii_case("false"))
                        .unwrap_or(true);
                    cur_host = None;
                    cur_port = None;
                    cur_db = None;
                    cur_url = None;
                    cur_user = None;
                    cur_type = None;
                    cur_props.clear();
                } else if in_ds && tag == "connection" {
                    cur_host = attr_val(e, "host");
                    cur_port = attr_val(e, "port").and_then(|p| p.parse::<u16>().ok());
                    cur_db = attr_val(e, "database").or_else(|| attr_val(e, "server"));
                    cur_url = attr_val(e, "url");
                    cur_user = attr_val(e, "user");
                    cur_type = attr_val(e, "type");
                } else if in_ds && tag == "property" {
                    if let (Some(k), Some(v)) = (attr_val(e, "name"), attr_val(e, "value")) {
                        cur_props.insert(k, v);
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if tag == "data-source" && in_ds {
                    in_ds = false;
                    let mut db_type = resolve_dbeaver_engine(
                        &cur_driver,
                        &cur_provider,
                        cur_url.as_deref(),
                        &cur_name,
                    );
                    if let Some(ref url) = cur_url {
                        if let Ok(parsed) = parse_connection_url(
                            url,
                            Some(db_type.as_pacer_db_type()),
                            Some(&cur_driver),
                        ) {
                            if matches!(db_type, DbEngineType::Unknown(_)) {
                                db_type = parsed.db_type;
                            }
                            if cur_host.is_none() {
                                cur_host = parsed.host;
                            }
                            if cur_port.is_none() {
                                cur_port = parsed.port;
                            }
                            if cur_db.is_none() {
                                cur_db = parsed.database;
                            }
                        }
                    }
                    if cur_port.is_none() {
                        cur_port = db_type.default_port();
                    }
                    let jdbc_url = cur_url.clone().unwrap_or_else(|| {
                        build_jdbc_url(
                            &db_type,
                            cur_host.as_deref(),
                            cur_port,
                            cur_db.as_deref(),
                            &cur_props,
                            None,
                        )
                    });
                    let env = infer_dbeaver_env(
                        cur_type.as_deref(),
                        &cur_name,
                        cur_folder.as_deref(),
                    );
                    let fingerprint = ConnectionFingerprint {
                        db_type: db_type.clone(),
                        host: cur_host.clone(),
                        port: cur_port,
                        database: cur_db.clone(),
                    };
                    let direct = DirectConfig {
                        db_type: db_type.clone(),
                        host: cur_host.clone(),
                        port: cur_port,
                        database: cur_db.clone(),
                        jdbc_url,
                        driver_ref: if cur_driver.is_empty() {
                            None
                        } else {
                            Some(cur_driver.clone())
                        },
                        driver_class: if cur_provider.is_empty() {
                            None
                        } else {
                            Some(cur_provider.clone())
                        },
                        username: cur_user.clone(),
                        driver_properties: cur_props.clone(),
                        ssh: None,
                        ssl: None,
                        read_only: cur_ro,
                        schema_pattern: None,
                        connection_type: Some("default".to_string()),
                        auth_type: Some("userPassword".to_string()),
                        password_save_mode: Some(if cur_save_pwd {
                            "forever".to_string()
                        } else {
                            "session".to_string()
                        }),
                        socket_path: None,
                        has_saved_password: false,
                    };
                    out.push(ImportedDbeaverConnection {
                        profile: ConnectionProfile {
                            id: format!("dbeaver-{cur_id}"),
                            name: cur_name.clone(),
                            project_id: None,
                            source_uuid: Some(cur_id.clone()),
                            db_type,
                            env,
                            color: None,
                            comment: None,
                            tags: vec!["DBeaver".to_string()],
                            resource_groups: Vec::new(),
                            capabilities: CapabilityFlags::direct_full(cur_ro),
                            mode: ConnectionMode::Direct(direct),
                            fingerprint,
                        },
                        password: None,
                        save_password: cur_save_pwd,
                    });
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(CoreError::ParseError(format!(
                    "解析 DBeaver XML 失败: {e}"
                )))
            }
            _ => {}
        }
    }

    Ok(out)
}

/// Exports PacerGrip connection profiles into a clean DBeaver `data-sources.json` (never includes passwords).
pub fn export_to_dbeaver_json(profiles: &[ConnectionProfile]) -> CoreResult<String> {
    let mut connections = Map::new();

    for p in profiles {
        let direct = match &p.mode {
            ConnectionMode::Direct(d) | ConnectionMode::Hybrid { direct: d, .. } => d,
            ConnectionMode::PacerManaged { .. } => continue,
        };

        let (provider, driver) = engine_to_dbeaver_driver(&direct.db_type);
        let dbeaver_type = match p.env {
            EnvLevel::Prod => "prod",
            EnvLevel::Test => "test",
            EnvLevel::Dev | EnvLevel::Unknown => "dev",
        };

        let mut cfg = Map::new();
        if let Some(ref h) = direct.host {
            cfg.insert("host".to_string(), Value::String(h.clone()));
        }
        if let Some(port) = direct.port {
            cfg.insert("port".to_string(), Value::String(port.to_string()));
        }
        if let Some(ref db) = direct.database {
            cfg.insert("database".to_string(), Value::String(db.clone()));
        }
        cfg.insert("url".to_string(), Value::String(direct.jdbc_url.clone()));
        cfg.insert(
            "configurationType".to_string(),
            Value::String(
                if direct.connection_type.as_deref() == Some("urlOnly") {
                    "URL".to_string()
                } else {
                    "MANUAL".to_string()
                },
            ),
        );
        cfg.insert("type".to_string(), Value::String(dbeaver_type.to_string()));
        if let Some(ref u) = direct.username {
            cfg.insert("user".to_string(), Value::String(u.clone()));
        }
        if !direct.driver_properties.is_empty() {
            let mut props = Map::new();
            for (k, v) in &direct.driver_properties {
                props.insert(k.clone(), Value::String(v.clone()));
            }
            cfg.insert("properties".to_string(), Value::Object(props));
        }

        let mut conn_obj = Map::new();
        conn_obj.insert("provider".to_string(), Value::String(provider.to_string()));
        conn_obj.insert("driver".to_string(), Value::String(driver.to_string()));
        conn_obj.insert("name".to_string(), Value::String(p.name.clone()));
        conn_obj.insert(
            "save-password".to_string(),
            Value::Bool(direct.password_save_mode.as_deref() != Some("never")),
        );
        conn_obj.insert("read-only".to_string(), Value::Bool(direct.read_only));
        if let Some(ref comment) = p.comment {
            conn_obj.insert("description".to_string(), Value::String(comment.clone()));
        }
        conn_obj.insert("configuration".to_string(), Value::Object(cfg));

        let key = p
            .source_uuid
            .clone()
            .unwrap_or_else(|| p.id.clone());
        connections.insert(key, Value::Object(conn_obj));
    }

    let root = json!({
        "folders": {},
        "connections": Value::Object(connections),
        "connection-types": {
            "dev": { "name": "Development", "color": "255,255,255", "auto-commit": true },
            "test": { "name": "Test", "color": "196,255,181", "auto-commit": true },
            "prod": { "name": "Production", "color": "247,159,129", "confirm-execute": true }
        }
    });

    serde_json::to_string_pretty(&root)
        .map_err(|e| CoreError::ParseError(format!("导出 DBeaver JSON 失败: {e}")))
}

/// Decrypts DBeaver Community's `credentials-config.json` (AES-128-CBC with 16-byte IV prefix),
/// or reads it directly if already plaintext UTF-8 JSON.
pub fn decrypt_dbeaver_credentials(
    raw_bytes: &[u8],
) -> CoreResult<HashMap<String, DbeaverCredentialPair>> {
    // 1. Plaintext JSON fast-path
    if let Ok(val) = serde_json::from_slice::<Value>(raw_bytes) {
        return Ok(extract_credentials_from_json(&val));
    }

    // 2. DBeaver AES-128-CBC encrypted binary (16-byte IV + PKCS7 ciphertext)
    if raw_bytes.len() <= 16 {
        return Err(CoreError::ParseError(
            "credentials-config.json 长度无效".to_string(),
        ));
    }

    let (iv, ciphertext) = raw_bytes.split_at(16);
    let mut buf = ciphertext.to_vec();
    let dec = Aes128CbcDec::new_from_slices(&DBEAVER_LOCAL_AES_KEY, iv)
        .map_err(|e| CoreError::ParseError(format!("AES 初始化失败: {e}")))?;
    let decrypted = dec
        .decrypt_padded_mut::<Pkcs7>(&mut buf)
        .map_err(|_| CoreError::ParseError("无法解密 DBeaver credentials-config.json".to_string()))?;

    let val: Value = serde_json::from_slice(decrypted)
        .map_err(|e| CoreError::ParseError(format!("credentials-config.json JSON 无效: {e}")))?;
    Ok(extract_credentials_from_json(&val))
}

fn extract_credentials_from_json(val: &Value) -> HashMap<String, DbeaverCredentialPair> {
    let mut map = HashMap::new();
    let Some(root_obj) = val.as_object() else {
        return map;
    };
    for (conn_id, entry) in root_obj {
        let Some(entry_obj) = entry.as_object() else {
            continue;
        };
        let conn_sec = entry_obj
            .get("#connection")
            .and_then(Value::as_object)
            .unwrap_or(entry_obj);
        let user = json_str(conn_sec, "user").or_else(|| json_str(conn_sec, "username"));
        let password = json_str(conn_sec, "password");
        if user.is_some() || password.is_some() {
            map.insert(conn_id.clone(), DbeaverCredentialPair { user, password });
        }
    }
    map
}

fn load_sibling_credentials(dir: Option<&Path>) -> HashMap<String, DbeaverCredentialPair> {
    let Some(parent) = dir else {
        return HashMap::new();
    };
    let cred_path = parent.join("credentials-config.json");
    if !cred_path.is_file() {
        return HashMap::new();
    }
    std::fs::read(&cred_path)
        .ok()
        .and_then(|bytes| decrypt_dbeaver_credentials(&bytes).ok())
        .unwrap_or_default()
}

fn resolve_dbeaver_engine(
    driver: &str,
    provider: &str,
    url: Option<&str>,
    name: &str,
) -> DbEngineType {
    let d_low = driver.to_ascii_lowercase();
    let p_low = provider.to_ascii_lowercase();

    if d_low.contains("tidb") {
        return DbEngineType::TiDb;
    }
    if d_low.contains("oceanbase") || p_low.contains("oceanbase") {
        return DbEngineType::OceanBase;
    }
    if d_low.contains("mariadb") {
        return DbEngineType::MariaDb;
    }
    if d_low.contains("opengauss") || d_low.contains("gauss") {
        return DbEngineType::OpenGauss;
    }
    if d_low.contains("kingbase") {
        return DbEngineType::Kingbase;
    }
    if d_low == "dm" || d_low.contains("dameng") {
        return DbEngineType::Dameng;
    }
    if d_low.contains("sqlite") || p_low.contains("sqlite") {
        return DbEngineType::Sqlite;
    }
    if d_low.contains("postgres") || p_low.contains("postgres") {
        return DbEngineType::PostgreSql;
    }
    if d_low.contains("oracle") || p_low.contains("oracle") {
        return DbEngineType::Oracle;
    }
    if d_low.contains("mssql") || d_low.contains("sqlserver") || p_low.contains("sqlserver") {
        return DbEngineType::SqlServer;
    }
    if d_low.contains("clickhouse") || p_low.contains("clickhouse") {
        return DbEngineType::ClickHouse;
    }
    if d_low.contains("mysql") || p_low.contains("mysql") {
        if name.to_ascii_lowercase().contains("tidb") {
            return DbEngineType::TiDb;
        }
        return DbEngineType::MySql;
    }

    if let Some(u) = url {
        if let Ok(parsed) = parse_connection_url(u, None, None) {
            return parsed.db_type;
        }
    }

    DbEngineType::from_str_loose(if !driver.is_empty() { driver } else { provider })
}

fn engine_to_dbeaver_driver(engine: &DbEngineType) -> (&'static str, &'static str) {
    match engine {
        DbEngineType::TiDb => ("mysql", "tidb"),
        DbEngineType::MariaDb => ("mysql", "mariaDB"),
        DbEngineType::OceanBase => ("mysql", "oceanbase"),
        DbEngineType::MySql => ("mysql", "mysql8"),
        DbEngineType::PostgreSql => ("postgresql", "postgres-jdbc"),
        DbEngineType::OpenGauss => ("postgresql", "opengauss"),
        DbEngineType::Kingbase => ("postgresql", "kingbase"),
        DbEngineType::Sqlite => ("sqlite", "sqlite_jdbc"),
        DbEngineType::Oracle => ("oracle", "oracle_thin"),
        DbEngineType::Dameng => ("generic", "dameng"),
        DbEngineType::SqlServer => ("sqlserver", "mssql_jdbc_ms_new"),
        DbEngineType::ClickHouse => ("clickhouse", "clickhouse"),
        DbEngineType::Unknown(_) => ("generic", "generic"),
    }
}

fn infer_dbeaver_env(conn_type: Option<&str>, name: &str, folder: Option<&str>) -> EnvLevel {
    if let Some(t) = conn_type {
        let low = t.to_ascii_lowercase();
        if low.contains("prod") {
            return EnvLevel::Prod;
        }
        if low.contains("test") || low.contains("uat") || low.contains("qa") || low.contains("stg")
        {
            return EnvLevel::Test;
        }
    }

    let combined = format!("{} {}", name, folder.unwrap_or("")).to_ascii_uppercase();
    if combined.contains("PROD") || combined.contains("生产") || combined.contains("线上") {
        EnvLevel::Prod
    } else if combined.contains("UAT")
        || combined.contains("TEST")
        || combined.contains("SIT")
        || combined.contains("PRE")
        || combined.contains("测试")
    {
        EnvLevel::Test
    } else {
        EnvLevel::Dev
    }
}

fn extract_dbeaver_ssh(cfg: &Map<String, Value>) -> Option<SshTunnelConfig> {
    let ssh = cfg
        .get("handlers")
        .and_then(Value::as_object)
        .and_then(|h| h.get("ssh_tunnel"))
        .and_then(Value::as_object)?;

    let enabled = ssh.get("enabled").and_then(Value::as_bool).unwrap_or(false);
    let props = ssh.get("properties").and_then(Value::as_object)?;
    let host = json_str(props, "host")?;
    let port = json_port(props, "port").unwrap_or(22);
    let username = json_str(props, "user").unwrap_or_default();
    let auth_type = json_str(props, "authType").unwrap_or_else(|| "PASSWORD".to_string());
    let key_file = json_str(props, "keyPath");

    Some(SshTunnelConfig {
        id: None,
        host,
        port,
        username,
        auth_type,
        key_file,
        enabled,
    })
}

fn extract_dbeaver_ssl(
    cfg: &Map<String, Value>,
    driver_properties: &BTreeMap<String, String>,
) -> Option<SslConfig> {
    if let Some(ssl) = cfg
        .get("handlers")
        .and_then(Value::as_object)
        .and_then(|h| h.get("mysql_ssl").or_else(|| h.get("postgre_ssl")))
        .and_then(Value::as_object)
    {
        let enabled = ssl.get("enabled").and_then(Value::as_bool).unwrap_or(false);
        let props = ssl.get("properties").and_then(Value::as_object);
        let mode = props.and_then(|p| json_str(p, "sslMode").or_else(|| json_str(p, "ssl.mode")));
        let ca_cert = props.and_then(|p| json_str(p, "ssl.ca.cert"));
        return Some(SslConfig {
            enabled,
            ca_cert,
            client_cert: None,
            client_key: None,
            mode,
        });
    }

    if let Some(use_ssl) = driver_properties
        .get("useSSL")
        .or_else(|| driver_properties.get("ssl"))
    {
        let enabled = !use_ssl.eq_ignore_ascii_case("false");
        return Some(SslConfig {
            enabled,
            mode: Some(if enabled {
                "preferred".to_string()
            } else {
                "disabled".to_string()
            }),
            ..Default::default()
        });
    }

    None
}

fn rgb_csv_to_hex(csv: &str) -> Option<String> {
    let parts: Vec<u8> = csv
        .split(',')
        .filter_map(|p| p.trim().parse::<u8>().ok())
        .collect();
    if parts.len() == 3 {
        Some(format!("#{:02X}{:02X}{:02X}", parts[0], parts[1], parts[2]))
    } else {
        None
    }
}

fn json_str(map: &Map<String, Value>, key: &str) -> Option<String> {
    map.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
}

fn json_port(map: &Map<String, Value>, key: &str) -> Option<u16> {
    match map.get(key)? {
        Value::Number(n) => n.as_u64().and_then(|v| u16::try_from(v).ok()),
        Value::String(s) => s.trim().parse::<u16>().ok(),
        _ => None,
    }
}

fn attr_val(e: &quick_xml::events::BytesStart<'_>, name: &str) -> Option<String> {
    for attr in e.attributes().flatten() {
        if attr.key.as_ref() == name.as_bytes() {
            if let Ok(v) = attr.unescape_value() {
                let s = v.trim();
                if !s.is_empty() {
                    return Some(s.to_string());
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use cbc::cipher::BlockEncryptMut;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    use super::*;

    type Aes128CbcEnc = cbc::Encryptor<Aes128>;

    #[test]
    fn test_tidb_jdbc_mysql_url_preserves_driver_and_empty_db() {
        let parsed = parse_connection_url(
            "jdbc:mysql://9.20.160.17:4000",
            Some("TIDB"),
            None,
        )
        .unwrap();
        assert_eq!(parsed.db_type, DbEngineType::TiDb);
        assert_eq!(parsed.host.as_deref(), Some("9.20.160.17"));
        assert_eq!(parsed.port, Some(4000));
        assert_eq!(parsed.database, None);

        let rebuilt = build_jdbc_url(
            &DbEngineType::TiDb,
            Some("9.20.160.17"),
            Some(4000),
            None,
            &BTreeMap::new(),
            None,
        );
        assert_eq!(rebuilt, "jdbc:mysql://9.20.160.17:4000");
    }

    #[test]
    fn test_native_uri_with_credentials_and_params() {
        let parsed = parse_connection_url(
            "mysql://policy_d01_uat:s3cr3t@9.20.160.17:4000/policy_db?useSSL=false&serverTimezone=Asia/Shanghai",
            Some("TIDB"),
            None,
        )
        .unwrap();
        assert_eq!(parsed.db_type, DbEngineType::TiDb);
        assert_eq!(parsed.host.as_deref(), Some("9.20.160.17"));
        assert_eq!(parsed.port, Some(4000));
        assert_eq!(parsed.database.as_deref(), Some("policy_db"));
        assert_eq!(parsed.username.as_deref(), Some("policy_d01_uat"));
        assert_eq!(parsed.password.as_deref(), Some("s3cr3t"));
        assert_eq!(
            parsed.params.get("useSSL").map(String::as_str),
            Some("false")
        );
    }

    #[test]
    fn test_parse_dbeaver_json_and_export_roundtrip() {
        let sample = r#"{
          "folders": { "UAT": {} },
          "connections": {
            "tidb-18d9-uat": {
              "provider": "mysql",
              "driver": "tidb",
              "name": "policy-UAT-9.20.160.17",
              "save-password": true,
              "read-only": false,
              "folder": "UAT",
              "configuration": {
                "host": "9.20.160.17",
                "port": "4000",
                "url": "jdbc:mysql://9.20.160.17:4000",
                "configurationType": "MANUAL",
                "type": "test",
                "user": "policy_d01_uat",
                "properties": { "useSSL": "false" },
                "handlers": {
                  "ssh_tunnel": {
                    "type": "TUNNEL",
                    "enabled": true,
                    "properties": { "host": "10.0.0.1", "port": 22, "user": "jump" }
                  }
                }
              }
            },
            "pg-prod-1": {
              "provider": "postgresql",
              "driver": "postgres-jdbc",
              "name": "Billing Prod",
              "read-only": true,
              "configuration": {
                "url": "jdbc:postgresql://10.10.1.9:5432/billing",
                "configurationType": "URL",
                "type": "prod",
                "user": "readonly"
              }
            }
          }
        }"#;

        let list = parse_dbeaver_json(sample, None).unwrap();
        assert_eq!(list.len(), 2);

        let tidb = list
            .iter()
            .find(|c| c.profile.name == "policy-UAT-9.20.160.17")
            .unwrap();
        assert_eq!(tidb.profile.db_type, DbEngineType::TiDb);
        assert_eq!(tidb.profile.env, EnvLevel::Test);
        if let ConnectionMode::Direct(ref d) = tidb.profile.mode {
            assert_eq!(d.host.as_deref(), Some("9.20.160.17"));
            assert_eq!(d.port, Some(4000));
            assert_eq!(d.database, None);
            assert_eq!(d.username.as_deref(), Some("policy_d01_uat"));
            assert!(d.ssh.as_ref().is_some_and(|s| s.enabled && s.host == "10.0.0.1"));
        } else {
            panic!("expected Direct mode");
        }

        let pg = list
            .iter()
            .find(|c| c.profile.name == "Billing Prod")
            .unwrap();
        assert_eq!(pg.profile.db_type, DbEngineType::PostgreSql);
        assert_eq!(pg.profile.env, EnvLevel::Prod);
        if let ConnectionMode::Direct(ref d) = pg.profile.mode {
            assert_eq!(d.host.as_deref(), Some("10.10.1.9"));
            assert_eq!(d.port, Some(5432));
            assert_eq!(d.database.as_deref(), Some("billing"));
            assert_eq!(d.connection_type.as_deref(), Some("urlOnly"));
            assert!(d.read_only);
        }

        // Export must succeed and never leak passwords
        let profiles: Vec<ConnectionProfile> = list.into_iter().map(|i| i.profile).collect();
        let exported = export_to_dbeaver_json(&profiles).unwrap();
        assert!(exported.contains("policy-UAT-9.20.160.17"));
        assert!(exported.contains("jdbc:mysql://9.20.160.17:4000"));
        assert!(!exported.contains("\"password\""));
    }

    #[test]
    fn test_dbeaver_aes_credentials_and_dbp_archive_import() {
        let cred_json = r##"{"tidb-1":{"#connection":{"user":"policy_d01_uat","password":"my_uat_password"}}}"##;
        let iv = [0x11u8; 16];
        let pt = cred_json.as_bytes();
        let mut buf = vec![0u8; pt.len() + 16];
        buf[..pt.len()].copy_from_slice(pt);
        let enc = Aes128CbcEnc::new_from_slices(&DBEAVER_LOCAL_AES_KEY, &iv).unwrap();
        let ct = enc.encrypt_padded_mut::<Pkcs7>(&mut buf, pt.len()).unwrap();

        let mut encrypted_file = Vec::with_capacity(16 + ct.len());
        encrypted_file.extend_from_slice(&iv);
        encrypted_file.extend_from_slice(ct);

        let ds_json = r#"{
          "connections": {
            "tidb-1": {
              "provider": "mysql",
              "driver": "tidb",
              "name": "policy-UAT-9.20.160.17",
              "save-password": true,
              "configuration": {
                "host": "9.20.160.17",
                "port": "4000",
                "url": "jdbc:mysql://9.20.160.17:4000"
              }
            }
          }
        }"#;

        let mut zip_buf = Cursor::new(Vec::new());
        {
            let mut zw = ZipWriter::new(&mut zip_buf);
            let opts = SimpleFileOptions::default();
            zw.start_file("General/.dbeaver/data-sources.json", opts)
                .unwrap();
            zw.write_all(ds_json.as_bytes()).unwrap();
            zw.start_file("General/.dbeaver/credentials-config.json", opts)
                .unwrap();
            zw.write_all(&encrypted_file).unwrap();
            zw.finish().unwrap();
        }

        let imported = import_dbeaver_zip(zip_buf.get_ref(), true).unwrap();
        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].password.as_deref(), Some("my_uat_password"));
        if let ConnectionMode::Direct(ref d) = imported[0].profile.mode {
            assert_eq!(d.username.as_deref(), Some("policy_d01_uat"));
            assert!(d.has_saved_password);
        }
    }

    #[test]
    fn test_parse_legacy_dbeaver_xml() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <data-sources>
          <data-source id="mysql-legacy-1" provider="mysql" driver="mysql8" name="Legacy MySQL" read-only="false">
            <connection host="192.168.1.50" port="3306" database="erp" url="jdbc:mysql://192.168.1.50:3306/erp" user="erp_ro" type="dev"/>
          </data-source>
        </data-sources>"#;

        let list = parse_dbeaver_xml(xml).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].profile.name, "Legacy MySQL");
        assert_eq!(list[0].profile.db_type, DbEngineType::MySql);
    }
}
