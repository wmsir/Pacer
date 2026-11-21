<script setup lang="ts">
import { computed, nextTick, reactive, ref, watch } from 'vue';
import { useI18n } from '../i18n';
import {
  pickDataGripImportFile,
  type ConnectionDraft,
  type ConnectionProfile,
  type DirectConfig,
  type EnvLevel,
  type RiskLevel,
} from '../api/ipc';
import { formatBytes } from '../utils/sqlTools';

export type ActiveModalKind =
  | 'none'
  | 'connection'
  | 'import'
  | 'encoding'
  | 'danger'
  | 'search'
  | 'password'
  | 'shortcuts';

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

const props = defineProps<{
  activeModal: ActiveModalKind;
  editingProfile: ConnectionProfile | null;
  encodingContext: EncodingModalContext | null;
  dangerContext: DangerModalContext | null;
  passwordProfile: ConnectionProfile | null;
  searchItems: SearchEverywhereItem[];
  exportedXml: string;
  busy: boolean;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
  (e: 'save-connection', draft: ConnectionDraft, password: string | null, testFirst: boolean): void;
  (e: 'import-path', path: string): void;
  (e: 'import-xml', xml: string): void;
  (e: 'export-xml'): void;
  (e: 'reload-encoding', encoding: string): void;
  (e: 'convert-encoding', encoding: string, hasBom: boolean, lineEnding: 'CRLF' | 'LF'): void;
  (e: 'confirm-danger'): void;
  (e: 'submit-password', profileId: string, password: string): void;
  (e: 'select-search-item', item: SearchEverywhereItem): void;
  (e: 'notify', message: string): void;
}>();

const { t, locale } = useI18n();

const ENGINE_OPTIONS = [
  { value: 'sqlite', label: 'SQLite (本地文件 / 嵌入式)', defaultPort: null },
  { value: 'mySql', label: 'MySQL', defaultPort: 3306 },
  { value: 'postgreSql', label: 'PostgreSQL', defaultPort: 5432 },
  { value: 'mariaDb', label: 'MariaDB', defaultPort: 3306 },
  { value: 'tiDb', label: 'TiDB (MySQL 协议)', defaultPort: 4000 },
  { value: 'oceanBase', label: 'OceanBase (MySQL 模式)', defaultPort: 2881 },
  { value: 'oracle', label: 'Oracle (元数据配置)', defaultPort: 1521 },
  { value: 'dameng', label: '达梦 Dameng DM8 (元数据配置)', defaultPort: 5236 },
  { value: 'kingbase', label: '人大金仓 KingbaseES (PG 协议)', defaultPort: 54321 },
  { value: 'openGauss', label: 'openGauss (PG 协议)', defaultPort: 5432 },
  { value: 'sqlServer', label: 'SQL Server (元数据配置)', defaultPort: 1433 },
  { value: 'clickHouse', label: 'ClickHouse (元数据配置)', defaultPort: 8123 },
];

const ENCODING_OPTIONS = [
  { value: 'UTF-8', label: 'UTF-8（推荐 · 无 BOM）', hasBom: false },
  { value: 'UTF-8 BOM', label: 'UTF-8 with BOM（兼容 Excel / Windows 记事本）', hasBom: true },
  { value: 'GBK', label: 'GBK / GB18030（中文旧版 SQL / Windows 经典编码）', hasBom: false },
  { value: 'UTF-16 LE', label: 'UTF-16 LE（Windows Unicode 导出常见）', hasBom: true },
  { value: 'UTF-16 BE', label: 'UTF-16 BE', hasBom: true },
  { value: 'Windows-1252', label: 'Windows-1252 / Latin-1', hasBom: false },
];

// Connection Form State
const connForm = reactive<{
  id: string | null;
  name: string;
  dbType: string;
  host: string;
  port: string;
  database: string;
  jdbcUrl: string;
  username: string;
  password: string;
  env: EnvLevel;
  readOnly: boolean;
  comment: string;
}>({
  id: null,
  name: '',
  dbType: 'mySql',
  host: '127.0.0.1',
  port: '3306',
  database: '',
  jdbcUrl: '',
  username: 'root',
  password: '',
  env: 'DEV',
  readOnly: false,
  comment: '',
});

const isSqliteEngine = computed(() => connForm.dbType === 'sqlite');

function normalizeDbTypeString(dbType: string | Record<string, string>): string {
  if (typeof dbType === 'string') return dbType;
  return Object.values(dbType)[0] ?? 'mySql';
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
    if (!connForm.database) {
      connForm.database = 'C:/data/sample.db';
    }
  } else if (!connForm.host) {
    connForm.host = '127.0.0.1';
  }
}

watch(
  () => [props.activeModal, props.editingProfile] as const,
  ([modal, prof]) => {
    if (modal !== 'connection') return;
    if (prof) {
      const cfg = (prof.mode?.config ?? {}) as Partial<DirectConfig>;
      connForm.id = prof.id;
      connForm.name = prof.name;
      connForm.dbType = normalizeDbTypeString(prof.dbType);
      connForm.host = cfg.host ?? prof.fingerprint.host ?? '';
      connForm.port =
        cfg.port !== null && cfg.port !== undefined
          ? String(cfg.port)
          : prof.fingerprint.port
            ? String(prof.fingerprint.port)
            : '';
      connForm.database = cfg.database ?? prof.fingerprint.database ?? '';
      connForm.jdbcUrl = cfg.jdbcUrl ?? '';
      connForm.username = cfg.username ?? '';
      connForm.password = '';
      connForm.env = prof.env ?? 'DEV';
      connForm.readOnly = Boolean(cfg.readOnly);
      connForm.comment = prof.comment ?? '';
    } else {
      connForm.id = null;
      connForm.name = '';
      connForm.dbType = 'mySql';
      connForm.host = '127.0.0.1';
      connForm.port = '3306';
      connForm.database = '';
      connForm.jdbcUrl = '';
      connForm.username = 'root';
      connForm.password = '';
      connForm.env = 'DEV';
      connForm.readOnly = false;
      connForm.comment = '';
    }
  },
);

function submitConnection(testFirst: boolean) {
  const name =
    connForm.name.trim() ||
    (isSqliteEngine.value ? 'SQLite 本地库' : `${connForm.dbType}@${connForm.host || 'localhost'}`);
  const parsedPort = connForm.port.trim() ? Number(connForm.port.trim()) : null;
  const draft: ConnectionDraft = {
    id: connForm.id,
    name,
    dbType: connForm.dbType,
    host: isSqliteEngine.value ? null : connForm.host.trim() || null,
    port: isSqliteEngine.value || !parsedPort || Number.isNaN(parsedPort) ? null : parsedPort,
    database: connForm.database.trim() || null,
    jdbcUrl: connForm.jdbcUrl.trim() || null,
    username: isSqliteEngine.value ? null : connForm.username.trim() || null,
    env: connForm.env,
    readOnly: connForm.readOnly,
    comment: connForm.comment.trim() || null,
  };
  const pwd = connForm.password.length > 0 ? connForm.password : null;
  emit('save-connection', draft, pwd, testFirst);
}

// Import / Export Modal State
const importPath = ref('');
const importXml = ref('');

async function browseImportFile() {
  try {
    const picked = await pickDataGripImportFile();
    if (picked) {
      importPath.value = picked;
    }
  } catch (err) {
    emit('notify', String(err));
  }
}

async function copyExportedXml() {
  if (!props.exportedXml) return;
  try {
    await navigator.clipboard.writeText(props.exportedXml);
    emit('notify', t('copiedNotice'));
  } catch {
    emit('notify', '剪贴板写入失败');
  }
}

// Encoding Modal State
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

// Password Prompt State
const sessionPassword = ref('');
const passwordInputRef = ref<HTMLInputElement | null>(null);

watch(
  () => props.activeModal,
  async (modal) => {
    if (modal === 'password') {
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

// Search Everywhere State
const searchQuery = ref('');
const searchActiveIdx = ref(0);
const searchInputRef = ref<HTMLInputElement | null>(null);

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

const shortcutRows = computed(() =>
  locale.value === 'zh-CN'
    ? [
        { key: 'Ctrl + Enter', desc: '执行选中 SQL；无选区时执行光标所在 SQL 语句或 Markdown SQL 块' },
        { key: 'Ctrl + Shift + Enter', desc: '顺序执行当前脚本的全部语句 / 当前 Markdown 的全部 SQL 块' },
        { key: 'Ctrl + Alt + L', desc: '一键格式化当前 SQL 语句' },
        { key: 'Ctrl + O', desc: '打开本地 .md / .sql 文件（自动检测编码与行尾符）' },
        { key: 'Ctrl + S', desc: '按选定编码与换行符保存当前文件' },
        { key: 'Ctrl + N', desc: '新建查询控制台标签页' },
        { key: 'Ctrl + K', desc: '打开随处搜索（快速跳转命令 / 数据源 / 表 / Markdown SQL 块）' },
        { key: 'Ctrl + F', desc: '在编辑器内查找与替换' },
      ]
    : [
        { key: 'Ctrl + Enter', desc: 'Execute selected SQL, enclosing Markdown SQL fence, or statement at cursor' },
        { key: 'Ctrl + Shift + Enter', desc: 'Execute all statements or all SQL blocks in Markdown' },
        { key: 'Ctrl + Alt + L', desc: 'Reformat SQL' },
        { key: 'Ctrl + O', desc: 'Open local .md / .sql file (auto-detects encoding & EOL)' },
        { key: 'Ctrl + S', desc: 'Save file with chosen encoding & line separator' },
        { key: 'Ctrl + N', desc: 'New Query Console tab' },
        { key: 'Ctrl + K', desc: 'Search Everywhere (actions, data sources, tables, MD SQL blocks)' },
        { key: 'Ctrl + F', desc: 'Find & Replace in editor' },
      ],
);
</script>

<template>
  <!-- 1. New / Edit Data Source Modal -->
  <div
    v-if="activeModal === 'connection'"
    class="modal-backdrop"
    @click.self="emit('close')"
  >
    <div class="modal-card">
      <div class="modal-header">
        <span>{{ editingProfile ? t('connModalEdit') : t('connModalCreate') }}</span>
        <button class="icon-btn compact" @click="emit('close')">×</button>
      </div>

      <div class="form-grid">
        <div class="form-field">
          <label>{{ t('connName') }}</label>
          <input
            v-model="connForm.name"
            class="field"
            placeholder="例如：生产订单库 MySQL / 本地演示库"
          />
        </div>

        <div class="form-field">
          <label>{{ t('connEngine') }}</label>
          <select v-model="connForm.dbType" class="field" @change="onEngineChange">
            <option v-for="opt in ENGINE_OPTIONS" :key="opt.value" :value="opt.value">
              {{ opt.label }}
            </option>
          </select>
        </div>

        <template v-if="isSqliteEngine">
          <div class="form-field" style="grid-column: 1 / -1">
            <label>{{ t('connSqlitePath') }}</label>
            <input
              v-model="connForm.database"
              class="field"
              placeholder="例如：C:/data/app.db 或 :memory:"
            />
          </div>
        </template>

        <template v-else>
          <div class="form-field">
            <label>{{ t('connHost') }}</label>
            <input v-model="connForm.host" class="field" placeholder="127.0.0.1" />
          </div>

          <div class="form-field">
            <label>{{ t('connPort') }}</label>
            <input v-model="connForm.port" class="field" placeholder="3306" />
          </div>

          <div class="form-field">
            <label>{{ t('connDb') }}</label>
            <input v-model="connForm.database" class="field" placeholder="可留空连接实例" />
          </div>

          <div class="form-field">
            <label>{{ t('connUser') }}</label>
            <input v-model="connForm.username" class="field" placeholder="root / postgres" />
          </div>

          <div class="form-field" style="grid-column: 1 / -1">
            <label>{{ t('connPassword') }}</label>
            <input
              v-model="connForm.password"
              type="password"
              class="field"
              placeholder="编辑已有连接时留空则保留当前会话缓存密码"
            />
          </div>
        </template>

        <div class="form-field">
          <label>{{ t('connEnv') }}</label>
          <select v-model="connForm.env" class="field">
            <option value="DEV">DEV（开发环境 · 绿色）</option>
            <option value="TEST">TEST（测试环境 · 琥珀色）</option>
            <option value="PROD">PROD（生产环境 · 红色高危保护）</option>
          </select>
        </div>

        <div class="form-field">
          <label>备注说明</label>
          <input v-model="connForm.comment" class="field" placeholder="可选，将同步至 DataGrip XML" />
        </div>

        <div class="form-field" style="grid-column: 1 / -1">
          <label>{{ t('connJdbcUrl') }}</label>
          <input
            v-model="connForm.jdbcUrl"
            class="field"
            style="font-family: var(--font-mono)"
            placeholder="jdbc:mysql://127.0.0.1:3306/app_db"
          />
        </div>

        <label
          style="
            grid-column: 1 / -1;
            display: flex;
            align-items: center;
            gap: 8px;
            font-size: 12px;
            cursor: pointer;
          "
        >
          <input v-model="connForm.readOnly" type="checkbox" />
          <span>{{ t('connReadOnly') }}</span>
        </label>
      </div>

      <div
        style="
          display: flex;
          justify-content: flex-end;
          gap: 8px;
          margin-top: 16px;
          padding-top: 12px;
          border-top: 1px solid var(--border);
        "
      >
        <button class="btn" @click="emit('close')">{{ t('btnCancel') }}</button>
        <button class="btn" :disabled="busy" @click="submitConnection(false)">
          {{ t('btnSave') }}
        </button>
        <button class="btn btn-primary" :disabled="busy" @click="submitConnection(true)">
          {{ t('connTestAndSave') }}
        </button>
      </div>
    </div>
  </div>

  <!-- 2. DataGrip Import / Export Modal -->
  <div
    v-else-if="activeModal === 'import'"
    class="modal-backdrop"
    @click.self="emit('close')"
  >
    <div class="modal-card">
      <div class="modal-header">
        <span>{{ t('importModalTitle') }}</span>
        <button class="icon-btn compact" @click="emit('close')">×</button>
      </div>

      <div class="form-field" style="margin-bottom: 12px">
        <label>{{ t('importPathLabel') }}</label>
        <div style="display: flex; gap: 6px">
          <input
            v-model="importPath"
            class="field"
            placeholder="例如：C:\Projects\app\.idea\dataSources.xml 或 settings.zip"
          />
          <button class="btn" @click="browseImportFile">{{ t('importPickFile') }}</button>
          <button
            class="btn btn-primary"
            :disabled="busy || !importPath.trim()"
            @click="emit('import-path', importPath.trim())"
          >
            导入路径
          </button>
        </div>
      </div>

      <div class="form-field" style="margin-bottom: 12px">
        <label>{{ t('importXmlLabel') }}</label>
        <textarea
          v-model="importXml"
          class="field"
          rows="6"
          style="font-family: var(--font-mono); resize: vertical"
          placeholder='<data-source name="Prod MySQL" uuid="..."><jdbc-url>jdbc:mysql://127.0.0.1:3306/app</jdbc-url></data-source>'
        />
        <div style="display: flex; justify-content: flex-end; gap: 8px; margin-top: 6px">
          <button
            class="btn btn-primary"
            :disabled="busy || !importXml.trim()"
            @click="emit('import-xml', importXml)"
          >
            {{ t('importRunBtn') }}
          </button>
        </div>
      </div>

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
            {{ t('exportDataGrip') }}（绝不包含明文密码）
          </span>
          <div style="display: flex; gap: 6px">
            <button class="btn btn-sm" :disabled="busy" @click="emit('export-xml')">
              {{ t('exportGenBtn') }}
            </button>
            <button
              v-if="exportedXml"
              class="btn btn-sm btn-primary"
              @click="copyExportedXml"
            >
              {{ t('copyXml') }}
            </button>
          </div>
        </div>
        <pre v-if="exportedXml" class="code-viewer" style="max-height: 180px">{{ exportedXml }}</pre>
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
        <span>🔑 {{ t('passwordPromptTitle') }}</span>
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

  <!-- 7. Keyboard Shortcuts Reference Modal -->
  <div
    v-else-if="activeModal === 'shortcuts'"
    class="modal-backdrop"
    @click.self="emit('close')"
  >
    <div class="modal-card" style="width: min(600px, 92vw)">
      <div class="modal-header">
        <span>⌨ {{ t('shortcutsTitle') }}</span>
        <button class="icon-btn compact" @click="emit('close')">×</button>
      </div>

      <table class="dg-grid" style="border: 1px solid var(--border)">
        <thead>
          <tr>
            <th style="width: 190px">快捷键</th>
            <th>功能说明</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="row in shortcutRows" :key="row.key">
            <td>
              <span class="pill pill-neutral" style="font-family: var(--font-mono)">
                {{ row.key }}
              </span>
            </td>
            <td style="white-space: normal; font-family: var(--font-ui)">{{ row.desc }}</td>
          </tr>
        </tbody>
      </table>

      <div style="display: flex; justify-content: flex-end; margin-top: 12px">
        <button class="btn btn-primary" @click="emit('close')">{{ t('btnClose') }}</button>
      </div>
    </div>
  </div>
</template>
