<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue';
import {
  executeSql,
  isTauriRuntime,
  listColumns,
  listDatabases,
  listForeignKeys,
  listIndexes,
  listRoutines,
  listTables,
  pickSaveDocPath,
  saveTextFile,
  type ColumnMeta,
  type ConnectionProfile,
  type DatabaseMeta,
  type ForeignKeyMeta,
  type IndexMeta,
  type RoutineMeta,
  type TableMeta,
} from '../api/ipc';
import {
  buildTablePageQuerySql,
  buildTransferCreateTableSql,
  computeDataDiff,
  generateDataDictionaryHtml,
  generateDataDictionaryMarkdown,
  generateDataDiffSyncSql,
  type DataDiffEntry,
  type DataDiffKind,
  type DataDiffSummary,
  type DictionaryTableEntry,
} from '../utils/commercialTools';
import {
  downloadTextFile,
  formatSqlLiteral,
  getEngineDisplayName,
  isConnectionProfileReadOnly,
  isRedisEngineKey,
  quoteIdentifier,
} from '../utils/sqlTools';

export type CommercialModalKind = 'none' | 'data-diff' | 'data-transfer' | 'data-dict';

const props = defineProps<{
  modalKind: CommercialModalKind;
  connections: ConnectionProfile[];
  databasesByProfile: Record<string, DatabaseMeta[]>;
  tablesByKey: Record<string, TableMeta[]>;
  columnsByKey: Record<string, ColumnMeta[]>;
  indexesByKey: Record<string, IndexMeta[]>;
  foreignKeysByKey: Record<string, ForeignKeyMeta[]>;
  routinesByKey: Record<string, RoutineMeta[]>;
  initialProfileId: string;
  initialDbName: string;
  initialTableName: string;
  busy: boolean;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
  (e: 'send-sql-to-editor', sql: string, profileId?: string, dbName?: string): void;
  (
    e: 'run-statements',
    statements: string[],
    label: string,
    profileId: string,
    dbName: string,
    refreshDb?: boolean,
  ): void;
  (e: 'notify', message: string): void;
}>();

function engineKeyOfProfile(prof: ConnectionProfile | undefined | null): string {
  if (!prof) return 'mysql';
  if (typeof prof.dbType === 'string') return prof.dbType;
  return Object.values(prof.dbType)[0] ?? 'mysql';
}

function qualifyTargetTable(tableName: string, dbName: string, engineKey: string): string {
  const q = (id: string) => quoteIdentifier(id, engineKey);
  const db = dbName.trim();
  if (db && db !== 'main' && db !== 'default') {
    return `${q(db)}.${q(tableName)}`;
  }
  return q(tableName);
}

const sqlConnections = computed(() =>
  props.connections.filter((c) => !isRedisEngineKey(engineKeyOfProfile(c))),
);

const localDbs = reactive<Record<string, DatabaseMeta[]>>({});
const localTables = reactive<Record<string, TableMeta[]>>({});
const localCols = reactive<Record<string, ColumnMeta[]>>({});
const localIdxs = reactive<Record<string, IndexMeta[]>>({});
const localFks = reactive<Record<string, ForeignKeyMeta[]>>({});
const localRoutines = reactive<Record<string, RoutineMeta[]>>({});

async function fetchDbsCached(profileId: string): Promise<DatabaseMeta[]> {
  if (!profileId) return [];
  if (props.databasesByProfile[profileId]?.length) return props.databasesByProfile[profileId];
  if (localDbs[profileId]?.length) return localDbs[profileId];
  const dbs = await listDatabases(profileId);
  localDbs[profileId] = dbs;
  return dbs;
}

async function fetchTablesCached(profileId: string, dbName: string): Promise<TableMeta[]> {
  if (!profileId || !dbName) return [];
  const k = `${profileId}:${dbName}`;
  if (props.tablesByKey[k]?.length) return props.tablesByKey[k];
  if (localTables[k]?.length) return localTables[k];
  const tbls = await listTables(profileId, dbName);
  localTables[k] = tbls;
  return tbls;
}

async function fetchColsCached(
  profileId: string,
  dbName: string,
  tableName: string,
): Promise<ColumnMeta[]> {
  if (!profileId || !dbName || !tableName) return [];
  const k = `${profileId}:${dbName}:${tableName}`;
  if (props.columnsByKey[k]?.length) return props.columnsByKey[k];
  if (localCols[k]?.length) return localCols[k];
  const cols = await listColumns(profileId, dbName, tableName);
  localCols[k] = cols;
  return cols;
}

async function fetchIndexesCached(
  profileId: string,
  dbName: string,
  tableName: string,
): Promise<IndexMeta[]> {
  if (!profileId || !dbName || !tableName) return [];
  const k = `${profileId}:${dbName}:${tableName}`;
  if (props.indexesByKey[k]) return props.indexesByKey[k];
  if (localIdxs[k]) return localIdxs[k];
  try {
    const idxs = await listIndexes(profileId, dbName, tableName);
    localIdxs[k] = idxs;
    return idxs;
  } catch {
    return [];
  }
}

async function fetchFksCached(
  profileId: string,
  dbName: string,
  tableName: string,
): Promise<ForeignKeyMeta[]> {
  if (!profileId || !dbName || !tableName) return [];
  const k = `${profileId}:${dbName}:${tableName}`;
  if (props.foreignKeysByKey[k]) return props.foreignKeysByKey[k];
  if (localFks[k]) return localFks[k];
  try {
    const fks = await listForeignKeys(profileId, dbName, tableName);
    localFks[k] = fks;
    return fks;
  } catch {
    return [];
  }
}

async function fetchRoutinesCached(profileId: string, dbName: string): Promise<RoutineMeta[]> {
  if (!profileId || !dbName) return [];
  const k = `${profileId}:${dbName}`;
  if (props.routinesByKey[k]) return props.routinesByKey[k];
  if (localRoutines[k]) return localRoutines[k];
  try {
    const list = await listRoutines(profileId, dbName);
    localRoutines[k] = list;
    return list;
  } catch {
    return [];
  }
}

async function runSelectRows(
  profileId: string,
  dbName: string,
  sql: string,
  limit: number,
): Promise<{ columns: string[]; rows: Record<string, unknown>[] }> {
  const columns: string[] = [];
  const rows: Record<string, unknown>[] = [];
  const summary = await executeSql(profileId, dbName || null, sql, limit, (chunk) => {
    if (columns.length === 0 && chunk.columns.length > 0) {
      columns.push(...chunk.columns);
    }
    if (chunk.rows.length > 0) {
      rows.push(...chunk.rows);
    }
  });
  if (!summary.success) {
    throw new Error(summary.errorMessage || '查询失败');
  }
  return { columns, rows };
}

function formatDiffVal(v: unknown): string {
  if (v === null || v === undefined) return '<NULL>';
  if (typeof v === 'object') return JSON.stringify(v);
  const s = String(v);
  return s.length > 80 ? `${s.slice(0, 77)}...` : s;
}

// ==================== 1. Data Diff & Sync ====================
const diffSrcProfileId = ref('');
const diffSrcDb = ref('');
const diffSrcTable = ref('');
const diffSrcDbs = ref<DatabaseMeta[]>([]);
const diffSrcTables = ref<TableMeta[]>([]);

const diffTgtProfileId = ref('');
const diffTgtDb = ref('');
const diffTgtTable = ref('');
const diffTgtDbs = ref<DatabaseMeta[]>([]);
const diffTgtTables = ref<TableMeta[]>([]);

const diffCandidateCols = ref<string[]>([]);
const diffKeyCols = ref<string[]>([]);
const diffSampleLimit = ref(1000);
const diffRunning = ref(false);
const diffError = ref('');
const diffResult = ref<DataDiffSummary | null>(null);
const diffFilterKind = ref<'diff-only' | 'all' | DataDiffKind>('diff-only');
const diffIncludeInserts = ref(true);
const diffIncludeUpdates = ref(true);
const diffIncludeDeletes = ref(false);
const diffConfirmProd = ref(false);

const diffSrcProfile = computed(
  () => sqlConnections.value.find((c) => c.id === diffSrcProfileId.value) ?? null,
);
const diffTgtProfile = computed(
  () => sqlConnections.value.find((c) => c.id === diffTgtProfileId.value) ?? null,
);

const filteredDiffEntries = computed<DataDiffEntry[]>(() => {
  const res = diffResult.value;
  if (!res) return [];
  if (diffFilterKind.value === 'all') return res.entries.slice(0, 300);
  if (diffFilterKind.value === 'diff-only') {
    return res.entries.filter((e) => e.kind !== 'same').slice(0, 300);
  }
  return res.entries.filter((e) => e.kind === diffFilterKind.value).slice(0, 300);
});

const diffSyncStatements = computed<string[]>(() => {
  if (!diffResult.value || !diffTgtTable.value) return [];
  return generateDataDiffSyncSql(diffResult.value, {
    targetTable: diffTgtTable.value,
    targetDb: diffTgtDb.value,
    engineKey: engineKeyOfProfile(diffTgtProfile.value),
    includeInserts: diffIncludeInserts.value,
    includeUpdates: diffIncludeUpdates.value,
    includeDeletes: diffIncludeDeletes.value,
  });
});

async function loadDiffKeyColumns() {
  diffCandidateCols.value = [];
  diffKeyCols.value = [];
  if (!diffSrcProfileId.value || !diffSrcDb.value || !diffSrcTable.value) return;
  try {
    const srcCols = await fetchColsCached(
      diffSrcProfileId.value,
      diffSrcDb.value,
      diffSrcTable.value,
    );
    let tgtCols: ColumnMeta[] = [];
    if (diffTgtProfileId.value && diffTgtDb.value && diffTgtTable.value) {
      tgtCols = await fetchColsCached(
        diffTgtProfileId.value,
        diffTgtDb.value,
        diffTgtTable.value,
      ).catch(() => []);
    }
    const union = Array.from(
      new Set([...srcCols.map((c) => c.name), ...tgtCols.map((c) => c.name)]),
    );
    diffCandidateCols.value = union;
    const pks = srcCols.filter((c) => c.isPrimaryKey).map((c) => c.name);
    diffKeyCols.value = pks.length > 0 ? pks : union.slice(0, 1);
  } catch {
    // non-fatal
  }
}

async function onDiffSideChange(side: 'src' | 'tgt', level: 'profile' | 'db' | 'table') {
  diffResult.value = null;
  diffError.value = '';
  diffConfirmProd.value = false;
  if (side === 'src') {
    if (level === 'profile') {
      diffSrcDbs.value = await fetchDbsCached(diffSrcProfileId.value).catch(() => []);
      if (!diffSrcDbs.value.some((d) => d.name === diffSrcDb.value)) {
        diffSrcDb.value = diffSrcDbs.value[0]?.name ?? '';
      }
    }
    if (level === 'profile' || level === 'db') {
      diffSrcTables.value = diffSrcDb.value
        ? await fetchTablesCached(diffSrcProfileId.value, diffSrcDb.value).catch(() => [])
        : [];
      if (!diffSrcTables.value.some((t) => t.name === diffSrcTable.value)) {
        diffSrcTable.value = diffSrcTables.value[0]?.name ?? '';
      }
    }
  } else {
    if (level === 'profile') {
      diffTgtDbs.value = await fetchDbsCached(diffTgtProfileId.value).catch(() => []);
      if (!diffTgtDbs.value.some((d) => d.name === diffTgtDb.value)) {
        diffTgtDb.value = diffTgtDbs.value[0]?.name ?? '';
      }
    }
    if (level === 'profile' || level === 'db') {
      diffTgtTables.value = diffTgtDb.value
        ? await fetchTablesCached(diffTgtProfileId.value, diffTgtDb.value).catch(() => [])
        : [];
      if (diffSrcTable.value && diffTgtTables.value.some((t) => t.name === diffSrcTable.value)) {
        diffTgtTable.value = diffSrcTable.value;
      } else if (!diffTgtTables.value.some((t) => t.name === diffTgtTable.value)) {
        diffTgtTable.value = diffTgtTables.value[0]?.name ?? '';
      }
    }
  }
  await loadDiffKeyColumns();
}

function toggleDiffKeyCol(col: string) {
  if (diffKeyCols.value.includes(col)) {
    if (diffKeyCols.value.length <= 1) {
      emit('notify', '至少保留 1 个主键/匹配列');
      return;
    }
    diffKeyCols.value = diffKeyCols.value.filter((c) => c !== col);
  } else {
    diffKeyCols.value = [...diffKeyCols.value, col];
  }
}

async function runDataDiffCompare() {
  if (!diffSrcProfileId.value || !diffSrcDb.value || !diffSrcTable.value) {
    diffError.value = '请选择源数据源与源表';
    return;
  }
  if (!diffTgtProfileId.value || !diffTgtDb.value || !diffTgtTable.value) {
    diffError.value = '请选择目标数据源与目标表';
    return;
  }
  diffRunning.value = true;
  diffError.value = '';
  diffResult.value = null;
  diffConfirmProd.value = false;
  try {
    const srcEngine = engineKeyOfProfile(diffSrcProfile.value);
    const tgtEngine = engineKeyOfProfile(diffTgtProfile.value);
    const keyCols = diffKeyCols.value;
    const orderBy =
      keyCols.length > 0
        ? keyCols.map((k) => quoteIdentifier(k, srcEngine)).join(', ')
        : undefined;
    const tgtOrderBy =
      keyCols.length > 0
        ? keyCols.map((k) => quoteIdentifier(k, tgtEngine)).join(', ')
        : undefined;

    const srcSql = buildTablePageQuerySql({
      dbName: diffSrcDb.value,
      tableName: diffSrcTable.value,
      engineKey: srcEngine,
      orderByClause: orderBy,
      limit: diffSampleLimit.value,
      offset: 0,
    });
    const tgtSql = buildTablePageQuerySql({
      dbName: diffTgtDb.value,
      tableName: diffTgtTable.value,
      engineKey: tgtEngine,
      orderByClause: tgtOrderBy,
      limit: diffSampleLimit.value,
      offset: 0,
    });

    const [srcRes, tgtRes, srcCols, tgtCols] = await Promise.all([
      runSelectRows(diffSrcProfileId.value, diffSrcDb.value, srcSql, diffSampleLimit.value),
      runSelectRows(diffTgtProfileId.value, diffTgtDb.value, tgtSql, diffSampleLimit.value),
      fetchColsCached(diffSrcProfileId.value, diffSrcDb.value, diffSrcTable.value).catch(
        () => [] as ColumnMeta[],
      ),
      fetchColsCached(diffTgtProfileId.value, diffTgtDb.value, diffTgtTable.value).catch(
        () => [] as ColumnMeta[],
      ),
    ]);

    const sourceColumns =
      srcRes.columns.length > 0 ? srcRes.columns : srcCols.map((c) => c.name);
    const targetColumns =
      tgtRes.columns.length > 0 ? tgtRes.columns : tgtCols.map((c) => c.name);

    diffResult.value = computeDataDiff({
      sourceColumns,
      sourceRows: srcRes.rows,
      targetColumns,
      targetRows: tgtRes.rows,
      keyColumns: diffKeyCols.value,
    });
  } catch (err) {
    diffError.value = String(err);
  } finally {
    diffRunning.value = false;
  }
}

async function copyTextSafe(text: string, msg: string) {
  if (!text) return;
  try {
    await navigator.clipboard.writeText(text);
    emit('notify', msg);
  } catch {
    emit('notify', '复制到剪贴板失败，请尝试发送到编辑器');
  }
}

function sendDiffSyncToEditor() {
  if (diffSyncStatements.value.length === 0) return;
  const header = `-- Data Sync Script (${diffSrcTable.value} -> ${diffTgtTable.value})\n`;
  emit(
    'send-sql-to-editor',
    header + diffSyncStatements.value.join('\n'),
    diffTgtProfileId.value,
    diffTgtDb.value,
  );
  emit('close');
}

function executeDiffSyncSql() {
  if (diffSyncStatements.value.length === 0) return;
  const tgt = diffTgtProfile.value;
  if (!tgt) return;
  if (isConnectionProfileReadOnly(tgt)) {
    emit('notify', `目标连接「${tgt.name}」为只读模式，已禁止执行同步写入`);
    return;
  }
  if (tgt.env === 'PROD' && !diffConfirmProd.value) {
    emit('notify', '目标为 PROD 生产环境，请先勾选下方生产确认选项');
    return;
  }
  emit(
    'run-statements',
    diffSyncStatements.value,
    `数据同步 ${diffTgtTable.value}`,
    diffTgtProfileId.value,
    diffTgtDb.value,
    true,
  );
  emit('close');
}

// ==================== 2. Cross-Engine Data Transfer Wizard ====================
const xferSrcProfileId = ref('');
const xferSrcDb = ref('');
const xferSrcDbs = ref<DatabaseMeta[]>([]);
const xferSrcTables = ref<TableMeta[]>([]);
const xferTableFilter = ref('');
const xferSelectedTables = ref<string[]>([]);

const xferTgtProfileId = ref('');
const xferTgtDb = ref('');
const xferTgtDbs = ref<DatabaseMeta[]>([]);

const xferCreateDdl = ref(true);
const xferCopyData = ref(true);
const xferBatchSize = ref(100);
const xferRowLimitPerTable = ref(5000);
const xferProdConfirmed = ref(false);
const xferRunning = ref(false);
const xferDoneCount = ref(0);
const xferTotalCount = ref(0);
const xferLogs = ref<string[]>([]);

const xferSrcProfile = computed(
  () => sqlConnections.value.find((c) => c.id === xferSrcProfileId.value) ?? null,
);
const xferTgtProfile = computed(
  () => sqlConnections.value.find((c) => c.id === xferTgtProfileId.value) ?? null,
);

const filteredXferTables = computed(() => {
  const q = xferTableFilter.value.trim().toLowerCase();
  const base = xferSrcTables.value.filter(
    (t) => !(t.tableType ?? '').toUpperCase().includes('VIEW'),
  );
  if (!q) return base;
  return base.filter((t) => t.name.toLowerCase().includes(q));
});

async function onXferSideChange(side: 'src' | 'tgt', level: 'profile' | 'db') {
  xferProdConfirmed.value = false;
  if (side === 'src') {
    if (level === 'profile') {
      xferSrcDbs.value = await fetchDbsCached(xferSrcProfileId.value).catch(() => []);
      if (!xferSrcDbs.value.some((d) => d.name === xferSrcDb.value)) {
        xferSrcDb.value = xferSrcDbs.value[0]?.name ?? '';
      }
    }
    xferSrcTables.value = xferSrcDb.value
      ? await fetchTablesCached(xferSrcProfileId.value, xferSrcDb.value).catch(() => [])
      : [];
    if (
      props.initialTableName &&
      xferSrcTables.value.some((t) => t.name === props.initialTableName)
    ) {
      xferSelectedTables.value = [props.initialTableName];
    } else {
      xferSelectedTables.value = xferSrcTables.value
        .filter((t) => !(t.tableType ?? '').toUpperCase().includes('VIEW'))
        .slice(0, 10)
        .map((t) => t.name);
    }
  } else {
    if (level === 'profile') {
      xferTgtDbs.value = await fetchDbsCached(xferTgtProfileId.value).catch(() => []);
      if (!xferTgtDbs.value.some((d) => d.name === xferTgtDb.value)) {
        xferTgtDb.value = xferTgtDbs.value[0]?.name ?? '';
      }
    }
  }
}

function toggleXferTable(name: string) {
  if (xferSelectedTables.value.includes(name)) {
    xferSelectedTables.value = xferSelectedTables.value.filter((n) => n !== name);
  } else {
    xferSelectedTables.value = [...xferSelectedTables.value, name];
  }
}

function selectAllXferTables(selectAll: boolean) {
  xferSelectedTables.value = selectAll
    ? filteredXferTables.value.map((t) => t.name)
    : [];
}

function buildInsertBatches(opts: {
  tableName: string;
  targetDb: string;
  targetEngineKey: string;
  columns: string[];
  rows: Record<string, unknown>[];
  batchSize: number;
}): string[] {
  if (opts.rows.length === 0 || opts.columns.length === 0) return [];
  const q = (id: string) => quoteIdentifier(id, opts.targetEngineKey);
  const tableRef = qualifyTargetTable(opts.tableName, opts.targetDb, opts.targetEngineKey);
  const colList = opts.columns.map(q).join(', ');
  const stmts: string[] = [];
  const size = Math.max(1, opts.batchSize);

  for (let i = 0; i < opts.rows.length; i += size) {
    const slice = opts.rows.slice(i, i + size);
    const tuples = slice.map(
      (r) => `(${opts.columns.map((c) => formatSqlLiteral(r[c])).join(', ')})`,
    );
    stmts.push(`INSERT INTO ${tableRef} (${colList}) VALUES\n${tuples.join(',\n')};`);
  }
  return stmts;
}

async function previewFirstTransferSql() {
  const firstTable = xferSelectedTables.value[0];
  if (!firstTable || !xferSrcProfileId.value || !xferSrcDb.value) {
    emit('notify', '请先至少选择 1 张源表');
    return;
  }
  const srcEngine = engineKeyOfProfile(xferSrcProfile.value);
  const tgtEngine = engineKeyOfProfile(xferTgtProfile.value);
  const chunks: string[] = [
    `-- 跨库迁移 SQL 预览: ${xferSrcDb.value}.${firstTable} -> ${xferTgtDb.value || '(default)'}.${firstTable}`,
  ];
  try {
    const cols = await fetchColsCached(xferSrcProfileId.value, xferSrcDb.value, firstTable);
    if (xferCreateDdl.value && cols.length > 0) {
      chunks.push(
        buildTransferCreateTableSql({
          targetDb: xferTgtDb.value,
          tableName: firstTable,
          columns: cols,
          targetEngineKey: tgtEngine,
        }),
      );
    }
    if (xferCopyData.value) {
      const selSql = buildTablePageQuerySql({
        dbName: xferSrcDb.value,
        tableName: firstTable,
        engineKey: srcEngine,
        limit: Math.min(20, xferRowLimitPerTable.value),
        offset: 0,
      });
      const res = await runSelectRows(
        xferSrcProfileId.value,
        xferSrcDb.value,
        selSql,
        Math.min(20, xferRowLimitPerTable.value),
      );
      const colNames = res.columns.length > 0 ? res.columns : cols.map((c) => c.name);
      chunks.push(
        ...buildInsertBatches({
          tableName: firstTable,
          targetDb: xferTgtDb.value,
          targetEngineKey: tgtEngine,
          columns: colNames,
          rows: res.rows,
          batchSize: xferBatchSize.value,
        }),
      );
    }
    emit(
      'send-sql-to-editor',
      chunks.join('\n\n'),
      xferTgtProfileId.value || undefined,
      xferTgtDb.value || undefined,
    );
    emit('close');
  } catch (err) {
    emit('notify', `生成预览 SQL 失败: ${String(err)}`);
  }
}

async function runDataTransfer() {
  if (!xferSrcProfileId.value || !xferSrcDb.value || !xferTgtProfileId.value || !xferTgtDb.value) {
    emit('notify', '请完整选择源库与目标库');
    return;
  }
  if (xferSelectedTables.value.length === 0) {
    emit('notify', '请至少选择 1 张需要迁移的表');
    return;
  }
  const tgtProf = xferTgtProfile.value;
  if (!tgtProf) return;
  if (isConnectionProfileReadOnly(tgtProf)) {
    emit('notify', `目标连接「${tgtProf.name}」为只读模式，无法写入迁移数据`);
    return;
  }
  if (tgtProf.env === 'PROD' && !xferProdConfirmed.value) {
    emit('notify', '目标连接为 PROD 生产环境，请先勾选下方安全确认复选框');
    return;
  }

  xferRunning.value = true;
  xferDoneCount.value = 0;
  xferTotalCount.value = xferSelectedTables.value.length;
  xferLogs.value = [];

  const srcEngine = engineKeyOfProfile(xferSrcProfile.value);
  const tgtEngine = engineKeyOfProfile(tgtProf);
  let successTables = 0;

  try {
    for (const tbl of xferSelectedTables.value) {
      try {
        const cols = await fetchColsCached(xferSrcProfileId.value, xferSrcDb.value, tbl);
        if (xferCreateDdl.value) {
          if (cols.length === 0) {
            xferLogs.value.push(`⚠ [${tbl}] 未获取到字段信息，跳过建表`);
          } else {
            const ddl = buildTransferCreateTableSql({
              targetDb: xferTgtDb.value,
              tableName: tbl,
              columns: cols,
              targetEngineKey: tgtEngine,
            });
            const ddlRes = await executeSql(xferTgtProfileId.value, xferTgtDb.value, ddl, null, () => {});
            if (!ddlRes.success) {
              throw new Error(`建表失败: ${ddlRes.errorMessage || '未知错误'}`);
            }
          }
        }

        let insertedRows = 0;
        if (xferCopyData.value) {
          const selSql = buildTablePageQuerySql({
            dbName: xferSrcDb.value,
            tableName: tbl,
            engineKey: srcEngine,
            limit: xferRowLimitPerTable.value,
            offset: 0,
          });
          const srcData = await runSelectRows(
            xferSrcProfileId.value,
            xferSrcDb.value,
            selSql,
            xferRowLimitPerTable.value,
          );
          const colNames = srcData.columns.length > 0 ? srcData.columns : cols.map((c) => c.name);
          const batches = buildInsertBatches({
            tableName: tbl,
            targetDb: xferTgtDb.value,
            targetEngineKey: tgtEngine,
            columns: colNames,
            rows: srcData.rows,
            batchSize: xferBatchSize.value,
          });
          for (const stmt of batches) {
            const insRes = await executeSql(
              xferTgtProfileId.value,
              xferTgtDb.value,
              stmt,
              null,
              () => {},
            );
            if (!insRes.success) {
              throw new Error(`写入失败: ${insRes.errorMessage || '未知错误'}`);
            }
          }
          insertedRows = srcData.rows.length;
        }
        successTables++;
        xferLogs.value.push(`✓ [${tbl}] 迁移成功${xferCopyData.value ? ` (${insertedRows} 行)` : ' (仅建表)'}`);
      } catch (err) {
        xferLogs.value.push(`✕ [${tbl}] 失败: ${String(err)}`);
      }
      xferDoneCount.value += 1;
    }
    emit('notify', `数据迁移完成：成功 ${successTables}/${xferTotalCount.value} 张表`);
  } finally {
    xferRunning.value = false;
  }
}

// ==================== 3. Data Dictionary Export ====================
const dictProfileId = ref('');
const dictDbName = ref('');
const dictDbs = ref<DatabaseMeta[]>([]);
const dictTables = ref<TableMeta[]>([]);
const dictSelectedTables = ref<string[]>([]);
const dictIncludeIndexes = ref(true);
const dictIncludeFks = ref(true);
const dictIncludeRoutines = ref(true);
const dictFormat = ref<'md' | 'html'>('md');
const dictGenerating = ref(false);
const dictProgressText = ref('');
const dictPreviewContent = ref('');

const dictProfile = computed(
  () => sqlConnections.value.find((c) => c.id === dictProfileId.value) ?? null,
);

async function onDictSelectionChange(level: 'profile' | 'db') {
  dictPreviewContent.value = '';
  if (level === 'profile') {
    dictDbs.value = await fetchDbsCached(dictProfileId.value).catch(() => []);
    if (!dictDbs.value.some((d) => d.name === dictDbName.value)) {
      dictDbName.value = dictDbs.value[0]?.name ?? '';
    }
  }
  dictTables.value = dictDbName.value
    ? await fetchTablesCached(dictProfileId.value, dictDbName.value).catch(() => [])
    : [];
  if (
    props.initialTableName &&
    dictTables.value.some((t) => t.name === props.initialTableName)
  ) {
    dictSelectedTables.value = [props.initialTableName];
  } else {
    dictSelectedTables.value = dictTables.value.map((t) => t.name);
  }
}

function toggleDictTable(name: string) {
  if (dictSelectedTables.value.includes(name)) {
    dictSelectedTables.value = dictSelectedTables.value.filter((n) => n !== name);
  } else {
    dictSelectedTables.value = [...dictSelectedTables.value, name];
  }
}

async function generateDictionaryPreview() {
  if (!dictProfileId.value || !dictDbName.value || !dictProfile.value) {
    emit('notify', '请选择数据源和数据库');
    return;
  }
  const chosen = dictTables.value.filter((t) => dictSelectedTables.value.includes(t.name));
  if (chosen.length === 0) {
    emit('notify', '请至少选择 1 张表');
    return;
  }

  dictGenerating.value = true;
  dictProgressText.value = `准备加载 0/${chosen.length} 张表元数据...`;
  try {
    const entries: DictionaryTableEntry[] = [];
    const chunkSize = 6;
    for (let i = 0; i < chosen.length; i += chunkSize) {
      const slice = chosen.slice(i, i + chunkSize);
      const batchRes = await Promise.all(
        slice.map(async (meta): Promise<DictionaryTableEntry> => {
          const [columns, indexes, foreignKeys] = await Promise.all([
            fetchColsCached(dictProfileId.value, dictDbName.value, meta.name).catch(() => []),
            dictIncludeIndexes.value
              ? fetchIndexesCached(dictProfileId.value, dictDbName.value, meta.name)
              : Promise.resolve([]),
            dictIncludeFks.value
              ? fetchFksCached(dictProfileId.value, dictDbName.value, meta.name)
              : Promise.resolve([]),
          ]);
          return { meta, columns, indexes, foreignKeys };
        }),
      );
      entries.push(...batchRes);
      dictProgressText.value = `已加载 ${Math.min(entries.length, chosen.length)}/${chosen.length} 张表元数据...`;
    }

    const routines = dictIncludeRoutines.value
      ? await fetchRoutinesCached(dictProfileId.value, dictDbName.value)
      : [];

    const payload = {
      profileName: dictProfile.value.name,
      engineKey: engineKeyOfProfile(dictProfile.value),
      dbName: dictDbName.value,
      generatedAt: new Date().toLocaleString(),
      tables: entries,
      routines,
    };
    dictPreviewContent.value =
      dictFormat.value === 'html'
        ? generateDataDictionaryHtml(payload)
        : generateDataDictionaryMarkdown(payload);
    dictProgressText.value = `已生成 ${entries.length} 张表的数据字典 (${dictFormat.value.toUpperCase()})`;
  } catch (err) {
    emit('notify', `生成数据字典失败: ${String(err)}`);
  } finally {
    dictGenerating.value = false;
  }
}

async function saveDictionaryFile() {
  if (!dictPreviewContent.value) {
    await generateDictionaryPreview();
  }
  if (!dictPreviewContent.value) return;
  const ext = dictFormat.value;
  const safeDb = (dictDbName.value || 'database').replace(/[^\w.\-\u4e00-\u9fa5]/g, '_');
  const defaultFileName = `${safeDb}-data-dictionary.${ext}`;
  try {
    if (isTauriRuntime()) {
      const targetPath = await pickSaveDocPath(defaultFileName, ext);
      if (!targetPath) return;
      await saveTextFile(targetPath, dictPreviewContent.value, 'UTF-8', 'LF', false);
      emit('notify', `数据字典已保存 -> ${targetPath}`);
    } else {
      downloadTextFile(
        defaultFileName,
        dictPreviewContent.value,
        ext === 'html' ? 'text/html;charset=utf-8' : 'text/markdown;charset=utf-8',
      );
      emit('notify', `已下载 ${defaultFileName}`);
    }
  } catch (err) {
    emit('notify', `导出数据字典失败: ${String(err)}`);
  }
}

// ==================== Lifecycle Initialization ====================
watch(
  () => props.modalKind,
  async (kind) => {
    if (kind === 'data-diff') {
      const defSrc = props.initialProfileId || sqlConnections.value[0]?.id || '';
      diffSrcProfileId.value = defSrc;
      diffSrcDb.value = props.initialDbName || '';
      diffSrcTable.value = props.initialTableName || '';
      diffTgtProfileId.value =
        sqlConnections.value.find((c) => c.id !== defSrc)?.id ?? defSrc;
      diffTgtDb.value = '';
      diffTgtTable.value = '';
      await onDiffSideChange('src', 'profile');
      await onDiffSideChange('tgt', 'profile');
    } else if (kind === 'data-transfer') {
      const defSrc = props.initialProfileId || sqlConnections.value[0]?.id || '';
      xferSrcProfileId.value = defSrc;
      xferSrcDb.value = props.initialDbName || '';
      xferTgtProfileId.value =
        sqlConnections.value.find((c) => c.id !== defSrc)?.id ?? defSrc;
      xferTgtDb.value = '';
      xferLogs.value = [];
      xferDoneCount.value = 0;
      xferTotalCount.value = 0;
      await onXferSideChange('src', 'profile');
      await onXferSideChange('tgt', 'profile');
    } else if (kind === 'data-dict') {
      dictProfileId.value = props.initialProfileId || sqlConnections.value[0]?.id || '';
      dictDbName.value = props.initialDbName || '';
      dictProgressText.value = '';
      await onDictSelectionChange('profile');
    }
  },
);
</script>

<template>
  <!-- 1. Data Diff & Sync SQL Modal -->
  <div v-if="modalKind === 'data-diff'" class="modal-backdrop" @click.self="emit('close')">
    <div class="modal-card commercial-modal-card" style="width: min(1040px, 95vw)">
      <div class="modal-header">
        <span>🧬 表数据内容对比与同步 SQL</span>
        <button class="icon-btn compact" @click="emit('close')">×</button>
      </div>

      <div class="commercial-grid-2">
        <div class="commercial-side-box">
          <div class="commercial-side-title">基准源表 (Source)</div>
          <div class="commercial-picker-row">
            <select
              v-model="diffSrcProfileId"
              class="field"
              @change="onDiffSideChange('src', 'profile')"
            >
              <option v-for="c in sqlConnections" :key="c.id" :value="c.id">
                {{ c.name }} ({{ getEngineDisplayName(engineKeyOfProfile(c)) }})
              </option>
            </select>
            <select v-model="diffSrcDb" class="field" @change="onDiffSideChange('src', 'db')">
              <option v-for="d in diffSrcDbs" :key="d.name" :value="d.name">{{ d.name }}</option>
            </select>
            <select v-model="diffSrcTable" class="field" @change="onDiffSideChange('src', 'table')">
              <option v-for="t in diffSrcTables" :key="t.name" :value="t.name">
                {{ t.name }}
              </option>
            </select>
          </div>
        </div>

        <div class="commercial-side-box">
          <div class="commercial-side-title">目标对比表 (Target)</div>
          <div class="commercial-picker-row">
            <select
              v-model="diffTgtProfileId"
              class="field"
              @change="onDiffSideChange('tgt', 'profile')"
            >
              <option v-for="c in sqlConnections" :key="c.id" :value="c.id">
                {{ c.name }} ({{ getEngineDisplayName(engineKeyOfProfile(c)) }})
              </option>
            </select>
            <select v-model="diffTgtDb" class="field" @change="onDiffSideChange('tgt', 'db')">
              <option v-for="d in diffTgtDbs" :key="d.name" :value="d.name">{{ d.name }}</option>
            </select>
            <select v-model="diffTgtTable" class="field" @change="onDiffSideChange('tgt', 'table')">
              <option v-for="t in diffTgtTables" :key="t.name" :value="t.name">
                {{ t.name }}
              </option>
            </select>
          </div>
        </div>
      </div>

      <div class="commercial-toolbar-row">
        <div class="commercial-key-chips">
          <span class="commercial-muted-label">匹配键列:</span>
          <button
            v-for="col in diffCandidateCols.slice(0, 14)"
            :key="col"
            class="commercial-chip"
            :class="{ active: diffKeyCols.includes(col) }"
            type="button"
            @click="toggleDiffKeyCol(col)"
          >
            {{ col }}
          </button>
          <span v-if="diffCandidateCols.length === 0" class="commercial-muted-label">默认首列</span>
        </div>

        <div style="display: flex; align-items: center; gap: 8px; flex-shrink: 0">
          <span class="commercial-muted-label">采样上限</span>
          <select v-model.number="diffSampleLimit" class="field" style="width: 88px; padding: 3px 6px">
            <option :value="200">200</option>
            <option :value="500">500</option>
            <option :value="1000">1000</option>
            <option :value="3000">3000</option>
          </select>
          <button
            class="btn btn-sm btn-primary"
            :disabled="diffRunning || busy"
            @click="runDataDiffCompare"
          >
            {{ diffRunning ? '对比中...' : '开始数据对比' }}
          </button>
        </div>
      </div>

      <div v-if="diffError" class="commercial-error-banner">{{ diffError }}</div>

      <template v-if="diffResult">
        <div class="commercial-kpi-row">
          <div style="display: flex; align-items: center; gap: 6px; flex-wrap: wrap">
            <button
              class="commercial-chip"
              :class="{ active: diffFilterKind === 'diff-only' }"
              @click="diffFilterKind = 'diff-only'"
            >
              仅看差异 ({{ diffResult.addedCount + diffResult.modifiedCount + diffResult.removedCount }})
            </button>
            <button
              class="commercial-chip"
              :class="{ active: diffFilterKind === 'added' }"
              @click="diffFilterKind = 'added'"
            >
              + 新增 {{ diffResult.addedCount }}
            </button>
            <button
              class="commercial-chip"
              :class="{ active: diffFilterKind === 'modified' }"
              @click="diffFilterKind = 'modified'"
            >
              ~ 修改 {{ diffResult.modifiedCount }}
            </button>
            <button
              class="commercial-chip"
              :class="{ active: diffFilterKind === 'removed' }"
              @click="diffFilterKind = 'removed'"
            >
              - 仅目标存在 {{ diffResult.removedCount }}
            </button>
            <button
              class="commercial-chip"
              :class="{ active: diffFilterKind === 'all' }"
              @click="diffFilterKind = 'all'"
            >
              全部 (含相同 {{ diffResult.sameCount }})
            </button>
          </div>
          <span class="commercial-muted-label">键列: {{ diffResult.keyColumns.join(', ') }}</span>
        </div>

        <div class="commercial-scroll-box" style="max-height: 280px">
          <table class="commercial-mini-table">
            <thead>
              <tr>
                <th style="width: 90px">状态</th>
                <th style="width: 220px">匹配键</th>
                <th>列级差异 (源 -> 目标)</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="(entry, idx) in filteredDiffEntries" :key="idx">
                <td>
                  <span class="diff-badge" :class="`diff-${entry.kind}`">
                    {{
                      entry.kind === 'added'
                        ? '新增'
                        : entry.kind === 'modified'
                          ? '修改'
                          : entry.kind === 'removed'
                            ? '删除'
                            : '相同'
                    }}
                  </span>
                </td>
                <td style="font-family: var(--font-mono)">{{ entry.keyDisplay }}</td>
                <td style="font-family: var(--font-mono)">
                  <template v-if="entry.kind === 'modified'">
                    <span
                      v-for="col in entry.changedColumns"
                      :key="col"
                      class="diff-col-tag"
                    >
                      {{ col }}: {{ formatDiffVal(entry.targetRow?.[col]) }} → {{ formatDiffVal(entry.sourceRow?.[col]) }}
                    </span>
                  </template>
                  <span v-else-if="entry.kind === 'added'" class="commercial-muted-label">
                    目标表缺失此行，可生成 INSERT
                  </span>
                  <span v-else-if="entry.kind === 'removed'" class="commercial-muted-label">
                    源表无此行，目标表多余
                  </span>
                  <span v-else class="commercial-muted-label">内容完全一致</span>
                </td>
              </tr>
              <tr v-if="filteredDiffEntries.length === 0">
                <td colspan="3" style="text-align: center; padding: 18px" class="commercial-muted-label">
                  当前筛选条件下无记录
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <div class="commercial-sync-footer">
          <div style="display: flex; align-items: center; gap: 12px; flex-wrap: wrap">
            <label class="commercial-check">
              <input v-model="diffIncludeInserts" type="checkbox" />
              <span>包含 INSERT ({{ diffResult.addedCount }})</span>
            </label>
            <label class="commercial-check">
              <input v-model="diffIncludeUpdates" type="checkbox" />
              <span>包含 UPDATE ({{ diffResult.modifiedCount }})</span>
            </label>
            <label class="commercial-check">
              <input v-model="diffIncludeDeletes" type="checkbox" />
              <span>包含 DELETE ({{ diffResult.removedCount }})</span>
            </label>
            <label
              v-if="diffTgtProfile?.env === 'PROD'"
              class="commercial-check"
              style="color: var(--danger)"
            >
              <input v-model="diffConfirmProd" type="checkbox" />
              <span>已确认向 PROD 生产库执行同步</span>
            </label>
          </div>

          <div style="display: flex; align-items: center; gap: 6px">
            <button
              class="btn btn-sm"
              :disabled="diffSyncStatements.length === 0"
              @click="copyTextSafe(diffSyncStatements.join('\n'), `已复制 ${diffSyncStatements.length} 条同步 SQL`)"
            >
              复制 SQL ({{ diffSyncStatements.length }})
            </button>
            <button
              class="btn btn-sm"
              :disabled="diffSyncStatements.length === 0"
              @click="sendDiffSyncToEditor"
            >
              发送到编辑器
            </button>
            <button
              class="btn btn-sm btn-primary"
              :disabled="diffSyncStatements.length === 0 || (diffTgtProfile ? isConnectionProfileReadOnly(diffTgtProfile) : true)"
              @click="executeDiffSyncSql"
            >
              执行同步 SQL
            </button>
          </div>
        </div>
      </template>

      <div v-else-if="!diffRunning" class="commercial-empty-tip">
        选择源表和目标表后点击「开始数据对比」，将按匹配键比对行级内容并自动生成可审核的 INSERT / UPDATE / DELETE 同步脚本。
      </div>
    </div>
  </div>

  <!-- 2. Cross-Engine Data Transfer Wizard Modal -->
  <div v-else-if="modalKind === 'data-transfer'" class="modal-backdrop" @click.self="emit('close')">
    <div class="modal-card commercial-modal-card" style="width: min(980px, 95vw)">
      <div class="modal-header">
        <span>🚚 跨库 / 跨引擎数据迁移向导</span>
        <button class="icon-btn compact" @click="emit('close')">×</button>
      </div>

      <div class="commercial-grid-2">
        <div class="commercial-side-box">
          <div class="commercial-side-title">1. 源数据库连接</div>
          <div class="commercial-picker-row">
            <select
              v-model="xferSrcProfileId"
              class="field"
              @change="onXferSideChange('src', 'profile')"
            >
              <option v-for="c in sqlConnections" :key="c.id" :value="c.id">
                {{ c.name }} ({{ getEngineDisplayName(engineKeyOfProfile(c)) }})
              </option>
            </select>
            <select v-model="xferSrcDb" class="field" @change="onXferSideChange('src', 'db')">
              <option v-for="d in xferSrcDbs" :key="d.name" :value="d.name">{{ d.name }}</option>
            </select>
          </div>
        </div>

        <div class="commercial-side-box">
          <div class="commercial-side-title">2. 目标数据库连接 (支持跨引擎 DDL 方言转换)</div>
          <div class="commercial-picker-row">
            <select
              v-model="xferTgtProfileId"
              class="field"
              @change="onXferSideChange('tgt', 'profile')"
            >
              <option v-for="c in sqlConnections" :key="c.id" :value="c.id">
                {{ c.name }} ({{ getEngineDisplayName(engineKeyOfProfile(c)) }})
              </option>
            </select>
            <select v-model="xferTgtDb" class="field" @change="onXferSideChange('tgt', 'db')">
              <option v-for="d in xferTgtDbs" :key="d.name" :value="d.name">{{ d.name }}</option>
            </select>
          </div>
        </div>
      </div>

      <div class="commercial-toolbar-row" style="margin-top: 10px">
        <div style="display: flex; align-items: center; gap: 8px; flex: 1">
          <input
            v-model="xferTableFilter"
            class="field"
            style="max-width: 240px"
            placeholder="筛选待迁移数据表..."
          />
          <button class="btn btn-sm" @click="selectAllXferTables(true)">全选</button>
          <button class="btn btn-sm" @click="selectAllXferTables(false)">清空</button>
          <span class="commercial-muted-label">已选 {{ xferSelectedTables.length }} 张表</span>
        </div>

        <div style="display: flex; align-items: center; gap: 10px; flex-wrap: wrap">
          <label class="commercial-check">
            <input v-model="xferCreateDdl" type="checkbox" />
            <span>自动建表 (IF NOT EXISTS)</span>
          </label>
          <label class="commercial-check">
            <input v-model="xferCopyData" type="checkbox" />
            <span>迁移数据行</span>
          </label>
          <span class="commercial-muted-label">批次</span>
          <select v-model.number="xferBatchSize" class="field" style="width: 74px; padding: 3px 6px">
            <option :value="50">50</option>
            <option :value="100">100</option>
            <option :value="200">200</option>
          </select>
          <span class="commercial-muted-label">上限</span>
          <select
            v-model.number="xferRowLimitPerTable"
            class="field"
            style="width: 84px; padding: 3px 6px"
          >
            <option :value="1000">1000</option>
            <option :value="5000">5000</option>
            <option :value="10000">10000</option>
          </select>
        </div>
      </div>

      <div class="commercial-checklist-box">
        <label
          v-for="tbl in filteredXferTables"
          :key="tbl.name"
          class="commercial-check-item"
        >
          <input
            type="checkbox"
            :checked="xferSelectedTables.includes(tbl.name)"
            @change="toggleXferTable(tbl.name)"
          />
          <span style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap">
            {{ tbl.name }}
          </span>
        </label>
      </div>

      <div
        v-if="xferTgtProfile?.env === 'PROD'"
        class="commercial-prod-guard"
      >
        <label class="commercial-check" style="color: var(--danger); font-weight: 600">
          <input v-model="xferProdConfirmed" type="checkbox" />
          <span>目标为 PROD 生产连接「{{ xferTgtProfile.name }}」，勾选确认向生产库执行跨库迁移写入</span>
        </label>
      </div>

      <div v-if="xferTotalCount > 0 || xferLogs.length > 0" class="commercial-progress-box">
        <div style="display: flex; justify-content: space-between; font-size: 11.5px; margin-bottom: 4px">
          <span>迁移进度: {{ xferDoneCount }} / {{ xferTotalCount }}</span>
          <span v-if="xferRunning">正在执行...</span>
        </div>
        <div class="commercial-progress-track">
          <div
            class="commercial-progress-fill"
            :style="{
              width: `${xferTotalCount > 0 ? Math.round((xferDoneCount / xferTotalCount) * 100) : 0}%`,
            }"
          />
        </div>
        <div v-if="xferLogs.length > 0" class="commercial-log-box">
          <div v-for="(line, idx) in xferLogs" :key="idx">{{ line }}</div>
        </div>
      </div>

      <div class="modal-footer" style="margin-top: 10px">
        <button
          class="btn btn-sm"
          :disabled="xferRunning || xferSelectedTables.length === 0"
          @click="previewFirstTransferSql"
        >
          预览首表迁移 SQL
        </button>
        <div style="display: flex; gap: 8px">
          <button class="btn btn-sm" :disabled="xferRunning" @click="emit('close')">关闭</button>
          <button
            class="btn btn-sm btn-primary"
            :disabled="
              xferRunning ||
              xferSelectedTables.length === 0 ||
              (xferTgtProfile ? isConnectionProfileReadOnly(xferTgtProfile) : true)
            "
            @click="runDataTransfer"
          >
            {{ xferRunning ? '迁移中...' : '开始执行迁移' }}
          </button>
        </div>
      </div>
    </div>
  </div>

  <!-- 3. Data Dictionary Generator Modal -->
  <div v-else-if="modalKind === 'data-dict'" class="modal-backdrop" @click.self="emit('close')">
    <div class="modal-card commercial-modal-card" style="width: min(980px, 95vw)">
      <div class="modal-header">
        <span>📘 数据库数据字典生成与导出</span>
        <button class="icon-btn compact" @click="emit('close')">×</button>
      </div>

      <div class="commercial-toolbar-row">
        <div style="display: flex; align-items: center; gap: 8px; flex-wrap: wrap">
          <select
            v-model="dictProfileId"
            class="field"
            style="width: 210px"
            @change="onDictSelectionChange('profile')"
          >
            <option v-for="c in sqlConnections" :key="c.id" :value="c.id">
              {{ c.name }} ({{ getEngineDisplayName(engineKeyOfProfile(c)) }})
            </option>
          </select>
          <select
            v-model="dictDbName"
            class="field"
            style="width: 170px"
            @change="onDictSelectionChange('db')"
          >
            <option v-for="d in dictDbs" :key="d.name" :value="d.name">{{ d.name }}</option>
          </select>
          <button
            class="btn btn-sm"
            @click="dictSelectedTables = dictTables.map((t) => t.name)"
          >
            全选表 ({{ dictTables.length }})
          </button>
          <button class="btn btn-sm" @click="dictSelectedTables = []">清空</button>
        </div>

        <div style="display: flex; align-items: center; gap: 10px; flex-wrap: wrap">
          <label class="commercial-check">
            <input v-model="dictIncludeIndexes" type="checkbox" />
            <span>索引</span>
          </label>
          <label class="commercial-check">
            <input v-model="dictIncludeFks" type="checkbox" />
            <span>外键</span>
          </label>
          <label class="commercial-check">
            <input v-model="dictIncludeRoutines" type="checkbox" />
            <span>过程/触发器</span>
          </label>
          <select v-model="dictFormat" class="field" style="width: 138px; padding: 3px 6px">
            <option value="md">Markdown (.md)</option>
            <option value="html">HTML 文档 (.html)</option>
          </select>
          <button
            class="btn btn-sm btn-primary"
            :disabled="dictGenerating || dictSelectedTables.length === 0"
            @click="generateDictionaryPreview"
          >
            {{ dictGenerating ? '生成中...' : '生成预览' }}
          </button>
        </div>
      </div>

      <div class="commercial-checklist-box" style="max-height: 120px">
        <label v-for="tbl in dictTables" :key="tbl.name" class="commercial-check-item">
          <input
            type="checkbox"
            :checked="dictSelectedTables.includes(tbl.name)"
            @change="toggleDictTable(tbl.name)"
          />
          <span style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap">
            {{ tbl.name }}
          </span>
        </label>
      </div>

      <div v-if="dictProgressText" class="commercial-muted-label" style="margin-top: 6px">
        {{ dictProgressText }}
      </div>

      <pre class="commercial-dict-preview">{{
        dictPreviewContent || '点击「生成预览」汇总选中表的字段、主键、默认值、注释、索引、外键及例程元数据。'
      }}</pre>

      <div class="modal-footer" style="margin-top: 10px">
        <button
          class="btn btn-sm"
          :disabled="!dictPreviewContent"
          @click="copyTextSafe(dictPreviewContent, '已复制数据字典内容到剪贴板')"
        >
          复制内容
        </button>
        <div style="display: flex; gap: 8px">
          <button class="btn btn-sm" @click="emit('close')">关闭</button>
          <button
            class="btn btn-sm btn-primary"
            :disabled="dictGenerating || dictSelectedTables.length === 0"
            @click="saveDictionaryFile"
          >
            导出保存到文件 ({{ dictFormat === 'html' ? '.html' : '.md' }})
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
