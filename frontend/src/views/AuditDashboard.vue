<template>
  <div class="audit-dashboard-container page-container">
    <!-- 顶部标题与说明 -->
    <div class="header-action">
      <div class="title-area">
        <h2 class="page-title">SQL 审计与企业安全合规中心 (Audit & Compliance Center)</h2>
        <div class="page-subtitle">
          全量数据库操作链路防篡改审计、SM3 哈希存证校验、慢 SQL 性能分析及多格式合规报告导出
        </div>
      </div>
      <div class="action-btns">
        <el-button :icon="Refresh" :loading="loading" @click="fetchAllData">刷新大屏</el-button>
        <el-button type="success" :icon="Download" :loading="exportLoading" @click="handleExportCsv">
          一键导出审计日志 (CSV)
        </el-button>
      </div>
    </div>

    <!-- 1. 核心指标概览卡片 -->
    <div class="metrics-grid">
      <div class="metric-card card-blue">
        <div class="card-icon-wrap"><el-icon><Document /></el-icon></div>
        <div class="card-info">
          <div class="card-num">{{ stats.totalCount || 0 }}</div>
          <div class="card-title">全量审计 SQL 吞吐</div>
        </div>
      </div>

      <div class="metric-card card-green">
        <div class="card-icon-wrap"><el-icon><Check /></el-icon></div>
        <div class="card-info">
          <div class="card-num">{{ stats.successRate || '100' }}%</div>
          <div class="card-title">执行成功率</div>
        </div>
      </div>

      <div class="metric-card card-orange">
        <div class="card-icon-wrap"><el-icon><Clock /></el-icon></div>
        <div class="card-info">
          <div class="card-num">{{ stats.slowSqlCount || 0 }}</div>
          <div class="card-title">慢 SQL 告警 (≥300ms)</div>
        </div>
      </div>

      <div class="metric-card card-purple">
        <div class="card-icon-wrap"><el-icon><Stamp /></el-icon></div>
        <div class="card-info">
          <div class="card-num">{{ stats.complianceScore || 98 }}<span style="font-size: 14px; font-weight: normal;"> / 100</span></div>
          <div class="card-title">安全合规综合评分</div>
        </div>
      </div>
    </div>

    <!-- 2. 可视化图表分析大屏 (ECharts) -->
    <div class="charts-row">
      <!-- 24小时 SQL 流量与拦截时序走势 -->
      <div class="chart-card">
        <div class="chart-header">
          <div class="chart-title">
            <el-icon color="#409EFF"><TrendCharts /></el-icon>
            <span>24小时 SQL 执行走势与异常拦截时序</span>
          </div>
          <el-tag size="small" type="primary" effect="plain">实时监控</el-tag>
        </div>
        <div ref="trendChartRef" class="echart-box"></div>
      </div>

      <!-- SQL 操作类型分布占比 -->
      <div class="chart-card">
        <div class="chart-header">
          <div class="chart-title">
            <el-icon color="#67C23A"><PieChart /></el-icon>
            <span>SQL 指令类型分布占比</span>
          </div>
          <el-tag size="small" type="success" effect="plain">操作分布</el-tag>
        </div>
        <div ref="pieChartRef" class="echart-box"></div>
      </div>
    </div>

    <!-- 3. 慢 SQL TOP 10 监控榜单 -->
    <div class="section-card-wrapper">
      <el-card shadow="hover">
        <template #header>
          <div class="section-header-flex">
            <div class="section-title">
              <el-icon color="#E6A23C"><WarningFilled /></el-icon>
              <span>慢 SQL TOP 10 性能分析与影响诊断</span>
            </div>
            <el-tag size="small" type="warning" effect="dark">重点优化</el-tag>
          </div>
        </template>
        <el-table :data="stats.slowSqlTop10 || []" border stripe size="small" style="width: 100%">
          <el-table-column prop="id" label="审计ID" width="90" align="center" />
          <el-table-column prop="ticketId" label="关联工单" width="180" align="center">
            <template #default="{ row }">
              <span v-if="row.ticketId" class="ticket-link" @click="goToTicket(row.ticketId)">
                #{{ row.ticketId }}
              </span>
              <span v-else style="color: #94a3b8;">系统直通</span>
            </template>
          </el-table-column>
          <el-table-column prop="executeSql" label="执行 SQL 语句" min-width="320" show-overflow-tooltip>
            <template #default="{ row }">
              <code class="sql-code-text">{{ row.executeSql }}</code>
            </template>
          </el-table-column>
          <el-table-column prop="costTimeMs" label="执行耗时" width="130" align="center">
            <template #default="{ row }">
              <el-tag :type="row.costTimeMs >= 500 ? 'danger' : 'warning'" effect="dark">
                {{ row.costTimeMs }} ms
              </el-tag>
            </template>
          </el-table-column>
          <el-table-column prop="status" label="状态" width="100" align="center">
            <template #default="{ row }">
              <el-tag :type="row.status === 'SUCCESS' ? 'success' : 'danger'" effect="light">
                {{ row.status }}
              </el-tag>
            </template>
          </el-table-column>
        </el-table>
      </el-card>
    </div>

    <!-- 4. 全量 SQL 审计日志与在线查询审计中心 -->
    <div class="section-card-wrapper">
      <el-card shadow="hover">
        <el-tabs v-model="activeAuditTab" type="border-card">
          <!-- ==================== Tab 1：变更工单与底层执行防篡改审计 ==================== -->
          <el-tab-pane label="📜 变更工单与底层执行防篡改审计" name="ticket_audit">
            <div class="section-header-flex" style="margin-bottom: 14px;">
              <div class="section-title">
                <el-icon color="#409EFF"><Search /></el-icon>
                <span>工单执行防篡改存证链 (共 {{ logsTotal }} 条记录)</span>
              </div>
              <div class="right-filters">
                <el-input
                  v-model="filters.keyword"
                  placeholder="搜索 SQL 内容 / 存证哈希 / 错误信息..."
                  clearable
                  style="width: 260px;"
                  :prefix-icon="Search"
                  @keyup.enter="fetchLogs"
                  @clear="fetchLogs"
                />
                <el-select v-model="filters.status" placeholder="状态" style="width: 110px;" @change="fetchLogs">
                  <el-option label="全部状态" value="ALL" />
                  <el-option label="SUCCESS" value="SUCCESS" />
                  <el-option label="FAILED" value="FAILED" />
                </el-select>
                <el-button type="primary" :icon="Search" @click="fetchLogs">查询</el-button>
                <el-button :icon="Download" type="success" plain @click="handleExportCsv">导出日志 (CSV)</el-button>
              </div>
            </div>

            <el-table :data="logList" border stripe size="small" style="width: 100%" v-loading="logsLoading">
              <el-table-column prop="id" label="审计 ID" width="90" align="center" />
              <el-table-column prop="ticketId" label="关联工单" width="160" align="center">
                <template #default="{ row }">
                  <span v-if="row.ticketId" class="ticket-link" @click="goToTicket(row.ticketId)">
                    #{{ row.ticketId }}
                  </span>
                  <span v-else style="color: #94a3b8;">直接执行</span>
                </template>
              </el-table-column>
              <el-table-column prop="executeSql" label="执行 SQL 语句" min-width="280" show-overflow-tooltip>
                <template #default="{ row }">
                  <code class="sql-code-text">{{ row.executeSql }}</code>
                </template>
              </el-table-column>
              <el-table-column prop="costTimeMs" label="耗时" width="100" align="center">
                <template #default="{ row }">
                  <span :style="{ color: row.costTimeMs >= 300 ? '#e6a23c' : '#67c23a', fontWeight: 'bold' }">
                    {{ row.costTimeMs }} ms
                  </span>
                </template>
              </el-table-column>
              <el-table-column prop="status" label="状态" width="100" align="center">
                <template #default="{ row }">
                  <el-tag :type="row.status === 'SUCCESS' ? 'success' : 'danger'" effect="dark">
                    {{ row.status }}
                  </el-tag>
                </template>
              </el-table-column>
              <el-table-column prop="currentHash" label="SM3 防篡改存证哈希" min-width="180">
                <template #default="{ row }">
                  <el-tooltip :content="`当前哈希: ${row.currentHash || 'N/A'}\n前序哈希: ${row.previousHash || 'N/A'}`" placement="top">
                    <span class="hash-text">{{ row.currentHash ? row.currentHash.substring(0, 16) + '...' : '-' }}</span>
                  </el-tooltip>
                </template>
              </el-table-column>
              <el-table-column label="操作" width="110" align="center" fixed="right">
                <template #default="{ row }">
                  <el-button link type="primary" size="small" @click="showDetailModal(row)">查看明细</el-button>
                </template>
              </el-table-column>
            </el-table>

            <div class="pagination-wrap">
              <el-pagination
                v-model:current-page="pagination.page"
                v-model:page-size="pagination.size"
                :page-sizes="[10, 20, 50, 100]"
                layout="total, sizes, prev, pager, next, jumper"
                :total="logsTotal"
                @size-change="fetchLogs"
                @current-change="fetchLogs"
              />
            </div>
          </el-tab-pane>

          <!-- ==================== Tab 2：在线安全查询与 EXPLAIN 操作审计 ==================== -->
          <el-tab-pane label="🔍 在线查询与 EXPLAIN 操作审计" name="query_audit">
            <div class="section-header-flex" style="margin-bottom: 14px; flex-wrap: wrap; gap: 8px;">
              <div class="section-title">
                <el-icon color="#67C23A"><Search /></el-icon>
                <span>在线查询审计日志 (共 {{ queryLogsTotal }} 条)</span>
              </div>
              <div class="right-filters" style="display: flex; gap: 8px; align-items: center; flex-wrap: wrap;">
                <el-input
                  v-model="queryFilters.keyword"
                  placeholder="搜索查询 SQL / 报错详情..."
                  clearable
                  style="width: 220px;"
                  :prefix-icon="Search"
                  @keyup.enter="fetchQueryLogs"
                  @clear="fetchQueryLogs"
                />
                <el-input
                  v-model="queryFilters.username"
                  placeholder="执行人姓名/账号"
                  clearable
                  style="width: 140px;"
                  @keyup.enter="fetchQueryLogs"
                  @clear="fetchQueryLogs"
                />
                <el-select v-model="queryFilters.opType" placeholder="操作类型" style="width: 110px;" @change="fetchQueryLogs">
                  <el-option label="全部类型" value="ALL" />
                  <el-option label="SELECT" value="SELECT" />
                  <el-option label="EXPLAIN" value="EXPLAIN" />
                  <el-option label="SHOW" value="SHOW" />
                  <el-option label="DESC" value="DESC" />
                </el-select>
                <el-select v-model="queryFilters.status" placeholder="状态" style="width: 100px;" @change="fetchQueryLogs">
                  <el-option label="全部状态" value="ALL" />
                  <el-option label="SUCCESS" value="SUCCESS" />
                  <el-option label="FAILED" value="FAILED" />
                </el-select>
                <el-button type="primary" :icon="Search" @click="fetchQueryLogs">查询</el-button>
                <el-button type="warning" plain :icon="Setting" @click="openQueryAuditConfigModal">
                  ⚙️ 数据库审计策略配置
                </el-button>
              </div>
            </div>

            <el-table :data="queryLogList" border stripe size="small" style="width: 100%" v-loading="queryLogsLoading">
              <el-table-column prop="id" label="审计ID" width="80" align="center" />
              <el-table-column prop="createTime" label="执行时间" width="150" align="center">
                <template #default="{ row }">
                  <span style="font-size: 12px; color: #475569;">{{ formatTime(row.createTime) }}</span>
                </template>
              </el-table-column>
              <el-table-column prop="realName" label="执行人员" width="130" align="center">
                <template #default="{ row }">
                  <span style="font-weight: 600;">{{ row.realName || row.username }}</span>
                  <span style="font-size: 11px; color: #94a3b8; display: block;">@{{ row.username }}</span>
                </template>
              </el-table-column>
              <el-table-column prop="opType" label="操作类型" width="95" align="center">
                <template #default="{ row }">
                  <el-tag size="small" :type="row.opType === 'EXPLAIN' ? 'warning' : 'success'" effect="dark">
                    {{ row.opType }}
                  </el-tag>
                </template>
              </el-table-column>
              <el-table-column prop="dbName" label="目标数据库" width="180" show-overflow-tooltip>
                <template #default="{ row }">
                  <span style="font-weight: 600; color: #0284c7;">{{ row.instanceName }}</span>
                  <span style="color: #64748b; font-size: 11px; display: block;">/ {{ row.dbName }}</span>
                </template>
              </el-table-column>
              <el-table-column prop="sqlText" label="查询 SQL 语句" min-width="260" show-overflow-tooltip>
                <template #default="{ row }">
                  <code class="sql-code-text">{{ row.sqlText }}</code>
                </template>
              </el-table-column>
              <el-table-column prop="costMs" label="耗时" width="90" align="center">
                <template #default="{ row }">
                  <span :style="{ color: row.costMs >= 500 ? '#ef4444' : '#10b981', fontWeight: 'bold' }">
                    {{ row.costMs }}ms
                  </span>
                </template>
              </el-table-column>
              <el-table-column prop="resultRows" label="返回行数" width="90" align="center">
                <template #default="{ row }">
                  <span style="font-weight: 600;">{{ row.resultRows }} 行</span>
                </template>
              </el-table-column>
              <el-table-column prop="status" label="状态" width="85" align="center">
                <template #default="{ row }">
                  <el-tag :type="row.status === 'SUCCESS' ? 'success' : 'danger'" effect="plain">
                    {{ row.status }}
                  </el-tag>
                </template>
              </el-table-column>
              <el-table-column label="操作" width="95" align="center" fixed="right">
                <template #default="{ row }">
                  <el-button link type="primary" size="small" @click="showQueryDetailModal(row)">明细</el-button>
                </template>
              </el-table-column>
            </el-table>

            <div class="pagination-wrap">
              <el-pagination
                v-model:current-page="queryPagination.page"
                v-model:page-size="queryPagination.size"
                :page-sizes="[10, 20, 50, 100]"
                layout="total, sizes, prev, pager, next, jumper"
                :total="queryLogsTotal"
                @size-change="fetchQueryLogs"
                @current-change="fetchQueryLogs"
              />
            </div>
          </el-tab-pane>
        </el-tabs>
      </el-card>
    </div>

    <!-- 工单 SQL 明细弹窗 -->
    <el-dialog
      title="工单执行审计日志完整详情与防篡改存证"
      v-model="detailModalVisible"
      width="680px"
      append-to-body
    >
      <div v-if="selectedLog" class="log-detail-box">
        <div class="detail-item">
          <span class="d-label">审计日志 ID：</span>
          <span class="d-val">{{ selectedLog.id }}</span>
        </div>
        <div class="detail-item">
          <span class="d-label">关联工单 ID：</span>
          <span class="d-val">#{{ selectedLog.ticketId || '直接查询' }}</span>
        </div>
        <div class="detail-item">
          <span class="d-label">执行状态：</span>
          <el-tag :type="selectedLog.status === 'SUCCESS' ? 'success' : 'danger'">{{ selectedLog.status }}</el-tag>
        </div>
        <div class="detail-item">
          <span class="d-label">执行耗时：</span>
          <span class="d-val">{{ selectedLog.costTimeMs }} ms</span>
        </div>
        <div class="detail-item full-width">
          <span class="d-label">完整 SQL 语句：</span>
          <pre class="sql-code-block">{{ selectedLog.executeSql }}</pre>
        </div>
        <div class="detail-item full-width" v-if="selectedLog.errorTrace">
          <span class="d-label">异常堆栈信息：</span>
          <pre class="error-code-block">{{ selectedLog.errorTrace }}</pre>
        </div>
        <div class="detail-item full-width">
          <span class="d-label">SM3 当前存证哈希：</span>
          <code class="hash-block">{{ selectedLog.currentHash || '无' }}</code>
        </div>
        <div class="detail-item full-width">
          <span class="d-label">前序区块哈希 (防篡改链)：</span>
          <code class="hash-block">{{ selectedLog.previousHash || '0000000000000000000000000000000000000000000000000000000000000000' }}</code>
        </div>
      </div>
      <template #footer>
        <el-button @click="detailModalVisible = false">关闭</el-button>
      </template>
    </el-dialog>

    <!-- 在线查询审计明细弹窗 -->
    <el-dialog
      title="🔍 在线查询操作审计完整明细"
      v-model="queryDetailModalVisible"
      width="680px"
      append-to-body
    >
      <div v-if="selectedQueryLog" class="log-detail-box">
        <div class="detail-item">
          <span class="d-label">审计记录 ID：</span>
          <span class="d-val">{{ selectedQueryLog.id }}</span>
        </div>
        <div class="detail-item">
          <span class="d-label">执行人员：</span>
          <span class="d-val">{{ selectedQueryLog.realName }} ({{ selectedQueryLog.username }})</span>
        </div>
        <div class="detail-item">
          <span class="d-label">目标库表：</span>
          <span class="d-val">{{ selectedQueryLog.instanceName }} / {{ selectedQueryLog.dbName }}</span>
        </div>
        <div class="detail-item">
          <span class="d-label">操作类型：</span>
          <el-tag size="small" type="primary">{{ selectedQueryLog.opType }}</el-tag>
        </div>
        <div class="detail-item">
          <span class="d-label">执行耗时：</span>
          <span class="d-val">{{ selectedQueryLog.costMs }} ms</span>
        </div>
        <div class="detail-item">
          <span class="d-label">返回行数：</span>
          <span class="d-val">{{ selectedQueryLog.resultRows }} 行</span>
        </div>
        <div class="detail-item full-width">
          <span class="d-label">完整查询 SQL 语句：</span>
          <pre class="sql-code-block">{{ selectedQueryLog.sqlText }}</pre>
        </div>
        <div class="detail-item full-width" v-if="selectedQueryLog.errorMsg">
          <span class="d-label">错误原因详情：</span>
          <pre class="error-code-block">{{ selectedQueryLog.errorMsg }}</pre>
        </div>
      </div>
      <template #footer>
        <el-button @click="queryDetailModalVisible = false">关闭</el-button>
      </template>
    </el-dialog>

    <!-- 数据库查询审计策略配置弹窗 -->
    <el-dialog
      title="⚙️ 数据库在线查询审计策略配置"
      v-model="auditConfigModalVisible"
      width="640px"
      append-to-body
    >
      <div style="margin-bottom: 12px; font-size: 13px; color: #475569; background: #f0fdf4; border: 1px solid #bbf7d0; border-radius: 6px; padding: 10px 14px;">
        💡 <b>审计策略说明</b>：您可以按具体数据库维度配置是否开启在线查询与 EXPLAIN 审计。支持对核心生产库全量审计，对测试/非敏感库灵活关闭审计。
      </div>

      <el-form label-width="140px" size="default">
        <el-form-item label="全局审计默认策略">
          <el-switch
            v-model="auditPolicyForm.globalEnabled"
            active-text="默认全部数据库开启审计"
            inactive-text="默认关闭审计"
          />
        </el-form-item>

        <el-form-item label="禁用审计数据库黑名单">
          <el-select
            v-model="auditPolicyForm.disabledDatabases"
            multiple
            filterable
            clearable
            collapse-tags
            collapse-tags-tooltip
            placeholder="搜索并勾选需要【禁用查询审计】的数据库"
            style="width: 100%;"
          >
            <el-option
              v-for="db in allDatabaseFlatOptions"
              :key="db.value"
              :label="db.label"
              :value="db.value"
            />
          </el-select>
          <div style="font-size: 11px; color: #94a3b8; margin-top: 4px;">
            在黑名单中的数据库执行 SELECT / EXPLAIN 时将不再生成审计日志
          </div>
        </el-form-item>

        <el-form-item label="强制开启审计白名单">
          <el-select
            v-model="auditPolicyForm.enabledDatabases"
            multiple
            filterable
            clearable
            collapse-tags
            collapse-tags-tooltip
            placeholder="搜索并勾选需要【强制开启审计】的核心数据库"
            style="width: 100%;"
          >
            <el-option
              v-for="db in allDatabaseFlatOptions"
              :key="db.value"
              :label="db.label"
              :value="db.value"
            />
          </el-select>
        </el-form-item>
      </el-form>

      <template #footer>
        <el-button @click="auditConfigModalVisible = false">取消</el-button>
        <el-button type="primary" :loading="saveAuditConfigLoading" @click="handleSaveAuditConfig">
          保存策略配置
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, nextTick } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import {
  Refresh,
  Download,
  Document,
  Check,
  Clock,
  Stamp,
  TrendCharts,
  PieChart,
  WarningFilled,
  Search,
  Setting
} from '@element-plus/icons-vue'
import * as echarts from 'echarts'
import request from '../utils/request'

const router = useRouter()

const loading = ref(false)
const exportLoading = ref(false)
const logsLoading = ref(false)
const stats = ref<any>({})

const activeAuditTab = ref('ticket_audit')

// 1. 工单变更审计日志状态
const logList = ref<any[]>([])
const logsTotal = ref(0)
const pagination = ref({ page: 1, size: 20 })
const filters = ref({ keyword: '', status: 'ALL' })

// 2. 在线查询审计日志状态
const queryLogList = ref<any[]>([])
const queryLogsTotal = ref(0)
const queryLogsLoading = ref(false)
const queryPagination = ref({ page: 1, size: 20 })
const queryFilters = ref({ keyword: '', username: '', opType: 'ALL', status: 'ALL' })
const selectedQueryLog = ref<any>(null)
const queryDetailModalVisible = ref(false)

// 3. 数据库查询审计策略配置状态
const auditConfigModalVisible = ref(false)
const saveAuditConfigLoading = ref(false)
const auditPolicyForm = ref({
  globalEnabled: true,
  disabledDatabases: [] as string[],
  enabledDatabases: [] as string[]
})
const allDatabaseFlatOptions = ref<{ label: string; value: string }[]>([])

const trendChartRef = ref<HTMLElement | null>(null)
const pieChartRef = ref<HTMLElement | null>(null)
let trendChartInstance: echarts.ECharts | null = null
let pieChartInstance: echarts.ECharts | null = null

const detailModalVisible = ref(false)
const selectedLog = ref<any>(null)

const fetchAllData = async () => {
  loading.value = true
  try {
    await Promise.all([fetchDashboardStats(), fetchLogs(), fetchQueryLogs(), loadAllDatabaseOptions()])
    ElMessage.success('审计合规数据已同步')
  } catch (e: any) {
    ElMessage.error(e.message || '数据加载失败')
  } finally {
    loading.value = false
  }
}

const formatTime = (timeStr?: string | Date) => {
  if (!timeStr) return '-'
  try {
    const d = new Date(timeStr)
    if (isNaN(d.getTime())) return String(timeStr)
    const y = d.getFullYear()
    const m = String(d.getMonth() + 1).padStart(2, '0')
    const day = String(d.getDate()).padStart(2, '0')
    const hh = String(d.getHours()).padStart(2, '0')
    const mm = String(d.getMinutes()).padStart(2, '0')
    const ss = String(d.getSeconds()).padStart(2, '0')
    return `${y}-${m}-${day} ${hh}:${mm}:${ss}`
  } catch {
    return String(timeStr)
  }
}

// 加载在线查询审计日志
const fetchQueryLogs = async () => {
  queryLogsLoading.value = true
  try {
    const res: any = await request.get('/v1/audit/query-logs', {
      params: {
        keyword: queryFilters.value.keyword || undefined,
        username: queryFilters.value.username || undefined,
        opType: queryFilters.value.opType === 'ALL' ? undefined : queryFilters.value.opType,
        status: queryFilters.value.status === 'ALL' ? undefined : queryFilters.value.status,
        page: queryPagination.value.page,
        size: queryPagination.value.size
      }
    })
    if (res.data) {
      queryLogList.value = res.data.records || []
      queryLogsTotal.value = res.data.total || 0
    }
  } catch (err) {
    console.error('Fetch query logs error', err)
  } finally {
    queryLogsLoading.value = false
  }
}

const showQueryDetailModal = (row: any) => {
  selectedQueryLog.value = row
  queryDetailModalVisible.value = true
}

// 加载所有实例下的数据库用于策略配置
const loadAllDatabaseOptions = async () => {
  try {
    const instRes: any = await request.get('/v1/instance/list')
    const instList = Array.isArray(instRes.data) ? instRes.data : []
    const opts: { label: string; value: string }[] = []

    for (const inst of instList) {
      try {
        const dbsRes: any = await request.get(`/v1/instance/${inst.id}/databases`)
        const dbs = Array.isArray(dbsRes.data) ? dbsRes.data : []
        dbs.forEach((db: string) => {
          opts.push({
            label: `${inst.name} / ${db}`,
            value: `${inst.id}:${db}`
          })
          opts.push({
            label: `${db} (所有实例通用库名)`,
            value: db
          })
        })
      } catch (e) {}
    }
    // 去重
    const map = new Map()
    opts.forEach(it => {
      if (!map.has(it.value)) map.set(it.value, it)
    })
    allDatabaseFlatOptions.value = Array.from(map.values())
  } catch (err) {
    console.error('Load databases for audit config error', err)
  }
}

// 打开查询审计策略配置弹窗
const openQueryAuditConfigModal = async () => {
  auditConfigModalVisible.value = true
  try {
    const res: any = await request.get('/v1/audit/query-config')
    if (res.data) {
      auditPolicyForm.value = {
        globalEnabled: res.data.globalEnabled ?? true,
        disabledDatabases: res.data.disabledDatabases || [],
        enabledDatabases: res.data.enabledDatabases || []
      }
    }
    if (allDatabaseFlatOptions.value.length === 0) {
      loadAllDatabaseOptions()
    }
  } catch (err) {
    console.error('Load audit config error', err)
  }
}

// 保存查询审计策略配置
const handleSaveAuditConfig = async () => {
  saveAuditConfigLoading.value = true
  try {
    await request.post('/v1/audit/query-config', auditPolicyForm.value)
    ElMessage.success('数据库在线查询审计策略配置保存成功！')
    auditConfigModalVisible.value = false
  } catch (err: any) {
    ElMessage.error(err.response?.data?.message || err.message || '保存策略配置失败')
  } finally {
    saveAuditConfigLoading.value = false
  }
}

const fetchDashboardStats = async () => {
  try {
    const res: any = await request.get('/v1/audit/dashboard-stats')
    stats.value = res.data || {}
    await nextTick()
    renderTrendChart()
    renderPieChart()
  } catch (e) {
    // ignore
  }
}

const fetchLogs = async () => {
  logsLoading.value = true
  try {
    const res: any = await request.get('/v1/audit/logs', {
      params: {
        keyword: filters.value.keyword,
        status: filters.value.status === 'ALL' ? undefined : filters.value.status,
        page: pagination.value.page,
        size: pagination.value.size
      }
    })
    logList.value = res.data?.records || []
    logsTotal.value = res.data?.total || 0
  } catch (e) {
    // ignore
  } finally {
    logsLoading.value = false
  }
}

const renderTrendChart = () => {
  if (!trendChartRef.value) return
  if (!trendChartInstance) {
    trendChartInstance = echarts.init(trendChartRef.value)
  }
  const trendData = stats.value.hourlyTrend || []
  const hours = trendData.map((d: any) => d.hour)
  const executed = trendData.map((d: any) => d.executedCount)
  const blocked = trendData.map((d: any) => d.blockedCount)

  const option = {
    tooltip: { trigger: 'axis' },
    legend: { data: ['执行吞吐量', '安全拦截数'], bottom: 0 },
    grid: { left: '3%', right: '4%', bottom: '12%', top: '8%', containLabel: true },
    xAxis: { type: 'category', boundaryGap: false, data: hours },
    yAxis: { type: 'value' },
    series: [
      {
        name: '执行吞吐量',
        type: 'line',
        smooth: true,
        data: executed,
        areaStyle: { color: 'rgba(64,158,255,0.2)' },
        itemStyle: { color: '#409EFF' }
      },
      {
        name: '安全拦截数',
        type: 'line',
        smooth: true,
        data: blocked,
        itemStyle: { color: '#F56C6C' }
      }
    ]
  }
  trendChartInstance.setOption(option)
}

const renderPieChart = () => {
  if (!pieChartRef.value) return
  if (!pieChartInstance) {
    pieChartInstance = echarts.init(pieChartRef.value)
  }
  const distData = stats.value.operationDistribution || []

  const option = {
    tooltip: { trigger: 'item', formatter: '{b}: {c} 次 ({d}%)' },
    legend: { orient: 'vertical', right: '5%', top: 'center' },
    series: [
      {
        name: '操作类型',
        type: 'pie',
        radius: ['45%', '70%'],
        center: ['40%', '50%'],
        avoidLabelOverlap: false,
        itemStyle: {
          borderRadius: 6,
          borderColor: '#fff',
          borderWidth: 2
        },
        label: { show: false },
        data: distData
      }
    ]
  }
  pieChartInstance.setOption(option)
}

const handleExportCsv = () => {
  exportLoading.value = true
  const keyword = encodeURIComponent(filters.value.keyword || '')
  const status = filters.value.status === 'ALL' ? '' : encodeURIComponent(filters.value.status)
  const url = `/api/v1/audit/export?keyword=${keyword}&status=${status}`

  const link = document.createElement('a')
  link.href = url
  link.setAttribute('download', 'pacersql_audit_report.csv')
  document.body.appendChild(link)
  link.click()
  document.body.removeChild(link)

  setTimeout(() => {
    exportLoading.value = false
    ElMessage.success('审计日志报表导出已触发下载！')
  }, 800)
}

const goToTicket = (ticketId: number | string) => {
  router.push(`/ticket/${ticketId}`)
}

const showDetailModal = (row: any) => {
  selectedLog.value = row
  detailModalVisible.value = true
}

const handleResize = () => {
  trendChartInstance?.resize()
  pieChartInstance?.resize()
}

onMounted(() => {
  fetchAllData()
  window.addEventListener('resize', handleResize)
})

onBeforeUnmount(() => {
  window.removeEventListener('resize', handleResize)
  trendChartInstance?.dispose()
  pieChartInstance?.dispose()
})
</script>

<style scoped>
.audit-dashboard-container {
  padding: 24px;
}

.metrics-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 16px;
  margin-bottom: 20px;
}

.metric-card {
  display: flex;
  align-items: center;
  padding: 18px 20px;
  border-radius: 10px;
  color: #fff;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.05);
}

.card-blue { background: linear-gradient(135deg, #3b82f6, #1d4ed8); }
.card-green { background: linear-gradient(135deg, #10b981, #047857); }
.card-orange { background: linear-gradient(135deg, #f59e0b, #b45309); }
.card-purple { background: linear-gradient(135deg, #8b5cf6, #6d28d9); }

.card-icon-wrap {
  font-size: 32px;
  margin-right: 16px;
  opacity: 0.9;
}

.card-num {
  font-size: 26px;
  font-weight: 700;
  line-height: 1.1;
}

.card-title {
  font-size: 13px;
  opacity: 0.85;
  margin-top: 4px;
}

.charts-row {
  display: grid;
  grid-template-columns: 2fr 1fr;
  gap: 16px;
  margin-bottom: 20px;
}

.chart-card {
  background: #fff;
  border-radius: 10px;
  padding: 16px 20px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.04);
}

.chart-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}

.chart-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 600;
  color: #1e293b;
  font-size: 15px;
}

.echart-box {
  width: 100%;
  height: 280px;
}

.section-card-wrapper {
  margin-bottom: 20px;
}

.section-header-flex {
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: 12px;
}

.section-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 600;
  color: #1e293b;
  font-size: 15px;
}

.right-filters {
  display: flex;
  align-items: center;
  gap: 10px;
}

.sql-code-text {
  font-family: monospace;
  font-size: 12px;
  color: #334155;
  background: #f1f5f9;
  padding: 2px 6px;
  border-radius: 4px;
}

.ticket-link {
  font-family: monospace;
  font-weight: 600;
  color: #409eff;
  cursor: pointer;
}

.ticket-link:hover {
  text-decoration: underline;
}

.hash-text {
  font-family: monospace;
  font-size: 11px;
  color: #64748b;
}

.pagination-wrap {
  display: flex;
  justify-content: flex-end;
  margin-top: 16px;
}

.log-detail-box {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.detail-item {
  display: flex;
  align-items: flex-start;
}

.detail-item.full-width {
  flex-direction: column;
  gap: 6px;
}

.d-label {
  font-weight: 600;
  color: #475569;
  min-width: 130px;
  font-size: 13px;
}

.d-val {
  color: #1e293b;
  font-size: 13px;
}

.sql-code-block, .error-code-block {
  background: #1e293b;
  color: #f8fafc;
  padding: 12px;
  border-radius: 6px;
  font-size: 12px;
  font-family: monospace;
  overflow-x: auto;
  max-height: 200px;
  white-space: pre-wrap;
  width: 100%;
}

.error-code-block {
  background: #450a0a;
  color: #fca5a5;
}

.hash-block {
  background: #f8fafc;
  border: 1px solid #e2e8f0;
  padding: 6px 8px;
  border-radius: 4px;
  font-size: 11px;
  color: #475569;
  word-break: break-all;
  width: 100%;
}
</style>
