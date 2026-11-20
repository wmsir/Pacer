use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use reqwest::multipart::Form;
use reqwest::{Client, Method, RequestBuilder};
use serde::de::DeserializeOwned;
use serde::Serialize;

use pacer_core::{CoreError, CoreResult};

use crate::types::{
    AiSqlRequest, AiText2SqlRequest, ApiResult, ApprovalActionRequest, DbSchemaDetailDto,
    DbSessionDto, DbVariableDto, DbaFeedbackRequest, DryRunResultDto, LoginRequest, LoginResponse,
    MaskingRuleDto, PacerInstance, PageResultDto, PendingApprovalDto, QrLoginDto,
    SaveInstancePayload, SqlQueryRequest, SqlQueryResultDto, SqlTicketDto, SubmitTicketRequest,
    TestConnectionParamsRequest, TestConnectionResultDto, TicketOperationLogDto, UserInfoDto,
};

/// Async HTTP SDK for `wmsir/Pacer` (`wmDB`) governance platform.
#[derive(Clone)]
pub struct PacerClient {
    http: Client,
    base_url: String,
    token: Arc<RwLock<Option<String>>>,
    tenant_id: Arc<RwLock<String>>,
}

impl PacerClient {
    pub fn new(base_url: impl Into<String>) -> CoreResult<Self> {
        let http = Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(8))
            .build()
            .map_err(|err| CoreError::Transport(format!("failed initializing HTTP client: {err}")))?;

        Ok(Self {
            http,
            base_url: base_url.into().trim_end_matches('/').to_string(),
            token: Arc::new(RwLock::new(None)),
            tenant_id: Arc::new(RwLock::new("1".to_string())),
        })
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn set_token(&self, token: Option<String>) {
        if let Ok(mut guard) = self.token.write() {
            *guard = token;
        }
    }

    pub fn token(&self) -> Option<String> {
        self.token.read().ok().and_then(|g| g.clone())
    }

    pub fn set_tenant_id(&self, tenant_id: impl Into<String>) {
        let val = tenant_id.into();
        if val.trim().is_empty() {
            return;
        }
        if let Ok(mut guard) = self.tenant_id.write() {
            *guard = val;
        }
    }

    pub fn tenant_id(&self) -> String {
        self.tenant_id
            .read()
            .ok()
            .map(|g| g.clone())
            .unwrap_or_else(|| "1".to_string())
    }

    fn endpoint(&self, path: &str) -> String {
        if path.starts_with('/') {
            format!("{}{}", self.base_url, path)
        } else {
            format!("{}/{}", self.base_url, path)
        }
    }

    fn prepare(&self, method: Method, path: &str) -> RequestBuilder {
        let mut req = self.http.request(method, self.endpoint(path));
        req = req.header("X-Tenant-Id", self.tenant_id());
        if let Some(tok) = self.token() {
            if !tok.is_empty() {
                req = req.bearer_auth(tok);
            }
        }
        req
    }

    async fn send_json<T: DeserializeOwned>(&self, req: RequestBuilder) -> CoreResult<T> {
        let resp = req
            .send()
            .await
            .map_err(|err| CoreError::Transport(format!("HTTP request failed: {err}")))?;

        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|err| CoreError::Transport(format!("failed reading response body: {err}")))?;

        if status.as_u16() == 401 {
            return Err(CoreError::AuthRequired(extract_error_msg(&body)));
        }
        if status.as_u16() == 403 {
            return Err(CoreError::PermissionDenied(extract_error_msg(&body)));
        }
        if !status.is_success() {
            if let Ok(api) = serde_json::from_str::<ApiResult<serde_json::Value>>(&body) {
                return Err(CoreError::ApiError {
                    code: api.code,
                    message: api.message.unwrap_or_else(|| format!("HTTP {status}")),
                });
            }
            return Err(CoreError::Transport(format!("HTTP {status}: {body}")));
        }

        let envelope: ApiResult<T> = serde_json::from_str(&body).map_err(|err| {
            CoreError::ParseError(format!("invalid Pacer JSON response: {err}; body={body}"))
        })?;
        envelope.into_result()
    }

    async fn send_empty(&self, req: RequestBuilder) -> CoreResult<()> {
        let resp = req
            .send()
            .await
            .map_err(|err| CoreError::Transport(format!("HTTP request failed: {err}")))?;

        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|err| CoreError::Transport(format!("failed reading response body: {err}")))?;

        if status.as_u16() == 401 {
            return Err(CoreError::AuthRequired(extract_error_msg(&body)));
        }
        if status.as_u16() == 403 {
            return Err(CoreError::PermissionDenied(extract_error_msg(&body)));
        }

        let envelope: ApiResult<serde_json::Value> = serde_json::from_str(&body).map_err(|err| {
            CoreError::ParseError(format!("invalid Pacer JSON response: {err}; body={body}"))
        })?;
        envelope.into_empty()
    }

    // =========================================================
    // Auth & QR Login
    // =========================================================

    pub async fn login(&self, req: &LoginRequest) -> CoreResult<LoginResponse> {
        let resp: LoginResponse = self
            .send_json(self.prepare(Method::POST, "/api/v1/auth/login").json(req))
            .await?;
        self.set_token(Some(resp.token.clone()));
        Ok(resp)
    }

    pub async fn qr_generate(&self, channel: &str) -> CoreResult<QrLoginDto> {
        self.send_json(
            self.prepare(Method::POST, "/api/v1/auth/qr/generate")
                .query(&[("channel", channel)]),
        )
        .await
    }

    pub async fn qr_status(&self, qr_key: &str) -> CoreResult<QrLoginDto> {
        let dto: QrLoginDto = self
            .send_json(
                self.prepare(Method::GET, "/api/v1/auth/qr/status")
                    .query(&[("qrKey", qr_key)]),
            )
            .await?;
        if dto.status.eq_ignore_ascii_case("CONFIRMED") {
            if let Some(ref tok) = dto.token {
                self.set_token(Some(tok.clone()));
            }
        }
        Ok(dto)
    }

    pub async fn user_info(&self) -> CoreResult<UserInfoDto> {
        self.send_json(self.prepare(Method::GET, "/api/v1/auth/user-info"))
            .await
    }

    // =========================================================
    // Instances & Metadata
    // =========================================================

    pub async fn list_instances(&self) -> CoreResult<Vec<PacerInstance>> {
        self.send_json(self.prepare(Method::GET, "/api/v1/instance/list"))
            .await
    }

    pub async fn get_instance(&self, id: i64) -> CoreResult<PacerInstance> {
        self.send_json(self.prepare(Method::GET, &format!("/api/v1/instance/{id}")))
            .await
    }

    pub async fn test_connection_params(
        &self,
        req: &TestConnectionParamsRequest,
    ) -> CoreResult<TestConnectionResultDto> {
        self.send_json(
            self.prepare(Method::POST, "/api/v1/instance/test-connection-params")
                .json(req),
        )
        .await
    }

    pub async fn test_connection(&self, id: i64) -> CoreResult<TestConnectionResultDto> {
        self.send_json(self.prepare(
            Method::POST,
            &format!("/api/v1/instance/{id}/test-connection"),
        ))
        .await
    }

    pub async fn save_instance(&self, payload: &SaveInstancePayload) -> CoreResult<()> {
        self.send_empty(
            self.prepare(Method::POST, "/api/v1/instance/save")
                .json(payload),
        )
        .await
    }

    pub async fn list_databases(&self, id: i64) -> CoreResult<Vec<String>> {
        self.send_json(self.prepare(Method::GET, &format!("/api/v1/instance/{id}/databases")))
            .await
    }

    pub async fn list_databases_detail(&self, id: i64) -> CoreResult<Vec<DbSchemaDetailDto>> {
        self.send_json(self.prepare(
            Method::GET,
            &format!("/api/v1/instance/{id}/databases-detail"),
        ))
        .await
    }

    pub async fn list_tables(
        &self,
        id: i64,
        db_name: &str,
    ) -> CoreResult<Vec<BTreeMap<String, serde_json::Value>>> {
        let encoded_db = encode_segment(db_name);
        self.send_json(self.prepare(
            Method::GET,
            &format!("/api/v1/instance/{id}/databases/{encoded_db}/tables"),
        ))
        .await
    }

    pub async fn list_columns(
        &self,
        id: i64,
        db_name: &str,
        table_name: &str,
    ) -> CoreResult<Vec<BTreeMap<String, serde_json::Value>>> {
        let encoded_db = encode_segment(db_name);
        let encoded_table = encode_segment(table_name);
        self.send_json(self.prepare(
            Method::GET,
            &format!("/api/v1/instance/{id}/databases/{encoded_db}/tables/{encoded_table}/columns"),
        ))
        .await
    }

    pub async fn list_sessions(&self, id: i64) -> CoreResult<Vec<DbSessionDto>> {
        self.send_json(self.prepare(Method::GET, &format!("/api/v1/instance/{id}/sessions")))
            .await
    }

    pub async fn kill_session(&self, id: i64, process_id: i64) -> CoreResult<()> {
        self.send_empty(self.prepare(
            Method::POST,
            &format!("/api/v1/instance/{id}/sessions/{process_id}/kill"),
        ))
        .await
    }

    pub async fn list_variables_page(
        &self,
        id: i64,
        page: u32,
        size: u32,
        keyword: Option<&str>,
    ) -> CoreResult<PageResultDto<DbVariableDto>> {
        let mut req = self
            .prepare(Method::GET, &format!("/api/v1/instance/{id}/variables/page"))
            .query(&[("page", page.to_string()), ("size", size.to_string())]);
        if let Some(kw) = keyword.filter(|s| !s.is_empty()) {
            req = req.query(&[("keyword", kw)]);
        }
        self.send_json(req).await
    }

    // =========================================================
    // Query, Dry-Run, & Tickets
    // =========================================================

    pub async fn execute_query(&self, req: &SqlQueryRequest) -> CoreResult<SqlQueryResultDto> {
        self.send_json(self.prepare(Method::POST, "/api/v1/ticket/query").json(req))
            .await
    }

    pub async fn execute_explain(&self, req: &SqlQueryRequest) -> CoreResult<SqlQueryResultDto> {
        self.send_json(
            self.prepare(Method::POST, "/api/v1/ticket/explain")
                .json(req),
        )
        .await
    }

    /// Calls `POST /api/v1/ticket/dry-run` using `multipart/form-data` matching Spring `@RequestParam`.
    pub async fn dry_run(
        &self,
        instance_id: i64,
        db_name: Option<&str>,
        sql_text: &str,
    ) -> CoreResult<DryRunResultDto> {
        let mut form = Form::new()
            .text("instanceId", instance_id.to_string())
            .text("sqlText", sql_text.to_string());
        if let Some(db) = db_name.filter(|s| !s.trim().is_empty()) {
            form = form.text("dbName", db.to_string());
        }

        self.send_json(
            self.prepare(Method::POST, "/api/v1/ticket/dry-run")
                .multipart(form),
        )
        .await
    }

    /// Calls `POST /api/v1/ticket/submit` using `multipart/form-data` matching Spring `@RequestParam`.
    pub async fn submit_ticket(&self, req: &SubmitTicketRequest) -> CoreResult<SqlTicketDto> {
        let mut form = Form::new()
            .text("instanceId", req.instance_id.to_string())
            .text(
                "type",
                req.ticket_type
                    .clone()
                    .unwrap_or_else(|| "SQL_AUDIT".to_string()),
            )
            .text("reason", req.reason.clone())
            .text("sqlText", req.sql_text.clone());

        if let Some(db) = req.db_name.as_deref().filter(|s| !s.trim().is_empty()) {
            form = form.text("dbName", db.to_string());
        }
        if let Some(rows) = req.expected_rows {
            form = form.text("expectedRows", rows.to_string());
        }
        if let Some(rb) = req
            .rollback_sql_text
            .as_deref()
            .filter(|s| !s.trim().is_empty())
        {
            form = form.text("rollbackSqlText", rb.to_string());
        }
        if let Some(cf) = req
            .custom_field_values
            .as_deref()
            .filter(|s| !s.trim().is_empty())
        {
            form = form.text("customFieldValues", cf.to_string());
        }

        self.send_json(
            self.prepare(Method::POST, "/api/v1/ticket/submit")
                .multipart(form),
        )
        .await
    }

    pub async fn pending_approvals(&self) -> CoreResult<Vec<PendingApprovalDto>> {
        self.send_json(self.prepare(Method::GET, "/api/v1/ticket/pending-approvals"))
            .await
    }

    pub async fn list_tickets(&self) -> CoreResult<Vec<SqlTicketDto>> {
        self.send_json(self.prepare(Method::GET, "/api/v1/ticket/list"))
            .await
    }

    pub async fn ticket_detail(&self, id: &str) -> CoreResult<serde_json::Value> {
        self.send_json(self.prepare(Method::GET, &format!("/api/v1/ticket/{id}/detail")))
            .await
    }

    pub async fn ticket_download_url(&self, id: &str) -> CoreResult<String> {
        #[derive(serde::Deserialize)]
        struct UrlWrap {
            url: String,
        }
        let res: UrlWrap = self
            .send_json(self.prepare(Method::GET, &format!("/api/v1/ticket/{id}/download-url")))
            .await?;
        Ok(res.url)
    }

    pub async fn ticket_logs(&self, id: &str) -> CoreResult<Vec<TicketOperationLogDto>> {
        self.send_json(self.prepare(Method::GET, &format!("/api/v1/ticket/{id}/logs")))
            .await
    }

    pub async fn approve_ticket(
        &self,
        id: &str,
        req: &ApprovalActionRequest,
    ) -> CoreResult<serde_json::Value> {
        self.send_json(
            self.prepare(Method::POST, &format!("/api/v1/ticket/{id}/approve"))
                .json(req),
        )
        .await
    }

    pub async fn reject_ticket(
        &self,
        id: &str,
        comment: Option<&str>,
    ) -> CoreResult<()> {
        let body = ApprovalActionRequest {
            execution_mode: None,
            scheduled_time: None,
            batch_size: None,
            interval_ms: None,
            comment: comment.map(str::to_string),
        };
        self.send_empty(
            self.prepare(Method::POST, &format!("/api/v1/ticket/{id}/reject"))
                .json(&body),
        )
        .await
    }

    pub async fn submit_dba_feedback(
        &self,
        id: &str,
        req: &DbaFeedbackRequest,
    ) -> CoreResult<()> {
        self.send_empty(
            self.prepare(Method::POST, &format!("/api/v1/ticket/{id}/feedback"))
                .json(req),
        )
        .await
    }

    pub async fn execute_now(&self, id: &str) -> CoreResult<serde_json::Value> {
        self.send_json(self.prepare(Method::POST, &format!("/api/v1/ticket/{id}/execute-now")))
            .await
    }

    // =========================================================
    // AI & Governance
    // =========================================================

    pub async fn ai_text2sql(
        &self,
        prompt: &str,
        db_type: &str,
    ) -> CoreResult<serde_json::Value> {
        self.send_json(
            self.prepare(Method::POST, "/api/v1/ai/text2sql")
                .json(&AiText2SqlRequest {
                    prompt: prompt.to_string(),
                    db_type: db_type.to_string(),
                }),
        )
        .await
    }

    pub async fn ai_optimize(&self, sql: &str) -> CoreResult<String> {
        self.ai_sql_endpoint("/api/v1/ai/optimize", sql).await
    }

    pub async fn ai_explain(&self, sql: &str) -> CoreResult<String> {
        self.ai_sql_endpoint("/api/v1/ai/explain", sql).await
    }

    pub async fn ai_rewrite(&self, sql: &str) -> CoreResult<String> {
        self.ai_sql_endpoint("/api/v1/ai/rewrite", sql).await
    }

    pub async fn ai_risk(&self, sql: &str) -> CoreResult<String> {
        self.ai_sql_endpoint("/api/v1/ai/risk", sql).await
    }

    pub async fn ai_explain_plan(&self, sql: &str) -> CoreResult<String> {
        self.ai_sql_endpoint("/api/v1/ai/explain-plan", sql).await
    }

    async fn ai_sql_endpoint(&self, path: &str, sql: &str) -> CoreResult<String> {
        self.send_json(self.prepare(Method::POST, path).json(&AiSqlRequest {
            sql: sql.to_string(),
        }))
        .await
    }

    pub async fn ai_dba_ask(&self, question: &str) -> CoreResult<String> {
        #[derive(Serialize)]
        struct AskReq<'a> {
            question: &'a str,
        }
        self.send_json(
            self.prepare(Method::POST, "/api/v1/ai-dba/ask")
                .json(&AskReq { question }),
        )
        .await
    }

    pub async fn masking_rules(
        &self,
        instance_id: Option<i64>,
        db_name: Option<&str>,
        table_name: Option<&str>,
    ) -> CoreResult<Vec<MaskingRuleDto>> {
        let mut req = self.prepare(Method::GET, "/api/v1/masking-rule/list");
        if let Some(id) = instance_id {
            req = req.query(&[("instanceId", id.to_string())]);
        }
        if let Some(db) = db_name.filter(|s| !s.is_empty()) {
            req = req.query(&[("dbName", db)]);
        }
        if let Some(tbl) = table_name.filter(|s| !s.is_empty()) {
            req = req.query(&[("tableName", tbl)]);
        }
        self.send_json(req).await
    }

    pub async fn generate_inspection(
        &self,
        instance_id: i64,
    ) -> CoreResult<serde_json::Value> {
        self.send_json(
            self.prepare(Method::POST, "/api/v1/inspection/generate")
                .query(&[("instanceId", instance_id.to_string())]),
        )
        .await
    }
}

fn encode_segment(raw: &str) -> String {
    url::form_urlencoded::byte_serialize(raw.as_bytes()).collect()
}

fn extract_error_msg(body: &str) -> String {
    serde_json::from_str::<ApiResult<serde_json::Value>>(body)
        .ok()
        .and_then(|r| r.message)
        .unwrap_or_else(|| body.to_string())
}
