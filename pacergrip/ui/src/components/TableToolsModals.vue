<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue';
import {
  openTextFile,
  pickDataImportFile,
  readXlsxSheet,
  type ColumnMeta,
  type EnvLevel,
  type XlsxSheetPreview,
} from '../api/ipc';
import IdeIcon from './IdeIcon.vue';
import {
  buildBatchInsertStatements,
  buildRenameTableSql,
  downloadTextFile,
  generateCreateTableDdl,
  generateEntityCode,
  getEngineDisplayName,
  parseImportData,
  type CreateTableColumnDraft,
  type CreateTableIndexDraft,
  type EntityCodeLang,
} from '../utils/sqlTools';

export type ActiveToolModalKind =
  | 'none'
  | 'import-data'
  | 'codegen'
  | 'create-table'
  | 'rename-table';

export interface TableToolContext {
  profileId: string;
  profileName: string;
  profileEnv: EnvLevel;
  engineKey: string;
  dbName: string;
  tableName: string;
  columns: ColumnMeta[];
}

export interface DbToolContext {
  profileId: string;
  profileName: string;
  profileEnv: EnvLevel;
  engineKey: string;
  dbName: string;
}

const props = defineProps<{
  activeToolModal: ActiveToolModalKind;
  tableContext: TableToolContext | null;
  dbContext: DbToolContext | null;
  busy: boolean;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
  (
    e: 'run-statements',
    statements: string[],
    label: string,
    profileId: string,
    dbName: string,
    refreshDb?: boolean,
  ): void;
  (e: 'send-sql-to-editor', sql: string, profileId?: string, dbName?: string): void;
  (e: 'notify', message: string): void;
}>();

// ==================== 1. Excel / CSV / TSV / JSON Data Import ====================
const importRawText = ref('');
const importSourceFile = ref('');
const importXlsxPreview = ref<XlsxSheetPreview | null>(null);
const importFormat = ref<'auto' | 'csv' | 'json'>('auto');
const importDelimiter = ref<'auto' | ',' | '\t' | ';'>('auto');
const importHasHeader = ref(true);
const importBatchSize = ref(100);
const importMode = ref<'insert' | 'ignore' | 'upsert'>('ignore');
const importConflictKeys = ref<string[]>([]);
const columnMappings = reactive<Record<string, string>>({});

function toggleConflictKey(colName: string) {
  if (importConflictKeys.value.includes(colName)) {
    importConflictKeys.value = importConflictKeys.value.filter((c) => c !== colName);
  } else {
    importConflictKeys.value = [...importConflictKeys.value, colName];
  }
}

const parsedImport = computed(() => {
  if (importXlsxPreview.value) {
    const pv = importXlsxPreview.value;
    if (importHasHeader.value) {
      const headers = pv.headers;
      const rows = pv.rows.map((cells) => {
        const obj: Record<string, unknown> = {};
        for (let i = 0; i < headers.length; i++) {
          obj[headers[i]] = cells[i] ?? '';
        }
        return obj;
      });
      return {
        detectedFormat: 'xlsx',
        headers,
        rows,
        delimiter: '',
        error: null as string | null,
      };
    }
    const allRows = pv.headers.length > 0 ? [pv.headers, ...pv.rows] : pv.rows;
    const width = Math.max(0, ...allRows.map((r) => r.length));
    const headers = Array.from({ length: width }, (_, i) => `col_${i + 1}`);
    const rows = allRows.map((cells) => {
      const obj: Record<string, unknown> = {};
      for (let i = 0; i < headers.length; i++) {
        obj[headers[i]] = cells[i] ?? '';
      }
      return obj;
    });
    return {
      detectedFormat: 'xlsx',
      headers,
      rows,
      delimiter: '',
      error: null as string | null,
    };
  }
  try {
    return {
      ...parseImportData(importRawText.value, importFormat.value, {
        delimiter: importDelimiter.value === 'auto' ? undefined : importDelimiter.value,
        hasHeader: importHasHeader.value,
      }),
      error: null as string | null,
    };
  } catch (err) {
    return {
      detectedFormat: 'csv',
      headers: [] as string[],
      rows: [] as Array<Record<string, unknown>>,
      delimiter: ',',
      error: String(err),
    };
  }
});

watch(
  () => [props.activeToolModal, props.tableContext, parsedImport.value.headers.join('|')],
  () => {
    if (props.activeToolModal !== 'import-data' || !props.tableContext) return;
    const tableCols = props.tableContext.columns;
    const normalize = (s: string) => s.toLowerCase().replace(/[^a-z0-9]/g, '');
    for (const h of parsedImport.value.headers) {
      if (columnMappings[h] !== undefined && (columnMappings[h] === '' || tableCols.some((c) => c.name === columnMappings[h]))) {
        continue;
      }
      const exact = tableCols.find((c) => c.name.toLowerCase() === h.toLowerCase());
      if (exact) {
        columnMappings[h] = exact.name;
        continue;
      }
      const fuzzy = tableCols.find((c) => normalize(c.name) === normalize(h));
      columnMappings[h] = fuzzy ? fuzzy.name : '';
    }
  },
);

watch(
  () => [props.activeToolModal, props.tableContext?.tableName],
  ([kind]) => {
    if (kind === 'import-data') {
      importSourceFile.value = '';
      importRawText.value = '';
      importXlsxPreview.value = null;
      importConflictKeys.value =
        props.tableContext?.columns.filter((c) => c.isPrimaryKey).map((c) => c.name) ?? [];
      for (const k of Object.keys(columnMappings)) delete columnMappings[k];
    }
  },
);

async function switchXlsxSheet(sheetName: string) {
  if (!importSourceFile.value) return;
  try {
    for (const k of Object.keys(columnMappings)) delete columnMappings[k];
    importXlsxPreview.value = await readXlsxSheet(importSourceFile.value, sheetName, 2000);
    emit('notify', `已切换 Excel 工作表：${importXlsxPreview.value.activeSheet}`);
  } catch (err) {
    emit('notify', `读取工作表失败: ${String(err)}`);
  }
}

async function browseImportDataFile() {
  try {
    const path = await pickDataImportFile();
    if (!path) return;
    for (const k of Object.keys(columnMappings)) delete columnMappings[k];
    if (path.toLowerCase().endsWith('.xlsx')) {
      importSourceFile.value = path;
      importRawText.value = '';
      const preview = await readXlsxSheet(path, null, 2000);
      importXlsxPreview.value = preview;
      emit(
        'notify',
        `已解析 Excel 工作簿「${preview.activeSheet}」(${preview.rows.length}${preview.truncated ? `+ / 共 ${preview.totalRows}` : ''} 行)`,
      );
      return;
    }
    importXlsxPreview.value = null;
    const opened = await openTextFile(path);
    importSourceFile.value = path;
    importRawText.value = opened.content;
    emit('notify', `已载入数据文件 (${opened.encoding})：${opened.name}`);
  } catch (err) {
    emit('notify', `读取数据文件失败: ${String(err)}`);
  }
}

function buildMappedImportStatements(): string[] {
  const ctx = props.tableContext;
  if (!ctx || parsedImport.value.rows.length === 0) return [];
  const activePairs = parsedImport.value.headers
    .map((src) => ({ src, dst: columnMappings[src] || '' }))
    .filter((p) => Boolean(p.dst));
  if (activePairs.length === 0) return [];

  const targetColumns: string[] = [];
  const targetColumnTypes: Record<string, string> = {};
  const srcByTarget: Record<string, string> = {};
  for (const p of activePairs) {
    if (!srcByTarget[p.dst]) {
      srcByTarget[p.dst] = p.src;
      targetColumns.push(p.dst);
      const colMeta = ctx.columns.find((c) => c.name === p.dst);
      if (colMeta) targetColumnTypes[p.dst] = colMeta.dataType;
    }
  }

  const mappedRows = parsedImport.value.rows.map((r) => {
    const out: Record<string, unknown> = {};
    for (const dst of targetColumns) {
      out[dst] = r[srcByTarget[dst]];
    }
    return out;
  });

  return buildBatchInsertStatements({
    tableName: ctx.tableName,
    dbName: ctx.dbName,
    engineKey: ctx.engineKey,
    targetColumns,
    targetColumnTypes,
    rows: mappedRows,
    batchSize: importBatchSize.value,
    ignoreDuplicates: importMode.value === 'ignore',
    mode: importMode.value,
    conflictKeys: importConflictKeys.value,
  });
}

function sendImportSqlToEditor() {
  const ctx = props.tableContext;
  const stmts = buildMappedImportStatements();
  if (!ctx || stmts.length === 0) {
    emit('notify', '请先载入有效数据并至少映射一个目标列');
    return;
  }
  const sql = `-- 批量导入 ${ctx.dbName}.${ctx.tableName} (${parsedImport.value.rows.length} 行)\n${stmts.join('\n')}\n`;
  emit('send-sql-to-editor', sql, ctx.profileId, ctx.dbName);
  emit('close');
}

function executeImportStatements() {
  const ctx = props.tableContext;
  const stmts = buildMappedImportStatements();
  if (!ctx || stmts.length === 0) {
    emit('notify', '请先载入有效数据并至少映射一个目标列');
    return;
  }
  emit('run-statements', stmts, `导入 ${ctx.tableName}`, ctx.profileId, ctx.dbName, false);
  emit('close');
}

// ==================== 2. Entity / Model Code Generator ====================
const codegenLang = ref<EntityCodeLang>('java');
const codegenCamelCase = ref(true);
const codegenOrmTags = ref(true);
const codegenComments = ref(true);
const codegenPythonStyle = ref<'dataclass' | 'pydantic'>('pydantic');

const generatedEntity = computed(() => {
  const ctx = props.tableContext;
  if (!ctx) return { filename: 'Entity.txt', code: '' };
  return generateEntityCode(ctx.tableName, ctx.columns, codegenLang.value, {
    camelCase: codegenCamelCase.value,
    withOrmTags: codegenOrmTags.value,
    withComments: codegenComments.value,
    pythonStyle: codegenPythonStyle.value,
  });
});

async function copyGeneratedCode() {
  try {
    await navigator.clipboard.writeText(generatedEntity.value.code);
    emit('notify', `已复制 ${generatedEntity.value.filename} 源码到剪贴板`);
  } catch {
    emit('notify', '复制代码失败');
  }
}

function downloadGeneratedCode() {
  downloadTextFile(
    generatedEntity.value.filename,
    generatedEntity.value.code,
    'text/plain;charset=utf-8',
  );
  emit('notify', `已导出 ${generatedEntity.value.filename}`);
}

// ==================== 3. Visual Create Table Designer ====================
const designerTableName = ref('new_table');
const designerComment = ref('');
const designerColumns = ref<CreateTableColumnDraft[]>([]);
const designerIndexes = ref<CreateTableIndexDraft[]>([]);

const COMMON_COLUMN_TYPES = [
  'BIGINT',
  'INT',
  'TINYINT',
  'VARCHAR(128)',
  'VARCHAR(64)',
  'TEXT',
  'DECIMAL(18,2)',
  'BOOLEAN',
  'DATETIME',
  'TIMESTAMP',
  'DATE',
  'JSON',
];

function resetTableDesigner() {
  designerTableName.value = 'biz_record';
  designerComment.value = '';
  designerColumns.value = [
    {
      name: 'id',
      dataType: 'BIGINT',
      length: '',
      nullable: false,
      primaryKey: true,
      autoIncrement: true,
      defaultValue: '',
      comment: '主键ID',
    },
    {
      name: 'title',
      dataType: 'VARCHAR',
      length: '128',
      nullable: false,
      primaryKey: false,
      autoIncrement: false,
      defaultValue: '',
      comment: '名称',
    },
    {
      name: 'status',
      dataType: 'TINYINT',
      length: '',
      nullable: false,
      primaryKey: false,
      autoIncrement: false,
      defaultValue: '1',
      comment: '状态',
    },
    {
      name: 'created_at',
      dataType: 'DATETIME',
      length: '',
      nullable: false,
      primaryKey: false,
      autoIncrement: false,
      defaultValue: 'CURRENT_TIMESTAMP',
      comment: '创建时间',
    },
  ];
  designerIndexes.value = [];
}

watch(
  () => props.activeToolModal,
  (kind) => {
    if (kind === 'create-table') {
      resetTableDesigner();
    } else if (kind === 'rename-table' && props.tableContext) {
      renameNewTable.value = `${props.tableContext.tableName}_new`;
    }
  },
);

function addDesignerColumn() {
  designerColumns.value.push({
    name: `col_${designerColumns.value.length + 1}`,
    dataType: 'VARCHAR',
    length: '128',
    nullable: true,
    primaryKey: false,
    autoIncrement: false,
    defaultValue: '',
    comment: '',
  });
}

function moveDesignerColumn(idx: number, delta: -1 | 1) {
  const next = idx + delta;
  if (next < 0 || next >= designerColumns.value.length) return;
  const arr = designerColumns.value;
  [arr[idx], arr[next]] = [arr[next], arr[idx]];
}

function addDesignerIndex() {
  const firstCol = designerColumns.value[0]?.name || 'id';
  designerIndexes.value.push({
    name: `idx_${designerTableName.value.trim() || 'tbl'}_${designerIndexes.value.length + 1}`,
    columns: firstCol,
    unique: false,
  });
}

const generatedCreateDdl = computed(() => {
  const ctx = props.dbContext;
  if (!ctx) return '';
  return generateCreateTableDdl(
    {
      dbName: ctx.dbName,
      tableName: designerTableName.value,
      comment: designerComment.value,
      columns: designerColumns.value,
      indexes: designerIndexes.value,
    },
    ctx.engineKey,
  );
});

async function copyCreateDdl() {
  try {
    await navigator.clipboard.writeText(generatedCreateDdl.value);
    emit('notify', '已复制 CREATE TABLE DDL');
  } catch {
    emit('notify', '复制 DDL 失败');
  }
}

function sendCreateDdlToEditor() {
  const ctx = props.dbContext;
  if (!ctx) return;
  emit('send-sql-to-editor', `${generatedCreateDdl.value}\n`, ctx.profileId, ctx.dbName);
  emit('close');
}

function executeCreateDdl() {
  const ctx = props.dbContext;
  if (!ctx) return;
  if (!designerTableName.value.trim()) {
    emit('notify', '请填写新表名');
    return;
  }
  const stmts = generatedCreateDdl.value
    .split(';')
    .map((s) => s.trim())
    .filter(Boolean);
  if (stmts.length === 0) return;
  emit(
    'run-statements',
    stmts,
    `建表 ${designerTableName.value.trim()}`,
    ctx.profileId,
    ctx.dbName,
    true,
  );
  emit('close');
}

// ==================== 4. Rename Table Modal ====================
const renameNewTable = ref('');

const generatedRenameSql = computed(() => {
  const ctx = props.tableContext;
  if (!ctx) return '';
  return buildRenameTableSql(
    ctx.dbName,
    ctx.tableName,
    renameNewTable.value.trim() || `${ctx.tableName}_new`,
    ctx.engineKey,
  );
});

function sendRenameToEditor() {
  const ctx = props.tableContext;
  if (!ctx) return;
  emit('send-sql-to-editor', `${generatedRenameSql.value}\n`, ctx.profileId, ctx.dbName);
  emit('close');
}

function executeRenameTable() {
  const ctx = props.tableContext;
  if (!ctx) return;
  const target = renameNewTable.value.trim();
  if (!target || target === ctx.tableName) {
    emit('notify', '请输入与原表名不同的新表名');
    return;
  }
  const cleanSql = generatedRenameSql.value.replace(/;+\s*$/, '');
  emit('run-statements', [cleanSql], `重命名 ${ctx.tableName}`, ctx.profileId, ctx.dbName, true);
  emit('close');
}
</script>

<template>
  <!-- 1. CSV / TSV / JSON Data Import Wizard -->
  <div
    v-if="activeToolModal === 'import-data' && tableContext"
    class="modal-backdrop"
    @click.self="emit('close')"
  >
    <div class="modal-card" style="width: min(860px, 95vw)">
      <div class="modal-header">
        <span>
          📥 数据导入向导 ·
          <span style="font-family: var(--font-mono)">
            {{ tableContext.dbName }}.{{ tableContext.tableName }}
          </span>
          <span class="pill pill-neutral" style="margin-left: 6px">
            {{ getEngineDisplayName(tableContext.engineKey) }}
          </span>
        </span>
        <button class="icon-btn compact" @click="emit('close')">×</button>
      </div>

      <div style="display: flex; align-items: center; gap: 8px; flex-wrap: wrap; margin-bottom: 10px">
        <button class="btn btn-sm" @click="browseImportDataFile">
          <IdeIcon name="folder-open" :size="12" /> 选择 Excel (.xlsx) / CSV / JSON...
        </button>
        <select
          v-if="importXlsxPreview"
          :value="importXlsxPreview.activeSheet"
          class="field"
          style="width: auto; font-size: 11.5px"
          @change="switchXlsxSheet(($event.target as HTMLSelectElement).value)"
        >
          <option v-for="sn in importXlsxPreview.sheetNames" :key="sn" :value="sn">
            Sheet: {{ sn }}
          </option>
        </select>
        <select
          v-if="!importXlsxPreview"
          v-model="importFormat"
          class="field"
          style="width: auto; font-size: 11.5px"
        >
          <option value="auto">格式：自动识别</option>
          <option value="csv">CSV / TSV</option>
          <option value="json">JSON 数组</option>
        </select>
        <select
          v-if="!importXlsxPreview"
          v-model="importDelimiter"
          class="field"
          style="width: auto; font-size: 11.5px"
          :disabled="importFormat === 'json'"
        >
          <option value="auto">分隔符：自动</option>
          <option value=",">逗号 (,)</option>
          <option value="	">制表符 (Tab)</option>
          <option value=";">分号 (;)</option>
        </select>
        <label style="display: inline-flex; align-items: center; gap: 4px; font-size: 11.5px; cursor: pointer">
          <input v-model="importHasHeader" type="checkbox" />
          <span>首行为列名</span>
        </label>
        <select v-model="importMode" class="field" style="width: auto; font-size: 11.5px" title="冲突写入策略">
          <option value="insert">普通插入 (INSERT)</option>
          <option value="ignore">忽略冲突 (IGNORE)</option>
          <option value="upsert">冲突更新 (UPSERT)</option>
        </select>
        <select v-model.number="importBatchSize" class="field" style="width: auto; font-size: 11.5px">
          <option :value="50">每批 50 行</option>
          <option :value="100">每批 100 行</option>
          <option :value="500">每批 500 行</option>
        </select>
      </div>

      <div
        v-if="importMode === 'upsert' && tableContext"
        style="display: flex; align-items: center; gap: 6px; flex-wrap: wrap; margin-bottom: 8px; padding: 6px 8px; border: 1px solid var(--border); border-radius: 6px; font-size: 11px"
      >
        <span style="color: var(--text-secondary); font-weight: 600">冲突判定键 (PG/SQLite):</span>
        <label
          v-for="col in tableContext.columns"
          :key="`ck-${col.name}`"
          style="display: inline-flex; align-items: center; gap: 3px; cursor: pointer"
        >
          <input
            type="checkbox"
            :checked="importConflictKeys.includes(col.name)"
            @change="toggleConflictKey(col.name)"
          />
          <span>{{ col.name }}</span>
        </label>
      </div>

      <div v-if="importSourceFile" style="font-size: 11px; color: var(--text-secondary); margin-bottom: 6px; font-family: var(--font-mono); display: flex; justify-content: space-between; align-items: center; gap: 8px">
        <span style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap">
          已载入：{{ importSourceFile }}
          <template v-if="importXlsxPreview?.truncated">
            (预览前 {{ importXlsxPreview.rows.length }} / 共 {{ importXlsxPreview.totalRows }} 行)
          </template>
        </span>
        <button
          v-if="importXlsxPreview"
          class="icon-btn compact"
          @click="importXlsxPreview = null; importSourceFile = ''"
        >
          清除重选
        </button>
      </div>

      <textarea
        v-if="!importXlsxPreview"
        v-model="importRawText"
        class="field"
        rows="4"
        style="font-family: var(--font-mono); font-size: 11.5px; margin-bottom: 8px; resize: vertical"
        placeholder="粘贴 CSV / TSV 文本或 JSON 数组（也可点击上方按钮直接读取 Excel .xlsx 工作簿 / CSV / JSON 文件）..."
      />

      <div v-if="parsedImport.error" style="color: var(--prod); font-size: 12px; margin-bottom: 8px">
        解析失败：{{ parsedImport.error }}
      </div>

      <template v-else-if="parsedImport.headers.length > 0">
        <div style="font-size: 11.5px; font-weight: 600; margin-bottom: 5px; display: flex; justify-content: space-between">
          <span>
            字段映射（已识别 {{ parsedImport.detectedFormat.toUpperCase() }} · 共 {{ parsedImport.rows.length }} 行）
          </span>
          <span style="font-weight: 400; color: var(--text-secondary)">
            目标表共 {{ tableContext.columns.length }} 列
          </span>
        </div>

        <div class="designer-scroll" style="max-height: 150px; margin-bottom: 10px">
          <table class="designer-table">
            <thead>
              <tr>
                <th>源字段</th>
                <th>样例值 (第 1 行)</th>
                <th>映射到目标表列</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="srcCol in parsedImport.headers" :key="srcCol">
                <td style="font-family: var(--font-mono); font-weight: 600">{{ srcCol }}</td>
                <td style="font-family: var(--font-mono); color: var(--text-secondary); max-width: 240px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap">
                  {{ parsedImport.rows[0]?.[srcCol] ?? '' }}
                </td>
                <td>
                  <select v-model="columnMappings[srcCol]" class="field" style="padding: 2px 6px; font-size: 11.5px">
                    <option value="">&lt;忽略此列&gt;</option>
                    <option v-for="tc in tableContext.columns" :key="tc.name" :value="tc.name">
                      {{ tc.name }} ({{ tc.dataType }})
                    </option>
                  </select>
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <div style="font-size: 11.5px; font-weight: 600; margin-bottom: 4px">
          数据预览（前 {{ Math.min(5, parsedImport.rows.length) }} 行）
        </div>
        <div class="designer-scroll" style="max-height: 130px; margin-bottom: 10px">
          <table class="designer-table">
            <thead>
              <tr>
                <th v-for="h in parsedImport.headers" :key="h">{{ h }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="(r, rIdx) in parsedImport.rows.slice(0, 5)" :key="rIdx">
                <td v-for="h in parsedImport.headers" :key="h" style="font-family: var(--font-mono)">
                  {{ r[h] ?? '' }}
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </template>

      <div style="display: flex; justify-content: flex-end; gap: 8px; margin-top: 10px">
        <button class="btn" @click="emit('close')">取消</button>
        <button
          class="btn"
          :disabled="parsedImport.rows.length === 0"
          @click="sendImportSqlToEditor"
        >
          生成 INSERT 到编辑器
        </button>
        <button
          class="btn btn-primary"
          :disabled="busy || parsedImport.rows.length === 0"
          @click="executeImportStatements"
        >
          <IdeIcon name="run" :size="12" />
          立即导入执行 ({{ parsedImport.rows.length }} 行)
        </button>
      </div>
    </div>
  </div>

  <!-- 2. Entity / Model Code Generator -->
  <div
    v-else-if="activeToolModal === 'codegen' && tableContext"
    class="modal-backdrop"
    @click.self="emit('close')"
  >
    <div class="modal-card" style="width: min(820px, 94vw)">
      <div class="modal-header">
        <span>
          💻 实体 / 模型代码生成器 ·
          <span style="font-family: var(--font-mono)">
            {{ tableContext.dbName }}.{{ tableContext.tableName }}
          </span>
        </span>
        <button class="icon-btn compact" @click="emit('close')">×</button>
      </div>

      <div class="conn-tabs-bar" style="margin: 0 0 10px; padding-bottom: 8px">
        <div class="conn-tab-list">
          <button
            v-for="lang in [
              { id: 'java', label: 'Java (Lombok/MyBatis)' },
              { id: 'go', label: 'Go (GORM/JSON)' },
              { id: 'typescript', label: 'TypeScript' },
              { id: 'python', label: 'Python' },
              { id: 'rust', label: 'Rust (Serde)' },
            ]"
            :key="lang.id"
            class="conn-tab"
            :class="{ active: codegenLang === lang.id }"
            @click="codegenLang = lang.id as EntityCodeLang"
          >
            {{ lang.label }}
          </button>
        </div>
      </div>

      <div style="display: flex; align-items: center; gap: 14px; flex-wrap: wrap; margin-bottom: 10px; font-size: 12px">
        <label style="display: inline-flex; align-items: center; gap: 5px; cursor: pointer">
          <input v-model="codegenCamelCase" type="checkbox" />
          <span>小驼峰字段 (camelCase)</span>
        </label>
        <label style="display: inline-flex; align-items: center; gap: 5px; cursor: pointer">
          <input v-model="codegenOrmTags" type="checkbox" />
          <span>包含 ORM / 序列化注解</span>
        </label>
        <label style="display: inline-flex; align-items: center; gap: 5px; cursor: pointer">
          <input v-model="codegenComments" type="checkbox" />
          <span>保留字段注释</span>
        </label>
        <select
          v-if="codegenLang === 'python'"
          v-model="codegenPythonStyle"
          class="field"
          style="width: auto; font-size: 11.5px"
        >
          <option value="pydantic">Pydantic BaseModel</option>
          <option value="dataclass">标准 dataclass</option>
        </select>
        <span style="margin-left: auto; font-family: var(--font-mono); font-size: 11px; color: var(--text-secondary)">
          {{ generatedEntity.filename }}
        </span>
      </div>

      <pre class="code-viewer" style="max-height: 410px; white-space: pre">{{ generatedEntity.code }}</pre>

      <div style="display: flex; justify-content: flex-end; gap: 8px; margin-top: 12px">
        <button class="btn" @click="emit('close')">关闭</button>
        <button class="btn" @click="downloadGeneratedCode">
          下载 {{ generatedEntity.filename }}
        </button>
        <button class="btn btn-primary" @click="copyGeneratedCode">
          <IdeIcon name="copy" :size="12" /> 复制代码
        </button>
      </div>
    </div>
  </div>

  <!-- 3. Visual Create Table Designer -->
  <div
    v-else-if="activeToolModal === 'create-table' && dbContext"
    class="modal-backdrop"
    @click.self="emit('close')"
  >
    <div class="modal-card" style="width: min(920px, 96vw)">
      <div class="modal-header">
        <span>
          ✨ 可视化建表设计器 ·
          <span style="font-family: var(--font-mono)">{{ dbContext.dbName }}</span>
          <span class="pill pill-neutral" style="margin-left: 6px">
            {{ getEngineDisplayName(dbContext.engineKey) }}
          </span>
        </span>
        <button class="icon-btn compact" @click="emit('close')">×</button>
      </div>

      <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 10px; margin-bottom: 10px">
        <div class="form-field">
          <label>新数据表名</label>
          <input
            v-model="designerTableName"
            class="field"
            style="font-family: var(--font-mono)"
            placeholder="例如：user_orders"
          />
        </div>
        <div class="form-field">
          <label>表注释 (Comment)</label>
          <input v-model="designerComment" class="field" placeholder="可选，表用途说明" />
        </div>
      </div>

      <div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 5px">
        <span style="font-size: 12px; font-weight: 600">字段结构设计 ({{ designerColumns.length }})</span>
        <button class="btn btn-sm" @click="addDesignerColumn">
          <IdeIcon name="plus" :size="11" /> 添加字段
        </button>
      </div>

      <div class="designer-scroll" style="max-height: 215px; margin-bottom: 10px">
        <table class="designer-table">
          <thead>
            <tr>
              <th>字段名</th>
              <th>类型</th>
              <th style="width: 70px">长度</th>
              <th style="width: 34px" title="主键">PK</th>
              <th style="width: 34px" title="自增">AI</th>
              <th style="width: 44px">非空</th>
              <th>默认值</th>
              <th>注释</th>
              <th style="width: 76px">操作</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(col, idx) in designerColumns" :key="idx">
              <td>
                <input v-model="col.name" class="field" style="font-family: var(--font-mono); padding: 2px 5px" />
              </td>
              <td>
                <input
                  v-model="col.dataType"
                  list="pacer-col-types"
                  class="field"
                  style="font-family: var(--font-mono); padding: 2px 5px"
                />
              </td>
              <td>
                <input v-model="col.length" class="field" style="font-family: var(--font-mono); padding: 2px 5px" placeholder="如 128" />
              </td>
              <td style="text-align: center">
                <input v-model="col.primaryKey" type="checkbox" />
              </td>
              <td style="text-align: center">
                <input v-model="col.autoIncrement" type="checkbox" />
              </td>
              <td style="text-align: center">
                <input type="checkbox" :checked="!col.nullable" @change="col.nullable = !col.nullable" />
              </td>
              <td>
                <input v-model="col.defaultValue" class="field" style="font-family: var(--font-mono); padding: 2px 5px" placeholder="可选" />
              </td>
              <td>
                <input v-model="col.comment" class="field" style="padding: 2px 5px" placeholder="注释" />
              </td>
              <td>
                <div style="display: flex; gap: 2px">
                  <button class="icon-btn compact" title="上移" @click="moveDesignerColumn(idx, -1)">↑</button>
                  <button class="icon-btn compact" title="下移" @click="moveDesignerColumn(idx, 1)">↓</button>
                  <button
                    class="icon-btn compact"
                    title="删除字段"
                    :disabled="designerColumns.length <= 1"
                    @click="designerColumns.splice(idx, 1)"
                  >
                    ×
                  </button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
        <datalist id="pacer-col-types">
          <option v-for="tOpt in COMMON_COLUMN_TYPES" :key="tOpt" :value="tOpt" />
        </datalist>
      </div>

      <div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 5px">
        <span style="font-size: 12px; font-weight: 600">普通 / 唯一索引 ({{ designerIndexes.length }})</span>
        <button class="btn btn-sm" @click="addDesignerIndex">
          <IdeIcon name="plus" :size="11" /> 添加索引
        </button>
      </div>

      <div v-if="designerIndexes.length > 0" class="designer-scroll" style="max-height: 110px; margin-bottom: 10px">
        <table class="designer-table">
          <tbody>
            <tr v-for="(idxItem, i) in designerIndexes" :key="i">
              <td>
                <input v-model="idxItem.name" class="field" style="font-family: var(--font-mono); padding: 2px 5px" placeholder="索引名" />
              </td>
              <td style="width: 72px">
                <label style="display: inline-flex; align-items: center; gap: 4px; font-size: 11px; cursor: pointer">
                  <input v-model="idxItem.unique" type="checkbox" /> 唯一
                </label>
              </td>
              <td>
                <input v-model="idxItem.columns" class="field" style="font-family: var(--font-mono); padding: 2px 5px" placeholder="索引列，多列以逗号分隔" />
              </td>
              <td style="width: 34px; text-align: center">
                <button class="icon-btn compact" @click="designerIndexes.splice(i, 1)">×</button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <div style="font-size: 11.5px; font-weight: 600; margin-bottom: 4px">
        实时生成 DDL 预览（{{ getEngineDisplayName(dbContext.engineKey) }} 方言）
      </div>
      <pre class="code-viewer" style="max-height: 165px">{{ generatedCreateDdl }}</pre>

      <div style="display: flex; justify-content: flex-end; gap: 8px; margin-top: 12px">
        <button class="btn" @click="emit('close')">取消</button>
        <button class="btn" @click="copyCreateDdl">
          <IdeIcon name="copy" :size="12" /> 复制 DDL
        </button>
        <button class="btn" @click="sendCreateDdlToEditor">发送到编辑器</button>
        <button class="btn btn-primary" :disabled="busy" @click="executeCreateDdl">
          <IdeIcon name="run" :size="12" /> 立即执行建表
        </button>
      </div>
    </div>
  </div>

  <!-- 4. Rename Table Modal -->
  <div
    v-else-if="activeToolModal === 'rename-table' && tableContext"
    class="modal-backdrop"
    @click.self="emit('close')"
  >
    <div class="modal-card" style="width: min(500px, 92vw)">
      <div class="modal-header">
        <span>
          ✏️ 重命名数据表 ·
          <span style="font-family: var(--font-mono)">{{ tableContext.tableName }}</span>
        </span>
        <button class="icon-btn compact" @click="emit('close')">×</button>
      </div>

      <div class="form-field" style="margin-bottom: 10px">
        <label>新表名（所在库：{{ tableContext.dbName }}）</label>
        <input
          v-model="renameNewTable"
          class="field"
          style="font-family: var(--font-mono)"
          placeholder="输入新表名"
          @keydown.enter.prevent="executeRenameTable"
        />
      </div>

      <pre class="code-viewer" style="margin-bottom: 12px">{{ generatedRenameSql }}</pre>

      <div style="display: flex; justify-content: flex-end; gap: 8px">
        <button class="btn" @click="emit('close')">取消</button>
        <button class="btn" @click="sendRenameToEditor">发送到编辑器</button>
        <button
          class="btn btn-primary"
          :disabled="busy || !renameNewTable.trim() || renameNewTable.trim() === tableContext.tableName"
          @click="executeRenameTable"
        >
          确认重命名
        </button>
      </div>
    </div>
  </div>
</template>
