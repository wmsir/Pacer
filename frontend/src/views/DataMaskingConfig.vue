<template>
  <div class="data-masking-container page-container">
    <!-- 顶部标题与说明 -->
    <div class="header-action">
      <div class="title-area">
        <h2 class="page-title">动态数据脱敏配置 (Dynamic Data Masking)</h2>
        <div class="page-subtitle">选择目标数据库实例、Schema 与数据表，为敏感字段配置脱敏算法，保护生产数据安全防外泄</div>
      </div>
    </div>

    <el-tabs v-model="activeTab" class="custom-tabs">
      <!-- Tab 1: 按表字段配置脱敏 -->
      <el-tab-pane label="表字段脱敏配置" name="config">
        <!-- 级联选择栏 -->
        <el-card shadow="never" class="filter-card">
          <div class="cascade-selector-row">
            <div class="selector-item">
              <span class="selector-label">1. 数据库实例：</span>
              <el-select
                v-model="selectedInstanceId"
                placeholder="请选择目标实例"
                style="width: 240px;"
                @change="handleInstanceChange"
              >
                <el-option
                  v-for="item in instanceList"
                  :key="item.id"
                  :label="`${item.name} (${item.dbType || 'mysql'})`"
                  :value="item.id"
                />
              </el-select>
            </div>

            <div class="selector-item">
              <span class="selector-label">2. 目标数据库：</span>
              <el-select
                v-model="selectedDbName"
                placeholder="请选择数据库"
                style="width: 200px;"
                :disabled="!selectedInstanceId"
                @change="handleDbChange"
              >
                <el-option
                  v-for="db in dbList"
                  :key="db"
                  :label="db"
                  :value="db"
                />
              </el-select>
            </div>

            <div class="selector-item">
              <span class="selector-label">3. 数据表 (Table)：</span>
              <el-select
                v-model="selectedTableName"
                placeholder="请选择数据表"
                style="width: 240px;"
                filterable
                :disabled="!selectedDbName"
                @change="handleTableChange"
              >
                <el-option
                  v-for="tbl in tableList"
                  :key="tbl.tableName"
                  :label="tbl.tableComment ? `${tbl.tableName} (${tbl.tableComment})` : tbl.tableName"
                  :value="tbl.tableName"
                />
              </el-select>
            </div>

            <div class="action-buttons">
              <el-button type="primary" :icon="Check" :loading="saveLoading" :disabled="!selectedTableName || columns.length === 0" @click="handleSaveTableRules">
                保存脱敏配置
              </el-button>
              <el-button :icon="Refresh" :loading="columnLoading" :disabled="!selectedTableName" @click="fetchTableColumns">
                刷新字段
              </el-button>
            </div>
          </div>
        </el-card>

        <!-- 字段脱敏规则表格 -->
        <div class="table-wrapper" v-if="selectedTableName" v-loading="columnLoading">
          <div class="table-header-info">
            <div class="current-table-tag">
              当前配置表：<b>{{ selectedDbName }}.{{ selectedTableName }}</b>
              <span class="col-count-text">（共 {{ columns.length }} 个字段，已配置 {{ configuredRulesCount }} 个脱敏字段）</span>
            </div>
            <div class="quick-preset-btns">
              <el-button size="small" type="primary" plain @click="handleAutoSuggestMasking">
                <el-icon style="margin-right: 4px;"><MagicStick /></el-icon>
                AI 智能识别敏感字段
              </el-button>
              <el-button size="small" @click="handleResetAllColumns">清空脱敏</el-button>
            </div>
          </div>

          <el-table :data="columns" border stripe style="width: 100%">
            <el-table-column type="index" label="序号" width="60" align="center" />

            <el-table-column prop="columnName" label="字段名称" min-width="160">
              <template #default="scope">
                <div style="display: flex; align-items: center; gap: 6px;">
                  <span style="font-family: monospace; font-weight: 600; color: #303133;">{{ scope.row.columnName }}</span>
                  <el-tag v-if="scope.row.columnKey === 'PRI'" size="small" type="danger" effect="plain">主键 PK</el-tag>
                </div>
              </template>
            </el-table-column>

            <el-table-column prop="columnType" label="字段类型" width="130" align="center">
              <template #default="scope">
                <el-tag size="small" type="info" effect="plain">{{ scope.row.columnType }}</el-tag>
              </template>
            </el-table-column>

            <el-table-column prop="columnComment" label="字段注释与说明" min-width="150" show-overflow-tooltip>
              <template #default="scope">
                <span style="color: #606266; font-size: 13px;">{{ scope.row.columnComment || '-' }}</span>
              </template>
            </el-table-column>

            <el-table-column label="脱敏算法规则" width="220">
              <template #default="scope">
                <el-select
                  v-model="scope.row.ruleType"
                  size="small"
                  style="width: 100%;"
                  @change="() => updateLivePreview(scope.row)"
                >
                  <el-option
                    v-for="alg in algorithmList"
                    :key="alg.type"
                    :label="alg.name"
                    :value="alg.type"
                  />
                </el-select>
              </template>
            </el-table-column>

            <el-table-column label="实时脱敏效果预览 (样例)" min-width="220">
              <template #default="scope">
                <div v-if="scope.row.ruleType && scope.row.ruleType !== 'NONE'" class="preview-box">
                  <span class="preview-raw">{{ getSampleOriginal(scope.row.ruleType) }}</span>
                  <el-icon style="margin: 0 6px; color: #909399;"><Right /></el-icon>
                  <span class="preview-masked">{{ scope.row.previewVal || getSampleMasked(scope.row.ruleType) }}</span>
                </div>
                <span v-else style="color: #c0c4cc; font-size: 12px;">明文输出 (未脱敏)</span>
              </template>
            </el-table-column>

            <el-table-column label="规则状态" width="100" align="center">
              <template #default="scope">
                <el-switch
                  v-model="scope.row.status"
                  :active-value="1"
                  :inactive-value="0"
                  size="small"
                  :disabled="!scope.row.ruleType || scope.row.ruleType === 'NONE'"
                />
              </template>
            </el-table-column>
          </el-table>
        </div>

        <el-empty v-else description="请在上方依次选择【数据库实例】、【目标数据库】及【数据表】开始配置脱敏规则" />
      </el-tab-pane>

      <!-- Tab 2: 已配置规则全局看板 -->
      <el-tab-pane label="全库生效脱敏规则看板" name="overview">
        <div class="overview-header">
          <el-input
            v-model="overviewKeyword"
            placeholder="搜索表名/字段名/脱敏算法..."
            clearable
            style="width: 320px;"
            :prefix-icon="Search"
            @clear="fetchAllRules"
            @keyup.enter="fetchAllRules"
          />
          <el-button :icon="Refresh" :loading="overviewLoading" @click="fetchAllRules">刷新看板</el-button>
        </div>

        <div class="table-wrapper">
          <el-table :data="allRules" border stripe style="width: 100%" v-loading="overviewLoading">
            <el-table-column prop="id" label="ID" width="70" align="center" />
            <el-table-column prop="dbName" label="目标数据库" width="150" />
            <el-table-column prop="tableName" label="数据表" width="180">
              <template #default="scope">
                <span style="font-family: monospace; font-weight: 600;">{{ scope.row.tableName }}</span>
              </template>
            </el-table-column>
            <el-table-column prop="columnName" label="脱敏字段" width="160">
              <template #default="scope">
                <el-tag size="small" type="primary" effect="plain" style="font-family: monospace;">
                  {{ scope.row.columnName }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="ruleType" label="脱敏算法" width="160">
              <template #default="scope">
                <el-tag size="small" :type="getAlgTagType(scope.row.ruleType)">
                  {{ formatAlgName(scope.row.ruleType) }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="columnComment" label="字段说明" min-width="160" show-overflow-tooltip />
            <el-table-column prop="status" label="状态" width="100" align="center">
              <template #default="scope">
                <el-tag :type="scope.row.status === 1 ? 'success' : 'info'">
                  {{ scope.row.status === 1 ? '生效中' : '已停用' }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column label="操作" width="160" fixed="right" align="center">
              <template #default="scope">
                <el-button
                  size="small"
                  :type="scope.row.status === 1 ? 'warning' : 'success'"
                  plain
                  @click="handleToggleOverviewStatus(scope.row)"
                >
                  {{ scope.row.status === 1 ? '停用' : '启用' }}
                </el-button>
                <el-button size="small" type="danger" plain :icon="Delete" @click="handleDeleteRule(scope.row)">
                  删除
                </el-button>
              </template>
            </el-table-column>
          </el-table>

          <!-- 分页控制栏 -->
          <div class="pagination-bar" style="display: flex; justify-content: flex-end; margin-top: 16px;">
            <el-pagination
              v-model:current-page="pagination.current"
              v-model:page-size="pagination.size"
              :page-sizes="[10, 20, 50]"
              :total="pagination.total"
              layout="total, sizes, prev, pager, next, jumper"
              background
              @size-change="handleSizeChange"
              @current-change="handleCurrentChange"
            />
          </div>
        </div>
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Refresh, Check, MagicStick, Right, Search, Delete } from '@element-plus/icons-vue'
import request from '../utils/request'

const route = useRoute()

interface AlgorithmItem {
  type: string
  name: string
  description: string
  sampleOriginal: string
  sampleMasked: string
}

interface ColumnConfigItem {
  columnName: string
  columnType: string
  columnKey: string
  columnComment: string
  ruleType: string
  customRegex?: string
  customReplacement?: string
  status: number
  previewVal?: string
}

const activeTab = ref('config')
const selectedInstanceId = ref<number | null>(null)
const selectedDbName = ref<string>('')
const selectedTableName = ref<string>('')

const instanceList = ref<any[]>([])
const dbList = ref<string[]>([])
const tableList = ref<any[]>([])
const columns = ref<ColumnConfigItem[]>([])
const algorithmList = ref<AlgorithmItem[]>([])
const allRules = ref<any[]>([])

const columnLoading = ref(false)
const saveLoading = ref(false)
const overviewLoading = ref(false)
const overviewKeyword = ref('')

const configuredRulesCount = computed(() => {
  return columns.value.filter(c => c.ruleType && c.ruleType !== 'NONE' && c.status === 1).length
})

const fetchAlgorithms = async () => {
  try {
    const res: any = await request.get('/v1/masking-rule/algorithms')
    algorithmList.value = Array.isArray(res.data) ? res.data : []
  } catch (error) {
    // fallback
  }
}

const fetchInstances = async () => {
  try {
    const res: any = await request.get('/v1/instance/list')
    instanceList.value = Array.isArray(res.data) ? res.data : []

    const qInstId = route.query.instanceId ? Number(route.query.instanceId) : null
    const qDb = route.query.dbName ? String(route.query.dbName) : ''
    const qTbl = route.query.tableName ? String(route.query.tableName) : ''

    if (qInstId && instanceList.value.some(i => i.id === qInstId)) {
      selectedInstanceId.value = qInstId
      await handleInstanceChange(qDb, qTbl)
    } else if (instanceList.value.length > 0 && !selectedInstanceId.value) {
      selectedInstanceId.value = instanceList.value[0].id
      await handleInstanceChange()
    }
  } catch (error) {
    ElMessage.error('获取实例列表失败')
  }
}

const handleInstanceChange = async (targetDb?: string, targetTable?: string) => {
  selectedDbName.value = ''
  selectedTableName.value = ''
  dbList.value = []
  tableList.value = []
  columns.value = []

  if (!selectedInstanceId.value) return
  try {
    const res: any = await request.get(`/v1/instance/${selectedInstanceId.value}/databases`)
    dbList.value = Array.isArray(res.data) ? res.data : []
    if (dbList.value.length > 0) {
      if (targetDb && dbList.value.includes(targetDb)) {
        selectedDbName.value = targetDb
      } else {
        selectedDbName.value = dbList.value[0]
      }
      await handleDbChange(targetTable)
    }
  } catch (error) {
    ElMessage.error('获取数据库列表失败')
  }
}

const handleDbChange = async (targetTable?: string) => {
  selectedTableName.value = ''
  tableList.value = []
  columns.value = []

  if (!selectedInstanceId.value || !selectedDbName.value) return
  try {
    const res: any = await request.get(`/v1/instance/${selectedInstanceId.value}/databases/${selectedDbName.value}/tables`)
    tableList.value = Array.isArray(res.data) ? res.data : []
    if (tableList.value.length > 0) {
      if (targetTable && tableList.value.some(t => t.tableName === targetTable)) {
        selectedTableName.value = targetTable
      } else {
        selectedTableName.value = tableList.value[0].tableName
      }
      fetchTableColumns()
    }
  } catch (error) {
    ElMessage.error('获取数据表列表失败')
  }
}

const handleTableChange = () => {
  if (selectedTableName.value) {
    fetchTableColumns()
  }
}

const fetchTableColumns = async () => {
  if (!selectedInstanceId.value || !selectedDbName.value || !selectedTableName.value) return
  columnLoading.value = true
  try {
    const [colRes, ruleRes]: any = await Promise.all([
      request.get(`/v1/instance/${selectedInstanceId.value}/databases/${selectedDbName.value}/tables/${selectedTableName.value}/columns`),
      request.get('/v1/masking-rule/list', {
        params: {
          instanceId: selectedInstanceId.value,
          dbName: selectedDbName.value,
          tableName: selectedTableName.value
        }
      })
    ])

    const rawCols = Array.isArray(colRes.data) ? colRes.data : []
    const existingRules = Array.isArray(ruleRes.data) ? ruleRes.data : []
    const ruleMap = new Map<string, any>()
    existingRules.forEach((r: any) => {
      ruleMap.set(r.columnName.toLowerCase(), r)
    })

    columns.value = rawCols.map((c: any) => {
      const match = ruleMap.get(c.columnName.toLowerCase())
      return {
        columnName: c.columnName,
        columnType: c.columnType,
        columnKey: c.columnKey,
        columnComment: c.columnComment,
        ruleType: match ? match.ruleType : 'NONE',
        customRegex: match ? match.customRegex : '',
        customReplacement: match ? match.customReplacement : '',
        status: match ? match.status : 1
      }
    })
  } catch (error) {
    ElMessage.error('加载字段结构失败')
  } finally {
    columnLoading.value = false
  }
}

const getSampleOriginal = (ruleType: string) => {
  const item = algorithmList.value.find(a => a.type === ruleType)
  return item ? item.sampleOriginal : ''
}

const getSampleMasked = (ruleType: string) => {
  const item = algorithmList.value.find(a => a.type === ruleType)
  return item ? item.sampleMasked : ''
}

const updateLivePreview = async (row: ColumnConfigItem) => {
  if (!row.ruleType || row.ruleType === 'NONE') {
    row.previewVal = ''
    return
  }
  try {
    const res: any = await request.post('/v1/masking-rule/preview', {
      ruleType: row.ruleType,
      sampleValue: getSampleOriginal(row.ruleType),
      customRegex: row.customRegex,
      customReplacement: row.customReplacement
    })
    row.previewVal = res.data || getSampleMasked(row.ruleType)
  } catch (e) {
    row.previewVal = getSampleMasked(row.ruleType)
  }
}

const handleAutoSuggestMasking = () => {
  let count = 0
  columns.value.forEach(col => {
    const name = col.columnName.toLowerCase()
    const comment = (col.columnComment || '').toLowerCase()

    if (name.includes('phone') || name.includes('mobile') || name.includes('tel') || comment.includes('手机') || comment.includes('电话')) {
      col.ruleType = 'PHONE'
      col.status = 1
      count++
    } else if (name.includes('id_card') || name.includes('idcard') || name.includes('identity') || comment.includes('身份证')) {
      col.ruleType = 'ID_CARD'
      col.status = 1
      count++
    } else if (name.includes('real_name') || name.includes('realname') || name === 'name' || comment.includes('姓名') || comment.includes('真实姓名')) {
      col.ruleType = 'NAME'
      col.status = 1
      count++
    } else if (name.includes('email') || name.includes('mail') || comment.includes('邮箱')) {
      col.ruleType = 'EMAIL'
      col.status = 1
      count++
    } else if (name.includes('bank') || name.includes('card_no') || comment.includes('银行卡')) {
      col.ruleType = 'BANK_CARD'
      col.status = 1
      count++
    } else if (name.includes('password') || name.includes('pwd') || name.includes('secret') || comment.includes('密码')) {
      col.ruleType = 'PASSWORD'
      col.status = 1
      count++
    } else if (name.includes('address') || comment.includes('地址') || comment.includes('住址')) {
      col.ruleType = 'ADDRESS'
      col.status = 1
      count++
    }
  })

  ElMessage.success(`AI 智能识别完成，已自动为您匹配 ${count} 个敏感字段脱敏规则！`)
}

const handleResetAllColumns = () => {
  columns.value.forEach(col => {
    col.ruleType = 'NONE'
    col.status = 1
    col.previewVal = ''
  })
  ElMessage.info('已重置所有字段规则')
}

const handleSaveTableRules = async () => {
  if (!selectedInstanceId.value || !selectedDbName.value || !selectedTableName.value) return
  saveLoading.value = true

  const payload = columns.value.map(col => ({
    instanceId: selectedInstanceId.value,
    dbName: selectedDbName.value,
    tableName: selectedTableName.value,
    columnName: col.columnName,
    columnType: col.columnType,
    columnComment: col.columnComment,
    ruleType: col.ruleType || 'NONE',
    customRegex: col.customRegex,
    customReplacement: col.customReplacement,
    status: col.status,
    description: `字段 ${col.columnName} 规则`
  }))

  try {
    await request.post('/v1/masking-rule/save', payload)
    ElMessage.success(`数据表【${selectedTableName.value}】脱敏规则已保存并实时生效！`)
    fetchTableColumns()
    fetchAllRules()
  } catch (err: any) {
    ElMessage.error(err.response?.data?.message || '保存脱敏规则失败')
  } finally {
    saveLoading.value = false
  }
}

const pagination = ref({
  current: 1,
  size: 10,
  total: 0
})

const fetchAllRules = async () => {
  overviewLoading.value = true
  try {
    const res: any = await request.get('/v1/masking-rule/page', {
      params: {
        page: pagination.value.current,
        size: pagination.value.size,
        keyword: overviewKeyword.value.trim()
      }
    })
    if (res.data && res.data.records) {
      allRules.value = res.data.records
      pagination.value.total = res.data.total
      pagination.value.current = res.data.current
      pagination.value.size = res.data.size
    } else {
      allRules.value = Array.isArray(res.data) ? res.data : []
      pagination.value.total = allRules.value.length
    }
  } catch (error) {
    ElMessage.error('加载脱敏规则看板失败')
  } finally {
    overviewLoading.value = false
  }
}

const handleSizeChange = (val: number) => {
  pagination.value.size = val
  pagination.value.current = 1
  fetchAllRules()
}

const handleCurrentChange = (val: number) => {
  pagination.value.current = val
  fetchAllRules()
}

const handleToggleOverviewStatus = async (row: any) => {
  try {
    await request.post(`/v1/masking-rule/${row.id}/toggle-status`)
    ElMessage.success(row.status === 1 ? '已停用脱敏' : '已启用脱敏')
    fetchAllRules()
  } catch (error) {
    ElMessage.error('切换状态失败')
  }
}

const handleDeleteRule = async (row: any) => {
  try {
    await ElMessageBox.confirm(`确认删除字段【${row.tableName}.${row.columnName}】的脱敏规则吗？`, '删除确认', {
      type: 'warning',
      confirmButtonText: '确认删除',
      cancelButtonText: '取消'
    })
    await request.delete(`/v1/masking-rule/${row.id}`)
    ElMessage.success('删除成功')
    fetchAllRules()
  } catch (err) {
    // cancel
  }
}

const formatAlgName = (type: string) => {
  const item = algorithmList.value.find(a => a.type === type)
  return item ? item.name : type
}

const getAlgTagType = (type: string) => {
  if (type === 'PHONE') return 'warning'
  if (type === 'ID_CARD') return 'danger'
  if (type === 'PASSWORD') return 'danger'
  if (type === 'EMAIL') return 'success'
  if (type === 'BANK_CARD') return 'warning'
  return 'primary'
}

onMounted(() => {
  fetchAlgorithms()
  fetchInstances()
  fetchAllRules()
})
</script>

<style scoped>
.header-action {
  margin-bottom: 16px;
}

.page-subtitle {
  font-size: 13px;
  color: #909399;
  margin-top: 4px;
}

.custom-tabs {
  background: #ffffff;
  padding: 16px;
  border-radius: 8px;
}

.filter-card {
  margin-bottom: 16px;
  background: #f8fafc;
  border: 1px solid #e2e8f0;
}

.cascade-selector-row {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 16px;
}

.selector-item {
  display: flex;
  align-items: center;
}

.selector-label {
  font-weight: 500;
  font-size: 13px;
  color: #475569;
  margin-right: 8px;
}

.action-buttons {
  margin-left: auto;
  display: flex;
  gap: 8px;
}

.table-header-info {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px;
  background: #f1f5f9;
  border-radius: 6px 6px 0 0;
  border: 1px solid #e2e8f0;
  border-bottom: none;
}

.current-table-tag {
  font-size: 14px;
  color: #1e293b;
}

.col-count-text {
  font-size: 12px;
  color: #64748b;
  margin-left: 6px;
}

.table-wrapper {
  background: #ffffff;
  overflow-x: auto;
}

.preview-box {
  display: flex;
  align-items: center;
  font-family: monospace;
  font-size: 12px;
}

.preview-raw {
  color: #94a3b8;
  text-decoration: line-through;
}

.preview-masked {
  color: #0284c7;
  font-weight: 600;
  background: #e0f2fe;
  padding: 2px 6px;
  border-radius: 4px;
}

.overview-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}
</style>
