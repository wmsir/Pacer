use std::collections::BTreeMap;

use async_trait::async_trait;
use tokio::sync::mpsc;

use pacer_core::{
    CapabilityFlags, ColumnMeta, CoreError, CoreResult, DatabaseMeta, DbBackend, ExplainResult,
    PingMetrics, QueryChunk, QueryReq, QuerySummary, TableMeta,
};

use crate::client::PacerClient;
use crate::types::{parse_pacer_string_list, PacerInstance, SqlQueryRequest};

const STREAM_CHUNK_SIZE: usize = 500;

/// `DbBackend` implementation routing all metadata and read-only queries through `wmsir/Pacer`.
/// Keeps production DB credentials completely off the desktop client.
pub struct PacerProxyBackend {
    client: PacerClient,
    instance: PacerInstance,
    capabilities: CapabilityFlags,
}

impl PacerProxyBackend {
    pub fn new(client: PacerClient, instance: PacerInstance) -> Self {
        let ops = parse_pacer_string_list(instance.supported_ops.as_deref());
        let capabilities = CapabilityFlags::from_pacer_ops(&ops);
        Self {
            client,
            instance,
            capabilities,
        }
    }

    pub fn instance(&self) -> &PacerInstance {
        &self.instance
    }
}

#[async_trait]
impl DbBackend for PacerProxyBackend {
    async fn test_connection(&self) -> CoreResult<PingMetrics> {
        let res = self.client.test_connection(self.instance.id).await?;
        let version = match (&res.database_product_name, &res.database_product_version) {
            (Some(name), Some(ver)) => Some(format!("{name} {ver}")),
            (Some(name), None) => Some(name.clone()),
            _ => res.database_product_version,
        };
        Ok(PingMetrics {
            success: res.success,
            latency_ms: res.latency_ms,
            server_version: version,
            message: res
                .message
                .or(res.error_message)
                .unwrap_or_else(|| "ok".to_string()),
        })
    }

    async fn list_databases(&self) -> CoreResult<Vec<DatabaseMeta>> {
        match self.client.list_databases_detail(self.instance.id).await {
            Ok(details) => Ok(details
                .into_iter()
                .map(|d| DatabaseMeta {
                    name: d.db_name,
                    charset: d.charset,
                    collation: d.collation,
                    table_count: Some(d.table_count),
                    data_size_mb: Some(d.data_size_mb),
                    is_system: d.is_system,
                    comment: d.comment,
                })
                .collect()),
            Err(_) => {
                let names = self.client.list_databases(self.instance.id).await?;
                Ok(names
                    .into_iter()
                    .map(|name| DatabaseMeta {
                        name,
                        charset: None,
                        collation: None,
                        table_count: None,
                        data_size_mb: None,
                        is_system: false,
                        comment: None,
                    })
                    .collect())
            }
        }
    }

    async fn list_tables(&self, db: &str) -> CoreResult<Vec<TableMeta>> {
        let raw_tables = self.client.list_tables(self.instance.id, db).await?;
        Ok(raw_tables
            .into_iter()
            .filter_map(|m| {
                let name = json_str(&m, &["tableName", "TABLE_NAME", "name"])?;
                Some(TableMeta {
                    name,
                    table_type: json_str(&m, &["tableType", "TABLE_TYPE"]),
                    engine: json_str(&m, &["engine", "ENGINE"]),
                    estimated_rows: json_u64(&m, &["tableRows", "TABLE_ROWS"]),
                    data_size_kb: json_u64(&m, &["dataLength", "DATA_LENGTH"])
                        .map(|bytes| (bytes as f64) / 1024.0),
                    comment: json_str(&m, &["tableComment", "TABLE_COMMENT", "comment"])
                        .filter(|s| !s.is_empty()),
                })
            })
            .collect())
    }

    async fn list_columns(&self, db: &str, table: &str) -> CoreResult<Vec<ColumnMeta>> {
        let raw_cols = self
            .client
            .list_columns(self.instance.id, db, table)
            .await?;
        let masking_rules = self
            .client
            .masking_rules(Some(self.instance.id), Some(db), Some(table))
            .await
            .unwrap_or_default();

        let mut rule_by_col = BTreeMap::new();
        for rule in masking_rules {
            if rule.status.unwrap_or(1) == 1 {
                if let (Some(col), Some(rtype)) = (rule.column_name, rule.rule_type) {
                    rule_by_col.insert(col.to_ascii_lowercase(), rtype);
                }
            }
        }

        Ok(raw_cols
            .into_iter()
            .filter_map(|m| {
                let name = json_str(&m, &["columnName", "COLUMN_NAME", "name"])?;
                let data_type = json_str(&m, &["columnType", "COLUMN_TYPE", "dataType"])
                    .unwrap_or_else(|| "unknown".to_string());
                let nullable = json_str(&m, &["isNullable", "IS_NULLABLE"])
                    .map(|v| !v.eq_ignore_ascii_case("NO") && !v.eq_ignore_ascii_case("false"))
                    .unwrap_or(true);
                let is_primary_key = json_str(&m, &["columnKey", "COLUMN_KEY"])
                    .is_some_and(|k| k.eq_ignore_ascii_case("PRI"));
                let default_value = json_str(&m, &["columnDefault", "COLUMN_DEFAULT"]);
                let comment = json_str(&m, &["columnComment", "COLUMN_COMMENT"])
                    .filter(|s| !s.is_empty());
                let masking_rule = rule_by_col.get(&name.to_ascii_lowercase()).cloned();

                Some(ColumnMeta {
                    name,
                    data_type,
                    nullable,
                    is_primary_key,
                    default_value,
                    comment,
                    masking_rule,
                })
            })
            .collect())
    }

    async fn get_table_ddl(&self, _db: &str, _table: &str) -> CoreResult<String> {
        Err(CoreError::Unsupported(
            "Pacer backend DDL endpoint not enabled yet".to_string(),
        ))
    }

    async fn execute_query(
        &self,
        req: QueryReq,
        tx: mpsc::Sender<QueryChunk>,
    ) -> CoreResult<QuerySummary> {
        let payload = SqlQueryRequest {
            instance_id: self.instance.id,
            db_name: req.database.clone(),
            sql: req.sql,
            limit: req.limit,
        };
        let res = self.client.execute_query(&payload).await?;

        let columns = res.columns;
        let masked_columns = res.masked_columns;
        let total_rows = res.rows.len();

        if res.rows.is_empty() {
            let _ = tx
                .send(QueryChunk {
                    columns: columns.clone(),
                    rows: Vec::new(),
                    batch_index: 0,
                    is_last: true,
                    masked_columns: masked_columns.clone(),
                })
                .await;
        } else {
            let total_batches = total_rows.div_ceil(STREAM_CHUNK_SIZE);
            for (batch_index, slice) in res.rows.chunks(STREAM_CHUNK_SIZE).enumerate() {
                let is_last = batch_index + 1 == total_batches;
                let chunk = QueryChunk {
                    columns: columns.clone(),
                    rows: slice.to_vec(),
                    batch_index,
                    is_last,
                    masked_columns: masked_columns.clone(),
                };
                if tx.send(chunk).await.is_err() {
                    break;
                }
            }
        }

        Ok(QuerySummary {
            success: res.success,
            database: res
                .database_name
                .or(req.database)
                .unwrap_or_else(|| "default".to_string()),
            total_rows: res.total_rows.max(total_rows),
            affected_rows: None,
            is_dml_or_ddl: false,
            duration_ms: res.duration_ms,
            route_node: res.route_node,
            route_description: res.route_description,
            masked_columns,
            error_message: res.error_message,
        })
    }

    async fn explain(&self, db: &str, sql: &str) -> CoreResult<ExplainResult> {
        let payload = SqlQueryRequest {
            instance_id: self.instance.id,
            db_name: Some(db.to_string()),
            sql: sql.to_string(),
            limit: Some(100),
        };
        let res = self.client.execute_explain(&payload).await?;
        Ok(ExplainResult {
            success: res.success,
            columns: res.columns,
            rows: res.rows,
            duration_ms: res.duration_ms,
            error_message: res.error_message,
        })
    }

    fn capabilities(&self) -> CapabilityFlags {
        self.capabilities.clone()
    }
}

fn json_str(map: &BTreeMap<String, serde_json::Value>, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(val) = map.get(*key) {
            if let Some(s) = val.as_str() {
                return Some(s.to_string());
            }
            if !val.is_null() {
                return Some(val.to_string());
            }
        }
    }
    None
}

fn json_u64(map: &BTreeMap<String, serde_json::Value>, keys: &[&str]) -> Option<u64> {
    for key in keys {
        if let Some(val) = map.get(*key) {
            if let Some(n) = val.as_u64() {
                return Some(n);
            }
            if let Some(s) = val.as_str() {
                if let Ok(n) = s.parse::<u64>() {
                    return Some(n);
                }
            }
        }
    }
    None
}
