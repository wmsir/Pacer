import { Channel, invoke } from '@tauri-apps/api/core';

export type EnvLevel = 'DEV' | 'TEST' | 'PROD' | 'UNKNOWN';
export type SqlAccessKind = 'READ_ONLY' | 'WRITE_DML' | 'WRITE_DDL' | 'ADMIN_OR_DCL' | 'UNKNOWN';
export type RiskLevel = 'INFO' | 'MEDIUM' | 'HIGH' | 'CRITICAL';

export interface CapabilityFlags {
  canQuery: boolean;
  canDml: boolean;
  canDdl: boolean;
  canExport: boolean;
  canDryRun: boolean;
  hasMasking: boolean;
  canRollback: boolean;
  canRelease: boolean;
}

export interface DirectConfig {
  dbType: string | Record<string, string>;
  host?: string | null;
  port?: number | null;
  database?: string | null;
  jdbcUrl: string;
  driverRef?: string | null;
  driverClass?: string | null;
  username?: string | null;
  readOnly: boolean;
  ssh?: {
    enabled: boolean;
    host: string;
    port: number;
    username: string;
  } | null;
}

export interface ConnectionFingerprint {
  dbType: string | Record<string, string>;
  host?: string | null;
  port?: number | null;
  database?: string | null;
}

export interface ConnectionProfile {
  id: string;
  name: string;
  sourceUuid?: string | null;
  dbType: string | Record<string, string>;
  env: EnvLevel;
  color?: string | null;
  comment?: string | null;
  tags: string[];
  resourceGroups: string[];
  capabilities: CapabilityFlags;
  mode: {
    kind: 'direct' | 'hybrid' | 'pacerManaged';
    config: DirectConfig | Record<string, unknown>;
  };
  fingerprint: ConnectionFingerprint;
}

export interface ConnectionDraft {
  id?: string | null;
  name: string;
  dbType: string | Record<string, string>;
  host?: string | null;
  port?: number | null;
  database?: string | null;
  jdbcUrl?: string | null;
  username?: string | null;
  env?: EnvLevel;
  readOnly?: boolean;
  comment?: string | null;
}

export interface PingMetrics {
  success: boolean;
  latencyMs: number;
  serverVersion?: string | null;
  message: string;
}

export interface DatabaseMeta {
  name: string;
  charset?: string | null;
  collation?: string | null;
  tableCount?: number | null;
  dataSizeMb?: number | null;
  isSystem: boolean;
  comment?: string | null;
}

export interface TableMeta {
  name: string;
  tableType?: string | null;
  engine?: string | null;
  estimatedRows?: number | null;
  dataSizeKb?: number | null;
  comment?: string | null;
}

export interface ColumnMeta {
  name: string;
  dataType: string;
  nullable: boolean;
  isPrimaryKey: boolean;
  defaultValue?: string | null;
  comment?: string | null;
  maskingRule?: string | null;
}

export interface QueryChunk {
  columns: string[];
  rows: Record<string, unknown>[];
  batchIndex: number;
  isLast: boolean;
  maskedColumns: Record<string, string>;
}

export interface QuerySummary {
  success: boolean;
  database: string;
  totalRows: number;
  affectedRows?: number | null;
  isDmlOrDdl: boolean;
  durationMs: number;
  routeNode?: string | null;
  routeDescription?: string | null;
  maskedColumns: Record<string, string>;
  errorMessage?: string | null;
}

export interface ExplainResult {
  success: boolean;
  columns: string[];
  rows: Record<string, unknown>[];
  durationMs: number;
  errorMessage?: string | null;
}

export interface ScriptAnalysis {
  overallKind: SqlAccessKind;
  requiresTicket: boolean;
  totalExpectedRowsHint?: number | null;
  statements: Array<{
    index: number;
    sql: string;
    accessKind: SqlAccessKind;
    statementType: string;
    expectedRowsHint?: number | null;
    warnings: Array<{
      code: string;
      level: RiskLevel;
      message: string;
      statementIndex: number;
    }>;
  }>;
}

export interface QueryHistoryEntry {
  id: string;
  timestamp: string;
  profileId: string;
  profileName: string;
  database?: string | null;
  sql: string;
  durationMs: number;
  rowCount: number;
  affectedRows?: number | null;
  success: boolean;
  errorMessage?: string | null;
}

export interface ExtensionMeta {
  id: string;
  name: string;
  version: string;
  kind: 'GOVERNANCE' | 'AI_ASSISTANT' | 'DRIVER' | 'AUTH';
  description: string;
  enabled: boolean;
}

export function isTauriRuntime(): boolean {
  return (
    typeof window !== 'undefined' &&
    Boolean((window as unknown as Record<string, unknown>).__TAURI_INTERNALS__)
  );
}

function assertTauri(): void {
  if (!isTauriRuntime()) {
    throw new Error('当前运行在纯浏览器视图，请运行 `cargo tauri dev` 启动桌面端以连接原生 Rust 数据库驱动。');
  }
}

export async function listConnections(): Promise<ConnectionProfile[]> {
  assertTauri();
  return invoke('list_connections');
}

export async function saveConnection(
  draft: ConnectionDraft,
  password?: string | null,
): Promise<ConnectionProfile> {
  assertTauri();
  return invoke('save_connection', { draft, password: password ?? null });
}

export async function deleteConnection(profileId: string): Promise<void> {
  assertTauri();
  return invoke('delete_connection', { profileId });
}

export async function testConnection(
  profileId: string,
  password?: string | null,
): Promise<PingMetrics> {
  assertTauri();
  return invoke('test_connection_cmd', { profileId, password: password ?? null });
}

export async function createDemoSqliteConnection(): Promise<ConnectionProfile> {
  assertTauri();
  return invoke('create_demo_sqlite_connection');
}

export async function importDataGripXml(xml: string): Promise<ConnectionProfile[]> {
  assertTauri();
  return invoke('import_datagrip_xml_cmd', { xml });
}

export async function importDataGripPath(path: string): Promise<ConnectionProfile[]> {
  assertTauri();
  return invoke('import_datagrip_path_cmd', { path });
}

export async function exportDataGripXml(profileIds?: string[]): Promise<string> {
  assertTauri();
  return invoke('export_datagrip_xml_cmd', { profileIds: profileIds ?? null });
}

export async function listDatabases(
  profileId: string,
  password?: string | null,
): Promise<DatabaseMeta[]> {
  assertTauri();
  return invoke('list_databases_cmd', { profileId, password: password ?? null });
}

export async function listTables(profileId: string, dbName: string): Promise<TableMeta[]> {
  assertTauri();
  return invoke('list_tables_cmd', { profileId, dbName });
}

export async function listColumns(
  profileId: string,
  dbName: string,
  tableName: string,
): Promise<ColumnMeta[]> {
  assertTauri();
  return invoke('list_columns_cmd', { profileId, dbName, tableName });
}

export async function getTableDdl(
  profileId: string,
  dbName: string,
  tableName: string,
): Promise<string> {
  assertTauri();
  return invoke('get_table_ddl_cmd', { profileId, dbName, tableName });
}

export async function executeSql(
  profileId: string,
  dbName: string | null,
  sql: string,
  limit: number | null,
  onData: (chunk: QueryChunk) => void,
): Promise<QuerySummary> {
  assertTauri();
  const onChunk = new Channel<QueryChunk>();
  onChunk.onmessage = onData;
  return invoke('execute_sql_cmd', {
    profileId,
    dbName,
    sql,
    limit,
    onChunk,
  });
}

export async function explainSql(
  profileId: string,
  dbName: string,
  sql: string,
): Promise<ExplainResult> {
  assertTauri();
  return invoke('explain_sql_cmd', { profileId, dbName, sql });
}

export async function analyzeSql(
  sql: string,
  dbType: string | Record<string, string>,
): Promise<ScriptAnalysis> {
  if (!isTauriRuntime()) {
    return {
      overallKind: 'READ_ONLY',
      requiresTicket: false,
      statements: [],
      totalExpectedRowsHint: null,
    };
  }
  return invoke('analyze_sql_cmd', { sql, dbType });
}

export async function listQueryHistory(): Promise<QueryHistoryEntry[]> {
  assertTauri();
  return invoke('list_query_history_cmd');
}

export async function listExtensions(): Promise<ExtensionMeta[]> {
  if (!isTauriRuntime()) return [];
  return invoke('list_extensions_cmd');
}
