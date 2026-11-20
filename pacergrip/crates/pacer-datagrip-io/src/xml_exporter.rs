use std::io::Cursor;

use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;

use pacer_core::{
    ConnectionMode, ConnectionProfile, CoreError, CoreResult, DbEngineType, DirectConfig,
};

/// Exports connection profiles as DataGrip clipboard-compatible XML.
/// Security invariant: passwords and secret credentials are NEVER serialized.
pub fn export_to_datagrip_xml(profiles: &[ConnectionProfile]) -> CoreResult<String> {
    let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 2);

    let mut comp = BytesStart::new("component");
    comp.push_attribute(("name", "DataSourceManagerImpl"));
    comp.push_attribute(("format", "xml"));
    comp.push_attribute(("multifile-model", "true"));
    write_err(writer.write_event(Event::Start(comp)))?;

    for profile in profiles {
        let mut ds = BytesStart::new("data-source");
        ds.push_attribute(("source", "LOCAL"));
        ds.push_attribute(("name", profile.name.as_str()));
        let uuid_str = profile
            .source_uuid
            .as_deref()
            .unwrap_or(profile.id.as_str());
        ds.push_attribute(("uuid", uuid_str));
        write_err(writer.write_event(Event::Start(ds)))?;

        match &profile.mode {
            ConnectionMode::Direct(direct) | ConnectionMode::Hybrid { direct, .. } => {
                write_direct_children(&mut writer, profile, direct)?;
            }
            ConnectionMode::PacerManaged { instance_id, .. } => {
                let driver_ref = default_driver_ref(&profile.db_type);
                let jdbc_url = synthesize_jdbc_url(profile);
                write_text_elem(&mut writer, "driver-ref", driver_ref)?;
                write_text_elem(&mut writer, "jdbc-url", &jdbc_url)?;
                write_text_elem(
                    &mut writer,
                    "read-only",
                    if !profile.capabilities.can_dml && !profile.capabilities.can_ddl {
                        "true"
                    } else {
                        "false"
                    },
                )?;
                let note = profile.comment.clone().unwrap_or_else(|| {
                    format!("Pacer managed instance #{instance_id} (credentials managed by Pacer)")
                });
                write_text_elem(&mut writer, "comment", &note)?;
            }
        }

        if let Some(color) = &profile.color {
            write_text_elem(&mut writer, "color", color)?;
        }

        write_err(writer.write_event(Event::End(BytesEnd::new("data-source"))))?;
    }

    write_err(writer.write_event(Event::End(BytesEnd::new("component"))))?;

    let bytes = writer.into_inner().into_inner();
    String::from_utf8(bytes)
        .map_err(|err| CoreError::ParseError(format!("UTF-8 encoding error: {err}")))
}

fn write_direct_children(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    profile: &ConnectionProfile,
    direct: &DirectConfig,
) -> CoreResult<()> {
    if let Some(driver_ref) = &direct.driver_ref {
        write_text_elem(writer, "driver-ref", driver_ref)?;
    } else {
        write_text_elem(writer, "driver-ref", default_driver_ref(&direct.db_type))?;
    }

    if let Some(driver_class) = &direct.driver_class {
        write_text_elem(writer, "jdbc-driver", driver_class)?;
    }

    write_text_elem(writer, "jdbc-url", &direct.jdbc_url)?;

    if let Some(user) = &direct.username {
        write_text_elem(writer, "user-name", user)?;
    }

    if direct.read_only {
        write_text_elem(writer, "read-only", "true")?;
    }

    if let Some(comment) = &profile.comment {
        write_text_elem(writer, "comment", comment)?;
    }

    if !direct.driver_properties.is_empty() {
        write_err(writer.write_event(Event::Start(BytesStart::new("driver-properties"))))?;
        for (k, v) in &direct.driver_properties {
            let mut prop = BytesStart::new("property");
            prop.push_attribute(("name", k.as_str()));
            prop.push_attribute(("value", v.as_str()));
            write_err(writer.write_event(Event::Empty(prop)))?;
        }
        write_err(writer.write_event(Event::End(BytesEnd::new("driver-properties"))))?;
    }

    if let Some(ssh) = &direct.ssh {
        write_err(writer.write_event(Event::Start(BytesStart::new("ssh-properties"))))?;
        write_text_elem(
            writer,
            "enabled",
            if ssh.enabled { "true" } else { "false" },
        )?;
        if let Some(id) = &ssh.id {
            write_text_elem(writer, "ssh-config-id", id)?;
        }
        if !ssh.host.is_empty() {
            write_text_elem(writer, "host", &ssh.host)?;
            write_text_elem(writer, "port", &ssh.port.to_string())?;
        }
        if !ssh.username.is_empty() {
            write_text_elem(writer, "user", &ssh.username)?;
        }
        if !ssh.auth_type.is_empty() {
            write_text_elem(writer, "auth-type", &ssh.auth_type)?;
        }
        if let Some(key_file) = &ssh.key_file {
            write_text_elem(writer, "key-file", key_file)?;
        }
        write_err(writer.write_event(Event::End(BytesEnd::new("ssh-properties"))))?;
    }

    if let Some(ssl) = &direct.ssl {
        write_err(writer.write_event(Event::Start(BytesStart::new("ssl-config"))))?;
        write_text_elem(
            writer,
            "enabled",
            if ssl.enabled { "true" } else { "false" },
        )?;
        if let Some(ca) = &ssl.ca_cert {
            write_text_elem(writer, "ca-cert", ca)?;
        }
        if let Some(cert) = &ssl.client_cert {
            write_text_elem(writer, "client-cert", cert)?;
        }
        if let Some(key) = &ssl.client_key {
            write_text_elem(writer, "client-key", key)?;
        }
        if let Some(mode) = &ssl.mode {
            write_text_elem(writer, "mode", mode)?;
        }
        write_err(writer.write_event(Event::End(BytesEnd::new("ssl-config"))))?;
    }

    Ok(())
}

fn write_text_elem(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    tag: &str,
    value: &str,
) -> CoreResult<()> {
    write_err(writer.write_event(Event::Start(BytesStart::new(tag))))?;
    write_err(writer.write_event(Event::Text(BytesText::new(value))))?;
    write_err(writer.write_event(Event::End(BytesEnd::new(tag))))?;
    Ok(())
}

fn write_err<T>(res: std::io::Result<T>) -> CoreResult<T> {
    res.map_err(|err| CoreError::ParseError(format!("xml write error: {err}")))
}

fn default_driver_ref(db_type: &DbEngineType) -> &str {
    match db_type {
        DbEngineType::MySql => "mysql.8",
        DbEngineType::MariaDb => "mariadb",
        DbEngineType::PostgreSql => "postgresql",
        DbEngineType::Oracle => "oracle",
        DbEngineType::Dameng => "dm",
        DbEngineType::Kingbase => "kingbase",
        DbEngineType::OpenGauss => "opengauss",
        DbEngineType::OceanBase => "oceanbase",
        DbEngineType::TiDb => "tidb",
        DbEngineType::SqlServer => "sqlserver.ms",
        DbEngineType::Sqlite => "sqlite.xerial",
        DbEngineType::ClickHouse => "clickhouse",
        DbEngineType::Unknown(_) => "generic",
    }
}

fn synthesize_jdbc_url(profile: &ConnectionProfile) -> String {
    let host = profile.fingerprint.host.as_deref().unwrap_or("localhost");
    let port = profile
        .fingerprint
        .effective_port()
        .map(|p| format!(":{p}"))
        .unwrap_or_default();
    let db = profile
        .fingerprint
        .database
        .as_deref()
        .unwrap_or_default();
    let suffix = if db.is_empty() {
        String::new()
    } else {
        format!("/{db}")
    };
    match profile.db_type {
        DbEngineType::Oracle => format!("jdbc:oracle:thin:@//{host}{port}{suffix}"),
        DbEngineType::SqlServer => {
            if db.is_empty() {
                format!("jdbc:sqlserver://{host}{port}")
            } else {
                format!("jdbc:sqlserver://{host}{port};databaseName={db}")
            }
        }
        DbEngineType::Dameng => format!("jdbc:dm://{host}{port}{suffix}"),
        DbEngineType::Kingbase => format!("jdbc:kingbase8://{host}{port}{suffix}"),
        DbEngineType::OpenGauss => format!("jdbc:opengauss://{host}{port}{suffix}"),
        DbEngineType::PostgreSql => format!("jdbc:postgresql://{host}{port}{suffix}"),
        DbEngineType::Sqlite => format!("jdbc:sqlite:{db}"),
        _ => format!("jdbc:mysql://{host}{port}{suffix}"),
    }
}
