use std::collections::HashMap;

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

use pacer_core::{CoreError, CoreResult, SshTunnelConfig};

/// Parses JetBrains DataGrip `sshConfigs.xml` into a map keyed by SSH config `id`.
pub fn parse_ssh_configs_xml(xml: &str) -> CoreResult<HashMap<String, SshTunnelConfig>> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut configs = HashMap::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(e)) | Ok(Event::Start(e)) => {
                if local_name_eq(e.name().as_ref(), b"sshConfig") {
                    if let Some((id, cfg)) = extract_ssh_config(&e)? {
                        configs.insert(id, cfg);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(err) => {
                return Err(CoreError::ParseError(format!(
                    "failed parsing sshConfigs.xml: {err}"
                )))
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(configs)
}

fn extract_ssh_config(e: &BytesStart<'_>) -> CoreResult<Option<(String, SshTunnelConfig)>> {
    let mut id = None;
    let mut host = None;
    let mut port: u16 = 22;
    let mut username = String::new();
    let mut auth_type = "OPEN_SSH".to_string();
    let mut key_file = None;
    let mut enabled = true;

    for attr in e.attributes() {
        let attr = attr.map_err(|err| CoreError::ParseError(format!("invalid xml attribute: {err}")))?;
        let key = attr.key.as_ref();
        let val = attr
            .unescape_value()
            .map_err(|err| CoreError::ParseError(format!("invalid xml attribute value: {err}")))?
            .into_owned();

        match key {
            b"id" => id = Some(val),
            b"host" => host = Some(val),
            b"port" => {
                if let Ok(p) = val.parse::<u16>() {
                    port = p;
                }
            }
            b"username" | b"user" => username = val,
            b"authType" => auth_type = val,
            b"keyPath" | b"keyFile" => {
                if !val.is_empty() {
                    key_file = Some(val);
                }
            }
            b"enabled" => enabled = !val.eq_ignore_ascii_case("false"),
            _ => {}
        }
    }

    let Some(cfg_id) = id.filter(|s| !s.is_empty()) else {
        return Ok(None);
    };

    Ok(Some((
        cfg_id.clone(),
        SshTunnelConfig {
            id: Some(cfg_id),
            host: host.unwrap_or_default(),
            port,
            username,
            auth_type,
            key_file,
            enabled,
        },
    )))
}

fn local_name_eq(raw: &[u8], expected: &[u8]) -> bool {
    let stripped = match raw.iter().rposition(|b| *b == b':') {
        Some(idx) => &raw[idx + 1..],
        None => raw,
    };
    stripped == expected
}
