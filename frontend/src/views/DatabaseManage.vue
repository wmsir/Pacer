<template>
  <div class="database-manage-container page-container">
    <!-- 顶部操作栏 -->
    <div class="header-action">
      <div class="title-area">
        <h2 class="page-title">数据库管理 (Schema Management)</h2>
        <div class="page-subtitle">查看实例下所有数据库 (Schema)、数据表数量与空间占用，支持新建数据库与安全删除</div>
      </div>
      <div class="action-area">
        <el-select
          v-model="selectedInstanceId"
          placeholder="请选择目标数据库实例"
          style="width: 280px; margin-right: 12px;"
          @change="fetchDatabases"
        >
          <el-option
            v-for="item in instanceList"
            :key="item.id"
            :label="`${item.name} (${item.dbType || 'mysql'})`"
            :value="item.id"
          />
        </el-select>
        <el-button :icon="Refresh" :loading="loading" @click="fetchDatabases">刷新</el-button>
        <el-button type="primary" :icon="Plus" :disabled="!selectedInstanceId" @click="handleOpenCreateDialog">
          新建数据库
        </el-button>
      </div>
    </div>

    <!-- 统计指标概览条 -->
    <el-row :gutter="16" class="metrics-row" v-if="selectedInstanceId">
      <el-col :span="6">
        <div class="metric-card">
          <span class="m-label">数据库总数</span>
          <span class="m-value">{{ databases.length }} <small>个</small></span>
        </div>
      </el-col>
      <el-col :span="6">
        <div class="metric-card">
          <span class="m-label">业务数据库</span>
          <span class="m-value highlight">{{ businessDatabasesCount }} <small>个</small></span>
        </div>
      </el-col>
      <el-col :span="6">
        <div class="metric-card">
          <span class="m-label">数据表总数</span>
          <span class="m-value">{{ totalTableCount }} <small>张</small></span>
        </div>
      </el-col>
      <el-col :span="6">
        <div class="metric-card">
          <span class="m-label">数据存储总占用</span>
          <span class="m-value">{{ totalDataSizeMB }} <small>MB</small></span>
        </div>
      </el-col>
    </el-row>

    <!-- 数据库列表表格 -->
    <div class="table-wrapper">
      <el-table :data="pagedDatabases" border stripe style="width: 100%" v-loading="loading">
        <el-table-column type="index" label="序号" width="60" align="center" />

        <el-table-column prop="dbName" label="数据库名称 (Schema)" min-width="180">
          <template #default="scope">
            <div style="display: flex; align-items: center; gap: 8px;">
              <el-icon :color="scope.row.isSystem ? '#909399' : '#409EFF'"><Coin /></el-icon>
              <span style="font-weight: 600; font-family: monospace;">{{ scope.row.dbName }}</span>
              <el-tag v-if="scope.row.isSystem" size="small" type="info" effect="plain">系统库</el-tag>
            </div>
          </template>
        </el-table-column>

        <el-table-column prop="charset" label="默认字符集" width="130" align="center">
          <template #default="scope">
            <el-tag size="small" type="success" effect="plain">{{ scope.row.charset || 'utf8mb4' }}</el-tag>
          </template>
        </el-table-column>

        <el-table-column prop="collation" label="排序规则 (Collation)" width="180" show-overflow-tooltip>
          <template #default="scope">
            <span style="font-size: 12px; color: #606266;">{{ scope.row.collation || 'utf8mb4_0900_ai_ci' }}</span>
          </template>
        </el-table-column>

        <el-table-column prop="tableCount" label="数据表数量" width="130" align="center" sortable>
          <template #default="scope">
            <el-tag
              size="small"
              :type="scope.row.tableCount > 0 ? 'primary' : 'info'"
              effect="light"
              style="font-weight: 600; padding: 0 10px;"
            >
              {{ scope.row.tableCount }} 张
            </el-tag>
          </template>
        </el-table-column>

        <el-table-column prop="dataSizeMB" label="数据占用大小" width="130" align="right">
          <template #default="scope">
            <span style="font-weight: 600; color: #E6A23C;">{{ scope.row.dataSizeMB }} MB</span>
          </template>
        </el-table-column>

        <el-table-column prop="comment" label="用途类型" min-width="140" show-overflow-tooltip />

        <el-table-column label="操作" width="220" fixed="right" align="center">
          <template #default="scope">
            <el-button
              size="small"
              type="warning"
              plain
              :icon="Hide"
              @click="handleGoToMasking(scope.row)"
            >
              字段脱敏
            </el-button>
            <el-button
              size="small"
              type="danger"
              plain
              :icon="Delete"
              :disabled="scope.row.isSystem"
              @click="handleDropDatabase(scope.row)"
            >
              删除库
            </el-button>
          </template>
        </el-table-column>
      </el-table>

      <!-- 分页控制栏 -->
      <div class="pagination-bar" style="display: flex; justify-content: flex-end; margin-top: 16px;">
        <el-pagination
          v-model:current-page="currentPage"
          v-model:page-size="pageSize"
          :page-sizes="[10, 20, 50]"
          :total="databases.length"
          layout="total, sizes, prev, pager, next, jumper"
          background
        />
      </div>
    </div>

    <!-- 新建数据库弹窗 -->
    <el-dialog title="新建数据库 (Create Database)" v-model="createDialogVisible" width="520px" destroy-on-close>
      <el-form ref="createFormRef" :model="createForm" :rules="createRules" label-width="110px">
        <el-form-item label="数据库名称" prop="dbName">
          <el-input v-model="createForm.dbName" placeholder="如：order_service_db (仅支持字母数字下划线)" />
        </el-form-item>

        <el-form-item label="字符集" prop="charset">
          <el-select v-model="createForm.charset" style="width: 100%;">
            <el-option label="utf8mb4 (推荐，完整支持 Emoji 及全部 Unicode 字符)" value="utf8mb4" />
            <el-option label="utf8 (标准 UTF-8)" value="utf8" />
            <el-option label="gbk (支持中文简体)" value="gbk" />
            <el-option label="latin1 (ISO-8859-1)" value="latin1" />
          </el-select>
        </el-form-item>

        <el-form-item label="排序规则" prop="collation">
          <el-select v-model="createForm.collation" style="width: 100%;">
            <el-option label="utf8mb4_0900_ai_ci (MySQL 8.0 默认)" value="utf8mb4_0900_ai_ci" />
            <el-option label="utf8mb4_general_ci (通用高效不区分大小写)" value="utf8mb4_general_ci" />
            <el-option label="utf8mb4_bin (二进制精确比对区分大小写)" value="utf8mb4_bin" />
            <el-option label="gbk_chinese_ci (GBK 默认)" value="gbk_chinese_ci" />
          </el-select>
        </el-form-item>

        <el-form-item label="业务备注">
          <el-input v-model="createForm.comment" placeholder="可选，说明该数据库业务用途" />
        </el-form-item>
      </el-form>

      <template #footer>
        <el-button @click="createDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="createLoading" @click="handleCreateDatabase">
          立即创建
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import { Refresh, Plus, Delete, Coin, Hide } from '@element-plus/icons-vue'
import request from '../utils/request'

const router = useRouter()

interface DbSchemaItem {
  dbName: string
  charset: string
  collation: string
  tableCount: number
  dataSizeMB: number
  isSystem: boolean
  comment?: string
}

const loading = ref(false)
const createLoading = ref(false)
const createDialogVisible = ref(false)
const selectedInstanceId = ref<number | null>(null)
const instanceList = ref<any[]>([])
const databases = ref<DbSchemaItem[]>([])

const currentPage = ref(1)
const pageSize = ref(10)

const pagedDatabases = computed(() => {
  const start = (currentPage.value - 1) * pageSize.value
  return databases.value.slice(start, start + pageSize.value)
})

const createFormRef = ref<FormInstance>()

const createForm = ref({
  dbName: '',
  charset: 'utf8mb4',
  collation: 'utf8mb4_0900_ai_ci',
  comment: ''
})

const createRules = ref<FormRules>({
  dbName: [
    { required: true, message: '请输入数据库名称', trigger: 'blur' },
    { pattern: /^[a-zA-Z0-9_]+$/, message: '只能包含字母、数字和下划线', trigger: 'blur' }
  ],
  charset: [{ required: true, message: '请选择字符集', trigger: 'change' }]
})

const businessDatabasesCount = computed(() => {
  return databases.value.filter(d => !d.isSystem).length
})

const totalTableCount = computed(() => {
  return databases.value.reduce((acc, cur) => acc + (cur.tableCount || 0), 0)
})

const totalDataSizeMB = computed(() => {
  const total = databases.value.reduce((acc, cur) => acc + (cur.dataSizeMB || 0), 0)
  return Math.round(total * 100) / 100
})

const fetchInstances = async () => {
  try {
    const res: any = await request.get('/v1/instance/list')
    instanceList.value = Array.isArray(res.data) ? res.data : []
    if (instanceList.value.length > 0 && !selectedInstanceId.value) {
      selectedInstanceId.value = instanceList.value[0].id
      fetchDatabases()
    }
  } catch (error) {
    ElMessage.error('获取实例列表失败')
  }
}

const fetchDatabases = async () => {
  if (!selectedInstanceId.value) return
  loading.value = true
  try {
    const res: any = await request.get(`/v1/instance/${selectedInstanceId.value}/databases-detail`)
    databases.value = Array.isArray(res.data) ? res.data : []
  } catch (error) {
    ElMessage.error('获取数据库详情列表失败')
  } finally {
    loading.value = false
  }
}

const handleOpenCreateDialog = () => {
  createForm.value = {
    dbName: '',
    charset: 'utf8mb4',
    collation: 'utf8mb4_0900_ai_ci',
    comment: ''
  }
  createDialogVisible.value = true
}

const handleCreateDatabase = async () => {
  if (!createFormRef.value) return
  await createFormRef.value.validate(async (valid) => {
    if (!valid) return
    createLoading.value = true
    try {
      await request.post(`/v1/instance/${selectedInstanceId.value}/databases/create`, createForm.value)
      ElMessage.success(`数据库【${createForm.value.dbName}】创建成功！`)
      createDialogVisible.value = false
      fetchDatabases()
    } catch (err: any) {
      ElMessage.error(err.response?.data?.message || '创建数据库失败')
    } finally {
      createLoading.value = false
    }
  })
}

const handleDropDatabase = async (row: DbSchemaItem) => {
  try {
    await ElMessageBox.prompt(
      `删除数据库属于高危不可逆操作，将永久清空【${row.dbName}】库下全部 ${row.tableCount} 张数据表！请输入数据库名 "${row.dbName}" 确认删除：`,
      '高危警告：确认删除数据库',
      {
        confirmButtonText: '强制删除',
        cancelButtonText: '取消',
        confirmButtonClass: 'el-button--danger',
        inputPattern: new RegExp(`^${row.dbName}$`),
        inputErrorMessage: '输入的数据库名称不匹配，操作已取消'
      }
    )

    await request.delete(`/v1/instance/${selectedInstanceId.value}/databases/${row.dbName}`)
    ElMessage.success(`数据库【${row.dbName}】已安全删除`)
    fetchDatabases()
  } catch (err) {
    if (err !== 'cancel') {
      ElMessage.error('删除数据库失败')
    }
  }
}

const handleGoToMasking = (row: DbSchemaItem) => {
  router.push({
    path: '/data-masking',
    query: {
      instanceId: selectedInstanceId.value,
      dbName: row.dbName
    }
  })
}

onMounted(() => {
  fetchInstances()
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
