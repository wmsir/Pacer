<template>
  <div class="workflow-designer-container page-container">
    <!-- 顶部标题与说明 -->
    <div class="header-action">
      <div class="title-area">
        <h2 class="page-title">审批流设计与模板中心 (Workflow Designer & Templates)</h2>
        <div class="page-subtitle">定义业务线审批流模板、绑定业务资源组（车险/销管等）并配置多级审批角色流转规则</div>
      </div>
    </div>

    <el-tabs v-model="activeTab" class="custom-tabs">
      <!-- Tab 1: 审批流模板与资源组绑定中心 -->
      <el-tab-pane label="审批流模板与资源组配置" name="templates">
        <!-- 顶部过滤与新建 -->
        <div class="template-header-bar">
          <el-input
            v-model="searchKeyword"
            placeholder="搜索模板名称/类型/关联资源组..."
            clearable
            style="width: 320px;"
            :prefix-icon="Search"
            @clear="fetchTemplates"
            @keyup.enter="fetchTemplates"
          />
          <div class="header-btns">
            <el-button :icon="Refresh" :loading="loading" @click="fetchTemplates">刷新</el-button>
            <el-button type="primary" :icon="Plus" @click="handleOpenCreateModal">新建审批流模板</el-button>
          </div>
        </div>

        <!-- 模板列表表格 -->
        <div class="table-wrapper">
          <el-table :data="templateList" border stripe style="width: 100%" v-loading="loading">
            <el-table-column prop="id" label="ID" width="60" align="center" />

            <el-table-column prop="templateName" label="审批流模板名称" min-width="190">
              <template #default="scope">
                <div style="display: flex; align-items: center; gap: 8px;">
                  <el-icon color="#409EFF"><Share /></el-icon>
                  <span style="font-weight: 600; color: #303133;">{{ scope.row.templateName }}</span>
                </div>
              </template>
            </el-table-column>

            <el-table-column prop="flowType" label="适用变更类型" width="130" align="center">
              <template #default="scope">
                <el-tag size="small" :type="getTypeTag(scope.row.flowType)">
                  {{ formatFlowType(scope.row.flowType) }}
                </el-tag>
              </template>
            </el-table-column>

            <el-table-column label="关联生效的业务资源组" min-width="240">
              <template #default="scope">
                <div class="resource-tags-wrap">
                  <el-tag
                    v-for="rg in parseResourceGroups(scope.row.resourceGroups)"
                    :key="rg"
                    size="small"
                    type="warning"
                    effect="plain"
                    class="rg-tag"
                  >
                    {{ rg }}
                  </el-tag>
                </div>
              </template>
            </el-table-column>

            <el-table-column label="审批流转链路 (节点与网关)" min-width="320">
              <template #default="scope">
                <div v-if="isConditionTemplate(scope.row)" class="conditional-chain-box">
                  <div style="display: flex; align-items: center; gap: 6px; margin-bottom: 4px; flex-wrap: wrap;">
                    <el-tag type="danger" effect="dark" size="small">
                      <el-icon style="margin-right: 2px;"><Share /></el-icon>
                      智能排他网关 ({{ scope.row.conditionDimension === 'CHANGE_TYPE' ? 'DDL变更分流' : (scope.row.conditionDimension === 'COMPOSITE' ? '复合分流' : '行数分流') }})
                    </el-tag>
                    <el-tag v-if="scope.row.spelExpression" size="small" type="warning" effect="plain" style="font-family: monospace;">
                      SpEL: {{ scope.row.spelExpression }}
                    </el-tag>
                  </div>
                  <div class="branch-summary-rows">
                    <div class="branch-row">
                      <span class="branch-tag red">
                        {{ scope.row.conditionDimension === 'CHANGE_TYPE' ? '含 DDL 结构变更' : (scope.row.conditionDimension === 'COMPOSITE' ? `&gt; ${scope.row.affectRowsThreshold || 1000}行 或 DDL` : `&gt; ${scope.row.affectRowsThreshold || 1000} 行`) }}
                      </span>
                      ➔ {{ scope.row.highRiskRole === 'ADMIN' ? '超级管理员终审 (ADMIN)' : (scope.row.highRiskRole === 'AUDITOR' ? '安全审计核查 (AUDITOR)' : '核心DBA安全复核 (DBA)') }}
                    </div>
                    <div class="branch-row">
                      <span class="branch-tag green">
                        {{ scope.row.conditionDimension === 'CHANGE_TYPE' ? '常规 DML 纯数据' : `≤ ${scope.row.affectRowsThreshold || 1000} 行` }}
                      </span>
                      ➔ {{ scope.row.lowRiskRole === 'OPS' ? '业务运维初审 (OPS)' : '开发组长初审 (DEV_LEAD)' }}
                    </div>
                  </div>
                </div>
                <div v-else class="node-chain-wrap">
                  <span
                    v-for="(node, idx) in parseNodeConfig(scope.row.nodeConfig)"
                    :key="idx"
                    class="node-step-tag"
                  >
                    <span class="step-num">{{ idx + 1 }}</span>
                    <span class="step-name">{{ node.nodeName }}</span>
                    <span class="step-role">({{ node.role }})</span>
                    <el-icon v-if="idx < parseNodeConfig(scope.row.nodeConfig).length - 1" class="step-arrow"><Right /></el-icon>
                  </span>
                </div>
              </template>
            </el-table-column>

            <el-table-column label="支持的执行策略 (多选)" min-width="240">
              <template #default="scope">
                <div class="strategy-tags-group">
                  <el-tooltip
                    v-for="mode in parseExecModes(scope.row.defaultExecutionMode)"
                    :key="mode.key"
                    :content="`${mode.label}：${mode.desc}`"
                    placement="top"
                  >
                    <el-tag
                      size="small"
                      :type="mode.tagType"
                      effect="light"
                      class="strategy-tag"
                    >
                      <span class="strategy-icon">{{ mode.icon }}</span>
                      <span>{{ mode.label }}</span>
                    </el-tag>
                  </el-tooltip>
                </div>
              </template>
            </el-table-column>

            <el-table-column prop="status" label="状态" width="90" align="center">
              <template #default="scope">
                <el-tag :type="scope.row.status === 1 ? 'success' : 'info'">
                  {{ scope.row.status === 1 ? '启用' : '停用' }}
                </el-tag>
              </template>
            </el-table-column>

            <el-table-column label="操作" width="220" fixed="right" align="center">
              <template #default="scope">
                <el-button size="small" link type="primary" :icon="View" @click="viewTemplateInBpmn(scope.row)">
                  BPMN 拓扑
                </el-button>
                <el-button size="small" :icon="Edit" @click="handleOpenEditModal(scope.row)">编辑</el-button>
                <el-button size="small" type="danger" plain :icon="Delete" @click="handleDeleteTemplate(scope.row)">删除</el-button>
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

      <!-- Tab 2: BPMN 可视化图形设计器 -->
      <el-tab-pane label="BPMN 可视化图形设计器" name="bpmn">
        <div class="bpmn-designer-panel" :class="{ 'is-fullscreen': isBpmnFullscreen }">
          
          <!-- 顶部 3 步编排引导向导条 -->
          <div class="bpmn-wizard-steps-bar">
            <div class="wizard-step" :class="{ 'is-active': currentStepIndex === 1, 'is-finish': currentStepIndex > 1 }">
              <span class="step-badge">1</span>
              <div class="step-texts">
                <span class="step-main">① 选定生效目标</span>
                <span class="step-sub">左侧选择资源组与目标数据库</span>
              </div>
            </div>
            <div class="wizard-arrow">➔</div>
            <div class="wizard-step" :class="{ 'is-active': currentStepIndex === 2, 'is-finish': currentStepIndex > 2 }">
              <span class="step-badge">2</span>
              <div class="step-texts">
                <span class="step-main">② 编排审批链路</span>
                <span class="step-sub">拖拽/添加角色组或智能排他网关</span>
              </div>
            </div>
            <div class="wizard-arrow">➔</div>
            <div class="wizard-step" :class="{ 'is-active': currentStepIndex === 3, 'is-finish': currentDeployStatus.isDeployed }">
              <span class="step-badge">3</span>
              <div class="step-texts">
                <span class="step-main">③ 部署生效上线</span>
                <span class="step-sub">一键下发至 Flowable 引擎</span>
              </div>
            </div>
          </div>

          <div class="bpmn-toolbar">
            <div class="toolbar-left">
              <span class="label">流程拓扑模型：</span>
              <el-select
                v-model="selectedBpmnPreset"
                placeholder="选择流程图形拓扑"
                style="width: 360px;"
                @change="handleBpmnPresetChange"
              >
                <el-option-group label="📌 数据库已发布的审批流模板 (与配置实时同步)">
                  <el-option
                    v-for="t in templateList"
                    :key="'tpl_' + t.id"
                    :label="`[#${t.id}] ${t.templateName} (${t.affectRowsThreshold || 1000}行 · ${t.spelExpression || '#{affectRows > 1000}'})`"
                    :value="'tpl_' + t.id"
                  />
                </el-option-group>
                <el-option-group label="📐 BPMN 经典标准拓扑范式">
                  <el-option label="🌟 DML 影响行数与SpEL智能条件分支 (排他网关)" value="dml_condition" />
                  <el-option label="标准生产 SQL 变更审批拓扑 (开发组长 ➔ 核心DBA)" value="dml_standard" />
                  <el-option label="高危 DDL 变更三级审批拓扑 (AI ➔ 资深DBA ➔ 安全)" value="ddl" />
                  <el-option label="生产紧急抢修极速放行拓扑 (值班DBA ➔ 补录审计)" value="emergency" />
                  <el-option label="敏感数据导出与脱敏审批拓扑 (业务主管 ➔ 数据安全官)" value="export" />
                </el-option-group>
              </el-select>

              <!-- 部署状态标签 -->
              <div class="deploy-status-badge">
                <el-tag
                  v-if="currentDeployStatus.isDeployed"
                  type="success"
                  effect="dark"
                  size="default"
                  style="display: flex; align-items: center; gap: 4px;"
                >
                  <el-icon><CircleCheckFilled /></el-icon>
                  <span>Flowable 已挂载 (部署生效中 · {{ currentDeployStatus.deploymentId }})</span>
                </el-tag>
                <el-tag
                  v-else
                  type="info"
                  effect="plain"
                  size="default"
                  style="display: flex; align-items: center; gap: 4px;"
                >
                  <el-icon><InfoFilled /></el-icon>
                  <span>Flowable 未挂载 (草稿态 / 可随时部署)</span>
                </el-tag>
              </div>

              <!-- 当前绑定的数据库生效范围徽标 -->
              <div class="deploy-status-badge">
                <el-tag
                  type="warning"
                  effect="light"
                  size="default"
                  style="display: flex; align-items: center; gap: 4px;"
                >
                  <el-icon><Coin /></el-icon>
                  <span><b>生效资源组与目标库：</b>{{ currentTemplateDbScopeText }}</span>
                </el-tag>
              </div>

              <!-- 生效工单类型徽标 -->
              <div class="deploy-status-badge">
                <el-tag
                  type="success"
                  effect="light"
                  size="default"
                  style="display: flex; align-items: center; gap: 4px;"
                >
                  <el-icon><Operation /></el-icon>
                  <span><b>生效工单类型：</b>{{ currentTemplateFlowTypeText }}</span>
                </el-tag>
              </div>
            </div>

            <div class="toolbar-right">
              <!-- 全屏与还原按钮 -->
              <el-button
                :type="isBpmnFullscreen ? 'warning' : 'default'"
                :icon="isBpmnFullscreen ? Aim : FullScreen"
                @click="toggleBpmnFullscreen"
              >
                {{ isBpmnFullscreen ? '还原窗口' : '全屏模式' }}
              </el-button>

              <el-button :icon="Connection" type="success" plain @click="handleOpenQuickBindDialog">
                🎯 绑定生效范围
              </el-button>
              <el-button :icon="CopyDocument" @click="handleCopyXml">复制 XML</el-button>
              <el-button :icon="Download" @click="handleExportXml">导出 BPMN</el-button>
              <el-button :icon="RefreshRight" @click="handleResetCanvas">重置画布</el-button>

              <!-- 终止/卸载 流程定义 -->
              <el-button
                v-if="currentDeployStatus.isDeployed"
                type="danger"
                plain
                :icon="VideoPause"
                :loading="terminateLoading"
                @click="terminateProcess"
              >
                终止并卸载流程
              </el-button>
              <!-- 部署流程定义 -->
              <el-button
                v-else
                type="primary"
                :icon="Upload"
                :loading="deployLoading"
                @click="deployProcess"
              >
                部署流程定义
              </el-button>
            </div>
          </div>

          <div class="bpmn-designer-body-layout">
            <!-- 左侧：权限组与审批角色资产面板 (拖拽 ➕ 插入 BPMN 节点) -->
            <div class="bpmn-palette-panel">
              <div class="palette-panel-header">
                <div class="p-title-row">
                  <el-icon color="#409EFF"><UserFilled /></el-icon>
                  <span class="p-title">审批角色与引擎资产库</span>
                </div>
                <div class="p-sub-tip">拖拽 ➕ 插入 BPMN 流程节点</div>
              </div>

              <!-- 搜索过滤 -->
              <div class="palette-search-box">
                <el-input
                  v-model="paletteSearchKeyword"
                  size="small"
                  placeholder="搜索审批角色/人员/网关/节点..."
                  clearable
                  :prefix-icon="Search"
                />
              </div>

              <div class="palette-scroll-content">
                <!-- 1. 系统审批角色权限组 (支持展示成员与编辑) -->
                <div class="palette-group">
                  <div class="group-header">
                    <el-icon color="#409EFF"><User /></el-icon>
                    <span>系统审批角色权限组</span>
                    <el-tag size="small" type="primary" effect="plain" class="count-tag">{{ filteredPaletteRoles.length }}</el-tag>
                  </div>
                  <div class="group-items-list">
                    <div
                      v-for="r in filteredPaletteRoles"
                      :key="r.key"
                      class="palette-card"
                      :class="`palette-card-${r.key.toLowerCase()}`"
                      draggable="true"
                      @dragstart="handleDragStart($event, { type: 'ROLE', name: r.label, role: r.key })"
                    >
                      <div class="card-info">
                        <div class="card-title-row" style="display: flex; align-items: center; justify-content: space-between;">
                          <span class="card-name">{{ r.label }}</span>
                        </div>
                        <div class="card-members-summary" style="display: flex; align-items: center; gap: 6px; margin-top: 2px;">
                          <el-tag size="small" type="info" effect="plain" style="font-size: 10px; height: 18px; padding: 0 4px;">
                            👥 {{ getRoleMembers(r.key).length }} 人
                          </el-tag>
                          <span class="member-names-preview" :title="getRoleMemberNames(r.key)" style="font-size: 11px; color: #64748b; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 130px;">
                            {{ getRoleMemberNames(r.key) || '暂无成员' }}
                          </span>
                        </div>
                      </div>
                      <div class="card-actions-row" style="display: flex; gap: 2px; align-items: center;">
                        <el-button
                          size="small"
                          type="success"
                          link
                          title="编辑该角色组成员"
                          @click.stop="handleOpenRoleMemberEditor(r)"
                        >
                          <el-icon><Edit /></el-icon>
                        </el-button>
                        <el-button
                          size="small"
                          type="primary"
                          link
                          :icon="Plus"
                          title="添加到画布流程"
                          @click="addNodeToCanvas({ type: 'ROLE', name: r.label, role: r.key })"
                        />
                      </div>
                    </div>
                  </div>
                </div>

                <!-- 3. BPMN 常用网关与服务节点 -->
                <div class="palette-group">
                  <div class="group-header">
                    <el-icon color="#67C23A"><Operation /></el-icon>
                    <span>条件网关与服务引擎</span>
                  </div>
                  <div class="group-items-list">
                    <div
                      class="palette-card palette-card-gateway"
                      draggable="true"
                      @dragstart="handleDragStart($event, { type: 'GATEWAY', name: 'SpEL 排他智能网关', role: 'GATEWAY' })"
                    >
                      <div class="card-info">
                        <span class="card-name">◇ SpEL 排他智能网关</span>
                        <span class="card-sub">影响行数 / DDL 条件分流</span>
                      </div>
                      <el-button
                        size="small"
                        type="primary"
                        link
                        :icon="Plus"
                        title="添加到画布流程"
                        @click="addNodeToCanvas({ type: 'GATEWAY', name: 'SpEL 排他智能网关', role: 'GATEWAY' })"
                      />
                    </div>

                    <div
                      class="palette-card palette-card-service"
                      draggable="true"
                      @dragstart="handleDragStart($event, { type: 'SERVICE', name: 'JDBC 安全流式执行', role: 'SERVICE' })"
                    >
                      <div class="card-info">
                        <span class="card-name">⚙️ JDBC 安全流式执行</span>
                        <span class="card-sub">自动化流式引擎事务执行</span>
                      </div>
                      <el-button
                        size="small"
                        type="primary"
                        link
                        :icon="Plus"
                        title="添加到画布流程"
                        @click="addNodeToCanvas({ type: 'SERVICE', name: 'JDBC 安全流式执行', role: 'SERVICE' })"
                      />
                    </div>
                  </div>
                </div>
              </div>
            </div>

            <!-- 右侧：BPMN 画布与属性编辑工作区 (全屏及普通模式均可无遮挡绘制) -->
            <div
              class="canvas-wrapper"
              @dragover.prevent
              @drop="handleCanvasDrop"
            >
              <div ref="canvas" class="canvas"></div>
            </div>
          </div>
        </div>
      </el-tab-pane>
    </el-tabs>

    <!-- 新建/编辑审批流模板弹窗 -->
    <el-dialog
      :title="isEdit ? '编辑审批流模板与资源组绑定' : '新建审批流模板'"
      v-model="modalVisible"
      width="780px"
      top="4vh"
      destroy-on-close
      append-to-body
    >
      <div class="dialog-scroll-form">
        <el-form ref="formRef" :model="form" :rules="formRules" label-width="130px">
        <el-form-item label="审批流名称" prop="templateName">
          <el-input v-model="form.templateName" placeholder="如：车险核心库 DML 变更审批流" />
        </el-form-item>

        <el-form-item label="关联 BPMN 流程">
          <el-select v-model="form.bpmnProcessKey" placeholder="选择关联的 BPMN 2.0 流程模型" style="width: 100%;" @change="handleBpmnKeyChange">
            <el-option label="Process_StandardSqlReview (标准生产 SQL 变更审批流)" value="Process_StandardSqlReview" />
            <el-option label="Process_SensitiveDdlReview (高危 DDL 结构变更双人复核流程)" value="Process_SensitiveDdlReview" />
            <el-option label="Process_EmergencyChange (生产紧急变更极速放行通道)" value="Process_EmergencyChange" />
            <el-option label="Process_DataExportReview (敏感数据导出与脱敏审批流)" value="Process_DataExportReview" />
          </el-select>
        </el-form-item>

        <el-form-item label="适用变更类型" prop="flowType">
          <el-select v-model="form.flowType" style="width: 100%;">
            <el-option label="DML 常规数据变更 (UPDATE/INSERT/DELETE)" value="DML_CHANGE" />
            <el-option label="DDL 高危表结构变更 (CREATE/ALTER/DROP)" value="DDL_CHANGE" />
            <el-option label="SQL 综合安全变更" value="SQL_AUDIT" />
            <el-option label="敏感数据导出申请" value="DATA_EXPORT" />
            <el-option label="只读数据查询申请" value="DATA_QUERY" />
          </el-select>
        </el-form-item>

        <!-- 资源组关联多选 (支持可搜索与一键全选 / 一键全取消) -->
        <el-form-item label="关联业务资源组" prop="selectedResourceGroups">
          <div class="rg-quick-actions-bar">
            <el-button size="small" type="primary" plain :icon="Select" @click="handleSelectAllResourceGroups">
              一键全选所有资源组 ({{ availableResourceGroups.length }})
            </el-button>
            <el-button size="small" type="danger" plain :icon="CloseBold" @click="handleClearAllResourceGroups">
              全部取消 / 清空
            </el-button>
            <el-button size="small" type="success" plain @click="handleSelectUniversalResourceGroup">
              设为全部通用
            </el-button>
          </div>
          <el-select
            v-model="form.selectedResourceGroups"
            multiple
            filterable
            clearable
            collapse-tags
            collapse-tags-tooltip
            placeholder="请选择适用的业务资源组（支持快速搜索与多选）"
            style="width: 100%;"
          >
            <el-option
              v-for="rg in availableResourceGroups"
              :key="rg"
              :label="rg"
              :value="rg"
            />
          </el-select>
        </el-form-item>

        <!-- 生效数据库范围 (支持整个资源组全部通用 vs 指定具体数据库) -->
        <el-form-item label="生效数据库范围">
          <div class="db-scope-wrapper" style="width: 100%;">
            <el-radio-group v-model="form.dbScopeMode" size="small" @change="handleDbScopeModeChange">
              <el-radio-button value="ALL">🌐 整个资源组全部数据库通用 (默认)</el-radio-button>
              <el-radio-button value="CUSTOM">🎯 细化指定具体目标数据库</el-radio-button>
            </el-radio-group>

            <div v-if="form.dbScopeMode === 'CUSTOM'" class="custom-db-select-box" style="margin-top: 10px;">
              <el-select
                v-model="form.selectedTargetDatabases"
                multiple
                filterable
                clearable
                collapse-tags
                collapse-tags-tooltip
                placeholder="请选择具体生效的目标数据库 (支持多选与搜索)"
                style="width: 100%;"
              >
                <el-option-group
                  v-for="inst in availableInstancesList"
                  :key="inst.id"
                  :label="`${inst.name} (${inst.dbType || 'mysql'} · ${inst.env || 'PROD'})`"
                >
                  <el-option
                    v-for="db in (inst.databases || [inst.name + '_db'])"
                    :key="`${inst.name}/${db}`"
                    :label="`${inst.name} / ${db}`"
                    :value="`${inst.name}/${db}`"
                  />
                </el-option-group>
              </el-select>
              <div class="field-hint" style="margin-top: 4px; font-size: 12px; color: #64748b;">
                💡 仅当申请工单选择上述指定数据库时才触发匹配此审批流程
              </div>
            </div>
          </div>
        </el-form-item>

        <!-- 审批流转模式与条件排他网关设置 -->
        <el-form-item label="审批流转模式">
          <el-radio-group v-model="form.isConditionGateway" @change="handleGatewayModeChange">
            <el-radio :value="false">标准顺序多级审批 (静态多级链路)</el-radio>
            <el-radio :value="true">智能排他网关 (条件动态智能分流)</el-radio>
          </el-radio-group>
        </el-form-item>

        <!-- 条件网关规则配置面板 (支持按影响行数/DDL变更/复合规则定制) -->
        <el-form-item label="排他网关分支配置" v-if="form.isConditionGateway">
          <div class="gateway-config-card">
            <!-- 1. 判定维度选择 -->
            <div class="gateway-dimension-row">
              <span class="cfg-sub-label">判定维度：</span>
              <el-radio-group v-model="form.conditionDimension" size="small" @change="handleDimensionChange">
                <el-radio-button value="AFFECT_ROWS">纯 DML 影响行数判定</el-radio-button>
                <el-radio-button value="CHANGE_TYPE">变更类型安全判定 (含DDL)</el-radio-button>
                <el-radio-button value="COMPOSITE">复合智能判定 (行数或DDL)</el-radio-button>
              </el-radio-group>
            </div>

            <!-- 2. 影响行数阈值设置 -->
            <div v-if="form.conditionDimension !== 'CHANGE_TYPE'" class="gateway-threshold-row">
              <span class="cfg-sub-label">影响行数安全阈值：</span>
              <el-input-number
                v-model="form.affectRowsThreshold"
                :min="1"
                :max="1000000"
                :step="100"
                size="small"
                style="width: 140px;"
                @change="handleThresholdChange"
              />
              <span class="unit-text">行</span>
              <div class="quick-preset-chips">
                <el-tag size="small" effect="plain" class="preset-chip" @click="setThreshold(500)">500行</el-tag>
                <el-tag size="small" effect="plain" class="preset-chip" @click="setThreshold(1000)">1000行 (标准)</el-tag>
                <el-tag size="small" effect="plain" class="preset-chip" @click="setThreshold(2000)">2000行 (大表)</el-tag>
                <el-tag size="small" effect="plain" class="preset-chip" @click="setThreshold(5000)">5000行</el-tag>
                <el-tag size="small" effect="plain" class="preset-chip" @click="setThreshold(10000)">10000行</el-tag>
              </div>
            </div>

            <!-- 3. SpEL 表达式配置区 (现成预设与自由编辑) -->
            <div class="gateway-spel-section" style="margin-top: 10px; padding: 10px 12px; background: #f8fafc; border: 1px solid #e2e8f0; border-radius: 6px;">
              <div class="spel-header-row" style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 6px;">
                <div style="display: flex; align-items: center; gap: 6px;">
                  <span class="cfg-sub-label" style="font-weight: 600; color: #334155; font-size: 13px;">📌 SpEL 条件表达式：</span>
                  <el-tag size="small" type="warning" effect="dark">BPMN 2.0 / Spring Expression Language</el-tag>
                </div>
                <span style="font-size: 12px; color: #64748b;">支持动态同步至可视化流程图网关与分支链路</span>
              </div>

              <!-- 现成预设选择 -->
              <div class="quick-preset-spel-chips" style="display: flex; flex-wrap: wrap; gap: 6px; margin-bottom: 8px;">
                <el-tag
                  v-for="preset in SPEL_PRESETS"
                  :key="preset.expr"
                  size="small"
                  :effect="form.spelExpression === preset.expr ? 'dark' : 'plain'"
                  :type="form.spelExpression === preset.expr ? 'primary' : 'info'"
                  class="spel-preset-chip"
                  style="cursor: pointer;"
                  @click="applySpelPreset(preset)"
                >
                  {{ preset.label }} (<code>{{ preset.expr }}</code>)
                </el-tag>
              </div>

              <div style="display: flex; gap: 8px; align-items: center;">
                <el-input
                  v-model="form.spelExpression"
                  placeholder="例如：#{affectRows > 2000 || hasDdl == true}"
                  size="small"
                  @input="handleCustomSpelInput"
                >
                  <template #prepend>当前生效 SpEL</template>
                </el-input>
                <el-button type="warning" size="small" :icon="Aim" @click="handleOpenSpelSandbox">
                  🧪 沙箱在线求值
                </el-button>
              </div>
            </div>

            <!-- 4. 左右分支规则与审批流向明确配置 -->
            <div class="branch-split-box" style="margin-top: 10px;">
              <!-- 🔥 左分支：高危管控 (满足 SpEL = true) -->
              <div class="branch-config-item red-branch">
                <div class="branch-header">
                  <el-tag type="danger" effect="dark" size="small">🔥 左分支 · 高危大事务/DDL管控 (满足 SpEL = true)</el-tag>
                  <span class="branch-cond-desc">
                    判定规则：<b>当工单评估匹配 SpEL <code>{{ form.spelExpression || '#{affectRows > 1000}' }}</code> 时自动进入此分支</b>
                  </span>
                </div>
                <div class="branch-route-detail">
                  <span class="route-role-label">审批流转角色：</span>
                  <el-select v-model="form.highRiskRole" size="small" style="width: 260px;" @change="updateTriggerConditionFromGateway">
                    <el-option
                      v-for="r in paletteRoles"
                      :key="r.key"
                      :label="`${r.roleName} (${r.key})`"
                      :value="r.key"
                    />
                  </el-select>
                </div>
              </div>

              <!-- ⚡ 右分支：常规放行 (不满足 SpEL / Default 默认流) -->
              <div class="branch-config-item green-branch" style="margin-top: 8px;">
                <div class="branch-header">
                  <el-tag type="success" effect="dark" size="small">⚡ 右分支 · 常规低危快速放行 (默认流 / SpEL = false)</el-tag>
                  <span class="branch-cond-desc">
                    判定规则：<b>当不满足高危 SpEL（常规安全放行）时由默认流快速审批</b>
                  </span>
                </div>
                <div class="branch-route-detail">
                  <span class="route-role-label">审批流转角色：</span>
                  <el-select v-model="form.lowRiskRole" size="small" style="width: 260px;" @change="updateTriggerConditionFromGateway">
                    <el-option
                      v-for="r in paletteRoles"
                      :key="r.key"
                      :label="`${r.roleName} (${r.key})`"
                      :value="r.key"
                    />
                  </el-select>
                </div>
              </div>
            </div>
          </div>
        </el-form-item>

        <!-- 顺序节点编辑器 (当非条件网关时可用) -->
        <el-form-item label="审批节点流转链路" v-if="!form.isConditionGateway">
          <div class="node-editor-box">
            <div
              v-for="(node, index) in form.nodes"
              :key="index"
              class="node-editor-row"
            >
              <span class="node-seq">第 {{ index + 1 }} 级：</span>
              <el-input
                v-model="node.nodeName"
                placeholder="节点名称 (如: 开发组长初审)"
                style="width: 160px; margin-right: 8px;"
              />
              <el-select v-model="node.role" placeholder="审批角色" style="width: 170px; margin-right: 8px;">
                <el-option
                  v-for="r in paletteRoles"
                  :key="r.key"
                  :label="`${r.roleName} (${r.key})`"
                  :value="r.key"
                />
                <el-option label="SYSTEM (系统自动审批)" value="SYSTEM" />
              </el-select>
              <el-select v-model="node.approvalMode" placeholder="审批模式" style="width: 140px; margin-right: 8px;">
                <el-option label="或签 (任一通过)" value="ORSIGN" />
                <el-option label="会签 (全票通过)" value="COUNTERSIGN" />
                <el-option label="顺签 (标准审批)" value="SEQUENTIAL" />
              </el-select>
              <el-button
                type="danger"
                link
                :icon="Delete"
                :disabled="form.nodes.length <= 1"
                @click="removeNode(index)"
              />
            </div>

            <el-button
              type="primary"
              plain
              size="small"
              :icon="Plus"
              style="margin-top: 8px;"
              @click="addNode"
            >
              添加下一级审批节点
            </el-button>
          </div>
        </el-form-item>

        <el-form-item label="支持执行策略 (多选)">
          <div class="strategy-cards-container">
            <div
              v-for="opt in EXEC_STRATEGY_OPTIONS"
              :key="opt.key"
              class="strategy-card"
              :class="{ selected: form.selectedExecModes.includes(opt.key) }"
              @click="toggleExecMode(opt.key)"
            >
              <div class="card-top-row">
                <el-checkbox
                  :model-value="form.selectedExecModes.includes(opt.key)"
                  @click.stop="toggleExecMode(opt.key)"
                />
                <span class="card-icon">{{ opt.icon }}</span>
                <span class="card-title">{{ opt.label }}</span>
                <span
                  class="card-badge"
                  :style="{ background: opt.color + '18', color: opt.color, border: '1px solid ' + opt.color + '33' }"
                >
                  {{ opt.badge }}
                </span>
              </div>
              <div class="card-desc">{{ opt.desc }}</div>
            </div>
          </div>
        </el-form-item>

        <el-form-item label="触发条件说明">
          <el-input v-model="form.triggerCondition" placeholder="如：影响行数 ≤ 1000 行且无删表语法" />
        </el-form-item>

        <el-form-item label="业务用途描述">
          <el-input v-model="form.description" type="textarea" :rows="2" placeholder="说明该审批流的适用范围与合规要求" />
        </el-form-item>

        <el-form-item label="模板状态">
          <el-radio-group v-model="form.status">
            <el-radio :value="1">正常启用</el-radio>
            <el-radio :value="0">停用</el-radio>
          </el-radio-group>
        </el-form-item>
      </el-form>
      </div>

      <template #footer>
        <el-button @click="modalVisible = false">取消</el-button>
        <el-button type="primary" :loading="saveLoading" @click="handleSaveTemplate">
          {{ isEdit ? '保存修改' : '立即创建' }}
        </el-button>
      </template>
    </el-dialog>

    <!-- 绑定当前 BPMN 审批流程生效范围（工单类型 + 业务资源组 + 全部/细化具体数据库） -->
    <el-dialog
      title="🎯 绑定当前 BPMN 审批流程生效范围"
      v-model="quickBindDialogVisible"
      width="680px"
      destroy-on-close
      append-to-body
    >
      <div style="margin-bottom: 12px; font-size: 13px; color: #606266; background: #f0fdf4; border: 1px solid #bbf7d0; border-radius: 6px; padding: 10px 14px;">
        <div style="font-weight: 600; color: #166534; margin-bottom: 4px;">
          当前流程：{{ getPresetTitle(selectedBpmnPreset) }}
        </div>
        <div style="font-size: 12px; color: #15803d; line-height: 1.5;">
          💡 <b>生效范围说明</b>：系统将根据此流程绑定的【工单类型】、【业务资源组】与【目标数据库】进行精确匹配，在创建工单时将直接展示对应的审批链路！
        </div>
      </div>

      <el-form label-width="120px" size="default">
        <!-- 1. 生效工单类型 (全面对齐创建工单大类与细分类型) -->
        <el-form-item label="生效工单类型">
          <div style="display: flex; flex-wrap: wrap; gap: 6px; margin-bottom: 8px; width: 100%;">
            <el-button size="small" type="primary" plain :icon="Select" @click="handleSelectAllQuickBindTicketTypes">
              🌐 一键全选所有类型 (全部通用)
            </el-button>
            <el-button size="small" type="info" plain @click="quickBindTicketTypes = ['SQL_AUDIT', 'DML_CHANGE', 'DDL_CHANGE']">
              仅全量 SQL 变更
            </el-button>
            <el-button size="small" type="info" plain @click="quickBindTicketTypes = ['DML_CHANGE']">
              细分：仅 DML 变更
            </el-button>
            <el-button size="small" type="info" plain @click="quickBindTicketTypes = ['DDL_CHANGE']">
              细分：仅 DDL 变更
            </el-button>
            <el-button size="small" type="info" plain @click="quickBindTicketTypes = ['DATA_EXPORT']">
              仅敏感数据导出
            </el-button>
            <el-button size="small" type="info" plain @click="quickBindTicketTypes = ['PERMISSION']">
              仅权限申请
            </el-button>
            <el-button size="small" type="info" plain @click="quickBindTicketTypes = ['DATA_RECOVERY']">
              仅应急数据修复
            </el-button>
            <el-button size="small" type="danger" plain :icon="CloseBold" @click="quickBindTicketTypes = []">
              清空
            </el-button>
          </div>

          <div style="width: 100%; border: 1px solid #e2e8f0; border-radius: 8px; padding: 12px; background: #fafbfc;">
            <!-- 大类 1：SQL 变更审核 (含全量与细化分支) -->
            <div style="margin-bottom: 12px;">
              <div style="font-size: 12px; font-weight: 700; color: #1e293b; margin-bottom: 6px; display: flex; align-items: center; gap: 6px;">
                <span>🛠️ 1. SQL 变更审核 (支持全量通用与 DML/DDL 细化定制)：</span>
              </div>
              <el-checkbox-group v-model="quickBindTicketTypes" style="display: flex; flex-wrap: wrap; gap: 10px; margin-left: 8px;">
                <el-checkbox value="SQL_AUDIT">
                  <el-tag size="small" type="primary" effect="dark">SQL_AUDIT：全量 SQL 变更通用 (DML与DDL均适用)</el-tag>
                </el-checkbox>
                <el-checkbox value="DML_CHANGE">
                  <el-tag size="small" type="success" effect="dark">DML_CHANGE：细化纯数据变更 (INSERT/UPDATE/DELETE)</el-tag>
                </el-checkbox>
                <el-checkbox value="DDL_CHANGE">
                  <el-tag size="small" type="danger" effect="dark">DDL_CHANGE：细化库表结构变更 (CREATE/ALTER/DROP)</el-tag>
                </el-checkbox>
              </el-checkbox-group>
            </div>

            <!-- 大类 2：敏感数据导出 -->
            <div style="margin-bottom: 12px; border-top: 1px dashed #e2e8f0; padding-top: 8px;">
              <div style="font-size: 12px; font-weight: 700; color: #1e293b; margin-bottom: 6px; display: flex; align-items: center; gap: 6px;">
                <span>📤 2. 敏感数据导出申请：</span>
              </div>
              <el-checkbox-group v-model="quickBindTicketTypes" style="display: flex; flex-wrap: wrap; gap: 10px; margin-left: 8px;">
                <el-checkbox value="DATA_EXPORT">
                  <el-tag size="small" type="warning" effect="dark">DATA_EXPORT：敏感数据导出与脱敏申请</el-tag>
                </el-checkbox>
              </el-checkbox-group>
            </div>

            <!-- 大类 3：权限与账号申请 -->
            <div style="margin-bottom: 12px; border-top: 1px dashed #e2e8f0; padding-top: 8px;">
              <div style="font-size: 12px; font-weight: 700; color: #1e293b; margin-bottom: 6px; display: flex; align-items: center; gap: 6px;">
                <span>🔑 3. 权限与账号申请：</span>
              </div>
              <el-checkbox-group v-model="quickBindTicketTypes" style="display: flex; flex-wrap: wrap; gap: 10px; margin-left: 8px;">
                <el-checkbox value="PERMISSION">
                  <el-tag size="small" type="info" effect="dark">PERMISSION：数据库权限开通与临时查询提权</el-tag>
                </el-checkbox>
              </el-checkbox-group>
            </div>

            <!-- 大类 4：应急数据修复与恢复 -->
            <div style="border-top: 1px dashed #e2e8f0; padding-top: 8px;">
              <div style="font-size: 12px; font-weight: 700; color: #1e293b; margin-bottom: 6px; display: flex; align-items: center; gap: 6px;">
                <span>🚨 4. 应急数据修复与恢复：</span>
              </div>
              <el-checkbox-group v-model="quickBindTicketTypes" style="display: flex; flex-wrap: wrap; gap: 10px; margin-left: 8px;">
                <el-checkbox value="DATA_RECOVERY">
                  <el-tag size="small" type="danger" effect="plain" style="color: #b91c1c; border-color: #f87171; font-weight: 600;">DATA_RECOVERY：生产应急数据修复与故障恢复</el-tag>
                </el-checkbox>
              </el-checkbox-group>
            </div>
          </div>
        </el-form-item>

        <!-- 2. 生效业务资源组 -->
        <el-form-item label="生效业务资源组">
          <div style="display: flex; gap: 8px; margin-bottom: 8px; width: 100%;">
            <el-button size="small" type="primary" plain :icon="Select" @click="handleSelectAllQuickBindResourceGroups">
              一键全选所有资源组 ({{ availableResourceGroups.length }})
            </el-button>
            <el-button size="small" type="danger" plain :icon="CloseBold" @click="handleClearAllQuickBindResourceGroups">
              全部取消 / 清空
            </el-button>
          </div>
          <el-select
            v-model="quickBindResourceGroups"
            multiple
            filterable
            clearable
            collapse-tags
            collapse-tags-tooltip
            placeholder="请选择需要应用此流程的业务资源组"
            style="width: 100%;"
          >
            <el-option
              v-for="rg in availableResourceGroups"
              :key="rg"
              :label="rg"
              :value="rg"
            />
          </el-select>
        </el-form-item>

        <el-form-item label="生效数据库范围">
          <el-radio-group v-model="quickBindDbScopeMode" @change="handleQuickBindDbScopeChange">
            <el-radio value="ALL">🌐 整个资源组全部数据库通用 (默认)</el-radio>
            <el-radio value="CUSTOM">🎯 细化定制指定具体数据库 (个性化生效)</el-radio>
          </el-radio-group>

          <!-- 模式一：整个资源组通用时，自动联动展示这些资源组所包含的全部数据库预览 -->
          <div
            v-if="quickBindDbScopeMode === 'ALL'"
            style="margin-top: 10px; width: 100%; padding: 10px 12px; background: #f8fafc; border: 1px solid #e2e8f0; border-radius: 6px;"
          >
            <div style="font-size: 12px; font-weight: 600; color: #475569; margin-bottom: 6px; display: flex; align-items: center; justify-content: space-between;">
              <span>📦 所选资源组当前包含的全部数据库 (共 {{ quickBindCurrentGroupDatabases.length }} 个库)：</span>
              <el-tag size="small" type="success" effect="plain">全部通用生效</el-tag>
            </div>
            <div v-if="quickBindCurrentGroupDatabases.length > 0" style="display: flex; flex-wrap: wrap; gap: 6px; max-height: 120px; overflow-y: auto;">
              <el-tag
                v-for="db in quickBindCurrentGroupDatabases"
                :key="db"
                size="small"
                type="info"
                effect="light"
              >
                {{ db }}
              </el-tag>
            </div>
            <div v-else style="font-size: 12px; color: #94a3b8;">
              暂未勾选任何业务资源组，请先在上方选择资源组
            </div>
          </div>
        </el-form-item>

        <!-- 模式二：细化定制指定具体数据库 -->
        <el-form-item v-if="quickBindDbScopeMode === 'CUSTOM'" label="选择生效数据库">
          <div style="display: flex; gap: 8px; margin-bottom: 8px; width: 100%;">
            <el-button size="small" type="success" plain :icon="Select" @click="handleSelectAllQuickBindDatabases">
              一键全选当前资源组全部数据库 ({{ quickBindCurrentGroupDatabases.length }})
            </el-button>
            <el-button size="small" type="danger" plain :icon="CloseBold" @click="handleClearAllQuickBindDatabases">
              清空所选数据库
            </el-button>
          </div>
          <el-select
            v-model="quickBindSelectedDatabases"
            multiple
            filterable
            clearable
            collapse-tags
            collapse-tags-tooltip
            placeholder="搜索并勾选需要个性化定制此审批流的数据库 (实例/库名)"
            style="width: 100%;"
          >
            <el-option
              v-for="db in allDatabaseFlatOptions"
              :key="db.value"
              :label="db.label"
              :value="db.value"
            >
              <div style="display: flex; justify-content: space-between; align-items: center;">
                <span style="font-weight: 500;">{{ db.label }}</span>
                <el-tag size="small" type="info" effect="plain">{{ db.type }}</el-tag>
              </div>
            </el-option>
          </el-select>
          <div style="font-size: 11px; color: #94a3b8; margin-top: 4px;">
            已提供全系统已接入的数据库实例与业务库供个性化绑定（共 {{ allDatabaseFlatOptions.length }} 个可用库）
          </div>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="quickBindDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="quickBindLoading" @click="handleExecuteQuickBind">
          保存并立即生效绑定
        </el-button>
      </template>
    </el-dialog>

    <!-- SpEL 沙箱在线测试弹窗 -->
    <el-dialog
      title="🧪 SpEL 智能条件表达式沙箱测试"
      v-model="sandboxVisible"
      width="580px"
      append-to-body
    >
      <el-form label-position="top" size="small">
        <el-form-item label="待测 SpEL 条件表达式：">
          <el-input v-model="sandboxForm.spelExpression" placeholder="#{affectRows > 2000 || hasDdl == true}" />
        </el-form-item>
        <div style="display: flex; gap: 16px; flex-wrap: wrap;">
          <el-form-item label="预估影响行数 (affectRows)">
            <el-input-number v-model="sandboxForm.affectRows" :min="0" :max="1000000" style="width: 160px;" />
          </el-form-item>
          <el-form-item label="包含 DDL 变更 (hasDdl)">
            <el-switch v-model="sandboxForm.hasDdl" active-text="包含 DDL" inactive-text="纯 DML" />
          </el-form-item>
          <el-form-item label="部署环境 (environment)">
            <el-select v-model="sandboxForm.environment" style="width: 140px;">
              <el-option label="PROD (生产)" value="PROD" />
              <el-option label="STAGE (预发)" value="STAGE" />
              <el-option label="TEST (测试)" value="TEST" />
            </el-select>
          </el-form-item>
        </div>

        <div v-if="sandboxResult" style="margin-top: 14px; padding: 12px; border-radius: 6px; background: #f8fafc; border: 1px solid #e2e8f0;">
          <div style="display: flex; align-items: center; gap: 8px; margin-bottom: 6px;">
            <span style="font-weight: 600;">求值结果：</span>
            <el-tag :type="sandboxResult.matched ? 'danger' : 'success'" effect="dark">
              {{ sandboxResult.matched ? '🔥 命中高危分支 (true)' : '⚡ 命中常规低危分支 (false)' }}
            </el-tag>
            <el-tag v-if="sandboxResult.syntaxValid" type="success" size="small">语法合法</el-tag>
            <el-tag v-else type="danger" size="small">语法错误</el-tag>
          </div>
          <div style="font-size: 13px; color: #475569;">{{ sandboxResult.explanation }}</div>
          <div v-if="sandboxResult.errorMessage" style="color: #ef4444; font-size: 12px; margin-top: 4px;">{{ sandboxResult.errorMessage }}</div>
        </div>
      </el-form>
      <template #footer>
        <el-button @click="sandboxVisible = false">关闭</el-button>
        <el-button type="primary" :loading="sandboxLoading" @click="executeSpelSandbox">
          立即执行沙箱求值
        </el-button>
      </template>
    </el-dialog>

    <!-- 角色权限组成员维护与编辑弹窗 -->
    <el-dialog
      :title="`👥 维护【${currentEditingRole?.label}】角色权限组成员`"
      v-model="roleMemberModalVisible"
      width="640px"
      destroy-on-close
      append-to-body
    >
      <div style="margin-bottom: 12px; font-size: 13px; color: #64748b;">
        勾选系统人员将其分配至 <b>{{ currentEditingRole?.label }}</b> 角色组（支持按姓名/账号/部门实时搜索）：
      </div>
      <el-input
        v-model="memberSearchKeyword"
        placeholder="搜索用户姓名 / 账号 / 部门..."
        size="small"
        clearable
        :prefix-icon="Search"
        style="margin-bottom: 10px;"
      />
      <div class="user-check-list-box" style="max-height: 340px; overflow-y: auto; border: 1px solid #e2e8f0; border-radius: 6px; padding: 10px; background: #fafafa;">
        <el-checkbox-group v-model="selectedRoleUserIds">
          <div
            v-for="user in filteredAllUsers"
            :key="user.id"
            style="display: flex; align-items: center; justify-content: space-between; padding: 8px 10px; border-bottom: 1px dashed #e2e8f0; background: #ffffff; margin-bottom: 4px; border-radius: 4px;"
          >
            <el-checkbox :value="user.id">
              <span style="font-weight: 600; color: #1e293b;">{{ user.realName || user.username }}</span>
              <span style="color: #64748b; font-size: 12px; margin-left: 6px;">({{ user.department || '未分配部门' }})</span>
            </el-checkbox>
            <div style="display: flex; align-items: center; gap: 6px;">
              <span style="font-size: 11px; color: #94a3b8;">账号: {{ user.username }}</span>
              <el-tag size="small" :type="user.role === currentEditingRole?.key ? 'success' : 'info'" effect="light">
                {{ user.role === currentEditingRole?.key ? '本组成员' : `当前: ${user.role}` }}
              </el-tag>
            </div>
          </div>
        </el-checkbox-group>
      </div>
      <template #footer>
        <div style="display: flex; justify-content: space-between; align-items: center; width: 100%;">
          <el-button size="small" type="warning" plain @click="router.push('/roles')">
            ⚙️ 前往「系统角色与权限管理」大盘
          </el-button>
          <div>
            <el-button @click="roleMemberModalVisible = false">取消</el-button>
            <el-button type="primary" :loading="saveRoleMembersLoading" @click="handleSaveRoleMembers">
              保存成员配置
            </el-button>
          </div>
        </div>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import {
  Refresh,
  Plus,
  Edit,
  Delete,
  Search,
  Share,
  Right,
  CopyDocument,
  Download,
  RefreshRight,
  Upload,
  Select,
  CloseBold,
  View,
  Connection,
  Aim,
  FullScreen,
  VideoPause,
  CircleCheckFilled,
  InfoFilled,
  UserFilled,
  User,
  Operation,
  Coin
} from '@element-plus/icons-vue'
// @ts-ignore
import BpmnModeler from 'bpmn-js/lib/Modeler'
import 'bpmn-js/dist/assets/diagram-js.css'
import 'bpmn-js/dist/assets/bpmn-js.css'
import 'bpmn-js/dist/assets/bpmn-font/css/bpmn-embedded.css'
import request from '../utils/request'

const router = useRouter()

// ==================== 全量系统用户与角色数据联动 ====================
const allUsers = ref<any[]>([])
const availableInstancesList = ref<any[]>([])
const allResourceGroupDetails = ref<any[]>([])

export interface PaletteRoleItem {
  key: string
  label: string
  roleName: string
  color: string
  description?: string
  permissions?: string
}

const DEFAULT_PALETTE_ROLES: PaletteRoleItem[] = [
  { key: 'ADMIN', label: '超级管理员终审 (ADMIN)', roleName: '超级管理员', color: '#f59e0b', description: '拥有平台最高特权，具备全量功能页签与全权审批' },
  { key: 'DBA', label: '核心DBA安全复核 (DBA)', roleName: '核心数据库管理员', color: '#ef4444', description: '负责高危工单终审、实例纳管与 BPMN 流程编排' },
  { key: 'DEV_LEAD', label: '直属开发组长初审 (DEV_LEAD)', roleName: '业务开发组长', color: '#3b82f6', description: '负责业务工单初审、DML 审核与团队日常变更管控' },
  { key: 'DEV', label: '研发工程师 (DEV)', roleName: '研发工程师', color: '#06b6d4', description: '工单发起与 SQL 开发自测' },
  { key: 'AUDITOR', label: '安全合规审计员 (AUDITOR)', roleName: '安全合规审计员', color: '#8b5cf6', description: '负责安全大盘监控、数据脱敏配置与合规审计' },
  { key: 'OPS', label: '业务系统运维初审 (OPS)', roleName: '业务系统运维', color: '#10b981', description: '日常运维调度与生产巡检放行' },
  { key: 'SECURITY', label: '数据安全官 (SECURITY)', roleName: '数据安全官', color: '#ec4899', description: '全平台敏感数据资产保护与合规管控' }
]

const paletteRoles = ref<PaletteRoleItem[]>([...DEFAULT_PALETTE_ROLES])

const ROLE_COLOR_MAP: Record<string, string> = {
  ADMIN: '#f59e0b',
  DBA: '#ef4444',
  DEV_LEAD: '#3b82f6',
  DEV: '#06b6d4',
  AUDITOR: '#8b5cf6',
  OPS: '#10b981',
  SECURITY: '#ec4899'
}

const fetchRoles = async () => {
  try {
    const res: any = await request.get('/v1/role/list')
    if (Array.isArray(res.data) && res.data.length > 0) {
      paletteRoles.value = res.data.map((r: any) => {
        const code = r.roleCode || r.code || ''
        const name = r.roleName || r.name || code
        const color = ROLE_COLOR_MAP[code] || '#6366f1'
        return {
          key: code,
          label: `${name} (${code})`,
          roleName: name,
          color: color,
          description: r.description || '',
          permissions: r.permissions || ''
        }
      })
    }
  } catch (e) {
    console.warn('Fetch roles failed, fallback to defaults', e)
  }
}

const fetchUsers = async () => {
  try {
    const res: any = await request.get('/v1/user/list')
    allUsers.value = Array.isArray(res.data) ? res.data : []
  } catch (e) {}
}

const fetchInstances = async () => {
  try {
    const res: any = await request.get('/v1/instance/list')
    availableInstancesList.value = Array.isArray(res.data) ? res.data : []
  } catch (e) {}
}

const fetchResourceGroupDetails = async () => {
  try {
    const res: any = await request.get('/v1/resource-group/list')
    allResourceGroupDetails.value = Array.isArray(res.data) ? res.data : []
  } catch (e) {}
}

const currentStepIndex = computed(() => {
  if (currentDeployStatus.value.isDeployed) return 3
  if (selectedBpmnPreset.value) return 2
  return 1
})

const getRoleMembers = (roleKey: string) => {
  if (!allUsers.value || !roleKey) return []
  return allUsers.value.filter(u => {
    if (!u || (u.status !== 1 && u.status !== '1')) return false
    const r = u.role || ''
    if (r === roleKey) return true
    const roles = r.split(',').map((s: string) => s.trim())
    return roles.includes(roleKey)
  })
}

const getRoleMemberNames = (roleKey: string) => {
  const members = getRoleMembers(roleKey)
  if (members.length === 0) return ''
  return members.map(m => m.realName || m.username).join('、')
}

// 角色组成员编辑弹窗状态
const roleMemberModalVisible = ref(false)
const currentEditingRole = ref<any>(null)
const memberSearchKeyword = ref('')
const selectedRoleUserIds = ref<number[]>([])
const saveRoleMembersLoading = ref(false)

const filteredAllUsers = computed(() => {
  const kw = memberSearchKeyword.value.trim().toLowerCase()
  if (!kw) return allUsers.value
  return allUsers.value.filter(u =>
    (u.realName && u.realName.toLowerCase().includes(kw)) ||
    (u.username && u.username.toLowerCase().includes(kw)) ||
    (u.department && u.department.toLowerCase().includes(kw))
  )
})

const handleOpenRoleMemberEditor = (roleItem: any) => {
  currentEditingRole.value = roleItem
  memberSearchKeyword.value = ''
  selectedRoleUserIds.value = getRoleMembers(roleItem.key).map(u => u.id)
  roleMemberModalVisible.value = true
}

const handleSaveRoleMembers = async () => {
  if (!currentEditingRole.value) return
  saveRoleMembersLoading.value = true
  const roleKey = currentEditingRole.value.key
  try {
    for (const user of allUsers.value) {
      const isSelected = selectedRoleUserIds.value.includes(user.id)
      const userRoles = (user.role || '').split(',').map((s: string) => s.trim()).filter(Boolean)
      const currentHasRole = userRoles.includes(roleKey)
      if (isSelected && !currentHasRole) {
        // 分配到新角色
        userRoles.push(roleKey)
        await request.post('/v1/user/save', { ...user, role: userRoles.join(',') })
      } else if (!isSelected && currentHasRole) {
        // 从当前角色移出
        const updatedRoles = userRoles.filter((r: string) => r !== roleKey)
        const finalRole = updatedRoles.length > 0 ? updatedRoles.join(',') : 'DEV'
        await request.post('/v1/user/save', { ...user, role: finalRole })
      }
    }
    ElMessage.success(`【${currentEditingRole.value.label}】角色组成员已成功更新！`)
    roleMemberModalVisible.value = false
    await fetchUsers()
  } catch (err: any) {
    ElMessage.error('保存角色组成员失败：' + (err.message || '网络异常'))
  } finally {
    saveRoleMembersLoading.value = false
  }
}

// ==================== BPMN 权限组与审批角色资产库 ====================
const paletteSearchKeyword = ref('')
const draggedItem = ref<any>(null)

const filteredPaletteRoles = computed(() => {
  const kw = paletteSearchKeyword.value.trim().toLowerCase()
  if (!kw) return paletteRoles.value
  return paletteRoles.value.filter(r => {
    const memberNames = getRoleMemberNames(r.key).toLowerCase()
    return r.label.toLowerCase().includes(kw) || r.key.toLowerCase().includes(kw) || (r.description && r.description.toLowerCase().includes(kw)) || memberNames.includes(kw)
  })
})

const getDatabasesByResourceGroup = (rgName: string) => {
  const dbs: string[] = []
  availableInstancesList.value.forEach(inst => {
    if (inst.resourceGroup === rgName || (inst.resourceGroups && inst.resourceGroups.includes(rgName)) || rgName === '全部业务资源组通用') {
      if (Array.isArray(inst.databases) && inst.databases.length > 0) {
        inst.databases.forEach((db: string) => {
          const dbKey = `${inst.name}/${db}`
          if (!dbs.includes(dbKey)) dbs.push(dbKey)
        })
      } else {
        dbs.push(`${inst.name}/main_db`)
      }
    }
  })
  if (dbs.length === 0) {
    dbs.push('car_prod_mysql/car_insurance_db', 'car_prod_mysql/car_claim_db')
  }
  return dbs
}

const handleDragStart = (e: DragEvent, item: any) => {
  draggedItem.value = item
  if (e.dataTransfer) {
    e.dataTransfer.setData('text/plain', JSON.stringify(item))
    e.dataTransfer.effectAllowed = 'copy'
  }
}

const addNodeToCanvas = (item: { type: string; name: string; role: string }) => {
  if (!bpmnModeler) {
    ElMessage.warning('BPMN 画布尚未初始化完成')
    return
  }
  try {
    const modeling = bpmnModeler.get('modeling')
    const elementFactory = bpmnModeler.get('elementFactory')
    const canvas = bpmnModeler.get('canvas')
    const bpmnFactory = bpmnModeler.get('bpmnFactory')
    const rootElement = canvas.getRootElement()

    const x = 380 + Math.floor(Math.random() * 160)
    const y = 140 + Math.floor(Math.random() * 120)

    let shapeType = 'bpmn:UserTask'
    if (item.type === 'GATEWAY') shapeType = 'bpmn:ExclusiveGateway'
    else if (item.type === 'SERVICE') shapeType = 'bpmn:ServiceTask'

    const businessObj = bpmnFactory.create(shapeType, {
      name: `${item.name}`,
      id: `${shapeType.split(':')[1] || 'Node'}_${Date.now().toString().slice(-6)}`
    })

    const newElement = elementFactory.createShape({
      type: shapeType,
      businessObject: businessObj
    })

    modeling.createShape(newElement, { x, y }, rootElement)
    ElMessage.success(`已成功在流程画布中生成【${item.name}】节点！`)
  } catch (err: any) {
    ElMessage.success(`已为流程挂载【${item.name}】权限属性！`)
  }
}

const handleCanvasDrop = (e: DragEvent) => {
  e.preventDefault()
  if (draggedItem.value) {
    addNodeToCanvas(draggedItem.value)
    draggedItem.value = null
  } else if (e.dataTransfer) {
    try {
      const dataStr = e.dataTransfer.getData('text/plain')
      if (dataStr) {
        const item = JSON.parse(dataStr)
        addNodeToCanvas(item)
      }
    } catch (err) {}
  }
}

interface ApprovalNodeItem {
  step: number
  nodeName: string
  role: string
  approvalMode?: 'ORSIGN' | 'COUNTERSIGN' | 'SEQUENTIAL'
}

interface WorkflowTemplateItem {
  id: number | null
  templateName: string
  bpmnProcessKey?: string
  flowType: string
  resourceGroups: string
  selectedResourceGroups?: string[]
  targetDatabases?: string
  nodeConfig: string
  nodes?: ApprovalNodeItem[]
  conditionDimension?: string
  affectRowsThreshold?: number
  highRiskRole?: string
  lowRiskRole?: string
  spelExpression?: string
  triggerCondition?: string
  defaultExecutionMode: string
  status: number
  description?: string
}

const currentTemplateDbScopeText = computed(() => {
  if (selectedBpmnPreset.value.startsWith('tpl_')) {
    const tplId = Number(selectedBpmnPreset.value.replace('tpl_', ''))
    const found = templateList.value.find(t => t.id === tplId)
    if (found) {
      const rgs = parseResourceGroups(found.resourceGroups).join(', ') || '通用'
      if (!found.targetDatabases || found.targetDatabases.includes('ALL') || found.targetDatabases === '[]') {
        return `${rgs} · [全部业务库通用]`
      }
      try {
        const dbs = JSON.parse(found.targetDatabases)
        if (Array.isArray(dbs) && dbs.length > 0) {
          return `${rgs} · [${dbs.join(', ')}]`
        }
      } catch (e) {}
      return `${rgs} · [全部业务库通用]`
    }
  }
  return '全部业务资源组 · [全部数据库通用]'
})

const currentTemplateFlowTypeText = computed(() => {
  if (selectedBpmnPreset.value.startsWith('tpl_')) {
    const tplId = Number(selectedBpmnPreset.value.replace('tpl_', ''))
    const found = templateList.value.find(t => t.id === tplId)
    if (found) {
      return formatFlowTypeBadge(found.flowType)
    }
  }
  return '全部工单类型通用'
})

const ALL_TICKET_TYPES = ['SQL_AUDIT', 'DML_CHANGE', 'DDL_CHANGE', 'DATA_EXPORT', 'PERMISSION', 'DATA_RECOVERY']

const formatFlowTypeBadge = (flowType?: string) => {
  if (!flowType || flowType === 'ALL') return '全部工单类型通用 (ALL)'
  const map: Record<string, string> = {
    SQL_AUDIT: 'SQL 变更审核 (全量)',
    DML_CHANGE: 'DML 数据变更',
    DDL_CHANGE: 'DDL 结构变更',
    DATA_EXPORT: '敏感数据导出',
    PERMISSION: '权限与账号申请',
    DATA_QUERY: '数据查询提权',
    DATA_RECOVERY: '应急数据修复与恢复'
  }
  const parts = flowType.split(',').map(s => s.trim())
  const labels = parts.map(p => map[p] || p)
  return labels.join('、')
}

const handleDbScopeModeChange = (mode: string) => {
  if (mode === 'ALL') {
    form.value.selectedTargetDatabases = ['ALL']
  } else {
    if (form.value.selectedTargetDatabases.includes('ALL')) {
      form.value.selectedTargetDatabases = []
    }
  }
}

const SPEL_PRESETS = [
  { label: '影响行数 > 1000 行', expr: '#{affectRows > 1000}', dim: 'AFFECT_ROWS', threshold: 1000 },
  { label: '影响行数 > 2000 行 (大表)', expr: '#{affectRows > 2000}', dim: 'AFFECT_ROWS', threshold: 2000 },
  { label: '影响行数 > 5000 行', expr: '#{affectRows > 5000}', dim: 'AFFECT_ROWS', threshold: 5000 },
  { label: '包含 DDL 库表结构变更', expr: '#{hasDdl == true}', dim: 'CHANGE_TYPE', threshold: 1000 },
  { label: '复合：行数 > 2000 或 DDL', expr: '#{affectRows > 2000 || hasDdl == true}', dim: 'COMPOSITE', threshold: 2000 },
  { label: '复合：行数 > 1000 或 DDL', expr: '#{affectRows > 1000 || hasDdl == true}', dim: 'COMPOSITE', threshold: 1000 },
  { label: '高危删表/结构变更', expr: "#{ticketType == 'DDL_CHANGE' || sql.toUpperCase().contains('DROP')}", dim: 'CHANGE_TYPE', threshold: 1000 }
]

const EXEC_STRATEGY_OPTIONS = [
  {
    key: 'IMMEDIATE',
    label: '立即流式执行',
    icon: '⚡',
    tagType: 'success' as const,
    color: '#67C23A',
    badge: '高效在线',
    desc: '审批通过后立即由平台流式下发至目标库事务中执行，实时捕获影响行数与报错'
  },
  {
    key: 'SCHEDULED',
    label: '定时窗口执行',
    icon: '⏰',
    tagType: 'warning' as const,
    color: '#E6A23C',
    badge: '低峰维护',
    desc: '审批通过后挂起并设定低峰维护窗口（如凌晨 02:00:00），由定时调度器自动下发'
  },
  {
    key: 'MANUAL_DBA',
    label: '转DBA线下工具',
    icon: '🛠️',
    tagType: 'danger' as const,
    color: '#F56C6C',
    badge: '专家兜底',
    desc: '生成标准执行变更工单与脱敏脚本，转交 DBA 使用 DMS / gh-ost / pt-osc 工具线下执行后反馈归档'
  },
  {
    key: 'CANARY_BATCH',
    label: '分批灰度执行',
    icon: '🔄',
    tagType: 'primary' as const,
    color: '#409EFF',
    badge: '大批量防护',
    desc: '针对大事务分片循环 Commit（如每批 1000 行），有效避免主从复制延迟与长事务行锁暴涨'
  },
  {
    key: 'DRY_RUN_ONLY',
    label: '仅演练不落库',
    icon: '🧪',
    tagType: 'info' as const,
    color: '#909399',
    badge: '只读演练',
    desc: '严格执行 AST 语法校验与影响行数预演，自动 ROLLBACK，不落盘真实数据，适用于安全审查'
  }
]

const activeTab = ref('templates')
const loading = ref(false)
const saveLoading = ref(false)
const deployLoading = ref(false)
const searchKeyword = ref('')
const templateList = ref<WorkflowTemplateItem[]>([])
const availableResourceGroups = ref<string[]>([])

const modalVisible = ref(false)
const isEdit = ref(false)
const formRef = ref<FormInstance>()

const form = ref<{
  id: number | null
  templateName: string
  bpmnProcessKey?: string
  flowType: string
  isConditionGateway?: boolean
  conditionDimension?: string
  affectRowsThreshold?: number
  highRiskRole?: string
  lowRiskRole?: string
  spelExpression?: string
  selectedResourceGroups: string[]
  dbScopeMode: 'ALL' | 'CUSTOM'
  selectedTargetDatabases: string[]
  nodes: ApprovalNodeItem[]
  selectedExecModes: string[]
  triggerCondition: string
  description: string
  status: number
}>({
  id: null,
  templateName: '',
  bpmnProcessKey: 'Process_StandardSqlReview',
  flowType: 'DML_CHANGE',
  isConditionGateway: false,
  conditionDimension: 'AFFECT_ROWS',
  affectRowsThreshold: 1000,
  highRiskRole: 'DBA',
  lowRiskRole: 'DEV_LEAD',
  spelExpression: '#{affectRows > 1000}',
  selectedResourceGroups: [],
  dbScopeMode: 'ALL',
  selectedTargetDatabases: ['ALL'],
  nodes: [
    { step: 1, nodeName: '开发组长初审', role: 'DEV_LEAD' },
    { step: 2, nodeName: '核心DBA安全复审', role: 'DBA' }
  ],
  selectedExecModes: ['IMMEDIATE', 'SCHEDULED'],
  triggerCondition: '',
  description: '',
  status: 1
})

const formRules = ref<FormRules>({
  templateName: [{ required: true, message: '请输入审批流名称', trigger: 'blur' }],
  flowType: [{ required: true, message: '请选择变更类型', trigger: 'change' }]
})

// BPMN 画布与部署控制相关
const canvas = ref<HTMLElement | null>(null)
let bpmnModeler: any = null
const selectedBpmnPreset = ref('dml_condition')
const isBpmnFullscreen = ref(false)
const terminateLoading = ref(false)
const currentDeployStatus = ref<{
  isDeployed: boolean
  deploymentId?: string
  deployTime?: string
  status?: string
}>({
  isDeployed: false
})

// 绑定生效范围弹窗状态
const quickBindDialogVisible = ref(false)
const quickBindLoading = ref(false)
const quickBindTicketTypes = ref<string[]>([...ALL_TICKET_TYPES])
const quickBindResourceGroups = ref<string[]>([])
const quickBindDbScopeMode = ref<'ALL' | 'CUSTOM'>('ALL')
const quickBindSelectedDatabases = ref<string[]>([])

const handleSelectAllQuickBindTicketTypes = () => {
  quickBindTicketTypes.value = [...ALL_TICKET_TYPES]
  ElMessage.success('已全选所有工单变更类型')
}

const handleQuickBindDbScopeChange = (mode: string) => {
  if (mode === 'ALL') {
    quickBindSelectedDatabases.value = []
  }
}

const handleSelectAllQuickBindResourceGroups = () => {
  quickBindResourceGroups.value = [...availableResourceGroups.value]
  ElMessage.success(`已全选全部 ${availableResourceGroups.value.length} 个业务资源组`)
}

const handleClearAllQuickBindResourceGroups = () => {
  quickBindResourceGroups.value = []
  ElMessage.info('已清空所选业务资源组')
}

// 联动计算所选资源组包含的全部数据库列表
const quickBindCurrentGroupDatabases = computed(() => {
  const result: string[] = []
  quickBindResourceGroups.value.forEach(rgName => {
    const dbs = getDatabasesByResourceGroup(rgName)
    dbs.forEach(d => {
      if (!result.includes(d)) result.push(d)
    })
  })
  return result
})

const handleSelectAllQuickBindDatabases = () => {
  if (quickBindCurrentGroupDatabases.value.length === 0) {
    ElMessage.warning('当前所选资源组下暂无数据库')
    return
  }
  quickBindSelectedDatabases.value = [...quickBindCurrentGroupDatabases.value]
  ElMessage.success(`已全选当前资源组的全部 ${quickBindSelectedDatabases.value.length} 个数据库`)
}

const handleClearAllQuickBindDatabases = () => {
  quickBindSelectedDatabases.value = []
  ElMessage.info('已清空所选数据库')
}

const allDatabaseFlatOptions = computed(() => {
  const list: { label: string; value: string; instanceName: string; dbName: string; type: string }[] = []
  availableInstancesList.value.forEach(inst => {
    const instName = inst.name || '默认实例'
    const dbType = inst.dbType || 'MYSQL'
    if (Array.isArray(inst.databases) && inst.databases.length > 0) {
      inst.databases.forEach((db: string) => {
        list.push({
          label: `${instName} / ${db}`,
          value: `${instName}/${db}`,
          instanceName: instName,
          dbName: db,
          type: dbType
        })
      })
    } else {
      list.push({
        label: `${instName} / main_db`,
        value: `${instName}/main_db`,
        instanceName: instName,
        dbName: 'main_db',
        type: dbType
      })
    }
  })
  if (list.length === 0) {
    list.push(
      { label: '阿里云RDS-车险与销管核心 / car_insurance_db', value: '阿里云RDS-车险与销管核心/car_insurance_db', instanceName: '阿里云RDS', dbName: 'car_insurance_db', type: 'MYSQL' },
      { label: '阿里云RDS-车险与销管核心 / car_claim_db', value: '阿里云RDS-车险与销管核心/car_claim_db', instanceName: '阿里云RDS', dbName: 'car_claim_db', type: 'MYSQL' }
    )
  }
  return list
})

const fetchResourceGroups = async () => {
  try {
    const res: any = await request.get('/v1/resource-group/list')
    const list = Array.isArray(res.data) ? res.data : []
    allResourceGroupDetails.value = list
    availableResourceGroups.value = list.map((g: any) => g.groupName)
  } catch (error) {
    // ignore
  }
}

const pagination = ref({
  current: 1,
  size: 10,
  total: 0
})

const fetchTemplates = async () => {
  loading.value = true
  try {
    const res: any = await request.get('/v1/workflow/template/page', {
      params: {
        page: pagination.value.current,
        size: pagination.value.size,
        keyword: searchKeyword.value.trim()
      }
    })
    if (res.data && res.data.records) {
      templateList.value = res.data.records
      pagination.value.total = res.data.total
      pagination.value.current = res.data.current
      pagination.value.size = res.data.size
    } else {
      templateList.value = Array.isArray(res.data) ? res.data : []
      pagination.value.total = templateList.value.length
    }
  } catch (error) {
    ElMessage.error('获取审批流模板列表失败')
  } finally {
    loading.value = false
  }
}

const handleSizeChange = (val: number) => {
  pagination.value.size = val
  pagination.value.current = 1
  fetchTemplates()
}

const handleCurrentChange = (val: number) => {
  pagination.value.current = val
  fetchTemplates()
}

const parseResourceGroups = (jsonStr: string): string[] => {
  if (!jsonStr) return []
  try {
    const arr = JSON.parse(jsonStr)
    return Array.isArray(arr) ? arr : [jsonStr]
  } catch (e) {
    return [jsonStr]
  }
}

const parseNodeConfig = (jsonStr: string): ApprovalNodeItem[] => {
  if (!jsonStr) return []
  try {
    const arr = JSON.parse(jsonStr)
    return Array.isArray(arr) ? arr : []
  } catch (e) {
    return []
  }
}

const parseExecModes = (modeData: string) => {
  if (!modeData) return [EXEC_STRATEGY_OPTIONS[0]]
  let keys: string[] = []
  try {
    const parsed = JSON.parse(modeData)
    if (Array.isArray(parsed)) {
      keys = parsed
    } else if (typeof parsed === 'string') {
      keys = [parsed]
    }
  } catch (e) {
    if (modeData.includes(',')) {
      keys = modeData.split(',').map(s => s.trim())
    } else {
      keys = [modeData.trim()]
    }
  }
  return keys.map(k => {
    const found = EXEC_STRATEGY_OPTIONS.find(o => o.key === k)
    return found || {
      key: k,
      label: k,
      icon: '⚙️',
      tagType: 'info' as const,
      color: '#909399',
      badge: '扩展策略',
      desc: '自定义执行策略'
    }
  })
}

const handleSelectAllResourceGroups = () => {
  form.value.selectedResourceGroups = [...availableResourceGroups.value]
  ElMessage.success(`已全选 ${availableResourceGroups.value.length} 个业务资源组`)
}

const handleClearAllResourceGroups = () => {
  form.value.selectedResourceGroups = []
  ElMessage.info('已清空所选业务资源组')
}

const handleSelectUniversalResourceGroup = () => {
  form.value.selectedResourceGroups = ['全部业务资源组通用']
  ElMessage.success('已设为全部业务资源组通用')
}

const toggleExecMode = (key: string) => {
  const idx = form.value.selectedExecModes.indexOf(key)
  if (idx > -1) {
    if (form.value.selectedExecModes.length > 1) {
      form.value.selectedExecModes.splice(idx, 1)
    } else {
      ElMessage.warning('至少需要保留一种执行策略')
    }
  } else {
    form.value.selectedExecModes.push(key)
  }
}

const isConditionTemplate = (tpl: any) => {
  if (!tpl) return false
  const name = tpl.templateName || ''
  const config = tpl.nodeConfig || ''
  return config.includes('"role":"GATEWAY"') || config.includes('"role": "GATEWAY"') || (name.includes('智能条件分支') && !name.includes('四级'))
}

const applySpelPreset = (preset: typeof SPEL_PRESETS[0]) => {
  form.value.spelExpression = preset.expr
  form.value.conditionDimension = preset.dim
  form.value.affectRowsThreshold = preset.threshold
  updateTriggerConditionFromGateway()
}

const handleCustomSpelInput = () => {
  updateTriggerConditionFromGateway()
}

const handleDimensionChange = (val: any) => {
  const th = form.value.affectRowsThreshold || 1000
  if (val === 'CHANGE_TYPE') {
    form.value.spelExpression = '#{hasDdl == true}'
  } else if (val === 'COMPOSITE') {
    form.value.spelExpression = `#{affectRows > ${th} || hasDdl == true}`
  } else {
    form.value.spelExpression = `#{affectRows > ${th}}`
  }
  updateTriggerConditionFromGateway()
}

const handleThresholdChange = (val: any) => {
  const th = val || 1000
  const dim = form.value.conditionDimension || 'AFFECT_ROWS'
  if (dim === 'COMPOSITE') {
    form.value.spelExpression = `#{affectRows > ${th} || hasDdl == true}`
  } else if (dim === 'AFFECT_ROWS') {
    form.value.spelExpression = `#{affectRows > ${th}}`
  }
  updateTriggerConditionFromGateway()
}

const updateTriggerConditionFromGateway = () => {
  const dim = form.value.conditionDimension || 'AFFECT_ROWS'
  const th = form.value.affectRowsThreshold || 1000
  const highRole = form.value.highRiskRole || 'DBA'
  const lowRole = form.value.lowRiskRole || 'DEV_LEAD'
  const spel = form.value.spelExpression || `#{affectRows > ${th}}`

  const highRoleLabel = highRole === 'ADMIN' ? '超级管理员 (ADMIN)' : (highRole === 'AUDITOR' ? '安全审计员 (AUDITOR)' : '核心 DBA')
  const lowRoleLabel = lowRole === 'OPS' ? '业务运维 (OPS)' : '开发组长 (DEV_LEAD)'

  if (dim === 'AFFECT_ROWS') {
    form.value.triggerCondition = `影响行数 > ${th} (SpEL: ${spel}) 需${highRoleLabel}审核；影响行数 ≤ ${th} 由${lowRoleLabel}初审`
  } else if (dim === 'CHANGE_TYPE') {
    form.value.triggerCondition = `包含 DDL 库表结构变更 (SpEL: ${spel}) 需${highRoleLabel}审核；常规 DML 纯数据由${lowRoleLabel}初审`
  } else {
    form.value.triggerCondition = `包含 DDL 变更或影响行数 > ${th} (SpEL: ${spel}) 需${highRoleLabel}审核；常规 DML 影响行数 ≤ ${th} 由${lowRoleLabel}初审`
  }
}

const setThreshold = (val: number) => {
  form.value.affectRowsThreshold = val
  const dim = form.value.conditionDimension || 'AFFECT_ROWS'
  if (dim === 'COMPOSITE') {
    form.value.spelExpression = `#{affectRows > ${val} || hasDdl == true}`
  } else if (dim === 'AFFECT_ROWS') {
    form.value.spelExpression = `#{affectRows > ${val}}`
  }
  updateTriggerConditionFromGateway()
}

const handleGatewayModeChange = (val: any) => {
  if (val) {
    if (!form.value.conditionDimension) form.value.conditionDimension = 'AFFECT_ROWS'
    if (!form.value.affectRowsThreshold) form.value.affectRowsThreshold = 1000
    if (!form.value.highRiskRole) form.value.highRiskRole = 'DBA'
    if (!form.value.lowRiskRole) form.value.lowRiskRole = 'DEV_LEAD'
    if (!form.value.spelExpression) form.value.spelExpression = '#{affectRows > 1000}'
    updateTriggerConditionFromGateway()
    form.value.nodes = [
      { step: 1, nodeName: '影响行数智能排他网关判定', role: 'GATEWAY' },
      { step: 2, nodeName: '核心DBA安全复核 (高危分支)', role: 'DBA' },
      { step: 3, nodeName: '运维/开发组长初审 (常规分支)', role: 'DEV_LEAD' }
    ]
  } else {
    form.value.triggerCondition = '常规 UPDATE / INSERT / DELETE 变更'
    form.value.nodes = [
      { step: 1, nodeName: '开发组长初审', role: 'DEV_LEAD' },
      { step: 2, nodeName: '核心DBA安全复审', role: 'DBA' }
    ]
  }
}

const handleBpmnKeyChange = (key: string) => {
  if (key === 'Process_SensitiveDdlReview') {
    form.value.isConditionGateway = false
    form.value.flowType = 'DDL_CHANGE'
    form.value.nodes = [
      { step: 1, nodeName: 'AI智能锁表与性能分析', role: 'AUDITOR' },
      { step: 2, nodeName: '资深DBA审核', role: 'DBA' },
      { step: 3, nodeName: '安全合规架构师复核', role: 'ADMIN' }
    ]
  } else if (key === 'Process_EmergencyChange') {
    form.value.isConditionGateway = false
    form.value.flowType = 'SQL_AUDIT'
    form.value.nodes = [
      { step: 1, nodeName: '值班DBA极速放行', role: 'DBA' },
      { step: 2, nodeName: '事后部门主管补录审计', role: 'DEV_LEAD' }
    ]
  } else if (key === 'Process_DataExportReview') {
    form.value.isConditionGateway = false
    form.value.flowType = 'DATA_EXPORT'
    form.value.nodes = [
      { step: 1, nodeName: '业务部门主管审批', role: 'DEV_LEAD' },
      { step: 2, nodeName: '数据安全合规官审批', role: 'AUDITOR' }
    ]
  } else {
    form.value.flowType = 'DML_CHANGE'
    form.value.nodes = [
      { step: 1, nodeName: '开发组长初审', role: 'DEV_LEAD' },
      { step: 2, nodeName: '核心DBA安全复审', role: 'DBA' }
    ]
  }
}

const handleOpenCreateModal = () => {
  isEdit.value = false
  form.value = {
    id: null,
    templateName: '',
    bpmnProcessKey: 'Process_StandardSqlReview',
    flowType: 'DML_CHANGE',
    isConditionGateway: false,
    conditionDimension: 'AFFECT_ROWS',
    affectRowsThreshold: 1000,
    highRiskRole: 'DBA',
    lowRiskRole: 'DEV_LEAD',
    spelExpression: '#{affectRows > 1000}',
    selectedResourceGroups: ['车险承保资源组'],
    dbScopeMode: 'ALL',
    selectedTargetDatabases: ['ALL'],
    nodes: [
      { step: 1, nodeName: '开发组长初审', role: 'DEV_LEAD' },
      { step: 2, nodeName: '核心DBA安全复审', role: 'DBA' }
    ],
    selectedExecModes: ['IMMEDIATE', 'SCHEDULED'],
    triggerCondition: '常规 UPDATE / INSERT / DELETE 变更',
    description: '',
    status: 1
  }
  modalVisible.value = true
}

const handleOpenEditModal = (row: WorkflowTemplateItem) => {
  isEdit.value = true
  const rgs = parseResourceGroups(row.resourceGroups)
  const nodes = parseNodeConfig(row.nodeConfig)
  const modes = parseExecModes(row.defaultExecutionMode).map(m => m.key)
  const isCond = isConditionTemplate(row)

  const defSpel = row.conditionDimension === 'CHANGE_TYPE'
    ? '#{hasDdl == true}'
    : (row.conditionDimension === 'COMPOSITE'
      ? `#{affectRows > ${row.affectRowsThreshold || 1000} || hasDdl == true}`
      : `#{affectRows > ${row.affectRowsThreshold || 1000}}`)

  let dbMode: 'ALL' | 'CUSTOM' = 'ALL'
  let targetDbs: string[] = ['ALL']
  if (row.targetDatabases) {
    try {
      const parsed = JSON.parse(row.targetDatabases)
      if (Array.isArray(parsed) && parsed.length > 0 && !parsed.includes('ALL')) {
        dbMode = 'CUSTOM'
        targetDbs = parsed
      }
    } catch (e) {}
  }

  form.value = {
    id: row.id,
    templateName: row.templateName,
    bpmnProcessKey: row.bpmnProcessKey || 'Process_StandardSqlReview',
    flowType: row.flowType,
    isConditionGateway: isCond,
    conditionDimension: row.conditionDimension || 'AFFECT_ROWS',
    affectRowsThreshold: row.affectRowsThreshold || 1000,
    highRiskRole: row.highRiskRole || 'DBA',
    lowRiskRole: row.lowRiskRole || 'DEV_LEAD',
    spelExpression: row.spelExpression || defSpel,
    selectedResourceGroups: rgs.length > 0 ? rgs : ['车险承保资源组'],
    dbScopeMode: dbMode,
    selectedTargetDatabases: targetDbs,
    nodes: nodes.length > 0 ? nodes : [{ step: 1, nodeName: '开发组长初审', role: 'DEV_LEAD' }],
    selectedExecModes: modes.length > 0 ? modes : ['IMMEDIATE'],
    triggerCondition: row.triggerCondition || '',
    description: row.description || '',
    status: row.status
  }
  if (isCond && (!row.triggerCondition || row.triggerCondition.trim() === '')) {
    updateTriggerConditionFromGateway()
  }
  modalVisible.value = true
}

const sandboxVisible = ref(false)
const sandboxLoading = ref(false)
const sandboxForm = ref({
  spelExpression: '#{affectRows > 2000 || hasDdl == true}',
  affectRows: 2500,
  hasDdl: false,
  environment: 'PROD'
})
const sandboxResult = ref<any>(null)

const handleOpenSpelSandbox = () => {
  sandboxForm.value.spelExpression = form.value.spelExpression || '#{affectRows > 2000 || hasDdl == true}'
  sandboxForm.value.affectRows = form.value.affectRowsThreshold || 2500
  sandboxResult.value = null
  sandboxVisible.value = true
}

const executeSpelSandbox = async () => {
  sandboxLoading.value = true
  try {
    const res: any = await request.post('/v1/workflow/template/evaluate-spel', {
      spelExpression: sandboxForm.value.spelExpression,
      context: {
        affectRows: sandboxForm.value.affectRows,
        hasDdl: sandboxForm.value.hasDdl,
        environment: sandboxForm.value.environment,
        ticketType: form.value.flowType
      }
    })
    sandboxResult.value = res.data
    if (res.data?.syntaxValid) {
      ElMessage.success('SpEL 沙箱求值完成')
    } else {
      ElMessage.warning('SpEL 表达式语法存在错误')
    }
  } catch (e: any) {
    ElMessage.error(e.response?.data?.message || '求值失败')
  } finally {
    sandboxLoading.value = false
  }
}

const addNode = () => {
  const nextStep = form.value.nodes.length + 1
  form.value.nodes.push({
    step: nextStep,
    nodeName: `第 ${nextStep} 级审核`,
    role: nextStep === 3 ? 'ADMIN' : 'DBA',
    approvalMode: 'ORSIGN'
  })
}

const removeNode = (index: number) => {
  form.value.nodes.splice(index, 1)
  form.value.nodes.forEach((n, idx) => {
    n.step = idx + 1
  })
}

const handleSaveTemplate = async () => {
  if (!formRef.value) return
  await formRef.value.validate(async (valid) => {
    if (!valid) return
    saveLoading.value = true
    try {
      let finalNodes = form.value.nodes
      let finalCond = form.value.triggerCondition
      if (form.value.isConditionGateway) {
        updateTriggerConditionFromGateway()
        finalCond = form.value.triggerCondition
        const highRole = form.value.highRiskRole || 'DBA'
        const lowRole = form.value.lowRiskRole || 'DEV_LEAD'
        finalNodes = [
          { step: 1, nodeName: `智能排他网关判定 (SpEL: ${form.value.spelExpression || '#{affectRows > 1000}'})`, role: 'GATEWAY' } as any,
          { step: 2, nodeName: `高危分支审核 (${highRole})`, role: highRole },
          { step: 3, nodeName: `常规分支审核 (${lowRole})`, role: lowRole }
        ]
      }
      const payload = {
        id: form.value.id,
        templateName: form.value.templateName.trim(),
        flowType: form.value.flowType,
        resourceGroups: JSON.stringify(form.value.selectedResourceGroups),
        targetDatabases: JSON.stringify(form.value.dbScopeMode === 'ALL' ? ['ALL'] : form.value.selectedTargetDatabases),
        nodeConfig: JSON.stringify(finalNodes),
        conditionDimension: form.value.conditionDimension || 'AFFECT_ROWS',
        affectRowsThreshold: form.value.affectRowsThreshold || 1000,
        highRiskRole: form.value.highRiskRole || 'DBA',
        lowRiskRole: form.value.lowRiskRole || 'DEV_LEAD',
        spelExpression: form.value.spelExpression || '#{affectRows > 1000}',
        defaultExecutionMode: JSON.stringify(form.value.selectedExecModes),
        triggerCondition: finalCond,
        description: form.value.description,
        status: form.value.status
      }
      await request.post('/v1/workflow/template/save', payload)
      ElMessage.success(isEdit.value ? '修改成功！' : '创建成功！')
      modalVisible.value = false
      await fetchTemplates()
      if (selectedBpmnPreset.value.startsWith('tpl_')) {
        handleBpmnPresetChange()
      }
    } catch (err: any) {
      ElMessage.error(err.response?.data?.message || '保存审批流模板失败')
    } finally {
      saveLoading.value = false
    }
  })
}

const handleDeleteTemplate = async (row: WorkflowTemplateItem) => {
  try {
    await ElMessageBox.confirm(`确认删除审批流模板【${row.templateName}】吗？`, '删除确认', {
      type: 'warning',
      confirmButtonText: '确认删除',
      cancelButtonText: '取消'
    })
    await request.delete(`/v1/workflow/template/${row.id}`)
    ElMessage.success('删除成功')
    fetchTemplates()
  } catch (e) {
    // cancel
  }
}

// 动态根据节点配置与条件生成标准 BPMN 2.0 XML (支持动态 SpEL 表达式与排他网关分支)
const generateBpmnXmlFromNodes = (
  templateName: string,
  nodes: ApprovalNodeItem[],
  flowType: string,
  _triggerCondition?: string,
  isConditionGateway?: boolean,
  conditionDimension?: string,
  affectRowsThreshold?: number,
  highRiskRole?: string,
  lowRiskRole?: string,
  spelExpression?: string
): string => {
  const isCond = isConditionGateway || (templateName && templateName.includes('智能条件分支') && !templateName.includes('四级'))

  const th = affectRowsThreshold || 1000
  const dim = conditionDimension || 'AFFECT_ROWS'
  const highRole = highRiskRole || 'DBA'
  const lowRole = lowRiskRole || 'DEV_LEAD'
  const spel = spelExpression || (dim === 'CHANGE_TYPE' ? '#{hasDdl == true}' : (dim === 'COMPOSITE' ? `#{affectRows > ${th} || hasDdl == true}` : `#{affectRows > ${th}}`))

  const highRoleLabel = highRole === 'ADMIN' ? '超级管理员终审' : (highRole === 'AUDITOR' ? '合规安全审计核查' : '核心DBA安全复核')
  const lowRoleLabel = lowRole === 'OPS' ? '业务运维管理员初审' : '业务开发组长初审'

  if (isCond) {
    return `<?xml version="1.0" encoding="UTF-8"?>
<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL"
             xmlns:bpmndi="http://www.omg.org/spec/BPMN/20100524/DI"
             xmlns:dc="http://www.omg.org/spec/DD/20100524/DC"
             xmlns:di="http://www.omg.org/spec/DD/20100524/DI"
             xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
             id="Definitions_ConditionalDml"
             targetNamespace="http://wmdb.com/schema/bpmn">
  <process id="Process_ConditionalDmlReview" name="${templateName || 'DML 影响行数智能条件分支审批流'}" isExecutable="true">
    <startEvent id="Start_1" name="提交变更申请" />
    <sequenceFlow id="Flow_ToGateway" sourceRef="Start_1" targetRef="Gateway_Rows" />
    <exclusiveGateway id="Gateway_Rows" name="SpEL排他网关: ${spel}" />
    <sequenceFlow id="Flow_HighRisk" name="🔥 左分支 (高危): ${spel}" sourceRef="Gateway_Rows" targetRef="Task_HighRiskDba">
      <conditionExpression xsi:type="tFormalExpression"><![CDATA[${spel}]]></conditionExpression>
    </sequenceFlow>
    <userTask id="Task_HighRiskDba" name="${highRoleLabel} (${highRole})" />
    <sequenceFlow id="Flow_DbaToExec" sourceRef="Task_HighRiskDba" targetRef="Task_ServiceExec" />
    <sequenceFlow id="Flow_LowRisk" name="⚡ 右分支 (常规): 默认放行流" sourceRef="Gateway_Rows" targetRef="Task_LowRiskOps" />
    <userTask id="Task_LowRiskOps" name="${lowRoleLabel} (${lowRole})" />
    <sequenceFlow id="Flow_OpsToExec" sourceRef="Task_LowRiskOps" targetRef="Task_ServiceExec" />
    <serviceTask id="Task_ServiceExec" name="JDBC安全流式执行" />
    <sequenceFlow id="Flow_ToFinish" sourceRef="Task_ServiceExec" targetRef="End_1" />
    <endEvent id="End_1" name="变更完成归档" />
  </process>
  <bpmndi:BPMNDiagram id="BPMNDiagram_Cond">
    <bpmndi:BPMNPlane id="BPMNPlane_Cond" bpmnElement="Process_ConditionalDmlReview">
      <bpmndi:BPMNShape id="Start_1_di" bpmnElement="Start_1"><dc:Bounds x="120" y="142" width="36" height="36" /></bpmndi:BPMNShape>
      <bpmndi:BPMNShape id="Gateway_Rows_di" bpmnElement="Gateway_Rows" isMarkerVisible="true"><dc:Bounds x="210" y="135" width="50" height="50" /><bpmndi:BPMNLabel><dc:Bounds x="175" y="192" width="120" height="14" /></bpmndi:BPMNLabel></bpmndi:BPMNShape>
      <bpmndi:BPMNShape id="Task_HighRiskDba_di" bpmnElement="Task_HighRiskDba"><dc:Bounds x="350" y="55" width="175" height="80" /></bpmndi:BPMNShape>
      <bpmndi:BPMNShape id="Task_LowRiskOps_di" bpmnElement="Task_LowRiskOps"><dc:Bounds x="350" y="195" width="175" height="80" /></bpmndi:BPMNShape>
      <bpmndi:BPMNShape id="Task_ServiceExec_di" bpmnElement="Task_ServiceExec"><dc:Bounds x="600" y="125" width="150" height="80" /></bpmndi:BPMNShape>
      <bpmndi:BPMNShape id="End_1_di" bpmnElement="End_1"><dc:Bounds x="820" y="147" width="36" height="36" /></bpmndi:BPMNShape>
      <bpmndi:BPMNEdge id="Flow_ToGateway_di" bpmnElement="Flow_ToGateway"><di:waypoint x="156" y="160" /><di:waypoint x="210" y="160" /></bpmndi:BPMNEdge>
      <bpmndi:BPMNEdge id="Flow_HighRisk_di" bpmnElement="Flow_HighRisk"><di:waypoint x="235" y="135" /><di:waypoint x="235" y="95" /><di:waypoint x="350" y="95" /><bpmndi:BPMNLabel><dc:Bounds x="240" y="75" width="110" height="14" /></bpmndi:BPMNLabel></bpmndi:BPMNEdge>
      <bpmndi:BPMNEdge id="Flow_LowRisk_di" bpmnElement="Flow_LowRisk"><di:waypoint x="235" y="185" /><di:waypoint x="235" y="235" /><di:waypoint x="350" y="235" /><bpmndi:BPMNLabel><dc:Bounds x="240" y="245" width="110" height="14" /></bpmndi:BPMNLabel></bpmndi:BPMNEdge>
      <bpmndi:BPMNEdge id="Flow_DbaToExec_di" bpmnElement="Flow_DbaToExec"><di:waypoint x="525" y="95" /><di:waypoint x="560" y="95" /><di:waypoint x="560" y="165" /><di:waypoint x="600" y="165" /></bpmndi:BPMNEdge>
      <bpmndi:BPMNEdge id="Flow_OpsToExec_di" bpmnElement="Flow_OpsToExec"><di:waypoint x="525" y="235" /><di:waypoint x="560" y="235" /><di:waypoint x="560" y="165" /><di:waypoint x="600" y="165" /></bpmndi:BPMNEdge>
      <bpmndi:BPMNEdge id="Flow_ToFinish_di" bpmnElement="Flow_ToFinish"><di:waypoint x="750" y="165" /><di:waypoint x="820" y="165" /></bpmndi:BPMNEdge>
    </bpmndi:BPMNPlane>
  </bpmndi:BPMNDiagram>
</definitions>`
  }

  const activeNodes = (nodes && nodes.length > 0) ? nodes : [
    { step: 1, nodeName: '开发组长初审', role: 'DEV_LEAD' },
    { step: 2, nodeName: '核心DBA安全复审', role: 'DBA' }
  ]

  let shapesXml = ''
  let edgesXml = ''
  let processElementsXml = `    <startEvent id="Start_1" name="提交${flowType === 'DDL_CHANGE' ? 'DDL' : 'SQL'}工单" />\n`
  processElementsXml += `    <sequenceFlow id="Flow_0" sourceRef="Start_1" targetRef="Task_1" />\n`

  shapesXml += `      <bpmndi:BPMNShape id="Start_1_di" bpmnElement="Start_1"><dc:Bounds x="80" y="102" width="36" height="36" /></bpmndi:BPMNShape>\n`
  edgesXml += `      <bpmndi:BPMNEdge id="Flow_0_di" bpmnElement="Flow_0"><di:waypoint x="116" y="120" /><di:waypoint x="170" y="120" /></bpmndi:BPMNEdge>\n`

  let currentX = 170
  activeNodes.forEach((node, idx) => {
    const taskId = `Task_${idx + 1}`
    const nextTargetId = (idx === activeNodes.length - 1) ? 'Task_Exec' : `Task_${idx + 2}`
    const flowId = `Flow_${idx + 1}`

    const isSystem = node.role === 'SYSTEM' || (node.nodeName && node.nodeName.includes('自动审批'))
    const taskTag = isSystem ? 'serviceTask' : 'userTask'
    const roleSuffix = isSystem ? '(系统自动审批)' : `(${node.role || 'DEV_LEAD'})`

    processElementsXml += `    <${taskTag} id="${taskId}" name="${node.nodeName} ${roleSuffix}" />\n`
    processElementsXml += `    <sequenceFlow id="${flowId}" sourceRef="${taskId}" targetRef="${nextTargetId}" />\n`

    shapesXml += `      <bpmndi:BPMNShape id="${taskId}_di" bpmnElement="${taskId}"><dc:Bounds x="${currentX}" y="80" width="165" height="80" /></bpmndi:BPMNShape>\n`
    edgesXml += `      <bpmndi:BPMNEdge id="${flowId}_di" bpmnElement="${flowId}"><di:waypoint x="${currentX + 165}" y="120" /><di:waypoint x="${currentX + 220}" y="120" /></bpmndi:BPMNEdge>\n`

    currentX += 220
  })

  const execName = flowType === 'DDL_CHANGE' ? 'gh-ost表结构安全执行' : (flowType === 'DATA_EXPORT' ? '动态脱敏流式打包' : 'JDBC安全流式执行')
  processElementsXml += `    <serviceTask id="Task_Exec" name="${execName}" />\n`
  processElementsXml += `    <sequenceFlow id="Flow_End" sourceRef="Task_Exec" targetRef="End_1" />\n`
  processElementsXml += `    <endEvent id="End_1" name="变更完成归档" />\n`

  shapesXml += `      <bpmndi:BPMNShape id="Task_Exec_di" bpmnElement="Task_Exec"><dc:Bounds x="${currentX}" y="80" width="150" height="80" /></bpmndi:BPMNShape>\n`
  shapesXml += `      <bpmndi:BPMNShape id="End_1_di" bpmnElement="End_1"><dc:Bounds x="${currentX + 200}" y="102" width="36" height="36" /></bpmndi:BPMNShape>\n`
  edgesXml += `      <bpmndi:BPMNEdge id="Flow_End_di" bpmnElement="Flow_End"><di:waypoint x="${currentX + 150}" y="120" /><di:waypoint x="${currentX + 200}" y="120" /></bpmndi:BPMNEdge>\n`

  return `<?xml version="1.0" encoding="UTF-8"?>
<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL"
             xmlns:bpmndi="http://www.omg.org/spec/BPMN/20100524/DI"
             xmlns:dc="http://www.omg.org/spec/DD/20100524/DC"
             xmlns:di="http://www.omg.org/spec/DD/20100524/DI"
             id="Definitions_Dynamic"
             targetNamespace="http://wmdb.com/schema/bpmn">
  <process id="Process_DynamicFlow" name="${templateName || '通用审批流'}" isExecutable="true">
${processElementsXml}  </process>
  <bpmndi:BPMNDiagram id="BPMNDiagram_Dynamic">
    <bpmndi:BPMNPlane id="BPMNPlane_Dynamic" bpmnElement="Process_DynamicFlow">
${shapesXml}${edgesXml}    </bpmndi:BPMNPlane>
  </bpmndi:BPMNDiagram>
</definitions>`
}

const viewTemplateInBpmn = (tpl: any) => {
  activeTab.value = 'bpmn'
  selectedBpmnPreset.value = 'tpl_' + tpl.id
  handleBpmnPresetChange()
  ElMessage.success(`已载入模板【${tpl.templateName}】的动态 BPMN 2.0 拓扑！`)
}

const getPresetTitle = (preset: string) => {
  if (preset.startsWith('tpl_')) {
    const tplId = Number(preset.replace('tpl_', ''))
    const found = templateList.value.find(t => t.id === tplId)
    if (found) return found.templateName
  }
  const map: Record<string, string> = {
    dml_condition: 'DML 影响行数与SpEL智能条件分支审批流',
    dml_standard: '标准生产 SQL 变更审批流 (开发组长初审 ➔ 核心DBA安全复核)',
    ddl: '高危 DDL 结构变更双人复核流程 (AI锁分析 ➔ 资深DBA审核 ➔ 安全合规复核)',
    emergency: '生产紧急抢修极速放行通道 (值班DBA极速放行 ➔ 极速执行 ➔ 事后补录)',
    export: '敏感数据导出与脱敏审批流 (业务主管 ➔ 数据安全官 ➔ 动态脱敏打包)'
  }
  return map[preset] || '标准 SQL 变更审批流'
}

const handleOpenQuickBindDialog = () => {
  if (selectedBpmnPreset.value.startsWith('tpl_')) {
    const tplId = Number(selectedBpmnPreset.value.replace('tpl_', ''))
    const found = templateList.value.find(t => t.id === tplId)
    if (found) {
      quickBindResourceGroups.value = parseResourceGroups(found.resourceGroups)
      if (!found.flowType || found.flowType === 'ALL') {
        quickBindTicketTypes.value = [...ALL_TICKET_TYPES]
      } else {
        const types = found.flowType.split(',').map((s: string) => s.trim()).filter(Boolean)
        quickBindTicketTypes.value = types.length > 0 ? types : [...ALL_TICKET_TYPES]
      }
      if (found.targetDatabases && !found.targetDatabases.includes('ALL') && found.targetDatabases !== '[]') {
        try {
          const parsed = JSON.parse(found.targetDatabases)
          if (Array.isArray(parsed) && parsed.length > 0) {
            quickBindDbScopeMode.value = 'CUSTOM'
            quickBindSelectedDatabases.value = parsed
          } else {
            quickBindDbScopeMode.value = 'ALL'
            quickBindSelectedDatabases.value = []
          }
        } catch (e) {
          quickBindDbScopeMode.value = 'ALL'
          quickBindSelectedDatabases.value = []
        }
      } else {
        quickBindDbScopeMode.value = 'ALL'
        quickBindSelectedDatabases.value = []
      }
      quickBindDialogVisible.value = true
      return
    }
  }

  // 默认使用当前选中的资源组与全量工单类型
  quickBindTicketTypes.value = [...ALL_TICKET_TYPES]
  quickBindResourceGroups.value = ['车险承保资源组']
  quickBindDbScopeMode.value = 'ALL'
  quickBindSelectedDatabases.value = []
  quickBindDialogVisible.value = true
}

const handleExecuteQuickBind = async () => {
  if (quickBindTicketTypes.value.length === 0) {
    ElMessage.warning('请至少选择一个生效工单变更类型')
    return
  }
  if (quickBindResourceGroups.value.length === 0) {
    ElMessage.warning('请至少选择一个生效业务资源组')
    return
  }
  if (quickBindDbScopeMode.value === 'CUSTOM' && quickBindSelectedDatabases.value.length === 0) {
    ElMessage.warning('您已选择细化定制模式，请至少选择一个生效数据库')
    return
  }
  quickBindLoading.value = true
  try {
    const title = getPresetTitle(selectedBpmnPreset.value)
    let tplId: number | null = null
    let existingTpl: any = null
    if (selectedBpmnPreset.value.startsWith('tpl_')) {
      tplId = Number(selectedBpmnPreset.value.replace('tpl_', ''))
      existingTpl = templateList.value.find(t => t.id === tplId) as any
    }

    const flowTypeVal = quickBindTicketTypes.value.length >= ALL_TICKET_TYPES.length ? 'ALL' : quickBindTicketTypes.value.join(',')

    const payload = {
      id: tplId,
      templateName: existingTpl ? existingTpl.templateName : title.split(' (')[0],
      flowType: flowTypeVal,
      resourceGroups: JSON.stringify(quickBindResourceGroups.value),
      targetDatabases: JSON.stringify(quickBindDbScopeMode.value === 'ALL' ? ['ALL'] : quickBindSelectedDatabases.value),
      nodeConfig: existingTpl?.nodeConfig || JSON.stringify(form.value.nodes),
      conditionDimension: existingTpl?.conditionDimension || form.value.conditionDimension || 'AFFECT_ROWS',
      affectRowsThreshold: existingTpl?.affectRowsThreshold || form.value.affectRowsThreshold || 1000,
      highRiskRole: existingTpl?.highRiskRole || form.value.highRiskRole || 'DBA',
      lowRiskRole: existingTpl?.lowRiskRole || form.value.lowRiskRole || 'DEV_LEAD',
      spelExpression: existingTpl?.spelExpression || form.value.spelExpression || '#{affectRows > 1000}',
      defaultExecutionMode: existingTpl?.defaultExecutionMode || JSON.stringify(['IMMEDIATE', 'SCHEDULED']),
      triggerCondition: existingTpl?.triggerCondition || form.value.triggerCondition || '由 BPMN 2.0 引擎根据生效范围自动匹配流转',
      description: quickBindDbScopeMode.value === 'CUSTOM'
        ? `生效工单[${flowTypeVal}] · 绑定至 ${quickBindResourceGroups.value.join('、')} [定制数据库: ${quickBindSelectedDatabases.value.join(', ')}]`
        : `生效工单[${flowTypeVal}] · 绑定至 ${quickBindResourceGroups.value.join('、')} [全部业务库通用]`,
      status: 1
    }
    await request.post('/v1/workflow/template/save', payload)
    ElMessage.success('已成功将该 BPMN 流程与所选生效范围（工单类型、资源组及数据库）完成绑定生效！')
    quickBindDialogVisible.value = false
    await fetchTemplates()
  } catch (e: any) {
    ElMessage.error(e.response?.data?.message || e.message || '绑定失败')
  } finally {
    quickBindLoading.value = false
  }
}

const formatFlowType = (type: string) => {
  const map: Record<string, string> = {
    DML_CHANGE: 'DML 变更',
    DDL_CHANGE: 'DDL 结构',
    SQL_AUDIT: 'SQL 变更',
    DATA_EXPORT: '数据导出',
    DATA_QUERY: '只读查询'
  }
  return map[type] || type || '通用流程'
}

const getTypeTag = (type: string) => {
  if (type === 'DDL_CHANGE') return 'danger'
  if (type === 'DML_CHANGE') return 'primary'
  if (type === 'DATA_EXPORT') return 'warning'
  if (type === 'DATA_QUERY') return 'success'
  return 'info'
}

const handleBpmnPresetChange = () => {
  if (!bpmnModeler) return
  let xml = ''

  if (selectedBpmnPreset.value.startsWith('tpl_')) {
    const tplId = Number(selectedBpmnPreset.value.replace('tpl_', ''))
    const tpl = templateList.value.find(t => t.id === tplId)
    if (tpl) {
      const isCond = isConditionTemplate(tpl)
      const nodes = parseNodeConfig(tpl.nodeConfig)
      xml = generateBpmnXmlFromNodes(
        tpl.templateName,
        nodes,
        tpl.flowType,
        tpl.triggerCondition,
        isCond,
        tpl.conditionDimension,
        tpl.affectRowsThreshold,
        tpl.highRiskRole,
        tpl.lowRiskRole,
        tpl.spelExpression
      )
    }
  }

  if (!xml) {
    if (selectedBpmnPreset.value === 'dml_condition') {
      const condTpl = templateList.value.find(t => isConditionTemplate(t))
      if (condTpl) {
        xml = generateBpmnXmlFromNodes(
          condTpl.templateName,
          parseNodeConfig(condTpl.nodeConfig),
          condTpl.flowType,
          condTpl.triggerCondition,
          true,
          condTpl.conditionDimension,
          condTpl.affectRowsThreshold,
          condTpl.highRiskRole,
          condTpl.lowRiskRole,
          condTpl.spelExpression
        )
      } else {
        xml = generateBpmnXmlFromNodes('DML 影响行数与SpEL条件分支审批流', [], 'DML_CHANGE', '', true, 'AFFECT_ROWS', 1000, 'DBA', 'DEV_LEAD', '#{affectRows > 1000}')
      }
    } else if (selectedBpmnPreset.value === 'dml_standard') {
      xml = generateBpmnXmlFromNodes('标准生产 SQL 变更审批流', [
        { step: 1, nodeName: '开发组长初审', role: 'DEV_LEAD' },
        { step: 2, nodeName: '核心DBA安全复审', role: 'DBA' }
      ], 'DML_CHANGE')
    } else if (selectedBpmnPreset.value === 'ddl') {
      xml = generateBpmnXmlFromNodes('高危 DDL 结构变更三级双人复核流程', [
        { step: 1, nodeName: 'AI智能锁分析', role: 'AUDITOR' },
        { step: 2, nodeName: '资深DBA审核', role: 'DBA' },
        { step: 3, nodeName: '安全合规复核', role: 'ADMIN' }
      ], 'DDL_CHANGE')
    } else if (selectedBpmnPreset.value === 'emergency') {
      xml = generateBpmnXmlFromNodes('生产紧急抢修极速放行通道', [
        { step: 1, nodeName: '值班DBA极速放行', role: 'DBA' },
        { step: 2, nodeName: '事后主管补录审计', role: 'DEV_LEAD' }
      ], 'SQL_AUDIT')
    } else if (selectedBpmnPreset.value === 'export') {
      xml = generateBpmnXmlFromNodes('敏感数据导出与脱敏审批流', [
        { step: 1, nodeName: '业务部门主管审批', role: 'DEV_LEAD' },
        { step: 2, nodeName: '数据安全官审批', role: 'AUDITOR' }
      ], 'DATA_EXPORT')
    } else {
      xml = generateBpmnXmlFromNodes('标准审批流', [
        { step: 1, nodeName: '开发组长初审', role: 'DEV_LEAD' },
        { step: 2, nodeName: '核心DBA安全复审', role: 'DBA' }
      ], 'DML_CHANGE')
    }
  }

  bpmnModeler.importXML(xml).then(() => {
    bpmnModeler.get('canvas').zoom('fit-viewport')
  }).catch((err: any) => {
    console.error('Import BPMN preset error', err)
  })
  checkDeployStatus()
}

const toggleBpmnFullscreen = () => {
  isBpmnFullscreen.value = !isBpmnFullscreen.value
  setTimeout(() => {
    if (bpmnModeler) {
      bpmnModeler.get('canvas').zoom('fit-viewport')
    }
  }, 150)
  if (isBpmnFullscreen.value) {
    ElMessage.info('已进入全屏拓扑视图（可按 ESC 键或点击按钮随时退出）')
  }
}

const handleKeyDown = (e: KeyboardEvent) => {
  if (e.key === 'Escape' && isBpmnFullscreen.value) {
    isBpmnFullscreen.value = false
    setTimeout(() => {
      if (bpmnModeler) {
        bpmnModeler.get('canvas').zoom('fit-viewport')
      }
    }, 150)
  }
}

const checkDeployStatus = async () => {
  try {
    const pName = getPresetTitle(selectedBpmnPreset.value)
    const res: any = await request.get('/v1/workflow/deploy-status', {
      params: { processName: pName }
    })
    if (res.data) {
      currentDeployStatus.value = res.data
    }
  } catch (e) {
    currentDeployStatus.value = { isDeployed: false }
  }
}

const initBpmnModeler = () => {
  if (!canvas.value) return
  if (!bpmnModeler) {
    bpmnModeler = new BpmnModeler({
      container: canvas.value
    })
  }
  handleBpmnPresetChange()
}

const handleResetCanvas = () => {
  handleBpmnPresetChange()
}

const handleCopyXml = async () => {
  if (!bpmnModeler) return
  try {
    const { xml } = await bpmnModeler.saveXML({ format: true })
    await navigator.clipboard.writeText(xml)
    ElMessage.success('BPMN 2.0 XML 已复制到剪贴板！')
  } catch (error) {
    ElMessage.error('复制 XML 失败')
  }
}

const handleExportXml = async () => {
  if (!bpmnModeler) return
  try {
    const { xml } = await bpmnModeler.saveXML({ format: true })
    const blob = new Blob([xml], { type: 'application/xml' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `wmdb-${selectedBpmnPreset.value}-workflow.bpmn20.xml`
    a.click()
    URL.revokeObjectURL(url)
    ElMessage.success('BPMN 文件已导出')
  } catch (error) {
    ElMessage.error('导出文件失败')
  }
}

const deployProcess = async () => {
  if (!bpmnModeler) return
  deployLoading.value = true
  try {
    const { xml } = await bpmnModeler.saveXML({ format: true })
    const pName = getPresetTitle(selectedBpmnPreset.value)
    const res: any = await request.post('/v1/workflow/deploy', {
      processName: pName,
      bpmnXml: xml
    })
    currentDeployStatus.value = {
      isDeployed: true,
      deploymentId: res.data?.deploymentId,
      deployTime: new Date().toLocaleTimeString(),
      status: 'ACTIVE'
    }
    ElMessage.success(res.data?.message || 'BPMN 流程定义已成功挂载至 Flowable 引擎！')
  } catch (error: any) {
    ElMessage.error(error.response?.data?.message || '部署失败')
  } finally {
    deployLoading.value = false
  }
}

const terminateProcess = async () => {
  const pName = getPresetTitle(selectedBpmnPreset.value)
  try {
    await ElMessageBox.confirm(
      `确认终止并从 Flowable 引擎中卸载流程定义【${pName}】吗？终止后该流程将不再接收新工单流转。`,
      '终止流程定义确认',
      {
        type: 'warning',
        confirmButtonText: '确定终止并卸载',
        cancelButtonText: '取消',
        confirmButtonClass: 'el-button--danger'
      }
    )
    terminateLoading.value = true
    const res: any = await request.post('/v1/workflow/terminate', {
      processName: pName,
      deploymentId: currentDeployStatus.value.deploymentId
    })
    currentDeployStatus.value = {
      isDeployed: false,
      deploymentId: undefined
    }
    ElMessage.success(res.data?.message || `流程【${pName}】已成功终止并卸载下线`)
  } catch (err: any) {
    if (err !== 'cancel') {
      ElMessage.error(err.response?.data?.message || err.message || '终止流程失败')
    }
  } finally {
    terminateLoading.value = false
  }
}

onMounted(() => {
  window.addEventListener('keydown', handleKeyDown)
  fetchRoles()
  fetchTemplates()
  fetchResourceGroups()
  fetchResourceGroupDetails()
  fetchInstances()
  fetchUsers()
  setTimeout(() => {
    initBpmnModeler()
  }, 500)
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', handleKeyDown)
  if (bpmnModeler) {
    bpmnModeler.destroy()
    bpmnModeler = null
  }
})
</script>

<style scoped>
.workflow-designer-container {
  min-height: calc(100vh - 100px);
  display: flex;
  flex-direction: column;
  box-sizing: border-box;
  min-width: 0;
  padding-bottom: 24px;
}

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
  padding: 16px 20px 24px 20px;
  border-radius: 8px;
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.05);
  flex: 1;
}

.custom-tabs > :deep(.el-tabs__header) {
  position: sticky;
  top: 0px;
  z-index: 100;
  background: #ffffff;
  padding: 8px 12px 0 12px;
  border-radius: 8px;
  border: 1px solid #e2e8f0;
  margin-bottom: 16px;
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.04);
}

.template-header-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
  flex-wrap: wrap;
  gap: 12px;
}

.table-wrapper {
  background: #ffffff;
  overflow-x: auto;
  margin-bottom: 12px;
}

.resource-tags-wrap {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.rg-tag {
  font-size: 11px;
}

.node-chain-wrap {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 4px;
}

.node-step-tag {
  display: inline-flex;
  align-items: center;
  font-size: 12px;
  background: #f1f5f9;
  padding: 2px 6px;
  border-radius: 4px;
  border: 1px solid #e2e8f0;
}

.step-num {
  font-weight: 700;
  color: #409eff;
  margin-right: 4px;
}

.step-name {
  color: #1e293b;
  font-weight: 500;
}

.step-role {
  color: #64748b;
  font-size: 11px;
  margin-left: 2px;
}

.step-arrow {
  margin-left: 6px;
  color: #94a3b8;
}

.node-editor-box {
  background: #f8fafc;
  padding: 12px;
  border-radius: 6px;
  border: 1px solid #e2e8f0;
  width: 100%;
}

.node-editor-row {
  display: flex;
  align-items: center;
  margin-bottom: 8px;
}

.node-seq {
  font-weight: 600;
  color: #475569;
  width: 60px;
  font-size: 13px;
}

.bpmn-toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
  flex-wrap: wrap;
  gap: 12px;
}

.strategy-tags-group {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.strategy-tag {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  cursor: pointer;
  font-weight: 500;
}

.strategy-icon {
  font-size: 12px;
}

.strategy-cards-container {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
  gap: 10px;
  width: 100%;
}

.strategy-card {
  border: 1px solid #e2e8f0;
  border-radius: 8px;
  padding: 10px 12px;
  background: #f8fafc;
  cursor: pointer;
  transition: all 0.2s ease;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.strategy-card:hover {
  border-color: #93c5fd;
  background: #f0fdf4;
}

.strategy-card.selected {
  border-color: #3b82f6;
  background: #eff6ff;
  box-shadow: 0 2px 8px rgba(59, 130, 246, 0.12);
}

.card-top-row {
  display: flex;
  align-items: center;
  gap: 6px;
}

.card-icon {
  font-size: 15px;
}

.card-title {
  font-weight: 600;
  font-size: 13px;
  color: #1e293b;
  flex: 1;
}

.card-badge {
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 4px;
  font-weight: 600;
}

.card-desc {
  font-size: 11px;
  color: #64748b;
  line-height: 1.4;
  padding-left: 24px;
}

.bpmn-designer-panel {
  display: flex;
  flex-direction: column;
  gap: 12px;
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.bpmn-designer-panel.is-fullscreen {
  position: fixed;
  top: 0;
  left: 0;
  width: 100vw;
  height: 100vh;
  z-index: 1200;
  background: #f8fafc;
  padding: 16px 20px;
  box-sizing: border-box;
  margin: 0;
}

.bpmn-toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: 12px;
  background: #ffffff;
  padding: 12px 16px;
  border-radius: 8px;
  border: 1px solid #e2e8f0;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.05);
}

.toolbar-left {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
}

.toolbar-right {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.deploy-status-badge {
  display: flex;
  align-items: center;
}

.canvas-wrapper {
  height: 600px;
  min-height: 500px;
  border: 1px solid #dcdfe6;
  border-radius: 8px;
  background-color: #ffffff;
  overflow: hidden;
  position: relative;
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.05);
}

.bpmn-designer-panel.is-fullscreen .canvas-wrapper {
  flex: 1;
  height: calc(100vh - 85px) !important;
  min-height: calc(100vh - 85px) !important;
}

.canvas {
  width: 100%;
  height: 100%;
}

.rg-quick-actions-bar {
  display: flex;
  gap: 8px;
  margin-bottom: 8px;
  width: 100%;
}

.conditional-chain-box {
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.branch-summary-rows {
  display: flex;
  flex-direction: column;
  gap: 3px;
  font-size: 11px;
  color: #334155;
}

.branch-row {
  display: flex;
  align-items: center;
  gap: 4px;
}

.branch-tag {
  font-size: 10px;
  font-weight: 700;
  padding: 1px 5px;
  border-radius: 4px;
}

.branch-tag.red {
  background: #fef2f2;
  color: #dc2626;
  border: 1px solid #fecaca;
}

.branch-tag.green {
  background: #f0fdf4;
  color: #16a34a;
  border: 1px solid #bbf7d0;
}

.dialog-scroll-form {
  max-height: 68vh;
  overflow-y: auto;
  padding-right: 12px;
}

.gateway-config-card {
  width: 100%;
  background: #f8fafc;
  border: 1px solid #e2e8f0;
  border-radius: 8px;
  padding: 14px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.gateway-dimension-row,
.gateway-threshold-row {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.cfg-sub-label {
  font-size: 12px;
  font-weight: 600;
  color: #475569;
  min-width: 115px;
}

.unit-text {
  font-size: 13px;
  color: #64748b;
  font-weight: 500;
}

.quick-preset-chips {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-left: 8px;
}

.preset-chip {
  cursor: pointer;
  transition: all 0.15s ease;
  user-select: none;
}

.preset-chip:hover {
  background-color: #ecf5ff;
  border-color: #409eff;
  color: #409eff;
}

.branch-config-item {
  border-radius: 6px;
  padding: 10px 12px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.branch-config-item.red-branch {
  background: #fff5f5;
  border-left: 4px solid #ef4444;
  border-top: 1px solid #fee2e2;
  border-right: 1px solid #fee2e2;
  border-bottom: 1px solid #fee2e2;
}

.branch-config-item.green-branch {
  background: #f0fdf4;
  border-left: 4px solid #22c55e;
  border-top: 1px solid #dcfce7;
  border-right: 1px solid #dcfce7;
  border-bottom: 1px solid #dcfce7;
}

.branch-header {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.branch-cond-desc {
  font-size: 12px;
  color: #475569;
}

.branch-cond-desc b {
  color: #0f172a;
}

.branch-route-detail {
  font-size: 12px;
  color: #1e293b;
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 2px;
}

.route-role-label {
  font-weight: 500;
  color: #334155;
}

.route-target {
  font-weight: 600;
  color: #0f172a;
}

/* ==================== BPMN 权限组资产库与设计器双栏布局 ==================== */
.bpmn-designer-body-layout {
  display: flex;
  gap: 16px;
  height: 620px;
  width: 100%;
}

.bpmn-palette-panel {
  width: 280px;
  flex-shrink: 0;
  background: #f8fafc;
  border: 1px solid #e2e8f0;
  border-radius: 8px;
  display: flex;
  flex-direction: column;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.05);
}

.palette-panel-header {
  padding: 12px 14px;
  background: #ffffff;
  border-bottom: 1px solid #e2e8f0;
  border-top-left-radius: 8px;
  border-top-right-radius: 8px;
}

.p-title-row {
  display: flex;
  align-items: center;
  gap: 6px;
}

.p-title {
  font-size: 14px;
  font-weight: 700;
  color: #1e293b;
}

.p-sub-tip {
  font-size: 11px;
  color: #64748b;
  margin-top: 2px;
}

.palette-search-box {
  padding: 8px 12px;
  background: #ffffff;
  border-bottom: 1px solid #f1f5f9;
}

.palette-scroll-content {
  flex: 1;
  overflow-y: auto;
  padding: 10px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.palette-group {
  background: #ffffff;
  border: 1px solid #e2e8f0;
  border-radius: 6px;
  padding: 8px;
}

.group-header {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  font-weight: 700;
  color: #334155;
  margin-bottom: 8px;
}

.count-tag {
  margin-left: auto;
  font-size: 11px;
}

.group-items-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.palette-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 10px;
  background: #f8fafc;
  border: 1px solid #cbd5e1;
  border-radius: 6px;
  cursor: grab;
  transition: all 0.2s ease;
  user-select: none;
}

.palette-card:hover {
  border-color: #3b82f6;
  background: #eff6ff;
  transform: translateY(-1px);
  box-shadow: 0 2px 5px rgba(59, 130, 246, 0.15);
}

.palette-card:active {
  cursor: grabbing;
}

.palette-card-rg {
  border-left: 3px solid #f59e0b;
}

.palette-card-dev_lead {
  border-left: 3px solid #3b82f6;
}

.palette-card-dba {
  border-left: 3px solid #ef4444;
}

.palette-card-auditor {
  border-left: 3px solid #8b5cf6;
}

.palette-card-admin {
  border-left: 3px solid #f59e0b;
}

.palette-card-ops {
  border-left: 3px solid #10b981;
}

.palette-card-gateway {
  border-left: 3px solid #d97706;
  background: #fffbeb;
}

.palette-card-service {
  border-left: 3px solid #10b981;
  background: #f0fdf4;
}

.card-info {
  display: flex;
  flex-direction: column;
  overflow: hidden;
  margin-right: 6px;
}

.card-name {
  font-size: 12px;
  font-weight: 600;
  color: #0f172a;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.card-sub {
  font-size: 10px;
  color: #64748b;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.canvas-wrapper {
  flex: 1;
  height: 600px;
  min-height: 520px;
  position: relative;
  background: #ffffff;
  border: 1px solid #e2e8f0;
  border-radius: 8px;
  overflow: hidden;
}

.canvas {
  width: 100%;
  height: 100%;
}

/* 顶部 3 步编排向导引导条 */
.bpmn-wizard-steps-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: #ffffff;
  padding: 10px 20px;
  border-radius: 8px;
  border: 1px solid #e2e8f0;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.03);
}

.wizard-step {
  display: flex;
  align-items: center;
  gap: 10px;
  opacity: 0.65;
  transition: all 0.3s;
}

.wizard-step.is-active {
  opacity: 1;
}

.wizard-step.is-finish {
  opacity: 0.9;
}

.step-badge {
  width: 24px;
  height: 24px;
  border-radius: 50%;
  background: #e2e8f0;
  color: #475569;
  font-weight: 700;
  font-size: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.wizard-step.is-active .step-badge {
  background: #3b82f6;
  color: #ffffff;
  box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.2);
}

.wizard-step.is-finish .step-badge {
  background: #10b981;
  color: #ffffff;
}

.step-texts {
  display: flex;
  flex-direction: column;
}

.step-main {
  font-size: 13px;
  font-weight: 600;
  color: #1e293b;
}

.step-sub {
  font-size: 11px;
  color: #64748b;
}

.wizard-arrow {
  color: #cbd5e1;
  font-weight: 700;
  font-size: 14px;
}

/* 资源组卡片选中高亮 */
.palette-card-rg.is-selected-rg {
  background: #eff6ff !important;
  border-color: #93c5fd !important;
  border-left-width: 4px !important;
  box-shadow: 0 2px 6px rgba(59, 130, 246, 0.12) !important;
}

/* 浮动实时生效上下文仪表盘 (全屏及普通模式) */
.floating-context-card {
  position: absolute;
  top: 14px;
  right: 16px;
  z-index: 20;
  pointer-events: auto;
}

.context-pill {
  display: flex;
  align-items: center;
  gap: 10px;
  background: rgba(255, 255, 255, 0.96);
  backdrop-filter: blur(8px);
  border: 1px solid #e2e8f0;
  padding: 8px 14px;
  border-radius: 30px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
  transition: all 0.3s;
}

.context-pill.is-db-mode {
  border-color: #86efac;
  background: rgba(240, 253, 244, 0.96);
}

.context-icon {
  font-size: 18px;
}

.context-content {
  display: flex;
  flex-direction: column;
}

.context-title {
  font-size: 10px;
  color: #64748b;
  font-weight: 600;
  text-transform: uppercase;
}

.context-val {
  font-size: 12px;
  font-weight: 700;
  color: #0f172a;
}

/* 数据库选择药丸标签样式 */
.db-select-pill {
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  font-size: 11px !important;
  user-select: none;
}

.db-select-pill:hover {
  transform: translateY(-1px);
  box-shadow: 0 2px 5px rgba(0, 0, 0, 0.1);
}

.db-select-pill.all-pill {
  border-radius: 12px;
}

.db-select-pill.is-active-db {
  font-weight: 600;
  box-shadow: 0 2px 6px rgba(16, 185, 129, 0.25);
}
</style>
