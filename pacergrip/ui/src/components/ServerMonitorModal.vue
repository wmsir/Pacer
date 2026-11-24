<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import { executeSql, type ConnectionProfile } from '../api/ipc';
import IdeIcon from './IdeIcon.vue';
import { getEngineDisplayName, isConnectionProfileReadOnly } from '../utils/sqlTools';

const props = defineProps<{
  visible: boolean;
  profile: ConnectionProfile | null;
  busy: boolean;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
  (e: 'kill-session', sql: string, label: string, profile: ConnectionProfile): void;
  (e: 'notify', message: string): void;
}>();

const loading = ref(false);
const errorMsg = ref('');
const autoRefresh = ref(false);
const lastRefreshedAt = ref('');
const filterText = ref('');
const kpis = ref<Array<{ label: string; value: string; hint?: string }>>([]);
const sessionCols = ref<string[]>([]);
const sessionRows = ref<Array<Record<string, unknown>>>([]);
const redisSlowRows = ref<Array<Record<string, unknown>>>([]);
const activeRedisTab = ref<'clients' | 'slowlog'>('clients');

let timerHandle: number | null = null;
let refreshInFlight = false;

function stopTimer() {
  if (timerHandle !== null) {
    window.clearTimeout(timerHandle);
    timerHandle = null;
  }
}

function scheduleNextPoll() {
  stopTimer();
  if (!props.visible || !autoRefresh.value) return;
  timerHandle = window.setTimeout(() => {
    timerHandle = null;
    void refreshMonitor(true);
  }, 5000);
}

onBeforeUnmount(stopTimer);

function engineKey(): string {
  const p = props.profile;
  if (!p) return 'mysql';
  if (typeof p.dbType === 'string') return p.dbType;
  return Object.values(p.dbType)[0] ?? 'mysql';
}

function isReadOnlyProfile(): boolean {
  return isConnectionProfileReadOnly(props.profile);
}

function engineKind(): 'mysql' | 'postgres' | 'redis' | 'sqlite' | 'other' {
  const k = engineKey().toLowerCase();
  if (k.includes('redis')) return 'redis';
  if (k.includes('sqlite')) return 'sqlite';
  if (k.includes('postgres') || k.includes('kingbase') || k.includes('gauss')) return 'postgres';
  if (k.includes('mysql') || k.includes('maria') || k.includes('tidb') || k.includes('oceanbase')) {
    return 'mysql';
  }
  return 'other';
}

async function runCollectSql(
  profileId: string,
  sql: string,
): Promise<{ columns: string[]; rows: Array<Record<string, unknown>> }> {
  let columns: string[] = [];
  const rows: Array<Record<string, unknown>> = [];
  await executeSql(profileId, null, sql, 200, (chunk) => {
    if (columns.length === 0 && chunk.columns.length > 0) {
      columns = chunk.columns;
    }
    rows.push(...chunk.rows);
  });
  return { columns, rows };
}

function formatUptime(secRaw: unknown): string {
  const secs = Number(secRaw);
  if (!Number.isFinite(secs) || secs < 0) return String(secRaw ?? '-');
  const days = Math.floor(secs / 86400);
  const hrs = Math.floor((secs % 86400) / 3600);
  const mins = Math.floor((secs % 3600) / 60);
  if (days > 0) return `${days}d ${hrs}h`;
  if (hrs > 0) return `${hrs}h ${mins}m`;
  return `${mins}m ${Math.floor(secs % 60)}s`;
}

async function refreshMonitor(silent = false) {
  if (refreshInFlight) return;
  const prof = props.profile;
  if (!prof || !props.visible) return;
  refreshInFlight = true;
  stopTimer();
  if (!silent) loading.value = true;
  errorMsg.value = '';
  try {
    const kind = engineKind();
    if (kind === 'mysql') {
      const [statusRes, procRes] = await Promise.all([
        runCollectSql(prof.id, 'SHOW GLOBAL STATUS'),
        runCollectSql(prof.id, 'SHOW FULL PROCESSLIST'),
      ]);
      const statusMap = new Map<string, string>();
      for (const r of statusRes.rows) {
        const k = String(r.Variable_name ?? r.variable_name ?? '');
        const v = String(r.Value ?? r.value ?? '');
        if (k) statusMap.set(k.toLowerCase(), v);
      }
      kpis.value = [
        { label: '当前连接数', value: statusMap.get('threads_connected') ?? String(procRes.rows.length) },
        { label: '活跃执行线程', value: statusMap.get('threads_running') ?? '-' },
        { label: '累计查询次数', value: statusMap.get('questions') ?? statusMap.get('queries') ?? '-' },
        { label: '慢查询数量', value: statusMap.get('slow_queries') ?? '0' },
        { label: '运行时长', value: formatUptime(statusMap.get('uptime')) },
      ];
      sessionCols.value = procRes.columns;
      sessionRows.value = procRes.rows;
    } else if (kind === 'postgres') {
      const actRes = await runCollectSql(
        prof.id,
        `SELECT pid, usename AS user, datname AS database_name, COALESCE(client_addr::text, 'local') AS host, state, COALESCE(wait_event_type, '-') AS wait_type, LEFT(COALESCE(query, ''), 180) AS query FROM pg_stat_activity ORDER BY pid DESC LIMIT 120`,
      );
      const total = actRes.rows.length;
      const activeCnt = actRes.rows.filter((r) => String(r.state) === 'active').length;
      const waitingCnt = actRes.rows.filter((r) => r.wait_type && r.wait_type !== '-').length;
      kpis.value = [
        { label: '总会话数', value: String(total) },
        { label: 'Active 执行中', value: String(activeCnt) },
        { label: '等待事件会话', value: String(waitingCnt) },
        { label: 'Idle 空闲会话', value: String(total - activeCnt) },
      ];
      sessionCols.value = actRes.columns;
      sessionRows.value = actRes.rows;
    } else if (kind === 'redis') {
      const infoRes = await runCollectSql(prof.id, 'INFO');
      const kv = new Map<string, string>();
      for (const r of infoRes.rows) {
        const p = String(r.property ?? '');
        const v = String(r.value ?? '');
        if (p) kv.set(p.toLowerCase(), v);
      }
      const hits = Number(kv.get('keyspace_hits') ?? 0);
      const misses = Number(kv.get('keyspace_misses') ?? 0);
      const ratio = hits + misses > 0 ? `${((hits / (hits + misses)) * 100).toFixed(1)}%` : '-';
      kpis.value = [
        { label: 'Redis 版本', value: kv.get('redis_version') ?? '-' },
        { label: '连接客户端', value: kv.get('connected_clients') ?? '-' },
        { label: '已用内存', value: kv.get('used_memory_human') ?? '-' },
        { label: '瞬时 QPS', value: kv.get('instantaneous_ops_per_sec') ?? '-' },
        { label: '缓存命中率', value: ratio },
        { label: '运行时长', value: formatUptime(kv.get('uptime_in_seconds')) },
      ];
      try {
        const clRes = await runCollectSql(prof.id, 'CLIENT LIST');
        const rawText = String(clRes.rows[0]?.result ?? '');
        const parsedClients = rawText
          .split(/\r?\n/)
          .map((l) => l.trim())
          .filter(Boolean)
          .slice(0, 100)
          .map((line) => {
            const obj: Record<string, string> = {};
            for (const tok of line.split(/\s+/)) {
              const eq = tok.indexOf('=');
              if (eq > 0) obj[tok.slice(0, eq)] = tok.slice(eq + 1);
            }
            return obj;
          });
        sessionCols.value = ['id', 'addr', 'db', 'age', 'idle', 'cmd'];
        sessionRows.value = parsedClients;
      } catch {
        sessionCols.value = [];
        sessionRows.value = [];
      }
      try {
        const slRes = await runCollectSql(prof.id, 'SLOWLOG GET 20');
        redisSlowRows.value = slRes.rows;
      } catch {
        redisSlowRows.value = [];
      }
    } else if (kind === 'sqlite') {
      const [pc, ps, jm, fk] = await Promise.all([
        runCollectSql(prof.id, 'PRAGMA page_count'),
        runCollectSql(prof.id, 'PRAGMA page_size'),
        runCollectSql(prof.id, 'PRAGMA journal_mode'),
        runCollectSql(prof.id, 'PRAGMA foreign_keys'),
      ]);
      const pages = Number(Object.values(pc.rows[0] ?? {})[0] ?? 0);
      const pageSize = Number(Object.values(ps.rows[0] ?? {})[0] ?? 4096);
      kpis.value = [
        { label: '总页数', value: String(pages) },
        { label: '页大小', value: `${pageSize} B` },
        { label: '数据文件估算', value: `${((pages * pageSize) / 1024).toFixed(1)} KB` },
        { label: '日志模式', value: String(Object.values(jm.rows[0] ?? {})[0] ?? '-') },
        { label: '外键约束', value: Number(Object.values(fk.rows[0] ?? {})[0] ?? 0) ? '启用' : '关闭' },
      ];
      sessionCols.value = [];
      sessionRows.value = [];
    } else {
      errorMsg.value = `暂不支持对 ${getEngineDisplayName(engineKey())} 执行自动化服务器监控探测。`;
    }
    lastRefreshedAt.value = new Date().toLocaleTimeString();
  } catch (err) {
    errorMsg.value = String(err);
  } finally {
    refreshInFlight = false;
    loading.value = false;
    if (props.visible && autoRefresh.value) {
      scheduleNextPoll();
    }
  }
}

const filteredSessionRows = computed(() => {
  const rows =
    engineKind() === 'redis' && activeRedisTab.value === 'slowlog'
      ? redisSlowRows.value
      : sessionRows.value;
  const q = filterText.value.trim().toLowerCase();
  if (!q) return rows;
  return rows.filter((row) =>
    Object.values(row).some((v) => String(v ?? '').toLowerCase().includes(q)),
  );
});

function handleKillRow(row: Record<string, unknown>) {
  const prof = props.profile;
  if (!prof) return;
  const kind = engineKind();
  if (kind === 'mysql') {
    const id = row.Id ?? row.id;
    if (id === undefined || id === null) return;
    emit('kill-session', `KILL ${id};`, `终止 MySQL 会话 #${id}`, prof);
  } else if (kind === 'postgres') {
    const pid = row.pid ?? row.PID;
    if (pid === undefined || pid === null) return;
    emit('kill-session', `SELECT pg_terminate_backend(${pid});`, `终止 PG 进程 #${pid}`, prof);
  }
}

watch(
  () => [props.visible, props.profile?.id],
  ([vis]) => {
    if (vis) {
      void refreshMonitor();
    } else {
      stopTimer();
    }
  },
);

watch(
  () => [props.visible, autoRefresh.value],
  ([vis, auto]) => {
    stopTimer();
    if (vis && auto) {
      scheduleNextPoll();
    }
  },
);
</script>

<template>
  <div v-if="visible && profile" class="modal-backdrop" @click.self="emit('close')">
    <div class="modal-card" style="width: min(940px, 95vw)">
      <div class="modal-header">
        <span style="display: inline-flex; align-items: center; gap: 8px">
          <span>📊 实例性能监控与活跃会话管理</span>
          <span class="pill pill-neutral">
            {{ profile.name }} · {{ getEngineDisplayName(engineKey()) }}
          </span>
        </span>
        <div style="display: flex; align-items: center; gap: 8px">
          <label style="display: inline-flex; align-items: center; gap: 4px; font-size: 11.5px; cursor: pointer">
            <input v-model="autoRefresh" type="checkbox" /> 5s 自动刷新
          </label>
          <button class="btn btn-sm" :disabled="loading" @click="refreshMonitor(false)">
            <IdeIcon name="refresh" :size="12" /> {{ loading ? '刷新中...' : '立即刷新' }}
          </button>
          <button class="icon-btn compact" @click="emit('close')">×</button>
        </div>
      </div>

      <div v-if="errorMsg" class="status-banner-error" style="margin-bottom: 10px">
        {{ errorMsg }}
      </div>

      <!-- KPI Cards -->
      <div class="monitor-kpi-grid">
        <div v-for="k in kpis" :key="k.label" class="monitor-kpi-card">
          <div class="monitor-kpi-label">{{ k.label }}</div>
          <div class="monitor-kpi-value">{{ k.value }}</div>
        </div>
      </div>

      <div style="display: flex; align-items: center; justify-content: space-between; margin: 10px 0 6px; gap: 8px">
        <div style="display: flex; align-items: center; gap: 6px">
          <strong style="font-size: 12px">
            {{
              engineKind() === 'redis'
                ? 'Redis 运行诊断列表'
                : engineKind() === 'sqlite'
                  ? 'SQLite 文件参数摘要'
                  : `活跃会话 / 正在执行的 SQL (${filteredSessionRows.length})`
            }}
          </strong>
          <template v-if="engineKind() === 'redis'">
            <button
              class="btn btn-sm"
              :class="{ 'btn-primary': activeRedisTab === 'clients' }"
              @click="activeRedisTab = 'clients'"
            >
              客户端 ({{ sessionRows.length }})
            </button>
            <button
              class="btn btn-sm"
              :class="{ 'btn-primary': activeRedisTab === 'slowlog' }"
              @click="activeRedisTab = 'slowlog'"
            >
              慢查询 ({{ redisSlowRows.length }})
            </button>
          </template>
        </div>
        <div style="display: flex; align-items: center; gap: 8px">
          <input
            v-model="filterText"
            class="field"
            style="width: 210px; padding: 3px 8px"
            placeholder="过滤用户、库名、SQL..."
          />
          <span v-if="lastRefreshedAt" style="font-size: 11px; color: var(--text-muted)">
            更新于 {{ lastRefreshedAt }}
          </span>
        </div>
      </div>

      <div
        v-if="sessionCols.length > 0 || (engineKind() === 'redis' && activeRedisTab === 'slowlog')"
        class="designer-scroll"
        style="max-height: 310px"
      >
        <table class="designer-table">
          <thead>
            <tr>
              <th
                v-for="col in engineKind() === 'redis' && activeRedisTab === 'slowlog'
                  ? Object.keys(redisSlowRows[0] ?? { value: '' })
                  : sessionCols"
                :key="col"
              >
                {{ col }}
              </th>
              <th v-if="engineKind() === 'mysql' || engineKind() === 'postgres'" style="width: 82px; text-align: right">
                管理
              </th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(row, rIdx) in filteredSessionRows" :key="rIdx">
              <td
                v-for="col in engineKind() === 'redis' && activeRedisTab === 'slowlog'
                  ? Object.keys(redisSlowRows[0] ?? { value: '' })
                  : sessionCols"
                :key="col"
                style="font-family: var(--font-mono); font-size: 11px; max-width: 260px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap"
                :title="String(row[col] ?? '')"
              >
                {{ row[col] ?? '-' }}
              </td>
              <td
                v-if="engineKind() === 'mysql' || engineKind() === 'postgres'"
                style="text-align: right"
              >
                <button
                  class="cell-inline-btn"
                  style="color: var(--prod)"
                  :disabled="busy || isReadOnlyProfile()"
                  :title="isReadOnlyProfile() ? '只读数据源不可终止会话' : '安全检查后终止该连接会话'"
                  @click="handleKillRow(row)"
                >
                  终止
                </button>
              </td>
            </tr>
            <tr v-if="filteredSessionRows.length === 0">
              <td :colspan="Math.max(1, sessionCols.length + 1)" style="text-align: center; padding: 18px; color: var(--text-muted)">
                暂无匹配记录
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </div>
</template>
