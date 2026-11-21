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
  host: string;
  port: number;
  username: string;
  authType: string;
  keyFile?: string | null;
  enabled: boolean;
}

export interface SslConfig {
  enabled: boolean;
  caCert?: string | null;
  clientCert?: string | null;
  clientKey?: string | null;
  mode?: string | null;
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
  ssh?: SshTunnelConfig | null;
  schemaPattern?: string | null;
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

export async function listIndexes(
  profileId: string,
  dbName: string,
  tableName: string,
): Promise<IndexMeta[]> {
  assertTauri();
  return invoke('list_indexes_cmd', { profileId, dbName, tableName });
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

export async function listQueryHistory(): Promise<QueryHistoryEntry[]> {
  assertTauri();
  return invoke('list_query_history_cmd');
}

export async function listExtensions(): Promise<ExtensionMeta[]> {
  if (!isTauriRuntime()) return [];
  return invoke('list_extensions_cmd');
}
