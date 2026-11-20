use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::CoreResult;
use crate::model::{ConnectionProfile, DbEngineType};

/// Category of an optional IDE extension (e.g. Pacer governance, AI assistant, custom driver).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExtensionKind {
    Governance,
    AiAssistant,
    Driver,
    Auth,
}

/// Descriptive metadata exposed to the desktop UI Extensions panel.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionMeta {
    pub id: String,
    pub name: String,
    pub version: String,
    pub kind: ExtensionKind,
    pub description: String,
    pub enabled: bool,
}

/// Decision returned by a governance extension prior to executing SQL on a connection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PreExecuteDecision {
    Allow,
    RequireApproval { reason: String },
    Block { reason: String },
}

/// Execution telemetry emitted to registered audit/governance extensions after query completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEvent {
    pub connection_id: String,
    pub connection_name: String,
    pub database: Option<String>,
    pub sql: String,
    pub duration_ms: u64,
    pub rows: usize,
    pub affected_rows: Option<u64>,
    pub success: bool,
    pub error_message: Option<String>,
}

/// Hook point for external governance platforms (e.g. `wmsir/Pacer` approval/audit integration).
#[async_trait]
pub trait GovernanceExtension: Send + Sync {
    fn meta(&self) -> ExtensionMeta;

    async fn before_execute(
        &self,
        profile: &ConnectionProfile,
        db: Option<&str>,
        sql: &str,
    ) -> CoreResult<PreExecuteDecision>;

    async fn report_audit(&self, event: &AuditEvent) -> CoreResult<()>;
}

/// Hook point for optional AI SQL assistants (Text2SQL, query optimization, risk analysis).
#[async_trait]
pub trait AiSqlExtension: Send + Sync {
    fn meta(&self) -> ExtensionMeta;

    async fn text_to_sql(&self, prompt: &str, engine: &DbEngineType) -> CoreResult<String>;

    async fn optimize_sql(&self, sql: &str, engine: &DbEngineType) -> CoreResult<String>;
}

/// Runtime registry of optional IDE extensions (empty by default in standalone DataGrip mode).
#[derive(Default, Clone)]
pub struct ExtensionRegistry {
    governance: Vec<Arc<dyn GovernanceExtension>>,
    ai: Vec<Arc<dyn AiSqlExtension>>,
}

impl ExtensionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_governance(&mut self, ext: Arc<dyn GovernanceExtension>) {
        self.governance.push(ext);
    }

    pub fn register_ai(&mut self, ext: Arc<dyn AiSqlExtension>) {
        self.ai.push(ext);
    }

    pub fn list_extensions(&self) -> Vec<ExtensionMeta> {
        let mut out: Vec<ExtensionMeta> = self.governance.iter().map(|g| g.meta()).collect();
        out.extend(self.ai.iter().map(|a| a.meta()));
        out
    }

    pub async fn check_before_execute(
        &self,
        profile: &ConnectionProfile,
        db: Option<&str>,
        sql: &str,
    ) -> CoreResult<PreExecuteDecision> {
        for ext in &self.governance {
            let decision = ext.before_execute(profile, db, sql).await?;
            if !matches!(decision, PreExecuteDecision::Allow) {
                return Ok(decision);
            }
        }
        Ok(PreExecuteDecision::Allow)
    }

    pub async fn emit_audit(&self, event: &AuditEvent) {
        for ext in &self.governance {
            let _ = ext.report_audit(event).await;
        }
    }

    pub fn governance_extensions(&self) -> &[Arc<dyn GovernanceExtension>] {
        &self.governance
    }

    pub fn ai_extensions(&self) -> &[Arc<dyn AiSqlExtension>] {
        &self.ai
    }
}
