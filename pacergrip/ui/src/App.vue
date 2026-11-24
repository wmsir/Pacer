<script setup lang="ts">
import { computed, markRaw, onBeforeUnmount, onMounted, reactive, ref, watch } from 'vue';
import { useI18n } from './i18n';
import {
  analyzeSql,
  assignConnectionProject,
  cancelQuery,
  commitTransaction,
  createDemoMarkdownFile,
  createDemoSqliteConnection,
  createSamplePlugin,
  deleteConnection,
  deleteProject,
  executeSql,
  explainSql,
  exportConnectionsXlsx,
  exportDbeaverDbp,
  exportDbeaverFolder,
  exportDbeaverJson,
  exportSqlDump,
  extractStatementAtCursor,
  formatSql,
  getTableDdl,
  getTransactionStatus,
  getWorkspaceInfo,
  importDbeaverPath,
  importDbeaverText,
  isTauriRuntime,
  listColumns,
  listConnections,
  listDatabases,
  listExtensions,
  listForeignKeys,
  listIndexes,
  listLocalPlugins,
  listProjects,
  listQueryHistory,
  listRoutines,
  listTables,
  listWorkspaceFiles,
  openTextFile,
  pickDirectoryToMount,
  pickExportFolderPath,
  pickSaveDbeaverDbpPath,
  pickSaveFilePath,
  pickSaveSqlDumpPath,
  pickSaveXlsxPath,
  pickSqlScriptFile,
  pickTextFileToOpen,
  revealInExplorer,
  rollbackTransaction,
  saveConnection,
  saveConsoleScratch,
  saveProject,
  saveTextFile,
  setTransactionMode,
  testConnection,
  type ColumnMeta,
  type ConnectionDraft,
  type ConnectionProfile,
  type DatabaseMeta,
  type DirectConfig,
  type ExtensionMeta,
  type ForeignKeyMeta,
  type IndexMeta,
  type LocalPluginManifest,
  type MarkdownSqlBlock,
  type OpenedFileDto,
  type ProjectDraft,
  type QueryHistoryEntry,
  type RoutineMeta,
  type ScriptAnalysis,
  type TableMeta,
  type TransactionStatus,
  type WorkspaceFileEntry,
  type WorkspaceInfoDto,
  type WorkspaceProject,
} from './api/ipc';
import {
  BUILTIN_SNIPPETS,
  buildDropTableSql,
  buildExplainPlanSql,
  buildRedisCountCommand,
  buildRedisInspectCommand,
  buildTruncateTableSql,
  convertSelectionToSqlIn,
  countLines,
  escapeSqlAsCodeString,
  estimateUtf8Bytes,
  extractMarkdownSqlBlocksLocal,
  extractRedisCommandAtLine,
  extractSingleTableTarget,
  extractSqlParameters,
  formatBytes,
  formatSqlLiteral,
  generateMockInsertSql,
  getEngineDisplayName,
  getExplainModesForEngine,
  isRedisEngineKey,
  isWriteOrDdlStatement,
  matchesFilterPattern,
  minifySqlText,
  parseMarkdownSegments,
  quoteIdentifier,
  quoteRedisArg,
  splitRedisCommands,
  transformSqlTextCase,
  type ExplainPlanMode,
  type RedisEditableType,
  type SqlSnippet,
} from './utils/sqlTools';
import {
  buildTableCountQuerySql,
  buildTablePageQuerySql,
} from './utils/commercialTools';
import {
  FILE_MANAGER_LABEL,
  IS_MAC,
  formatComboDisplay,
  matchesCombo,
  useKeymap,
} from './utils/keymap';
import { useLiveTemplates, type SqlLiveTemplate } from './utils/liveTemplates';
import CodeEditor, { type EditorCursorTelemetry } from './components/CodeEditor.vue';
import DatabaseTree, {
  type ConnectionHealthInfo,
  type FavoriteTableEntry,
} from './components/DatabaseTree.vue';
import FileExplorer from './components/FileExplorer.vue';
import IdeIcon from './components/IdeIcon.vue';
import ResultGrid, {
  type ConsoleLogEntry,
  type GridMutationPreviewPayload,
  type ResultTabItem,
} from './components/ResultGrid.vue';
import Modals, {
  type ActiveModalKind,
  type DangerModalContext,
  type EncodingModalContext,
  type ParamModalContext,
  type SearchEverywhereItem,
} from './components/Modals.vue';
import TableToolsModals, {
  type ActiveToolModalKind,
  type DbToolContext,
  type TableToolContext,
} from './components/TableToolsModals.vue';
import AdvancedToolsModals, {
  type AdvancedModalKind,
  type RedisEditorInitialData,
} from './components/AdvancedToolsModals.vue';
import ServerMonitorModal from './components/ServerMonitorModal.vue';
import ErDiagramModal from './components/ErDiagramModal.vue';
import CommercialToolsModals, {
  type CommercialModalKind,
} from './components/CommercialToolsModals.vue';

interface EditorTab {
  id: string;
  title: string;
  path: string | null;
  fileKind: string;
  content: string;
  savedContent: string;
  encoding: string;
  hasBom: boolean;
  lineEnding: 'CRLF' | 'LF';
  byteSize: number;
  readOnly: boolean;
  profileId: string;
  dbName: string;
  mdViewMode: 'source' | 'split' | 'preview';
  cursor: EditorCursorTelemetry;
}

const { t, locale, toggleLocale } = useI18n();
const keymap = useKeymap();

// Theme (Darcula Dark default, IntelliJ Light, Mung Bean Eye-Care)
export type AppTheme = 'dark' | 'light' | 'eyecare';

function loadInitialTheme(): AppTheme {
  const saved = localStorage.getItem('pacergrip.theme');
  if (saved === 'light' || saved === 'eyecare') return saved;
  return 'dark';
}

const theme = ref<AppTheme>(loadInitialTheme());

function applyTheme(next: AppTheme) {
  theme.value = next;
  document.documentElement.setAttribute('data-theme', next);
  localStorage.setItem('pacergrip.theme', next);
}

function cycleTheme() {
  const order: AppTheme[] = ['dark', 'light', 'eyecare'];
  const next = order[(order.indexOf(theme.value) + 1) % order.length];
  applyTheme(next);
}

// Layout & Panels (Local Files defaults to RIGHT side, user-configurable Left/Right!)
const filesDockSide = ref<'left' | 'right'>(
  localStorage.getItem('pacer.filesDockSide') === 'left' ? 'left' : 'right',
);
const leftTool = ref<'database' | 'files' | 'snippets' | null>('database');
const rightTool = ref<'files' | 'structure' | 'extensions' | null>(
  filesDockSide.value === 'right' ? 'files' : 'structure',
);
const bottomVisible = ref<boolean>(true);
const leftWidth = ref<number>(Number(localStorage.getItem('pacergrip.leftWidth')) || 276);
const rightWidth = ref<number>(Number(localStorage.getItem('pacergrip.rightWidth')) || 286);
const bottomHeight = ref<number>(Number(localStorage.getItem('pacergrip.bottomHeight')) || 270);
const openMenu = ref<
  'project' | 'file' | 'edit' | 'database' | 'tools' | 'help' | 'theme' | 'explain-plan' | null
>(null);
const showDemoManual = ref<boolean>(localStorage.getItem('pacer.hideDemoManual') !== '1');
const editorFontSize = ref<number>(
  Math.max(11, Math.min(22, Number(localStorage.getItem('pacer.editorFontSize')) || 13)),
);
const editorWordWrap = ref<boolean>(localStorage.getItem('pacer.editorWordWrap') === '1');

function setEditorFontSize(next: number) {
  const clamped = Math.max(11, Math.min(22, next));
  editorFontSize.value = clamped;
  localStorage.setItem('pacer.editorFontSize', String(clamped));
}

function toggleEditorWordWrap() {
  editorWordWrap.value = !editorWordWrap.value;
  localStorage.setItem('pacer.editorWordWrap', editorWordWrap.value ? '1' : '0');
  setStatus(editorWordWrap.value ? '已开启编辑器自动换行' : '已关闭编辑器自动换行');
}

function setShowDemoManual(visible: boolean) {
  showDemoManual.value = visible;
  localStorage.setItem('pacer.hideDemoManual', visible ? '0' : '1');
}

function toggleFilesDockSide() {
  const next: 'left' | 'right' = filesDockSide.value === 'right' ? 'left' : 'right';
  filesDockSide.value = next;
  localStorage.setItem('pacer.filesDockSide', next);
  if (next === 'left') {
    if (rightTool.value === 'files') rightTool.value = 'structure';
    leftTool.value = 'files';
  } else {
    if (leftTool.value === 'files') leftTool.value = 'database';
    rightTool.value = 'files';
  }
  setStatus(`「本地文件与脚本」面板已切换至${next === 'right' ? '右侧' : '左侧'}`);
}

function openFilesPanel() {
  if (filesDockSide.value === 'left') {
    leftTool.value = 'files';
  } else {
    rightTool.value = 'files';
  }
}

function toggleFilesPanel() {
  if (filesDockSide.value === 'left') {
    leftTool.value = leftTool.value === 'files' ? null : 'files';
  } else {
    rightTool.value = rightTool.value === 'files' ? null : 'files';
  }
}

// Resize Dragging
const draggingPane = ref<'left' | 'right' | 'bottom' | null>(null);
let dragStartPos = 0;
let dragStartSize = 0;

function startResize(pane: 'left' | 'right' | 'bottom', e: MouseEvent) {
  e.preventDefault();
  draggingPane.value = pane;
  dragStartPos = pane === 'bottom' ? e.clientY : e.clientX;
  dragStartSize =
    pane === 'left' ? leftWidth.value : pane === 'right' ? rightWidth.value : bottomHeight.value;
  window.addEventListener('mousemove', onResizeMove);
  window.addEventListener('mouseup', stopResize);
}

function onResizeMove(e: MouseEvent) {
  if (!draggingPane.value) return;
  if (draggingPane.value === 'left') {
    const next = Math.max(200, Math.min(480, dragStartSize + (e.clientX - dragStartPos)));
    leftWidth.value = next;
  } else if (draggingPane.value === 'right') {
    const next = Math.max(210, Math.min(460, dragStartSize - (e.clientX - dragStartPos)));
    rightWidth.value = next;
  } else if (draggingPane.value === 'bottom') {
    const next = Math.max(130, Math.min(540, dragStartSize - (e.clientY - dragStartPos)));
    bottomHeight.value = next;
  }
}

function stopResize() {
  if (draggingPane.value === 'left') {
    localStorage.setItem('pacergrip.leftWidth', String(leftWidth.value));
  } else if (draggingPane.value === 'right') {
    localStorage.setItem('pacergrip.rightWidth', String(rightWidth.value));
  } else if (draggingPane.value === 'bottom') {
    localStorage.setItem('pacergrip.bottomHeight', String(bottomHeight.value));
  }
  draggingPane.value = null;
  window.removeEventListener('mousemove', onResizeMove);
  window.removeEventListener('mouseup', stopResize);
}

// Projects & Database State
const projects = ref<WorkspaceProject[]>([
  {
    id: 'default-project',
    name: '默认项目',
    description: '默认多数据源工作区项目',
    color: '#3574F0',
    rootDir: null,
    createdAt: '',
  },
]);
const activeProjectId = ref<string>(
  localStorage.getItem('pacer.activeProjectId') || 'default-project',
);
const expandedProjects = reactive<Record<string, boolean>>(
  loadJsonStorage<Record<string, boolean>>('pacer.expandedProjects', { 'default-project': true }),
);

const activeProject = computed(
  () => projects.value.find((p) => p.id === activeProjectId.value) ?? projects.value[0],
);

const projectConnections = computed(() => {
  const isFirst = projects.value[0]?.id === activeProjectId.value;
  return connections.value.filter(
    (c) => c.projectId === activeProjectId.value || (isFirst && !c.projectId),
  );
});

const FILE_DB_BINDINGS_KEY = 'pacer.fileDbBindings.v1';

interface FileDbBinding {
  projectId?: string;
  profileId: string;
  dbName: string;
  updatedAt: number;
}

function normalizeBindingPath(path: string): string {
  const norm = path.replace(/\\/g, '/').trim();
  return IS_MAC ? norm : norm.toLowerCase();
}

function loadFileDbBindings(): Record<string, FileDbBinding> {
  try {
    const raw = localStorage.getItem(FILE_DB_BINDINGS_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw);
    return parsed && typeof parsed === 'object' ? (parsed as Record<string, FileDbBinding>) : {};
  } catch {
    return {};
  }
}

const fileDbBindings: Record<string, FileDbBinding> = loadFileDbBindings();

function getFileDbBinding(path?: string | null): FileDbBinding | null {
  if (!path) return null;
  const key = normalizeBindingPath(path);
  return fileDbBindings[key] ?? null;
}

function saveFileDbBinding(
  path: string | null | undefined,
  profileId: string,
  dbName: string,
  projectId?: string,
) {
  if (!path || !profileId) return;
  const key = normalizeBindingPath(path);
  fileDbBindings[key] = {
    projectId,
    profileId,
    dbName: dbName || '',
    updatedAt: Date.now(),
  };
  const keys = Object.keys(fileDbBindings);
  if (keys.length > 300) {
    keys
      .sort((a, b) => (fileDbBindings[a]?.updatedAt ?? 0) - (fileDbBindings[b]?.updatedAt ?? 0))
      .slice(0, keys.length - 300)
      .forEach((oldKey) => {
        delete fileDbBindings[oldKey];
      });
  }
  try {
    localStorage.setItem(FILE_DB_BINDINGS_KEY, JSON.stringify(fileDbBindings));
  } catch {
    // non-fatal
  }
}

const connections = ref<ConnectionProfile[]>([]);
const rawDatabasesByProfile = reactive<Record<string, DatabaseMeta[]>>({});
const databasesByProfile = reactive<Record<string, DatabaseMeta[]>>({});
const tablesByKey = reactive<Record<string, TableMeta[]>>({});
const columnsByKey = reactive<Record<string, ColumnMeta[]>>({});
const indexesByKey = reactive<Record<string, IndexMeta[]>>({});
const foreignKeysByKey = reactive<Record<string, ForeignKeyMeta[]>>({});
const routinesByKey = reactive<Record<string, RoutineMeta[]>>({});
const txStatusByProfile = reactive<Record<string, TransactionStatus>>({});
const expandedProfiles = reactive<Record<string, boolean>>({});
const expandedDatabases = reactive<Record<string, boolean>>({});
const expandedTables = reactive<Record<string, boolean>>({});
const extensions = ref<ExtensionMeta[]>([]);

// Right Panel: Inspected Table Structure
const inspectedTable = ref<{ profileId: string; dbName: string; tableName: string } | null>(null);

const inspectedColumns = computed(() => {
  if (!inspectedTable.value) return [];
  const k = `${inspectedTable.value.profileId}:${inspectedTable.value.dbName}:${inspectedTable.value.tableName}`;
  return columnsByKey[k] ?? [];
});

const inspectedIndexes = computed(() => {
  if (!inspectedTable.value) return [];
  const k = `${inspectedTable.value.profileId}:${inspectedTable.value.dbName}:${inspectedTable.value.tableName}`;
  return indexesByKey[k] ?? [];
});

const inspectedForeignKeys = computed(() => {
  if (!inspectedTable.value) return [];
  const k = `${inspectedTable.value.profileId}:${inspectedTable.value.dbName}:${inspectedTable.value.tableName}`;
  return foreignKeysByKey[k] ?? [];
});

// Files Explorer (Recursive Tree) & Snippets State
const mountedDir = ref<string>(localStorage.getItem('pacergrip.mountedDir') || '');
const rootDirExpanded = ref<boolean>(true);
const workspaceEntries = ref<WorkspaceFileEntry[]>([]);
const fileTreeChildren = reactive<Record<string, WorkspaceFileEntry[]>>({});
const expandedFileDirs = reactive<Record<string, boolean>>({});
const fileExplorerRef = ref<InstanceType<typeof FileExplorer> | null>(null);
const recentFiles = ref<string[]>(loadJsonStorage<string[]>('pacergrip.recentFiles', []));
const showRecentFiles = ref<boolean>(localStorage.getItem('pacer.showRecentFiles') !== '0');
const customSnippets = ref<SqlSnippet[]>(loadJsonStorage<SqlSnippet[]>('pacergrip.snippets', []));
const workspaceInfo = ref<WorkspaceInfoDto | null>(null);
const localPlugins = ref<LocalPluginManifest[]>([]);
const consoleScratchPaths = reactive<Record<string, string>>({});
const lastSavedScratchContent: Record<string, string> = {};
const snippetFilter = ref('');
const closedTabsStack = ref<EditorTab[]>([]);

function loadJsonStorage<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key);
    return raw ? (JSON.parse(raw) as T) : fallback;
  } catch {
    return fallback;
  }
}

const allSnippets = computed(() => {
  const pluginSnippets: SqlSnippet[] = [];
  for (const plg of localPlugins.value) {
    if (!plg.enabled) continue;
    for (let i = 0; i < plg.snippets.length; i++) {
      const s = plg.snippets[i];
      pluginSnippets.push({
        id: `plugin-${plg.id}-${i}`,
        title: s.title,
        category: s.category || plg.name,
        sql: s.sql,
      });
    }
  }
  const merged = [...customSnippets.value, ...pluginSnippets, ...BUILTIN_SNIPPETS];
  const q = snippetFilter.value.trim().toLowerCase();
  if (!q) return merged;
  return merged.filter(
    (s) =>
      s.title.toLowerCase().includes(q) ||
      s.category.toLowerCase().includes(q) ||
      s.sql.toLowerCase().includes(q),
  );
});

// Editor Tabs State
let consoleCounter = 1;
const defaultConsoleSql = `-- Pacer · 原生 SQL 控制台
-- 快捷键：${formatComboDisplay('Ctrl+Enter')} 执行当前语句或选中 SQL ｜ ${formatComboDisplay('Ctrl+Shift+Enter')} 执行全部 ｜ ${formatComboDisplay('Ctrl+O')} 打开 .md/.sql 文件
-- 提示：支持多项目多数据源管理、结果集直接双击修改生成 SQL，以及 Markdown 选中 SQL 直接执行

SELECT
  c.full_name AS 客户名称,
  c.country AS 国家地区,
  c.tier AS 客户等级,
  COUNT(o.id) AS 订单数,
  ROUND(COALESCE(SUM(o.total_amount), 0), 2) AS 订单总额
FROM customers c
LEFT JOIN orders o ON o.customer_id = c.id
GROUP BY c.id, c.full_name, c.country, c.tier
ORDER BY 订单总额 DESC;
`;

function createEmptyCursor(content: string): EditorCursorTelemetry {
  return {
    line: 1,
    col: 1,
    lineCount: countLines(content),
    charCount: content.length,
    cursorOffset: 0,
    selectedText: '',
    selectedChars: 0,
    selectedLines: 0,
  };
}

const tabs = ref<EditorTab[]>([
  {
    id: 'console-1',
    title: 'console_1.sql',
    path: null,
    fileKind: 'sql',
    content: defaultConsoleSql,
    savedContent: defaultConsoleSql,
    encoding: 'UTF-8',
    hasBom: false,
    lineEnding: 'LF',
    byteSize: estimateUtf8Bytes(defaultConsoleSql),
    readOnly: false,
    profileId: '',
    dbName: '',
    mdViewMode: 'source',
    cursor: createEmptyCursor(defaultConsoleSql),
  },
]);
const activeTabId = ref<string>('console-1');
const selectedProfileId = ref<string>('');
const selectedDbName = ref<string>('');
const editorRef = ref<InstanceType<typeof CodeEditor> | null>(null);

const activeTab = computed<EditorTab | null>(
  () => tabs.value.find((tab) => tab.id === activeTabId.value) ?? tabs.value[0] ?? null,
);

const activeProfile = computed(() => {
  const pid = activeTab.value?.profileId || selectedProfileId.value;
  return connections.value.find((c) => c.id === pid) ?? null;
});

const activeDatabases = computed(() => {
  const pid = activeTab.value?.profileId || selectedProfileId.value;
  return pid ? databasesByProfile[pid] ?? [] : [];
});

const activeEngineKey = computed(() => {
  const prof = activeProfile.value;
  if (!prof) return 'sqlite';
  return typeof prof.dbType === 'string' ? prof.dbType : Object.values(prof.dbType)[0] ?? 'sqlite';
});

const activeSchemaMap = computed<Record<string, string[]>>(() => {
  const tab = activeTab.value;
  const profileId = tab?.profileId || selectedProfileId.value;
  const dbName = tab?.dbName || selectedDbName.value;
  if (!profileId || !dbName) return {};
  const tKey = `${profileId}:${dbName}`;
  const tables = tablesByKey[tKey] ?? [];
  const map: Record<string, string[]> = {};
  for (const tbl of tables) {
    const cKey = `${profileId}:${dbName}:${tbl.name}`;
    const cols = columnsByKey[cKey] ?? [];
    map[tbl.name] = cols.map((c) => c.name);
  }
  return map;
});

const isMarkdownTab = computed(() => activeTab.value?.fileKind === 'markdown');
const debouncedMdContent = ref('');
let mdDebounceTimer: number | null = null;
watch(
  () => [activeTab.value?.id, activeTab.value?.fileKind, activeTab.value?.content] as const,
  ([id, kind, content], prev) => {
    if (mdDebounceTimer !== null) {
      window.clearTimeout(mdDebounceTimer);
      mdDebounceTimer = null;
    }
    if (kind !== 'markdown' || typeof content !== 'string') {
      debouncedMdContent.value = '';
      return;
    }
    const prevId = prev?.[0];
    if (id !== prevId || content.length <= 2000) {
      debouncedMdContent.value = content;
      return;
    }
    mdDebounceTimer = window.setTimeout(() => {
      mdDebounceTimer = null;
      debouncedMdContent.value = activeTab.value?.id === id ? (activeTab.value?.content ?? '') : '';
    }, 180);
  },
  { immediate: true },
);

const activeMdBlocks = computed<MarkdownSqlBlock[]>(() => {
  if (!isMarkdownTab.value || !debouncedMdContent.value) return [];
  return extractMarkdownSqlBlocksLocal(debouncedMdContent.value);
});

const activeMdSegments = computed(() => {
  if (!isMarkdownTab.value || !debouncedMdContent.value) return [];
  return parseMarkdownSegments(debouncedMdContent.value);
});

const activeDialectLabel = computed(() => {
  if (!activeTab.value) return 'SQL';
  if (activeTab.value.fileKind === 'markdown') return 'Markdown + SQL';
  if (activeTab.value.fileKind === 'json') return 'JSON';
  if (activeTab.value.fileKind === 'xml') return 'XML';
  if (isRedisEngineKey(activeEngineKey.value)) return 'Redis 命令';
  return `SQL (${getEngineDisplayName(activeEngineKey.value)})`;
});

const activeExplainMeta = computed(() => getExplainModesForEngine(activeEngineKey.value));
const activeExplainAnalyzeOption = computed(
  () => activeExplainMeta.value.options.find((o) => o.mode === 'analyze') ?? null,
);

const MAX_ROWS_STORAGE_KEY = 'pacer.maxRowsLimit.v1';

function loadInitialMaxRowsLimit(): number | null {
  try {
    const raw = localStorage.getItem(MAX_ROWS_STORAGE_KEY);
    if (raw === null) return 100;
    if (raw === '0') return null;
    const parsed = Number(raw);
    if (Number.isFinite(parsed) && parsed > 0) return Math.trunc(parsed);
  } catch {
    // non-fatal
  }
  return 100;
}

function setMaxRowsLimit(rawVal: string) {
  const next = rawVal === '0' ? null : Math.max(1, Math.trunc(Number(rawVal) || 100));
  maxRowsLimit.value = next;
  try {
    localStorage.setItem(MAX_ROWS_STORAGE_KEY, next === null ? '0' : String(next));
  } catch {
    // non-fatal
  }
}

// Services / Results / AST / History State
const maxRowsLimit = ref<number | null>(loadInitialMaxRowsLimit());
const activeServiceTab = ref<'result' | 'output' | 'ddl' | 'history' | 'problems'>('result');
const resultTabs = ref<ResultTabItem[]>([]);
const activeResultTabId = ref<string>('');
const resultGridRef = ref<InstanceType<typeof ResultGrid> | null>(null);
const consoleLogs = ref<ConsoleLogEntry[]>([]);
const ddlText = ref<string>('');
const ddlTitle = ref<string>('');
const historyEntries = ref<QueryHistoryEntry[]>([]);
const astAnalysis = ref<ScriptAnalysis | null>(null);
const busy = ref(false);
const queryExecuting = ref(false);
const activeQueryId = ref<string | null>(null);
const queryElapsedMs = ref(0);
let queryTimerHandle: number | null = null;
const splitTabId = ref<string | null>(null);
const splitEditorRef = ref<InstanceType<typeof CodeEditor> | null>(null);
const splitTab = computed<EditorTab | null>(
  () => tabs.value.find((t) => t.id === splitTabId.value) ?? null,
);
const commercialModalKind = ref<CommercialModalKind>('none');
const commercialProfileId = ref('');
const commercialDbName = ref('');
const commercialTableName = ref('');
const activeTxStatus = computed<TransactionStatus>(() => {
  const pid = activeProfile.value?.id;
  if (!pid) return { supported: false, manualMode: false, inTransaction: false, pendingStatements: 0 };
  return (
    txStatusByProfile[pid] ?? {
      supported: false,
      manualMode: false,
      inTransaction: false,
      pendingStatements: 0,
    }
  );
});
const statusMessage = ref('就绪 · 支持多项目多数据源管理、DBeaver 导入、结果集直接修改与 Markdown 选中执行 SQL');
const statusIsError = ref(false);

// Modals State
const activeModal = ref<ActiveModalKind>('none');
const shortcutsInitialTab = ref<'keymap' | 'templates'>('keymap');
const liveTemplatesStore = useLiveTemplates();
const quickLiveTemplates = computed(() => {
  const priority = ['sel', 'selw', 'selc', 'selo', 'selg', 'ins', 'upd', 'del'];
  const picked: SqlLiveTemplate[] = [];
  for (const ab of priority) {
    const found = liveTemplatesStore.findByAbbrev(ab);
    if (found) picked.push(found);
  }
  return picked;
});

function openShortcutsModal(tab: 'keymap' | 'templates' = 'keymap') {
  shortcutsInitialTab.value = tab;
  activeModal.value = 'shortcuts';
}

function handleEnsureActiveSchemaLoaded() {
  const prof = activeProfile.value;
  const db = activeTab.value?.dbName || selectedDbName.value;
  if (!prof || !db) return;
  const key = `${prof.id}:${db}`;
  if (!tablesByKey[key]) {
    void loadTablesForDb(prof, db);
  }
}

function handleEditorTablePicked(tableName: string) {
  const prof = activeProfile.value;
  const db = activeTab.value?.dbName || selectedDbName.value;
  if (!prof || !db || !tableName) return;
  const cKey = `${prof.id}:${db}:${tableName}`;
  if (!columnsByKey[cKey]) {
    void loadTableMetadata(prof, db, tableName);
  }
}

function handleQuickExpandLiveTemplate(tpl: SqlLiveTemplate) {
  if (!activeTab.value) {
    createNewConsoleTab('');
  }
  window.setTimeout(() => {
    editorRef.value?.expandTemplate(tpl);
  }, 0);
}
const editingProject = ref<WorkspaceProject | null>(null);
const editingProfile = ref<ConnectionProfile | null>(null);
const gridSqlContext = ref<GridMutationPreviewPayload | null>(null);
const exportedJson = ref('');
const dangerContext = ref<DangerModalContext | null>(null);
const pendingDangerAction = ref<(() => Promise<void>) | null>(null);
const paramContext = ref<ParamModalContext | null>(null);
const pendingParamExec = ref<{
  label: string;
  profile: ConnectionProfile;
  dbName: string;
} | null>(null);
const passwordProfile = ref<ConnectionProfile | null>(null);
const pendingAfterPassword = ref<(() => Promise<void>) | null>(null);
const pendingCloseTabId = ref<string | null>(null);
const unsavedContext = ref<{ title: string; path: string | null } | null>(null);
const activeToolModal = ref<ActiveToolModalKind>('none');
const toolTableContext = ref<TableToolContext | null>(null);
const toolDbContext = ref<DbToolContext | null>(null);

// v0.9.6 Advanced Tools / Monitor / ER Diagram / Favorites State
const FAVORITES_STORAGE_KEY = 'pacer.favorites.v1';

function loadInitialFavorites(): FavoriteTableEntry[] {
  try {
    const raw = localStorage.getItem(FAVORITES_STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      if (Array.isArray(parsed)) return parsed;
    }
  } catch {
    // ignore storage errors
  }
  return [];
}

const favorites = ref<FavoriteTableEntry[]>(loadInitialFavorites());
const batchTesting = ref(false);
const advancedModalKind = ref<AdvancedModalKind>('none');
const advancedInitialProfileId = ref('');
const advancedInitialDbName = ref('');
const advancedInitialTableName = ref('');
const redisEditorInit = ref<RedisEditorInitialData | null>(null);
const serverMonitorVisible = ref(false);
const serverMonitorProfile = ref<ConnectionProfile | null>(null);
const erDiagramVisible = ref(false);
const erDiagramProfile = ref<ConnectionProfile | null>(null);
const erDiagramDbName = ref('');

function isRedisProfile(profile: ConnectionProfile | null | undefined): boolean {
  return Boolean(profile && isRedisEngineKey(engineKeyOf(profile)));
}

const encodingModalContext = computed<EncodingModalContext | null>(() => {
  const tab = activeTab.value;
  if (!tab) return null;
  return {
    title: tab.title,
    path: tab.path,
    encoding: tab.encoding,
    hasBom: tab.hasBom,
    lineEnding: tab.lineEnding,
    lineCount: tab.cursor.lineCount,
    byteSize: tab.byteSize,
  };
});

function setStatus(msg: string, isError = false) {
  statusMessage.value = msg;
  statusIsError.value = isError;
}

function pushProjectLog(opts: {
  projectId?: string | null;
  profileId?: string;
  profileName: string;
  dbName?: string;
  level: ConsoleLogEntry['level'];
  summary: string;
  detail?: string;
  durationMs?: number;
}) {
  const defaultPid = projects.value[0]?.id || 'default-project';
  const pid = opts.projectId || (opts.profileId ? activeProjectId.value : defaultPid) || defaultPid;
  const ts = new Date().toLocaleTimeString('zh-CN', { hour12: false });
  consoleLogs.value.push({
    id: `log-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`,
    timestamp: ts,
    projectId: pid,
    profileId: opts.profileId,
    profileName: opts.profileName,
    dbName: opts.dbName,
    level: opts.level,
    summary: opts.summary,
    detail: opts.detail,
    durationMs: opts.durationMs,
  });
  if (consoleLogs.value.length > 1200) {
    consoleLogs.value.splice(0, consoleLogs.value.length - 1200);
  }
}

function appendLog(line: string, level: ConsoleLogEntry['level'] = 'info', profileName = 'Pacer') {
  pushProjectLog({
    projectId: activeProjectId.value,
    profileName,
    level,
    summary: line,
  });
}

function clearConsoleLogs(projectId: string) {
  const defaultPid = projects.value[0]?.id || 'default-project';
  if (!projectId || projectId === 'ALL') {
    consoleLogs.value = [];
    setStatus('已清空全部项目的控制台输出日志');
  } else {
    consoleLogs.value = consoleLogs.value.filter(
      (entry) => (entry.projectId || defaultPid) !== projectId,
    );
    const projName = projects.value.find((p) => p.id === projectId)?.name ?? projectId;
    setStatus(`已清空项目「${projName}」的控制台输出日志`);
  }
}

function isTabDirty(tab: EditorTab): boolean {
  return tab.content !== tab.savedContent;
}

function updateTabContent(next: string) {
  updateTabContentByTarget(activeTab.value, next);
}

function updateTabContentByTarget(tab: EditorTab | null | undefined, next: string) {
  if (!tab) return;
  tab.content = next;
  tab.byteSize = estimateUtf8Bytes(next);
  tab.cursor.lineCount = countLines(next);
  tab.cursor.charCount = next.length;
  scheduleAstInspect();
  scheduleSaveSession();
}

function handleSplitCursorChange(telemetry: EditorCursorTelemetry) {
  if (splitTab.value) {
    splitTab.value.cursor = telemetry;
  }
}

function handleCursorChange(telemetry: EditorCursorTelemetry) {
  const tab = activeTab.value;
  if (!tab) return;
  const prevSel = tab.cursor.selectedText.trim();
  const nextSel = telemetry.selectedText.trim();
  tab.cursor = telemetry;
  if (nextSel && nextSel !== prevSel) {
    scheduleAstInspect();
  }
}

// Debounced live AST analysis
let astTimer: number | null = null;
function scheduleAstInspect() {
  if (astTimer !== null) window.clearTimeout(astTimer);
  astTimer = window.setTimeout(() => {
    void refreshAstInspection();
  }, 450);
}

async function refreshAstInspection() {
  const tab = activeTab.value;
  if (!tab) return;
  let sqlCandidate = tab.cursor.selectedText.trim();
  if (!sqlCandidate) {
    if (tab.fileKind === 'markdown') {
      const enclosing = findEnclosingMarkdownBlock(tab.cursor.line);
      sqlCandidate = enclosing?.sql ?? activeMdBlocks.value[0]?.sql ?? '';
    } else {
      sqlCandidate = tab.content.trim();
    }
  }
  if (!sqlCandidate || sqlCandidate.length > 200_000) {
    astAnalysis.value = null;
    return;
  }
  try {
    astAnalysis.value = await analyzeSql(sqlCandidate, activeEngineKey.value);
  } catch {
    // Keep last valid analysis on transient syntax state while typing
  }
}

function findEnclosingMarkdownBlock(cursorLine: number): MarkdownSqlBlock | null {
  return (
    activeMdBlocks.value.find((b) => cursorLine >= b.startLine && cursorLine <= b.endLine) ?? null
  );
}

// ==================== Project & Connection Operations ====================

async function refreshProjectsAndConnections() {
  if (!isTauriRuntime()) {
    setStatus(t('browserFallbackNotice'));
    return;
  }
  try {
    const projList = await listProjects();
    if (projList.length > 0) {
      projects.value = projList;
      if (!projList.some((p) => p.id === activeProjectId.value)) {
        activeProjectId.value = projList[0].id;
        localStorage.setItem('pacer.activeProjectId', activeProjectId.value);
      }
      // Ensure active project is expanded on launch
      if (expandedProjects[activeProjectId.value] === undefined) {
        expandedProjects[activeProjectId.value] = true;
      }
    }

    let list = await listConnections();
    if (list.length === 0) {
      const demo = await createDemoSqliteConnection(activeProjectId.value);
      list = [demo];
      pushProjectLog({
        projectId: demo.projectId,
        profileId: demo.id,
        profileName: demo.name,
        level: 'info',
        summary: '自动初始化内置 SQLite 示例数据源',
      });
    }
    connections.value = list;

    const preferred =
      list.find((c) => c.projectId === activeProjectId.value) ?? list[0];
    if (preferred) {
      if (!selectedProfileId.value) {
        selectedProfileId.value = preferred.id;
      }
      if (activeTab.value && !activeTab.value.profileId) {
        activeTab.value.profileId = preferred.id;
        await loadDatabasesForProfile(preferred, true);
      } else if (!activeTab.value) {
        await loadDatabasesForProfile(preferred, true);
      }
    }

    const curProj = activeProject.value;
    if (curProj?.rootDir) {
      await loadDirectory(curProj.rootDir);
    }
  } catch (err) {
    setStatus(`加载项目与数据源失败: ${String(err)}`, true);
  }
}

async function selectProject(projectId: string) {
  openMenu.value = null;
  activeProjectId.value = projectId;
  expandedProjects[projectId] = true;
  localStorage.setItem('pacer.activeProjectId', projectId);
  localStorage.setItem('pacer.expandedProjects', JSON.stringify(expandedProjects));
  const proj = projects.value.find((p) => p.id === projectId);
  if (!proj) return;

  const isFirst = projects.value[0]?.id === projectId;
  const matchingConns = connections.value.filter(
    (c) => c.projectId === projectId || (isFirst && !c.projectId),
  );
  const currentPid = activeTab.value?.profileId || selectedProfileId.value;
  const currentBelongs = matchingConns.some((c) => c.id === currentPid);
  if (!currentBelongs) {
    const next = matchingConns[0];
    selectedProfileId.value = next?.id ?? '';
    selectedDbName.value = '';
    if (activeTab.value) {
      activeTab.value.profileId = next?.id ?? '';
      activeTab.value.dbName = '';
    }
    if (next) {
      await loadDatabasesForProfile(next, true);
      if (activeTab.value?.path && activeTab.value.profileId === next.id) {
        saveFileDbBinding(
          activeTab.value.path,
          next.id,
          activeTab.value.dbName || '',
          next.projectId || projectId,
        );
      }
    }
  }
  if (proj.rootDir) {
    void loadDirectory(proj.rootDir);
  }
  setStatus(`已切换至项目: ${proj.name}（${matchingConns.length} 个数据源）`);
}

function toggleProjectNode(project: WorkspaceProject) {
  const wasExpanded = Boolean(expandedProjects[project.id]);
  expandedProjects[project.id] = !wasExpanded;
  localStorage.setItem('pacer.expandedProjects', JSON.stringify(expandedProjects));
  if (activeProjectId.value !== project.id) {
    activeProjectId.value = project.id;
    localStorage.setItem('pacer.activeProjectId', project.id);
    if (project.rootDir) {
      void loadDirectory(project.rootDir);
    }
    setStatus(`已切换至项目: ${project.name}`);
  }
}

function collapseAllDatabaseTree() {
  for (const k of Object.keys(expandedTables)) expandedTables[k] = false;
  for (const k of Object.keys(expandedDatabases)) expandedDatabases[k] = false;
  for (const k of Object.keys(expandedProfiles)) expandedProfiles[k] = false;
  setStatus('已折叠全部数据源与库表节点');
}

function openProjectModal(project: WorkspaceProject | null) {
  editingProject.value = project;
  activeModal.value = 'project';
}

async function handleSaveProject(draft: ProjectDraft) {
  busy.value = true;
  try {
    const saved = await saveProject(draft);
    projects.value = await listProjects();
    activeModal.value = 'none';
    await selectProject(saved.id);
    pushProjectLog({
      projectId: saved.id,
      profileName: saved.name,
      level: 'success',
      summary: draft.id ? '已更新项目配置' : '已创建新项目',
    });
  } catch (err) {
    setStatus(`保存项目失败: ${String(err)}`, true);
  } finally {
    busy.value = false;
  }
}

async function handleDeleteProject(projectId: string) {
  busy.value = true;
  try {
    const remaining = await deleteProject(projectId);
    projects.value = remaining;
    connections.value = await listConnections();
    activeModal.value = 'none';
    if (remaining[0]) {
      await selectProject(remaining[0].id);
    }
    setStatus('已删除项目，原项目数据源已自动移入默认项目');
  } catch (err) {
    setStatus(`删除项目失败: ${String(err)}`, true);
  } finally {
    busy.value = false;
  }
}

async function handleMoveProfileProject(profile: ConnectionProfile, targetProjectId: string) {
  try {
    await assignConnectionProject(profile.id, targetProjectId);
    connections.value = await listConnections();
    const targetName = projects.value.find((p) => p.id === targetProjectId)?.name ?? targetProjectId;
    setStatus(`已将数据源「${profile.name}」移动至项目「${targetName}」`);
  } catch (err) {
    setStatus(`移动数据源失败: ${String(err)}`, true);
  }
}

const connHealthByProfile = reactive<Record<string, ConnectionHealthInfo>>({});

function setConnHealth(
  profileId: string,
  status: 'ok' | 'error' | 'testing',
  message?: string,
  latencyMs?: number,
) {
  connHealthByProfile[profileId] = {
    status,
    message,
    latencyMs,
    checkedAt: new Date().toLocaleTimeString(),
  };
}

function handleModalTestResult(
  profileId: string,
  ok: boolean,
  message: string,
  latencyMs?: number,
) {
  setConnHealth(profileId, ok ? 'ok' : 'error', message, latencyMs);
}

function connHealthTooltip(profile: ConnectionProfile): string {
  const h = connHealthByProfile[profile.id];
  if (!h) return `${profile.name} (${profile.env}) · 尚未检测连接`;
  if (h.status === 'ok') {
    const lat = h.latencyMs !== undefined ? ` (${h.latencyMs}ms)` : '';
    return `${profile.name} · 连接正常可使用${lat}`;
  }
  if (h.status === 'error') {
    return `${profile.name} · 无法连接：${h.message || '请检查配置'}`;
  }
  return `${profile.name} · 正在检测连接...`;
}

function looksLikeConnectionFailure(msg: string): boolean {
  const low = msg.toLowerCase();
  return (
    low.includes('transport error') ||
    low.includes('connection refused') ||
    low.includes('connection failed') ||
    low.includes('timed out') ||
    low.includes('timeout') ||
    low.includes('authentication') ||
    low.includes('access denied') ||
    low.includes('wrongpass') ||
    low.includes('closed by remote') ||
    low.includes('handshake failed') ||
    low.includes('eof')
  );
}

async function loadDatabasesForProfile(profile: ConnectionProfile, autoSelectFirst = false) {
  setConnHealth(profile.id, 'testing', '正在加载数据库列表...');
  try {
    const rawDbs = await listDatabases(profile.id);
    setConnHealth(profile.id, 'ok', `已加载 ${rawDbs.length} 个数据库`);
    rawDatabasesByProfile[profile.id] = rawDbs;
    const cfg = (profile.mode?.config ?? {}) as Partial<DirectConfig>;
    const preferredDb = (cfg.database ?? profile.fingerprint.database ?? '').trim();
    const dbs = cfg.schemaPattern?.trim()
      ? rawDbs.filter((d) => matchesFilterPattern(d.name, cfg.schemaPattern, preferredDb))
      : rawDbs;
    databasesByProfile[profile.id] = dbs;
    expandedProfiles[profile.id] = true;
    const defaultDb =
      (preferredDb ? dbs.find((d) => d.name === preferredDb)?.name : undefined) ??
      dbs.find((d) => !d.isSystem)?.name ??
      dbs[0]?.name ??
      '';
    if (autoSelectFirst && defaultDb) {
      if (selectedProfileId.value === profile.id && !selectedDbName.value) {
        selectedDbName.value = defaultDb;
      }
      if (activeTab.value && activeTab.value.profileId === profile.id) {
        if (!activeTab.value.dbName) {
          activeTab.value.dbName = defaultDb;
        }
        await loadTablesForDb(profile, activeTab.value.dbName);
      } else if (!activeTab.value && selectedProfileId.value === profile.id) {
        await loadTablesForDb(profile, defaultDb);
      }
    }
  } catch (err) {
    const msg = String(err);
    setConnHealth(profile.id, 'error', msg);
    if (msg.toLowerCase().includes('password') || msg.includes('密码') || msg.includes('denied')) {
      passwordProfile.value = profile;
      pendingAfterPassword.value = () => loadDatabasesForProfile(profile, autoSelectFirst);
      activeModal.value = 'password';
      return;
    }
    setStatus(`加载库列表失败 (${profile.name}): ${msg}`, true);
    pushProjectLog({
      projectId: profile.projectId,
      profileId: profile.id,
      profileName: profile.name,
      level: 'error',
      summary: '加载数据库列表失败',
      detail: msg,
    });
  }
}

async function loadTablesForDb(profile: ConnectionProfile, dbName: string) {
  const key = `${profile.id}:${dbName}`;
  try {
    const rawTables = await listTables(profile.id, dbName);
    const cfg = (profile.mode?.config ?? {}) as Partial<DirectConfig>;
    const tables = cfg.objectFilter?.trim()
      ? rawTables.filter((t) => matchesFilterPattern(t.name, cfg.objectFilter))
      : rawTables;
    tablesByKey[key] = tables;
    expandedDatabases[key] = true;
    if (!isRedisProfile(profile)) {
      listRoutines(profile.id, dbName)
        .then((rts) => {
          routinesByKey[key] = rts;
        })
        .catch(() => {});
    }
    // Prefetch first few tables' columns silently and apply in one batch
    const toPrefetch = tables
      .slice(0, 12)
      .filter((tbl) => !columnsByKey[`${profile.id}:${dbName}:${tbl.name}`]);
    if (toPrefetch.length > 0) {
      void Promise.allSettled(
        toPrefetch.map((tbl) =>
          listColumns(profile.id, dbName, tbl.name).then((cols) => ({
            key: `${profile.id}:${dbName}:${tbl.name}`,
            cols,
          })),
        ),
      ).then((results) => {
        for (const r of results) {
          if (r.status === 'fulfilled') {
            columnsByKey[r.value.key] = r.value.cols;
          }
        }
      });
    }
  } catch (err) {
    setStatus(`加载表列表失败 (${dbName}): ${String(err)}`, true);
  }
}

function connectionsInProjectById(projectId: string): ConnectionProfile[] {
  const isFirst = projects.value[0]?.id === projectId;
  return connections.value.filter(
    (c) => c.projectId === projectId || (isFirst && !c.projectId),
  );
}

function syncActiveProjectForProfile(profile: ConnectionProfile) {
  const pid = profile.projectId || projects.value[0]?.id || 'default-project';
  if (activeProjectId.value !== pid) {
    activeProjectId.value = pid;
    expandedProjects[pid] = true;
    localStorage.setItem('pacer.activeProjectId', pid);
    localStorage.setItem('pacer.expandedProjects', JSON.stringify(expandedProjects));
  }
}

async function applyTabDatabaseBinding(
  tab: EditorTab | null | undefined,
  preferSavedFileBinding = false,
) {
  if (!tab) return;
  if (preferSavedFileBinding && tab.path) {
    const saved = getFileDbBinding(tab.path);
    if (saved && connections.value.some((c) => c.id === saved.profileId)) {
      tab.profileId = saved.profileId;
      tab.dbName = saved.dbName || '';
    }
  }
  const prof = connections.value.find((c) => c.id === tab.profileId);
  if (!prof) return;
  syncActiveProjectForProfile(prof);
  selectedProfileId.value = prof.id;
  selectedDbName.value = tab.dbName || '';
  if (!databasesByProfile[prof.id]) {
    await loadDatabasesForProfile(prof, !tab.dbName);
  }
  const targetDb = tab.dbName || selectedDbName.value;
  if (targetDb && !tablesByKey[`${prof.id}:${targetDb}`]) {
    void loadTablesForDb(prof, targetDb);
  }
}

async function toggleProfileNode(profile: ConnectionProfile) {
  syncActiveProjectForProfile(profile);
  selectedProfileId.value = profile.id;
  if (activeTab.value) {
    activeTab.value.profileId = profile.id;
    if (activeTab.value.path) {
      saveFileDbBinding(
        activeTab.value.path,
        profile.id,
        activeTab.value.dbName || '',
        profile.projectId || activeProjectId.value,
      );
    }
  }
  if (expandedProfiles[profile.id]) {
    expandedProfiles[profile.id] = false;
    return;
  }
  await loadDatabasesForProfile(profile, true);
}

async function toggleDatabaseNode(profile: ConnectionProfile, dbName: string) {
  syncActiveProjectForProfile(profile);
  selectedProfileId.value = profile.id;
  selectedDbName.value = dbName;
  if (activeTab.value) {
    activeTab.value.profileId = profile.id;
    activeTab.value.dbName = dbName;
    if (activeTab.value.path) {
      saveFileDbBinding(
        activeTab.value.path,
        profile.id,
        dbName,
        profile.projectId || activeProjectId.value,
      );
    }
  }
  const key = `${profile.id}:${dbName}`;
  if (expandedDatabases[key]) {
    expandedDatabases[key] = false;
    return;
  }
  await loadTablesForDb(profile, dbName);
}

async function loadTableMetadata(profile: ConnectionProfile, dbName: string, tableName: string) {
  const key = `${profile.id}:${dbName}:${tableName}`;
  const isRedis = isRedisProfile(profile);
  try {
    const [cols, idxs, fks] = await Promise.all([
      listColumns(profile.id, dbName, tableName),
      listIndexes(profile.id, dbName, tableName).catch(() => [] as IndexMeta[]),
      isRedis
        ? Promise.resolve([] as ForeignKeyMeta[])
        : listForeignKeys(profile.id, dbName, tableName).catch(() => [] as ForeignKeyMeta[]),
    ]);
    columnsByKey[key] = cols;
    indexesByKey[key] = idxs;
    foreignKeysByKey[key] = fks;
  } catch (err) {
    setStatus(`读取表元数据失败 (${tableName}): ${String(err)}`, true);
  }
}

async function toggleTableNode(profile: ConnectionProfile, dbName: string, tableName: string) {
  const key = `${profile.id}:${dbName}:${tableName}`;
  expandedTables[key] = !expandedTables[key];
  if (expandedTables[key] && !columnsByKey[key]) {
    await loadTableMetadata(profile, dbName, tableName);
  }
}

async function selectTableNode(profile: ConnectionProfile, dbName: string, tableName: string) {
  syncActiveProjectForProfile(profile);
  inspectedTable.value = { profileId: profile.id, dbName, tableName };
  if (rightTool.value === null) {
    rightTool.value = 'structure';
  }
  await loadTableMetadata(profile, dbName, tableName);
}

async function refreshTransactionStatus(profileId: string) {
  const prof = connections.value.find((c) => c.id === profileId);
  if (!prof || isRedisProfile(prof) || !isTauriRuntime()) return;
  try {
    txStatusByProfile[profileId] = await getTransactionStatus(profileId);
  } catch {
    // non-fatal
  }
}

async function handleProfileSelectChange(profileId: string) {
  selectedProfileId.value = profileId;
  selectedDbName.value = '';
  const tab = activeTab.value;
  if (tab) {
    tab.profileId = profileId;
    tab.dbName = '';
  }
  const prof = connections.value.find((c) => c.id === profileId);
  if (prof) {
    syncActiveProjectForProfile(prof);
    void refreshTransactionStatus(prof.id);
    await loadDatabasesForProfile(prof, true);
    const finalDb = activeTab.value?.dbName || selectedDbName.value || '';
    if (activeTab.value?.path && activeTab.value.profileId === prof.id) {
      saveFileDbBinding(
        activeTab.value.path,
        prof.id,
        finalDb,
        prof.projectId || activeProjectId.value,
      );
      setStatus(
        `已为 ${activeTab.value.title} 记住执行数据源: ${prof.name}${finalDb ? ` / ${finalDb}` : ''}`,
      );
    }
  }
}

async function handleDbSelectChange(dbName: string) {
  selectedDbName.value = dbName;
  const tab = activeTab.value;
  if (tab) {
    tab.dbName = dbName;
    if (tab.path && activeProfile.value) {
      saveFileDbBinding(
        tab.path,
        activeProfile.value.id,
        dbName,
        activeProfile.value.projectId || activeProjectId.value,
      );
      setStatus(
        `已为 ${tab.title} 记住执行数据库: ${activeProfile.value.name}${dbName ? ` / ${dbName}` : ''}`,
      );
    }
  }
  if (!activeProfile.value) return;
  if (dbName) {
    await loadTablesForDb(activeProfile.value, dbName);
  }
}

async function handleSaveConnection(
  draft: ConnectionDraft,
  password: string | null,
  closeAfter: boolean,
) {
  busy.value = true;
  try {
    const saved = await saveConnection(draft, password);
    connections.value = await listConnections();
    editingProfile.value = saved;
    selectedProfileId.value = saved.id;
    if (activeTab.value) {
      activeTab.value.profileId = saved.id;
    }
    setStatus(`已保存数据源: ${saved.name}`);
    pushProjectLog({
      projectId: saved.projectId,
      profileId: saved.id,
      profileName: saved.name,
      level: 'success',
      summary: '已保存数据源配置',
    });
    for (const k of Object.keys(tablesByKey)) {
      if (k.startsWith(`${saved.id}:`)) {
        delete tablesByKey[k];
      }
    }
    void loadDatabasesForProfile(saved, closeAfter);
    if (closeAfter) {
      activeModal.value = 'none';
    }
  } catch (err) {
    const msg = String(err);
    setStatus(`保存数据源失败: ${msg}`, true);
    appendLog(`保存数据源失败: ${msg}`, 'error');
  } finally {
    busy.value = false;
  }
}

async function handleTestProfile(profile: ConnectionProfile) {
  busy.value = true;
  setConnHealth(profile.id, 'testing', '正在测试连接...');
  try {
    const ping = await testConnection(profile.id);
    if (ping.success) {
      setConnHealth(profile.id, 'ok', ping.serverVersion || ping.message, ping.latencyMs);
      setStatus(`连接测试成功: ${profile.name} (${ping.latencyMs}ms · ${ping.serverVersion ?? ''})`);
      pushProjectLog({
        projectId: profile.projectId,
        profileId: profile.id,
        profileName: profile.name,
        level: 'success',
        durationMs: ping.latencyMs,
        summary: `连接测试成功 (${ping.latencyMs} ms · ${ping.serverVersion ?? 'OK'})`,
      });
    } else {
      setConnHealth(profile.id, 'error', ping.message, ping.latencyMs);
      setStatus(`连接测试失败: ${ping.message}`, true);
      pushProjectLog({
        projectId: profile.projectId,
        profileId: profile.id,
        profileName: profile.name,
        level: 'error',
        summary: '连接测试失败',
        detail: ping.message,
      });
    }
  } catch (err) {
    const msg = String(err);
    setConnHealth(profile.id, 'error', msg);
    setStatus(`连接测试出错: ${msg}`, true);
    pushProjectLog({
      projectId: profile.projectId,
      profileId: profile.id,
      profileName: profile.name,
      level: 'error',
      summary: '连接测试异常',
      detail: msg,
    });
  } finally {
    busy.value = false;
  }
}

async function handleDeleteProfile(profile: ConnectionProfile) {
  try {
    await deleteConnection(profile.id);
    connections.value = await listConnections();
    if (selectedProfileId.value === profile.id) {
      selectedProfileId.value = connections.value[0]?.id ?? '';
      selectedDbName.value = '';
    }
    if (activeTab.value?.profileId === profile.id) {
      activeTab.value.profileId = connections.value[0]?.id ?? '';
      activeTab.value.dbName = '';
    }
    setStatus(`已删除数据源: ${profile.name}`);
  } catch (err) {
    setStatus(`删除数据源失败: ${String(err)}`, true);
  }
}

async function handleInitDemoSqlite(targetProjectId?: string) {
  busy.value = true;
  try {
    const pid = targetProjectId || activeProjectId.value;
    const demo = await createDemoSqliteConnection(pid);
    connections.value = await listConnections();
    expandedProjects[pid] = true;
    selectedProfileId.value = demo.id;
    if (activeTab.value) {
      activeTab.value.profileId = demo.id;
    }
    await loadDatabasesForProfile(demo, true);
    setStatus(`示例 SQLite 数据库已就绪: ${demo.name}`);
  } catch (err) {
    setStatus(`初始化示例库失败: ${String(err)}`, true);
  } finally {
    busy.value = false;
  }
}

async function handleSubmitPassword(profileId: string, password: string) {
  busy.value = true;
  setConnHealth(profileId, 'testing', '正在验证密码...');
  try {
    const ping = await testConnection(profileId, password);
    if (!ping.success) {
      setConnHealth(profileId, 'error', ping.message, ping.latencyMs);
      setStatus(`认证失败: ${ping.message}`, true);
      return;
    }
    setConnHealth(profileId, 'ok', ping.serverVersion || ping.message, ping.latencyMs);
    activeModal.value = 'none';
    setStatus(`认证成功 (${ping.latencyMs}ms)`);
    if (pendingAfterPassword.value) {
      const fn = pendingAfterPassword.value;
      pendingAfterPassword.value = null;
      await fn();
    }
  } catch (err) {
    const msg = String(err);
    setConnHealth(profileId, 'error', msg);
    setStatus(`密码认证失败: ${msg}`, true);
  } finally {
    busy.value = false;
  }
}

async function handleImportDbeaverPath(path: string, includePasswords: boolean) {
  busy.value = true;
  try {
    const imported = await importDbeaverPath(path, includePasswords, activeProjectId.value);
    connections.value = await listConnections();
    setStatus(`已从 DBeaver 导入 ${imported.length} 个数据源到「${activeProject.value?.name ?? '当前项目'}」`);
    pushProjectLog({
      projectId: activeProjectId.value,
      profileName: 'DBeaver 导入',
      level: 'success',
      summary: `成功导入 ${imported.length} 个数据源`,
      detail: path,
    });
    activeModal.value = 'none';
  } catch (err) {
    const msg = String(err);
    setStatus(`导入 DBeaver 路径失败: ${msg}`, true);
    pushProjectLog({
      projectId: activeProjectId.value,
      profileName: 'DBeaver 导入',
      level: 'error',
      summary: '导入 DBeaver 路径失败',
      detail: msg,
    });
  } finally {
    busy.value = false;
  }
}

async function handleImportDbeaverText(text: string) {
  busy.value = true;
  try {
    const imported = await importDbeaverText(text, activeProjectId.value);
    connections.value = await listConnections();
    setStatus(`已从 DBeaver 配置解析导入 ${imported.length} 个数据源`);
    pushProjectLog({
      projectId: activeProjectId.value,
      profileName: 'DBeaver 导入',
      level: 'success',
      summary: `成功解析并导入 ${imported.length} 个数据源`,
    });
    activeModal.value = 'none';
  } catch (err) {
    const msg = String(err);
    setStatus(`解析 DBeaver 配置失败: ${msg}`, true);
    pushProjectLog({
      projectId: activeProjectId.value,
      profileName: 'DBeaver 导入',
      level: 'error',
      summary: '解析 DBeaver 配置失败',
      detail: msg,
    });
  } finally {
    busy.value = false;
  }
}

async function handleExportDbeaverJson() {
  busy.value = true;
  try {
    exportedJson.value = await exportDbeaverJson();
    setStatus('已生成 DBeaver 兼容 data-sources.json（不含明文密码）');
  } catch (err) {
    setStatus(`导出 DBeaver JSON 失败: ${String(err)}`, true);
  } finally {
    busy.value = false;
  }
}

async function handleExportDbeaverDbp(
  includePasswords = true,
  profileIds?: string[],
  defaultName = 'pacer-dbeaver-export.dbp',
) {
  busy.value = true;
  try {
    const savePath = await pickSaveDbeaverDbpPath(defaultName);
    if (!savePath) return;
    const res = await exportDbeaverDbp(savePath, includePasswords, profileIds);
    const pwdTip = res.includedPasswords ? '含 AES 加密账号密码' : '不含密码';
    setStatus(`已导出 ${res.count} 个数据源到 DBeaver 项目包 (${pwdTip}) -> ${res.path}`);
    pushProjectLog({
      projectId: activeProjectId.value,
      profileName: 'DBeaver 导出',
      level: 'success',
      summary: `已导出 ${res.count} 个数据源 (${pwdTip})`,
      detail: res.path,
    });
  } catch (err) {
    setStatus(`导出 DBeaver 项目包失败: ${String(err)}`, true);
  } finally {
    busy.value = false;
  }
}

async function handleExportDbeaverFolder(includePasswords = true, profileIds?: string[]) {
  busy.value = true;
  try {
    const dirPath = await pickExportFolderPath();
    if (!dirPath) return;
    const res = await exportDbeaverFolder(dirPath, includePasswords, profileIds);
    const pwdTip = res.includedPasswords ? '含加密凭证文件 credentials-config.json' : '仅连接结构';
    setStatus(`已导出 ${res.count} 个数据源至目录 (${pwdTip}) -> ${res.path}`);
    pushProjectLog({
      projectId: activeProjectId.value,
      profileName: 'DBeaver 目录导出',
      level: 'success',
      summary: `已导出 ${res.count} 个数据源 (${res.files.length} 个文件)`,
      detail: res.files.join('\n'),
    });
  } catch (err) {
    setStatus(`导出 DBeaver 目录失败: ${String(err)}`, true);
  } finally {
    busy.value = false;
  }
}

async function handleExportConnectionsXlsx(
  includePasswords = true,
  profileIds?: string[],
  defaultName = 'pacer-connections-accounts.xlsx',
) {
  busy.value = true;
  try {
    const savePath = await pickSaveXlsxPath(defaultName);
    if (!savePath) return;
    const res = await exportConnectionsXlsx(savePath, includePasswords, profileIds);
    setStatus(`已导出 ${res.count} 条数据源账号到 Excel (.xlsx) -> ${res.path}`);
    pushProjectLog({
      projectId: activeProjectId.value,
      profileName: 'Excel 账号导出',
      level: 'success',
      summary: `已导出 ${res.count} 条数据源账号到 Excel`,
      detail: res.path,
    });
  } catch (err) {
    setStatus(`导出 Excel (.xlsx) 失败: ${String(err)}`, true);
  } finally {
    busy.value = false;
  }
}

async function handleExportSingleProfileToDbeaver(profile: ConnectionProfile) {
  const safe = profile.name.replace(/[^\w.\-\u4e00-\u9fa5]/g, '_');
  await handleExportDbeaverDbp(true, [profile.id], `${safe || 'connection'}.dbp`);
}

async function handleExportProjectToDbeaver(project: WorkspaceProject) {
  const ids = connectionsInProjectById(project.id).map((c) => c.id);
  if (ids.length === 0) {
    setStatus(`项目「${project.name}」暂无数据源可导出`, true);
    return;
  }
  const safe = project.name.replace(/[^\w.\-\u4e00-\u9fa5]/g, '_');
  await handleExportDbeaverDbp(true, ids, `${safe || 'project'}-dbeaver.dbp`);
}

// ==================== Table Actions ====================

async function ensureTableColumns(
  profile: ConnectionProfile,
  dbName: string,
  tableName: string,
): Promise<ColumnMeta[]> {
  const cKey = `${profile.id}:${dbName}:${tableName}`;
  if (!columnsByKey[cKey]) {
    await loadTableMetadata(profile, dbName, tableName);
  }
  return columnsByKey[cKey] ?? [];
}

function insertGeneratedSqlSnippet(profile: ConnectionProfile, dbName: string, snippet: string) {
  if (!activeTab.value) {
    createNewConsoleTab(snippet, profile.id, dbName);
  } else {
    editorRef.value?.insertText(snippet);
  }
}

async function handleQueryTop100(profile: ConnectionProfile, dbName: string, tableName: string) {
  syncActiveProjectForProfile(profile);
  selectedProfileId.value = profile.id;
  selectedDbName.value = dbName;
  if (activeTab.value && !activeTab.value.path) {
    activeTab.value.profileId = profile.id;
    activeTab.value.dbName = dbName;
  }
  if (isRedisProfile(profile)) {
    const tbl = (tablesByKey[`${profile.id}:${dbName}`] ?? []).find((t) => t.name === tableName);
    const limit = maxRowsLimit.value ?? 100;
    const cmd = buildRedisInspectCommand(tableName, tbl?.tableType, limit);
    await executeStatementsBatch([cmd], `${tableName} (${tbl?.tableType || 'key'})`, profile, dbName);
    return;
  }
  const engineKey = engineKeyOf(profile);
  const pageSize = maxRowsLimit.value ?? 100;
  const sql = `${buildTablePageQuerySql({
    dbName,
    tableName,
    engineKey,
    limit: pageSize,
    offset: 0,
  })};`;
  await executeStatementsBatch([sql], `${tableName} (前${pageSize}行)`, profile, dbName, tableName);
}

async function handleCountTable(profile: ConnectionProfile, dbName: string, tableName: string) {
  syncActiveProjectForProfile(profile);
  if (isRedisProfile(profile)) {
    const tbl = (tablesByKey[`${profile.id}:${dbName}`] ?? []).find((t) => t.name === tableName);
    const cmd = buildRedisCountCommand(tableName, tbl?.tableType);
    await executeStatementsBatch([cmd], `${tableName} (长度统计)`, profile, dbName);
    return;
  }
  const engineKey = engineKeyOf(profile);
  const quotedTable = quoteIdentifier(tableName, engineKey);
  const sql = `SELECT COUNT(*) AS total_rows FROM ${quotedTable};`;
  await executeStatementsBatch([sql], `${tableName} (行数统计)`, profile, dbName, tableName);
}

async function handleViewDdl(profile: ConnectionProfile, dbName: string, tableName: string) {
  busy.value = true;
  try {
    const ddl = await getTableDdl(profile.id, dbName, tableName);
    ddlText.value = ddl;
    ddlTitle.value = `${profile.name} · ${dbName}.${tableName}`;
    activeServiceTab.value = 'ddl';
    setStatus(
      isRedisProfile(profile) ? `已加载 Redis Key 详情: ${tableName}` : `已加载表 DDL: ${tableName}`,
    );
  } catch (err) {
    setStatus(`获取详情失败: ${String(err)}`, true);
  } finally {
    busy.value = false;
  }
}

async function handleGenSelect(profile: ConnectionProfile, dbName: string, tableName: string) {
  if (isRedisProfile(profile)) {
    const tbl = (tablesByKey[`${profile.id}:${dbName}`] ?? []).find((t) => t.name === tableName);
    const limit = maxRowsLimit.value ?? 100;
    const cmd = `${buildRedisInspectCommand(tableName, tbl?.tableType, limit)}\n`;
    insertGeneratedSqlSnippet(profile, dbName, cmd);
    setStatus(`已生成 Redis Key ${tableName} 查询命令到编辑器`);
    return;
  }
  const cols = await ensureTableColumns(profile, dbName, tableName);
  const engineKey = engineKeyOf(profile);
  const quotedTable = quoteIdentifier(tableName, engineKey);
  const colList =
    cols.length > 0 ? cols.map((c) => `  ${quoteIdentifier(c.name, engineKey)}`).join(',\n') : '  *';
  const snippet = `SELECT\n${colList}\nFROM ${quotedTable}\nLIMIT 100;\n`;
  insertGeneratedSqlSnippet(profile, dbName, snippet);
  setStatus(`已生成 ${tableName} 查询语句到编辑器`);
}

async function handleGenInsert(profile: ConnectionProfile, dbName: string, tableName: string) {
  const cols = await ensureTableColumns(profile, dbName, tableName);
  const engineKey = engineKeyOf(profile);
  const quotedTable = quoteIdentifier(tableName, engineKey);
  const targetCols = cols.length > 0 ? cols.map((c) => c.name) : ['col1', 'col2'];
  const qCols = targetCols.map((c) => `  ${quoteIdentifier(c, engineKey)}`);
  const qVals =
    cols.length > 0
      ? cols.map((c) => `  :${c.name} /* ${c.dataType} */`)
      : ['  :val1', '  :val2'];
  const snippet = `INSERT INTO ${quotedTable} (\n${qCols.join(',\n')}\n) VALUES (\n${qVals.join(',\n')}\n);\n`;
  insertGeneratedSqlSnippet(profile, dbName, snippet);
  setStatus(`已生成 ${tableName} INSERT 模板（支持参数绑定执行）`);
}

async function handleGenUpdate(profile: ConnectionProfile, dbName: string, tableName: string) {
  const cols = await ensureTableColumns(profile, dbName, tableName);
  const engineKey = engineKeyOf(profile);
  const quotedTable = quoteIdentifier(tableName, engineKey);
  const pks = cols.filter((c) => c.isPrimaryKey);
  const nonPks = cols.filter((c) => !c.isPrimaryKey);
  const setTargets = nonPks.length > 0 ? nonPks.slice(0, 6) : cols.slice(0, 4);
  const setLines =
    setTargets.length > 0
      ? setTargets.map((c) => `  ${quoteIdentifier(c.name, engineKey)} = :${c.name}`)
      : ['  col1 = :val1'];
  const whereClause =
    pks.length > 0
      ? pks.map((pk) => `${quoteIdentifier(pk.name, engineKey)} = :${pk.name}`).join(' AND ')
      : cols[0]
        ? `${quoteIdentifier(cols[0].name, engineKey)} = :${cols[0].name}`
        : 'id = :id';
  const snippet = `UPDATE ${quotedTable}\nSET\n${setLines.join(',\n')}\nWHERE ${whereClause};\n`;
  insertGeneratedSqlSnippet(profile, dbName, snippet);
  setStatus(`已生成 ${tableName} UPDATE 安全模板`);
}

async function handleCopyTableColumns(profile: ConnectionProfile, dbName: string, tableName: string) {
  const cols = await ensureTableColumns(profile, dbName, tableName);
  if (cols.length === 0) {
    setStatus(`未读取到表 ${tableName} 的字段列表`, true);
    return;
  }
  const text = cols.map((c) => c.name).join(', ');
  await copyText(text);
  setStatus(`已复制 ${tableName} 全部 ${cols.length} 个列名到剪贴板`);
}

async function handleRefreshProfile(profile: ConnectionProfile) {
  await loadDatabasesForProfile(profile, false);
  setStatus(`已刷新数据源「${profile.name}」的数据库列表`);
}

async function handleRefreshDatabase(profile: ConnectionProfile, dbName: string) {
  await loadTablesForDb(profile, dbName);
  setStatus(`已刷新 ${profile.name} / ${dbName} 表列表`);
}

async function handleRefreshTable(profile: ConnectionProfile, dbName: string, tableName: string) {
  await loadTableMetadata(profile, dbName, tableName);
  setStatus(`已刷新表 ${tableName} 字段与索引元数据`);
}

function handleDuplicateProfile(profile: ConnectionProfile) {
  const cloned: ConnectionProfile = {
    ...profile,
    id: '',
    name: `${profile.name} (副本)`,
  };
  openConnectionModal(cloned);
}

function handleNewConsoleForDb(profile: ConnectionProfile, dbName: string) {
  syncActiveProjectForProfile(profile);
  selectedProfileId.value = profile.id;
  selectedDbName.value = dbName;
  const seed = isRedisProfile(profile)
    ? `-- Redis 命令控制台: ${profile.name} (${dbName})\n-- 常用命令: PING | INFO keyspace | SCAN 0 COUNT 50 | GET key | HGETALL key | LRANGE key 0 99 | ZRANGE key 0 99 WITHSCORES\nSCAN 0 COUNT 50\n`
    : '';
  createNewConsoleTab(seed, profile.id, dbName);
  setStatus(`已新建查询控制台 -> ${profile.name} / ${dbName}`);
}

// ==================== High-Frequency Table Productivity Handlers ====================

function handleOpenCreateTable(profile: ConnectionProfile, dbName: string) {
  if (isRedisProfile(profile)) {
    setStatus('Redis 为键值数据库，请在控制台直接执行 SET / HSET / LPUSH / ZADD 创建键值', true);
    return;
  }
  syncActiveProjectForProfile(profile);
  selectedProfileId.value = profile.id;
  selectedDbName.value = dbName;
  toolDbContext.value = {
    profileId: profile.id,
    profileName: profile.name,
    profileEnv: profile.env,
    engineKey: engineKeyOf(profile),
    dbName,
  };
  activeToolModal.value = 'create-table';
}

function openCreateTableFromToolbar() {
  const prof = activeProfile.value ?? connections.value[0];
  if (!prof) {
    setStatus('请先创建并连接数据源，再使用可视化新建数据表', true);
    return;
  }
  const dbName = activeTab.value?.dbName || selectedDbName.value || databasesByProfile[prof.id]?.[0]?.name || '';
  if (!dbName) {
    setStatus('请先选择一个目标数据库，再使用可视化新建数据表', true);
    return;
  }
  handleOpenCreateTable(prof, dbName);
}

async function handleExportDbDdl(profile: ConnectionProfile, dbName: string) {
  if (isRedisProfile(profile)) {
    setStatus('Redis 不使用建表 DDL，可点击任意 Key 查看详细信息模板');
    return;
  }
  busy.value = true;
  try {
    let tables = tablesByKey[`${profile.id}:${dbName}`];
    if (!tables || tables.length === 0) {
      tables = await listTables(profile.id, dbName);
      tablesByKey[`${profile.id}:${dbName}`] = tables;
    }
    if (tables.length === 0) {
      setStatus(`数据库 ${dbName} 暂无可导出的表`, true);
      return;
    }
    const parts: string[] = [
      `-- Pacer 全量建表 DDL 导出`,
      `-- 数据源: ${profile.name} (${engineKeyOf(profile)})`,
      `-- 数据库: ${dbName}`,
      `-- 表数量: ${tables.length}`,
      `-- 导出时间: ${new Date().toLocaleString()}`,
      '',
    ];
    for (const tbl of tables) {
      parts.push(`-- ----------------------------`);
      parts.push(`-- Table structure for ${tbl.name}`);
      parts.push(`-- ----------------------------`);
      try {
        const ddl = await getTableDdl(profile.id, dbName, tbl.name);
        const trimmed = ddl.trim();
        parts.push(trimmed.endsWith(';') ? trimmed : `${trimmed};`);
      } catch (err) {
        parts.push(`-- [跳过 ${tbl.name}] 获取 DDL 失败: ${String(err)}`);
      }
      parts.push('');
    }
    createNewConsoleTab(parts.join('\n'), profile.id, dbName);
    if (activeTab.value) {
      activeTab.value.title = `${dbName}_ddl.sql`;
    }
    setStatus(`已导出 ${dbName} 共 ${tables.length} 张表的建表 DDL 到新标签页`);
  } catch (err) {
    setStatus(`导出全库 DDL 失败: ${String(err)}`, true);
  } finally {
    busy.value = false;
  }
}

async function handleOpenImportTableData(
  profile: ConnectionProfile,
  dbName: string,
  tableName: string,
) {
  const cols = await ensureTableColumns(profile, dbName, tableName);
  toolTableContext.value = {
    profileId: profile.id,
    profileName: profile.name,
    profileEnv: profile.env,
    engineKey: engineKeyOf(profile),
    dbName,
    tableName,
    columns: cols,
  };
  activeToolModal.value = 'import-data';
}

async function handleOpenGenEntityCode(
  profile: ConnectionProfile,
  dbName: string,
  tableName: string,
) {
  const cols = await ensureTableColumns(profile, dbName, tableName);
  toolTableContext.value = {
    profileId: profile.id,
    profileName: profile.name,
    profileEnv: profile.env,
    engineKey: engineKeyOf(profile),
    dbName,
    tableName,
    columns: cols,
  };
  activeToolModal.value = 'codegen';
}

async function handleGenMockData(
  profile: ConnectionProfile,
  dbName: string,
  tableName: string,
) {
  const cols = await ensureTableColumns(profile, dbName, tableName);
  if (cols.length === 0) {
    setStatus(`未能读取 ${tableName} 的字段定义，请先刷新元数据`, true);
    return;
  }
  const sql = generateMockInsertSql(tableName, dbName, cols, engineKeyOf(profile), 10);
  insertGeneratedSqlSnippet(profile, dbName, `${sql}\n`);
  setStatus(`已生成 ${tableName} 10 行 Mock INSERT 数据到编辑器`);
}

async function handleOpenRenameTable(
  profile: ConnectionProfile,
  dbName: string,
  tableName: string,
) {
  toolTableContext.value = {
    profileId: profile.id,
    profileName: profile.name,
    profileEnv: profile.env,
    engineKey: engineKeyOf(profile),
    dbName,
    tableName,
    columns: columnsByKey[`${profile.id}:${dbName}:${tableName}`] ?? [],
  };
  activeToolModal.value = 'rename-table';
}

async function handleTruncateTable(
  profile: ConnectionProfile,
  dbName: string,
  tableName: string,
) {
  const sql = buildTruncateTableSql(dbName, tableName, engineKeyOf(profile));
  await runStatementsWithGuard([sql], `清空 ${tableName}`, profile, dbName, true);
}

async function handleDropTable(
  profile: ConnectionProfile,
  dbName: string,
  tableName: string,
) {
  const sql = buildDropTableSql(dbName, tableName, engineKeyOf(profile));
  await runStatementsWithGuard(
    [sql],
    isRedisProfile(profile) ? `删除 Key ${tableName}` : `删除表 ${tableName}`,
    profile,
    dbName,
    true,
  );
}

async function handleRunToolStatements(
  statements: string[],
  label: string,
  profileId: string,
  dbName: string,
  refreshDb = false,
) {
  activeToolModal.value = 'none';
  const prof = connections.value.find((c) => c.id === profileId);
  if (!prof) {
    setStatus('未找到目标数据源连接', true);
    return;
  }
  await runStatementsWithGuard(statements, label, prof, dbName, true);
  if (refreshDb) {
    await loadTablesForDb(prof, dbName);
  }
}

function handleSendToolSqlToEditor(sql: string, profileId?: string, dbName?: string) {
  activeToolModal.value = 'none';
  const prof = profileId ? connections.value.find((c) => c.id === profileId) : activeProfile.value;
  if (prof && dbName) {
    insertGeneratedSqlSnippet(prof, dbName, `${sql}\n`);
  } else if (activeTab.value) {
    editorRef.value?.insertText(`${sql}\n`);
  } else {
    createNewConsoleTab(`${sql}\n`, profileId, dbName);
  }
  setStatus('已将生成的语句写入控制台编辑器');
}

// ==================== v0.9.6 High-Frequency Utilities ====================

function toggleFavorite(profile: ConnectionProfile, dbName: string, tableName: string) {
  const idx = favorites.value.findIndex(
    (f) => f.profileId === profile.id && f.dbName === dbName && f.tableName === tableName,
  );
  if (idx >= 0) {
    favorites.value.splice(idx, 1);
    setStatus(`已从常用收藏移除: ${dbName}.${tableName}`);
  } else {
    favorites.value.unshift({
      profileId: profile.id,
      dbName,
      tableName,
      isRedis: isRedisProfile(profile),
    });
    if (favorites.value.length > 60) {
      favorites.value.length = 60;
    }
    setStatus(`已收藏常用对象: ${dbName}.${tableName}`);
  }
  try {
    localStorage.setItem(FAVORITES_STORAGE_KEY, JSON.stringify(favorites.value));
  } catch {
    // non-fatal
  }
}

async function handleTestAllConnections() {
  if (batchTesting.value) return;
  const projectConns = connectionsInProjectById(activeProjectId.value);
  const targets = projectConns.length > 0 ? projectConns : connections.value;
  if (targets.length === 0) {
    setStatus('当前项目暂无可测试的数据源连接', true);
    return;
  }
  batchTesting.value = true;
  let okCount = 0;
  let errCount = 0;
  try {
    for (const c of targets) {
      setConnHealth(c.id, 'testing', '正在批量检测...');
    }
    const batchSize = 4;
    for (let i = 0; i < targets.length; i += batchSize) {
      const slice = targets.slice(i, i + batchSize);
      await Promise.all(
        slice.map(async (prof) => {
          try {
            const ping = await testConnection(prof.id);
            if (ping.success) {
              okCount += 1;
              setConnHealth(prof.id, 'ok', ping.serverVersion || ping.message, ping.latencyMs);
            } else {
              errCount += 1;
              setConnHealth(prof.id, 'error', ping.message, ping.latencyMs);
            }
          } catch (err) {
            errCount += 1;
            setConnHealth(prof.id, 'error', String(err));
          }
        }),
      );
    }
    setStatus(`批量测活完成：${okCount} 个连接正常，${errCount} 个异常`, errCount > 0 && okCount === 0);
  } finally {
    batchTesting.value = false;
  }
}

function resolveActiveDbTarget(
  explicitProfile?: ConnectionProfile | null,
  explicitDb?: string,
): { profile: ConnectionProfile; dbName: string } | null {
  const prof = explicitProfile ?? activeProfile.value ?? connections.value[0];
  if (!prof) {
    setStatus('请先创建或选择一个数据源连接', true);
    return null;
  }
  const dbName =
    explicitDb ||
    (prof.id === activeTab.value?.profileId ? activeTab.value?.dbName : '') ||
    (prof.id === selectedProfileId.value ? selectedDbName.value : '') ||
    databasesByProfile[prof.id]?.[0]?.name ||
    (isRedisProfile(prof) ? 'db0' : '');
  return { profile: prof, dbName };
}

function handleOpenObjectSearch(profile?: ConnectionProfile, dbName?: string) {
  const target = resolveActiveDbTarget(profile, dbName);
  advancedInitialProfileId.value = target?.profile.id ?? connections.value[0]?.id ?? '';
  advancedInitialDbName.value = target?.dbName ?? '';
  advancedInitialTableName.value = '';
  advancedModalKind.value = 'object-search';
}

function handleOpenSchemaDiff(
  profile?: ConnectionProfile,
  dbName?: string,
  tableName?: string,
) {
  const target = resolveActiveDbTarget(profile, dbName);
  advancedInitialProfileId.value = target?.profile.id ?? '';
  advancedInitialDbName.value = target?.dbName ?? '';
  advancedInitialTableName.value = tableName ?? inspectedTable.value?.tableName ?? '';
  advancedModalKind.value = 'schema-diff';
}

function handleOpenAlterTable(profile: ConnectionProfile, dbName: string, tableName: string) {
  if (isRedisProfile(profile)) {
    void handleOpenRedisEditKey(profile, dbName, tableName);
    return;
  }
  syncActiveProjectForProfile(profile);
  advancedInitialProfileId.value = profile.id;
  advancedInitialDbName.value = dbName;
  advancedInitialTableName.value = tableName;
  advancedModalKind.value = 'alter-table';
}

async function runSimpleCollect(
  profileId: string,
  dbName: string,
  sql: string,
): Promise<Array<Record<string, unknown>>> {
  const rows: Array<Record<string, unknown>> = [];
  await executeSql(profileId, dbName, sql, 500, (chunk) => {
    rows.push(...chunk.rows);
  });
  return rows;
}

function handleOpenRedisNewKey(profile: ConnectionProfile, dbName: string) {
  syncActiveProjectForProfile(profile);
  advancedInitialProfileId.value = profile.id;
  advancedInitialDbName.value = dbName || 'db0';
  advancedInitialTableName.value = '';
  redisEditorInit.value = {
    key: 'app:new:key',
    keyType: 'string',
    ttlSeconds: -1,
    stringValue: '{"createdBy":"Pacer"}',
    hashEntries: [{ field: 'name', value: 'pacer' }],
    listItems: ['item_1'],
    setMembers: ['member_1'],
    zsetMembers: [{ member: 'member_1', score: '100' }],
    isNew: true,
  };
  advancedModalKind.value = 'redis-editor';
}

async function handleOpenRedisEditKey(
  profile: ConnectionProfile,
  dbName: string,
  keyName: string,
) {
  syncActiveProjectForProfile(profile);
  busy.value = true;
  try {
    const qk = quoteRedisArg(keyName);
    const [typeRows, ttlRows] = await Promise.all([
      runSimpleCollect(profile.id, dbName, `TYPE ${qk}`),
      runSimpleCollect(profile.id, dbName, `TTL ${qk}`),
    ]);
    const rawType = String(typeRows[0]?.result ?? 'string').toLowerCase();
    const keyType: RedisEditableType =
      rawType === 'hash' || rawType === 'list' || rawType === 'set' || rawType === 'zset'
        ? rawType
        : 'string';
    const rawTtl = Number(ttlRows[0]?.result ?? -1);
    const ttlSeconds = Number.isFinite(rawTtl) ? rawTtl : -1;

    const init: RedisEditorInitialData = {
      key: keyName,
      keyType,
      ttlSeconds,
      isNew: false,
    };

    if (keyType === 'string') {
      const valRows = await runSimpleCollect(profile.id, dbName, `GET ${qk}`);
      init.stringValue = String(valRows[0]?.result ?? '');
    } else if (keyType === 'hash') {
      const hRows = await runSimpleCollect(profile.id, dbName, `HGETALL ${qk}`);
      init.hashEntries = hRows.map((r) => ({
        field: String(r.field ?? ''),
        value: String(r.value ?? ''),
      }));
    } else if (keyType === 'list') {
      const lRows = await runSimpleCollect(profile.id, dbName, `LRANGE ${qk} 0 499`);
      init.listItems = lRows.map((r) => String(r.value ?? r.element ?? Object.values(r)[0] ?? ''));
    } else if (keyType === 'set') {
      const sRows = await runSimpleCollect(profile.id, dbName, `SMEMBERS ${qk}`);
      init.setMembers = sRows.map((r) => String(r.value ?? r.member ?? Object.values(r)[0] ?? ''));
    } else if (keyType === 'zset') {
      const zRows = await runSimpleCollect(profile.id, dbName, `ZRANGE ${qk} 0 499 WITHSCORES`);
      init.zsetMembers = zRows.map((r) => ({
        member: String(r.member ?? ''),
        score: String(r.score ?? '0'),
      }));
    }

    advancedInitialProfileId.value = profile.id;
    advancedInitialDbName.value = dbName || 'db0';
    advancedInitialTableName.value = keyName;
    redisEditorInit.value = init;
    advancedModalKind.value = 'redis-editor';
  } catch (err) {
    setStatus(`读取 Redis Key (${keyName}) 内容失败: ${String(err)}`, true);
  } finally {
    busy.value = false;
  }
}

async function handleDumpSql(
  profile: ConnectionProfile,
  dbName: string,
  tableNames: string[] | null,
  includeData: boolean,
) {
  if (isRedisProfile(profile)) {
    setStatus('Redis 不支持导出 SQL 转储文件，请在结果网格中导出 XLSX/JSON/CSV', true);
    return;
  }
  const baseName =
    tableNames && tableNames.length === 1
      ? `${tableNames[0]}${includeData ? '_full' : '_schema'}.sql`
      : `${dbName || profile.name}${includeData ? '_full' : '_schema'}.sql`;
  const safeFileName = baseName.replace(/[^\w.\-\u4e00-\u9fa5]/g, '_');
  const savePath = await pickSaveSqlDumpPath(safeFileName);
  if (!savePath) return;

  busy.value = true;
  try {
    const res = await exportSqlDump({
      profileId: profile.id,
      dbName,
      tableNames,
      includeData,
      maxRowsPerTable: 20000,
      path: savePath,
    });
    const truncNote =
      res.truncatedTables.length > 0
        ? `（单表上限 20,000 行截断: ${res.truncatedTables.join(', ')}）`
        : '';
    setStatus(`SQL 转储完成：${res.tableCount} 张表，${res.rowCount} 行数据 -> ${res.path}${truncNote}`);
    pushProjectLog({
      projectId: profile.projectId,
      profileId: profile.id,
      profileName: profile.name,
      dbName,
      level: 'success',
      summary: `已转储 SQL 文件 (${res.tableCount} 表, ${res.rowCount} 行)${truncNote}`,
      detail: res.path,
    });
  } catch (err) {
    setStatus(`转储 SQL 文件失败: ${String(err)}`, true);
  } finally {
    busy.value = false;
  }
}

async function handleRunLocalSqlFile(profile: ConnectionProfile, dbName: string) {
  const path = await pickSqlScriptFile();
  if (!path) return;
  try {
    const opened = await openTextFile(path);
    const content = opened.content.trim();
    if (!content) {
      setStatus('所选 SQL 脚本文件为空', true);
      return;
    }
    const isRedis = isRedisProfile(profile);
    let stmts: string[];
    if (isRedis) {
      stmts = splitRedisCommands(content);
    } else {
      const analysis = await analyzeSql(content, profile.dbType);
      stmts =
        analysis.statements.length > 0
          ? analysis.statements.map((s) => s.sql.trim()).filter(Boolean)
          : [content];
    }
    if (stmts.length === 0) {
      setStatus('未解析出可执行语句', true);
      return;
    }
    await runStatementsWithGuard(stmts, `运行脚本 ${opened.name}`, profile, dbName, true);
  } catch (err) {
    setStatus(`读取或执行本地 SQL 文件失败: ${String(err)}`, true);
  }
}

function handleOpenServerMonitor(profile?: ConnectionProfile) {
  const prof = profile ?? activeProfile.value ?? connections.value[0] ?? null;
  if (!prof) {
    setStatus('请先创建并选择数据源，再打开服务器监控', true);
    return;
  }
  serverMonitorProfile.value = prof;
  serverMonitorVisible.value = true;
}

async function handleKillServerSession(sql: string, label: string, profile: ConnectionProfile) {
  await runStatementsWithGuard([sql], label, profile, '', true);
}

async function handleOpenErDiagram(profile?: ConnectionProfile, dbName?: string) {
  const target = resolveActiveDbTarget(profile, dbName);
  if (!target || !target.dbName) {
    setStatus('请先选择一个关系型数据库后再查看 ER 关系图', true);
    return;
  }
  if (isRedisProfile(target.profile)) {
    setStatus('Redis 为键值数据库，不适用 ER 表关系图', true);
    return;
  }
  syncActiveProjectForProfile(target.profile);
  busy.value = true;
  try {
    const tKey = `${target.profile.id}:${target.dbName}`;
    let tables = tablesByKey[tKey];
    if (!tables || tables.length === 0) {
      tables = await listTables(target.profile.id, target.dbName);
      tablesByKey[tKey] = tables;
    }
    const toFetch = tables
      .slice(0, 48)
      .filter(
        (tb) =>
          !columnsByKey[`${target.profile.id}:${target.dbName}:${tb.name}`] ||
          !foreignKeysByKey[`${target.profile.id}:${target.dbName}:${tb.name}`],
      );
    for (let i = 0; i < toFetch.length; i += 6) {
      const batch = toFetch.slice(i, i + 6);
      await Promise.all(
        batch.map(async (tb) => {
          const ck = `${target.profile.id}:${target.dbName}:${tb.name}`;
          const [colsRes, fksRes] = await Promise.all([
            columnsByKey[ck]
              ? Promise.resolve(columnsByKey[ck])
              : listColumns(target.profile.id, target.dbName, tb.name).catch(() => [] as ColumnMeta[]),
            foreignKeysByKey[ck]
              ? Promise.resolve(foreignKeysByKey[ck])
              : listForeignKeys(target.profile.id, target.dbName, tb.name).catch(() => [] as ForeignKeyMeta[]),
          ]);
          columnsByKey[ck] = colsRes;
          foreignKeysByKey[ck] = fksRes;
        }),
      );
    }
    erDiagramProfile.value = target.profile;
    erDiagramDbName.value = target.dbName;
    erDiagramVisible.value = true;
  } catch (err) {
    setStatus(`加载 ER 关系图元数据失败: ${String(err)}`, true);
  } finally {
    busy.value = false;
  }
}

async function handleRunAdvancedStatements(
  statements: string[],
  label: string,
  profileId: string,
  dbName: string,
  refreshDb = false,
) {
  const editedTable = advancedInitialTableName.value;
  advancedModalKind.value = 'none';
  const prof = connections.value.find((c) => c.id === profileId);
  if (!prof) {
    setStatus('未找到目标数据源连接', true);
    return;
  }
  await runStatementsWithGuard(statements, label, prof, dbName, true);
  if (refreshDb) {
    if (editedTable) {
      delete columnsByKey[`${prof.id}:${dbName}:${editedTable}`];
      delete indexesByKey[`${prof.id}:${dbName}:${editedTable}`];
    }
    await loadTablesForDb(prof, dbName);
    if (editedTable && !isRedisProfile(prof)) {
      await loadTableMetadata(prof, dbName, editedTable);
    }
  }
}

function handleSendAdvancedSqlToEditor(sql: string, profileId?: string, dbName?: string) {
  advancedModalKind.value = 'none';
  handleSendToolSqlToEditor(sql, profileId, dbName);
}

function handleJumpQueryFromTool(profileId: string, dbName: string, tableName: string) {
  const prof = connections.value.find((c) => c.id === profileId);
  if (prof) {
    void handleQueryTop100(prof, dbName, tableName);
  }
}

function handleJumpStructureFromTool(profileId: string, dbName: string, tableName: string) {
  const prof = connections.value.find((c) => c.id === profileId);
  if (prof) {
    void selectTableNode(prof, dbName, tableName);
  }
}

// ==================== SQL Execution Engine (with High-Risk Guard) ====================

async function runActiveEditor(runAll: boolean) {
  const tab = activeTab.value;
  if (!tab) return;
  const prof = activeProfile.value;
  if (!prof) {
    setStatus('请先在顶部工具栏选择一个目标数据源', true);
    return;
  }
  await executeEditorTabSource(tab, prof, runAll);
}

async function runSplitEditor(runAll: boolean) {
  const tab = splitTab.value;
  if (!tab) return;
  const prof = connections.value.find((c) => c.id === tab.profileId) ?? activeProfile.value;
  if (!prof) {
    setStatus('请先为分屏窗口选择目标数据源', true);
    return;
  }
  await executeEditorTabSource(tab, prof, runAll);
}

async function executeEditorTabSource(tab: EditorTab, prof: ConnectionProfile, runAll: boolean) {
  const isRedis = isRedisProfile(prof);

  // 1. Selected text ALWAYS has highest priority (works in .md, .sql, .txt!)
  const selected = tab.cursor.selectedText.trim();
  if (!runAll && selected) {
    const stmts = isRedis ? splitRedisCommands(selected) : [selected];
    await runStatementsWithGuard(
      stmts.length > 0 ? stmts : [selected],
      isRedis ? '选中 Redis 命令' : '选中 SQL',
      prof,
      tab.dbName,
    );
    return;
  }

  // 2. Markdown mode: execute enclosing SQL block or all fenced SQL blocks
  if (tab.fileKind === 'markdown') {
    if (runAll) {
      const blocks = extractMarkdownSqlBlocksLocal(tab.content);
      if (blocks.length === 0) {
        setStatus(t('mdNoBlocks'), true);
        return;
      }
      await runStatementsWithGuard(
        blocks.map((b) => b.sql),
        tab.title,
        prof,
        tab.dbName,
      );
      return;
    }
    const blocks = extractMarkdownSqlBlocksLocal(tab.content);
    const block =
      blocks.find((b) => tab.cursor.line >= b.startLine && tab.cursor.line <= b.endLine) ?? null;
    if (block) {
      await runStatementsWithGuard(
        [block.sql],
        block.title ? `${block.title}` : `SQL #${block.index + 1}`,
        prof,
        tab.dbName,
      );
      return;
    }
    setStatus('请先选中要执行的 SQL 文本，或将光标移入 ```sql 代码块内，或点击代码块上方「▶ 执行」', true);
    return;
  }

  // 3. Redis console: line-oriented commands without semicolons
  if (isRedis) {
    if (!runAll) {
      const cmd = extractRedisCommandAtLine(tab.content, tab.cursor.line) ?? tab.content.trim();
      if (!cmd) {
        setStatus('当前编辑器为空，无可执行 Redis 命令（如 GET user:1001、SCAN 0 COUNT 50）', true);
        return;
      }
      await runStatementsWithGuard([cmd], tab.title, prof, tab.dbName);
    } else {
      const cmds = splitRedisCommands(tab.content);
      if (cmds.length === 0) {
        setStatus('当前 Redis 控制台为空', true);
        return;
      }
      await runStatementsWithGuard(cmds, tab.title, prof, tab.dbName);
    }
    return;
  }

  // 4. Regular SQL console/file: single statement at cursor or full script
  const engineKey = engineKeyOf(prof);
  if (!runAll) {
    const stmt = await extractStatementAtCursor(tab.content, tab.cursor.cursorOffset);
    const toRun = (stmt ?? tab.content).trim();
    if (!toRun) {
      setStatus('当前编辑器为空，无可执行的 SQL 语句', true);
      return;
    }
    await runStatementsWithGuard([toRun], tab.title, prof, tab.dbName);
  } else {
    const full = tab.content.trim();
    if (!full) {
      setStatus('当前脚本为空', true);
      return;
    }
    const analysis = await analyzeSql(full, engineKey);
    const stmts =
      analysis.statements.length > 0
        ? analysis.statements.map((s) => s.sql.trim()).filter(Boolean)
        : [full];
    await runStatementsWithGuard(stmts, tab.title, prof, tab.dbName);
  }
}

async function runMarkdownBlock(block: MarkdownSqlBlock) {
  const prof = activeProfile.value;
  if (!prof) {
    setStatus('请先选择目标数据源再执行 Markdown SQL 代码块', true);
    return;
  }
  const title = block.title ? `#${block.index + 1} ${block.title}` : `SQL 块 #${block.index + 1}`;
  await runStatementsWithGuard([block.sql], title, prof, activeTab.value?.dbName ?? '');
}

async function explainMarkdownBlock(block: MarkdownSqlBlock) {
  await runExplainSql(block.sql);
}

function openMarkdownBlockInConsole(block: MarkdownSqlBlock) {
  const header = block.title ? `-- 来源文档: ${activeTab.value?.title} · ${block.title}\n` : '';
  createNewConsoleTab(`${header}${block.sql}\n`);
}

async function runStatementsWithGuard(
  statements: string[],
  label: string,
  profile: ConnectionProfile,
  dbName: string,
  skipParams = false,
) {
  const isRedis = isRedisProfile(profile);
  const joined = statements.join(isRedis ? '\n' : ';\n');

  // Redis keys naturally contain colons (`user:1001`); never trigger SQL named-parameter prompt for Redis.
  if (!skipParams && !isRedis) {
    const params = extractSqlParameters(joined);
    if (params.length > 0) {
      paramContext.value = {
        profileName: profile.name,
        dbName,
        statements,
        params,
      };
      pendingParamExec.value = { label, profile, dbName };
      activeModal.value = 'params';
      return;
    }
  }

  let analysis: ScriptAnalysis | null = null;
  try {
    analysis = await analyzeSql(joined, profile.dbType);
    astAnalysis.value = analysis;
  } catch {
    analysis = null;
  }

  const highWarnings =
    analysis?.statements.flatMap((s) =>
      s.warnings.filter((w) => w.level === 'HIGH' || w.level === 'CRITICAL'),
    ) ?? [];

  const joinedUpper = joined.toUpperCase();
  for (const plg of localPlugins.value) {
    if (!plg.enabled) continue;
    for (const rule of plg.sqlRules) {
      const pat = rule.pattern.trim().toUpperCase();
      if (pat && joinedUpper.includes(pat)) {
        highWarnings.push({
          code: rule.code,
          level: rule.level === 'CRITICAL' ? 'CRITICAL' : 'HIGH',
          message: `[插件 ${plg.name}] ${rule.message}`,
          statementIndex: 0,
        });
      }
    }
  }

  const isProdWrite =
    profile.env === 'PROD' &&
    analysis !== null &&
    (analysis.overallKind === 'WRITE_DML' ||
      analysis.overallKind === 'WRITE_DDL' ||
      analysis.overallKind === 'ADMIN_OR_DCL');

  if (highWarnings.length > 0 || isProdWrite) {
    const warnings = [...highWarnings];
    if (isProdWrite && warnings.length === 0) {
      warnings.push({
        code: 'PROD_WRITE_GUARD',
        level: 'HIGH',
        message: '目标数据源已标记为 PROD（生产环境），写操作需要人工二次确认。',
        statementIndex: 0,
      });
    }
    dangerContext.value = {
      profileName: profile.name,
      env: profile.env,
      dbName,
      sql: joined,
      warnings: warnings.map((w) => ({ code: w.code, level: w.level, message: w.message })),
    };
    pendingDangerAction.value = () => executeStatementsBatch(statements, label, profile, dbName);
    activeModal.value = 'danger';
    return;
  }

  await executeStatementsBatch(statements, label, profile, dbName);
}

async function handleConfirmParams(boundStatements: string[]) {
  activeModal.value = 'none';
  const meta = pendingParamExec.value;
  pendingParamExec.value = null;
  if (!meta) return;
  await runStatementsWithGuard(boundStatements, meta.label, meta.profile, meta.dbName, true);
}

async function handleRunParamsRaw(rawStatements: string[]) {
  activeModal.value = 'none';
  const meta = pendingParamExec.value;
  pendingParamExec.value = null;
  if (!meta) return;
  await runStatementsWithGuard(rawStatements, meta.label, meta.profile, meta.dbName, true);
}

async function confirmDangerExecution() {
  activeModal.value = 'none';
  if (pendingDangerAction.value) {
    const fn = pendingDangerAction.value;
    pendingDangerAction.value = null;
    await fn();
  }
}

function engineKeyOf(profile: ConnectionProfile): string {
  return typeof profile.dbType === 'string'
    ? profile.dbType
    : Object.values(profile.dbType)[0] ?? 'sqlite';
}

function isProfileReadOnly(profile: ConnectionProfile): boolean {
  const cfg = (profile.mode?.config ?? {}) as Partial<DirectConfig>;
  return Boolean(cfg.readOnly) || !profile.capabilities.canDml;
}

async function resolveTablePrimaryKeys(
  profile: ConnectionProfile,
  dbName: string,
  tableName: string,
): Promise<string[]> {
  const cKey = `${profile.id}:${dbName}:${tableName}`;
  let cols = columnsByKey[cKey];
  if (!cols && dbName) {
    try {
      cols = await listColumns(profile.id, dbName, tableName);
      columnsByKey[cKey] = cols;
    } catch {
      cols = [];
    }
  }
  return (cols ?? []).filter((c) => c.isPrimaryKey).map((c) => c.name);
}

async function resolveTableForeignKeys(
  profile: ConnectionProfile,
  dbName: string,
  tableName: string,
): Promise<ForeignKeyMeta[]> {
  const k = `${profile.id}:${dbName}:${tableName}`;
  let fks = foreignKeysByKey[k];
  if (!fks && dbName) {
    try {
      fks = await listForeignKeys(profile.id, dbName, tableName);
      foreignKeysByKey[k] = fks;
    } catch {
      fks = [];
    }
  }
  return fks ?? [];
}

function startQueryTimer(): string {
  stopQueryTimer();
  const qid = `q-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
  activeQueryId.value = qid;
  queryElapsedMs.value = 0;
  const startedAt = Date.now();
  queryTimerHandle = window.setInterval(() => {
    queryElapsedMs.value = Date.now() - startedAt;
  }, 100);
  return qid;
}

function stopQueryTimer() {
  if (queryTimerHandle !== null) {
    window.clearInterval(queryTimerHandle);
    queryTimerHandle = null;
  }
  activeQueryId.value = null;
}

async function handleCancelRunningQuery() {
  const qid = activeQueryId.value;
  if (!qid) return;
  try {
    const ok = await cancelQuery(qid);
    if (ok) {
      setStatus('已发送停止查询指令，正在终止当前执行...');
      pushProjectLog({
        projectId: activeProjectId.value,
        profileId: activeProfile.value?.id,
        profileName: activeProfile.value?.name || '查询控制',
        level: 'warn',
        summary: '用户中止了正在执行的查询',
      });
    }
  } catch (err) {
    setStatus(`取消查询失败: ${String(err)}`, true);
  }
}

async function executeStatementsBatch(
  statements: string[],
  label: string,
  profile: ConnectionProfile,
  dbName: string,
  explicitTableName?: string,
) {
  busy.value = true;
  queryExecuting.value = true;
  const queryId = startQueryTimer();
  activeServiceTab.value = 'result';
  bottomVisible.value = true;
  const newTabs: ResultTabItem[] = [];
  let totalRowsAll = 0;
  let totalMsAll = 0;
  let lastError: string | null = null;
  const targetProjectId = profile.projectId || activeProjectId.value;
  const availableTables = (tablesByKey[`${profile.id}:${dbName}`] ?? []).map((t) => t.name);
  const engineKey = engineKeyOf(profile);
  const readOnly = isProfileReadOnly(profile);
  const isRedis = isRedisProfile(profile);

  if (!explicitTableName && activeTab.value?.path) {
    saveFileDbBinding(activeTab.value.path, profile.id, dbName, targetProjectId);
  }

  try {
    for (let idx = 0; idx < statements.length; idx += 1) {
      const sql = statements[idx].trim();
      if (!sql) continue;
      const tabId = `res-${Date.now()}-${idx}`;
      const tabTitle = statements.length > 1 ? `${label} #${idx + 1}` : label;
      const detected = isRedis
        ? null
        : explicitTableName
          ? { db: dbName, table: explicitTableName }
          : extractSingleTableTarget(sql);
      const effectiveDb = detected?.db || dbName;
      const sourceTable = isRedis ? null : (detected?.table ?? null);

      // Kick off primary key & FK metadata lookup in parallel so wide queries never stall waiting for SHOW COLUMNS.
      const pkPromise =
        !isRedis && sourceTable && effectiveDb
          ? resolveTablePrimaryKeys(profile, effectiveDb, sourceTable).catch(() => [] as string[])
          : Promise.resolve([] as string[]);
      const fkPromise =
        !isRedis && sourceTable && effectiveDb
          ? resolveTableForeignKeys(profile, effectiveDb, sourceTable).catch(
              () => [] as ForeignKeyMeta[],
            )
          : Promise.resolve([] as ForeignKeyMeta[]);

      const rawColumns: string[] = [];
      const rawRows: Record<string, unknown>[] = [];

      pushProjectLog({
        projectId: targetProjectId,
        profileId: profile.id,
        profileName: profile.name,
        dbName: effectiveDb || undefined,
        level: 'sql',
        summary: isRedis ? '执行 Redis 命令' : '执行 SQL',
        detail: sql,
      });

      const summary = await executeSql(
        profile.id,
        effectiveDb || null,
        sql,
        maxRowsLimit.value,
        (chunk) => {
          if (rawColumns.length === 0 && chunk.columns.length > 0) {
            rawColumns.push(...chunk.columns);
          }
          if (chunk.rows.length > 0) {
            rawRows.push(...chunk.rows);
          }
        },
        queryId,
      );

      let finalCols = rawColumns;
      if (
        !isRedis &&
        summary.success &&
        !summary.isDmlOrDdl &&
        finalCols.length === 0 &&
        sourceTable &&
        effectiveDb &&
        (explicitTableName ||
          /^\s*select\s+(?:distinct\s+)?(?:\*|[a-z0-9_$`"\[\]]+\.\*)\s+from\b/i.test(sql))
      ) {
        const cKey = `${profile.id}:${effectiveDb}:${sourceTable}`;
        let metaCols = columnsByKey[cKey];
        if (!metaCols) {
          try {
            metaCols = await listColumns(profile.id, effectiveDb, sourceTable);
            columnsByKey[cKey] = metaCols;
          } catch {
            metaCols = [];
          }
        }
        if (metaCols && metaCols.length > 0) {
          finalCols = metaCols.map((c) => c.name);
        }
      }

      const [primaryKeys, foreignKeys] = await Promise.all([pkPromise, fkPromise]);
      const normalizedPageSize = [50, 100, 200, 500, 1000].includes(maxRowsLimit.value ?? 100)
        ? (maxRowsLimit.value ?? 100)
        : 100;
      const item: ResultTabItem = {
        id: tabId,
        title: tabTitle,
        sql,
        profileId: profile.id,
        profileName: profile.name,
        profileEnv: profile.env,
        engineKey,
        dbName: effectiveDb,
        readOnly: readOnly || isRedis,
        sourceTable,
        availableTables: isRedis ? [] : availableTables,
        primaryKeys,
        foreignKeys,
        columns: markRaw(finalCols),
        rows: markRaw(rawRows),
        summary,
        serverWhere: '',
        serverOrderBy: '',
        pageIndex: 0,
        pageSize: normalizedPageSize,
        totalRowCount: null,
      };

      totalMsAll += summary.durationMs;
      totalRowsAll += summary.totalRows;

      if (summary.success) {
        setConnHealth(profile.id, 'ok', summary.routeDescription || undefined, summary.durationMs);
        pushProjectLog({
          projectId: targetProjectId,
          profileId: profile.id,
          profileName: profile.name,
          dbName: effectiveDb || undefined,
          level: 'success',
          durationMs: summary.durationMs,
          summary: `${
            summary.isDmlOrDdl
              ? `执行完成，影响 ${summary.affectedRows ?? 0} 行`
              : `查询成功，返回 ${summary.totalRows} 行`
          } (${summary.durationMs} ms)`,
        });
      } else {
        lastError = summary.errorMessage ?? '执行失败';
        if (looksLikeConnectionFailure(lastError)) {
          setConnHealth(profile.id, 'error', lastError);
        }
        pushProjectLog({
          projectId: targetProjectId,
          profileId: profile.id,
          profileName: profile.name,
          dbName: effectiveDb || undefined,
          level: 'error',
          durationMs: summary.durationMs,
          summary: `SQL 执行失败 (${summary.durationMs} ms)`,
          detail: lastError,
        });
      }
      newTabs.push(item);
    }

    if (newTabs.length > 0) {
      resultTabs.value = mergeResultTabsWithPins(newTabs);
      resultGridRef.value?.pruneTabState(resultTabs.value.map((r) => r.id));
      activeResultTabId.value = newTabs[0].id;
      activeServiceTab.value = lastError && newTabs[0].columns.length === 0 ? 'output' : 'result';
    }

    if (lastError) {
      setStatus(`执行异常: ${lastError}`, true);
    } else {
      setStatus(
        `执行成功 · ${newTabs.length} 条${isRedis ? '命令' : '语句'} · ${totalRowsAll} 行 · ${totalMsAll} ms (${profile.name})`,
      );
      if (
        dbName &&
        statements.some((s) =>
          /^(create|drop|alter|rename|truncate|del|unlink|flushdb|flushall)\b/i.test(s.trim()),
        )
      ) {
        void loadTablesForDb(profile, dbName);
      }
    }
    void refreshHistory();
  } catch (err) {
    const msg = String(err);
    if (looksLikeConnectionFailure(msg)) {
      setConnHealth(profile.id, 'error', msg);
    }
    pushProjectLog({
      projectId: targetProjectId,
      profileId: profile.id,
      profileName: profile.name,
      dbName: dbName || undefined,
      level: 'error',
      summary: 'SQL 执行异常',
      detail: msg,
    });
    activeServiceTab.value = 'output';
    setStatus(`执行 SQL 出错: ${msg}`, true);
  } finally {
    stopQueryTimer();
    queryExecuting.value = false;
    busy.value = false;
    if (!isRedis) {
      void refreshTransactionStatus(profile.id);
    }
  }
}

async function handleServerQueryPage(
  tab: ResultTabItem,
  opts: { where: string; orderBy: string; pageIndex: number; pageSize: number },
) {
  if (!tab.sourceTable) return;
  const prof = connections.value.find((c) => c.id === tab.profileId);
  if (!prof) {
    setStatus('该结果集关联的数据源已不存在', true);
    return;
  }
  const sql = buildTablePageQuerySql({
    dbName: tab.dbName,
    tableName: tab.sourceTable,
    engineKey: tab.engineKey,
    whereClause: opts.where,
    orderByClause: opts.orderBy,
    limit: opts.pageSize,
    offset: opts.pageIndex * opts.pageSize,
  });
  busy.value = true;
  queryExecuting.value = true;
  const queryId = startQueryTimer();
  try {
    const rawCols: string[] = [];
    const rawRows: Record<string, unknown>[] = [];
    const summary = await executeSql(
      prof.id,
      tab.dbName || null,
      sql,
      opts.pageSize,
      (chunk) => {
        if (rawCols.length === 0 && chunk.columns.length > 0) {
          rawCols.push(...chunk.columns);
        }
        if (chunk.rows.length > 0) {
          rawRows.push(...chunk.rows);
        }
      },
      queryId,
    );
    const target = resultTabs.value.find((r) => r.id === tab.id) ?? tab;
    if (summary.success) {
      if (rawCols.length > 0) {
        target.columns = markRaw(rawCols);
      }
      target.rows = markRaw(rawRows);
      target.summary = summary;
      target.sql = `${sql};`;
      if ((target.serverWhere ?? '') !== opts.where) {
        target.totalRowCount = null;
      }
      target.serverWhere = opts.where;
      target.serverOrderBy = opts.orderBy;
      target.pageIndex = opts.pageIndex;
      target.pageSize = opts.pageSize;
      setStatus(
        `分页筛选完成 · 第 ${opts.pageIndex + 1} 页 · ${rawRows.length} 行 · ${summary.durationMs} ms`,
      );
    } else {
      setStatus(`服务端筛选失败: ${summary.errorMessage || '未知错误'}`, true);
    }
  } catch (err) {
    setStatus(`服务端分页查询异常: ${String(err)}`, true);
  } finally {
    stopQueryTimer();
    queryExecuting.value = false;
    busy.value = false;
  }
}

async function handleServerCountRows(tab: ResultTabItem) {
  if (!tab.sourceTable) return;
  const prof = connections.value.find((c) => c.id === tab.profileId);
  if (!prof) return;
  const countSql = buildTableCountQuerySql({
    dbName: tab.dbName,
    tableName: tab.sourceTable,
    engineKey: tab.engineKey,
    whereClause: tab.serverWhere,
  });
  busy.value = true;
  try {
    const rows: Record<string, unknown>[] = [];
    const summary = await executeSql(prof.id, tab.dbName || null, countSql, 5, (chunk) => {
      if (chunk.rows.length > 0) rows.push(...chunk.rows);
    });
    if (summary.success && rows.length > 0) {
      const firstVal = Object.values(rows[0])[0];
      const countNum = Number(firstVal ?? 0);
      const target = resultTabs.value.find((r) => r.id === tab.id) ?? tab;
      target.totalRowCount = Number.isFinite(countNum) ? countNum : 0;
      setStatus(`表 ${tab.sourceTable} 匹配总行数: ${target.totalRowCount.toLocaleString()} 条`);
    } else if (!summary.success) {
      setStatus(`统计总行数失败: ${summary.errorMessage || '未知错误'}`, true);
    }
  } catch (err) {
    setStatus(`统计总行数出错: ${String(err)}`, true);
  } finally {
    busy.value = false;
  }
}

async function handleJumpForeignKey(tab: ResultTabItem, fk: ForeignKeyMeta, cellValue: unknown) {
  const prof = connections.value.find((c) => c.id === tab.profileId);
  if (!prof) {
    setStatus('未找到当前结果集的数据源', true);
    return;
  }
  const whereClause = `${quoteIdentifier(fk.referencedColumn, tab.engineKey)} = ${formatSqlLiteral(cellValue)}`;
  const sql = `${buildTablePageQuerySql({
    dbName: tab.dbName,
    tableName: fk.referencedTable,
    engineKey: tab.engineKey,
    whereClause,
    limit: 100,
    offset: 0,
  })};`;
  await executeStatementsBatch(
    [sql],
    `${fk.referencedTable} ← ${fk.columnName}`,
    prof,
    tab.dbName,
    fk.referencedTable,
  );
}

async function handleToggleTransactionMode() {
  const prof = activeProfile.value;
  if (!prof || isRedisProfile(prof)) return;
  const nextManual = !activeTxStatus.value.manualMode;
  busy.value = true;
  try {
    const st = await setTransactionMode(prof.id, nextManual);
    txStatusByProfile[prof.id] = st;
    setStatus(
      nextManual
        ? `已为「${prof.name}」开启手动事务模式 (Manual Tx)`
        : `已为「${prof.name}」恢复自动提交模式 (Auto-Commit)`,
    );
  } catch (err) {
    setStatus(`切换事务模式失败: ${String(err)}`, true);
  } finally {
    busy.value = false;
  }
}

async function handleCommitTransaction() {
  const prof = activeProfile.value;
  if (!prof || isRedisProfile(prof)) return;
  busy.value = true;
  try {
    const st = await commitTransaction(prof.id);
    txStatusByProfile[prof.id] = st;
    setStatus(`事务已提交 (COMMIT) · ${prof.name}`);
    pushProjectLog({
      projectId: prof.projectId || activeProjectId.value,
      profileId: prof.id,
      profileName: prof.name,
      level: 'success',
      summary: '手动事务已 COMMIT 提交',
    });
  } catch (err) {
    setStatus(`COMMIT 失败: ${String(err)}`, true);
  } finally {
    busy.value = false;
  }
}

async function handleRollbackTransaction() {
  const prof = activeProfile.value;
  if (!prof || isRedisProfile(prof)) return;
  busy.value = true;
  try {
    const st = await rollbackTransaction(prof.id);
    txStatusByProfile[prof.id] = st;
    setStatus(`事务已回滚 (ROLLBACK) · ${prof.name}`);
    pushProjectLog({
      projectId: prof.projectId || activeProjectId.value,
      profileId: prof.id,
      profileName: prof.name,
      level: 'warn',
      summary: '手动事务已 ROLLBACK 回滚',
    });
  } catch (err) {
    setStatus(`ROLLBACK 失败: ${String(err)}`, true);
  } finally {
    busy.value = false;
  }
}

function toggleSplitEditor() {
  if (splitTabId.value) {
    splitTabId.value = null;
    return;
  }
  if (!activeTab.value) {
    setStatus('请先打开至少 1 个编辑器标签页再开启分屏', true);
    return;
  }
  const other = tabs.value.find((t) => t.id !== activeTab.value?.id);
  splitTabId.value = other?.id ?? activeTab.value.id;
}

function handleOpenCommercialTool(
  kind: CommercialModalKind,
  profile?: ConnectionProfile,
  dbName?: string,
  tableName?: string,
) {
  const prof = profile ?? activeProfile.value;
  if (prof) syncActiveProjectForProfile(prof);
  commercialProfileId.value = prof?.id ?? '';
  commercialDbName.value = dbName ?? activeTab.value?.dbName ?? selectedDbName.value ?? '';
  commercialTableName.value = tableName ?? inspectedTable.value?.tableName ?? '';
  commercialModalKind.value = kind;
}

function handleSendCommercialSqlToEditor(sql: string, profileId?: string, dbName?: string) {
  commercialModalKind.value = 'none';
  handleSendToolSqlToEditor(sql, profileId, dbName);
}

async function handleRunCommercialStatements(
  statements: string[],
  label: string,
  profileId: string,
  dbName: string,
  refreshDb = false,
) {
  commercialModalKind.value = 'none';
  const prof = connections.value.find((c) => c.id === profileId);
  if (!prof) {
    setStatus('未找到目标数据源连接', true);
    return;
  }
  await runStatementsWithGuard(statements, label, prof, dbName, true);
  if (refreshDb && dbName) {
    await loadTablesForDb(prof, dbName);
  }
}

function handleInspectRoutine(profile: ConnectionProfile, dbName: string, routine: RoutineMeta) {
  const header = `-- ${routine.kind}: ${dbName}.${routine.name}${routine.returnType ? ` -> ${routine.returnType}` : ''}${routine.comment ? `\n-- ${routine.comment}` : ''}\n`;
  ddlTitle.value = `${profile.name} · ${dbName}.${routine.name} (${routine.kind})`;
  ddlText.value = routine.definition?.trim()
    ? `${header}\n${routine.definition}`
    : `${header}\n-- 当前数据库驱动未返回例程源码正文。`;
  activeServiceTab.value = 'ddl';
  bottomVisible.value = true;
}

function handlePreviewGridMutations(payload: GridMutationPreviewPayload) {
  gridSqlContext.value = payload;
  activeModal.value = 'grid-sql';
}

async function handleExecuteGridSql(payload: GridMutationPreviewPayload, editedSql: string) {
  const profile = connections.value.find((c) => c.id === payload.profileId);
  if (!profile) {
    setStatus('未找到对应的目标数据源', true);
    return;
  }
  activeModal.value = 'none';
  busy.value = true;
  const targetProjectId = profile.projectId || activeProjectId.value;
  let totalAffected = 0;
  let failedErr: string | null = null;

  try {
    const analysis = await analyzeSql(editedSql, profile.dbType);
    const stmts =
      analysis.statements.length > 0
        ? analysis.statements.map((s) => s.sql.trim()).filter(Boolean)
        : editedSql
            .split(';')
            .map((s) => s.trim())
            .filter(Boolean);

    for (const stmt of stmts) {
      pushProjectLog({
        projectId: targetProjectId,
        profileId: profile.id,
        profileName: profile.name,
        dbName: payload.dbName || undefined,
        level: 'sql',
        summary: `结果集修改提交 (${payload.tableName})`,
        detail: stmt,
      });

      const summary = await executeSql(
        profile.id,
        payload.dbName || null,
        stmt,
        null,
        () => {},
      );
      if (summary.success) {
        totalAffected += summary.affectedRows ?? 0;
        pushProjectLog({
          projectId: targetProjectId,
          profileId: profile.id,
          profileName: profile.name,
          dbName: payload.dbName || undefined,
          level: 'success',
          durationMs: summary.durationMs,
          summary: `修改成功，影响 ${summary.affectedRows ?? 0} 行 (${summary.durationMs} ms)`,
        });
      } else {
        failedErr = summary.errorMessage ?? '修改提交失败';
        pushProjectLog({
          projectId: targetProjectId,
          profileId: profile.id,
          profileName: profile.name,
          dbName: payload.dbName || undefined,
          level: 'error',
          summary: '结果集修改提交失败',
          detail: failedErr,
        });
        break;
      }
    }

    if (failedErr) {
      activeServiceTab.value = 'output';
      setStatus(`数据修改失败: ${failedErr}`, true);
    } else {
      resultGridRef.value?.clearDraft(payload.resultTabId);
      setStatus(`数据修改成功 · 共执行 ${stmts.length} 条 SQL，影响 ${totalAffected} 行`);

      // Automatically refresh the originating SELECT result tab in-place
      const targetTab = resultTabs.value.find((r) => r.id === payload.resultTabId);
      if (targetTab) {
        const refreshedCols: string[] = [];
        const refreshedRows: Record<string, unknown>[] = [];
        const summary = await executeSql(
          profile.id,
          targetTab.dbName || null,
          targetTab.sql,
          maxRowsLimit.value,
          (chunk) => {
            if (refreshedCols.length === 0 && chunk.columns.length > 0) {
              refreshedCols.push(...chunk.columns);
            }
            if (chunk.rows.length > 0) {
              refreshedRows.push(...chunk.rows);
            }
          },
        );
        targetTab.columns = refreshedCols.length > 0 ? refreshedCols : targetTab.columns;
        targetTab.rows = refreshedRows;
        targetTab.summary = summary;
      }
      await refreshHistory();
    }
  } catch (err) {
    const msg = String(err);
    pushProjectLog({
      projectId: targetProjectId,
      profileId: profile.id,
      profileName: profile.name,
      dbName: payload.dbName || undefined,
      level: 'error',
      summary: '结果集修改提交异常',
      detail: msg,
    });
    activeServiceTab.value = 'output';
    setStatus(`数据修改异常: ${msg}`, true);
  } finally {
    busy.value = false;
  }
}

async function handleRerunResultTab(tab: ResultTabItem) {
  const prof = connections.value.find((c) => c.id === tab.profileId);
  if (!prof) {
    setStatus('该结果集关联的数据源已不存在', true);
    return;
  }
  await executeStatementsBatch(
    [tab.sql],
    tab.title,
    prof,
    tab.dbName,
    tab.sourceTable ?? undefined,
  );
}

async function handleRunConsoleQuickSql(sql: string) {
  const prof = activeProfile.value;
  if (!prof) {
    setStatus('请先在顶部工具栏选择一个目标数据源', true);
    return;
  }
  await runStatementsWithGuard(
    [sql],
    '控制台快捷执行',
    prof,
    activeTab.value?.dbName || selectedDbName.value,
  );
}

async function resolveActiveSqlForExplain(): Promise<string> {
  const tab = activeTab.value;
  if (!tab) return '';
  let sql = tab.cursor.selectedText.trim();
  if (!sql && tab.fileKind === 'markdown') {
    sql = findEnclosingMarkdownBlock(tab.cursor.line)?.sql ?? '';
  }
  if (!sql) {
    if (isRedisProfile(activeProfile.value)) {
      sql = extractRedisCommandAtLine(tab.content, tab.cursor.line) ?? tab.content;
    } else {
      sql = (await extractStatementAtCursor(tab.content, tab.cursor.cursorOffset)) ?? tab.content;
    }
  }
  return sql.trim();
}

async function runExplainActive() {
  const sql = await resolveActiveSqlForExplain();
  await runExplainSql(sql);
}

async function handleGenerateExplainPlan(
  mode: ExplainPlanMode = 'standard',
  target: 'new-console' | 'insert-editor' | 'run' | 'copy' = 'new-console',
) {
  openMenu.value = null;
  const rawSql = await resolveActiveSqlForExplain();
  if (!rawSql) {
    setStatus('当前编辑器没有可生成执行计划的语句', true);
    return;
  }

  const plan = buildExplainPlanSql(rawSql, activeEngineKey.value, mode);
  const profId = activeTab.value?.profileId || selectedProfileId.value || undefined;
  const dbName = activeTab.value?.dbName || selectedDbName.value || undefined;

  if (target === 'copy') {
    try {
      await navigator.clipboard.writeText(plan.fullScript);
      setStatus(`已复制 ${plan.dialectLabel} 执行计划 SQL (${plan.modeLabel})`);
    } catch (err) {
      setStatus(`复制执行计划 SQL 失败: ${String(err)}`, true);
    }
    return;
  }

  if (target === 'insert-editor') {
    const tab = activeTab.value;
    if (!tab || tab.readOnly || tab.fileKind === 'markdown') {
      createNewConsoleTab(plan.fullScript, profId, dbName);
      setStatus(
        `${tab?.fileKind === 'markdown' ? 'Markdown 文档' : '当前只读页签'}已改为在新控制台生成 ${plan.dialectLabel} 执行计划 (${plan.modeLabel})`,
      );
      return;
    }
    const base = tab.content.trimEnd();
    const nextDoc = base ? `${base}\n\n${plan.fullScript}` : plan.fullScript;
    updateTabContent(nextDoc);
    setStatus(`已在当前编辑器末尾插入 ${plan.dialectLabel} 执行计划 SQL (${plan.modeLabel})`);
    return;
  }

  if (target === 'run') {
    const prof = activeProfile.value;
    if (!prof || !plan.canRunDirectly) {
      createNewConsoleTab(plan.fullScript, profId, dbName);
      setStatus(
        `${plan.dialectLabel} 当前为语法方言生成模式（未绑定直连驱动），已在新控制台生成 ${plan.modeLabel}`,
      );
      return;
    }
    if (isRedisProfile(prof)) {
      if (plan.mode === 'verbose') {
        await executeStatementsBatch(['INFO'], 'Redis INFO', prof, dbName || 'db0');
      } else {
        await runExplainSql(rawSql, 'Redis · Key 诊断');
      }
      return;
    }
    if (plan.executesQuery && (prof.env === 'PROD' || isWriteOrDdlStatement(rawSql))) {
      createNewConsoleTab(plan.fullScript, profId, dbName);
      setStatus(
        `安全保护：${plan.modeLabel} 会实际运行语句（当前为 PROD 生产库或写操作），已在新控制台生成，请人工确认后再执行`,
      );
      return;
    }
    await runExplainSql(plan.sql, `${plan.dialectLabel} · ${plan.modeLabel}`);
    return;
  }

  createNewConsoleTab(plan.fullScript, profId, dbName);
  setStatus(
    `已按 ${plan.dialectLabel} 方言生成执行计划 (${plan.modeLabel})，按 ${keymap.getDisplay('runCurrent')} 可执行查看`,
  );
}

async function runExplainSql(sql: string, customTitle?: string) {
  const prof = activeProfile.value;
  const cleaned = sql.trim().replace(/;+\s*$/, '');
  if (!prof || !cleaned) {
    setStatus('请先选择数据源并提供有效 SQL 以查看执行计划', true);
    return;
  }
  busy.value = true;
  try {
    const dbName = activeTab.value?.dbName || selectedDbName.value || 'main';
    const res = await explainSql(prof.id, dbName, cleaned);
    const tabId = `explain-${Date.now()}`;
    const item: ResultTabItem = {
      id: tabId,
      title: `${customTitle || 'EXPLAIN'} (${res.durationMs}ms)`,
      sql: cleaned,
      profileId: prof.id,
      profileName: prof.name,
      profileEnv: prof.env,
      engineKey: engineKeyOf(prof),
      dbName,
      readOnly: true,
      sourceTable: null,
      availableTables: [],
      primaryKeys: [],
      columns: res.columns,
      rows: res.rows,
      summary: {
        success: res.success,
        database: dbName,
        totalRows: res.rows.length,
        affectedRows: null,
        isDmlOrDdl: false,
        durationMs: res.durationMs,
        maskedColumns: {},
        errorMessage: res.errorMessage ?? null,
      },
    };
    resultTabs.value = mergeResultTabsWithPins([item]);
    resultGridRef.value?.pruneTabState(resultTabs.value.map((r) => r.id));
    activeResultTabId.value = tabId;
    activeServiceTab.value = 'result';
    pushProjectLog({
      projectId: prof.projectId,
      profileId: prof.id,
      profileName: prof.name,
      dbName,
      level: res.success ? 'success' : 'error',
      durationMs: res.durationMs,
      summary: res.success
        ? `${customTitle || 'EXPLAIN'} 完成 (${res.rows.length} steps, ${res.durationMs} ms)`
        : `${customTitle || 'EXPLAIN'} 执行失败`,
      detail: res.success ? cleaned : (res.errorMessage ?? undefined),
    });
    setStatus(res.success ? `执行计划生成成功 (${res.durationMs}ms)` : `EXPLAIN 失败: ${res.errorMessage}`, !res.success);
  } catch (err) {
    const msg = String(err);
    pushProjectLog({
      projectId: prof.projectId,
      profileId: prof.id,
      profileName: prof.name,
      level: 'error',
      summary: 'EXPLAIN 执行异常',
      detail: msg,
    });
    setStatus(`EXPLAIN 出错: ${msg}`, true);
  } finally {
    busy.value = false;
  }
}

async function formatActiveSql() {
  const tab = activeTab.value;
  if (!tab || tab.readOnly) return;
  if (tab.fileKind === 'markdown') {
    setStatus('Markdown 文档暂不对全文执行 SQL 格式化；可在控制台打开单个 SQL 块后格式化');
    return;
  }
  try {
    const formatted = await formatSql(tab.content);
    updateTabContent(formatted);
    setStatus('已完成 SQL 格式化');
  } catch (err) {
    setStatus(`SQL 格式化失败: ${String(err)}`, true);
  }
}

function handleConvertSelectionToIn() {
  const tab = activeTab.value;
  if (!tab || tab.readOnly) return;
  const sel = tab.cursor.selectedText;
  if (!sel.trim()) {
    setStatus('请先选中多行/逗号分隔的列表文本（如 1001, 1002）', true);
    return;
  }
  const inClause = convertSelectionToSqlIn(sel);
  editorRef.value?.replaceSelection(inClause);
  setStatus(`已将选区转换为 SQL IN 条件: ${inClause.slice(0, 60)}${inClause.length > 60 ? '...' : ''}`);
}

function handleTransformSqlCase(mode: 'upper' | 'lower') {
  const tab = activeTab.value;
  if (!tab || tab.readOnly) return;
  const sel = tab.cursor.selectedText;
  if (sel.trim()) {
    editorRef.value?.replaceSelection(transformSqlTextCase(sel, mode));
    setStatus(`已将选中 SQL 转为${mode === 'upper' ? '大写' : '小写'}`);
  } else {
    updateTabContent(transformSqlTextCase(tab.content, mode));
    setStatus(`已将全文 SQL 转为${mode === 'upper' ? '大写' : '小写'}`);
  }
}

function handleMinifyActiveSql() {
  const tab = activeTab.value;
  if (!tab || tab.readOnly) return;
  const sel = tab.cursor.selectedText;
  if (sel.trim()) {
    editorRef.value?.replaceSelection(minifySqlText(sel));
    setStatus('已将选中 SQL 压缩为单行');
  } else {
    updateTabContent(minifySqlText(tab.content));
    setStatus('已将当前控制台 SQL 压缩为单行');
  }
}

async function handleCopySqlAsCode(style: 'java' | 'json') {
  const tab = activeTab.value;
  if (!tab) return;
  const source = tab.cursor.selectedText.trim() || tab.content.trim();
  if (!source) {
    setStatus('当前编辑器为空，无内容可复制', true);
    return;
  }
  const code = escapeSqlAsCodeString(source, style);
  await copyText(code);
  setStatus(`已复制为 ${style === 'java' ? 'Java StringConcat' : 'JSON String'} 格式`);
}

async function refreshHistory() {
  if (!isTauriRuntime()) return;
  try {
    historyEntries.value = await listQueryHistory();
  } catch {
    // non-fatal
  }
}

function mergeResultTabsWithPins(incoming: ResultTabItem[]): ResultTabItem[] {
  const pinnedExisting = resultTabs.value.filter((r) => r.pinned).slice(0, 12);
  const unpinnedExisting = resultTabs.value.filter((r) => !r.pinned);
  const maxTotal = 16;
  return [...pinnedExisting, ...incoming, ...unpinnedExisting].slice(
    0,
    Math.max(maxTotal, pinnedExisting.length + incoming.length),
  );
}

function togglePinResultTab(id: string) {
  const target = resultTabs.value.find((r) => r.id === id);
  if (target) {
    target.pinned = !target.pinned;
  }
}

function closeOtherResultTabs(keepId: string) {
  resultTabs.value = resultTabs.value.filter((r) => r.id === keepId || r.pinned);
  resultGridRef.value?.pruneTabState(resultTabs.value.map((r) => r.id));
  if (!resultTabs.value.some((r) => r.id === activeResultTabId.value)) {
    activeResultTabId.value = keepId;
  }
}

function closeResultTab(id: string) {
  resultTabs.value = resultTabs.value.filter((r) => r.id !== id);
  resultGridRef.value?.pruneTabState(resultTabs.value.map((r) => r.id));
  if (activeResultTabId.value === id) {
    activeResultTabId.value = resultTabs.value[0]?.id ?? '';
  }
}

function loadHistorySqlIntoEditor(entry: QueryHistoryEntry) {
  createNewConsoleTab(entry.sql, entry.profileId, entry.database ?? '');
  setStatus(`已从执行历史载入 SQL (${entry.profileName})`);
}

// ==================== File & Markdown Operations ====================

function createNewConsoleTab(initialSql = '', profileId?: string, dbName?: string) {
  consoleCounter += 1;
  const id = `console-${Date.now()}-${consoleCounter}`;
  const current = activeTab.value;
  const resolvedProfileId =
    profileId ??
    current?.profileId ??
    (selectedProfileId.value || projectConnections.value[0]?.id || connections.value[0]?.id || '');
  const resolvedDbName = dbName ?? current?.dbName ?? selectedDbName.value ?? '';
  selectedProfileId.value = resolvedProfileId;
  selectedDbName.value = resolvedDbName;
  const tab: EditorTab = {
    id,
    title: `console_${consoleCounter}.sql`,
    path: null,
    fileKind: 'sql',
    content: initialSql,
    savedContent: initialSql,
    encoding: 'UTF-8',
    hasBom: false,
    lineEnding: 'LF',
    byteSize: estimateUtf8Bytes(initialSql),
    readOnly: false,
    profileId: resolvedProfileId,
    dbName: resolvedDbName,
    mdViewMode: 'source',
    cursor: createEmptyCursor(initialSql),
  };
  tabs.value.push(tab);
  activeTabId.value = id;
  scheduleAstInspect();
  if (isTauriRuntime()) {
    void saveConsoleScratch(tab.title, tab.content)
      .then((p) => {
        if (p) {
          consoleScratchPaths[tab.id] = p;
          lastSavedScratchContent[tab.id] = tab.content;
        }
      })
      .catch(() => {});
  }
}

function pushClosedTabs(list: EditorTab[]) {
  if (list.length === 0) return;
  for (let i = list.length - 1; i >= 0; i--) {
    const item = list[i];
    if (item.profileId) selectedProfileId.value = item.profileId;
    if (item.dbName) selectedDbName.value = item.dbName;
    closedTabsStack.value.unshift({ ...item });
  }
  if (closedTabsStack.value.length > 30) {
    closedTabsStack.value.length = 30;
  }
}

function closeEditorTab(id: string, force = false) {
  const idx = tabs.value.findIndex((t) => t.id === id);
  if (idx < 0) return;
  const closing = tabs.value[idx];
  if (!force && closing.path && isTabDirty(closing)) {
    pendingCloseTabId.value = id;
    unsavedContext.value = { title: closing.title, path: closing.path };
    activeModal.value = 'unsaved';
    return;
  }
  pushClosedTabs([closing]);
  tabs.value = tabs.value.filter((t) => t.id !== id);
  if (splitTabId.value === id) {
    splitTabId.value = null;
  }
  if (tabs.value.length === 0) {
    activeTabId.value = '';
    astAnalysis.value = null;
    return;
  }
  if (activeTabId.value === id) {
    const next = tabs.value[Math.max(0, idx - 1)] ?? tabs.value[0];
    if (next) activeTabId.value = next.id;
  }
}

async function confirmUnsavedSaveClose() {
  const targetId = pendingCloseTabId.value;
  activeModal.value = 'none';
  pendingCloseTabId.value = null;
  unsavedContext.value = null;
  if (!targetId) return;
  const prevActive = activeTabId.value;
  activeTabId.value = targetId;
  await saveActiveTab(false);
  const stillDirty = tabs.value.find((t) => t.id === targetId && isTabDirty(t));
  if (!stillDirty) {
    closeEditorTab(targetId, true);
  } else if (prevActive && tabs.value.some((t) => t.id === prevActive)) {
    activeTabId.value = prevActive;
  }
}

function confirmUnsavedDiscardClose() {
  const targetId = pendingCloseTabId.value;
  activeModal.value = 'none';
  pendingCloseTabId.value = null;
  unsavedContext.value = null;
  if (targetId) {
    closeEditorTab(targetId, true);
  }
}

function closeOtherTabs(keepId: string) {
  const target = tabs.value.find((t) => t.id === keepId);
  if (!target) return;
  const toClose = tabs.value.filter((t) => t.id !== keepId);
  pushClosedTabs(toClose);
  tabs.value = [target];
  activeTabId.value = target.id;
  if (splitTabId.value && splitTabId.value !== keepId) splitTabId.value = null;
  scheduleAstInspect();
}

function closeAllTabs() {
  if (tabs.value.length === 0) return;
  pushClosedTabs(tabs.value);
  tabs.value = [];
  activeTabId.value = '';
  splitTabId.value = null;
  astAnalysis.value = null;
}

function closeTabsToLeft(targetId: string) {
  const targetIdx = tabs.value.findIndex((t) => t.id === targetId);
  if (targetIdx <= 0) return;
  const toClose = tabs.value.slice(0, targetIdx);
  pushClosedTabs(toClose);
  tabs.value = tabs.value.slice(targetIdx);
  if (!tabs.value.some((t) => t.id === activeTabId.value)) {
    activeTabId.value = targetId;
  }
  if (splitTabId.value && !tabs.value.some((t) => t.id === splitTabId.value)) {
    splitTabId.value = null;
  }
}

function closeTabsToRight(targetId: string) {
  const targetIdx = tabs.value.findIndex((t) => t.id === targetId);
  if (targetIdx < 0 || targetIdx >= tabs.value.length - 1) return;
  const toClose = tabs.value.slice(targetIdx + 1);
  pushClosedTabs(toClose);
  tabs.value = tabs.value.slice(0, targetIdx + 1);
  if (!tabs.value.some((t) => t.id === activeTabId.value)) {
    activeTabId.value = targetId;
  }
  if (splitTabId.value && !tabs.value.some((t) => t.id === splitTabId.value)) {
    splitTabId.value = null;
  }
}

function closeUnmodifiedTabs() {
  const toClose = tabs.value.filter((t) => !isTabDirty(t));
  if (toClose.length === 0) return;
  pushClosedTabs(toClose);
  tabs.value = tabs.value.filter((t) => isTabDirty(t));
  if (tabs.value.length === 0) {
    activeTabId.value = '';
    splitTabId.value = null;
    astAnalysis.value = null;
  } else {
    if (!tabs.value.some((t) => t.id === activeTabId.value)) {
      activeTabId.value = tabs.value[0].id;
    }
    if (splitTabId.value && !tabs.value.some((t) => t.id === splitTabId.value)) {
      splitTabId.value = null;
    }
  }
}

// Editor Tab Right-Click Context Menu
const tabCtxMenu = reactive<{
  visible: boolean;
  x: number;
  y: number;
  tabId: string;
}>({
  visible: false,
  x: 0,
  y: 0,
  tabId: '',
});

const tabCtxTab = computed<EditorTab | null>(
  () => tabs.value.find((t) => t.id === tabCtxMenu.tabId) ?? null,
);

const tabCtxIndex = computed<number>(
  () => tabs.value.findIndex((t) => t.id === tabCtxMenu.tabId),
);

function openTabContextMenu(ev: MouseEvent, tab: EditorTab) {
  ev.preventDefault();
  ev.stopPropagation();
  openMenu.value = null;
  const maxX = Math.max(8, window.innerWidth - 220);
  const maxY = Math.max(8, window.innerHeight - 310);
  tabCtxMenu.x = Math.min(ev.clientX, maxX);
  tabCtxMenu.y = Math.min(ev.clientY, maxY);
  tabCtxMenu.tabId = tab.id;
  tabCtxMenu.visible = true;
}

function reopenLastClosedTab() {
  const last = closedTabsStack.value.shift();
  if (!last) return;
  const revived: EditorTab = { ...last, id: `tab-${Date.now()}` };
  tabs.value.push(revived);
  activeTabId.value = revived.id;
  if (revived.profileId) selectedProfileId.value = revived.profileId;
  if (revived.dbName) selectedDbName.value = revived.dbName;
  scheduleAstInspect();
  setStatus(`已恢复标签页: ${revived.title}`);
}

function switchAdjacentTab(delta: number) {
  if (tabs.value.length <= 1) return;
  const idx = tabs.value.findIndex((t) => t.id === activeTabId.value);
  if (idx < 0) return;
  const nextIdx = (idx + delta + tabs.value.length) % tabs.value.length;
  activeTabId.value = tabs.value[nextIdx].id;
}

function recordRecentFile(path: string) {
  const norm = path.replace(/\\/g, '/');
  recentFiles.value = [norm, ...recentFiles.value.filter((p) => p !== norm)].slice(0, 12);
  localStorage.setItem('pacergrip.recentFiles', JSON.stringify(recentFiles.value));
}

function openFileDtoInTab(dto: OpenedFileDto) {
  const existing = tabs.value.find((t) => t.path && t.path === dto.path);
  const defaultMdMode = window.innerWidth >= 1120 ? 'split' : 'source';
  const savedBinding = getFileDbBinding(dto.path);
  const boundProf = savedBinding
    ? connections.value.find((c) => c.id === savedBinding.profileId)
    : undefined;

  let targetTab: EditorTab;
  if (existing) {
    existing.content = dto.content;
    existing.savedContent = dto.content;
    existing.encoding = dto.encoding;
    existing.hasBom = dto.hasBom;
    existing.lineEnding = dto.lineEnding === 'CRLF' ? 'CRLF' : 'LF';
    existing.byteSize = dto.byteSize;
    existing.fileKind = dto.fileKind;
    existing.cursor = createEmptyCursor(dto.content);
    if (boundProf && savedBinding) {
      existing.profileId = boundProf.id;
      existing.dbName = savedBinding.dbName || '';
    }
    targetTab = existing;
    activeTabId.value = existing.id;
  } else {
    const current = activeTab.value;
    const id = `file-${Date.now()}`;
    const resolvedProfileId =
      (boundProf ? boundProf.id : undefined) ??
      current?.profileId ??
      (selectedProfileId.value || projectConnections.value[0]?.id || connections.value[0]?.id || '');
    const resolvedDbName =
      (boundProf && savedBinding ? savedBinding.dbName || '' : undefined) ??
      current?.dbName ??
      selectedDbName.value ??
      '';
    targetTab = {
      id,
      title: dto.name,
      path: dto.path,
      fileKind: dto.fileKind,
      content: dto.content,
      savedContent: dto.content,
      encoding: dto.encoding,
      hasBom: dto.hasBom,
      lineEnding: dto.lineEnding === 'CRLF' ? 'CRLF' : 'LF',
      byteSize: dto.byteSize,
      readOnly: false,
      profileId: resolvedProfileId,
      dbName: resolvedDbName,
      mdViewMode: dto.fileKind === 'markdown' ? defaultMdMode : 'source',
      cursor: createEmptyCursor(dto.content),
    };
    tabs.value.push(targetTab);
    activeTabId.value = id;
  }
  void applyTabDatabaseBinding(targetTab, false);
  recordRecentFile(dto.path);
  setStatus(
    `已打开 ${dto.name} · ${dto.encoding} · ${dto.lineEnding} · 共 ${dto.lineCount} 行 · ${formatBytes(dto.byteSize)}${
      dto.sqlBlocks.length > 0 ? ` · 检测到 ${dto.sqlBlocks.length} 个可执行 SQL 块` : ''
    }${
      boundProf && savedBinding
        ? ` · 已自动选中库: ${boundProf.name}${savedBinding.dbName ? `/${savedBinding.dbName}` : ''}`
        : ''
    }`,
  );
  scheduleAstInspect();
}

async function openFileFromDialog() {
  try {
    const picked = await pickTextFileToOpen();
    if (!picked) return;
    await openLocalFileByPath(picked);
  } catch (err) {
    setStatus(`打开文件失败: ${String(err)}`, true);
  }
}

async function openLocalFileByPath(path: string, encodingOverride?: string) {
  busy.value = true;
  try {
    const dto = await openTextFile(path, encodingOverride);
    openFileDtoInTab(dto);
  } catch (err) {
    setStatus(`读取文件失败: ${String(err)}`, true);
  } finally {
    busy.value = false;
  }
}

async function openDemoMarkdownHandbook() {
  busy.value = true;
  try {
    const dto = await createDemoMarkdownFile();
    openFileDtoInTab(dto);
  } catch (err) {
    setStatus(`打开示例 Markdown 手册失败: ${String(err)}`, true);
  } finally {
    busy.value = false;
  }
}

async function saveActiveTab(forceSaveAs = false) {
  const tab = activeTab.value;
  if (!tab) return;
  try {
    let targetPath = forceSaveAs ? null : tab.path;
    if (!targetPath) {
      targetPath = await pickSaveFilePath(tab.title);
      if (!targetPath) return;
    }
    const stats = await saveTextFile(
      targetPath,
      tab.content,
      tab.encoding,
      tab.lineEnding,
      tab.hasBom,
    );
    tab.path = stats.path;
    tab.title = stats.path.split('/').pop() ?? tab.title;
    tab.savedContent = tab.content;
    tab.encoding = stats.encoding;
    tab.hasBom = stats.hasBom;
    tab.lineEnding = stats.lineEnding === 'CRLF' ? 'CRLF' : 'LF';
    tab.byteSize = stats.byteSize;
    if (tab.title.toLowerCase().endsWith('.md') || tab.title.toLowerCase().endsWith('.markdown')) {
      tab.fileKind = 'markdown';
    }
    recordRecentFile(stats.path);
    if (tab.profileId) {
      saveFileDbBinding(stats.path, tab.profileId, tab.dbName || '', activeProjectId.value);
    }
    setStatus(
      t('savedNotice', {
        name: tab.title,
        encoding: stats.encoding,
        eol: stats.lineEnding,
        lines: stats.lineCount,
      }),
    );
    if (mountedDir.value) {
      await loadDirectory(mountedDir.value);
    }
  } catch (err) {
    setStatus(`保存文件失败: ${String(err)}`, true);
  }
}

async function handleReloadEncoding(encoding: string) {
  const tab = activeTab.value;
  if (!tab?.path) return;
  await openLocalFileByPath(tab.path, encoding);
  activeModal.value = 'none';
}

function handleConvertEncoding(encoding: string, hasBom: boolean, lineEnding: 'CRLF' | 'LF') {
  const tab = activeTab.value;
  if (!tab) return;
  tab.encoding = encoding;
  tab.hasBom = hasBom;
  tab.lineEnding = lineEnding;
  // Force dirty indicator if user explicitly changed save encoding on a disk file
  if (tab.path) {
    tab.savedContent = tab.savedContent === tab.content ? `${tab.savedContent}\0__enc_dirty__` : tab.savedContent;
  }
  activeModal.value = 'none';
  setStatus(`已切换保存格式为 ${encoding} (${lineEnding})，按 ${keymap.getDisplay('saveFile')} 保存生效`);
}

function toggleLineEndingQuick() {
  const tab = activeTab.value;
  if (!tab) return;
  tab.lineEnding = tab.lineEnding === 'LF' ? 'CRLF' : 'LF';
  setStatus(`当前行尾符已切换为 ${tab.lineEnding}`);
}

async function mountFolderDialog() {
  try {
    const dir = await pickDirectoryToMount();
    if (!dir) return;
    await loadDirectory(dir);
    openFilesPanel();
  } catch (err) {
    setStatus(`挂载本地目录失败: ${String(err)}`, true);
  }
}

async function loadDirectory(dirPath: string) {
  if (!dirPath) return;
  try {
    const norm = dirPath.replace(/\\/g, '/').replace(/\/+$/, '') || dirPath;
    workspaceEntries.value = await listWorkspaceFiles(norm);
    mountedDir.value = norm;
    rootDirExpanded.value = true;
    fileTreeChildren[norm] = workspaceEntries.value;
    localStorage.setItem('pacergrip.mountedDir', mountedDir.value);
    // Refresh any currently expanded subdirectories too
    for (const expandedPath of Object.keys(expandedFileDirs)) {
      if (expandedFileDirs[expandedPath] && expandedPath.startsWith(norm)) {
        listWorkspaceFiles(expandedPath)
          .then((kids) => {
            fileTreeChildren[expandedPath] = kids;
          })
          .catch(() => {});
      }
    }
  } catch (err) {
    setStatus(`读取目录失败: ${String(err)}`, true);
  }
}

function toggleRootDirExpanded() {
  rootDirExpanded.value = !rootDirExpanded.value;
}

function closeMountedDir() {
  mountedDir.value = '';
  workspaceEntries.value = [];
  for (const k of Object.keys(expandedFileDirs)) {
    delete expandedFileDirs[k];
  }
  for (const k of Object.keys(fileTreeChildren)) {
    delete fileTreeChildren[k];
  }
  localStorage.removeItem('pacergrip.mountedDir');
  setStatus('已关闭当前打开的文件夹');
}

function toggleShowRecentFiles() {
  showRecentFiles.value = !showRecentFiles.value;
  localStorage.setItem('pacer.showRecentFiles', showRecentFiles.value ? '1' : '0');
}

function removeRecentFile(targetPath: string) {
  const norm = targetPath.replace(/\\/g, '/').toLowerCase();
  recentFiles.value = recentFiles.value.filter(
    (p) => p.replace(/\\/g, '/').toLowerCase() !== norm,
  );
  localStorage.setItem('pacergrip.recentFiles', JSON.stringify(recentFiles.value));
}

function clearRecentFiles() {
  recentFiles.value = [];
  localStorage.setItem('pacergrip.recentFiles', '[]');
}

async function toggleFileDir(dirPath: string) {
  if (expandedFileDirs[dirPath]) {
    expandedFileDirs[dirPath] = false;
    return;
  }
  try {
    fileTreeChildren[dirPath] = await listWorkspaceFiles(dirPath);
    expandedFileDirs[dirPath] = true;
  } catch (err) {
    setStatus(`展开目录失败: ${String(err)}`, true);
  }
}

function collapseAllFileDirs() {
  for (const k of Object.keys(expandedFileDirs)) {
    expandedFileDirs[k] = false;
  }
}

async function locateFileInTreeByPath(filePath: string) {
  if (!filePath) {
    setStatus('当前标签页尚未保存到本地磁盘，无法定位位置', true);
    return;
  }
  openFilesPanel();
  rootDirExpanded.value = true;
  const normFile = filePath.replace(/\\/g, '/');
  const lastSlash = normFile.lastIndexOf('/');
  const parent = lastSlash > 0 ? normFile.slice(0, lastSlash) : normFile;
  const normRoot = mountedDir.value.replace(/\\/g, '/').replace(/\/+$/, '');

  // If no mounted dir or active file lies outside mounted dir, mount its parent folder automatically
  if (!normRoot || !normFile.toLowerCase().startsWith(`${normRoot.toLowerCase()}/`)) {
    await loadDirectory(parent);
    await fileExplorerRef.value?.scrollToActiveFile(normFile);
    setStatus(`已自动挂载并定位文件: ${normFile}`);
    return;
  }

  // Walk every ancestor folder from mountedDir down to parent and expand in tree
  const rel = parent.slice(normRoot.length).replace(/^\/+/, '');
  const segments = rel ? rel.split('/').filter(Boolean) : [];
  let cursor = normRoot;
  let currentEntries = workspaceEntries.value;
  for (const seg of segments) {
    const matched = currentEntries.find(
      (entry) => entry.isDir && entry.name.toLowerCase() === seg.toLowerCase(),
    );
    cursor = matched ? matched.path : `${cursor}/${seg}`;
    try {
      const kids = await listWorkspaceFiles(cursor);
      fileTreeChildren[cursor] = kids;
      expandedFileDirs[cursor] = true;
      currentEntries = kids;
    } catch {
      break;
    }
  }
  await fileExplorerRef.value?.scrollToActiveFile(normFile);
  setStatus(`已在文件树中定位: ${normFile}`);
}

async function locateActiveFileInTree() {
  await locateFileInTreeByPath(activeTab.value?.path ?? '');
}

async function handleRevealInExplorer(targetPath: string) {
  if (!targetPath) return;
  try {
    await revealInExplorer(targetPath);
  } catch (err) {
    setStatus(`打开系统位置失败: ${String(err)}`, true);
  }
}

// ==================== Snippets ====================

function insertSnippetIntoEditor(sql: string) {
  if (!activeTab.value) {
    createNewConsoleTab(sql);
  } else {
    editorRef.value?.insertText(`\n${sql}\n`);
  }
  setStatus('已插入 SQL 代码片段到编辑器');
}

function saveCurrentAsSnippet() {
  const tab = activeTab.value;
  if (!tab) return;
  const sql = (tab.cursor.selectedText.trim() || tab.content.trim()).slice(0, 4000);
  if (!sql) return;
  const title = sql.split('\n')[0].replace(/^--\s*/, '').slice(0, 32) || '自定义 SQL 片段';
  const entry: SqlSnippet = {
    id: `custom-${Date.now()}`,
    title,
    category: '自定义',
    sql,
  };
  customSnippets.value = [entry, ...customSnippets.value].slice(0, 40);
  localStorage.setItem('pacergrip.snippets', JSON.stringify(customSnippets.value));
  setStatus(`已保存代码片段: ${title}`);
}

function removeCustomSnippet(id: string) {
  customSnippets.value = customSnippets.value.filter((s) => s.id !== id);
  localStorage.setItem('pacergrip.snippets', JSON.stringify(customSnippets.value));
}

// ==================== Search Everywhere (Ctrl+K) ====================

const searchItems = computed<SearchEverywhereItem[]>(() => {
  const items: SearchEverywhereItem[] = [
    { id: 'act-open-file', category: 'action', label: t('openFile'), detail: '打开本地 Markdown / SQL 文件并检测编码', badge: keymap.getDisplay('openFile') },
    { id: 'act-open-folder', category: 'action', label: t('openFolder'), detail: '挂载本地目录到文件资源管理器', badge: '文件' },
    { id: 'act-demo-md', category: 'action', label: t('openDemoMd'), detail: '体验 Markdown 内联 CodeLens 与选中执行 SQL', badge: '示例' },
    { id: 'act-new-project', category: 'action', label: t('newProject'), detail: '新建多数据源工作区项目（共享统一控制台日志）', badge: '项目' },
    { id: 'act-dock-files', category: 'action', label: `将本地文件面板移至${filesDockSide.value === 'right' ? '左侧' : '右侧'}`, detail: '自由配置本地文件与脚本停靠在左侧或右侧', badge: '布局' },
    { id: 'act-locate-file', category: 'action', label: '在文件树中定位当前打开的文件', detail: '自动展开目录树并滚动高亮当前文档', badge: keymap.getDisplay('locateFile') },
    { id: 'act-new-conn', category: 'action', label: t('newConnection'), detail: '配置 TiDB / MySQL / PostgreSQL / SQLite / Redis 数据源', badge: '数据库' },
    { id: 'act-open-datasources', category: 'action', label: '数据源与连接配置管理器...', detail: '按快捷键快速打开数据源列表与连接属性设置窗', badge: keymap.getDisplay('openDataSources') },
    { id: 'act-test-all-conn', category: 'action', label: '⚡ 批量测活当前项目数据源', detail: '并发探测数据源连通性并更新绿/红健康状态指示点', badge: '运维' },
    { id: 'act-create-table', category: 'action', label: '可视化新建数据表...', detail: '零手写 DDL，可视化设计字段、主键、自增与索引并实时预览生成 SQL', badge: '建模' },
    { id: 'act-object-search', category: 'action', label: '🔍 在数据库中全局查找表/字段/Key...', detail: '跨表检索表名、列名、注释与 Redis Key 模式', badge: '搜索' },
    { id: 'act-schema-diff', category: 'action', label: '⚖️ 表结构对比与同步 DDL...', detail: '对比任意两张表或跨环境差异并生成 ALTER 同步脚本', badge: '比对' },
    { id: 'act-data-diff', category: 'action', label: '🧬 表数据内容对比与同步 SQL...', detail: '按主键比对两表真实数据行差异并生成 INSERT/UPDATE/DELETE 同步脚本', badge: '同步' },
    { id: 'act-data-transfer', category: 'action', label: '🚚 跨库 / 跨引擎数据迁移向导...', detail: '支持 MySQL/PostgreSQL/SQLite 自动方言建表与分批高效迁移数据', badge: '迁移' },
    { id: 'act-data-dict', category: 'action', label: '📘 生成并导出数据库数据字典...', detail: '批量汇总字段/索引/外键/例程元数据并导出 Markdown / HTML 文档', badge: '文档' },
    { id: 'act-split-editor', category: 'action', label: '⧉ 开启 / 关闭双栏分屏编辑器', detail: '同时并排查看或对照编辑两份 SQL 脚本', badge: '分屏' },
    { id: 'act-er-diagram', category: 'action', label: '🗺️ 查看当前库 ER 关系图谱...', detail: '交互式画布展示数据表关联并支持导出 SVG', badge: 'ER图' },
    { id: 'act-server-monitor', category: 'action', label: '📊 打开实例服务器监控与会话管理...', detail: '实时监控 QPS/内存/活跃会话，支持安全终止异常查询', badge: '监控' },
    { id: 'act-import', category: 'action', label: t('importDbeaver'), detail: '扫描本机 DBeaver 或导入 data-sources.json / .dbp / .xml', badge: 'DBeaver' },
    { id: 'act-new-console', category: 'action', label: t('newConsole'), detail: '新建 SQL 查询控制台标签页', badge: keymap.getDisplay('newConsole') },
    {
      id: 'act-gen-explain',
      category: 'action',
      label: `生成当前查询执行计划 (${activeExplainMeta.value.dialectLabel})`,
      detail: '按当前数据源方言生成标准 / ANALYZE / 详细执行计划 SQL',
      badge: activeExplainMeta.value.dialectLabel,
    },
    { id: 'act-format', category: 'action', label: t('formatSql'), detail: '格式化当前控制台 SQL', badge: keymap.getDisplay('formatSql') },
    { id: 'act-minify-sql', category: 'action', label: '压缩为单行 SQL', detail: '去除换行与注释，适合日志/命令行快速复制粘贴', badge: 'SQL' },
    { id: 'act-copy-java-sql', category: 'action', label: '复制 SQL 为 Java 代码字符串', detail: '自动转义引号与换行，直接粘贴进 Java/Spring 代码', badge: '代码' },
    { id: 'act-theme-dark', category: 'action', label: `切换主题：${t('themeDark')}`, detail: '经典深色 Darcula 界面与编辑器配色', badge: '主题' },
    { id: 'act-theme-light', category: 'action', label: `切换主题：${t('themeLight')}`, detail: '明亮清爽 IntelliJ Light 界面与编辑器配色', badge: '主题' },
    { id: 'act-theme-eyecare', category: 'action', label: `切换主题：${t('themeEyecare')}`, detail: '柔和绿豆沙护眼色（#CCE8CF），适合长时间阅读', badge: '护眼' },
    { id: 'act-lang', category: 'action', label: `切换界面语言 (${locale.value === 'zh-CN' ? 'English' : '简体中文'})`, detail: '中英文界面即时切换', badge: 'i18n' },
    { id: 'act-encoding', category: 'action', label: t('encodingModalTitle'), detail: '重新解码 GBK/UTF-8 BOM 或转换保存编码', badge: '编码' },
    { id: 'act-shortcuts', category: 'action', label: '快捷键配置 (Keymap)', detail: '自定义修改所有菜单与编辑器快捷键，支持 IDEA / VS Code 预设', badge: keymap.getDisplay('openKeymap') },
    { id: 'act-live-templates', category: 'action', label: 'SQL 缩写模板配置 (sel + Tab)', detail: '自定义输入缩写按 Tab 展开 SQL，并自动弹出当前库表与字段列表', badge: 'Tab' },
  ];

  for (const prof of connections.value) {
    items.push({
      id: `conn-${prof.id}`,
      category: 'connection',
      label: prof.name,
      detail: `${prof.env} · ${prof.fingerprint.host ?? prof.fingerprint.database ?? ''}`,
      badge: '数据源',
      payload: prof,
    });
  }

  for (const [k, tables] of Object.entries(tablesByKey)) {
    const sepIdx = k.indexOf(':');
    if (sepIdx < 0) continue;
    const profileId = k.slice(0, sepIdx);
    const dbName = k.slice(sepIdx + 1);
    const prof = connections.value.find((c) => c.id === profileId);
    if (!prof) continue;
    for (const tbl of tables) {
      items.push({
        id: `tbl-${k}-${tbl.name}`,
        category: 'table',
        label: tbl.name,
        detail: `${prof.name} / ${dbName}`,
        badge: '数据表',
        payload: { profile: prof, dbName, tableName: tbl.name },
      });
    }
  }

  for (const tab of tabs.value) {
    items.push({
      id: `tab-${tab.id}`,
      category: 'tab',
      label: tab.title,
      detail: tab.path ?? '查询控制台',
      badge: tab.fileKind.toUpperCase(),
      payload: tab.id,
    });
  }

  for (const blk of activeMdBlocks.value) {
    items.push({
      id: `mdblk-${blk.index}`,
      category: 'md-block',
      label: `SQL 块 #${blk.index + 1}${blk.title ? ` · ${blk.title}` : ''}`,
      detail: `第 ${blk.startLine}-${blk.endLine} 行 · ${blk.sql.slice(0, 60)}`,
      badge: 'Markdown SQL',
      payload: blk,
    });
  }

  return items;
});

async function handleSelectSearchItem(item: SearchEverywhereItem) {
  activeModal.value = 'none';
  if (item.category === 'action') {
    if (item.id === 'act-open-file') await openFileFromDialog();
    else if (item.id === 'act-open-folder') await mountFolderDialog();
    else if (item.id === 'act-demo-md') await openDemoMarkdownHandbook();
    else if (item.id === 'act-new-project') openProjectModal(null);
    else if (item.id === 'act-dock-files') toggleFilesDockSide();
    else if (item.id === 'act-locate-file') await locateActiveFileInTree();
    else if (item.id === 'act-new-conn') {
      editingProfile.value = null;
      activeModal.value = 'connection';
    } else if (item.id === 'act-open-datasources') openDataSourcesDialog(false);
    else if (item.id === 'act-test-all-conn') await handleTestAllConnections();
    else if (item.id === 'act-create-table') openCreateTableFromToolbar();
    else if (item.id === 'act-object-search') handleOpenObjectSearch();
    else if (item.id === 'act-schema-diff') handleOpenSchemaDiff();
    else if (item.id === 'act-data-diff') handleOpenCommercialTool('data-diff');
    else if (item.id === 'act-data-transfer') handleOpenCommercialTool('data-transfer');
    else if (item.id === 'act-data-dict') handleOpenCommercialTool('data-dict');
    else if (item.id === 'act-split-editor') toggleSplitEditor();
    else if (item.id === 'act-er-diagram') await handleOpenErDiagram();
    else if (item.id === 'act-server-monitor') handleOpenServerMonitor();
    else if (item.id === 'act-import') activeModal.value = 'import';
    else if (item.id === 'act-new-console') createNewConsoleTab('');
    else if (item.id === 'act-gen-explain') await handleGenerateExplainPlan('standard', 'new-console');
    else if (item.id === 'act-format') await formatActiveSql();
    else if (item.id === 'act-minify-sql') handleMinifyActiveSql();
    else if (item.id === 'act-copy-java-sql') await handleCopySqlAsCode('java');
    else if (item.id === 'act-theme-dark') applyTheme('dark');
    else if (item.id === 'act-theme-light') applyTheme('light');
    else if (item.id === 'act-theme-eyecare') applyTheme('eyecare');
    else if (item.id === 'act-lang') toggleLocale();
    else if (item.id === 'act-encoding') activeModal.value = 'encoding';
    else if (item.id === 'act-shortcuts') openShortcutsModal('keymap');
    else if (item.id === 'act-live-templates') openShortcutsModal('templates');
  } else if (item.category === 'connection' && item.payload) {
    const prof = item.payload as ConnectionProfile;
    await handleProfileSelectChange(prof.id);
    leftTool.value = 'database';
  } else if (item.category === 'table' && item.payload) {
    const p = item.payload as { profile: ConnectionProfile; dbName: string; tableName: string };
    await selectTableNode(p.profile, p.dbName, p.tableName);
    await handleQueryTop100(p.profile, p.dbName, p.tableName);
  } else if (item.category === 'tab' && typeof item.payload === 'string') {
    activeTabId.value = item.payload;
  } else if (item.category === 'md-block' && item.payload) {
    const blk = item.payload as MarkdownSqlBlock;
    editorRef.value?.jumpToLine(blk.contentStartLine);
  }
}

// ==================== Configurable Keymap Global Dispatcher ====================
let lastShiftTapAt = 0;
let otherKeyDuringShift = false;

function onGlobalKeyup(e: KeyboardEvent) {
  if (!keymap.doubleShiftSearch.value || activeModal.value === 'shortcuts') return;
  if (e.key === 'Shift' && !otherKeyDuringShift) {
    const now = Date.now();
    if (now - lastShiftTapAt < 350) {
      lastShiftTapAt = 0;
      activeModal.value = activeModal.value === 'search' ? 'none' : 'search';
    } else {
      lastShiftTapAt = now;
    }
  }
}

function onGlobalKeydown(e: KeyboardEvent) {
  if (e.key !== 'Shift') {
    otherKeyDuringShift = true;
  } else {
    otherKeyDuringShift = false;
  }

  // While recording inside the Keymap Configurator modal, let the modal capture raw keys cleanly
  if (activeModal.value === 'shortcuts') {
    return;
  }

  const mod = e.ctrlKey || e.metaKey;
  const keyLow = e.key.toLowerCase();

  // Close current SQL / file editor tab (configurable + IDEA Ctrl+F4 fallback)
  if (matchesCombo(e, keymap.getCombo('closeTab')) || (mod && e.key === 'F4')) {
    e.preventDefault();
    e.stopPropagation();
    if (activeModal.value !== 'none') {
      activeModal.value = 'none';
    } else if (activeTab.value) {
      closeEditorTab(activeTab.value.id);
    }
    return;
  }

  if (matchesCombo(e, keymap.getCombo('reopenTab'))) {
    e.preventDefault();
    reopenLastClosedTab();
    return;
  }

  if (
    matchesCombo(e, keymap.getCombo('searchEverywhere')) ||
    (mod && e.shiftKey && keyLow === 'a')
  ) {
    e.preventDefault();
    activeModal.value = activeModal.value === 'search' ? 'none' : 'search';
    return;
  }

  if (
    matchesCombo(e, keymap.getCombo('openDataSources')) ||
    (mod && e.altKey && e.shiftKey && e.code === 'KeyS') ||
    (IS_MAC && mod && !e.altKey && !e.shiftKey && e.code === 'Semicolon')
  ) {
    e.preventDefault();
    e.stopPropagation();
    openDataSourcesDialog(true);
    return;
  }

  if (matchesCombo(e, keymap.getCombo('cancelQuery'))) {
    if (queryExecuting.value) {
      e.preventDefault();
      void handleCancelRunningQuery();
      return;
    }
  }

  if (matchesCombo(e, keymap.getCombo('openKeymap'))) {
    e.preventDefault();
    openShortcutsModal('keymap');
    return;
  }

  if (
    matchesCombo(e, keymap.getCombo('newConsole')) ||
    (mod && e.shiftKey && keyLow === 'l')
  ) {
    e.preventDefault();
    createNewConsoleTab('');
    return;
  }

  if (matchesCombo(e, keymap.getCombo('openFile'))) {
    e.preventDefault();
    void openFileFromDialog();
    return;
  }

  if (matchesCombo(e, keymap.getCombo('saveFileAs'))) {
    e.preventDefault();
    void saveActiveTab(true);
    return;
  }

  if (matchesCombo(e, keymap.getCombo('saveFile'))) {
    e.preventDefault();
    e.stopPropagation();
    void saveActiveTab(false);
    return;
  }

  if (matchesCombo(e, keymap.getCombo('locateFile'))) {
    e.preventDefault();
    void locateActiveFileInTree();
    return;
  }

  if (matchesCombo(e, keymap.getCombo('toggleDbPanel'))) {
    e.preventDefault();
    toggleLeftTool('database');
    return;
  }

  if (matchesCombo(e, keymap.getCombo('toggleFilesPanel'))) {
    e.preventDefault();
    toggleFilesPanel();
    return;
  }

  if (matchesCombo(e, keymap.getCombo('toggleBottomPanel'))) {
    e.preventDefault();
    bottomVisible.value = !bottomVisible.value;
    return;
  }

  if (matchesCombo(e, keymap.getCombo('prevTab'))) {
    e.preventDefault();
    switchAdjacentTab(-1);
    return;
  }

  if (matchesCombo(e, keymap.getCombo('nextTab'))) {
    e.preventDefault();
    switchAdjacentTab(1);
    return;
  }

  if (matchesCombo(e, keymap.getCombo('zoomIn'))) {
    e.preventDefault();
    setEditorFontSize(editorFontSize.value + 1);
    return;
  }

  if (matchesCombo(e, keymap.getCombo('zoomOut'))) {
    e.preventDefault();
    setEditorFontSize(editorFontSize.value - 1);
    return;
  }

  if (matchesCombo(e, keymap.getCombo('zoomReset'))) {
    e.preventDefault();
    setEditorFontSize(13);
    return;
  }

  const targetEl = e.target as HTMLElement | null;
  const inEditor = Boolean(targetEl?.closest?.('.cm-editor'));
  const inInput =
    targetEl instanceof HTMLInputElement ||
    targetEl instanceof HTMLTextAreaElement ||
    targetEl instanceof HTMLSelectElement;
  if (activeModal.value === 'none' && !inEditor && !inInput) {
    if (matchesCombo(e, keymap.getCombo('runCurrent'))) {
      e.preventDefault();
      void runActiveEditor(false);
      return;
    }
    if (matchesCombo(e, keymap.getCombo('runAll'))) {
      e.preventDefault();
      void runActiveEditor(true);
      return;
    }
    if (matchesCombo(e, keymap.getCombo('explainSql'))) {
      e.preventDefault();
      void runExplainActive();
      return;
    }
    if (matchesCombo(e, keymap.getCombo('formatSql'))) {
      e.preventDefault();
      void formatActiveSql();
      return;
    }
  }

  if (e.key === 'Escape') {
    openMenu.value = null;
    tabCtxMenu.visible = false;
    if (activeModal.value !== 'none') {
      activeModal.value = 'none';
    } else {
      editorRef.value?.focus();
    }
  }
}

function onGlobalClick() {
  openMenu.value = null;
  tabCtxMenu.visible = false;
}

async function copyText(text: string) {
  try {
    await navigator.clipboard.writeText(text);
    setStatus(t('copiedNotice'));
  } catch {
    setStatus('复制失败', true);
  }
}

function onMdBlockJump(e: Event) {
  const idx = Number((e.target as HTMLSelectElement).value);
  const blk = activeMdBlocks.value[idx];
  if (blk) {
    editorRef.value?.jumpToLine(blk.contentStartLine);
  }
}

function toggleLeftTool(tool: 'database' | 'files' | 'snippets') {
  leftTool.value = leftTool.value === tool ? null : tool;
}

function toggleRightTool(tool: 'files' | 'structure' | 'extensions') {
  rightTool.value = rightTool.value === tool ? null : tool;
}

function openConnectionModal(profile: ConnectionProfile | null, targetProjectId?: string) {
  if (!profile && targetProjectId) {
    activeProjectId.value = targetProjectId;
    localStorage.setItem('pacer.activeProjectId', targetProjectId);
  }
  editingProfile.value = profile;
  activeModal.value = 'connection';
}

function openDataSourcesDialog(toggleIfOpen = false) {
  if (toggleIfOpen && activeModal.value === 'connection') {
    activeModal.value = 'none';
    return;
  }
  editingProfile.value = activeProfile.value ?? connections.value[0] ?? null;
  activeModal.value = 'connection';
}

function handleDuplicateProfileInModal(profile: ConnectionProfile) {
  const cloned: ConnectionProfile = {
    ...profile,
    id: '',
    name: `${profile.name} (副本)`,
  };
  editingProfile.value = cloned;
  activeModal.value = 'connection';
}

async function handleDeleteProfileInModal(profile: ConnectionProfile) {
  await handleDeleteProfile(profile);
  if (activeModal.value === 'connection') {
    editingProfile.value = connections.value[0] ?? null;
  }
}

function openImportModalForProject(targetProjectId?: string) {
  if (targetProjectId) {
    activeProjectId.value = targetProjectId;
    localStorage.setItem('pacer.activeProjectId', targetProjectId);
  }
  activeModal.value = 'import';
}

// ==================== Workspace Tabs Session Persistence ====================
const SESSION_TABS_KEY = 'pacer.session.tabs.v1';

interface SavedSessionTab {
  title: string;
  path: string | null;
  fileKind: string;
  content?: string;
  profileId: string;
  dbName: string;
  mdViewMode: 'source' | 'split' | 'preview';
  readOnly: boolean;
}

function openWorkspaceDirInTree() {
  const dir = workspaceInfo.value?.workspaceDir;
  if (!dir) return;
  openFilesPanel();
  void loadDirectory(dir);
}

async function refreshLocalPlugins() {
  if (!isTauriRuntime()) return;
  try {
    localPlugins.value = await listLocalPlugins();
    setStatus(`已加载 ${localPlugins.value.length} 个本地工作空间插件`);
  } catch (err) {
    setStatus(`扫描本地插件失败: ${String(err)}`, true);
  }
}

async function handleCreateSamplePlugin() {
  if (!isTauriRuntime()) return;
  try {
    localPlugins.value = await createSamplePlugin();
    setStatus('已在 pacer-workspace/plugins 生成示例插件，立即热加载生效');
    if (mountedDir.value && workspaceInfo.value?.workspaceDir) {
      const mNorm = mountedDir.value.toLowerCase();
      const wNorm = workspaceInfo.value.workspaceDir.toLowerCase();
      if (wNorm.startsWith(mNorm) || mNorm.startsWith(wNorm)) {
        await loadDirectory(mountedDir.value);
      }
    }
  } catch (err) {
    setStatus(`生成示例插件失败: ${String(err)}`, true);
  }
}

let sessionSaveTimer: number | null = null;
function scheduleSaveSession() {
  if (sessionSaveTimer !== null) window.clearTimeout(sessionSaveTimer);
  sessionSaveTimer = window.setTimeout(() => {
    try {
      const payload = {
        activeIndex: tabs.value.findIndex((t) => t.id === activeTabId.value),
        selectedProfileId: selectedProfileId.value,
        selectedDbName: selectedDbName.value,
        tabs: tabs.value.slice(0, 20).map(
          (t): SavedSessionTab => ({
            title: t.title,
            path: t.path,
            fileKind: t.fileKind,
            content: t.path ? undefined : t.content.slice(0, 50000),
            profileId: t.profileId,
            dbName: t.dbName,
            mdViewMode: t.mdViewMode,
            readOnly: t.readOnly,
          }),
        ),
      };
      localStorage.setItem(SESSION_TABS_KEY, JSON.stringify(payload));
    } catch {
      // non-fatal
    }
    if (isTauriRuntime()) {
      for (const tab of tabs.value) {
        if (
          !tab.path &&
          tab.content.length <= 500_000 &&
          lastSavedScratchContent[tab.id] !== tab.content
        ) {
          const snapshot = tab.content;
          void saveConsoleScratch(tab.title, snapshot)
            .then((savedPath) => {
              if (savedPath) {
                consoleScratchPaths[tab.id] = savedPath;
                lastSavedScratchContent[tab.id] = snapshot;
              }
            })
            .catch(() => {});
        }
      }
    }
  }, 1200);
}

async function restoreEditorSession() {
  try {
    const raw = localStorage.getItem(SESSION_TABS_KEY);
    if (!raw) return;
    const saved = JSON.parse(raw) as {
      activeIndex?: number;
      selectedProfileId?: string;
      selectedDbName?: string;
      tabs?: SavedSessionTab[];
    };
    if (saved.selectedProfileId) selectedProfileId.value = saved.selectedProfileId;
    if (saved.selectedDbName) selectedDbName.value = saved.selectedDbName;
    if (!Array.isArray(saved.tabs)) return;

    const restored: EditorTab[] = [];
    for (let i = 0; i < saved.tabs.length; i++) {
      const s = saved.tabs[i];
      if (s.path && isTauriRuntime()) {
        try {
          const dto = await openTextFile(s.path);
          const fileBinding = getFileDbBinding(dto.path);
          const fallbackProfId = s.profileId || fileBinding?.profileId || selectedProfileId.value || connections.value[0]?.id || '';
          const fallbackDb = s.dbName || fileBinding?.dbName || '';
          restored.push({
            id: `restored-${Date.now()}-${i}`,
            title: dto.name,
            path: dto.path,
            fileKind: dto.fileKind,
            content: dto.content,
            savedContent: dto.content,
            encoding: dto.encoding,
            hasBom: dto.hasBom,
            lineEnding: dto.lineEnding === 'CRLF' ? 'CRLF' : 'LF',
            byteSize: dto.byteSize,
            readOnly: Boolean(s.readOnly),
            profileId: fallbackProfId,
            dbName: fallbackDb,
            mdViewMode: s.mdViewMode || 'split',
            cursor: createEmptyCursor(dto.content),
          });
        } catch {
          // File may have been moved/deleted; skip silently
        }
      } else if (!s.path) {
        const content = s.content ?? '';
        consoleCounter += 1;
        restored.push({
          id: `restored-console-${Date.now()}-${i}`,
          title: s.title || `console_${consoleCounter}.sql`,
          path: null,
          fileKind: s.fileKind || 'sql',
          content,
          savedContent: content,
          encoding: 'UTF-8',
          hasBom: false,
          lineEnding: 'LF',
          byteSize: estimateUtf8Bytes(content),
          readOnly: Boolean(s.readOnly),
          profileId: s.profileId || selectedProfileId.value || connections.value[0]?.id || '',
          dbName: s.dbName || '',
          mdViewMode: 'source',
          cursor: createEmptyCursor(content),
        });
      }
    }
    if (saved.tabs.length === 0) {
      tabs.value = [];
      activeTabId.value = '';
    } else if (restored.length > 0) {
      tabs.value = restored;
      const idx = Math.max(0, Math.min(saved.activeIndex ?? 0, restored.length - 1));
      activeTabId.value = restored[idx].id;
      void applyTabDatabaseBinding(restored[idx], false);
    }
  } catch {
    // non-fatal
  }
}

watch(
  [
    () =>
      tabs.value
        .map(
          (t) =>
            `${t.id}:${t.title}:${t.path ?? ''}:${t.profileId}:${t.dbName}:${t.mdViewMode}:${t.readOnly ? 1 : 0}`,
        )
        .join('|'),
    activeTabId,
    selectedProfileId,
    selectedDbName,
  ],
  () => {
    scheduleSaveSession();
  },
);

watch(activeTabId, () => {
  void applyTabDatabaseBinding(activeTab.value, false);
  scheduleAstInspect();
});

onMounted(async () => {
  applyTheme(theme.value);
  window.addEventListener('keydown', onGlobalKeydown, { capture: true });
  window.addEventListener('keyup', onGlobalKeyup);
  window.addEventListener('click', onGlobalClick);

  await refreshProjectsAndConnections();
  await restoreEditorSession();
  await refreshHistory();
  if (isTauriRuntime()) {
    workspaceInfo.value = await getWorkspaceInfo().catch(() => null);
    extensions.value = await listExtensions().catch(() => []);
    localPlugins.value = await listLocalPlugins().catch(() => []);
    for (const tab of tabs.value) {
      if (!tab.path) {
        void saveConsoleScratch(tab.title, tab.content)
          .then((savedPath) => {
            if (savedPath) {
              consoleScratchPaths[tab.id] = savedPath;
              lastSavedScratchContent[tab.id] = tab.content;
            }
          })
          .catch(() => {});
      }
    }
    const dirToMount = mountedDir.value || workspaceInfo.value?.workspaceDir || '';
    if (dirToMount) {
      await loadDirectory(dirToMount);
    }
  }
  scheduleAstInspect();
});

onBeforeUnmount(() => {
  stopQueryTimer();
  window.removeEventListener('keydown', onGlobalKeydown, { capture: true });
  window.removeEventListener('keyup', onGlobalKeyup);
  window.removeEventListener('click', onGlobalClick);
  window.removeEventListener('mousemove', onResizeMove);
  window.removeEventListener('mouseup', stopResize);
});
</script>

<template>
  <div class="ide-shell">
    <!-- ==================== 1. Top Header (38px JetBrains New UI) ==================== -->
    <header class="ide-header">
      <div class="header-left">
        <div class="brand-mark" :title="t('appSubtitle')">
          <span class="brand-logo">P</span>
          <span>{{ t('appTitle') }}</span>
        </div>

        <nav class="menu-bar" @click.stop>
          <!-- 文件 -->
          <div class="menu-trigger">
            <button
              class="menu-btn"
              :class="{ open: openMenu === 'file' }"
              @click="openMenu = openMenu === 'file' ? null : 'file'"
            >
              {{ t('menuFile') }}
            </button>
            <div v-if="openMenu === 'file'" class="dropdown-menu">
              <button class="dropdown-item" @click="openMenu = null; openFileFromDialog()">
                <span>{{ t('openFile') }}</span>
                <span class="shortcut-hint">{{ keymap.getDisplay('openFile') }}</span>
              </button>
              <button class="dropdown-item" @click="openMenu = null; mountFolderDialog()">
                <span>{{ t('openFolder') }}</span>
              </button>
              <button class="dropdown-item" @click="openMenu = null; createNewConsoleTab('')">
                <span>{{ t('newConsole') }}</span>
                <span class="shortcut-hint">{{ keymap.getDisplay('newConsole') }}</span>
              </button>
              <div class="dropdown-sep" />
              <button class="dropdown-item" @click="openMenu = null; saveActiveTab(false)">
                <span>{{ t('saveFile') }}</span>
                <span class="shortcut-hint">{{ keymap.getDisplay('saveFile') }}</span>
              </button>
              <button class="dropdown-item" @click="openMenu = null; saveActiveTab(true)">
                <span>{{ t('saveFileAs') }}</span>
                <span class="shortcut-hint">{{ keymap.getDisplay('saveFileAs') }}</span>
              </button>
              <div class="dropdown-sep" />
              <button class="dropdown-item" @click="openMenu = null; activeModal = 'encoding'">
                <span>{{ t('encodingModalTitle') }}...</span>
              </button>
              <button class="dropdown-item" @click="openMenu = null; openDemoMarkdownHandbook()">
                <span>{{ t('openDemoMd') }}</span>
              </button>
            </div>
          </div>

          <!-- 编辑 -->
          <div class="menu-trigger">
            <button
              class="menu-btn"
              :class="{ open: openMenu === 'edit' }"
              @click="openMenu = openMenu === 'edit' ? null : 'edit'"
            >
              {{ t('menuEdit') }}
            </button>
            <div v-if="openMenu === 'edit'" class="dropdown-menu">
              <button class="dropdown-item" @click="openMenu = null; formatActiveSql()">
                <span>{{ t('formatSql') }}</span>
                <span class="shortcut-hint">{{ keymap.getDisplay('formatSql') }}</span>
              </button>
              <button class="dropdown-item" @click="openMenu = null; handleConvertSelectionToIn()">
                <span>选中文本转为 SQL IN (...)</span>
              </button>
              <button class="dropdown-item" @click="openMenu = null; handleTransformSqlCase('upper')">
                <span>SQL 语句/选区转大写</span>
              </button>
              <button class="dropdown-item" @click="openMenu = null; handleTransformSqlCase('lower')">
                <span>SQL 语句/选区转小写</span>
              </button>
              <button class="dropdown-item" @click="openMenu = null; handleMinifyActiveSql()">
                <span>压缩为单行 SQL</span>
              </button>
              <button class="dropdown-item" @click="openMenu = null; handleCopySqlAsCode('java')">
                <span>复制为代码字符串 (Java String)</span>
              </button>
              <button class="dropdown-item" @click="openMenu = null; handleCopySqlAsCode('json')">
                <span>复制为单行 JSON 字符串</span>
              </button>
              <button class="dropdown-item" @click="openMenu = null; saveCurrentAsSnippet()">
                <span>保存当前 SQL 为代码片段</span>
              </button>
              <div class="dropdown-sep" />
              <button
                class="dropdown-item"
                @click="openMenu = null; activeTab && (activeTab.readOnly = !activeTab.readOnly)"
              >
                <span>{{ activeTab?.readOnly ? '解除只读保护' : '设为只读模式' }}</span>
              </button>
            </div>
          </div>

          <!-- 数据库 -->
          <div class="menu-trigger">
            <button
              class="menu-btn"
              :class="{ open: openMenu === 'database' }"
              @click="openMenu = openMenu === 'database' ? null : 'database'"
            >
              {{ t('menuDatabase') }}
            </button>
            <div v-if="openMenu === 'database'" class="dropdown-menu">
              <button class="dropdown-item" @click="openMenu = null; openDataSourcesDialog(false)">
                <span>⚙️ 数据源与连接配置...</span>
                <span class="shortcut-hint">{{ keymap.getDisplay('openDataSources') }}</span>
              </button>
              <button class="dropdown-item" @click="openMenu = null; openConnectionModal(null)">
                <span>{{ t('newConnection') }}...</span>
              </button>
              <button class="dropdown-item" @click="openMenu = null; handleTestAllConnections()">
                <span>⚡ 批量测试当前项目连接</span>
              </button>
              <button class="dropdown-item" @click="openMenu = null; openCreateTableFromToolbar()">
                <span>✨ 可视化新建数据表...</span>
              </button>
              <button
                class="dropdown-item"
                @click="
                  openMenu = null;
                  handleOpenErDiagram();
                "
              >
                <span>🗺️ 查看数据库 ER 关系图...</span>
              </button>
              <button
                class="dropdown-item"
                @click="
                  openMenu = null;
                  (() => {
                    const tTarget = resolveActiveDbTarget();
                    if (tTarget && tTarget.dbName) handleDumpSql(tTarget.profile, tTarget.dbName, null, true);
                  })();
                "
              >
                <span>💾 转储当前库 SQL（结构 + 数据）...</span>
              </button>
              <button
                class="dropdown-item"
                @click="
                  openMenu = null;
                  (() => {
                    const tTarget = resolveActiveDbTarget();
                    if (tTarget) handleRunLocalSqlFile(tTarget.profile, tTarget.dbName);
                  })();
                "
              >
                <span>▶️ 运行本地 SQL 文件...</span>
              </button>
              <div class="dropdown-sep" />
              <button class="dropdown-item" @click="openMenu = null; activeModal = 'import'">
                <span>{{ t('importDbeaver') }}</span>
              </button>
              <button
                class="dropdown-item"
                @click="openMenu = null; activeModal = 'import'; handleExportDbeaverJson()"
              >
                <span>{{ t('exportDbeaver') }} (含账号密码 .dbp)</span>
              </button>
              <button
                class="dropdown-item"
                @click="openMenu = null; handleExportConnectionsXlsx(true)"
              >
                <span>📊 导出数据源账号为 Excel (.xlsx)...</span>
              </button>
              <div class="dropdown-sep" />
              <button class="dropdown-item" @click="openMenu = null; handleInitDemoSqlite()">
                <span>{{ t('openDemoDb') }}</span>
              </button>
            </div>
          </div>

          <!-- 工具 -->
          <div class="menu-trigger">
            <button
              class="menu-btn"
              :class="{ open: openMenu === 'tools' }"
              @click="openMenu = openMenu === 'tools' ? null : 'tools'"
            >
              {{ t('menuTools') }}
            </button>
            <div v-if="openMenu === 'tools'" class="dropdown-menu">
              <button class="dropdown-item" @click="openMenu = null; handleOpenObjectSearch()">
                <span>🔍 在数据库中全局查找表/字段/Key...</span>
              </button>
              <button class="dropdown-item" @click="openMenu = null; handleOpenSchemaDiff()">
                <span>⚖️ 表结构对比与同步 DDL...</span>
              </button>
              <button class="dropdown-item" @click="openMenu = null; handleOpenCommercialTool('data-diff')">
                <span>🧬 表数据内容对比与同步 SQL...</span>
              </button>
              <button class="dropdown-item" @click="openMenu = null; handleOpenCommercialTool('data-transfer')">
                <span>🚚 跨库 / 跨引擎数据迁移向导...</span>
              </button>
              <button class="dropdown-item" @click="openMenu = null; handleOpenCommercialTool('data-dict')">
                <span>📘 生成并导出数据库数据字典 (.md / .html)...</span>
              </button>
              <button class="dropdown-item" @click="openMenu = null; toggleSplitEditor()">
                <span>⧉ {{ splitTab ? '关闭分屏对比编辑器' : '开启分屏对比编辑器' }}</span>
              </button>
              <button class="dropdown-item" @click="openMenu = null; handleOpenServerMonitor()">
                <span>📊 服务器监控与活跃会话管理...</span>
              </button>
              <button class="dropdown-item" @click="openMenu = null; handleGenerateExplainPlan('standard', 'new-console')">
                <span style="display: inline-flex; align-items: center; gap: 6px">
                  <IdeIcon name="explain" :size="13" />
                  <span>生成当前查询执行计划 ({{ activeExplainMeta.dialectLabel }})</span>
                </span>
              </button>
              <button class="dropdown-item" @click="openMenu = null; activeModal = 'search'">
                <span>随处搜索 / 查找操作</span>
                <span class="shortcut-hint">{{ keymap.getDisplay('searchEverywhere') }}</span>
              </button>
              <button class="dropdown-item" @click="openMenu = null; openShortcutsModal('keymap')">
                <span style="display: inline-flex; align-items: center; gap: 6px">
                  <IdeIcon name="keyboard" :size="13" />
                  <span>快捷键配置 (Keymap)...</span>
                </span>
                <span class="shortcut-hint">{{ keymap.getDisplay('openKeymap') }}</span>
              </button>
              <button class="dropdown-item" @click="openMenu = null; openShortcutsModal('templates')">
                <span style="display: inline-flex; align-items: center; gap: 6px">
                  <IdeIcon name="snippets" :size="13" />
                  <span>SQL 缩写模板 (sel + Tab)...</span>
                </span>
                <span class="shortcut-hint">Tab</span>
              </button>
              <button class="dropdown-item" @click="openMenu = null; toggleFilesDockSide()">
                <span>文件与脚本面板停靠：移至{{ filesDockSide === 'right' ? '左侧' : '右侧' }}</span>
              </button>
              <button
                v-if="workspaceInfo"
                class="dropdown-item"
                @click="openMenu = null; openWorkspaceDirInTree()"
              >
                <span>在文件树中打开工作空间 (pacer-workspace)</span>
              </button>
              <button
                v-if="workspaceInfo"
                class="dropdown-item"
                @click="openMenu = null; handleRevealInExplorer(workspaceInfo.workspaceDir)"
              >
                <span>在{{ FILE_MANAGER_LABEL }}中打开工作空间目录</span>
              </button>
              <div class="dropdown-sep" />
              <button class="dropdown-item" @click="openMenu = null; applyTheme('dark')">
                <span style="display: inline-flex; align-items: center; gap: 6px">
                  <span class="theme-swatch" style="background: #1e1f22" />
                  <span>{{ t('themeDark') }}</span>
                </span>
                <IdeIcon v-if="theme === 'dark'" name="check" :size="12" />
              </button>
              <button class="dropdown-item" @click="openMenu = null; applyTheme('light')">
                <span style="display: inline-flex; align-items: center; gap: 6px">
                  <span class="theme-swatch" style="background: #ffffff" />
                  <span>{{ t('themeLight') }}</span>
                </span>
                <IdeIcon v-if="theme === 'light'" name="check" :size="12" />
              </button>
              <button class="dropdown-item" @click="openMenu = null; applyTheme('eyecare')">
                <span style="display: inline-flex; align-items: center; gap: 6px">
                  <span class="theme-swatch" style="background: #cce8cf" />
                  <span>{{ t('themeEyecare') }}</span>
                </span>
                <IdeIcon v-if="theme === 'eyecare'" name="check" :size="12" />
              </button>
              <button class="dropdown-item" @click="openMenu = null; toggleLocale()">
                <span>切换中英文 ({{ locale }})</span>
              </button>
              <div class="dropdown-sep" />
              <button class="dropdown-item" @click="openMenu = null; rightTool = 'extensions'">
                <span>{{ t('toolExtensions') }}</span>
              </button>
            </div>
          </div>

          <!-- 帮助 -->
          <div class="menu-trigger">
            <button
              class="menu-btn"
              :class="{ open: openMenu === 'help' }"
              @click="openMenu = openMenu === 'help' ? null : 'help'"
            >
              {{ t('menuHelp') }}
            </button>
            <div v-if="openMenu === 'help'" class="dropdown-menu">
              <button class="dropdown-item" @click="openMenu = null; openShortcutsModal('keymap')">
                <span>快捷键配置 (Keymap)...</span>
                <span class="shortcut-hint">{{ keymap.getDisplay('openKeymap') }}</span>
              </button>
              <button class="dropdown-item" @click="openMenu = null; openShortcutsModal('templates')">
                <span>SQL 缩写模板 (sel + Tab)...</span>
                <span class="shortcut-hint">Tab</span>
              </button>
              <button class="dropdown-item" @click="openMenu = null; openDemoMarkdownHandbook()">
                <span>{{ t('openDemoMd') }}</span>
              </button>
              <button class="dropdown-item" @click="openMenu = null; setShowDemoManual(!showDemoManual)">
                <span>{{ showDemoManual ? t('hideDemoManual') : t('showDemoManual') }}</span>
              </button>
            </div>
          </div>
        </nav>
      </div>

      <!-- Centered Pacer Run Widget -->
      <div class="header-center">
        <div class="run-widget">
          <!-- Workspace Project Switcher docked immediately left of Data Source -->
          <div class="menu-trigger" @click.stop>
            <button
              class="project-switcher in-run-widget"
              :title="`工作区项目：${activeProject?.name || '默认项目'}${activeProject?.description ? ` · ${activeProject.description}` : ''}`"
              @click="openMenu = openMenu === 'project' ? null : 'project'"
            >
              <span
                class="project-dot"
                :style="{ background: activeProject?.color || 'var(--accent)' }"
              />
              <span style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap">
                {{ activeProject?.name || '默认项目' }}
              </span>
              <span style="opacity: 0.65; font-size: 10px">▾</span>
            </button>

            <div v-if="openMenu === 'project'" class="dropdown-menu" style="min-width: 245px">
              <div style="padding: 4px 10px; font-size: 10.5px; color: var(--text-secondary)">
                切换工作区项目
              </div>
              <button
                v-for="proj in projects"
                :key="proj.id"
                class="dropdown-item"
                @click="selectProject(proj.id)"
              >
                <span style="display: inline-flex; align-items: center; gap: 7px; min-width: 0">
                  <span class="project-dot" :style="{ background: proj.color || 'var(--accent)' }" />
                  <span style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap">
                    {{ proj.name }}
                  </span>
                </span>
                <span class="shortcut-hint">
                  {{ connectionsInProjectById(proj.id).length }} 库
                  {{ proj.id === activeProjectId ? '✓' : '' }}
                </span>
              </button>
              <div class="dropdown-sep" />
              <button class="dropdown-item" @click="openMenu = null; openProjectModal(null)">
                <span style="display: inline-flex; align-items: center; gap: 6px">
                  <IdeIcon name="project-add" :size="14" />
                  {{ t('newProject') }}
                </span>
              </button>
              <button
                v-if="activeProject"
                class="dropdown-item"
                @click="openMenu = null; openProjectModal(activeProject)"
              >
                <span style="display: inline-flex; align-items: center; gap: 6px">
                  <IdeIcon name="edit" :size="13" />
                  {{ t('manageProject') }}
                </span>
              </button>
            </div>
          </div>

          <span class="run-widget-sep" />

          <span
            v-if="activeProfile"
            class="conn-status-dot"
            :class="connHealthByProfile[activeProfile.id]?.status ?? 'unknown'"
            style="margin-left: 4px"
            :title="connHealthTooltip(activeProfile)"
          />
          <select
            :value="activeTab?.profileId || selectedProfileId"
            title="执行数据源（按左侧项目树分组展示全部数据源）"
            @change="handleProfileSelectChange(($event.target as HTMLSelectElement).value)"
          >
            <option value="">{{ t('noConnectionSelected') }}</option>
            <template v-if="projects.length > 1">
              <optgroup v-for="proj in projects" :key="proj.id" :label="proj.name">
                <option
                  v-for="prof in connectionsInProjectById(proj.id)"
                  :key="prof.id"
                  :value="prof.id"
                >
                  {{ prof.name }} [{{ prof.env }}]
                </option>
              </optgroup>
            </template>
            <template v-else>
              <option v-for="prof in connections" :key="prof.id" :value="prof.id">
                {{ prof.name }} [{{ prof.env }}]
              </option>
            </template>
          </select>

          <span class="run-widget-sep" />

          <select
            :value="activeTab?.dbName || selectedDbName"
            title="当前执行数据库 / Schema（打开本地文件时自动记住并恢复）"
            @change="handleDbSelectChange(($event.target as HTMLSelectElement).value)"
          >
            <option value="">{{ t('noSchemaSelected') }}</option>
            <option
              v-if="
                (activeTab?.dbName || selectedDbName) &&
                !activeDatabases.some((d) => d.name === (activeTab?.dbName || selectedDbName))
              "
              :value="activeTab?.dbName || selectedDbName"
            >
              {{ activeTab?.dbName || selectedDbName }}
            </option>
            <option v-for="db in activeDatabases" :key="db.name" :value="db.name">
              {{ db.name }}
            </option>
          </select>

          <span class="run-widget-sep" />

          <button
            class="icon-btn run-btn"
            :disabled="busy || !activeProfile || !activeTab"
            :title="`${activeTab?.cursor.selectedText.trim() ? t('runSelected') : t('runCurrent')} (${keymap.getDisplay('runCurrent')})`"
            @click="runActiveEditor(false)"
          >
            <IdeIcon name="run" :size="13" />
            <span>
              {{
                activeTab?.cursor.selectedText.trim()
                  ? locale === 'zh-CN'
                    ? '执行选中'
                    : 'Run Sel'
                  : t('runShort')
              }}
            </span>
          </button>

          <button
            class="icon-btn"
            :disabled="busy || !activeProfile || !activeTab"
            :title="`${t('runAll')} (${keymap.getDisplay('runAll')})`"
            @click="runActiveEditor(true)"
          >
            <IdeIcon name="run-all" :size="13" color="var(--run-green-bright)" />
          </button>

          <button
            v-if="queryExecuting"
            class="icon-btn btn-stop-query"
            :title="`停止正在执行的查询 (${keymap.getDisplay('cancelQuery')})`"
            @click="handleCancelRunningQuery"
          >
            <span>■ 停止</span>
            <span style="font-family: var(--font-mono)">{{ (queryElapsedMs / 1000).toFixed(1) }}s</span>
          </button>

          <button
            class="icon-btn"
            :disabled="busy || !activeProfile || !activeTab"
            :title="`${t('explainBtn')} · ${activeExplainMeta.dialectLabel} (${keymap.getDisplay('explainSql')})`"
            @click="runExplainActive"
          >
            <IdeIcon name="explain" :size="14" />
          </button>

          <button
            class="icon-btn"
            :disabled="busy || !activeTab || isMarkdownTab"
            :title="`${t('formatSql')} (${keymap.getDisplay('formatSql')})`"
            @click="formatActiveSql"
          >
            <IdeIcon name="format" :size="14" />
          </button>

          <span class="run-widget-sep" />

          <select
            :value="maxRowsLimit === null ? '0' : String(maxRowsLimit)"
            :title="`${t('maxRows')}（默认 100；与 sel + Tab 查询条数联动）`"
            style="max-width: 82px"
            @change="setMaxRowsLimit(($event.target as HTMLSelectElement).value)"
          >
            <option value="100">100 行</option>
            <option value="500">500 行</option>
            <option value="1000">1000 行</option>
            <option value="5000">5000 行</option>
            <option value="0">{{ t('unlimited') }}</option>
          </select>
        </div>
      </div>

      <!-- Right Global Actions -->
      <div class="header-right">
        <button
          class="icon-btn"
          :title="`随处搜索 (双击 Shift / ${keymap.getDisplay('searchEverywhere')})`"
          @click="activeModal = 'search'"
        >
          <IdeIcon name="search" :size="14" />
          <span class="shortcut-hint search-hint-text">Shift×2</span>
        </button>
        <button
          class="icon-btn"
          :title="`快捷键与 SQL 缩写模板 (${keymap.getDisplay('openKeymap')})`"
          @click="openShortcutsModal('keymap')"
        >
          <IdeIcon name="keyboard" :size="14" />
        </button>
        <div class="menu-trigger" @click.stop>
          <button
            class="icon-btn"
            :title="`切换界面主题（当前：${
              theme === 'dark'
                ? t('themeDark')
                : theme === 'light'
                  ? t('themeLight')
                  : t('themeEyecare')
            }）`"
            @click="openMenu = openMenu === 'theme' ? null : 'theme'"
            @dblclick.stop="openMenu = null; cycleTheme()"
          >
            <IdeIcon
              :name="theme === 'dark' ? 'moon' : theme === 'light' ? 'sun' : 'leaf'"
              :size="14"
              :color="theme === 'eyecare' ? '#227a52' : ''"
            />
          </button>
          <div v-if="openMenu === 'theme'" class="dropdown-menu" style="left: auto; right: 0; min-width: 175px">
            <button class="dropdown-item" @click="openMenu = null; applyTheme('dark')">
              <span style="display: inline-flex; align-items: center; gap: 6px">
                <span class="theme-swatch" style="background: #1e1f22" />
                <IdeIcon name="moon" :size="13" />
                <span>{{ t('themeDark') }}</span>
              </span>
              <IdeIcon v-if="theme === 'dark'" name="check" :size="12" />
            </button>
            <button class="dropdown-item" @click="openMenu = null; applyTheme('light')">
              <span style="display: inline-flex; align-items: center; gap: 6px">
                <span class="theme-swatch" style="background: #ffffff" />
                <IdeIcon name="sun" :size="13" />
                <span>{{ t('themeLight') }}</span>
              </span>
              <IdeIcon v-if="theme === 'light'" name="check" :size="12" />
            </button>
            <button class="dropdown-item" @click="openMenu = null; applyTheme('eyecare')">
              <span style="display: inline-flex; align-items: center; gap: 6px">
                <span class="theme-swatch" style="background: #cce8cf" />
                <IdeIcon name="leaf" :size="13" color="#227a52" />
                <span>{{ t('themeEyecare') }}</span>
              </span>
              <IdeIcon v-if="theme === 'eyecare'" name="check" :size="12" />
            </button>
          </div>
        </div>
        <button class="icon-btn" title="切换中英文界面" @click="toggleLocale">
          {{ t('langSwitch') }}
        </button>
        <button class="btn btn-sm" :title="t('newProject')" @click="openProjectModal(null)">
          <IdeIcon name="project-add" :size="14" />
          <span>{{ t('newProjectShort') }}</span>
        </button>
        <button
          class="btn btn-primary btn-sm"
          :title="`打开数据源与连接配置 (${keymap.getDisplay('openDataSources')})`"
          @click="openConnectionModal(null)"
        >
          <IdeIcon name="plus" :size="13" />
          <span>{{ t('newConnection') }}</span>
        </button>
      </div>
    </header>

    <!-- ==================== 2. IDE Main Body ==================== -->
    <div class="ide-body">
      <!-- Left 38px Tool Window Stripe -->
      <aside class="tool-stripe left">
        <button
          class="stripe-btn"
          :class="{ active: leftTool === 'database' }"
          :title="`${t('toolDatabase')} (${keymap.getDisplay('toggleDbPanel')})`"
          @click="toggleLeftTool('database')"
        >
          <IdeIcon name="database" :size="16" />
          <span v-if="connections.length > 0" class="stripe-badge">{{ connections.length }}</span>
        </button>
        <button
          v-if="filesDockSide === 'left'"
          class="stripe-btn"
          :class="{ active: leftTool === 'files' }"
          :title="`${t('toolFiles')} (${keymap.getDisplay('toggleFilesPanel')})`"
          @click="toggleLeftTool('files')"
        >
          <IdeIcon name="folder-open" :size="16" />
        </button>
        <button
          class="stripe-btn"
          :class="{ active: leftTool === 'snippets' }"
          :title="t('toolSnippets')"
          @click="toggleLeftTool('snippets')"
        >
          <IdeIcon name="snippets" :size="16" />
        </button>
      </aside>

      <!-- Left Tool Window -->
      <section
        v-if="leftTool !== null"
        class="tool-window left"
        :style="{ width: `${leftWidth}px` }"
      >
        <DatabaseTree
          v-if="leftTool === 'database'"
          :projects="projects"
          :active-project-id="activeProjectId"
          :expanded-projects="expandedProjects"
          :connections="connections"
          :active-profile-id="activeTab?.profileId || selectedProfileId"
          :active-db-name="activeTab?.dbName || selectedDbName"
          :databases-by-profile="databasesByProfile"
          :tables-by-key="tablesByKey"
          :columns-by-key="columnsByKey"
          :indexes-by-key="indexesByKey"
          :foreign-keys-by-key="foreignKeysByKey"
          :routines-by-key="routinesByKey"
          :expanded-profiles="expandedProfiles"
          :expanded-databases="expandedDatabases"
          :expanded-tables="expandedTables"
          :conn-health-by-profile="connHealthByProfile"
          :favorites="favorites"
          :batch-testing="batchTesting"
          @new-project="openProjectModal(null)"
          @edit-project="openProjectModal"
          @delete-project="handleDeleteProject"
          @toggle-project="toggleProjectNode"
          @collapse-all-tree="collapseAllDatabaseTree"
          @move-profile-project="handleMoveProfileProject"
          @new-connection="openConnectionModal(null, $event)"
          @import-dbeaver="openImportModalForProject($event)"
          @export-dbeaver-profile="handleExportSingleProfileToDbeaver"
          @export-dbeaver-project="handleExportProjectToDbeaver"
          @init-demo="handleInitDemoSqlite($event)"
          @toggle-profile="toggleProfileNode"
          @toggle-database="toggleDatabaseNode"
          @toggle-table="toggleTableNode"
          @select-table="selectTableNode"
          @query-top100="handleQueryTop100"
          @count-table="handleCountTable"
          @view-ddl="handleViewDdl"
          @gen-select="handleGenSelect"
          @gen-insert="handleGenInsert"
          @gen-update="handleGenUpdate"
          @copy-table-columns="handleCopyTableColumns"
          @refresh-table="handleRefreshTable"
          @refresh-profile="handleRefreshProfile"
          @duplicate-profile="handleDuplicateProfile"
          @refresh-database="handleRefreshDatabase"
          @new-console-for-db="handleNewConsoleForDb"
          @create-table="handleOpenCreateTable"
          @export-db-ddl="handleExportDbDdl"
          @import-table-data="handleOpenImportTableData"
          @gen-entity-code="handleOpenGenEntityCode"
          @gen-mock-data="handleGenMockData"
          @rename-table="handleOpenRenameTable"
          @truncate-table="handleTruncateTable"
          @drop-table="handleDropTable"
          @test-profile="handleTestProfile"
          @test-all-connections="handleTestAllConnections"
          @open-object-search="handleOpenObjectSearch"
          @open-schema-diff="handleOpenSchemaDiff"
          @open-alter-table="handleOpenAlterTable"
          @open-er-diagram="handleOpenErDiagram"
          @open-server-monitor="handleOpenServerMonitor"
          @open-data-diff="(p, d, tName) => handleOpenCommercialTool('data-diff', p, d, tName)"
          @open-data-transfer="(p, d, tName) => handleOpenCommercialTool('data-transfer', p, d, tName)"
          @open-data-dict="(p, d, tName) => handleOpenCommercialTool('data-dict', p, d, tName)"
          @inspect-routine="handleInspectRoutine"
          @dump-sql="handleDumpSql"
          @run-sql-file="handleRunLocalSqlFile"
          @open-redis-new-key="handleOpenRedisNewKey"
          @edit-redis-key="handleOpenRedisEditKey"
          @toggle-favorite="toggleFavorite"
          @edit-profile="openConnectionModal"
          @delete-profile="handleDeleteProfile"
          @copy-text="copyText"
        />

        <FileExplorer
          v-else-if="leftTool === 'files' && filesDockSide === 'left'"
          ref="fileExplorerRef"
          :mounted-dir="mountedDir"
          :root-expanded="rootDirExpanded"
          :root-entries="workspaceEntries"
          :children-by-dir="fileTreeChildren"
          :expanded-dirs="expandedFileDirs"
          :recent-files="recentFiles"
          :show-recent-files="showRecentFiles"
          :active-file-path="activeTab?.path ?? null"
          :dock-side="filesDockSide"
          :workspace-dir="workspaceInfo?.workspaceDir ?? ''"
          :show-demo-manual="showDemoManual"
          @mount-folder="mountFolderDialog"
          @open-workspace-dir="openWorkspaceDirInTree"
          @toggle-root-dir="toggleRootDirExpanded"
          @close-mounted-dir="closeMountedDir"
          @toggle-dir="toggleFileDir"
          @collapse-all-dirs="collapseAllFileDirs"
          @open-file-dialog="openFileFromDialog"
          @open-file="openLocalFileByPath"
          @locate-active-file="locateActiveFileInTree"
          @reveal-path="handleRevealInExplorer"
          @toggle-dock-side="toggleFilesDockSide"
          @toggle-show-recent="toggleShowRecentFiles"
          @remove-recent-file="removeRecentFile"
          @clear-recent-files="clearRecentFiles"
          @open-demo-md="openDemoMarkdownHandbook"
          @refresh-dir="loadDirectory(mountedDir)"
        />

        <!-- SQL Snippets Tool Window -->
        <div v-else-if="leftTool === 'snippets'" style="display: flex; flex-direction: column; height: 100%">
          <div class="tw-header">
            <span class="tw-title">
              <IdeIcon name="snippets" :size="14" />
              <span class="tw-title-text">{{ t('toolSnippets') }}</span>
            </span>
            <div class="tw-actions" style="gap: 4px">
              <button
                class="icon-btn compact"
                title="管理与自定义 SQL 缩写模板 (输入 sel 按 Tab 自动展开)"
                @click="openShortcutsModal('templates')"
              >
                缩写模板
              </button>
              <button
                class="icon-btn compact"
                title="保存当前选中/编辑器 SQL 为片段"
                @click="saveCurrentAsSnippet"
              >
                + 保存
              </button>
            </div>
          </div>
          <div class="tw-search">
            <input v-model="snippetFilter" class="field" placeholder="筛选 SQL 模板..." />
          </div>
          <div class="tw-body">
            <div v-if="quickLiveTemplates.length > 0" class="live-tpl-strip">
              <div class="live-tpl-strip-title">
                <span>SQL 缩写快插 (编辑器输入缩写按 Tab)</span>
                <button
                  class="icon-btn compact"
                  title="自定义或新增 SQL 缩写模板"
                  @click="openShortcutsModal('templates')"
                >
                  自定义
                </button>
              </div>
              <div class="live-tpl-chips">
                <button
                  v-for="tpl in quickLiveTemplates"
                  :key="tpl.id"
                  class="live-tpl-chip"
                  :title="`${tpl.description}\n点击立即插入，或输入 ${tpl.abbrev} 后按 Tab 展开`"
                  @click="handleQuickExpandLiveTemplate(tpl)"
                >
                  <span class="live-tpl-abbr">{{ tpl.abbrev }}</span>
                  <span class="live-tpl-tab">Tab</span>
                </button>
              </div>
            </div>
            <div
              v-for="snip in allSnippets"
              :key="snip.id"
              style="
                padding: 8px;
                margin-bottom: 6px;
                border: 1px solid var(--border);
                border-radius: 6px;
                background: var(--bg-chrome);
              "
            >
              <div
                style="
                  display: flex;
                  align-items: center;
                  justify-content: space-between;
                  gap: 6px;
                  margin-bottom: 4px;
                "
              >
                <span style="font-size: 12px; font-weight: 600">{{ snip.title }}</span>
                <span class="pill pill-neutral">{{ snip.category }}</span>
              </div>
              <pre
                style="
                  margin: 0 0 6px;
                  padding: 6px;
                  background: var(--bg-editor);
                  border-radius: 4px;
                  font-family: var(--font-mono);
                  font-size: 11px;
                  max-height: 78px;
                  overflow: hidden;
                  color: var(--text-secondary);
                "
              >{{ snip.sql }}</pre>
              <div style="display: flex; justify-content: flex-end; gap: 4px">
                <button
                  v-if="snip.id.startsWith('custom-')"
                  class="btn btn-sm"
                  @click="removeCustomSnippet(snip.id)"
                >
                  {{ t('btnDelete') }}
                </button>
                <button class="btn btn-sm btn-primary" @click="insertSnippetIntoEditor(snip.sql)">
                  {{ t('insertSnippet') }}
                </button>
              </div>
            </div>
          </div>
        </div>
      </section>

      <!-- Left Vertical Resizer -->
      <div
        v-if="leftTool !== null"
        class="resizer-v"
        :class="{ dragging: draggingPane === 'left' }"
        @mousedown="startResize('left', $event)"
      />

      <!-- ==================== Center Editor + Services Split ==================== -->
      <main class="center-col">
        <!-- Multi-tab Bar -->
        <div class="editor-tabs-bar">
          <div
            v-for="tab in tabs"
            :key="tab.id"
            class="editor-tab"
            :class="[
              { active: tab.id === activeTabId },
              tab.id === activeTabId && activeProfile ? `env-${activeProfile.env}` : '',
            ]"
            :title="tab.path ?? tab.title"
            @click="activeTabId = tab.id"
            @contextmenu.prevent="openTabContextMenu($event, tab)"
            @auxclick.middle.prevent="closeEditorTab(tab.id)"
          >
            <IdeIcon :name="tab.fileKind === 'markdown' ? 'md-file' : 'sql-file'" :size="13" />
            <span>{{ tab.title }}</span>
            <span v-if="isTabDirty(tab)" class="dirty-dot" title="未保存更改" />
            <button
              class="tab-close-btn"
              :title="
                IS_MAC
                  ? `关闭标签页 (${keymap.getDisplay('closeTab')})`
                  : `关闭标签页 (${keymap.getDisplay('closeTab')} / Ctrl+F4)`
              "
              @click.stop="closeEditorTab(tab.id)"
            >
              <IdeIcon name="close" :size="11" />
            </button>
          </div>

          <button
            class="icon-btn"
            style="margin: 3px 4px"
            :title="`${t('newConsole')} (${keymap.getDisplay('newConsole')})`"
            @click="createNewConsoleTab('')"
          >
            <IdeIcon name="plus" :size="14" />
          </button>
          <button
            class="icon-btn"
            style="margin: 3px 2px"
            :title="`${t('openFile')} (${keymap.getDisplay('openFile')})`"
            @click="openFileFromDialog"
          >
            <IdeIcon name="folder-open" :size="14" />
          </button>
          <span v-if="showDemoManual" class="demo-chip">
            <button :title="t('openDemoMd')" style="display: inline-flex; align-items: center; gap: 4px" @click="openDemoMarkdownHandbook">
              <IdeIcon name="md-file" :size="12" />
              <span>示例手册</span>
            </button>
            <button
              :title="t('hideDemoManual')"
              style="opacity: 0.7; font-weight: 700"
              @click.stop="setShowDemoManual(false)"
            >
              ×
            </button>
          </span>
        </div>

        <!-- Contextual Editor Sub-Toolbar -->
        <div v-if="activeTab" class="editor-subbar">
          <div class="subbar-group subbar-left">
            <template v-if="activeTab.cursor.selectedText.trim()">
              <button
                class="btn btn-sm btn-primary"
                style="flex-shrink: 0; white-space: nowrap; font-variant-numeric: tabular-nums"
                :disabled="busy || !activeProfile"
                :title="`${t('runSelected')} (${activeTab.cursor.selectedChars} 字符 · ${keymap.getDisplay('runCurrent')})`"
                @click="runActiveEditor(false)"
              >
                <IdeIcon name="run" :size="11" />
                {{ locale === 'zh-CN' ? '执行选中' : 'Run Sel' }} ({{ activeTab.cursor.selectedChars }})
              </button>
              <button
                class="icon-btn compact"
                style="flex-shrink: 0"
                title="将选中的多行/逗号 ID 列表一键转为 SQL IN (...) 条件"
                @click="handleConvertSelectionToIn"
              >
                转 IN (...)
              </button>
            </template>

            <template v-if="isMarkdownTab">
              <div
                style="
                  display: inline-flex;
                  border: 1px solid var(--border);
                  border-radius: 5px;
                  overflow: hidden;
                  flex-shrink: 0;
                "
              >
                <button
                  class="icon-btn compact"
                  :class="{ active: activeTab.mdViewMode === 'source' }"
                  @click="activeTab.mdViewMode = 'source'"
                >
                  {{ t('mdSourceOnly') }}
                </button>
                <button
                  class="icon-btn compact"
                  :class="{ active: activeTab.mdViewMode === 'split' }"
                  @click="activeTab.mdViewMode = 'split'"
                >
                  {{ t('mdSplitPreview') }}
                </button>
                <button
                  class="icon-btn compact"
                  :class="{ active: activeTab.mdViewMode === 'preview' }"
                  @click="activeTab.mdViewMode = 'preview'"
                >
                  {{ t('mdPreviewOnly') }}
                </button>
              </div>

              <select
                v-if="activeMdBlocks.length > 0"
                class="field"
                style="width: auto; max-width: 220px; padding: 2px 6px; font-size: 11px; flex-shrink: 0"
                @change="onMdBlockJump"
              >
                <option value="-1">
                  {{ t('mdSqlBlocks') }} ({{ activeMdBlocks.length }}) — 选择跳转
                </option>
                <option v-for="blk in activeMdBlocks" :key="blk.index" :value="blk.index">
                  #{{ blk.index + 1 }} {{ blk.title || blk.language }} (L{{ blk.startLine }}-L{{ blk.endLine }})
                </option>
              </select>

              <span class="subbar-path-hint" :title="t('mdRunHint')">
                {{ t('mdRunHint') }}
              </span>
            </template>

            <template v-else-if="!activeTab.cursor.selectedText.trim()">
              <span
                class="subbar-path-hint"
                :title="
                  activeTab.path ||
                  consoleScratchPaths[activeTab.id] ||
                  `临时查询控制台（按 ${keymap.getDisplay('saveFile')} 可保存为 .sql / .md）`
                "
              >
                {{
                  activeTab.path ||
                  (consoleScratchPaths[activeTab.id]
                    ? `工作空间临时脚本: ${consoleScratchPaths[activeTab.id]}`
                    : `临时查询控制台（按 ${keymap.getDisplay('saveFile')} 可保存为 .sql / .md）`)
                }}
              </span>
            </template>
          </div>

          <div class="subbar-group subbar-right">
            <div class="explain-split-wrap" @click.stop>
              <button
                class="btn btn-sm explain-gen-btn"
                :disabled="busy || !activeTab"
                :title="`按当前数据源方言 (${activeExplainMeta.dialectLabel}) 在新控制台生成对应执行计划 SQL`"
                @click="handleGenerateExplainPlan('standard', 'new-console')"
              >
                <IdeIcon name="explain" :size="12" />
                <span>{{ t('genExplainPlanBtn') }}</span>
                <span class="explain-dialect-tag">{{ activeExplainMeta.dialectLabel }}</span>
              </button>
              <button
                class="btn btn-sm explain-gen-caret"
                :class="{ open: openMenu === 'explain-plan' }"
                :disabled="busy || !activeTab"
                title="选择执行计划模式（标准 / ANALYZE / 详细 / 插入当前编辑器 / 直接执行）"
                @click="openMenu = openMenu === 'explain-plan' ? null : 'explain-plan'"
              >
                ▾
              </button>
              <div
                v-if="openMenu === 'explain-plan'"
                class="dropdown-menu align-right explain-plan-menu"
              >
                <div class="explain-menu-head">
                  {{ activeExplainMeta.dialectLabel }} · 生成执行计划 SQL
                </div>
                <button
                  v-for="opt in activeExplainMeta.options"
                  :key="opt.mode"
                  class="dropdown-item"
                  :title="opt.description"
                  @click="handleGenerateExplainPlan(opt.mode, 'new-console')"
                >
                  <span style="display: inline-flex; align-items: center; gap: 6px">
                    <span>{{ opt.label }}</span>
                    <span v-if="opt.executesQuery" class="explain-exec-badge">实际执行</span>
                  </span>
                  <span class="shortcut-hint">{{ opt.shortSyntax }}</span>
                </button>
                <div class="dropdown-sep" />
                <button
                  class="dropdown-item"
                  :disabled="!activeTab || activeTab.readOnly || isMarkdownTab"
                  @click="handleGenerateExplainPlan('standard', 'insert-editor')"
                >
                  <span>插入到当前编辑器末尾</span>
                </button>
                <button
                  class="dropdown-item"
                  @click="handleGenerateExplainPlan('standard', 'copy')"
                >
                  <span>复制 {{ activeExplainMeta.dialectLabel }} 执行计划 SQL</span>
                </button>
                <template v-if="activeExplainMeta.canRunDirectly && activeProfile">
                  <div class="dropdown-sep" />
                  <button
                    class="dropdown-item"
                    @click="handleGenerateExplainPlan('standard', 'run')"
                  >
                    <span>直接生成并运行 ({{ activeExplainMeta.options[0].shortSyntax }})</span>
                  </button>
                  <button
                    v-if="
                      activeExplainAnalyzeOption &&
                      activeExplainAnalyzeOption.shortSyntax !== activeExplainMeta.options[0].shortSyntax
                    "
                    class="dropdown-item"
                    :title="activeExplainAnalyzeOption.description"
                    @click="handleGenerateExplainPlan('analyze', 'run')"
                  >
                    <span style="display: inline-flex; align-items: center; gap: 6px">
                      <span>直接生成并运行 ({{ activeExplainAnalyzeOption.shortSyntax }})</span>
                      <span
                        v-if="activeExplainAnalyzeOption.executesQuery"
                        class="explain-exec-badge"
                      >
                        实际执行
                      </span>
                    </span>
                  </button>
                </template>
              </div>
            </div>

            <div
              v-if="activeProfile && !isRedisProfile(activeProfile) && !isMarkdownTab"
              class="tx-bar-group"
            >
              <button
                class="btn btn-sm"
                :class="{ 'btn-primary': activeTxStatus.manualMode }"
                title="切换当前连接会话的事务模式：自动提交 (Auto) / 手动提交 (Manual)"
                @click="handleToggleTransactionMode"
              >
                Tx: {{ activeTxStatus.manualMode ? '手动' : '自动' }}
              </button>
              <template v-if="activeTxStatus.manualMode || activeTxStatus.inTransaction">
                <span v-if="activeTxStatus.inTransaction" class="pill pill-test">
                  未提交 {{ activeTxStatus.pendingStatements }}
                </span>
                <button
                  class="btn btn-sm"
                  style="color: var(--run-green-bright)"
                  :disabled="!activeTxStatus.inTransaction || busy"
                  title="提交当前手动事务 (COMMIT)"
                  @click="handleCommitTransaction"
                >
                  Commit
                </button>
                <button
                  class="btn btn-sm"
                  style="color: var(--prod)"
                  :disabled="!activeTxStatus.inTransaction || busy"
                  title="回滚当前手动事务 (ROLLBACK)"
                  @click="handleRollbackTransaction"
                >
                  Rollback
                </button>
              </template>
            </div>

            <button
              class="btn btn-sm"
              :class="{ 'btn-primary': Boolean(splitTab) }"
              title="开启 / 关闭右侧分屏对比编辑器"
              @click="toggleSplitEditor"
            >
              ⧉ 分屏
            </button>

            <span
              v-if="astAnalysis"
              class="pill"
              :class="
                astAnalysis.overallKind === 'READ_ONLY'
                  ? 'pill-dev'
                  : astAnalysis.overallKind === 'WRITE_DML'
                    ? 'pill-test'
                    : 'pill-prod'
              "
              style="cursor: pointer"
              title="点击查看 SQL 静态检查详情"
              @click="activeServiceTab = 'problems'"
            >
              AST: {{ astAnalysis.overallKind }}
              <template
                v-if="astAnalysis.statements.reduce((a, s) => a + s.warnings.length, 0) > 0"
              >
                · ⚠ {{ astAnalysis.statements.reduce((a, s) => a + s.warnings.length, 0) }}
              </template>
            </span>

            <button
              v-if="activeTab.path || consoleScratchPaths[activeTab.id]"
              class="icon-btn compact"
              title="在文件树中定位当前文件"
              @click="locateFileInTreeByPath(activeTab.path || consoleScratchPaths[activeTab.id])"
            >
              <IdeIcon name="locate" :size="13" />
            </button>
            <button
              v-if="activeTab.path || consoleScratchPaths[activeTab.id]"
              class="icon-btn compact"
              :title="`在${FILE_MANAGER_LABEL}中打开所在位置`"
              @click="handleRevealInExplorer(activeTab.path || consoleScratchPaths[activeTab.id])"
            >
              <IdeIcon name="reveal" :size="13" />
            </button>

            <button
              class="btn btn-sm"
              :disabled="!isTabDirty(activeTab) && Boolean(activeTab.path)"
              @click="saveActiveTab(false)"
            >
              <IdeIcon name="save" :size="12" />
              {{ t('btnSave') }}
            </button>
          </div>
        </div>

        <!-- PROD Environment Guard Banner -->
        <div v-if="activeTab && activeProfile?.env === 'PROD'" class="prod-env-ribbon">
          <span>⚠ 当前连接属于 <strong>PROD 生产环境</strong>（{{ activeProfile.name }}），DML/DDL 变更将强制弹出二次确认</span>
          <span style="opacity: 0.85">{{ activeTab.dbName ? `目标库: ${activeTab.dbName}` : '' }}</span>
        </div>

        <!-- Split Editor + Safe Markdown Preview -->
        <div v-if="activeTab" class="editor-split-area">
          <div
            v-show="!isMarkdownTab || activeTab.mdViewMode !== 'preview'"
            class="editor-pane"
          >
            <CodeEditor
              ref="editorRef"
              :model-value="activeTab.content"
              :file-kind="activeTab.fileKind"
              :engine-key="activeEngineKey"
              :theme="theme"
              :read-only="activeTab.readOnly"
              :font-size="editorFontSize"
              :word-wrap="editorWordWrap"
              :schema-map="activeSchemaMap"
              :max-rows-limit="maxRowsLimit"
              @update:model-value="updateTabContent"
              @cursor-change="handleCursorChange"
              @run-shortcut="runActiveEditor"
              @explain-shortcut="runExplainActive"
              @save-shortcut="saveActiveTab(false)"
              @format-shortcut="formatActiveSql"
              @run-md-block="runMarkdownBlock"
              @explain-md-block="explainMarkdownBlock"
              @open-md-block-console="openMarkdownBlockInConsole"
              @ensure-schema-loaded="handleEnsureActiveSchemaLoaded"
              @table-picked="handleEditorTablePicked"
            />
          </div>

          <!-- XSS-safe Interactive Markdown Preview Pane (zero v-html) -->
          <div
            v-if="isMarkdownTab && activeTab.mdViewMode !== 'source'"
            class="md-preview-pane"
          >
            <template v-for="(seg, idx) in activeMdSegments" :key="idx">
              <component
                :is="`h${Math.min(seg.level, 4)}`"
                v-if="seg.kind === 'heading'"
                style="cursor: pointer"
                title="点击跳转到源码行"
                @click="editorRef?.jumpToLine(seg.line)"
              >
                {{ seg.text }}
              </component>

              <hr
                v-else-if="seg.kind === 'hr'"
                style="border: none; border-top: 1px solid var(--border); margin: 12px 0"
              />

              <blockquote v-else-if="seg.kind === 'blockquote'">
                {{ seg.text }}
              </blockquote>

              <ul v-else-if="seg.kind === 'list'" style="margin: 6px 0; padding-left: 20px">
                <li v-for="(item, liIdx) in seg.items" :key="liIdx">{{ item }}</li>
              </ul>

              <p v-else-if="seg.kind === 'paragraph'" style="margin: 6px 0; white-space: pre-wrap">
                {{ seg.text }}
              </p>

              <!-- Interactive Runnable SQL Card -->
              <div v-else-if="seg.kind === 'sql-code'" class="md-sql-card">
                <div class="md-sql-card-head">
                  <span>
                    SQL 块 #{{ seg.block.index + 1 }} · {{ seg.block.language.toUpperCase() }}
                    · L{{ seg.block.startLine }}-L{{ seg.block.endLine }}
                  </span>
                  <div style="display: flex; gap: 5px">
                    <button
                      class="cm-md-sql-lens-btn"
                      :disabled="busy || !activeProfile"
                      @click="runMarkdownBlock(seg.block)"
                    >
                      <IdeIcon name="run" :size="11" /> {{ t('runBlock') }}
                    </button>
                    <button
                      class="cm-md-sql-lens-btn secondary"
                      :disabled="busy || !activeProfile"
                      @click="explainMarkdownBlock(seg.block)"
                    >
                      <IdeIcon name="explain" :size="11" /> EXPLAIN
                    </button>
                    <button
                      class="cm-md-sql-lens-btn secondary"
                      @click="openMarkdownBlockInConsole(seg.block)"
                    >
                      ⧉ 控制台打开
                    </button>
                  </div>
                </div>
                <pre>{{ seg.block.sql }}</pre>
              </div>

              <div v-else-if="seg.kind === 'code'" class="md-sql-card">
                <div class="md-sql-card-head">
                  <span>{{ seg.language }}</span>
                </div>
                <pre>{{ seg.text }}</pre>
              </div>
            </template>
          </div>

          <!-- Split Dual-Pane Secondary SQL Editor -->
          <div
            v-if="splitTab && (!isMarkdownTab || activeTab.mdViewMode === 'source')"
            class="editor-pane editor-pane-split"
          >
            <div class="split-pane-header">
              <div style="display: flex; align-items: center; gap: 6px; min-width: 0">
                <span style="font-size: 11px; color: var(--text-secondary)">分屏:</span>
                <select
                  v-model="splitTabId"
                  class="field"
                  style="width: auto; max-width: 220px; padding: 1px 6px; font-size: 11px"
                >
                  <option v-for="tb in tabs" :key="tb.id" :value="tb.id">
                    {{ tb.title }}
                  </option>
                </select>
              </div>
              <div style="display: flex; align-items: center; gap: 4px">
                <button
                  class="btn btn-sm"
                  :disabled="busy"
                  title="执行分屏编辑器当前 SQL"
                  @click="runSplitEditor(false)"
                >
                  ▶ 运行
                </button>
                <button
                  class="icon-btn compact"
                  title="关闭分屏"
                  @click="splitTabId = null"
                >
                  ×
                </button>
              </div>
            </div>
            <CodeEditor
              ref="splitEditorRef"
              :model-value="splitTab.content"
              :file-kind="splitTab.fileKind"
              :engine-key="activeEngineKey"
              :theme="theme"
              :read-only="splitTab.readOnly"
              :font-size="editorFontSize"
              :word-wrap="editorWordWrap"
              :schema-map="activeSchemaMap"
              :max-rows-limit="maxRowsLimit"
              @update:model-value="updateTabContentByTarget(splitTab, $event)"
              @cursor-change="handleSplitCursorChange"
              @run-shortcut="runSplitEditor"
            />
          </div>
        </div>

        <!-- Empty Editor State (when all console / file tabs are closed) -->
        <div v-else class="editor-empty-workspace">
          <div style="display: inline-flex; align-items: center; gap: 10px">
            <span class="brand-logo" style="width: 30px; height: 30px; font-size: 16px">P</span>
            <span class="editor-empty-title">没有打开的 SQL 控制台或文档</span>
          </div>
          <div class="editor-empty-shortcuts">
            <span>新建查询控制台</span>
            <span><kbd>{{ keymap.getDisplay('newConsole') }}</kbd></span>
            <span>打开 .md / .sql 文件</span>
            <span><kbd>{{ keymap.getDisplay('openFile') }}</kbd></span>
            <span>恢复刚关闭的标签页</span>
            <span><kbd>{{ keymap.getDisplay('reopenTab') }}</kbd></span>
            <span>随处搜索 / 快捷键配置</span>
            <span><kbd>{{ keymap.getDisplay('searchEverywhere') }}</kbd> / <kbd>{{ keymap.getDisplay('openKeymap') }}</kbd></span>
          </div>
          <div style="display: flex; gap: 8px; flex-wrap: wrap">
            <button class="btn btn-primary btn-sm" @click="createNewConsoleTab('')">
              <IdeIcon name="plus" :size="13" />
              新建查询控制台
            </button>
            <button class="btn btn-sm" @click="openFileFromDialog">
              <IdeIcon name="folder-open" :size="13" />
              打开本地文件
            </button>
            <button
              v-if="closedTabsStack.length > 0"
              class="btn btn-sm"
              @click="reopenLastClosedTab"
            >
              <IdeIcon name="refresh" :size="13" />
              恢复刚关闭的标签页
            </button>
          </div>
        </div>

        <!-- Horizontal Resizer -->
        <div
          v-if="bottomVisible"
          class="resizer-h"
          :class="{ dragging: draggingPane === 'bottom' }"
          @mousedown="startResize('bottom', $event)"
        />

        <!-- Bottom Services / Result Grid Panel -->
        <section v-if="bottomVisible" class="services-panel" :style="{ height: `${bottomHeight}px` }">
          <ResultGrid
            ref="resultGridRef"
            :projects="projects"
            :active-project-id="activeProjectId"
            :active-project-name="activeProject?.name ?? '默认项目'"
            :connections="connections"
            :result-tabs="resultTabs"
            :active-result-tab-id="activeResultTabId"
            :active-service-tab="activeServiceTab"
            :console-logs="consoleLogs"
            :ddl-text="ddlText"
            :ddl-title="ddlTitle"
            :history-entries="historyEntries"
            :ast-analysis="astAnalysis"
            :executing="queryExecuting"
            @update:active-service-tab="activeServiceTab = $event"
            @update:active-result-tab-id="activeResultTabId = $event"
            @close-result-tab="closeResultTab"
            @toggle-pin-result-tab="togglePinResultTab"
            @close-other-result-tabs="closeOtherResultTabs"
            @load-history-sql="loadHistorySqlIntoEditor"
            @preview-grid-mutations="handlePreviewGridMutations"
            @clear-console-logs="clearConsoleLogs"
            @run-console-sql="handleRunConsoleQuickSql"
            @send-sql-to-editor="createNewConsoleTab($event)"
            @rerun-result="handleRerunResultTab"
            @server-query-page="handleServerQueryPage"
            @server-count-rows="handleServerCountRows"
            @jump-foreign-key="handleJumpForeignKey"
            @open-data-diff="
              (tab) => {
                const p = connections.find((c) => c.id === tab.profileId);
                handleOpenCommercialTool('data-diff', p, tab.dbName, tab.sourceTable ?? undefined);
              }
            "
            @notify="setStatus($event)"
          />
        </section>
      </main>

      <!-- Right Vertical Resizer -->
      <div
        v-if="rightTool !== null"
        class="resizer-v"
        :class="{ dragging: draggingPane === 'right' }"
        @mousedown="startResize('right', $event)"
      />

      <!-- Right Tool Window (Files [default] / Structure & Indexes / Extension Slots) -->
      <section
        v-if="rightTool !== null"
        class="tool-window right"
        :style="{ width: `${rightWidth}px` }"
      >
        <FileExplorer
          v-if="rightTool === 'files' && filesDockSide === 'right'"
          ref="fileExplorerRef"
          :mounted-dir="mountedDir"
          :root-expanded="rootDirExpanded"
          :root-entries="workspaceEntries"
          :children-by-dir="fileTreeChildren"
          :expanded-dirs="expandedFileDirs"
          :recent-files="recentFiles"
          :show-recent-files="showRecentFiles"
          :active-file-path="activeTab?.path ?? null"
          :dock-side="filesDockSide"
          :workspace-dir="workspaceInfo?.workspaceDir ?? ''"
          :show-demo-manual="showDemoManual"
          @mount-folder="mountFolderDialog"
          @open-workspace-dir="openWorkspaceDirInTree"
          @toggle-root-dir="toggleRootDirExpanded"
          @close-mounted-dir="closeMountedDir"
          @toggle-dir="toggleFileDir"
          @collapse-all-dirs="collapseAllFileDirs"
          @open-file-dialog="openFileFromDialog"
          @open-file="openLocalFileByPath"
          @locate-active-file="locateActiveFileInTree"
          @reveal-path="handleRevealInExplorer"
          @toggle-dock-side="toggleFilesDockSide"
          @toggle-show-recent="toggleShowRecentFiles"
          @remove-recent-file="removeRecentFile"
          @clear-recent-files="clearRecentFiles"
          @open-demo-md="openDemoMarkdownHandbook"
          @refresh-dir="loadDirectory(mountedDir)"
        />

        <!-- Structure & Indexes Inspector -->
        <template v-else-if="rightTool === 'structure'">
          <div class="tw-header">
            <span>{{ t('toolStructure') }}</span>
            <div v-if="inspectedTable" class="tw-actions" style="gap: 4px">
              <template
                v-if="!isRedisProfile(connections.find((c) => c.id === inspectedTable?.profileId))"
              >
                <button
                  class="icon-btn compact"
                  title="导入 CSV / JSON 数据到当前表"
                  @click="
                    () => {
                      const p = connections.find((c) => c.id === inspectedTable?.profileId);
                      if (p && inspectedTable) {
                        handleOpenImportTableData(p, inspectedTable.dbName, inspectedTable.tableName);
                      }
                    }
                  "
                >
                  导入
                </button>
                <button
                  class="icon-btn compact"
                  title="生成实体代码 (Java/Go/TS/Python/Rust)"
                  @click="
                    () => {
                      const p = connections.find((c) => c.id === inspectedTable?.profileId);
                      if (p && inspectedTable) {
                        handleOpenGenEntityCode(p, inspectedTable.dbName, inspectedTable.tableName);
                      }
                    }
                  "
                >
                  代码
                </button>
                <button
                  class="icon-btn compact"
                  title="生成 10 行 Mock INSERT 数据"
                  @click="
                    () => {
                      const p = connections.find((c) => c.id === inspectedTable?.profileId);
                      if (p && inspectedTable) {
                        handleGenMockData(p, inspectedTable.dbName, inspectedTable.tableName);
                      }
                    }
                  "
                >
                  Mock
                </button>
              </template>
              <button
                class="icon-btn compact"
                title="查看建表 DDL / Key 操作详情"
                @click="
                  () => {
                    const p = connections.find((c) => c.id === inspectedTable?.profileId);
                    if (p && inspectedTable) {
                      handleViewDdl(p, inspectedTable.dbName, inspectedTable.tableName);
                    }
                  }
                "
              >
                {{
                  isRedisProfile(connections.find((c) => c.id === inspectedTable?.profileId))
                    ? 'INFO'
                    : 'DDL'
                }}
              </button>
            </div>
          </div>

          <div class="tw-body">
            <div v-if="!inspectedTable" class="empty-state">
              {{ t('emptyStructureHint') }}
            </div>
            <template v-else>
              <div
                style="
                  padding: 6px 8px;
                  margin-bottom: 8px;
                  background: var(--bg-chrome);
                  border: 1px solid var(--border);
                  border-radius: 6px;
                  font-size: 12px;
                "
              >
                <div style="font-weight: 700; font-family: var(--font-mono)">
                  {{ inspectedTable.dbName }}.{{ inspectedTable.tableName }}
                </div>
                <div style="font-size: 11px; color: var(--text-secondary); margin-top: 2px">
                  {{ t('columnsLabel') }}: {{ inspectedColumns.length }} ·
                  {{ t('indexesLabel') }}: {{ inspectedIndexes.length }}
                </div>
              </div>

              <div style="font-size: 11px; font-weight: 700; color: var(--text-secondary); margin: 8px 2px 4px">
                {{ t('columnsLabel') }} ({{ inspectedColumns.length }})
              </div>
              <div
                v-for="col in inspectedColumns"
                :key="col.name"
                style="
                  padding: 4px 6px;
                  border-bottom: 1px solid var(--border-subtle);
                  font-size: 11.5px;
                "
              >
                <div style="display: flex; align-items: center; justify-content: space-between; gap: 6px">
                  <div style="min-width: 0; overflow: hidden; text-overflow: ellipsis">
                    <IdeIcon v-if="col.isPrimaryKey" name="key" :size="12" style="margin-right: 4px" />
                    <span style="font-family: var(--font-mono)">{{ col.name }}</span>
                  </div>
                  <div style="display: flex; align-items: center; gap: 4px; flex-shrink: 0">
                    <span style="color: var(--text-secondary); font-family: var(--font-mono); font-size: 10.5px">
                      {{ col.dataType }}
                    </span>
                    <span
                      class="pill"
                      :class="col.nullable ? 'pill-neutral' : 'pill-test'"
                    >
                      {{ col.nullable ? t('nullableYes') : t('nullableNo') }}
                    </span>
                  </div>
                </div>
                <div
                  v-if="col.defaultValue || col.comment"
                  style="
                    font-size: 10.5px;
                    color: var(--text-muted);
                    margin-top: 2px;
                    display: flex;
                    gap: 8px;
                    flex-wrap: wrap;
                  "
                >
                  <span v-if="col.defaultValue" style="font-family: var(--font-mono)">
                    default={{ col.defaultValue }}
                  </span>
                  <span v-if="col.comment">{{ col.comment }}</span>
                </div>
              </div>

              <div style="font-size: 11px; font-weight: 700; color: var(--text-secondary); margin: 12px 2px 4px">
                {{ t('indexesLabel') }} ({{ inspectedIndexes.length }})
              </div>
              <div
                v-if="inspectedIndexes.length === 0"
                style="font-size: 11px; color: var(--text-muted); padding: 4px 6px"
              >
                无额外索引或驱动暂未返回索引信息
              </div>
              <div
                v-for="idx in inspectedIndexes"
                :key="idx.name"
                style="
                  padding: 5px 6px;
                  border-bottom: 1px solid var(--border-subtle);
                  font-size: 11.5px;
                "
              >
                <div style="display: flex; align-items: center; gap: 5px">
                  <span v-if="idx.unique" class="pill pill-dev">{{ t('uniqueLabel') }}</span>
                  <span style="font-family: var(--font-mono); font-weight: 600">{{ idx.name }}</span>
                </div>
                <div style="font-size: 10.5px; color: var(--text-secondary); font-family: var(--font-mono); margin-top: 2px">
                  ({{ idx.columns.join(', ') }})
                </div>
              </div>

              <div style="font-size: 11px; font-weight: 700; color: var(--text-secondary); margin: 12px 2px 4px">
                外键关联 ({{ inspectedForeignKeys.length }})
              </div>
              <div
                v-if="inspectedForeignKeys.length === 0"
                style="font-size: 11px; color: var(--text-muted); padding: 4px 6px"
              >
                无外键约束
              </div>
              <div
                v-for="fk in inspectedForeignKeys"
                :key="`${fk.name}-${fk.columnName}`"
                style="
                  padding: 5px 6px;
                  border-bottom: 1px solid var(--border-subtle);
                  font-size: 11.5px;
                "
              >
                <div style="font-family: var(--font-mono); font-weight: 600">
                  🔗 {{ fk.columnName }} → {{ fk.referencedTable }}.{{ fk.referencedColumn }}
                </div>
                <div style="font-size: 10.5px; color: var(--text-secondary); margin-top: 2px">
                  {{ fk.name }}
                  <template v-if="fk.onDelete"> · ON DELETE {{ fk.onDelete }}</template>
                  <template v-if="fk.onUpdate"> · ON UPDATE {{ fk.onUpdate }}</template>
                </div>
              </div>
            </template>
          </div>
        </template>

        <!-- Pacer Extension Center (Hot-Loaded Local Plugins + Native Rust Traits) -->
        <template v-else-if="rightTool === 'extensions'">
          <div class="tw-header">
            <span class="tw-title">
              <IdeIcon name="extensions" :size="14" />
              <span class="tw-title-text">{{ t('extModalTitle') }}</span>
            </span>
            <div class="tw-actions">
              <button
                class="icon-btn compact"
                title="重新扫描本地扩展插件"
                @click="refreshLocalPlugins"
              >
                <IdeIcon name="refresh" :size="13" />
              </button>
            </div>
          </div>
          <div class="tw-body" style="padding: 10px; font-size: 12px; line-height: 1.55">
            <p style="margin: 0 0 6px">{{ t('extEmptyP1') }}</p>
            <p style="margin: 0 0 10px; color: var(--text-secondary)">{{ t('extEmptyP2') }}</p>

            <!-- 1. Local Workspace JSON Plugin Pack -->
            <div
              style="
                padding: 8px 10px;
                border: 1px solid var(--border);
                border-radius: 6px;
                background: var(--bg-chrome);
                margin-bottom: 10px;
              "
            >
              <div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 5px">
                <strong>工作空间本地插件（热加载）</strong>
                <span class="pill pill-dev">{{ localPlugins.length }} 个已加载</span>
              </div>
              <div
                v-if="workspaceInfo"
                style="
                  font-family: var(--font-mono);
                  font-size: 10.5px;
                  color: var(--text-secondary);
                  word-break: break-all;
                  margin-bottom: 8px;
                "
              >
                目录: {{ workspaceInfo.pluginsDir }}
                <span class="pill pill-neutral" style="margin-left: 4px">
                  {{
                    workspaceInfo.isPortable
                      ? IS_MAC
                        ? '应用同级工作空间'
                        : 'EXE 同级工作空间'
                      : '系统数据目录'
                  }}
                </span>
              </div>
              <div style="display: flex; gap: 6px; flex-wrap: wrap; margin-bottom: 6px">
                <button class="btn btn-primary btn-sm" @click="handleCreateSamplePlugin">
                  生成示例插件包
                </button>
                <button
                  v-if="workspaceInfo"
                  class="btn btn-sm"
                  @click="handleRevealInExplorer(workspaceInfo.pluginsDir)"
                >
                  打开插件目录
                </button>
                <button class="btn btn-sm" @click="refreshLocalPlugins">
                  重新扫描
                </button>
              </div>
              <div style="font-size: 11px; color: var(--text-secondary)">
                扩展方法：在插件目录下新建 <code>&lt;插件名&gt;/pacer-plugin.json</code>，配置
                <code>snippets</code>（自动加入「SQL 代码片段」）与 <code>sqlRules</code>（自动注入执行安全拦截规则），无需重新编译即可即时生效。
              </div>

              <div v-if="localPlugins.length > 0" style="margin-top: 8px; display: flex; flex-direction: column; gap: 6px">
                <div
                  v-for="plg in localPlugins"
                  :key="plg.id"
                  style="
                    padding: 6px 8px;
                    border: 1px solid var(--border);
                    border-radius: 5px;
                    background: var(--bg-editor);
                  "
                >
                  <div style="display: flex; align-items: center; justify-content: space-between; gap: 6px">
                    <strong style="font-size: 11.5px; overflow: hidden; text-overflow: ellipsis">
                      {{ plg.name }}
                    </strong>
                    <span class="pill pill-neutral">v{{ plg.version || '1.0' }}</span>
                  </div>
                  <div style="font-size: 11px; color: var(--text-secondary); margin-top: 2px">
                    {{ plg.description }}
                  </div>
                  <div style="display: flex; align-items: center; justify-content: space-between; margin-top: 4px; font-size: 10.5px; color: var(--text-muted)">
                    <span>模板 {{ plg.snippets.length }} 条 · 审计规则 {{ plg.sqlRules.length }} 条</span>
                    <button
                      class="cell-inline-btn"
                      @click="openLocalFileByPath(plg.manifestPath)"
                    >
                      编辑 JSON
                    </button>
                  </div>
                </div>
              </div>
            </div>

            <!-- 2. Native Rust Trait Extension Hooks -->
            <div
              style="
                padding: 8px 10px;
                border: 1px solid var(--border);
                border-radius: 6px;
                background: var(--bg-chrome);
                margin-bottom: 8px;
              "
            >
              <div style="font-weight: 700; margin-bottom: 6px">Rust 内核 Trait 扩展接口</div>
              <div
                style="
                  font-family: var(--font-mono);
                  font-size: 10.8px;
                  color: var(--text-secondary);
                  word-break: break-all;
                  display: flex;
                  flex-direction: column;
                  gap: 4px;
                "
              >
                <div>1. <code>pacer_core::extension::GovernanceExtension</code>（执行前审批拦截 &amp; 审计上报）</div>
                <div>2. <code>pacer_core::extension::AiSqlExtension</code>（Text2SQL &amp; 查询优化）</div>
                <div>3. 编译开关：<code>cargo build --features pacer-extension</code></div>
              </div>
            </div>

            <div v-if="extensions.length > 0">
              <div style="font-weight: 700; margin-bottom: 6px">已启用内核插件</div>
              <div
                v-for="ext in extensions"
                :key="ext.id"
                style="
                  padding: 7px 8px;
                  border: 1px solid var(--border);
                  border-radius: 6px;
                  margin-bottom: 6px;
                "
              >
                <div style="display: flex; justify-content: space-between; align-items: center">
                  <strong>{{ ext.name }}</strong>
                  <span class="pill pill-neutral">{{ ext.kind }}</span>
                </div>
                <div style="font-size: 11px; color: var(--text-secondary); margin-top: 3px">
                  {{ ext.description }}
                </div>
              </div>
            </div>
            <div v-else style="font-size: 11px; color: var(--text-muted)">
              当前未启用编译期内核插件（纯原生独立运行模式）。
            </div>
          </div>
        </template>
      </section>

      <!-- Right 38px Tool Window Stripe -->
      <aside class="tool-stripe right">
        <button
          v-if="filesDockSide === 'right'"
          class="stripe-btn"
          :class="{ active: rightTool === 'files' }"
          :title="`${t('toolFiles')} (${keymap.getDisplay('toggleFilesPanel')})`"
          @click="toggleRightTool('files')"
        >
          <IdeIcon name="folder-open" :size="16" />
        </button>
        <button
          class="stripe-btn"
          :class="{ active: rightTool === 'structure' }"
          :title="t('toolStructure')"
          @click="toggleRightTool('structure')"
        >
          <IdeIcon name="structure" :size="16" />
        </button>
        <button
          class="stripe-btn"
          :class="{ active: rightTool === 'extensions' }"
          :title="t('toolExtensions')"
          @click="toggleRightTool('extensions')"
        >
          <IdeIcon name="extensions" :size="16" />
        </button>
      </aside>
    </div>

    <!-- ==================== 3. Bottom 24px JetBrains Status Bar ==================== -->
    <footer class="ide-statusbar">
      <div class="status-left">
        <span
          v-if="activeProfile"
          class="pill"
          :class="
            activeProfile.env === 'PROD'
              ? 'pill-prod'
              : activeProfile.env === 'TEST'
                ? 'pill-test'
                : 'pill-dev'
          "
          style="margin-right: 6px"
        >
          {{ activeProfile.env }}
        </span>
        <span class="status-msg" :class="{ error: statusIsError }" :title="statusMessage">
          {{ busy ? '⏳ 正在执行... ' : '' }}{{ statusMessage }}
        </span>
      </div>

      <div v-if="activeTab" class="status-right">
        <!-- Cursor Position & Selection -->
        <button
          class="status-widget"
          title="光标位置与选区统计"
          @click="editorRef?.focus()"
        >
          {{
            t('statusLineCol', {
              line: activeTab.cursor.line,
              col: activeTab.cursor.col,
            })
          }}
          <template v-if="activeTab.cursor.selectedChars > 0">
            ·
            {{
              t('statusSelection', {
                chars: activeTab.cursor.selectedChars,
                lines: activeTab.cursor.selectedLines,
              })
            }}
          </template>
        </button>

        <span class="status-divider" />

        <!-- Total Line Count -->
        <span class="status-widget" style="cursor: default" title="文档总行数">
          {{ t('statusTotalLines', { count: activeTab.cursor.lineCount }) }}
        </span>

        <span class="status-divider" />

        <!-- Byte Size -->
        <span class="status-widget" style="cursor: default" title="文件字节大小">
          {{ formatBytes(activeTab.byteSize) }}
        </span>

        <span class="status-divider" />

        <!-- Line Ending Toggle -->
        <button
          class="status-widget"
          title="点击切换行尾换行符 (CRLF / LF)"
          @click="toggleLineEndingQuick"
        >
          {{ activeTab.lineEnding }}
        </button>

        <span class="status-divider" />

        <!-- File Encoding (Clickable -> Encoding Modal) -->
        <button
          class="status-widget"
          title="点击切换文件解码编码或转换保存编码 (UTF-8 / UTF-8 BOM / GBK / UTF-16)"
          @click="activeModal = 'encoding'"
        >
          {{ activeTab.encoding }}
        </button>

        <span class="status-divider" />

        <!-- Indentation -->
        <span class="status-widget" style="cursor: default">
          {{ t('statusSpaces') }}
        </span>

        <span class="status-divider" />

        <!-- Word Wrap Toggle -->
        <button
          class="status-widget"
          :style="editorWordWrap ? { color: 'var(--accent)', fontWeight: 600 } : undefined"
          title="点击切换编辑器自动换行"
          @click="toggleEditorWordWrap"
        >
          <IdeIcon name="wrap" :size="12" />
          {{ editorWordWrap ? '已换行' : '不换行' }}
        </button>

        <span class="status-divider" />

        <!-- Font Size Widget -->
        <button
          class="status-widget"
          :title="`编辑器字号（${keymap.getDisplay('zoomIn')} / ${keymap.getDisplay('zoomOut')} 调整，点击重置 13px）`"
          @click="setEditorFontSize(13)"
        >
          {{ editorFontSize }}px
        </button>

        <span class="status-divider" />

        <!-- Dialect / File Kind -->
        <span class="status-widget" style="cursor: default">
          {{ activeDialectLabel }}
        </span>

        <span class="status-divider" />

        <!-- Read-Only Toggle -->
        <button
          class="status-widget"
          title="点击切换当前标签页只读/可写状态"
          @click="activeTab.readOnly = !activeTab.readOnly"
        >
          <IdeIcon :name="activeTab.readOnly ? 'lock' : 'unlock'" :size="12" />
          {{ activeTab.readOnly ? t('statusReadOnly') : t('statusEditable') }}
        </button>
      </div>
    </footer>

    <!-- ==================== All Modals ==================== -->
    <Modals
      :active-modal="activeModal"
      :projects="projects"
      :active-project-id="activeProjectId"
      :editing-project="editingProject"
      :editing-profile="editingProfile"
      :editing-profile-databases="editingProfile ? (rawDatabasesByProfile[editingProfile.id] ?? []) : []"
      :grid-sql-context="gridSqlContext"
      :encoding-context="encodingModalContext"
      :danger-context="dangerContext"
      :param-context="paramContext"
      :password-profile="passwordProfile"
      :unsaved-context="unsavedContext"
      :search-items="searchItems"
      :exported-json="exportedJson"
      :shortcuts-initial-tab="shortcutsInitialTab"
      :max-rows-limit="maxRowsLimit"
      :engine-key="activeEngineKey"
      :connections="connections"
      :conn-health-by-profile="connHealthByProfile"
      :busy="busy"
      @close="
        pendingCloseTabId = null;
        pendingParamExec = null;
        unsavedContext = null;
        activeModal = 'none';
      "
      @save-project="handleSaveProject"
      @delete-project="handleDeleteProject"
      @execute-grid-sql="handleExecuteGridSql"
      @save-connection="handleSaveConnection"
      @select-edit-profile="editingProfile = $event"
      @new-edit-profile="editingProfile = null"
      @duplicate-edit-profile="handleDuplicateProfileInModal"
      @delete-edit-profile="handleDeleteProfileInModal"
      @import-dbeaver-path="handleImportDbeaverPath"
      @import-dbeaver-text="handleImportDbeaverText"
      @export-dbeaver-json="handleExportDbeaverJson"
      @export-dbeaver-dbp="handleExportDbeaverDbp"
      @export-dbeaver-folder="handleExportDbeaverFolder"
      @export-connections-xlsx="handleExportConnectionsXlsx"
      @test-profile-result="handleModalTestResult"
      @reload-encoding="handleReloadEncoding"
      @convert-encoding="handleConvertEncoding"
      @confirm-danger="confirmDangerExecution"
      @confirm-params="handleConfirmParams"
      @run-params-raw="handleRunParamsRaw"
      @submit-password="handleSubmitPassword"
      @select-search-item="handleSelectSearchItem"
      @unsaved-save-close="confirmUnsavedSaveClose"
      @unsaved-discard-close="confirmUnsavedDiscardClose"
      @notify="setStatus($event)"
    />

    <TableToolsModals
      :active-tool-modal="activeToolModal"
      :table-context="toolTableContext"
      :db-context="toolDbContext"
      :busy="busy"
      @close="activeToolModal = 'none'"
      @run-statements="handleRunToolStatements"
      @send-sql-to-editor="handleSendToolSqlToEditor"
      @notify="setStatus($event)"
    />

    <AdvancedToolsModals
      :modal-kind="advancedModalKind"
      :connections="connections"
      :databases-by-profile="databasesByProfile"
      :tables-by-key="tablesByKey"
      :columns-by-key="columnsByKey"
      :indexes-by-key="indexesByKey"
      :initial-profile-id="advancedInitialProfileId"
      :initial-db-name="advancedInitialDbName"
      :initial-table-name="advancedInitialTableName"
      :redis-init-data="redisEditorInit"
      :busy="busy"
      @close="advancedModalKind = 'none'"
      @jump-query="handleJumpQueryFromTool"
      @jump-structure="handleJumpStructureFromTool"
      @send-sql-to-editor="handleSendAdvancedSqlToEditor"
      @run-statements="handleRunAdvancedStatements"
      @notify="setStatus($event)"
    />

    <ServerMonitorModal
      :visible="serverMonitorVisible"
      :profile="serverMonitorProfile"
      :busy="busy"
      @close="serverMonitorVisible = false"
      @kill-session="handleKillServerSession"
      @notify="setStatus($event)"
    />

    <ErDiagramModal
      :visible="erDiagramVisible"
      :profile="erDiagramProfile"
      :db-name="erDiagramDbName"
      :tables="erDiagramProfile && erDiagramDbName ? (tablesByKey[`${erDiagramProfile.id}:${erDiagramDbName}`] ?? []) : []"
      :columns-by-key="columnsByKey"
      :foreign-keys-by-key="foreignKeysByKey"
      @close="erDiagramVisible = false"
      @query-table="
        (prof, db, tbl) => {
          erDiagramVisible = false;
          handleQueryTop100(prof, db, tbl);
        }
      "
      @notify="setStatus($event)"
    />

    <CommercialToolsModals
      :modal-kind="commercialModalKind"
      :connections="connections"
      :databases-by-profile="databasesByProfile"
      :tables-by-key="tablesByKey"
      :columns-by-key="columnsByKey"
      :indexes-by-key="indexesByKey"
      :foreign-keys-by-key="foreignKeysByKey"
      :routines-by-key="routinesByKey"
      :initial-profile-id="commercialProfileId"
      :initial-db-name="commercialDbName"
      :initial-table-name="commercialTableName"
      :busy="busy"
      @close="commercialModalKind = 'none'"
      @send-sql-to-editor="handleSendCommercialSqlToEditor"
      @run-statements="handleRunCommercialStatements"
      @notify="setStatus($event)"
    />

    <!-- Editor Tab Right-Click Context Menu -->
    <div
      v-if="tabCtxMenu.visible && tabCtxTab"
      class="context-menu"
      :style="{ left: `${tabCtxMenu.x}px`, top: `${tabCtxMenu.y}px`, zIndex: 95 }"
      @click.stop
    >
      <button
        class="dropdown-item"
        @click="
          tabCtxMenu.visible = false;
          closeEditorTab(tabCtxTab.id);
        "
      >
        <span>关闭</span>
        <span class="shortcut-hint">{{ keymap.getDisplay('closeTab') }}</span>
      </button>
      <button
        class="dropdown-item"
        :disabled="tabs.length <= 1"
        @click="
          tabCtxMenu.visible = false;
          closeOtherTabs(tabCtxTab.id);
        "
      >
        <span>关闭其他标签页</span>
      </button>
      <button
        class="dropdown-item"
        @click="
          tabCtxMenu.visible = false;
          closeAllTabs();
        "
      >
        <span>关闭所有标签页</span>
      </button>
      <button
        class="dropdown-item"
        :disabled="tabCtxIndex <= 0"
        @click="
          tabCtxMenu.visible = false;
          closeTabsToLeft(tabCtxTab.id);
        "
      >
        <span>关闭左侧标签页</span>
      </button>
      <button
        class="dropdown-item"
        :disabled="tabCtxIndex < 0 || tabCtxIndex >= tabs.length - 1"
        @click="
          tabCtxMenu.visible = false;
          closeTabsToRight(tabCtxTab.id);
        "
      >
        <span>关闭右侧标签页</span>
      </button>
      <button
        class="dropdown-item"
        :disabled="!tabs.some((t) => !isTabDirty(t))"
        @click="
          tabCtxMenu.visible = false;
          closeUnmodifiedTabs();
        "
      >
        <span>关闭未修改的标签页</span>
      </button>
      <div class="dropdown-sep" />
      <button
        class="dropdown-item"
        :disabled="closedTabsStack.length === 0"
        @click="
          tabCtxMenu.visible = false;
          reopenLastClosedTab();
        "
      >
        <span>恢复刚关闭的标签页</span>
        <span class="shortcut-hint">{{ keymap.getDisplay('reopenTab') }}</span>
      </button>
      <template v-if="tabCtxTab.path">
        <div class="dropdown-sep" />
        <button
          class="dropdown-item"
          @click="
            tabCtxMenu.visible = false;
            locateFileInTreeByPath(tabCtxTab.path!);
          "
        >
          <span>在文件树中定位</span>
        </button>
        <button
          class="dropdown-item"
          @click="
            tabCtxMenu.visible = false;
            handleRevealInExplorer(tabCtxTab.path!);
          "
        >
          <span>在{{ FILE_MANAGER_LABEL }}中定位</span>
        </button>
        <button
          class="dropdown-item"
          @click="
            tabCtxMenu.visible = false;
            copyText(tabCtxTab.path!);
          "
        >
          <span>复制文件路径</span>
        </button>
      </template>
      <template v-else>
        <div class="dropdown-sep" />
        <button
          class="dropdown-item"
          @click="
            tabCtxMenu.visible = false;
            copyText(tabCtxTab.title);
          "
        >
          <span>复制标签页名称</span>
        </button>
      </template>
    </div>
  </div>
</template>
