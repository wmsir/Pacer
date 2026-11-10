<template>
  <div class="create-ticket-workbench page-container">
    <!-- 顶部导航与全局操作 -->
    <div class="workbench-header">
      <div class="header-left">
        <el-button class="back-btn" :icon="ArrowLeft" circle title="返回工单中心" @click="handleBack" />
        <div class="header-titles">
          <div class="crumb-line">
            <span class="crumb-link" @click="handleBack">SQL 工单中心</span>
            <span class="crumb-sep">/</span>
            <span>新建工单</span>
          </div>
          <h2 class="main-title">新建 SQL 变更工单</h2>
          <span class="sub-title">多库变更审批 · DML 影响行数事务级预检 · 回滚方案安全审计</span>
        </div>
      </div>
      <div class="header-right">
        <span class="hdr-pill" :class="safetyPolicies.enforceDryRun ? 'hdr-pill-danger' : 'hdr-pill-muted'">
          🛡️ {{ safetyPolicies.enforceDryRun ? '强制预执行校验' : '推荐预执行校验' }}
        </span>
        <span class="hdr-pill hdr-pill-success">
          <span class="hdr-dot"></span>当前环境：{{ currentEnvTag }}
        </span>
        <el-button :icon="CopyDocument" @click="openInNewWindow">新窗口打开</el-button>
      </div>
    </div>

    <!-- 吸顶步骤导航：显示各环节完成度，点击快速定位 -->
    <div class="create-step-nav">
      <template v-for="(s, sIdx) in creationSteps" :key="s.key">
        <div
          class="step-nav-item"
          :class="{ 'is-done': s.done, 'is-required-pending': !s.done && s.required }"
          @click="scrollToSection(s.anchor)"
        >
          <span class="step-nav-circle">
            <el-icon v-if="s.done"><Check /></el-icon>
            <template v-else>{{ sIdx + 1 }}</template>
          </span>
          <span class="step-nav-text">
            <span class="step-nav-title">{{ s.title }}</span>
            <span class="step-nav-desc">{{ s.done ? '已完成' : (s.required ? '待完成' : '可选') }}</span>
          </span>
        </div>
        <div v-if="sIdx < creationSteps.length - 1" class="step-nav-line" :class="{ 'is-done': s.done }"></div>
      </template>
      <div class="step-nav-progress">
        <span class="progress-num">{{ creationProgress }}%</span>
        <span class="progress-label">就绪度</span>
      </div>
    </div>

    <!-- 重新发起 / 复制工单提示条 -->
    <el-alert
      v-if="cloneSourceTicket"
      type="success"
      show-icon
      :closable="false"
      style="margin-bottom: 16px; border: 1px solid #67C23A;"
    >
      <template #title>
        <div style="display: flex; align-items: center; justify-content: space-between; width: 100%;">
          <span>🔄 <b>【重新发起 / 复制申请模式】</b> 已为您自动加载历史工单 <b>#{{ cloneSourceTicket.id }}</b> (状态: {{ getTicketStatusText(cloneSourceTicket.status) }}) 的全部配置、待执行 SQL 与回滚方案！</span>
          <el-button size="small" type="primary" link @click="cloneSourceTicket = null">清除提示</el-button>
        </div>
      </template>
    </el-alert>

    <!-- 主体：新建 SQL 变更工单流程链路 -->
    <div class="ticket-creation-stream">
          <!-- 1. 基础信息、目标数据库与工单属性设定 -->
          <div class="stream-section-card" id="form-item-basic" :class="{ 'is-collapsed-card': isBasicInfoCollapsed }">
            <div class="section-badge-row">
              <div class="section-badge" style="margin-bottom: 0; padding-bottom: 0; border-bottom: none;">
                <span class="badge-num">1</span>
                <span class="badge-title">基础信息、目标数据库与工单属性设定</span>
                <el-tag
                  size="small"
                  :type="creationSteps[0]?.done ? 'success' : 'warning'"
                  effect="light"
                  style="margin-left: 6px; font-weight: 600;"
                >
                  {{ creationSteps[0]?.done ? '✓ 基础配置已就绪' : '待完善必填项' }}
                </el-tag>
              </div>
              <div class="section-header-actions">
                <el-button
                  v-if="routingPreview && !isBasicInfoCollapsed"
                  size="small"
                  text
                  bg
                  @click="isBpmnPreviewCollapsed = !isBpmnPreviewCollapsed"
                >
                  <el-icon style="margin-right: 4px;"><Share /></el-icon>
                  {{ isBpmnPreviewCollapsed ? '展开审批流转图' : '收起审批流转图' }}
                </el-button>
                <el-button
                  size="small"
                  :type="isBasicInfoCollapsed ? 'primary' : 'default'"
                  plain
                  :icon="isBasicInfoCollapsed ? ArrowDown : ArrowUp"
                  @click="isBasicInfoCollapsed = !isBasicInfoCollapsed"
                >
                  {{ isBasicInfoCollapsed ? '展开基础信息' : '收起基础信息' }}
                </el-button>
              </div>
            </div>

            <!-- 收起状态下的精简摘要栏 -->
            <div v-if="isBasicInfoCollapsed" class="basic-collapsed-summary" @click="isBasicInfoCollapsed = false">
              <div class="summary-Left-pills">
                <span class="summary-chip">
                  <span class="chip-k">资源组</span>
                  <b class="chip-v">{{ form.resourceGroup || '未选择' }}</b>
                </span>
                <span class="summary-sep">/</span>
                <span class="summary-chip">
                  <span class="chip-k">目标实例</span>
                  <b class="chip-v">{{ currentSelectedInstance ? `${currentSelectedInstance.name} (${currentSelectedInstance.env || 'PROD'})` : '未选择' }}</b>
                </span>
                <span class="summary-sep">/</span>
                <span class="summary-chip">
                  <span class="chip-k">执行库</span>
                  <b class="chip-v chip-mono">{{ form.dbName || '未选择' }}</b>
                </span>
                <span class="summary-sep">/</span>
                <span class="summary-chip">
                  <span class="chip-k">工单类型</span>
                  <b class="chip-v">{{ ticketTypeLabel }}</b>
                </span>
                <template v-if="routingPreview">
                  <span class="summary-sep">/</span>
                  <span class="summary-chip summary-chip-flow">
                    <el-icon><Share /></el-icon>
                    <span>{{ routingPreview.templateName }}</span>
                  </span>
                </template>
              </div>
              <span class="summary-expand-hint">点击展开修改 <el-icon><ArrowDown /></el-icon></span>
            </div>

            <div v-show="!isBasicInfoCollapsed" class="basic-section-body">
            <el-form :model="form" label-position="top" class="custom-form compact-basic-form">
              <el-row :gutter="16">
                <!-- 所属业务资源组 -->
                <el-col :xs="24" :sm="12" :md="6">
                  <el-form-item label="所属业务资源组" required>
                    <el-select
                      v-model="form.resourceGroup"
                      placeholder="选择业务资源组"
                      style="width: 100%;"
                      filterable
                      @change="handleResourceGroupChange"
                    >
                      <el-option
                        v-for="rg in availableResourceGroups"
                        :key="rg"
                        :label="rg"
                        :value="rg"
                      >
                        <div style="display: flex; justify-content: space-between; align-items: center;">
                          <span style="font-weight: 500;">{{ rg }}</span>
                          <el-tag
                            v-if="currentUserResourceGroups.includes(rg)"
                            size="small"
                            type="success"
                            effect="plain"
                          >
                            当前用户归属
                          </el-tag>
                        </div>
                      </el-option>
                    </el-select>
                    <div class="field-hint" v-if="currentUserResourceGroups.length > 0">
                      默认归属：<b>{{ currentUserResourceGroups.join('、') }}</b>
                    </div>
                  </el-form-item>
                </el-col>

                <!-- 目标数据库实例 -->
                <el-col :xs="24" :sm="12" :md="7" id="form-item-instance">
                  <el-form-item label="目标数据库实例" required>
                    <div style="display: flex; gap: 6px; width: 100%;">
                      <el-select
                        v-model="form.instanceId"
                        placeholder="请选择目标数据库实例"
                        style="flex: 1; min-width: 0;"
                        @change="handleInstanceChange"
                        :loading="instancesLoading"
                        :disabled="filteredInstances.length === 0"
                        :no-data-text="`资源组【${form.resourceGroup || '未选择'}】下暂无已授权数据库实例`"
                      >
                        <el-option
                          v-for="inst in filteredInstances"
                          :key="inst.id"
                          :label="`${inst.name} (${inst.env || 'PROD'} · ${inst.dbType || 'mysql'})`"
                          :value="inst.id"
                        >
                          <div style="display: flex; justify-content: space-between; align-items: center;">
                            <span style="font-weight: 600; color: #1e293b;">{{ inst.name }}</span>
                            <div style="display: flex; gap: 6px; align-items: center;">
                              <el-tag size="small" :type="inst.env === 'PROD' ? 'danger' : 'info'">{{ inst.env || 'PROD' }}</el-tag>
                              <span style="color: #64748b; font-size: 12px;">{{ inst.dbType }}</span>
                            </div>
                          </div>
                        </el-option>
                      </el-select>
                      <el-tooltip content="同步该实例最新 DML/DDL 管控权限" placement="top">
                        <el-button :icon="Refresh" @click="refreshInstancePermissions" :loading="instancesLoading" plain title="同步权限" />
                      </el-tooltip>
                    </div>
                    <div class="field-hint" v-if="currentSelectedInstance">
                      环境：<b>{{ currentSelectedInstance.env || 'PROD' }}</b> · 引擎：<b>{{ currentSelectedInstance.dbType || 'MySQL' }}</b>
                    </div>
                  </el-form-item>
                </el-col>

                <!-- 目标数据库 -->
                <el-col :xs="24" :sm="12" :md="5" id="form-item-dbname">
                  <el-form-item label="目标数据库" required>
                    <el-select
                      v-model="form.dbName"
                      placeholder="请选择具体执行数据库"
                      style="width: 100%;"
                      filterable
                      :loading="databasesLoading"
                      :disabled="!form.instanceId || filteredInstances.length === 0"
                      no-data-text="未探测到数据库，请先选择实例"
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
                    <div class="field-hint" style="display: flex; align-items: center; gap: 4px;">
                      <span>已探测</span>
                      <el-tag
                        :type="availableDatabases.length > 0 ? 'primary' : 'info'"
                        size="small"
                        effect="dark"
                        class="number-highlight-pill"
                      >
                        {{ availableDatabases.length }}
                      </el-tag>
                      <span>个业务库</span>
                    </div>
                  </el-form-item>
                </el-col>

                <!-- 工单类型与 SQL 细分类型 -->
                <el-col :xs="24" :sm="12" :md="6">
                  <el-form-item label="工单类型与变更模式" required>
                    <div style="display: flex; gap: 6px; width: 100%;">
                      <el-select v-model="form.type" placeholder="工单类型" style="flex: 1.2; min-width: 0;">
                        <el-option label="SQL 变更审核" value="SQL_AUDIT" />
                        <el-option label="敏感数据导出" value="DATA_EXPORT" />
                        <el-option label="权限与账号申请" value="PERMISSION" />
                        <el-option label="应急数据修复" value="DATA_RECOVERY" />
                      </el-select>
                      <el-select
                        v-if="form.type === 'SQL_AUDIT'"
                        v-model="form.sqlSubtype"
                        placeholder="细分类型"
                        style="flex: 1; min-width: 0;"
                      >
                        <el-option label="🤖 自动检测" value="AUTO" />
                        <el-option label="📝 纯 DML" value="DML_CHANGE" />
                        <el-option label="⚠️ 含 DDL" value="DDL_CHANGE" />
                      </el-select>
                    </div>
                    <div v-if="form.type === 'SQL_AUDIT' && form.sqlSubtype === 'AUTO' && detectedSqlSubtype" class="field-hint" style="color: #2563eb; font-weight: 600;">
                      {{ detectedSqlSubtype.icon }} 智能识别：{{ detectedSqlSubtype.label }}
                    </div>
                    <div v-else class="field-hint">
                      根据类型与影响行数自动匹配审批流
                    </div>
                  </el-form-item>
                </el-col>
              </el-row>

              <el-row :gutter="16" style="align-items: stretch;">
                <!-- 申请原因 / 变更说明 -->
                <el-col :xs="24" :sm="12">
                  <el-form-item label="申请原因 / 变更说明" style="margin-bottom: 8px;">
                    <el-input
                      v-model="form.reason"
                      type="textarea"
                      :rows="2"
                      placeholder="请简要描述本次变更的业务诉求与发布背景..."
                    />
                  </el-form-item>
                </el-col>

                <!-- 实例业务标签与支持操作管控展示（合并去重紧凑版） -->
                <el-col :xs="24" :sm="12">
                  <div v-if="currentSelectedInstance" class="instance-security-meta-card">
                    <div class="meta-row">
                      <span class="meta-label">🏷️ 实例业务标签：</span>
                      <div class="meta-tag-list">
                        <el-tag
                          v-for="t in parseInstanceTags(currentSelectedInstance.tags)"
                          :key="t"
                          size="small"
                          :type="getTagColorType(t)"
                          effect="light"
                        >
                          {{ t }}
                        </el-tag>
                        <span v-if="parseInstanceTags(currentSelectedInstance.tags).length === 0" style="color: #94a3b8; font-size: 12px;">核心业务生产库</span>
                      </div>
                    </div>
                    <div class="meta-row">
                      <span class="meta-label">🛡️ 授权管控操作：</span>
                      <div class="meta-tag-list">
                        <el-tag
                          v-for="op in currentInstanceSupportedOps"
                          :key="op"
                          size="small"
                          :type="op.includes('上线') ? 'success' : (op.includes('查询') ? 'primary' : (op.includes('DML') ? 'warning' : (op.includes('DDL') ? 'danger' : 'info')))"
                          effect="light"
                        >
                          {{ op }}
                        </el-tag>
                      </div>
                    </div>
                  </div>
                  <div v-else-if="filteredInstances.length === 0" style="margin-top: 4px;">
                    <el-alert
                      type="warning"
                      :closable="false"
                      show-icon
                      :title="`资源组【${form.resourceGroup || '未选择'}】下暂未授权任何数据库实例`"
                      description="请在【实例管理】中为数据库实例绑定该资源组，或切换其他业务资源组。"
                    />
                  </div>
                  <div v-else style="margin-top: 4px;">
                    <div class="field-hint" style="padding: 10px; background: #f8fafc; border-radius: 6px; border: 1px dashed #e2e8f0;">
                      👈 请选择目标数据库实例以查看实例标签与操作管控策略
                    </div>
                  </div>

                  <!-- 操作权限拦截警告 -->
                  <el-alert
                    v-if="currentSelectedInstance && !isCurrentTicketTypeSupported.supported"
                    :title="isCurrentTicketTypeSupported.message"
                    type="error"
                    show-icon
                    :closable="false"
                    style="margin-top: 8px;"
                  />
                </el-col>
              </el-row>

              <!-- 业务资源组个性化定制扩展字段 (按资源组配置动态呈现) -->
              <template v-if="currentResourceGroupFields.length > 0">
                <el-divider content-position="left" style="margin: 14px 0 12px 0;">
                  <span style="font-size: 12.5px; font-weight: 600; color: #2563eb;">
                    📌 资源组【{{ form.resourceGroup }}】工单定制属性与执行约束
                  </span>
                </el-divider>

                <el-row :gutter="16">
                  <el-col
                    v-for="field in currentResourceGroupFields"
                    :key="field.fieldKey"
                    :id="'custom-field-' + field.fieldKey"
                    :xs="24"
                    :sm="field.fieldType === 'TEXTAREA' ? 24 : 8"
                    class="custom-field-col"
                  >
                    <el-form-item :label="field.fieldName" :required="field.required" style="margin-bottom: 10px;">
                      <!-- 文本 / 版本号输入 -->
                      <el-input
                        v-if="field.fieldType === 'TEXT'"
                        v-model="customForm[field.fieldKey]"
                        :placeholder="field.placeholder || `请输入${field.fieldName}`"
                        clearable
                      />

                      <!-- 多行文本 / 预案说明输入框 -->
                      <div v-else-if="field.fieldType === 'TEXTAREA'" class="custom-textarea-wrapper">
                        <el-input
                          v-model="customForm[field.fieldKey]"
                          type="textarea"
                          :rows="2"
                          :placeholder="field.placeholder || `请输入${field.fieldName}`"
                          class="custom-field-textarea"
                          show-word-limit
                          maxlength="1000"
                          resize="vertical"
                        />
                        <div class="textarea-tip">
                          <el-icon style="color: #f59e0b; margin-right: 4px;"><Warning /></el-icon>
                          请简要描述异常应对措施、回滚步骤及责任人信息。
                        </div>
                      </div>

                      <!-- 年月日日期选择器 -->
                      <el-date-picker
                        v-else-if="field.fieldType === 'DATE'"
                        v-model="customForm[field.fieldKey]"
                        type="date"
                        format="YYYY-MM-DD"
                        value-format="YYYY-MM-DD"
                        :placeholder="field.placeholder || '请选择计划上线年月日 (YYYY-MM-DD)'"
                        style="width: 100%;"
                      />

                      <!-- 允许执行时间窗口 (年月日 时分秒 范围选择器) -->
                      <el-date-picker
                        v-else-if="field.fieldType === 'DATETIME_RANGE' || field.fieldType === 'TIME_RANGE'"
                        v-model="customForm[field.fieldKey]"
                        type="datetimerange"
                        format="YYYY-MM-DD HH:mm:ss"
                        value-format="YYYY-MM-DD HH:mm:ss"
                        range-separator="至"
                        start-placeholder="开始时间"
                        end-placeholder="结束时间"
                        style="width: 100%;"
                      />
                    </el-form-item>
                  </el-col>
                </el-row>
              </template>
            </el-form>

            <!-- 审批流程与流转节点预估看板 (BPMN 2.0 动态流程分支全景) -->
            <div v-if="routingPreview" class="routing-preview-card" :class="{ 'is-pinned-flow': routingPreview.isPinned }">
              <div class="routing-card-header">
                <div class="preview-header-left">
                  <div class="preview-icon-badge" :class="{ 'is-pinned': routingPreview.isPinned }">
                    <el-icon :size="15"><Share /></el-icon>
                  </div>
                  <span class="preview-title">当前匹配 BPMN 审批流程：<b>{{ routingPreview.templateName }}</b></span>
                </div>
                <div class="preview-header-right-tags">
                  <span class="bpmn-top-pill" :class="isDryRunEnforced ? 'pill-danger' : 'pill-success'">
                    🛡️ 预检策略：{{ isDryRunEnforced ? '强制预执行校验 (必须通过)' : '推荐预执行校验 (允许跳过)' }}
                  </span>
                  <span v-if="routingPreview.spelExpression" class="bpmn-top-pill pill-spel">
                    SpEL: {{ routingPreview.spelExpression }}
                  </span>
                  <span class="bpmn-top-pill" :class="routingPreview.isPinned ? 'pill-pinned' : 'pill-smart'">
                    {{ routingPreview.isPinned ? '🔥 专属固定审批流 (专库专流)' : '⚡ 动态综合智能决策' }}
                  </span>
                  <el-button
                    size="small"
                    link
                    type="primary"
                    @click="isBpmnPreviewCollapsed = !isBpmnPreviewCollapsed"
                  >
                    {{ isBpmnPreviewCollapsed ? '展开流转画布' : '收起画布' }}
                  </el-button>
                </div>
              </div>

              <!-- 结构化判定依据栏与实时命中路径指示器 -->
              <div class="decision-basis-bar">
                <div class="basis-left-items">
                  <span class="basis-main-label">🎯 判定依据：</span>
                  <template v-if="parsedRoutingBasis.structured">
                    <span class="basis-chip">
                      资源组 <b>{{ parsedRoutingBasis.resourceGroup }}</b>
                    </span>
                    <span class="basis-plus">+</span>
                    <span class="basis-chip">
                      变更类型 <b>{{ parsedRoutingBasis.changeType }}</b>
                    </span>
                    <template v-if="parsedRoutingBasis.tags.length > 0">
                      <span class="basis-plus">+</span>
                      <span class="basis-chip">
                        实例标签
                        <span v-for="t in parsedRoutingBasis.tags" :key="t" class="basis-mini-tag">{{ t }}</span>
                      </span>
                    </template>
                    <span v-if="parsedRoutingBasis.thresholdNote" class="basis-chip basis-chip-rule">
                      📏 {{ parsedRoutingBasis.thresholdNote }}
                    </span>
                  </template>
                  <template v-else>
                    <span class="basis-raw-text">{{ routingPreview.routingReason }}</span>
                  </template>
                  <span v-if="routingPreview.triggerCondition" class="basis-chip basis-chip-trigger">
                    💡 触发规则：{{ routingPreview.triggerCondition }}
                  </span>
                </div>

                <!-- 实时分支流向指示与沙盘预览切换 (仅排他网关显示) -->
                <div v-if="routingPreview.isGateway" class="basis-right-controls">
                  <div class="active-route-pill" :class="isGatewayHighRiskActive ? 'route-pill-high' : 'route-pill-normal'">
                    <span class="route-pulse-dot" :style="{ background: isGatewayHighRiskActive ? '#dc2626' : '#2563eb' }"></span>
                    <span>
                      当前实时流向：{{ isGatewayHighRiskActive ? '左分支 · 核心DBA安全复核 (高危)' : '右分支 · 业务开发组长初审 (常规)' }}
                    </span>
                  </div>
                  <div class="branch-preview-switcher" title="点击可切换查看不同分支命中时的节点流转状态">
                    <button
                      type="button"
                      class="sw-btn"
                      :class="{ active: gatewayPreviewMode === 'auto' }"
                      @click="gatewayPreviewMode = 'auto'"
                    >自动跟随</button>
                    <button
                      type="button"
                      class="sw-btn sw-normal"
                      :class="{ active: gatewayPreviewMode === 'normal' }"
                      @click="gatewayPreviewMode = 'normal'"
                    >⚡ 常规流</button>
                    <button
                      type="button"
                      class="sw-btn sw-high"
                      :class="{ active: gatewayPreviewMode === 'high' }"
                      @click="gatewayPreviewMode = 'high'"
                    >🔥 高危流</button>
                  </div>
                </div>
              </div>

              <!-- 模式 A：BPMN 2.0 分支图形可视化看板 (5 阶段全景高清流转图) -->
              <div v-if="routingPreview.isGateway" v-show="!isBpmnPreviewCollapsed" class="bpmn-flow-visual-box">
                <!-- 顶部 5 阶段步骤导航轨 -->
                <div class="bpmn-stage-header-track">
                  <div class="stage-col-head"><span class="stage-step-num">01</span> 变更发起</div>
                  <div></div>
                  <div class="stage-col-head"><span class="stage-step-num">02</span> 智能排他网关</div>
                  <div></div>
                  <div class="stage-col-head"><span class="stage-step-num">03</span> 条件分支审批 (二选一路由)</div>
                  <div></div>
                  <div class="stage-col-head"><span class="stage-step-num">04</span> 引擎事务执行</div>
                  <div></div>
                  <div class="stage-col-head"><span class="stage-step-num">05</span> 完结归档</div>
                </div>

                <!-- 5 阶段主流程网格 -->
                <div class="bpmn-stage-grid">
                  <!-- 阶段 01：起点 -->
                  <div class="bpmn-node-pill bpmn-start-pill">
                    <div class="pill-main-row">
                      <div class="bpmn-circle start-circle"></div>
                      <span class="bpmn-label">提交变更申请</span>
                    </div>
                    <span class="pill-sub-text">工单创建与预检触发</span>
                  </div>

                  <!-- 连线 1：起点 -> 排他网关 -->
                  <div class="bpmn-svg-arrow-wrap">
                    <svg class="bpmn-flow-svg-line" width="44" height="36" viewBox="0 0 44 36">
                      <line x1="2" y1="18" x2="34" y2="18" stroke="#10b981" stroke-width="2.5" class="bpmn-flow-dash-anim" />
                      <polygon points="33,13 42,18 33,23" fill="#10b981" />
                    </svg>
                  </div>

                  <!-- 阶段 02：SpEL 排他条件网关 -->
                  <div class="bpmn-gateway-card">
                    <div class="bpmn-gateway-diamond">
                      <span class="diamond-x">✕</span>
                    </div>
                    <div class="gw-card-title">SpEL 排他网关</div>
                    <code class="gw-spel-pill">{{ routingPreview.spelExpression || '#{affectRows > 1000 || hasDdl == true}' }}</code>
                  </div>

                  <!-- 分叉导向 SVG 连线 (带 TRUE / FALSE 判定标签与动态流光) -->
                  <div class="bpmn-branch-fork-svg-wrap">
                    <svg class="bpmn-fork-svg" width="68" height="158" viewBox="0 0 68 158">
                      <!-- 上分支 (高危 TRUE) -->
                      <path
                        d="M 2 79 C 30 79, 28 34, 58 34"
                        fill="none"
                        :stroke="isGatewayHighRiskActive ? '#dc2626' : '#cbd5e1'"
                        :stroke-width="isGatewayHighRiskActive ? 3 : 2"
                        :class="isGatewayHighRiskActive ? 'bpmn-flow-dash-anim' : ''"
                        stroke-dasharray="5,3"
                      />
                      <polygon
                        points="56,29 66,34 56,39"
                        :fill="isGatewayHighRiskActive ? '#dc2626' : '#94a3b8'"
                      />
                      <rect x="10" y="18" width="36" height="15" rx="4" :fill="isGatewayHighRiskActive ? '#dc2626' : '#fee2e2'" :stroke="isGatewayHighRiskActive ? '#dc2626' : '#fca5a5'" stroke-width="1" />
                      <text x="28" y="28.5" font-size="9" font-weight="800" :fill="isGatewayHighRiskActive ? '#ffffff' : '#dc2626'" text-anchor="middle">TRUE</text>

                      <!-- 下分支 (常规 FALSE) -->
                      <path
                        d="M 2 79 C 30 79, 28 124, 58 124"
                        fill="none"
                        :stroke="!isGatewayHighRiskActive ? '#2563eb' : '#cbd5e1'"
                        :stroke-width="!isGatewayHighRiskActive ? 3 : 2"
                        :class="!isGatewayHighRiskActive ? 'bpmn-flow-dash-anim' : ''"
                        stroke-dasharray="5,3"
                      />
                      <polygon
                        points="56,119 66,124 56,129"
                        :fill="!isGatewayHighRiskActive ? '#2563eb' : '#94a3b8'"
                      />
                      <rect x="9" y="124" width="38" height="15" rx="4" :fill="!isGatewayHighRiskActive ? '#2563eb' : '#dbeafe'" :stroke="!isGatewayHighRiskActive ? '#2563eb' : '#93c5fd'" stroke-width="1" />
                      <text x="28" y="134.5" font-size="9" font-weight="800" :fill="!isGatewayHighRiskActive ? '#ffffff' : '#1d4ed8'" text-anchor="middle">FALSE</text>
                    </svg>
                  </div>

                  <!-- 阶段 03：上下双分支任务卡片 -->
                  <div class="bpmn-branches-col">
                    <!-- 上分支：高危分支 (DBA) -->
                    <div
                      class="bpmn-branch-card branch-card-high"
                      :class="isGatewayHighRiskActive ? 'is-active-high' : 'is-standby-branch'"
                    >
                      <div class="branch-card-topbar">
                        <span class="branch-tag-label high-label">
                          🔥 左分支 (高危): <code>{{ routingPreview.spelExpression || '#{affectRows > 1000 || hasDdl == true}' }}</code>
                        </span>
                        <span class="branch-hit-badge" :class="isGatewayHighRiskActive ? 'badge-hit-high' : 'badge-standby'">
                          {{ isGatewayHighRiskActive ? '🔥 当前命中路径' : '候选分支' }}
                        </span>
                      </div>
                      <div class="branch-card-body">
                        <div class="task-icon-col icon-box-dba">
                          <el-icon :size="18" color="#dc2626"><UserFilled /></el-icon>
                        </div>
                        <div class="task-title-col">
                          <div class="t-name-row">
                            <span class="t-name">核心DBA安全复核</span>
                            <span class="role-code-chip chip-dba">{{ routingPreview.highRiskRole || 'DBA' }}</span>
                          </div>
                          <div class="t-role">具备数据库最高执行与否决特权</div>
                        </div>
                      </div>
                    </div>

                    <!-- 下分支：常规分支 (DEV_LEAD) -->
                    <div
                      class="bpmn-branch-card branch-card-normal"
                      :class="!isGatewayHighRiskActive ? 'is-active-normal' : 'is-standby-branch'"
                    >
                      <div class="branch-card-topbar">
                        <span class="branch-tag-label normal-label">
                          ⚡ 右分支 (常规): 默认安全放行流 (行数 ≤ {{ routingPreview.affectRowsThreshold || 1000 }})
                        </span>
                        <span class="branch-hit-badge" :class="!isGatewayHighRiskActive ? 'badge-hit-normal' : 'badge-standby'">
                          {{ !isGatewayHighRiskActive ? '✓ 当前命中路径' : '候选分支' }}
                        </span>
                      </div>
                      <div class="branch-card-body">
                        <div class="task-icon-col icon-box-lead">
                          <el-icon :size="18" color="#2563eb"><User /></el-icon>
                        </div>
                        <div class="task-title-col">
                          <div class="t-name-row">
                            <span class="t-name">业务开发组长初审</span>
                            <span class="role-code-chip chip-lead">{{ routingPreview.lowRiskRole || 'DEV_LEAD' }}</span>
                          </div>
                          <div class="t-role">业务逻辑与日常常规变更初审</div>
                        </div>
                      </div>
                    </div>
                  </div>

                  <!-- 汇聚导向 SVG 连线 -->
                  <div class="bpmn-branch-join-svg-wrap">
                    <svg class="bpmn-join-svg" width="68" height="158" viewBox="0 0 68 158">
                      <!-- 上分支汇聚连线 -->
                      <path
                        d="M 2 34 C 34 34, 34 79, 58 79"
                        fill="none"
                        :stroke="isGatewayHighRiskActive ? '#10b981' : '#cbd5e1'"
                        :stroke-width="isGatewayHighRiskActive ? 3 : 2"
                        :class="isGatewayHighRiskActive ? 'bpmn-flow-dash-anim' : ''"
                        stroke-dasharray="5,3"
                      />
                      <!-- 下分支汇聚连线 -->
                      <path
                        d="M 2 124 C 34 124, 34 79, 58 79"
                        fill="none"
                        :stroke="!isGatewayHighRiskActive ? '#10b981' : '#cbd5e1'"
                        :stroke-width="!isGatewayHighRiskActive ? 3 : 2"
                        :class="!isGatewayHighRiskActive ? 'bpmn-flow-dash-anim' : ''"
                        stroke-dasharray="5,3"
                      />
                      <polygon points="56,74 66,79 56,84" fill="#10b981" />
                    </svg>
                  </div>

                  <!-- 阶段 04：JDBC 流式执行 -->
                  <div class="bpmn-task-card task-service">
                    <div class="task-icon-col icon-box-service">
                      <el-icon :size="18" color="#16a34a"><Setting /></el-icon>
                    </div>
                    <div class="task-title-col">
                      <div class="t-name-row">
                        <span class="t-name">JDBC安全流式执行</span>
                      </div>
                      <div class="t-role">底层事务控制与自动备份保护</div>
                    </div>
                  </div>

                  <!-- 连线 2：JDBC执行 -> 归档完成 -->
                  <div class="bpmn-svg-arrow-wrap">
                    <svg class="bpmn-flow-svg-line" width="44" height="36" viewBox="0 0 44 36">
                      <line x1="2" y1="18" x2="34" y2="18" stroke="#10b981" stroke-width="2.5" class="bpmn-flow-dash-anim" />
                      <polygon points="33,13 42,18 33,23" fill="#10b981" />
                    </svg>
                  </div>

                  <!-- 阶段 05：归档完成 -->
                  <div class="bpmn-node-pill bpmn-end-pill">
                    <div class="pill-main-row">
                      <div class="bpmn-circle end-circle"></div>
                      <span class="bpmn-label">变更完成归档</span>
                    </div>
                    <span class="pill-sub-text">审计留痕与通知推送</span>
                  </div>
                </div>
              </div>

              <!-- 模式 B：顺序多级审批流 / 测试免审直通流 / 递进混合审批流 (清晰线性管道) -->
              <div v-else v-show="!isBpmnPreviewCollapsed" class="bpmn-flow-visual-box bpmn-pipeline-flow">
                <div class="bpmn-linear-track">
                  <!-- 1. 起点 -->
                  <div class="bpmn-node-pill bpmn-start-pill">
                    <div class="pill-main-row">
                      <div class="bpmn-circle start-circle"></div>
                      <span class="bpmn-label">提交变更申请</span>
                    </div>
                    <span class="pill-sub-text">阶段 01 · 发起</span>
                  </div>

                  <!-- 动态解析节点 -->
                  <template v-for="(node, nIdx) in (routingPreview.nodes || []).slice(1)" :key="nIdx">
                    <div class="bpmn-svg-arrow-wrap">
                      <svg class="bpmn-flow-svg-line" width="40" height="28" viewBox="0 0 40 28">
                        <line x1="2" y1="14" x2="30" y2="14" stroke="#10b981" stroke-width="2.5" class="bpmn-flow-dash-anim" />
                        <polygon points="29,9 38,14 29,19" fill="#10b981" />
                      </svg>
                    </div>

                    <!-- 节点卡片 -->
                    <div
                      class="bpmn-task-card"
                      :class="getNodeCardClass(node.role)"
                      style="min-width: 170px; padding: 10px 14px;"
                    >
                      <div class="task-icon-col icon-box-lead">
                        <el-icon :size="18"><component :is="getNodeIcon(node.role)" /></el-icon>
                      </div>
                      <div class="task-title-col">
                        <div class="t-name-row">
                          <span class="t-name">{{ node.nodeName }}</span>
                          <span class="role-code-chip chip-lead">STEP 0{{ Number(nIdx) + 2 }}</span>
                        </div>
                        <div class="t-role">{{ node.approverRole || formatRoleText(node.role) }}</div>
                      </div>
                    </div>
                  </template>

                  <!-- 终止归档节点 -->
                  <div class="bpmn-svg-arrow-wrap">
                    <svg class="bpmn-flow-svg-line" width="40" height="28" viewBox="0 0 40 28">
                      <line x1="2" y1="14" x2="30" y2="14" stroke="#10b981" stroke-width="2.5" class="bpmn-flow-dash-anim" />
                      <polygon points="29,9 38,14 29,19" fill="#10b981" />
                    </svg>
                  </div>
                  <div class="bpmn-node-pill bpmn-end-pill">
                    <div class="pill-main-row">
                      <div class="bpmn-circle end-circle"></div>
                      <span class="bpmn-label">变更完成归档</span>
                    </div>
                    <span class="pill-sub-text">审计留痕归档</span>
                  </div>
                </div>
              </div>

              <!-- 影响行数语法提示框 -->
              <div v-show="!isBpmnPreviewCollapsed" class="syntax-tip-card" style="margin-top: 12px;">
                <div class="tip-header">
                  <el-icon><InfoFilled /></el-icon>
                  <span>DML 影响行数注解说明</span>
                </div>
                <div class="tip-body">
                  在每条 DML 语句上方书写 <code>-- 1</code> 或 <code>-- 影响行数: 1</code>，系统将在目标库事务中自动校验影响行数是否严格一致，杜绝多删漏更风险！
                </div>
              </div>
            </div>

            <!-- 基础信息底部快捷收起引导条 -->
            <div class="basic-bottom-fold-bar">
              <span class="fold-bar-hint">
                💡 基础信息填写完毕后，可收起本区块以释放更大 SQL 编写视口
              </span>
              <div style="display: flex; gap: 8px; align-items: center;">
                <el-button size="small" plain :icon="ArrowUp" @click="collapseBasicAndGoSql">
                  收起基础信息并编写 SQL
                </el-button>
              </div>
            </div>
            </div>
          </div>

          <!-- 2. SQL 执行脚本编辑与上传 -->
          <div class="stream-section-card" id="form-item-sql-workbench" :class="{ 'is-fullscreen-mode': isSqlFullscreen }">
            <div class="section-badge-row">
              <div class="section-badge">
                <span class="badge-num">2</span>
                <span class="badge-title">SQL 执行脚本编辑与上传</span>
              </div>
              <div class="editor-header-actions">
                <el-button
                  size="small"
                  :type="isSqlFullscreen ? 'warning' : 'primary'"
                  plain
                  :icon="isSqlFullscreen ? ScaleToOriginal : FullScreen"
                  @click="toggleSqlFullscreen"
                >
                  {{ isSqlFullscreen ? '🔲 还原窗口 (Esc)' : '⛶ 全屏编辑工作台' }}
                </el-button>
              </div>
            </div>

            <div class="input-mode-tabs">
              <el-radio-group v-model="sqlInputMode" size="default">
                <el-radio-button value="TEXT">在线编辑 / 粘贴 SQL</el-radio-button>
                <el-radio-button value="FILE">上传 .sql 附件</el-radio-button>
              </el-radio-group>

              <div class="quick-tools" v-if="sqlInputMode === 'TEXT'">
                <el-button size="small" type="primary" plain :icon="DocumentCopy" @click="insertDmlSample">
                  插入 DML 影响行数范例 (-- 1)
                </el-button>
                <el-button size="small" link @click="form.sqlText = ''">清空</el-button>
              </div>
            </div>

            <!-- 在线 SQL Monaco 编辑工作台 (支持深浅色主题、高/中/低三色风险标定、右侧建议面板与并排对比) -->
            <div v-if="sqlInputMode === 'TEXT'" class="editor-workbench-container" style="margin-top: 10px;">
              <SqlMonacoWorkbench v-model="form.sqlText" />
            </div>

            <!-- 文件上传 -->
            <div v-if="sqlInputMode === 'FILE'" class="upload-wrapper">
              <el-upload
                class="upload-box"
                drag
                action="#"
                :auto-upload="false"
                :on-change="handleFileChange"
                :limit="1"
                accept=".sql,.txt"
              >
                <el-icon class="el-icon--upload"><UploadFilled /></el-icon>
                <div class="el-upload__text">将 .sql 文件拖拽到此处，或 <em>点击上传</em></div>
                <template #tip>
                  <div class="el-upload__tip">支持超大 SQL 附件流式处理，文件内同样支持 <code>-- 1</code> 注解</div>
                </template>
              </el-upload>
            </div>
          </div>

          <!-- 3. 数据备份与回滚方案 (Rollback & Backup Scheme) - 支持动态配置启用/禁用 -->
          <div v-if="isStep3Visible" class="stream-section-card" id="form-item-rollback">
            <div class="section-badge-row">
              <div class="section-badge">
                <span class="badge-num">3</span>
                <span class="badge-title">数据备份与回滚方案 (Rollback / Backup Scheme)</span>
              </div>
              <el-tag size="small" :type="safetyPolicies.requireBackup ? 'danger' : 'info'" effect="light">
                {{ safetyPolicies.requireBackup ? '🛡️ 安全规范：DML/DDL 必填' : '推荐填写' }}
              </el-tag>
            </div>

            <div class="input-mode-tabs" style="margin-top: 8px;">
              <el-radio-group v-model="rollbackInputMode" size="default">
                <el-radio-button value="TEXT">在线编写 / 粘贴回滚 SQL</el-radio-button>
                <el-radio-button value="FILE">上传 .sql 备份附件</el-radio-button>
              </el-radio-group>

              <div class="quick-tools" v-if="rollbackInputMode === 'TEXT'">
                <el-button size="small" type="success" plain :icon="DocumentCopy" @click="insertRollbackSample">
                  插入标准回滚 SQL 范例
                </el-button>
                <el-button size="small" link @click="form.rollbackSqlText = ''">清空</el-button>
              </div>
            </div>

            <!-- 在线回滚 SQL 输入框 -->
            <div v-if="rollbackInputMode === 'TEXT'" class="editor-workbench-container rollback-workbench">
              <div class="code-editor-box" style="height: 150px; background: #f0fdf4;">
                <!-- 动态行号列 -->
                <div class="editor-gutter" style="background: #e2fbe8; border-color: #bbf7d0;">
                  <div v-for="n in Math.max(rollbackStats.lineCount, 1)" :key="n" class="gutter-num" style="color: #16a34a;">{{ n }}</div>
                </div>

                <!-- 原生文本编辑区 -->
                <textarea
                  v-model="form.rollbackSqlText"
                  class="editor-textarea-native rollback-textarea-native"
                  placeholder="在此输入反向回滚 SQL 脚本（如对应 UPDATE 的旧值恢复语句或 DELETE 的补偿插入语句），确保异常时秒级回退..."
                  spellcheck="false"
                ></textarea>
              </div>

              <!-- 编辑器底部信息状态栏 -->
              <div class="editor-status-bar" style="background: #f0fdf4; border-color: #bbf7d0;">
                <div class="status-left">
                  <span class="status-chip">
                    <el-icon style="vertical-align: middle; margin-right: 2px;"><Tickets /></el-icon>
                    行数: <b>{{ rollbackStats.lineCount }}</b> 行
                  </span>
                  <span class="status-chip">
                    <el-icon style="vertical-align: middle; margin-right: 2px;"><Document /></el-icon>
                    字符数: <b>{{ rollbackStats.charCount }}</b> 字符
                  </span>
                  <span class="status-chip">
                    <el-icon style="vertical-align: middle; margin-right: 2px;"><Check /></el-icon>
                    语句数: <b>{{ rollbackStats.stmtCount }}</b> 条
                  </span>
                </div>
                <div class="status-right">
                  <el-tag
                    v-if="rollbackStats.invalidStatements.length === 0 && form.rollbackSqlText.trim()"
                    size="small"
                    type="success"
                    effect="light"
                  >
                    ✅ 回滚 SQL 语法合规
                  </el-tag>
                  <el-tag
                    v-else-if="rollbackStats.invalidStatements.length > 0"
                    size="small"
                    type="danger"
                    effect="dark"
                  >
                    ❌ 存在 {{ rollbackStats.invalidStatements.length }} 处非 SQL 文本
                  </el-tag>
                </div>
              </div>
            </div>

            <!-- 回滚真实性与关联性智能分析看板 -->
            <div v-if="form.rollbackSqlText && form.rollbackSqlText.trim()" class="rollback-correlation-card" style="margin-top: 10px;">
              <el-alert
                :type="rollbackCorrelation.type"
                show-icon
                :closable="false"
              >
                <template #title>
                  <div style="font-weight: 700; font-size: 13px;">
                    {{ rollbackCorrelation.title }}
                  </div>
                </template>
                <template #default>
                  <div style="font-size: 12px; line-height: 1.6; margin-top: 4px;">
                    <div>{{ rollbackCorrelation.desc }}</div>
                    <div style="display: flex; gap: 14px; margin-top: 6px; flex-wrap: wrap; align-items: center;">
                      <span style="color: #475569;">
                        🎯 执行脚本涉及表：
                        <el-tag v-for="t in rollbackCorrelation.execTables" :key="t" size="small" type="primary" effect="plain" style="margin-right: 4px;">{{ t }}</el-tag>
                        <span v-if="rollbackCorrelation.execTables.length === 0" style="color: #94a3b8;">暂未解析到表名</span>
                      </span>
                      <span style="color: #475569;">
                        🔄 回滚方案目标表：
                        <el-tag v-for="t in rollbackCorrelation.rollbackTables" :key="t" size="small" :type="rollbackCorrelation.matched ? 'success' : 'danger'" effect="plain" style="margin-right: 4px;">{{ t }}</el-tag>
                        <span v-if="rollbackCorrelation.rollbackTables.length === 0" style="color: #94a3b8;">暂未解析到表名</span>
                      </span>
                    </div>
                  </div>
                </template>
              </el-alert>
            </div>

            <!-- 回滚/备份文件上传 -->
            <div v-if="rollbackInputMode === 'FILE'" class="upload-wrapper">
              <el-upload
                class="upload-box"
                drag
                action="#"
                :auto-upload="false"
                :on-change="handleRollbackFileChange"
                :limit="1"
                accept=".sql,.txt,.gz,.dump"
              >
                <el-icon class="el-icon--upload"><UploadFilled /></el-icon>
                <div class="el-upload__text">将备份/回滚 .sql 附件拖拽到此处，或 <em>点击上传</em></div>
                <template #tip>
                  <div class="el-upload__tip">支持上传预先导出的业务快照 .sql 或 mysqldump 备份包</div>
                </template>
              </el-upload>
            </div>

            <!-- 若未开启第 4 步预执行校验，则在第 3 步底部展示提交栏 -->
            <div v-if="!isStep4Visible" class="submit-footer-bar">
              <el-button @click="handleResetForm">重置表单</el-button>
              <el-button
                type="primary"
                size="large"
                :icon="Check"
                :loading="submitLoading"
                @click="handleSubmitTicket"
              >
                {{ isCurrentInstanceTestEnv ? '🚀 确认提交并直接执行变更 (测试环境直通)' : '确认提交工单' }}
              </el-button>
            </div>
          </div>

          <!-- 若第 3 步与第 4 步均未开启，则在第 2 步后备用提交栏 -->
          <div v-if="!isStep3Visible && !isStep4Visible" class="stream-section-card" style="padding: 16px 20px;">
            <div class="submit-footer-bar" style="margin-top: 0; padding-top: 0; border-top: none;">
              <el-button @click="handleResetForm">重置表单</el-button>
              <el-button
                type="primary"
                size="large"
                :icon="Check"
                :loading="submitLoading"
                @click="handleSubmitTicket"
              >
                {{ isCurrentInstanceTestEnv ? '🚀 确认提交并直接执行变更 (测试环境直通)' : '确认提交工单' }}
              </el-button>
            </div>
          </div>

          <!-- 4. 事务级预执行校验 (Dry-Run) 与 提交工单 - 支持动态配置启用/禁用 -->
          <div v-if="isStep4Visible" class="stream-section-card" id="form-item-dryrun">
            <div class="section-badge" style="display: flex; align-items: center; justify-content: space-between; width: 100%;">
              <div style="display: flex; align-items: center; gap: 6px;">
                <span class="badge-num">{{ step4Index }}</span>
                <span class="badge-title">
                  <span v-if="isDryRunEnforced" style="color: #f56c6c; font-weight: bold; font-size: 16px; margin-right: 2px;">*</span>
                  事务级预执行校验 (Dry-Run) 与 提交工单
                </span>
                <el-tag :type="isDryRunEnforced ? 'danger' : 'info'" size="small" :effect="isDryRunEnforced ? 'dark' : 'light'" style="font-weight: 600;">
                  {{ isDryRunEnforced ? '⚠️ 策略要求必须校验' : '💡 推荐执行校验' }}
                </el-tag>
              </div>
            </div>

            <div class="dry-run-action-bar" :style="isDryRunEnforced ? 'background: #fff1f2; border: 1px solid #fecdd3; border-radius: 8px; padding: 12px 16px;' : ''">
              <el-button
                :type="isDryRunEnforced ? 'danger' : 'warning'"
                :icon="VideoPlay"
                :loading="dryRunLoading"
                @click="handleDryRun"
              >
                在目标库【{{ form.dbName || '未选' }}】执行事务级预校验
              </el-button>
              <span class="dry-run-desc" :style="isDryRunEnforced ? 'color: #9f1239; font-weight: 500;' : ''">
                模拟执行比对每条 DML 实际影响行数，并在最后自动执行 <code>ROLLBACK</code> 回滚事务
                <b v-if="isDryRunEnforced">（系统当前已开启强制策略：提交工单前必须通过预执行校验！）</b>
                <span v-else>（当前资源组策略为推荐校验，允许跳过直接提单）</span>
              </span>
            </div>

            <!-- 校验结果表格 -->
            <div v-if="dryRunResult" class="dry-run-result-box">
              <el-alert
                :title="dryRunResult.summaryMessage"
                :type="dryRunResult.passed ? 'success' : 'error'"
                :closable="false"
                show-icon
                style="margin-bottom: 8px;"
              />

              <div v-if="isCurrentInstanceTestEnv" class="dry-run-gateway-hint" style="background: #f0fdf4; border-color: #86efac; color: #166534;">
                <el-icon style="color: #16a34a; margin-right: 6px; font-size: 16px;"><Check /></el-icon>
                <span>🎯 <b>测试/开发环境免审直通</b>：目标为【{{ currentSelectedInstance?.env || 'TEST' }}】测试环境，提交并通过预校验后将<b>直接自动执行变更并归档</b>，无需等待人工多级审批流转！</span>
              </div>

              <div v-else-if="dryRunResult.totalActualRows !== undefined" class="dry-run-gateway-hint">
                <el-icon style="color: #409EFF; margin-right: 4px;"><Share /></el-icon>
                <span>智能排他网关判定：预执行累计影响行数为 <b>{{ dryRunResult.totalActualRows }}</b> 行（判定阈值：{{ routingPreview?.affectRowsThreshold || 1000 }} 行，SpEL: <code>{{ routingPreview?.spelExpression || '#{affectRows > 1000}' }}</code>） ➔ 提交后将自动路由至【<b :style="{ color: dryRunResult.totalActualRows > (routingPreview?.affectRowsThreshold || 1000) ? '#F56C6C' : '#67C23A' }">{{ dryRunResult.totalActualRows > (routingPreview?.affectRowsThreshold || 1000) ? `${routingPreview?.highRiskRole || '核心 DBA'} 审核 (触发高危管控)` : `${routingPreview?.lowRiskRole || '运维/开发组长'} 审核 (常规放行)` }}</b>】</span>
              </div>

              <el-table :data="dryRunResult.items" size="small" border stripe max-height="240px">
                <el-table-column prop="index" label="#" width="45" align="center" />
                <el-table-column prop="statementType" label="类型" width="95" align="center">
                  <template #default="{ row }">
                    <el-tag
                      size="small"
                      :type="row.statementType === '非SQL文本' || row.status === 'INVALID' ? 'danger' : (row.isDml ? 'primary' : 'info')"
                    >
                      {{ row.statementType }}
                    </el-tag>
                  </template>
                </el-table-column>
                <el-table-column prop="sqlSnippet" label="SQL 片段" min-width="180" show-overflow-tooltip>
                  <template #default="{ row }">
                    <span :style="{ color: row.status === 'INVALID' ? '#dc2626' : 'inherit', fontWeight: row.status === 'INVALID' ? '600' : 'normal' }">
                      {{ row.sqlSnippet }}
                    </span>
                  </template>
                </el-table-column>
                <el-table-column prop="expectedRows" label="声明预期行数" width="105" align="center">
                  <template #default="{ row }">
                    <span v-if="row.expectedRows !== null" style="font-weight: bold; color: #409EFF;">
                      {{ row.expectedRows }} 行
                    </span>
                    <span v-else style="color: #909399;">未指定</span>
                  </template>
                </el-table-column>
                <el-table-column prop="actualRows" label="预执行影响" width="105" align="center">
                  <template #default="{ row }">
                    <span :style="{ color: row.status === 'MISMATCHED' || row.status === 'INVALID' ? '#F56C6C' : '#67C23A', fontWeight: 'bold' }">
                      {{ row.actualRows }} 行
                    </span>
                  </template>
                </el-table-column>
                <el-table-column prop="status" label="比对结果" width="105" align="center">
                  <template #default="{ row }">
                    <el-tag
                      size="small"
                      :type="row.status === 'MATCHED' ? 'success' : (row.status === 'MISMATCHED' || row.status === 'INVALID' || row.status === 'ERROR' ? 'danger' : 'info')"
                    >
                      {{ row.status === 'MATCHED' ? '一致通过' : (row.status === 'MISMATCHED' ? '行数不符' : (row.status === 'INVALID' ? '非SQL文本' : row.status)) }}
                    </el-tag>
                  </template>
                </el-table-column>
                <el-table-column prop="durationMs" label="耗时" width="70" align="center">
                  <template #default="{ row }">{{ row.durationMs }}ms</template>
                </el-table-column>
              </el-table>
            </div>

            <!-- 提交页并排展示“原 SQL / 改后 SQL”终审核验区 -->
            <div v-if="form.sqlText && form.sqlText.trim()" class="submit-diff-audit-box" style="margin-top: 18px; border: 1px solid #cbd5e1; border-radius: 8px; overflow: hidden; background: #ffffff;">
              <div style="display: flex; justify-content: space-between; align-items: center; padding: 10px 14px; background: #f8fafc; border-bottom: 1px solid #e2e8f0;">
                <div style="font-weight: 700; font-size: 13.5px; color: #1e293b; display: flex; align-items: center; gap: 8px;">
                  <span>⚖️ 提交审核确认：原 SQL 与 改后目标执行 SQL 并排比对</span>
                  <el-tag size="small" type="success" effect="plain">变更就绪</el-tag>
                </div>
                <div style="font-size: 12px; color: #64748b;">
                  提交前终审：核对原始脚本与安全规范化脚本的一致性
                </div>
              </div>
              <div class="submit-diff-grid">
                <div style="border-right: 1px solid #cbd5e1; display: flex; flex-direction: column;">
                  <div style="padding: 6px 12px; background: #f1f5f9; font-size: 12px; font-weight: 600; color: #475569; border-bottom: 1px solid #e2e8f0;">
                    📄 原输入 SQL (Original)
                  </div>
                  <div style="flex: 1; overflow: auto; padding: 10px; font-family: monospace; font-size: 12px; background: #fdfdfd; white-space: pre-wrap; color: #334155;">{{ form.sqlText }}</div>
                </div>
                <div style="display: flex; flex-direction: column;">
                  <div style="padding: 6px 12px; background: #f0fdf4; font-size: 12px; font-weight: 600; color: #166534; border-bottom: 1px solid #e2e8f0;">
                    ✨ 改后执行 SQL (Modified & Safe)
                  </div>
                  <div style="flex: 1; overflow: auto; padding: 10px; font-family: monospace; font-size: 12px; background: #fcfdfc; white-space: pre-wrap; color: #047857; font-weight: 500;">{{ getStandardizedSql(form.sqlText) }}</div>
                </div>
              </div>
            </div>

            <!-- 底部提交栏 -->
            <div class="submit-footer-bar">
              <div class="submit-ready-hint" :class="{ 'is-ready': missingRequiredSteps.length === 0 }">
                <el-icon><component :is="missingRequiredSteps.length === 0 ? Check : Warning" /></el-icon>
                <span v-if="missingRequiredSteps.length === 0">所有必填环节已就绪，可以提交</span>
                <span v-else>尚未完成：{{ missingRequiredSteps.join('、') }}</span>
              </div>
              <el-button @click="handleResetForm">重置表单</el-button>
              <el-button
                type="primary"
                size="large"
                :icon="Check"
                :loading="submitLoading"
                @click="handleSubmitTicket"
              >
                {{ isCurrentInstanceTestEnv ? '🚀 确认提交并直接执行变更 (测试环境直通)' : '确认提交工单' }}
              </el-button>
            </div>
          </div>
        </div>
      </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount, watch } from 'vue'
import SqlMonacoWorkbench from '../components/SqlMonacoWorkbench.vue'
import { useRouter, useRoute } from 'vue-router'
import { ElMessage, ElNotification } from 'element-plus'
import {
  ArrowLeft, ArrowUp, ArrowDown, CopyDocument, Check, DocumentCopy,
  UploadFilled, VideoPlay,
  InfoFilled, FullScreen, ScaleToOriginal,
  Share, Refresh, Warning, Tickets, Document,
  User, UserFilled, Operation, Setting, Lock
} from '@element-plus/icons-vue'
import request from '../utils/request'
import { useUserStore } from '../store/user'

const userStore = useUserStore()
const router = useRouter()
const route = useRoute()
interface InstanceItem {
  id: number
  name: string
  dbType: string
  env: string
  databaseName?: string
  resourceGroups?: string
  tags?: string
  supportedOps?: string
  fixedWorkflowTemplateId?: number
  fixedWorkflowTemplateName?: string
  instanceConfig?: string
}

const instances = ref<InstanceItem[]>([])
const instancesLoading = ref(false)

const currentUserResourceGroup = ref('')
const currentUserResourceGroups = ref<string[]>([])
const availableResourceGroups = ref<string[]>([])

// 审批流预估路由结果
const routingPreview = ref<any>(null)
const routingLoading = ref(false)
const gatewayPreviewMode = ref<'auto' | 'normal' | 'high'>('auto')

// 统计 SQL 注解声明的预估行数
const estimateDeclaredRowsFromSql = (sql: string): number => {
  if (!sql) return 1
  const regex = /^\s*--\s*(?:影响行数\s*[:：]?\s*)?(\d+)\s*$/gm
  let total = 0
  let matched = false
  let m: RegExpExecArray | null
  while ((m = regex.exec(sql)) !== null) {
    matched = true
    total += parseInt(m[1], 10) || 0
  }
  return matched ? Math.max(total, 1) : 1
}

// 排他网关当前激活分支判定 (支持自动跟随工单 / 手动切换沙盘预览)
const isGatewayHighRiskActive = computed(() => {
  if (gatewayPreviewMode.value === 'high') return true
  if (gatewayPreviewMode.value === 'normal') return false
  if (dryRunResult.value?.totalActualRows !== undefined) {
    const threshold = routingPreview.value?.affectRowsThreshold || 1000
    if (dryRunResult.value.totalActualRows > threshold) return true
  }
  const declaredRows = estimateDeclaredRowsFromSql(form.value.sqlText || '')
  const threshold = routingPreview.value?.affectRowsThreshold || 1000
  if (declaredRows > threshold) return true
  if (detectedSqlSubtype.value?.type === 'DDL_CHANGE' || form.value.sqlSubtype === 'DDL_CHANGE') return true
  return Boolean(routingPreview.value?.isHighRisk)
})

// 结构化解析判定依据文本，避免长串括号与原始 JSON 数组直接堆砌
const parsedRoutingBasis = computed(() => {
  const raw = String(routingPreview.value?.routingReason || '')
  const rgMatch = raw.match(/资源组【([^】]+)】/)
  const ctMatch = raw.match(/变更类型【([^】]+)】/)
  if (!rgMatch && !ctMatch) {
    return {
      structured: false,
      resourceGroup: '',
      changeType: '',
      tags: [] as string[],
      thresholdNote: ''
    }
  }
  let tags: string[] = []
  const tagArrMatch = raw.match(/实例标签(\[[^\]]+\])/)
  if (tagArrMatch) {
    try {
      const parsed = JSON.parse(tagArrMatch[1])
      if (Array.isArray(parsed)) {
        tags = parsed.map(x => String(x))
      }
    } catch {
      tags = tagArrMatch[1].replace(/[\[\]"']/g, '').split(',').map(s => s.trim()).filter(Boolean)
    }
  }
  const noteMatch = raw.match(/[（(]([^）)]*阈值[^）)]*)[）)]/)
  return {
    structured: true,
    resourceGroup: rgMatch ? rgMatch[1] : (form.value.resourceGroup || '默认资源组'),
    changeType: ctMatch ? ctMatch[1] : (form.value.type || 'SQL_AUDIT'),
    tags,
    thresholdNote: noteMatch ? noteMatch[1] : ''
  }
})

// 全局安全策略配置
const safetyPolicies = ref({
  enforceDryRun: true,
  requireBackup: true,
  maxQueryRows: 1000
})

// 实例标签与支持操作管控逻辑
const currentSelectedInstance = computed(() => {
  if (!form.value.instanceId) return null
  return filteredInstances.value.find(i => String(i.id) === String(form.value.instanceId)) || null
})

// 当前选中实例是否为测试/开发免审直通环境
const isCurrentInstanceTestEnv = computed(() => {
  const env = currentSelectedInstance.value?.env?.toUpperCase()
  return env === 'TEST' || env === 'DEV' || env === 'UAT' || env === 'SIT' || env === 'LOCAL' || (routingPreview.value?.templateName && routingPreview.value.templateName.includes('测试'))
})

const parseInstanceTags = (tagsRaw?: string): string[] => {
  if (!tagsRaw) return []
  try {
    const arr = JSON.parse(tagsRaw)
    return Array.isArray(arr) ? arr : []
  } catch (e) {
    return tagsRaw.split(/[,，]/).filter(Boolean)
  }
}

const getTagColorType = (tag: string) => {
  if (tag.includes('生产') || tag.includes('敏感') || tag.includes('核心')) return 'danger'
  if (tag.includes('只读') || tag.includes('从库')) return 'primary'
  if (tag.includes('测试')) return 'warning'
  if (tag.includes('集群')) return 'success'
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

// 判定当前操作是否被实例支持
const isCurrentTicketTypeSupported = computed(() => {
  const ops = currentInstanceSupportedOps.value
  const sql = form.value.sqlText?.toUpperCase() || ''
  const isDml = form.value.type === 'DATA_RECOVERY' || sql.includes('INSERT') || sql.includes('UPDATE') || sql.includes('DELETE')
  const isDdl = sql.includes('CREATE') || sql.includes('ALTER') || sql.includes('DROP') || sql.includes('TRUNCATE')

  if (isDml && !ops.includes('支持DML变更')) {
    return { supported: false, message: '目标实例当前未开启【支持DML变更】权限，严禁提交 DML 数据变更工单！' }
  }
  if (isDdl && !ops.includes('支持DDL结构变更') && !ops.includes('支持DDL变更')) {
    return { supported: false, message: '目标实例当前未开启【支持DDL结构变更】权限，严禁提交 DDL 库表结构工单！' }
  }
  if (form.value.type === 'DATA_EXPORT' && !ops.includes('支持数据导出')) {
    return { supported: false, message: '目标实例当前未开启【支持数据导出】权限，无法提交数据导出申请！' }
  }
  if (!ops.includes('支持上线')) {
    return { supported: false, message: '目标实例当前未开启【支持上线】权限，无法提交发布工单！' }
  }
  return { supported: true, message: '' }
})

// ==================== 新建工单状态 ====================
const availableDatabases = ref<string[]>([])
const databasesLoading = ref(false)
const submitLoading = ref(false)
const dryRunLoading = ref(false)
const isSqlFullscreen = ref(false)
const isBasicInfoCollapsed = ref(false)
const isBpmnPreviewCollapsed = ref(false)
const sqlInputMode = ref<'TEXT' | 'FILE'>('TEXT')
const rollbackInputMode = ref<'TEXT' | 'FILE'>('TEXT')
const dryRunResult = ref<any>(null)

const form = ref({
  resourceGroup: '',
  instanceId: '' as string | number,
  dbName: '',
  type: 'SQL_AUDIT',
  sqlSubtype: 'AUTO' as 'AUTO' | 'DML_CHANGE' | 'DDL_CHANGE',
  reason: '',
  sqlText: '',
  file: null as File | null,
  rollbackSqlText: '',
  rollbackFile: null as File | null
})

// 生成改后规范化/安全加固 SQL (关键字大写、分号规范)
const getStandardizedSql = (raw: string) => {
  if (!raw) return ''
  const keywords = [
    'select', 'insert', 'update', 'delete', 'from', 'where', 'into', 'values',
    'set', 'and', 'or', 'join', 'left', 'right', 'inner', 'on', 'group', 'by',
    'order', 'asc', 'desc', 'limit', 'offset', 'having', 'as', 'create', 'table',
    'alter', 'drop', 'add', 'column', 'primary', 'key', 'not', 'null', 'default'
  ]
  let lines = raw.split('\n')
  return lines.map(line => {
    let mod = line
    keywords.forEach(kw => {
      const reg = new RegExp(`\\b${kw}\\b`, 'gi')
      mod = mod.replace(reg, kw.toUpperCase())
    })
    const tr = mod.trim()
    if (tr && !tr.startsWith('--') && !tr.startsWith('#') && !tr.startsWith('/*') && !tr.endsWith(';')) {
      mod = mod + ';'
    }
    return mod
  }).join('\n')
}

// 资源组可配置扩展字段管理
interface FormFieldConfig {
  fieldKey: string
  fieldName: string
  fieldType: string
  placeholder: string
  enabled: boolean
  required: boolean
  defaultRange?: string[]
}

const customForm = ref<Record<string, any>>({
  releaseVersion: '',
  releaseDate: '',
  executionTimeRange: ['00:00:00', '06:00:00'],
  demandNo: ''
})

const resourceGroupFormConfigsMap = ref<Record<string, FormFieldConfig[]>>({})
const resourceGroupFullConfigMap = ref<Record<string, any>>({})

const currentRgConfig = computed(() => {
  const rg = form.value.resourceGroup
  const cfg = (rg && resourceGroupFullConfigMap.value[rg]) || {}
  return {
    enforceDryRun: routingPreview.value?.enforceDryRun ?? cfg.enforceDryRun ?? true,
    enableStep3Rollback: routingPreview.value?.enableStep3Rollback ?? cfg.enableStep3Rollback ?? true,
    enableStep4DryRun: routingPreview.value?.enableStep4DryRun ?? cfg.enableStep4DryRun ?? true
  }
})

const isStep3Visible = computed(() => currentRgConfig.value.enableStep3Rollback !== false)
const isStep4Visible = computed(() => currentRgConfig.value.enableStep4DryRun !== false)
const isDryRunEnforced = computed(() => isStep4Visible.value && currentRgConfig.value.enforceDryRun !== false)

const step4Index = computed(() => isStep3Visible.value ? 4 : 3)

// ==================== 吸顶步骤导航与就绪度 ====================
const creationSteps = computed(() => {
  const steps: { key: string; title: string; anchor: string; done: boolean; required: boolean }[] = [
    {
      key: 'basic',
      title: '基础信息',
      anchor: 'form-item-basic',
      done: Boolean(form.value.resourceGroup && form.value.instanceId && form.value.dbName && form.value.type),
      required: true
    },
    {
      key: 'sql',
      title: 'SQL 脚本',
      anchor: 'form-item-sql-workbench',
      done: sqlInputMode.value === 'TEXT' ? Boolean(form.value.sqlText && form.value.sqlText.trim()) : Boolean(form.value.file),
      required: true
    }
  ]
  if (isStep3Visible.value) {
    steps.push({
      key: 'rollback',
      title: '回滚方案',
      anchor: 'form-item-rollback',
      done: rollbackInputMode.value === 'TEXT' ? Boolean(form.value.rollbackSqlText && form.value.rollbackSqlText.trim()) : Boolean(form.value.rollbackFile),
      required: Boolean(safetyPolicies.value.requireBackup)
    })
  }
  if (isStep4Visible.value) {
    steps.push({
      key: 'dryrun',
      title: '预校验与提交',
      anchor: 'form-item-dryrun',
      done: Boolean(dryRunResult.value?.passed),
      required: isDryRunEnforced.value
    })
  }
  return steps
})

const creationProgress = computed(() => {
  const list = creationSteps.value
  if (list.length === 0) return 0
  return Math.round((list.filter(s => s.done).length / list.length) * 100)
})

const missingRequiredSteps = computed(() => creationSteps.value.filter(s => s.required && !s.done).map(s => s.title))

const ticketTypeLabel = computed(() => {
  const map: Record<string, string> = {
    SQL_AUDIT: 'SQL 变更审核',
    DATA_EXPORT: '敏感数据导出',
    PERMISSION: '权限与账号申请',
    DATA_RECOVERY: '应急数据修复'
  }
  return map[form.value.type] || form.value.type || 'SQL 变更审核'
})

const scrollToSection = (anchor: string) => {
  if (anchor === 'form-item-basic' && isBasicInfoCollapsed.value) {
    isBasicInfoCollapsed.value = false
  }
  const el = document.getElementById(anchor)
  if (el) {
    el.scrollIntoView({ behavior: 'smooth', block: 'start' })
  }
}

const collapseBasicAndGoSql = () => {
  isBasicInfoCollapsed.value = true
  setTimeout(() => {
    scrollToSection('form-item-sql-workbench')
  }, 60)
}

const currentResourceGroupFields = computed(() => {
  const selectedInst = instances.value.find(i => String(i.id) === String(form.value.instanceId))
  // 如果当前选中的实例有定制扩展字段与必填约束，则优先采用/合并实例定制字段
  if (selectedInst && selectedInst.instanceConfig) {
    try {
      const parsedInst = JSON.parse(selectedInst.instanceConfig)
      if (parsedInst && Array.isArray(parsedInst.formFieldList) && parsedInst.formFieldList.length > 0) {
        return parsedInst.formFieldList.filter((f: any) => f.enabled)
      }
    } catch (e) {}
  }

  const rg = form.value.resourceGroup
  if (!rg || !resourceGroupFormConfigsMap.value[rg]) {
    // 默认提供通用的已启用字段
    return [
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
        required: false
      },
      {
        fieldKey: 'demandNo',
        fieldName: '关联需求/任务编号',
        fieldType: 'TEXT',
        placeholder: '请输入 JIRA / 禅道任务单号',
        enabled: true,
        required: false
      }
    ].filter(f => f.enabled)
  }
  return resourceGroupFormConfigsMap.value[rg].filter(f => f.enabled)
})

const VALID_SQL_KEYWORDS = new Set([
  'SELECT', 'INSERT', 'UPDATE', 'DELETE', 'REPLACE',
  'CREATE', 'ALTER', 'DROP', 'TRUNCATE', 'RENAME',
  'SHOW', 'DESC', 'DESCRIBE', 'EXPLAIN', 'USE', 'SET',
  'GRANT', 'REVOKE', 'LOCK', 'UNLOCK', 'START', 'BEGIN', 'COMMIT', 'ROLLBACK', 'SAVEPOINT',
  'ANALYZE', 'OPTIMIZE', 'CHECK', 'REPAIR', 'CALL', 'DO', 'HANDLER', 'LOAD', 'WITH',
  'KILL', 'FLUSH', 'RESET', 'CHANGE', 'STOP', 'SOURCE', 'COMMENT', 'FLASHBACK', 'PURGE',
  'MERGE', 'UPSERT'
])

const sqlEditorStats = computed(() => {
  const text = form.value.sqlText || ''
  const charCount = text.length
  const lines = text.split('\n')
  const lineCount = lines.length

  // 解析语句
  const rawSegments = text.split(';').map(s => s.trim()).filter(s => s.length > 0)
  const statements: any[] = []
  const invalidStatements: any[] = []
  let dmlCount = 0

  for (let i = 0; i < rawSegments.length; i++) {
    const raw = rawSegments[i]
    const clean = raw.replace(/\/\*[\s\S]*?\*\//g, '').replace(/--.*?(\r?\n|$)/g, '').trim()
    if (!clean) continue

    const firstWord = clean.split(/\s+/)[0].replace(/[^a-zA-Z0-9_]/g, '').toUpperCase()
    const isDml = ['INSERT', 'UPDATE', 'DELETE', 'REPLACE'].includes(firstWord)
    if (isDml) dmlCount++

    const isValid = VALID_SQL_KEYWORDS.has(firstWord)
    const item = {
      index: i + 1,
      firstWord,
      isDml,
      isValid,
      snippet: raw.length > 40 ? raw.substring(0, 40) + '...' : raw
    }
    statements.push(item)
    if (!isValid) {
      invalidStatements.push(item)
    }
  }

  return {
    charCount,
    lineCount,
    stmtCount: statements.length,
    dmlCount,
    statements,
    invalidStatements
  }
})

const DDL_KEYWORDS_SET = new Set(['CREATE', 'ALTER', 'DROP', 'TRUNCATE', 'RENAME', 'COMMENT', 'FLASHBACK', 'PURGE'])
const DML_KEYWORDS_SET = new Set(['INSERT', 'UPDATE', 'DELETE', 'REPLACE', 'MERGE', 'UPSERT'])

const detectedSqlSubtype = computed(() => {
  const text = (form.value.sqlText || '').trim()
  if (!text) return null

  const rawSegments = text.split(';').map(s => s.trim()).filter(s => s.length > 0)
  let hasDdl = false
  let hasDml = false
  const ddlTypes = new Set<string>()
  let dmlCount = 0

  for (const raw of rawSegments) {
    const clean = raw.replace(/\/\*[\s\S]*?\*\//g, '').replace(/--.*?(\r?\n|$)/g, '').trim()
    if (!clean) continue

    const firstWord = clean.split(/\s+/)[0].replace(/[^a-zA-Z0-9_]/g, '').toUpperCase()
    if (DDL_KEYWORDS_SET.has(firstWord)) {
      hasDdl = true
      ddlTypes.add(firstWord)
    }
    if (DML_KEYWORDS_SET.has(firstWord)) {
      hasDml = true
      dmlCount++
    }
  }

  if (hasDdl) {
    return {
      type: 'DDL_CHANGE' as const,
      label: '包含 DDL 结构变更',
      tagType: 'warning' as const,
      icon: '⚠️',
      desc: `识别到 ${Array.from(ddlTypes).join('/')} 等结构定义指令${dmlCount > 0 ? ` (混合 ${dmlCount} 条 DML 数据操作)` : ''}`
    }
  } else if (hasDml) {
    return {
      type: 'DML_CHANGE' as const,
      label: '纯 DML 数据变更',
      tagType: 'success' as const,
      icon: '📝',
      desc: `识别到 ${dmlCount} 条纯数据写入/更新语句，无结构定义变更`
    }
  } else if (rawSegments.length > 0) {
    return {
      type: 'SQL_AUDIT' as const,
      label: '通用 SQL 变更 / 查询',
      tagType: 'info' as const,
      icon: '🔍',
      desc: '未检测到明确的 DML/DDL 变更语句'
    }
  }
  return null
})

const extractTablesFromSql = (sql: string): Set<string> => {
  const tables = new Set<string>()
  if (!sql) return tables
  const clean = sql.replace(/\/\*[\s\S]*?\*\//g, '').replace(/--.*?(\r?\n|$)/g, ' ')
  const regex = /\b(?:FROM|INTO|UPDATE|TABLE|TRUNCATE)\s+(?:IF\s+EXISTS\s+|IF\s+NOT\s+EXISTS\s+)?([`'"\w.]+)/gi
  let match: RegExpExecArray | null
  while ((match = regex.exec(clean)) !== null) {
    let t = match[1].replace(/[`'"]/g, '').trim()
    if (t.includes('.')) {
      t = t.substring(t.lastIndexOf('.') + 1)
    }
    if (t && !VALID_SQL_KEYWORDS.has(t.toUpperCase()) && !['SELECT', 'WHERE', 'SET', 'JOIN', 'AS'].includes(t.toUpperCase())) {
      tables.add(t.toLowerCase())
    }
  }
  return tables
}

const rollbackStats = computed(() => {
  const text = form.value.rollbackSqlText || ''
  const charCount = text.length
  const lines = text.split('\n')
  const lineCount = lines.length

  const rawSegments = text.split(';').map(s => s.trim()).filter(s => s.length > 0)
  const statements: any[] = []
  const invalidStatements: any[] = []
  let dmlCount = 0

  for (let i = 0; i < rawSegments.length; i++) {
    const raw = rawSegments[i]
    const clean = raw.replace(/\/\*[\s\S]*?\*\//g, '').replace(/--.*?(\r?\n|$)/g, '').trim()
    if (!clean) continue

    const firstWord = clean.split(/\s+/)[0].replace(/[^a-zA-Z0-9_]/g, '').toUpperCase()
    const isDml = ['INSERT', 'UPDATE', 'DELETE', 'REPLACE'].includes(firstWord)
    if (isDml) dmlCount++

    const isValid = VALID_SQL_KEYWORDS.has(firstWord)
    const item = {
      index: i + 1,
      firstWord,
      isDml,
      isValid,
      snippet: raw.length > 40 ? raw.substring(0, 40) + '...' : raw
    }
    statements.push(item)
    if (!isValid) {
      invalidStatements.push(item)
    }
  }

  const tables = extractTablesFromSql(text)

  return {
    charCount,
    lineCount,
    stmtCount: statements.length,
    dmlCount,
    statements,
    invalidStatements,
    tables
  }
})

const rollbackCorrelation = computed(() => {
  const execTables = extractTablesFromSql(form.value.sqlText || '')
  const rollbackTables = rollbackStats.value.tables
  const hasRollbackText = form.value.rollbackSqlText && form.value.rollbackSqlText.trim().length > 0

  if (!hasRollbackText) {
    return {
      status: 'EMPTY',
      type: 'info',
      title: '暂未填写回滚脚本',
      desc: '请编写与待执行 SQL 逆向对应的真实回滚补偿脚本（例如：INSERT 对应 DELETE 补偿语句）。',
      execTables: Array.from(execTables),
      rollbackTables: [],
      matched: false
    }
  }

  if (rollbackStats.value.invalidStatements.length > 0) {
    const first = rollbackStats.value.invalidStatements[0]
    return {
      status: 'INVALID_SQL',
      type: 'error',
      title: '🚨 回滚方案中包含非 SQL 文本',
      desc: `在第 ${first.index} 处检测到非标准 SQL 指令【${first.snippet}】，必须填写真实合法的 SQL 语句！`,
      execTables: Array.from(execTables),
      rollbackTables: Array.from(rollbackTables),
      matched: false
    }
  }

  if (execTables.size > 0 && rollbackTables.size > 0) {
    const common: string[] = []
    for (const t of execTables) {
      if (rollbackTables.has(t)) {
        common.push(t)
      }
    }

    if (common.length === 0) {
      return {
        status: 'TABLE_MISMATCH',
        type: 'error',
        title: '🚨 目标表不匹配：疑似非真实回滚方案',
        desc: `执行脚本涉及表【${Array.from(execTables).join(', ')}】，而回滚脚本涉及表【${Array.from(rollbackTables).join(', ')}】，两者无任何重合！请核对是否填错了回滚方案。`,
        execTables: Array.from(execTables),
        rollbackTables: Array.from(rollbackTables),
        matched: false
      }
    }

    // 检查逆向操作对应关系
    const execFirstWords = sqlEditorStats.value.statements.map((s: any) => s.firstWord)
    const rollbackFirstWords = rollbackStats.value.statements.map((s: any) => s.firstWord)
    let isReverse = false
    if (execFirstWords.includes('INSERT') && rollbackFirstWords.includes('DELETE')) isReverse = true
    if (execFirstWords.includes('DELETE') && (rollbackFirstWords.includes('INSERT') || rollbackFirstWords.includes('REPLACE'))) isReverse = true
    if (execFirstWords.includes('UPDATE') && rollbackFirstWords.includes('UPDATE')) isReverse = true
    if (execFirstWords.includes('CREATE') && rollbackFirstWords.includes('DROP')) isReverse = true
    if (execFirstWords.includes('ALTER') && rollbackFirstWords.includes('ALTER')) isReverse = true

    if (isReverse) {
      return {
        status: 'MATCHED_REVERSE',
        type: 'success',
        title: '✅ 真实回滚方案关联校验通过 (逆向补偿完美契合)',
        desc: `目标表【${common.join(', ')}】完全一致，且操作符合经典逆向补偿模型（例如 INSERT 变更 ➔ DELETE 补偿回退），可保障异常时秒级安全回退！`,
        execTables: Array.from(execTables),
        rollbackTables: Array.from(rollbackTables),
        matched: true
      }
    }

    return {
      status: 'MATCHED_SAME_TABLE',
      type: 'success',
      title: '✅ 目标表一致性校验通过',
      desc: `回滚方案与执行脚本均操作共同目标表【${common.join(', ')}】。`,
      execTables: Array.from(execTables),
      rollbackTables: Array.from(rollbackTables),
      matched: true
    }
  }

  return {
    status: 'VALID',
    type: 'success',
    title: '✅ 回滚 SQL 语法合规',
    desc: '回滚脚本为合法 SQL 语句。',
    execTables: Array.from(execTables),
    rollbackTables: Array.from(rollbackTables),
    matched: true
  }
})

const toggleSqlFullscreen = () => {
  isSqlFullscreen.value = !isSqlFullscreen.value
}

const handleRollbackFileChange = (file: any) => {
  form.value.rollbackFile = file.raw
}

const insertRollbackSample = () => {
  form.value.rollbackSqlText = `-- 1
-- 数据回滚补偿语句：恢复 typ_preference 表配置数据
DELETE FROM typ_preference 
WHERE name = 'copyClearFieldsConfig' AND profile IN ('18080003');`
  ElMessage.success('已插入标准回滚 SQL 脚本范例')
}

const fetchSafetyPolicies = async () => {
  try {
    const res: any = await request.get('/v1/config/safety-policies')
    if (res.data) {
      safetyPolicies.value = {
        enforceDryRun: res.data.enforceDryRun ?? true,
        requireBackup: res.data.requireBackup ?? true,
        maxQueryRows: res.data.maxQueryRows ?? 1000
      }
    }
  } catch (e) {
    // fallback
  }
}

// 根据当前所选业务资源组，动态过滤可访问的数据库实例
const filteredInstances = computed(() => {
  if (!form.value.resourceGroup) {
    return []
  }
  if (form.value.resourceGroup === '全部业务资源组通用') {
    return instances.value
  }
  return instances.value.filter(inst => {
    if (!inst.resourceGroups) return false
    try {
      const rgs = typeof inst.resourceGroups === 'string' ? JSON.parse(inst.resourceGroups) : inst.resourceGroups
      if (Array.isArray(rgs)) {
        return rgs.includes(form.value.resourceGroup) || rgs.includes('全部业务资源组通用')
      }
    } catch (e) {
      return inst.resourceGroups.includes(form.value.resourceGroup) || inst.resourceGroups.includes('全部业务资源组通用')
    }
    return false
  })
})

const handleResourceGroupChange = async (_val?: string) => {
  const matching = filteredInstances.value
  if (matching.length > 0) {
    if (!matching.some(i => String(i.id) === String(form.value.instanceId))) {
      form.value.instanceId = matching[0].id
      await loadDatabasesForInstance(matching[0].id)
      await fetchRoutingPreview()
    }
  } else {
    // 当前资源组下有 0 个实例时，立即清空 instanceId 与 dbName，绝不能残留 1 或任何历史值
    form.value.instanceId = ''
    form.value.dbName = ''
    availableDatabases.value = []
    routingPreview.value = null
  }
}

const currentEnvTag = computed(() => {
  const target = instances.value.find(i => String(i.id) === String(form.value.instanceId))
  return target ? `${target.name} [${target.env}]` : '未连接'
})

const handleBack = () => {
  router.push('/ticket-list')
}

const openInNewWindow = () => {
  const href = router.resolve('/ticket-create').href
  window.open(href, '_blank')
}

const loadUserInfo = () => {
  try {
    const raw = localStorage.getItem('user') || localStorage.getItem('userInfo')
    if (raw) {
      const u = JSON.parse(raw)
      const rgRaw = u.resourceGroups || u.resourceGroup
      if (rgRaw) {
        if (Array.isArray(rgRaw)) {
          currentUserResourceGroups.value = rgRaw.filter(Boolean)
        } else if (typeof rgRaw === 'string') {
          let str = rgRaw.trim()
          if (str.startsWith('[') && str.endsWith(']')) {
            try {
              const parsed = JSON.parse(str)
              if (Array.isArray(parsed)) {
                currentUserResourceGroups.value = parsed.filter(Boolean)
              }
            } catch (e) {
              str = str.substring(1, str.length - 1)
            }
          }
          if (currentUserResourceGroups.value.length === 0) {
            currentUserResourceGroups.value = str.split(/[,，]/).map((s: string) => s.replace(/["']/g, '').trim()).filter(Boolean)
          }
        }
        if (currentUserResourceGroups.value.length > 0) {
          currentUserResourceGroup.value = currentUserResourceGroups.value[0]
          if (!form.value.resourceGroup) {
            form.value.resourceGroup = currentUserResourceGroups.value[0]
          }
        }
      }
    }
  } catch (e) {}
}

const loadResourceGroups = async () => {
  try {
    if (!userStore.userInfo) {
      await userStore.fetchUserInfo()
    }

    // 拉取系统正规业务资源组列表（单一真实数据源）
    const res: any = await request.get('/v1/resource-group/list')
    const allRgs = Array.isArray(res.data) ? res.data : []
    allRgs.forEach((rg: any) => {
      if (rg.groupName && rg.formConfig) {
        try {
          const parsed = JSON.parse(rg.formConfig)
          if (Array.isArray(parsed)) {
            resourceGroupFormConfigsMap.value[rg.groupName] = parsed
            resourceGroupFullConfigMap.value[rg.groupName] = { fields: parsed, enforceDryRun: true, enableStep3Rollback: true, enableStep4DryRun: true }
          } else if (typeof parsed === 'object') {
            resourceGroupFormConfigsMap.value[rg.groupName] = Array.isArray(parsed.fields) ? parsed.fields : []
            resourceGroupFullConfigMap.value[rg.groupName] = parsed
          }
        } catch (e) {}
      }
    })

    const allRealNames = allRgs.map((g: any) => g.groupName)
    const isAdmin = userStore.userInfo?.role === 'ADMIN' || userStore.userInfo?.username === 'testadmin1'
    const myGroups: string[] = (userStore.userInfo?.resourceGroups && userStore.userInfo.resourceGroups.length > 0)
      ? userStore.userInfo.resourceGroups
      : (currentUserResourceGroups.value.length > 0 ? currentUserResourceGroups.value : [])

    currentUserResourceGroups.value = myGroups

    // 超级管理员展示系统维护的全部真实业务资源组；普通用户展示其所属的真实业务资源组
    if (isAdmin || myGroups.length === 0) {
      availableResourceGroups.value = allRealNames.length > 0 ? allRealNames : ['车险承保资源组', '销管系统资源组']
    } else {
      const filteredMyGroups = myGroups.filter(g => allRealNames.includes(g))
      availableResourceGroups.value = filteredMyGroups.length > 0 ? filteredMyGroups : allRealNames
    }

    if (availableResourceGroups.value.length > 0) {
      if (!form.value.resourceGroup || !availableResourceGroups.value.includes(form.value.resourceGroup)) {
        form.value.resourceGroup = availableResourceGroups.value[0]
        currentUserResourceGroup.value = availableResourceGroups.value[0]
      }
    }
  } catch (e) {
    console.error('Failed to load resource groups', e)
    availableResourceGroups.value = ['车险承保资源组', '销管系统资源组']
    if (!form.value.resourceGroup && availableResourceGroups.value.length > 0) {
      form.value.resourceGroup = availableResourceGroups.value[0]
    }
  }
}

// 加载实例列表
const loadInstances = async () => {
  instancesLoading.value = true
  try {
    const res: any = await request.get('/v1/instance/list')
    instances.value = Array.isArray(res.data) ? res.data : []
    const matching = filteredInstances.value
    if (matching.length > 0) {
      const targetId = matching[0].id
      form.value.instanceId = targetId
      await loadDatabasesForInstance(targetId)
      await fetchRoutingPreview()
    } else {
      // 0 个实例时清空 instanceId 与 dbName
      form.value.instanceId = ''
      form.value.dbName = ''
      availableDatabases.value = []
      routingPreview.value = null
    }
  } catch (err) {
    console.error('Failed to load instances', err)
  } finally {
    instancesLoading.value = false
  }
}

// 加载数据库列表
const loadDatabasesForInstance = async (instanceId: number | string) => {
  if (!instanceId) return
  databasesLoading.value = true

  try {
    const res: any = await request.get(`/v1/instance/${instanceId}/databases`)
    const dbs = Array.isArray(res.data) && res.data.length > 0 ? res.data : ['huiqitong_erp']
    availableDatabases.value = dbs
    if (!form.value.dbName || !dbs.includes(form.value.dbName)) {
      form.value.dbName = dbs[0]
    }
  } catch (err) {
    console.error('Failed to load databases', err)
    const inst = instances.value.find(i => String(i.id) === String(instanceId))
    const fallbackDb = inst?.databaseName || 'huiqitong_erp'
    availableDatabases.value = [fallbackDb]
    form.value.dbName = fallbackDb
  } finally {
    databasesLoading.value = false
  }
}

const getNodeCardClass = (role?: string) => {
  if (!role) return 'task-lead'
  const r = role.toUpperCase()
  if (r.includes('SYSTEM') || r.includes('AUTO') || r.includes('预检') || r.includes('直通')) return 'task-lead'
  if (r.includes('DBA')) return 'task-dba'
  if (r.includes('ADMIN')) return 'task-dba'
  if (r.includes('SERVICE') || r.includes('EXEC')) return 'task-service'
  if (r.includes('AUDITOR') || r.includes('SECURITY')) return 'task-lead'
  return 'task-lead'
}

const getNodeIcon = (role?: string) => {
  if (!role) return User
  const r = role.toUpperCase()
  if (r.includes('SYSTEM') || r.includes('AUTO') || r.includes('预检') || r.includes('直通')) return Operation
  if (r.includes('DBA')) return UserFilled
  if (r.includes('ADMIN')) return UserFilled
  if (r.includes('SERVICE') || r.includes('EXEC')) return Setting
  if (r.includes('AUDITOR') || r.includes('SECURITY')) return Lock
  return User
}

const formatRoleText = (role?: string) => {
  if (!role) return '审批人'
  const map: Record<string, string> = {
    DEV_LEAD: '业务开发组长',
    DBA: '核心数据库管理员',
    ADMIN: '系统管理员终审',
    AUDITOR: '安全合规审计员',
    OPS: '业务系统运维',
    SECURITY: '数据安全官',
    SYSTEM: '系统预检引擎 (自动)',
    SERVICE: 'JDBC 流式执行引擎'
  }
  return map[role.toUpperCase()] || role
}

const fetchRoutingPreview = async () => {
  if (!form.value.instanceId) {
    routingPreview.value = null
    return
  }
  routingLoading.value = true
  try {
    let effectiveType = form.value.type
    if (form.value.type === 'SQL_AUDIT') {
      if (form.value.sqlSubtype === 'DML_CHANGE') {
        effectiveType = 'DML_CHANGE'
      } else if (form.value.sqlSubtype === 'DDL_CHANGE') {
        effectiveType = 'DDL_CHANGE'
      } else {
        const detected = detectedSqlSubtype.value
        if (detected && (detected.type === 'DML_CHANGE' || detected.type === 'DDL_CHANGE')) {
          effectiveType = detected.type
        } else {
          effectiveType = 'SQL_AUDIT'
        }
      }
    }

    const estimatedRows = dryRunResult.value?.totalActualRows !== undefined
      ? dryRunResult.value.totalActualRows
      : estimateDeclaredRowsFromSql(form.value.sqlText || '')

    const payload = {
      instanceId: form.value.instanceId,
      dbName: form.value.dbName,
      resourceGroup: form.value.resourceGroup,
      ticketType: effectiveType,
      sqlSnippet: form.value.sqlText,
      expectedRows: estimatedRows
    }
    const res: any = await request.post('/v1/workflow/template/preview-routing', payload)
    routingPreview.value = res.data
  } catch (e) {
    console.warn('Fetch workflow routing preview failed', e)
  } finally {
    routingLoading.value = false
  }
}

let sqlAnalysisTimer: any = null
let lastNotifiedSubtype: string | null = null

const handleSqlEditFinished = (isExplicitAction: boolean = false) => {
  if (form.value.instanceId) {
    fetchRoutingPreview()
  }
  const detected = detectedSqlSubtype.value
  if (!detected || !form.value.sqlText.trim()) return

  if (form.value.type === 'SQL_AUDIT' && form.value.sqlSubtype === 'AUTO') {
    if (isExplicitAction || (lastNotifiedSubtype !== null && lastNotifiedSubtype !== detected.type)) {
      ElMessage({
        type: detected.type === 'DDL_CHANGE' ? 'warning' : 'success',
        message: `🤖 SQL 编辑识别完毕：已自动识别为【${detected.label}】(${detected.desc})，已自动匹配对应审批流！`,
        grouping: true,
        duration: 3000
      })
    }
    lastNotifiedSubtype = detected.type
  }
}

watch(() => [form.value.instanceId, form.value.dbName, form.value.type, form.value.sqlSubtype, form.value.resourceGroup], () => {
  if (form.value.instanceId) {
    fetchRoutingPreview()
  }
}, { immediate: true })

watch(() => form.value.sqlText, () => {
  if (sqlAnalysisTimer) clearTimeout(sqlAnalysisTimer)
  sqlAnalysisTimer = setTimeout(() => {
    handleSqlEditFinished(false)
  }, 400)
})

const refreshInstancePermissions = async () => {
  instancesLoading.value = true
  try {
    const res: any = await request.get('/v1/instance/list')
    instances.value = Array.isArray(res.data) ? res.data : []
    ElMessage.success('已从服务器同步最新实例管控权限与扩展字段配置！')
    await fetchRoutingPreview()
  } catch (e) {
    ElMessage.error('同步实例权限失败')
  } finally {
    instancesLoading.value = false
  }
}

const handleInstanceChange = async (val: any) => {
  dryRunResult.value = null
  if (val) {
    try {
      const res: any = await request.get(`/v1/instance/${val}`)
      if (res.data && res.data.id) {
        const idx = instances.value.findIndex(i => i.id === res.data.id)
        if (idx > -1) {
          instances.value[idx] = res.data
        }
      }
    } catch (ignored) {}
  }
  await loadDatabasesForInstance(val)
  await fetchRoutingPreview()
}

// 插入 DML 影响行数范例
const insertDmlSample = () => {
  form.value.sqlText = `-- 1
INSERT INTO typ_preference (PREFERENCEID, CREATEDATE, UPDATEDATE, DESCRIPTION, actortype, kind, name, value, profile)
SELECT COALESCE(MAX(PREFERENCEID), 0) + 1, NOW(), NOW(), '复制续保清空配置-18080003分片1', 'System', 'copyClearFieldsConfig', 'copyClearFieldsConfig', 'plcWarrLineSupply:netPlatSendBackFlag=0,netPlatFlagCode=null', '18080003'
FROM typ_preference
WHERE PREFERENCEID >= 0000000000000001 AND PREFERENCEID <= 9999999999999999;

-- 1
INSERT INTO typ_preference (PREFERENCEID, CREATEDATE, UPDATEDATE, DESCRIPTION, actortype, kind, name, value, profile)
SELECT COALESCE(MAX(PREFERENCEID), 0) + 1, NOW(), NOW(), '复制续保清空配置-18080003分片2', 'System', 'copyClearFieldsConfig', 'copyClearFieldsConfig', 'plcWarrLineSupply:netPlatFlagName=null,netPlatNumber=null', '18080003'
FROM typ_preference
WHERE PREFERENCEID >= 0000000000000001 AND PREFERENCEID <= 9999999999999999;`
  ElMessage.success('已插入 DML 影响行数注解范例')
  handleSqlEditFinished(true)
}

const handleFileChange = (file: any) => {
  form.value.file = file.raw
  if (file.raw) {
    const reader = new FileReader()
    reader.onload = (e) => {
      const content = e.target?.result as string
      if (content) {
        form.value.sqlText = content
        handleSqlEditFinished(true)
      }
    }
    reader.readAsText(file.raw)
  }
}

// 预执行校验
const handleDryRun = async () => {
  if (!form.value.instanceId) {
    ElMessage.warning('请选择目标数据库实例')
    return
  }
  if (!form.value.dbName) {
    ElMessage.warning('请选择目标数据库')
    return
  }

  const hasText = form.value.sqlText && form.value.sqlText.trim().length > 0
  const hasFile = form.value.file !== null
  if (!hasText && !hasFile) {
    ElMessage.warning('请输入 SQL 语句或上传 SQL 附件')
    return
  }

  // 校验是否存在非 SQL 文本
  if (sqlInputMode.value === 'TEXT' && sqlEditorStats.value.invalidStatements.length > 0) {
    const first = sqlEditorStats.value.invalidStatements[0]
    ElMessage.error(`检测到第 ${first.index} 处内容为非 SQL 文本【${first.snippet}】，请修改或删除非 SQL 内容后再执行预校验！`)
    return
  }

  dryRunLoading.value = true
  try {
    const formData = new FormData()
    formData.append('instanceId', String(form.value.instanceId))
    formData.append('dbName', form.value.dbName)
    if (sqlInputMode.value === 'TEXT' && form.value.sqlText) {
      formData.append('sqlText', form.value.sqlText)
    } else if (form.value.file) {
      formData.append('file', form.value.file)
    }

    const res: any = await request.post('/v1/ticket/dry-run', formData, {
      headers: { 'Content-Type': 'multipart/form-data' }
    })

    if (res.data && res.data.items) {
      res.data.totalActualRows = res.data.items.reduce((sum: number, it: any) => sum + (it.actualRows || 0), 0)
    }

    dryRunResult.value = res.data
    if (res.data?.passed) {
      ElMessage.success(`预执行通过！目标库【${form.value.dbName}】影响行数与注解完全一致`)
    } else {
      ElMessage.error(res.data?.summaryMessage || '预执行校验发现影响行数不匹配！')
    }
  } catch (error) {
    console.error('Dry-run error', error)
  } finally {
    dryRunLoading.value = false
  }
}

// 智能平滑定位与自动聚焦缺失项
const scrollToAndFocusField = (fieldKey: string, fieldName: string) => {
  ElMessage.warning(`【资源组配置约束】请填写必填项：${fieldName}`)

  setTimeout(() => {
    const el = document.getElementById(`custom-field-${fieldKey}`)
    if (el) {
      el.scrollIntoView({ behavior: 'smooth', block: 'center' })
      el.classList.add('field-highlight-pulse')
      setTimeout(() => {
        el.classList.remove('field-highlight-pulse')
      }, 2200)

      // 尝试自动聚焦内部输入框
      const inputEl = el.querySelector('input, textarea') as HTMLInputElement | HTMLTextAreaElement
      if (inputEl) {
        inputEl.focus()
      }
    }
  }, 60)
}

const scrollToElement = (elementId: string, message: string) => {
  ElMessage.warning(message)
  setTimeout(() => {
    const el = document.getElementById(elementId)
    if (el) {
      el.scrollIntoView({ behavior: 'smooth', block: 'center' })
      el.classList.add('field-highlight-pulse')
      setTimeout(() => {
        el.classList.remove('field-highlight-pulse')
      }, 2200)
      const inputEl = el.querySelector('input, textarea') as HTMLInputElement | HTMLTextAreaElement
      if (inputEl) {
        inputEl.focus()
      }
    }
  }, 60)
}

// 提交工单
const handleSubmitTicket = async () => {
  if (!form.value.instanceId) {
    scrollToElement('form-item-instance', '请选择目标数据库实例')
    return
  }
  if (!form.value.dbName) {
    scrollToElement('form-item-dbname', '请选择目标数据库')
    return
  }

  // 0. 校验目标实例是否开启了对应操作权限 (Supported Ops)
  if (!isCurrentTicketTypeSupported.value.supported) {
    ElMessage.error(isCurrentTicketTypeSupported.value.message)
    return
  }

  // 0.1 校验资源组个性化配置的必填扩展字段 (如上线版本号、年月日、执行窗口等) - 缺失时自动滚动并聚焦点位
  for (const field of currentResourceGroupFields.value) {
    if (field.required) {
      const val = customForm.value[field.fieldKey]
      if (val === undefined || val === null || val === '' || (Array.isArray(val) && val.length === 0)) {
        scrollToAndFocusField(field.fieldKey, field.fieldName)
        return
      }
    }
  }

  const isSqlType = form.value.type === 'SQL_AUDIT' || form.value.type === 'DATA_RECOVERY'
  if (isSqlType) {
    const hasText = form.value.sqlText && form.value.sqlText.trim().length > 0
    const hasFile = form.value.file !== null
    if (!hasText && !hasFile) {
      scrollToElement('form-item-sql-workbench', '请输入 SQL 语句或上传附件')
      return
    }

    // 校验是否存在非 SQL 文本
    if (sqlInputMode.value === 'TEXT' && sqlEditorStats.value.invalidStatements.length > 0) {
      const first = sqlEditorStats.value.invalidStatements[0]
      ElMessage.error(`提交拦截：检测到第 ${first.index} 处包含非 SQL 文本【${first.snippet}】，请修改或删除非 SQL 内容后再提交！`)
      return
    }

    // 1. 检查数据备份 / 回滚方案必填策略（仅在启用步骤 3 且策略要求备份时生效）
    if (isStep3Visible.value && safetyPolicies.value.requireBackup) {
      const hasRollbackText = form.value.rollbackSqlText && form.value.rollbackSqlText.trim().length > 0
      const hasRollbackFile = form.value.rollbackFile !== null
      if (!hasRollbackText && !hasRollbackFile) {
        scrollToElement('form-item-rollback', '【生产安全规范】DML / DDL 变更必须提供数据备份或回滚方案（可在线粘贴回滚SQL或上传备份附件）！')
        return
      }
    }

    // 1.1 校验回滚方案中是否包含非 SQL 文本或目标表不匹配
    if (isStep3Visible.value && rollbackInputMode.value === 'TEXT' && form.value.rollbackSqlText && form.value.rollbackSqlText.trim().length > 0) {
      if (rollbackCorrelation.value.status === 'INVALID_SQL') {
        scrollToElement('form-item-rollback', '【回滚方案校验拦截】回滚内容中包含非 SQL 文本，必须输入真实合法的 SQL 语句！')
        return
      }
      if (rollbackCorrelation.value.status === 'TABLE_MISMATCH') {
        scrollToElement('form-item-rollback', `【回滚方案校验拦截】回滚脚本涉及表【${rollbackCorrelation.value.rollbackTables.join(', ')}】与执行脚本涉及表【${rollbackCorrelation.value.execTables.join(', ')}】不匹配，疑似非真实回滚方案！`)
        return
      }
    }

    // 2. 检查资源组强制预执行校验策略（仅在启用步骤 4 且开启强制策略时生效）
    if (isStep4Visible.value && isDryRunEnforced.value) {
      if (!dryRunResult.value || !dryRunResult.value.passed) {
        scrollToElement('form-item-dryrun', '【资源组安全策略】当前资源组已配置「强制预执行校验 (必须通过)」，提交前必须先执行并成功通过事务级预校验！')
        return
      }
    }
  }

  submitLoading.value = true
  try {
    const formData = new FormData()
    formData.append('instanceId', String(form.value.instanceId))
    formData.append('dbName', form.value.dbName)
    formData.append('type', form.value.type)
    if (form.value.reason) {
      formData.append('reason', form.value.reason)
    }
    if (sqlInputMode.value === 'TEXT' && form.value.sqlText) {
      formData.append('sqlText', form.value.sqlText)
    }
    if (sqlInputMode.value === 'FILE' && form.value.file) {
      formData.append('file', form.value.file)
    }
    if (rollbackInputMode.value === 'TEXT' && form.value.rollbackSqlText) {
      formData.append('rollbackSqlText', form.value.rollbackSqlText)
    }
    if (rollbackInputMode.value === 'FILE' && form.value.rollbackFile) {
      formData.append('rollbackFile', form.value.rollbackFile)
    }

    // 计算并传递预执行累计影响行数
    let totalExpectedRows = 0
    if (dryRunResult.value?.totalActualRows !== undefined) {
      totalExpectedRows = dryRunResult.value.totalActualRows
    } else if (dryRunResult.value?.totalExpectedRows !== undefined) {
      totalExpectedRows = dryRunResult.value.totalExpectedRows
    }
    if (totalExpectedRows > 0) {
      formData.append('expectedRows', String(totalExpectedRows))
    }

    // 附加自定义扩展字段值
    if (currentResourceGroupFields.value.length > 0) {
      formData.append('customFieldValues', JSON.stringify(customForm.value))
    }

    const res: any = await request.post('/v1/ticket/submit', formData, {
      headers: { 'Content-Type': 'multipart/form-data' }
    })

    if (isCurrentInstanceTestEnv.value) {
      ElMessage.success('工单已成功提交并在测试环境中直接自动执行！')
    } else {
      ElMessage.success('工单提交成功，已完成预校验与回滚方案归档并进入审批流！')
    }
    router.push(`/ticket/${res.data.id}`)
  } catch (error: any) {
    console.error('Submit ticket error', error)
  } finally {
    submitLoading.value = false
  }
}

const handleResetForm = () => {
  dryRunResult.value = null
  form.value.reason = ''
  form.value.sqlText = ''
  form.value.file = null
  form.value.rollbackSqlText = ''
  form.value.rollbackFile = null
  ElMessage.info('表单已重置')
}

// 复制/重新申请工单历史数据载入
const cloneSourceTicket = ref<any>(null)

const getTicketStatusText = (status: string) => {
  const map: Record<string, string> = {
    AUDITING: '审批中',
    PENDING_APPROVAL: '待审批',
    APPROVED: '审批通过 / 待执行',
    EXECUTING: '正在执行',
    EXECUTED: '执行成功',
    FAILED: '执行失败',
    REJECTED: '已驳回',
    TERMINATED: '已终止'
  }
  return map[status] || status
}

const handleKeyDown = (e: KeyboardEvent) => {
  if (e.key === 'Escape' && isSqlFullscreen.value) {
    isSqlFullscreen.value = false
  }
}

const loadTicketForClone = async (ticketId: number | string) => {
  try {
    const res: any = await request.get(`/v1/ticket/${ticketId}/detail`)
    const data = res.data
    if (data && data.ticket) {
      cloneSourceTicket.value = {
        id: data.ticket.id,
        status: data.ticket.status,
        type: data.ticket.type,
        applicantName: data.ticket.applicantName
      }
      form.value.instanceId = data.ticket.instanceId
      if (data.ticket.instanceId) {
        await loadDatabasesForInstance(data.ticket.instanceId)
      }
      form.value.dbName = data.ticket.dbName || ''
      form.value.type = data.ticket.type || 'SQL_AUDIT'
      form.value.reason = data.ticket.reason || ''
      if (data.detail) {
        sqlInputMode.value = 'TEXT'
        form.value.sqlText = data.detail.sqlText || ''
        rollbackInputMode.value = 'TEXT'
        form.value.rollbackSqlText = data.detail.rollbackSqlText || ''
        if (data.detail.customFieldValues) {
          try {
            customForm.value = JSON.parse(data.detail.customFieldValues)
          } catch (e) {}
        }
      }
      // 触发审批流动态预判
      await fetchRoutingPreview()
      ElNotification({
        title: '已自动加载历史工单信息',
        message: `已为您将历史工单 #${data.ticket.id} 的所有配置、SQL 脚本与回滚方案自动载入，您可以修改后重新提交！`,
        type: 'success',
        duration: 5000
      })
    }
  } catch (err: any) {
    ElMessage.error(err.message || '加载原工单数据失败')
  }
}

onMounted(async () => {
  window.addEventListener('keydown', handleKeyDown)
  try {
    fetchSafetyPolicies()
    loadUserInfo()
    await loadResourceGroups()
    await loadInstances()
    const fromTicketId = route.query.fromTicketId
    if (fromTicketId) {
      await loadTicketForClone(fromTicketId as string)
    }
  } catch (err) {
    console.error('Initialization error in CreateTicket workbench:', err)
  }
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', handleKeyDown)
})
</script>

<style scoped>
.create-ticket-workbench {
  min-height: 100%;
  height: auto;
  width: 100%;
  max-width: 100%;
  min-width: 0;
  display: flex;
  flex-direction: column;
  box-sizing: border-box;
}

.workbench-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 14px;
  flex-wrap: wrap;
  gap: 12px;
  padding: 16px 20px;
  background: linear-gradient(120deg, #ffffff 0%, #f5f9ff 60%, #eef4ff 100%);
  border: 1px solid #e2e8f0;
  border-radius: 12px;
  box-shadow: 0 1px 3px rgba(15, 23, 42, 0.04);
}

.header-left {
  display: flex;
  align-items: center;
  gap: 14px;
}

.back-btn {
  flex-shrink: 0;
}

.crumb-line {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: #94a3b8;
  margin-bottom: 2px;
}

.crumb-link {
  cursor: pointer;
  color: #64748b;
  transition: color 0.15s;
}

.crumb-link:hover {
  color: #2563eb;
}

.crumb-sep {
  color: #cbd5e1;
}

.header-titles .main-title {
  font-size: 20px;
  font-weight: 800;
  color: #0f172a;
  margin: 0;
  letter-spacing: 0.2px;
}

.header-titles .sub-title {
  font-size: 12.5px;
  color: #64748b;
  margin-top: 4px;
  display: block;
}

.header-right {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.hdr-pill {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 30px;
  padding: 0 12px;
  border-radius: 999px;
  font-size: 12.5px;
  font-weight: 600;
  border: 1px solid transparent;
}

.hdr-pill-danger {
  background: #fff1f2;
  border-color: #fecdd3;
  color: #be123c;
}

.hdr-pill-muted {
  background: #f1f5f9;
  border-color: #e2e8f0;
  color: #475569;
}

.hdr-pill-success {
  background: #ecfdf5;
  border-color: #a7f3d0;
  color: #047857;
}

.hdr-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: #10b981;
  box-shadow: 0 0 0 3px rgba(16, 185, 129, 0.18);
}

/* 吸顶步骤导航 */
.create-step-nav {
  position: sticky;
  top: 0;
  z-index: 20;
  display: flex;
  align-items: center;
  flex-shrink: 0;
  min-height: 54px;
  gap: 4px;
  padding: 10px 16px;
  margin-bottom: 4px;
  background: rgba(255, 255, 255, 0.94);
  backdrop-filter: blur(8px);
  border: 1px solid #e2e8f0;
  border-radius: 12px;
  box-shadow: 0 4px 14px rgba(15, 23, 42, 0.05);
  overflow-x: auto;
  overflow-y: hidden;
}

.step-nav-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  border-radius: 8px;
  cursor: pointer;
  flex-shrink: 0;
  transition: background 0.15s ease;
}

.step-nav-item:hover {
  background: #f1f5f9;
}

.step-nav-circle {
  width: 26px;
  height: 26px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  font-weight: 800;
  background: #e2e8f0;
  color: #475569;
  transition: all 0.2s ease;
}

.step-nav-item.is-required-pending .step-nav-circle {
  background: #eff6ff;
  color: #2563eb;
  box-shadow: inset 0 0 0 2px #93c5fd;
}

.step-nav-item.is-done .step-nav-circle {
  background: linear-gradient(135deg, #10b981, #059669);
  color: #ffffff;
  box-shadow: 0 2px 6px rgba(16, 185, 129, 0.3);
}

.step-nav-text {
  display: flex;
  flex-direction: column;
  line-height: 1.25;
}

.step-nav-title {
  font-size: 13px;
  font-weight: 700;
  color: #1e293b;
}

.step-nav-desc {
  font-size: 11px;
  color: #94a3b8;
}

.step-nav-item.is-done .step-nav-desc {
  color: #059669;
}

.step-nav-item.is-required-pending .step-nav-desc {
  color: #2563eb;
}

.step-nav-line {
  flex: 1;
  min-width: 24px;
  height: 2px;
  border-radius: 2px;
  background: #e2e8f0;
}

.step-nav-line.is-done {
  background: linear-gradient(90deg, #10b981, #6ee7b7);
}

.step-nav-progress {
  margin-left: 12px;
  padding-left: 14px;
  border-left: 1px solid #e2e8f0;
  display: flex;
  flex-direction: column;
  align-items: center;
  flex-shrink: 0;
}

.progress-num {
  font-size: 17px;
  font-weight: 800;
  color: #2563eb;
  font-family: 'JetBrains Mono', Consolas, monospace;
  line-height: 1.1;
}

.progress-label {
  font-size: 10.5px;
  color: #94a3b8;
}

.workbench-tabs {
  flex: 1;
  background-color: #ffffff;
  border-radius: 6px;
  box-shadow: 0 1px 4px rgba(0, 21, 41, 0.06);
}

.tab-label-custom {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-weight: 600;
  font-size: 14px;
}

/* ==================== 页签 1 布局样式 ==================== */
.ticket-creation-stream {
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding: 12px 0 24px 0;
  width: 100%;
  max-width: 100%;
  min-width: 0;
  box-sizing: border-box;
}

.stream-section-card {
  position: relative;
  background: #ffffff;
  border: 1px solid #e2e8f0;
  border-radius: 12px;
  padding: 18px 22px 20px 22px;
  width: 100%;
  max-width: 100%;
  min-width: 0;
  box-sizing: border-box;
  box-shadow: 0 1px 3px rgba(15, 23, 42, 0.04);
  transition: box-shadow 0.2s ease, border-color 0.2s ease;
  scroll-margin-top: 84px;
}

.stream-section-card.is-collapsed-card {
  padding-bottom: 14px;
}

.section-header-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.basic-collapsed-summary {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: 10px;
  margin-top: 10px;
  padding: 10px 14px;
  background: linear-gradient(90deg, #f8fafc 0%, #eff6ff 100%);
  border: 1px solid #cbd5e1;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s ease;
}

.basic-collapsed-summary:hover {
  border-color: #60a5fa;
  background: linear-gradient(90deg, #f1f5f9 0%, #dbeafe 100%);
}

.summary-Left-pills {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
  font-size: 12.5px;
}

.summary-chip {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  color: #334155;
}

.summary-chip .chip-k {
  color: #64748b;
  font-size: 12px;
}

.summary-chip .chip-v {
  color: #0f172a;
  font-weight: 700;
}

.summary-chip .chip-mono {
  font-family: 'JetBrains Mono', Consolas, monospace;
  color: #2563eb;
}

.summary-chip-flow {
  background: #eff6ff;
  border: 1px solid #bfdbfe;
  color: #1d4ed8;
  padding: 2px 8px;
  border-radius: 999px;
  font-weight: 600;
  font-size: 12px;
}

.summary-sep {
  color: #cbd5e1;
}

.summary-expand-hint {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 12px;
  font-weight: 600;
  color: #2563eb;
}

.basic-section-body {
  margin-top: 12px;
  padding-top: 12px;
  border-top: 1px dashed #e2e8f0;
}

.compact-basic-form :deep(.el-form-item) {
  margin-bottom: 12px;
}

.compact-basic-form :deep(.el-form-item__label) {
  padding-bottom: 4px;
  line-height: 1.35;
}

.basic-bottom-fold-bar {
  margin-top: 12px;
  padding-top: 10px;
  border-top: 1px dashed #e2e8f0;
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: 8px;
}

.fold-bar-hint {
  font-size: 12px;
  color: #64748b;
}

.stream-section-card::before {
  content: '';
  position: absolute;
  left: 0;
  top: 18px;
  width: 3px;
  height: 22px;
  border-radius: 0 3px 3px 0;
  background: linear-gradient(180deg, #3b82f6, #2563eb);
}

.stream-section-card:hover {
  box-shadow: 0 6px 18px rgba(15, 23, 42, 0.06);
  border-color: #cbd5e1;
}

.section-badge {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 16px;
  padding-bottom: 12px;
  border-bottom: 1px dashed #e2e8f0;
}

.badge-num {
  width: 26px;
  height: 26px;
  background: linear-gradient(135deg, #3b82f6 0%, #2563eb 100%);
  color: #fff;
  border-radius: 8px;
  font-size: 13px;
  font-weight: 800;
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 2px 6px rgba(37, 99, 235, 0.25);
  flex-shrink: 0;
}

.badge-title {
  font-size: 15.5px;
  font-weight: 700;
  color: #0f172a;
}

.custom-form :deep(.el-form-item__label) {
  font-weight: 600;
  color: #334155;
}

.field-hint {
  font-size: 12px;
  color: #94a3b8;
  margin-top: 4px;
  line-height: 1.5;
}

.instance-security-meta-card {
  margin-top: 8px;
  padding: 10px 12px;
  background: linear-gradient(180deg, #f8fafc 0%, #ffffff 100%);
  border: 1px solid #e2e8f0;
  border-radius: 8px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.meta-row {
  display: flex;
  align-items: center;
  font-size: 12px;
  line-height: 1.4;
}

.meta-label {
  font-weight: 600;
  color: #475569;
  min-width: 105px;
}

.meta-tag-list {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 4px;
}

.syntax-tip-card {
  margin-top: 20px;
  background: #ecf5ff;
  border-left: 4px solid #409EFF;
  border-radius: 4px;
  padding: 12px 14px;
}

.syntax-tip-card .tip-header {
  display: flex;
  align-items: center;
  gap: 6px;
  font-weight: 600;
  color: #409EFF;
  font-size: 13px;
  margin-bottom: 6px;
}

.syntax-tip-card .tip-body {
  font-size: 12px;
  color: #606266;
  line-height: 1.6;
}

.syntax-tip-card code {
  background: #d9ecff;
  padding: 1px 5px;
  border-radius: 3px;
  color: #e6a23c;
  font-weight: bold;
}

.input-mode-tabs {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 10px;
  flex-wrap: wrap;
  gap: 10px;
}

.editor-wrapper {
  border: 1px solid #dcdfe6;
  border-radius: 6px;
  overflow: hidden;
}

.code-textarea :deep(textarea) {
  font-family: Consolas, 'Fira Code', Monaco, monospace;
  font-size: 13px;
  line-height: 1.6;
  background-color: #fafbfc;
  border: none;
}

/* 应急回滚预案说明等 TEXTAREA 类型资源组字段样式 */
.custom-textarea-wrapper {
  width: 100%;
}

.custom-field-textarea :deep(.el-textarea__inner) {
  font-size: 13px;
  line-height: 1.7;
  border-color: #e2e8f0;
  border-radius: 6px;
  background: #fffdf0;
  color: #334155;
  padding: 10px 12px;
  transition: border-color 0.2s, box-shadow 0.2s;
}

.custom-field-textarea :deep(.el-textarea__inner:focus) {
  border-color: #f59e0b;
  box-shadow: 0 0 0 2px rgba(245, 158, 11, 0.15);
}

.textarea-tip {
  display: flex;
  align-items: center;
  margin-top: 6px;
  padding: 6px 10px;
  background: #fffbeb;
  border: 1px solid #fde68a;
  border-radius: 5px;
  font-size: 12px;
  color: #92400e;
  line-height: 1.5;
}

.upload-box {
  width: 100%;
}

.dry-run-action-bar {
  display: flex;
  align-items: center;
  gap: 14px;
  margin-bottom: 12px;
  flex-wrap: wrap;
}

.dry-run-desc {
  font-size: 13px;
  color: #909399;
}

.dry-run-desc code {
  background: #f4f4f5;
  padding: 1px 4px;
  border-radius: 3px;
  color: #e6a23c;
}

.dry-run-result-box {
  margin-top: 10px;
  background: #fafbfc;
  padding: 12px;
  border-radius: 6px;
  border: 1px solid #ebeef5;
}

.submit-footer-bar {
  position: sticky;
  bottom: 0;
  z-index: 15;
  margin: 24px -22px -20px -22px;
  padding: 14px 22px;
  border-top: 1px solid #e2e8f0;
  border-radius: 0 0 12px 12px;
  background: rgba(255, 255, 255, 0.96);
  backdrop-filter: blur(6px);
  box-shadow: 0 -6px 16px rgba(15, 23, 42, 0.05);
  display: flex;
  justify-content: flex-end;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
}

.submit-ready-hint {
  margin-right: auto;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 12.5px;
  font-weight: 600;
  color: #b45309;
  background: #fffbeb;
  border: 1px solid #fde68a;
  padding: 5px 12px;
  border-radius: 999px;
}

.submit-ready-hint.is-ready {
  color: #047857;
  background: #ecfdf5;
  border-color: #a7f3d0;
}

.submit-diff-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  height: 230px;
  overflow: hidden;
}

@media (max-width: 900px) {
  .workbench-header {
    padding: 14px;
  }
  .header-titles .sub-title,
  .step-nav-desc,
  .step-nav-progress {
    display: none;
  }
  .stream-section-card {
    padding: 14px 14px 16px 14px;
  }
  .submit-footer-bar {
    margin: 20px -14px -16px -14px;
    padding: 12px 14px;
  }
  .submit-ready-hint {
    width: 100%;
  }
  .submit-diff-grid {
    grid-template-columns: 1fr;
    height: auto;
  }
  .submit-diff-grid > div {
    max-height: 200px;
  }
}

/* ==================== 页签 2 样式 ==================== */
.query-workbench-layout {
  display: flex;
  flex-direction: column;
  gap: 14px;
  padding: 8px 0;
}

.query-toolbar-card {
  background: #f8f9fb;
  border: 1px solid #e4e7ed;
  border-radius: 6px;
  padding: 14px 18px;
}

.toolbar-form-row {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 16px;
}

.form-item-inline {
  display: flex;
  align-items: center;
  gap: 8px;
}

.form-item-inline .item-label {
  font-size: 13px;
  font-weight: 600;
  color: #606266;
}

.action-buttons-group {
  margin-left: auto;
  display: flex;
  gap: 10px;
}

.quick-query-templates {
  margin-top: 12px;
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
}

.template-label {
  font-size: 12px;
  color: #909399;
  font-weight: 500;
}

.clickable-tag {
  cursor: pointer;
  transition: all 0.2s;
}

.clickable-tag:hover {
  border-color: #409EFF;
  color: #409EFF;
  background: #ecf5ff;
}

.query-editor-card {
  border: 1px solid #dcdfe6;
  border-radius: 6px;
  background: #ffffff;
  overflow: hidden;
}

.editor-header-bar {
  background: #f5f7fa;
  padding: 8px 14px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  border-bottom: 1px solid #e4e7ed;
}

.editor-header-bar .header-title {
  font-size: 13px;
  font-weight: 600;
  color: #303133;
  display: flex;
  align-items: center;
  gap: 6px;
}

.query-code-textarea :deep(textarea) {
  font-family: Consolas, 'Fira Code', Monaco, monospace;
  font-size: 13px;
  line-height: 1.5;
  background: #282c34;
  color: #abb2bf;
  border: none;
}

.query-result-card {
  border: 1px solid #e4e7ed;
  border-radius: 6px;
  background: #ffffff;
  min-height: 260px;
}

.result-status-bar {
  background: #f5f7fa;
  padding: 10px 16px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  border-bottom: 1px solid #ebeef5;
}

.status-badge {
  font-size: 12px;
  font-weight: bold;
  padding: 2px 8px;
  border-radius: 4px;
  background: #909399;
  color: #ffffff;
  margin-right: 10px;
}

.status-badge.success {
  background: #67C23A;
}

.status-badge.error {
  background: #F56C6C;
}

.meta-info {
  font-size: 13px;
  color: #606266;
}

.meta-info b {
  color: #303133;
}

.table-container {
  padding: 10px;
}

.null-val {
  color: #c0c4cc;
  font-style: italic;
}

.empty-state {
  padding: 30px 0;
}

/* ==================== SQL 编辑器真全屏与回滚样式 ==================== */
.section-badge-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

/* ==================== SQL 编辑工作台 (行号、字符数、全屏模式) ==================== */
.editor-workbench-container {
  display: flex;
  flex-direction: column;
  border: 1px solid #dcdfe6;
  border-radius: 6px;
  background: #ffffff;
  overflow: hidden;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.04);
  transition: border-color 0.2s, box-shadow 0.2s;
}

.editor-workbench-container:focus-within {
  border-color: #409EFF;
  box-shadow: 0 0 0 2px rgba(64, 158, 255, 0.15);
}

.code-editor-box {
  display: flex;
  height: 220px;
  background: #fafbfc;
  position: relative;
}

.editor-gutter {
  width: 52px;
  min-width: 52px;
  background: #f1f5f9;
  border-right: 1px solid #e2e8f0;
  padding: 10px 0;
  overflow: hidden;
  user-select: none;
  text-align: right;
  box-sizing: border-box;
}

.gutter-num {
  font-family: Consolas, 'Fira Code', Menlo, Monaco, monospace;
  font-size: 13px;
  line-height: 21px;
  color: #94a3b8;
  padding-right: 12px;
  height: 21px;
}

.editor-textarea-native {
  flex: 1;
  border: none;
  outline: none;
  resize: none;
  background: transparent;
  padding: 10px 14px;
  font-family: Consolas, 'Fira Code', Menlo, Monaco, monospace;
  font-size: 13.5px;
  line-height: 21px;
  color: #1e293b;
  box-sizing: border-box;
  overflow-y: auto;
  white-space: pre;
}

.editor-textarea-native::placeholder {
  color: #94a3b8;
  font-style: italic;
}

.editor-status-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
  padding: 6px 14px;
  background: #f8fafc;
  border-top: 1px solid #e2e8f0;
  font-size: 12px;
  color: #64748b;
}

.status-left {
  display: flex;
  align-items: center;
  gap: 16px;
  flex-wrap: wrap;
}

.status-chip {
  display: inline-flex;
  align-items: center;
}

.status-chip b {
  color: #1e293b;
  margin: 0 2px;
}

.status-right {
  display: flex;
  align-items: center;
  gap: 10px;
}

.esc-hint {
  font-size: 11.5px;
  color: #94a3b8;
}

.esc-hint b {
  color: #475569;
  background: #e2e8f0;
  padding: 1px 5px;
  border-radius: 3px;
}

/* 全屏工作台模式 */
.stream-section-card.is-fullscreen-mode {
  position: fixed !important;
  top: 0 !important;
  left: 0 !important;
  width: 100vw !important;
  height: 100vh !important;
  z-index: 3500 !important;
  background: #ffffff !important;
  padding: 16px 28px !important;
  box-sizing: border-box !important;
  margin: 0 !important;
  border-radius: 0 !important;
  display: flex !important;
  flex-direction: column !important;
  box-shadow: 0 0 40px rgba(0, 0, 0, 0.35) !important;
  animation: fadeInFullscreen 0.2s ease-in-out;
}

.stream-section-card.is-fullscreen-mode .editor-workbench-container {
  flex: 1;
  height: calc(100vh - 160px);
  margin-top: 10px;
}

.stream-section-card.is-fullscreen-mode .code-editor-box {
  flex: 1;
  height: 100%;
}

.stream-section-card.is-fullscreen-mode .editor-textarea-native {
  font-size: 15px;
  line-height: 24px;
}

.stream-section-card.is-fullscreen-mode .gutter-num {
  font-size: 14px;
  line-height: 24px;
  height: 24px;
}

@keyframes fadeInFullscreen {
  from { opacity: 0; transform: scale(0.98); }
  to { opacity: 1; transform: scale(1); }
}

/* 必填项缺失时自动高亮聚焦聚光动效 */
@keyframes pulseHighlight {
  0% { box-shadow: 0 0 0 0 rgba(239, 68, 68, 0.4); transform: scale(1); }
  50% { box-shadow: 0 0 0 8px rgba(239, 68, 68, 0.2); transform: scale(1.008); }
  100% { box-shadow: 0 0 0 0 rgba(239, 68, 68, 0); transform: scale(1); }
}

.field-highlight-pulse {
  animation: pulseHighlight 0.7s ease-in-out 3;
  border-radius: 8px;
  background-color: #fff1f2 !important;
  transition: all 0.3s ease;
}

.field-highlight-pulse :deep(.el-input__wrapper),
.field-highlight-pulse :deep(.el-textarea__inner),
.field-highlight-pulse :deep(.editor-workbench-container) {
  border-color: #ef4444 !important;
  box-shadow: 0 0 0 3px rgba(239, 68, 68, 0.25) !important;
}

.rollback-textarea :deep(.el-textarea__inner) {
  border-color: #86efac;
  background-color: #f0fdf4;
  font-family: monospace;
}

.dry-run-gateway-hint {
  margin-bottom: 10px;
  background: #f8fafc;
  border: 1px solid #e2e8f0;
  border-radius: 4px;
  padding: 8px 12px;
  font-size: 12px;
  color: #334155;
  display: flex;
  align-items: center;
}

/* ==================== 审批流程预估预览卡片 & BPMN 2.0 五阶段高清流转画布 ==================== */
.routing-preview-card {
  margin-top: 10px;
  background: #ffffff;
  border: 1px solid #cbd5e1;
  border-top: 3px solid #2563eb;
  border-radius: 12px;
  padding: 12px 16px;
  width: 100%;
  max-width: 100%;
  min-width: 0;
  box-sizing: border-box;
  overflow: hidden;
  box-shadow: 0 4px 16px rgba(15, 23, 42, 0.05);
}

.routing-preview-card.is-pinned-flow {
  border-top-color: #d97706;
  background: linear-gradient(180deg, #fffbeb 0%, #ffffff 100%);
}

.routing-card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: 10px;
  padding-bottom: 10px;
  border-bottom: 1px dashed #e2e8f0;
}

.preview-header-left {
  display: flex;
  align-items: center;
  gap: 8px;
}

.preview-icon-badge {
  width: 26px;
  height: 26px;
  border-radius: 7px;
  background: #eff6ff;
  border: 1px solid #bfdbfe;
  color: #2563eb;
  display: flex;
  align-items: center;
  justify-content: center;
}

.preview-icon-badge.is-pinned {
  background: #fef3c7;
  border-color: #fde68a;
  color: #d97706;
}

.preview-title {
  font-weight: 600;
  font-size: 13.5px;
  color: #334155;
}

.preview-title b {
  font-weight: 800;
  color: #0f172a;
}

.preview-header-right-tags {
  display: flex;
  gap: 6px;
  align-items: center;
  flex-wrap: wrap;
}

.bpmn-top-pill {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 3px 9px;
  border-radius: 6px;
  font-size: 11.5px;
  font-weight: 600;
}

.bpmn-top-pill.pill-danger {
  background: #fff1f2;
  border: 1px solid #fecdd3;
  color: #e11d48;
}

.bpmn-top-pill.pill-success {
  background: #ecfdf5;
  border: 1px solid #a7f3d0;
  color: #059669;
}

.bpmn-top-pill.pill-spel {
  background: #fffbeb;
  border: 1px solid #fde68a;
  color: #b45309;
  font-family: 'JetBrains Mono', Consolas, monospace;
}

.bpmn-top-pill.pill-smart {
  background: linear-gradient(135deg, #10b981, #059669);
  color: #ffffff;
  box-shadow: 0 2px 6px rgba(16, 185, 129, 0.22);
}

.bpmn-top-pill.pill-pinned {
  background: linear-gradient(135deg, #f59e0b, #d97706);
  color: #ffffff;
}

/* 结构化判定依据栏 */
.decision-basis-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: 8px;
  background: #f8fafc;
  border: 1px solid #e2e8f0;
  border-radius: 8px;
  padding: 8px 12px;
  margin: 10px 0 12px 0;
  font-size: 12px;
}

.basis-left-items {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 6px;
}

.basis-main-label {
  font-weight: 700;
  color: #1e293b;
}

.basis-chip {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  background: #ffffff;
  border: 1px solid #cbd5e1;
  border-radius: 5px;
  padding: 2px 7px;
  color: #475569;
  font-size: 11.5px;
}

.basis-chip b {
  color: #0f172a;
  font-weight: 700;
}

.basis-mini-tag {
  background: #eff6ff;
  color: #2563eb;
  border: 1px solid #bfdbfe;
  border-radius: 4px;
  padding: 0 5px;
  font-size: 10.5px;
  font-weight: 600;
}

.basis-chip-rule {
  background: #fffbeb;
  border-color: #fde68a;
  color: #92400e;
}

.basis-chip-trigger {
  background: #f1f5f9;
  border-color: #cbd5e1;
  color: #475569;
}

.basis-plus {
  color: #94a3b8;
  font-weight: 700;
}

.basis-raw-text {
  color: #334155;
  font-weight: 500;
}

.basis-right-controls {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.active-route-pill {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 3px 10px;
  border-radius: 999px;
  font-size: 11.5px;
  font-weight: 700;
}

.route-pill-normal {
  background: #eff6ff;
  border: 1px solid #93c5fd;
  color: #1d4ed8;
}

.route-pill-high {
  background: #fef2f2;
  border: 1px solid #fca5a5;
  color: #b91c1c;
}

.route-pulse-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  display: inline-block;
  animation: routePulseGlow 1.6s infinite;
}

@keyframes routePulseGlow {
  0%, 100% { transform: scale(1); opacity: 1; }
  50% { transform: scale(1.35); opacity: 0.55; }
}

.branch-preview-switcher {
  display: inline-flex;
  background: #e2e8f0;
  padding: 2px;
  border-radius: 6px;
  gap: 2px;
}

.sw-btn {
  border: none;
  background: transparent;
  color: #475569;
  font-size: 11px;
  font-weight: 600;
  padding: 2px 7px;
  border-radius: 4px;
  cursor: pointer;
  transition: all 0.15s ease;
}

.sw-btn:hover {
  color: #0f172a;
}

.sw-btn.active {
  background: #ffffff;
  color: #0f172a;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
}

.sw-btn.sw-normal.active {
  background: #2563eb;
  color: #ffffff;
}

.sw-btn.sw-high.active {
  background: #dc2626;
  color: #ffffff;
}

/* BPMN 2.0 流程图主画布 */
.bpmn-flow-visual-box {
  display: block;
  width: 100%;
  max-width: 100%;
  min-width: 0;
  box-sizing: border-box;
  background:
    radial-gradient(#cbd5e1 1px, transparent 1px),
    linear-gradient(180deg, #f8fafc 0%, #f1f5f9 100%);
  background-size: 16px 16px, 100% 100%;
  padding: 12px 16px 16px 16px;
  border-radius: 10px;
  border: 1px solid #cbd5e1;
  overflow-x: auto;
  box-shadow: inset 0 1px 2px rgba(255, 255, 255, 0.8), 0 2px 6px rgba(0, 0, 0, 0.03);
}

/* 顶部 5 阶段步骤条 */
.bpmn-stage-header-track {
  display: grid;
  grid-template-columns: minmax(115px, 1fr) 36px minmax(145px, 1.2fr) 56px minmax(260px, 2.2fr) 56px minmax(150px, 1.2fr) 36px minmax(115px, 1fr);
  align-items: center;
  margin-bottom: 10px;
  padding-bottom: 8px;
  border-bottom: 1px solid rgba(203, 213, 225, 0.75);
  min-width: 920px;
}

.stage-col-head {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 5px;
  font-size: 11px;
  font-weight: 700;
  color: #475569;
}

.stage-step-num {
  background: #2563eb;
  color: #ffffff;
  border-radius: 4px;
  padding: 1px 5px;
  font-size: 10px;
  font-family: monospace;
  font-weight: 800;
}

/* 5 阶段主流程网格 */
.bpmn-stage-grid {
  display: grid;
  grid-template-columns: minmax(115px, 1fr) 36px minmax(145px, 1.2fr) 56px minmax(260px, 2.2fr) 56px minmax(150px, 1.2fr) 36px minmax(115px, 1fr);
  align-items: center;
  min-width: 920px;
}

.bpmn-linear-track {
  display: flex;
  align-items: center;
  flex-wrap: nowrap;
  gap: 6px;
  padding: 4px 2px;
}

.bpmn-node-pill {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 3px;
  padding: 10px 12px;
  border-radius: 12px;
  font-size: 13px;
  font-weight: 700;
  white-space: nowrap;
  flex-shrink: 0;
  box-shadow: 0 2px 6px rgba(15, 23, 42, 0.06);
}

.pill-main-row {
  display: flex;
  align-items: center;
  gap: 6px;
}

.pill-sub-text {
  font-size: 10px;
  color: #64748b;
  font-weight: 500;
}

.bpmn-start-pill {
  background: linear-gradient(135deg, #ecfdf5 0%, #d1fae5 100%);
  border: 2px solid #059669;
  color: #065f46;
}

.bpmn-end-pill {
  background: linear-gradient(135deg, #ffffff 0%, #f1f5f9 100%);
  border: 2px solid #334155;
  color: #0f172a;
}

.bpmn-circle {
  width: 12px;
  height: 12px;
  border-radius: 50%;
}

.start-circle {
  background: #10b981;
  border: 2px solid #047857;
  box-shadow: 0 0 0 3px rgba(16, 185, 129, 0.2);
}

.end-circle {
  background: #1e293b;
  border: 3px solid #64748b;
}

.bpmn-svg-arrow-wrap,
.bpmn-branch-fork-svg-wrap,
.bpmn-branch-join-svg-wrap {
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

/* SpEL 排他条件网关卡片 */
.bpmn-gateway-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  background: #ffffff;
  border: 1.5px solid #f59e0b;
  border-radius: 10px;
  padding: 9px 8px;
  box-shadow: 0 3px 10px rgba(245, 158, 11, 0.12);
}

.bpmn-gateway-diamond {
  width: 34px;
  height: 34px;
  border: 2px solid #d97706;
  border-radius: 6px;
  background: linear-gradient(135deg, #fef3c7 0%, #fde68a 100%);
  transform: rotate(45deg);
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 7px;
  box-shadow: 0 2px 5px rgba(217, 119, 6, 0.2);
  transition: transform 0.2s ease;
}

.bpmn-gateway-diamond:hover {
  transform: rotate(45deg) scale(1.06);
}

.diamond-x {
  transform: rotate(-45deg);
  font-weight: 900;
  font-size: 15px;
  color: #92400e;
}

.gw-card-title {
  font-size: 11.5px;
  font-weight: 800;
  color: #1e293b;
  margin-bottom: 3px;
}

.gw-spel-pill {
  color: #b45309;
  font-family: 'JetBrains Mono', Consolas, monospace;
  font-size: 10px;
  background: #fffbeb;
  border: 1px solid #fcd34d;
  padding: 2px 5px;
  border-radius: 4px;
  text-align: center;
  line-height: 1.3;
  word-break: break-all;
}

/* 上下双分支卡片堆栈 */
.bpmn-branches-col {
  display: flex;
  flex-direction: column;
  gap: 10px;
  flex-shrink: 0;
}

.bpmn-branch-card {
  border-radius: 10px;
  padding: 8px 11px;
  background: #ffffff;
  border: 2px solid #cbd5e1;
  display: flex;
  flex-direction: column;
  gap: 5px;
  transition: all 0.25s ease;
}

.branch-card-high {
  border-color: #fca5a5;
  background: linear-gradient(135deg, #fff5f5 0%, #ffffff 100%);
}

.branch-card-normal {
  border-color: #93c5fd;
  background: linear-gradient(135deg, #eff6ff 0%, #ffffff 100%);
}

.bpmn-branch-card.is-active-high {
  border-color: #dc2626;
  background: linear-gradient(135deg, #fef2f2 0%, #fff1f2 100%);
  box-shadow: 0 4px 14px rgba(220, 38, 38, 0.16), 0 0 0 3px rgba(239, 68, 68, 0.12);
}

.bpmn-branch-card.is-active-normal {
  border-color: #2563eb;
  background: linear-gradient(135deg, #eff6ff 0%, #dbeafe 100%);
  box-shadow: 0 4px 14px rgba(37, 99, 235, 0.16), 0 0 0 3px rgba(59, 130, 246, 0.14);
}

.bpmn-branch-card.is-standby-branch {
  opacity: 0.72;
  filter: saturate(0.75);
}

.branch-card-topbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 4px;
}

.branch-tag-label {
  font-size: 10.5px;
  font-weight: 700;
  line-height: 1.2;
}

.branch-tag-label code {
  font-family: 'JetBrains Mono', Consolas, monospace;
  font-size: 10px;
}

.high-label {
  color: #dc2626;
}

.normal-label {
  color: #2563eb;
}

.branch-hit-badge {
  font-size: 10px;
  font-weight: 800;
  padding: 1px 6px;
  border-radius: 999px;
  white-space: nowrap;
  flex-shrink: 0;
}

.badge-hit-high {
  background: #dc2626;
  color: #ffffff;
}

.badge-hit-normal {
  background: #2563eb;
  color: #ffffff;
}

.badge-standby {
  background: #e2e8f0;
  color: #64748b;
}

.branch-card-body {
  display: flex;
  align-items: center;
  gap: 9px;
}

.bpmn-task-card {
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 10px 12px;
  border-radius: 10px;
  border: 2px solid #cbd5e1;
  background: #ffffff;
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.05);
  transition: transform 0.2s ease, box-shadow 0.2s ease;
}

.bpmn-task-card:hover {
  transform: translateY(-1px);
  box-shadow: 0 4px 10px rgba(0, 0, 0, 0.08);
}

.task-dba {
  border-color: #ef4444;
  background: #fef2f2;
}

.task-lead {
  border-color: #3b82f6;
  background: #eff6ff;
}

.task-service {
  border-color: #10b981;
  background: linear-gradient(135deg, #f0fdf4 0%, #dcfce7 100%);
  box-shadow: 0 3px 10px rgba(16, 185, 129, 0.12);
}

.task-icon-col {
  width: 30px;
  height: 30px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.icon-box-dba {
  background: #fee2e2;
  border: 1px solid #fecaca;
}

.icon-box-lead {
  background: #dbeafe;
  border: 1px solid #bfdbfe;
}

.icon-box-service {
  background: #dcfce7;
  border: 1px solid #bbf7d0;
}

.task-title-col {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.t-name-row {
  display: flex;
  align-items: center;
  gap: 5px;
  flex-wrap: wrap;
}

.t-name {
  font-size: 12.5px;
  font-weight: 800;
  color: #0f172a;
}

.role-code-chip {
  font-size: 10px;
  font-family: monospace;
  font-weight: 700;
  padding: 0 5px;
  border-radius: 4px;
}

.chip-dba {
  background: #fee2e2;
  color: #b91c1c;
}

.chip-lead {
  background: #dbeafe;
  color: #1d4ed8;
}

.t-role {
  font-size: 11px;
  color: #64748b;
}

/* SVG 动画流动虚线 */
.bpmn-flow-dash-anim {
  stroke-dasharray: 6 4;
  animation: bpmnFlowDash 1s linear infinite;
}

@keyframes bpmnFlowDash {
  to {
    stroke-dashoffset: -20;
  }
}

.number-highlight-pill {
  font-weight: 800;
  font-family: monospace;
  font-size: 12px;
  border-radius: 12px;
  padding: 0 8px;
  height: 20px;
  line-height: 20px;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.12);
}
</style>
