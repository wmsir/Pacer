<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import {
  analyzeSql,
  createDemoSqliteConnection,
  deleteConnection,
  executeSql,
  explainSql,
  exportDataGripXml,
  getTableDdl,
  importDataGripPath,
  importDataGripXml,
  isTauriRuntime,
  listColumns,
  listConnections,
  listDatabases,
  listExtensions,
  listQueryHistory,
  listTables,
  saveConnection,
  testConnection,
  type ColumnMeta,
  type ConnectionDraft,
  type ConnectionProfile,
  type DatabaseMeta,
  type DirectConfig,
  type EnvLevel,
  type ExtensionMeta,
  type QueryHistoryEntry,
  type QuerySummary,
  type ScriptAnalysis,
  type TableMeta,
} from './api/ipc';

interface ConsoleTab {
  id: string;
  title: string;
  sql: string;
  profileId: string;
  dbName: string;
}

const tauriReady = isTauriRuntime();
const connections = ref<ConnectionProfile[]>([]);
const extensions = ref<ExtensionMeta[]>([]);

const databasesByProfile = ref<Record<string, DatabaseMeta[]>>({});
const tablesByKey = ref<Record<string, TableMeta[]>>({});
const columnsByKey = ref<Record<string, ColumnMeta[]>>({});
const expandedProfiles = ref<Record<string, boolean>>({});
const expandedDatabases = ref<Record<string, boolean>>({});
const expandedTables = ref<Record<string, boolean>>({});

const consoles = ref<ConsoleTab[]>([
  {
    id: 'console-1',
    title: 'console_1.sql',
    sql: 'SELECT c.full_name, c.tier, o.order_no, o.total_amount, o.status\nFROM orders o\nJOIN customers c ON o.customer_id = c.id\nORDER BY o.id;',
    profileId: '',
    dbName: '',
  },
]);
const activeConsoleId = ref('console-1');
let consoleSeq = 1;

const activeConsole = computed(
  () => consoles.value.find((c) => c.id === activeConsoleId.value) ?? consoles.value[0],
);

const activeProfile = computed(
  () => connections.value.find((p) => p.id === activeConsole.value?.profileId) ?? null,
);

const availableDatabases = computed<DatabaseMeta[]>(() => {
  if (!activeProfile.value) return [];
  return databasesByProfile.value[activeProfile.value.id] ?? [];
});

const astAnalysis = ref<ScriptAnalysis | null>(null);
const activeOutputTab = ref<'grid' | 'ddl' | 'history'>('grid');
const gridColumns = ref<string[]>([]);
const gridRows = ref<Record<string, unknown>[]>([]);
const gridFilter = ref('');
const querySummary = ref<QuerySummary | null>(null);
const ddlText = ref('');
const ddlTargetLabel = ref('');
const historyEntries = ref<QueryHistoryEntry[]>([]);

const statusBanner = ref<{ text: string; isError: boolean } | null>(null);
const busy = ref(false);

const showConnModal = ref(false);
const editingProfileId = ref<string | null>(null);
const connForm = ref<{
  name: string;
  dbType: string;
  host: string;
  port: string;
  database: string;
  username: string;
  password: string;
  jdbcUrl: string;
  env: EnvLevel;
  readOnly: boolean;
  comment: string;
}>({
  name: '',
  dbType: 'sqlite',
  host: 'localhost',
  port: '',
  database: '',
  username: '',
  password: '',
  jdbcUrl: '',
  env: 'DEV',
  readOnly: false,
  comment: '',
});

const showImportModal = ref(false);
const importXmlText = ref('');
const importPathText = ref('');
const exportedXmlPreview = ref('');

const showExtensionsModal = ref(false);

const passwordPrompt = ref<{
  visible: boolean;
  profileId: string;
  profileName: string;
  value: string;
  onConfirm: ((pwd: string) => Promise<void>) | null;
}>({
  visible: false,
  profileId: '',
  profileName: '',
  value: '',
  onConfirm: null,
});

const ENGINE_OPTIONS = [
  { value: 'sqlite', label: 'SQLite (Native)', defaultPort: '' },
  { value: 'mySql', label: 'MySQL (Native)', defaultPort: '3306' },
  { value: 'mariaDb', label: 'MariaDB (Native)', defaultPort: '3306' },
  { value: 'postgreSql', label: 'PostgreSQL (Native)', defaultPort: '5432' },
  { value: 'tiDb', label: 'TiDB (MySQL Protocol)', defaultPort: '4000' },
  { value: 'oceanBase', label: 'OceanBase (MySQL Protocol)', defaultPort: '2881' },
  { value: 'openGauss', label: 'openGauss (PG Protocol)', defaultPort: '5432' },
  { value: 'kingbase', label: 'KingbaseES (PG Protocol)', defaultPort: '54321' },
  { value: 'oracle', label: 'Oracle (Import/AST)', defaultPort: '1521' },
  { value: 'dameng', label: 'Dameng DM8 (Import/AST)', defaultPort: '5236' },
  { value: 'sqlServer', label: 'SQL Server (Import/AST)', defaultPort: '1433' },
];

const filteredGridRows = computed(() => {
  const needle = gridFilter.value.trim().toLowerCase();
  if (!needle) return gridRows.value;
  return gridRows.value.filter((row) =>
    gridColumns.value.some((col) =>
      String(row[col] ?? '')
        .toLowerCase()
        .includes(needle),
    ),
  );
});

function engineKey(dbType: string | Record<string, string>): string {
  if (typeof dbType === 'string') return dbType;
  return Object.values(dbType)[0] ?? 'unknown';
}

function engineLabel(dbType: string | Record<string, string>): string {
  return engineKey(dbType).toUpperCase();
}

function envClass(env: string): string {
  if (env === 'PROD') return 'badge-prod';
  if (env === 'TEST') return 'badge-test';
  if (env === 'DEV') return 'badge-dev';
  return 'badge-neutral';
}

function setBanner(text: string, isError = false) {
  statusBanner.value = { text, isError };
}

let astTimer: ReturnType<typeof setTimeout> | null = null;
watch(
  [() => activeConsole.value?.sql, activeProfile],
  () => {
    if (astTimer) clearTimeout(astTimer);
    astTimer = setTimeout(async () => {
      if (!activeConsole.value) return;
      try {
        astAnalysis.value = await analyzeSql(
          activeConsole.value.sql,
          activeProfile.value?.dbType ?? 'sqlite',
        );
      } catch {
        // Ignore transient AST parse noise while typing.
      }
    }, 150);
  },
  { immediate: true },
);

async function reloadConnections() {
  connections.value = await listConnections();
  if (activeConsole.value && !activeConsole.value.profileId && connections.value.length > 0) {
    activeConsole.value.profileId = connections.value[0].id;
    activeConsole.value.dbName =
      (connections.value[0].fingerprint.database as string) || 'main';
  }
}

async function handleCreateDemoDb() {
  busy.value = true;
  try {
    const demo = await createDemoSqliteConnection();
    await reloadConnections();
    if (activeConsole.value) {
      activeConsole.value.profileId = demo.id;
      activeConsole.value.dbName = 'main';
    }
    await toggleExpandProfile(demo, true);
    setBanner(`已就绪示例 SQLite 数据库: ${demo.name}`);
  } catch (err) {
    setBanner(`创建示例数据库失败: ${String(err)}`, true);
  } finally {
    busy.value = false;
  }
}

function openNewConnectionModal() {
  editingProfileId.value = null;
  connForm.value = {
    name: '',
    dbType: 'sqlite',
    host: 'localhost',
    port: '',
    database: '',
    username: '',
    password: '',
    jdbcUrl: '',
    env: 'DEV',
    readOnly: false,
    comment: '',
  };
  showConnModal.value = true;
}

function openEditConnectionModal(profile: ConnectionProfile) {
  editingProfileId.value = profile.id;
  const direct =
    profile.mode.kind === 'direct' ? (profile.mode.config as DirectConfig) : undefined;
  const eng = engineKey(profile.dbType);
  connForm.value = {
    name: profile.name,
    dbType: eng,
    host: direct?.host ?? profile.fingerprint.host ?? 'localhost',
    port: String(direct?.port ?? profile.fingerprint.port ?? ''),
    database: direct?.database ?? profile.fingerprint.database ?? '',
    username: direct?.username ?? '',
    password: '',
    jdbcUrl: direct?.jdbcUrl ?? '',
    env: profile.env === 'UNKNOWN' ? 'DEV' : profile.env,
    readOnly: direct?.readOnly ?? !profile.capabilities.canDml,
    comment: profile.comment ?? '',
  };
  showConnModal.value = true;
}

function onFormEngineChange() {
  const opt = ENGINE_OPTIONS.find((o) => o.value === connForm.value.dbType);
  if (opt && !editingProfileId.value) {
    connForm.value.port = opt.defaultPort;
  }
}

async function handleSaveConnection(testAfterSave = false) {
  if (!connForm.value.name.trim()) {
    setBanner('请输入连接名称', true);
    return;
  }
  busy.value = true;
  try {
    const portNum = connForm.value.port.trim() ? Number(connForm.value.port.trim()) : null;
    const draft: ConnectionDraft = {
      id: editingProfileId.value,
      name: connForm.value.name.trim(),
      dbType: connForm.value.dbType,
      host: connForm.value.dbType === 'sqlite' ? null : connForm.value.host.trim() || null,
      port: connForm.value.dbType === 'sqlite' ? null : portNum,
      database: connForm.value.database.trim() || null,
      jdbcUrl: connForm.value.jdbcUrl.trim() || null,
      username: connForm.value.username.trim() || null,
      env: connForm.value.env,
      readOnly: connForm.value.readOnly,
      comment: connForm.value.comment.trim() || null,
    };
    const saved = await saveConnection(draft, connForm.value.password || null);
    await reloadConnections();
    if (activeConsole.value) {
      activeConsole.value.profileId = saved.id;
      activeConsole.value.dbName = (saved.fingerprint.database as string) || '';
    }
    if (testAfterSave) {
      const ping = await testConnection(saved.id, connForm.value.password || null);
      setBanner(`连接测试成功: ${ping.serverVersion ?? ''} (${ping.latencyMs} ms)`);
    } else {
      setBanner(`已保存连接: ${saved.name}`);
    }
    showConnModal.value = false;
  } catch (err) {
    setBanner(`保存/测试连接失败: ${String(err)}`, true);
  } finally {
    busy.value = false;
  }
}

async function handleDeleteConnection(profile: ConnectionProfile) {
  if (!window.confirm(`确认删除连接 "${profile.name}"？`)) return;
  try {
    await deleteConnection(profile.id);
    await reloadConnections();
    setBanner(`已删除连接: ${profile.name}`);
  } catch (err) {
    setBanner(`删除连接失败: ${String(err)}`, true);
  }
}

function promptPasswordThen(profile: ConnectionProfile, action: (pwd: string) => Promise<void>) {
  passwordPrompt.value = {
    visible: true,
    profileId: profile.id,
    profileName: profile.name,
    value: '',
    onConfirm: action,
  };
}

async function submitPasswordPrompt() {
  const fn = passwordPrompt.value.onConfirm;
  const pwd = passwordPrompt.value.value;
  passwordPrompt.value.visible = false;
  passwordPrompt.value.onConfirm = null;
  if (fn) {
    await fn(pwd);
  }
}

async function handleTestProfile(profile: ConnectionProfile) {
  busy.value = true;
  try {
    const ping = await testConnection(profile.id);
    setBanner(`[${profile.name}] ${ping.serverVersion ?? 'Connected'} · ${ping.latencyMs} ms`);
  } catch (err) {
    const msg = String(err);
    if (msg.toLowerCase().includes('access denied') || msg.toLowerCase().includes('password')) {
      promptPasswordThen(profile, async (pwd) => {
        try {
          const ping = await testConnection(profile.id, pwd);
          setBanner(`[${profile.name}] ${ping.serverVersion ?? 'Connected'} · ${ping.latencyMs} ms`);
        } catch (err2) {
          setBanner(`连接测试失败: ${String(err2)}`, true);
        }
      });
    } else {
      setBanner(`连接测试失败: ${msg}`, true);
    }
  } finally {
    busy.value = false;
  }
}

async function toggleExpandProfile(profile: ConnectionProfile, forceOpen = false) {
  const currentlyOpen = Boolean(expandedProfiles.value[profile.id]);
  if (currentlyOpen && !forceOpen) {
    expandedProfiles.value[profile.id] = false;
    return;
  }
  if (activeConsole.value) {
    activeConsole.value.profileId = profile.id;
  }
  try {
    const dbs = await listDatabases(profile.id);
    databasesByProfile.value[profile.id] = dbs;
    expandedProfiles.value[profile.id] = true;
    if (activeConsole.value && dbs.length > 0 && !activeConsole.value.dbName) {
      const preferred = dbs.find((d) => !d.isSystem) ?? dbs[0];
      activeConsole.value.dbName = preferred.name;
    }
  } catch (err) {
    const msg = String(err);
    if (msg.toLowerCase().includes('access denied') || msg.toLowerCase().includes('password')) {
      promptPasswordThen(profile, async (pwd) => {
        try {
          const dbs = await listDatabases(profile.id, pwd);
          databasesByProfile.value[profile.id] = dbs;
          expandedProfiles.value[profile.id] = true;
        } catch (err2) {
          setBanner(`加载数据库失败: ${String(err2)}`, true);
        }
      });
    } else {
      setBanner(`加载数据库失败: ${msg}`, true);
    }
  }
}

async function toggleExpandDatabase(profile: ConnectionProfile, dbName: string) {
  const key = `${profile.id}:${dbName}`;
  if (activeConsole.value) {
    activeConsole.value.profileId = profile.id;
    activeConsole.value.dbName = dbName;
  }
  if (expandedDatabases.value[key]) {
    expandedDatabases.value[key] = false;
    return;
  }
  try {
    tablesByKey.value[key] = await listTables(profile.id, dbName);
    expandedDatabases.value[key] = true;
  } catch (err) {
    setBanner(`加载表失败: ${String(err)}`, true);
  }
}

async function toggleExpandTable(profile: ConnectionProfile, dbName: string, tableName: string) {
  const key = `${profile.id}:${dbName}:${tableName}`;
  if (expandedTables.value[key]) {
    expandedTables.value[key] = false;
    return;
  }
  try {
    columnsByKey.value[key] = await listColumns(profile.id, dbName, tableName);
    expandedTables.value[key] = true;
  } catch (err) {
    setBanner(`加载字段失败: ${String(err)}`, true);
  }
}

async function previewTop100(profile: ConnectionProfile, dbName: string, tableName: string) {
  if (!activeConsole.value) return;
  activeConsole.value.profileId = profile.id;
  activeConsole.value.dbName = dbName;
  const eng = engineKey(profile.dbType);
  const quoted =
    eng === 'postgreSql' || eng === 'openGauss' || eng === 'kingbase'
      ? `"${dbName}"."${tableName}"`
      : eng === 'sqlite'
        ? `"${tableName}"`
        : `\`${dbName}\`.\`${tableName}\``;
  activeConsole.value.sql = `SELECT * FROM ${quoted} LIMIT 100;`;
  await runSql();
}

async function viewTableDdl(profile: ConnectionProfile, dbName: string, tableName: string) {
  busy.value = true;
  try {
    ddlText.value = await getTableDdl(profile.id, dbName, tableName);
    ddlTargetLabel.value = `${profile.name} · ${dbName}.${tableName}`;
    activeOutputTab.value = 'ddl';
  } catch (err) {
    setBanner(`获取 DDL 失败: ${String(err)}`, true);
  } finally {
    busy.value = false;
  }
}

async function runSql() {
  if (!activeConsole.value || !activeProfile.value) {
    setBanner('请先选择或创建数据库连接', true);
    return;
  }

  busy.value = true;
  statusBanner.value = null;
  gridColumns.value = [];
  gridRows.value = [];
  gridFilter.value = '';
  querySummary.value = null;
  activeOutputTab.value = 'grid';

  try {
    const summary = await executeSql(
      activeProfile.value.id,
      activeConsole.value.dbName || null,
      activeConsole.value.sql,
      1000,
      (chunk) => {
        if (gridColumns.value.length === 0 && chunk.columns.length > 0) {
          gridColumns.value = chunk.columns;
        }
        gridRows.value.push(...chunk.rows);
      },
    );
    querySummary.value = summary;
    if (summary.isDmlOrDdl) {
      setBanner(
        `执行完成，影响行数: ${summary.affectedRows ?? 0} (${summary.durationMs} ms)`,
      );
      const dbKey = `${activeProfile.value.id}:${activeConsole.value.dbName}`;
      if (expandedDatabases.value[dbKey]) {
        tablesByKey.value[dbKey] = await listTables(
          activeProfile.value.id,
          activeConsole.value.dbName,
        );
      }
    }
    historyEntries.value = await listQueryHistory().catch(() => historyEntries.value);
  } catch (err) {
    setBanner(`执行失败: ${String(err)}`, true);
    historyEntries.value = await listQueryHistory().catch(() => historyEntries.value);
  } finally {
    busy.value = false;
  }
}

async function runExplain() {
  if (!activeConsole.value || !activeProfile.value) return;
  busy.value = true;
  activeOutputTab.value = 'grid';
  gridColumns.value = [];
  gridRows.value = [];
  try {
    const res = await explainSql(
      activeProfile.value.id,
      activeConsole.value.dbName || 'main',
      activeConsole.value.sql,
    );
    gridColumns.value = res.columns;
    gridRows.value = res.rows;
    querySummary.value = {
      success: res.success,
      database: activeConsole.value.dbName || 'main',
      totalRows: res.rows.length,
      affectedRows: null,
      isDmlOrDdl: false,
      durationMs: res.durationMs,
      maskedColumns: {},
      errorMessage: res.errorMessage,
    };
  } catch (err) {
    setBanner(`EXPLAIN 失败: ${String(err)}`, true);
  } finally {
    busy.value = false;
  }
}

async function handleImportDataGrip() {
  busy.value = true;
  try {
    let imported: ConnectionProfile[] = [];
    if (importXmlText.value.trim()) {
      imported = await importDataGripXml(importXmlText.value);
    } else if (importPathText.value.trim()) {
      imported = await importDataGripPath(importPathText.value.trim());
    }
    await reloadConnections();
    if (imported.length > 0 && activeConsole.value) {
      activeConsole.value.profileId = imported[0].id;
    }
    setBanner(`成功导入并合并 ${imported.length} 个 DataGrip 连接`);
    showImportModal.value = false;
  } catch (err) {
    setBanner(`DataGrip 导入失败: ${String(err)}`, true);
  } finally {
    busy.value = false;
  }
}

async function handleExportDataGrip() {
  try {
    exportedXmlPreview.value = await exportDataGripXml();
    showImportModal.value = true;
  } catch (err) {
    setBanner(`导出 DataGrip XML 失败: ${String(err)}`, true);
  }
}

function downloadTextFile(filename: string, content: string, mime: string) {
  const blob = new Blob([content], { type: mime });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}

function exportResultCsv() {
  if (gridColumns.value.length === 0) return;
  const esc = (v: unknown) => {
    if (v === null || v === undefined) return '';
    const s = String(v);
    return /[",\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
  };
  const lines = [
    gridColumns.value.map(esc).join(','),
    ...filteredGridRows.value.map((r) => gridColumns.value.map((c) => esc(r[c])).join(',')),
  ];
  downloadTextFile('query_result.csv', lines.join('\n'), 'text/csv;charset=utf-8');
}

function exportResultJson() {
  if (filteredGridRows.value.length === 0) return;
  downloadTextFile(
    'query_result.json',
    JSON.stringify(filteredGridRows.value, null, 2),
    'application/json;charset=utf-8',
  );
}

function addConsoleTab() {
  consoleSeq += 1;
  const id = `console-${consoleSeq}`;
  consoles.value.push({
    id,
    title: `console_${consoleSeq}.sql`,
    sql: 'SELECT 1;',
    profileId: activeConsole.value?.profileId ?? '',
    dbName: activeConsole.value?.dbName ?? '',
  });
  activeConsoleId.value = id;
}

function closeConsoleTab(id: string) {
  if (consoles.value.length <= 1) return;
  const idx = consoles.value.findIndex((c) => c.id === id);
  consoles.value = consoles.value.filter((c) => c.id !== id);
  if (activeConsoleId.value === id) {
    activeConsoleId.value = consoles.value[Math.max(0, idx - 1)].id;
  }
}

function loadHistorySql(entry: QueryHistoryEntry) {
  if (!activeConsole.value) return;
  activeConsole.value.sql = entry.sql;
  activeConsole.value.profileId = entry.profileId;
  if (entry.database) {
    activeConsole.value.dbName = entry.database;
  }
}

function onEditorKeyDown(ev: KeyboardEvent) {
  if ((ev.ctrlKey || ev.metaKey) && ev.key === 'Enter') {
    ev.preventDefault();
    void runSql();
  }
}

function copyText(text: string) {
  void navigator.clipboard?.writeText(text);
  setBanner('已复制到剪贴板');
}

onMounted(async () => {
  if (!tauriReady) {
    setBanner('当前处于纯浏览器预览；请通过 `cargo tauri dev` 启动桌面端以启用原生 SQL 驱动与 DataGrip 导入。');
    return;
  }
  extensions.value = await listExtensions().catch(() => []);
  await reloadConnections().catch(() => {});
  if (connections.value.length === 0) {
    await handleCreateDemoDb();
  }
});
</script>

<template>
  <div class="workbench">
    <header class="topbar">
      <div class="brand">
        <span>PacerGrip</span>
        <span class="brand-tag">Rust + Tauri v2 · DataGrip Native</span>
      </div>
      <div class="topbar-actions">
        <button class="btn btn-primary" @click="openNewConnectionModal">+ 新建连接</button>
        <button class="btn" :disabled="busy" @click="handleCreateDemoDb">示例 SQLite 库</button>
        <button class="btn" @click="showImportModal = true">导入 / 导出 DataGrip XML</button>
        <button class="btn" @click="showExtensionsModal = true">
          扩展插槽 ({{ extensions.length }})
        </button>
      </div>
    </header>

    <div v-if="statusBanner" class="status-banner" :class="{ error: statusBanner.isError }">
      <span>{{ statusBanner.text }}</span>
      <button class="btn btn-xs" @click="statusBanner = null">×</button>
    </div>

    <div class="main-split">
      <aside class="sidebar">
        <div class="sidebar-section">
          <div class="section-header">
            <span>Database Explorer ({{ connections.length }})</span>
          </div>

          <div
            v-for="p in connections"
            :key="p.id"
            class="conn-card"
            :class="{ active: activeConsole?.profileId === p.id }"
            @click="toggleExpandProfile(p)"
          >
            <div class="conn-title-row">
              <span>{{ p.name }}</span>
              <span class="badge" :class="envClass(p.env)">{{ p.env }}</span>
            </div>
            <div class="conn-meta">
              <span class="badge badge-neutral">{{ engineLabel(p.dbType) }}</span>
              <span>
                {{
                  p.fingerprint.host
                    ? `${p.fingerprint.host}:${p.fingerprint.port ?? ''}`
                    : p.fingerprint.database || 'local'
                }}
              </span>
              <span v-if="!p.capabilities.canDml" class="badge badge-neutral">RO</span>
            </div>
            <div class="conn-actions" @click.stop>
              <button class="btn btn-xs" @click="handleTestProfile(p)">Test</button>
              <button class="btn btn-xs" @click="openEditConnectionModal(p)">Edit</button>
              <button class="btn btn-xs btn-danger" @click="handleDeleteConnection(p)">Del</button>
            </div>

            <div v-if="expandedProfiles[p.id] && databasesByProfile[p.id]" class="tree-list" @click.stop>
              <div v-for="db in databasesByProfile[p.id]" :key="db.name">
                <div
                  class="tree-node"
                  :class="{
                    selected:
                      activeConsole?.profileId === p.id && activeConsole?.dbName === db.name,
                  }"
                  @click="toggleExpandDatabase(p, db.name)"
                >
                  <span>{{ db.name }}</span>
                  <span style="opacity: 0.6">{{ db.tableCount ?? '' }}</span>
                </div>

                <div
                  v-if="
                    expandedDatabases[`${p.id}:${db.name}`] &&
                    tablesByKey[`${p.id}:${db.name}`]
                  "
                  class="tree-list"
                >
                  <div v-for="tbl in tablesByKey[`${p.id}:${db.name}`]" :key="tbl.name">
                    <div class="tree-node" @click="toggleExpandTable(p, db.name, tbl.name)">
                      <span>{{ tbl.name }}</span>
                      <span class="tree-inline-actions" @click.stop>
                        <button
                          class="btn btn-xs"
                          title="Query Top 100"
                          @click="previewTop100(p, db.name, tbl.name)"
                        >
                          100
                        </button>
                        <button
                          class="btn btn-xs"
                          title="Show Table DDL"
                          @click="viewTableDdl(p, db.name, tbl.name)"
                        >
                          DDL
                        </button>
                      </span>
                    </div>

                    <div
                      v-if="
                        expandedTables[`${p.id}:${db.name}:${tbl.name}`] &&
                        columnsByKey[`${p.id}:${db.name}:${tbl.name}`]
                      "
                      class="tree-list"
                    >
                      <div
                        v-for="col in columnsByKey[`${p.id}:${db.name}:${tbl.name}`]"
                        :key="col.name"
                        class="tree-node"
                      >
                        <span>{{ col.isPrimaryKey ? '🔑 ' : '' }}{{ col.name }}</span>
                        <span style="opacity: 0.6">{{ col.dataType }}</span>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </aside>

      <main class="center-pane">
        <div class="console-tabs">
          <div
            v-for="tab in consoles"
            :key="tab.id"
            class="console-tab"
            :class="{ active: tab.id === activeConsole?.id }"
            @click="activeConsoleId = tab.id"
          >
            <span>{{ tab.title }}</span>
            <span
              v-if="consoles.length > 1"
              class="tab-close"
              @click.stop="closeConsoleTab(tab.id)"
            >
              ×
            </span>
          </div>
          <button class="btn btn-xs" @click="addConsoleTab">+</button>
        </div>

        <div v-if="activeConsole" class="editor-header">
          <div style="display: flex; align-items: center; gap: 8px; flex-wrap: wrap">
            <select v-model="activeConsole.profileId" class="field">
              <option value="" disabled>选择连接...</option>
              <option v-for="p in connections" :key="p.id" :value="p.id">
                {{ p.name }} ({{ engineLabel(p.dbType) }})
              </option>
            </select>
            <select
              v-if="availableDatabases.length > 0"
              v-model="activeConsole.dbName"
              class="field"
            >
              <option v-for="db in availableDatabases" :key="db.name" :value="db.name">
                {{ db.name }}
              </option>
            </select>
            <input
              v-else
              v-model="activeConsole.dbName"
              class="field"
              style="width: 140px"
              placeholder="Schema / DB"
            />
          </div>

          <div class="editor-toolbar">
            <button class="btn btn-primary" :disabled="busy" @click="runSql">
              运行 (Ctrl+Enter)
            </button>
            <button class="btn" :disabled="busy || !activeProfile" @click="runExplain">
              EXPLAIN
            </button>
            <button class="btn" :disabled="gridRows.length === 0" @click="exportResultCsv">
              导出 CSV
            </button>
            <button class="btn" :disabled="gridRows.length === 0" @click="exportResultJson">
              导出 JSON
            </button>
            <button class="btn" @click="handleExportDataGrip">导出 XML</button>
          </div>
        </div>

        <div class="ast-ribbon">
          <div style="display: flex; align-items: center; gap: 8px">
            <span
              class="badge"
              :class="astAnalysis?.requiresTicket ? 'badge-test' : 'badge-dev'"
            >
              {{ astAnalysis?.overallKind ?? 'READ_ONLY' }}
            </span>
            <span style="color: var(--text-secondary)">
              {{ astAnalysis?.statements.length ?? 0 }} statement(s)
            </span>
          </div>
          <div style="color: #fcd34d; font-size: 11px">
            {{
              astAnalysis?.statements
                .flatMap((s) => s.warnings)
                .map((w) => `[${w.level}] ${w.message}`)
                .join(' | ')
            }}
          </div>
        </div>

        <textarea
          v-if="activeConsole"
          v-model="activeConsole.sql"
          class="sql-editor"
          spellcheck="false"
          @keydown="onEditorKeyDown"
        />

        <div class="output-tabs">
          <div class="tab-buttons">
            <button
              class="tab-btn"
              :class="{ active: activeOutputTab === 'grid' }"
              @click="activeOutputTab = 'grid'"
            >
              结果集 ({{ filteredGridRows.length }})
            </button>
            <button
              class="tab-btn"
              :class="{ active: activeOutputTab === 'ddl' }"
              @click="activeOutputTab = 'ddl'"
            >
              Table DDL
            </button>
            <button
              class="tab-btn"
              :class="{ active: activeOutputTab === 'history' }"
              @click="activeOutputTab = 'history'"
            >
              历史记录 ({{ historyEntries.length }})
            </button>
          </div>

          <div style="display: flex; align-items: center; gap: 8px">
            <input
              v-if="activeOutputTab === 'grid' && gridRows.length > 0"
              v-model="gridFilter"
              class="field"
              style="width: 180px; padding: 3px 8px"
              placeholder="过滤当前结果..."
            />
            <span v-if="querySummary" style="font-size: 11px; color: var(--text-secondary)">
              {{ querySummary.durationMs }} ms ·
              {{
                querySummary.isDmlOrDdl
                  ? `影响 ${querySummary.affectedRows ?? 0} 行`
                  : `${querySummary.totalRows} 行`
              }}
            </span>
          </div>
        </div>

        <div class="output-body">
          <div v-if="activeOutputTab === 'grid'">
            <div v-if="gridColumns.length === 0" style="color: var(--text-secondary)">
              选择或新建连接，编写 SQL 后按 Ctrl+Enter 运行。支持流式分页渲染、DML 影响行数统计与 EXPLAIN 计划。
            </div>
            <div v-else class="data-table-wrap">
              <table class="grid">
                <thead>
                  <tr>
                    <th v-for="col in gridColumns" :key="col">{{ col }}</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="(row, idx) in filteredGridRows" :key="idx">
                    <td
                      v-for="col in gridColumns"
                      :key="col"
                      :class="{ 'null-cell': row[col] === null }"
                    >
                      {{ row[col] === null ? '<NULL>' : row[col] }}
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>

          <div v-else-if="activeOutputTab === 'ddl'" style="display: flex; flex-direction: column; gap: 8px">
            <div style="display: flex; justify-content: space-between; align-items: center">
              <strong>{{ ddlTargetLabel || '点击左侧表的 DDL 按钮查看建表语句' }}</strong>
              <button v-if="ddlText" class="btn btn-xs" @click="copyText(ddlText)">复制 DDL</button>
            </div>
            <pre class="code-block">{{ ddlText || '暂无 DDL 内容' }}</pre>
          </div>

          <div v-else>
            <div v-if="historyEntries.length === 0" style="color: var(--text-secondary)">
              暂无执行历史记录。
            </div>
            <div v-else class="data-table-wrap">
              <table class="grid">
                <thead>
                  <tr>
                    <th>时间</th>
                    <th>连接</th>
                    <th>状态</th>
                    <th>耗时</th>
                    <th>行数 / 影响</th>
                    <th>SQL</th>
                  </tr>
                </thead>
                <tbody>
                  <tr
                    v-for="h in historyEntries"
                    :key="h.id"
                    style="cursor: pointer"
                    @click="loadHistorySql(h)"
                  >
                    <td>{{ h.timestamp }}</td>
                    <td>{{ h.profileName }}</td>
                    <td>
                      <span class="badge" :class="h.success ? 'badge-dev' : 'badge-prod'">
                        {{ h.success ? 'OK' : 'ERR' }}
                      </span>
                    </td>
                    <td>{{ h.durationMs }} ms</td>
                    <td>{{ h.affectedRows != null ? `aff: ${h.affectedRows}` : h.rowCount }}</td>
                    <td>{{ h.sql }}</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </div>
      </main>
    </div>

    <!-- New / Edit Connection Modal -->
    <div v-if="showConnModal" class="modal-backdrop" @click.self="showConnModal = false">
      <div class="modal-card">
        <div style="display: flex; justify-content: space-between; margin-bottom: 12px">
          <strong>{{ editingProfileId ? '编辑数据库连接' : '新建数据库连接' }}</strong>
          <button class="btn" @click="showConnModal = false">关闭</button>
        </div>
        <div class="form-grid">
          <div class="form-row">
            <label>连接名称</label>
            <input v-model="connForm.name" class="field" placeholder="例如 Local MySQL 8" />
          </div>
          <div class="form-row">
            <label>数据库引擎</label>
            <select v-model="connForm.dbType" class="field" @change="onFormEngineChange">
              <option v-for="opt in ENGINE_OPTIONS" :key="opt.value" :value="opt.value">
                {{ opt.label }}
              </option>
            </select>
          </div>
          <template v-if="connForm.dbType !== 'sqlite'">
            <div class="form-row">
              <label>Host</label>
              <input v-model="connForm.host" class="field" placeholder="127.0.0.1" />
            </div>
            <div class="form-row">
              <label>Port</label>
              <input v-model="connForm.port" class="field" placeholder="3306" />
            </div>
          </template>
          <div class="form-row">
            <label>{{ connForm.dbType === 'sqlite' ? 'SQLite 文件路径' : 'Database / Schema' }}</label>
            <input
              v-model="connForm.database"
              class="field"
              :placeholder="connForm.dbType === 'sqlite' ? 'C:/data/app.db' : 'orders'"
            />
          </div>
          <div class="form-row">
            <label>环境标记</label>
            <select v-model="connForm.env" class="field">
              <option value="DEV">DEV</option>
              <option value="TEST">TEST</option>
              <option value="PROD">PROD</option>
            </select>
          </div>
          <template v-if="connForm.dbType !== 'sqlite'">
            <div class="form-row">
              <label>用户名</label>
              <input v-model="connForm.username" class="field" placeholder="root" />
            </div>
            <div class="form-row">
              <label>密码（仅内存会话使用，绝不明文落盘）</label>
              <input v-model="connForm.password" type="password" class="field" placeholder="******" />
            </div>
          </template>
        </div>
        <div class="form-row" style="margin-top: 10px">
          <label>自定义 JDBC URL（可选，留空自动生成）</label>
          <input v-model="connForm.jdbcUrl" class="field" placeholder="jdbc:mysql://localhost:3306/orders" />
        </div>
        <div style="display: flex; align-items: center; justify-content: space-between; margin-top: 12px">
          <label style="display: flex; align-items: center; gap: 6px; font-size: 12px">
            <input v-model="connForm.readOnly" type="checkbox" />
            只读连接（拦截 DML / DDL）
          </label>
          <div style="display: flex; gap: 8px">
            <button class="btn" :disabled="busy" @click="handleSaveConnection(true)">测试并保存</button>
            <button class="btn btn-primary" :disabled="busy" @click="handleSaveConnection(false)">保存</button>
          </div>
        </div>
      </div>
    </div>

    <!-- Import / Export DataGrip XML Modal -->
    <div v-if="showImportModal" class="modal-backdrop" @click.self="showImportModal = false">
      <div class="modal-card">
        <div style="display: flex; justify-content: space-between; margin-bottom: 12px">
          <strong>DataGrip 连接导入 / 导出（剪贴板 XML / .idea / Settings .zip）</strong>
          <button class="btn" @click="showImportModal = false">关闭</button>
        </div>
        <div style="display: flex; flex-direction: column; gap: 10px">
          <input
            v-model="importPathText"
            class="field"
            placeholder="本地 dataSources.xml / .idea 目录 / settings.zip 路径（可选）"
          />
          <textarea
            v-model="importXmlText"
            class="field"
            rows="7"
            placeholder="或直接粘贴 DataGrip Copy Data Source(s) to Clipboard 的 XML..."
          />
          <div style="display: flex; gap: 8px">
            <button class="btn btn-primary" :disabled="busy" @click="handleImportDataGrip">
              解析并导入连接
            </button>
            <button class="btn" @click="handleExportDataGrip">生成 DataGrip XML</button>
            <button
              v-if="exportedXmlPreview"
              class="btn"
              @click="copyText(exportedXmlPreview)"
            >
              复制导出 XML
            </button>
          </div>
          <pre v-if="exportedXmlPreview" class="code-block">{{ exportedXmlPreview }}</pre>
        </div>
      </div>
    </div>

    <!-- Session Password Modal -->
    <div
      v-if="passwordPrompt.visible"
      class="modal-backdrop"
      @click.self="passwordPrompt.visible = false"
    >
      <div class="modal-card" style="width: min(420px, 90vw)">
        <div style="display: flex; justify-content: space-between; margin-bottom: 10px">
          <strong>输入连接密码（仅会话内存）</strong>
          <button class="btn" @click="passwordPrompt.visible = false">取消</button>
        </div>
        <div style="margin-bottom: 8px; color: var(--text-secondary)">
          连接: {{ passwordPrompt.profileName }}
        </div>
        <div style="display: flex; gap: 8px">
          <input
            v-model="passwordPrompt.value"
            type="password"
            class="field"
            style="flex: 1"
            placeholder="数据库密码"
            @keyup.enter="submitPasswordPrompt"
          />
          <button class="btn btn-primary" @click="submitPasswordPrompt">连接</button>
        </div>
      </div>
    </div>

    <!-- Extensions Modal -->
    <div v-if="showExtensionsModal" class="modal-backdrop" @click.self="showExtensionsModal = false">
      <div class="modal-card">
        <div style="display: flex; justify-content: space-between; margin-bottom: 12px">
          <strong>IDE 扩展插槽（Extension Registry）</strong>
          <button class="btn" @click="showExtensionsModal = false">关闭</button>
        </div>
        <div v-if="extensions.length === 0" style="color: var(--text-secondary); line-height: 1.6">
          <p>当前以纯本地 DataGrip 桌面模式运行，未挂载外部平台插件。</p>
          <p>
            已保留以下 Rust 扩展契约供后续按需接入（如 <code>wmsir/Pacer</code> 协同治理平台）：
          </p>
          <ul>
            <li><code>pacer_core::GovernanceExtension</code> — <code>before_execute</code> 预审拦截与 <code>report_audit</code> 审计上报</li>
            <li><code>pacer_core::AiSqlExtension</code> — Text2SQL 与 SQL 优化重写</li>
            <li><code>pacer_core::DbBackend</code> &amp; <code>pacer-client</code>（Cargo feature <code>pacer-extension</code>）</li>
          </ul>
        </div>
        <div v-else>
          <div v-for="ext in extensions" :key="ext.id" class="conn-card">
            <div class="conn-title-row">
              <span>{{ ext.name }} ({{ ext.version }})</span>
              <span class="badge badge-neutral">{{ ext.kind }}</span>
            </div>
            <div class="conn-meta">{{ ext.description }}</div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
