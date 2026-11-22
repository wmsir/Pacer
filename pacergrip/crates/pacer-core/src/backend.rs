use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::error::CoreResult;
use crate::model::{
    CapabilityFlags, ColumnMeta, DatabaseMeta, ExplainResult, IndexMeta, ObjectSearchHit,
    PingMetrics, QueryChunk, QueryReq, QuerySummary, TableMeta,
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

    async fn search_objects(&self, _db: &str, _query: &str) -> CoreResult<Vec<ObjectSearchHit>> {
        Ok(Vec::new())
    }

    async fn get_table_ddl(&self, db: &str, table: &str) -> CoreResult<String>;

    async fn execute_query(
        &self,
        req: QueryReq,
        tx: mpsc::Sender<QueryChunk>,
    ) -> CoreResult<QuerySummary>;

    async fn explain(&self, db: &str, sql: &str) -> CoreResult<ExplainResult>;

    fn capabilities(&self) -> CapabilityFlags;
}
