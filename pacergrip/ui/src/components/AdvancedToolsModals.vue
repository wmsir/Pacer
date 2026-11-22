<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue';
import {
  listColumns,
  listDatabases,
  listIndexes,
  listTables,
  searchObjects,
  type ColumnMeta,
  type ConnectionProfile,
  type DatabaseMeta,
  type IndexMeta,
  type ObjectSearchHit,
  type TableMeta,
} from '../api/ipc';
import IdeIcon from './IdeIcon.vue';
import {
  buildAlterTableDdl,
  buildRedisSaveCommands,
  computeTableSchemaDiff,
  getEngineDisplayName,
  isConnectionProfileReadOnly,
  isRedisEngineKey,
  quoteIdentifier,
  type AlterColumnDraft,
  type AlterIndexDraft,
  type RedisEditableType,
} from '../utils/sqlTools';

export type AdvancedModalKind =
  | 'none'
  | 'object-search'
  | 'schema-diff'
  | 'alter-table'
  | 'redis-editor';

export interface RedisEditorInitialData {
  key: string;
  keyType: RedisEditableType;
  ttlSeconds: number | null;
  stringValue?: string;
  hashEntries?: Array<{ field: string; value: string }>;
  listItems?: string[];
  setMembers?: string[];
  zsetMembers?: Array<{ member: string; score: string }>;
  isNew: boolean;
}

const props = defineProps<{
  modalKind: AdvancedModalKind;
  connections: ConnectionProfile[];
  databasesByProfile: Record<string, DatabaseMeta[]>;
  tablesByKey: Record<string, TableMeta[]>;
  columnsByKey: Record<string, ColumnMeta[]>;
  indexesByKey: Record<string, IndexMeta[]>;
  initialProfileId: string;
  initialDbName: string;
  initialTableName: string;
  redisInitData?: RedisEditorInitialData | null;
  busy: boolean;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
  (e: 'jump-query', profileId: string, dbName: string, tableName: string): void;
  (e: 'jump-structure', profileId: string, dbName: string, tableName: string): void;
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

const COMMON_ALTER_TYPES = [
  'BIGINT',
  'INT',
  'TINYINT',
  'VARCHAR(64)',
  'VARCHAR(128)',
  'VARCHAR(255)',
  'TEXT',
  'JSON',
  'DECIMAL(12,2)',
  'DATETIME',
  'TIMESTAMP',
  'DATE',
  'BOOLEAN',
];

function engineKeyOfProfile(prof: ConnectionProfile | undefined | null): string {
  if (!prof) return 'mysql';
  if (typeof prof.dbType === 'string') return prof.dbType;
  return Object.values(prof.dbType)[0] ?? 'mysql';
}

function isProfileReadOnly(prof: ConnectionProfile | undefined | null): boolean {
  return isConnectionProfileReadOnly(prof);
}

// Local lazy metadata cache so cross-connection diff & search work even before expanding tree nodes
const localDbs = reactive<Record<string, DatabaseMeta[]>>({});
const localTables = reactive<Record<string, TableMeta[]>>({});
const localCols = reactive<Record<string, ColumnMeta[]>>({});
const localIdxs = reactive<Record<string, IndexMeta[]>>({});

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
  if (props.indexesByKey[k]?.length) return props.indexesByKey[k];
  if (localIdxs[k]) return localIdxs[k];
  try {
    const idxs = await listIndexes(profileId, dbName, tableName);
    localIdxs[k] = idxs;
    return idxs;
  } catch {
    return [];
  }
}

// ==================== 1. Global Object / Column Search ====================
const objSearchProfileId = ref('');
const objSearchDbName = ref('');
const objSearchDbs = ref<DatabaseMeta[]>([]);
const objSearchQuery = ref('');
const objSearchFilter = ref<'all' | 'table' | 'column' | 'key'>('all');
const objSearchLoading = ref(false);
const objSearchError = ref('');
const objSearchHits = ref<ObjectSearchHit[]>([]);

const objSearchProfile = computed(
  () => props.connections.find((c) => c.id === objSearchProfileId.value) ?? null,
);

const filteredSearchHits = computed(() => {
  if (objSearchFilter.value === 'all') return objSearchHits.value;
  if (objSearchFilter.value === 'table') {
    return objSearchHits.value.filter((h) => h.kind === 'table' || h.kind === 'view');
  }
  return objSearchHits.value.filter((h) => h.kind === objSearchFilter.value);
});

async function onObjSearchProfileChange() {
  objSearchHits.value = [];
  objSearchError.value = '';
  try {
    const dbs = await fetchDbsCached(objSearchProfileId.value);
    objSearchDbs.value = dbs;
    if (!dbs.some((d) => d.name === objSearchDbName.value)) {
      objSearchDbName.value = dbs.find((d) => !d.isSystem)?.name ?? dbs[0]?.name ?? '';
    }
  } catch (err) {
    objSearchError.value = String(err);
  }
}

async function runObjectSearch() {
  const q = objSearchQuery.value.trim();
  if (!objSearchProfileId.value || !objSearchDbName.value) {
    objSearchError.value = '请先选择目标数据源和数据库';
    return;
  }
  if (!q) {
    objSearchError.value = '请输入表名、字段名、注释或 Key 关键词';
    return;
  }
  objSearchLoading.value = true;
  objSearchError.value = '';
  try {
    objSearchHits.value = await searchObjects(objSearchProfileId.value, objSearchDbName.value, q);
  } catch (err) {
    objSearchError.value = String(err);
  } finally {
    objSearchLoading.value = false;
  }
}

function buildWhereSnippetFromHit(hit: ObjectSearchHit) {
  const prof = objSearchProfile.value;
  const eng = engineKeyOfProfile(prof);
  const q = (id: string) => quoteIdentifier(id, eng);
  const tbl = q(hit.tableName);
  const sql = hit.columnName
    ? `SELECT * FROM ${tbl} WHERE ${q(hit.columnName)} = '' LIMIT 100;`
    : `SELECT * FROM ${tbl} LIMIT 100;`;
  emit('send-sql-to-editor', sql, objSearchProfileId.value, hit.database || objSearchDbName.value);
}

async function copyTextSafe(text: string, label: string) {
  try {
    await navigator.clipboard.writeText(text);
    emit('notify', `已复制${label}: ${text}`);
  } catch {
    emit('notify', text);
  }
}

// ==================== 2. Visual Table Schema Diff & Sync DDL ====================
const sqlConnections = computed(() =>
  props.connections.filter((c) => !isRedisEngineKey(engineKeyOfProfile(c))),
);

const diffSrcProfileId = ref('');
const diffSrcDb = ref('');
const diffSrcTable = ref('');
const diffSrcDbs = ref<DatabaseMeta[]>([]);
const diffSrcTables = ref<TableMeta[]>([]);
const diffSrcCols = ref<ColumnMeta[]>([]);
const diffSrcIdxs = ref<IndexMeta[]>([]);

const diffTgtProfileId = ref('');
const diffTgtDb = ref('');
const diffTgtTable = ref('');
const diffTgtDbs = ref<DatabaseMeta[]>([]);
const diffTgtTables = ref<TableMeta[]>([]);
const diffTgtCols = ref<ColumnMeta[]>([]);
const diffTgtIdxs = ref<IndexMeta[]>([]);

const diffLoading = ref(false);
const diffError = ref('');

const diffTgtProfile = computed(
  () => sqlConnections.value.find((c) => c.id === diffTgtProfileId.value) ?? null,
);

async function loadDiffSideCatalog(side: 'src' | 'tgt', level: 'profile' | 'db' | 'table') {
  diffError.value = '';
  diffLoading.value = true;
  try {
    const pid = side === 'src' ? diffSrcProfileId.value : diffTgtProfileId.value;
    if (!pid) return;
    if (level === 'profile') {
      const dbs = await fetchDbsCached(pid);
      if (side === 'src') {
        diffSrcDbs.value = dbs;
        if (!dbs.some((d) => d.name === diffSrcDb.value)) {
          diffSrcDb.value = dbs.find((d) => !d.isSystem)?.name ?? dbs[0]?.name ?? '';
        }
      } else {
        diffTgtDbs.value = dbs;
        if (!dbs.some((d) => d.name === diffTgtDb.value)) {
          diffTgtDb.value = dbs.find((d) => !d.isSystem)?.name ?? dbs[0]?.name ?? '';
        }
      }
    }

    const dbName = side === 'src' ? diffSrcDb.value : diffTgtDb.value;
    if (level === 'profile' || level === 'db') {
      if (dbName) {
        const tbls = await fetchTablesCached(pid, dbName);
        if (side === 'src') {
          diffSrcTables.value = tbls;
          if (!tbls.some((t) => t.name === diffSrcTable.value)) {
            diffSrcTable.value = tbls[0]?.name ?? '';
          }
        } else {
          diffTgtTables.value = tbls;
          if (!tbls.some((t) => t.name === diffTgtTable.value)) {
            diffTgtTable.value = tbls.find((t) => t.name === diffSrcTable.value)?.name ?? tbls[0]?.name ?? '';
          }
        }
      }
    }

    const tblName = side === 'src' ? diffSrcTable.value : diffTgtTable.value;
    if (dbName && tblName) {
      const [cols, idxs] = await Promise.all([
        fetchColsCached(pid, dbName, tblName),
        fetchIndexesCached(pid, dbName, tblName),
      ]);
      if (side === 'src') {
        diffSrcCols.value = cols;
        diffSrcIdxs.value = idxs;
      } else {
        diffTgtCols.value = cols;
        diffTgtIdxs.value = idxs;
      }
    } else if (side === 'src') {
      diffSrcCols.value = [];
      diffSrcIdxs.value = [];
    } else {
      diffTgtCols.value = [];
      diffTgtIdxs.value = [];
    }
  } catch (err) {
    diffError.value = String(err);
  } finally {
    diffLoading.value = false;
  }
}

const schemaDiffResult = computed(() => {
  if (!diffSrcTable.value || !diffTgtTable.value) return null;
  return computeTableSchemaDiff({
    sourceDb: diffSrcDb.value,
    sourceTable: diffSrcTable.value,
    sourceColumns: diffSrcCols.value,
    sourceIndexes: diffSrcIdxs.value,
    targetDb: diffTgtDb.value,
    targetTable: diffTgtTable.value,
    targetColumns: diffTgtCols.value,
    targetIndexes: diffTgtIdxs.value,
    targetEngineKey: engineKeyOfProfile(diffTgtProfile.value),
  });
});

function executeSchemaSync() {
  const res = schemaDiffResult.value;
  if (!res || res.syncStatements.length === 0 || !diffTgtProfile.value) return;
  const runnable = res.syncStatements.filter((s) => !s.trim().startsWith('--'));
  if (runnable.length === 0) {
    emit('notify', '没有可直接执行的同步 DDL 语句（部分方言差异仅生成参考注释）');
    return;
  }
  if (isProfileReadOnly(diffTgtProfile.value)) {
    emit('notify', `目标数据源「${diffTgtProfile.value.name}」处于只读模式，已拦截同步变更`);
    return;
  }
  emit(
    'run-statements',
    runnable,
    `结构同步 ${diffTgtTable.value}`,
    diffTgtProfile.value.id,
    diffTgtDb.value,
    true,
  );
}

// ==================== 3. Visual Alter Table Designer ====================
const alterProfile = computed(
  () => props.connections.find((c) => c.id === props.initialProfileId) ?? null,
);
const alterEngineKey = computed(() => engineKeyOfProfile(alterProfile.value));
const alterOrigCols = ref<ColumnMeta[]>([]);
const alterOrigIndexes = ref<IndexMeta[]>([]);
const alterDraftCols = ref<AlterColumnDraft[]>([]);
const alterDraftIndexes = ref<AlterIndexDraft[]>([]);
const alterLoading = ref(false);
const alterError = ref('');

async function loadAlterMetadata() {
  if (!props.initialProfileId || !props.initialDbName || !props.initialTableName) return;
  alterLoading.value = true;
  alterError.value = '';
  try {
    const [cols, idxs] = await Promise.all([
      fetchColsCached(props.initialProfileId, props.initialDbName, props.initialTableName),
      fetchIndexesCached(props.initialProfileId, props.initialDbName, props.initialTableName),
    ]);
    alterOrigCols.value = cols;
    alterOrigIndexes.value = idxs;
    alterDraftCols.value = cols.map((c) => ({
      originalName: c.name,
      name: c.name,
      dataType: c.dataType,
      nullable: c.nullable,
      defaultValue: c.defaultValue ?? '',
      comment: c.comment ?? '',
      isNew: false,
      deleted: false,
    }));
    alterDraftIndexes.value = idxs
      .filter((i) => i.name.toUpperCase() !== 'PRIMARY' && i.indexType?.toUpperCase() !== 'PRIMARY')
      .map((i) => ({
        originalName: i.name,
        name: i.name,
        unique: i.unique,
        columns: i.columns.join(', '),
        isNew: false,
        deleted: false,
      }));
  } catch (err) {
    alterError.value = String(err);
  } finally {
    alterLoading.value = false;
  }
}

function addAlterColumn() {
  alterDraftCols.value.push({
    name: `col_${alterDraftCols.value.length + 1}`,
    dataType: 'VARCHAR(128)',
    nullable: true,
    defaultValue: '',
    comment: '',
    isNew: true,
    deleted: false,
  });
}

function addAlterIndex() {
  alterDraftIndexes.value.push({
    name: `idx_${props.initialTableName}_${alterDraftIndexes.value.length + 1}`,
    unique: false,
    columns: alterDraftCols.value.find((c) => !c.deleted)?.name ?? 'id',
    isNew: true,
    deleted: false,
  });
}

const alterDdlResult = computed(() =>
  buildAlterTableDdl({
    dbName: props.initialDbName,
    tableName: props.initialTableName,
    originalColumns: alterOrigCols.value,
    draftColumns: alterDraftCols.value,
    originalIndexes: alterOrigIndexes.value,
    draftIndexes: alterDraftIndexes.value,
    engineKey: alterEngineKey.value,
  }),
);

function executeAlterTable() {
  const stmts = alterDdlResult.value.statements;
  if (stmts.length === 0) {
    emit('notify', '当前未检测到表结构变更');
    return;
  }
  if (isProfileReadOnly(alterProfile.value)) {
    emit('notify', `数据源「${alterProfile.value?.name}」为只读连接，已禁止 ALTER TABLE`);
    return;
  }
  emit(
    'run-statements',
    stmts,
    `修改表结构 ${props.initialTableName}`,
    props.initialProfileId,
    props.initialDbName,
    true,
  );
}

// ==================== 4. Redis Visual Key-Value & TTL Editor ====================
const redisDraftKey = ref('');
const redisDraftOriginalKey = ref('');
const redisDraftType = ref<RedisEditableType>('string');
const redisDraftIsNew = ref(true);
const redisDraftTtl = ref<string>('-1');
const redisStringVal = ref('');
const redisHashRows = ref<Array<{ field: string; value: string }>>([]);
const redisListItems = ref<string[]>([]);
const redisSetMembers = ref<string[]>([]);
const redisZsetRows = ref<Array<{ member: string; score: string }>>([]);
const redisJsonStatus = ref('');

const generatedRedisCommands = computed(() => {
  const ttlRaw = redisDraftTtl.value.trim();
  const ttlNum = ttlRaw === '' ? null : Number(ttlRaw);
  return buildRedisSaveCommands({
    originalKey: redisDraftOriginalKey.value || undefined,
    key: redisDraftKey.value,
    keyType: redisDraftType.value,
    stringValue: redisStringVal.value,
    hashEntries: redisHashRows.value,
    listItems: redisListItems.value,
    setMembers: redisSetMembers.value,
    zsetMembers: redisZsetRows.value,
    ttlSeconds: ttlNum !== null && Number.isFinite(ttlNum) ? ttlNum : null,
    isNew: redisDraftIsNew.value,
  });
});

function formatRedisStringJson(pretty: boolean) {
  try {
    const parsed = JSON.parse(redisStringVal.value);
    redisStringVal.value = pretty ? JSON.stringify(parsed, null, 2) : JSON.stringify(parsed);
    redisJsonStatus.value = 'JSON 校验通过';
  } catch (err) {
    redisJsonStatus.value = `JSON 解析失败: ${String(err)}`;
  }
}

function executeRedisSave() {
  const cmds = generatedRedisCommands.value;
  if (cmds.length === 0) {
    emit('notify', '请填写有效的 Redis Key 名称与内容');
    return;
  }
  const prof = props.connections.find((c) => c.id === props.initialProfileId);
  if (isProfileReadOnly(prof)) {
    emit('notify', `Redis 数据源「${prof?.name}」处于只读模式，无法写入 Key`);
    return;
  }
  emit(
    'run-statements',
    cmds,
    `${redisDraftIsNew.value ? '新建' : '保存'} Redis Key ${redisDraftKey.value}`,
    props.initialProfileId,
    props.initialDbName || 'db0',
    true,
  );
}

// ==================== Modal Lifecycle Initializer ====================
watch(
  () => [props.modalKind, props.initialProfileId, props.initialDbName, props.initialTableName, props.redisInitData],
  async ([kind]) => {
    if (kind === 'object-search') {
      objSearchProfileId.value = props.initialProfileId || props.connections[0]?.id || '';
      objSearchDbName.value = props.initialDbName || '';
      objSearchFilter.value = 'all';
      objSearchHits.value = [];
      objSearchError.value = '';
      await onObjSearchProfileChange();
    } else if (kind === 'schema-diff') {
      const defSrc = props.initialProfileId || sqlConnections.value[0]?.id || '';
      diffSrcProfileId.value = defSrc;
      diffSrcDb.value = props.initialDbName || '';
      diffSrcTable.value = props.initialTableName || '';
      diffTgtProfileId.value =
        sqlConnections.value.find((c) => c.id !== defSrc)?.id ?? defSrc;
      diffTgtDb.value = '';
      diffTgtTable.value = '';
      await loadDiffSideCatalog('src', 'profile');
      await loadDiffSideCatalog('tgt', 'profile');
    } else if (kind === 'alter-table') {
      await loadAlterMetadata();
    } else if (kind === 'redis-editor') {
      const init = props.redisInitData;
      redisJsonStatus.value = '';
      if (init) {
        redisDraftIsNew.value = init.isNew;
        redisDraftKey.value = init.key;
        redisDraftOriginalKey.value = init.key;
        redisDraftType.value = init.keyType;
        redisDraftTtl.value = init.ttlSeconds !== null ? String(init.ttlSeconds) : '-1';
        redisStringVal.value = init.stringValue ?? '';
        redisHashRows.value = init.hashEntries?.length
          ? init.hashEntries.map((x) => ({ ...x }))
          : [{ field: 'field_1', value: '' }];
        redisListItems.value = init.listItems?.length ? [...init.listItems] : ['item_1'];
        redisSetMembers.value = init.setMembers?.length ? [...init.setMembers] : ['member_1'];
        redisZsetRows.value = init.zsetMembers?.length
          ? init.zsetMembers.map((x) => ({ ...x }))
          : [{ member: 'member_1', score: '100' }];
      } else {
        redisDraftIsNew.value = true;
        redisDraftKey.value = 'app:demo:key';
        redisDraftOriginalKey.value = '';
        redisDraftType.value = 'string';
        redisDraftTtl.value = '-1';
        redisStringVal.value = '{"hello":"pacer"}';
        redisHashRows.value = [{ field: 'name', value: 'pacer' }];
        redisListItems.value = ['item_1'];
        redisSetMembers.value = ['tag_1'];
        redisZsetRows.value = [{ member: 'player_1', score: '100' }];
      }
    }
  },
);
</script>

<template>
  <!-- 1. Global Object & Column Search Modal -->
  <div
    v-if="modalKind === 'object-search'"
    class="modal-backdrop"
    @click.self="emit('close')"
  >
    <div class="modal-card" style="width: min(820px, 94vw)">
      <div class="modal-header">
        <span>🔍 在数据库中查找对象与字段 / Redis Key</span>
        <button class="icon-btn compact" @click="emit('close')">×</button>
      </div>

      <div class="search-controls-row">
        <select
          v-model="objSearchProfileId"
          class="field"
          style="width: 190px"
          @change="onObjSearchProfileChange"
        >
          <option v-for="c in connections" :key="c.id" :value="c.id">
            {{ c.name }} ({{ getEngineDisplayName(engineKeyOfProfile(c)) }})
          </option>
        </select>

        <select v-model="objSearchDbName" class="field" style="width: 155px">
          <option v-for="d in objSearchDbs" :key="d.name" :value="d.name">
            {{ d.name }}
          </option>
        </select>

        <input
          v-model="objSearchQuery"
          class="field"
          style="flex: 1"
          placeholder="输入表名、列名、注释或 Redis Key 关键词（回车检索）..."
          @keydown.enter.prevent="runObjectSearch"
        />

        <button class="btn btn-primary btn-sm" :disabled="objSearchLoading" @click="runObjectSearch">
          <IdeIcon name="search" :size="12" /> {{ objSearchLoading ? '检索中...' : '查找' }}
        </button>
      </div>

      <div style="display: flex; align-items: center; justify-content: space-between; margin: 8px 0">
        <div style="display: flex; gap: 6px">
          <button
            class="btn btn-sm"
            :class="{ 'btn-primary': objSearchFilter === 'all' }"
            @click="objSearchFilter = 'all'"
          >
            全部 ({{ objSearchHits.length }})
          </button>
          <button
            class="btn btn-sm"
            :class="{ 'btn-primary': objSearchFilter === 'table' }"
            @click="objSearchFilter = 'table'"
          >
            表 / 视图
          </button>
          <button
            class="btn btn-sm"
            :class="{ 'btn-primary': objSearchFilter === 'column' }"
            @click="objSearchFilter = 'column'"
          >
            字段 / 列
          </button>
          <button
            class="btn btn-sm"
            :class="{ 'btn-primary': objSearchFilter === 'key' }"
            @click="objSearchFilter = 'key'"
          >
            Redis Key
          </button>
        </div>
        <span style="font-size: 11px; color: var(--text-secondary)">
          最多返回 150 条匹配结果
        </span>
      </div>

      <div v-if="objSearchError" class="status-banner-error" style="margin-bottom: 8px">
        {{ objSearchError }}
      </div>

      <div class="designer-scroll" style="max-height: 360px">
        <div
          v-if="filteredSearchHits.length === 0"
          style="padding: 28px; text-align: center; color: var(--text-secondary); font-size: 12px"
        >
          {{ objSearchLoading ? '正在扫描元数据...' : '输入关键词并点击「查找」，支持按表名、字段名、注释与 Redis Key 模式搜索。' }}
        </div>
        <table v-else class="designer-table">
          <thead>
            <tr>
              <th style="width: 74px">类型</th>
              <th>表 / Key</th>
              <th>字段名</th>
              <th>数据类型</th>
              <th>注释 / TTL</th>
              <th style="width: 210px; text-align: right">快捷操作</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(hit, idx) in filteredSearchHits" :key="`${hit.kind}-${hit.tableName}-${hit.columnName ?? idx}`">
              <td>
                <span
                  class="pill"
                  :class="
                    hit.kind === 'column'
                      ? 'pill-dev'
                      : hit.kind === 'key'
                        ? 'pill-test'
                        : 'pill-neutral'
                  "
                >
                  {{ hit.kind.toUpperCase() }}
                </span>
              </td>
              <td style="font-family: var(--font-mono); font-weight: 600">{{ hit.tableName }}</td>
              <td style="font-family: var(--font-mono)">{{ hit.columnName || '-' }}</td>
              <td style="font-family: var(--font-mono); color: var(--text-secondary)">
                {{ hit.dataType || '-' }}
              </td>
              <td style="color: var(--text-secondary); max-width: 170px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap">
                {{ hit.comment || '-' }}
              </td>
              <td style="text-align: right; white-space: nowrap">
                <button
                  class="cell-inline-btn"
                  @click="
                    emit('jump-query', objSearchProfileId, hit.database || objSearchDbName, hit.tableName);
                    emit('close');
                  "
                >
                  查前100
                </button>
                <button
                  class="cell-inline-btn"
                  @click="
                    emit('jump-structure', objSearchProfileId, hit.database || objSearchDbName, hit.tableName);
                    emit('close');
                  "
                >
                  结构
                </button>
                <button
                  v-if="hit.kind !== 'key'"
                  class="cell-inline-btn"
                  @click="buildWhereSnippetFromHit(hit)"
                >
                  SQL
                </button>
                <button
                  class="cell-inline-btn"
                  @click="
                    copyTextSafe(
                      hit.columnName ? `${hit.tableName}.${hit.columnName}` : hit.tableName,
                      '名称',
                    )
                  "
                >
                  复制
                </button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </div>

  <!-- 2. Visual Table Schema Diff & Sync DDL Modal -->
  <div
    v-else-if="modalKind === 'schema-diff'"
    class="modal-backdrop"
    @click.self="emit('close')"
  >
    <div class="modal-card" style="width: min(920px, 95vw)">
      <div class="modal-header">
        <span>⚖️ 表结构对比与同步 DDL（支持跨数据源 / 跨环境对比）</span>
        <button class="icon-btn compact" @click="emit('close')">×</button>
      </div>

      <div class="form-grid" style="margin-bottom: 10px">
        <div class="diff-side-box">
          <div style="font-size: 11.5px; font-weight: 700; margin-bottom: 6px; color: var(--dev)">
            基准源表（Source · 期望结构）
          </div>
          <div style="display: flex; gap: 6px; flex-wrap: wrap">
            <select v-model="diffSrcProfileId" class="field" style="flex: 1" @change="loadDiffSideCatalog('src', 'profile')">
              <option v-for="c in sqlConnections" :key="c.id" :value="c.id">{{ c.name }}</option>
            </select>
            <select v-model="diffSrcDb" class="field" style="width: 130px" @change="loadDiffSideCatalog('src', 'db')">
              <option v-for="d in diffSrcDbs" :key="d.name" :value="d.name">{{ d.name }}</option>
            </select>
            <select v-model="diffSrcTable" class="field" style="width: 150px" @change="loadDiffSideCatalog('src', 'table')">
              <option v-for="tb in diffSrcTables" :key="tb.name" :value="tb.name">{{ tb.name }}</option>
            </select>
          </div>
        </div>

        <div class="diff-side-box">
          <div style="font-size: 11.5px; font-weight: 700; margin-bottom: 6px; color: var(--test)">
            同步目标表（Target · 待迁移修改）
          </div>
          <div style="display: flex; gap: 6px; flex-wrap: wrap">
            <select v-model="diffTgtProfileId" class="field" style="flex: 1" @change="loadDiffSideCatalog('tgt', 'profile')">
              <option v-for="c in sqlConnections" :key="c.id" :value="c.id">{{ c.name }}</option>
            </select>
            <select v-model="diffTgtDb" class="field" style="width: 130px" @change="loadDiffSideCatalog('tgt', 'db')">
              <option v-for="d in diffTgtDbs" :key="d.name" :value="d.name">{{ d.name }}</option>
            </select>
            <select v-model="diffTgtTable" class="field" style="width: 150px" @change="loadDiffSideCatalog('tgt', 'table')">
              <option v-for="tb in diffTgtTables" :key="tb.name" :value="tb.name">{{ tb.name }}</option>
            </select>
          </div>
        </div>
      </div>

      <div v-if="diffError" class="status-banner-error" style="margin-bottom: 8px">
        {{ diffError }}
      </div>

      <template v-if="schemaDiffResult">
        <div style="display: flex; align-items: center; gap: 6px; flex-wrap: wrap; margin-bottom: 8px">
          <span class="pill pill-dev">新增字段 +{{ schemaDiffResult.stats.added }}</span>
          <span class="pill pill-test">修改字段 ~{{ schemaDiffResult.stats.modified }}</span>
          <span class="pill pill-prod">多余/待删 -{{ schemaDiffResult.stats.removed }}</span>
          <span class="pill pill-neutral">索引变更 {{ schemaDiffResult.stats.indexChanges }}</span>
          <span class="pill pill-neutral">一致字段 {{ schemaDiffResult.stats.same }}</span>
          <span v-if="diffLoading" style="font-size: 11px; color: var(--text-secondary)">正在刷新结构...</span>
        </div>

        <div
          v-if="schemaDiffResult.warnings.length > 0"
          class="diff-warning-box"
        >
          <div v-for="(w, wi) in schemaDiffResult.warnings" :key="wi">{{ w }}</div>
        </div>

        <div class="designer-scroll" style="max-height: 220px">
          <table class="designer-table">
            <thead>
              <tr>
                <th style="width: 78px">状态</th>
                <th>对象名称</th>
                <th>源定义</th>
                <th>目标定义</th>
                <th>差异说明</th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="cd in schemaDiffResult.columnDiffs"
                :key="`col-${cd.name}`"
                :class="`diff-row-${cd.status}`"
              >
                <td>
                  <span class="pill" :class="cd.status === 'added' ? 'pill-dev' : cd.status === 'removed' ? 'pill-prod' : cd.status === 'modified' ? 'pill-test' : 'pill-neutral'">
                    {{ cd.status === 'added' ? '+ 新增' : cd.status === 'removed' ? '- 删除' : cd.status === 'modified' ? '~ 修改' : '= 一致' }}
                  </span>
                </td>
                <td style="font-family: var(--font-mono); font-weight: 600">{{ cd.name }}</td>
                <td style="font-family: var(--font-mono); font-size: 11px">
                  {{ cd.source ? `${cd.source.dataType} ${cd.source.nullable ? 'NULL' : 'NOT NULL'}` : '-' }}
                </td>
                <td style="font-family: var(--font-mono); font-size: 11px">
                  {{ cd.target ? `${cd.target.dataType} ${cd.target.nullable ? 'NULL' : 'NOT NULL'}` : '-' }}
                </td>
                <td style="font-size: 11.5px">{{ cd.changes.join('；') || '完全一致' }}</td>
              </tr>
              <tr
                v-for="ix in schemaDiffResult.indexDiffs"
                :key="`idx-${ix.name}`"
                :class="`diff-row-${ix.status}`"
              >
                <td>
                  <span class="pill" :class="ix.status === 'added' ? 'pill-dev' : ix.status === 'removed' ? 'pill-prod' : ix.status === 'modified' ? 'pill-test' : 'pill-neutral'">
                    索引
                  </span>
                </td>
                <td style="font-family: var(--font-mono); font-weight: 600">{{ ix.name }}</td>
                <td style="font-family: var(--font-mono); font-size: 11px">
                  {{ ix.source ? `(${ix.source.columns.join(', ')})` : '-' }}
                </td>
                <td style="font-family: var(--font-mono); font-size: 11px">
                  {{ ix.target ? `(${ix.target.columns.join(', ')})` : '-' }}
                </td>
                <td style="font-size: 11.5px">{{ ix.changes.join('；') || '完全一致' }}</td>
              </tr>
            </tbody>
          </table>
        </div>

        <div style="font-size: 11.5px; font-weight: 600; margin-bottom: 4px">
          生成的目标同步 DDL（{{ getEngineDisplayName(engineKeyOfProfile(diffTgtProfile)) }} 方言）
        </div>
        <pre class="code-viewer" style="max-height: 150px">{{
          schemaDiffResult.syncStatements.length > 0
            ? schemaDiffResult.syncStatements.join('\n')
            : '-- 源表与目标表结构完全一致，无需同步变更。'
        }}</pre>

        <div style="display: flex; justify-content: flex-end; gap: 8px; margin-top: 12px">
          <button class="btn" @click="emit('close')">关闭</button>
          <button
            class="btn"
            :disabled="schemaDiffResult.syncStatements.length === 0"
            @click="copyTextSafe(schemaDiffResult.syncStatements.join('\n'), '同步 DDL')"
          >
            <IdeIcon name="copy" :size="12" /> 复制 DDL
          </button>
          <button
            class="btn"
            :disabled="schemaDiffResult.syncStatements.length === 0"
            @click="
              emit(
                'send-sql-to-editor',
                schemaDiffResult.syncStatements.join('\n'),
                diffTgtProfileId,
                diffTgtDb,
              )
            "
          >
            发送到编辑器
          </button>
          <button
            class="btn btn-primary"
            :disabled="busy || schemaDiffResult.syncStatements.length === 0"
            @click="executeSchemaSync"
          >
            <IdeIcon name="run" :size="12" /> 执行同步 DDL
          </button>
        </div>
      </template>
    </div>
  </div>

  <!-- 3. Visual Alter Table Designer Modal -->
  <div
    v-else-if="modalKind === 'alter-table'"
    class="modal-backdrop"
    @click.self="emit('close')"
  >
    <div class="modal-card" style="width: min(920px, 95vw)">
      <div class="modal-header">
        <span>
          🛠️ 可视化修改表结构 ·
          <span style="font-family: var(--font-mono)">{{ initialDbName }}.{{ initialTableName }}</span>
        </span>
        <button class="icon-btn compact" @click="emit('close')">×</button>
      </div>

      <div v-if="alterError" class="status-banner-error" style="margin-bottom: 8px">
        {{ alterError }}
      </div>

      <div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 6px">
        <span style="font-size: 12px; font-weight: 600">
          字段列表 ({{ alterDraftCols.filter((c) => !c.deleted).length }})
          <span v-if="alterLoading" style="color: var(--text-secondary); font-weight: 400"> · 加载中...</span>
        </span>
        <button class="btn btn-sm" @click="addAlterColumn">
          <IdeIcon name="plus" :size="11" /> 新增字段
        </button>
      </div>

      <div class="designer-scroll" style="max-height: 230px">
        <table class="designer-table">
          <thead>
            <tr>
              <th>字段名</th>
              <th>数据类型</th>
              <th style="width: 54px; text-align: center">可空</th>
              <th>默认值</th>
              <th>注释</th>
              <th style="width: 68px; text-align: center">操作</th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="(col, idx) in alterDraftCols"
              :key="idx"
              :class="{ 'diff-row-removed': col.deleted, 'diff-row-added': col.isNew && !col.deleted }"
            >
              <td>
                <input
                  v-model="col.name"
                  class="field"
                  :disabled="col.deleted"
                  style="font-family: var(--font-mono); padding: 2px 5px"
                />
              </td>
              <td>
                <input
                  v-model="col.dataType"
                  list="pacer-alter-col-types"
                  class="field"
                  :disabled="col.deleted"
                  style="font-family: var(--font-mono); padding: 2px 5px"
                />
              </td>
              <td style="text-align: center">
                <input v-model="col.nullable" type="checkbox" :disabled="col.deleted" />
              </td>
              <td>
                <input v-model="col.defaultValue" class="field" :disabled="col.deleted" style="padding: 2px 5px" />
              </td>
              <td>
                <input v-model="col.comment" class="field" :disabled="col.deleted" style="padding: 2px 5px" />
              </td>
              <td style="text-align: center">
                <button
                  v-if="col.isNew"
                  class="cell-inline-btn"
                  @click="alterDraftCols.splice(idx, 1)"
                >
                  移除
                </button>
                <button
                  v-else
                  class="cell-inline-btn"
                  @click="col.deleted = !col.deleted"
                >
                  {{ col.deleted ? '恢复' : '删除' }}
                </button>
              </td>
            </tr>
          </tbody>
        </table>
        <datalist id="pacer-alter-col-types">
          <option v-for="tOpt in COMMON_ALTER_TYPES" :key="tOpt" :value="tOpt" />
        </datalist>
      </div>

      <div style="display: flex; align-items: center; justify-content: space-between; margin: 6px 0">
        <span style="font-size: 12px; font-weight: 600">普通 / 唯一索引</span>
        <button class="btn btn-sm" @click="addAlterIndex">
          <IdeIcon name="plus" :size="11" /> 新增索引
        </button>
      </div>

      <div v-if="alterDraftIndexes.length > 0" class="designer-scroll" style="max-height: 110px">
        <table class="designer-table">
          <tbody>
            <tr
              v-for="(ix, i) in alterDraftIndexes"
              :key="i"
              :class="{ 'diff-row-removed': ix.deleted, 'diff-row-added': ix.isNew && !ix.deleted }"
            >
              <td>
                <input v-model="ix.name" class="field" :disabled="!ix.isNew || ix.deleted" style="font-family: var(--font-mono); padding: 2px 5px" />
              </td>
              <td style="width: 72px">
                <label style="display: inline-flex; align-items: center; gap: 4px; font-size: 11px">
                  <input v-model="ix.unique" type="checkbox" :disabled="!ix.isNew || ix.deleted" /> 唯一
                </label>
              </td>
              <td>
                <input v-model="ix.columns" class="field" :disabled="!ix.isNew || ix.deleted" style="font-family: var(--font-mono); padding: 2px 5px" placeholder="逗号分隔列名" />
              </td>
              <td style="width: 68px; text-align: center">
                <button v-if="ix.isNew" class="cell-inline-btn" @click="alterDraftIndexes.splice(i, 1)">移除</button>
                <button v-else class="cell-inline-btn" @click="ix.deleted = !ix.deleted">{{ ix.deleted ? '恢复' : '删除' }}</button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <div v-if="alterDdlResult.warnings.length > 0" class="diff-warning-box">
        <div v-for="(w, wi) in alterDdlResult.warnings" :key="wi">{{ w }}</div>
      </div>

      <div style="font-size: 11.5px; font-weight: 600; margin-bottom: 4px">
        ALTER DDL 预览（{{ getEngineDisplayName(alterEngineKey) }} 方言）
      </div>
      <pre class="code-viewer" style="max-height: 140px">{{
        alterDdlResult.statements.length > 0
          ? alterDdlResult.statements.join('\n')
          : '-- 暂无表结构改动，请调整上方字段或索引后查看预览。'
      }}</pre>

      <div style="display: flex; justify-content: flex-end; gap: 8px; margin-top: 12px">
        <button class="btn" @click="emit('close')">取消</button>
        <button
          class="btn"
          :disabled="alterDdlResult.statements.length === 0"
          @click="copyTextSafe(alterDdlResult.statements.join('\n'), 'ALTER DDL')"
        >
          <IdeIcon name="copy" :size="12" /> 复制 SQL
        </button>
        <button
          class="btn"
          :disabled="alterDdlResult.statements.length === 0"
          @click="
            emit('send-sql-to-editor', alterDdlResult.statements.join('\n'), initialProfileId, initialDbName)
          "
        >
          发送到编辑器
        </button>
        <button
          class="btn btn-primary"
          :disabled="busy || alterDdlResult.statements.length === 0"
          @click="executeAlterTable"
        >
          <IdeIcon name="run" :size="12" /> 执行结构变更
        </button>
      </div>
    </div>
  </div>

  <!-- 4. Redis Visual Key-Value & TTL Editor Modal -->
  <div
    v-else-if="modalKind === 'redis-editor'"
    class="modal-backdrop"
    @click.self="emit('close')"
  >
    <div class="modal-card" style="width: min(720px, 94vw)">
      <div class="modal-header">
        <span>
          {{ redisDraftIsNew ? '➕ 可视化新建 Redis Key' : '✏️ 可视化编辑 Redis Key 与 TTL' }}
          · <span style="font-family: var(--font-mono)">{{ initialDbName || 'db0' }}</span>
        </span>
        <button class="icon-btn compact" @click="emit('close')">×</button>
      </div>

      <div class="form-grid" style="margin-bottom: 10px">
        <div class="form-field">
          <label>Key 名称</label>
          <input v-model="redisDraftKey" class="field" style="font-family: var(--font-mono)" />
        </div>
        <div class="form-field">
          <label>数据类型</label>
          <select v-model="redisDraftType" class="field" :disabled="!redisDraftIsNew">
            <option value="string">string (字符串 / JSON)</option>
            <option value="hash">hash (哈希表)</option>
            <option value="list">list (列表)</option>
            <option value="set">set (集合)</option>
            <option value="zset">zset (有序集合)</option>
          </select>
        </div>
      </div>

      <div style="display: flex; align-items: center; gap: 6px; margin-bottom: 10px; flex-wrap: wrap">
        <span style="font-size: 11.5px; color: var(--text-secondary)">TTL (秒，-1 表示永不过期):</span>
        <input v-model="redisDraftTtl" class="field" style="width: 110px; font-family: var(--font-mono)" />
        <button class="btn btn-sm" @click="redisDraftTtl = '-1'">永久 (-1)</button>
        <button class="btn btn-sm" @click="redisDraftTtl = '300'">5 分钟</button>
        <button class="btn btn-sm" @click="redisDraftTtl = '3600'">1 小时</button>
        <button class="btn btn-sm" @click="redisDraftTtl = '86400'">1 天</button>
      </div>

      <!-- String Body -->
      <div v-if="redisDraftType === 'string'" style="margin-bottom: 10px">
        <div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 4px">
          <span style="font-size: 11.5px; font-weight: 600">String 值</span>
          <div style="display: flex; align-items: center; gap: 6px">
            <span v-if="redisJsonStatus" style="font-size: 11px; color: var(--text-secondary)">
              {{ redisJsonStatus }}
            </span>
            <button class="btn btn-sm" @click="formatRedisStringJson(true)">格式化 JSON</button>
            <button class="btn btn-sm" @click="formatRedisStringJson(false)">压缩 JSON</button>
          </div>
        </div>
        <textarea
          v-model="redisStringVal"
          class="field"
          rows="7"
          style="width: 100%; font-family: var(--font-mono); resize: vertical"
        />
      </div>

      <!-- Hash Body -->
      <div v-else-if="redisDraftType === 'hash'" style="margin-bottom: 10px">
        <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 4px">
          <span style="font-size: 11.5px; font-weight: 600">Hash 字段 ({{ redisHashRows.length }})</span>
          <button class="btn btn-sm" @click="redisHashRows.push({ field: '', value: '' })">+ 新增字段</button>
        </div>
        <div class="designer-scroll" style="max-height: 200px">
          <table class="designer-table">
            <thead>
              <tr>
                <th>Field</th>
                <th>Value</th>
                <th style="width: 50px"></th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="(item, idx) in redisHashRows" :key="idx">
                <td><input v-model="item.field" class="field" style="font-family: var(--font-mono); padding: 2px 5px" /></td>
                <td><input v-model="item.value" class="field" style="font-family: var(--font-mono); padding: 2px 5px" /></td>
                <td style="text-align: center">
                  <button class="cell-inline-btn" @click="redisHashRows.splice(idx, 1)">删除</button>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <!-- List / Set Body -->
      <div v-else-if="redisDraftType === 'list' || redisDraftType === 'set'" style="margin-bottom: 10px">
        <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 4px">
          <span style="font-size: 11.5px; font-weight: 600">
            {{ redisDraftType === 'list' ? 'List 列表元素' : 'Set 集合成员' }}
          </span>
          <button
            class="btn btn-sm"
            @click="redisDraftType === 'list' ? redisListItems.push('') : redisSetMembers.push('')"
          >
            + 新增元素
          </button>
        </div>
        <div class="designer-scroll" style="max-height: 200px">
          <table class="designer-table">
            <tbody>
              <tr
                v-for="(_, idx) in redisDraftType === 'list' ? redisListItems : redisSetMembers"
                :key="idx"
              >
                <td style="width: 44px; color: var(--text-secondary)">#{{ idx }}</td>
                <td>
                  <input
                    v-if="redisDraftType === 'list'"
                    v-model="redisListItems[idx]"
                    class="field"
                    style="font-family: var(--font-mono); padding: 2px 5px"
                  />
                  <input
                    v-else
                    v-model="redisSetMembers[idx]"
                    class="field"
                    style="font-family: var(--font-mono); padding: 2px 5px"
                  />
                </td>
                <td style="width: 56px; text-align: center">
                  <button
                    class="cell-inline-btn"
                    @click="
                      redisDraftType === 'list'
                        ? redisListItems.splice(idx, 1)
                        : redisSetMembers.splice(idx, 1)
                    "
                  >
                    删除
                  </button>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <!-- ZSet Body -->
      <div v-else style="margin-bottom: 10px">
        <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 4px">
          <span style="font-size: 11.5px; font-weight: 600">ZSet 成员与分值 (Score)</span>
          <button class="btn btn-sm" @click="redisZsetRows.push({ member: '', score: '0' })">+ 新增成员</button>
        </div>
        <div class="designer-scroll" style="max-height: 200px">
          <table class="designer-table">
            <thead>
              <tr>
                <th>Member</th>
                <th style="width: 130px">Score</th>
                <th style="width: 50px"></th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="(item, idx) in redisZsetRows" :key="idx">
                <td><input v-model="item.member" class="field" style="font-family: var(--font-mono); padding: 2px 5px" /></td>
                <td><input v-model="item.score" class="field" style="font-family: var(--font-mono); padding: 2px 5px" /></td>
                <td style="text-align: center">
                  <button class="cell-inline-btn" @click="redisZsetRows.splice(idx, 1)">删除</button>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <div style="font-size: 11.5px; font-weight: 600; margin-bottom: 4px">将执行的 Redis 命令</div>
      <pre class="code-viewer" style="max-height: 110px">{{ generatedRedisCommands.join('\n') || '-- 请输入有效 Key 与内容' }}</pre>

      <div style="display: flex; justify-content: flex-end; gap: 8px; margin-top: 12px">
        <button class="btn" @click="emit('close')">取消</button>
        <button
          class="btn"
          :disabled="generatedRedisCommands.length === 0"
          @click="
            emit('send-sql-to-editor', generatedRedisCommands.join('\n'), initialProfileId, initialDbName)
          "
        >
          写入控制台
        </button>
        <button
          class="btn btn-primary"
          :disabled="busy || generatedRedisCommands.length === 0"
          @click="executeRedisSave"
        >
          <IdeIcon name="save" :size="12" /> 保存到 Redis
        </button>
      </div>
    </div>
  </div>
</template>
