<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, reactive, ref, watch } from 'vue';
import { useI18n } from '../i18n';
import {
  discoverDbeaver,
  pickDbeaverFolder,
  pickDbeaverImportFile,
  pickDirectoryToMount,
  testConnectionDraft,
  type ConnectionDraft,
  type ConnectionProfile,
  type DbeaverDiscoveryItem,
  type DirectConfig,
  type EnvLevel,
  type ProjectDraft,
  type RiskLevel,
  type WorkspaceProject,
} from '../api/ipc';
import type { GridMutationPreviewPayload } from './ResultGrid.vue';
import IdeIcon from './IdeIcon.vue';
import { formatBytes } from '../utils/sqlTools';
import {
  SHORTCUT_ACTIONS,
  SHORTCUT_CATEGORY_LABELS,
  eventToCombo,
  formatComboDisplay,
  useKeymap,
  type ShortcutActionDef,
  type ShortcutActionId,
  type ShortcutCategory,
} from '../utils/keymap';

export type ActiveModalKind =
  | 'none'
  | 'connection'
  | 'project'
  | 'grid-sql'
  | 'import'
  | 'encoding'
  | 'danger'
  | 'search'
  | 'password'
  | 'shortcuts'
  | 'unsaved';

export interface EncodingModalContext {
  title: string;
  path: string | null;
  encoding: string;
  hasBom: boolean;
  lineEnding: 'CRLF' | 'LF';
  lineCount: number;
  byteSize: number;
}

export interface DangerModalContext {
  profileName: string;
  env: EnvLevel;
  dbName: string;
  sql: string;
  warnings: Array<{
    code: string;
    level: RiskLevel;
    message: string;
  }>;
}

export interface SearchEverywhereItem {
  id: string;
  category: 'action' | 'connection' | 'table' | 'tab' | 'md-block';
  label: string;
  detail: string;
  badge: string;
  payload?: unknown;
}

interface SavedConnTemplate {
  name: string;
  dbType: string;
  connectionType: 'default' | 'socket' | 'urlOnly';
  host: string;
  port: string;
  database: string;
  username: string;
  jdbcUrl: string;
  sslMode: string;
}

const props = defineProps<{
  activeModal: ActiveModalKind;
  projects: WorkspaceProject[];
  activeProjectId: string;
  editingProject: WorkspaceProject | null;
  editingProfile: ConnectionProfile | null;
  gridSqlContext: GridMutationPreviewPayload | null;
  encodingContext: EncodingModalContext | null;
  dangerContext: DangerModalContext | null;
  passwordProfile: ConnectionProfile | null;
  unsavedContext?: { title: string; path: string | null } | null;
  searchItems: SearchEverywhereItem[];
  exportedJson: string;
  busy: boolean;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
  (e: 'save-project', draft: ProjectDraft): void;
  (e: 'delete-project', projectId: string): void;
  (e: 'execute-grid-sql', payload: GridMutationPreviewPayload, editedSql: string): void;
  (e: 'save-connection', draft: ConnectionDraft, password: string | null, closeAfter: boolean): void;
  (e: 'import-dbeaver-path', path: string, includePasswords: boolean): void;
  (e: 'import-dbeaver-text', text: string): void;
  (e: 'export-dbeaver-json'): void;
  (e: 'reload-encoding', encoding: string): void;
  (e: 'convert-encoding', encoding: string, hasBom: boolean, lineEnding: 'CRLF' | 'LF'): void;
  (e: 'confirm-danger'): void;
  (e: 'submit-password', profileId: string, password: string): void;
  (e: 'select-search-item', item: SearchEverywhereItem): void;
  (e: 'unsaved-save-close'): void;
  (e: 'unsaved-discard-close'): void;
  (e: 'notify', message: string): void;
}>();

const { t, locale } = useI18n();

const ENGINE_OPTIONS = [
  { value: 'tiDb', label: 'TiDB', badge: 'TiDB', defaultPort: 4000, scheme: 'jdbc:mysql' },
  { value: 'mySql', label: 'MySQL', badge: 'MySQL', defaultPort: 3306, scheme: 'jdbc:mysql' },
  { value: 'mariaDb', label: 'MariaDB', badge: 'MariaDB', defaultPort: 3306, scheme: 'jdbc:mariadb' },
  { value: 'postgreSql', label: 'PostgreSQL', badge: 'PG', defaultPort: 5432, scheme: 'jdbc:postgresql' },
  { value: 'oceanBase', label: 'OceanBase (MySQL 模式)', badge: 'OB', defaultPort: 2881, scheme: 'jdbc:oceanbase' },
  { value: 'sqlite', label: 'SQLite (本地嵌入式)', badge: 'SQLite', defaultPort: null, scheme: 'jdbc:sqlite' },
  { value: 'kingbase', label: '人大金仓 KingbaseES', badge: 'KB', defaultPort: 54321, scheme: 'jdbc:kingbase8' },
  { value: 'openGauss', label: 'openGauss', badge: 'OG', defaultPort: 5432, scheme: 'jdbc:opengauss' },
  { value: 'oracle', label: 'Oracle (元数据配置)', badge: 'ORA', defaultPort: 1521, scheme: 'jdbc:oracle:thin:@/' },
  { value: 'dameng', label: '达梦 Dameng DM8 (元数据配置)', badge: 'DM8', defaultPort: 5236, scheme: 'jdbc:dm' },
  { value: 'sqlServer', label: 'SQL Server (元数据配置)', badge: 'MSSQL', defaultPort: 1433, scheme: 'jdbc:sqlserver' },
  { value: 'clickHouse', label: 'ClickHouse (元数据配置)', badge: 'CH', defaultPort: 8123, scheme: 'jdbc:clickhouse' },
];

const COLOR_OPTIONS = [
  { value: '', label: '无颜色 (No color)', env: null as EnvLevel | null },
  { value: '#22C55E', label: '绿色 · DEV 开发', env: 'DEV' as EnvLevel },
  { value: '#F59E0B', label: '琥珀色 · UAT / TEST', env: 'TEST' as EnvLevel },
  { value: '#EF4444', label: '红色 · PROD 生产', env: 'PROD' as EnvLevel },
  { value: '#3B82F6', label: '蓝色 · 默认', env: null as EnvLevel | null },
  { value: '#A855F7', label: '紫色 · 备库/只读', env: null as EnvLevel | null },
];

const ENCODING_OPTIONS = [
  { value: 'UTF-8', label: 'UTF-8（推荐 · 无 BOM）', hasBom: false },
  { value: 'UTF-8 BOM', label: 'UTF-8 with BOM（兼容 Excel / Windows 记事本）', hasBom: true },
  { value: 'GBK', label: 'GBK / GB18030（中文旧版 SQL / Windows 经典编码）', hasBom: false },
  { value: 'UTF-16 LE', label: 'UTF-16 LE（Windows Unicode 导出常见）', hasBom: true },
  { value: 'UTF-16 BE', label: 'UTF-16 BE', hasBom: true },
  { value: 'Windows-1252', label: 'Windows-1252 / Latin-1', hasBom: false },
];

// ==================== 1. Connection Settings Dialog State ====================
const connTab = ref<'general' | 'options' | 'ssh' | 'schemas' | 'advanced'>('general');
const showCommentBox = ref(false);
const showPasswordPlain = ref(false);
const urlExpanded = ref(false);
const testingDraft = ref(false);
const testResult = ref<{ ok: boolean; text: string } | null>(null);
const templates = ref<SavedConnTemplate[]>(loadTemplates());

function loadTemplates(): SavedConnTemplate[] {
  try {
    const raw = localStorage.getItem('pacergrip.connTemplates');
    return raw ? (JSON.parse(raw) as SavedConnTemplate[]) : [];
  } catch {
    return [];
  }
}

const connForm = reactive<{
  id: string | null;
  projectId: string;
  name: string;
  color: string;
  comment: string;
  dbType: string;
  connectionType: 'default' | 'socket' | 'urlOnly';
  host: string;
  port: string;
  socketPath: string;
  authType: 'userPassword' | 'noAuth';
  username: string;
  password: string;
  passwordSaveMode: 'forever' | 'session' | 'never';
  hasSavedPassword: boolean;
  database: string;
  jdbcUrl: string;
  env: EnvLevel;
  readOnly: boolean;
  sslMode: string;
  sshEnabled: boolean;
  sshHost: string;
  sshPort: string;
  sshUser: string;
  sshKeyFile: string;
  schemaPattern: string;
  advancedProps: Array<{ key: string; value: string }>;
}>({
  id: null,
  projectId: '',
  name: '',
  color: '',
  comment: '',
  dbType: 'tiDb',
  connectionType: 'default',
  host: '127.0.0.1',
  port: '4000',
  socketPath: '',
  authType: 'userPassword',
  username: 'root',
  password: '',
  passwordSaveMode: 'forever',
  hasSavedPassword: false,
  database: '',
  jdbcUrl: 'jdbc:mysql://127.0.0.1:4000',
  env: 'DEV',
  readOnly: false,
  sslMode: '',
  sshEnabled: false,
  sshHost: '',
  sshPort: '22',
  sshUser: '',
  sshKeyFile: '',
  schemaPattern: '',
  advancedProps: [],
});

const isSqliteEngine = computed(() => connForm.dbType === 'sqlite');

function normalizeDbTypeString(dbType: string | Record<string, string>): string {
  if (typeof dbType === 'string') return dbType;
  return Object.values(dbType)[0] ?? 'mySql';
}

function buildUrlFromFields(): string {
  const h = connForm.host.trim() || 'localhost';
  const p = connForm.port.trim();
  const db = connForm.database.trim();
  const portPart = p ? `:${p}` : '';
  const dbPart = db ? `/${db}` : '';

  let base = '';
  if (connForm.dbType === 'sqlite') {
    base = `jdbc:sqlite:${db}`;
  } else if (connForm.dbType === 'oracle') {
    base = `jdbc:oracle:thin:@//${h}${portPart}${dbPart}`;
  } else if (connForm.dbType === 'sqlServer') {
    base = db ? `jdbc:sqlserver://${h}${portPart};databaseName=${db}` : `jdbc:sqlserver://${h}${portPart}`;
  } else if (connForm.dbType === 'postgreSql') {
    base = `jdbc:postgresql://${h}${portPart}${dbPart}`;
  } else if (connForm.dbType === 'mariaDb') {
    base = `jdbc:mariadb://${h}${portPart}${dbPart}`;
  } else if (connForm.dbType === 'oceanBase') {
    base = `jdbc:oceanbase://${h}${portPart}${dbPart}`;
  } else if (connForm.dbType === 'kingbase') {
    base = `jdbc:kingbase8://${h}${portPart}${dbPart}`;
  } else if (connForm.dbType === 'openGauss') {
    base = `jdbc:opengauss://${h}${portPart}${dbPart}`;
  } else if (connForm.dbType === 'dameng') {
    base = `jdbc:dm://${h}${portPart}${dbPart}`;
  } else if (connForm.dbType === 'clickHouse') {
    base = `jdbc:clickhouse://${h}${portPart}${dbPart}`;
  } else {
    // TiDB and MySQL both use standard `jdbc:mysql://host:port[/db]`
    base = `jdbc:mysql://${h}${portPart}${dbPart}`;
  }

  const pairs = connForm.advancedProps
    .map((kv) => ({ k: kv.key.trim(), v: kv.value.trim() }))
    .filter((kv) => kv.k.length > 0);

  if (connForm.connectionType === 'socket' && connForm.socketPath.trim()) {
    if (!pairs.some((kv) => kv.k === 'socket')) {
      pairs.push({ k: 'socket', v: connForm.socketPath.trim() });
    }
  }

  if (pairs.length === 0) return base;
  const sep = connForm.dbType === 'sqlServer' ? ';' : '&';
  const joiner = connForm.dbType === 'sqlServer' ? ';' : base.includes('?') ? '&' : '?';
  return `${base}${joiner}${pairs.map((kv) => `${kv.k}=${kv.v}`).join(sep)}`;
}

function syncUrlFromFields() {
  if (connForm.connectionType === 'urlOnly') return;
  connForm.jdbcUrl = buildUrlFromFields();
}

function onEngineChange() {
  const opt = ENGINE_OPTIONS.find((o) => o.value === connForm.dbType);
  if (opt && opt.defaultPort) {
    connForm.port = String(opt.defaultPort);
  }
  if (connForm.dbType === 'sqlite') {
    connForm.host = '';
    connForm.port = '';
    connForm.username = '';
    connForm.authType = 'noAuth';
    if (!connForm.database) {
      connForm.database = 'C:/data/sample.db';
    }
  } else {
    if (!connForm.host) connForm.host = '127.0.0.1';
    if (connForm.authType === 'noAuth') connForm.authType = 'userPassword';
  }
  syncUrlFromFields();
}

function onColorChange() {
  const chosen = COLOR_OPTIONS.find((c) => c.value === connForm.color);
  if (chosen?.env) {
    connForm.env = chosen.env;
  }
}

/**
 * Real-time reverse parser: editing or pasting ANY JDBC URL or native URI
 * (`jdbc:mysql://9.20.160.17:4000`, `mysql://user:pass@host:3306/db?useSSL=false`, etc.)
 * automatically binds back to Host, Port, Database, Driver, User, Password & Advanced params!
 */
function onUrlManualInput() {
  const raw = connForm.jdbcUrl.trim();
  if (!raw) return;

  const afterJdbc = raw.toLowerCase().startsWith('jdbc:') ? raw.slice(5) : raw;

  if (afterJdbc.toLowerCase().startsWith('sqlite:')) {
    connForm.dbType = 'sqlite';
    connForm.database = afterJdbc.slice(7).replace(/^\/\//, '').split('?')[0] ?? '';
    return;
  }

  const schemeIdx = afterJdbc.indexOf('://');
  if (schemeIdx <= 0) return;

  const scheme = afterJdbc.slice(0, schemeIdx).split(':')[0].toLowerCase();
  // Refine driver only if scheme belongs to a different family (preserve TiDB/OceanBase/MariaDB on jdbc:mysql://)
  if (scheme === 'mysql') {
    if (!['tiDb', 'mySql', 'mariaDb', 'oceanBase'].includes(connForm.dbType)) {
      connForm.dbType = 'mySql';
    }
  } else if (scheme === 'mariadb') {
    connForm.dbType = 'mariaDb';
  } else if (scheme === 'tidb') {
    connForm.dbType = 'tiDb';
  } else if (scheme === 'oceanbase') {
    connForm.dbType = 'oceanBase';
  } else if (scheme === 'postgresql' || scheme === 'postgres' || scheme === 'pgsql') {
    if (!['postgreSql', 'kingbase', 'openGauss'].includes(connForm.dbType)) {
      connForm.dbType = 'postgreSql';
    }
  } else if (scheme === 'kingbase8' || scheme === 'kingbase') {
    connForm.dbType = 'kingbase';
  } else if (scheme === 'opengauss') {
    connForm.dbType = 'openGauss';
  } else if (scheme === 'dm') {
    connForm.dbType = 'dameng';
  } else if (scheme === 'clickhouse') {
    connForm.dbType = 'clickHouse';
  } else if (scheme === 'sqlserver') {
    connForm.dbType = 'sqlServer';
  }

  const remainder = afterJdbc.slice(schemeIdx + 3);
  const qIdx = remainder.indexOf('?');
  const beforeQuery = qIdx >= 0 ? remainder.slice(0, qIdx) : remainder;
  const queryStr = qIdx >= 0 ? remainder.slice(qIdx + 1) : '';

  const semiIdx = beforeQuery.indexOf(';');
  const beforeSemi = semiIdx >= 0 ? beforeQuery.slice(0, semiIdx) : beforeQuery;
  const semiStr = semiIdx >= 0 ? beforeQuery.slice(semiIdx + 1) : '';

  const slashIdx = beforeSemi.indexOf('/');
  const authority = slashIdx >= 0 ? beforeSemi.slice(0, slashIdx) : beforeSemi;
  const pathDb = slashIdx >= 0 ? beforeSemi.slice(slashIdx + 1).replace(/\/+$/, '') : '';

  // Parse optional user:pass@host:port
  const firstEndpoint = authority.split(',')[0] ?? '';
  const atIdx = firstEndpoint.lastIndexOf('@');
  let hostPort = firstEndpoint;
  if (atIdx >= 0) {
    const userInfo = firstEndpoint.slice(0, atIdx);
    hostPort = firstEndpoint.slice(atIdx + 1);
    const colonIdx = userInfo.indexOf(':');
    if (colonIdx >= 0) {
      const u = userInfo.slice(0, colonIdx).trim();
      const p = userInfo.slice(colonIdx + 1);
      if (u) connForm.username = u;
      if (p) connForm.password = p;
    } else if (userInfo.trim()) {
      connForm.username = userInfo.trim();
    }
  }

  const lastColon = hostPort.lastIndexOf(':');
  if (lastColon > 0 && /^\d+$/.test(hostPort.slice(lastColon + 1))) {
    connForm.host = hostPort.slice(0, lastColon);
    connForm.port = hostPort.slice(lastColon + 1);
  } else if (hostPort.trim()) {
    connForm.host = hostPort.trim();
  }

  connForm.database = pathDb;

  // Sync query params into Advanced tab
  const parsedProps: Array<{ key: string; value: string }> = [];
  for (const part of [...queryStr.split('&'), ...semiStr.split(';')]) {
    const eq = part.indexOf('=');
    if (eq > 0) {
      const k = part.slice(0, eq).trim();
      const v = part.slice(eq + 1).trim();
      if (k) {
        parsedProps.push({ key: k, value: v });
        if (k.toLowerCase() === 'databasename' && !connForm.database) {
          connForm.database = v;
        }
        if (k === 'socket' || k === 'socketPath') {
          connForm.socketPath = v;
        }
      }
    }
  }
  connForm.advancedProps = parsedProps;
}

function addAdvancedProp() {
  connForm.advancedProps.push({ key: '', value: '' });
}

function removeAdvancedProp(index: number) {
  connForm.advancedProps.splice(index, 1);
  syncUrlFromFields();
}

function saveCurrentAsTemplate() {
  const name = connForm.name.trim() || `${connForm.dbType} 模板`;
  const tpl: SavedConnTemplate = {
    name,
    dbType: connForm.dbType,
    connectionType: connForm.connectionType,
    host: connForm.host,
    port: connForm.port,
    database: connForm.database,
    username: connForm.username,
    jdbcUrl: connForm.jdbcUrl,
    sslMode: connForm.sslMode,
  };
  templates.value = [tpl, ...templates.value.filter((x) => x.name !== name)].slice(0, 12);
  localStorage.setItem('pacergrip.connTemplates', JSON.stringify(templates.value));
  emit('notify', `已保存连接模板: ${name}`);
}

function applyTemplate(tplName: string) {
  const tpl = templates.value.find((x) => x.name === tplName);
  if (!tpl) return;
  connForm.dbType = tpl.dbType;
  connForm.connectionType = tpl.connectionType;
  connForm.host = tpl.host;
  connForm.port = tpl.port;
  connForm.database = tpl.database;
  connForm.username = tpl.username;
  connForm.jdbcUrl = tpl.jdbcUrl;
  connForm.sslMode = tpl.sslMode;
}

function buildDraftPayload(): ConnectionDraft {
  const name =
    connForm.name.trim() ||
    (isSqliteEngine.value
      ? 'SQLite 本地库'
      : `${connForm.dbType}-${connForm.host || 'localhost'}`);
  const parsedPort = connForm.port.trim() ? Number(connForm.port.trim()) : null;
  const propsMap: Record<string, string> = {};
  for (const kv of connForm.advancedProps) {
    const k = kv.key.trim();
    if (k) propsMap[k] = kv.value.trim();
  }

  return {
    id: connForm.id,
    projectId: connForm.projectId || props.activeProjectId || null,
    name,
    dbType: connForm.dbType,
    host: isSqliteEngine.value ? null : connForm.host.trim() || null,
    port: isSqliteEngine.value || !parsedPort || Number.isNaN(parsedPort) ? null : parsedPort,
    database: connForm.database.trim() || null,
    jdbcUrl: connForm.jdbcUrl.trim() || null,
    username:
      isSqliteEngine.value || connForm.authType === 'noAuth'
        ? null
        : connForm.username.trim() || null,
    env: connForm.env,
    color: connForm.color || null,
    readOnly: connForm.readOnly,
    comment: connForm.comment.trim() || null,
    connectionType: connForm.connectionType,
    authType: connForm.authType,
    passwordSaveMode: connForm.passwordSaveMode,
    socketPath: connForm.connectionType === 'socket' ? connForm.socketPath.trim() || null : null,
    driverProperties: propsMap,
    sslMode: connForm.sslMode || null,
    ssh: connForm.sshEnabled && connForm.sshHost.trim()
      ? {
          enabled: true,
          host: connForm.sshHost.trim(),
          port: Number(connForm.sshPort) || 22,
          username: connForm.sshUser.trim(),
          authType: connForm.sshKeyFile.trim() ? 'KEY_PAIR' : 'PASSWORD',
          keyFile: connForm.sshKeyFile.trim() || null,
        }
      : null,
    schemaPattern: connForm.schemaPattern.trim() || null,
  };
}

function formatServerVersion(version?: string | null, fallback?: string): string {
  if (!version) return fallback || 'Connected';
  const tidbMatch = version.match(/TiDB-v?([0-9.]+)/i);
  if (tidbMatch) return `TiDB ${tidbMatch[1]}`;
  return version.length > 44 ? `${version.slice(0, 44)}...` : version;
}

async function runInlineTestConnection() {
  testingDraft.value = true;
  testResult.value = null;
  try {
    const draft = buildDraftPayload();
    const pwd = connForm.password.length > 0 ? connForm.password : null;
    const ping = await testConnectionDraft(draft, pwd);
    if (ping.success) {
      const label = formatServerVersion(ping.serverVersion, ping.message);
      testResult.value = { ok: true, text: `${label} (${ping.latencyMs} ms)` };
    } else {
      testResult.value = { ok: false, text: ping.message };
    }
  } catch (err) {
    testResult.value = { ok: false, text: String(err) };
  } finally {
    testingDraft.value = false;
  }
}

function submitConnection(closeAfter: boolean) {
  const draft = buildDraftPayload();
  const pwd = connForm.password.length > 0 ? connForm.password : null;
  emit('save-connection', draft, pwd, closeAfter);
}

watch(
  () => [props.activeModal, props.editingProfile] as const,
  ([modal, prof]) => {
    if (modal !== 'connection') return;
    connTab.value = 'general';
    testResult.value = null;
    showPasswordPlain.value = false;
    urlExpanded.value = false;

    if (prof) {
      const cfg = (prof.mode?.config ?? {}) as Partial<DirectConfig>;
      connForm.id = prof.id;
      connForm.projectId = prof.projectId || props.activeProjectId || '';
      connForm.name = prof.name;
      connForm.color = prof.color ?? '';
      connForm.comment = prof.comment ?? '';
      showCommentBox.value = Boolean(prof.comment);
      connForm.dbType = normalizeDbTypeString(prof.dbType);
      connForm.connectionType =
        cfg.connectionType === 'socket' || cfg.connectionType === 'urlOnly'
          ? cfg.connectionType
          : 'default';
      connForm.host = cfg.host ?? prof.fingerprint.host ?? '';
      connForm.port =
        cfg.port !== null && cfg.port !== undefined
          ? String(cfg.port)
          : prof.fingerprint.port
            ? String(prof.fingerprint.port)
            : '';
      connForm.socketPath = cfg.socketPath ?? '';
      connForm.authType = cfg.authType === 'noAuth' ? 'noAuth' : 'userPassword';
      connForm.username = cfg.username ?? '';
      connForm.password = '';
      connForm.passwordSaveMode =
        cfg.passwordSaveMode === 'session' || cfg.passwordSaveMode === 'never'
          ? cfg.passwordSaveMode
          : 'forever';
      connForm.hasSavedPassword = Boolean(cfg.hasSavedPassword);
      connForm.database = cfg.database ?? prof.fingerprint.database ?? '';
      connForm.jdbcUrl = cfg.jdbcUrl || buildUrlFromFields();
      connForm.env = prof.env ?? 'DEV';
      connForm.readOnly = Boolean(cfg.readOnly);
      connForm.sslMode = cfg.ssl?.mode ?? (cfg.ssl && !cfg.ssl.enabled ? 'disabled' : '');
      connForm.sshEnabled = Boolean(cfg.ssh?.enabled);
      connForm.sshHost = cfg.ssh?.host ?? '';
      connForm.sshPort = cfg.ssh?.port ? String(cfg.ssh.port) : '22';
      connForm.sshUser = cfg.ssh?.username ?? '';
      connForm.sshKeyFile = cfg.ssh?.keyFile ?? '';
      connForm.schemaPattern = cfg.schemaPattern ?? '';
      connForm.advancedProps = Object.entries(cfg.driverProperties ?? {}).map(([key, value]) => ({
        key,
        value,
      }));
    } else {
      connForm.id = null;
      connForm.projectId = props.activeProjectId || props.projects[0]?.id || '';
      connForm.name = '';
      connForm.color = '';
      connForm.comment = '';
      showCommentBox.value = false;
      connForm.dbType = 'tiDb';
      connForm.connectionType = 'default';
      connForm.host = '127.0.0.1';
      connForm.port = '4000';
      connForm.socketPath = '';
      connForm.authType = 'userPassword';
      connForm.username = 'root';
      connForm.password = '';
      connForm.passwordSaveMode = 'forever';
      connForm.hasSavedPassword = false;
      connForm.database = '';
      connForm.jdbcUrl = 'jdbc:mysql://127.0.0.1:4000';
      connForm.env = 'DEV';
      connForm.readOnly = false;
      connForm.sslMode = '';
      connForm.sshEnabled = false;
      connForm.sshHost = '';
      connForm.sshPort = '22';
      connForm.sshUser = '';
      connForm.sshKeyFile = '';
      connForm.schemaPattern = '';
      connForm.advancedProps = [];
    }
  },
);

// ==================== 8. Project Create/Edit & 9. Grid Mutation SQL Preview ====================
const PROJECT_COLORS = ['#3574F0', '#22C55E', '#F59E0B', '#EF4444', '#A855F7', '#06B6D4', '#EC4899'];
const projectForm = reactive<{
  id: string | null;
  name: string;
  description: string;
  color: string;
  rootDir: string;
}>({
  id: null,
  name: '',
  description: '',
  color: '#3574F0',
  rootDir: '',
});

const editableGridSql = ref('');

watch(
  () => [props.activeModal, props.editingProject, props.gridSqlContext] as const,
  ([modal, proj, gridCtx]) => {
    if (modal === 'project') {
      if (proj) {
        projectForm.id = proj.id;
        projectForm.name = proj.name;
        projectForm.description = proj.description ?? '';
        projectForm.color = proj.color ?? '#3574F0';
        projectForm.rootDir = proj.rootDir ?? '';
      } else {
        projectForm.id = null;
        projectForm.name = '';
        projectForm.description = '';
        projectForm.color = PROJECT_COLORS[props.projects.length % PROJECT_COLORS.length];
        projectForm.rootDir = '';
      }
    } else if (modal === 'grid-sql' && gridCtx) {
      editableGridSql.value = gridCtx.plan.statements.join('\n');
    }
  },
);

async function browseProjectRootDir() {
  try {
    const dir = await pickDirectoryToMount();
    if (dir) projectForm.rootDir = dir;
  } catch (err) {
    emit('notify', String(err));
  }
}

function submitProjectForm() {
  if (!projectForm.name.trim()) {
    emit('notify', '请输入项目名称');
    return;
  }
  emit('save-project', {
    id: projectForm.id,
    name: projectForm.name.trim(),
    description: projectForm.description.trim() || null,
    color: projectForm.color || '#3574F0',
    rootDir: projectForm.rootDir.trim() || null,
  });
}

async function copyGridSqlText() {
  if (!editableGridSql.value.trim()) return;
  try {
    await navigator.clipboard.writeText(editableGridSql.value);
    emit('notify', t('copiedNotice'));
  } catch {
    emit('notify', '剪贴板写入失败');
  }
}

function confirmExecuteGridSql() {
  if (!props.gridSqlContext || !editableGridSql.value.trim()) return;
  emit('execute-grid-sql', props.gridSqlContext, editableGridSql.value);
}

// ==================== 2. DBeaver Import / Export Modal State ====================
const importPath = ref('');
const importText = ref('');
const includeDbeaverPasswords = ref(true);
const discoveredDbeaver = ref<DbeaverDiscoveryItem[]>([]);
const scanningDbeaver = ref(false);

async function scanLocalDbeaver() {
  scanningDbeaver.value = true;
  try {
    discoveredDbeaver.value = await discoverDbeaver();
  } catch {
    discoveredDbeaver.value = [];
  } finally {
    scanningDbeaver.value = false;
  }
}

async function browseDbeaverFile() {
  try {
    const picked = await pickDbeaverImportFile();
    if (picked) importPath.value = picked;
  } catch (err) {
    emit('notify', String(err));
  }
}

async function browseDbeaverFolder() {
  try {
    const picked = await pickDbeaverFolder();
    if (picked) importPath.value = picked;
  } catch (err) {
    emit('notify', String(err));
  }
}

async function copyExportedJson() {
  if (!props.exportedJson) return;
  try {
    await navigator.clipboard.writeText(props.exportedJson);
    emit('notify', t('copiedNotice'));
  } catch {
    emit('notify', '剪贴板写入失败');
  }
}

// ==================== 3. Encoding Modal State ====================
const selectedEncoding = ref('UTF-8');
const selectedLineEnding = ref<'CRLF' | 'LF'>('LF');

watch(
  () => [props.activeModal, props.encodingContext] as const,
  ([modal, ctx]) => {
    if (modal === 'encoding' && ctx) {
      selectedEncoding.value = ctx.encoding || 'UTF-8';
      selectedLineEnding.value = ctx.lineEnding === 'CRLF' ? 'CRLF' : 'LF';
    }
  },
);

function triggerConvertEncoding() {
  const opt = ENCODING_OPTIONS.find((o) => o.value === selectedEncoding.value);
  emit(
    'convert-encoding',
    selectedEncoding.value,
    opt ? opt.hasBom : selectedEncoding.value === 'UTF-8 BOM',
    selectedLineEnding.value,
  );
}

// ==================== 4. Password Prompt & Search Everywhere ====================
const sessionPassword = ref('');
const passwordInputRef = ref<HTMLInputElement | null>(null);
const searchQuery = ref('');
const searchActiveIdx = ref(0);
const searchInputRef = ref<HTMLInputElement | null>(null);

watch(
  () => props.activeModal,
  async (modal) => {
    if (modal !== 'shortcuts') {
      cancelRecordingKey();
    } else {
      keymapSearch.value = '';
    }
    if (modal === 'import') {
      await scanLocalDbeaver();
    } else if (modal === 'password') {
      sessionPassword.value = '';
      await nextTick();
      passwordInputRef.value?.focus();
    } else if (modal === 'search') {
      searchQuery.value = '';
      searchActiveIdx.value = 0;
      await nextTick();
      searchInputRef.value?.focus();
    }
  },
);

function submitPasswordPrompt() {
  if (!props.passwordProfile) return;
  emit('submit-password', props.passwordProfile.id, sessionPassword.value);
}

const filteredSearchItems = computed(() => {
  const q = searchQuery.value.trim().toLowerCase();
  if (!q) return props.searchItems.slice(0, 30);
  return props.searchItems
    .filter(
      (item) =>
        item.label.toLowerCase().includes(q) ||
        item.detail.toLowerCase().includes(q) ||
        item.badge.toLowerCase().includes(q),
    )
    .slice(0, 40);
});

watch(filteredSearchItems, () => {
  searchActiveIdx.value = 0;
});

function onSearchKeydown(e: KeyboardEvent) {
  const total = filteredSearchItems.value.length;
  if (e.key === 'Escape') {
    emit('close');
    return;
  }
  if (total === 0) return;
  if (e.key === 'ArrowDown') {
    e.preventDefault();
    searchActiveIdx.value = (searchActiveIdx.value + 1) % total;
  } else if (e.key === 'ArrowUp') {
    e.preventDefault();
    searchActiveIdx.value = (searchActiveIdx.value - 1 + total) % total;
  } else if (e.key === 'Enter') {
    e.preventDefault();
    const chosen = filteredSearchItems.value[searchActiveIdx.value];
    if (chosen) emit('select-search-item', chosen);
  }
}

// ==================== Interactive Keymap Configurator ====================
const userKeymap = useKeymap();
const keymapSearch = ref('');
const keymapCategory = ref<'ALL' | ShortcutCategory>('ALL');
const recordingActionId = ref<ShortcutActionId | null>(null);
const keymapConflict = ref<{
  targetId: ShortcutActionId;
  combo: string;
  conflictWith: ShortcutActionDef;
} | null>(null);

const filteredKeymapActions = computed(() => {
  const q = keymapSearch.value.trim().toLowerCase();
  return SHORTCUT_ACTIONS.filter((act) => {
    if (keymapCategory.value !== 'ALL' && act.category !== keymapCategory.value) return false;
    if (q) {
      const bound = userKeymap.getDisplay(act.id).toLowerCase();
      const hay = `${act.labelZh} ${act.labelEn} ${bound}`.toLowerCase();
      if (!hay.includes(q)) return false;
    }
    return true;
  });
});

function handleKeymapCaptureKeydown(e: KeyboardEvent) {
  if (!recordingActionId.value) return;
  e.preventDefault();
  e.stopPropagation();

  if (e.key === 'Escape') {
    cancelRecordingKey();
    return;
  }
  if (e.key === 'Backspace' || e.key === 'Delete') {
    const id = recordingActionId.value;
    userKeymap.clearBinding(id);
    cancelRecordingKey();
    emit('notify', '已清空该功能快捷键');
    return;
  }

  const combo = eventToCombo(e);
  if (!combo) return;

  const targetId = recordingActionId.value;
  const conflict = userKeymap.findConflict(combo, targetId);
  if (conflict) {
    keymapConflict.value = { targetId, combo, conflictWith: conflict };
    window.removeEventListener('keydown', handleKeymapCaptureKeydown, { capture: true });
    recordingActionId.value = null;
    return;
  }

  userKeymap.setBinding(targetId, combo);
  cancelRecordingKey();
  emit('notify', `已更新快捷键: ${formatComboDisplay(combo)}`);
}

function startRecordingKey(actionId: ShortcutActionId) {
  keymapConflict.value = null;
  recordingActionId.value = actionId;
  window.removeEventListener('keydown', handleKeymapCaptureKeydown, { capture: true });
  window.addEventListener('keydown', handleKeymapCaptureKeydown, { capture: true });
}

function cancelRecordingKey() {
  recordingActionId.value = null;
  keymapConflict.value = null;
  window.removeEventListener('keydown', handleKeymapCaptureKeydown, { capture: true });
}

function applyConflictOverwrite() {
  if (!keymapConflict.value) return;
  const { targetId, combo, conflictWith } = keymapConflict.value;
  userKeymap.clearBinding(conflictWith.id);
  userKeymap.setBinding(targetId, combo);
  keymapConflict.value = null;
  emit('notify', `已覆盖并绑定快捷键 ${formatComboDisplay(combo)}`);
}

onBeforeUnmount(() => {
  window.removeEventListener('keydown', handleKeymapCaptureKeydown, { capture: true });
});
</script>

<template>
  <!-- 1. JetBrains-Style Connection Editor Modal (Bidirectional URL Binding + Inline Test Connection) -->
  <div
    v-if="activeModal === 'connection'"
    class="modal-backdrop"
    @click.self="emit('close')"
  >
    <div class="conn-modal-card">
      <div class="conn-modal-body">
        <!-- Top Name + Color Row -->
        <div class="conn-top-grid">
          <label class="conn-row-label">{{ t('connName') }}:</label>
          <input
            v-model="connForm.name"
            class="field"
            placeholder="例如：policy-UAT-9.20.160.17"
          />
          <label class="conn-row-label">{{ t('connColor') }}:</label>
          <select v-model="connForm.color" class="field" @change="onColorChange">
            <option v-for="c in COLOR_OPTIONS" :key="c.value" :value="c.value">
              {{ c.label }}
            </option>
          </select>
        </div>

        <div style="padding-left: 82px; margin-top: 2px">
          <button
            v-if="!showCommentBox"
            class="conn-link-btn"
            @click="showCommentBox = true"
          >
            {{ t('connAddComment') }}
          </button>
          <input
            v-else
            v-model="connForm.comment"
            class="field"
            style="margin-top: 4px"
            placeholder="填写连接备注说明..."
          />
        </div>

        <!-- Tabs Row: General | Options | SSH/SSL | Schemas | Advanced  +  Save as Template -->
        <div class="conn-tabs-bar">
          <div class="conn-tab-list">
            <button
              class="conn-tab"
              :class="{ active: connTab === 'general' }"
              @click="connTab = 'general'"
            >
              {{ locale === 'zh-CN' ? '常规 (General)' : 'General' }}
            </button>
            <button
              class="conn-tab"
              :class="{ active: connTab === 'options' }"
              @click="connTab = 'options'"
            >
              {{ locale === 'zh-CN' ? '选项 (Options)' : 'Options' }}
            </button>
            <button
              class="conn-tab"
              :class="{ active: connTab === 'ssh' }"
              @click="connTab = 'ssh'"
            >
              SSH/SSL
            </button>
            <button
              class="conn-tab"
              :class="{ active: connTab === 'schemas' }"
              @click="connTab = 'schemas'"
            >
              {{ locale === 'zh-CN' ? '架构 (Schemas)' : 'Schemas' }}
            </button>
            <button
              class="conn-tab"
              :class="{ active: connTab === 'advanced' }"
              @click="connTab = 'advanced'"
            >
              {{ locale === 'zh-CN' ? '高级 (Advanced)' : 'Advanced' }}
            </button>
          </div>

          <div style="display: flex; align-items: center; gap: 8px">
            <select
              v-if="templates.length > 0"
              class="field"
              style="width: auto; max-width: 120px; padding: 3px 6px; font-size: 11px"
              @change="applyTemplate(($event.target as HTMLSelectElement).value)"
            >
              <option value="">载入模板...</option>
              <option v-for="tpl in templates" :key="tpl.name" :value="tpl.name">
                {{ tpl.name }}
              </option>
            </select>
            <button class="conn-link-btn" @click="saveCurrentAsTemplate">
              {{ t('saveAsTemplate') }}
            </button>
          </div>
        </div>

        <!-- TAB: GENERAL -->
        <div v-if="connTab === 'general'" class="conn-rows">
          <!-- Project Binding (when multiple projects exist or creating new) -->
          <div v-if="projects.length > 0" class="conn-row">
            <label class="conn-row-label">所属项目:</label>
            <select v-model="connForm.projectId" class="field">
              <option v-for="proj in projects" :key="proj.id" :value="proj.id">
                {{ proj.name }}
              </option>
            </select>
          </div>

          <!-- Driver -->
          <div class="conn-row">
            <label class="conn-row-label">{{ t('connEngine') }}:</label>
            <select v-model="connForm.dbType" class="field" @change="onEngineChange">
              <option v-for="opt in ENGINE_OPTIONS" :key="opt.value" :value="opt.value">
                {{ opt.label }}
              </option>
            </select>
          </div>

          <!-- Connection type: default | Unix Socket | URL only -->
          <div v-if="!isSqliteEngine" class="conn-row">
            <label class="conn-row-label">{{ t('connType') }}:</label>
            <div class="seg-group">
              <button
                class="seg-btn"
                :class="{ active: connForm.connectionType === 'default' }"
                @click="connForm.connectionType = 'default'; syncUrlFromFields()"
              >
                default
              </button>
              <button
                class="seg-btn"
                :class="{ active: connForm.connectionType === 'socket' }"
                @click="connForm.connectionType = 'socket'; syncUrlFromFields()"
              >
                Unix Socket
              </button>
              <button
                class="seg-btn"
                :class="{ active: connForm.connectionType === 'urlOnly' }"
                @click="connForm.connectionType = 'urlOnly'"
              >
                URL only
              </button>
            </div>
          </div>

          <!-- SQLite Path -->
          <div v-if="isSqliteEngine" class="conn-row">
            <label class="conn-row-label">{{ t('connSqlitePath') }}:</label>
            <input
              v-model="connForm.database"
              class="field"
              placeholder="C:/data/app.db 或 :memory:"
              @input="syncUrlFromFields"
            />
          </div>

          <template v-else>
            <!-- Host + Port Row (when default) -->
            <div v-if="connForm.connectionType === 'default'" class="conn-row">
              <label class="conn-row-label">{{ t('connHost') }}:</label>
              <div class="conn-inline-pair">
                <input
                  v-model="connForm.host"
                  class="field"
                  placeholder="9.20.160.17"
                  @input="syncUrlFromFields"
                />
                <label class="conn-row-label">{{ t('connPort') }}:</label>
                <input
                  v-model="connForm.port"
                  class="field"
                  placeholder="4000"
                  @input="syncUrlFromFields"
                />
              </div>
            </div>

            <!-- Unix Socket Row -->
            <div v-if="connForm.connectionType === 'socket'" class="conn-row">
              <label class="conn-row-label">Socket:</label>
              <input
                v-model="connForm.socketPath"
                class="field"
                placeholder="/var/run/mysqld/mysqld.sock"
                @input="syncUrlFromFields"
              />
            </div>

            <!-- Authentication -->
            <div class="conn-row">
              <label class="conn-row-label">{{ t('connAuth') }}:</label>
              <select v-model="connForm.authType" class="field">
                <option value="userPassword">User &amp; Password（用户名和密码）</option>
                <option value="noAuth">No Auth（无认证）</option>
              </select>
            </div>

            <!-- User & Password (when userPassword) -->
            <template v-if="connForm.authType === 'userPassword'">
              <div class="conn-row">
                <label class="conn-row-label">{{ t('connUser') }}:</label>
                <input
                  v-model="connForm.username"
                  class="field"
                  style="max-width: 72%"
                  placeholder="policy_d01_uat"
                />
              </div>

              <div class="conn-row">
                <label class="conn-row-label">{{ t('connPassword') }}:</label>
                <div class="conn-inline-pair">
                  <div class="url-input-wrap">
                    <input
                      v-model="connForm.password"
                      :type="showPasswordPlain ? 'text' : 'password'"
                      class="field"
                      style="font-family: var(--font-ui)"
                      :placeholder="connForm.hasSavedPassword ? '<hidden>' : '请输入数据库密码'"
                    />
                    <button
                      class="url-expand-btn"
                      type="button"
                      title="显示/隐藏密码"
                      @click="showPasswordPlain = !showPasswordPlain"
                    >
                      {{ showPasswordPlain ? '🙈' : '👁' }}
                    </button>
                  </div>
                  <label class="conn-row-label">{{ t('connSaveMode') }}:</label>
                  <select v-model="connForm.passwordSaveMode" class="field">
                    <option value="forever">Forever（永久）</option>
                    <option value="session">Until restart（会话）</option>
                    <option value="never">Never（不保存）</option>
                  </select>
                </div>
              </div>
            </template>

            <!-- Database (optional, can be left blank!) -->
            <div v-if="connForm.connectionType !== 'urlOnly'" class="conn-row">
              <label class="conn-row-label">{{ t('connDb') }}:</label>
              <input
                v-model="connForm.database"
                class="field"
                placeholder="可留空（连接实例后浏览所有数据库）"
                @input="syncUrlFromFields"
              />
            </div>
          </template>

          <!-- URL (Always visible, bidirectional binding + overrides above) -->
          <div class="conn-row align-start">
            <label class="conn-row-label" style="padding-top: 5px">{{ t('connJdbcUrl') }}:</label>
            <div>
              <div class="url-input-wrap">
                <textarea
                  v-if="urlExpanded"
                  v-model="connForm.jdbcUrl"
                  class="field"
                  rows="3"
                  style="resize: vertical"
                  placeholder="jdbc:mysql://9.20.160.17:4000"
                  @input="onUrlManualInput"
                />
                <input
                  v-else
                  v-model="connForm.jdbcUrl"
                  class="field"
                  placeholder="jdbc:mysql://9.20.160.17:4000"
                  @input="onUrlManualInput"
                />
                <button
                  class="url-expand-btn"
                  type="button"
                  title="展开 / 折叠长 URL 编辑框"
                  @click="urlExpanded = !urlExpanded"
                >
                  ⤢
                </button>
              </div>
              <div class="conn-helper-hint">{{ t('connUrlOverrides') }}</div>
            </div>
          </div>
        </div>

        <!-- TAB: OPTIONS -->
        <div v-else-if="connTab === 'options'" class="conn-rows">
          <div class="conn-row">
            <label class="conn-row-label">{{ t('connEnv') }}:</label>
            <select v-model="connForm.env" class="field">
              <option value="DEV">DEV（开发环境 · 绿色标签条）</option>
              <option value="TEST">TEST / UAT（测试环境 · 琥珀色标签条）</option>
              <option value="PROD">PROD（生产环境 · 红色高危写拦截）</option>
            </select>
          </div>
          <div class="conn-row">
            <label class="conn-row-label">安全保护:</label>
            <label style="display: flex; align-items: center; gap: 8px; cursor: pointer; font-size: 12px">
              <input v-model="connForm.readOnly" type="checkbox" />
              <span>{{ t('connReadOnly') }}</span>
            </label>
          </div>
          <div class="conn-row align-start">
            <label class="conn-row-label" style="padding-top: 5px">备注:</label>
            <textarea
              v-model="connForm.comment"
              class="field"
              rows="3"
              placeholder="连接用途说明、负责人或集群信息..."
            />
          </div>
        </div>

        <!-- TAB: SSH/SSL -->
        <div v-else-if="connTab === 'ssh'" class="conn-rows">
          <div class="conn-row">
            <label class="conn-row-label">SSL / TLS 模式:</label>
            <select v-model="connForm.sslMode" class="field">
              <option value="">默认（Preferred 优先尝试 TLS，不支持时降级）</option>
              <option value="disabled">禁用（Disabled / useSSL=false）</option>
              <option value="required">强制加密（Required）</option>
              <option value="verify_ca">校验证书链（Verify CA）</option>
              <option value="verify_identity">严格校验主机名（Verify Identity）</option>
            </select>
          </div>

          <div class="conn-row">
            <label class="conn-row-label">SSH 跳板机:</label>
            <label style="display: flex; align-items: center; gap: 8px; cursor: pointer; font-size: 12px">
              <input v-model="connForm.sshEnabled" type="checkbox" />
              <span>保存 SSH 隧道元数据配置（与 DBeaver 导入兼容）</span>
            </label>
          </div>

          <template v-if="connForm.sshEnabled">
            <div class="conn-row">
              <label class="conn-row-label">SSH 主机:</label>
              <div class="conn-inline-pair">
                <input v-model="connForm.sshHost" class="field" placeholder="bastion.internal" />
                <label class="conn-row-label">端口:</label>
                <input v-model="connForm.sshPort" class="field" placeholder="22" />
              </div>
            </div>
            <div class="conn-row">
              <label class="conn-row-label">SSH 用户名:</label>
              <input v-model="connForm.sshUser" class="field" placeholder="ops" />
            </div>
            <div class="conn-row">
              <label class="conn-row-label">私钥路径:</label>
              <input v-model="connForm.sshKeyFile" class="field" placeholder="C:/Users/.../.ssh/id_rsa" />
            </div>
          </template>
        </div>

        <!-- TAB: SCHEMAS -->
        <div v-else-if="connTab === 'schemas'" class="conn-rows">
          <div class="conn-row">
            <label class="conn-row-label">默认 Schema 筛选:</label>
            <input
              v-model="connForm.schemaPattern"
              class="field"
              placeholder="例如：policy_*（留空显示全部可见数据库/Schema）"
            />
          </div>
          <div class="conn-helper-hint" style="padding-left: 122px">
            连接成功后可在左侧数据库树中自由展开实例下的任意库表；TiDB / MySQL 允许 Database 留空直接连接。
          </div>
        </div>

        <!-- TAB: ADVANCED -->
        <div v-else-if="connTab === 'advanced'" class="conn-rows">
          <div style="display: flex; justify-content: space-between; align-items: center">
            <span style="font-size: 12px; color: var(--text-secondary)">
              JDBC URL 扩展连接参数（修改后自动同步到 URL）
            </span>
            <button class="btn btn-sm" @click="addAdvancedProp">+ 添加参数</button>
          </div>

          <div
            v-for="(kv, idx) in connForm.advancedProps"
            :key="idx"
            style="display: grid; grid-template-columns: 1fr 1fr auto; gap: 6px; align-items: center"
          >
            <input
              v-model="kv.key"
              class="field"
              style="font-family: var(--font-mono)"
              placeholder="参数名 (如 useSSL / serverTimezone)"
              @input="syncUrlFromFields"
            />
            <input
              v-model="kv.value"
              class="field"
              style="font-family: var(--font-mono)"
              placeholder="参数值 (如 false / Asia/Shanghai)"
              @input="syncUrlFromFields"
            />
            <button class="icon-btn compact" title="删除参数" @click="removeAdvancedProp(idx)">
              ×
            </button>
          </div>

          <div v-if="connForm.advancedProps.length === 0" class="empty-state" style="padding: 12px 0">
            暂无额外连接参数。点击右上角「+ 添加参数」，或直接在「常规」页的 URL 中拼接 `?key=value`。
          </div>
        </div>
      </div>

      <!-- Footer: Left Test Connection + inline version | Right OK / Cancel / Apply -->
      <div class="conn-modal-footer">
        <div class="conn-test-zone">
          <button
            class="conn-link-btn"
            style="font-weight: 600"
            :disabled="testingDraft || busy"
            @click="runInlineTestConnection"
          >
            {{ t('connTestBtn') }}
          </button>

          <span v-if="testingDraft" class="conn-test-status muted">⏳ 正在测试连通性...</span>
          <span
            v-else-if="testResult"
            class="conn-test-status"
            :class="testResult.ok ? 'ok' : 'err'"
            :title="testResult.text"
          >
            {{ testResult.ok ? `✓ ${testResult.text}` : `✕ ${testResult.text}` }}
          </span>
        </div>

        <div style="display: flex; align-items: center; gap: 8px">
          <button class="btn btn-primary" :disabled="busy" @click="submitConnection(true)">
            {{ t('btnOk') }}
          </button>
          <button class="btn" @click="emit('close')">
            {{ t('btnCancel') }}
          </button>
          <button class="btn" :disabled="busy" @click="submitConnection(false)">
            {{ t('btnApply') }}
          </button>
        </div>
      </div>
    </div>
  </div>

  <!-- 2. DBeaver Connection Import / Export Modal -->
  <div
    v-else-if="activeModal === 'import'"
    class="modal-backdrop"
    @click.self="emit('close')"
  >
    <div class="modal-card">
      <div class="modal-header">
        <span>🦫 {{ t('importModalTitle') }}</span>
        <button class="icon-btn compact" @click="emit('close')">×</button>
      </div>

      <!-- Auto-Discovered Local DBeaver Workspaces -->
      <div style="margin-bottom: 14px">
        <div
          style="
            display: flex;
            align-items: center;
            justify-content: space-between;
            margin-bottom: 6px;
          "
        >
          <span style="font-size: 12px; font-weight: 600">
            本机自动检测到的 DBeaver 连接文件
          </span>
          <button class="btn btn-sm" :disabled="scanningDbeaver" @click="scanLocalDbeaver">
            {{ scanningDbeaver ? '扫描中...' : t('dbeaverScanBtn') }}
          </button>
        </div>

        <label
          style="
            display: flex;
            align-items: center;
            gap: 6px;
            font-size: 11.5px;
            color: var(--text-secondary);
            margin-bottom: 8px;
            cursor: pointer;
          "
        >
          <input v-model="includeDbeaverPasswords" type="checkbox" />
          <span>{{ t('dbeaverIncludePwd') }}</span>
        </label>

        <div v-if="discoveredDbeaver.length > 0">
          <div v-for="item in discoveredDbeaver" :key="item.path" class="dbeaver-card">
            <div style="min-width: 0; flex: 1">
              <div style="display: flex; align-items: center; gap: 6px; font-size: 12px; font-weight: 600">
                <span>{{ item.workspaceName }}</span>
                <span class="pill pill-dev">{{ item.connectionCount }} 个连接</span>
                <span class="pill pill-neutral">{{ item.format }}</span>
                <span v-if="item.hasCredentialsFile" class="pill pill-test">含密码库</span>
              </div>
              <div
                style="
                  font-size: 11px;
                  color: var(--text-secondary);
                  font-family: var(--font-mono);
                  overflow: hidden;
                  text-overflow: ellipsis;
                  white-space: nowrap;
                  margin-top: 2px;
                "
                :title="item.path"
              >
                {{ item.path }}
              </div>
            </div>
            <button
              class="btn btn-primary btn-sm"
              :disabled="busy"
              @click="emit('import-dbeaver-path', item.path, includeDbeaverPasswords)"
            >
              一键导入
            </button>
          </div>
        </div>
        <div
          v-else
          style="
            padding: 8px 10px;
            border-radius: 6px;
            background: var(--bg-editor);
            border: 1px dashed var(--border);
            font-size: 11.5px;
            color: var(--text-secondary);
          "
        >
          未在默认路径（%APPDATA%\DBeaverData\workspace6）检测到 DBeaver 工作区，可在下方手动选择 DBeaver 导出的 `.dbp` / `data-sources.json` 文件或 `.dbeaver` 目录。
        </div>
      </div>

      <!-- Manual File / Folder Path Import -->
      <div class="form-field" style="margin-bottom: 12px">
        <label>{{ t('importPathLabel') }}</label>
        <div style="display: flex; gap: 6px; flex-wrap: wrap">
          <input
            v-model="importPath"
            class="field"
            style="flex: 1; min-width: 240px"
            placeholder="例如：C:\Users\...\DBeaverData\workspace6\General\.dbeaver\data-sources.json 或 export.dbp"
          />
          <button class="btn" @click="browseDbeaverFile">{{ t('importPickFile') }}</button>
          <button class="btn" @click="browseDbeaverFolder">{{ t('importPickFolder') }}</button>
          <button
            class="btn btn-primary"
            :disabled="busy || !importPath.trim()"
            @click="emit('import-dbeaver-path', importPath.trim(), includeDbeaverPasswords)"
          >
            导入路径
          </button>
        </div>
      </div>

      <!-- Paste JSON / Legacy XML -->
      <div class="form-field" style="margin-bottom: 12px">
        <label>{{ t('importTextLabel') }}</label>
        <textarea
          v-model="importText"
          class="field"
          rows="5"
          style="font-family: var(--font-mono); resize: vertical"
          placeholder='{"connections": {"tidb-1": {"provider": "mysql", "driver": "tidb", "name": "policy-UAT", "configuration": {"url": "jdbc:mysql://9.20.160.17:4000", "user": "policy_d01_uat"}}}}'
        />
        <div style="display: flex; justify-content: flex-end; gap: 8px; margin-top: 6px">
          <button
            class="btn btn-primary"
            :disabled="busy || !importText.trim()"
            @click="emit('import-dbeaver-text', importText)"
          >
            {{ t('importRunBtn') }}
          </button>
        </div>
      </div>

      <!-- Export as DBeaver data-sources.json -->
      <div style="border-top: 1px solid var(--border); padding-top: 12px">
        <div
          style="
            display: flex;
            align-items: center;
            justify-content: space-between;
            margin-bottom: 8px;
          "
        >
          <span style="font-size: 12px; font-weight: 600">
            {{ t('exportDbeaver') }}（导出为标准 data-sources.json，不含明文密码）
          </span>
          <div style="display: flex; gap: 6px">
            <button class="btn btn-sm" :disabled="busy" @click="emit('export-dbeaver-json')">
              {{ t('exportGenBtn') }}
            </button>
            <button
              v-if="exportedJson"
              class="btn btn-sm btn-primary"
              @click="copyExportedJson"
            >
              {{ t('copyJson') }}
            </button>
          </div>
        </div>
        <pre v-if="exportedJson" class="code-viewer" style="max-height: 170px">{{ exportedJson }}</pre>
      </div>
    </div>
  </div>

  <!-- 3. File Encoding & Line Separator Modal -->
  <div
    v-else-if="activeModal === 'encoding' && encodingContext"
    class="modal-backdrop"
    @click.self="emit('close')"
  >
    <div class="modal-card" style="width: min(560px, 92vw)">
      <div class="modal-header">
        <span>{{ t('encodingModalTitle') }}</span>
        <button class="icon-btn compact" @click="emit('close')">×</button>
      </div>

      <p style="margin: 0 0 12px; font-size: 12px; color: var(--text-secondary)">
        {{ t('encodingDesc') }}
      </p>

      <div
        style="
          padding: 9px 12px;
          background: var(--bg-editor);
          border: 1px solid var(--border);
          border-radius: 6px;
          margin-bottom: 12px;
          font-size: 12px;
          display: flex;
          flex-direction: column;
          gap: 4px;
        "
      >
        <div>
          <strong>文件：</strong>
          <span style="font-family: var(--font-mono)">
            {{ encodingContext.path || encodingContext.title }}
          </span>
        </div>
        <div style="color: var(--text-secondary)">
          当前状态：{{ encodingContext.encoding }}
          · {{ encodingContext.lineEnding }}
          · 共 {{ encodingContext.lineCount }} 行
          · {{ formatBytes(encodingContext.byteSize) }}
        </div>
      </div>

      <div class="form-grid" style="margin-bottom: 14px">
        <div class="form-field" style="grid-column: 1 / -1">
          <label>目标字符编码</label>
          <select v-model="selectedEncoding" class="field">
            <option v-for="enc in ENCODING_OPTIONS" :key="enc.value" :value="enc.value">
              {{ enc.label }}
            </option>
          </select>
        </div>

        <div class="form-field" style="grid-column: 1 / -1">
          <label>行尾换行符 (Line Separator)</label>
          <div style="display: flex; gap: 8px">
            <button
              class="btn"
              :class="{ 'btn-primary': selectedLineEnding === 'LF' }"
              @click="selectedLineEnding = 'LF'"
            >
              LF (\n · Unix / macOS / Git 推荐)
            </button>
            <button
              class="btn"
              :class="{ 'btn-primary': selectedLineEnding === 'CRLF' }"
              @click="selectedLineEnding = 'CRLF'"
            >
              CRLF (\r\n · Windows)
            </button>
          </div>
        </div>
      </div>

      <div style="display: flex; justify-content: space-between; gap: 8px; flex-wrap: wrap">
        <button
          class="btn"
          :disabled="!encodingContext.path || busy"
          :title="encodingContext.path ? '' : '仅磁盘文件支持重新解码'"
          @click="emit('reload-encoding', selectedEncoding)"
        >
          {{ t('encodingReload') }}
        </button>
        <div style="display: flex; gap: 8px">
          <button class="btn" @click="emit('close')">{{ t('btnCancel') }}</button>
          <button class="btn btn-primary" @click="triggerConvertEncoding">
            {{ t('encodingConvert') }}
          </button>
        </div>
      </div>
    </div>
  </div>

  <!-- 4. High-Risk SQL Safety Guard Modal -->
  <div
    v-else-if="activeModal === 'danger' && dangerContext"
    class="modal-backdrop"
    @click.self="emit('close')"
  >
    <div class="modal-card" style="width: min(620px, 92vw)">
      <div class="modal-header" style="color: var(--prod)">
        <span>⚠ {{ t('dangerTitle') }}</span>
        <button class="icon-btn compact" @click="emit('close')">×</button>
      </div>

      <p style="margin: 0 0 10px; font-size: 12px">{{ t('dangerDesc') }}</p>

      <div style="display: flex; align-items: center; gap: 8px; margin-bottom: 10px">
        <span
          class="pill"
          :class="
            dangerContext.env === 'PROD'
              ? 'pill-prod'
              : dangerContext.env === 'TEST'
                ? 'pill-test'
                : 'pill-dev'
          "
        >
          {{ dangerContext.env }}
        </span>
        <strong>{{ dangerContext.profileName }}</strong>
        <span v-if="dangerContext.dbName" style="color: var(--text-secondary)">
          / {{ dangerContext.dbName }}
        </span>
      </div>

      <div
        v-for="(w, idx) in dangerContext.warnings"
        :key="idx"
        style="
          padding: 7px 10px;
          margin-bottom: 6px;
          border-radius: 5px;
          background: var(--prod-bg);
          border: 1px solid rgba(247, 84, 100, 0.35);
          font-size: 12px;
        "
      >
        <span class="pill pill-prod" style="margin-right: 6px">{{ w.level }}</span>
        <strong>{{ w.code }}</strong> — {{ w.message }}
      </div>

      <pre class="code-viewer" style="max-height: 200px; margin: 10px 0">{{ dangerContext.sql }}</pre>

      <div style="display: flex; justify-content: flex-end; gap: 8px; margin-top: 12px">
        <button class="btn" @click="emit('close')">{{ t('btnCancel') }}</button>
        <button class="btn btn-danger" @click="emit('confirm-danger')">
          {{ t('dangerConfirm') }}
        </button>
      </div>
    </div>
  </div>

  <!-- 5. Search Everywhere (Ctrl+K) Modal -->
  <div
    v-else-if="activeModal === 'search'"
    class="modal-backdrop"
    style="align-items: flex-start; padding-top: 12vh"
    @click.self="emit('close')"
  >
    <div class="search-modal">
      <div class="search-input-bar">
        <span style="color: var(--text-secondary)">⌘</span>
        <input
          ref="searchInputRef"
          v-model="searchQuery"
          placeholder="搜索数据源、数据表、打开的标签页、Markdown SQL 块或快捷操作..."
          @keydown="onSearchKeydown"
        />
        <span class="shortcut-hint">ESC</span>
      </div>
      <div class="search-results">
        <div
          v-for="(item, idx) in filteredSearchItems"
          :key="item.id"
          class="search-row"
          :class="{ active: idx === searchActiveIdx }"
          @mouseenter="searchActiveIdx = idx"
          @click="emit('select-search-item', item)"
        >
          <div style="min-width: 0">
            <div style="font-size: 12.5px; font-weight: 500">{{ item.label }}</div>
            <div style="font-size: 11px; opacity: 0.72">{{ item.detail }}</div>
          </div>
          <span class="pill pill-neutral">{{ item.badge }}</span>
        </div>
        <div v-if="filteredSearchItems.length === 0" class="empty-state">
          未找到匹配项，请尝试其他关键字。
        </div>
      </div>
    </div>
  </div>

  <!-- 6. Session Password Prompt Modal -->
  <div
    v-else-if="activeModal === 'password' && passwordProfile"
    class="modal-backdrop"
    @click.self="emit('close')"
  >
    <div class="modal-card" style="width: min(440px, 90vw)">
      <div class="modal-header">
        <span style="display: inline-flex; align-items: center; gap: 6px">
          <IdeIcon name="key" :size="15" /> {{ t('passwordPromptTitle') }}
        </span>
        <button class="icon-btn compact" @click="emit('close')">×</button>
      </div>

      <p style="margin: 0 0 8px; font-size: 12px">
        连接目标：<strong>{{ passwordProfile.name }}</strong>
      </p>
      <p style="margin: 0 0 12px; font-size: 11px; color: var(--text-secondary)">
        {{ t('passwordPromptHint') }}
      </p>

      <input
        ref="passwordInputRef"
        v-model="sessionPassword"
        type="password"
        class="field"
        placeholder="请输入数据库访问密码..."
        @keydown.enter="submitPasswordPrompt"
      />

      <div style="display: flex; justify-content: flex-end; gap: 8px; margin-top: 14px">
        <button class="btn" @click="emit('close')">{{ t('btnCancel') }}</button>
        <button class="btn btn-primary" :disabled="busy" @click="submitPasswordPrompt">
          {{ t('btnConnect') }}
        </button>
      </div>
    </div>
  </div>

  <!-- 7. Interactive Keymap Configuration Modal -->
  <div
    v-else-if="activeModal === 'shortcuts'"
    class="modal-backdrop"
    @click.self="
      cancelRecordingKey();
      emit('close');
    "
  >
    <div class="modal-card" style="width: min(760px, 94vw)">
      <div class="modal-header">
        <span style="display: inline-flex; align-items: center; gap: 7px">
          <IdeIcon name="keyboard" :size="16" />
          <span>快捷键配置 (Keymap) · 点击快捷键即可自定义录制</span>
        </span>
        <button
          class="icon-btn compact"
          @click="
            cancelRecordingKey();
            emit('close');
          "
        >
          ×
        </button>
      </div>

      <!-- Preset & Search Controls -->
      <div
        style="
          display: flex;
          align-items: center;
          justify-content: space-between;
          gap: 8px;
          flex-wrap: wrap;
          margin-bottom: 10px;
        "
      >
        <div style="display: flex; align-items: center; gap: 6px; flex-wrap: wrap">
          <input
            v-model="keymapSearch"
            class="field"
            style="width: 210px; padding: 4px 8px"
            placeholder="搜索功能名称或快捷键..."
          />
          <button
            class="btn btn-sm"
            :class="{ 'btn-primary': userKeymap.activePreset.value === 'idea' }"
            @click="
              userKeymap.applyPreset('idea');
              emit('notify', '已切换为 JetBrains IDEA 默认快捷键方案');
            "
          >
            JetBrains IDEA 默认
          </button>
          <button
            class="btn btn-sm"
            :class="{ 'btn-primary': userKeymap.activePreset.value === 'vscode' }"
            @click="
              userKeymap.applyPreset('vscode');
              emit('notify', '已切换为 VS Code / DBeaver 快捷方案');
            "
          >
            VS Code / DBeaver
          </button>
        </div>

        <div style="display: flex; align-items: center; gap: 10px">
          <label
            style="
              display: inline-flex;
              align-items: center;
              gap: 5px;
              font-size: 11.5px;
              color: var(--text-secondary);
              cursor: pointer;
            "
          >
            <input
              type="checkbox"
              :checked="userKeymap.doubleShiftSearch.value"
              @change="userKeymap.setDoubleShiftSearch(($event.target as HTMLInputElement).checked)"
            />
            <span>双击 Shift 随处搜索</span>
          </label>
          <button
            class="btn btn-sm"
            @click="
              userKeymap.resetAllBindings();
              emit('notify', '已恢复全部默认快捷键');
            "
          >
            恢复默认
          </button>
        </div>
      </div>

      <!-- Category Filter Tabs -->
      <div style="display: flex; gap: 5px; flex-wrap: wrap; margin-bottom: 10px">
        <button
          class="icon-btn compact"
          :class="{ active: keymapCategory === 'ALL' }"
          @click="keymapCategory = 'ALL'"
        >
          全部 ({{ SHORTCUT_ACTIONS.length }})
        </button>
        <button
          v-for="(meta, catKey) in SHORTCUT_CATEGORY_LABELS"
          :key="catKey"
          class="icon-btn compact"
          :class="{ active: keymapCategory === catKey }"
          @click="keymapCategory = catKey"
        >
          {{ locale === 'zh-CN' ? meta.zh : meta.en }}
        </button>
      </div>

      <!-- Conflict Prompt Banner -->
      <div
        v-if="keymapConflict"
        style="
          display: flex;
          align-items: center;
          justify-content: space-between;
          gap: 8px;
          padding: 8px 10px;
          margin-bottom: 10px;
          border-radius: 6px;
          background: var(--test-bg);
          border: 1px solid rgba(224, 168, 46, 0.45);
          font-size: 12px;
        "
      >
        <span>
          快捷键 <strong style="font-family: var(--font-mono)">{{ formatComboDisplay(keymapConflict.combo) }}</strong>
          已被「<strong>{{ keymapConflict.conflictWith.labelZh }}</strong>」占用，是否覆盖？
        </span>
        <div style="display: flex; gap: 6px; flex-shrink: 0">
          <button class="btn btn-sm" @click="keymapConflict = null">取消</button>
          <button class="btn btn-sm btn-primary" @click="applyConflictOverwrite">
            覆盖并清空原绑定
          </button>
        </div>
      </div>

      <!-- Action List Table -->
      <div style="max-height: 360px; overflow-y: auto; border: 1px solid var(--border); border-radius: 6px">
        <table class="dg-grid" style="border: none">
          <thead>
            <tr>
              <th>功能说明</th>
              <th style="width: 105px">分组</th>
              <th style="width: 205px">当前快捷键</th>
              <th style="width: 120px">操作</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="act in filteredKeymapActions" :key="act.id">
              <td style="white-space: normal; font-family: var(--font-ui)">
                {{ locale === 'zh-CN' ? act.labelZh : act.labelEn }}
              </td>
              <td>
                <span class="pill pill-neutral">
                  {{ locale === 'zh-CN' ? SHORTCUT_CATEGORY_LABELS[act.category].zh : SHORTCUT_CATEGORY_LABELS[act.category].en }}
                </span>
              </td>
              <td>
                <span v-if="recordingActionId === act.id" class="keymap-record-box">
                  请按下新组合键 (Esc 取消 / Backspace 清空)...
                </span>
                <button
                  v-else
                  class="pill pill-neutral"
                  style="
                    font-family: var(--font-mono);
                    cursor: pointer;
                    border: 1px solid var(--border);
                  "
                  title="点击录制新快捷键"
                  @click="startRecordingKey(act.id)"
                >
                  {{ userKeymap.getDisplay(act.id) }}
                </button>
              </td>
              <td>
                <div style="display: flex; align-items: center; gap: 4px">
                  <button
                    v-if="recordingActionId === act.id"
                    class="btn btn-sm"
                    @click="cancelRecordingKey"
                  >
                    取消
                  </button>
                  <button
                    v-else
                    class="btn btn-sm"
                    @click="startRecordingKey(act.id)"
                  >
                    修改
                  </button>
                  <button
                    v-if="userKeymap.getCombo(act.id) !== act.defaultCombo"
                    class="btn btn-sm"
                    title="恢复此项默认快捷键"
                    @click="userKeymap.resetBinding(act.id)"
                  >
                    重置
                  </button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <div
        style="
          display: flex;
          align-items: center;
          justify-content: space-between;
          margin-top: 12px;
          font-size: 11px;
          color: var(--text-secondary);
        "
      >
        <span>提示：点击任意快捷键徽章后直接按组合键即可生效（如 Ctrl+Q、F5、Alt+L），修改立即自动保存。</span>
        <button
          class="btn btn-primary"
          @click="
            cancelRecordingKey();
            emit('close');
          "
        >
          {{ t('btnClose') }}
        </button>
      </div>
    </div>
  </div>

  <!-- 8. Project Create / Edit Modal -->
  <div
    v-else-if="activeModal === 'project'"
    class="modal-backdrop"
    @click.self="emit('close')"
  >
    <div class="modal-card" style="width: min(520px, 92vw)">
      <div class="modal-header">
        <span style="display: inline-flex; align-items: center; gap: 7px">
          <IdeIcon
            :name="editingProject ? 'project-open' : 'project-add'"
            :size="16"
            :color="projectForm.color"
          />
          {{ editingProject ? t('projectModalEdit') : t('projectModalCreate') }}
        </span>
        <button class="icon-btn compact" @click="emit('close')">×</button>
      </div>

      <div class="conn-rows">
        <div class="conn-row">
          <label class="conn-row-label">{{ t('projectName') }}:</label>
          <input
            v-model="projectForm.name"
            class="field"
            placeholder="例如：保险核心 UAT / 订单微服务集群"
            @keydown.enter="submitProjectForm"
          />
        </div>

        <div class="conn-row">
          <label class="conn-row-label">{{ t('projectColor') }}:</label>
          <div style="display: flex; align-items: center; gap: 8px">
            <button
              v-for="clr in PROJECT_COLORS"
              :key="clr"
              type="button"
              style="
                width: 22px;
                height: 22px;
                border-radius: 5px;
                border: 2px solid transparent;
                cursor: pointer;
              "
              :style="{
                background: clr,
                borderColor: projectForm.color === clr ? 'var(--text-primary)' : 'transparent',
              }"
              @click="projectForm.color = clr"
            />
          </div>
        </div>

        <div class="conn-row">
          <label class="conn-row-label">{{ t('projectDesc') }}:</label>
          <input
            v-model="projectForm.description"
            class="field"
            placeholder="可选，说明该项目包含的业务库与环境"
          />
        </div>

        <div class="conn-row">
          <label class="conn-row-label">绑定目录:</label>
          <div style="display: flex; gap: 6px">
            <input
              v-model="projectForm.rootDir"
              class="field"
              placeholder="可选，切换项目时自动挂载该目录的 .md / .sql 文件"
            />
            <button class="btn btn-sm" @click="browseProjectRootDir">选择...</button>
          </div>
        </div>
      </div>

      <div
        style="
          display: flex;
          align-items: center;
          justify-content: space-between;
          margin-top: 16px;
          padding-top: 12px;
          border-top: 1px solid var(--border);
        "
      >
        <button
          v-if="editingProject && projects.length > 1"
          class="btn btn-danger"
          :disabled="busy"
          @click="emit('delete-project', editingProject.id)"
        >
          {{ t('deleteProject') }}
        </button>
        <span v-else />
        <div style="display: flex; gap: 8px">
          <button class="btn" @click="emit('close')">{{ t('btnCancel') }}</button>
          <button class="btn btn-primary" :disabled="busy || !projectForm.name.trim()" @click="submitProjectForm">
            {{ t('btnSave') }}
          </button>
        </div>
      </div>
    </div>
  </div>

  <!-- 9. Editable Result Grid -> Generated SQL Preview & Confirm Execute Modal -->
  <div
    v-else-if="activeModal === 'grid-sql' && gridSqlContext"
    class="modal-backdrop"
    @click.self="emit('close')"
  >
    <div class="modal-card" style="width: min(740px, 94vw)">
      <div class="modal-header">
        <span style="display: inline-flex; align-items: center; gap: 6px">
          <IdeIcon name="bolt" :size="15" /> 预览并执行数据修改 SQL
        </span>
        <button class="icon-btn compact" @click="emit('close')">×</button>
      </div>

      <div style="display: flex; align-items: center; gap: 8px; flex-wrap: wrap; margin-bottom: 10px">
        <span
          class="pill"
          :class="
            gridSqlContext.profileEnv === 'PROD'
              ? 'pill-prod'
              : gridSqlContext.profileEnv === 'TEST'
                ? 'pill-test'
                : 'pill-dev'
          "
        >
          {{ gridSqlContext.profileEnv }}
        </span>
        <strong>{{ gridSqlContext.profileName }}</strong>
        <span style="color: var(--text-secondary)">
          / {{ gridSqlContext.dbName || 'default' }} · 表
          <code style="font-family: var(--font-mono)">{{ gridSqlContext.tableName }}</code>
        </span>
        <span v-if="gridSqlContext.plan.usedPrimaryKey" class="pill pill-dev">
          主键条件: {{ gridSqlContext.plan.keyColumns.join(', ') }}
        </span>
        <span v-else class="pill pill-test">
          无主键（全列精确匹配）
        </span>
      </div>

      <div
        v-if="gridSqlContext.profileEnv === 'PROD'"
        style="
          padding: 8px 10px;
          margin-bottom: 10px;
          border-radius: 6px;
          background: var(--prod-bg);
          border: 1px solid rgba(247, 84, 100, 0.45);
          color: var(--prod);
          font-size: 12px;
        "
      >
        ⚠ 当前目标属于 <strong>PROD 生产环境</strong>，请仔细确认以下生成的 SQL 与 WHERE 条件无误后再点击「确认执行」！
      </div>

      <div
        v-if="gridSqlContext.plan.warning"
        style="
          padding: 7px 10px;
          margin-bottom: 10px;
          border-radius: 6px;
          background: var(--test-bg);
          border: 1px solid rgba(224, 168, 46, 0.4);
          color: var(--test);
          font-size: 11.5px;
        "
      >
        {{ gridSqlContext.plan.warning }}
      </div>

      <div class="form-field">
        <label>以下为根据表格修改自动生成的 SQL（支持执行前手工微调，按分号逐条执行）：</label>
        <textarea
          v-model="editableGridSql"
          class="field"
          rows="9"
          style="font-family: var(--font-mono); font-size: 12px; line-height: 1.55; resize: vertical"
        />
      </div>

      <div
        style="
          display: flex;
          align-items: center;
          justify-content: space-between;
          margin-top: 14px;
          padding-top: 10px;
          border-top: 1px solid var(--border);
        "
      >
        <button class="btn" @click="copyGridSqlText">
          <IdeIcon name="copy" :size="13" /> 复制 SQL
        </button>
        <div style="display: flex; gap: 8px">
          <button class="btn" @click="emit('close')">{{ t('btnCancel') }}</button>
          <button
            class="btn"
            :class="gridSqlContext.profileEnv === 'PROD' ? 'btn-danger' : 'btn-primary'"
            :disabled="busy || !editableGridSql.trim()"
            @click="confirmExecuteGridSql"
          >
            <IdeIcon name="run" :size="12" /> 确认执行并刷新结果集
          </button>
        </div>
      </div>
    </div>
  </div>

  <!-- 10. Unsaved Disk Document Close Confirmation Modal -->
  <div
    v-else-if="activeModal === 'unsaved' && unsavedContext"
    class="modal-backdrop"
    @click.self="emit('close')"
  >
    <div class="modal-card" style="width: min(460px, 92vw)">
      <div class="modal-header" style="color: var(--test)">
        <span style="display: inline-flex; align-items: center; gap: 6px">
          <IdeIcon name="warning" :size="16" />
          <span>保存文件更改？</span>
        </span>
        <button class="icon-btn compact" @click="emit('close')">×</button>
      </div>

      <p style="margin: 0 0 8px; font-size: 12.5px">
        文档 <strong>{{ unsavedContext.title }}</strong> 包含尚未保存的修改，关闭前是否保存？
      </p>
      <p
        v-if="unsavedContext.path"
        style="
          margin: 0 0 14px;
          font-size: 11px;
          color: var(--text-secondary);
          font-family: var(--font-mono);
          word-break: break-all;
        "
      >
        {{ unsavedContext.path }}
      </p>

      <div style="display: flex; justify-content: flex-end; gap: 8px; margin-top: 12px">
        <button class="btn" @click="emit('close')">{{ t('btnCancel') }}</button>
        <button class="btn btn-danger" @click="emit('unsaved-discard-close')">
          不保存并关闭
        </button>
        <button class="btn btn-primary" :disabled="busy" @click="emit('unsaved-save-close')">
          保存并关闭
        </button>
      </div>
    </div>
  </div>
</template>
