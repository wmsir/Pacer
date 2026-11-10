<template>
  <div class="resource-group-page page-container">
    <!-- 多页签切换栏系统 (Tab Workbench) -->
    <el-tabs
      v-model="activeTab"
      type="card"
      class="rg-workbench-tabs"
      @tab-remove="handleTabRemove"
      @tab-change="handleTabChange"
    >
      <!-- 主页签 1: 业务资源组列表 (固定不可关闭) -->
      <el-tab-pane name="list" :closable="false">
        <template #label>
          <div class="tab-label-item">
            <el-icon color="#409EFF"><Suitcase /></el-icon>
            <span>业务资源组管理 (Resource Groups)</span>
          </div>
        </template>

        <div class="tab-inner-content">
          <!-- 顶部操作栏 -->
          <div class="header-action">
            <div class="title-area">
              <h2 class="page-title">业务资源组管理 (Resource Groups)</h2>
              <div class="page-subtitle">定义业务线与资产隔离边界，配置所属部门、开发组长初审责任人、DBA 负责人及具体数据库绑定的审批流程</div>
            </div>
            <div class="action-area">
              <el-input
                v-model="searchKeyword"
                placeholder="搜索资源组名称/部门/开发组长..."
                clearable
                style="width: 280px; margin-right: 12px;"
                :prefix-icon="Search"
                @clear="fetchResourceGroups"
                @keyup.enter="fetchResourceGroups"
              />
              <el-button :icon="Refresh" :loading="loading" @click="fetchResourceGroups">刷新</el-button>
              <el-button type="primary" :icon="Plus" @click="handleOpenCreateTab">新建资源组</el-button>
            </div>
          </div>

          <!-- 资源组表格 -->
          <div class="table-wrapper">
            <el-table :data="resourceGroups" border stripe style="width: 100%" v-loading="loading">
              <el-table-column prop="id" label="ID" width="70" align="center" />

              <el-table-column prop="groupName" label="资源组名称" min-width="180">
                <template #default="scope">
                  <div style="display: flex; align-items: center; gap: 8px;">
                    <el-icon color="#E6A23C"><FolderOpened /></el-icon>
                    <el-link type="primary" :underline="false" style="font-weight: 600;" @click="handleOpenEditTab(scope.row)">
                      {{ scope.row.groupName }}
                    </el-link>
                  </div>
                </template>
              </el-table-column>

              <el-table-column prop="deptName" label="所属部门 / 业务线" width="140">
                <template #default="scope">
                  <el-tag size="small" type="info" effect="plain">{{ scope.row.deptName || '未指定' }}</el-tag>
                </template>
              </el-table-column>

              <el-table-column prop="devLead" label="开发组长 (初审责任人)" width="160">
                <template #default="scope">
                  <el-tag size="small" type="warning">
                    <el-icon style="margin-right: 2px;"><User /></el-icon>
                    {{ scope.row.devLead || '暂未指定' }}
                  </el-tag>
                </template>
              </el-table-column>

              <el-table-column prop="dbaLead" label="DBA 负责人" width="130">
                <template #default="scope">
                  <span style="font-weight: 500;">{{ scope.row.dbaLead || '核心DBA' }}</span>
                </template>
              </el-table-column>

              <el-table-column prop="description" label="业务职责与说明" min-width="200" show-overflow-tooltip />

              <el-table-column prop="status" label="状态" width="90" align="center">
                <template #default="scope">
                  <el-tag :type="scope.row.status === 1 ? 'success' : 'danger'">
                    {{ scope.row.status === 1 ? '正常' : '已停用' }}
                  </el-tag>
                </template>
              </el-table-column>

              <el-table-column label="操作" width="180" fixed="right" align="center">
                <template #default="scope">
                  <el-button size="small" type="primary" plain :icon="Edit" @click="handleOpenEditTab(scope.row)">
                    编辑绑定
                  </el-button>
                  <el-button size="small" type="danger" plain :icon="Delete" @click="handleDelete(scope.row)">
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
        </div>
      </el-tab-pane>

      <!-- 动态编辑/新建页签 (支持多任务切换与随时关闭) -->
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
          <!-- 顶部快捷导航与保存操作栏 (无遮挡设计) -->
          <div class="edit-nav-top-bar">
            <div class="nav-left">
              <el-button :icon="ArrowLeft" @click="activeTab = 'list'" plain size="default">
                返回资源组列表
              </el-button>
              <div class="nav-title-group">
                <span class="nav-main-title">
                  {{ tab.isEdit ? `编辑业务资源组：${tab.form.groupName || '未命名'}` : '新建业务资源组' }}
                </span>
                <span class="nav-sub-desc">
                  配置资源组基础属性、预检策略、工单步骤开关与具体业务库绑定专属审批流
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
                {{ tab.isEdit ? '保存配置并生效' : '立即创建资源组' }}
              </el-button>
            </div>
          </div>

          <!-- 编辑表单流式内容 -->
          <el-form
            :ref="(el: any) => tab.formRef = el"
            :model="tab.form"
            :rules="rules"
            label-position="top"
            class="edit-tab-form"
          >
            <!-- 1. 基础信息配置卡片 -->
            <el-card shadow="hover" class="config-card mb-20">
              <template #header>
                <div class="card-header-title">
                  <span class="step-num-badge">1</span>
                  <el-icon color="#409EFF"><FolderOpened /></el-icon>
                  <span>资源组基础属性与负责人 (Basic Attributes)</span>
                </div>
              </template>

              <el-row :gutter="20">
                <el-col :xs="24" :sm="12" :md="8">
                  <el-form-item label="业务资源组名称" prop="groupName">
                    <el-input
                      v-model="tab.form.groupName"
                      placeholder="如：车险承保资源组 / 水险财产险资源组"
                      @change="() => handleGroupNameChanged(tab)"
                    />
                  </el-form-item>
                </el-col>

                <el-col :xs="24" :sm="12" :md="8">
                  <el-form-item label="所属部门 / 业务线" prop="deptName">
                    <el-input v-model="tab.form.deptName" placeholder="如：车险事业部 / 理赔运营中心" />
                  </el-form-item>
                </el-col>

                <el-col :xs="24" :sm="12" :md="4">
                  <el-form-item label="开发组长 (初审责任人)" prop="devLead">
                    <el-input v-model="tab.form.devLead" placeholder="初审开发组长账号" />
                  </el-form-item>
                </el-col>

                <el-col :xs="24" :sm="12" :md="4">
                  <el-form-item label="DBA 负责人">
                    <el-input v-model="tab.form.dbaLead" placeholder="默认：核心DBA" />
                  </el-form-item>
                </el-col>
              </el-row>

              <el-row :gutter="20">
                <el-col :xs="24" :sm="18">
                  <el-form-item label="业务用途与职责说明">
                    <el-input
                      v-model="tab.form.description"
                      type="textarea"
                      :rows="2"
                      placeholder="清晰描述该资源组负责的业务系统与数据资产边界..."
                    />
                  </el-form-item>
                </el-col>
                <el-col :xs="24" :sm="6">
                  <el-form-item label="资源组启用状态">
                    <el-radio-group v-model="tab.form.status" style="margin-top: 6px;">
                      <el-radio :value="1">正常启用</el-radio>
                      <el-radio :value="0">停用</el-radio>
                    </el-radio-group>
                  </el-form-item>
                </el-col>
              </el-row>
            </el-card>

            <!-- 2. 工单提单步骤与预检安全策略配置卡片 (截图优化：独立卡片整齐垂直排版) -->
            <el-card shadow="hover" class="config-card mb-20">
              <template #header>
                <div class="card-header-title">
                  <span class="step-num-badge">2</span>
                  <el-icon color="#E6A23C"><Lock /></el-icon>
                  <span>工单提单步骤与预检安全策略配置 (Safety & Step Policies)</span>
                </div>
              </template>

              <div class="safety-policy-panel">
                <el-row :gutter="20">
                  <!-- 左列：预检执行策略 (整齐卡片单选) -->
                  <el-col :xs="24" :md="12">
                    <div class="policy-card">
                      <div class="policy-title">
                        <span style="color: #f59e0b; font-weight: 700;">⚡ 预检执行策略</span>
                        <el-tooltip content="配置申请人在提单时是否必须点击并通过事务级预执行校验" placement="top">
                          <el-icon><InfoFilled /></el-icon>
                        </el-tooltip>
                      </div>
                      
                      <div class="policy-radio-cards-group">
                        <!-- 选项 1: 强制预执行校验 -->
                        <div
                          class="policy-radio-item-card"
                          :class="{ 'is-active': tab.rgPolicyConfig.enforceDryRun === true }"
                          @click="tab.rgPolicyConfig.enforceDryRun = true"
                        >
                          <el-radio :value="true" v-model="tab.rgPolicyConfig.enforceDryRun" class="custom-card-radio">
                            <div class="radio-content-block">
                              <div class="radio-title-text danger-text">
                                🛡️ 强制预执行校验 (必须通过)
                              </div>
                              <div class="radio-desc-text">
                                必须在目标库点击预执行且比对影响行数一致通过后，才允许提交工单
                              </div>
                            </div>
                          </el-radio>
                        </div>

                        <!-- 选项 2: 推荐预执行校验 -->
                        <div
                          class="policy-radio-item-card"
                          :class="{ 'is-active': tab.rgPolicyConfig.enforceDryRun === false }"
                          @click="tab.rgPolicyConfig.enforceDryRun = false"
                        >
                          <el-radio :value="false" v-model="tab.rgPolicyConfig.enforceDryRun" class="custom-card-radio">
                            <div class="radio-content-block">
                              <div class="radio-title-text success-text">
                                💡 推荐预执行校验 (允许跳过)
                              </div>
                              <div class="radio-desc-text">
                                提供预执行校验工具辅助审查，未执行或校验异常时允许继续提单
                              </div>
                            </div>
                          </el-radio>
                        </div>
                      </div>
                    </div>
                  </el-col>

                  <!-- 右列：工单提单步骤启用开关 -->
                  <el-col :xs="24" :md="12">
                    <div class="policy-card">
                      <div class="policy-title">
                        <span style="color: #3b82f6; font-weight: 700;">📑 工单提单步骤启用开关</span>
                        <el-tooltip content="控制新建 SQL 变更工单中心是否显示回滚方案与预执行校验步骤" placement="top">
                          <el-icon><InfoFilled /></el-icon>
                        </el-tooltip>
                      </div>
                      <div class="policy-switches-group">
                        <div class="switch-item-box">
                          <div class="switch-left">
                            <span class="step-badge-tag">步骤 3</span>
                            <div class="switch-title-wrap">
                              <div class="switch-title">数据回滚方案与补偿脚本 (Rollback)</div>
                              <div class="switch-desc">控制提单界面第 3 步是否展示回滚方案与备份附件上传</div>
                            </div>
                          </div>
                          <el-switch v-model="tab.rgPolicyConfig.enableStep3Rollback" active-text="启用" inactive-text="禁用" />
                        </div>

                        <div class="switch-item-box" style="margin-top: 10px;">
                          <div class="switch-left">
                            <span class="step-badge-tag">步骤 4</span>
                            <div class="switch-title-wrap">
                              <div class="switch-title">事务级预执行校验 (Dry-Run) 区域</div>
                              <div class="switch-desc">控制提单界面第 4 步是否展示模拟执行比对面板</div>
                            </div>
                          </div>
                          <el-switch v-model="tab.rgPolicyConfig.enableStep4DryRun" active-text="启用" inactive-text="禁用" />
                        </div>
                      </div>
                    </div>
                  </el-col>
                </el-row>
              </div>
            </el-card>

            <!-- 3. 授权数据库实例、具体业务库与审批流对应关系矩阵 (支持打开审批流设计器新页签) -->
            <el-card shadow="hover" class="config-card mb-20">
              <template #header>
                <div class="card-header-title" style="justify-content: space-between; width: 100%;">
                  <div style="display: flex; align-items: center; gap: 8px;">
                    <span class="step-num-badge">3</span>
                    <el-icon color="#67C23A"><Coin /></el-icon>
                    <span>授权数据库实例、具体业务库与审批流对应关系矩阵 (共 {{ tab.rgDbMatrixList.length }} 个库)</span>
                  </div>
                  <div style="display: flex; align-items: center; gap: 10px;">
                    <el-button
                      size="small"
                      type="warning"
                      plain
                      :icon="Share"
                      @click="handleOpenWorkflowDesignerTab"
                    >
                      📐 新建/设计审批流程 (打开独立页签)
                    </el-button>
                    <el-button
                      size="small"
                      type="primary"
                      link
                      :icon="Refresh"
                      @click="loadTabDbMatrix(tab)"
                    >
                      刷新实例与流程
                    </el-button>
                  </div>
                </div>
              </template>

              <div class="db-matrix-section" v-loading="tab.dbMatrixLoading">
                <!-- 快捷引导直达条 -->
                <div class="matrix-jump-guide-bar">
                  <div class="guide-left">
                    <el-icon color="#059669"><InfoFilled /></el-icon>
                    <span>💡 如果没有符合当前业务需要的审批流程，可点击右侧按钮直接打开新页签新建/配置 BPMN 流程：</span>
                  </div>
                  <el-button
                    type="success"
                    size="small"
                    :icon="Share"
                    @click="handleOpenWorkflowDesignerTab"
                  >
                    前往【审批流设计中心】新建/编辑 (新页签)
                  </el-button>
                </div>

                <div v-if="tab.rgDbMatrixList.length > 0" class="matrix-table-wrap">
                  <el-table :data="tab.rgDbMatrixList" size="small" border stripe style="width: 100%;">
                    <el-table-column prop="instanceName" label="所属数据库实例" min-width="190">
                      <template #default="{ row }">
                        <div style="display: flex; align-items: center; gap: 6px;">
                          <el-tag size="small" type="info">{{ row.dbType || 'mysql' }}</el-tag>
                          <span style="font-weight: 600; color: #1e293b;">{{ row.instanceName }}</span>
                        </div>
                      </template>
                    </el-table-column>

                    <el-table-column prop="dbName" label="目标数据库" min-width="160">
                      <template #default="{ row }">
                        <el-tag size="small" type="success" effect="light" style="font-weight: 600;">
                          🗃️ {{ row.dbName }}
                        </el-tag>
                      </template>
                    </el-table-column>

                    <el-table-column label="绑定审批流程 (Workflow)" min-width="320">
                      <template #default="{ row }">
                        <div style="display: flex; align-items: center; gap: 6px;">
                          <el-select
                            v-model="row.customWorkflow"
                            placeholder="选择该数据库绑定的审批流"
                            style="flex: 1;"
                            size="small"
                          >
                            <el-option label="⚡ 跟随资源组通用/默认流程" value="DEFAULT" />
                            <el-option
                              v-for="tpl in availableWorkflowTemplates"
                              :key="tpl.id"
                              :label="`🎯 ${tpl.templateName} (${tpl.flowType})`"
                              :value="tpl.templateName"
                            />
                          </el-select>
                          <el-tooltip content="没有符合的流程？点击打开审批流设计器页签" placement="top">
                            <el-button
                              circle
                              size="small"
                              :icon="Plus"
                              type="warning"
                              plain
                              @click="handleOpenWorkflowDesignerTab"
                            />
                          </el-tooltip>
                        </div>
                      </template>
                    </el-table-column>

                    <!-- 跳转数据库实例参数配置操作列 -->
                    <el-table-column label="实例参数与工单字段" width="165" align="center">
                      <template #default="{ row }">
                        <el-button
                          size="small"
                          type="primary"
                          link
                          :icon="Setting"
                          @click="handleJumpToInstanceConfig(row)"
                        >
                          ⚙️ 参数配置
                        </el-button>
                      </template>
                    </el-table-column>
                  </el-table>
                </div>
                <div v-else class="empty-matrix-tip">
                  <el-alert
                    type="info"
                    :closable="false"
                    show-icon
                    :title="`暂无授权数据库实例归属本资源组【${tab.form.groupName || '未指定'}】`"
                    description="您可以在【实例管理】中将实例的所属资源组设置为本组，系统将自动识别并加载该实例下所有具体数据库。"
                  />
                </div>
              </div>
            </el-card>

            <!-- 底部保存操作按钮栏 -->
            <div class="tab-bottom-action-bar">
              <el-button size="large" @click="handleCloseTab(tab.name)">取消并关闭</el-button>
              <el-button
                type="primary"
                size="large"
                :icon="Check"
                :loading="tab.saveLoading"
                @click="handleSaveTab(tab)"
              >
                {{ tab.isEdit ? '保存配置并生效' : '立即创建资源组' }}
              </el-button>
            </div>
          </el-form>
        </div>
      </el-tab-pane>

      <!-- 动态独立页签: 审批流设计与模板中心 (Workflow Designer) -->
      <el-tab-pane
        v-if="isWorkflowDesignerTabOpen"
        name="workflow_designer_tab"
        :closable="true"
      >
        <template #label>
          <div class="tab-label-item">
            <el-icon color="#8B5CF6"><Share /></el-icon>
            <span>📐 审批流设计与模板中心 (Workflow Designer)</span>
          </div>
        </template>

        <div class="tab-inner-content embedded-designer-container">
          <WorkflowDesigner />
        </div>
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import {
  Refresh,
  Plus,
  Edit,
  Delete,
  Search,
  FolderOpened,
  User,
  Suitcase,
  Lock,
  Coin,
  ArrowLeft,
  Check,
  InfoFilled,
  Share,
  CloseBold,
  Setting
} from '@element-plus/icons-vue'
import request from '../utils/request'
import WorkflowDesigner from './WorkflowDesigner.vue'

const router = useRouter()

interface WorkflowTemplateOption {
  id: number
  templateName: string
  flowType: string
}

interface ResourceGroupItem {
  id: number | null
  groupName: string
  deptName?: string
  devLead?: string
  dbaLead?: string
  description?: string
  workflowTemplates?: string
  formConfig?: string
  status: number
}

interface RgDbMatrixItem {
  instanceId: number | string
  instanceName: string
  dbType: string
  dbName: string
  customWorkflow: string // 'DEFAULT' or templateName
}

interface FormFieldConfig {
  fieldKey: string
  fieldName: string
  fieldType: string
  placeholder: string
  enabled: boolean
  required: boolean
  defaultRange?: string[]
}

interface DynamicTabItem {
  name: string
  title: string
  isEdit: boolean
  saveLoading: boolean
  dbMatrixLoading: boolean
  form: ResourceGroupItem
  formRef?: FormInstance
  rgPolicyConfig: {
    enforceDryRun: boolean
    enableStep3Rollback: boolean
    enableStep4DryRun: boolean
  }
  rgDbMatrixList: RgDbMatrixItem[]
  formFieldList: FormFieldConfig[]
}

const loading = ref(false)
const searchKeyword = ref('')
const resourceGroups = ref<ResourceGroupItem[]>([])
const availableWorkflowTemplates = ref<WorkflowTemplateOption[]>([])
const allInstances = ref<any[]>([])

// 多页签系统状态
const activeTab = ref('list')
const dynamicTabs = ref<DynamicTabItem[]>([])
const isWorkflowDesignerTabOpen = ref(false)

const rules = ref<FormRules>({
  groupName: [{ required: true, message: '请输入资源组名称', trigger: 'blur' }]
})

const defaultFields: FormFieldConfig[] = [
  {
    fieldKey: 'releaseVersion',
    fieldName: '上线发布版本号',
    fieldType: 'TEXT',
    placeholder: '请输入本次发布的版本号（例如：V20260822.01）',
    enabled: true,
    required: true
  },
  {
    fieldKey: 'releaseDate',
    fieldName: '上线执行日期 (年月日)',
    fieldType: 'DATE',
    placeholder: '请选择计划上线年月日',
    enabled: true,
    required: true
  },
  {
    fieldKey: 'executionTimeRange',
    fieldName: '允许执行时间窗口',
    fieldType: 'TIME_RANGE',
    placeholder: '选择允许变更执行的时间段',
    enabled: true,
    required: true,
    defaultRange: ['00:00:00', '06:00:00']
  },
  {
    fieldKey: 'demandNo',
    fieldName: '关联需求 / 禅道 / Jira 编号',
    fieldType: 'TEXT',
    placeholder: '例如：PROJ-9821 或 REQ-2026-08',
    enabled: false,
    required: false
  }
]

const pagination = ref({
  current: 1,
  size: 10,
  total: 0
})

const handleSizeChange = (val: number) => {
  pagination.value.size = val
  pagination.value.current = 1
  fetchResourceGroups()
}

const handleCurrentChange = (val: number) => {
  pagination.value.current = val
  fetchResourceGroups()
}

// 获取审批流列表
const fetchWorkflowTemplates = async () => {
  try {
    const res: any = await request.get('/v1/workflow/template/list')
    availableWorkflowTemplates.value = Array.isArray(res.data) ? res.data : []
  } catch (error) {
    // ignore
  }
}

// 获取所有实例
const fetchAllInstances = async () => {
  try {
    const res: any = await request.get('/v1/instance/list')
    allInstances.value = Array.isArray(res.data) ? res.data : []
  } catch (e) {}
}

// 获取资源组列表
const fetchResourceGroups = async () => {
  loading.value = true
  try {
    const res: any = await request.get('/v1/resource-group/list')
    const list: ResourceGroupItem[] = Array.isArray(res.data) ? res.data : []
    let filtered = list
    if (searchKeyword.value.trim()) {
      const kw = searchKeyword.value.trim().toLowerCase()
      filtered = list.filter(item =>
        (item.groupName && item.groupName.toLowerCase().includes(kw)) ||
        (item.deptName && item.deptName.toLowerCase().includes(kw)) ||
        (item.devLead && item.devLead.toLowerCase().includes(kw))
      )
    }
    pagination.value.total = filtered.length
    const start = (pagination.value.current - 1) * pagination.value.size
    resourceGroups.value = filtered.slice(start, start + pagination.value.size)
  } catch (error) {
    ElMessage.error('加载业务资源组列表失败')
  } finally {
    loading.value = false
  }
}

// 打开审批流设计器独立页签
const handleOpenWorkflowDesignerTab = () => {
  isWorkflowDesignerTabOpen.value = true
  activeTab.value = 'workflow_designer_tab'
}

// Tab 切换监听（当从设计器切回编辑页时自动刷新流程模板列表）
const handleTabChange = (tabName: string) => {
  if (tabName !== 'workflow_designer_tab') {
    fetchWorkflowTemplates()
  }
}

// 加载指定 Tab 的实例与具体数据库矩阵
const loadTabDbMatrix = async (tab: DynamicTabItem) => {
  const targetTab = dynamicTabs.value.find(t => t.name === tab.name) || tab
  const currentRgName = targetTab.form.groupName
  targetTab.dbMatrixLoading = true
  try {
    // 强制确保已获取所有数据库实例
    let instList = allInstances.value
    if (!instList || instList.length === 0) {
      const res: any = await request.get('/v1/instance/list')
      instList = Array.isArray(res.data) ? res.data : []
      allInstances.value = instList
    }
    await fetchWorkflowTemplates()

    // 匹配归属于当前资源组的实例
    let matchedInsts = instList.filter(inst => {
      if (!currentRgName) return true
      if (!inst.resourceGroups) return true
      try {
        const rgs = typeof inst.resourceGroups === 'string' ? JSON.parse(inst.resourceGroups) : inst.resourceGroups
        if (Array.isArray(rgs)) {
          return rgs.includes(currentRgName) || rgs.includes('全部业务资源组通用') || rgs.length === 0
        }
      } catch (e) {
        return inst.resourceGroups.includes(currentRgName) || inst.resourceGroups.includes('全部业务资源组通用')
      }
      return true
    })

    // 如果未精确匹配到特定资源组，展示系统可用实例，防止出现 0 个库
    if (matchedInsts.length === 0 && instList.length > 0) {
      matchedInsts = instList
    }

    // 并发拉取各个实例下的数据库
    const matrix: RgDbMatrixItem[] = []
    await Promise.all(
      matchedInsts.map(async inst => {
        try {
          const dbRes: any = await request.get(`/v1/instance/${inst.id}/databases`)
          const dbList: string[] = Array.isArray(dbRes.data) ? dbRes.data : []
          if (dbList.length === 0) {
            matrix.push({
              instanceId: inst.id,
              instanceName: inst.name,
              dbType: inst.dbType || 'mysql',
              dbName: 'default',
              customWorkflow: 'DEFAULT'
            })
          } else {
            for (const db of dbList) {
              matrix.push({
                instanceId: inst.id,
                instanceName: inst.name,
                dbType: inst.dbType || 'mysql',
                dbName: db,
                customWorkflow: 'DEFAULT'
              })
            }
          }
        } catch (e) {
          matrix.push({
            instanceId: inst.id,
            instanceName: inst.name,
            dbType: inst.dbType || 'mysql',
            dbName: 'default',
            customWorkflow: 'DEFAULT'
          })
        }
      })
    )

    // 按照实例 ID 与数据库名称排序
    matrix.sort((a, b) => {
      if (a.instanceName === b.instanceName) {
        return a.dbName.localeCompare(b.dbName)
      }
      return a.instanceName.localeCompare(b.instanceName)
    })

    // 回填已保存的定制映射
    let dbWorkflowMappings: Record<string, string> = {}
    if (targetTab.form.formConfig) {
      try {
        const parsed = JSON.parse(targetTab.form.formConfig)
        if (parsed && typeof parsed === 'object' && !Array.isArray(parsed) && parsed.dbWorkflowMappings) {
          dbWorkflowMappings = parsed.dbWorkflowMappings
        }
      } catch (e) {}
    }

    matrix.forEach(item => {
      const key = `${item.instanceId}:${item.dbName}`
      if (dbWorkflowMappings[key]) {
        item.customWorkflow = dbWorkflowMappings[key]
      }
    })

    // 响应式深赋值
    targetTab.rgDbMatrixList = [...matrix]
  } finally {
    targetTab.dbMatrixLoading = false
  }
}

const handleGroupNameChanged = (tab: DynamicTabItem) => {
  loadTabDbMatrix(tab)
}

const handleJumpToInstanceConfig = (row: RgDbMatrixItem) => {
  router.push({
    path: '/instance-config',
    query: { instanceId: String(row.instanceId) }
  })
}

// 打开新建页签
const handleOpenCreateTab = () => {
  const tabName = 'create_tab'
  const existing = dynamicTabs.value.find(t => t.name === tabName)
  if (existing) {
    activeTab.value = tabName
    loadTabDbMatrix(existing)
    return
  }

  const newTab: DynamicTabItem = {
    name: tabName,
    title: '➕ 新建业务资源组',
    isEdit: false,
    saveLoading: false,
    dbMatrixLoading: false,
    form: {
      id: null,
      groupName: '',
      deptName: '',
      devLead: '',
      dbaLead: '核心DBA',
      description: '',
      workflowTemplates: '["标准 DML 常规两级审批流"]',
      status: 1
    },
    rgPolicyConfig: {
      enforceDryRun: true,
      enableStep3Rollback: true,
      enableStep4DryRun: true
    },
    rgDbMatrixList: [],
    formFieldList: JSON.parse(JSON.stringify(defaultFields))
  }

  dynamicTabs.value.push(newTab)
  activeTab.value = tabName

  const targetTab = dynamicTabs.value.find(t => t.name === tabName) || newTab
  loadTabDbMatrix(targetTab)
}

// 打开编辑页签
const handleOpenEditTab = (row: ResourceGroupItem) => {
  const tabName = `edit_${row.id || row.groupName}`
  const existing = dynamicTabs.value.find(t => t.name === tabName)
  if (existing) {
    activeTab.value = tabName
    loadTabDbMatrix(existing)
    return
  }

  let fields: FormFieldConfig[] = JSON.parse(JSON.stringify(defaultFields))
  let enforceDryRun = true
  let enableStep3Rollback = true
  let enableStep4DryRun = true

  if (row.formConfig) {
    try {
      const parsed = JSON.parse(row.formConfig)
      if (Array.isArray(parsed)) {
        fields = parsed
      } else if (parsed && typeof parsed === 'object') {
        if (Array.isArray(parsed.fields)) {
          fields = parsed.fields
        }
        if (parsed.enforceDryRun !== undefined) enforceDryRun = Boolean(parsed.enforceDryRun)
        if (parsed.enableStep3Rollback !== undefined) enableStep3Rollback = Boolean(parsed.enableStep3Rollback)
        if (parsed.enableStep4DryRun !== undefined) enableStep4DryRun = Boolean(parsed.enableStep4DryRun)
      }
    } catch (e) {}
  }

  const editTab: DynamicTabItem = {
    name: tabName,
    title: `⚙️ ${row.groupName}`,
    isEdit: true,
    saveLoading: false,
    dbMatrixLoading: false,
    form: { ...row },
    rgPolicyConfig: {
      enforceDryRun,
      enableStep3Rollback,
      enableStep4DryRun
    },
    rgDbMatrixList: [],
    formFieldList: fields
  }

  dynamicTabs.value.push(editTab)
  activeTab.value = tabName

  // 获取响应式代理对象并执行异步加载
  const targetTab = dynamicTabs.value.find(t => t.name === tabName) || editTab
  loadTabDbMatrix(targetTab)
}

// 关闭指定页签
const handleCloseTab = (tabName: string) => {
  if (tabName === 'workflow_designer_tab') {
    isWorkflowDesignerTabOpen.value = false
    activeTab.value = 'list'
    fetchWorkflowTemplates()
    return
  }

  const idx = dynamicTabs.value.findIndex(t => t.name === tabName)
  if (idx !== -1) {
    dynamicTabs.value.splice(idx, 1)
  }
  if (activeTab.value === tabName) {
    activeTab.value = 'list'
  }
}

const handleTabRemove = (targetName: string) => {
  handleCloseTab(targetName)
}

// 保存 Tab 内容
const handleSaveTab = async (tab: DynamicTabItem) => {
  if (!tab.form.groupName || !tab.form.groupName.trim()) {
    ElMessage.warning('请输入资源组名称')
    return
  }

  tab.saveLoading = true
  try {
    // 组装数据库审批流映射
    const dbWorkflowMappings: Record<string, string> = {}
    tab.rgDbMatrixList.forEach(item => {
      if (item.customWorkflow && item.customWorkflow !== 'DEFAULT') {
        dbWorkflowMappings[`${item.instanceId}:${item.dbName}`] = item.customWorkflow
      }
    })

    const fullFormConfig = {
      enforceDryRun: tab.rgPolicyConfig.enforceDryRun,
      enableStep3Rollback: tab.rgPolicyConfig.enableStep3Rollback,
      enableStep4DryRun: tab.rgPolicyConfig.enableStep4DryRun,
      dbWorkflowMappings,
      fields: tab.formFieldList
    }

    const payload = {
      ...tab.form,
      workflowTemplates: tab.form.workflowTemplates || '["标准 DML 常规两级审批流"]',
      formConfig: JSON.stringify(fullFormConfig)
    }

    await request.post('/v1/resource-group/save', payload)
    ElMessage.success(`${tab.isEdit ? '业务资源组与审批流绑定修改' : '新建业务资源组'}已成功保存生效！`)

    // 刷新列表
    await fetchResourceGroups()

    // 关闭当前 Tab 并切回列表
    handleCloseTab(tab.name)
  } catch (error: any) {
    ElMessage.error(error.message || '保存业务资源组配置失败')
  } finally {
    tab.saveLoading = false
  }
}

// 删除资源组
const handleDelete = (row: ResourceGroupItem) => {
  ElMessageBox.confirm(
    `确定要删除业务资源组【${row.groupName}】吗？删除后将影响该组下数据库工单的流转路由！`,
    '安全警告',
    {
      confirmButtonText: '确定删除',
      cancelButtonText: '取消',
      type: 'warning'
    }
  ).then(async () => {
    try {
      await request.post(`/v1/resource-group/delete/${row.id}`)
      ElMessage.success('业务资源组删除成功')
      fetchResourceGroups()
    } catch (e: any) {
      ElMessage.error(e.message || '删除失败')
    }
  }).catch(() => {})
}

onMounted(() => {
  fetchResourceGroups()
  fetchWorkflowTemplates()
  fetchAllInstances()
})
</script>

<style scoped>
.resource-group-page {
  width: 100%;
}

.rg-workbench-tabs {
  width: 100%;
}

.rg-workbench-tabs :deep(.el-tabs__content) {
  overflow: visible !important;
}

/* ==================== 企业级一体化置顶工作台头 (无冗余边框) ==================== */
.rg-workbench-tabs > :deep(.el-tabs__header) {
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

.rg-workbench-tabs :deep(.el-tabs__item) {
  font-weight: 600;
  font-size: 13px;
  transition: all 0.2s ease;
  background: #f8fafc;
  margin-right: 4px;
  border-radius: 6px 6px 0 0;
  border: 1px solid #e2e8f0;
  border-bottom: none;
}

.rg-workbench-tabs :deep(.el-tabs__item.is-active) {
  background: #ffffff;
  color: #3b82f6;
  border-color: #cbd5e1;
}

.tab-label-item {
  display: flex;
  align-items: center;
  gap: 6px;
}

.tab-inner-content {
  background: transparent;
}

.header-action {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-top: 14px;
  margin-bottom: 16px;
  background: #ffffff;
  padding: 16px 20px;
  border-radius: 8px;
  border: 1px solid #e2e8f0;
}

.page-title {
  font-size: 18px;
  font-weight: 700;
  color: #1e293b;
  margin: 0 0 4px 0;
}

.page-subtitle {
  font-size: 12px;
  color: #64748b;
}

.action-area {
  display: flex;
  align-items: center;
}

.table-wrapper {
  background: #ffffff;
  border-radius: 8px;
  padding: 16px;
  border: 1px solid #e2e8f0;
}

/* ==================== 编辑页签特有样式 (一体化极简吸顶导航) ==================== */
.edit-tab-content {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.edit-nav-top-bar {
  position: sticky;
  top: 42px;
  z-index: 99;
  display: flex;
  justify-content: space-between;
  align-items: center;
  background: #ffffff;
  padding: 10px 16px;
  border-radius: 0 0 8px 8px;
  border: 1px solid #e2e8f0;
  border-top: 1px solid #f1f5f9;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.05);
  margin-bottom: 4px;
}

.nav-left {
  display: flex;
  align-items: center;
  gap: 16px;
}

.nav-title-group {
  display: flex;
  flex-direction: column;
}

.nav-main-title {
  font-size: 17px;
  font-weight: 700;
  color: #1e293b;
}

.nav-sub-desc {
  font-size: 12px;
  color: #64748b;
  margin-top: 2px;
}

.nav-actions {
  display: flex;
  align-items: center;
  gap: 10px;
}

.config-card {
  border-radius: 8px;
  border: 1px solid #e2e8f0;
  background: #ffffff;
}

.mb-20 {
  margin-bottom: 16px;
}

.card-header-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 15px;
  font-weight: 700;
  color: #1e293b;
}

.step-num-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  background: #3b82f6;
  color: #ffffff;
  border-radius: 50%;
  font-size: 12px;
  font-weight: 800;
}

/* ==================== 策略面板高质感排版 (修复截图问题) ==================== */
.safety-policy-panel {
  padding: 4px 0;
}

.policy-card {
  border: 1px solid #e2e8f0;
  border-radius: 8px;
  padding: 16px 18px;
  background: #f8fafc;
  height: 100%;
  display: flex;
  flex-direction: column;
}

.policy-title {
  font-size: 14px;
  font-weight: 700;
  margin-bottom: 14px;
  display: flex;
  align-items: center;
  gap: 6px;
}

.policy-radio-cards-group {
  display: flex;
  flex-direction: column;
  gap: 12px;
  width: 100%;
}

.policy-radio-item-card {
  border: 1.5px solid #e2e8f0;
  border-radius: 8px;
  padding: 12px 14px;
  background: #ffffff;
  cursor: pointer;
  transition: all 0.2s ease;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.04);
}

.policy-radio-item-card:hover {
  border-color: #93c5fd;
  background: #f0f9ff;
  transform: translateY(-1px);
}

.policy-radio-item-card.is-active {
  border-color: #3b82f6;
  background: #eff6ff;
  box-shadow: 0 2px 6px rgba(59, 130, 246, 0.12);
}

.custom-card-radio {
  display: flex;
  align-items: flex-start;
  width: 100%;
  margin-right: 0;
  height: auto;
  white-space: normal;
}

.custom-card-radio :deep(.el-radio__label) {
  padding-left: 10px;
  white-space: normal;
  line-height: 1.4;
  width: 100%;
}

.radio-content-block {
  display: flex;
  flex-direction: column;
  gap: 4px;
  width: 100%;
}

.radio-title-text {
  font-size: 13.5px;
  font-weight: 700;
}

.danger-text {
  color: #dc2626;
}

.success-text {
  color: #16a34a;
}

.radio-desc-text {
  font-size: 12px;
  color: #64748b;
  line-height: 1.4;
}

.policy-switches-group {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.switch-item-box {
  display: flex;
  justify-content: space-between;
  align-items: center;
  background: #ffffff;
  padding: 12px 14px;
  border-radius: 8px;
  border: 1.5px solid #e2e8f0;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.04);
}

.switch-left {
  display: flex;
  align-items: flex-start;
  gap: 10px;
}

.step-badge-tag {
  font-size: 11px;
  font-weight: 800;
  background: #dbeafe;
  color: #1d4ed8;
  padding: 3px 8px;
  border-radius: 4px;
  white-space: nowrap;
}

.switch-title-wrap {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.switch-title {
  font-size: 13px;
  font-weight: 700;
  color: #1e293b;
}

.switch-desc {
  font-size: 11.5px;
  color: #64748b;
}

/* ==================== 矩阵与引导样式 ==================== */
.db-matrix-section {
  padding: 4px 0;
}

.matrix-jump-guide-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: #f0fdf4;
  border: 1px dashed #86efac;
  padding: 10px 14px;
  border-radius: 6px;
  margin-bottom: 14px;
}

.guide-left {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  font-weight: 500;
  color: #166534;
}

.tab-bottom-action-bar {
  display: flex;
  justify-content: flex-end;
  gap: 14px;
  background: #ffffff;
  padding: 16px 20px;
  border-radius: 8px;
  border: 1px solid #e2e8f0;
  margin-top: 10px;
}

.embedded-designer-container {
  padding: 4px 0;
}
</style>
