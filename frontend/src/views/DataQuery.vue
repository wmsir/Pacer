<template>
  <div class="data-query-workbench page-container">
    <!-- 顶部工作台标题栏 -->
    <div class="workbench-header">
      <div class="header-left">
        <div class="header-titles">
          <h2 class="main-title">数据查询控制台 (Data Explorer)</h2>
          <span class="sub-title">选择实例与数据库，即时浏览库表、查看字段结构及执行只读安全 SQL 查询</span>
        </div>
      </div>
      <div class="header-right">
        <el-tag effect="plain" type="danger" style="font-weight: 600; margin-right: 8px;">
          🔒 严格只读模式 (Read-Only)
        </el-tag>
        <el-button :icon="CopyDocument" @click="openInNewWindow">在新窗口打开</el-button>
      </div>
    </div>

    <!-- 顶部数据源选择与操作工具栏 -->
    <div class="query-top-toolbar">
      <div class="toolbar-items">
        <div class="toolbar-item">
          <span class="item-label">目标实例：</span>
          <el-select
            v-model="selectedInstanceId"
            placeholder="请选择数据库实例"
            style="width: 300px;"
            @change="handleInstanceChange"
            :loading="instancesLoading"
          >
            <el-option
              v-for="inst in instances"
              :key="inst.id"
              :label="`${inst.name} (${inst.env || 'PROD'} · ${inst.dbType || 'mysql'})`"
              :value="inst.id"
            >
              <div style="display: flex; justify-content: space-between; align-items: center;">
                <span style="font-weight: 600; color: #1e293b;">{{ inst.name }}</span>
                <div style="display: flex; gap: 6px; align-items: center;">
                  <el-tag size="small" :type="inst.env === 'PROD' ? 'danger' : 'info'">{{ inst.env || 'PROD' }}</el-tag>
                  <el-tag size="small" :type="dbTypeTagType(inst.dbType)" effect="dark">{{ inst.dbType || 'mysql' }}</el-tag>
                </div>
              </div>
            </el-option>
          </el-select>
          <!-- 当前选中实例的数据库类型徽标 -->
          <el-tag
            v-if="selectedInstanceId && currentDbType"
            :type="dbTypeTagType(currentDbType)"
            effect="dark"
            style="margin-left: 8px; font-size: 13px; font-weight: 600; letter-spacing: 0.5px;"
          >
            {{ currentDbType.toUpperCase() }}
          </el-tag>
        </div>

        <div class="toolbar-item">
          <span class="item-label">执行数据库：</span>
          <el-select
            v-model="selectedDbName"
            placeholder="选择具体目标数据库"
            style="width: 200px;"
            @change="handleDbChange"
            :loading="databasesLoading"
            :disabled="!selectedInstanceId"
            no-data-text="未探测到数据库"
          >
            <el-option
              v-for="db in availableDatabases"
              :key="db"
              :label="db"
              :value="db"
            >
              <span style="float: left; font-family: monospace; font-weight: 600;">{{ db }}</span>
              <el-tag size="small" type="success" style="float: right;">业务库</el-tag>
            </el-option>
          </el-select>
        </div>

        <div class="toolbar-item">
          <span class="item-label">最大行数：</span>
          <el-select v-model="queryLimit" style="width: 105px;">
            <el-option label="50 行" :value="50" />
            <el-option label="100 行" :value="100" />
            <el-option label="200 行" :value="200" />
            <el-option label="500 行" :value="500" />
            <el-option label="1000 行" :value="1000" />
          </el-select>
        </div>

        <div class="toolbar-actions">
          <el-button
            type="primary"
            :icon="VideoPlay"
            :loading="currentTab?.loading || false"
            :disabled="!isQuerySupported"
            @click="handleExecuteQuery"
          >
            执行查询 (Ctrl+Enter)
          </el-button>
          <el-button
            type="success"
            plain
            :icon="Tickets"
            :loading="explainLoading"
            :disabled="!isQuerySupported"
            @click="handleExecuteExplain"
          >
            ⚡ 执行计划 (EXPLAIN)
          </el-button>
          <el-button type="warning" plain :icon="Hide" @click="() => handleGoToMasking()">
            🛡️ 脱敏配置
          </el-button>
          <el-tooltip
            :disabled="isQueryExportSupported"
            content="当前数据库实例未开启【支持数据导出】权限，严禁导出脱库"
            placement="top"
          >
            <span>
              <el-button
                :icon="Download"
                @click="handleExportCsv"
                :disabled="!isQueryExportSupported || !currentTab?.result || !currentTab?.result?.rows || currentTab.result.rows.length === 0"
              >
                导出 CSV
              </el-button>
            </span>
          </el-tooltip>
          <el-tooltip
            :disabled="isQueryExportSupported"
            content="当前数据库实例未开启【支持数据导出】权限，严禁导出脱库"
            placement="top"
          >
            <span>
              <el-button
                :icon="DocumentCopy"
                @click="handleExportJson"
                :disabled="!isQueryExportSupported || !currentTab?.result || !currentTab?.result?.rows || currentTab.result.rows.length === 0"
              >
                导出 JSON
              </el-button>
            </span>
          </el-tooltip>
        </div>
      </div>
    </div>

    <!-- 主体左右分栏布局 -->
    <div class="main-split-layout">
      <!-- 左侧：数据表目录树 / 列表 (支持一键收起/展开) -->
      <div class="left-table-sidebar" :class="{ 'is-collapsed': isTableSidebarCollapsed }">
        <div class="sidebar-header" v-if="!isTableSidebarCollapsed">
          <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px;">
            <span class="sidebar-title">
              <el-icon><Grid /></el-icon>
              <span>数据表列表 ({{ filteredTables.length }})</span>
            </span>
            <div style="display: flex; align-items: center; gap: 4px;">
              <el-button size="small" link :icon="Refresh" @click="loadTablesForDb(selectedDbName)" :loading="tablesLoading">
                刷新
              </el-button>
              <el-tooltip content="收起库表目录栏以扩大编辑空间" placement="top">
                <el-button size="small" link :icon="Fold" @click="isTableSidebarCollapsed = true" />
              </el-tooltip>
            </div>
          </div>
          <el-input
            v-model="tableSearchQuery"
            placeholder="搜索表名或注释..."
            size="small"
            clearable
            :prefix-icon="Search"
          />
        </div>

        <div v-else class="sidebar-collapsed-rail" @click="isTableSidebarCollapsed = false" title="点击展开数据表列表">
          <el-icon :size="16" color="#2563eb"><Expand /></el-icon>
          <span class="collapsed-vertical-text">展开库表目录 ({{ filteredTables.length }})</span>
        </div>

        <div v-if="!isTableSidebarCollapsed" class="table-list-scroll" v-loading="tablesLoading">
          <div
            v-for="tbl in filteredTables"
            :key="tbl.tableName"
            class="table-list-item"
            :class="{ active: currentActiveTable === tbl.tableName }"
            @click="handleSelectTable(tbl.tableName)"
          >
            <div class="tbl-item-top">
              <el-icon class="tbl-icon"><Tickets /></el-icon>
              <span class="tbl-name" :title="tbl.tableName">{{ tbl.tableName }}</span>
            </div>
            <div class="tbl-item-desc" v-if="tbl.tableComment">
              {{ tbl.tableComment }}
            </div>
          </div>

          <div v-if="!tablesLoading && filteredTables.length === 0" class="empty-tables-tip">
            {{ selectedDbName ? '当前库下未发现数据表' : '请先在上方选择实例与数据库' }}
          </div>
        </div>
      </div>

      <!-- 右侧：多查询窗口标签栏 + SQL 编辑区与多结果展示 -->
      <div class="right-query-main">
        <!-- 多查询窗口标签栏 -->
        <div class="query-tabs-header-bar">
          <el-tabs
            v-model="activeTabId"
            type="card"
            editable
            @edit="handleTabEdit"
            class="multi-query-tabs"
          >
            <el-tab-pane
              v-for="tab in queryTabs"
              :key="tab.id"
              :name="tab.id"
            >
              <template #label>
                <div class="tab-pill-label" @dblclick="handleRenameTab(tab)">
                  <el-icon style="margin-right: 4px; font-size: 13px;"><Document /></el-icon>
                  <span class="tab-title-text" :title="tab.name">{{ tab.name }}</span>
                  <span v-if="tab.result && tab.result.success" class="tab-rows-tag">
                    {{ tab.result.totalRows }}行
                  </span>
                  <span v-else-if="tab.result && !tab.result.success" class="tab-err-tag">
                    报错
                  </span>
                  <el-tooltip content="重命名窗口" placement="top" :show-after="500">
                    <el-icon class="tab-edit-icon" @click.stop="handleRenameTab(tab)"><EditPen /></el-icon>
                  </el-tooltip>
                </div>
              </template>
            </el-tab-pane>
          </el-tabs>
        </div>

        <!-- SQL 编辑器卡片 -->
        <div class="editor-box" v-if="currentTab">
          <div class="editor-tools">
            <div class="tools-left">
              <span class="editor-label">
                <el-icon><EditPen /></el-icon>
                <span><b>{{ currentTab.name }}</b> · 只读 SQL 查询</span>
              </span>
              <el-tag size="small" effect="plain" type="info" class="quick-sql-tag" @click="setSql(`SELECT * FROM ${currentActiveTable || 'typ_preference'} LIMIT 50;`)">
                SELECT *
              </el-tag>
              <el-tag size="small" effect="plain" type="info" class="quick-sql-tag" @click="setSql(`SELECT COUNT(*) FROM ${currentActiveTable || 'typ_preference'};`)">
                COUNT(*)
              </el-tag>
              <el-tag size="small" effect="plain" type="info" class="quick-sql-tag" @click="setSql('SHOW TABLES;')">
                SHOW TABLES
              </el-tag>
              <el-tag size="small" effect="plain" type="info" class="quick-sql-tag" @click="setSql(`DESCRIBE ${currentActiveTable || 'typ_preference'};`)">
                DESC 表结构
              </el-tag>
            </div>
            <div class="tools-right">
              <el-button size="small" link @click="currentTab.sql = ''">清空语句</el-button>
            </div>
          </div>

          <el-input
            v-model="currentTab.sql"
            type="textarea"
            :rows="5"
            placeholder="请输入只读 SQL 查询语句（如：SELECT * FROM 表名 LIMIT 50;），按 Ctrl+Enter 立即执行..."
            class="ide-code-textarea"
            @keydown.ctrl.enter="handleExecuteQuery"
            @keydown.meta.enter="handleExecuteQuery"
          />
        </div>

        <!-- 结果展示 Tabs：数据结果集 vs 表结构定义 -->
        <div class="result-tabs-container" v-if="currentTab">
          <el-tabs v-model="currentTab.resultTab" class="custom-result-tabs">
            <!-- 结果集页签 -->
            <el-tab-pane label="数据结果集 (Data Grid)" name="data_grid">
              <div class="grid-card">
                <div class="grid-status-bar">
                  <div class="status-left">
                    <span class="status-tag" :class="{ success: currentTab.result?.success, error: currentTab.result && !currentTab.result.success }">
                      {{ currentTab.result ? (currentTab.result.success ? '查询成功' : '执行失败') : '就绪' }}
                    </span>
                    <span v-if="currentTab.result" class="meta-text">
                      目标库：<b>{{ currentTab.result.databaseName }}</b> | 返回：<b>{{ currentTab.result.totalRows }}</b> 行 | 耗时：<b>{{ currentTab.result.durationMs }}ms</b>
                      <el-tag
                        v-if="currentTab.result.routeNode"
                        size="small"
                        :type="currentTab.result.routeNode === 'SLAVE' ? 'success' : 'warning'"
                        effect="light"
                        style="margin-left: 8px; font-weight: 500;"
                        :title="currentTab.result.routeDescription"
                      >
                        {{ currentTab.result.routeNode === 'SLAVE' ? '🟢 只读从库 (Slave)' : '🟡 核心主库 (Master)' }}
                      </el-tag>
                    </span>
                    <span v-else class="meta-text">
                      点击左侧表名或在上方输入 SQL 点击【执行查询】
                    </span>
                  </div>
                  <div class="status-right" v-if="currentTab.result && currentTab.result.rows && currentTab.result.rows.length > 0">
                    <el-input
                      v-model="currentTab.filterText"
                      placeholder="在当前窗口结果中过滤..."
                      size="small"
                      clearable
                      style="width: 200px;"
                    />
                  </div>
                </div>

                <!-- 错误报错提示 -->
                <div v-if="currentTab.result && !currentTab.result.success" style="padding: 14px;">
                  <el-alert
                    :title="currentTab.result.errorMessage || '查询报错'"
                    type="error"
                    :closable="false"
                    show-icon
                  />
                </div>

                <!-- 动态表格 -->
                <div v-if="currentTab.result && currentTab.result.success" class="table-render-wrap">
                  <el-table
                    :data="pagedQueryResultRows"
                    size="small"
                    border
                    stripe
                    highlight-current-row
                    height="360px"
                    style="width: 100%; font-family: monospace;"
                  >
                    <el-table-column type="index" label="#" width="45" align="center" fixed />
                    <el-table-column
                      v-for="col in (currentTab.result.columns || [])"
                      :key="col"
                      :prop="col"
                      :label="col"
                      min-width="155"
                      show-overflow-tooltip
                    >
                      <template #header>
                        <div style="display: inline-flex; align-items: center; gap: 4px;">
                          <span style="font-weight: 600;">{{ col }}</span>
                          <el-tag
                            v-if="getMaskRuleForCol(col)"
                            size="small"
                            type="warning"
                            effect="light"
                            style="font-size: 10px; padding: 0 4px; height: 18px; line-height: 16px; border-radius: 3px;"
                          >
                            🛡️ {{ getMaskRuleForCol(col) }}
                          </el-tag>
                        </div>
                      </template>
                      <template #default="{ row }">
                        <div style="display: flex; align-items: center; justify-content: space-between;">
                          <span :class="{ 'null-val': row[col] === 'NULL', 'masked-val': !!getMaskRuleForCol(col) }">
                            {{ row[col] }}
                          </span>
                          <el-tooltip
                            v-if="getMaskRuleForCol(col) && row[col] !== 'NULL'"
                            :content="`🔒 该字段已生效【${getMaskRuleForCol(col)}】动态脱敏保护`"
                            placement="top"
                          >
                            <el-tag size="small" type="danger" effect="plain" style="font-size: 10px; height: 16px; line-height: 14px; padding: 0 3px; margin-left: 4px;">
                              脱敏
                            </el-tag>
                          </el-tooltip>
                        </div>
                      </template>
                    </el-table-column>
                  </el-table>

                  <!-- 查询结果分页栏 -->
                  <div class="pagination-bar" style="display: flex; justify-content: flex-end; margin-top: 10px;">
                    <el-pagination
                      v-model:current-page="currentTab.currentPage"
                      v-model:page-size="currentTab.pageSize"
                      :page-sizes="[20, 50, 100, 200]"
                      :total="filteredQueryResultRows.length"
                      layout="total, sizes, prev, pager, next, jumper"
                      background
                      small
                    />
                  </div>
                </div>

                <div v-if="!currentTab.result" class="empty-result-placeholder">
                  <el-empty description="在左侧点击任意数据表或输入 SELECT 语句开始查询" />
                </div>
              </div>
            </el-tab-pane>

            <!-- 表结构元数据页签 -->
            <el-tab-pane label="表结构元数据 (Schema)" name="schema">
              <div class="schema-card">
                <div class="schema-header">
                  <span>当前数据表：<b style="color: #409EFF; font-family: monospace;">{{ currentActiveTable || '未选择' }}</b></span>
                  <el-button size="small" link :icon="Refresh" @click="loadTableColumns(currentActiveTable)" :disabled="!currentActiveTable">
                    刷新字段元数据
                  </el-button>
                </div>

                <el-table :data="tableColumnsData" size="small" border stripe height="380px" v-loading="columnsLoading">
                  <el-table-column type="index" label="#" width="45" align="center" />
                  <el-table-column prop="columnName" label="字段名称" min-width="160" show-overflow-tooltip>
                    <template #default="{ row }">
                      <span style="font-family: monospace; font-weight: 600;">{{ row.columnName }}</span>
                      <el-tag size="small" type="danger" v-if="row.columnKey === 'PRI'" style="margin-left: 6px;">主键</el-tag>
                    </template>
                  </el-table-column>
                  <el-table-column prop="columnType" label="字段类型" min-width="140">
                    <template #default="{ row }">
                      <span style="font-family: monospace; font-size: 12px; color: #475569;">{{ row.columnType }}</span>
                    </template>
                  </el-table-column>
                  <el-table-column prop="isNullable" label="允许为空" width="90" align="center">
                    <template #default="{ row }">
                      <el-tag size="small" :type="row.isNullable === 'YES' ? 'success' : 'danger'">
                        {{ row.isNullable === 'YES' ? 'YES' : 'NO' }}
                      </el-tag>
                    </template>
                  </el-table-column>
                  <el-table-column prop="columnDefault" label="默认值" min-width="120" show-overflow-tooltip>
                    <template #default="{ row }">
                      <span style="color: #909399; font-family: monospace;">{{ row.columnDefault !== null ? row.columnDefault : 'NULL' }}</span>
                    </template>
                  </el-table-column>
                  <el-table-column prop="columnComment" label="字段注释" min-width="180" show-overflow-tooltip />
                </el-table>
              </div>
            </el-tab-pane>
          </el-tabs>
        </div>
      </div>
    </div>

    <!-- SQL 执行计划 (EXPLAIN) 深度分析弹窗 -->
    <el-dialog
      title="⚡ SQL 执行计划深度分析 (EXPLAIN Analysis)"
      v-model="explainDialogVisible"
      width="920px"
      append-to-body
      destroy-on-close
    >
      <div v-if="explainResult" class="explain-container">
        <!-- 待测 SQL 预览 -->
        <div style="margin-bottom: 12px; background: #f8fafc; border: 1px solid #e2e8f0; border-radius: 6px; padding: 10px 14px;">
          <div style="font-size: 11px; font-weight: 600; color: #64748b; margin-bottom: 4px;">待分析 SQL 语句：</div>
          <code style="font-size: 13px; color: #0f172a; word-break: break-all;">{{ explainResult.sql }}</code>
        </div>

        <!-- 智能诊断建议卡片 -->
        <div style="margin-bottom: 14px; background: #f0fdf4; border: 1px solid #bbf7d0; border-radius: 6px; padding: 12px 16px;">
          <div style="font-weight: 700; color: #166534; font-size: 13px; margin-bottom: 6px; display: flex; align-items: center; gap: 6px;">
            <span>💡 性能与索引命中智能诊断建议：</span>
          </div>
          <div style="font-size: 12px; color: #15803d; line-height: 1.6;">
            <div v-for="(tip, idx) in explainDiagnostics" :key="idx" style="margin-bottom: 3px;">
              • {{ tip }}
            </div>
          </div>
        </div>

        <!-- 执行计划结构化表格 -->
        <el-table :data="explainResult.rows || []" size="small" border stripe style="width: 100%; font-family: monospace;">
          <el-table-column
            v-for="col in explainResult.columns || []"
            :key="col"
            :prop="col"
            :label="col"
            min-width="110"
            show-overflow-tooltip
          >
            <template #default="{ row }">
              <span v-if="col.toLowerCase() === 'type'">
                <el-tag
                  size="small"
                  :type="row[col] === 'ALL' ? 'danger' : (['eq_ref', 'const', 'ref', 'range'].includes(row[col]) ? 'success' : 'warning')"
                  effect="dark"
                >
                  {{ row[col] }}
                </el-tag>
              </span>
              <span v-else-if="col.toLowerCase() === 'key'">
                <el-tag size="small" :type="row[col] && row[col] !== 'NULL' ? 'success' : 'info'" effect="plain">
                  {{ row[col] }}
                </el-tag>
              </span>
              <span v-else :class="{ 'null-val': row[col] === 'NULL' }">
                {{ row[col] }}
              </span>
            </template>
          </el-table-column>
        </el-table>
      </div>

      <template #footer>
        <el-button @click="explainDialogVisible = false">关闭分析</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  CopyDocument, DocumentCopy, VideoPlay, Download, Grid,
  Tickets, Refresh, Search, EditPen, Hide, Document, Fold, Expand
} from '@element-plus/icons-vue'
import request from '../utils/request'

const router = useRouter()

interface InstanceItem {
  id: number
  name: string
  dbType: string
  env: string
  tags?: string
  supportedOps?: string
}

interface TableItem {
  tableName: string
  tableComment: string
  tableRows: number
  dataLength: number
}

interface QueryTabItem {
  id: string
  name: string
  sql: string
  limit: number
  result: any
  resultTab: string
  filterText: string
  currentPage: number
  pageSize: number
  loading: boolean
}

const instances = ref<InstanceItem[]>([])
const availableDatabases = ref<string[]>([])
const tableList = ref<TableItem[]>([])
const tableColumnsData = ref<any[]>([])

const selectedInstanceId = ref<number | string>('')
const selectedDbName = ref('')
const currentActiveTable = ref('')
const tableSearchQuery = ref('')
const queryLimit = ref(100)
const isTableSidebarCollapsed = ref(false)

let tabCounter = 1
const queryTabs = ref<QueryTabItem[]>([
  {
    id: 'tab_1',
    name: '查询窗口 1',
    sql: '',
    limit: 100,
    result: null,
    resultTab: 'data_grid',
    filterText: '',
    currentPage: 1,
    pageSize: 50,
    loading: false
  }
])
const activeTabId = ref('tab_1')

const currentTab = computed(() => {
  return queryTabs.value.find(t => t.id === activeTabId.value) || queryTabs.value[0]
})

const instancesLoading = ref(false)
const databasesLoading = ref(false)
const tablesLoading = ref(false)
const columnsLoading = ref(false)
const explainLoading = ref(false)
const explainDialogVisible = ref(false)
const explainResult = ref<any>(null)

const handleTabEdit = (targetName: any, action: 'add' | 'remove') => {
  if (action === 'add') {
    tabCounter++
    const newId = `tab_${Date.now()}`
    const newName = `查询窗口 ${tabCounter}`
    queryTabs.value.push({
      id: newId,
      name: newName,
      sql: currentActiveTable.value ? `SELECT * FROM ${currentActiveTable.value} LIMIT ${queryLimit.value};` : '',
      limit: queryLimit.value,
      result: null,
      resultTab: 'data_grid',
      filterText: '',
      currentPage: 1,
      pageSize: 50,
      loading: false
    })
    activeTabId.value = newId
    ElMessage.success(`已新建【${newName}】`)
  } else if (action === 'remove') {
    if (queryTabs.value.length <= 1) {
      queryTabs.value[0].sql = ''
      queryTabs.value[0].result = null
      ElMessage.info('已重置当前查询窗口')
      return
    }
    const idx = queryTabs.value.findIndex(t => t.id === targetName)
    if (idx !== -1) {
      queryTabs.value.splice(idx, 1)
      if (activeTabId.value === targetName) {
        const nextTab = queryTabs.value[idx] || queryTabs.value[idx - 1]
        activeTabId.value = nextTab.id
      }
    }
  }
}

const handleRenameTab = async (tab: QueryTabItem) => {
  try {
    const { value } = await ElMessageBox.prompt('请输入新的查询窗口名称：', '重命名查询窗口', {
      confirmButtonText: '确定',
      cancelButtonText: '取消',
      inputValue: tab.name,
      inputPattern: /\S+/,
      inputErrorMessage: '窗口名称不能为空'
    })
    if (value && value.trim()) {
      tab.name = value.trim()
      ElMessage.success('重命名成功')
    }
  } catch (e) {}
}

const explainDiagnostics = computed(() => {
  if (!explainResult.value || !explainResult.value.rows || explainResult.value.rows.length === 0) {
    return ['未获取到执行计划详情']
  }
  const tips: string[] = []
  const rows = explainResult.value.rows

  let hasAllScan = false
  let hasIndexScan = false
  let hasFilesort = false
  let hasTempTable = false

  rows.forEach((r: any) => {
    const type = String(r.type || r.TYPE || '').toUpperCase()
    const extra = String(r.Extra || r.extra || '')
    const key = String(r.key || r.KEY || '')

    if (type === 'ALL') hasAllScan = true
    if (['EQ_REF', 'CONST', 'REF', 'RANGE'].includes(type) || (key && key !== 'NULL')) hasIndexScan = true
    if (extra.includes('Using filesort')) hasFilesort = true
    if (extra.includes('Using temporary')) hasTempTable = true
  })

  if (hasAllScan) {
    tips.push('⚠️ 存在全表扫描 (type = ALL)：若表数据量较大，建议在 WHERE / JOIN 关联字段上建立合适的索引以提升查询效率。')
  } else if (hasIndexScan) {
    tips.push('✓ 成功命中索引：查询已有效利用索引进行快速定位检索，性能表现优异。')
  }

  if (hasFilesort) {
    tips.push('⚠️ 包含文件排序 (Using filesort)：ORDER BY 排序未能完全利用索引覆盖，建议根据排序字段与过滤字段联合建立复合索引。')
  }
  if (hasTempTable) {
    tips.push('⚠️ 包含内存/磁盘临时表 (Using temporary)：GROUP BY 或 DISTINCT 产生临时表，若数据量大可能产生性能瓶颈。')
  }
  if (tips.length === 0) {
    tips.push('✓ SQL 执行计划结构正常，未检测到明显性能隐患。')
  }
  return tips
})

const currentSelectedInstance = computed(() => {
  return instances.value.find(i => String(i.id) === String(selectedInstanceId.value))
})

// 当前选中实例的数据库类型
const currentDbType = computed(() => currentSelectedInstance.value?.dbType || '')

// 根据数据库类型返回标签颜色：MySQL/TiDB→success, Oracle/达梦→warning, PostgreSQL/openGauss→primary, 其他→info
const dbTypeTagType = (dbType?: string): string => {
  const t = (dbType || '').toLowerCase()
  if (t === 'mysql' || t === 'tidb' || t === 'oceanbase') return 'success'
  if (t === 'oracle' || t === 'dameng') return 'warning'
  if (t === 'postgresql' || t === 'opengauss') return 'primary'
  if (t === 'kingbase') return 'danger'
  return 'info'
}

const parseInstanceSupportedOps = (opsRaw?: string): string[] => {
  if (!opsRaw) return ['支持上线', '支持查询', '支持DML变更', '支持DDL结构变更', '支持数据导出', '支持事务预执行', '支持数据脱敏', '支持历史回滚']
  try {
    const arr = JSON.parse(opsRaw)
    return Array.isArray(arr) && arr.length > 0 ? arr : ['支持上线', '支持查询', '支持DML变更', '支持DDL结构变更']
  } catch (e) {
    return opsRaw.split(/[,，]/).filter(Boolean)
  }
}

const currentInstanceSupportedOps = computed(() => {
  return parseInstanceSupportedOps(currentSelectedInstance.value?.supportedOps)
})

const isQuerySupported = computed(() => {
  return currentInstanceSupportedOps.value.includes('支持查询')
})

const isQueryExportSupported = computed(() => {
  return currentInstanceSupportedOps.value.includes('支持数据导出')
})

const openInNewWindow = () => {
  const href = router.resolve('/data-query').href
  window.open(href, '_blank')
}

// 加载实例列表
const loadInstances = async () => {
  instancesLoading.value = true
  try {
    const res: any = await request.get('/v1/instance/list')
    instances.value = Array.isArray(res.data) ? res.data : []
    if (instances.value.length > 0) {
      selectedInstanceId.value = instances.value[0].id
      await loadDatabases(instances.value[0].id)
    }
  } catch (err) {
    console.error('Failed to load instances', err)
  } finally {
    instancesLoading.value = false
  }
}

// 加载数据库列表
const loadDatabases = async (instanceId: number | string) => {
  if (!instanceId) return
  databasesLoading.value = true
  try {
    const res: any = await request.get(`/v1/instance/${instanceId}/databases`)
    availableDatabases.value = Array.isArray(res.data) ? res.data : []
    if (availableDatabases.value.length > 0) {
      selectedDbName.value = availableDatabases.value[0]
      await loadTablesForDb(availableDatabases.value[0])
    } else {
      selectedDbName.value = ''
      tableList.value = []
    }
  } catch (err) {
    console.error('Failed to load databases', err)
  } finally {
    databasesLoading.value = false
  }
}

// 加载指定库的数据表列表
const loadTablesForDb = async (dbName: string) => {
  if (!selectedInstanceId.value || !dbName) return
  tablesLoading.value = true
  try {
    const res: any = await request.get(`/v1/instance/${selectedInstanceId.value}/databases/${dbName}/tables`)
    tableList.value = Array.isArray(res.data) ? res.data : []
    if (tableList.value.length > 0) {
      currentActiveTable.value = tableList.value[0].tableName
      if (!currentTab.value.sql) {
        currentTab.value.sql = `SELECT * FROM ${tableList.value[0].tableName} LIMIT ${queryLimit.value};`
      }
      loadTableColumns(tableList.value[0].tableName)
    } else {
      currentActiveTable.value = ''
      tableColumnsData.value = []
    }
  } catch (err) {
    console.error('Failed to load tables', err)
    tableList.value = []
  } finally {
    tablesLoading.value = false
  }
}

// 加载指定表的字段元数据
const loadTableColumns = async (tableName: string) => {
  if (!selectedInstanceId.value || !selectedDbName.value || !tableName) return
  columnsLoading.value = true
  try {
    const res: any = await request.get(`/v1/instance/${selectedInstanceId.value}/databases/${selectedDbName.value}/tables/${tableName}/columns`)
    tableColumnsData.value = Array.isArray(res.data) ? res.data : []
  } catch (err) {
    console.error('Failed to load columns', err)
    tableColumnsData.value = []
  } finally {
    columnsLoading.value = false
  }
}

const handleInstanceChange = async (val: any) => {
  if (currentTab.value) {
    currentTab.value.result = null
  }
  await loadDatabases(val)
}

const handleDbChange = async (val: any) => {
  if (currentTab.value) {
    currentTab.value.result = null
  }
  await loadTablesForDb(val)
}

const handleSelectTable = (tblName: string) => {
  currentActiveTable.value = tblName
  if (currentTab.value) {
    currentTab.value.sql = `SELECT * FROM ${tblName} LIMIT ${queryLimit.value};`
  }
  loadTableColumns(tblName)
  handleExecuteQuery()
}

const setSql = (sql: string) => {
  if (currentTab.value) {
    currentTab.value.sql = sql
  }
}

const filteredTables = computed(() => {
  if (!tableSearchQuery.value) return tableList.value
  const q = tableSearchQuery.value.toLowerCase()
  return tableList.value.filter(t =>
    t.tableName.toLowerCase().includes(q) ||
    (t.tableComment && t.tableComment.toLowerCase().includes(q))
  )
})

// 执行只读查询
const handleExecuteQuery = async () => {
  if (!selectedInstanceId.value) {
    ElMessage.warning('请先选择目标数据库实例')
    return
  }
  if (!selectedDbName.value) {
    ElMessage.warning('请选择目标数据库')
    return
  }
  if (!currentTab.value || !currentTab.value.sql || currentTab.value.sql.trim().length === 0) {
    ElMessage.warning('请输入待执行的 SQL 查询语句')
    return
  }

  if (!isQuerySupported.value) {
    ElMessage.error('当前数据库实例未开启【支持查询】操作权限，严禁在线执行数据检索！')
    return
  }

  currentTab.value.loading = true
  currentTab.value.resultTab = 'data_grid'
  currentTab.value.currentPage = 1
  try {
    const payload = {
      instanceId: selectedInstanceId.value,
      dbName: selectedDbName.value,
      sql: currentTab.value.sql,
      limit: queryLimit.value
    }
    const res: any = await request.post('/v1/ticket/query', payload)
    currentTab.value.result = res.data
    if (res.data?.success) {
      ElMessage.success(`【${currentTab.value.name}】查询成功，耗时 ${res.data.durationMs}ms，共返回 ${res.data.totalRows} 行数据`)
    } else {
      ElMessage.error(res.data?.errorMessage || '查询执行报错')
    }
  } catch (err: any) {
    console.error('Execute query failed', err)
  } finally {
    if (currentTab.value) {
      currentTab.value.loading = false
    }
  }
}

// 执行 SQL 执行计划 (EXPLAIN) 分析
const handleExecuteExplain = async () => {
  if (!selectedInstanceId.value) {
    ElMessage.warning('请先选择目标数据库实例')
    return
  }
  if (!selectedDbName.value) {
    ElMessage.warning('请选择目标数据库')
    return
  }
  if (!currentTab.value || !currentTab.value.sql || currentTab.value.sql.trim().length === 0) {
    ElMessage.warning('请输入待分析执行计划的 SQL 语句')
    return
  }

  explainLoading.value = true
  try {
    const payload = {
      instanceId: selectedInstanceId.value,
      dbName: selectedDbName.value,
      sql: currentTab.value.sql
    }
    const res: any = await request.post('/v1/ticket/explain', payload)
    if (res.data && res.data.success) {
      explainResult.value = res.data
      explainDialogVisible.value = true
      ElMessage.success('执行计划分析成功生成！')
    } else {
      ElMessage.error(res.data?.errorMessage || '执行计划生成失败')
    }
  } catch (err: any) {
    ElMessage.error(err.response?.data?.message || err.message || '生成执行计划异常')
  } finally {
    explainLoading.value = false
  }
}

const filteredQueryResultRows = computed(() => {
  const curRes = currentTab.value?.result
  if (!curRes || !curRes.rows) return []
  const text = (currentTab.value?.filterText || '').trim().toLowerCase()
  if (!text) return curRes.rows
  return curRes.rows.filter((row: any) => {
    return Object.values(row).some(v => String(v).toLowerCase().includes(text))
  })
})

const pagedQueryResultRows = computed(() => {
  const page = currentTab.value?.currentPage || 1
  const size = currentTab.value?.pageSize || 50
  const start = (page - 1) * size
  return filteredQueryResultRows.value.slice(start, start + size)
})

const handleExportCsv = () => {
  if (!isQueryExportSupported.value) {
    ElMessage.error('当前数据库实例未开启【支持数据导出】权限，严禁导出脱库！')
    return
  }
  const curRes = currentTab.value?.result
  if (!curRes || !curRes.rows || curRes.rows.length === 0) {
    ElMessage.warning('当前窗口暂无查询结果可供导出')
    return
  }
  const cols = curRes.columns || []
  const rows = curRes.rows || []

  let csvContent = '\uFEFF' + cols.join(',') + '\n'
  rows.forEach((r: any) => {
    const line = cols.map((c: string) => `"${String(r[c] !== undefined && r[c] !== null ? r[c] : '').replace(/"/g, '""')}"`).join(',')
    csvContent += line + '\n'
  })

  const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8;' })
  const url = URL.createObjectURL(blob)
  const link = document.createElement('a')
  link.href = url
  link.download = `${selectedDbName.value}_${currentTab.value?.name || 'query'}_${Date.now()}.csv`
  link.click()
  URL.revokeObjectURL(url)
  ElMessage.success('CSV 数据导出成功')
}

const handleExportJson = () => {
  if (!isQueryExportSupported.value) {
    ElMessage.error('当前数据库实例未开启【支持数据导出】权限，严禁导出脱库！')
    return
  }
  const curRes = currentTab.value?.result
  if (!curRes || !curRes.rows || curRes.rows.length === 0) {
    ElMessage.warning('当前窗口暂无查询结果可供导出')
    return
  }
  const jsonStr = JSON.stringify(curRes.rows, null, 2)
  const blob = new Blob([jsonStr], { type: 'application/json;charset=utf-8;' })
  const url = URL.createObjectURL(blob)
  const link = document.createElement('a')
  link.href = url
  link.download = `${selectedDbName.value}_${currentTab.value?.name || 'query'}_${Date.now()}.json`
  link.click()
  URL.revokeObjectURL(url)
  ElMessage.success('JSON 数据导出成功')
}

const handleGoToMasking = (tableName?: string) => {
  const query: any = {}
  if (selectedInstanceId.value) query.instanceId = selectedInstanceId.value
  if (selectedDbName.value) query.dbName = selectedDbName.value
  if (typeof tableName === 'string' && tableName) {
    query.tableName = tableName
  } else if (currentActiveTable.value) {
    query.tableName = currentActiveTable.value
  }
  router.push({ path: '/data-masking', query })
}

const getMaskRuleForCol = (col: string): string => {
  if (!col) return ''
  // 1. 优先读取后端直接返回的动态脱敏字段映射 (字段名 -> 规则名)
  const res = currentTab.value?.result
  if (res?.maskedColumns && res.maskedColumns[col]) {
    return res.maskedColumns[col]
  }
  // 2. 字段名敏感模式匹配兜底提示
  const lower = col.toLowerCase()
  if (lower.includes('phone') || lower.includes('mobile') || lower.includes('tel')) return '手机号掩码'
  if (lower.includes('id_card') || lower.includes('idcard') || lower.includes('identity')) return '身份证掩码'
  if (lower.includes('email') || lower.includes('mail')) return '邮箱掩码'
  if (lower.includes('bank') || lower.includes('card_no')) return '银行卡掩码'
  if (lower.includes('password') || lower.includes('passwd') || lower.includes('secret') || lower.includes('token')) return '密码掩码'
  if (lower.includes('address') || lower.includes('addr')) return '地址掩码'
  if (lower === 'name' || lower === 'real_name' || lower === 'username' || lower.includes('owner')) return '姓名掩码'
  return ''
}

onMounted(() => {
  loadInstances()
})
</script>

<style scoped>
.data-query-workbench {
  min-height: 100%;
  height: auto;
  width: 100%;
  min-width: 0;
  display: flex;
  flex-direction: column;
  box-sizing: border-box;
}

.workbench-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
  flex-wrap: wrap;
  gap: 10px;
}

.header-titles .main-title {
  font-size: 18px;
  font-weight: 700;
  color: #1f2d3d;
  margin: 0;
}

.header-titles .sub-title {
  font-size: 13px;
  color: #909399;
  margin-top: 4px;
  display: block;
}

.header-right {
  display: flex;
  align-items: center;
}

.query-top-toolbar {
  background: #ffffff;
  border: 1px solid #e4e7ed;
  border-radius: 6px;
  padding: 12px 16px;
  margin-bottom: 12px;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.04);
}

.toolbar-items {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 16px;
}

.toolbar-item {
  display: flex;
  align-items: center;
  gap: 8px;
}

.item-label {
  font-size: 13px;
  font-weight: 600;
  color: #606266;
}

.toolbar-actions {
  margin-left: auto;
  display: flex;
  gap: 10px;
}

/* ==================== 左右分栏布局 ==================== */
.main-split-layout {
  display: flex;
  gap: 14px;
  flex: 1;
  min-height: 520px;
}

.left-table-sidebar {
  width: 270px;
  background: #ffffff;
  border: 1px solid #e4e7ed;
  border-radius: 6px;
  display: flex;
  flex-direction: column;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.04);
  flex-shrink: 0;
  transition: width 0.22s ease;
}

.left-table-sidebar.is-collapsed {
  width: 40px;
}

.sidebar-collapsed-rail {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 14px 4px;
  gap: 10px;
  cursor: pointer;
  height: 100%;
  color: #475569;
  user-select: none;
  transition: background 0.2s;
}

.sidebar-collapsed-rail:hover {
  background: #eff6ff;
  color: #2563eb;
}

.collapsed-vertical-text {
  writing-mode: vertical-rl;
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 2px;
}

.sidebar-header {
  padding: 12px;
  border-bottom: 1px solid #ebeef5;
  background: #f8f9fb;
}

.sidebar-title {
  font-size: 13px;
  font-weight: 700;
  color: #303133;
  display: flex;
  align-items: center;
  gap: 6px;
}

.table-list-scroll {
  flex: 1;
  overflow-y: auto;
  padding: 6px;
}

.table-list-item {
  padding: 8px 10px;
  border-radius: 4px;
  cursor: pointer;
  margin-bottom: 4px;
  transition: all 0.2s;
}

.table-list-item:hover {
  background-color: #ecf5ff;
}

.table-list-item.active {
  background-color: #d9ecff;
  border-left: 3px solid #409EFF;
}

.tbl-item-top {
  display: flex;
  align-items: center;
  gap: 6px;
}

.tbl-icon {
  color: #409EFF;
  font-size: 14px;
}

.tbl-name {
  font-size: 13px;
  font-weight: 600;
  color: #303133;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.tbl-item-desc {
  font-size: 11px;
  color: #909399;
  margin-top: 2px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.empty-tables-tip {
  text-align: center;
  color: #909399;
  font-size: 12px;
  padding: 30px 10px;
}

/* ==================== 右侧主区 ==================== */
.right-query-main {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-width: 0;
}

.editor-box {
  background: #ffffff;
  border: 1px solid #e4e7ed;
  border-radius: 6px;
  overflow: hidden;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.04);
}

.editor-tools {
  background: #f5f7fa;
  padding: 8px 14px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  border-bottom: 1px solid #e4e7ed;
}

.tools-left {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.editor-label {
  font-size: 13px;
  font-weight: 600;
  color: #303133;
  display: flex;
  align-items: center;
  gap: 6px;
  margin-right: 6px;
}

.quick-sql-tag {
  cursor: pointer;
  transition: all 0.2s;
}

.quick-sql-tag:hover {
  border-color: #409EFF;
  color: #409EFF;
  background: #ecf5ff;
}

.ide-code-textarea :deep(textarea) {
  font-family: Consolas, 'Fira Code', Monaco, monospace;
  font-size: 13px;
  line-height: 1.5;
  background: #282c34;
  color: #abb2bf;
  border: none;
}

.result-tabs-container {
  flex: 1;
  background: #ffffff;
  border: 1px solid #e4e7ed;
  border-radius: 6px;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.04);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.custom-result-tabs :deep(.el-tabs__header) {
  margin: 0;
  background: #fafbfc;
  padding: 0 12px;
}

.grid-card {
  display: flex;
  flex-direction: column;
}

.grid-status-bar {
  background: #f8f9fa;
  padding: 8px 14px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  border-bottom: 1px solid #ebeef5;
}

.status-tag {
  font-size: 12px;
  font-weight: bold;
  padding: 2px 8px;
  border-radius: 4px;
  background: #909399;
  color: #ffffff;
  margin-right: 10px;
}

.status-tag.success {
  background: #67C23A;
}

.status-tag.error {
  background: #F56C6C;
}

.meta-text {
  font-size: 12px;
  color: #606266;
}

.meta-text b {
  color: #303133;
}

.table-render-wrap {
  padding: 8px;
}

.null-val {
  color: #c0c4cc;
  font-style: italic;
}

.empty-result-placeholder {
  padding: 40px 0;
}

.schema-card {
  padding: 10px;
}

.schema-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 6px 4px 12px 4px;
  font-size: 13px;
  color: #606266;
}

/* ==================== 多查询窗口标签栏样式 ==================== */
.query-tabs-header-bar {
  background: #f8fafc;
  border: 1px solid #e2e8f0;
  border-bottom: none;
  border-top-left-radius: 6px;
  border-top-right-radius: 6px;
  padding: 4px 8px 0 8px;
}

.multi-query-tabs :deep(.el-tabs__header) {
  margin: 0;
  border-bottom: none;
}

.multi-query-tabs :deep(.el-tabs__nav) {
  border-radius: 4px 4px 0 0;
  border-color: #cbd5e1;
}

.multi-query-tabs :deep(.el-tabs__item) {
  font-size: 12.5px;
  font-weight: 500;
  height: 34px;
  line-height: 34px;
  padding: 0 14px !important;
  color: #64748b;
  border-color: #cbd5e1;
  background: #f1f5f9;
  transition: all 0.2s ease;
}

.multi-query-tabs :deep(.el-tabs__item.is-active) {
  color: #2563eb;
  background: #ffffff;
  font-weight: 600;
  border-bottom-color: #ffffff;
}

.tab-pill-label {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  cursor: pointer;
}

.tab-title-text {
  max-width: 130px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tab-rows-tag {
  font-size: 10.5px;
  background: #ecfdf5;
  color: #059669;
  border: 1px solid #a7f3d0;
  border-radius: 10px;
  padding: 0 5px;
  line-height: 16px;
  font-weight: 600;
}

.tab-err-tag {
  font-size: 10.5px;
  background: #fef2f2;
  color: #dc2626;
  border: 1px solid #fecaca;
  border-radius: 10px;
  padding: 0 5px;
  line-height: 16px;
  font-weight: 600;
}

.tab-edit-icon {
  margin-left: 2px;
  font-size: 12px;
  color: #94a3b8;
  opacity: 0;
  transition: opacity 0.2s;
}

.tab-pill-label:hover .tab-edit-icon {
  opacity: 1;
}
</style>
