pub mod client;
pub mod proxy_backend;
pub mod types;

pub use client::PacerClient;
pub use proxy_backend::PacerProxyBackend;
pub use types::*;

#[cfg(test)]
mod tests {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::sync::mpsc;

    use pacer_core::{ConnectionMode, DbBackend, DbEngineType, EnvLevel, QueryReq};

    use super::*;

    #[test]
    fn test_deserialize_ticket_18_digit_id_and_pending_numeric_id() {
        let ticket_json = r#"{
            "code": "00000",
            "message": "操作成功",
            "data": {
                "id": "184930284756192041",
                "businessKey": "TICKET-2026-001",
                "instanceId": 12,
                "status": "AUDITING",
                "type": "SQL_AUDIT",
                "dbName": "orders"
            }
        }"#;
        let parsed: ApiResult<SqlTicketDto> = serde_json::from_str(ticket_json).unwrap();
        let ticket = parsed.into_result().unwrap();
        assert_eq!(ticket.id, "184930284756192041");
        assert_eq!(ticket.ticket_type.as_deref(), Some("SQL_AUDIT"));

        let pending_json = r#"{
            "code": "00000",
            "data": [{
                "id": 9007199254740999,
                "businessKey": "BK-99",
                "type": "DDL_CHANGE",
                "instanceName": "Prod MySQL",
                "currentNodeName": "DBA复核"
            }]
        }"#;
        let pending: ApiResult<Vec<PendingApprovalDto>> =
            serde_json::from_str(pending_json).unwrap();
        let list = pending.into_result().unwrap();
        assert_eq!(list[0].id, "9007199254740999");
        assert_eq!(list[0].current_node_name.as_deref(), Some("DBA复核"));
    }

    #[test]
    fn test_pacer_instance_conversion_and_tolerant_lists() {
        let inst: PacerInstance = serde_json::from_str(
            r#"{
                "id": 42,
                "name": "核心承保生产库",
                "dbType": "mysql",
                "host": "10.20.30.40",
                "port": 3306,
                "databaseName": "policy_core",
                "env": "PROD",
                "resourceGroups": "[\"车险承保资源组\", \"核心账务资源组\"]",
                "tags": "核心生产库,高可用集群",
                "supportedOps": "[\"支持查询\", \"支持数据脱敏\", \"支持事务预执行\"]"
            }"#,
        )
        .unwrap();

        let profile = inst.to_connection_profile("http://localhost:8088/");
        assert_eq!(profile.id, "pacer:42");
        assert_eq!(profile.db_type, DbEngineType::MySql);
        assert_eq!(profile.env, EnvLevel::Prod);
        assert_eq!(profile.tags, vec!["核心生产库", "高可用集群"]);
        assert_eq!(
            profile.resource_groups,
            vec!["车险承保资源组", "核心账务资源组"]
        );
        assert!(profile.capabilities.can_query);
        assert!(profile.capabilities.has_masking);
        assert!(profile.capabilities.can_dry_run);
        assert!(!profile.capabilities.can_dml);
        assert!(matches!(
            profile.mode,
            ConnectionMode::PacerManaged {
                instance_id: 42,
                ..
            }
        ));
    }

    #[test]
    fn test_dry_run_item_accepts_jackson_dml_field() {
        let raw = r#"{
            "code": "00000",
            "data": {
                "passed": true,
                "totalStatements": 1,
                "dmlCount": 1,
                "totalExpectedRows": 2,
                "totalActualRows": 2,
                "items": [{
                    "index": 1,
                    "statementType": "UPDATE",
                    "dml": true,
                    "sqlSnippet": "UPDATE t SET a = 1 WHERE id IN (1,2)",
                    "expectedRows": 2,
                    "actualRows": 2,
                    "status": "MATCHED",
                    "durationMs": 5
                }]
            }
        }"#;
        let parsed: ApiResult<DryRunResultDto> = serde_json::from_str(raw).unwrap();
        let res = parsed.into_result().unwrap();
        assert!(res.passed);
        assert!(res.items[0].is_dml);
    }

    async fn spawn_mock_pacer() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            loop {
                let Ok((mut stream, _)) = listener.accept().await else {
                    break;
                };
                tokio::spawn(async move {
                    let mut buf = vec![0u8; 32768];
                    let mut total = 0;
                    // Read until headers end + body if Content-Length present.
                    let mut expected_total = None;
                    loop {
                        let n = stream.read(&mut buf[total..]).await.unwrap_or(0);
                        if n == 0 {
                            break;
                        }
                        total += n;
                        let s = String::from_utf8_lossy(&buf[..total]);
                        if expected_total.is_none() {
                            if let Some(hdr_end) = s.find("\r\n\r\n") {
                                let headers = &s[..hdr_end];
                                let content_len = headers
                                    .lines()
                                    .find_map(|l| {
                                        let (k, v) = l.split_once(':')?;
                                        if k.eq_ignore_ascii_case("content-length") {
                                            v.trim().parse::<usize>().ok()
                                        } else {
                                            None
                                        }
                                    })
                                    .unwrap_or(0);
                                expected_total = Some(hdr_end + 4 + content_len);
                            }
                        }
                        if let Some(exp) = expected_total {
                            if total >= exp {
                                break;
                            }
                        }
                    }

                    let req_text = String::from_utf8_lossy(&buf[..total]).to_string();
                    let first_line = req_text.lines().next().unwrap_or("");
                    let lower_headers = req_text.to_ascii_lowercase();

                    let body = if first_line.starts_with("POST /api/v1/auth/login") {
                        r#"{"code":"00000","message":"ok","data":{"token":"jwt-test-token","username":"testadmin1","realName":"系统管理员","role":"ADMIN"}}"#.to_string()
                    } else if first_line.starts_with("POST /api/v1/ticket/query") {
                        assert!(lower_headers.contains("authorization: bearer jwt-test-token"));
                        assert!(lower_headers.contains("x-tenant-id: 1"));
                        r#"{"code":"00000","data":{"success":true,"databaseName":"orders","columns":["id","phone"],"rows":[{"id":1,"phone":"138****0001"}],"maskedColumns":{"phone":"手机号掩码"},"totalRows":1,"durationMs":11,"routeNode":"SLAVE","routeDescription":"已智能路由至只读从库"}}"#.to_string()
                    } else if first_line.starts_with("POST /api/v1/ticket/dry-run") {
                        assert!(lower_headers.contains("multipart/form-data"));
                        assert!(req_text.contains("name=\"instanceId\""));
                        assert!(req_text.contains("name=\"sqlText\""));
                        r#"{"code":"00000","data":{"passed":true,"totalStatements":1,"dmlCount":1,"totalExpectedRows":1,"totalActualRows":1,"summaryMessage":"校验通过","items":[{"index":1,"statementType":"UPDATE","dml":true,"expectedRows":1,"actualRows":1,"status":"MATCHED","durationMs":3}]}}"#.to_string()
                    } else {
                        r#"{"code":"A0404","message":"not found"}"#.to_string()
                    };

                    let resp = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = stream.write_all(resp.as_bytes()).await;
                });
            }
        });

        format!("http://{addr}")
    }

    #[tokio::test]
    async fn test_mock_pacer_login_query_streaming_and_multipart_dry_run() {
        let base_url = spawn_mock_pacer().await;
        let client = PacerClient::new(&base_url).unwrap();

        let login = client
            .login(&LoginRequest::with_password("testadmin1", "123456"))
            .await
            .unwrap();
        assert_eq!(login.token, "jwt-test-token");
        assert_eq!(client.token().as_deref(), Some("jwt-test-token"));

        let instance = PacerInstance {
            tenant_id: Some("1".to_string()),
            id: 7,
            name: "Orders Slave".to_string(),
            db_type: Some("mysql".to_string()),
            host: Some("10.0.0.5".to_string()),
            port: Some(3306),
            database_name: Some("orders".to_string()),
            jdbc_url: None,
            read_only_jdbc_url: None,
            username: None,
            env: Some("PROD".to_string()),
            status: Some("APPROVED".to_string()),
            resource_groups: None,
            tags: None,
            supported_ops: Some("[\"支持查询\", \"支持数据脱敏\"]".to_string()),
            connection_tunnel: None,
            fixed_workflow_template_id: None,
            fixed_workflow_template_name: None,
            description: None,
            instance_config: None,
        };

        let backend = PacerProxyBackend::new(client.clone(), instance);
        let (tx, mut rx) = mpsc::channel(8);
        let summary = backend
            .execute_query(
                QueryReq {
                    database: Some("orders".to_string()),
                    sql: "SELECT id, phone FROM t_customer LIMIT 10".to_string(),
                    limit: Some(10),
                    query_id: None,
                },
                tx,
            )
            .await
            .unwrap();

        assert!(summary.success);
        assert_eq!(summary.route_node.as_deref(), Some("SLAVE"));
        assert_eq!(
            summary.masked_columns.get("phone").map(String::as_str),
            Some("手机号掩码")
        );

        let chunk = rx.recv().await.unwrap();
        assert!(chunk.is_last);
        assert_eq!(chunk.rows.len(), 1);
        assert_eq!(chunk.rows[0]["phone"], "138****0001");

        let dry = client
            .dry_run(7, Some("orders"), "-- 1\nUPDATE orders SET status='OK' WHERE id=1")
            .await
            .unwrap();
        assert!(dry.passed);
        assert_eq!(dry.total_actual_rows, 1);
    }

    #[test]
    fn test_api_error_maps_auth_and_permission_codes() {
        let err401 = ApiResult::<serde_json::Value> {
            code: "A0220".to_string(),
            message: Some("token expired".to_string()),
            data: None,
        }
        .into_result()
        .unwrap_err();
        assert!(matches!(err401, pacer_core::CoreError::AuthRequired(_)));

        let err403 = ApiResult::<serde_json::Value> {
            code: "A0403".to_string(),
            message: Some("read only denied".to_string()),
            data: None,
        }
        .into_result()
        .unwrap_err();
        assert!(matches!(err403, pacer_core::CoreError::PermissionDenied(_)));
    }
}
