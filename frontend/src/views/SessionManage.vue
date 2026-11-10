<template>
  <div class="session-manage-container page-container">
    <!-- 顶部操作栏 -->
    <div class="header-action">
      <div class="title-area">
        <h2 class="page-title">会话管理 (Process & Session Management)</h2>
        <div class="page-subtitle">监控目标数据库当前活动连接、慢查询与执行中的 SQL 语句，支持一键 Kill 异常会话</div>
      </div>
      <div class="action-area">
        <el-select
          v-model="selectedInstanceId"
          placeholder="请选择目标数据库实例"
          style="width: 280px; margin-right: 12px;"
          @change="fetchSessions"
        >
          <el-option
            v-for="item in instanceList"
            :key="item.id"
            :label="`${item.name} (${item.dbType || 'mysql'})`"
            :value="item.id"
          />
        </el-select>

        <el-switch
          v-model="autoRefresh"
          active-text="自动刷新 (5s)"
          style="margin-right: 12px;"
          @change="handleAutoRefreshChange"
        />

        <el-button :icon="Refresh" :loading="loading" @click="fetchSessions">刷新</el-button>
      </div>
    </div>

    <!-- 统计指标概览 -->
    <el-row :gutter="16" class="metrics-row" v-if="selectedInstanceId">
      <el-col :span="6">
        <div class="metric-card">
          <span class="m-label">当前活跃连接数</span>
          <span class="m-value">{{ sessions.length }} <small>个</small></span>
        </div>
      </el-col>
      <el-col :span="6">
        <div class="metric-card">
          <span class="m-label">正在执行查询 (Query)</span>
          <span class="m-value highlight">{{ queryCount }} <small>个</small></span>
        </div>
      </el-col>
      <el-col :span="6">
        <div class="metric-card">
          <span class="m-label">慢执行会话 (≥5s)</span>
          <span class="m-value danger">{{ slowSessionCount }} <small>个</small></span>
        </div>
      </el-col>
      <el-col :span="6">
        <div class="metric-card">
          <span class="m-label">空闲等待会话 (Sleep)</span>
          <span class="m-value">{{ sleepCount }} <small>个</small></span>
        </div>
      </el-col>
    </el-row>

    <!-- 会话列表表格 -->
    <div class="table-wrapper">
      <el-table :data="pagedSessions" border stripe style="width: 100%" v-loading="loading">
        <el-table-column prop="id" label="Process ID" width="110" align="center">
          <template #default="scope">
            <span style="font-family: monospace; font-weight: 600;">#{{ scope.row.id }}</span>
          </template>
        </el-table-column>

        <el-table-column prop="user" label="连接用户" width="160" show-overflow-tooltip>
          <template #default="scope">
            <div style="display: flex; align-items: center; gap: 4px;">
              <span style="font-weight: 500;">{{ scope.row.user }}</span>
              <el-tag v-if="isSystemUser(scope.row.user)" size="small" type="warning" effect="plain" style="font-size: 11px; padding: 0 4px;">云/系统</el-tag>
            </div>
          </template>
        </el-table-column>

        <el-table-column prop="host" label="客户端主机 (Host:Port)" width="180" show-overflow-tooltip>
          <template #default="scope">
            <span style="font-family: monospace; font-size: 12px;">{{ scope.row.host }}</span>
          </template>
        </el-table-column>

        <el-table-column prop="db" label="数据库 (DB)" width="140" show-overflow-tooltip>
          <template #default="scope">
            <el-tag size="small" type="info" effect="plain">{{ scope.row.db || '-' }}</el-tag>
          </template>
        </el-table-column>

        <el-table-column prop="command" label="命令 (Command)" width="120" align="center">
          <template #default="scope">
            <el-tag size="small" :type="scope.row.command === 'Query' ? 'danger' : 'info'">
              {{ scope.row.command }}
            </el-tag>
          </template>
        </el-table-column>

        <el-table-column prop="time" label="耗时 (Time)" width="110" align="center" sortable>
          <template #default="scope">
            <span :style="{ fontWeight: '600', color: scope.row.time >= 5 ? '#F56C6C' : '#303133' }">
              {{ scope.row.time }}s
            </span>
          </template>
        </el-table-column>

        <el-table-column prop="state" label="状态 (State)" width="140" show-overflow-tooltip />

        <el-table-column prop="info" label="正在执行的 SQL 语句 (Info)" min-width="260" show-overflow-tooltip>
          <template #default="scope">
            <span v-if="scope.row.info" style="font-family: monospace; font-size: 12px; color: #409EFF;">
              {{ scope.row.info }}
            </span>
            <span v-else style="color: #c0c4cc; font-size: 12px;">(空闲)</span>
          </template>
        </el-table-column>

        <el-table-column label="操作" width="130" fixed="right" align="center">
          <template #default="scope">
            <el-button
              size="small"
              type="danger"
              plain
              :icon="CloseBold"
              @click="handleKillSession(scope.row)"
            >
              Kill 进程
            </el-button>
          </template>
        </el-table-column>
      </el-table>

      <!-- 分页控制栏 -->
      <div class="pagination-bar" style="display: flex; justify-content: flex-end; margin-top: 16px;">
        <el-pagination
          v-model:current-page="currentPage"
          v-model:page-size="pageSize"
          :page-sizes="[10, 20, 50, 100]"
          :total="sessions.length"
          layout="total, sizes, prev, pager, next, jumper"
          background
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Refresh, CloseBold } from '@element-plus/icons-vue'
import request from '../utils/request'

interface SessionItem {
  id: number
  user: string
  host: string
  db: string
  command: string
  time: number
  state: string
  info: string
}

const loading = ref(false)
const autoRefresh = ref(false)
const timer = ref<any>(null)
const selectedInstanceId = ref<number | null>(null)
const instanceList = ref<any[]>([])
const sessions = ref<SessionItem[]>([])

const currentPage = ref(1)
const pageSize = ref(10)

const pagedSessions = computed(() => {
  const start = (currentPage.value - 1) * pageSize.value
  return sessions.value.slice(start, start + pageSize.value)
})

const queryCount = computed(() => {
  return sessions.value.filter(s => s.command === 'Query').length
})

const slowSessionCount = computed(() => {
  return sessions.value.filter(s => s.time >= 5 && s.command === 'Query').length
})

const sleepCount = computed(() => {
  return sessions.value.filter(s => s.command === 'Sleep').length
})

const isSystemUser = (user: string) => {
  if (!user) return false
  const u = user.toLowerCase()
  return u === 'aliyun_root' || u === 'system user' || u === 'event_scheduler' || u.startsWith('rds_')
}

const fetchInstances = async () => {
  try {
    const res: any = await request.get('/v1/instance/list')
    instanceList.value = Array.isArray(res.data) ? res.data : []
    if (instanceList.value.length > 0 && !selectedInstanceId.value) {
      selectedInstanceId.value = instanceList.value[0].id
      fetchSessions()
    }
  } catch (error) {
    ElMessage.error('获取实例列表失败')
  }
}

const fetchSessions = async () => {
  if (!selectedInstanceId.value) return
  loading.value = true
  try {
    const res: any = await request.get(`/v1/instance/${selectedInstanceId.value}/sessions`)
    sessions.value = Array.isArray(res.data) ? res.data : []
  } catch (error) {
    ElMessage.error('获取活动会话列表失败')
  } finally {
    loading.value = false
  }
}

const handleAutoRefreshChange = (val: boolean) => {
  if (timer.value) {
    clearInterval(timer.value)
    timer.value = null
  }
  if (val) {
    timer.value = setInterval(() => {
      fetchSessions()
    }, 5000)
  }
}

const handleKillSession = async (row: SessionItem) => {
  try {
    await ElMessageBox.confirm(
      `确认强制终止会话 #${row.id}（用户: ${row.user}，执行命令: ${row.command}，耗时: ${row.time}s）吗？`,
      '终止会话确认',
      {
        type: 'warning',
        confirmButtonText: '确定 Kill',
        cancelButtonText: '取消',
        confirmButtonClass: 'el-button--danger'
      }
    )

    await request.post(`/v1/instance/${selectedInstanceId.value}/sessions/${row.id}/kill`)
    ElMessage.success(`会话 #${row.id} 已成功终止`)
    fetchSessions()
  } catch (err: any) {
    if (err !== 'cancel') {
      ElMessage.error(err.response?.data?.message || err.message || '终止会话失败')
    }
  }
}

onMounted(() => {
  fetchInstances()
})

onUnmounted(() => {
  if (timer.value) {
    clearInterval(timer.value)
  }
})
</script>

<style scoped>
.header-action {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
  flex-wrap: wrap;
  gap: 12px;
}

.page-subtitle {
  font-size: 13px;
  color: #909399;
  margin-top: 4px;
}

.action-area {
  display: flex;
  align-items: center;
}

.metrics-row {
  margin-bottom: 20px;
}

.metric-card {
  background: #ffffff;
  padding: 14px 18px;
  border-radius: 8px;
  border: 1px solid #e4e7ed;
  display: flex;
  flex-direction: column;
}

.metric-card .m-label {
  font-size: 12px;
  color: #909399;
}

.metric-card .m-value {
  font-size: 22px;
  font-weight: 700;
  color: #303133;
  margin-top: 6px;
}

.metric-card .m-value.highlight {
  color: #409EFF;
}

.metric-card .m-value.danger {
  color: #F56C6C;
}

.metric-card .m-value small {
  font-size: 13px;
  font-weight: normal;
  color: #909399;
}

.table-wrapper {
  background: #ffffff;
  border-radius: 8px;
  overflow-x: auto;
}
</style>
