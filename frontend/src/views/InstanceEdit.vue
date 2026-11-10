<template>
  <div class="instance-edit-container page-container">
    <!-- 顶部导航栏 -->
    <div class="top-nav-bar">
      <div class="nav-left">
        <el-button :icon="ArrowLeft" @click="handleBack" plain>返回实例列表</el-button>
        <div class="title-wrap">
          <h2 class="page-title">{{ isEdit ? `编辑数据库实例：${form.name || '未命名'}` : '新增数据库实例' }}</h2>
          <div class="page-subtitle">配置数据库连接参数、绑定业务资源组、配置实例标签与细粒度支持操作范围</div>
        </div>
      </div>
      <div class="nav-actions">
        <el-button type="success" plain :icon="Connection" :loading="testingLoading" @click="handleTestConnection">
          测试连接
        </el-button>
        <el-button @click="handleBack">取消</el-button>
        <el-button type="primary" :icon="Check" :loading="saveLoading" @click="handleSave">
          {{ isEdit ? '保存修改并生效' : '立即创建并上线' }}
        </el-button>
      </div>
    </div>

    <!-- 主表单区域 (全景单列纵向流式架构 1 ➔ 2 ➔ 3 ➔ 4 ➔ 5) -->
    <el-form ref="formRef" :model="form" :rules="rules" label-position="top" class="edit-form instance-edit-stream" v-loading="pageLoading">
      
      <!-- 1️⃣ 基础连接与网络认证配置 -->
      <el-card shadow="hover" class="config-card stream-card mb-20">
        <template #header>
          <div class="card-header-title">
            <span class="step-num-badge">1</span>
            <el-icon color="#409EFF"><Coin /></el-icon>
            <span class="step-title-text">基础连接与认证凭据 (Basic Connection & Auth)</span>
          </div>
        </template>

        <el-row :gutter="16">
          <el-col :xs="24" :sm="12" :md="8">
            <el-form-item label="实例名称" prop="name">
              <el-input v-model="form.name" placeholder="如：车险核心生产库 / 营销管理达梦集群" />
            </el-form-item>
          </el-col>
          <el-col :xs="24" :sm="12" :md="5">
            <el-form-item label="运行环境" prop="env">
              <el-select v-model="form.env" style="width: 100%;">
                <el-option label="DEV 开发环境" value="DEV" />
                <el-option label="TEST 测试环境" value="TEST" />
                <el-option label="PROD 生产环境" value="PROD" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :xs="24" :sm="12" :md="6">
            <el-form-item label="数据库引擎类型" prop="dbType">
              <el-select v-model="form.dbType" style="width: 100%;" @change="handleDbTypeChange">
                <el-option label="MySQL (5.7 / 8.x / RDS)" value="mysql" />
                <el-option label="达梦数据库 (DM8)" value="dameng" />
                <el-option label="PostgreSQL (12+ / 14+)" value="postgresql" />
                <el-option label="Oracle (11g / 19c)" value="oracle" />
                <el-option label="TiDB (分布式数据库)" value="tidb" />
                <el-option label="OceanBase (MySQL 模式)" value="oceanbase" />
                <el-option label="人大金仓 (KingbaseES)" value="kingbase" />
                <el-option label="华为 openGauss" value="opengauss" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :xs="24" :sm="12" :md="5">
            <el-form-item label="连接隧道">
              <el-select v-model="form.connectionTunnel" style="width: 100%;">
                <el-option label="DIRECT (直连)" value="DIRECT" />
                <el-option label="SSH_TUNNEL (SSH代理隧道)" value="SSH_TUNNEL" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>

        <!-- 结构化连接配置 (IP / 端口 / 库名) -->
        <div class="structured-box" style="margin-bottom: 16px;">
          <div class="sub-section-title">
            <el-icon><LocationInformation /></el-icon>
            <span>网络与主机配置 (结构化填写)</span>
          </div>
          <el-row :gutter="16">
            <el-col :xs="24" :sm="14" :md="12">
              <el-form-item label="数据库主机 / IP 地址" prop="host">
                <el-input
                  v-model="form.host"
                  placeholder="如: rm-uf6ab...mysql.rds.aliyuncs.com 或 127.0.0.1"
                  @input="syncStructuredToJdbcUrl"
                />
              </el-form-item>
            </el-col>
            <el-col :xs="12" :sm="5" :md="4">
              <el-form-item label="端口号" prop="port">
                <el-input-number
                  v-model="form.port"
                  :min="1"
                  :max="65535"
                  controls-position="right"
                  style="width: 100%;"
                  @change="syncStructuredToJdbcUrl"
                />
              </el-form-item>
            </el-col>
            <el-col :xs="12" :sm="5" :md="8">
              <el-form-item label="默认数据库名 (Schema / Catalog)" prop="databaseName">
                <el-input
                  v-model="form.databaseName"
                  placeholder="如: huiqitong_erp (用于探测和初始化连接)"
                  @input="syncStructuredToJdbcUrl"
                />
              </el-form-item>
            </el-col>
          </el-row>
        </div>

        <!-- 账号与密码 与 状态 -->
        <el-row :gutter="16">
          <el-col :xs="24" :sm="12" :md="8">
            <el-form-item label="数据库账号" prop="username">
              <el-input v-model="form.username" placeholder="连接用户名 (如: root)" />
            </el-form-item>
          </el-col>
          <el-col :xs="24" :sm="12" :md="8">
            <el-form-item label="连接密码" prop="passwordCipher">
              <el-input
                type="password"
                v-model="form.passwordCipher"
                show-password
                placeholder="落库采用国密 SM4 对称加密"
              />
            </el-form-item>
          </el-col>
          <el-col :xs="24" :sm="12" :md="8">
            <el-form-item label="实例状态">
              <el-radio-group v-model="form.status" style="margin-top: 4px;">
                <el-radio value="APPROVED">启用 (上线运行)</el-radio>
                <el-radio value="DISABLED">禁用 (下线停用)</el-radio>
              </el-radio-group>
            </el-form-item>
          </el-col>
        </el-row>

        <el-row :gutter="16">
          <el-col :xs="24" :sm="12" :md="12">
            <el-form-item label="主库 JDBC 连接串 (自动双向联动)" prop="jdbcUrl">
              <el-input
                v-model="form.jdbcUrl"
                type="textarea"
                :rows="2"
                placeholder="jdbc:mysql://host:port/dbname?useSSL=false"
                @input="syncJdbcUrlToStructured"
              />
            </el-form-item>
          </el-col>
          <el-col :xs="24" :sm="12" :md="12">
            <el-form-item label="只读从库 JDBC URL (可选)">
              <el-input
                v-model="form.readOnlyJdbcUrl"
                type="textarea"
                :rows="2"
                placeholder="用于数据查询控制台只读查询分离，留空则使用主库"
              />
            </el-form-item>
          </el-col>
        </el-row>

        <el-form-item label="业务用途描述">
          <el-input v-model="form.description" placeholder="可选，简述实例归属业务部门、系统模块或架构用途" />
        </el-form-item>

        <!-- 连通性测试结果面板 -->
        <div v-if="testResult" class="test-feedback-card" :class="testResult.success ? 'is-success' : 'is-error'">
          <div class="feedback-header">
            <div style="display: flex; align-items: center; gap: 8px;">
              <el-icon :color="testResult.success ? '#67C23A' : '#F56C6C'" size="20">
                <CircleCheckFilled v-if="testResult.success" />
                <CircleCloseFilled v-else />
              </el-icon>
              <span class="feedback-title">{{ testResult.success ? '数据库连接正常，探测成功！' : '数据库连接失败' }}</span>
            </div>
            <el-tag v-if="testResult.success" type="success" effect="dark" size="small">
              握手耗时: {{ testResult.latencyMs }}ms
            </el-tag>
          </div>

          <div v-if="testResult.success" class="feedback-body">
            <div><b>数据库引擎：</b>{{ testResult.databaseProductName }} (版本: {{ testResult.databaseProductVersion || '未知' }})</div>
            <div><b>驱动程序：</b>{{ testResult.driverName || '官方 JDBC 驱动' }}</div>
          </div>
          <div v-else class="feedback-body error-msg">
            <b>报错详情：</b>{{ testResult.errorMessage || testResult.message }}
          </div>
        </div>
      </el-card>

      <!-- 2️⃣ 授权业务资源组配置 (Shuttle Transfer) -->
      <el-card shadow="hover" class="config-card stream-card mb-20">
        <template #header>
          <div class="card-header-title">
            <span class="step-num-badge">2</span>
            <el-icon color="#E6A23C"><UserFilled /></el-icon>
            <span class="step-title-text">授权业务资源组绑定 (Resource Groups)</span>
            <el-tooltip content="只有属于所选资源组的用户，才有权限在工单与数据查询中访问使用该实例" placement="top">
              <el-icon style="margin-left: 6px; cursor: pointer; color: #909399;"><QuestionFilled /></el-icon>
            </el-tooltip>
          </div>
        </template>

        <div class="transfer-wrapper">
          <div class="shuttle-box">
            <div class="shuttle-panel">
              <div class="shuttle-header">
                <span>可用资源组 ({{ availableResourceGroups.length }})</span>
              </div>
              <el-input
                v-model="rgAvailableFilter"
                placeholder="过滤可用资源组..."
                clearable
                size="small"
                :prefix-icon="Search"
                class="filter-input"
              />
              <div class="shuttle-list">
                <div
                  v-for="item in filteredAvailableRGs"
                  :key="item"
                  class="shuttle-item"
                  :class="{ 'is-selected': selectedAvailableRGs.includes(item) }"
                  @click="toggleSelectAvailableRG(item)"
                >
                  <el-checkbox :model-value="selectedAvailableRGs.includes(item)" @click.stop="toggleSelectAvailableRG(item)" />
                  <span class="item-text">{{ item }}</span>
                </div>
                <div v-if="filteredAvailableRGs.length === 0" class="empty-text">无匹配资源组</div>
              </div>
              <div class="shuttle-footer">
                <el-button link size="small" @click="selectAllAvailableRGs">全选</el-button>
              </div>
            </div>

            <!-- 穿梭操作按钮组 -->
            <div class="shuttle-controls">
              <el-button
                type="primary"
                circle
                :icon="ArrowRight"
                :disabled="selectedAvailableRGs.length === 0"
                @click="moveToSelectedRGs"
              />
              <el-button
                type="primary"
                circle
                :icon="ArrowLeft"
                :disabled="selectedChosenRGs.length === 0"
                @click="moveToAvailableRGs"
              />
            </div>

            <div class="shuttle-panel chosen-panel">
              <div class="shuttle-header chosen-header">
                <span>选中的资源组 ({{ chosenResourceGroups.length }})</span>
                <el-icon color="#67C23A"><Check /></el-icon>
              </div>
              <el-input
                v-model="rgChosenFilter"
                placeholder="过滤已选资源组..."
                clearable
                size="small"
                :prefix-icon="Search"
                class="filter-input"
              />
              <div class="shuttle-list">
                <div
                  v-for="item in filteredChosenRGs"
                  :key="item"
                  class="shuttle-item"
                  :class="{ 'is-selected': selectedChosenRGs.includes(item) }"
                  @click="toggleSelectChosenRG(item)"
                >
                  <el-checkbox :model-value="selectedChosenRGs.includes(item)" @click.stop="toggleSelectChosenRG(item)" />
                  <span class="item-text">{{ item }}</span>
                </div>
                <div v-if="filteredChosenRGs.length === 0" class="empty-text">未绑定任何资源组</div>
              </div>
              <div class="shuttle-footer">
                <el-button link type="danger" size="small" @click="removeAllChosenRGs">删除全部</el-button>
              </div>
            </div>
          </div>
        </div>

        <!-- 业务与流程联动机制说明 -->
        <div class="flow-relation-tip" style="margin-top: 14px;">
          <el-icon color="#409EFF"><InfoFilled /></el-icon>
          <span><b>审批流联动机制：</b>当开发人员提交此实例的 SQL 变更时，审批流引擎将根据上述绑定的资源组，自动路由给对应组内的<b>【开发组长】</b>进行初审。</span>
        </div>
      </el-card>

      <!-- 3️⃣ 实例业务标签管理 (Instance Business Tags) -->
      <el-card shadow="hover" class="config-card stream-card mb-20">
        <template #header>
          <div class="card-header-title">
            <span class="step-num-badge">3</span>
            <el-icon color="#67C23A"><CollectionTag /></el-icon>
            <span class="step-title-text">实例业务标签管理 (Instance Tags)</span>
            <el-tooltip content="用于在实例列表、工作台与审批流中标识数据库的业务重要级、架构属性及数据敏感分类" placement="top">
              <el-icon style="margin-left: 6px; cursor: pointer; color: #909399;"><QuestionFilled /></el-icon>
            </el-tooltip>
          </div>
        </template>

        <!-- 快捷预设标签添加 -->
        <div class="tag-preset-area">
          <span class="preset-label">快捷预设标签：</span>
          <div class="preset-pills">
            <el-button
              v-for="pt in tagPresets"
              :key="pt"
              size="small"
              plain
              :type="chosenTagsList.includes(pt) ? 'success' : 'info'"
              @click="toggleTagPreset(pt)"
              style="margin: 2px 4px;"
            >
              {{ chosenTagsList.includes(pt) ? '✓ ' : '+ ' }}{{ pt }}
            </el-button>
          </div>
        </div>

        <!-- 当前已选标签列表与自定义输入 -->
        <div class="tag-input-container">
          <div class="tag-badges-box">
            <el-tag
              v-for="tag in chosenTagsList"
              :key="tag"
              closable
              :type="getTagColorType(tag)"
              effect="light"
              style="margin-right: 8px; margin-bottom: 6px;"
              @close="removeTag(tag)"
            >
              {{ tag }}
            </el-tag>
            <span v-if="chosenTagsList.length === 0" style="color: #94a3b8; font-size: 13px;">暂未配置标签，请从上方快捷点击或在下方输入</span>
          </div>

          <div class="tag-add-bar">
            <el-input
              v-model="customTagInput"
              placeholder="输入自定义标签名后按回车添加..."
              size="small"
              style="width: 320px; margin-right: 8px;"
              @keyup.enter="addCustomTag"
            />
            <el-button size="small" type="primary" plain @click="addCustomTag">添加自定义标签</el-button>
          </div>
        </div>
      </el-card>

      <!-- 4️⃣ 专属固定审批流配置 (Pinned Workflow) -->
      <el-card shadow="hover" class="config-card stream-card mb-20">
        <template #header>
          <div class="card-header-title">
            <span class="step-num-badge">4</span>
            <el-icon color="#E6A23C"><Share /></el-icon>
            <span class="step-title-text">专属固定审批流配置 (Pinned Workflow · 专库专流)</span>
            <el-tooltip content="针对特定特殊数据库，可绑定一套固定审批流，跳过系统的动态综合决策直接强制生效" placement="top">
              <el-icon style="margin-left: 6px; cursor: pointer; color: #909399;"><QuestionFilled /></el-icon>
            </el-tooltip>
          </div>
        </template>

        <div class="pinned-flow-content">
          <el-form-item label="审批流决策机制">
            <el-radio-group v-model="workflowRoutingMode" @change="handleWorkflowModeChange" class="custom-radio-group">
              <el-radio value="DYNAMIC">
                <div class="radio-title">⚡ 动态综合智能决策 (默认)</div>
                <div class="radio-desc">根据工单类型、影响行数、实例标签与资源组自动计算最优审批流</div>
              </el-radio>
              <el-radio value="PINNED" style="margin-top: 10px;">
                <div class="radio-title pinned-title">🔥 指定专属固定审批流 (专库专流)</div>
                <div class="radio-desc">此实例所有工单均强制走所选的固定流转链路，跳过任何动态判定</div>
              </el-radio>
            </el-radio-group>
          </el-form-item>

          <div v-if="workflowRoutingMode === 'PINNED'" class="pinned-select-box">
            <el-form-item label="选择绑定的专属固定审批流模板" required>
              <el-select
                v-model="form.fixedWorkflowTemplateId"
                placeholder="请选择专属固定审批流模板"
                style="width: 100%;"
                clearable
                @change="handleSelectPinnedTemplate"
              >
                <el-option
                  v-for="tpl in allWorkflowTemplates"
                  :key="tpl.id"
                  :label="`${tpl.templateName} (${tpl.flowType})`"
                  :value="tpl.id"
                >
                  <div style="display: flex; justify-content: space-between; align-items: center;">
                    <span style="font-weight: 600;">{{ tpl.templateName }}</span>
                    <el-tag size="small" type="warning">{{ tpl.flowType }}</el-tag>
                  </div>
                </el-option>
              </el-select>

              <!-- 快捷直达流程设计中心引导栏 -->
              <div class="workflow-jump-banner" style="margin-top: 10px; background: #f0fdf4; border: 1px dashed #86efac; padding: 10px 14px; border-radius: 6px; display: flex; align-items: center; justify-content: space-between;">
                <span class="jump-tip" style="font-size: 12px; color: #166534; font-weight: 500;">
                  💡 没有符合当前业务需要的审批流程？
                </span>
                <el-button type="success" link :icon="Share" @click="goToWorkflowDesigner" style="font-weight: 600;">
                  前往【审批流设计中心】新建/编辑流程 ➔
                </el-button>
              </div>
            </el-form-item>

            <!-- 选中的固定模板详情预览 -->
            <div v-if="selectedPinnedTemplate" class="pinned-template-preview">
              <div class="preview-row">
                <b>🎯 触发与生效说明：</b>{{ selectedPinnedTemplate.triggerCondition || '所有工单强制生效' }}
              </div>
              <div class="preview-row">
                <b>📋 适用类型：</b><el-tag size="small" type="info">{{ selectedPinnedTemplate.flowType }}</el-tag>
              </div>
              <div class="preview-row" style="margin-top: 6px;">
                <b>🔗 预定审批流转链路：</b>
                <div class="pipeline-tags">
                  <el-tag
                    v-for="(n, idx) in parseNodes(selectedPinnedTemplate.nodeConfig)"
                    :key="idx"
                    type="success"
                    effect="plain"
                    size="small"
                    style="margin: 2px 6px 2px 0;"
                  >
                    {{ Number(idx) + 1 }}. {{ n.nodeName }} ({{ n.role }})
                  </el-tag>
                </div>
              </div>
            </div>
          </div>
        </div>
      </el-card>

      <!-- 5️⃣ 支持操作范围与管控矩阵 (Supported Operations Matrix) -->
      <el-card shadow="hover" class="config-card stream-card mb-20">
        <template #header>
          <div class="card-header-title" style="justify-content: space-between; width: 100%;">
            <div style="display: flex; align-items: center;">
              <span class="step-num-badge">5</span>
              <el-icon color="#409EFF"><Operation /></el-icon>
              <span class="step-title-text">支持操作范围与管控影响 (Supported Operations Matrix)</span>
              <el-tooltip content="严格定义此实例所允许的操作类型。未勾选的操作将在工单提交、在线查询或发布执行时被全链路强行拦截" placement="top">
                <el-icon style="margin-left: 6px; cursor: pointer; color: #909399;"><QuestionFilled /></el-icon>
              </el-tooltip>
            </div>
            <div class="op-preset-btns">
              <el-button size="small" link type="primary" @click="applyFullOpsPreset">全开 (全功能)</el-button>
              <el-button size="small" link type="warning" @click="applyReadOnlyPreset">只读分析库</el-button>
              <el-button size="small" link type="success" @click="applySafeProdPreset">安全生产库</el-button>
            </div>
          </div>
        </template>

        <!-- 8 大操作配置卡片网格 (响应式多列布局) -->
        <div class="op-matrix-grid">
          <div
            v-for="op in ALL_OP_DEFINITIONS"
            :key="op.name"
            class="op-card-item"
            :class="{ 'is-active': chosenOpsList.includes(op.name) }"
            @click="toggleOperation(op.name)"
          >
            <div class="op-card-top">
              <div class="op-title-area">
                <span class="op-icon">{{ op.icon }}</span>
                <span class="op-name">{{ op.name }}</span>
              </div>
              <el-switch
                :model-value="chosenOpsList.includes(op.name)"
                @click.stop
                @change="toggleOperation(op.name)"
                size="small"
              />
            </div>
            <div class="op-impact-desc">
              <b>管控影响：</b>{{ op.description }}
            </div>
            <div class="op-target-tag">
              <span class="impact-scope">影响模块: {{ op.scope }}</span>
            </div>
          </div>
        </div>
      </el-card>

      <!-- 底部操作悬浮/固定栏 -->
      <div class="bottom-action-bar">
        <el-button type="success" plain :icon="Connection" :loading="testingLoading" @click="handleTestConnection">
          测试连通性
        </el-button>
        <el-button @click="handleBack">取消</el-button>
        <el-button type="primary" :icon="Check" :loading="saveLoading" @click="handleSave">
          {{ isEdit ? '保存修改并生效' : '立即创建并上线' }}
        </el-button>
      </div>
    </el-form>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import {
  ArrowLeft,
  Check,
  Connection,
  Coin,
  LocationInformation,
  UserFilled,
  Operation,
  Search,
  ArrowRight,
  CircleCheckFilled,
  CircleCloseFilled,
  InfoFilled,
  QuestionFilled,
  CollectionTag,
  Share
} from '@element-plus/icons-vue'
import request from '../utils/request'

const route = useRoute()
const router = useRouter()

const pageLoading = ref(false)
const saveLoading = ref(false)
const testingLoading = ref(false)
const formRef = ref<FormInstance>()
const testResult = ref<any>(null)

const isEdit = computed(() => !!route.params.id || !!route.query.id)
const instanceId = computed(() => (route.params.id as string) || (route.query.id as string))

// 8 大标准支持操作与全系统影响范围定义
const ALL_OP_DEFINITIONS = [
  {
    name: '支持上线',
    icon: '🚀',
    scope: '工单发布引擎',
    description: '允许将已审批通过的工单在此实例上调度或立即执行发布。若关闭，工单无法在该实例上线执行。'
  },
  {
    name: '支持查询',
    icon: '🔍',
    scope: '数据查询控制台',
    description: '允许在数据查询控制台和工作台查询此实例数据。若关闭，查询控制台将禁止向此实例发起查询。'
  },
  {
    name: '支持DML变更',
    icon: '📝',
    scope: 'SQL 变更工单',
    description: '允许提交 INSERT / UPDATE / DELETE 数据变更工单。若关闭，创建 DML 工单时将直接拦截报警。'
  },
  {
    name: '支持DDL结构变更',
    icon: '🏗️',
    scope: 'SQL 结构工单',
    description: '允许提交 CREATE / ALTER / DROP 库表结构变更工单。若关闭，创建 DDL 工单时将直接拦截。'
  },
  {
    name: '支持数据导出',
    icon: '📤',
    scope: '数据安全防脱库',
    description: '允许在在线查询和工单执行后导出 CSV/JSON 结果。若关闭，前端导出功能禁用以杜绝数据泄露。'
  },
  {
    name: '支持事务预执行',
    icon: '⚡',
    scope: 'SQL 预执行校验',
    description: '允许在工单提交前对此实例进行 SQL 事务级沙箱预执行测试与影响行数精准评估。'
  },
  {
    name: '支持数据脱敏',
    icon: '🛡️',
    scope: '动态脱敏引擎',
    description: '允许在数据查询时对该实例的手机号、身份证、密码等敏感字段执行动态脱敏保护。'
  },
  {
    name: '支持历史回滚',
    icon: '⏪',
    scope: '数据回滚方案',
    description: '允许在 DML 执行完成后自动生成并留存反向回滚 SQL 脚本，保障生产故障快速回退。'
  }
]

// 实例标签预设
const tagPresets = ref<string[]>([
  '核心生产库',
  '只读从库',
  '敏捷测试库',
  '高可用集群',
  '敏感数据资产',
  '金融账务核心',
  '归档备份库',
  '分库分表集群'
])

const chosenTagsList = ref<string[]>(['核心生产库'])
const customTagInput = ref('')
const chosenOpsList = ref<string[]>([
  '支持上线',
  '支持查询',
  '支持DML变更',
  '支持DDL结构变更',
  '支持数据导出',
  '支持事务预执行',
  '支持数据脱敏',
  '支持历史回滚'
])

// 表单数据
const form = ref({
  id: null as number | null,
  name: '',
  dbType: 'mysql',
  env: 'DEV',
  connectionTunnel: 'DIRECT',
  status: 'APPROVED',
  host: '127.0.0.1',
  port: 3306,
  databaseName: '',
  username: 'root',
  passwordCipher: '',
  jdbcUrl: '',
  readOnlyJdbcUrl: '',
  description: '',
  resourceGroups: '',
  tags: '',
  supportedOps: '',
  fixedWorkflowTemplateId: null as number | null,
  fixedWorkflowTemplateName: '',
  tenantId: '1'
})

const rules = ref<FormRules>({
  name: [{ required: true, message: '请输入实例名称', trigger: 'blur' }],
  dbType: [{ required: true, message: '请选择数据库类型', trigger: 'change' }],
  host: [{ required: true, message: '请输入数据库主机 / IP 地址', trigger: 'blur' }],
  port: [{ required: true, message: '请输入端口号', trigger: 'blur' }],
  username: [{ required: true, message: '请输入连接用户名', trigger: 'blur' }],
  passwordCipher: [{ required: true, message: '请输入连接密码', trigger: 'blur' }]
})

const defaultPorts: Record<string, number> = {
  mysql: 3306,
  tidb: 3306,
  oceanbase: 3306,
  postgresql: 5432,
  opengauss: 5432,
  oracle: 1521,
  dameng: 5236,
  kingbase: 54321
}

// -------------------- 资源组穿梭数据 --------------------
const allSystemResourceGroups = ref<string[]>([])
const chosenResourceGroups = ref<string[]>([])
const selectedAvailableRGs = ref<string[]>([])
const selectedChosenRGs = ref<string[]>([])
const rgAvailableFilter = ref('')
const rgChosenFilter = ref('')

const availableResourceGroups = computed(() => {
  return allSystemResourceGroups.value.filter(rg => !chosenResourceGroups.value.includes(rg))
})

const filteredAvailableRGs = computed(() => {
  if (!rgAvailableFilter.value) return availableResourceGroups.value
  const q = rgAvailableFilter.value.toLowerCase()
  return availableResourceGroups.value.filter(rg => rg.toLowerCase().includes(q))
})

const filteredChosenRGs = computed(() => {
  if (!rgChosenFilter.value) return chosenResourceGroups.value
  const q = rgChosenFilter.value.toLowerCase()
  return chosenResourceGroups.value.filter(rg => rg.toLowerCase().includes(q))
})

const toggleSelectAvailableRG = (item: string) => {
  const idx = selectedAvailableRGs.value.indexOf(item)
  if (idx > -1) selectedAvailableRGs.value.splice(idx, 1)
  else selectedAvailableRGs.value.push(item)
}

const toggleSelectChosenRG = (item: string) => {
  const idx = selectedChosenRGs.value.indexOf(item)
  if (idx > -1) selectedChosenRGs.value.splice(idx, 1)
  else selectedChosenRGs.value.push(item)
}

const selectAllAvailableRGs = () => {
  selectedAvailableRGs.value = [...filteredAvailableRGs.value]
}

const moveToSelectedRGs = () => {
  chosenResourceGroups.value.push(...selectedAvailableRGs.value)
  selectedAvailableRGs.value = []
}

const moveToAvailableRGs = () => {
  chosenResourceGroups.value = chosenResourceGroups.value.filter(item => !selectedChosenRGs.value.includes(item))
  selectedChosenRGs.value = []
}

const removeAllChosenRGs = () => {
  chosenResourceGroups.value = []
  selectedChosenRGs.value = []
}

// -------------------- 实例标签操作 --------------------
const getTagColorType = (tag: string) => {
  if (tag.includes('生产') || tag.includes('敏感') || tag.includes('核心')) return 'danger'
  if (tag.includes('只读') || tag.includes('从库')) return 'primary'
  if (tag.includes('测试')) return 'warning'
  if (tag.includes('集群')) return 'success'
  return 'info'
}

const toggleTagPreset = (preset: string) => {
  const idx = chosenTagsList.value.indexOf(preset)
  if (idx > -1) {
    chosenTagsList.value.splice(idx, 1)
  } else {
    chosenTagsList.value.push(preset)
  }
}

const addCustomTag = () => {
  const tag = customTagInput.value.trim()
  if (!tag) return
  if (!chosenTagsList.value.includes(tag)) {
    chosenTagsList.value.push(tag)
  }
  customTagInput.value = ''
}

const removeTag = (tag: string) => {
  const idx = chosenTagsList.value.indexOf(tag)
  if (idx > -1) {
    chosenTagsList.value.splice(idx, 1)
  }
}

// -------------------- 支持操作操作 --------------------
const toggleOperation = (opName: string) => {
  const idx = chosenOpsList.value.indexOf(opName)
  if (idx > -1) {
    chosenOpsList.value.splice(idx, 1)
  } else {
    chosenOpsList.value.push(opName)
  }
}

const applyFullOpsPreset = () => {
  chosenOpsList.value = ALL_OP_DEFINITIONS.map(o => o.name)
  ElMessage.success('已应用【全功能标准实例】支持操作配置')
}

const applyReadOnlyPreset = () => {
  chosenOpsList.value = ['支持查询', '支持数据导出', '支持数据脱敏']
  ElMessage.success('已应用【只读分析库】支持操作配置')
}

const applySafeProdPreset = () => {
  chosenOpsList.value = ['支持上线', '支持DML变更', '支持事务预执行', '支持数据脱敏', '支持历史回滚']
  ElMessage.success('已应用【安全生产变更库】支持操作配置')
}

// -------------------- 结构化与 JDBC URL 同步 --------------------
const handleDbTypeChange = (type: string) => {
  if (!form.value.port || Object.values(defaultPorts).includes(form.value.port)) {
    form.value.port = defaultPorts[type] || 3306
  }
  syncStructuredToJdbcUrl()
}

const syncStructuredToJdbcUrl = () => {
  const host = form.value.host?.trim() || '127.0.0.1'
  const port = form.value.port || defaultPorts[form.value.dbType] || 3306
  const db = form.value.databaseName?.trim() || ''
  const type = form.value.dbType?.toLowerCase() || 'mysql'

  if (type === 'postgresql' || type === 'opengauss') {
    form.value.jdbcUrl = `jdbc:postgresql://${host}:${port}/${db || 'postgres'}?useSSL=false&serverTimezone=UTC`
  } else if (type === 'oracle') {
    form.value.jdbcUrl = `jdbc:oracle:thin:@${host}:${port}:${db || 'ORCL'}`
  } else if (type === 'dameng') {
    form.value.jdbcUrl = `jdbc:dm://${host}:${port}${db ? '?schema=' + db : ''}`
  } else if (type === 'kingbase') {
    form.value.jdbcUrl = `jdbc:kingbase8://${host}:${port}/${db || 'SYSTEM'}`
  } else {
    form.value.jdbcUrl = `jdbc:mysql://${host}:${port}/${db}?useSSL=false&allowPublicKeyRetrieval=true&serverTimezone=UTC`
  }
}

const syncJdbcUrlToStructured = () => {
  const url = form.value.jdbcUrl?.trim()
  if (!url) return

  try {
    if (url.startsWith('jdbc:mysql://') || url.startsWith('jdbc:tidb://') || url.startsWith('jdbc:oceanbase://')) {
      const match = url.match(/jdbc:(?:mysql|tidb|oceanbase):\/\/([^:/]+)(?::(\d+))?(?:\/([^?]+))?/)
      if (match) {
        form.value.host = match[1]
        form.value.port = match[2] ? parseInt(match[2], 10) : 3306
        form.value.databaseName = match[3] || ''
      }
    } else if (url.startsWith('jdbc:postgresql://') || url.startsWith('jdbc:opengauss://')) {
      const match = url.match(/jdbc:(?:postgresql|opengauss):\/\/([^:/]+)(?::(\d+))?(?:\/([^?]+))?/)
      if (match) {
        form.value.host = match[1]
        form.value.port = match[2] ? parseInt(match[2], 10) : 5432
        form.value.databaseName = match[3] || ''
      }
    } else if (url.startsWith('jdbc:dm://')) {
      const match = url.match(/jdbc:dm:\/\/([^:/]+)(?::(\d+))?(?:\?schema=([^&]+))?/)
      if (match) {
        form.value.host = match[1]
        form.value.port = match[2] ? parseInt(match[2], 10) : 5236
        form.value.databaseName = match[3] || ''
      }
    }
  } catch (e) {
    console.warn('Parse JDBC URL failed', e)
  }
}

// -------------------- 测试连接 --------------------
const handleTestConnection = async () => {
  if (!form.value.host || !form.value.username) {
    ElMessage.warning('请先填写主机地址和用户名')
    return
  }

  testingLoading.value = true
  testResult.value = null
  try {
    const payload = {
      dbType: form.value.dbType,
      host: form.value.host,
      port: form.value.port,
      databaseName: form.value.databaseName,
      username: form.value.username,
      passwordCipher: form.value.passwordCipher,
      jdbcUrl: form.value.jdbcUrl
    }
    const res: any = await request.post('/v1/instance/test-connection-params', payload)
    testResult.value = res.data
    if (res.data?.success) {
      ElMessage.success('数据库连接测试成功！')
    } else {
      ElMessage.error(res.data?.errorMessage || '数据库连接测试失败')
    }
  } catch (err: any) {
    testResult.value = {
      success: false,
      errorMessage: err.message || '网络连接异常'
    }
  } finally {
    testingLoading.value = false
  }
}

// -------------------- 专属固定审批流配置 --------------------
const allWorkflowTemplates = ref<any[]>([])
const workflowRoutingMode = ref<'DYNAMIC' | 'PINNED'>('DYNAMIC')

const selectedPinnedTemplate = computed(() => {
  return allWorkflowTemplates.value.find(t => t.id === form.value.fixedWorkflowTemplateId)
})

const handleWorkflowModeChange = (mode: string) => {
  if (mode === 'DYNAMIC') {
    form.value.fixedWorkflowTemplateId = null
    form.value.fixedWorkflowTemplateName = ''
  }
}

const handleSelectPinnedTemplate = (tplId: number) => {
  const t = allWorkflowTemplates.value.find(x => x.id === tplId)
  form.value.fixedWorkflowTemplateName = t ? t.templateName : ''
}

const goToWorkflowDesigner = () => {
  router.push('/workflow-designer')
}

const parseNodes = (nodeConfigStr?: string) => {
  if (!nodeConfigStr) return []
  try {
    const arr = JSON.parse(nodeConfigStr)
    return Array.isArray(arr) ? arr : []
  } catch (e) {
    return []
  }
}

const fetchAllWorkflowTemplates = async () => {
  try {
    const res: any = await request.get('/v1/workflow/template/list')
    allWorkflowTemplates.value = Array.isArray(res.data) ? res.data : []
  } catch (e) {
    console.error('Failed to load workflow templates', e)
  }
}

// -------------------- 页面初始化与保存 --------------------
const fetchSystemResourceGroups = async () => {
  try {
    const res: any = await request.get('/v1/instance/resource-groups')
    allSystemResourceGroups.value = Array.isArray(res.data) ? res.data : []
  } catch (e) {
    console.error('Failed to load resource groups', e)
  }
}

const fetchInstanceData = async (id: string | number) => {
  pageLoading.value = true
  try {
    const res: any = await request.get(`/v1/instance/${id}`)
    const data = res.data
    if (data) {
      form.value = {
        ...form.value,
        ...data
      }

      // 解析专属固定审批流
      if (data.fixedWorkflowTemplateId) {
        workflowRoutingMode.value = 'PINNED'
        form.value.fixedWorkflowTemplateId = data.fixedWorkflowTemplateId
        form.value.fixedWorkflowTemplateName = data.fixedWorkflowTemplateName || ''
      } else {
        workflowRoutingMode.value = 'DYNAMIC'
        form.value.fixedWorkflowTemplateId = null
        form.value.fixedWorkflowTemplateName = ''
      }

      // 解析资源组
      if (data.resourceGroups) {
        try {
          const parsed = JSON.parse(data.resourceGroups)
          chosenResourceGroups.value = Array.isArray(parsed) ? parsed : [data.resourceGroups]
        } catch (e) {
          chosenResourceGroups.value = data.resourceGroups.split(/[,，]/).map((s: string) => s.trim()).filter(Boolean)
        }
      } else {
        chosenResourceGroups.value = []
      }

      // 解析标签
      if (data.tags) {
        try {
          const parsed = JSON.parse(data.tags)
          chosenTagsList.value = Array.isArray(parsed) ? parsed : [data.tags]
        } catch (e) {
          chosenTagsList.value = data.tags.split(/[,，]/).map((s: string) => s.trim()).filter(Boolean)
        }
      } else {
        chosenTagsList.value = ['核心生产库']
      }

      // 解析支持操作
      if (data.supportedOps) {
        try {
          const parsed = JSON.parse(data.supportedOps)
          chosenOpsList.value = Array.isArray(parsed) ? parsed : [data.supportedOps]
        } catch (e) {
          chosenOpsList.value = data.supportedOps.split(/[,，]/).map((s: string) => s.trim()).filter(Boolean)
        }
      } else {
        chosenOpsList.value = ALL_OP_DEFINITIONS.map(o => o.name)
      }
    }
  } catch (err) {
    console.error('Failed to load instance', err)
  } finally {
    pageLoading.value = false
  }
}

const handleSave = async () => {
  if (!formRef.value) return
  await formRef.value.validate(async (valid) => {
    if (!valid) return
    saveLoading.value = true
    try {
      const payload: any = {
        ...form.value,
        fixedWorkflowTemplateId: workflowRoutingMode.value === 'PINNED' ? form.value.fixedWorkflowTemplateId : null,
        fixedWorkflowTemplateName: workflowRoutingMode.value === 'PINNED' ? form.value.fixedWorkflowTemplateName : null,
        resourceGroups: JSON.stringify(chosenResourceGroups.value),
        tags: JSON.stringify(chosenTagsList.value),
        supportedOps: JSON.stringify(chosenOpsList.value)
      }

      if (form.value.id) {
        await request.post('/v1/instance/save', payload)
      } else {
        await request.post('/v1/instance/save', payload)
      }

      ElMessage.success('数据库实例、专属审批流与管控配置保存成功！')
      handleBack()
    } catch (err: any) {
      ElMessage.error(err.response?.data?.message || '保存失败')
    } finally {
      saveLoading.value = false
    }
  })
}

const handleBack = () => {
  router.push('/instance-list')
}

onMounted(async () => {
  await fetchSystemResourceGroups()
  await fetchAllWorkflowTemplates()
  if (isEdit.value && instanceId.value) {
    await fetchInstanceData(instanceId.value)
  }
})
</script>

<style scoped>
.top-nav-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
  background: #ffffff;
  padding: 14px 20px;
  border-radius: 8px;
  border: 1px solid #e2e8f0;
}

.nav-left {
  display: flex;
  align-items: center;
  gap: 16px;
}

.page-title {
  font-size: 18px;
  font-weight: 700;
  color: #1e293b;
  margin: 0;
}

.page-subtitle {
  font-size: 12px;
  color: #64748b;
  margin-top: 2px;
}

.instance-edit-stream {
  display: flex;
  flex-direction: column;
  gap: 16px;
  max-width: 1280px;
  margin: 0 auto;
}

.stream-card {
  border-radius: 8px;
  border: 1px solid #e2e8f0;
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.04);
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
  font-weight: 700;
}

.step-title-text {
  font-weight: 700;
  font-size: 15px;
  color: #1e293b;
}

.bottom-action-bar {
  display: flex;
  justify-content: flex-end;
  align-items: center;
  gap: 12px;
  padding: 16px 20px;
  background: #ffffff;
  border-radius: 8px;
  border: 1px solid #e2e8f0;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);
  margin-top: 8px;
  margin-bottom: 24px;
}

.config-card {
  border-radius: 8px;
  border: 1px solid #e2e8f0;
}

.mb-20 {
  margin-bottom: 20px;
}

.card-header-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 700;
  font-size: 15px;
  color: #1e293b;
}

.sub-section-title {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  font-weight: 600;
  color: #475569;
  margin-bottom: 12px;
}

.structured-box {
  background: #f8fafc;
  padding: 14px;
  border-radius: 6px;
  border: 1px solid #e2e8f0;
  margin-bottom: 16px;
}

.transfer-wrapper {
  margin-top: 8px;
}

.shuttle-box {
  display: flex;
  align-items: center;
  gap: 12px;
}

.shuttle-panel {
  flex: 1;
  border: 1px solid #e2e8f0;
  border-radius: 6px;
  background: #ffffff;
  display: flex;
  flex-direction: column;
  height: 240px;
}

.shuttle-header {
  padding: 8px 12px;
  font-size: 13px;
  font-weight: 600;
  color: #334155;
  background: #f8fafc;
  border-bottom: 1px solid #e2e8f0;
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.chosen-header {
  background: #f0fdf4;
  color: #166534;
}

.filter-input {
  padding: 6px 8px;
  border-bottom: 1px solid #f1f5f9;
}

.shuttle-list {
  flex: 1;
  overflow-y: auto;
  padding: 6px;
}

.shuttle-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 12px;
  color: #334155;
}

.shuttle-item:hover {
  background: #f1f5f9;
}

.shuttle-item.is-selected {
  background: #eff6ff;
  color: #2563eb;
}

.empty-text {
  text-align: center;
  font-size: 12px;
  color: #94a3b8;
  padding-top: 40px;
}

.shuttle-footer {
  padding: 6px 12px;
  border-top: 1px solid #e2e8f0;
  background: #f8fafc;
  display: flex;
  justify-content: flex-end;
}

.shuttle-controls {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.flow-relation-tip {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  font-size: 12px;
  color: #475569;
  background: #eff6ff;
  padding: 8px 12px;
  border-radius: 6px;
  margin-top: 12px;
  line-height: 1.5;
}

/* 实例标签区域 */
.tag-preset-area {
  display: flex;
  align-items: center;
  margin-bottom: 12px;
  flex-wrap: wrap;
  gap: 6px;
}

.preset-label {
  font-size: 12px;
  font-weight: 600;
  color: #475569;
}

.tag-input-container {
  background: #f8fafc;
  border: 1px solid #e2e8f0;
  border-radius: 6px;
  padding: 12px;
}

.tag-badges-box {
  min-height: 36px;
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  margin-bottom: 10px;
}

.tag-add-bar {
  display: flex;
  align-items: center;
}

/* 支持操作矩阵网格 */
.op-preset-btns {
  display: flex;
  gap: 6px;
}

.op-matrix-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
  gap: 12px;
  margin-top: 8px;
}

.op-card-item {
  background: #ffffff;
  border: 1px solid #e2e8f0;
  border-radius: 6px;
  padding: 12px;
  cursor: pointer;
  transition: all 0.2s ease;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
}

.op-card-item:hover {
  border-color: #93c5fd;
  box-shadow: 0 2px 8px rgba(0,0,0,0.05);
}

.op-card-item.is-active {
  border-color: #3b82f6;
  background: #f0f9ff;
}

.op-card-top {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 6px;
}

.op-title-area {
  display: flex;
  align-items: center;
  gap: 6px;
}

.op-icon {
  font-size: 16px;
}

.op-name {
  font-weight: 700;
  font-size: 13px;
  color: #1e293b;
}

.op-impact-desc {
  font-size: 11px;
  color: #64748b;
  line-height: 1.4;
  margin-bottom: 8px;
}

.op-target-tag {
  display: flex;
  justify-content: flex-end;
}

.impact-scope {
  font-size: 11px;
  color: #3b82f6;
  background: #eff6ff;
  padding: 2px 6px;
  border-radius: 4px;
}

.test-feedback-card {
  margin-top: 16px;
  padding: 12px;
  border-radius: 6px;
  border: 1px solid #e2e8f0;
}

.test-feedback-card.is-success {
  background: #f0fdf4;
  border-color: #86efac;
}

.test-feedback-card.is-error {
  background: #fef2f2;
  border-color: #fca5a5;
}

.feedback-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}

.feedback-title {
  font-weight: 600;
  font-size: 13px;
  color: #1e293b;
}

.feedback-body {
  font-size: 12px;
  color: #475569;
  line-height: 1.5;
}

.feedback-body.error-msg {
  color: #dc2626;
  font-family: monospace;
}

/* 专属固定审批流样式 */
.pinned-flow-content {
  background: #f8fafc;
  padding: 14px;
  border-radius: 6px;
  border: 1px solid #e2e8f0;
}

.custom-radio-group {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 8px;
}

.radio-title {
  font-size: 13px;
  font-weight: 600;
  color: #1e293b;
}

.pinned-title {
  color: #d97706;
}

.radio-desc {
  font-size: 11px;
  color: #64748b;
  margin-top: 2px;
}

.pinned-select-box {
  margin-top: 14px;
  padding-top: 12px;
  border-top: 1px dashed #cbd5e1;
}

.pinned-template-preview {
  background: #ffffff;
  border: 1px solid #fed7aa;
  border-radius: 6px;
  padding: 10px 12px;
  margin-top: 10px;
}

.preview-row {
  font-size: 12px;
  color: #334155;
  line-height: 1.6;
}

.pipeline-tags {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  margin-top: 4px;
}
</style>
