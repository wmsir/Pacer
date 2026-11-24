<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, reactive, ref, watch } from 'vue';
import { useI18n } from '../i18n';
import {
  exportRowsXlsx,
  isTauriRuntime,
  pickSaveXlsxPath,
  type ConnectionProfile,
  type EnvLevel,
  type ForeignKeyMeta,
  type QueryHistoryEntry,
  type QuerySummary,
  type ScriptAnalysis,
  type WorkspaceProject,
} from '../api/ipc';
import IdeIcon from './IdeIcon.vue';
import { IS_MAC } from '../utils/keymap';
import {
  appendWhereCondition,
  isExplainResult,
  parseExplainPlanVisual,
  type VisualExplainPlan,
} from '../utils/commercialTools';
import {
  buildChartDataFromRows,
  computeFixedVirtualRange,
  computeGridColumnWidths,
  computeVariableVirtualRange,
  decodeUnixTimestampHint,
  detectChartColumns,
  downloadTextFile,
  formatSqlLiteral,
  generateGridMutationSql,
  quoteIdentifier,
  toCsvWithBom,
  toInsertSql,
  toMarkdownTable,
  toTsv,
  toUpdateSql,
  type GridMutationPlan,
  type QuickChartType,
} from '../utils/sqlTools';

export interface ResultTabItem {
  id: string;
  title: string;
  sql: string;
  profileId: string;
  profileName: string;
  profileEnv: EnvLevel;
  engineKey: string;
  dbName: string;
  readOnly: boolean;
  pinned?: boolean;
  sourceTable: string | null;
  availableTables: string[];
  primaryKeys: string[];
  foreignKeys?: ForeignKeyMeta[];
  columns: string[];
  rows: Record<string, unknown>[];
  summary: QuerySummary | null;
  serverWhere?: string;
  serverOrderBy?: string;
  pageIndex?: number;
  pageSize?: number;
  totalRowCount?: number | null;
}

export interface ConsoleLogEntry {
  id: string;
  timestamp: string;
  projectId: string;
  profileId?: string;
  profileName: string;
  dbName?: string;
  level: 'info' | 'success' | 'warn' | 'error' | 'sql';
  summary: string;
  detail?: string;
  durationMs?: number;
}

export interface GridMutationPreviewPayload {
  resultTabId: string;
  profileId: string;
  profileName: string;
  profileEnv: EnvLevel;
  dbName: string;
  tableName: string;
  plan: GridMutationPlan;
}

interface TabEditDraft {
  modified: Record<number, Record<string, unknown>>;
  deleted: number[];
  inserted: Array<Record<string, unknown>>;
  overrideTable: string;
}

const props = defineProps<{
  resultTabs: ResultTabItem[];
  activeResultTabId: string;
  activeServiceTab: 'result' | 'output' | 'ddl' | 'history' | 'problems';
  projects: WorkspaceProject[];
  activeProjectId: string;
  activeProjectName: string;
  connections: ConnectionProfile[];
  consoleLogs: ConsoleLogEntry[];
  ddlText: string;
  ddlTitle: string;
  historyEntries: QueryHistoryEntry[];
  astAnalysis: ScriptAnalysis | null;
  executing?: boolean;
}>();

const emit = defineEmits<{
  (e: 'update:activeServiceTab', tab: 'result' | 'output' | 'ddl' | 'history' | 'problems'): void;
  (e: 'update:activeResultTabId', id: string): void;
  (e: 'close-result-tab', id: string): void;
  (e: 'toggle-pin-result-tab', id: string): void;
  (e: 'close-other-result-tabs', id: string): void;
  (e: 'load-history-sql', entry: QueryHistoryEntry): void;
  (e: 'preview-grid-mutations', payload: GridMutationPreviewPayload): void;
  (e: 'clear-console-logs', projectId: string): void;
  (e: 'run-console-sql', sql: string): void;
  (e: 'send-sql-to-editor', sql: string): void;
  (e: 'rerun-result', tab: ResultTabItem): void;
  (
    e: 'server-query-page',
    tab: ResultTabItem,
    opts: { where: string; orderBy: string; pageIndex: number; pageSize: number },
  ): void;
  (e: 'server-count-rows', tab: ResultTabItem): void;
  (e: 'jump-foreign-key', tab: ResultTabItem, fk: ForeignKeyMeta, cellValue: unknown): void;
  (e: 'open-data-diff', tab: ResultTabItem): void;
  (e: 'notify', message: string): void;
}>();

const { t } = useI18n();

// ==================== Grid State ====================
const filterText = ref('');
const serverWhereInput = ref('');
const serverOrderByInput = ref('');
const sortCol = ref<string | null>(null);
const sortDir = ref<'asc' | 'desc'>('asc');
const transposeMode = ref(false);
const recordFieldFilter = ref('');
const visualExplainMode = ref(false);
const chartMode = ref(false);
const chartType = ref<QuickChartType>('bar');
const chartLabelCol = ref('');
const chartValueCol = ref('');
const chartSvgRef = ref<SVGSVGElement | null>(null);
const showValueViewer = ref(false);
const selectedRowOrigIdx = ref<number>(0);
const selectedIsInserted = ref<boolean>(false);
const selectedColName = ref<string>('');
const valueViewerDraft = ref<string>('');
const selectedRowIndices = ref<number[]>([]);
const lastClickedDisplayIdx = ref<number | null>(null);
const pinnedColsByTab = reactive<Record<string, string[]>>({});

// Inline editing state
const editingCell = ref<{
  isInserted: boolean;
  rowIdx: number;
  col: string;
  draftText: string;
} | null>(null);
const inlineInputRef = ref<HTMLInputElement | null>(null);

// Column visibility & drag-width state
const hiddenColsByTab = reactive<Record<string, string[]>>({});
const colWidths = reactive<Record<string, number>>({});
const showColPicker = ref(false);
const colPickerSearch = ref('');
const showExportMenu = ref(false);

// Per-result-tab mutation drafts
const draftsByTab = reactive<Record<string, TabEditDraft>>({});

function ensureDraft(tabId: string): TabEditDraft {
  if (!draftsByTab[tabId]) {
    draftsByTab[tabId] = {
      modified: {},
      deleted: [],
      inserted: [],
      overrideTable: '',
    };
  }
  return draftsByTab[tabId];
}

function clearDraft(tabId: string) {
  draftsByTab[tabId] = {
    modified: {},
    deleted: [],
    inserted: [],
    overrideTable: draftsByTab[tabId]?.overrideTable ?? '',
  };
  editingCell.value = null;
}

defineExpose({ clearDraft });

const currentResult = computed(
  () =>
    props.resultTabs.find((tab) => tab.id === props.activeResultTabId) ??
    props.resultTabs[0] ??
    null,
);

const activeDraft = computed<TabEditDraft | null>(() => {
  const res = currentResult.value;
  if (!res) return null;
  return ensureDraft(res.id);
});

const hiddenColsSet = computed(() => {
  const res = currentResult.value;
  if (!res) return new Set<string>();
  return new Set(hiddenColsByTab[res.id] ?? []);
});

const visibleColumns = computed(() => {
  const res = currentResult.value;
  if (!res) return [];
  const hidden = hiddenColsSet.value;
  const shown = res.columns.filter((c) => !hidden.has(c));
  return shown.length > 0 ? shown : res.columns;
});

const filteredPickerColumns = computed(() => {
  const res = currentResult.value;
  if (!res) return [];
  const q = colPickerSearch.value.trim().toLowerCase();
  if (!q) return res.columns;
  return res.columns.filter((c) => c.toLowerCase().includes(q));
});

function toggleResultColumnVisible(col: string) {
  const res = currentResult.value;
  if (!res) return;
  const hidden = new Set(hiddenColsByTab[res.id] ?? []);
  if (hidden.has(col)) {
    hidden.delete(col);
  } else if (res.columns.length - hidden.size > 1) {
    hidden.add(col);
  } else {
    emit('notify', '至少保留一列可见');
    return;
  }
  hiddenColsByTab[res.id] = [...hidden];
}

function hideSpecificColumn(col: string) {
  const res = currentResult.value;
  if (!res) return;
  const hidden = new Set(hiddenColsByTab[res.id] ?? []);
  if (res.columns.length - hidden.size <= 1) {
    emit('notify', '至少保留一列可见');
    return;
  }
  hidden.add(col);
  hiddenColsByTab[res.id] = [...hidden];
  emit('notify', `已隐藏列 [${col}]，可在顶部「列」菜单恢复`);
}

function resetVisibleColumns() {
  const res = currentResult.value;
  if (!res) return;
  delete hiddenColsByTab[res.id];
}

function showOnlyFirst10Columns() {
  const res = currentResult.value;
  if (!res) return;
  hiddenColsByTab[res.id] = res.columns.slice(10);
}

const currentPinnedCols = computed(() => {
  const res = currentResult.value;
  if (!res) return [];
  const visSet = new Set(visibleColumns.value);
  return (pinnedColsByTab[res.id] ?? []).filter((c) => visSet.has(c));
});

const pinnedColumnsSet = computed(() => new Set(currentPinnedCols.value));

const scrollableColumns = computed(() =>
  visibleColumns.value.filter((c) => !pinnedColumnsSet.value.has(c)),
);

const fkByColMap = computed(() => {
  const map = new Map<string, ForeignKeyMeta>();
  for (const fk of currentResult.value?.foreignKeys ?? []) {
    if (!map.has(fk.columnName.toLowerCase())) {
      map.set(fk.columnName.toLowerCase(), fk);
    }
  }
  return map;
});

function getFkForCol(col: string): ForeignKeyMeta | undefined {
  return fkByColMap.value.get(col.toLowerCase());
}

function togglePinColumn(col: string) {
  const res = currentResult.value;
  if (!res || !col) return;
  const list = [...(pinnedColsByTab[res.id] ?? [])];
  const idx = list.indexOf(col);
  if (idx >= 0) {
    list.splice(idx, 1);
    emit('notify', `已取消固定列 [${col}]`);
  } else {
    list.push(col);
    emit('notify', `已固定列 [${col}] 到左侧`);
  }
  pinnedColsByTab[res.id] = list;
}

function clearPinnedColumns() {
  const res = currentResult.value;
  if (!res) return;
  delete pinnedColsByTab[res.id];
}

const isCurrentExplain = computed(() => {
  const res = currentResult.value;
  return Boolean(res && isExplainResult(res.columns, res.sql));
});

const visualExplainPlan = computed<VisualExplainPlan | null>(() => {
  const res = currentResult.value;
  if (!res || !isCurrentExplain.value) return null;
  return parseExplainPlanVisual(res.columns, res.rows, res.engineKey);
});

const recordFormColumns = computed(() => {
  const q = recordFieldFilter.value.trim().toLowerCase();
  if (!q) return visibleColumns.value;
  return visibleColumns.value.filter((c) => c.toLowerCase().includes(q));
});

const currentTransposeDisplayPos = computed(() => {
  const pos = displayedRows.value.findIndex((r) => r.origIdx === selectedRowOrigIdx.value);
  return pos >= 0 ? pos : 0;
});

function stepTransposeRow(delta: -1 | 1) {
  const list = displayedRows.value;
  if (list.length === 0) return;
  const nextPos = Math.max(0, Math.min(list.length - 1, currentTransposeDisplayPos.value + delta));
  const item = list[nextPos];
  if (!item) return;
  selectCell(false, item.origIdx, selectedColName.value || visibleColumns.value[0] || '');
}

function colStyle(col: string) {
  const res = currentResult.value;
  if (!res) return undefined;
  const w = colWidths[`${res.id}:${col}`];
  return w ? { width: `${w}px`, minWidth: `${w}px`, maxWidth: `${w}px` } : undefined;
}

function startColResize(e: MouseEvent, col: string) {
  const res = currentResult.value;
  if (!res) return;
  const th = (e.currentTarget as HTMLElement)?.parentElement;
  const startX = e.clientX;
  const key = `${res.id}:${col}`;
  const startWidth = colWidths[key] ?? (th ? Math.round(th.getBoundingClientRect().width) : 140);

  const onMove = (mv: MouseEvent) => {
    colWidths[key] = Math.max(52, Math.min(680, startWidth + (mv.clientX - startX)));
  };
  const onUp = () => {
    window.removeEventListener('mousemove', onMove);
    window.removeEventListener('mouseup', onUp);
  };
  window.addEventListener('mousemove', onMove);
  window.addEventListener('mouseup', onUp);
}

function resetColWidth(col: string) {
  const res = currentResult.value;
  if (!res) return;
  delete colWidths[`${res.id}:${col}`];
}

const effectiveTargetTable = computed(() => {
  const res = currentResult.value;
  if (!res) return '';
  return activeDraft.value?.overrideTable || res.sourceTable || '';
});

interface IndexedRow {
  origIdx: number;
  row: Record<string, unknown>;
}

const displayedRows = computed<IndexedRow[]>(() => {
  const res = currentResult.value;
  if (!res) return [];
  const draft = activeDraft.value;
  const q = filterText.value.trim().toLowerCase();

  let list: IndexedRow[] = res.rows.map((orig, idx) => {
    const mods = draft?.modified[idx];
    const merged = mods ? { ...orig, ...mods } : orig;
    return { origIdx: idx, row: merged };
  });

  if (q) {
    list = list.filter((item) =>
      res.columns.some((col) =>
        String(item.row[col] ?? '')
          .toLowerCase()
          .includes(q),
      ),
    );
  }

  if (sortCol.value) {
    const col = sortCol.value;
    const dir = sortDir.value === 'asc' ? 1 : -1;
    list.sort((a, b) => {
      const av = a.row[col];
      const bv = b.row[col];
      if (av === bv) return 0;
      if (av === null || av === undefined) return 1;
      if (bv === null || bv === undefined) return -1;
      if (typeof av === 'number' && typeof bv === 'number') return (av - bv) * dir;
      return String(av).localeCompare(String(bv)) * dir;
    });
  }
  return list;
});

const pendingStats = computed(() => {
  const draft = activeDraft.value;
  if (!draft) return { modifiedRows: 0, insertedRows: 0, deletedRows: 0, total: 0 };
  const deletedSet = new Set(draft.deleted);
  const modifiedRows = Object.entries(draft.modified).filter(
    ([idx, cols]) => !deletedSet.has(Number(idx)) && Object.keys(cols).length > 0,
  ).length;
  const insertedRows = draft.inserted.length;
  const deletedRows = draft.deleted.length;
  return {
    modifiedRows,
    insertedRows,
    deletedRows,
    total: modifiedRows + insertedRows + deletedRows,
  };
});

function isCellModified(origIdx: number, col: string): boolean {
  const draft = activeDraft.value;
  if (!draft || !draft.modified[origIdx]) return false;
  return Object.prototype.hasOwnProperty.call(draft.modified[origIdx], col);
}

const deletedRowSet = computed(() => new Set(activeDraft.value?.deleted ?? []));

function isRowDeleted(origIdx: number): boolean {
  return deletedRowSet.value.has(origIdx);
}

function onCellMouseEnter(ev: MouseEvent, origIdx: number, col: string) {
  const el = ev.currentTarget as HTMLElement | null;
  if (el) {
    el.title = originalCellTooltip(origIdx, col);
  }
}

function originalCellTooltip(origIdx: number, col: string): string {
  const res = currentResult.value;
  if (!res) return '';
  const origVal = res.rows[origIdx]?.[col];
  const tsHint = decodeUnixTimestampHint(origVal);
  if (!isCellModified(origIdx, col)) {
    return tsHint ? `Unix 时间解析: ${tsHint}\n双击编辑单元格` : '双击编辑单元格';
  }
  const newVal = activeDraft.value?.modified[origIdx]?.[col];
  return `已修改：${formatCell(origVal)} → ${formatCell(newVal)}（双击继续编辑）${
    tsHint ? `\n原值时间: ${tsHint}` : ''
  }`;
}

function coerceInputValue(rawText: string, originalSample: unknown): unknown {
  if (typeof originalSample === 'number') {
    const trimmed = rawText.trim();
    if (trimmed !== '' && !Number.isNaN(Number(trimmed))) {
      return Number(trimmed);
    }
  } else if (typeof originalSample === 'boolean') {
    const low = rawText.trim().toLowerCase();
    if (low === 'true') return true;
    if (low === 'false') return false;
  }
  return rawText;
}

async function startEditCell(isInserted: boolean, rowIdx: number, col: string) {
  const res = currentResult.value;
  if (!res) return;
  if (res.readOnly) {
    emit('notify', '当前数据源为只读模式，无法修改结果集');
    return;
  }
  const draft = ensureDraft(res.id);
  const currentVal = isInserted
    ? draft.inserted[rowIdx]?.[col]
    : Object.prototype.hasOwnProperty.call(draft.modified[rowIdx] ?? {}, col)
      ? draft.modified[rowIdx][col]
      : res.rows[rowIdx]?.[col];

  editingCell.value = {
    isInserted,
    rowIdx,
    col,
    draftText:
      currentVal === null || currentVal === undefined
        ? ''
        : typeof currentVal === 'object'
          ? JSON.stringify(currentVal)
          : String(currentVal),
  };
  await nextTick();
  inlineInputRef.value?.focus();
  inlineInputRef.value?.select();
}

function commitInlineEdit(setNull = false) {
  const ed = editingCell.value;
  const res = currentResult.value;
  if (!ed || !res) return;
  const draft = ensureDraft(res.id);

  if (ed.isInserted) {
    const targetRow = draft.inserted[ed.rowIdx];
    if (targetRow) {
      targetRow[ed.col] = setNull ? null : ed.draftText;
    }
  } else {
    const origVal = res.rows[ed.rowIdx]?.[ed.col];
    const nextVal = setNull ? null : coerceInputValue(ed.draftText, origVal);
    if (nextVal === origVal) {
      if (draft.modified[ed.rowIdx]) {
        delete draft.modified[ed.rowIdx][ed.col];
        if (Object.keys(draft.modified[ed.rowIdx]).length === 0) {
          delete draft.modified[ed.rowIdx];
        }
      }
    } else {
      if (!draft.modified[ed.rowIdx]) draft.modified[ed.rowIdx] = {};
      draft.modified[ed.rowIdx][ed.col] = nextVal;
    }
  }
  editingCell.value = null;
  syncValueViewerDraft();
}

function cancelInlineEdit() {
  editingCell.value = null;
}

function addNewRow() {
  const res = currentResult.value;
  if (!res) return;
  if (res.readOnly) {
    emit('notify', '当前数据源为只读模式，无法新增行');
    return;
  }
  const draft = ensureDraft(res.id);
  const blank: Record<string, unknown> = {};
  for (const col of res.columns) {
    blank[col] = '';
  }
  draft.inserted.push(blank);
  selectedIsInserted.value = true;
  selectedRowOrigIdx.value = draft.inserted.length - 1;
  selectedColName.value = res.columns[0] ?? '';
  if (res.columns[0]) {
    void startEditCell(true, draft.inserted.length - 1, res.columns[0]);
  }
}

function toggleDeleteSelectedRow() {
  const res = currentResult.value;
  if (!res) return;
  if (res.readOnly) {
    emit('notify', '当前数据源为只读模式，无法删除行');
    return;
  }
  const draft = ensureDraft(res.id);
  if (selectedIsInserted.value) {
    if (selectedRowOrigIdx.value >= 0 && selectedRowOrigIdx.value < draft.inserted.length) {
      draft.inserted.splice(selectedRowOrigIdx.value, 1);
      selectedIsInserted.value = false;
      selectedRowOrigIdx.value = 0;
    }
    return;
  }
  const idx = selectedRowOrigIdx.value;
  if (idx < 0 || idx >= res.rows.length) return;
  const pos = draft.deleted.indexOf(idx);
  if (pos >= 0) {
    draft.deleted.splice(pos, 1);
  } else {
    draft.deleted.push(idx);
  }
}

const selectedRowSet = computed(() => new Set(selectedRowIndices.value));

function onRowNumClick(ev: MouseEvent, origIdx: number, displayIdx: number) {
  selectedIsInserted.value = false;
  selectedRowOrigIdx.value = origIdx;
  if (selectedColName.value) {
    syncValueViewerDraft();
  }
  const rows = displayedRows.value;
  if (ev.shiftKey && lastClickedDisplayIdx.value !== null) {
    const start = Math.min(lastClickedDisplayIdx.value, displayIdx);
    const end = Math.max(lastClickedDisplayIdx.value, displayIdx);
    const merged = new Set(selectedRowIndices.value);
    for (let i = start; i <= end; i += 1) {
      if (rows[i]) merged.add(rows[i].origIdx);
    }
    selectedRowIndices.value = [...merged];
  } else if (ev.ctrlKey || ev.metaKey) {
    const idx = selectedRowIndices.value.indexOf(origIdx);
    if (idx >= 0) {
      selectedRowIndices.value = selectedRowIndices.value.filter((x) => x !== origIdx);
    } else {
      selectedRowIndices.value = [...selectedRowIndices.value, origIdx];
    }
    lastClickedDisplayIdx.value = displayIdx;
  } else {
    if (selectedRowIndices.value.length === 1 && selectedRowIndices.value[0] === origIdx) {
      selectedRowIndices.value = [];
    } else {
      selectedRowIndices.value = [origIdx];
    }
    lastClickedDisplayIdx.value = displayIdx;
  }
}

function toggleSelectAllDisplayedRows() {
  if (displayedRows.value.length === 0) return;
  if (selectedRowIndices.value.length === displayedRows.value.length) {
    selectedRowIndices.value = [];
  } else {
    selectedRowIndices.value = displayedRows.value.map((r) => r.origIdx);
  }
}

function duplicateRowsByIndices(indices: number[]) {
  const res = currentResult.value;
  if (!res) return;
  if (res.readOnly) {
    emit('notify', '当前数据源为只读模式，无法新增复制行');
    return;
  }
  const unique = Array.from(new Set(indices)).filter((i) => i >= 0 && i < res.rows.length);
  if (unique.length === 0) {
    emit('notify', '请先选择要复制的数据行');
    return;
  }
  const draft = ensureDraft(res.id);
  for (const idx of unique) {
    const src = res.rows[idx] ?? {};
    const merged = draft.modified[idx] ? { ...src, ...draft.modified[idx] } : { ...src };
    const clone: Record<string, unknown> = {};
    for (const col of res.columns) {
      clone[col] = merged[col] ?? null;
    }
    draft.inserted.push(clone);
  }
  const lastIdx = draft.inserted.length - 1;
  selectedIsInserted.value = true;
  selectedRowOrigIdx.value = lastIdx;
  emit('notify', `已克隆 ${unique.length} 行为待新增记录，可在底部查看或提交`);
}

function markSelectedRowsDeleted() {
  const res = currentResult.value;
  if (!res || res.readOnly) {
    emit('notify', '当前数据源为只读模式，无法标记删除');
    return;
  }
  if (selectedRowIndices.value.length === 0) return;
  const draft = ensureDraft(res.id);
  const delSet = new Set(draft.deleted);
  for (const idx of selectedRowIndices.value) {
    delSet.add(idx);
  }
  draft.deleted = [...delSet];
  emit('notify', `已将选中的 ${selectedRowIndices.value.length} 行标记为删除`);
}

function copySelectedRowsAsTsv() {
  const res = currentResult.value;
  if (!res || selectedRowIndices.value.length === 0) return;
  const set = new Set(selectedRowIndices.value);
  const rows = displayedRows.value.filter((r) => set.has(r.origIdx)).map((r) => r.row);
  if (rows.length === 0) return;
  void copyText(toTsv(visibleColumns.value, rows));
}

function copySelectedRowsAsInsert() {
  const res = currentResult.value;
  if (!res || selectedRowIndices.value.length === 0) return;
  const set = new Set(selectedRowIndices.value);
  const rows = displayedRows.value.filter((r) => set.has(r.origIdx)).map((r) => r.row);
  if (rows.length === 0) return;
  const tableName = effectiveTargetTable.value || 'target_table';
  void copyText(toInsertSql(tableName, visibleColumns.value, rows, res.engineKey));
}

function submitServerQuery(pageIndex = 0, pageSizeOverride?: number) {
  const res = currentResult.value;
  if (!res || !res.sourceTable) return;
  emit('server-query-page', res, {
    where: serverWhereInput.value.trim(),
    orderBy: serverOrderByInput.value.trim(),
    pageIndex: Math.max(0, pageIndex),
    pageSize: Math.max(1, pageSizeOverride ?? res.pageSize ?? 100),
  });
}

function resetServerQuery() {
  serverWhereInput.value = '';
  serverOrderByInput.value = '';
  submitServerQuery(0);
}

function applyCellServerFilter(operator: '=' | '!=' | 'IS NULL' | 'LIKE') {
  const res = currentResult.value;
  if (!res || !cellCtxMenu.col) return;
  const val = getContextCellRaw();
  serverWhereInput.value = appendWhereCondition(
    serverWhereInput.value,
    cellCtxMenu.col,
    operator,
    val,
    res.engineKey,
  );
  if (res.sourceTable) {
    submitServerQuery(0);
  } else {
    emit('notify', '当前为自定义 SQL 结果，已生成 WHERE 片段（仅单表查询支持一键重查）');
  }
}

function buildCurrentMutationPlan(): GridMutationPlan | null {
  const res = currentResult.value;
  const draft = activeDraft.value;
  if (!res || !draft) return null;
  const tableName = effectiveTargetTable.value.trim();
  if (!tableName) {
    emit('notify', '请先在变更栏中选择目标表名，再生成修改 SQL');
    return null;
  }
  const plan = generateGridMutationSql({
    tableName,
    dbName: res.dbName,
    engineKey: res.engineKey,
    columns: res.columns,
    primaryKeys: res.primaryKeys,
    originalRows: res.rows,
    modifiedCells: draft.modified,
    deletedRowIndices: draft.deleted,
    insertedRows: draft.inserted,
  });
  if (plan.statements.length === 0) {
    emit('notify', '当前没有可提交的有效变更');
    return null;
  }
  return plan;
}

function triggerPreviewMutations() {
  const res = currentResult.value;
  if (!res) return;
  const plan = buildCurrentMutationPlan();
  if (!plan) return;
  emit('preview-grid-mutations', {
    resultTabId: res.id,
    profileId: res.profileId,
    profileName: res.profileName,
    profileEnv: res.profileEnv,
    dbName: res.dbName,
    tableName: effectiveTargetTable.value.trim(),
    plan,
  });
}

function copyMutationsSql() {
  const plan = buildCurrentMutationPlan();
  if (!plan) return;
  void copyText(plan.statements.join('\n'));
}

function revertCurrentDraft() {
  if (!currentResult.value) return;
  clearDraft(currentResult.value.id);
  emit('notify', '已撤销当前结果集的全部未提交修改');
}

// ==================== Selection & Value Viewer ====================
function selectCell(isInserted: boolean, origIdx: number, col: string) {
  selectedIsInserted.value = isInserted;
  selectedRowOrigIdx.value = origIdx;
  selectedColName.value = col;
  syncValueViewerDraft();
}

const inspectedRawValue = computed(() => {
  const res = currentResult.value;
  if (!res || !selectedColName.value) return undefined;
  const draft = activeDraft.value;
  if (selectedIsInserted.value) {
    return draft?.inserted[selectedRowOrigIdx.value]?.[selectedColName.value];
  }
  if (draft?.modified[selectedRowOrigIdx.value] && Object.prototype.hasOwnProperty.call(draft.modified[selectedRowOrigIdx.value], selectedColName.value)) {
    return draft.modified[selectedRowOrigIdx.value][selectedColName.value];
  }
  return res.rows[selectedRowOrigIdx.value]?.[selectedColName.value];
});

function syncValueViewerDraft() {
  const val = inspectedRawValue.value;
  if (val === null || val === undefined) {
    valueViewerDraft.value = '';
    return;
  }
  if (typeof val === 'object') {
    valueViewerDraft.value = JSON.stringify(val, null, 2);
    return;
  }
  const str = String(val);
  if ((str.startsWith('{') && str.endsWith('}')) || (str.startsWith('[') && str.endsWith(']'))) {
    try {
      valueViewerDraft.value = JSON.stringify(JSON.parse(str), null, 2);
      return;
    } catch {
      // fall through
    }
  }
  valueViewerDraft.value = str;
}

const inspectedTimestampHint = computed(() => decodeUnixTimestampHint(inspectedRawValue.value));

function formatValueViewerJson(pretty: boolean) {
  const text = valueViewerDraft.value.trim();
  if (!text) return;
  try {
    const parsed = JSON.parse(text);
    valueViewerDraft.value = pretty ? JSON.stringify(parsed, null, 2) : JSON.stringify(parsed);
  } catch {
    emit('notify', '当前值不是合法的 JSON 格式，无法格式化/压缩');
  }
}

function applyValueViewerEdit(setNull = false) {
  const res = currentResult.value;
  if (!res || !selectedColName.value || res.readOnly) return;
  const draft = ensureDraft(res.id);
  if (selectedIsInserted.value) {
    const row = draft.inserted[selectedRowOrigIdx.value];
    if (row) row[selectedColName.value] = setNull ? null : valueViewerDraft.value;
  } else {
    const origVal = res.rows[selectedRowOrigIdx.value]?.[selectedColName.value];
    const nextVal = setNull ? null : coerceInputValue(valueViewerDraft.value, origVal);
    if (!draft.modified[selectedRowOrigIdx.value]) draft.modified[selectedRowOrigIdx.value] = {};
    draft.modified[selectedRowOrigIdx.value][selectedColName.value] = nextVal;
  }
  emit('notify', `已更新单元格 [${selectedColName.value}]，点击「预览并执行 SQL」提交`);
}

watch(
  () => [props.activeResultTabId, currentResult.value?.serverWhere, currentResult.value?.serverOrderBy],
  ([newId], oldVal) => {
    const res = currentResult.value;
    serverWhereInput.value = res?.serverWhere ?? '';
    serverOrderByInput.value = res?.serverOrderBy ?? '';
    if (!oldVal || newId !== oldVal[0]) {
      editingCell.value = null;
      selectedIsInserted.value = false;
      selectedRowOrigIdx.value = 0;
      selectedRowIndices.value = [];
      lastClickedDisplayIdx.value = null;
      selectedColName.value = res?.columns[0] ?? '';
      recordFieldFilter.value = '';
      visualExplainMode.value = Boolean(
        res && isExplainResult(res.columns, res.sql) && res.rows.length > 0,
      );
      if (visualExplainMode.value) {
        transposeMode.value = false;
        chartMode.value = false;
      }
      syncValueViewerDraft();
    }
  },
  { immediate: true },
);

function toggleSort(col: string) {
  if (sortCol.value === col) {
    if (sortDir.value === 'asc') sortDir.value = 'desc';
    else sortCol.value = null;
  } else {
    sortCol.value = col;
    sortDir.value = 'asc';
  }
}

function formatCell(val: unknown): string {
  if (val === null || val === undefined) return 'NULL';
  if (typeof val === 'object') return JSON.stringify(val);
  return String(val);
}

function isNumeric(val: unknown): boolean {
  return typeof val === 'number';
}

function doExportCsv() {
  const res = currentResult.value;
  if (!res) return;
  const rows = displayedRows.value.map((r) => r.row);
  downloadTextFile(
    `${res.title.replace(/[^\w.-]/g, '_')}.csv`,
    toCsvWithBom(visibleColumns.value, rows),
    'text/csv;charset=utf-8',
  );
  emit('notify', '已导出 UTF-8 BOM CSV 文件（Excel 打开不乱码）');
}

async function doExportXlsx() {
  const res = currentResult.value;
  if (!res) return;
  if (!isTauriRuntime()) {
    emit('notify', '真 Excel (.xlsx) 导出需在桌面端运行时执行；可先使用 CSV / TSV 导出');
    return;
  }
  const cols = visibleColumns.value;
  const rows = displayedRows.value.map((r) => r.row);
  const safeBase = (res.sourceTable || res.title || 'query_result')
    .replace(/[^\w.\-\u4e00-\u9fa5]/g, '_')
    .slice(0, 48);
  try {
    const savePath = await pickSaveXlsxPath(`${safeBase || 'query_result'}.xlsx`);
    if (!savePath) return;
    const count = await exportRowsXlsx(savePath, safeBase || 'QueryResult', cols, rows);
    emit(
      'notify',
      `已导出 Excel (.xlsx)：${count} 行（长整数已按文本防科学计数法）-> ${savePath}`,
    );
  } catch (err) {
    emit('notify', `导出 Excel (.xlsx) 失败: ${String(err)}`);
  }
}

function doExportJson() {
  const res = currentResult.value;
  if (!res) return;
  const cols = visibleColumns.value;
  const rows = displayedRows.value.map((r) => {
    const obj: Record<string, unknown> = {};
    for (const c of cols) obj[c] = r.row[c];
    return obj;
  });
  downloadTextFile(
    `${res.title.replace(/[^\w.-]/g, '_')}.json`,
    JSON.stringify(rows, null, 2),
    'application/json;charset=utf-8',
  );
  emit('notify', '已导出 JSON 文件');
}

function doCopyTsv() {
  const res = currentResult.value;
  if (!res) return;
  void copyText(toTsv(visibleColumns.value, displayedRows.value.map((r) => r.row)));
}

function doCopyMarkdown() {
  const res = currentResult.value;
  if (!res) return;
  void copyText(toMarkdownTable(visibleColumns.value, displayedRows.value.map((r) => r.row)));
}

function doCopyInsertSql() {
  const res = currentResult.value;
  if (!res) return;
  const tableName = effectiveTargetTable.value || 'target_table';
  void copyText(
    toInsertSql(
      tableName,
      visibleColumns.value,
      displayedRows.value.map((r) => r.row),
      res.engineKey,
    ),
  );
}

function doCopyUpdateSql() {
  const res = currentResult.value;
  if (!res) return;
  const tableName = effectiveTargetTable.value || 'target_table';
  void copyText(
    toUpdateSql(
      tableName,
      visibleColumns.value,
      res.primaryKeys,
      displayedRows.value.map((r) => r.row),
      res.engineKey,
    ),
  );
}

function doCopyColumnLines() {
  const col = cellCtxMenu.col || activeStatsColumn.value;
  if (!col) return;
  const vals = displayedRows.value
    .map((r) => r.row[col])
    .filter((v) => v !== null && v !== undefined)
    .map((v) => (typeof v === 'object' ? JSON.stringify(v) : String(v)));
  if (vals.length === 0) {
    emit('notify', `列「${col}」暂无非空值可复制`);
    return;
  }
  void copyText(vals.join('\n'));
}

function copyContextCellPrettyJson() {
  const raw = getContextCellRaw();
  if (raw === null || raw === undefined) {
    void copyText('NULL');
    return;
  }
  if (typeof raw === 'object') {
    void copyText(JSON.stringify(raw, null, 2));
    return;
  }
  const str = String(raw);
  try {
    void copyText(JSON.stringify(JSON.parse(str), null, 2));
  } catch {
    void copyText(str);
  }
}

const activeStatsColumn = computed(() => {
  const res = currentResult.value;
  if (!res || res.columns.length === 0) return '';
  if (selectedColName.value && res.columns.includes(selectedColName.value)) {
    return selectedColName.value;
  }
  return res.columns[0];
});

const columnStats = computed(() => {
  const col = activeStatsColumn.value;
  if (!col) return null;
  const rows = displayedRows.value;
  let nonNull = 0;
  const distinctSet = new Set<string>();
  const nums: number[] = [];
  for (const item of rows) {
    const val = item.row[col];
    if (val === null || val === undefined) continue;
    nonNull += 1;
    const strVal = typeof val === 'object' ? JSON.stringify(val) : String(val);
    distinctSet.add(strVal);
    if (typeof val === 'number' && Number.isFinite(val)) {
      nums.push(val);
    } else if (typeof val === 'string' && /^-?\d+(\.\d+)?$/.test(val.trim())) {
      const parsed = Number(val.trim());
      if (Number.isFinite(parsed) && val.trim().length <= 15) {
        nums.push(parsed);
      }
    }
  }
  const allNumeric = nonNull > 0 && nums.length === nonNull;
  let sum = 0;
  let min = 0;
  let max = 0;
  if (allNumeric) {
    min = nums[0];
    max = nums[0];
    for (const n of nums) {
      sum += n;
      if (n < min) min = n;
      if (n > max) max = n;
    }
  }
  return {
    column: col,
    total: rows.length,
    nonNull,
    distinct: distinctSet.size,
    isNumeric: allNumeric,
    sum: Number(sum.toFixed(4)),
    avg: allNumeric ? Number((sum / nums.length).toFixed(4)) : 0,
    min,
    max,
  };
});

function formatSqlInLiteral(val: unknown): string {
  if (typeof val === 'number' && Number.isFinite(val)) return String(val);
  const raw = typeof val === 'object' ? JSON.stringify(val) : String(val);
  return `'${raw.replace(/'/g, "''")}'`;
}

function doCopyColumnInSql() {
  const col = activeStatsColumn.value;
  if (!col) return;
  const seen = new Set<string>();
  const formatted: string[] = [];
  for (const item of displayedRows.value) {
    const val = item.row[col];
    if (val === null || val === undefined) continue;
    const token = formatSqlInLiteral(val);
    if (!seen.has(token)) {
      seen.add(token);
      formatted.push(token);
    }
  }
  if (formatted.length === 0) {
    emit('notify', `列「${col}」暂无非空值可复制`);
    return;
  }
  const clause = `(${formatted.join(', ')})`;
  void navigator.clipboard
    .writeText(clause)
    .then(() => emit('notify', `已复制列 ${col} 的 IN 条件（共 ${formatted.length} 项）`))
    .catch(() => emit('notify', '剪贴板复制失败'));
}

function doCopySelectedRowJson() {
  const item = displayedRows.value.find((r) => r.origIdx === selectedRowOrigIdx.value);
  const row = item?.row;
  if (!row) {
    emit('notify', '请先点击选择结果集中的任意一行');
    return;
  }
  void copyText(JSON.stringify(row, null, 2));
}

const cellCtxMenu = reactive<{
  visible: boolean;
  x: number;
  y: number;
  origIdx: number;
  col: string;
}>({
  visible: false,
  x: 0,
  y: 0,
  origIdx: 0,
  col: '',
});

function openCellContextMenu(ev: MouseEvent, origIdx: number, col: string) {
  selectCell(false, origIdx, col);
  const maxX = Math.max(8, window.innerWidth - 245);
  const maxY = Math.max(8, window.innerHeight - 420);
  cellCtxMenu.x = Math.min(ev.clientX, maxX);
  cellCtxMenu.y = Math.min(ev.clientY, maxY);
  cellCtxMenu.origIdx = origIdx;
  cellCtxMenu.col = col;
  cellCtxMenu.visible = true;
}

function getContextCellRaw(): unknown {
  return currentResult.value?.rows[cellCtxMenu.origIdx]?.[cellCtxMenu.col];
}

function copyContextCellWhere() {
  const res = currentResult.value;
  if (!res || !cellCtxMenu.col) return;
  const val = getContextCellRaw();
  const qCol = quoteIdentifier(cellCtxMenu.col, res.engineKey);
  const clause =
    val === null || val === undefined
      ? `${qCol} IS NULL`
      : `${qCol} = ${formatSqlLiteral(val)}`;
  void copyText(clause);
}

function copyContextRowInsertSql() {
  const res = currentResult.value;
  if (!res) return;
  const row = res.rows[cellCtxMenu.origIdx];
  if (!row) return;
  const tableName = effectiveTargetTable.value || 'target_table';
  void copyText(toInsertSql(tableName, res.columns, [row], res.engineKey));
}

function setContextCellNull() {
  const res = currentResult.value;
  if (!res || res.readOnly || !cellCtxMenu.col) return;
  const draft = ensureDraft(res.id);
  if (!draft.modified[cellCtxMenu.origIdx]) draft.modified[cellCtxMenu.origIdx] = {};
  draft.modified[cellCtxMenu.origIdx][cellCtxMenu.col] = null;
  emit('notify', `已将 [${cellCtxMenu.col}] 标记为 NULL，点击「预览并执行 SQL」提交`);
}

async function copyText(text: string) {
  try {
    await navigator.clipboard.writeText(text);
    emit('notify', t('copiedNotice'));
  } catch {
    emit('notify', '剪贴板复制失败');
  }
}

function onHorizontalWheelScroll(e: WheelEvent) {
  const el = e.currentTarget as HTMLElement | null;
  if (!el || el.scrollWidth <= el.clientWidth) return;
  if (Math.abs(e.deltaY) > Math.abs(e.deltaX) && e.deltaY !== 0) {
    e.preventDefault();
    el.scrollLeft += e.deltaY;
  }
}

// ==================== Shared Project Helpers (Matches Left Database Explorer) ====================
const defaultProjectId = computed(() => props.projects[0]?.id || 'default-project');

function connectionProjectId(conn: ConnectionProfile): string {
  return conn.projectId || defaultProjectId.value;
}

function connectionsInProject(pid: string): ConnectionProfile[] {
  return props.connections.filter((c) => connectionProjectId(c) === pid);
}

function projectNameById(pid?: string | null): string {
  if (!pid) return props.projects[0]?.name ?? '默认项目';
  return props.projects.find((p) => p.id === pid)?.name ?? '默认项目';
}

function projectColorById(pid?: string | null): string {
  if (!pid) return props.projects[0]?.color || 'var(--accent)';
  return props.projects.find((p) => p.id === pid)?.color || 'var(--accent)';
}

// ==================== Unified Output Console (Default: ALL Projects) ====================
const logProjectFilter = ref<string>('ALL');
const logProfileFilter = ref<string>('ALL');
const logOnlyErrors = ref<boolean>(false);
const logSearchText = ref<string>('');
const quickConsoleSql = ref<string>('');
const consoleScrollEl = ref<HTMLDivElement | null>(null);

const logScopedEntries = computed(() =>
  props.consoleLogs.filter((entry) => {
    const pid = entry.projectId || defaultProjectId.value;
    if (logProjectFilter.value !== 'ALL' && pid !== logProjectFilter.value) return false;
    if (logProfileFilter.value !== 'ALL' && entry.profileId !== logProfileFilter.value) return false;
    return true;
  }),
);

const errorLogCount = computed(
  () => logScopedEntries.value.filter((l) => l.level === 'error').length,
);

const filteredConsoleLogs = computed(() => {
  const q = logSearchText.value.trim().toLowerCase();
  return logScopedEntries.value.filter((entry) => {
    if (logOnlyErrors.value && entry.level !== 'error') return false;
    if (q) {
      const hay = `${projectNameById(entry.projectId)} ${entry.profileName} ${entry.dbName ?? ''} ${entry.summary} ${entry.detail ?? ''}`.toLowerCase();
      if (!hay.includes(q)) return false;
    }
    return true;
  });
});

watch([logProjectFilter, () => props.projects, () => props.connections], () => {
  if (
    logProjectFilter.value !== 'ALL' &&
    !props.projects.some((p) => p.id === logProjectFilter.value)
  ) {
    logProjectFilter.value = 'ALL';
  }
  if (logProfileFilter.value !== 'ALL') {
    const visibleConns =
      logProjectFilter.value === 'ALL'
        ? props.connections
        : connectionsInProject(logProjectFilter.value);
    if (!visibleConns.some((c) => c.id === logProfileFilter.value)) {
      logProfileFilter.value = 'ALL';
    }
  }
});

function formatLogEntryAsText(entry: ConsoleLogEntry): string {
  const proj = projectNameById(entry.projectId);
  const scope = entry.dbName ? `${entry.profileName}/${entry.dbName}` : entry.profileName;
  const tag = entry.level.toUpperCase();
  const head = `[${entry.timestamp}] [${tag}] [${proj} · ${scope}] ${entry.summary}`;
  return entry.detail ? `${head}\n${entry.detail}` : head;
}

function copyAllFilteredLogs() {
  const text = filteredConsoleLogs.value.map(formatLogEntryAsText).join('\n\n');
  if (!text) return;
  void copyText(text);
}

function copyOnlyErrorLogs() {
  const text = logScopedEntries.value
    .filter((e) => e.level === 'error')
    .map(formatLogEntryAsText)
    .join('\n\n');
  if (!text) {
    emit('notify', '当前筛选范围内暂无报错日志');
    return;
  }
  void copyText(text);
}

function submitQuickConsoleSql() {
  const sql = quickConsoleSql.value.trim();
  if (!sql) return;
  emit('run-console-sql', sql);
  quickConsoleSql.value = '';
}

watch(
  () => props.consoleLogs.length,
  async () => {
    if (props.activeServiceTab === 'output') {
      await nextTick();
      if (consoleScrollEl.value) {
        consoleScrollEl.value.scrollTop = consoleScrollEl.value.scrollHeight;
      }
    }
  },
);

// ==================== History Filtering (Consistent with Left Project Tree) ====================
const historyProjectFilter = ref<string>('ALL');
const historyProfileFilter = ref<string>('ALL');
const historyStatusFilter = ref<'ALL' | 'OK' | 'ERR'>('ALL');
const historySearchText = ref<string>('');

function resolveHistoryProjectId(entry: QueryHistoryEntry): string {
  if (entry.projectId) return entry.projectId;
  const conn = props.connections.find((c) => c.id === entry.profileId);
  return conn?.projectId || defaultProjectId.value;
}

const legacyHistoryProfiles = computed(() => {
  const activeIds = new Set(props.connections.map((c) => c.id));
  const map = new Map<string, string>();
  for (const h of props.historyEntries) {
    if (activeIds.has(h.profileId)) continue;
    const pid = resolveHistoryProjectId(h);
    if (historyProjectFilter.value === 'ALL' || pid === historyProjectFilter.value) {
      if (!map.has(h.profileId)) {
        map.set(h.profileId, `${h.profileName} (已移除)`);
      }
    }
  }
  return [...map.entries()].map(([id, name]) => ({ id, name }));
});

watch([historyProjectFilter, () => props.projects, () => props.connections], () => {
  if (
    historyProjectFilter.value !== 'ALL' &&
    !props.projects.some((p) => p.id === historyProjectFilter.value)
  ) {
    historyProjectFilter.value = 'ALL';
  }
  if (historyProfileFilter.value !== 'ALL') {
    const activeVisible =
      historyProjectFilter.value === 'ALL'
        ? props.connections
        : connectionsInProject(historyProjectFilter.value);
    const hasMatch =
      activeVisible.some((c) => c.id === historyProfileFilter.value) ||
      legacyHistoryProfiles.value.some((c) => c.id === historyProfileFilter.value);
    if (!hasMatch) {
      historyProfileFilter.value = 'ALL';
    }
  }
});

const filteredHistoryEntries = computed(() => {
  const q = historySearchText.value.trim().toLowerCase();
  return props.historyEntries.filter((item) => {
    const pid = resolveHistoryProjectId(item);
    if (historyProjectFilter.value !== 'ALL' && pid !== historyProjectFilter.value) return false;
    if (historyProfileFilter.value !== 'ALL' && item.profileId !== historyProfileFilter.value) {
      return false;
    }
    if (historyStatusFilter.value === 'OK' && !item.success) return false;
    if (historyStatusFilter.value === 'ERR' && item.success) return false;
    if (q) {
      const hay = `${projectNameById(pid)} ${item.profileName} ${item.database ?? ''} ${item.sql} ${item.errorMessage ?? ''}`.toLowerCase();
      if (!hay.includes(q)) return false;
    }
    return true;
  });
});

const astWarnings = computed(
  () => props.astAnalysis?.statements.flatMap((s) => s.warnings) ?? [],
);

// ==================== 2D Virtualized Grid Layout State ====================
const gridScrollRef = ref<HTMLDivElement | null>(null);
const scrollTop = ref(0);
const scrollLeft = ref(0);
const viewportHeight = ref(360);
const viewportWidth = ref(1100);

const VIRTUAL_ROW_HEIGHT = 27;
const COL_VIRTUAL_THRESHOLD = 20;

let scrollRafId: number | null = null;
let resizeObserver: ResizeObserver | null = null;

function syncViewportDimensions() {
  const el = gridScrollRef.value;
  if (!el) return;
  viewportHeight.value = Math.max(160, el.clientHeight);
  viewportWidth.value = Math.max(320, el.clientWidth);
}

function onGridScroll() {
  if (scrollRafId !== null) return;
  scrollRafId = window.requestAnimationFrame(() => {
    scrollRafId = null;
    const el = gridScrollRef.value;
    if (!el) return;
    scrollTop.value = el.scrollTop;
    scrollLeft.value = el.scrollLeft;
  });
}

onMounted(() => {
  syncViewportDimensions();
  if (typeof ResizeObserver !== 'undefined') {
    resizeObserver = new ResizeObserver(() => {
      syncViewportDimensions();
    });
    if (gridScrollRef.value) {
      resizeObserver.observe(gridScrollRef.value);
    }
  }
});

watch(gridScrollRef, (el, prev) => {
  if (prev && resizeObserver) resizeObserver.unobserve(prev);
  if (el) {
    if (resizeObserver) resizeObserver.observe(el);
    syncViewportDimensions();
  }
});

onBeforeUnmount(() => {
  if (scrollRafId !== null) {
    window.cancelAnimationFrame(scrollRafId);
    scrollRafId = null;
  }
  resizeObserver?.disconnect();
  resizeObserver = null;
});

watch(
  () => [props.activeResultTabId, filterText.value],
  () => {
    scrollTop.value = 0;
    scrollLeft.value = 0;
    if (gridScrollRef.value) {
      gridScrollRef.value.scrollTop = 0;
      gridScrollRef.value.scrollLeft = 0;
    }
  },
);

const currentColWidthOverrides = computed(() => {
  const res = currentResult.value;
  if (!res) return undefined;
  const prefix = `${res.id}:`;
  const out: Record<string, number> = {};
  let hasAny = false;
  for (const [k, v] of Object.entries(colWidths)) {
    if (k.startsWith(prefix)) {
      out[k.slice(prefix.length)] = v;
      hasAny = true;
    }
  }
  return hasAny ? out : undefined;
});

const pinnedWidthsPx = computed(() =>
  computeGridColumnWidths(
    currentPinnedCols.value,
    currentResult.value?.rows ?? [],
    currentColWidthOverrides.value,
  ),
);

const pinnedColItems = computed(() => {
  let left = 44;
  return currentPinnedCols.value.map((col, idx) => {
    const width = pinnedWidthsPx.value[idx] ?? 130;
    const item = { col, width, left };
    left += width;
    return item;
  });
});

const pinnedTotalWidth = computed(() =>
  pinnedColItems.value.reduce((sum, it) => sum + it.width, 0),
);

const scrollableColWidthsPx = computed(() =>
  computeGridColumnWidths(
    scrollableColumns.value,
    currentResult.value?.rows ?? [],
    currentColWidthOverrides.value,
  ),
);

const rowWindow = computed(() =>
  computeFixedVirtualRange(
    scrollTop.value,
    viewportHeight.value,
    VIRTUAL_ROW_HEIGHT,
    displayedRows.value.length,
    8,
  ),
);

const renderedRows = computed(() =>
  displayedRows.value.slice(rowWindow.value.start, rowWindow.value.end),
);

const colWindow = computed(() => {
  const widths = scrollableColWidthsPx.value;
  if (scrollableColumns.value.length <= COL_VIRTUAL_THRESHOLD) {
    const totalSize = widths.reduce((sum, w) => sum + w, 0);
    return { start: 0, end: widths.length, beforeSize: 0, afterSize: 0, totalSize };
  }
  return computeVariableVirtualRange(
    scrollLeft.value,
    Math.max(320, viewportWidth.value - 44 - pinnedTotalWidth.value),
    widths,
    5,
  );
});

const renderedColumns = computed(() => {
  const start = colWindow.value.start;
  const cols = scrollableColumns.value.slice(start, colWindow.value.end);
  const widths = scrollableColWidthsPx.value;
  return cols.map((col, idx) => ({
    col,
    width: widths[start + idx] ?? 130,
  }));
});

const renderedColSpan = computed(
  () =>
    1 +
    pinnedColItems.value.length +
    (colWindow.value.beforeSize > 0 ? 1 : 0) +
    renderedColumns.value.length +
    (colWindow.value.afterSize > 0 ? 1 : 0),
);

// ==================== Quick Chart View State & Geometry (Lazy-Evaluated) ====================
const CHART_PALETTE = [
  '#3574f0',
  '#5fad65',
  '#e0a82e',
  '#db5860',
  '#9c6ade',
  '#2aa198',
  '#f28b50',
  '#56b6c2',
];

const chartDetected = computed(() => {
  const res = currentResult.value;
  if (!chartMode.value || !res) {
    return {
      labelCandidates: [] as string[],
      numericColumns: [] as string[],
      defaultLabelCol: '',
      defaultValueCol: '',
    };
  }
  return detectChartColumns(
    res.columns,
    displayedRows.value.map((r) => r.row),
  );
});

watch(
  () => [chartMode.value, props.activeResultTabId, currentResult.value?.columns.join('|')],
  ([enabled]) => {
    if (!enabled) return;
    const det = chartDetected.value;
    chartLabelCol.value = det.defaultLabelCol;
    chartValueCol.value = det.defaultValueCol;
  },
  { immediate: true },
);

const chartDataset = computed(() => {
  if (!chartMode.value) {
    return { points: [], min: 0, max: 0, sum: 0, totalRows: 0 };
  }
  return buildChartDataFromRows(
    displayedRows.value.map((r) => r.row),
    chartLabelCol.value || chartDetected.value.defaultLabelCol,
    chartValueCol.value || chartDetected.value.defaultValueCol,
    24,
  );
});

const chartBars = computed(() => {
  if (!chartMode.value) return [];
  const pts = chartDataset.value.points;
  if (pts.length === 0) return [];
  const W = 680;
  const H = 260;
  const padL = 48;
  const padR = 16;
  const padT = 22;
  const padB = 46;
  const plotW = W - padL - padR;
  const plotH = H - padT - padB;
  const maxVal = Math.max(1, ...pts.map((p) => Math.abs(p.value)));
  const step = plotW / pts.length;
  const bw = Math.max(8, Math.min(44, step * 0.65));
  return pts.map((p, idx) => {
    const h = (Math.max(0, p.value) / maxVal) * plotH;
    const x = padL + idx * step + (step - bw) / 2;
    const y = padT + (plotH - h);
    return {
      ...p,
      x,
      y,
      w: bw,
      h: Math.max(2, h),
      cx: x + bw / 2,
      color: CHART_PALETTE[idx % CHART_PALETTE.length],
      shortLabel: p.label.length > 10 ? `${p.label.slice(0, 9)}…` : p.label,
    };
  });
});

const chartLinePointsAttr = computed(() => {
  if (!chartMode.value) return '';
  const pts = chartDataset.value.points;
  if (pts.length === 0) return '';
  const W = 680;
  const H = 260;
  const padL = 48;
  const padR = 24;
  const padT = 22;
  const padB = 46;
  const plotW = W - padL - padR;
  const plotH = H - padT - padB;
  const maxVal = Math.max(chartDataset.value.max, 1);
  const minVal = Math.min(chartDataset.value.min, 0);
  const span = Math.max(1e-6, maxVal - minVal);
  return pts
    .map((p, idx) => {
      const x = pts.length === 1 ? padL + plotW / 2 : padL + (idx / (pts.length - 1)) * plotW;
      const y = padT + plotH - ((p.value - minVal) / span) * plotH;
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(' ');
});

const chartLineNodes = computed(() => {
  if (!chartMode.value) return [];
  const raw = chartLinePointsAttr.value;
  if (!raw) return [];
  return raw.split(' ').map((pair, idx) => {
    const [x, y] = pair.split(',').map(Number);
    const pt = chartDataset.value.points[idx];
    return { ...pt, x, y, shortLabel: pt.label.length > 10 ? `${pt.label.slice(0, 9)}…` : pt.label };
  });
});

const chartPieSlices = computed(() => {
  if (!chartMode.value) return [];
  const positives = chartDataset.value.points.filter((p) => p.value > 0);
  const total = positives.reduce((acc, p) => acc + p.value, 0);
  if (total <= 0) return [];
  const cx = 140;
  const cy = 140;
  const r = 108;
  let angle = -Math.PI / 2;
  return positives.map((p, idx) => {
    const frac = p.value / total;
    const sweep = frac * Math.PI * 2;
    const x1 = cx + r * Math.cos(angle);
    const y1 = cy + r * Math.sin(angle);
    const nextAngle = angle + sweep;
    const x2 = cx + r * Math.cos(nextAngle);
    const y2 = cy + r * Math.sin(nextAngle);
    const largeArc = sweep > Math.PI ? 1 : 0;
    const isFull = positives.length === 1 || frac >= 0.9999;
    const d = isFull
      ? ''
      : `M ${cx} ${cy} L ${x1.toFixed(2)} ${y1.toFixed(2)} A ${r} ${r} 0 ${largeArc} 1 ${x2.toFixed(2)} ${y2.toFixed(2)} Z`;
    angle = nextAngle;
    return {
      ...p,
      pct: (frac * 100).toFixed(1),
      color: CHART_PALETTE[idx % CHART_PALETTE.length],
      d,
      isFull,
    };
  });
});

function toggleChartMode() {
  chartMode.value = !chartMode.value;
  if (chartMode.value) {
    transposeMode.value = false;
  }
}

function exportQuickChartSvg() {
  const el = chartSvgRef.value;
  if (!el) return;
  const serialized = new XMLSerializer().serializeToString(el);
  downloadTextFile(
    `${currentResult.value?.title || 'chart'}-${chartType.value}.svg`,
    serialized,
    'image/svg+xml;charset=utf-8',
  );
  emit('notify', '已导出当前结果图表 (.svg)');
}
</script>

<template>
  <div
    class="services-panel"
    style="height: 100%"
    @click="
      cellCtxMenu.visible = false;
      showColPicker = false;
      showExportMenu = false;
    "
  >
    <div class="services-tabs">
      <div class="services-tab-list">
        <button
          class="services-tab"
          :class="{ active: activeServiceTab === 'result' }"
          @click="emit('update:activeServiceTab', 'result')"
        >
          {{ t('tabResult') }}
          <span v-if="currentResult" style="opacity: 0.7">({{ displayedRows.length }})</span>
          <span v-if="pendingStats.total > 0" class="pill pill-test" style="margin-left: 4px">
            {{ pendingStats.total }} 待提交
          </span>
        </button>

        <button
          class="services-tab"
          :class="{ active: activeServiceTab === 'output' }"
          @click="emit('update:activeServiceTab', 'output')"
        >
          {{ t('tabOutput') }} ({{ consoleLogs.length }})
          <span v-if="errorLogCount > 0" class="pill pill-prod" style="margin-left: 4px">
            ✕ {{ errorLogCount }}
          </span>
        </button>

        <button
          class="services-tab"
          :class="{ active: activeServiceTab === 'ddl' }"
          @click="emit('update:activeServiceTab', 'ddl')"
        >
          {{ t('tabDdl') }}
        </button>

        <button
          class="services-tab"
          :class="{ active: activeServiceTab === 'history' }"
          @click="emit('update:activeServiceTab', 'history')"
        >
          {{ t('tabHistory') }} ({{ historyEntries.length }})
        </button>

        <button
          class="services-tab"
          :class="{ active: activeServiceTab === 'problems' }"
          @click="emit('update:activeServiceTab', 'problems')"
        >
          {{ t('tabProblems') }}
          <span v-if="astWarnings.length > 0" class="pill pill-test" style="margin-left: 4px">
            {{ astWarnings.length }}
          </span>
        </button>
      </div>

      <div class="services-tabs-meta">
        <span
          v-if="activeServiceTab === 'result' && resultTabs.length > 0"
          style="font-size: 11px; color: var(--text-muted)"
        >
          {{ resultTabs.length }} 个结果页
        </span>
        <button
          v-if="activeServiceTab === 'result' && resultTabs.length > 1 && currentResult"
          class="icon-btn compact"
          title="关闭其他未固定的结果页"
          @click="emit('close-other-result-tabs', currentResult.id)"
        >
          清其他
        </button>
      </div>
    </div>

    <div
      v-if="activeServiceTab === 'result' && resultTabs.length >= 1"
      class="result-tabs-strip"
    >
      <div class="result-tabs-scroll" @wheel="onHorizontalWheelScroll">
        <button
          v-for="rt in resultTabs"
          :key="rt.id"
          class="result-tab-pill"
          :class="{ active: rt.id === activeResultTabId }"
          :title="`${rt.title} · ${rt.rows.length} 行`"
          @click="emit('update:activeResultTabId', rt.id)"
        >
          <span
            class="pin-badge"
            :class="{ pinned: rt.pinned }"
            :title="rt.pinned ? '取消固定结果页' : '固定结果页（不被后续查询覆盖）'"
            @click.stop="emit('toggle-pin-result-tab', rt.id)"
          >
            <IdeIcon name="pin" :size="11" />
          </span>
          <span class="result-tab-title">{{ rt.title }}</span>
          <span class="result-tab-count">{{ rt.rows.length }}</span>
          <span
            class="result-tab-close"
            title="关闭该结果页"
            @click.stop="emit('close-result-tab', rt.id)"
          >
            ×
          </span>
        </button>
      </div>
    </div>

    <div v-if="executing" class="grid-loading-bar" />

    <div class="services-body">
      <!-- ==================== 1. Editable Result Grid Tab ==================== -->
      <template v-if="activeServiceTab === 'result'">
        <div
          v-if="executing && (!currentResult || currentResult.columns.length === 0)"
          class="empty-state grid-loading-panel"
        >
          <span class="pill pill-test">⏳ 正在执行查询...</span>
          <span>正在连接数据库执行并接收结果数据，请稍候...</span>
        </div>
        <div v-else-if="!currentResult || currentResult.columns.length === 0" class="empty-state">
          <div v-if="!currentResult">{{ t('emptyGridHint') }}</div>
          <div v-else-if="currentResult.summary?.isDmlOrDdl">
            <span class="pill pill-dev">执行成功</span>
            <span style="margin-left: 8px">
              {{ t('affectedRows', { count: currentResult.summary.affectedRows ?? 0 }) }} ·
              {{ currentResult.summary.durationMs }} ms
            </span>
          </div>
          <div v-else>
            查询完成，返回 0 行（{{ currentResult.summary?.durationMs ?? 0 }} ms）
          </div>
        </div>

        <template v-else>
          <!-- Pending Changes Action Bar (Appears when cells/rows modified) -->
          <div v-if="pendingStats.total > 0" class="grid-changes-bar">
            <div style="display: flex; align-items: center; gap: 8px; flex-wrap: wrap">
              <span class="pill pill-test">待提交变更</span>
              <span>
                <template v-if="pendingStats.modifiedRows > 0">更新 {{ pendingStats.modifiedRows }} 行 </template>
                <template v-if="pendingStats.insertedRows > 0">新增 {{ pendingStats.insertedRows }} 行 </template>
                <template v-if="pendingStats.deletedRows > 0">删除 {{ pendingStats.deletedRows }} 行 </template>
              </span>

              <span style="display: inline-flex; align-items: center; gap: 4px">
                <span style="color: var(--text-secondary)">目标表:</span>
                <select
                  v-if="!currentResult.sourceTable && currentResult.availableTables.length > 0 && activeDraft"
                  v-model="activeDraft.overrideTable"
                  class="field"
                  style="width: auto; padding: 2px 6px; font-size: 11px"
                >
                  <option value="">请选择目标表...</option>
                  <option v-for="tb in currentResult.availableTables" :key="tb" :value="tb">
                    {{ tb }}
                  </option>
                </select>
                <input
                  v-else-if="!currentResult.sourceTable && activeDraft"
                  v-model="activeDraft.overrideTable"
                  class="field"
                  style="width: 140px; padding: 2px 6px; font-size: 11px"
                  placeholder="输入目标表名"
                />
                <strong v-else style="font-family: var(--font-mono)">{{ effectiveTargetTable }}</strong>
              </span>

              <span v-if="currentResult.primaryKeys.length > 0" class="pill pill-dev">
                主键: {{ currentResult.primaryKeys.join(', ') }}
              </span>
              <span v-else class="pill pill-neutral">全字段匹配 WHERE</span>
            </div>

            <div style="display: flex; align-items: center; gap: 5px">
              <button class="btn btn-sm" @click="copyMutationsSql">
                <IdeIcon name="copy" :size="12" /> {{ t('copyChangesSql') }}
              </button>
              <button class="btn btn-sm" @click="revertCurrentDraft">↩ {{ t('revertChanges') }}</button>
              <button class="btn btn-sm btn-primary" @click="triggerPreviewMutations">
                <IdeIcon name="bolt" :size="12" /> {{ t('submitChanges') }}
              </button>
            </div>
          </div>

          <div class="grid-toolbar">
            <div style="display: flex; align-items: center; gap: 6px; flex-wrap: wrap">
              <button
                class="icon-btn compact"
                title="重新执行当前 SQL 刷新结果集"
                @click="emit('rerun-result', currentResult)"
              >
                <IdeIcon name="refresh" :size="12" /> 刷新
              </button>
              <input
                v-model="filterText"
                class="field"
                style="width: 175px; padding: 3px 7px"
                :placeholder="t('filterPlaceholder')"
              />
              <button
                class="icon-btn compact"
                :disabled="currentResult.readOnly"
                title="在结果集底部新增一行（生成 INSERT SQL）"
                @click="addNewRow"
              >
                <IdeIcon name="plus" :size="12" /> {{ t('addRow') }}
              </button>
              <button
                class="icon-btn compact"
                :disabled="currentResult.readOnly"
                title="标记/取消标记选中行删除（生成 DELETE SQL）"
                @click="toggleDeleteSelectedRow"
              >
                <IdeIcon name="trash" :size="12" /> {{ t('deleteRow') }}
              </button>
              <button
                class="icon-btn compact"
                :class="{ active: transposeMode }"
                @click="
                  transposeMode = !transposeMode;
                  if (transposeMode) {
                    chartMode = false;
                    visualExplainMode = false;
                  }
                "
              >
                ⇅ 表单/转置
              </button>
              <button
                v-if="isCurrentExplain"
                class="icon-btn compact"
                :class="{ active: visualExplainMode }"
                title="切换可视化 EXPLAIN 执行计划树与瓶颈诊断"
                @click="
                  visualExplainMode = !visualExplainMode;
                  if (visualExplainMode) {
                    chartMode = false;
                    transposeMode = false;
                  }
                "
              >
                🌳 执行计划图
              </button>
              <button
                class="icon-btn compact"
                :class="{ active: chartMode }"
                title="切换可视化统计图表（柱状 / 折线 / 饼图）"
                @click="
                  visualExplainMode = false;
                  toggleChartMode();
                "
              >
                📊 图表
              </button>
              <button
                class="icon-btn compact"
                :class="{ active: showValueViewer }"
                @click="showValueViewer = !showValueViewer"
              >
                👁 {{ t('valueViewer') }}
              </button>
              <button
                v-if="currentPinnedCols.length > 0"
                class="icon-btn compact active"
                title="取消全部左侧冻结列"
                @click="clearPinnedColumns"
              >
                📌 取消固定 ({{ currentPinnedCols.length }})
              </button>
              <span v-if="executing" class="pill pill-test">⏳ 正在执行新查询...</span>
              <span v-else-if="currentResult.summary" style="font-size: 11px; color: var(--text-secondary)">
                {{ currentResult.profileName }} · {{ currentResult.summary.durationMs }} ms ·
                {{ t('rowsCount', { count: displayedRows.length }) }}
                <template v-if="visibleColumns.length > 0"> · {{ visibleColumns.length }} 列</template>
                <template v-if="!currentResult.readOnly"> · 双击单元格可直接修改</template>
                <template v-else> · <IdeIcon name="lock" :size="11" /> 只读</template>
              </span>
            </div>

            <div style="display: flex; align-items: center; gap: 4px">
              <div class="grid-popover-wrap" @click.stop>
                <button
                  class="btn btn-sm"
                  :class="{ 'btn-primary': hiddenColsSet.size > 0 }"
                  title="显示 / 隐藏结果列（支持宽表快速筛选）"
                  @click="
                    showExportMenu = false;
                    showColPicker = !showColPicker;
                  "
                >
                  列 ({{ visibleColumns.length }}/{{ currentResult.columns.length }}) ▾
                </button>
                <div v-if="showColPicker" class="dropdown-menu align-right grid-col-popover">
                  <div style="padding: 4px 6px 6px; border-bottom: 1px solid var(--border-subtle)">
                    <input
                      v-model="colPickerSearch"
                      class="field"
                      style="padding: 3px 6px; font-size: 11px; margin-bottom: 5px"
                      placeholder="搜索列名..."
                    />
                    <div style="display: flex; gap: 4px">
                      <button class="icon-btn compact" @click="resetVisibleColumns">全显</button>
                      <button
                        v-if="currentResult.columns.length > 10"
                        class="icon-btn compact"
                        @click="showOnlyFirst10Columns"
                      >
                        仅前10列
                      </button>
                    </div>
                  </div>
                  <div style="max-height: 220px; overflow-y: auto; padding: 4px 0">
                    <label
                      v-for="c in filteredPickerColumns"
                      :key="c"
                      class="schema-check-row"
                      style="font-family: var(--font-mono); font-size: 11.5px"
                    >
                      <input
                        type="checkbox"
                        :checked="!hiddenColsSet.has(c)"
                        @change="toggleResultColumnVisible(c)"
                      />
                      <span style="overflow: hidden; text-overflow: ellipsis">{{ c }}</span>
                    </label>
                  </div>
                </div>
              </div>

              <button
                class="btn btn-sm"
                title="复制当前列为 SQL IN (...) 条件列表"
                @click="doCopyColumnInSql"
              >
                列 IN (...)
              </button>
              <button
                class="btn btn-sm"
                title="复制选中行为 JSON 对象"
                @click="doCopySelectedRowJson"
              >
                行 JSON
              </button>

              <div class="grid-popover-wrap" @click.stop>
                <button
                  class="btn btn-sm"
                  @click="
                    showColPicker = false;
                    showExportMenu = !showExportMenu;
                  "
                >
                  导出 / 复制 ▾
                </button>
                <div v-if="showExportMenu" class="dropdown-menu align-right" style="min-width: 225px">
                  <button
                    class="dropdown-item"
                    title="原生生成 .xlsx 工作簿；11位及以上长整数（如雪花ID/手机号）自动按文本写入，彻底避免科学计数法"
                    @click="showExportMenu = false; void doExportXlsx()"
                  >
                    <span>📊 导出为 Excel (.xlsx)</span>
                  </button>
                  <button class="dropdown-item" @click="showExportMenu = false; doCopyTsv()">
                    <span>复制为 Excel (TSV)</span>
                  </button>
                  <button class="dropdown-item" @click="showExportMenu = false; doExportCsv()">
                    <span>{{ t('exportCsv') }} (UTF-8)</span>
                  </button>
                  <button class="dropdown-item" @click="showExportMenu = false; doExportJson()">
                    <span>{{ t('exportJson') }}</span>
                  </button>
                  <div class="dropdown-sep" />
                  <button class="dropdown-item" @click="showExportMenu = false; doCopyMarkdown()">
                    <span>{{ t('copyMarkdown') }}</span>
                  </button>
                  <button class="dropdown-item" @click="showExportMenu = false; doCopyInsertSql()">
                    <span>{{ t('copyInsertSql') }}</span>
                  </button>
                  <button class="dropdown-item" @click="showExportMenu = false; doCopyUpdateSql()">
                    <span>复制为 UPDATE SQL</span>
                  </button>
                </div>
              </div>
            </div>
          </div>

          <!-- Server-Side WHERE / ORDER BY Toolbar (Single-Table Result Tabs) -->
          <div v-if="currentResult.sourceTable && !isCurrentExplain" class="grid-server-filter-bar">
            <span class="server-clause-tag">WHERE</span>
            <input
              v-model="serverWhereInput"
              class="field server-clause-input"
              placeholder="输入服务端过滤条件，如 status = 'ACTIVE' AND amount > 100 (Enter 执行)"
              @keydown.enter="submitServerQuery(0)"
            />
            <span class="server-clause-tag">ORDER BY</span>
            <input
              v-model="serverOrderByInput"
              class="field server-order-input"
              placeholder="排序字段，如 id DESC"
              @keydown.enter="submitServerQuery(0)"
            />
            <button class="btn btn-sm btn-primary" :disabled="executing" @click="submitServerQuery(0)">
              筛选执行
            </button>
            <button
              v-if="serverWhereInput || serverOrderByInput || (currentResult.pageIndex ?? 0) > 0"
              class="btn btn-sm"
              :disabled="executing"
              @click="resetServerQuery"
            >
              重置
            </button>
            <button
              class="btn btn-sm"
              title="打开当前表的数据内容对比与同步 SQL"
              @click="emit('open-data-diff', currentResult)"
            >
              🧬 数据对比
            </button>
          </div>

          <!-- Multi-Row Selection Bulk Actions Bar -->
          <div v-if="selectedRowIndices.length > 0" class="grid-bulk-bar">
            <span class="pill pill-accent">已选中 {{ selectedRowIndices.length }} 行</span>
            <button class="btn btn-sm" @click="copySelectedRowsAsTsv">复制选中 (TSV)</button>
            <button class="btn btn-sm" @click="copySelectedRowsAsInsert">生成 INSERT SQL</button>
            <button
              v-if="!currentResult.readOnly"
              class="btn btn-sm"
              @click="duplicateRowsByIndices(selectedRowIndices)"
            >
              📋 克隆为新增行
            </button>
            <button
              v-if="!currentResult.readOnly"
              class="btn btn-sm"
              style="color: var(--prod)"
              @click="markSelectedRowsDeleted"
            >
              标记删除
            </button>
            <button class="btn btn-sm" @click="selectedRowIndices = []">清空选择</button>
          </div>

          <div class="grid-split">
            <div ref="gridScrollRef" class="grid-scroll" @scroll.passive="onGridScroll">
              <!-- Visual EXPLAIN Execution Plan View -->
              <div v-if="visualExplainMode && visualExplainPlan" class="explain-visual-pane">
                <div class="explain-summary-header">
                  <div style="display: flex; align-items: center; gap: 8px; flex-wrap: wrap">
                    <span
                      class="pill"
                      :class="
                        visualExplainPlan.health === 'critical'
                          ? 'pill-prod'
                          : visualExplainPlan.health === 'warn'
                            ? 'pill-test'
                            : 'pill-dev'
                      "
                    >
                      {{
                        visualExplainPlan.health === 'critical'
                          ? '需要优化'
                          : visualExplainPlan.health === 'warn'
                            ? '存在瓶颈风险'
                            : '执行计划良好'
                      }}
                    </span>
                    <strong>{{ visualExplainPlan.headline }}</strong>
                    <span style="font-size: 11px; color: var(--text-secondary)">
                      引擎: {{ visualExplainPlan.engineFamily.toUpperCase() }} · 预估扫描行数:
                      {{ visualExplainPlan.totalEstimatedRows.toLocaleString() }}
                    </span>
                  </div>
                  <button class="btn btn-sm" @click="visualExplainMode = false">查看原始表格</button>
                </div>

                <div v-if="visualExplainPlan.summaryTips.length > 0" class="explain-tips-banner">
                  <div v-for="(tip, tIdx) in visualExplainPlan.summaryTips" :key="tIdx">
                    • {{ tip }}
                  </div>
                </div>

                <div class="explain-node-list">
                  <div
                    v-for="node in visualExplainPlan.nodes"
                    :key="node.id"
                    class="explain-node-wrap"
                    :style="{ paddingLeft: `${Math.min(node.depth, 8) * 22}px` }"
                  >
                    <div class="explain-node-card" :class="`sev-${node.severity}`">
                      <div class="explain-node-head">
                        <span class="explain-node-title">{{ node.title }}</span>
                        <span v-if="node.tableName" class="explain-meta-tag">{{ node.tableName }}</span>
                        <span v-if="node.accessType" class="explain-meta-tag">type: {{ node.accessType }}</span>
                        <span v-if="node.indexUsed" class="explain-meta-tag tag-ok">key: {{ node.indexUsed }}</span>
                        <span v-if="node.estimatedRows != null" class="explain-meta-tag">rows: {{ node.estimatedRows }}</span>
                        <span v-if="node.costLabel" class="explain-meta-tag">{{ node.costLabel }}</span>
                        <span v-if="node.filteredPct != null" class="explain-meta-tag">filtered: {{ node.filteredPct }}%</span>
                      </div>
                      <div v-if="node.subtitle" class="explain-node-sub">{{ node.subtitle }}</div>
                      <div v-if="node.extra" class="explain-node-extra">Extra: {{ node.extra }}</div>
                      <div v-if="node.tips.length > 0" class="explain-node-tips">
                        <span v-for="(tMsg, idx) in node.tips" :key="idx">⚠ {{ tMsg }}</span>
                      </div>
                    </div>
                  </div>
                </div>
              </div>

              <!-- Quick Visualization Chart Mode -->
              <div v-else-if="chartMode" class="chart-workspace">
                <div class="chart-toolbar">
                  <div style="display: flex; align-items: center; gap: 6px; flex-wrap: wrap">
                    <button
                      class="btn btn-sm"
                      :class="{ 'btn-primary': chartType === 'bar' }"
                      @click="chartType = 'bar'"
                    >
                      柱状图
                    </button>
                    <button
                      class="btn btn-sm"
                      :class="{ 'btn-primary': chartType === 'line' }"
                      @click="chartType = 'line'"
                    >
                      折线图
                    </button>
                    <button
                      class="btn btn-sm"
                      :class="{ 'btn-primary': chartType === 'pie' }"
                      @click="chartType = 'pie'"
                    >
                      饼图
                    </button>
                    <span style="font-size: 11px; color: var(--text-secondary); margin-left: 6px">维度列:</span>
                    <select v-model="chartLabelCol" class="field" style="width: 130px; padding: 2px 6px">
                      <option v-for="c in currentResult.columns" :key="c" :value="c">{{ c }}</option>
                    </select>
                    <span style="font-size: 11px; color: var(--text-secondary)">数值列:</span>
                    <select v-model="chartValueCol" class="field" style="width: 130px; padding: 2px 6px">
                      <option
                        v-for="c in chartDetected.numericColumns.length > 0 ? chartDetected.numericColumns : currentResult.columns"
                        :key="c"
                        :value="c"
                      >
                        {{ c }}
                      </option>
                    </select>
                  </div>
                  <div style="display: flex; align-items: center; gap: 8px">
                    <span v-if="chartDataset.points.length > 0" style="font-size: 11px; color: var(--text-secondary)">
                      样本 {{ chartDataset.points.length }} 项 · 合计 {{ chartDataset.sum }} · 峰值 {{ chartDataset.max }}
                    </span>
                    <button
                      v-if="chartDataset.points.length > 0"
                      class="btn btn-sm"
                      @click="exportQuickChartSvg"
                    >
                      导出 SVG
                    </button>
                  </div>
                </div>

                <div
                  v-if="chartDataset.points.length === 0"
                  style="padding: 28px; text-align: center; color: var(--text-secondary); font-size: 12px"
                >
                  当前结果集缺少可绘制的数值列（可从右上选择数值字段或切回数据表视图）。
                </div>

                <div v-else class="chart-body">
                  <svg
                    v-if="chartType === 'bar'"
                    ref="chartSvgRef"
                    class="chart-svg"
                    width="680"
                    height="260"
                    viewBox="0 0 680 260"
                    xmlns="http://www.w3.org/2000/svg"
                  >
                    <line x1="48" y1="214" x2="664" y2="214" stroke="var(--border, #4a4d54)" stroke-width="1" />
                    <g v-for="(bar, bIdx) in chartBars" :key="bIdx">
                      <rect
                        :x="bar.x"
                        :y="bar.y"
                        :width="bar.w"
                        :height="bar.h"
                        :fill="bar.color"
                        rx="2"
                      >
                        <title>{{ bar.label }}: {{ bar.value }}</title>
                      </rect>
                      <text
                        :x="bar.cx"
                        :y="bar.y - 4"
                        text-anchor="middle"
                        fill="var(--text-secondary, #868a91)"
                        font-size="10"
                      >
                        {{ bar.value }}
                      </text>
                      <text
                        :x="bar.cx"
                        y="232"
                        text-anchor="middle"
                        fill="var(--text-secondary, #868a91)"
                        font-size="10"
                      >
                        {{ bar.shortLabel }}
                      </text>
                    </g>
                  </svg>

                  <svg
                    v-else-if="chartType === 'line'"
                    ref="chartSvgRef"
                    class="chart-svg"
                    width="680"
                    height="260"
                    viewBox="0 0 680 260"
                    xmlns="http://www.w3.org/2000/svg"
                  >
                    <line x1="48" y1="214" x2="656" y2="214" stroke="var(--border, #4a4d54)" stroke-width="1" />
                    <polyline
                      :points="chartLinePointsAttr"
                      fill="none"
                      stroke="#3574f0"
                      stroke-width="2.2"
                    />
                    <g v-for="(pt, pIdx) in chartLineNodes" :key="pIdx">
                      <circle :cx="pt.x" :cy="pt.y" r="4" fill="#5fad65" stroke="#ffffff" stroke-width="1">
                        <title>{{ pt.label }}: {{ pt.value }}</title>
                      </circle>
                      <text
                        v-if="pIdx % Math.max(1, Math.ceil(chartLineNodes.length / 8)) === 0"
                        :x="pt.x"
                        y="234"
                        text-anchor="middle"
                        fill="var(--text-secondary, #868a91)"
                        font-size="10"
                      >
                        {{ pt.shortLabel }}
                      </text>
                    </g>
                  </svg>

                  <template v-else>
                    <svg
                      ref="chartSvgRef"
                      class="chart-svg"
                      width="280"
                      height="280"
                      viewBox="0 0 280 280"
                      xmlns="http://www.w3.org/2000/svg"
                    >
                      <g v-for="(sl, sIdx) in chartPieSlices" :key="sIdx">
                        <circle
                          v-if="sl.isFull"
                          cx="140"
                          cy="140"
                          r="108"
                          :fill="sl.color"
                        >
                          <title>{{ sl.label }}: {{ sl.value }} ({{ sl.pct }}%)</title>
                        </circle>
                        <path v-else :d="sl.d" :fill="sl.color" stroke="var(--bg-chrome, #2b2d30)" stroke-width="1.5">
                          <title>{{ sl.label }}: {{ sl.value }} ({{ sl.pct }}%)</title>
                        </path>
                      </g>
                    </svg>
                    <div class="chart-legend">
                      <div
                        v-for="(sl, sIdx) in chartPieSlices"
                        :key="sIdx"
                        style="display: flex; align-items: center; gap: 6px"
                      >
                        <span
                          style="width: 10px; height: 10px; border-radius: 2px; flex-shrink: 0"
                          :style="{ background: sl.color }"
                        />
                        <span style="flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap">
                          {{ sl.label }}
                        </span>
                        <span style="font-family: var(--font-mono); color: var(--text-secondary)">
                          {{ sl.value }} ({{ sl.pct }}%)
                        </span>
                      </div>
                    </div>
                  </template>
                </div>
              </div>

              <!-- Enhanced Record Form / Transpose View -->
              <div v-else-if="transposeMode && displayedRows[currentTransposeDisplayPos]" class="record-form-wrap">
                <div class="record-form-nav">
                  <div style="display: flex; align-items: center; gap: 6px">
                    <button
                      class="btn btn-sm"
                      :disabled="currentTransposeDisplayPos <= 0"
                      @click="stepTransposeRow(-1)"
                    >
                      ◀ 上一条
                    </button>
                    <span style="font-size: 11.5px; color: var(--text-secondary)">
                      第 {{ currentTransposeDisplayPos + 1 }} / {{ displayedRows.length }} 条 (原行 #{{ displayedRows[currentTransposeDisplayPos].origIdx + 1 }})
                    </span>
                    <button
                      class="btn btn-sm"
                      :disabled="currentTransposeDisplayPos >= displayedRows.length - 1"
                      @click="stepTransposeRow(1)"
                    >
                      下一条 ▶
                    </button>
                  </div>
                  <div style="display: flex; align-items: center; gap: 6px">
                    <input
                      v-model="recordFieldFilter"
                      class="field"
                      style="width: 170px; padding: 2px 6px; font-size: 11.5px"
                      placeholder="筛选字段名..."
                    />
                    <button
                      v-if="!currentResult.readOnly"
                      class="btn btn-sm"
                      @click="duplicateRowsByIndices([displayedRows[currentTransposeDisplayPos].origIdx])"
                    >
                      📋 复制为新增行
                    </button>
                    <button class="btn btn-sm" @click="transposeMode = false">返回表格</button>
                  </div>
                </div>
                <table class="dg-grid">
                  <thead>
                    <tr>
                      <th style="width: 240px">{{ t('columnsLabel') }}</th>
                      <th>当前记录字段值（双击可修改）</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr v-for="col in recordFormColumns" :key="col">
                      <td style="font-weight: 600">
                        <IdeIcon v-if="currentResult.primaryKeys.includes(col)" name="key" :size="11" style="margin-right: 4px" />
                        <span>{{ col }}</span>
                        <span
                          v-if="getFkForCol(col)"
                          class="fk-col-badge"
                          :title="`外键 -> ${getFkForCol(col)?.referencedTable}.${getFkForCol(col)?.referencedColumn}`"
                        >
                          FK
                        </span>
                      </td>
                      <td
                        :class="{
                          'selected-cell': selectedColName === col,
                          'cell-dirty': isCellModified(displayedRows[currentTransposeDisplayPos].origIdx, col),
                        }"
                        @click="selectCell(false, displayedRows[currentTransposeDisplayPos].origIdx, col)"
                        @dblclick="startEditCell(false, displayedRows[currentTransposeDisplayPos].origIdx, col)"
                      >
                        <div class="cell-value-wrap">
                          <span
                            v-if="
                              displayedRows[currentTransposeDisplayPos].row[col] === null ||
                              displayedRows[currentTransposeDisplayPos].row[col] === undefined
                            "
                            class="null-pill"
                          >
                            &lt;NULL&gt;
                          </span>
                          <span v-else>{{ formatCell(displayedRows[currentTransposeDisplayPos].row[col]) }}</span>
                          <button
                            v-if="
                              getFkForCol(col) &&
                              displayedRows[currentTransposeDisplayPos].row[col] !== null &&
                              displayedRows[currentTransposeDisplayPos].row[col] !== undefined
                            "
                            class="cell-fk-jump-btn"
                            :title="`跳转至 ${getFkForCol(col)?.referencedTable}`"
                            @click.stop="
                              emit(
                                'jump-foreign-key',
                                currentResult,
                                getFkForCol(col)!,
                                displayedRows[currentTransposeDisplayPos].row[col],
                              )
                            "
                          >
                            🔗
                          </button>
                        </div>
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>

              <!-- Standard 2D-Virtualized Editable Pacer Result Grid (with Left Pinned Columns) -->
              <table
                v-else
                class="dg-grid dg-grid-fixed"
                :style="{ width: `${44 + pinnedTotalWidth + colWindow.totalSize}px` }"
              >
                <colgroup>
                  <col style="width: 44px" />
                  <col v-for="pc in pinnedColItems" :key="`pcol-${pc.col}`" :style="{ width: `${pc.width}px` }" />
                  <col v-if="colWindow.beforeSize > 0" :style="{ width: `${colWindow.beforeSize}px` }" />
                  <col v-for="rc in renderedColumns" :key="`col-${rc.col}`" :style="{ width: `${rc.width}px` }" />
                  <col v-if="colWindow.afterSize > 0" :style="{ width: `${colWindow.afterSize}px` }" />
                </colgroup>
                <thead>
                  <tr>
                    <th
                      class="row-num sticky-rownum"
                      style="cursor: pointer"
                      title="点击全选 / 取消全选当前页行（支持 Shift/Ctrl 多选）"
                      @click="toggleSelectAllDisplayedRows"
                    >
                      #
                    </th>
                    <th
                      v-for="(pc, pIdx) in pinnedColItems"
                      :key="`ph-${pc.col}`"
                      class="pinned-col"
                      :class="{ 'pinned-col-last': pIdx === pinnedColItems.length - 1 }"
                      :style="{ left: `${pc.left}px` }"
                      @click="toggleSort(pc.col)"
                    >
                      <IdeIcon v-if="currentResult.primaryKeys.includes(pc.col)" name="key" :size="12" style="margin-right: 3px" />
                      <span>{{ pc.col }}</span>
                      <span
                        v-if="getFkForCol(pc.col)"
                        class="fk-col-badge"
                        :title="`外键 -> ${getFkForCol(pc.col)?.referencedTable}.${getFkForCol(pc.col)?.referencedColumn}`"
                      >
                        FK
                      </span>
                      <button
                        class="col-pin-btn active"
                        title="取消固定此列"
                        @click.stop="togglePinColumn(pc.col)"
                      >
                        📌
                      </button>
                      <span v-if="sortCol === pc.col" style="margin-left: 3px; color: var(--accent)">
                        {{ sortDir === 'asc' ? '▲' : '▼' }}
                      </span>
                      <span
                        class="col-resize-handle"
                        title="拖拽调整列宽（双击恢复自动宽度）"
                        @mousedown.stop.prevent="startColResize($event, pc.col)"
                        @dblclick.stop="resetColWidth(pc.col)"
                        @click.stop
                      />
                    </th>
                    <th v-if="colWindow.beforeSize > 0" class="virtual-spacer-cell" />
                    <th
                      v-for="rc in renderedColumns"
                      :key="rc.col"
                      @click="toggleSort(rc.col)"
                    >
                      <IdeIcon v-if="currentResult.primaryKeys.includes(rc.col)" name="key" :size="12" style="margin-right: 3px" />
                      <span>{{ rc.col }}</span>
                      <span
                        v-if="getFkForCol(rc.col)"
                        class="fk-col-badge"
                        :title="`外键 -> ${getFkForCol(rc.col)?.referencedTable}.${getFkForCol(rc.col)?.referencedColumn}`"
                      >
                        FK
                      </span>
                      <button
                        class="col-pin-btn"
                        title="固定此列到左侧（冻结列）"
                        @click.stop="togglePinColumn(rc.col)"
                      >
                        📌
                      </button>
                      <span v-if="sortCol === rc.col" style="margin-left: 3px; color: var(--accent)">
                        {{ sortDir === 'asc' ? '▲' : '▼' }}
                      </span>
                      <span
                        class="col-resize-handle"
                        title="拖拽调整列宽（双击恢复自动宽度）"
                        @mousedown.stop.prevent="startColResize($event, rc.col)"
                        @dblclick.stop="resetColWidth(rc.col)"
                        @click.stop
                      />
                    </th>
                    <th v-if="colWindow.afterSize > 0" class="virtual-spacer-cell" />
                  </tr>
                </thead>
                <tbody>
                  <tr
                    v-if="displayedRows.length === 0 && (activeDraft?.inserted.length ?? 0) === 0"
                    class="empty-data-row"
                  >
                    <td :colspan="renderedColSpan" class="empty-data-cell">
                      <template v-if="filterText.trim()">
                        <span>当前筛选「{{ filterText }}」下无匹配数据</span>
                        <button
                          class="btn btn-sm"
                          style="margin-left: 8px"
                          @click="filterText = ''"
                        >
                          清除筛选
                        </button>
                      </template>
                      <template v-else>
                        <span>查询结果为空（0 行）— 已保留当前查询字段结构</span>
                        <button
                          v-if="!currentResult.readOnly"
                          class="btn btn-sm"
                          style="margin-left: 10px"
                          @click="addNewRow"
                        >
                          <IdeIcon name="plus" :size="11" /> 新增一行
                        </button>
                      </template>
                    </td>
                  </tr>

                  <!-- Top Virtual Row Spacer -->
                  <tr v-if="rowWindow.beforeSize > 0" class="virtual-spacer-row">
                    <td :colspan="renderedColSpan" :style="{ height: `${rowWindow.beforeSize}px` }" />
                  </tr>

                  <!-- Virtualized Windowed DB Rows -->
                  <tr
                    v-for="(item, localIdx) in renderedRows"
                    :key="`orig-${item.origIdx}`"
                    class="dg-grid-row"
                    :class="{
                      'row-deleted': isRowDeleted(item.origIdx),
                      'row-selected': selectedRowSet.has(item.origIdx),
                    }"
                  >
                    <td
                      class="row-num sticky-rownum"
                      :class="{ 'row-selected': selectedRowSet.has(item.origIdx) }"
                      title="点击选择整行（Shift 连选，Ctrl/Cmd 多选）"
                      @click="onRowNumClick($event, item.origIdx, rowWindow.start + localIdx)"
                    >
                      {{ rowWindow.start + localIdx + 1 }}
                    </td>
                    <!-- Pinned Left Frozen Columns -->
                    <td
                      v-for="(pc, pIdx) in pinnedColItems"
                      :key="`pc-${pc.col}`"
                      class="pinned-col"
                      :class="{
                        'pinned-col-last': pIdx === pinnedColItems.length - 1,
                        'num-cell': isNumeric(item.row[pc.col]),
                        'selected-cell': !selectedIsInserted && selectedRowOrigIdx === item.origIdx && selectedColName === pc.col,
                        'cell-dirty': isCellModified(item.origIdx, pc.col),
                      }"
                      :style="{ left: `${pc.left}px` }"
                      @mouseenter="onCellMouseEnter($event, item.origIdx, pc.col)"
                      @click="selectCell(false, item.origIdx, pc.col)"
                      @dblclick="startEditCell(false, item.origIdx, pc.col)"
                      @contextmenu.prevent.stop="openCellContextMenu($event, item.origIdx, pc.col)"
                    >
                      <div
                        v-if="
                          editingCell &&
                          !editingCell.isInserted &&
                          editingCell.rowIdx === item.origIdx &&
                          editingCell.col === pc.col
                        "
                        class="cell-inline-editor"
                        @click.stop
                      >
                        <input
                          ref="inlineInputRef"
                          v-model="editingCell.draftText"
                          @keydown.enter.prevent="commitInlineEdit(false)"
                          @keydown.esc.prevent="cancelInlineEdit"
                        />
                        <button class="cell-inline-btn" title="设为 NULL" @click="commitInlineEdit(true)">
                          NULL
                        </button>
                        <button class="cell-inline-btn" title="确认修改 (Enter)" @click="commitInlineEdit(false)">
                          ✓
                        </button>
                      </div>
                      <div v-else class="cell-value-wrap">
                        <span v-if="item.row[pc.col] === null || item.row[pc.col] === undefined" class="null-pill">
                          &lt;NULL&gt;
                        </span>
                        <span v-else>{{ formatCell(item.row[pc.col]) }}</span>
                        <button
                          v-if="
                            getFkForCol(pc.col) &&
                            item.row[pc.col] !== null &&
                            item.row[pc.col] !== undefined
                          "
                          class="cell-fk-jump-btn"
                          :title="`跳转到 ${getFkForCol(pc.col)?.referencedTable}.${getFkForCol(pc.col)?.referencedColumn}`"
                          @click.stop="emit('jump-foreign-key', currentResult, getFkForCol(pc.col)!, item.row[pc.col])"
                        >
                          🔗
                        </button>
                      </div>
                    </td>
                    <td v-if="colWindow.beforeSize > 0" class="virtual-spacer-cell" />
                    <td
                      v-for="rc in renderedColumns"
                      :key="rc.col"
                      :class="{
                        'num-cell': isNumeric(item.row[rc.col]),
                        'selected-cell': !selectedIsInserted && selectedRowOrigIdx === item.origIdx && selectedColName === rc.col,
                        'cell-dirty': isCellModified(item.origIdx, rc.col),
                      }"
                      @mouseenter="onCellMouseEnter($event, item.origIdx, rc.col)"
                      @click="selectCell(false, item.origIdx, rc.col)"
                      @dblclick="startEditCell(false, item.origIdx, rc.col)"
                      @contextmenu.prevent.stop="openCellContextMenu($event, item.origIdx, rc.col)"
                    >
                      <div
                        v-if="
                          editingCell &&
                          !editingCell.isInserted &&
                          editingCell.rowIdx === item.origIdx &&
                          editingCell.col === rc.col
                        "
                        class="cell-inline-editor"
                        @click.stop
                      >
                        <input
                          ref="inlineInputRef"
                          v-model="editingCell.draftText"
                          @keydown.enter.prevent="commitInlineEdit(false)"
                          @keydown.esc.prevent="cancelInlineEdit"
                        />
                        <button class="cell-inline-btn" title="设为 NULL" @click="commitInlineEdit(true)">
                          NULL
                        </button>
                        <button class="cell-inline-btn" title="确认修改 (Enter)" @click="commitInlineEdit(false)">
                          ✓
                        </button>
                      </div>
                      <div v-else class="cell-value-wrap">
                        <span v-if="item.row[rc.col] === null || item.row[rc.col] === undefined" class="null-pill">
                          &lt;NULL&gt;
                        </span>
                        <span v-else>{{ formatCell(item.row[rc.col]) }}</span>
                        <button
                          v-if="
                            getFkForCol(rc.col) &&
                            item.row[rc.col] !== null &&
                            item.row[rc.col] !== undefined
                          "
                          class="cell-fk-jump-btn"
                          :title="`跳转到 ${getFkForCol(rc.col)?.referencedTable}.${getFkForCol(rc.col)?.referencedColumn}`"
                          @click.stop="emit('jump-foreign-key', currentResult, getFkForCol(rc.col)!, item.row[rc.col])"
                        >
                          🔗
                        </button>
                      </div>
                    </td>
                    <td v-if="colWindow.afterSize > 0" class="virtual-spacer-cell" />
                  </tr>

                  <!-- Bottom Virtual Row Spacer -->
                  <tr v-if="rowWindow.afterSize > 0" class="virtual-spacer-row">
                    <td :colspan="renderedColSpan" :style="{ height: `${rowWindow.afterSize}px` }" />
                  </tr>

                  <!-- Newly Inserted Rows -->
                  <tr
                    v-for="(insRow, insIdx) in activeDraft?.inserted ?? []"
                    :key="`ins-${insIdx}`"
                    class="dg-grid-row row-inserted"
                  >
                    <td class="row-num sticky-rownum" title="待新增行">+{{ insIdx + 1 }}</td>
                    <td
                      v-for="(pc, pIdx) in pinnedColItems"
                      :key="`pins-${pc.col}`"
                      class="pinned-col"
                      :class="{
                        'pinned-col-last': pIdx === pinnedColItems.length - 1,
                        'selected-cell': selectedIsInserted && selectedRowOrigIdx === insIdx && selectedColName === pc.col,
                      }"
                      :style="{ left: `${pc.left}px` }"
                      @click="selectCell(true, insIdx, pc.col)"
                      @dblclick="startEditCell(true, insIdx, pc.col)"
                    >
                      <div
                        v-if="
                          editingCell &&
                          editingCell.isInserted &&
                          editingCell.rowIdx === insIdx &&
                          editingCell.col === pc.col
                        "
                        class="cell-inline-editor"
                        @click.stop
                      >
                        <input
                          ref="inlineInputRef"
                          v-model="editingCell.draftText"
                          @keydown.enter.prevent="commitInlineEdit(false)"
                          @keydown.esc.prevent="cancelInlineEdit"
                        />
                        <button class="cell-inline-btn" @click="commitInlineEdit(true)">NULL</button>
                        <button class="cell-inline-btn" @click="commitInlineEdit(false)">✓</button>
                      </div>
                      <template v-else>
                        <span v-if="insRow[pc.col] === null" class="null-pill">&lt;NULL&gt;</span>
                        <span v-else-if="insRow[pc.col] === ''" style="color: var(--text-muted); font-style: italic">
                          &lt;双击填写&gt;
                        </span>
                        <span v-else>{{ formatCell(insRow[pc.col]) }}</span>
                      </template>
                    </td>
                    <td v-if="colWindow.beforeSize > 0" class="virtual-spacer-cell" />
                    <td
                      v-for="rc in renderedColumns"
                      :key="rc.col"
                      :class="{
                        'selected-cell': selectedIsInserted && selectedRowOrigIdx === insIdx && selectedColName === rc.col,
                      }"
                      @click="selectCell(true, insIdx, rc.col)"
                      @dblclick="startEditCell(true, insIdx, rc.col)"
                    >
                      <div
                        v-if="
                          editingCell &&
                          editingCell.isInserted &&
                          editingCell.rowIdx === insIdx &&
                          editingCell.col === rc.col
                        "
                        class="cell-inline-editor"
                        @click.stop
                      >
                        <input
                          ref="inlineInputRef"
                          v-model="editingCell.draftText"
                          @keydown.enter.prevent="commitInlineEdit(false)"
                          @keydown.esc.prevent="cancelInlineEdit"
                        />
                        <button class="cell-inline-btn" @click="commitInlineEdit(true)">NULL</button>
                        <button class="cell-inline-btn" @click="commitInlineEdit(false)">✓</button>
                      </div>
                      <template v-else>
                        <span v-if="insRow[rc.col] === null" class="null-pill">&lt;NULL&gt;</span>
                        <span v-else-if="insRow[rc.col] === ''" style="color: var(--text-muted); font-style: italic">
                          &lt;双击填写&gt;
                        </span>
                        <span v-else>{{ formatCell(insRow[rc.col]) }}</span>
                      </template>
                    </td>
                    <td v-if="colWindow.afterSize > 0" class="virtual-spacer-cell" />
                  </tr>
                </tbody>
              </table>
            </div>

            <!-- Interactive Value Viewer & Long-Text Editor -->
            <div v-if="showValueViewer" class="value-viewer">
              <div class="tw-header">
                <span style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap">
                  {{ selectedColName || t('valueViewer') }}
                </span>
                <div style="display: flex; gap: 3px; flex-shrink: 0">
                  <button
                    class="icon-btn compact"
                    title="格式化为多行缩进 JSON"
                    @click="formatValueViewerJson(true)"
                  >
                    格式化
                  </button>
                  <button
                    class="icon-btn compact"
                    title="压缩为单行 JSON"
                    @click="formatValueViewerJson(false)"
                  >
                    压缩
                  </button>
                  <button
                    class="icon-btn compact"
                    @click="copyText(valueViewerDraft || '<NULL>')"
                  >
                    {{ t('btnCopy') }}
                  </button>
                </div>
              </div>
              <div
                v-if="inspectedTimestampHint"
                style="
                  padding: 4px 8px;
                  font-size: 11px;
                  background: var(--accent-soft);
                  color: var(--accent);
                  border-bottom: 1px solid var(--border-subtle);
                  font-family: var(--font-mono);
                "
              >
                时间解析：{{ inspectedTimestampHint }}
              </div>
              <textarea
                v-model="valueViewerDraft"
                class="field"
                style="
                  flex: 1;
                  border: none;
                  border-radius: 0;
                  font-family: var(--font-mono);
                  font-size: 12px;
                  resize: none;
                "
                :readonly="currentResult.readOnly"
                placeholder="点击任意单元格查看/编辑完整内容或 JSON..."
              />
              <div
                v-if="!currentResult.readOnly && selectedColName"
                style="
                  display: flex;
                  justify-content: flex-end;
                  gap: 6px;
                  padding: 6px 8px;
                  border-top: 1px solid var(--border);
                "
              >
                <button class="btn btn-sm" @click="applyValueViewerEdit(true)">
                  {{ t('setNull') }}
                </button>
                <button class="btn btn-sm btn-primary" @click="applyValueViewerEdit(false)">
                  应用修改
                </button>
              </div>
            </div>
          </div>

          <!-- Server Pagination + Instant Column Summary Stats Bar -->
          <div v-if="columnStats || currentResult.sourceTable" class="grid-stats-bar">
            <div style="display: flex; align-items: center; gap: 10px; flex-wrap: wrap">
              <template v-if="columnStats">
                <span>
                  列统计 <strong style="color: var(--text-primary); font-family: var(--font-mono)">{{ columnStats.column }}</strong>
                </span>
                <span>行: <strong>{{ columnStats.total }}</strong></span>
                <span>非空: <strong>{{ columnStats.nonNull }}</strong></span>
                <span>去重: <strong>{{ columnStats.distinct }}</strong></span>
                <template v-if="columnStats.isNumeric">
                  <span>SUM: <strong>{{ columnStats.sum }}</strong></span>
                  <span>AVG: <strong>{{ columnStats.avg }}</strong></span>
                  <span>MIN: <strong>{{ columnStats.min }}</strong></span>
                  <span>MAX: <strong>{{ columnStats.max }}</strong></span>
                </template>
              </template>
            </div>
            <div v-if="currentResult.sourceTable" class="grid-pagination-bar">
              <span>每页</span>
              <select
                :value="currentResult.pageSize ?? 100"
                class="field"
                style="width: auto; padding: 1px 5px; font-size: 11px"
                :disabled="executing"
                @change="submitServerQuery(0, Number(($event.target as HTMLSelectElement).value) || 100)"
              >
                <option :value="50">50</option>
                <option :value="100">100</option>
                <option :value="200">200</option>
                <option :value="500">500</option>
                <option :value="1000">1000</option>
              </select>
              <button
                class="icon-btn compact"
                :disabled="executing || (currentResult.pageIndex ?? 0) <= 0"
                @click="submitServerQuery((currentResult.pageIndex ?? 0) - 1)"
              >
                ◀ 上一页
              </button>
              <span style="font-family: var(--font-mono)">
                第 {{ (currentResult.pageIndex ?? 0) + 1 }} 页
              </span>
              <button
                class="icon-btn compact"
                :disabled="executing || currentResult.rows.length < (currentResult.pageSize ?? 100)"
                @click="submitServerQuery((currentResult.pageIndex ?? 0) + 1)"
              >
                下一页 ▶
              </button>
              <button
                v-if="currentResult.totalRowCount == null"
                class="icon-btn compact"
                :disabled="executing"
                title="执行 SELECT COUNT(*) 获取精确总行数"
                @click="emit('server-count-rows', currentResult)"
              >
                统计总行数
              </button>
              <span v-else class="pill pill-neutral">
                共 {{ currentResult.totalRowCount.toLocaleString() }} 条
              </span>
            </div>
            <span v-else style="opacity: 0.78">提示：点击列名排序 · 点击 # 行号多选 · 点击「📌」可冻结列到左侧</span>
          </div>
        </template>
      </template>

      <!-- ==================== 2. Unified Project Output Console (Copyable + Red Errors) ==================== -->
      <template v-else-if="activeServiceTab === 'output'">
        <div class="console-toolbar">
          <div style="display: flex; align-items: center; gap: 6px; flex-wrap: wrap">
            <select
              v-model="logProjectFilter"
              class="field"
              style="width: auto; max-width: 175px; padding: 2px 6px; font-size: 11.5px"
              title="按项目筛选控制台日志（默认全部项目）"
            >
              <option value="ALL">全部项目</option>
              <option v-for="proj in projects" :key="proj.id" :value="proj.id">
                {{ proj.name }}
              </option>
            </select>

            <select
              v-model="logProfileFilter"
              class="field"
              style="width: auto; max-width: 195px; padding: 2px 6px; font-size: 11.5px"
              title="按数据源筛选控制台日志"
            >
              <option value="ALL">全部数据源</option>
              <template v-if="logProjectFilter === 'ALL' && projects.length > 1">
                <optgroup v-for="proj in projects" :key="proj.id" :label="proj.name">
                  <option
                    v-for="conn in connectionsInProject(proj.id)"
                    :key="conn.id"
                    :value="conn.id"
                  >
                    {{ conn.name }}
                  </option>
                </optgroup>
              </template>
              <template v-else>
                <option
                  v-for="conn in logProjectFilter === 'ALL' ? connections : connectionsInProject(logProjectFilter)"
                  :key="conn.id"
                  :value="conn.id"
                >
                  {{ conn.name }}
                </option>
              </template>
            </select>

            <button
              class="icon-btn compact"
              :class="{ active: logOnlyErrors }"
              @click="logOnlyErrors = !logOnlyErrors"
            >
              🔴 {{ t('onlyErrors') }} ({{ errorLogCount }})
            </button>

            <input
              v-model="logSearchText"
              class="field"
              style="width: 170px; padding: 2px 7px; font-size: 11.5px"
              placeholder="搜索日志或报错..."
            />
          </div>

          <div style="display: flex; align-items: center; gap: 5px; flex-wrap: wrap">
            <button class="btn btn-sm" :disabled="filteredConsoleLogs.length === 0" @click="copyAllFilteredLogs">
              <IdeIcon name="copy" :size="12" /> {{ t('copyAllLogs') }}
            </button>
            <button class="btn btn-sm btn-danger" :disabled="errorLogCount === 0" @click="copyOnlyErrorLogs">
              <IdeIcon name="copy" :size="12" /> {{ t('copyErrorLogs') }}
            </button>
            <button
              class="btn btn-sm"
              :disabled="logScopedEntries.length === 0"
              @click="emit('clear-console-logs', logProjectFilter)"
            >
              <IdeIcon name="trash" :size="12" /> {{ t('clearLogs') }}
            </button>
          </div>
        </div>

        <div ref="consoleScrollEl" class="console-output-area">
          <div v-if="filteredConsoleLogs.length === 0" class="empty-state" style="padding: 12px 4px">
            当前筛选范围内尚无控制台执行日志。所有项目及数据源的 SQL 执行、结果集修改提交与报错都会汇总展示在这里，支持鼠标划选与一键复制。
          </div>

          <div
            v-for="entry in filteredConsoleLogs"
            :key="entry.id"
            class="log-entry"
            :class="`level-${entry.level}`"
          >
            <div class="log-entry-head">
              <span class="log-ts">[{{ entry.timestamp }}]</span>
              <span class="pill pill-neutral" style="display: inline-flex; align-items: center; gap: 4px">
                <span
                  class="project-dot"
                  :style="{ background: projectColorById(entry.projectId), width: '6px', height: '6px' }"
                />
                {{ projectNameById(entry.projectId) }}
              </span>
              <span class="log-source-badge">
                {{ entry.dbName ? `${entry.profileName} / ${entry.dbName}` : entry.profileName }}
              </span>
              <span v-if="entry.level === 'error'" class="pill pill-prod">ERROR</span>
              <span v-else-if="entry.level === 'success'" class="pill pill-dev">OK</span>
              <span v-else-if="entry.level === 'warn'" class="pill pill-test">WARN</span>
              <span style="font-weight: 500">{{ entry.summary }}</span>

              <div class="log-entry-actions">
                <button
                  class="cell-inline-btn"
                  title="复制该条日志"
                  @click="copyText(formatLogEntryAsText(entry))"
                >
                  <IdeIcon name="copy" :size="11" /> 复制
                </button>
                <button
                  v-if="entry.level === 'sql' && entry.detail"
                  class="cell-inline-btn"
                  title="发送到查询编辑器"
                  @click="emit('send-sql-to-editor', entry.detail)"
                >
                  ↗ 编辑器
                </button>
              </div>
            </div>
            <pre v-if="entry.detail" class="log-detail">{{ entry.detail }}</pre>
          </div>
        </div>

        <!-- Quick Paste & Run Bar inside Console -->
        <div class="console-paste-bar">
          <span style="font-size: 11px; color: var(--text-secondary); white-space: nowrap">
            SQL 快捷粘贴执行 &gt;
          </span>
          <input
            v-model="quickConsoleSql"
            class="field"
            style="font-family: var(--font-mono); padding: 3px 8px; font-size: 12px"
            :placeholder="`可直接 ${IS_MAC ? '⌘+V' : 'Ctrl+V'} 粘贴单条 SQL 并按 Enter 在当前数据源执行...`"
            @keydown.enter="submitQuickConsoleSql"
          />
          <button
            class="btn btn-sm btn-primary"
            :disabled="!quickConsoleSql.trim()"
            @click="submitQuickConsoleSql"
          >
            <IdeIcon name="run" :size="11" /> 执行
          </button>
        </div>
      </template>

      <!-- ==================== 3. DDL Tab ==================== -->
      <template v-else-if="activeServiceTab === 'ddl'">
        <div class="grid-toolbar">
          <span style="font-weight: 600">{{ ddlTitle || t('ddlLabel') }}</span>
          <button v-if="ddlText" class="btn btn-sm" @click="copyText(ddlText)">
            {{ t('copyDdl') }}
          </button>
        </div>
        <div style="flex: 1; overflow: auto; padding: 10px; background: var(--bg-editor)">
          <pre class="code-viewer" style="border: none; padding: 0">{{
            ddlText || '在左侧数据库树右键数据表并选择「查看建表 DDL」以显示建表语句。'
          }}</pre>
        </div>
      </template>

      <!-- ==================== 4. History Tab (Filterable by Project & Data Source) ==================== -->
      <template v-else-if="activeServiceTab === 'history'">
        <div class="console-toolbar">
          <div style="display: flex; align-items: center; gap: 6px; flex-wrap: wrap">
            <select
              v-model="historyProjectFilter"
              class="field"
              style="width: auto; max-width: 175px; padding: 2px 6px; font-size: 11.5px"
              title="按项目筛选执行历史（与左侧项目列表一致）"
            >
              <option value="ALL">全部项目</option>
              <option v-for="proj in projects" :key="proj.id" :value="proj.id">
                {{ proj.name }}
              </option>
            </select>

            <select
              v-model="historyProfileFilter"
              class="field"
              style="width: auto; max-width: 195px; padding: 2px 6px; font-size: 11.5px"
              title="按数据源筛选执行历史"
            >
              <option value="ALL">全部数据源</option>
              <template v-if="historyProjectFilter === 'ALL' && projects.length > 1">
                <optgroup v-for="proj in projects" :key="proj.id" :label="proj.name">
                  <option
                    v-for="conn in connectionsInProject(proj.id)"
                    :key="conn.id"
                    :value="conn.id"
                  >
                    {{ conn.name }}
                  </option>
                </optgroup>
              </template>
              <template v-else>
                <option
                  v-for="conn in historyProjectFilter === 'ALL' ? connections : connectionsInProject(historyProjectFilter)"
                  :key="conn.id"
                  :value="conn.id"
                >
                  {{ conn.name }}
                </option>
              </template>
              <option v-for="opt in legacyHistoryProfiles" :key="opt.id" :value="opt.id">
                {{ opt.name }}
              </option>
            </select>

            <select
              v-model="historyStatusFilter"
              class="field"
              style="width: auto; padding: 2px 6px; font-size: 11.5px"
              title="按执行状态筛选"
            >
              <option value="ALL">全部状态</option>
              <option value="OK">仅成功</option>
              <option value="ERR">仅失败</option>
            </select>

            <input
              v-model="historySearchText"
              class="field"
              style="width: 180px; padding: 2px 7px; font-size: 11.5px"
              placeholder="搜索历史 SQL / 报错..."
            />
          </div>

          <span style="font-size: 11px; color: var(--text-secondary)">
            显示 {{ filteredHistoryEntries.length }} / {{ historyEntries.length }} 条记录
          </span>
        </div>

        <div v-if="filteredHistoryEntries.length === 0" class="empty-state">
          {{ historyEntries.length === 0 ? t('emptyHistoryHint') : '当前筛选条件下无匹配的执行历史记录。' }}
        </div>
        <div v-else class="grid-scroll">
          <table class="dg-grid">
            <thead>
              <tr>
                <th style="width: 140px">时间</th>
                <th style="width: 115px">项目</th>
                <th style="width: 155px">数据源 / 库</th>
                <th style="width: 68px">状态</th>
                <th style="width: 78px">耗时</th>
                <th>SQL 语句（点击载入编辑器）</th>
                <th style="width: 44px">操作</th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="item in filteredHistoryEntries"
                :key="item.id"
                style="cursor: pointer"
                @click="emit('load-history-sql', item)"
              >
                <td>{{ item.timestamp }}</td>
                <td>
                  <span class="pill pill-neutral">
                    {{ projectNameById(resolveHistoryProjectId(item)) }}
                  </span>
                </td>
                <td>{{ item.profileName }}{{ item.database ? ` / ${item.database}` : '' }}</td>
                <td>
                  <span class="pill" :class="item.success ? 'pill-dev' : 'pill-prod'">
                    {{ item.success ? '成功' : '失败' }}
                  </span>
                </td>
                <td class="num-cell">{{ item.durationMs }} ms</td>
                <td :title="item.errorMessage ? `${item.sql}\n\nERROR: ${item.errorMessage}` : item.sql">
                  <span>{{ item.sql }}</span>
                  <span
                    v-if="!item.success && item.errorMessage"
                    style="color: var(--prod); margin-left: 8px; font-weight: 500"
                  >
                    ✕ {{ item.errorMessage }}
                  </span>
                </td>
                <td @click.stop>
                  <button
                    class="icon-btn compact"
                    title="复制 SQL"
                    @click="copyText(item.sql)"
                  >
                    <IdeIcon name="copy" :size="12" />
                  </button>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </template>

      <!-- ==================== 5. AST Inspections / Problems Tab ==================== -->
      <template v-else-if="activeServiceTab === 'problems'">
        <div style="flex: 1; overflow: auto; padding: 10px">
          <div v-if="!astAnalysis" class="empty-state">暂无 SQL 检查结果。</div>
          <template v-else>
            <div style="display: flex; align-items: center; gap: 8px; margin-bottom: 10px">
              <span class="pill pill-neutral">分类: {{ astAnalysis.overallKind }}</span>
              <span class="pill" :class="astAnalysis.requiresTicket ? 'pill-prod' : 'pill-dev'">
                {{ astAnalysis.requiresTicket ? '含变更/高危语句' : '只读安全' }}
              </span>
              <span style="font-size: 11px; color: var(--text-secondary)">
                共解析 {{ astAnalysis.statements.length }} 条 SQL 语句
              </span>
            </div>

            <div v-if="astWarnings.length === 0" class="empty-state" style="padding: 8px 0">
              ✓ 未检测到高危 SQL 规则告警。
            </div>
            <div
              v-for="(w, idx) in astWarnings"
              :key="idx"
              style="
                display: flex;
                align-items: center;
                gap: 8px;
                padding: 7px 10px;
                margin-bottom: 6px;
                border-radius: 5px;
                border: 1px solid var(--border);
                background: var(--bg-chrome);
              "
            >
              <span
                class="pill"
                :class="w.level === 'CRITICAL' || w.level === 'HIGH' ? 'pill-prod' : 'pill-test'"
              >
                {{ w.level }}
              </span>
              <strong>{{ w.code }}</strong>
              <span style="color: var(--text-secondary)">{{ w.message }}</span>
            </div>
          </template>
        </div>
      </template>
    </div>

    <!-- Result Grid Cell Right-Click Context Menu -->
    <div
      v-if="cellCtxMenu.visible && currentResult"
      class="context-menu"
      :style="{ left: `${cellCtxMenu.x}px`, top: `${cellCtxMenu.y}px`, zIndex: 96 }"
      @click.stop
    >
      <button
        class="dropdown-item"
        @click="
          cellCtxMenu.visible = false;
          copyText(formatCell(getContextCellRaw()));
        "
      >
        <span>复制单元格值</span>
      </button>
      <button
        class="dropdown-item"
        @click="
          cellCtxMenu.visible = false;
          copyContextCellPrettyJson();
        "
      >
        <span>格式化复制为 JSON</span>
      </button>
      <button
        class="dropdown-item"
        @click="
          cellCtxMenu.visible = false;
          filterText = formatCell(getContextCellRaw());
        "
      >
        <span>按此值快速筛选</span>
      </button>
      <button
        class="dropdown-item"
        @click="
          cellCtxMenu.visible = false;
          copyContextCellWhere();
        "
      >
        <span>复制为 SQL WHERE 条件</span>
      </button>
      <button
        class="dropdown-item"
        @click="
          cellCtxMenu.visible = false;
          doCopyColumnLines();
        "
      >
        <span>复制整列值（逐行）</span>
      </button>
      <button
        class="dropdown-item"
        @click="
          cellCtxMenu.visible = false;
          doCopyColumnInSql();
        "
      >
        <span>复制整列为 IN (...)</span>
      </button>
      <button
        class="dropdown-item"
        @click="
          cellCtxMenu.visible = false;
          hideSpecificColumn(cellCtxMenu.col);
        "
      >
        <span>隐藏当前列 [{{ cellCtxMenu.col }}]</span>
      </button>
      <button
        class="dropdown-item"
        @click="
          cellCtxMenu.visible = false;
          togglePinColumn(cellCtxMenu.col);
        "
      >
        <span>
          {{ pinnedColumnsSet.has(cellCtxMenu.col) ? `取消固定列 [${cellCtxMenu.col}]` : `📌 固定列 [${cellCtxMenu.col}] 到左侧` }}
        </span>
      </button>
      <template v-if="currentResult.sourceTable">
        <div class="dropdown-sep" />
        <button
          class="dropdown-item"
          @click="
            cellCtxMenu.visible = false;
            applyCellServerFilter('=');
          "
        >
          <span>服务端重查：WHERE [{{ cellCtxMenu.col }}] = 当前值</span>
        </button>
        <button
          class="dropdown-item"
          @click="
            cellCtxMenu.visible = false;
            applyCellServerFilter('!=');
          "
        >
          <span>服务端重查：WHERE [{{ cellCtxMenu.col }}] != 当前值</span>
        </button>
        <button
          class="dropdown-item"
          @click="
            cellCtxMenu.visible = false;
            applyCellServerFilter('IS NULL');
          "
        >
          <span>服务端重查：WHERE [{{ cellCtxMenu.col }}] IS NULL</span>
        </button>
      </template>
      <button
        v-if="
          getFkForCol(cellCtxMenu.col) &&
          getContextCellRaw() !== null &&
          getContextCellRaw() !== undefined
        "
        class="dropdown-item"
        @click="
          cellCtxMenu.visible = false;
          emit(
            'jump-foreign-key',
            currentResult,
            getFkForCol(cellCtxMenu.col)!,
            getContextCellRaw(),
          );
        "
      >
        <span>
          🔗 跳转外键表 {{ getFkForCol(cellCtxMenu.col)?.referencedTable }}.{{ getFkForCol(cellCtxMenu.col)?.referencedColumn }}
        </span>
      </button>
      <div class="dropdown-sep" />
      <button
        class="dropdown-item"
        @click="
          cellCtxMenu.visible = false;
          doCopySelectedRowJson();
        "
      >
        <span>复制当前行为 JSON</span>
      </button>
      <button
        class="dropdown-item"
        @click="
          cellCtxMenu.visible = false;
          copyContextRowInsertSql();
        "
      >
        <span>复制当前行为 INSERT SQL</span>
      </button>
      <button
        class="dropdown-item"
        @click="
          cellCtxMenu.visible = false;
          showValueViewer = true;
        "
      >
        <span>在值检视器打开 / 格式化 JSON</span>
      </button>
      <template v-if="!currentResult.readOnly">
        <div class="dropdown-sep" />
        <button
          class="dropdown-item"
          @click="
            cellCtxMenu.visible = false;
            startEditCell(false, cellCtxMenu.origIdx, cellCtxMenu.col);
          "
        >
          <span>编辑此单元格</span>
        </button>
        <button
          class="dropdown-item"
          @click="
            cellCtxMenu.visible = false;
            duplicateRowsByIndices([cellCtxMenu.origIdx]);
          "
        >
          <span>📋 克隆此行为新增行</span>
        </button>
        <button
          class="dropdown-item"
          @click="
            cellCtxMenu.visible = false;
            setContextCellNull();
          "
        >
          <span>将此单元格设为 NULL</span>
        </button>
        <button
          class="dropdown-item"
          style="color: var(--prod)"
          @click="
            cellCtxMenu.visible = false;
            selectedIsInserted = false;
            selectedRowOrigIdx = cellCtxMenu.origIdx;
            toggleDeleteSelectedRow();
          "
        >
          <span>标记 / 取消删除整行</span>
        </button>
      </template>
    </div>
  </div>
</template>
