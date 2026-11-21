<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from '../i18n';
import type { QueryHistoryEntry, QuerySummary, ScriptAnalysis } from '../api/ipc';
import {
  downloadTextFile,
  toCsv,
  toInsertSql,
  toMarkdownTable,
} from '../utils/sqlTools';

export interface ResultTabItem {
  id: string;
  title: string;
  sql: string;
  columns: string[];
  rows: Record<string, unknown>[];
  summary: QuerySummary | null;
}

const props = defineProps<{
  resultTabs: ResultTabItem[];
  activeResultTabId: string;
  activeServiceTab: 'result' | 'output' | 'ddl' | 'history' | 'problems';
  consoleLogs: string[];
  ddlText: string;
  ddlTitle: string;
  historyEntries: QueryHistoryEntry[];
  astAnalysis: ScriptAnalysis | null;
}>();

const emit = defineEmits<{
  (e: 'update:activeServiceTab', tab: 'result' | 'output' | 'ddl' | 'history' | 'problems'): void;
  (e: 'update:activeResultTabId', id: string): void;
  (e: 'close-result-tab', id: string): void;
  (e: 'load-history-sql', entry: QueryHistoryEntry): void;
  (e: 'notify', message: string): void;
}>();

const { t } = useI18n();
const filterText = ref('');
const sortCol = ref<string | null>(null);
const sortDir = ref<'asc' | 'desc'>('asc');
const transposeMode = ref(false);
const showValueViewer = ref(false);
const selectedRowIdx = ref<number>(0);
const selectedColName = ref<string>('');

const currentResult = computed(
  () =>
    props.resultTabs.find((tab) => tab.id === props.activeResultTabId) ??
    props.resultTabs[0] ??
    null,
);

const displayedRows = computed(() => {
  const res = currentResult.value;
  if (!res) return [];
  const q = filterText.value.trim().toLowerCase();
  let list = q
    ? res.rows.filter((row) =>
        res.columns.some((col) =>
          String(row[col] ?? '')
            .toLowerCase()
            .includes(q),
        ),
      )
    : [...res.rows];

  if (sortCol.value) {
    const col = sortCol.value;
    const dir = sortDir.value === 'asc' ? 1 : -1;
    list.sort((a, b) => {
      const av = a[col];
      const bv = b[col];
      if (av === bv) return 0;
      if (av === null || av === undefined) return 1;
      if (bv === null || bv === undefined) return -1;
      if (typeof av === 'number' && typeof bv === 'number') return (av - bv) * dir;
      return String(av).localeCompare(String(bv)) * dir;
    });
  }
  return list;
});

const inspectedValue = computed(() => {
  const row = displayedRows.value[selectedRowIdx.value];
  if (!row || !selectedColName.value) return '';
  const val = row[selectedColName.value];
  if (val === null || val === undefined) return '<NULL>';
  if (typeof val === 'object') return JSON.stringify(val, null, 2);
  const str = String(val);
  if ((str.startsWith('{') && str.endsWith('}')) || (str.startsWith('[') && str.endsWith(']'))) {
    try {
      return JSON.stringify(JSON.parse(str), null, 2);
    } catch {
      return str;
    }
  }
  return str;
});

const astWarnings = computed(
  () => props.astAnalysis?.statements.flatMap((s) => s.warnings) ?? [],
);

function toggleSort(col: string) {
  if (sortCol.value === col) {
    if (sortDir.value === 'asc') {
      sortDir.value = 'desc';
    } else {
      sortCol.value = null;
    }
  } else {
    sortCol.value = col;
    sortDir.value = 'asc';
  }
}

function selectCell(rowIdx: number, col: string) {
  selectedRowIdx.value = rowIdx;
  selectedColName.value = col;
}

function formatCell(val: unknown): string {
  if (val === null || val === undefined) return '<NULL>';
  if (typeof val === 'object') return JSON.stringify(val);
  return String(val);
}

function isNumeric(val: unknown): boolean {
  return typeof val === 'number';
}

function doExportCsv() {
  if (!currentResult.value) return;
  downloadTextFile(
    `${currentResult.value.title}.csv`,
    toCsv(currentResult.value.columns, displayedRows.value),
    'text/csv;charset=utf-8',
  );
}

function doExportJson() {
  if (!currentResult.value) return;
  downloadTextFile(
    `${currentResult.value.title}.json`,
    JSON.stringify(displayedRows.value, null, 2),
    'application/json;charset=utf-8',
  );
}

function doCopyMarkdown() {
  if (!currentResult.value) return;
  void navigator.clipboard?.writeText(
    toMarkdownTable(currentResult.value.columns, displayedRows.value),
  );
  emit('notify', t('copiedNotice'));
}

function doCopyInsertSql() {
  if (!currentResult.value) return;
  const guessTable =
    currentResult.value.sql.match(/from\s+[`"\[]?([a-zA-Z0-9_$.]+)[`"\]]?/i)?.[1] ??
    'target_table';
  void navigator.clipboard?.writeText(
    toInsertSql(guessTable, currentResult.value.columns, displayedRows.value),
  );
  emit('notify', t('copiedNotice'));
}

function copyText(text: string) {
  void navigator.clipboard?.writeText(text);
  emit('notify', t('copiedNotice'));
}
</script>

<template>
  <div class="services-panel" style="height: 100%">
    <div class="services-tabs">
      <div class="services-tab-list">
        <button
          class="services-tab"
          :class="{ active: activeServiceTab === 'result' }"
          @click="emit('update:activeServiceTab', 'result')"
        >
          {{ t('tabResult') }}
          <span v-if="currentResult" style="opacity: 0.7">({{ displayedRows.length }})</span>
        </button>
        <button
          class="services-tab"
          :class="{ active: activeServiceTab === 'output' }"
          @click="emit('update:activeServiceTab', 'output')"
        >
          {{ t('tabOutput') }} ({{ consoleLogs.length }})
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

      <div v-if="resultTabs.length > 1 && activeServiceTab === 'result'" style="display: flex; gap: 4px">
        <button
          v-for="rt in resultTabs"
          :key="rt.id"
          class="btn btn-sm"
          :class="{ 'btn-primary': rt.id === activeResultTabId }"
          @click="emit('update:activeResultTabId', rt.id)"
        >
          {{ rt.title }}
          <span style="margin-left: 4px; opacity: 0.8" @click.stop="emit('close-result-tab', rt.id)">×</span>
        </button>
      </div>
    </div>

    <div class="services-body">
      <!-- Result Grid Tab -->
      <template v-if="activeServiceTab === 'result'">
        <div v-if="!currentResult || currentResult.columns.length === 0" class="empty-state">
          <div v-if="currentResult?.summary?.isDmlOrDdl">
            <span class="pill pill-dev">执行成功</span>
            <span style="margin-left: 8px">
              {{ t('affectedRows', { count: currentResult.summary.affectedRows ?? 0 }) }} ·
              {{ currentResult.summary.durationMs }} ms
            </span>
          </div>
          <div v-else>{{ t('emptyGridHint') }}</div>
        </div>

        <template v-else>
          <div class="grid-toolbar">
            <div style="display: flex; align-items: center; gap: 6px; flex-wrap: wrap">
              <input
                v-model="filterText"
                class="field"
                style="width: 190px; padding: 3px 7px"
                :placeholder="t('filterPlaceholder')"
              />
              <button
                class="icon-btn compact"
                :class="{ active: transposeMode }"
                @click="transposeMode = !transposeMode"
              >
                ⇅ {{ t('transposeView') }}
              </button>
              <button
                class="icon-btn compact"
                :class="{ active: showValueViewer }"
                @click="showValueViewer = !showValueViewer"
              >
                👁 {{ t('valueViewer') }}
              </button>
              <span v-if="currentResult.summary" style="font-size: 11px; color: var(--text-secondary)">
                {{ currentResult.summary.durationMs }} ms ·
                {{ t('rowsCount', { count: displayedRows.length }) }}
              </span>
            </div>

            <div style="display: flex; align-items: center; gap: 4px; flex-wrap: wrap">
              <button class="btn btn-sm" @click="doExportCsv">{{ t('exportCsv') }}</button>
              <button class="btn btn-sm" @click="doExportJson">{{ t('exportJson') }}</button>
              <button class="btn btn-sm" @click="doCopyMarkdown">{{ t('copyMarkdown') }}</button>
              <button class="btn btn-sm" @click="doCopyInsertSql">{{ t('copyInsertSql') }}</button>
            </div>
          </div>

          <div class="grid-split">
            <div class="grid-scroll">
              <!-- Transpose View -->
              <table v-if="transposeMode && displayedRows[selectedRowIdx]" class="dg-grid">
                <thead>
                  <tr>
                    <th style="width: 220px">{{ t('columnsLabel') }}</th>
                    <th>行 #{{ selectedRowIdx + 1 }} 值</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="col in currentResult.columns" :key="col">
                    <td style="font-weight: 600">{{ col }}</td>
                    <td
                      :class="{ 'selected-cell': selectedColName === col }"
                      @click="selectCell(selectedRowIdx, col)"
                    >
                      <span v-if="displayedRows[selectedRowIdx][col] === null" class="null-pill">
                        &lt;NULL&gt;
                      </span>
                      <span v-else>{{ formatCell(displayedRows[selectedRowIdx][col]) }}</span>
                    </td>
                  </tr>
                </tbody>
              </table>

              <!-- Standard DataGrip Grid -->
              <table v-else class="dg-grid">
                <thead>
                  <tr>
                    <th class="row-num">#</th>
                    <th
                      v-for="col in currentResult.columns"
                      :key="col"
                      @click="toggleSort(col)"
                    >
                      <span>{{ col }}</span>
                      <span v-if="sortCol === col" style="margin-left: 4px; color: var(--accent)">
                        {{ sortDir === 'asc' ? '▲' : '▼' }}
                      </span>
                    </th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="(row, rIdx) in displayedRows" :key="rIdx">
                    <td class="row-num">{{ rIdx + 1 }}</td>
                    <td
                      v-for="col in currentResult.columns"
                      :key="col"
                      :class="{
                        'num-cell': isNumeric(row[col]),
                        'selected-cell': selectedRowIdx === rIdx && selectedColName === col,
                      }"
                      @click="selectCell(rIdx, col)"
                      @dblclick="copyText(formatCell(row[col]))"
                    >
                      <span v-if="row[col] === null || row[col] === undefined" class="null-pill">
                        &lt;NULL&gt;
                      </span>
                      <span v-else>{{ formatCell(row[col]) }}</span>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>

            <div v-if="showValueViewer" class="value-viewer">
              <div class="tw-header">
                <span>{{ selectedColName || t('valueViewer') }}</span>
                <button
                  v-if="inspectedValue"
                  class="icon-btn compact"
                  @click="copyText(inspectedValue)"
                >
                  {{ t('btnCopy') }}
                </button>
              </div>
              <pre>{{ inspectedValue || '点击任意单元格以检视完整值 / 格式化 JSON' }}</pre>
            </div>
          </div>
        </template>
      </template>

      <!-- Output Log Tab -->
      <div v-else-if="activeServiceTab === 'output'" class="grid-scroll" style="padding: 8px 12px">
        <div
          v-for="(line, i) in consoleLogs"
          :key="i"
          style="font-family: var(--font-mono); font-size: 12px; line-height: 1.55"
        >
          {{ line }}
        </div>
      </div>

      <!-- DDL Tab -->
      <div v-else-if="activeServiceTab === 'ddl'" class="grid-scroll" style="padding: 10px 12px">
        <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px">
          <strong>{{ ddlTitle || t('tabDdl') }}</strong>
          <button v-if="ddlText" class="btn btn-sm" @click="copyText(ddlText)">
            {{ t('copyDdl') }}
          </button>
        </div>
        <pre class="code-viewer">{{ ddlText || t('emptyStructureHint') }}</pre>
      </div>

      <!-- History Tab -->
      <div v-else-if="activeServiceTab === 'history'" class="grid-scroll">
        <div v-if="historyEntries.length === 0" class="empty-state">
          {{ t('emptyHistoryHint') }}
        </div>
        <table v-else class="dg-grid">
          <thead>
            <tr>
              <th>时间</th>
              <th>数据源</th>
              <th>状态</th>
              <th>耗时</th>
              <th>结果</th>
              <th>SQL 语句</th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="h in historyEntries"
              :key="h.id"
              style="cursor: pointer"
              @click="emit('load-history-sql', h)"
            >
              <td>{{ h.timestamp }}</td>
              <td>{{ h.profileName }}</td>
              <td>
                <span class="pill" :class="h.success ? 'pill-dev' : 'pill-prod'">
                  {{ h.success ? 'OK' : 'ERR' }}
                </span>
              </td>
              <td class="num-cell">{{ h.durationMs }} ms</td>
              <td>{{ h.affectedRows != null ? `aff: ${h.affectedRows}` : `${h.rowCount} 行` }}</td>
              <td>{{ h.sql }}</td>
            </tr>
          </tbody>
        </table>
      </div>

      <!-- AST Inspections / Problems Tab -->
      <div v-else class="grid-scroll" style="padding: 8px 12px">
        <div v-if="astWarnings.length === 0" class="empty-state" style="padding: 12px 0">
          ✅ 当前脚本静态 AST 检查无高危风险警告。
        </div>
        <div v-else style="display: flex; flex-direction: column; gap: 6px">
          <div
            v-for="(w, i) in astWarnings"
            :key="i"
            style="
              display: flex;
              align-items: center;
              gap: 8px;
              padding: 6px 10px;
              border: 1px solid var(--border);
              border-radius: 5px;
              background: var(--bg-chrome);
            "
          >
            <span
              class="pill"
              :class="w.level === 'CRITICAL' || w.level === 'HIGH' ? 'pill-prod' : 'pill-test'"
            >
              {{ w.level }}
            </span>
            <code>{{ w.code }}</code>
            <span>语句 #{{ w.statementIndex + 1 }}: {{ w.message }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
