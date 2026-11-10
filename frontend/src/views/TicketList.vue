<template>
  <div class="ticket-list-container page-container">
    <!-- 页头：标题 + 主操作 -->
    <div class="list-hero">
      <div class="hero-left">
        <div class="hero-icon"><el-icon :size="22"><Tickets /></el-icon></div>
        <div>
          <h2 class="hero-title">SQL 工单中心</h2>
          <div class="hero-subtitle">SQL 变更与安全审批流转 · 全生命周期追踪 · 多维组合筛选</div>
        </div>
      </div>
      <div class="hero-actions">
        <el-tooltip content="在新窗口创建工单" placement="bottom">
          <el-button :icon="CopyDocument" circle @click="handleOpenNewWindow" />
        </el-tooltip>
        <el-button :icon="Refresh" :loading="loading" @click="loadTickets">刷新</el-button>
        <el-button type="primary" :icon="Plus" @click="handleGoCreate">新建工单</el-button>
      </div>
    </div>

    <!-- KPI 状态概览卡片 (点击即可快速筛选) -->
    <div class="kpi-grid">
      <div
        v-for="k in kpiCards"
        :key="k.key"
        class="kpi-card"
        :class="[`kpi-${k.tone}`, { 'is-active': activeStatusTab === k.key }]"
        @click="activeStatusTab = activeStatusTab === k.key ? 'ALL' : k.key"
      >
        <div class="kpi-top">
          <span class="kpi-label">{{ k.label }}</span>
          <span class="kpi-icon">{{ k.icon }}</span>
        </div>
        <div class="kpi-value">{{ k.value }}</div>
        <div class="kpi-foot">{{ activeStatusTab === k.key ? '已筛选 · 再次点击取消' : k.hint }}</div>
      </div>
    </div>

    <!-- 筛选面板：视角 + 状态 + 组合条件 -->
    <div class="filter-panel">
      <div class="filter-row-top">
        <div class="scope-chip" v-if="dataScopeInfo">
          <el-tag :type="getDataScopeTagType(dataScopeInfo.scope)" effect="dark" size="small" round>
            {{ dataScopeInfo.scopeName || '数据权限范围' }}
          </el-tag>
          <span class="scope-text">{{ dataScopeInfo.description }}</span>
        </div>
        <div class="perspective-seg">
          <button
            v-for="p in perspectiveOptions"
            :key="p.value"
            type="button"
            class="seg-btn"
            :class="{ active: userPerspectiveFilter === p.value }"
            @click="userPerspectiveFilter = p.value"
          >
            {{ p.label }}<span class="seg-count">{{ p.count }}</span>
          </button>
        </div>
      </div>

      <div class="status-chip-row">
        <button
          v-for="s in statusTabs"
          :key="s.key"
          type="button"
          class="status-chip"
          :class="[`chip-${s.tone}`, { active: activeStatusTab === s.key }]"
          @click="handleTabChange(s.key)"
        >
          <span class="chip-dot"></span>
          {{ s.label }}
          <span class="chip-count">{{ (statusCounts as any)[s.key] ?? 0 }}</span>
        </button>
      </div>

      <div class="filter-toolbar">
        <el-input
          v-model="searchKeyword"
          placeholder="搜索工单号 / 申请人 / 说明 / 实例 / 数据库"
          clearable
          :prefix-icon="Search"
          class="filter-search"
        />
        <el-select v-model="filterResourceGroup" placeholder="业务资源组" clearable class="filter-select">
          <el-option v-for="rg in availableResourceGroups" :key="rg" :label="rg" :value="rg" />
        </el-select>
        <el-select v-model="filterInstanceId" placeholder="数据库实例" clearable filterable class="filter-select">
          <el-option
            v-for="inst in instances"
            :key="inst.id"
            :label="`${inst.name} (${inst.env || 'PROD'})`"
            :value="inst.id"
          />
        </el-select>
        <el-select v-model="filterType" placeholder="工单类型" clearable class="filter-select filter-select-sm">
          <el-option label="SQL 变更审核" value="SQL_AUDIT" />
          <el-option label="DML 数据变更" value="DML_CHANGE" />
          <el-option label="DDL 结构变更" value="DDL_CHANGE" />
          <el-option label="敏感数据导出" value="DATA_EXPORT" />
          <el-option label="权限申请" value="PERMISSION" />
          <el-option label="账号申请" value="ACCOUNT" />
          <el-option label="库表申请" value="DB_TABLE" />
          <el-option label="应急数据恢复" value="DATA_RECOVERY" />
        </el-select>
        <el-button v-if="activeFilterCount > 0" :icon="RefreshRight" @click="handleResetFilter">
          清除筛选 ({{ activeFilterCount }})
        </el-button>
      </div>
    </div>

    <!-- 工单列表 -->
    <div class="table-panel">
      <div class="table-panel-header">
        <div class="tp-left">
          <span class="tp-title">工单列表</span>
          <span class="tp-count">共 <b>{{ filteredTickets.length }}</b> 条</span>
        </div>
        <span class="tp-tip">点击任意行查看详情与审批</span>
      </div>

      <div class="table-wrapper">
        <el-table
          :data="pagedTickets"
          style="width: 100%"
          v-loading="loading"
          row-class-name="ticket-row"
          :header-cell-style="{ background: '#f8fafc', color: '#475569', fontWeight: 700, fontSize: '12.5px' }"
          @row-click="(row: any) => viewDetail(row.id)"
        >
          <template #empty>
            <div class="empty-state">
              <div class="empty-icon">🗂️</div>
              <div class="empty-title">{{ tickets.length === 0 ? '还没有任何工单' : '没有符合条件的工单' }}</div>
              <div class="empty-desc">
                {{ tickets.length === 0 ? '发起第一张 SQL 变更工单，系统将自动匹配审批流。' : '尝试调整状态、视角或关键字等筛选条件。' }}
              </div>
              <el-button v-if="tickets.length === 0" type="primary" :icon="Plus" @click.stop="handleGoCreate">新建工单</el-button>
              <el-button v-else :icon="RefreshRight" @click.stop="handleResetFilter">清除全部筛选</el-button>
            </div>
          </template>

          <!-- 工单号 -->
          <el-table-column label="工单号" prop="id" width="110" fixed="left">
            <template #default="{ row }">
              <span class="ticket-id-badge">#{{ row.id }}</span>
            </template>
          </el-table-column>

          <!-- 申请说明 + 类型 -->
          <el-table-column label="申请说明" min-width="260">
            <template #default="{ row }">
              <div class="reason-cell">
                <div class="reason-text" :title="row.reason || ''">{{ row.reason || '无申请说明' }}</div>
                <div class="reason-meta">
                  <span class="type-chip" :class="`type-${getTicketTypeTagType(row.type)}`">{{ getTicketTypeLabel(row.type) }}</span>
                  <span class="meta-dept" :title="getInstanceDeptOrSystem(row)">{{ getInstanceDeptOrSystem(row) }}</span>
                </div>
              </div>
            </template>
          </el-table-column>

          <!-- 申请人 -->
          <el-table-column label="申请人" min-width="120" show-overflow-tooltip>
            <template #default="{ row }">
              <div class="applicant-cell">
                <el-avatar :size="26" class="applicant-avatar">
                  {{ getAvatarInitial(row.applicantName || row.applicantIdCard) }}
                </el-avatar>
                <span class="applicant-name-text">{{ cleanApplicantName(row.applicantName || row.applicantIdCard) }}</span>
              </div>
            </template>
          </el-table-column>

          <!-- 目标实例 / 数据库 -->
          <el-table-column label="目标实例 / 数据库" min-width="200">
            <template #default="{ row }">
              <div class="target-cell">
                <div class="target-line">
                  <span class="env-chip" :class="(getInstanceObj(row.instanceId)?.env || 'PROD') === 'PROD' ? 'env-prod' : 'env-other'">
                    {{ getInstanceObj(row.instanceId)?.env || 'PROD' }}
                  </span>
                  <span class="instance-name-text" :title="getInstanceName(row.instanceId)">
                    {{ getInstanceObj(row.instanceId)?.name || getInstanceName(row.instanceId) }}
                  </span>
                </div>
                <div class="db-line">
                  <span v-if="row.dbName" class="db-name">🗃️ {{ row.dbName }}</span>
                  <span v-else class="db-empty">全局 / 默认库</span>
                </div>
              </div>
            </template>
          </el-table-column>

          <!-- 资源组 -->
          <el-table-column label="资源组" min-width="140" show-overflow-tooltip>
            <template #default="{ row }">
              <span class="rg-chip">{{ getInstanceResourceGroup(row) }}</span>
            </template>
          </el-table-column>

          <!-- 状态 -->
          <el-table-column prop="status" label="流程状态" min-width="170">
            <template #default="{ row }">
              <div class="status-cell">
                <span class="status-pill" :class="`st-${getStatusType(row.status)}`">
                  <span class="st-dot"></span>{{ getStatusLabel(row.status) }}
                </span>
                <span v-if="row.status === 'AUDITING' || row.status === 'PENDING_APPROVAL'" class="status-sub">
                  {{ getPendingApprovalRole(row).icon }} 待{{ getPendingApprovalRole(row).roleName }}
                </span>
                <span v-else-if="row.status === 'WAITING_EXECUTION'" class="status-sub status-sub-warn">
                  ⏰ {{ formatExecutionWindow(row.executionWindow) }}
                </span>
                <span v-else-if="row.status === 'MANUAL_PROCESSING'" class="status-sub status-sub-warn">
                  🛠️ 待 DBA 线下执行
                </span>
              </div>
            </template>
          </el-table-column>

          <!-- 提交时间 -->
          <el-table-column label="提交时间" width="150">
            <template #default="{ row }">
              <div class="time-cell">
                <span class="time-rel">{{ formatRelativeTime(row.createTime || formatTicketTime(row.id)) }}</span>
                <span class="time-abs">{{ row.createTime || formatTicketTime(row.id) }}</span>
              </div>
            </template>
          </el-table-column>

          <!-- 操作 -->
          <el-table-column label="操作" width="200" fixed="right">
            <template #default="{ row }">
              <div class="row-actions" @click.stop>
                <el-button size="small" type="primary" link :icon="Tickets" @click="viewDetail(row.id)">
                  {{ ['AUDITING', 'PENDING_APPROVAL'].includes(row.status) ? '审批' : '查看' }}
                </el-button>
                <el-button
                  v-if="['AUDITING', 'PENDING_APPROVAL', 'SUBMITTED'].includes(row.status)"
                  size="small"
                  type="danger"
                  link
                  :icon="Bell"
                  :loading="urgingMap[row.id]"
                  :disabled="(urgeCooldownMap[row.id] || 0) > 0"
                  @click="handleUrgeTicket(row)"
                >
                  {{ (urgeCooldownMap[row.id] || 0) > 0 ? `${urgeCooldownMap[row.id]}s` : '催办' }}
                </el-button>
                <el-button
                  v-if="isMySubmittedTicket(row) && (row.status === 'AUDITING' || row.status === 'PENDING_APPROVAL')"
                  size="small"
                  type="warning"
                  link
                  :icon="Back"
                  @click="handleWithdrawTicket(row)"
                >
                  撤回
                </el-button>
                <el-button
                  v-if="['TERMINATED', 'FAILED', 'REJECTED', 'EXECUTED'].includes(row.status)"
                  size="small"
                  type="success"
                  link
                  :icon="RefreshRight"
                  @click="cloneTicket(row.id)"
                >
                  再来一单
                </el-button>
              </div>
            </template>
          </el-table-column>
        </el-table>
      </div>

      <div class="pagination-bar">
        <el-pagination
          v-model:current-page="currentPage"
          v-model:page-size="pageSize"
          :page-sizes="[10, 20, 50, 100]"
          :total="filteredTickets.length"
          layout="total, sizes, prev, pager, next, jumper"
          background
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import request from '../utils/request'
import { useUserStore } from '../store/user'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, Refresh, CopyDocument, Search, RefreshRight, Tickets, Back, Bell } from '@element-plus/icons-vue'

const router = useRouter()
const route = useRoute()
const userStore = useUserStore()

const tickets = ref<any[]>([])
const instances = ref<any[]>([])
const loading = ref(false)

// 分页状态
const currentPage = ref(1)
const pageSize = ref(10)

// 数据权限范围元数据
const dataScopeInfo = ref<any>({
  scope: 'RESOURCE_GROUP',
  scopeName: '所属业务资源组工单',
  description: '仅展示归属业务资源组相关及本人发起的工单'
})

// 用户视角筛选：ALL (全部权限工单), MY_SUBMIT (我发起的), PENDING_ME (待我初审/复核)
const userPerspectiveFilter = ref('ALL')

// 筛选字段
const activeStatusTab = ref('ALL')
const filterResourceGroup = ref('')
const filterInstanceId = ref<any>('')
const filterType = ref('')
const searchKeyword = ref('')
const availableResourceGroups = ref<string[]>([])

const getDataScopeTagType = (scope: string) => {
  if (scope === 'ALL') return 'danger'
  if (scope === 'RESOURCE_GROUP') return 'warning'
  return 'primary'
}

const loadResourceGroups = async () => {
  try {
    const res: any = await request.get('/v1/resource-group/list')
    const list = Array.isArray(res.data) ? res.data : []
    availableResourceGroups.value = list.map((rg: any) => rg.groupName).filter(Boolean)
  } catch (e) {
    availableResourceGroups.value = ['车险承保资源组', '销管系统资源组']
  }
}

const loadDataScopeInfo = async () => {
  try {
    const res: any = await request.get('/v1/ticket/data-scope')
    if (res.data) {
      dataScopeInfo.value = res.data
    }
  } catch (e) {
    // fallback
  }
}

const loadInstances = async () => {
  try {
    const res: any = await request.get('/v1/instance/list')
    instances.value = Array.isArray(res.data) ? res.data : []
  } catch (e) {
    // ignore
  }
}

const getInstanceObj = (id: number | string) => {
  return instances.value.find(i => String(i.id) === String(id)) || null
}

const getInstanceName = (id: number | string) => {
  const found = getInstanceObj(id)
  return found ? `${found.name} (${found.env || 'PROD'})` : `数据库实例 #${id}`
}

const getInstanceResourceGroup = (ticket: any) => {
  if (ticket.resourceGroup) return ticket.resourceGroup
  const inst = getInstanceObj(ticket.instanceId)
  if (inst?.resourceGroups) {
    try {
      const arr = JSON.parse(inst.resourceGroups)
      if (Array.isArray(arr) && arr.length > 0) return arr.join('、')
    } catch (e) {
      return inst.resourceGroups
    }
  }
  return '默认核心业务资源组'
}

const getInstanceDeptOrSystem = (ticket: any) => {
  const inst = getInstanceObj(ticket.instanceId)
  if (inst?.instanceConfig) {
    try {
      const conf = JSON.parse(inst.instanceConfig)
      if (conf.systemName || conf.department) {
        return `${conf.department || '产险研发中心'} · ${conf.systemName || '核心业务系统'}`
      }
    } catch (e) {}
  }
  if (inst?.description && (inst.description.includes('系统') || inst.description.includes('部'))) {
    return inst.description
  }
  const rg = getInstanceResourceGroup(ticket)
  if (rg.includes('车险') || (inst?.name && inst.name.includes('车险'))) {
    return '产险研发部 · 车险核心承保系统'
  }
  if (rg.includes('销管') || (inst?.name && inst.name.includes('销管'))) {
    return '渠道营销技术部 · 综合销管业务系统'
  }
  if (rg.includes('理赔') || (inst?.name && inst.name.includes('理赔'))) {
    return '运营理赔技术部 · 智能理赔核心系统'
  }
  return '基础架构部 · 企业核心数据中台'
}

const getPendingApprovalRole = (ticket: any) => {
  const type = ticket.type || 'SQL_AUDIT'
  const reason = (ticket.reason || '').toUpperCase()
  const hasDdl = type === 'DDL' || type === 'DDL_CHANGE' || reason.includes('CREATE') || reason.includes('ALTER') || reason.includes('DROP') || reason.includes('TRUNCATE')
  const isHighRisk = hasDdl || (ticket.affectRowsEstimate && ticket.affectRowsEstimate > 1000)
  if (isHighRisk) {
    return {
      roleKey: 'DBA',
      roleName: '核心 DBA 安全复核',
      tagType: 'danger',
      icon: '🛡️'
    }
  }
  return {
    roleKey: 'DEV_LEAD',
    roleName: '业务开发组长初审',
    tagType: 'warning',
    icon: '👤'
  }
}

const formatExecutionWindow = (win?: string) => {
  if (!win) return '计划窗口待调度'
  return win.replace('scheduled:', '').replace('scheduled', '').trim()
}

const isMySubmittedTicket = (ticket: any) => {
  if (!userStore.userInfo) return false
  const myIdCard = userStore.userInfo.idCard
  const myUsername = userStore.userInfo.username
  const myRealName = userStore.userInfo.realName
  const app = ticket.applicantIdCard || ''
  return app === myIdCard || app === myUsername || app === myRealName
}

const mySubmitCount = computed(() => {
  return tickets.value.filter(isMySubmittedTicket).length
})

const pendingMeCount = computed(() => {
  return tickets.value.filter((t: any) => {
    const isAuditing = t.status === 'AUDITING' || t.status === 'PENDING_APPROVAL' || t.status === 'MANUAL_PROCESSING'
    return isAuditing
  }).length
})

const loadTickets = async () => {
  loading.value = true
  try {
    const res: any = await request.get('/v1/ticket/list')
    tickets.value = Array.isArray(res.data) ? res.data : []
  } catch (error) {
    ElMessage.error('加载工单列表失败')
  } finally {
    loading.value = false
  }
}

const handleGoCreate = () => {
  router.push('/ticket-create')
}

const handleOpenNewWindow = () => {
  const href = router.resolve('/ticket-create').href
  window.open(href, '_blank')
}

const viewDetail = (id: number | string) => {
  router.push(`/ticket/${id}`)
}

const handleTabChange = (tabName: any) => {
  activeStatusTab.value = tabName
}

const cleanApplicantName = (name?: string) => {
  if (!name) return '系统'
  return name.replace(/[\(（].*?[\)）]/g, '').trim() || name
}

const getAvatarInitial = (name?: string) => {
  const clean = cleanApplicantName(name)
  return clean ? clean.charAt(0) : '用'
}

const handleResetFilter = () => {
  activeStatusTab.value = 'ALL'
  userPerspectiveFilter.value = 'ALL'
  filterResourceGroup.value = ''
  filterInstanceId.value = ''
  filterType.value = ''
  searchKeyword.value = ''
}

// 统计各状态工单数量
const statusCounts = computed(() => {
  const counts = {
    ALL: tickets.value.length,
    AUDITING: 0,
    MANUAL_PROCESSING: 0,
    WAITING_EXECUTION: 0,
    APPROVED_OR_EXEC: 0,
    EXECUTED: 0,
    REJECTED: 0,
    FAILED: 0
  }

  for (const t of tickets.value) {
    const st = t.status
    if (st === 'AUDITING' || st === 'PENDING_APPROVAL') counts.AUDITING++
    else if (st === 'MANUAL_PROCESSING') counts.MANUAL_PROCESSING++
    else if (st === 'WAITING_EXECUTION') counts.WAITING_EXECUTION++
    else if (st === 'APPROVED' || st === 'EXECUTING') counts.APPROVED_OR_EXEC++
    else if (st === 'EXECUTED') counts.EXECUTED++
    else if (st === 'REJECTED') counts.REJECTED++
    else if (st === 'FAILED') counts.FAILED++
  }

  return counts
})

// 多维组合筛选逻辑
const filteredTickets = computed(() => {
  return tickets.value.filter((item: any) => {
    // 0. 用户视角快捷筛选 (ALL / MY_SUBMIT / PENDING_ME)
    if (userPerspectiveFilter.value === 'MY_SUBMIT') {
      if (!isMySubmittedTicket(item)) return false
    } else if (userPerspectiveFilter.value === 'PENDING_ME') {
      const isPending = item.status === 'AUDITING' || item.status === 'PENDING_APPROVAL' || item.status === 'MANUAL_PROCESSING'
      if (!isPending) return false
    }

    // 1. 状态 Tab 筛选
    if (activeStatusTab.value === 'AUDITING') {
      if (item.status !== 'AUDITING' && item.status !== 'PENDING_APPROVAL') return false
    } else if (activeStatusTab.value === 'MANUAL_PROCESSING') {
      if (item.status !== 'MANUAL_PROCESSING') return false
    } else if (activeStatusTab.value === 'WAITING_EXECUTION') {
      if (item.status !== 'WAITING_EXECUTION') return false
    } else if (activeStatusTab.value === 'APPROVED_OR_EXEC') {
      if (item.status !== 'APPROVED' && item.status !== 'EXECUTING') return false
    } else if (activeStatusTab.value === 'EXECUTED') {
      if (item.status !== 'EXECUTED') return false
    } else if (activeStatusTab.value === 'REJECTED') {
      if (item.status !== 'REJECTED') return false
    } else if (activeStatusTab.value === 'FAILED') {
      if (item.status !== 'FAILED') return false
    }

    // 1.5 业务资源组筛选 (与列表展示口径一致：工单字段优先，其次回退实例绑定的资源组)
    if (filterResourceGroup.value && !getInstanceResourceGroup(item).includes(filterResourceGroup.value)) {
      return false
    }

    // 2. 目标实例筛选
    if (filterInstanceId.value && String(item.instanceId) !== String(filterInstanceId.value)) {
      return false
    }

    // 3. 工单类型筛选
    if (filterType.value && item.type !== filterType.value) {
      return false
    }

    // 4. 关键字搜索
    if (searchKeyword.value && searchKeyword.value.trim()) {
      const q = searchKeyword.value.trim().toLowerCase()
      const matchId = String(item.id).includes(q)
      const matchReason = item.reason && item.reason.toLowerCase().includes(q)
      const matchApplicant = (item.applicantIdCard && item.applicantIdCard.toLowerCase().includes(q)) || (item.applicantName && item.applicantName.toLowerCase().includes(q))
      const matchKey = item.businessKey && item.businessKey.toLowerCase().includes(q)
      const matchInst = getInstanceName(item.instanceId).toLowerCase().includes(q) || (item.dbName && String(item.dbName).toLowerCase().includes(q))
      if (!matchId && !matchReason && !matchApplicant && !matchKey && !matchInst) {
        return false
      }
    }

    return true
  })
})

const pagedTickets = computed(() => {
  const start = (currentPage.value - 1) * pageSize.value
  return filteredTickets.value.slice(start, start + pageSize.value)
})

// 状态快捷筛选标签
const statusTabs = [
  { key: 'ALL', label: '全部', tone: 'info' },
  { key: 'AUDITING', label: '待初审 / 复核', tone: 'warning' },
  { key: 'MANUAL_PROCESSING', label: '待 DBA 线下反馈', tone: 'orange' },
  { key: 'WAITING_EXECUTION', label: '待定时触发', tone: 'primary' },
  { key: 'APPROVED_OR_EXEC', label: '审批通过 / 执行中', tone: 'primary' },
  { key: 'EXECUTED', label: '已执行归档', tone: 'success' },
  { key: 'REJECTED', label: '已驳回', tone: 'info' },
  { key: 'FAILED', label: '执行失败', tone: 'danger' }
]

// KPI 概览卡片
const kpiCards = computed(() => [
  { key: 'AUDITING', label: '待审批', icon: '⏳', tone: 'warning', value: statusCounts.value.AUDITING, hint: '初审 / 复核流转中' },
  { key: 'MANUAL_PROCESSING', label: '待线下执行', icon: '🛠️', tone: 'orange', value: statusCounts.value.MANUAL_PROCESSING, hint: '等待 DBA 反馈结果' },
  { key: 'APPROVED_OR_EXEC', label: '已通过 / 执行中', icon: '⚡', tone: 'primary', value: statusCounts.value.APPROVED_OR_EXEC + statusCounts.value.WAITING_EXECUTION, hint: '含定时等待触发' },
  { key: 'EXECUTED', label: '已执行归档', icon: '✅', tone: 'success', value: statusCounts.value.EXECUTED, hint: '变更已完成归档' },
  { key: 'FAILED', label: '执行失败', icon: '⚠️', tone: 'danger', value: statusCounts.value.FAILED, hint: `另有驳回 ${statusCounts.value.REJECTED} 单` }
])

// 视角筛选选项
const perspectiveOptions = computed(() => [
  { value: 'ALL', label: '全部权限工单', count: tickets.value.length },
  { value: 'MY_SUBMIT', label: '我发起的', count: mySubmitCount.value },
  { value: 'PENDING_ME', label: '待处理', count: pendingMeCount.value }
])

// 当前生效的筛选条件数量
const activeFilterCount = computed(() => {
  let n = 0
  if (activeStatusTab.value !== 'ALL') n++
  if (userPerspectiveFilter.value !== 'ALL') n++
  if (filterResourceGroup.value) n++
  if (filterInstanceId.value) n++
  if (filterType.value) n++
  if (searchKeyword.value && searchKeyword.value.trim()) n++
  return n
})

// 相对时间展示 (刚刚 / x 分钟前 / x 小时前 / x 天前)
const formatRelativeTime = (timeStr: string) => {
  if (!timeStr || timeStr === '-') return '-'
  const t = new Date(String(timeStr).replace(/-/g, '/')).getTime()
  if (!t || Number.isNaN(t)) return String(timeStr).slice(0, 10)
  const diff = Date.now() - t
  if (diff < 0) return String(timeStr).slice(5, 16)
  const min = Math.floor(diff / 60000)
  if (min < 1) return '刚刚'
  if (min < 60) return `${min} 分钟前`
  const hr = Math.floor(min / 60)
  if (hr < 24) return `${hr} 小时前`
  const day = Math.floor(hr / 24)
  if (day < 30) return `${day} 天前`
  return String(timeStr).slice(0, 10)
}

watch(() => [activeStatusTab.value, userPerspectiveFilter.value, filterResourceGroup.value, filterInstanceId.value, filterType.value, searchKeyword.value], () => {
  currentPage.value = 1
})

const urgingMap = ref<Record<number, boolean>>({})
const urgeCooldownMap = ref<Record<number, number>>({})

const startUrgeCooldown = (ticketId: number, seconds = 60) => {
  urgeCooldownMap.value[ticketId] = seconds
  const timer = setInterval(() => {
    if ((urgeCooldownMap.value[ticketId] || 0) <= 1) {
      delete urgeCooldownMap.value[ticketId]
      clearInterval(timer)
    } else {
      urgeCooldownMap.value[ticketId]--
    }
  }, 1000)
}

const handleUrgeTicket = async (row: any) => {
  if ((urgeCooldownMap.value[row.id] || 0) > 0) {
    ElMessage.warning(`您刚已发起过催办，请等待 ${urgeCooldownMap.value[row.id]} 秒后再次催办`)
    return
  }

  try {
    const { value: reason } = await ElMessageBox.prompt(
      `即将向工单 #${row.id}（目标库: ${row.dbName || '默认库'}）当前节点的所有待审批责任人发送加急催办通知（支持企业微信工作消息、飞书富文本互动卡片、阿里钉钉）。\n\n您可补充催办加急说明：`,
      '⏰ 工单审批加急催办',
      {
        confirmButtonText: '立即发送催办通知',
        cancelButtonText: '取消',
        inputPlaceholder: '如：生产上线窗口临近，劳烦领导尽快协助审批！',
        inputValue: '生产上线窗口临近，劳烦领导尽快协助审批！',
        type: 'warning'
      }
    )

    urgingMap.value[row.id] = true
    const res: any = await request.post(`/v1/ticket/${row.id}/urge`, { reason: reason || '请尽快审批' })
    const msg = res?.data?.message || '加急催办通知已通过启用的企业微信、飞书、钉钉成功下发给当前待审批人！'
    ElMessage.success(msg)
    startUrgeCooldown(row.id, 60)
  } catch (err: any) {
    if (err !== 'cancel') {
      ElMessage.error(err?.response?.data?.message || err?.message || '发起催办失败')
    }
  } finally {
    urgingMap.value[row.id] = false
  }
}

const handleWithdrawTicket = async (row: any) => {
  try {
    await ElMessageBox.confirm(
      `确定要撤回工单 #${row.id} 吗？\n撤回后当前审批流将立即终止，并将自动载入该工单历史数据返回创建页面供您再次编辑。`,
      '撤回工单确认',
      {
        confirmButtonText: '确认撤回并编辑',
        cancelButtonText: '取消',
        type: 'warning'
      }
    )
    await request.post(`/v1/ticket/${row.id}/withdraw`)
    ElMessage.success('工单已成功撤回，正在为您加载原数据进入编辑模式...')
    router.push({ path: '/ticket-create', query: { fromTicketId: row.id } })
  } catch (err: any) {
    if (err !== 'cancel') {
      ElMessage.error(err.message || '撤回工单失败')
    }
  }
}

const cloneTicket = (id: string | number) => {
  router.push({ path: '/ticket-create', query: { fromTicketId: id } })
}

const getStatusType = (status: string) => {
  switch (status) {
    case 'APPROVED': return 'success'
    case 'EXECUTED': return 'success'
    case 'AUDITING': return 'warning'
    case 'PENDING_APPROVAL': return 'warning'
    case 'MANUAL_PROCESSING': return 'warning'
    case 'WAITING_EXECUTION': return 'primary'
    case 'EXECUTING': return 'primary'
    case 'REJECTED': return 'danger'
    case 'FAILED': return 'danger'
    case 'TERMINATED': return 'info'
    default: return 'info'
  }
}

const getStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    'AUDITING': '待初审/复核',
    'PENDING_APPROVAL': '待初审/复核',
    'APPROVED': '已审批通过',
    'WAITING_EXECUTION': '待定时触发',
    'MANUAL_PROCESSING': '待DBA线下反馈',
    'EXECUTING': '正在流式执行',
    'EXECUTED': '已执行归档',
    'REJECTED': '已驳回',
    'FAILED': '执行失败',
    'TERMINATED': '已终止'
  }
  return map[status] || status
}

const getTicketTypeLabel = (type: string) => {
  const map: Record<string, string> = {
    'SQL_AUDIT': 'SQL 变更审核',
    'DML_CHANGE': 'DML 数据变更',
    'DDL_CHANGE': 'DDL 结构变更',
    'DATA_EXPORT': '敏感数据导出',
    'PERMISSION': '权限申请',
    'ACCOUNT': '账号申请',
    'DB_TABLE': '库表申请',
    'DATA_RECOVERY': '应急数据恢复'
  }
  return map[type] || type || 'SQL 变更审核'
}

const getTicketTypeTagType = (type: string) => {
  switch (type) {
    case 'SQL_AUDIT':
    case 'DML_CHANGE':
      return 'primary'
    case 'DDL_CHANGE':
      return 'danger'
    case 'DATA_EXPORT':
      return 'warning'
    case 'DATA_RECOVERY':
      return 'danger'
    default:
      return 'info'
  }
}

const formatTicketTime = (timeOrId: any) => {
  if (!timeOrId) return '-'
  const str = String(timeOrId).trim()
  if (str.includes('-') && str.includes(':')) {
    return str
  }
  if (str.length >= 14 && (str.startsWith('2025') || str.startsWith('2026') || str.startsWith('2027'))) {
    const y = str.substring(0, 4)
    const m = str.substring(4, 6)
    const d = str.substring(6, 8)
    const h = str.substring(8, 10)
    const mi = str.substring(10, 12)
    const s = str.substring(12, 14)
    return `${y}-${m}-${d} ${h}:${mi}:${s}`
  }
  const num = Number(str)
  if (num && num > 1000000000000) {
    const d = new Date(num)
    const year = d.getFullYear()
    const month = String(d.getMonth() + 1).padStart(2, '0')
    const day = String(d.getDate()).padStart(2, '0')
    const hours = String(d.getHours()).padStart(2, '0')
    const minutes = String(d.getMinutes()).padStart(2, '0')
    const seconds = String(d.getSeconds()).padStart(2, '0')
    return `${year}-${month}-${day} ${hours}:${minutes}:${seconds}`
  }
  return str
}

// 监听路由参数中的 status
watch(
  () => route.query.status,
  (st) => {
    if (st && typeof st === 'string') {
      activeStatusTab.value = st
    }
  },
  { immediate: true }
)

onMounted(() => {
  loadDataScopeInfo()
  loadResourceGroups()
  loadInstances()
  loadTickets()
})
</script>

<style scoped>
.ticket-list-container {
  min-height: 100%;
  height: auto;
  width: 100%;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

/* ===== 页头 ===== */
.list-hero {
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: 12px;
  padding: 16px 20px;
  background: linear-gradient(120deg, #ffffff 0%, #f5f9ff 60%, #eef4ff 100%);
  border: 1px solid #e2e8f0;
  border-radius: 12px;
  box-shadow: 0 1px 3px rgba(15, 23, 42, 0.04);
}

.hero-left {
  display: flex;
  align-items: center;
  gap: 14px;
}

.hero-icon {
  width: 44px;
  height: 44px;
  border-radius: 12px;
  background: linear-gradient(135deg, #3b82f6 0%, #2563eb 100%);
  color: #ffffff;
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 4px 12px rgba(37, 99, 235, 0.28);
  flex-shrink: 0;
}

.hero-title {
  margin: 0;
  font-size: 20px;
  font-weight: 800;
  color: #0f172a;
}

.hero-subtitle {
  margin-top: 3px;
  font-size: 12.5px;
  color: #64748b;
}

.hero-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

/* ===== KPI 卡片 ===== */
.kpi-grid {
  display: grid;
  grid-template-columns: repeat(5, minmax(0, 1fr));
  gap: 12px;
}

.kpi-card {
  --tone: #2563eb;
  --tone-bg: #eff6ff;
  position: relative;
  background: #ffffff;
  border: 1px solid #e2e8f0;
  border-radius: 12px;
  padding: 12px 14px;
  cursor: pointer;
  overflow: hidden;
  transition: transform 0.15s ease, box-shadow 0.15s ease, border-color 0.15s ease;
}

.kpi-card::after {
  content: '';
  position: absolute;
  left: 0;
  right: 0;
  bottom: 0;
  height: 3px;
  background: var(--tone);
  opacity: 0.85;
}

.kpi-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 8px 18px rgba(15, 23, 42, 0.07);
}

.kpi-card.is-active {
  border-color: var(--tone);
  background: var(--tone-bg);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--tone) 15%, transparent);
}

.kpi-warning { --tone: #f59e0b; --tone-bg: #fffbeb; }
.kpi-orange  { --tone: #ea580c; --tone-bg: #fff7ed; }
.kpi-primary { --tone: #2563eb; --tone-bg: #eff6ff; }
.kpi-success { --tone: #10b981; --tone-bg: #ecfdf5; }
.kpi-danger  { --tone: #ef4444; --tone-bg: #fef2f2; }

.kpi-top {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.kpi-label {
  font-size: 12.5px;
  font-weight: 600;
  color: #475569;
}

.kpi-icon {
  width: 26px;
  height: 26px;
  border-radius: 8px;
  background: var(--tone-bg);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 14px;
}

.kpi-value {
  margin-top: 6px;
  font-size: 26px;
  font-weight: 800;
  color: #0f172a;
  font-family: 'JetBrains Mono', Consolas, monospace;
  line-height: 1.1;
}

.kpi-foot {
  margin-top: 4px;
  font-size: 11.5px;
  color: #94a3b8;
}

/* ===== 筛选面板 ===== */
.filter-panel {
  background: #ffffff;
  border: 1px solid #e2e8f0;
  border-radius: 12px;
  padding: 14px 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.filter-row-top {
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: 10px;
}

.scope-chip {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.scope-text {
  font-size: 12.5px;
  color: #475569;
}

.perspective-seg {
  display: inline-flex;
  background: #f1f5f9;
  border-radius: 10px;
  padding: 3px;
  gap: 2px;
}

.seg-btn {
  border: none;
  background: transparent;
  padding: 6px 12px;
  border-radius: 8px;
  font-size: 12.5px;
  font-weight: 600;
  color: #475569;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  transition: all 0.15s ease;
}

.seg-btn:hover {
  color: #0f172a;
}

.seg-btn.active {
  background: #ffffff;
  color: #1d4ed8;
  box-shadow: 0 1px 3px rgba(15, 23, 42, 0.12);
}

.seg-count {
  min-width: 20px;
  padding: 0 6px;
  border-radius: 999px;
  background: #e2e8f0;
  color: #475569;
  font-size: 11px;
  line-height: 18px;
  text-align: center;
}

.seg-btn.active .seg-count {
  background: #dbeafe;
  color: #1d4ed8;
}

.status-chip-row {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  padding-bottom: 12px;
  border-bottom: 1px dashed #e2e8f0;
}

.status-chip {
  --c: #64748b;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 5px 12px;
  border: 1px solid #e2e8f0;
  border-radius: 999px;
  background: #ffffff;
  font-size: 12.5px;
  font-weight: 600;
  color: #334155;
  cursor: pointer;
  transition: all 0.15s ease;
}

.status-chip:hover {
  border-color: var(--c);
  color: var(--c);
}

.status-chip.active {
  background: var(--c);
  border-color: var(--c);
  color: #ffffff;
}

.chip-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--c);
}

.status-chip.active .chip-dot {
  background: #ffffff;
}

.chip-count {
  font-size: 11px;
  padding: 0 6px;
  border-radius: 999px;
  background: #f1f5f9;
  color: #475569;
  line-height: 17px;
}

.status-chip.active .chip-count {
  background: rgba(255, 255, 255, 0.25);
  color: #ffffff;
}

.chip-info    { --c: #475569; }
.chip-warning { --c: #d97706; }
.chip-orange  { --c: #ea580c; }
.chip-primary { --c: #2563eb; }
.chip-success { --c: #059669; }
.chip-danger  { --c: #dc2626; }

.filter-toolbar {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}

.filter-search {
  width: 300px;
}

.filter-select {
  width: 190px;
}

.filter-select-sm {
  width: 150px;
}

/* ===== 列表面板 ===== */
.table-panel {
  background: #ffffff;
  border: 1px solid #e2e8f0;
  border-radius: 12px;
  overflow: hidden;
}

.table-panel-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px;
  border-bottom: 1px solid #f1f5f9;
}

.tp-left {
  display: flex;
  align-items: baseline;
  gap: 10px;
}

.tp-title {
  font-size: 15px;
  font-weight: 700;
  color: #0f172a;
}

.tp-count {
  font-size: 12.5px;
  color: #64748b;
}

.tp-count b {
  color: #2563eb;
}

.tp-tip {
  font-size: 12px;
  color: #94a3b8;
}

.table-wrapper {
  width: 100%;
  overflow-x: auto;
}

.table-wrapper :deep(.ticket-row) {
  cursor: pointer;
}

.table-wrapper :deep(.el-table__row:hover > td.el-table__cell) {
  background: #f5f9ff !important;
}

.table-wrapper :deep(.el-table__cell) {
  padding: 10px 0;
}

.ticket-id-badge {
  font-family: 'JetBrains Mono', Consolas, monospace;
  font-weight: 700;
  font-size: 12.5px;
  color: #1d4ed8;
  background: #eff6ff;
  border: 1px solid #dbeafe;
  padding: 2px 8px;
  border-radius: 6px;
}

.reason-cell {
  display: flex;
  flex-direction: column;
  gap: 5px;
  min-width: 0;
}

.reason-text {
  color: #0f172a;
  font-size: 13px;
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.reason-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.type-chip {
  flex-shrink: 0;
  font-size: 11px;
  font-weight: 600;
  padding: 1px 7px;
  border-radius: 5px;
  background: #f1f5f9;
  color: #475569;
}

.type-primary { background: #eff6ff; color: #1d4ed8; }
.type-danger  { background: #fef2f2; color: #b91c1c; }
.type-warning { background: #fffbeb; color: #b45309; }

.meta-dept {
  font-size: 11.5px;
  color: #94a3b8;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.applicant-cell {
  display: flex;
  align-items: center;
  gap: 8px;
}

.applicant-avatar {
  background: linear-gradient(135deg, #3b82f6 0%, #1d4ed8 100%);
  color: #ffffff;
  font-weight: 700;
  font-size: 12px;
  flex-shrink: 0;
}

.applicant-name-text {
  font-weight: 600;
  color: #1e293b;
  font-size: 13px;
}

.target-cell {
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}

.target-line {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}

.env-chip {
  flex-shrink: 0;
  font-size: 10px;
  font-weight: 800;
  padding: 0 6px;
  line-height: 17px;
  border-radius: 4px;
  letter-spacing: 0.3px;
}

.env-prod  { background: #fee2e2; color: #b91c1c; }
.env-other { background: #e0f2fe; color: #0369a1; }

.instance-name-text {
  font-weight: 600;
  color: #1e293b;
  font-size: 13px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.db-name {
  font-family: 'JetBrains Mono', Consolas, monospace;
  font-size: 12px;
  color: #047857;
}

.db-empty {
  font-size: 12px;
  color: #94a3b8;
}

.rg-chip {
  font-size: 12px;
  font-weight: 600;
  color: #92400e;
  background: #fffbeb;
  border: 1px solid #fde68a;
  padding: 2px 8px;
  border-radius: 6px;
}

.status-cell {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 4px;
}

.status-pill {
  --c: #64748b;
  --bg: #f1f5f9;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 2px 10px;
  border-radius: 999px;
  font-size: 12px;
  font-weight: 700;
  color: var(--c);
  background: var(--bg);
}

.st-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--c);
}

.st-success { --c: #047857; --bg: #ecfdf5; }
.st-warning { --c: #b45309; --bg: #fffbeb; }
.st-primary { --c: #1d4ed8; --bg: #eff6ff; }
.st-danger  { --c: #b91c1c; --bg: #fef2f2; }
.st-info    { --c: #475569; --bg: #f1f5f9; }

.st-warning .st-dot,
.st-primary .st-dot {
  animation: stPulse 1.6s infinite;
}

@keyframes stPulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.35; }
}

.status-sub {
  font-size: 11.5px;
  color: #64748b;
}

.status-sub-warn {
  color: #b45309;
}

.time-cell {
  display: flex;
  flex-direction: column;
  line-height: 1.35;
}

.time-rel {
  font-size: 12.5px;
  font-weight: 600;
  color: #334155;
}

.time-abs {
  font-size: 11px;
  color: #94a3b8;
  font-family: 'JetBrains Mono', Consolas, monospace;
}

.row-actions {
  display: flex;
  align-items: center;
  gap: 2px;
  flex-wrap: nowrap;
}

.row-actions :deep(.el-button + .el-button) {
  margin-left: 6px;
}

.empty-state {
  padding: 36px 0 28px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  line-height: 1.5;
}

.empty-icon {
  font-size: 36px;
}

.empty-title {
  font-size: 15px;
  font-weight: 700;
  color: #334155;
}

.empty-desc {
  font-size: 12.5px;
  color: #94a3b8;
  margin-bottom: 8px;
}

.pagination-bar {
  display: flex;
  justify-content: flex-end;
  padding: 12px 16px;
  border-top: 1px solid #f1f5f9;
}

@media (max-width: 1200px) {
  .kpi-grid {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }
}

@media (max-width: 768px) {
  .kpi-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
  .hero-subtitle,
  .tp-tip,
  .scope-text {
    display: none;
  }
  .filter-search,
  .filter-select,
  .filter-select-sm {
    width: 100%;
  }
  .perspective-seg {
    width: 100%;
    overflow-x: auto;
  }
  .pagination-bar {
    justify-content: center;
    overflow-x: auto;
  }
}

@media (prefers-reduced-motion: reduce) {
  .st-warning .st-dot,
  .st-primary .st-dot {
    animation: none;
  }
  .kpi-card:hover {
    transform: none;
  }
}
</style>
