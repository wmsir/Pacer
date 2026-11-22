<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue';
import type { ColumnMeta, ConnectionProfile, TableMeta } from '../api/ipc';
import IdeIcon from './IdeIcon.vue';
import {
  downloadTextFile,
  inferErRelationships,
  type ErEdge,
  type ErNodeLayout,
} from '../utils/sqlTools';

const props = defineProps<{
  visible: boolean;
  profile: ConnectionProfile | null;
  dbName: string;
  tables: TableMeta[];
  columnsByKey: Record<string, ColumnMeta[]>;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
  (e: 'query-table', profile: ConnectionProfile, dbName: string, tableName: string): void;
  (e: 'notify', message: string): void;
}>();

const svgRef = ref<SVGSVGElement | null>(null);
const filterText = ref('');
const onlyKeysMode = ref(false);
const zoom = ref(1);
const panX = ref(0);
const panY = ref(0);
const selectedTable = ref('');
const customPositions = reactive<Record<string, { x: number; y: number }>>({});

const dragState = ref<{
  mode: 'pan' | 'node';
  startX: number;
  startY: number;
  origX: number;
  origY: number;
  tableName?: string;
} | null>(null);

const baseGraph = computed(() => {
  if (!props.profile || !props.dbName) {
    return { nodes: [] as ErNodeLayout[], edges: [] as ErEdge[] };
  }
  const q = filterText.value.trim().toLowerCase();
  const filteredTables = q
    ? props.tables.filter((t) => t.name.toLowerCase().includes(q))
    : props.tables;
  return inferErRelationships(
    filteredTables.slice(0, 48).map((tbl) => ({
      tableName: tbl.name,
      comment: tbl.comment,
      estimatedRows: tbl.estimatedRows,
      columns:
        props.columnsByKey[`${props.profile!.id}:${props.dbName}:${tbl.name}`] ?? [],
    })),
  );
});

const positionedNodes = computed(() =>
  baseGraph.value.nodes.map((node) => {
    const pos = customPositions[node.tableName];
    return pos ? { ...node, x: pos.x, y: pos.y } : node;
  }),
);

const nodeMap = computed(() => {
  const map = new Map<string, ErNodeLayout>();
  for (const n of positionedNodes.value) {
    map.set(n.tableName, n);
  }
  return map;
});

function displayedColumns(node: ErNodeLayout): ColumnMeta[] {
  const cols = onlyKeysMode.value
    ? node.columns.filter(
        (c) => c.isPrimaryKey || c.name.toLowerCase().endsWith('_id') || c.name.toLowerCase() === 'id',
      )
    : node.columns;
  return cols.slice(0, 10);
}

function edgePathD(edge: ErEdge): string {
  const from = nodeMap.value.get(edge.fromTable);
  const to = nodeMap.value.get(edge.toTable);
  if (!from || !to) return '';
  if (from.tableName === to.tableName) {
    const sx = from.x + from.width;
    const sy = from.y + 42;
    return `M ${sx} ${sy} C ${sx + 48} ${sy - 28}, ${sx + 48} ${sy + 48}, ${sx} ${sy + 20}`;
  }
  const fromRight = from.x + from.width / 2 < to.x + to.width / 2;
  const x1 = fromRight ? from.x + from.width : from.x;
  const y1 = from.y + Math.min(from.height / 2, 70);
  const x2 = fromRight ? to.x : to.x + to.width;
  const y2 = to.y + Math.min(to.height / 2, 70);
  const dx = Math.max(48, Math.abs(x2 - x1) * 0.45);
  const cx1 = fromRight ? x1 + dx : x1 - dx;
  const cx2 = fromRight ? x2 - dx : x2 + dx;
  return `M ${x1} ${y1} C ${cx1} ${y1}, ${cx2} ${y2}, ${x2} ${y2}`;
}

function isEdgeHighlighted(edge: ErEdge): boolean {
  if (!selectedTable.value) return false;
  return edge.fromTable === selectedTable.value || edge.toTable === selectedTable.value;
}

const canvasExtent = computed(() => {
  let maxX = 1100;
  let maxY = 720;
  for (const n of positionedNodes.value) {
    maxX = Math.max(maxX, n.x + n.width + 80);
    maxY = Math.max(maxY, n.y + n.height + 80);
  }
  return { width: maxX, height: maxY };
});

function startCanvasPan(ev: MouseEvent) {
  dragState.value = {
    mode: 'pan',
    startX: ev.clientX,
    startY: ev.clientY,
    origX: panX.value,
    origY: panY.value,
  };
}

function startNodeDrag(ev: MouseEvent, node: ErNodeLayout) {
  selectedTable.value = node.tableName;
  dragState.value = {
    mode: 'node',
    tableName: node.tableName,
    startX: ev.clientX,
    startY: ev.clientY,
    origX: node.x,
    origY: node.y,
  };
}

function onPointerMove(ev: MouseEvent) {
  const st = dragState.value;
  if (!st) return;
  if (st.mode === 'pan') {
    panX.value = st.origX + (ev.clientX - st.startX);
    panY.value = st.origY + (ev.clientY - st.startY);
  } else if (st.mode === 'node' && st.tableName) {
    const scale = Math.max(0.25, zoom.value);
    customPositions[st.tableName] = {
      x: Math.round(st.origX + (ev.clientX - st.startX) / scale),
      y: Math.round(st.origY + (ev.clientY - st.startY) / scale),
    };
  }
}

function stopDrag() {
  dragState.value = null;
}

function handleWheel(ev: WheelEvent) {
  const delta = ev.deltaY < 0 ? 0.1 : -0.1;
  zoom.value = Number(Math.min(2.2, Math.max(0.35, zoom.value + delta)).toFixed(2));
}

function resetView() {
  zoom.value = 1;
  panX.value = 0;
  panY.value = 0;
  for (const k of Object.keys(customPositions)) delete customPositions[k];
}

function exportErSvg() {
  const el = svgRef.value;
  if (!el) return;
  const serialized = new XMLSerializer().serializeToString(el);
  downloadTextFile(`${props.dbName || 'database'}-er-diagram.svg`, serialized, 'image/svg+xml;charset=utf-8');
  emit('notify', `已导出 ${props.dbName} ER 关系图 (.svg)`);
}

watch(
  () => [props.visible, props.dbName],
  ([vis]) => {
    if (vis) {
      selectedTable.value = '';
      resetView();
    }
  },
);
</script>

<template>
  <div v-if="visible && profile" class="modal-backdrop" @click.self="emit('close')">
    <div class="modal-card" style="width: min(1060px, 96vw); padding: 14px">
      <div class="modal-header" style="margin-bottom: 8px">
        <span style="display: inline-flex; align-items: center; gap: 8px">
          <span>🗺️ 数据库 ER 关系图谱</span>
          <span class="pill pill-neutral">
            {{ profile.name }} / {{ dbName }} · {{ positionedNodes.length }} 张表 · {{ baseGraph.edges.length }} 条关联
          </span>
        </span>
        <button class="icon-btn compact" @click="emit('close')">×</button>
      </div>

      <div class="er-toolbar">
        <div style="display: flex; align-items: center; gap: 8px; flex-wrap: wrap">
          <input
            v-model="filterText"
            class="field"
            style="width: 190px; padding: 3px 8px"
            placeholder="筛选数据表..."
          />
          <label style="display: inline-flex; align-items: center; gap: 4px; font-size: 11.5px; cursor: pointer">
            <input v-model="onlyKeysMode" type="checkbox" /> 仅显示主键/外键列
          </label>
          <span style="font-size: 11px; color: var(--text-secondary)">
            拖拽空白平移 · 滚轮缩放 · 拖拽卡片调位 · 双击表卡片查前100行
          </span>
        </div>
        <div style="display: flex; align-items: center; gap: 5px">
          <button class="btn btn-sm" @click="zoom = Math.max(0.35, Number((zoom - 0.15).toFixed(2)))">－</button>
          <span style="font-size: 11px; min-width: 44px; text-align: center">{{ Math.round(zoom * 100) }}%</span>
          <button class="btn btn-sm" @click="zoom = Math.min(2.2, Number((zoom + 0.15).toFixed(2)))">＋</button>
          <button class="btn btn-sm" @click="resetView">重置布局</button>
          <button class="btn btn-primary btn-sm" @click="exportErSvg">
            <IdeIcon name="save" :size="12" /> 导出 SVG
          </button>
        </div>
      </div>

      <div
        class="er-canvas-wrap"
        @mousedown="startCanvasPan"
        @mousemove="onPointerMove"
        @mouseup="stopDrag"
        @mouseleave="stopDrag"
        @wheel.prevent="handleWheel"
      >
        <svg
          ref="svgRef"
          class="er-svg"
          xmlns="http://www.w3.org/2000/svg"
          :viewBox="`0 0 ${canvasExtent.width} ${canvasExtent.height}`"
        >
          <defs>
            <marker
              id="pacer-er-arrow"
              markerWidth="8"
              markerHeight="8"
              refX="7"
              refY="4"
              orient="auto"
            >
              <path d="M0,1 L7,4 L0,7 Z" fill="#6f737a" />
            </marker>
            <marker
              id="pacer-er-arrow-hi"
              markerWidth="8"
              markerHeight="8"
              refX="7"
              refY="4"
              orient="auto"
            >
              <path d="M0,1 L7,4 L0,7 Z" fill="#3574f0" />
            </marker>
          </defs>

          <g :transform="`translate(${panX}, ${panY}) scale(${zoom})`">
            <!-- Relationship Edges -->
            <path
              v-for="edge in baseGraph.edges"
              :key="edge.id"
              :d="edgePathD(edge)"
              fill="none"
              :stroke="isEdgeHighlighted(edge) ? '#3574f0' : '#6f737a'"
              :stroke-width="isEdgeHighlighted(edge) ? 2.2 : 1.3"
              :stroke-opacity="selectedTable && !isEdgeHighlighted(edge) ? 0.25 : 0.8"
              :marker-end="isEdgeHighlighted(edge) ? 'url(#pacer-er-arrow-hi)' : 'url(#pacer-er-arrow)'"
            >
              <title>{{ edge.inferredReason }}</title>
            </path>

            <!-- Table Entity Cards -->
            <g
              v-for="node in positionedNodes"
              :key="node.tableName"
              :transform="`translate(${node.x}, ${node.y})`"
              style="cursor: grab"
              @mousedown.stop="startNodeDrag($event, node)"
              @dblclick.stop="emit('query-table', profile, dbName, node.tableName)"
            >
              <rect
                :width="node.width"
                :height="node.height"
                rx="7"
                ry="7"
                fill="var(--bg-chrome, #2b2d30)"
                :stroke="selectedTable === node.tableName ? 'var(--accent, #3574f0)' : 'var(--border, #4a4d54)'"
                :stroke-width="selectedTable === node.tableName ? 2 : 1.2"
              />
              <rect
                :width="node.width"
                height="28"
                rx="7"
                ry="7"
                :fill="selectedTable === node.tableName ? 'var(--bg-selection, #2e436e)' : 'var(--bg-elevated, #35383e)'"
              />
              <text
                x="10"
                y="18"
                fill="var(--text-primary, #dfe1e5)"
                font-size="12"
                font-weight="700"
                font-family="JetBrains Mono, Consolas, monospace"
              >
                {{ node.tableName }}
              </text>

              <g
                v-for="(col, cIdx) in displayedColumns(node)"
                :key="col.name"
                :transform="`translate(0, ${34 + cIdx * 22})`"
              >
                <text
                  x="10"
                  y="14"
                  :fill="col.isPrimaryKey ? 'var(--test, #e0a82e)' : 'var(--text-primary, #dfe1e5)'"
                  font-size="11"
                  font-family="JetBrains Mono, Consolas, monospace"
                >
                  {{ col.isPrimaryKey ? 'PK ' : '' }}{{ col.name }}
                </text>
                <text
                  :x="node.width - 10"
                  y="14"
                  text-anchor="end"
                  fill="var(--text-secondary, #868a91)"
                  font-size="10.5"
                  font-family="JetBrains Mono, Consolas, monospace"
                >
                  {{ col.dataType }}
                </text>
              </g>
              <text
                v-if="node.columns.length > displayedColumns(node).length"
                x="10"
                :y="node.height - 8"
                fill="var(--text-muted, #868a91)"
                font-size="10"
              >
                + 还有 {{ node.columns.length - displayedColumns(node).length }} 个字段...
              </text>
            </g>
          </g>
        </svg>
      </div>
    </div>
  </div>
</template>
