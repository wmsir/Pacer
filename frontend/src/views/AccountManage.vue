<template>
  <div class="account-manage-container page-container">
    <!-- 顶部操作栏 -->
    <div class="header-action">
      <div class="title-area">
        <h2 class="page-title">账号管理 (Database Accounts & Privileges)</h2>
        <div class="page-subtitle">管理目标数据库实例的账号清单、客户端连接白名单主机与权限范围，支持创建账号与密码重置</div>
      </div>
      <div class="action-area">
        <el-select
          v-model="selectedInstanceId"
          placeholder="请选择目标数据库实例"
          style="width: 280px; margin-right: 12px;"
          @change="fetchAccounts"
        >
          <el-option
            v-for="item in instanceList"
            :key="item.id"
            :label="`${item.name} (${item.dbType || 'mysql'})`"
            :value="item.id"
          />
        </el-select>
        <el-button :icon="Refresh" :loading="loading" @click="fetchAccounts">刷新</el-button>
        <el-button type="primary" :icon="Plus" :disabled="!selectedInstanceId" @click="handleOpenCreateDialog">
          新建数据库账号
        </el-button>
      </div>
    </div>

    <!-- 账号列表表格 -->
    <div class="table-wrapper">
      <el-table :data="pagedAccounts" border stripe style="width: 100%" v-loading="loading">
        <el-table-column type="index" label="序号" width="60" align="center" />

        <el-table-column prop="user" label="数据库账号名 (User)" min-width="160">
          <template #default="scope">
            <div style="display: flex; align-items: center; gap: 8px;">
              <el-icon color="#409EFF"><UserFilled /></el-icon>
              <span style="font-weight: 600; font-family: monospace;">{{ scope.row.user }}</span>
            </div>
          </template>
        </el-table-column>

        <el-table-column prop="host" label="允许访问主机 (Host)" min-width="180">
          <template #default="scope">
            <el-tag size="small" type="info" effect="plain" style="font-family: monospace;">
              {{ scope.row.host }}
            </el-tag>
          </template>
        </el-table-column>

        <el-table-column prop="plugin" label="认证插件 (Plugin)" width="180" show-overflow-tooltip />

        <el-table-column prop="accountLocked" label="账号状态" width="120" align="center">
          <template #default="scope">
            <el-tag size="small" :type="scope.row.accountLocked === '正常' ? 'success' : 'danger'">
              {{ scope.row.accountLocked }}
            </el-tag>
          </template>
        </el-table-column>

        <el-table-column label="操作" width="220" fixed="right" align="center">
          <template #default="scope">
            <el-button size="small" :icon="Key" @click="handleOpenResetDialog(scope.row)">重置密码</el-button>
            <el-button size="small" type="danger" plain :icon="Delete" @click="handleDropAccount(scope.row)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>

      <!-- 分页控制栏 -->
      <div class="pagination-bar" style="display: flex; justify-content: flex-end; margin-top: 16px;">
        <el-pagination
          v-model:current-page="currentPage"
          v-model:page-size="pageSize"
          :page-sizes="[10, 20, 50]"
          :total="accounts.length"
          layout="total, sizes, prev, pager, next, jumper"
          background
        />
      </div>
    </div>

    <!-- 新建账号弹窗 -->
    <el-dialog title="新建数据库账号 (Create DB User)" v-model="createDialogVisible" width="540px" destroy-on-close>
      <el-form ref="createFormRef" :model="createForm" :rules="createRules" label-width="110px">
        <el-form-item label="账号名称" prop="username">
          <el-input v-model="createForm.username" placeholder="如: app_order_rw (仅限字母数字下划线)" />
        </el-form-item>

        <el-form-item label="主机白名单" prop="host">
          <el-input v-model="createForm.host" placeholder="如: % (所有主机) 或 192.168.1.%" />
        </el-form-item>

        <el-form-item label="连接密码" prop="password">
          <el-input type="password" v-model="createForm.password" show-password placeholder="请输入数据库账号连接密码" />
        </el-form-item>

        <el-form-item label="授权数据库">
          <el-input v-model="createForm.databaseName" placeholder="如: huiqitong_erp (留空则默认为全部库 *)" />
        </el-form-item>

        <el-form-item label="权限类型" prop="privilegeType">
          <el-select v-model="createForm.privilegeType" style="width: 100%;">
            <el-option label="读写权限 (SELECT, INSERT, UPDATE, DELETE)" value="DML_DQL" />
            <el-option label="只读查询权限 (仅 SELECT)" value="SELECT_ONLY" />
            <el-option label="完全管理权限 (ALL PRIVILEGES)" value="ALL" />
          </el-select>
        </el-form-item>
      </el-form>

      <template #footer>
        <el-button @click="createDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="createLoading" @click="handleCreateAccount">
          立即创建并授权
        </el-button>
      </template>
    </el-dialog>

    <!-- 重置密码弹窗 -->
    <el-dialog title="重置数据库账号密码" v-model="resetDialogVisible" width="460px" destroy-on-close>
      <div style="margin-bottom: 16px; font-size: 13px; color: #606266;">
        重置目标账号：<b>{{ currentResetUser?.user }}</b> @ <b>{{ currentResetUser?.host }}</b>
      </div>
      <el-form label-width="90px">
        <el-form-item label="新密码">
          <el-input type="password" v-model="newPassword" show-password placeholder="请输入新的数据库连接密码" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="resetDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="resetLoading" @click="handleResetPassword">
          确认重置
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import { Refresh, Plus, Delete, Key, UserFilled } from '@element-plus/icons-vue'
import request from '../utils/request'

interface AccountItem {
  user: string
  host: string
  plugin: string
  accountLocked: string
  privileges: string
}

const loading = ref(false)
const createLoading = ref(false)
const resetLoading = ref(false)
const createDialogVisible = ref(false)
const resetDialogVisible = ref(false)
const selectedInstanceId = ref<number | null>(null)
const instanceList = ref<any[]>([])
const accounts = ref<AccountItem[]>([])

const currentPage = ref(1)
const pageSize = ref(10)

const pagedAccounts = computed(() => {
  const start = (currentPage.value - 1) * pageSize.value
  return accounts.value.slice(start, start + pageSize.value)
})
const createFormRef = ref<FormInstance>()
const currentResetUser = ref<AccountItem | null>(null)
const newPassword = ref('')

const createForm = ref({
  username: '',
  host: '%',
  password: '',
  databaseName: '',
  privilegeType: 'DML_DQL'
})

const createRules = ref<FormRules>({
  username: [{ required: true, message: '请输入账号名称', trigger: 'blur' }],
  password: [{ required: true, message: '请输入密码', trigger: 'blur' }],
  privilegeType: [{ required: true, message: '请选择权限类型', trigger: 'change' }]
})

const fetchInstances = async () => {
  try {
    const res: any = await request.get('/v1/instance/list')
    instanceList.value = Array.isArray(res.data) ? res.data : []
    if (instanceList.value.length > 0 && !selectedInstanceId.value) {
      selectedInstanceId.value = instanceList.value[0].id
      fetchAccounts()
    }
  } catch (error) {
    ElMessage.error('获取实例列表失败')
  }
}

const fetchAccounts = async () => {
  if (!selectedInstanceId.value) return
  loading.value = true
  try {
    const res: any = await request.get(`/v1/instance/${selectedInstanceId.value}/accounts`)
    accounts.value = Array.isArray(res.data) ? res.data : []
  } catch (error) {
    ElMessage.error('获取账号列表失败')
  } finally {
    loading.value = false
  }
}

const handleOpenCreateDialog = () => {
  createForm.value = {
    username: '',
    host: '%',
    password: '',
    databaseName: '',
    privilegeType: 'DML_DQL'
  }
  createDialogVisible.value = true
}

const handleCreateAccount = async () => {
  if (!createFormRef.value) return
  await createFormRef.value.validate(async (valid) => {
    if (!valid) return
    createLoading.value = true
    try {
      await request.post(`/v1/instance/${selectedInstanceId.value}/accounts/create`, createForm.value)
      ElMessage.success(`数据库账号【${createForm.value.username}】创建成功！`)
      createDialogVisible.value = false
      fetchAccounts()
    } catch (err: any) {
      ElMessage.error(err.response?.data?.message || '创建账号失败')
    } finally {
      createLoading.value = false
    }
  })
}

const handleOpenResetDialog = (row: AccountItem) => {
  currentResetUser.value = row
  newPassword.value = ''
  resetDialogVisible.value = true
}

const handleResetPassword = async () => {
  if (!newPassword.value.trim()) {
    ElMessage.warning('请输入新密码')
    return
  }
  resetLoading.value = true
  try {
    await request.post(`/v1/instance/${selectedInstanceId.value}/accounts/reset-password`, {
      user: currentResetUser.value?.user,
      host: currentResetUser.value?.host,
      newPassword: newPassword.value.trim()
    })
    ElMessage.success('密码重置成功！')
    resetDialogVisible.value = false
  } catch (err: any) {
    ElMessage.error(err.response?.data?.message || '密码重置失败')
  } finally {
    resetLoading.value = false
  }
}

const handleDropAccount = async (row: AccountItem) => {
  try {
    await ElMessageBox.confirm(
      `确认删除数据库账号【${row.user}】@【${row.host}】吗？删除后对应客户端将无法再连接！`,
      '确认删除账号',
      {
        type: 'warning',
        confirmButtonText: '确认删除',
        cancelButtonText: '取消',
        confirmButtonClass: 'el-button--danger'
      }
    )

    await request.delete(`/v1/instance/${selectedInstanceId.value}/accounts?user=${encodeURIComponent(row.user)}&host=${encodeURIComponent(row.host)}`)
    ElMessage.success(`账号【${row.user}】已成功删除`)
    fetchAccounts()
  } catch (err) {
    if (err !== 'cancel') {
      ElMessage.error('删除账号失败')
    }
  }
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
  margin-bottom: 20px;
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

.table-wrapper {
  background: #ffffff;
  border-radius: 8px;
  overflow-x: auto;
}
</style>
