pub mod jdbc;
pub mod matcher;
pub mod ssh;
pub mod xml_exporter;
pub mod xml_parser;

pub use jdbc::{parse_jdbc_url, ParsedJdbcUrl};
pub use matcher::{match_profiles_with_pacer, MatchKind, MatchResult};
pub use ssh::parse_ssh_configs_xml;
pub use xml_exporter::export_to_datagrip_xml;
pub use xml_parser::{
    import_datagrip_zip, import_from_path, parse_datagrip_bundle, parse_datagrip_xml,
};

#[cfg(test)]
mod tests {
    use std::io::Write;

    use pacer_core::{
        CapabilityFlags, ConnectionFingerprint, ConnectionMode, ConnectionProfile, DbEngineType,
        EnvLevel,
    };
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    use super::*;

    #[test]
    fn test_jdbc_mysql_with_query_params() {
        let parsed = parse_jdbc_url(
            "jdbc:mysql://10.10.1.25:3307/orders_prod?useSSL=false&serverTimezone=Asia/Shanghai",
            Some("mysql.8"),
            None,
        )
        .unwrap();
        assert_eq!(parsed.db_type, DbEngineType::MySql);
        assert_eq!(parsed.host.as_deref(), Some("10.10.1.25"));
        assert_eq!(parsed.port, Some(3307));
        assert_eq!(parsed.database.as_deref(), Some("orders_prod"));
        assert_eq!(parsed.params.get("useSSL").map(String::as_str), Some("false"));
    }

    #[test]
    fn test_jdbc_domestic_engines_default_ports() {
        let dm = parse_jdbc_url("jdbc:dm://192.168.10.5/DAMENG", None, None).unwrap();
        assert_eq!(dm.db_type, DbEngineType::Dameng);
        assert_eq!(dm.port, Some(5236));
        assert_eq!(dm.database.as_deref(), Some("DAMENG"));

        let kb = parse_jdbc_url("jdbc:kingbase8://db.internal/paydb", None, None).unwrap();
        assert_eq!(kb.db_type, DbEngineType::Kingbase);
        assert_eq!(kb.port, Some(54321));

        let og = parse_jdbc_url("jdbc:opengauss://10.0.0.8:26000/core", None, None).unwrap();
        assert_eq!(og.db_type, DbEngineType::OpenGauss);
        assert_eq!(og.port, Some(26000));

        let tidb = parse_jdbc_url(
            "jdbc:mysql://tidb-vip:4000/analytics",
            Some("tidb"),
            None,
        )
        .unwrap();
        assert_eq!(tidb.db_type, DbEngineType::TiDb);
        assert_eq!(tidb.port, Some(4000));
    }

    #[test]
    fn test_jdbc_oracle_thin_sid_and_service_forms() {
        let svc = parse_jdbc_url("jdbc:oracle:thin:@//ora-host:1522/orclpdb1", None, None).unwrap();
        assert_eq!(svc.db_type, DbEngineType::Oracle);
        assert_eq!(svc.host.as_deref(), Some("ora-host"));
        assert_eq!(svc.port, Some(1522));
        assert_eq!(svc.database.as_deref(), Some("orclpdb1"));

        let sid = parse_jdbc_url("jdbc:oracle:thin:@10.2.3.4:1521:ORCL", None, None).unwrap();
        assert_eq!(sid.db_type, DbEngineType::Oracle);
        assert_eq!(sid.host.as_deref(), Some("10.2.3.4"));
        assert_eq!(sid.port, Some(1521));
        assert_eq!(sid.database.as_deref(), Some("ORCL"));
    }

    #[test]
    fn test_jdbc_sqlserver_semicolon_and_sqlite() {
        let mssql = parse_jdbc_url(
            "jdbc:sqlserver://mssql01.corp:1433;databaseName=erp_db;encrypt=false",
            None,
            None,
        )
        .unwrap();
        assert_eq!(mssql.db_type, DbEngineType::SqlServer);
        assert_eq!(mssql.host.as_deref(), Some("mssql01.corp"));
        assert_eq!(mssql.port, Some(1433));
        assert_eq!(mssql.database.as_deref(), Some("erp_db"));
        assert_eq!(mssql.params.get("encrypt").map(String::as_str), Some("false"));

        let sqlite = parse_jdbc_url("jdbc:sqlite:C:/data/local_cache.db", None, None).unwrap();
        assert_eq!(sqlite.db_type, DbEngineType::Sqlite);
        assert_eq!(sqlite.host, None);
        assert_eq!(sqlite.database.as_deref(), Some("C:/data/local_cache.db"));
    }

    #[test]
    fn test_parse_clipboard_xml_single_datasource() {
        let xml = r#"
        <data-source source="LOCAL" name="Prod Orders MySQL" uuid="11111111-2222-3333-4444-555555555555">
          <driver-ref>mysql.8</driver-ref>
          <jdbc-driver>com.mysql.cj.jdbc.Driver</jdbc-driver>
          <jdbc-url>jdbc:mysql://prod-db.internal:3306/orders?useSSL=true&amp;tinyInt1isBit=false</jdbc-url>
          <user-name>readonly_dev</user-name>
          <read-only>true</read-only>
          <color>#D93025</color>
          <comment>Core orders database</comment>
          <driver-properties>
            <property name="connectTimeout" value="3000" />
          </driver-properties>
        </data-source>
        "#;

        let profiles = parse_datagrip_xml(xml).unwrap();
        assert_eq!(profiles.len(), 1);
        let p = &profiles[0];
        assert_eq!(p.name, "Prod Orders MySQL");
        assert_eq!(p.env, EnvLevel::Prod);
        assert!(!p.capabilities.can_dml);
        assert!(p.capabilities.can_query);
        if let ConnectionMode::Direct(d) = &p.mode {
            assert!(d.jdbc_url.contains("useSSL=true&tinyInt1isBit=false"));
            assert_eq!(d.username.as_deref(), Some("readonly_dev"));
            assert_eq!(
                d.driver_properties.get("connectTimeout").map(String::as_str),
                Some("3000")
            );
        } else {
            panic!("expected direct mode");
        }
    }

    #[test]
    fn test_parse_component_bundle_merges_local_and_ssh() {
        let main_xml = r#"
        <project version="4">
          <component name="DataSourceManagerImpl" format="xml" multifile-model="true">
            <data-source source="LOCAL" name="PG Staging" uuid="uuid-pg-1">
              <driver-ref>postgresql</driver-ref>
              <jdbc-url>jdbc:postgresql://10.0.1.50:5432/staging</jdbc-url>
              <color>Amber</color>
            </data-source>
          </component>
        </project>
        "#;
        let local_xml = r#"
        <project version="4">
          <component name="DataSourceManagerImpl" format="xml" multifile-model="true">
            <data-source name="PG Staging" uuid="uuid-pg-1">
              <user-name>pg_ops</user-name>
              <ssh-properties>
                <enabled>true</enabled>
                <ssh-config-id>ssh-jump-01</ssh-config-id>
              </ssh-properties>
            </data-source>
          </component>
        </project>
        "#;
        let ssh_xml = r#"
        <application>
          <component name="SshConfigs">
            <configs>
              <sshConfig id="ssh-jump-01" host="bastion.corp" port="2222" username="jumpuser" authType="KEY_PAIR" keyPath="/home/dev/.ssh/id_ed25519" />
            </configs>
          </component>
        </application>
        "#;

        let profiles = parse_datagrip_bundle(main_xml, Some(local_xml), Some(ssh_xml)).unwrap();
        assert_eq!(profiles.len(), 1);
        let p = &profiles[0];
        assert_eq!(p.env, EnvLevel::Test);
        if let ConnectionMode::Direct(d) = &p.mode {
            assert_eq!(d.username.as_deref(), Some("pg_ops"));
            let ssh = d.ssh.as_ref().expect("ssh config should merge");
            assert!(ssh.enabled);
            assert_eq!(ssh.host, "bastion.corp");
            assert_eq!(ssh.port, 2222);
            assert_eq!(ssh.username, "jumpuser");
            assert_eq!(ssh.key_file.as_deref(), Some("/home/dev/.ssh/id_ed25519"));
        } else {
            panic!("expected direct mode");
        }
    }

    #[test]
    fn test_import_datagrip_zip_in_memory() {
        let mut cursor = std::io::Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut cursor);
            let opts = SimpleFileOptions::default();
            zip.start_file("options/dataSources.xml", opts).unwrap();
            zip.write_all(
                br#"<component name="DataSourceManagerImpl"><data-source name="Local Dev" uuid="u1"><driver-ref>mysql.8</driver-ref><jdbc-url>jdbc:mysql://127.0.0.1:3306/devdb</jdbc-url><color>Green</color></data-source></component>"#,
            )
            .unwrap();
            zip.start_file("options/dataSources.local.xml", opts).unwrap();
            zip.write_all(
                br#"<component name="DataSourceManagerImpl"><data-source name="Local Dev" uuid="u1"><user-name>dev_root</user-name></data-source></component>"#,
            )
            .unwrap();
            zip.finish().unwrap();
        }

        let profiles = import_datagrip_zip(cursor.get_ref()).unwrap();
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].env, EnvLevel::Dev);
        if let ConnectionMode::Direct(d) = &profiles[0].mode {
            assert_eq!(d.username.as_deref(), Some("dev_root"));
            assert_eq!(d.database.as_deref(), Some("devdb"));
        } else {
            panic!("expected direct mode");
        }
    }

    #[test]
    fn test_export_roundtrip_contains_no_password() {
        let xml = r#"
        <component name="DataSourceManagerImpl">
          <data-source source="LOCAL" name="DM8 Xinchuang" uuid="dm-uuid-1">
            <driver-ref>dm</driver-ref>
            <jdbc-url>jdbc:dm://10.8.8.8:5236/FINANCE</jdbc-url>
            <user-name>SYSDBA</user-name>
            <read-only>true</read-only>
            <color>#D93025</color>
          </data-source>
        </component>
        "#;
        let profiles = parse_datagrip_xml(xml).unwrap();
        let exported = export_to_datagrip_xml(&profiles).unwrap();
        assert!(!exported.to_ascii_lowercase().contains("password"));

        let reparsed = parse_datagrip_xml(&exported).unwrap();
        assert_eq!(reparsed.len(), 1);
        assert_eq!(reparsed[0].name, "DM8 Xinchuang");
        assert_eq!(reparsed[0].db_type, DbEngineType::Dameng);
        assert_eq!(reparsed[0].fingerprint.port, Some(5236));
        assert_eq!(reparsed[0].fingerprint.database.as_deref(), Some("FINANCE"));
    }

    #[test]
    fn test_fingerprint_matcher_ranks_exact_host_port_name() {
        let local_profiles = parse_datagrip_xml(
            r#"
            <component name="DataSourceManagerImpl">
              <data-source name="OrdersProd" uuid="l1">
                <driver-ref>mysql.8</driver-ref>
                <jdbc-url>jdbc:mysql://127.0.0.1:3306/orders</jdbc-url>
              </data-source>
              <data-source name="Billing" uuid="l2">
                <driver-ref>postgresql</driver-ref>
                <jdbc-url>jdbc:postgresql://pg.internal:5432/other_db</jdbc-url>
              </data-source>
              <data-source name="UnmatchedLocal" uuid="l3">
                <driver-ref>sqlite.xerial</driver-ref>
                <jdbc-url>jdbc:sqlite:/tmp/test.db</jdbc-url>
              </data-source>
            </component>
            "#,
        )
        .unwrap();

        let pacer_profiles = vec![
            ConnectionProfile {
                id: "pacer:101".to_string(),
                name: "核心订单生产库".to_string(),
                source_uuid: None,
                db_type: DbEngineType::MySql,
                env: EnvLevel::Prod,
                color: None,
                comment: None,
                tags: vec![],
                resource_groups: vec![],
                capabilities: CapabilityFlags::from_pacer_ops(&[]),
                mode: ConnectionMode::PacerManaged {
                    workspace_url: "http://localhost:8088".to_string(),
                    instance_id: 101,
                    env: EnvLevel::Prod,
                },
                fingerprint: ConnectionFingerprint {
                    db_type: DbEngineType::MySql,
                    host: Some("localhost".to_string()),
                    port: None, // defaults to 3306
                    database: Some("ORDERS".to_string()),
                },
            },
            ConnectionProfile {
                id: "pacer:102".to_string(),
                name: "PG计费库".to_string(),
                source_uuid: None,
                db_type: DbEngineType::PostgreSql,
                env: EnvLevel::Test,
                color: None,
                comment: None,
                tags: vec![],
                resource_groups: vec![],
                capabilities: CapabilityFlags::from_pacer_ops(&[]),
                mode: ConnectionMode::PacerManaged {
                    workspace_url: "http://localhost:8088".to_string(),
                    instance_id: 102,
                    env: EnvLevel::Test,
                },
                fingerprint: ConnectionFingerprint {
                    db_type: DbEngineType::PostgreSql,
                    host: Some("pg.internal".to_string()),
                    port: Some(5432),
                    database: Some("billing".to_string()),
                },
            },
        ];

        let results = match_profiles_with_pacer(&local_profiles, &pacer_profiles);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].match_kind, MatchKind::ExactHostPortDb);
        assert_eq!(results[0].matched_pacer_instance_id, Some(101));
        assert!(results[0].upgrade_recommended);

        assert_eq!(results[1].match_kind, MatchKind::HostPortOnly);
        assert_eq!(results[1].matched_pacer_instance_id, Some(102));
        assert!(results[1].upgrade_recommended);

        assert_eq!(results[2].match_kind, MatchKind::Unmatched);
        assert_eq!(results[2].matched_pacer_instance_id, None);
        assert!(!results[2].upgrade_recommended);
    }

    #[test]
    fn test_invalid_jdbc_url_returns_parse_error() {
        let err = parse_jdbc_url("http://localhost:3306/db", None, None).unwrap_err();
        assert!(err.to_string().contains("missing jdbc:"));
    }

    #[test]
    fn test_import_from_directory_and_xml_path() {
        let tmp = std::env::temp_dir().join(format!("pacergrip_test_{}", uuid::Uuid::new_v4()));
        let idea_dir = tmp.join(".idea");
        std::fs::create_dir_all(&idea_dir).unwrap();
        std::fs::write(
            idea_dir.join("dataSources.xml"),
            r#"<component name="DataSourceManagerImpl"><data-source name="DirTest" uuid="d1"><driver-ref>postgresql</driver-ref><jdbc-url>jdbc:postgresql://10.1.1.1/app</jdbc-url></data-source></component>"#,
        )
        .unwrap();
        std::fs::write(
            idea_dir.join("dataSources.local.xml"),
            r#"<component name="DataSourceManagerImpl"><data-source name="DirTest" uuid="d1"><user-name>dir_user</user-name></data-source></component>"#,
        )
        .unwrap();

        let from_dir = import_from_path(&tmp).unwrap();
        assert_eq!(from_dir.len(), 1);
        assert_eq!(from_dir[0].name, "DirTest");
        if let ConnectionMode::Direct(d) = &from_dir[0].mode {
            assert_eq!(d.username.as_deref(), Some("dir_user"));
        } else {
            panic!("expected direct mode");
        }

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_pacer_ops_capability_mapping() {
        let ops = vec!["支持查询".to_string(), "支持数据脱敏".to_string()];
        let caps = CapabilityFlags::from_pacer_ops(&ops);
        assert!(caps.can_query);
        assert!(caps.has_masking);
        assert!(!caps.can_dml);
        assert!(!caps.can_ddl);
        assert!(!caps.can_export);
    }
}
