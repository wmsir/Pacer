import { Channel, invoke } from '@tauri-apps/api/core';
import { open as dialogOpen, save as dialogSave } from '@tauri-apps/plugin-dialog';

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

export interface SshTunnelConfig {
  id?: string | null;
  configName?: string | null;
  host: string;
  port: number;
  username: string;
  authType: string;
  keyFile?: string | null;
  localPort?: number | null;
  enabled: boolean;
  password?: string | null;
  passphrase?: string | null;
}

export interface SslConfig {
  enabled: boolean;
  caCert?: string | null;
  clientCert?: string | null;
  clientKey?: string | null;
  mode?: string | null;
  useIdeTruststore?: boolean;
  useSystemTruststore?: boolean;
  clientKeyPasswordSaveMode?: string | null;
}

export interface ConnectionOptionsConfig {
  transactionControl?: string | null;
  switchSchema?: string | null;
  timeZone?: string | null;
  singleSessionMode?: boolean;
  keepAliveEnabled?: boolean;
  keepAliveSec?: number | null;
  autoDisconnectEnabled?: boolean;
  autoDisconnectSec?: number | null;
  startupScript?: string | null;
  autoSync?: boolean;
  introspectionLevel?: string | null;
  queryTimeoutSec?: number | null;
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
  driverProperties?: Record<string, string>;
  readOnly: boolean;
  ssh?: SshTunnelConfig | null;
  ssl?: SslConfig | null;
  schemaPattern?: string | null;
  objectFilter?: string | null;
  options?: ConnectionOptionsConfig | null;
  connectionType?: 'default' | 'socket' | 'urlOnly' | string | null;
  authType?: 'userPassword' | 'noAuth' | string | null;
  passwordSaveMode?: 'forever' | 'session' | 'never' | string | null;
  socketPath?: string | null;
  hasSavedPassword?: boolean;
}

export interface ConnectionFingerprint {
  dbType: string | Record<string, string>;
  host?: string | null;
  port?: number | null;
  database?: string | null;
}

export interface WorkspaceProject {
  id: string;
  name: string;
  description?: string | null;
  color?: string | null;
  rootDir?: string | null;
  createdAt: string;
}

export interface ProjectDraft {
  id?: string | null;
  name: string;
  description?: string | null;
  color?: string | null;
  rootDir?: string | null;
}

export interface ConnectionProfile {
  id: string;
  name: string;
  projectId?: string | null;
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
  projectId?: string | null;
  name: string;
  dbType: string | Record<string, string>;
  host?: string | null;
  port?: number | null;
  database?: string | null;
  jdbcUrl?: string | null;
  username?: string | null;
  env?: EnvLevel;
  color?: string | null;
  readOnly?: boolean;
  comment?: string | null;
  connectionType?: 'default' | 'socket' | 'urlOnly' | string | null;
  authType?: 'userPassword' | 'noAuth' | string | null;
  passwordSaveMode?: 'forever' | 'session' | 'never' | string | null;
  socketPath?: string | null;
  driverProperties?: Record<string, string>;
  sslMode?: string | null;
  ssl?: SslConfig | null;
  ssh?: SshTunnelConfig | null;
  schemaPattern?: string | null;
  objectFilter?: string | null;
  options?: ConnectionOptionsConfig | null;
}

export interface DbeaverDiscoveryItem {
  path: string;
  workspaceName: string;
  format: string;
  connectionCount: number;
  hasCredentialsFile: boolean;
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

export interface IndexMeta {
  name: string;
  unique: boolean;
  columns: string[];
  indexType?: string | null;
}

export interface ForeignKeyMeta {
  name: string;
  columnName: string;
  referencedTable: string;
  referencedColumn: string;
  onDelete?: string | null;
  onUpdate?: string | null;
}

export interface RoutineMeta {
  name: string;
  kind: string;
  returnType?: string | null;
  definition?: string | null;
  comment?: string | null;
}

export interface TransactionStatus {
  supported: boolean;
  manualMode: boolean;
  inTransaction: boolean;
  pendingStatements: number;
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
  projectId?: string | null;
  profileId: string;
  profileName: string;
  database?: string | null;
  sql: string;
  durationMs: number;
  rowCount: number;
  affectedRows?: number | null;
  success: boolean;
  errorMessage?: string | null;
  pinned?: boolean;
}

export interface ExportResultDto {
  path: string;
  count: number;
  includedPasswords: boolean;
  files: string[];
}

export interface ExtensionMeta {
  id: string;
  name: string;
  version: string;
  kind: 'GOVERNANCE' | 'AI_ASSISTANT' | 'DRIVER' | 'AUTH';
  description: string;
  enabled: boolean;
}

export interface MarkdownSqlBlock {
  index: number;
  language: string;
  startLine: number;
  endLine: number;
  contentStartLine: number;
  contentEndLine: number;
  sql: string;
  title?: string | null;
}

export interface OpenedFileDto {
  path: string;
  name: string;
  content: string;
  encoding: string;
  hasBom: boolean;
  lineEnding: string;
  lineCount: number;
  charCount: number;
  byteSize: number;
  fileKind: 'markdown' | 'sql' | 'json' | 'xml' | 'text' | string;
  sqlBlocks: MarkdownSqlBlock[];
}

export interface FileStatsDto {
  path: string;
  encoding: string;
  hasBom: boolean;
  lineEnding: string;
  lineCount: number;
  charCount: number;
  byteSize: number;
}

export interface WorkspaceFileEntry {
  path: string;
  name: string;
  isDir: boolean;
  size: number;
  extension?: string | null;
}

export function isTauriRuntime(): boolean {
  return (
    typeof window !== 'undefined' &&
    Boolean((window as unknown as Record<string, unknown>).__TAURI_INTERNALS__)
  );
}

function assertTauri(): void {
  if (!isTauriRuntime()) {
    throw new Error('当前运行在纯浏览器视图，请运行桌面端以启用原生 SQL 驱动与文件系统能力。');
  }
}

export async function pickTextFileToOpen(): Promise<string | null> {
  assertTauri();
  const res = await dialogOpen({
    multiple: false,
    directory: false,
    filters: [
      { name: 'SQL & Markdown', extensions: ['md', 'markdown', 'sql', 'ddl', 'dml', 'txt'] },
      { name: 'Markdown', extensions: ['md', 'markdown', 'mdx'] },
      { name: 'SQL 脚本', extensions: ['sql', 'ddl', 'dml', 'psql'] },
      { name: '所有文件', extensions: ['*'] },
    ],
  });
  return typeof res === 'string' ? res : null;
}

export async function pickDirectoryToMount(): Promise<string | null> {
  assertTauri();
  const res = await dialogOpen({
    multiple: false,
    directory: true,
  });
  return typeof res === 'string' ? res : null;
}

export async function pickSaveFilePath(defaultPath?: string): Promise<string | null> {
  assertTauri();
  const res = await dialogSave({
    defaultPath,
    filters: [
      { name: 'SQL / Markdown', extensions: ['sql', 'md', 'txt'] },
      { name: '所有文件', extensions: ['*'] },
    ],
  });
  return typeof res === 'string' ? res : null;
}

export async function pickDbeaverImportFile(): Promise<string | null> {
  assertTauri();
  const res = await dialogOpen({
    multiple: false,
    directory: false,
    filters: [
      { name: 'DBeaver Connections (JSON / DBP / ZIP / XML)', extensions: ['json', 'dbp', 'zip', 'xml'] },
      { name: '所有文件', extensions: ['*'] },
    ],
  });
  return typeof res === 'string' ? res : null;
}

export async function pickDbeaverFolder(): Promise<string | null> {
  assertTauri();
  const res = await dialogOpen({
    multiple: false,
    directory: true,
  });
  return typeof res === 'string' ? res : null;
}

export async function pickSaveDbeaverDbpPath(
  defaultPath = 'pacer-dbeaver-export.dbp',
): Promise<string | null> {
  assertTauri();
  const res = await dialogSave({
    defaultPath,
    filters: [
      { name: 'DBeaver Project Archive (.dbp)', extensions: ['dbp', 'zip'] },
      { name: '所有文件', extensions: ['*'] },
    ],
  });
  return typeof res === 'string' ? res : null;
}

export async function pickSaveXlsxPath(defaultPath = 'export.xlsx'): Promise<string | null> {
  assertTauri();
  const res = await dialogSave({
    defaultPath,
    filters: [
      { name: 'Excel 工作簿 (.xlsx)', extensions: ['xlsx'] },
      { name: '所有文件', extensions: ['*'] },
    ],
  });
  return typeof res === 'string' ? res : null;
}

export async function pickExportFolderPath(): Promise<string | null> {
  assertTauri();
  const res = await dialogOpen({
    multiple: false,
    directory: true,
  });
  return typeof res === 'string' ? res : null;
}

export async function pickCertificateOrKeyFile(
  kind: 'cert' | 'key' = 'cert',
): Promise<string | null> {
  assertTauri();
  const res = await dialogOpen({
    multiple: false,
    directory: false,
    filters: [
      {
        name: kind === 'key' ? 'Private Key / Identity Files' : 'Certificates / PEM Files',
        extensions: ['pem', 'crt', 'cer', 'key', 'p12', 'pfx', 'ppk'],
      },
      { name: '所有文件', extensions: ['*'] },
    ],
  });
  return typeof res === 'string' ? res : null;
}

export async function pickDataImportFile(): Promise<string | null> {
  assertTauri();
  const res = await dialogOpen({
    multiple: false,
    directory: false,
    filters: [
      { name: 'Excel / CSV / TSV / JSON Data', extensions: ['xlsx', 'csv', 'tsv', 'json', 'txt'] },
      { name: '所有文件', extensions: ['*'] },
    ],
  });
  return typeof res === 'string' ? res : null;
}

export async function pickQueryExportPath(
  defaultPath: string,
  format: 'csv' | 'xlsx' | 'json',
): Promise<string | null> {
  assertTauri();
  const filter =
    format === 'xlsx'
      ? { name: 'Excel 工作簿 (.xlsx)', extensions: ['xlsx'] }
      : format === 'csv'
        ? { name: 'CSV 表格 (.csv)', extensions: ['csv'] }
        : { name: 'JSON 数据 (.json)', extensions: ['json'] };
  const res = await dialogSave({
    defaultPath,
    filters: [filter, { name: '所有文件', extensions: ['*'] }],
  });
  return typeof res === 'string' ? res : null;
}

export async function pickSaveSqlDumpPath(defaultPath = 'dump.sql'): Promise<string | null> {
  assertTauri();
  const res = await dialogSave({
    defaultPath,
    filters: [
      { name: 'SQL 转储文件 (.sql)', extensions: ['sql'] },
      { name: '所有文件', extensions: ['*'] },
    ],
  });
  return typeof res === 'string' ? res : null;
}

export async function pickSqlScriptFile(): Promise<string | null> {
  assertTauri();
  const res = await dialogOpen({
    multiple: false,
    directory: false,
    filters: [
      { name: 'SQL 脚本文件 (.sql)', extensions: ['sql', 'txt'] },
      { name: '所有文件', extensions: ['*'] },
    ],
  });
  return typeof res === 'string' ? res : null;
}

export async function pickSaveDocPath(
  defaultPath: string,
  format: 'md' | 'html',
): Promise<string | null> {
  assertTauri();
  const res = await dialogSave({
    defaultPath,
    filters: [
      format === 'html'
        ? { name: 'HTML 数据字典 (.html)', extensions: ['html', 'htm'] }
        : { name: 'Markdown 数据字典 (.md)', extensions: ['md'] },
      { name: '所有文件', extensions: ['*'] },
    ],
  });
  return typeof res === 'string' ? res : null;
}

export async function listProjects(): Promise<WorkspaceProject[]> {
  assertTauri();
  return invoke('list_projects_cmd');
}

export async function saveProject(draft: ProjectDraft): Promise<WorkspaceProject> {
  assertTauri();
  return invoke('save_project_cmd', { draft });
}

export async function deleteProject(projectId: string): Promise<WorkspaceProject[]> {
  assertTauri();
  return invoke('delete_project_cmd', { projectId });
}

export async function assignConnectionProject(
  profileId: string,
  projectId: string,
): Promise<ConnectionProfile> {
  assertTauri();
  return invoke('assign_connection_project_cmd', { profileId, projectId });
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

export async function testConnectionDraft(
  draft: ConnectionDraft,
  password?: string | null,
): Promise<PingMetrics> {
  assertTauri();
  return invoke('test_connection_draft_cmd', { draft, password: password ?? null });
}

export async function createDemoSqliteConnection(
  projectId?: string | null,
): Promise<ConnectionProfile> {
  assertTauri();
  return invoke('create_demo_sqlite_connection', { projectId: projectId ?? null });
}

export async function discoverDbeaver(): Promise<DbeaverDiscoveryItem[]> {
  if (!isTauriRuntime()) return [];
  return invoke('discover_dbeaver_cmd');
}

export async function importDbeaverPath(
  path: string,
  includePasswords = true,
  projectId?: string | null,
): Promise<ConnectionProfile[]> {
  assertTauri();
  return invoke('import_dbeaver_path_cmd', {
    path,
    includePasswords,
    projectId: projectId ?? null,
  });
}

export async function importDbeaverText(
  text: string,
  projectId?: string | null,
): Promise<ConnectionProfile[]> {
  assertTauri();
  return invoke('import_dbeaver_text_cmd', { text, projectId: projectId ?? null });
}

export async function exportDbeaverJson(profileIds?: string[]): Promise<string> {
  assertTauri();
  return invoke('export_dbeaver_json_cmd', { profileIds: profileIds ?? null });
}

export async function exportDbeaverDbp(
  path: string,
  includePasswords = true,
  profileIds?: string[],
): Promise<ExportResultDto> {
  assertTauri();
  return invoke('export_dbeaver_dbp_cmd', {
    path,
    includePasswords,
    profileIds: profileIds ?? null,
  });
}

export async function exportDbeaverFolder(
  dirPath: string,
  includePasswords = true,
  profileIds?: string[],
): Promise<ExportResultDto> {
  assertTauri();
  return invoke('export_dbeaver_folder_cmd', {
    dirPath,
    includePasswords,
    profileIds: profileIds ?? null,
  });
}

export async function exportConnectionsXlsx(
  path: string,
  includePasswords = true,
  profileIds?: string[],
): Promise<ExportResultDto> {
  assertTauri();
  return invoke('export_connections_xlsx_cmd', {
    path,
    includePasswords,
    profileIds: profileIds ?? null,
  });
}

export async function exportRowsXlsx(
  path: string,
  sheetName: string,
  columns: string[],
  rows: Record<string, unknown>[],
): Promise<number> {
  assertTauri();
  return invoke('export_rows_xlsx_cmd', {
    path,
    sheetName,
    columns,
    rows,
  });
}

export async function listDatabases(
  profileId: string,
  password?: string | null,
): Promise<DatabaseMeta[]> {
  assertTauri();
  return invoke('list_databases_cmd', { profileId, password: password ?? null });
}

export async function listDatabasesDraft(
  draft: ConnectionDraft,
  password?: string | null,
): Promise<DatabaseMeta[]> {
  assertTauri();
  return invoke('list_databases_draft_cmd', { draft, password: password ?? null });
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

export async function listIndexes(
  profileId: string,
  dbName: string,
  tableName: string,
): Promise<IndexMeta[]> {
  assertTauri();
  return invoke('list_indexes_cmd', { profileId, dbName, tableName });
}

export async function listForeignKeys(
  profileId: string,
  dbName: string,
  tableName: string,
): Promise<ForeignKeyMeta[]> {
  if (!isTauriRuntime()) return [];
  return invoke('list_foreign_keys_cmd', { profileId, dbName, tableName });
}

export async function listRoutines(
  profileId: string,
  dbName: string,
): Promise<RoutineMeta[]> {
  if (!isTauriRuntime()) return [];
  return invoke('list_routines_cmd', { profileId, dbName });
}

export async function cancelQuery(queryId: string): Promise<boolean> {
  if (!isTauriRuntime()) return false;
  return invoke('cancel_query_cmd', { queryId });
}

export async function disconnectConnection(profileId: string): Promise<boolean> {
  if (!isTauriRuntime()) return false;
  return invoke('disconnect_connection_cmd', { profileId });
}

export async function listActiveConnections(): Promise<string[]> {
  if (!isTauriRuntime()) return [];
  return invoke('list_active_connections_cmd');
}

export async function getTransactionStatus(profileId: string): Promise<TransactionStatus> {
  if (!isTauriRuntime()) {
    return { supported: false, manualMode: false, inTransaction: false, pendingStatements: 0 };
  }
  return invoke('get_transaction_status_cmd', { profileId });
}

export async function setTransactionMode(
  profileId: string,
  manual: boolean,
): Promise<TransactionStatus> {
  assertTauri();
  return invoke('set_transaction_mode_cmd', { profileId, manual });
}

export async function commitTransaction(profileId: string): Promise<TransactionStatus> {
  assertTauri();
  return invoke('commit_transaction_cmd', { profileId });
}

export async function rollbackTransaction(profileId: string): Promise<TransactionStatus> {
  assertTauri();
  return invoke('rollback_transaction_cmd', { profileId });
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
  queryId?: string,
  timeoutSec?: number | null,
): Promise<QuerySummary> {
  assertTauri();
  const onChunk = new Channel<QueryChunk>();
  let sawLastChunk = false;
  let receivedRows = 0;
  let resolveLastChunk: (() => void) | null = null;
  const lastChunkDone = new Promise<void>((resolve) => {
    resolveLastChunk = resolve;
  });

  onChunk.onmessage = (chunk) => {
    receivedRows += chunk.rows?.length ?? 0;
    onData(chunk);
    if (chunk.isLast && !sawLastChunk) {
      sawLastChunk = true;
      resolveLastChunk?.();
    }
  };

  const summary = await invoke<QuerySummary>('execute_sql_cmd', {
    profileId,
    dbName,
    sql,
    limit,
    queryId: queryId ?? null,
    timeoutSec: timeoutSec ?? null,
    onChunk,
  });

  // Guard against WebView2 Channel message delivery arriving slightly after command return on wide tables.
  if (
    !sawLastChunk &&
    summary.success &&
    !summary.isDmlOrDdl &&
    (summary.totalRows > receivedRows || summary.totalRows === 0)
  ) {
    await Promise.race([
      lastChunkDone,
      new Promise<void>((resolve) => {
        setTimeout(resolve, 1500);
      }),
    ]);
  }
  return summary;
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

export async function formatSql(sql: string): Promise<string> {
  if (!isTauriRuntime()) return sql;
  return invoke('format_sql_cmd', { sql });
}

export async function extractStatementAtCursor(
  sql: string,
  cursorOffset: number,
): Promise<string | null> {
  if (!isTauriRuntime()) return sql;
  return invoke('extract_statement_at_cursor_cmd', { sql, cursorOffset });
}

export async function extractMarkdownSqlBlocks(content: string): Promise<MarkdownSqlBlock[]> {
  if (!isTauriRuntime()) return [];
  return invoke('extract_markdown_sql_blocks_cmd', { content });
}

export async function openTextFile(
  path: string,
  encodingOverride?: string | null,
): Promise<OpenedFileDto> {
  assertTauri();
  return invoke('open_text_file_cmd', {
    path,
    encodingOverride: encodingOverride ?? null,
  });
}

export async function saveTextFile(
  path: string,
  content: string,
  encoding: string,
  lineEnding: string,
  withBom: boolean,
): Promise<FileStatsDto> {
  assertTauri();
  return invoke('save_text_file_cmd', {
    path,
    content,
    encoding,
    lineEnding,
    withBom,
  });
}

export async function listWorkspaceFiles(dirPath: string): Promise<WorkspaceFileEntry[]> {
  assertTauri();
  return invoke('list_workspace_files_cmd', { dirPath });
}

export async function createDemoMarkdownFile(): Promise<OpenedFileDto> {
  assertTauri();
  return invoke('create_demo_markdown_file_cmd');
}

export async function revealInExplorer(path: string): Promise<void> {
  assertTauri();
  return invoke('reveal_in_explorer_cmd', { path });
}

export async function listQueryHistory(): Promise<QueryHistoryEntry[]> {
  assertTauri();
  return invoke('list_query_history_cmd');
}

export async function listExtensions(): Promise<ExtensionMeta[]> {
  if (!isTauriRuntime()) return [];
  return invoke('list_extensions_cmd');
}

export interface WorkspaceInfoDto {
  workspaceDir: string;
  consolesDir: string;
  pluginsDir: string;
  isPortable: boolean;
}

export interface PluginSnippetItem {
  title: string;
  category?: string | null;
  sql: string;
}

export interface PluginSqlRule {
  code: string;
  level: string;
  pattern: string;
  message: string;
}

export interface LocalPluginManifest {
  id: string;
  name: string;
  version: string;
  author?: string | null;
  description: string;
  enabled: boolean;
  snippets: PluginSnippetItem[];
  sqlRules: PluginSqlRule[];
  manifestPath: string;
}

export async function getWorkspaceInfo(): Promise<WorkspaceInfoDto | null> {
  if (!isTauriRuntime()) return null;
  return invoke('get_workspace_info_cmd');
}

export async function saveConsoleScratch(
  fileName: string,
  content: string,
): Promise<string | null> {
  if (!isTauriRuntime()) return null;
  return invoke('save_console_scratch_cmd', { fileName, content });
}

export async function listLocalPlugins(): Promise<LocalPluginManifest[]> {
  if (!isTauriRuntime()) return [];
  return invoke('list_local_plugins_cmd');
}

export async function createSamplePlugin(): Promise<LocalPluginManifest[]> {
  if (!isTauriRuntime()) return [];
  return invoke('create_sample_plugin_cmd');
}

export interface ObjectSearchHit {
  kind: string;
  database: string;
  tableName: string;
  columnName?: string | null;
  dataType?: string | null;
  comment?: string | null;
}

export interface SqlDumpResultDto {
  path: string;
  tableCount: number;
  rowCount: number;
  truncatedTables: string[];
}

export async function searchObjects(
  profileId: string,
  dbName: string,
  query: string,
): Promise<ObjectSearchHit[]> {
  assertTauri();
  return invoke('search_objects_cmd', { profileId, dbName, query });
}

export interface SqlScriptProgress {
  executedStatements: number;
  failedStatements: number;
  bytesRead: number;
  totalBytes: number;
  currentStatementPreview: string;
  lastError?: string | null;
  done: boolean;
}

export interface SqlScriptRunResult {
  totalStatements: number;
  succeededStatements: number;
  failedStatements: number;
  durationMs: number;
  cancelled: boolean;
  errors: string[];
}

export interface QueryExportProgress {
  exportedRows: number;
  bytesWritten: number;
  done: boolean;
}

export interface QueryExportResult {
  path: string;
  format: string;
  exportedRows: number;
  durationMs: number;
}

export interface XlsxSheetPreview {
  sheetNames: string[];
  activeSheet: string;
  headers: string[];
  rows: string[][];
  totalRows: number;
  truncated: boolean;
}

export async function toggleHistoryPin(historyId: string): Promise<boolean> {
  assertTauri();
  return invoke('toggle_history_pin_cmd', { historyId });
}

export async function deleteHistoryEntry(historyId: string): Promise<boolean> {
  assertTauri();
  return invoke('delete_history_entry_cmd', { historyId });
}

export async function readXlsxSheet(
  path: string,
  sheetName?: string | null,
  maxRows = 1000,
): Promise<XlsxSheetPreview> {
  assertTauri();
  return invoke('read_xlsx_sheet_cmd', {
    path,
    sheetName: sheetName ?? null,
    maxRows,
  });
}

export async function runSqlFileStream(
  profileId: string,
  dbName: string | null,
  path: string,
  continueOnError: boolean,
  onProgressUpdate: (progress: SqlScriptProgress) => void,
  queryId?: string,
): Promise<SqlScriptRunResult> {
  assertTauri();
  const onProgress = new Channel<SqlScriptProgress>();
  onProgress.onmessage = (p) => onProgressUpdate(p);
  return invoke('run_sql_file_cmd', {
    profileId,
    dbName,
    path,
    continueOnError,
    queryId: queryId ?? null,
    onProgress,
  });
}

export async function exportQueryToFile(req: {
  profileId: string;
  dbName: string | null;
  sql: string;
  format: 'csv' | 'xlsx' | 'json';
  path: string;
  sheetName?: string | null;
  queryId?: string;
  onProgressUpdate?: (progress: QueryExportProgress) => void;
}): Promise<QueryExportResult> {
  assertTauri();
  const onProgress = new Channel<QueryExportProgress>();
  onProgress.onmessage = (p) => req.onProgressUpdate?.(p);
  return invoke('export_query_to_file_cmd', {
    profileId: req.profileId,
    dbName: req.dbName,
    sql: req.sql,
    format: req.format,
    path: req.path,
    sheetName: req.sheetName ?? null,
    queryId: req.queryId ?? null,
    onProgress,
  });
}

export async function exportSqlDump(req: {
  profileId: string;
  dbName: string;
  tableNames?: string[] | null;
  includeData: boolean;
  maxRowsPerTable?: number;
  path: string;
}): Promise<SqlDumpResultDto> {
  assertTauri();
  return invoke('export_sql_dump_cmd', { req });
}
