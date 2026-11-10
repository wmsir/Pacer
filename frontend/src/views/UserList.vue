<template>
  <div class="user-list-container page-container">
    <!-- 多页签 Workbench 架构 (支持多用户编辑、新增与主列表无缝切换) -->
    <el-tabs
      v-model="activeTab"
      type="card"
      class="user-workbench-tabs"
      @tab-remove="handleCloseTab"
    >
      <!-- 主页签: 👥 系统用户列表 -->
      <el-tab-pane name="list" :closable="false">
        <template #label>
          <div class="tab-label-item">
            <el-icon><UserFilled /></el-icon>
            <span>👥 系统用户列表 (User Management)</span>
          </div>
        </template>

        <div class="tab-inner-content">
          <!-- 顶部操作栏 -->
          <div class="header-action">
            <div class="title-area">
              <h2 class="page-title">系统用户与权限管理 (User Management)</h2>
              <div class="page-subtitle">管理研发工程师、开发组长、DBA及管理员账号，支持一人配置多个角色与独立定制功能页签权限</div>
            </div>
            <div class="action-area">
              <el-input
                v-model="searchKeyword"
                placeholder="搜索姓名/用户名/手机号/身份证/角色/资源组..."
                clearable
                style="width: 320px; margin-right: 12px;"
                :prefix-icon="Search"
                @clear="fetchUsers"
                @keyup.enter="fetchUsers"
              />
              <el-button :icon="Refresh" :loading="loading" @click="fetchUsers">刷新</el-button>
              <el-button type="primary" :icon="Plus" @click="handleOpenCreateTab">新增用户</el-button>
            </div>
          </div>

          <!-- 用户列表表格 -->
          <div class="table-wrapper">
            <el-table :data="users" border stripe style="width: 100%" v-loading="loading">
              <el-table-column prop="id" label="ID" width="65" align="center" />

              <el-table-column label="真实姓名 (同名消歧)" min-width="160">
                <template #default="scope">
                  <div style="display: flex; align-items: center; gap: 6px; flex-wrap: wrap;">
                    <el-link
                      type="primary"
                      :underline="false"
                      style="font-weight: 600;"
                      @click="handleOpenEditTab(scope.row)"
                    >
                      {{ scope.row.realName || scope.row.username }}
                    </el-link>
                    <el-tag
                      v-if="scope.row.displayName && scope.row.displayName !== scope.row.realName"
                      size="small"
                      type="danger"
                      effect="light"
                      style="font-size: 11px; font-weight: 500;"
                    >
                      {{ scope.row.displayName.replace(scope.row.realName, '').trim() }}
                    </el-tag>
                  </div>
                </template>
              </el-table-column>

              <el-table-column prop="username" label="登录用户名" width="130" show-overflow-tooltip>
                <template #default="scope">
                  <span style="font-family: monospace; font-weight: 500;">{{ scope.row.username }}</span>
                </template>
              </el-table-column>

              <!-- 支持多角色标签展示 -->
              <el-table-column label="系统角色 (支持多角色)" min-width="180">
                <template #default="scope">
                  <div style="display: flex; gap: 4px; flex-wrap: wrap;">
                    <template v-if="parseUserRoles(scope.row.role || scope.row.roles).length > 0">
                      <el-tag
                        v-for="r in parseUserRoles(scope.row.role || scope.row.roles)"
                        :key="r"
                        size="small"
                        :type="getRoleTagType(r)"
                        effect="dark"
                      >
                        {{ formatRole(r) }}
                      </el-tag>
                    </template>
                    <el-tag v-else size="small" type="info">普通用户</el-tag>
                  </div>
                </template>
              </el-table-column>

              <!-- 页签功能权限摘要 -->
              <el-table-column label="生效页签权限" min-width="160">
                <template #default="scope">
                  <div class="user-perm-cell">
                    <template v-if="isUserAdmin(scope.row)">
                      <el-tag type="danger" size="small" effect="plain">👑 全量 17 个页签</el-tag>
                    </template>
                    <template v-else>
                      <el-tag type="primary" size="small" effect="plain">
                        共 {{ getUserEffectivePermCount(scope.row) }} 个页签
                      </el-tag>
                      <el-tag v-if="scope.row.permissions && scope.row.permissions.length > 0" type="warning" size="small" effect="light" style="margin-left: 4px;">
                        自定义
                      </el-tag>
                    </template>
                  </div>
                </template>
              </el-table-column>

              <el-table-column label="部门 / 工号" min-width="150">
                <template #default="scope">
                  <div v-if="scope.row.department || scope.row.jobNo">
                    <div style="font-weight: 500; color: #1e293b;">{{ scope.row.department || '-' }}</div>
                    <div style="color: #64748b; font-size: 12px; font-family: monospace;">{{ scope.row.jobNo ? `#${scope.row.jobNo}` : '' }}</div>
                  </div>
                  <span v-else style="color: #c0c4cc; font-size: 12px;">未设置</span>
                </template>
              </el-table-column>

              <el-table-column label="工单数据范围" min-width="140">
                <template #default="scope">
                  <el-tag size="small" :type="getDataScopeTagType(scope.row.ticketDataScope)" effect="plain">
                    {{ getDataScopeLabel(scope.row.ticketDataScope, scope.row.role || scope.row.roles) }}
                  </el-tag>
                </template>
              </el-table-column>

              <el-table-column label="所属业务资源组" min-width="200">
                <template #default="scope">
                  <div style="display: flex; gap: 4px; flex-wrap: wrap;">
                    <template v-if="parseResourceGroups(scope.row.resourceGroup || scope.row.resourceGroups).length > 0">
                      <el-tag
                        v-for="rg in parseResourceGroups(scope.row.resourceGroup || scope.row.resourceGroups)"
                        :key="rg"
                        size="small"
                        type="warning"
                        effect="plain"
                      >
                        {{ rg }}
                      </el-tag>
                    </template>
                    <span v-else style="color: #c0c4cc; font-size: 12px;">未分配 (全局默认)</span>
                  </div>
                </template>
              </el-table-column>

              <el-table-column prop="phone" label="手机号码" width="130">
                <template #default="scope">
                  <span style="font-family: monospace;">{{ scope.row.phone || '-' }}</span>
                </template>
              </el-table-column>

              <el-table-column prop="idCard" label="身份证号" width="180">
                <template #default="scope">
                  <span style="font-family: monospace; font-size: 12px;">{{ scope.row.idCard || '-' }}</span>
                </template>
              </el-table-column>

              <el-table-column prop="status" label="状态" width="80" align="center">
                <template #default="scope">
                  <el-tag size="small" :type="scope.row.status === 1 ? 'success' : 'danger'">
                    {{ scope.row.status === 1 ? '正常' : '停用' }}
                  </el-tag>
                </template>
              </el-table-column>

              <el-table-column label="操作" width="230" fixed="right" align="center">
                <template #default="scope">
                  <el-button size="small" type="primary" plain :icon="Edit" @click="handleOpenEditTab(scope.row)">编辑</el-button>
                  <el-button size="small" :icon="Key" @click="handleOpenResetDialog(scope.row)">重置密码</el-button>
                  <el-button
                    size="small"
                    :type="scope.row.status === 1 ? 'warning' : 'success'"
                    plain
                    @click="handleToggleStatus(scope.row)"
                  >
                    {{ scope.row.status === 1 ? '停用' : '启用' }}
                  </el-button>
                  <el-button size="small" type="danger" plain :icon="Delete" @click="handleDelete(scope.row)">删除</el-button>
                </template>
              </el-table-column>
            </el-table>

            <!-- 分页控制栏 -->
            <div class="pagination-bar" style="display: flex; justify-content: flex-end; margin-top: 16px;">
              <el-pagination
                v-model:current-page="pagination.current"
                v-model:page-size="pagination.size"
                :page-sizes="[10, 20, 50, 100]"
                :total="pagination.total"
                layout="total, sizes, prev, pager, next, jumper"
                background
                @size-change="handleSizeChange"
                @current-change="handleCurrentChange"
              />
            </div>
          </div>
        </div>
      </el-tab-pane>

      <!-- 动态页签 (支持多用户新增与编辑) -->
      <el-tab-pane
        v-for="tab in dynamicTabs"
        :key="tab.name"
        :name="tab.name"
        :closable="true"
      >
        <template #label>
          <div class="tab-label-item">
            <el-icon :color="tab.isEdit ? '#E6A23C' : '#67C23A'">
              <component :is="tab.isEdit ? Edit : Plus" />
            </el-icon>
            <span>{{ tab.title }}</span>
          </div>
        </template>

        <div class="tab-inner-content edit-tab-content">
          <!-- 企业级一体化吸顶操作导航条 -->
          <div class="edit-nav-top-bar">
            <div class="nav-left">
              <el-button :icon="ArrowLeft" @click="activeTab = 'list'" plain size="default">
                返回用户列表
              </el-button>
              <div class="nav-title-group">
                <span class="nav-main-title">
                  {{ tab.isEdit ? `编辑用户：${tab.form.realName || tab.form.username}` : '新增系统用户与权限授权' }}
                </span>
                <span class="nav-sub-desc">
                  配置用户基础信息、绑定多角色、分配业务资源组及自定义功能页签权限矩阵
                </span>
              </div>
            </div>
            <div class="nav-actions">
              <el-button @click="handleCloseTab(tab.name)" :icon="CloseBold" plain type="info">
                关闭页签
              </el-button>
              <el-button
                type="primary"
                :icon="Check"
                :loading="tab.saveLoading"
                @click="handleSaveTab(tab)"
              >
                保存配置并生效
              </el-button>
            </div>
          </div>

          <!-- 用户配置表单主体 -->
          <el-form :model="tab.form" :rules="rules" label-width="110px" class="user-edit-form-flow">
            <!-- 1. 用户基础信息与认证凭据 -->
            <el-card shadow="hover" class="config-card mb-20">
              <template #header>
                <div class="card-header-title">
                  <span class="step-num-badge">1</span>
                  <el-icon color="#409EFF"><User /></el-icon>
                  <span>用户身份认证与账号基础信息 (Account & Authentication)</span>
                </div>
              </template>

              <el-row :gutter="24">
                <el-col :span="12">
                  <el-form-item label="登录用户名" required>
                    <el-input v-model="tab.form.username" :disabled="tab.isEdit" placeholder="用于登录系统，如: zhangsan" />
                  </el-form-item>
                </el-col>
                <el-col :span="12">
                  <el-form-item label="真实姓名" required>
                    <el-input v-model="tab.form.realName" placeholder="如: 张三 (支持自动同名消歧)" />
                  </el-form-item>
                </el-col>
              </el-row>

              <el-row :gutter="24">
                <el-col :span="12">
                  <el-form-item :label="tab.isEdit ? '修改密码' : '登录初始密码'">
                    <el-input
                      type="password"
                      v-model="tab.form.passwordCipher"
                      show-password
                      :placeholder="tab.isEdit ? '若不修改请留空' : '默认初始密码为 123456'"
                    />
                  </el-form-item>
                </el-col>
                <el-col :span="12">
                  <el-form-item label="账号状态">
                    <el-radio-group v-model="tab.form.status">
                      <el-radio :value="1">正常启用</el-radio>
                      <el-radio :value="0">禁用停用</el-radio>
                    </el-radio-group>
                  </el-form-item>
                </el-col>
              </el-row>

              <el-row :gutter="24">
                <el-col :span="12">
                  <el-form-item label="所属部门">
                    <el-input v-model="tab.form.department" placeholder="如: 产险研发中心 · 车险技术部" clearable />
                  </el-form-item>
                </el-col>
                <el-col :span="12">
                  <el-form-item label="员工工号">
                    <el-input v-model="tab.form.jobNo" placeholder="如: WM-9527" clearable />
                  </el-form-item>
                </el-col>
              </el-row>

              <el-row :gutter="24">
                <el-col :span="12">
                  <el-form-item label="手机号码">
                    <el-input v-model="tab.form.phone" placeholder="用于短信快捷登录/通知，如: 13800000001" clearable />
                  </el-form-item>
                </el-col>
                <el-col :span="12">
                  <el-form-item label="身份证号码">
                    <el-input v-model="tab.form.idCard" placeholder="实名认证身份证号" clearable />
                  </el-form-item>
                </el-col>
              </el-row>

              <el-row :gutter="24">
                <el-col :span="12">
                  <el-form-item label="电子邮箱">
                    <el-input v-model="tab.form.email" placeholder="如: dev@wmdb.com" clearable />
                  </el-form-item>
                </el-col>
                <el-col :span="12">
                  <el-form-item label="企业微信">
                    <el-input v-model="tab.form.workWechat" placeholder="企微 UserID 或企微账号" clearable />
                  </el-form-item>
                </el-col>
              </el-row>

              <el-row :gutter="24">
                <el-col :span="12">
                  <el-form-item label="微信账号">
                    <el-input v-model="tab.form.wechat" placeholder="个人微信号" clearable />
                  </el-form-item>
                </el-col>
                <el-col :span="12">
                  <el-form-item label="钉钉账号">
                    <el-input v-model="tab.form.dingtalk" placeholder="钉钉号或绑定手机号" clearable />
                  </el-form-item>
                </el-col>
              </el-row>
            </el-card>

            <!-- 2. 系统角色授权 (多角色智能聚合) -->
            <el-card shadow="hover" class="config-card mb-20">
              <template #header>
                <div class="card-header-title">
                  <span class="step-num-badge">2</span>
                  <el-icon color="#E6A23C"><Lock /></el-icon>
                  <span>系统角色授权与多角色聚合 (Role Assignment)</span>
                </div>
              </template>

              <el-form-item label="配置系统角色" required>
                <el-select
                  v-model="tab.form.roles"
                  multiple
                  filterable
                  collapse-tags
                  collapse-tags-tooltip
                  placeholder="支持为用户同时配置多个系统角色（权限自动聚合）"
                  style="width: 100%;"
                  @change="() => handleTabRolesChange(tab)"
                >
                  <el-option
                    v-for="r in availableRoleList"
                    :key="r.roleCode"
                    :label="`${r.roleName} (${r.roleCode})`"
                    :value="r.roleCode"
                  >
                    <div style="display: flex; justify-content: space-between; align-items: center;">
                      <span>{{ r.roleName }}</span>
                      <el-tag size="small" :type="getRoleTagType(r.roleCode)">{{ r.roleCode }}</el-tag>
                    </div>
                  </el-option>
                </el-select>
                <div class="form-hint-text">
                  💡 提示：用户可同时兼任多个角色（如：同时具备「开发组长」初审与「DBA」安全复核特权），系统将自动合并所选角色的页签与功能权限。
                </div>
              </el-form-item>
            </el-card>

            <!-- 3. 业务资源组归属与工单数据查看范围 -->
            <el-card shadow="hover" class="config-card mb-20">
              <template #header>
                <div class="card-header-title">
                  <span class="step-num-badge">3</span>
                  <el-icon color="#67C23A"><Suitcase /></el-icon>
                  <span>业务资源组归属与工单数据范围 (Resource Groups & Data Scope)</span>
                </div>
              </template>

              <el-row :gutter="24">
                <el-col :span="12">
                  <el-form-item label="归属资源组">
                    <el-select
                      v-model="tab.form.resourceGroups"
                      multiple
                      filterable
                      collapse-tags
                      collapse-tags-tooltip
                      placeholder="选择用户归属的业务线资源组（支持多选）"
                      style="width: 100%;"
                    >
                      <el-option
                        v-for="rg in resourceGroupOptions"
                        :key="rg"
                        :label="`🗂️ ${rg}`"
                        :value="rg"
                      />
                    </el-select>
                  </el-form-item>
                </el-col>
                <el-col :span="12">
                  <el-form-item label="工单数据范围">
                    <el-select
                      v-model="tab.form.ticketDataScope"
                      placeholder="选择工单可见范围（未指定时根据角色智能自适应）"
                      style="width: 100%;"
                      clearable
                    >
                      <el-option label="⚡ 角色智能默认 (根据系统角色自动判定)" value="" />
                      <el-option label="🌐 全平台所有工单 (ALL - 超管/DBA/审计员)" value="ALL" />
                      <el-option label="🏢 归属业务资源组工单 (RESOURCE_GROUP - 组长/主管)" value="RESOURCE_GROUP" />
                      <el-option label="👤 仅本人发起与待办 (SELF - 普通开发/测试)" value="SELF" />
                    </el-select>
                  </el-form-item>
                </el-col>
              </el-row>
            </el-card>

            <!-- 4. 细粒度页签功能权限独立定制面板 -->
            <el-card shadow="hover" class="config-card mb-20">
              <template #header>
                <div class="card-header-title" style="justify-content: space-between; width: 100%;">
                  <div style="display: flex; align-items: center; gap: 8px;">
                    <span class="step-num-badge">4</span>
                    <el-icon color="#8B5CF6"><Tickets /></el-icon>
                    <span>功能页签与权限独立定制面板 (Custom Tab Permissions)</span>
                  </div>
                  <div class="perm-action-buttons">
                    <el-button size="small" type="primary" link @click="() => resetTabToRolePermissions(tab)">
                      重置为角色默认
                    </el-button>
                    <el-button size="small" type="success" link @click="() => selectAllTabPermissions(tab)">
                      一键全选
                    </el-button>
                    <el-button size="small" type="danger" link @click="() => clearAllTabPermissions(tab)">
                      清空
                    </el-button>
                  </div>
                </div>
              </template>

              <div class="user-perm-box">
                <div class="user-perm-header">
                  <div class="perm-inherit-info">
                    <span>当前生效页签权限：</span>
                    <el-tag size="small" type="primary" effect="dark">
                      {{ tab.selectedTabPermissions.length === allTabKeys.length || (tab.form.roles && tab.form.roles.includes('ADMIN')) ? '全部 17 个页签' : `${tab.selectedTabPermissions.length} / 17 个页签` }}
                    </el-tag>
                  </div>
                </div>

                <!-- 模块化页签复选框列表 -->
                <div class="module-perm-list">
                  <div
                    v-for="moduleNode in SYSTEM_PERMISSION_TREE"
                    :key="moduleNode.id"
                    class="module-card-item"
                  >
                    <div class="module-card-title">
                      <el-checkbox
                        :model-value="isModuleFullyChecked(moduleNode, tab.selectedTabPermissions)"
                        :indeterminate="isModuleIndeterminate(moduleNode, tab.selectedTabPermissions)"
                        @change="(val: boolean) => toggleModuleCheck(moduleNode, val, tab)"
                      >
                        <span style="font-weight: 700; font-size: 13px; color: #1e293b;">{{ moduleNode.name }}</span>
                      </el-checkbox>
                    </div>
                    <div class="module-children-grid">
                      <template v-if="moduleNode.children && moduleNode.children.length > 0">
                        <div
                          v-for="child in moduleNode.children"
                          :key="child.id"
                          class="perm-pill"
                          :class="{ checked: tab.selectedTabPermissions.includes(child.id) }"
                          @click="toggleSingleTabPermission(child.id, tab)"
                        >
                          <el-checkbox
                            :model-value="tab.selectedTabPermissions.includes(child.id)"
                            @click.stop="toggleSingleTabPermission(child.id, tab)"
                          />
                          <span class="perm-pill-name">{{ child.name }}</span>
                        </div>
                      </template>
                      <template v-else>
                        <div
                          class="perm-pill"
                          :class="{ checked: tab.selectedTabPermissions.includes(moduleNode.id) }"
                          @click="toggleSingleTabPermission(moduleNode.id, tab)"
                        >
                          <el-checkbox
                            :model-value="tab.selectedTabPermissions.includes(moduleNode.id)"
                            @click.stop="toggleSingleTabPermission(moduleNode.id, tab)"
                          />
                          <span class="perm-pill-name">{{ moduleNode.name }}</span>
                        </div>
                      </template>
                    </div>
                  </div>
                </div>
              </div>
            </el-card>

            <!-- 底部保存按钮栏 -->
            <div class="tab-bottom-action-bar">
              <el-button size="large" @click="handleCloseTab(tab.name)">取消并关闭</el-button>
              <el-button
                type="primary"
                size="large"
                :icon="Check"
                :loading="tab.saveLoading"
                @click="handleSaveTab(tab)"
              >
                {{ tab.isEdit ? '保存修改并生效' : '立即创建用户' }}
              </el-button>
            </div>
          </el-form>
        </div>
      </el-tab-pane>
    </el-tabs>

    <!-- 重置密码小弹窗 -->
    <el-dialog title="重置用户密码" v-model="resetDialogVisible" width="440px" destroy-on-close>
      <div style="margin-bottom: 16px; font-size: 13px; color: #606266;">
        重置目标用户：<b>{{ currentResetUser?.realName }}</b> ({{ currentResetUser?.username }})
      </div>
      <el-form label-width="80px">
        <el-form-item label="新密码">
          <el-input type="password" v-model="newPassword" show-password placeholder="默认重置为 123456" />
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
import { ref, onMounted, watch } from 'vue'
import { useRoute } from 'vue-router'
import {
  Plus, Edit, Delete, Refresh, Search, Key, User, UserFilled,
  Lock, Suitcase, Tickets, ArrowLeft, Check, CloseBold
} from '@element-plus/icons-vue'
import { ElMessage, ElMessageBox, type FormRules } from 'element-plus'
import request from '../utils/request'
import {
  SYSTEM_PERMISSION_TREE,
  PRESET_PERMISSION_PACKAGES,
  getAllTabKeys,
  type PermissionNode
} from '../utils/permissions'

interface UserItem {
  id: number | null
  tenantId?: string
  username: string
  realName: string
  displayName?: string
  phone?: string
  idCard?: string
  email?: string
  wechat?: string
  workWechat?: string
  dingtalk?: string
  feishu?: string
  department?: string
  jobNo?: string
  ticketDataScope?: string
  role?: string
  roles?: string[]
  resourceGroup?: string
  resourceGroups?: string[]
  permissions?: string[]
  customPermissions?: string[]
  passwordCipher?: string
  status: number
}

interface DynamicTabItem {
  name: string
  title: string
  isEdit: boolean
  saveLoading: boolean
  form: any
  selectedTabPermissions: string[]
}

const route = useRoute()
const activeTab = ref('list')
const dynamicTabs = ref<DynamicTabItem[]>([])

const users = ref<UserItem[]>([])
const loading = ref(false)
const searchKeyword = ref('')
const resourceGroupOptions = ref<string[]>([])
const availableRoleList = ref<any[]>([])

const allTabKeys = getAllTabKeys()

const pagination = ref({
  current: 1,
  size: 10,
  total: 0
})

const handleSizeChange = (val: number) => {
  pagination.value.size = val
  fetchUsers()
}

const handleCurrentChange = (val: number) => {
  pagination.value.current = val
  fetchUsers()
}

const rules = ref<FormRules>({
  username: [{ required: true, message: '请输入登录用户名', trigger: 'blur' }],
  realName: [{ required: true, message: '请输入真实姓名', trigger: 'blur' }],
  roles: [{ required: true, message: '请至少选择一个系统角色', trigger: 'change' }]
})

// 重置密码
const resetDialogVisible = ref(false)
const resetLoading = ref(false)
const currentResetUser = ref<UserItem | null>(null)
const newPassword = ref('123456')

const getDataScopeTagType = (scope?: string) => {
  if (scope === 'ALL') return 'danger'
  if (scope === 'RESOURCE_GROUP') return 'warning'
  if (scope === 'SELF') return 'primary'
  return 'info'
}

const getDataScopeLabel = (scope?: string, roleRaw?: any) => {
  if (scope === 'ALL') return '全平台工单 (ALL)'
  if (scope === 'RESOURCE_GROUP') return '本资源组工单'
  if (scope === 'SELF') return '仅本人相关'

  const roles = parseUserRoles(roleRaw)
  if (roles.includes('ADMIN') || roles.includes('DBA') || roles.includes('AUDITOR')) {
    return '智能: 全平台 (默认)'
  }
  if (roles.includes('DEV_LEAD') || roles.includes('LEAD')) {
    return '智能: 本资源组 (默认)'
  }
  return '智能: 仅本人 (默认)'
}

const parseUserRoles = (rolesRaw?: string | string[]): string[] => {
  if (!rolesRaw) return []
  if (Array.isArray(rolesRaw)) return rolesRaw
  try {
    const arr = JSON.parse(rolesRaw)
    if (Array.isArray(arr)) return arr
  } catch (e) {}
  return rolesRaw.split(/[,，]/).map(s => s.trim()).filter(Boolean)
}

const parseResourceGroups = (rgRaw?: string | string[]): string[] => {
  if (!rgRaw) return []
  if (Array.isArray(rgRaw)) return rgRaw
  try {
    const arr = JSON.parse(rgRaw)
    if (Array.isArray(arr)) return arr
  } catch (e) {}
  return rgRaw.split(/[,，]/).map(s => s.trim()).filter(Boolean)
}

const formatRole = (role?: string) => {
  const map: Record<string, string> = {
    ADMIN: '超级管理员',
    DBA: '核心数据库管理员',
    DEV: '研发工程师',
    AUDITOR: '安全合规审计员',
    DEV_LEAD: '业务开发组长'
  }
  return role ? (map[role] || role) : '普通用户'
}

const getRoleTagType = (role?: string) => {
  const map: Record<string, string> = {
    ADMIN: 'danger',
    DBA: 'warning',
    DEV: 'info',
    AUDITOR: 'success',
    DEV_LEAD: 'primary'
  }
  return role ? (map[role] || 'info') : 'info'
}

const isUserAdmin = (user: UserItem) => {
  const roles = parseUserRoles(user.role || user.roles)
  return roles.includes('ADMIN') || (user.username && user.username.toLowerCase() === 'admin')
}

const getUserEffectivePermCount = (user: UserItem) => {
  if (isUserAdmin(user)) return allTabKeys.length
  if (user.permissions && user.permissions.length > 0) {
    if (user.permissions.includes('*')) return allTabKeys.length
    return user.permissions.length
  }
  const roles = parseUserRoles(user.role || user.roles)
  const set = new Set<string>()
  roles.forEach(r => {
    const pkg = PRESET_PERMISSION_PACKAGES[r]
    const list = pkg ? pkg.paths : []
    list.forEach((k: string) => set.add(k))
  })
  return set.size
}

// 权限复选状态计算
const isModuleFullyChecked = (moduleNode: PermissionNode, selectedList: string[]) => {
  const keys = moduleNode.children && moduleNode.children.length > 0
    ? moduleNode.children.map(c => c.id)
    : [moduleNode.id]
  return keys.every(k => selectedList.includes(k))
}

const isModuleIndeterminate = (moduleNode: PermissionNode, selectedList: string[]) => {
  const keys = moduleNode.children && moduleNode.children.length > 0
    ? moduleNode.children.map(c => c.id)
    : [moduleNode.id]
  const checkedCount = keys.filter(k => selectedList.includes(k)).length
  return checkedCount > 0 && checkedCount < keys.length
}

const toggleModuleCheck = (moduleNode: PermissionNode, checked: boolean, tab: DynamicTabItem) => {
  const keys = moduleNode.children && moduleNode.children.length > 0
    ? moduleNode.children.map(c => c.id)
    : [moduleNode.id]
  if (checked) {
    keys.forEach(k => {
      if (!tab.selectedTabPermissions.includes(k)) tab.selectedTabPermissions.push(k)
    })
  } else {
    tab.selectedTabPermissions = tab.selectedTabPermissions.filter(k => !keys.includes(k))
  }
}

const toggleSingleTabPermission = (key: string, tab: DynamicTabItem) => {
  const idx = tab.selectedTabPermissions.indexOf(key)
  if (idx > -1) {
    tab.selectedTabPermissions.splice(idx, 1)
  } else {
    tab.selectedTabPermissions.push(key)
  }
}

const resetTabToRolePermissions = (tab: DynamicTabItem) => {
  const roles = tab.form.roles || []
  if (roles.includes('ADMIN')) {
    tab.selectedTabPermissions = [...allTabKeys]
    return
  }
  const set = new Set<string>()
  roles.forEach((r: string) => {
    const pkg = PRESET_PERMISSION_PACKAGES[r]
    const list = pkg ? pkg.paths : []
    list.forEach((k: string) => set.add(k))
  })
  tab.selectedTabPermissions = Array.from(set)
}

const selectAllTabPermissions = (tab: DynamicTabItem) => {
  tab.selectedTabPermissions = [...allTabKeys]
}

const clearAllTabPermissions = (tab: DynamicTabItem) => {
  tab.selectedTabPermissions = []
}

const handleTabRolesChange = (tab: DynamicTabItem) => {
  resetTabToRolePermissions(tab)
}

// 打开新增用户页签
const handleOpenCreateTab = () => {
  const tabName = 'create_user'
  const existing = dynamicTabs.value.find(t => t.name === tabName)
  if (existing) {
    activeTab.value = tabName
    return
  }

  const newTab: DynamicTabItem = {
    name: tabName,
    title: '➕ 新增用户',
    isEdit: false,
    saveLoading: false,
    form: {
      id: null,
      username: '',
      realName: '',
      phone: '',
      idCard: '',
      email: '',
      wechat: '',
      workWechat: '',
      dingtalk: '',
      feishu: '',
      department: '',
      jobNo: '',
      ticketDataScope: '',
      roles: ['DEV'],
      resourceGroups: resourceGroupOptions.value.length > 0 ? [resourceGroupOptions.value[0]] : ['车险承保资源组'],
      passwordCipher: '',
      status: 1
    },
    selectedTabPermissions: []
  }

  resetTabToRolePermissions(newTab)
  dynamicTabs.value.push(newTab)
  activeTab.value = tabName
}

// 打开编辑用户页签
const handleOpenEditTab = (row: UserItem) => {
  const tabName = `edit_user_${row.id}`
  const existing = dynamicTabs.value.find(t => t.name === tabName)
  if (existing) {
    activeTab.value = tabName
    return
  }

  const roleList = parseUserRoles(row.role || row.roles)
  const rgList = parseResourceGroups(row.resourceGroup || row.resourceGroups)

  let selectedPerms: string[] = []
  if (row.permissions && row.permissions.length > 0) {
    selectedPerms = row.permissions.includes('*') ? [...allTabKeys] : [...row.permissions]
  } else {
    const set = new Set<string>()
    roleList.forEach(r => {
      const pkg = PRESET_PERMISSION_PACKAGES[r]
      const list = pkg ? pkg.paths : []
      list.forEach((k: string) => set.add(k))
    })
    selectedPerms = Array.from(set)
  }

  const newTab: DynamicTabItem = {
    name: tabName,
    title: `✏️ 编辑：${row.realName || row.username}`,
    isEdit: true,
    saveLoading: false,
    form: {
      id: row.id,
      username: row.username,
      realName: row.realName,
      phone: row.phone || '',
      idCard: row.idCard || '',
      email: row.email || '',
      wechat: row.wechat || '',
      workWechat: row.workWechat || '',
      dingtalk: row.dingtalk || '',
      feishu: row.feishu || '',
      department: row.department || '',
      jobNo: row.jobNo || '',
      ticketDataScope: row.ticketDataScope || '',
      roles: roleList.length > 0 ? roleList : ['DEV'],
      resourceGroups: rgList.length > 0 ? rgList : ['车险承保资源组'],
      passwordCipher: '',
      status: row.status !== undefined ? row.status : 1
    },
    selectedTabPermissions: selectedPerms
  }

  dynamicTabs.value.push(newTab)
  activeTab.value = tabName
}

// 关闭页签
const handleCloseTab = (targetName: string) => {
  if (targetName === 'list') return
  const idx = dynamicTabs.value.findIndex(t => t.name === targetName)
  if (idx > -1) {
    dynamicTabs.value.splice(idx, 1)
    if (activeTab.value === targetName) {
      activeTab.value = dynamicTabs.value[idx - 1]?.name || 'list'
    }
  }
}

// 保存用户页签
const handleSaveTab = async (tab: DynamicTabItem) => {
  if (!tab.form.username || !tab.form.username.trim()) {
    ElMessage.warning('请输入登录用户名')
    return
  }
  if (!tab.form.realName || !tab.form.realName.trim()) {
    ElMessage.warning('请输入真实姓名')
    return
  }
  if (!tab.form.roles || tab.form.roles.length === 0) {
    ElMessage.warning('请至少选择一个系统角色')
    return
  }

  tab.saveLoading = true
  try {
    const payload: any = {
      id: tab.form.id,
      username: tab.form.username,
      realName: tab.form.realName,
      phone: tab.form.phone,
      idCard: tab.form.idCard,
      email: tab.form.email,
      wechat: tab.form.wechat,
      workWechat: tab.form.workWechat,
      dingtalk: tab.form.dingtalk,
      feishu: tab.form.feishu,
      department: tab.form.department,
      jobNo: tab.form.jobNo,
      ticketDataScope: tab.form.ticketDataScope || null,
      role: tab.form.roles.join(','),
      resourceGroup: JSON.stringify(tab.form.resourceGroups),
      status: tab.form.status
    }

    if (tab.form.passwordCipher && tab.form.passwordCipher.trim().length > 0) {
      payload.passwordCipher = tab.form.passwordCipher.trim()
    }

    let finalPerms = [...tab.selectedTabPermissions]
    if (finalPerms.length === allTabKeys.length || (tab.form.roles && tab.form.roles.includes('ADMIN'))) {
      finalPerms = ['*']
    }
    payload.permissions = JSON.stringify(finalPerms)

    await request.post('/v1/user/save', payload)
    ElMessage.success('用户与多角色/页签权限配置保存成功！')
    handleCloseTab(tab.name)
    fetchUsers()
  } catch (err: any) {
    ElMessage.error(err.response?.data?.message || err.message || '保存用户失败')
  } finally {
    tab.saveLoading = false
  }
}

const fetchUsers = async () => {
  loading.value = true
  try {
    const res: any = await request.get('/v1/user/list', {
      params: {
        page: pagination.value.current,
        size: pagination.value.size,
        keyword: searchKeyword.value
      }
    })
    if (res.data) {
      users.value = Array.isArray(res.data.records) ? res.data.records : (Array.isArray(res.data) ? res.data : [])
      pagination.value.total = res.data.total || users.value.length
    }
  } catch (error) {
    console.error('Fetch users error', error)
  } finally {
    loading.value = false
  }
}

const fetchRoles = async () => {
  try {
    const res: any = await request.get('/v1/role/list')
    availableRoleList.value = Array.isArray(res.data) ? res.data : []
  } catch (e) {
    availableRoleList.value = [
      { roleCode: 'ADMIN', roleName: '超级管理员' },
      { roleCode: 'DBA', roleName: '核心数据库管理员' },
      { roleCode: 'DEV', roleName: '研发工程师' },
      { roleCode: 'AUDITOR', roleName: '安全合规审计员' },
      { roleCode: 'DEV_LEAD', roleName: '业务开发组长' }
    ]
  }
}

const fetchResourceGroups = async () => {
  try {
    const res: any = await request.get('/v1/resource-group/list')
    const list = Array.isArray(res.data) ? res.data : []
    resourceGroupOptions.value = list.map((rg: any) => rg.groupName).filter(Boolean)
  } catch (e) {
    resourceGroupOptions.value = ['车险承保资源组', '销管系统资源组', '理赔精算资源组']
  }
}

const handleToggleStatus = async (row: UserItem) => {
  try {
    await request.post(`/v1/user/${row.id}/toggle-status`)
    ElMessage.success(`用户【${row.realName || row.username}】状态已更新`)
    fetchUsers()
  } catch (error) {
    console.error('Toggle status error', error)
  }
}

const handleOpenResetDialog = (row: UserItem) => {
  currentResetUser.value = row
  newPassword.value = '123456'
  resetDialogVisible.value = true
}

const handleResetPassword = async () => {
  if (!currentResetUser.value?.id) return
  resetLoading.value = true
  try {
    await request.post(`/v1/user/${currentResetUser.value.id}/reset-password`, {
      newPassword: newPassword.value
    })
    ElMessage.success(`用户【${currentResetUser.value.realName}】密码已成功重置为: ${newPassword.value}`)
    resetDialogVisible.value = false
  } catch (error) {
    console.error('Reset password error', error)
  } finally {
    resetLoading.value = false
  }
}

const handleDelete = async (row: UserItem) => {
  try {
    await ElMessageBox.confirm(`确认删除用户【${row.realName || row.username}】吗？删除后不可恢复！`, '警告', {
      type: 'warning',
      confirmButtonText: '确认删除',
      cancelButtonText: '取消'
    })
    await request.delete(`/v1/user/${row.id}`)
    ElMessage.success('删除成功')
    fetchUsers()
  } catch (error) {
    if (error !== 'cancel') {
      console.error('Delete user error', error)
    }
  }
}

onMounted(() => {
  if (route.query.role) {
    searchKeyword.value = String(route.query.role)
  } else if (route.query.keyword) {
    searchKeyword.value = String(route.query.keyword)
  }
  fetchRoles()
  fetchResourceGroups()
  fetchUsers()
})

watch(() => route.query, (q) => {
  if (q.role) {
    searchKeyword.value = String(q.role)
    pagination.value.current = 1
    fetchUsers()
  } else if (q.keyword) {
    searchKeyword.value = String(q.keyword)
    pagination.value.current = 1
    fetchUsers()
  }
})
</script>

<style scoped>
.user-list-container {
  width: 100%;
}

.user-workbench-tabs {
  width: 100%;
}

.user-workbench-tabs :deep(.el-tabs__content) {
  overflow: visible !important;
}

/* ==================== 一体化置顶工作台头 ==================== */
.user-workbench-tabs > :deep(.el-tabs__header) {
  position: sticky;
  top: 0px;
  z-index: 100;
  margin-bottom: 0px;
  background: #ffffff;
  border-radius: 8px 8px 0 0;
  padding: 8px 14px 0 14px;
  border: 1px solid #e2e8f0;
  border-bottom: none;
}

.user-workbench-tabs :deep(.el-tabs__item) {
  font-weight: 600;
  font-size: 13px;
  transition: all 0.2s ease;
  background: #f8fafc;
  margin-right: 4px;
  border-radius: 6px 6px 0 0;
}

.user-workbench-tabs :deep(.el-tabs__item.is-active) {
  background: #ffffff;
  color: #409EFF;
  border-bottom-color: #ffffff;
}

.tab-label-item {
  display: flex;
  align-items: center;
  gap: 6px;
}

.tab-inner-content {
  background: #ffffff;
  padding: 20px;
  border: 1px solid #e2e8f0;
  border-radius: 0 0 8px 8px;
  min-height: calc(100vh - 160px);
}

.edit-tab-content {
  padding-top: 0px;
}

/* ==================== 吸顶置顶操作导航条 ==================== */
.edit-nav-top-bar {
  position: sticky;
  top: 48px;
  z-index: 95;
  background: #ffffff;
  border-bottom: 1px solid #e2e8f0;
  padding: 12px 0;
  margin-bottom: 20px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: 12px;
}

.nav-left {
  display: flex;
  align-items: center;
  gap: 14px;
}

.nav-title-group {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.nav-main-title {
  font-size: 16px;
  font-weight: 700;
  color: #1e293b;
}

.nav-sub-desc {
  font-size: 12px;
  color: #64748b;
}

.nav-actions {
  display: flex;
  align-items: center;
  gap: 10px;
}

.config-card {
  border-radius: 8px;
  border: 1px solid #e2e8f0;
}

.card-header-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  font-weight: 700;
  color: #1e293b;
}

.step-num-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border-radius: 50%;
  background: #3b82f6;
  color: #ffffff;
  font-size: 12px;
  font-weight: 700;
}

.mb-20 {
  margin-bottom: 20px;
}

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
  width: 100%;
  overflow-x: auto;
}

.form-hint-text {
  font-size: 12px;
  color: #94a3b8;
  margin-top: 4px;
  line-height: 1.4;
}

.user-perm-box {
  width: 100%;
  border: 1px solid #e2e8f0;
  border-radius: 6px;
  background: #f8fafc;
  padding: 12px 14px;
  box-sizing: border-box;
}

.user-perm-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding-bottom: 8px;
  border-bottom: 1px solid #e2e8f0;
  margin-bottom: 10px;
}

.perm-inherit-info {
  font-size: 12.5px;
  color: #475569;
  display: flex;
  align-items: center;
  gap: 6px;
}

.perm-action-buttons {
  display: flex;
  gap: 8px;
}

.module-perm-list {
  max-height: 380px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding-right: 4px;
}

.module-card-item {
  background: #ffffff;
  border: 1px solid #e2e8f0;
  border-radius: 6px;
  padding: 10px 12px;
}

.module-card-title {
  margin-bottom: 8px;
  padding-bottom: 6px;
  border-bottom: 1px dashed #f1f5f9;
}

.module-children-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 8px;
}

.perm-pill {
  border: 1px solid #e2e8f0;
  border-radius: 4px;
  padding: 6px 10px;
  background: #f8fafc;
  display: flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  transition: all 0.2s ease;
}

.perm-pill:hover {
  background: #eff6ff;
  border-color: #93c5fd;
}

.perm-pill.checked {
  background: #eff6ff;
  border-color: #3b82f6;
}

.perm-pill-name {
  font-size: 12px;
  color: #334155;
}

.tab-bottom-action-bar {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  margin-top: 24px;
  padding-top: 16px;
  border-top: 1px solid #e2e8f0;
}
</style>
