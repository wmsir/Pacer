use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::error::CoreResult;
use crate::model::{
    CapabilityFlags, ColumnMeta, DatabaseMeta, ExplainResult, ForeignKeyMeta, IndexMeta,
    ObjectSearchHit, PingMetrics, QueryChunk, QueryReq, QuerySummary, RoutineMeta, TableMeta,
    TransactionStatus,
};

/// Unified database backend abstraction hiding direct socket drivers vs. Pacer governance proxy.
#[async_trait]
pub trait DbBackend: Send + Sync {
    async fn test_connection(&self) -> CoreResult<PingMetrics>;

    async fn list_databases(&self) -> CoreResult<Vec<DatabaseMeta>>;

    async fn list_tables(&self, db: &str) -> CoreResult<Vec<TableMeta>>;

    async fn list_columns(&self, db: &str, table: &str) -> CoreResult<Vec<ColumnMeta>>;

    async fn list_indexes(&self, _db: &str, _table: &str) -> CoreResult<Vec<IndexMeta>> {
        Ok(Vec::new())
    }

    async fn list_foreign_keys(
        &self,
        _db: &str,
        _table: &str,
    ) -> CoreResult<Vec<ForeignKeyMeta>> {
        Ok(Vec::new())
    }

    async fn list_routines(&self, _db: &str) -> CoreResult<Vec<RoutineMeta>> {
        Ok(Vec::new())
    }

    async fn search_objects(&self, _db: &str, _query: &str) -> CoreResult<Vec<ObjectSearchHit>> {
        Ok(Vec::new())
    }

    async fn get_table_ddl(&self, db: &str, table: &str) -> CoreResult<String>;

    async fn execute_query(
        &self,
        req: QueryReq,
        tx: mpsc::Sender<QueryChunk>,
    ) -> CoreResult<QuerySummary>;

    async fn cancel_running_query(&self, _query_id: &str) -> CoreResult<bool> {
        Ok(false)
    }

    async fn explain(&self, db: &str, sql: &str) -> CoreResult<ExplainResult>;

    async fn transaction_status(&self) -> CoreResult<TransactionStatus> {
        Ok(TransactionStatus::default())
    }

    async fn set_manual_transaction(&self, _enabled: bool) -> CoreResult<TransactionStatus> {
        Ok(TransactionStatus::default())
    }

    async fn commit_transaction(&self) -> CoreResult<TransactionStatus> {
        Ok(TransactionStatus::default())
    }

    async fn rollback_transaction(&self) -> CoreResult<TransactionStatus> {
        Ok(TransactionStatus::default())
    }

    fn capabilities(&self) -> CapabilityFlags;
}
