use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize};

use pacer_core::{
    CapabilityFlags, ConnectionFingerprint, ConnectionMode, ConnectionProfile, CoreError,
    CoreResult, DbEngineType, EnvLevel,
};

fn default_none<T>() -> Option<T> {
    None
}

/// Standard `com.wmdb.common.Result<T>` response envelope returned by `wmsir/Pacer`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(bound(deserialize = "T: serde::Deserialize<'de>"))]
pub struct ApiResult<T> {
    pub code: String,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default = "default_none")]
    pub data: Option<T>,
}

impl<T> ApiResult<T> {
    pub fn into_result(self) -> CoreResult<T> {
        if self.code == "00000" {
            self.data.ok_or_else(|| {
                CoreError::ParseError("Pacer API returned 00000 with empty data".to_string())
            })
        } else {
            Err(map_api_error(self.code, self.message))
        }
    }

    pub fn into_empty(self) -> CoreResult<()> {
        if self.code == "00000" {
            Ok(())
        } else {
            Err(map_api_error(self.code, self.message))
        }
    }
}

fn map_api_error(code: String, message: Option<String>) -> CoreError {
    let msg = message.unwrap_or_else(|| "Pacer API request failed".to_string());
    match code.as_str() {
        "A0220" | "A0230" => CoreError::AuthRequired(msg),
        "A0301" | "A0403" => CoreError::PermissionDenied(msg),
        _ => CoreError::ApiError { code, message: msg },
    }
}

/// Deserializes either a JSON string (`"123456789012345678"`) or JSON integer into a `String`.
pub fn de_id_as_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let v = serde_json::Value::deserialize(deserializer)?;
    match v {
        serde_json::Value::String(s) => Ok(s),
        serde_json::Value::Number(n) => Ok(n.to_string()),
        other => Err(serde::de::Error::custom(format!(
            "expected string or integer id, got {other}"
        ))),
    }
}

/// Deserializes optional string/integer IDs into `Option<String>`.
pub fn de_opt_id_as_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let v = Option::<serde_json::Value>::deserialize(deserializer)?;
    match v {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(s)) => Ok(Some(s)),
        Some(serde_json::Value::Number(n)) => Ok(Some(n.to_string())),
        Some(other) => Err(serde::de::Error::custom(format!(
            "expected optional string or integer id, got {other}"
        ))),
    }
}

/// Parses Pacer fields stored either as JSON arrays (`["a","b"]`) or comma-separated strings (`a,b`).
pub fn parse_pacer_string_list(raw: Option<&str>) -> Vec<String> {
    let Some(s) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return Vec::new();
    };
    if s.starts_with('[') {
        if let Ok(arr) = serde_json::from_str::<Vec<String>>(s) {
            return arr
                .into_iter()
                .map(|x| x.trim().to_string())
                .filter(|x| !x.is_empty())
                .collect();
        }
    }
    s.split(',')
        .map(|part| part.trim().trim_matches('"').trim_matches('\'').to_string())
        .filter(|x| !x.is_empty())
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub login_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    pub system_name: String,
}

impl LoginRequest {
    pub fn with_password(account: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            login_type: Some("ACCOUNT_PASSWORD".to_string()),
            account: Some(account.into()),
            password: Some(password.into()),
            phone: None,
            code: None,
            system_name: "PacerGrip Desktop".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub token: String,
    #[serde(default)]
    pub user_id: Option<i64>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub real_name: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QrLoginDto {
    pub qr_key: String,
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(default)]
    pub channel_name: Option<String>,
    #[serde(default)]
    pub qr_url: Option<String>,
    #[serde(default)]
    pub qr_content: Option<String>,
    pub status: String,
    #[serde(default)]
    pub status_msg: Option<String>,
    #[serde(default)]
    pub expire_seconds: Option<i64>,
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default)]
    pub user: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserInfoDto {
    #[serde(default)]
    pub id: Option<i64>,
    pub username: String,
    #[serde(default)]
    pub real_name: Option<String>,
    #[serde(default)]
    pub id_card: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(default)]
    pub resource_groups: Vec<String>,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub is_admin: bool,
    #[serde(default)]
    pub is_dba: bool,
    #[serde(default)]
    pub is_dev_lead: bool,
    #[serde(default)]
    pub is_auditor: bool,
}

/// Pacer `DbInstance` view model (intentionally never exposes `passwordCipher`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PacerInstance {
    #[serde(default)]
    pub tenant_id: Option<String>,
    pub id: i64,
    pub name: String,
    #[serde(default)]
    pub db_type: Option<String>,
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub database_name: Option<String>,
    #[serde(default)]
    pub jdbc_url: Option<String>,
    #[serde(default)]
    pub read_only_jdbc_url: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub env: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub resource_groups: Option<String>,
    #[serde(default)]
    pub tags: Option<String>,
    #[serde(default)]
    pub supported_ops: Option<String>,
    #[serde(default)]
    pub connection_tunnel: Option<String>,
    #[serde(default)]
    pub fixed_workflow_template_id: Option<i64>,
    #[serde(default)]
    pub fixed_workflow_template_name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub instance_config: Option<String>,
}

impl PacerInstance {
    pub fn to_connection_profile(&self, workspace_url: &str) -> ConnectionProfile {
        let engine = DbEngineType::from_str_loose(self.db_type.as_deref().unwrap_or("mysql"));
        let env = self
            .env
            .as_deref()
            .map(EnvLevel::from_str_loose)
            .unwrap_or(EnvLevel::Unknown);
        let ops = parse_pacer_string_list(self.supported_ops.as_deref());
        let tags = parse_pacer_string_list(self.tags.as_deref());
        let resource_groups = parse_pacer_string_list(self.resource_groups.as_deref());
        let capabilities = CapabilityFlags::from_pacer_ops(&ops);

        let fingerprint = ConnectionFingerprint {
            db_type: engine.clone(),
            host: self.host.clone(),
            port: self.port.or_else(|| engine.default_port()),
            database: self.database_name.clone(),
        };

        ConnectionProfile {
            id: format!("pacer:{}", self.id),
            name: self.name.clone(),
            source_uuid: None,
            db_type: engine,
            env,
            color: None,
            comment: self.description.clone(),
            tags,
            resource_groups,
            capabilities,
            mode: ConnectionMode::PacerManaged {
                workspace_url: workspace_url.trim_end_matches('/').to_string(),
                instance_id: self.id,
                env,
            },
            fingerprint,
        }
    }
}

/// Payload for `POST /api/v1/instance/save`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveInstancePayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    pub name: String,
    pub db_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jdbc_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password_cipher: Option<String>,
    pub env: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_groups: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supported_ops: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_tunnel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestConnectionParamsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub db_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jdbc_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestConnectionResultDto {
    pub success: bool,
    #[serde(default)]
    pub latency_ms: u64,
    #[serde(default)]
    pub database_product_name: Option<String>,
    #[serde(default)]
    pub database_product_version: Option<String>,
    #[serde(default)]
    pub driver_name: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbSchemaDetailDto {
    pub db_name: String,
    #[serde(default)]
    pub table_count: u64,
    #[serde(rename = "dataSizeMB", default)]
    pub data_size_mb: f64,
    #[serde(default)]
    pub charset: Option<String>,
    #[serde(default)]
    pub collation: Option<String>,
    /// Lombok `boolean isSystem` serializes in Jackson as `"system"`.
    #[serde(rename = "system", alias = "isSystem", default)]
    pub is_system: bool,
    #[serde(default)]
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbSessionDto {
    pub id: i64,
    #[serde(default)]
    pub user: Option<String>,
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub db: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub time: Option<i64>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub info: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbVariableDto {
    pub name: String,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(bound(deserialize = "T: serde::Deserialize<'de>"))]
pub struct PageResultDto<T> {
    #[serde(default = "Vec::new")]
    pub records: Vec<T>,
    #[serde(default)]
    pub total: u64,
    #[serde(default)]
    pub current: u64,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub pages: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SqlQueryRequest {
    pub instance_id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub db_name: Option<String>,
    pub sql: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SqlQueryResultDto {
    pub success: bool,
    #[serde(default)]
    pub database_name: Option<String>,
    #[serde(default)]
    pub sql: Option<String>,
    #[serde(default)]
    pub columns: Vec<String>,
    #[serde(default)]
    pub rows: Vec<BTreeMap<String, serde_json::Value>>,
    #[serde(default)]
    pub masked_columns: BTreeMap<String, String>,
    #[serde(default)]
    pub total_rows: usize,
    #[serde(default)]
    pub duration_ms: u64,
    #[serde(default)]
    pub error_message: Option<String>,
    #[serde(default)]
    pub route_node: Option<String>,
    #[serde(default)]
    pub route_description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DryRunItemDto {
    pub index: usize,
    #[serde(default)]
    pub statement_type: Option<String>,
    #[serde(rename = "dml", alias = "isDml", default)]
    pub is_dml: bool,
    #[serde(default)]
    pub sql_snippet: Option<String>,
    #[serde(default)]
    pub expected_rows: Option<i64>,
    #[serde(default)]
    pub actual_rows: Option<i64>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DryRunResultDto {
    pub passed: bool,
    #[serde(default)]
    pub total_statements: usize,
    #[serde(default)]
    pub dml_count: usize,
    #[serde(default)]
    pub total_expected_rows: i64,
    #[serde(default)]
    pub total_actual_rows: i64,
    #[serde(default)]
    pub summary_message: Option<String>,
    #[serde(default)]
    pub items: Vec<DryRunItemDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitTicketRequest {
    pub instance_id: i64,
    pub db_name: Option<String>,
    #[serde(rename = "ticketType", alias = "type")]
    pub ticket_type: Option<String>,
    pub reason: String,
    pub sql_text: String,
    pub expected_rows: Option<i64>,
    pub rollback_sql_text: Option<String>,
    pub custom_field_values: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SqlTicketDto {
    #[serde(deserialize_with = "de_id_as_string")]
    pub id: String,
    #[serde(default)]
    pub tenant_id: Option<String>,
    #[serde(default)]
    pub business_key: Option<String>,
    #[serde(default)]
    pub instance_id: Option<i64>,
    #[serde(default)]
    pub applicant_id_card: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub risk_level: Option<String>,
    #[serde(rename = "type", default)]
    pub ticket_type: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub flow_instance_id: Option<String>,
    #[serde(default)]
    pub execution_window: Option<String>,
    #[serde(default)]
    pub workflow_template_id: Option<i64>,
    #[serde(default)]
    pub workflow_template_name: Option<String>,
    #[serde(default)]
    pub applicant_name: Option<String>,
    #[serde(default)]
    pub db_name: Option<String>,
    #[serde(default)]
    pub create_time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SqlTicketDetailDto {
    #[serde(default, deserialize_with = "de_opt_id_as_string")]
    pub id: Option<String>,
    #[serde(default, deserialize_with = "de_opt_id_as_string")]
    pub ticket_id: Option<String>,
    #[serde(default)]
    pub sql_text: Option<String>,
    #[serde(default)]
    pub attachment_oss_key: Option<String>,
    #[serde(default)]
    pub affect_rows_estimate: Option<i64>,
    #[serde(default)]
    pub rollback_sql_text: Option<String>,
    #[serde(default)]
    pub rollback_oss_key: Option<String>,
    #[serde(default)]
    pub custom_field_values: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingApprovalDto {
    #[serde(deserialize_with = "de_id_as_string")]
    pub id: String,
    #[serde(default)]
    pub business_key: Option<String>,
    #[serde(rename = "type", default)]
    pub ticket_type: Option<String>,
    #[serde(default)]
    pub instance_name: Option<String>,
    #[serde(default)]
    pub db_name: Option<String>,
    #[serde(default)]
    pub applicant_id_card: Option<String>,
    #[serde(default)]
    pub applicant_name: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub current_node_name: Option<String>,
    #[serde(default)]
    pub affect_rows: Option<i64>,
    #[serde(default)]
    pub create_time: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalActionRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_size: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval_ms: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbaFeedbackRequest {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub affect_rows: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feedback_notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TicketOperationLogDto {
    #[serde(default, deserialize_with = "de_opt_id_as_string")]
    pub id: Option<String>,
    #[serde(default, deserialize_with = "de_opt_id_as_string")]
    pub ticket_id: Option<String>,
    #[serde(default)]
    pub operator_id_card: Option<String>,
    #[serde(default)]
    pub operator_name: Option<String>,
    #[serde(default)]
    pub operation_type: Option<String>,
    #[serde(default)]
    pub node_name: Option<String>,
    #[serde(default)]
    pub comment: Option<String>,
    #[serde(default)]
    pub created_time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiText2SqlRequest {
    pub prompt: String,
    pub db_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiSqlRequest {
    pub sql: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaskingRuleDto {
    #[serde(default)]
    pub id: Option<i64>,
    #[serde(default)]
    pub instance_id: Option<i64>,
    #[serde(default)]
    pub db_name: Option<String>,
    #[serde(default)]
    pub table_name: Option<String>,
    #[serde(default)]
    pub column_name: Option<String>,
    #[serde(default)]
    pub rule_type: Option<String>,
    #[serde(default)]
    pub status: Option<i32>,
    #[serde(default)]
    pub description: Option<String>,
}
