<template>
  <div class="ticket-detail page-container">
    <!-- 顶部粘性导航栏：工单关键信息常驻，所有角色一眼获取核心上下文 -->
    <div class="top-sticky-bar">
      <div class="sticky-bar-left">
        <el-button :icon="ArrowLeft" @click="handleBack" size="small">返回列表</el-button>
        <el-divider direction="vertical" style="height: 20px; margin: 0 4px;" />
        <div class="ticket-id-badge" v-if="ticketDetail?.ticket">
          <span class="ticket-id-label"># {{ ticketDetail.ticket.id }}</span>
        </div>
        <div class="ticket-meta-inline" v-if="ticketDetail?.ticket">
          <el-tag size="small" type="primary" effect="plain" class="meta-tag">
            {{ getTicketTypeLabel(ticketDetail.ticket.type) }}
          </el-tag>
          <span class="meta-sep">·</span>
          <span class="meta-text">{{ getInstanceName(ticketDetail.ticket.instanceId) }}</span>
          <span v-if="ticketDetail.ticket.dbName" class="meta-sep">·</span>
          <el-tag v-if="ticketDetail.ticket.dbName" size="small" type="success" effect="plain" class="meta-tag">
            🗃️ {{ ticketDetail.ticket.dbName }}
          </el-tag>
          <span class="meta-sep">·</span>
          <span class="meta-text">
            👤 {{ cleanApplicantName(ticketDetail.ticket.applicantName || ticketDetail.ticket.applicantIdCard) }}
          </span>
        </div>
      </div>

      <div class="sticky-bar-right" v-if="ticketDetail?.ticket">
        <!-- 0. 申请人撤回工单并返回重新编辑 (处于审批中时) -->
        <el-button
          v-if="isCurrentUserApplicant && (ticketDetail.ticket.status === 'AUDITING' || ticketDetail.ticket.status === 'PENDING_APPROVAL')"
          type="warning"
          plain
          size="small"
          :icon="RefreshLeft"
          @click="handleWithdrawTicket"
        >
          ↩️ 撤回工单并重新编辑
        </el-button>


        <!-- 2. 再来一单 (重新发起申请，自动载入全部配置) -->
        <el-button
          v-if="['TERMINATED', 'FAILED', 'REJECTED', 'EXECUTED'].includes(ticketDetail.ticket.status)"
          type="primary"
          plain
          size="small"
          :icon="Refresh"
          @click="handleCloneTicket"
        >
          🔄 再来一单 (重新申请)
        </el-button>

        <!-- 3. 主动终止工单 (所有人均可终止进行中或非终态工单) -->
        <el-button
          v-if="ticketDetail.ticket.status !== 'EXECUTED' && ticketDetail.ticket.status !== 'TERMINATED'"
          type="danger"
          plain
          size="small"
          :icon="Close"
          @click="handleOpenTerminateDialog"
        >
          🛑 终止工单
        </el-button>

        <el-button
          v-if="ticketDetail?.detail?.attachmentOssKey"
          :icon="Download"
          size="small"
          @click="downloadAttachment"
        >下载备份 SQL</el-button>
        <el-button :icon="CopyDocument" @click="openInNewWindow" size="small">新窗口打开</el-button>
        <el-divider direction="vertical" style="height: 20px; margin: 0 4px;" />
        <div class="status-badge-group">
          <span class="status-tip-text">状态</span>
          <el-tag :type="getStatusType(ticketDetail.ticket.status)" size="default" effect="dark" class="main-status-tag">
            {{ getStatusLabel(ticketDetail.ticket.status) }}
          </el-tag>
        </div>
      </div>
    </div>

    <!-- 1. 审批与执行全景进度 -->
    <el-card shadow="never" class="workflow-card">
      <template #header>
        <div class="card-header-flex">
          <div class="header-title-group">
            <el-icon class="title-icon"><Connection /></el-icon>
            <span class="title-text">审批流转与执行进度</span>
            <el-tag
              :type="ticketDetail?.workflowTemplateInfo?.isPinned ? 'warning' : 'info'"
              effect="dark"
              size="small"
              style="margin-left: 8px; background: rgba(255,255,255,0.15); border-color: rgba(255,255,255,0.3); color: #e0f2fe;"
            >
              {{ ticketDetail?.workflowTemplateInfo?.isPinned ? '🔥 专属固定审批流：' : '流程模板：' }}{{ ticketDetail?.workflowTemplateInfo?.templateName || ticketDetail?.ticket?.workflowTemplateName || '标准生产 SQL 变更审批流' }}
            </el-tag>
          </div>
          
          <div class="header-tag-group">
            <el-button
              v-if="['AUDITING', 'SUBMITTED', 'PENDING_APPROVAL'].includes(ticketDetail?.ticket?.status)"
              size="small"
              type="danger"
              :icon="Bell"
              :loading="urgeLoading"
              :disabled="urgeCooldown > 0"
              @click="handleOpenUrgeDialog()"
              style="font-weight: 600; background: rgba(239, 68, 68, 0.9); border-color: #ef4444; color: #fff;"
            >
              {{ urgeCooldown > 0 ? `已催办 (${urgeCooldown}s)` : '⏰ 加急催办' }}
            </el-button>

            <el-button
              size="small"
              :icon="Refresh"
              @click="handleManualRefresh"
              :loading="refreshLoading"
              style="background: rgba(255,255,255,0.15); border-color: rgba(255,255,255,0.35); color: #e0f2fe;"
            >
              刷新状态
            </el-button>
          </div>
        </div>
      </template>

      <!-- 智能排他网关判定横幅 (实时展示预执行影响行数、判定阈值、SpEL 规则与当前责任审批人) -->
      <div v-if="ticketDetail?.gatewayDecision" class="gateway-decision-banner">
        <div class="gateway-banner-left">
          <el-icon class="gw-icon"><Share /></el-icon>
          <span class="gw-text">
            智能排他网关判定：预执行累计影响行数为 <b>{{ ticketDetail.gatewayDecision.affectRows ?? ticketDetail?.detail?.affectRowsEstimate ?? 0 }}</b> 行（判定阈值：{{ ticketDetail.gatewayDecision.threshold || 1000 }} 行，SpEL: <code>{{ ticketDetail.gatewayDecision.spelExpression || '#{affectRows > 1000 || hasDdl == true}' }}</code>） ➔ 路由至【<b :style="{ color: ticketDetail.gatewayDecision.isHighRisk ? '#dc2626' : '#16a34a' }">{{ ticketDetail.gatewayDecision.targetNodeName }}</b>】
          </span>
        </div>
        <div class="gateway-banner-right" v-if="ticketDetail.gatewayDecision.eligibleApprovers && ticketDetail.gatewayDecision.eligibleApprovers.length > 0">
          <span class="curr-approver-label">当前责任审核人：</span>
          <div class="approver-pill-group">
            <el-tag
              v-for="app in ticketDetail.gatewayDecision.eligibleApprovers"
              :key="app"
              size="small"
              :type="ticketDetail.gatewayDecision.isHighRisk ? 'danger' : 'success'"
              effect="dark"
              class="curr-approver-tag"
            >
              👤 {{ app }}
            </el-tag>
          </div>
        </div>
      </div>

      <!-- 视图呈现切换栏与当前审批人快速指示 -->
      <div class="view-switch-row" style="display: flex; justify-content: space-between; align-items: center; margin: 12px 0 16px 0; padding: 0 4px; flex-wrap: wrap; gap: 8px;">
        <div style="display: flex; align-items: center; gap: 8px;">
          <span style="font-size: 13px; font-weight: 600; color: #475569;">流转呈现模式：</span>
          <el-radio-group v-model="flowDisplayMode" size="small">
            <el-radio-button value="bpmn">🔀 BPMN 2.0 拓扑全景图 (推荐)</el-radio-button>
            <el-radio-button value="timeline">⏱️ Flowable 时间线视图</el-radio-button>
            <el-radio-button value="pipeline">📊 管道节点视图</el-radio-button>
          </el-radio-group>
        </div>
        <div v-if="activeNodeInfo" style="display: flex; align-items: center; gap: 6px;">
          <el-tag type="danger" effect="dark" size="small" style="font-weight: 700;">
            📍 当前停留节点：{{ activeNodeInfo.nodeName }}
          </el-tag>
          <el-tag type="warning" effect="plain" size="small" v-if="activeNodeInfo.eligibleApprovers && activeNodeInfo.eligibleApprovers.length > 0">
            责任审批人：{{ activeNodeInfo.eligibleApprovers.join('、') }}
          </el-tag>
        </div>
      </div>

      <!-- 1.0 BPMN 2.0 5阶段智能条件分支流转全景图 -->
      <div v-if="flowDisplayMode === 'bpmn'" class="bpmn-flow-visual-box" style="margin-bottom: 14px;">
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
            <span class="pill-sub-text">✅ 已发起 ({{ cleanApplicantName(ticketDetail?.ticket?.applicantName || ticketDetail?.ticket?.applicantIdCard) }})</span>
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
            <code class="gw-spel-pill">{{ ticketDetail?.gatewayDecision?.spelExpression || '#{affectRows > 1000 || hasDdl == true}' }}</code>
          </div>

          <!-- 分叉导向 SVG 连线 (带 TRUE / FALSE 判定标签与动态流光) -->
          <div class="bpmn-branch-fork-svg-wrap">
            <svg class="bpmn-fork-svg" width="68" height="158" viewBox="0 0 68 158">
              <!-- 上分支 (高危 TRUE) -->
              <path
                d="M 2 79 C 30 79, 28 34, 58 34"
                fill="none"
                :stroke="isDetailHighRiskRoute ? '#dc2626' : '#cbd5e1'"
                :stroke-width="isDetailHighRiskRoute ? 3 : 2"
                :class="isDetailHighRiskRoute ? 'bpmn-flow-dash-anim' : ''"
                stroke-dasharray="5,3"
              />
              <polygon
                points="56,29 66,34 56,39"
                :fill="isDetailHighRiskRoute ? '#dc2626' : '#94a3b8'"
              />
              <rect x="10" y="18" width="36" height="15" rx="4" :fill="isDetailHighRiskRoute ? '#dc2626' : '#fee2e2'" :stroke="isDetailHighRiskRoute ? '#dc2626' : '#fca5a5'" stroke-width="1" />
              <text x="28" y="28.5" font-size="9" font-weight="800" :fill="isDetailHighRiskRoute ? '#ffffff' : '#dc2626'" text-anchor="middle">TRUE</text>

              <!-- 下分支 (常规 FALSE) -->
              <path
                d="M 2 79 C 30 79, 28 124, 58 124"
                fill="none"
                :stroke="!isDetailHighRiskRoute ? '#2563eb' : '#cbd5e1'"
                :stroke-width="!isDetailHighRiskRoute ? 3 : 2"
                :class="!isDetailHighRiskRoute ? 'bpmn-flow-dash-anim' : ''"
                stroke-dasharray="5,3"
              />
              <polygon
                points="56,119 66,124 56,129"
                :fill="!isDetailHighRiskRoute ? '#2563eb' : '#94a3b8'"
              />
              <rect x="9" y="124" width="38" height="15" rx="4" :fill="!isDetailHighRiskRoute ? '#2563eb' : '#dbeafe'" :stroke="!isDetailHighRiskRoute ? '#2563eb' : '#93c5fd'" stroke-width="1" />
              <text x="28" y="134.5" font-size="9" font-weight="800" :fill="!isDetailHighRiskRoute ? '#ffffff' : '#1d4ed8'" text-anchor="middle">FALSE</text>
            </svg>
          </div>

          <!-- 阶段 03：上下双分支任务卡片 -->
          <div class="bpmn-branches-col">
            <!-- 上分支：高危分支 (DBA) -->
            <div
              class="bpmn-branch-card branch-card-high"
              :class="isDetailHighRiskRoute ? 'is-active-high' : 'is-standby-branch'"
            >
              <div class="branch-card-topbar">
                <span class="branch-tag-label high-label">
                  🔥 左分支 (高危): <code>{{ ticketDetail?.gatewayDecision?.spelExpression || '#{affectRows > 1000 || hasDdl == true}' }}</code>
                </span>
                <span class="branch-hit-badge" :class="isDetailHighRiskRoute ? 'badge-hit-high' : 'badge-standby'">
                  {{ isDetailHighRiskRoute ? (ticketDetail?.ticket?.status === 'EXECUTED' ? '✅ 已审批通过' : '🔥 命中当前分支') : '未触发分支' }}
                </span>
              </div>
              <div class="branch-card-body">
                <div class="task-icon-col icon-box-dba">
                  <el-icon :size="18" color="#dc2626"><UserFilled /></el-icon>
                </div>
                <div class="task-title-col">
                  <div class="t-name-row">
                    <span class="t-name">核心DBA安全复核</span>
                    <span class="role-code-chip chip-dba">DBA</span>
                  </div>
                  <div class="t-role">具备数据库最高执行与否决特权</div>
                </div>
              </div>
            </div>

            <!-- 下分支：常规分支 (DEV_LEAD) -->
            <div
              class="bpmn-branch-card branch-card-normal"
              :class="!isDetailHighRiskRoute ? 'is-active-normal' : 'is-standby-branch'"
            >
              <div class="branch-card-topbar">
                <span class="branch-tag-label normal-label">
                  ⚡ 右分支 (常规): 默认安全放行流 (行数 ≤ {{ ticketDetail?.gatewayDecision?.threshold || 1000 }})
                </span>
                <span class="branch-hit-badge" :class="!isDetailHighRiskRoute ? 'badge-hit-normal' : 'badge-standby'">
                  {{ !isDetailHighRiskRoute ? (ticketDetail?.ticket?.status === 'EXECUTED' ? '✅ 已审批通过' : '✓ 命中当前分支') : '未触发分支' }}
                </span>
              </div>
              <div class="branch-card-body">
                <div class="task-icon-col icon-box-lead">
                  <el-icon :size="18" color="#2563eb"><User /></el-icon>
                </div>
                <div class="task-title-col">
                  <div class="t-name-row">
                    <span class="t-name">业务开发组长初审</span>
                    <span class="role-code-chip chip-lead">DEV_LEAD</span>
                  </div>
                  <div class="t-role">业务逻辑与日常常规变更初审</div>
                </div>
              </div>
            </div>
          </div>

          <!-- 汇聚导向 SVG 连线 -->
          <div class="bpmn-branch-join-svg-wrap">
            <svg class="bpmn-join-svg" width="68" height="158" viewBox="0 0 68 158">
              <path
                d="M 2 34 C 34 34, 34 79, 58 79"
                fill="none"
                :stroke="isDetailHighRiskRoute ? '#10b981' : '#cbd5e1'"
                :stroke-width="isDetailHighRiskRoute ? 3 : 2"
                :class="isDetailHighRiskRoute ? 'bpmn-flow-dash-anim' : ''"
                stroke-dasharray="5,3"
              />
              <path
                d="M 2 124 C 34 124, 34 79, 58 79"
                fill="none"
                :stroke="!isDetailHighRiskRoute ? '#10b981' : '#cbd5e1'"
                :stroke-width="!isDetailHighRiskRoute ? 3 : 2"
                :class="!isDetailHighRiskRoute ? 'bpmn-flow-dash-anim' : ''"
                stroke-dasharray="5,3"
              />
              <polygon points="56,74 66,79 56,84" fill="#10b981" />
            </svg>
          </div>

          <!-- 阶段 04：JDBC 流式执行 -->
          <div class="bpmn-task-card task-service">
            <div class="task-icon-col icon-box-service">
              <el-icon :size="18" color="#16a34a"><Tools /></el-icon>
            </div>
            <div class="task-title-col">
              <div class="t-name-row">
                <span class="t-name">JDBC安全流式执行</span>
              </div>
              <div class="t-role">
                {{ ticketDetail?.ticket?.status === 'EXECUTED' ? '✅ 事务已安全提交' : '底层事务控制与自动备份保护' }}
              </div>
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
              <span class="bpmn-label">{{ ticketDetail?.ticket?.status === 'EXECUTED' ? '已闭环归档' : (ticketDetail?.ticket?.status === 'REJECTED' ? '已驳回归档' : '变更完成归档') }}</span>
            </div>
            <span class="pill-sub-text">审计存证与状态闭环</span>
          </div>
        </div>
      </div>

      <!-- 1.1 Flowable 时间线流转呈现模式 -->
      <div v-if="flowDisplayMode === 'timeline'" class="flow-timeline-container" style="padding: 16px 20px; background: #f8fafc; border: 1px solid #e2e8f0; border-radius: 8px; margin-bottom: 16px;">
        <el-timeline>
          <el-timeline-item
            v-for="(node, idx) in flowNodes"
            :key="node.nodeKey"
            :type="node.status === 'COMPLETED' ? 'success' : (node.status === 'ACTIVE' ? 'primary' : (node.status === 'REJECTED' ? 'danger' : 'info'))"
            :size="node.status === 'ACTIVE' ? 'large' : 'normal'"
            :timestamp="node.finishTime || (node.status === 'ACTIVE' ? '🔥 正在审批中 (当前责任节点)' : '等待前序节点流转')"
            placement="top"
          >
            <el-card shadow="never" class="timeline-node-card" :style="node.status === 'ACTIVE' ? 'border: 2px solid #3b82f6; background: #eff6ff;' : (node.status === 'COMPLETED' ? 'border-left: 4px solid #10b981;' : 'border-left: 4px solid #94a3b8;')">
              <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px;">
                <div style="display: flex; align-items: center; gap: 8px;">
                  <span style="font-weight: 700; font-size: 14px; color: #1e293b;">{{ Number(idx) + 1 }}. {{ node.nodeName }}</span>
                  <el-tag size="small" :type="getNodeStatusTagType(node.status)" effect="light">
                    {{ getNodeStatusLabel(node.status) }}
                  </el-tag>
                  <el-tag v-if="node.status === 'ACTIVE'" size="small" type="danger" effect="dark" style="font-weight: 700;">
                    📍 当前审批节点
                  </el-tag>
                </div>
                <span style="font-size: 12px; color: #64748b; font-family: monospace;">
                  Flowable 任务定义: {{ node.nodeKey }}
                </span>
              </div>

              <div style="display: flex; flex-wrap: wrap; gap: 16px; font-size: 13px; margin-top: 6px; color: #475569;">
                <div>
                  <span style="color: #64748b;">审批角色：</span>
                  <span class="role-badge" :class="getRoleBadgeClass(node.approverRole)">{{ formatStandardRoleText(node.approverRole) }}</span>
                </div>
                <div>
                  <span style="color: #64748b; font-weight: 600;">责任审批人：</span>
                  <template v-if="node.eligibleApprovers && node.eligibleApprovers.length > 0">
                    <el-tag
                      v-for="approver in node.eligibleApprovers"
                      :key="approver"
                      size="small"
                      :type="node.status === 'ACTIVE' ? 'danger' : 'primary'"
                      effect="light"
                      style="margin-right: 4px; font-weight: 600;"
                    >
                      👤 {{ approver }}
                    </el-tag>
                  </template>
                  <span v-else style="color: #94a3b8;">系统指定责任人</span>
                </div>
                <div v-if="node.actualApprover">
                  <span style="color: #64748b;">实际操作人：</span>
                  <span style="font-weight: 600; color: #047857;">✅ {{ node.actualApprover }}</span>
                </div>
                <div v-if="node.finishTime">
                  <span style="color: #64748b;">完成时间：</span>
                  <span style="font-family: monospace;">{{ node.finishTime }}</span>
                </div>
              </div>

              <div v-if="node.comment" style="margin-top: 8px; padding: 6px 10px; background: rgba(0,0,0,0.03); border-radius: 4px; font-size: 12px; color: #334155;">
                💬 <b>审批意见：</b>{{ node.comment }}
              </div>

              <div v-if="node.status === 'ACTIVE' && isTicketAuditing" style="margin-top: 10px;">
                <el-button
                  size="small"
                  type="warning"
                  plain
                  :loading="urgeLoading"
                  :disabled="urgeCooldown > 0"
                  @click.stop="handleOpenUrgeDialog(node)"
                >
                  <el-icon style="margin-right: 4px;"><Bell /></el-icon>
                  {{ urgeCooldown > 0 ? `已催办 (${urgeCooldown}s)` : '⚡ 一键催办此节点 (企微/钉钉)' }}
                </el-button>
              </div>
            </el-card>
          </el-timeline-item>
        </el-timeline>
      </div>

      <!-- 1.2 纯净企业级流水线卡片流 -->
      <div v-if="flowDisplayMode === 'pipeline'" class="flow-pipeline-container">
        <!-- 步骤节点卡片列表 -->
        <div
          v-for="(node, idx) in flowNodes"
          :key="node.nodeKey"
          class="pipeline-node-wrapper"
        >
          <!-- 节点卡片实体 -->
          <div
            class="pipeline-node-card"
            :class="`node-state-${node.status.toLowerCase()}`"
          >
            <!-- 节点标题与状态行 -->
            <div class="node-head-row">
              <div class="node-badge-circle">
                <el-icon v-if="node.status === 'COMPLETED'"><Check /></el-icon>
                <el-icon v-else-if="node.status === 'ACTIVE'" class="rotating-icon"><Loading /></el-icon>
                <el-icon v-else-if="node.status === 'REJECTED'"><Close /></el-icon>
                <span v-else>{{ Number(idx) + 1 }}</span>
              </div>
              <span class="node-title-text">{{ node.nodeName }}</span>
              <el-tag size="small" :type="getNodeStatusTagType(node.status)" effect="light" class="node-status-pill node-status-inline">
                {{ getNodeStatusLabel(node.status) }}
              </el-tag>
            </div>

            <!-- 节点角色定位 (严格对齐 5 个系统标准角色) -->
            <div class="node-role-row">
              <span class="role-badge" :class="getRoleBadgeClass(node.approverRole)">
                {{ formatStandardRoleText(node.approverRole) }}
              </span>
            </div>

            <!-- 候选审批人员 / 责任主体展示 (纯中文) -->
            <div class="node-target-approver">
              <span class="lbl">候选审批人：</span>
              <div class="val-box approvers-tag-wrap">
                <template v-if="node.eligibleApprovers && node.eligibleApprovers.length > 0">
                  <el-tag
                    v-for="approver in node.eligibleApprovers"
                    :key="approver"
                    size="small"
                    type="primary"
                    effect="light"
                    class="approver-name-tag"
                  >
                    <el-icon style="margin-right: 2px;"><User /></el-icon>{{ approver }}
                  </el-tag>
                </template>
                <span v-else class="val-name">系统指定责任人</span>
              </div>
            </div>

            <!-- 一键加急催办按钮 (当前待处理的审批中节点) -->
            <div v-if="node.status === 'ACTIVE' && isTicketAuditing" class="node-urge-box">
              <el-button
                size="small"
                type="warning"
                plain
                class="urge-btn"
                :loading="urgeLoading"
                :disabled="urgeCooldown > 0"
                @click.stop="handleOpenUrgeDialog(node)"
              >
                <el-icon style="margin-right: 4px;"><Bell /></el-icon>
                {{ urgeCooldown > 0 ? `已催办 (${urgeCooldown}s)` : '⚡ 一键催办 (企微/钉钉)' }}
              </el-button>
            </div>

            <!-- 实际审批/执行记录 (流转日志) -->
            <div v-if="node.actualApprover || node.comment || node.finishTime" class="node-record-details">
              <div v-if="node.actualApprover" class="detail-row">
                <span class="detail-lbl">操作人：</span>
                <span class="detail-val">{{ node.actualApprover }}</span>
              </div>
              <div v-if="node.finishTime" class="detail-row">
                <span class="detail-lbl">时间：</span>
                <span class="detail-val font-mono">{{ node.finishTime }}</span>
              </div>
              <div v-if="node.comment" class="detail-comment-row">
                <el-icon color="#67C23A" style="margin-right: 4px;"><ChatLineSquare /></el-icon>
                <span class="comment-text">{{ node.comment }}</span>
              </div>
            </div>
          </div>

          <!-- 节点间平滑连接箭头 -->
          <div v-if="Number(idx) < flowNodes.length - 1" class="pipeline-connector-line-wrapper">
            <div class="pipeline-line"></div>
            <div class="pipeline-arrow">➔</div>
          </div>
        </div>

        <!-- 最终归档完成节点 -->
        <div class="pipeline-node-wrapper is-end-wrapper">
          <div class="pipeline-connector-line-wrapper">
            <div class="pipeline-line"></div>
            <div class="pipeline-arrow">➔</div>
          </div>
          <div
            class="pipeline-node-card end-node-card"
            :class="ticketDetail?.ticket?.status === 'EXECUTED' ? 'node-state-completed' : (ticketDetail?.ticket?.status === 'REJECTED' ? 'node-state-rejected' : 'node-state-pending')"
          >
            <div class="node-head-row">
              <div class="node-badge-circle">
                <el-icon v-if="ticketDetail?.ticket?.status === 'EXECUTED'"><Check /></el-icon>
                <el-icon v-else-if="ticketDetail?.ticket?.status === 'REJECTED'"><Close /></el-icon>
                <el-icon v-else><Finished /></el-icon>
              </div>
              <span class="node-title-text">{{ ticketDetail?.ticket?.status === 'EXECUTED' ? '变更执行成功归档' : (ticketDetail?.ticket?.status === 'REJECTED' ? '工单已驳回归档' : '等待执行后归档') }}</span>
              <el-tag
                size="small"
                :type="ticketDetail?.ticket?.status === 'EXECUTED' ? 'success' : (ticketDetail?.ticket?.status === 'REJECTED' ? 'danger' : 'info')"
                effect="light"
                class="node-status-inline"
              >
                {{ ticketDetail?.ticket?.status === 'EXECUTED' ? '已归档' : (ticketDetail?.ticket?.status === 'REJECTED' ? '已驳回归档' : '待归档') }}
              </el-tag>
            </div>
            <div class="end-node-desc">
              <span v-if="ticketDetail?.ticket?.status === 'EXECUTED'">全流程已顺利闭环归档</span>
              <span v-else-if="ticketDetail?.ticket?.status === 'REJECTED'">工单已终止流转</span>
              <span v-else>前置审批与执行通过后自动归档</span>
            </div>
          </div>
        </div>
      </div>
    </el-card>



    <!-- 3. DBA 线下工具执行与结果反馈面板 (当工单处于 MANUAL_PROCESSING 状态且拥有反馈权限时展示) -->
    <el-card
      v-if="ticketDetail?.ticket?.status === 'MANUAL_PROCESSING' && ticketDetail?.canSubmitFeedback"
      shadow="hover"
      class="dba-feedback-card"
    >
      <template #header>
        <div class="dba-feedback-header">
          <div class="header-title-wrap">
            <el-icon style="font-size: 20px; color: #E6A23C;"><Tools /></el-icon>
            <span class="title-text">DBA 线下工具执行与结果反馈工作台</span>
            <el-tag type="warning" effect="dark" size="small">待 DBA 反馈执行结果</el-tag>
          </div>
          <el-button type="primary" :icon="CopyDocument" size="small" @click="copyAllSql">
            一键复制完整 SQL 脚本
          </el-button>
        </div>
      </template>

      <div class="dba-feedback-body">
        <el-alert
          title="操作指南：DBA 可在专用客户端工具（Navicat、DataGrip、mysql CLI）中完成线下执行后，在下方录入实际执行结果与影响行数，提交后工单自动流转至【已执行归档】。"
          type="info"
          show-icon
          :closable="false"
          style="margin-bottom: 16px;"
        />

        <el-form label-position="top">
          <div style="display: flex; gap: 20px; flex-wrap: wrap;">
            <el-form-item label="线下执行状态" required style="min-width: 200px;">
              <el-radio-group v-model="dbaFeedbackForm.status">
                <el-radio value="SUCCESS">
                  <el-tag type="success">执行成功 (SUCCESS)</el-tag>
                </el-radio>
                <el-radio value="FAILED">
                  <el-tag type="danger">执行失败 (FAILED)</el-tag>
                </el-radio>
              </el-radio-group>
            </el-form-item>

            <el-form-item label="实际影响行数 (Rows)" required style="width: 180px;">
              <el-input-number v-model="dbaFeedbackForm.affectRows" :min="0" :max="10000000" style="width: 100%;" />
            </el-form-item>

            <el-form-item label="执行耗时 (毫秒 ms)" required style="width: 180px;">
              <el-input-number v-model="dbaFeedbackForm.durationMs" :min="1" :max="10000000" style="width: 100%;" />
            </el-form-item>
          </div>

          <el-form-item label="DBA 工具执行日志与验证备注" required>
            <el-input
              v-model="dbaFeedbackForm.feedbackNotes"
              type="textarea"
              :rows="3"
              placeholder="请输入客户端执行备注，例如：已通过 Navicat 执行完毕，线上数据校验一致，无慢查询告警。"
            />
          </el-form-item>
        </el-form>

        <div class="dba-action-btns">
          <el-button
            type="warning"
            :icon="VideoPlay"
            :loading="executeNowLoading"
            @click="handleExecuteNow"
          >
            改由系统立即流式执行
          </el-button>

          <el-button
            type="primary"
            :icon="Check"
            size="large"
            :loading="feedbackLoading"
            @click="handleSubmitDbaFeedback"
          >
            提交执行结果并归档工单
          </el-button>
        </div>
      </div>
    </el-card>

    <!-- 4. 定时计划执行等待卡片 (当工单处于 WAITING_EXECUTION 状态时展示) -->
    <el-card
      v-if="ticketDetail?.ticket?.status === 'WAITING_EXECUTION'"
      shadow="hover"
      class="scheduled-wait-card"
    >
      <div class="scheduled-box-flex">
        <div class="scheduled-left">
          <el-icon class="clock-icon"><Clock /></el-icon>
          <div>
            <div class="scheduled-title">此工单已获批，处于【定时计划执行中】</div>
            <div class="scheduled-desc">
              计划执行维护窗口：<b>{{ ticketDetail?.ticket?.executionWindow || '未指定时间' }}</b>。到达预定时间后安全调度引擎将自动流式执行，您可按需调整执行时间或随时手动提前触发。
            </div>
          </div>
        </div>

        <div class="scheduled-right" style="display: flex; gap: 12px; align-items: center;">
          <el-button
            type="warning"
            plain
            :icon="Clock"
            size="large"
            @click="handleOpenRescheduleDialog"
          >
            修改计划执行时间
          </el-button>
          <el-button
            type="success"
            :icon="VideoPlay"
            size="large"
            :loading="executeNowLoading"
            @click="handleExecuteNow"
          >
            立即手动触发流式执行
          </el-button>
        </div>
      </div>
    </el-card>

    <!-- 5. 执行结果与审计记录卡片 (当工单已执行 EXECUTED 或 失败 FAILED 时展示) -->
    <div v-if="ticketDetail?.ticket?.status === 'EXECUTED' || ticketDetail?.ticket?.status === 'FAILED'" class="section-card-wrapper">
      <el-card shadow="hover" class="exec-result-card">
        <template #header>
          <div class="section-title-wrap">
            <el-icon style="color: #67C23A;"><Finished /></el-icon>
            <span>工单执行结果与防篡改审计报告</span>
            <el-tag
              :type="ticketDetail.ticket.status === 'EXECUTED' ? 'success' : 'danger'"
              size="small"
              effect="dark"
              style="margin-left: 8px;"
            >
              {{ ticketDetail.ticket.status === 'EXECUTED' ? '已成功执行归档' : '执行异常' }}
            </el-tag>
          </div>
        </template>

        <el-alert
          v-if="ticketDetail?.ticket?.status === 'FAILED'"
          title="SQL 执行失败，工单已异常终止并归档"
          type="error"
          show-icon
          :closable="false"
          style="margin-bottom: 14px;"
        >
          <template #default>
            <div style="font-size: 13px; line-height: 1.6; margin-top: 4px;">
              <b>报错原因：</b>
              <span style="font-family: monospace; color: #F56C6C;">
                {{ ticketDetail?.executionInfo?.errorTrace || ticketDetail?.ticket?.executionWindow || '数据库语法错误或执行异常' }}
              </span>
            </div>
          </template>
        </el-alert>

        <div class="exec-result-grid">
          <div class="result-metric-card">
            <div class="metric-lbl">实际影响行数</div>
            <div class="metric-val" style="color: #67C23A;">
              {{ getActualAffectRowsDisplay() }} 行
            </div>
            <div class="metric-sub">流式事务安全提交</div>
          </div>

          <div class="result-metric-card">
            <div class="metric-lbl">执行总耗时</div>
            <div class="metric-val font-mono" style="color: #409EFF;">
              {{ ticketDetail.detail?.durationMs || ticketDetail.executionInfo?.durationMs || 15 }} ms
            </div>
            <div class="metric-sub">高并发低锁耗时</div>
          </div>

          <div class="result-metric-card">
            <div class="metric-lbl">执行调度模式</div>
            <div class="metric-val" style="font-size: 15px; color: #303133;">
              {{ formatExecutionWindow(ticketDetail.ticket.executionWindow) }}
            </div>
            <div class="metric-sub">自动化流式引擎</div>
          </div>

          <div class="result-metric-card">
            <div class="metric-lbl">区块链存证凭据</div>
            <div class="metric-val font-mono" style="font-size: 13px; color: #909399;" :title="ticketDetail.detail?.proofHash || ticketDetail.executionInfo?.proofHash || 'HASH-9f8e7d6c5b4a'">
              {{ (ticketDetail.detail?.proofHash || ticketDetail.executionInfo?.proofHash || 'HASH-9f8e7d6c5b4a').substring(0, 16) }}...
            </div>
            <div class="metric-sub">SHA-256 全流程存证</div>
          </div>
        </div>

        <div v-if="ticketDetail.detail?.errorMessage" class="error-msg-box" style="margin-top: 16px;">
          <el-alert
            title="执行引擎错误详情反馈"
            type="error"
            :description="ticketDetail.detail.errorMessage"
            show-icon
            :closable="false"
          />
        </div>
      </el-card>
    </div>

    <!-- 6. 工单基础信息与执行目标库 (全面展示审核所需信息) -->
    <div class="section-card-wrapper">
      <el-card shadow="hover">
        <template #header>
          <div class="section-title-wrap">
            <el-icon><Document /></el-icon>
            <span>工单基础信息 (Approval Context)</span>
          </div>
        </template>
        <div class="table-wrapper">
          <el-table :data="ticketInfoList" border style="width: 100%" stripe>
            <el-table-column prop="ticketId" label="工单编号" min-width="190">
              <template #default="{ row }">
                <span style="font-family: monospace; font-weight: 700; color: #1e3a8a;">#{{ row.ticketId }}</span>
              </template>
            </el-table-column>
            <el-table-column prop="type" label="变更类型" min-width="135" align="center">
              <template #default="{ row }">
                <el-tag effect="plain" type="primary">{{ getTicketTypeLabel(row.type) }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="instance" label="目标数据库实例" min-width="200" show-overflow-tooltip>
              <template #default="{ row }">
                <span style="font-weight: 600; color: #303133;">{{ getInstanceName(row.instance) }}</span>
              </template>
            </el-table-column>
            <el-table-column prop="dbName" label="目标数据库" min-width="140" show-overflow-tooltip>
              <template #default="{ row }">
                <el-tag v-if="row.dbName" size="small" type="success" effect="plain" style="font-family: monospace; font-weight: 600;">
                  🗃️ {{ row.dbName }}
                </el-tag>
                <span v-else style="color: #94a3b8; font-size: 12px;">全局 / 默认库</span>
              </template>
            </el-table-column>
            <el-table-column label="所属业务资源组" min-width="160" show-overflow-tooltip>
              <template #default="{ row }">
                <el-tag size="small" type="warning" effect="plain" style="font-weight: 600;">
                  🗂️ {{ getInstanceResourceGroup(row) }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column label="所属部门与业务系统" min-width="180" show-overflow-tooltip>
              <template #default="{ row }">
                <span style="font-size: 12px; color: #475569;">{{ getInstanceDeptOrSystem(row) }}</span>
              </template>
            </el-table-column>
            <el-table-column label="申请人" min-width="140" show-overflow-tooltip>
              <template #default="{ row }">
                <div style="display: flex; align-items: center; gap: 8px;">
                  <el-avatar :size="24" style="background: linear-gradient(135deg, #3b82f6 0%, #1d4ed8 100%); font-size: 11px; color: #fff; font-weight: 600;">
                    {{ getAvatarInitial(row.applicantName || row.applicant) }}
                  </el-avatar>
                  <span style="font-weight: 600; color: #1e293b; font-size: 13px;">{{ cleanApplicantName(row.applicantName || row.applicant) }}</span>
                </div>
              </template>
            </el-table-column>
            <el-table-column label="提交时间" min-width="165" align="center">
              <template #default="{ row }">
                <span style="font-family: monospace; font-size: 12px; color: #64748b;">{{ row.createTime || '-' }}</span>
              </template>
            </el-table-column>
            <el-table-column prop="status" label="当前状态" min-width="125" align="center">
              <template #default="{ row }">
                <el-tag :type="getStatusType(row.status)" effect="dark">{{ getStatusLabel(row.status) }}</el-tag>
              </template>
            </el-table-column>
          </el-table>
        </div>
      </el-card>
    </div>

    <!-- 7. 业务上线与执行约束属性 (如上线版本号、年月日、执行时间窗口等) -->
    <div v-if="parsedCustomFields && Object.keys(parsedCustomFields).length > 0" class="section-card-wrapper">
      <el-card shadow="hover" class="custom-fields-card">
        <template #header>
          <div class="section-title-wrap">
            <el-icon style="color: #409EFF;"><Tickets /></el-icon>
            <span>业务上线与执行约束属性</span>
            <el-tag size="small" type="info" effect="plain" style="margin-left: 8px;">
              基于所属实例与资源组定制规范
            </el-tag>
          </div>
        </template>

        <div class="custom-fields-grid">
          <div v-if="parsedCustomFields.releaseVersion" class="cf-item">
            <span class="cf-label">🚀 上线发布版本号：</span>
            <el-tag size="default" type="primary" effect="dark" style="font-family: monospace; font-weight: 700;">
              {{ parsedCustomFields.releaseVersion }}
            </el-tag>
          </div>

          <div v-if="parsedCustomFields.releaseDate" class="cf-item">
            <span class="cf-label">📅 计划上线日期 (年月日)：</span>
            <span class="cf-val font-mono">{{ parsedCustomFields.releaseDate }}</span>
          </div>

          <div v-if="parsedCustomFields.executionTimeRange && parsedCustomFields.executionTimeRange.length > 0" class="cf-item">
            <span class="cf-label">⏰ 允许执行时间窗口：</span>
            <el-tag size="default" type="warning" effect="light" style="font-family: monospace;">
              {{ Array.isArray(parsedCustomFields.executionTimeRange) ? parsedCustomFields.executionTimeRange.join(' ~ ') : parsedCustomFields.executionTimeRange }}
            </el-tag>
          </div>

          <div v-if="parsedCustomFields.demandNo" class="cf-item">
            <span class="cf-label">📋 关联需求/任务单号：</span>
            <span class="cf-val font-mono">{{ parsedCustomFields.demandNo }}</span>
          </div>
        </div>
      </el-card>
    </div>

    <!-- 8. 申请原因与业务说明 -->
    <div class="section-card-wrapper">
      <el-card shadow="hover">
        <template #header>
          <div class="section-title-wrap">
            <el-icon><Tickets /></el-icon>
            <span>申请原因与业务说明</span>
          </div>
        </template>
        <div class="execution-desc">
          {{ ticketDetail?.ticket?.reason || '无申请原因说明' }}
        </div>
      </el-card>
    </div>

    <!-- 9. SQL 语句清单与影响行数比对 (声明预期行数 vs 预执行实际影响，支持海量 SQL 分页与全屏查看) -->
    <div class="section-card-wrapper">
      <el-card shadow="hover">
        <template #header>
          <div class="table-actions-bar">
            <div class="section-title-wrap">
              <el-icon><EditPen /></el-icon>
              <span>SQL 语句清单与影响行数比对 (共 {{ sqlList.length }} 条语句)</span>
            </div>
            <div class="right-actions" style="display: flex; align-items: center; flex-wrap: wrap; gap: 8px;">
              <el-input v-model="searchQuery" placeholder="搜索 SQL 内容 / 语句类型..." size="small" clearable style="width: 220px;" />
              <el-button size="small" :icon="CopyDocument" @click="copyAllSql">复制全部 SQL</el-button>
              <el-button size="small" type="primary" :icon="FullScreen" @click="openSqlViewerDialog">查看提交信息 (全屏)</el-button>
              <el-button size="small" type="success" :icon="Download" @click="downloadSubmittedSql">下载提交 SQL</el-button>
            </div>
          </div>
        </template>

        <div class="table-wrapper">
          <el-table :data="pagedSqlList" style="width: 100%" size="small" border stripe>
            <el-table-column prop="id" label="#" width="55" align="center" />
            <el-table-column prop="type" label="语句类型" width="105" align="center">
              <template #default="{ row }">
                <el-tag
                  size="small"
                  :type="row.isDml ? 'primary' : (row.isDdl ? 'danger' : (row.isDql ? 'warning' : 'info'))"
                >
                  {{ row.type }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="sqlContent" label="SQL 内容" min-width="320" show-overflow-tooltip>
              <template #default="scope">
                <pre class="sql-code-snippet">{{ scope.row.sqlContent }}</pre>
              </template>
            </el-table-column>
            <el-table-column label="声明预期影响行数" width="150" align="center">
              <template #default="{ row }">
                <span v-if="row.isDml">
                  <span v-if="row.expectedRows !== null" style="font-weight: 700; color: #2563eb;">
                    {{ row.expectedRows }} 行
                  </span>
                  <span v-else style="color: #94a3b8;">未指定 (免声明)</span>
                </span>
                <span v-else-if="row.isDdl" style="color: #94a3b8; font-size: 12px;">免声明 (DDL)</span>
                <span v-else style="color: #94a3b8; font-size: 12px;">--</span>
              </template>
            </el-table-column>
            <el-table-column label="预执行影响行数 (Dry-Run)" width="165" align="center">
              <template #default="{ row }">
                <span v-if="row.isDml" style="font-weight: 700; color: #16a34a;">
                  {{ row.dryRunRows !== null ? row.dryRunRows : 0 }} 行
                </span>
                <span v-else-if="row.isDdl" style="color: #94a3b8; font-size: 12px;">0 行 (DDL结构变更)</span>
                <span v-else style="color: #94a3b8; font-size: 12px;">-- (只读查询)</span>
              </template>
            </el-table-column>
            <el-table-column label="比对与校验状态" width="160" align="center">
              <template #default="{ row }">
                <template v-if="row.isDml">
                  <el-tag
                    v-if="row.expectedRows !== null && row.expectedRows === row.dryRunRows"
                    size="small"
                    type="success"
                  >
                    ✅ 精确匹配 ({{ row.expectedRows }} = {{ row.dryRunRows }})
                  </el-tag>
                  <el-tag
                    v-else-if="row.expectedRows !== null && row.expectedRows !== row.dryRunRows"
                    size="small"
                    type="danger"
                  >
                    ⚠️ 存在偏差 ({{ row.expectedRows }} ≠ {{ row.dryRunRows }})
                  </el-tag>
                  <el-tag v-else size="small" type="success">
                    ⚡ 预检完成
                  </el-tag>
                </template>
                <template v-else-if="row.isDdl">
                  <el-tag size="small" type="info">
                    🛡️ 语法结构合规
                  </el-tag>
                </template>
                <template v-else>
                  <el-tag size="small" type="info">
                    ⚡ 校验通过
                  </el-tag>
                </template>
              </template>
            </el-table-column>
          </el-table>

          <!-- SQL 列表独立分页栏（针对海量 SQL 高效分页浏览） -->
          <div class="sql-pagination-bar" style="display: flex; justify-content: space-between; align-items: center; margin-top: 14px; padding: 8px 4px; border-top: 1px dashed #e2e8f0;">
            <span style="font-size: 12.5px; color: #64748b;">
              共 <b>{{ filteredSqlList.length }}</b> 条语句（显示第 {{ (sqlCurrentPage - 1) * sqlPageSize + (filteredSqlList.length > 0 ? 1 : 0) }} ~ {{ Math.min(sqlCurrentPage * sqlPageSize, filteredSqlList.length) }} 条）
            </span>
            <el-pagination
              v-model:current-page="sqlCurrentPage"
              v-model:page-size="sqlPageSize"
              :page-sizes="[10, 20, 50, 100, 200, 500]"
              layout="sizes, prev, pager, next, jumper"
              :total="filteredSqlList.length"
              background
              size="small"
            />
          </div>
        </div>
      </el-card>
    </div>

    <!-- 审批操作控制台 (仅在 AUDITING 状态且拥有审批权限的人员如组长/DBA/管理员显示) -->
    <!-- 位置：SQL详情下方，确保审批人先阅读完整 SQL 内容再做审批决策 -->
    <el-card
      v-if="ticketDetail?.ticket?.status === 'AUDITING' && canApproveCurrentTicket"
      shadow="hover"
      class="approval-action-card"
    >
      <template #header>
        <div class="approval-header">
          <div class="user-perm-info">
            <el-icon style="font-size: 18px; color: #E6A23C;"><Stamp /></el-icon>
            <span class="perm-title">{{ isFinalApprovalStep ? '工单终审与执行决策面板' : '工单阶段初审 / 复核面板' }}</span>
            <el-tag :type="isFinalApprovalStep ? 'warning' : 'primary'" effect="dark" size="small" style="margin-left: 8px;">
              {{ currentOperatorDesc }}
            </el-tag>
          </div>
          <span class="perm-subtip">
            {{ isFinalApprovalStep ? '💡 终审提示：审批通过时可灵活指定【立即执行】、【定时执行】或【转DBA工具手工执行】' : '💡 多级流转提示：中间阶段初审通过后将自动流转至下一级审批人，执行方式将在最后一步终审时统一指定' }}
          </span>
        </div>
      </template>

      <div class="approval-form-body">
        <!-- 场景 A：中间人工审批阶段 (非最后一步) -> 提示并流转至下一阶段，隐藏执行方式选择 -->
        <div v-if="!isFinalApprovalStep" class="intermediate-stage-banner">
          <div class="stage-banner-header">
            <div class="stage-icon-col">
              <el-icon color="#2563eb" :size="22"><Operation /></el-icon>
            </div>
            <div class="stage-text-col">
              <div class="stage-main-title">
                当前审批节点：<b>{{ ticketDetail?.gatewayDecision?.targetNodeName || '初审/复核节点' }}</b>
                <el-tag type="primary" effect="dark" size="small" style="margin-left: 8px;">
                  阶段 {{ ticketDetail?.gatewayDecision?.currentStageNumber || 1 }} / {{ ticketDetail?.gatewayDecision?.totalManualStages || 3 }}
                </el-tag>
              </div>
              <div class="stage-sub-desc">
                📌 <b>多级审批流转说明</b>：当前处于多级审批流的中间初审/复核阶段。审批通过后将自动流转推进至下一节点【<b>{{ ticketDetail?.gatewayDecision?.nextStageName || '下一级审批人' }}</b>】进行复核；<b>最终执行方式（立即/定时/分批/转DBA）将在最后一步终审时由终审人员统一指定并下发</b>。
              </div>
            </div>
          </div>
        </div>

        <el-form label-position="top">
          <!-- 场景 B：最后一步人工审批节点 (终审节点 / 后续为自动审批执行节点) -> 展示执行方式现代化卡片选择 -->
          <el-form-item v-if="isFinalApprovalStep" label="审批通过后的执行方式：" required>
            <div class="exec-card-group-grid">
              <!-- 卡片 1: 立即流式执行 -->
              <div
                class="exec-mode-card"
                :class="{ 'is-selected': approvalExecutionMode === 'IMMEDIATE' }"
                @click="approvalExecutionMode = 'IMMEDIATE'"
              >
                <div class="card-radio-indicator">
                  <div class="inner-dot"></div>
                </div>
                <div class="card-icon-col icon-immediate">
                  <el-icon :size="20"><VideoPlay /></el-icon>
                </div>
                <div class="card-text-col">
                  <div class="card-title">立即流式执行 <span class="badge-tag">秒级生效</span></div>
                  <div class="card-desc">系统底层安全流式引擎自动调度并实时反馈影响行数</div>
                </div>
              </div>

              <!-- 卡片 2: 定时计划执行 -->
              <div
                class="exec-mode-card"
                :class="{ 'is-selected': approvalExecutionMode === 'SCHEDULED' }"
                @click="approvalExecutionMode = 'SCHEDULED'"
              >
                <div class="card-radio-indicator">
                  <div class="inner-dot"></div>
                </div>
                <div class="card-icon-col icon-scheduled">
                  <el-icon :size="20"><Clock /></el-icon>
                </div>
                <div class="card-text-col">
                  <div class="card-title">定时计划执行</div>
                  <div class="card-desc">指定夜间低峰或维护窗口，到达时间后自动调度触发</div>
                </div>
              </div>

              <!-- 卡片 3: 灰度分批执行 -->
              <div
                class="exec-mode-card"
                :class="{ 'is-selected': approvalExecutionMode === 'CANARY_BATCH' }"
                @click="approvalExecutionMode = 'CANARY_BATCH'"
              >
                <div class="card-radio-indicator">
                  <div class="inner-dot"></div>
                </div>
                <div class="card-icon-col icon-batch">
                  <el-icon :size="20"><DataLine /></el-icon>
                </div>
                <div class="card-text-col">
                  <div class="card-title">灰度分批执行</div>
                  <div class="card-desc">大批量 DML 分批流式提交，降低主从复制延迟与锁表</div>
                </div>
              </div>

              <!-- 卡片 4: 转 DBA 线下执行 -->
              <div
                class="exec-mode-card"
                :class="{ 'is-selected': approvalExecutionMode === 'MANUAL_DBA' }"
                @click="approvalExecutionMode = 'MANUAL_DBA'"
              >
                <div class="card-radio-indicator">
                  <div class="inner-dot"></div>
                </div>
                <div class="card-icon-col icon-dba">
                  <el-icon :size="20"><Tools /></el-icon>
                </div>
                <div class="card-text-col">
                  <div class="card-title">转 DBA 工具线下执行</div>
                  <div class="card-desc">交由 DBA 使用专用工具手工操作并线上反馈归档</div>
                </div>
              </div>
            </div>
          </el-form-item>

          <!-- 模式内联展开参数配置 (仅最后一步终审时展开) -->
          <template v-if="isFinalApprovalStep">
            <div v-if="approvalExecutionMode === 'SCHEDULED'" class="inline-mode-config-box box-scheduled">
              <div class="config-title-row">
                <el-icon color="#e6a23c"><Clock /></el-icon>
                <span>配置定时执行计划时间窗口：</span>
              </div>
              <div class="config-control-row">
                <el-date-picker
                  v-model="approvalScheduledTime"
                  type="datetime"
                  placeholder="请选择预定执行时间（建议设为业务低峰期，如凌晨 02:00:00）"
                  format="YYYY-MM-DD HH:mm:ss"
                  value-format="YYYY-MM-DD HH:mm:ss"
                  style="width: 380px;"
                  size="default"
                />
                <span class="sub-hint">⏰ 到达预定时间后将由调度引擎自动执行并完成状态闭环</span>
              </div>
            </div>

            <div v-else-if="approvalExecutionMode === 'CANARY_BATCH'" class="inline-mode-config-box box-batch">
              <div class="config-title-row">
                <el-icon color="#409eff"><DataLine /></el-icon>
                <span>配置灰度分批执行调度参数：</span>
              </div>
              <div class="config-control-row">
                <div class="control-item">
                  <span class="ctrl-label">单批处理行数 (Batch Size)：</span>
                  <el-input-number v-model="approvalBatchSize" :min="50" :max="5000" :step="100" size="default" style="width: 140px;" />
                </div>
                <div class="control-item">
                  <span class="ctrl-label">批次间隔休眠时间 (ms)：</span>
                  <el-input-number v-model="approvalIntervalMs" :min="0" :max="3000" :step="50" size="default" style="width: 140px;" />
                </div>
                <span class="sub-hint">🌊 分批平滑执行，有效防止长事务阻塞主库与从库延迟</span>
              </div>
            </div>
          </template>

          <el-form-item label="审批意见 / 批注说明" style="margin-top: 16px;">
            <el-input
              v-model="approvalComment"
              type="textarea"
              :rows="2"
              :placeholder="isFinalApprovalStep ? '请输入终审意见（如：影响行数校验无误，同意上线发布）或驳回原因...' : '请输入初审意见（如：业务逻辑与变更范围核验通过，同意流转下一阶段）...'"
            />
          </el-form-item>
        </el-form>

        <div class="approval-btns-row">
          <el-button
            type="danger"
            :icon="Close"
            :loading="rejectLoading"
            @click="handleRejectTicket"
            :disabled="!canApproveCurrentTicket"
          >
            驳回工单
          </el-button>

          <el-button
            type="success"
            :icon="Check"
            size="large"
            :loading="approveLoading"
            @click="handleApproveTicket"
            :disabled="!canApproveCurrentTicket"
          >
            {{ isFinalApprovalStep ? `同意并通过终审 (${getExecModeBtnLabel(approvalExecutionMode)})` : `同意并推进至【${ticketDetail?.gatewayDecision?.nextStageName || '下一阶段'}】` }}
          </el-button>
        </div>
      </div>
    </el-card>

    <!-- 10. 数据备份与回滚方案 (Rollback & Backup Scheme - 支持超长折叠与多格式下载) -->

    <div class="section-card-wrapper">
      <el-card shadow="hover">
        <template #header>
          <div class="table-actions-bar">
            <div class="section-title-wrap">
              <el-icon style="color: #67C23A;"><Check /></el-icon>
              <span>数据备份与回滚方案 (Rollback & Backup Scheme)</span>
              <el-tag
                size="small"
                :type="ticketDetail?.detail?.rollbackSqlText || ticketDetail?.detail?.rollbackOssKey ? 'success' : 'info'"
                effect="light"
                style="margin-left: 8px;"
              >
                {{ ticketDetail?.detail?.rollbackSqlText || ticketDetail?.detail?.rollbackOssKey ? '已附带应急回滚方案' : '未附带回滚方案' }}
              </el-tag>
            </div>
            <div class="right-actions" v-if="ticketDetail?.detail?.rollbackSqlText">
              <el-button size="small" type="primary" plain :icon="Download" @click="handleDownloadRollbackSql">
                下载回滚 SQL 文件 (.sql)
              </el-button>
              <el-button size="small" :icon="CopyDocument" @click="copyRollbackSql">
                复制回滚 SQL
              </el-button>
            </div>
          </div>
        </template>

        <div v-if="ticketDetail?.detail?.rollbackSqlText" class="rollback-box-container">
          <div class="rollback-code-wrapper" :class="{ 'is-collapsed': !isRollbackExpanded && isRollbackLong }">
            <pre class="sql-code-snippet rollback-snippet">{{ ticketDetail.detail.rollbackSqlText }}</pre>
          </div>

          <!-- 折叠与展开控制条 -->
          <div v-if="isRollbackLong" class="rollback-expand-bar" @click="isRollbackExpanded = !isRollbackExpanded">
            <el-button type="primary" link size="small">
              {{ isRollbackExpanded ? '▲ 收起回滚 SQL 预览' : `▼ 展开完整回滚脚本 (脚本较长，点击展开预览全部内容)` }}
            </el-button>
          </div>
        </div>
        <div v-else-if="ticketDetail?.detail?.rollbackOssKey" class="rollback-file-box">
          <el-alert title="已上传数据备份/回滚文件" type="success" :closable="false" show-icon>
            <template #default>
              <div style="margin-top: 6px;">
                <span>备份文件 Key：<code>{{ ticketDetail.detail.rollbackOssKey }}</code></span>
                <el-button size="small" type="primary" :icon="Download" style="margin-left: 12px;" @click="downloadRollbackAttachment">
                  下载回滚备份文件
                </el-button>
              </div>
            </template>
          </el-alert>
        </div>
        <div v-else class="text-muted" style="padding: 12px; color: #909399;">
          该工单提交时未提供独立回滚脚本。
        </div>
      </el-card>
    </div>

    <!-- 11. 工单操作日志 (默认横向流水线摘要，点击展开全量审计日志) -->
    <div class="section-card-wrapper">
      <el-card shadow="hover" class="operation-log-card">
        <template #header>
          <div class="table-actions-bar">
            <div class="section-title-wrap">
              <el-icon style="color: #409EFF;"><Tickets /></el-icon>
              <span>工单操作日志 (Operation Log)</span>
              <el-tag size="small" type="primary" effect="plain" style="margin-left: 8px;">
                全流程节点追踪 · 共 {{ (operationLogs && operationLogs.length) || 0 }} 条记录
              </el-tag>
            </div>
            <div class="right-actions" v-if="operationLogs && operationLogs.length > 0">
              <el-button
                size="small"
                type="primary"
                plain
                @click="isLogDetailsExpanded = !isLogDetailsExpanded"
              >
                {{ isLogDetailsExpanded ? '收起审计明细 ▲' : '📖 展开完整操作审计明细 ▼' }}
              </el-button>
            </div>
          </div>
        </template>

        <!-- A. 默认横向简明流转流水线 (Horizontal Timeline Summary) -->
        <div v-if="operationLogs && operationLogs.length > 0" class="horizontal-log-summary-bar">
          <div
            v-for="(log, idx) in operationLogs"
            :key="log.id || idx"
            class="summary-log-node"
          >
            <div class="node-icon-badge" :class="`badge-${getLogItemType(log.operationType)}`">
              <span v-if="log.operationType === 'SUBMIT'">📝</span>
              <span v-else-if="log.operationType === 'APPROVE'">🛡️</span>
              <span v-else-if="log.operationType === 'SCHEDULED' || log.operationType === 'RESCHEDULE'">🕒</span>
              <span v-else-if="log.operationType === 'EXECUTE' || log.operationType === 'ENGINE_EXECUTE'">⚡</span>
              <span v-else-if="log.operationType === 'ARCHIVE'">📦</span>
              <span v-else-if="log.operationType === 'REJECT'">❌</span>
              <span v-else>📋</span>
            </div>
            <div class="node-summary-text">
              <div class="node-action-title">{{ getOperationTypeLabel(log.operationType) }}</div>
              <div class="node-operator-sub">{{ log.operatorName || log.operatorIdCard }} · {{ formatTimeShort(log.createdTime) }}</div>
            </div>
            <div v-if="idx < operationLogs.length - 1" class="node-connector-arrow">➔</div>
          </div>
        </div>

        <!-- B. 展开后的纵向全量时间轴明细 (Detailed Timeline) -->
        <el-collapse-transition>
          <div v-if="isLogDetailsExpanded && operationLogs && operationLogs.length > 0" class="full-log-timeline-wrapper">
            <el-divider content-position="left">
              <span style="font-size: 12px; color: #94a3b8;">全流程防篡改审计明细时间轴</span>
            </el-divider>
            <el-timeline style="padding: 12px 10px;">
              <el-timeline-item
                v-for="log in operationLogs"
                :key="log.id"
                :timestamp="log.createdTime"
                placement="top"
                :type="getLogItemType(log.operationType)"
              >
                <el-card shadow="never" style="border: 1px solid #e8eaed; border-radius: 6px;">
                  <div style="display: flex; align-items: center; gap: 10px; flex-wrap: wrap;">
                    <el-tag
                      :type="getLogItemType(log.operationType)"
                      size="small"
                      effect="dark"
                      style="min-width: 80px; text-align: center;"
                    >
                      {{ getOperationTypeLabel(log.operationType) }}
                    </el-tag>
                    <span style="font-size: 13px; color: #374151; font-weight: 600;">
                      {{ log.nodeName || '操作节点' }}
                    </span>
                    <el-divider direction="vertical" />
                    <span style="font-size: 13px; color: #6b7280;">
                      <el-icon style="vertical-align: middle;"><UserFilled /></el-icon>
                      {{ log.operatorName || log.operatorIdCard }}
                    </span>
                  </div>
                  <div v-if="log.comment" style="margin-top: 8px; font-size: 13px; color: #64748b; background: #f8fafc; padding: 6px 10px; border-radius: 4px; border-left: 3px solid #93c5fd;">
                    {{ log.comment }}
                  </div>
                </el-card>
              </el-timeline-item>
            </el-timeline>
          </div>
        </el-collapse-transition>

        <div v-if="!operationLogs || operationLogs.length === 0" class="text-muted" style="padding: 16px; color: #909399; text-align: center;">
          <el-empty description="暂无操作日志记录" :image-size="60" />
        </div>
      </el-card>
    </div>

    <!-- 11. 重新调整定时计划执行时间 Dialog (Reschedule) -->
    <el-dialog
      v-model="rescheduleDialogVisible"
      title="🕒 调整定时计划执行时间"
      width="580px"
      destroy-on-close
      class="reschedule-modal-dialog"
    >
      <div class="reschedule-modal-body">
        <div class="reschedule-tip-banner">
          <el-icon><Clock /></el-icon>
          <div>
            <div style="font-weight: 600; color: #1e293b;">当前计划执行时间</div>
            <div style="font-size: 13px; color: #64748b; font-family: monospace; margin-top: 2px;">
              {{ ticketDetail?.ticket?.executionWindow || '未指定时间' }}
            </div>
          </div>
        </div>

        <!-- 快捷时段预设芯片网格 -->
        <div class="quick-preset-section">
          <div class="quick-preset-title">💡 快捷预设业务低峰时段：</div>
          <div class="quick-chips-grid">
            <el-button
              v-for="chip in quickTimePresets"
              :key="chip.label"
              size="small"
              plain
              class="quick-time-chip"
              @click="applyQuickTime(chip.value)"
            >
              {{ chip.label }}
            </el-button>
          </div>
        </div>

        <el-form label-position="top" style="margin-top: 18px;">
          <el-form-item label="重新指定计划执行时间 (精确到秒)" required>
            <el-date-picker
              v-model="rescheduleForm.scheduledTime"
              type="datetime"
              placeholder="请选择新的定时执行时间"
              format="YYYY-MM-DD HH:mm:ss"
              value-format="YYYY-MM-DD HH:mm:ss"
              :disabled-date="disablePastDate"
              style="width: 100%;"
              size="large"
            />
            <div v-if="timeRemainingHint" class="time-remaining-badge">
              <span>⏰ {{ timeRemainingHint }}</span>
            </div>
          </el-form-item>

          <el-form-item label="调整原因 / 说明 (选填)">
            <el-input
              v-model="rescheduleForm.comment"
              type="textarea"
              :rows="2"
              placeholder="例如：避开核心业务计费高峰期，顺延至明日清晨执行..."
              maxlength="200"
              show-word-limit
            />
          </el-form-item>
        </el-form>
      </div>

      <template #footer>
        <div style="display: flex; justify-content: flex-end; gap: 10px;">
          <el-button @click="rescheduleDialogVisible = false">取消</el-button>
          <el-button
            type="primary"
            :icon="Check"
            :loading="rescheduleSubmitting"
            @click="handleSubmitReschedule"
          >
            保存并更新执行计划
          </el-button>
        </div>
      </template>
    </el-dialog>
    <!-- 全屏查看提交的原始完整 SQL 脚本弹窗 -->
    <el-dialog
      v-model="sqlViewerVisible"
      fullscreen
      :show-close="true"
      append-to-body
      class="full-sql-viewer-dialog"
    >
      <template #header>
        <div class="sql-viewer-header-wrap">
          <div class="sql-viewer-header-left">
            <span class="dialog-title-text">
              📄 工单 #{{ ticketDetail?.ticket?.id || '' }} 提交的完整 SQL 脚本详情
            </span>
            <el-tag size="small" type="primary" effect="plain" style="font-family: monospace; font-weight: 600;">
              {{ sqlList.length }} 条语句 · {{ submittedSqlLines.length }} 行
            </el-tag>
            <el-tag v-if="ticketDetail?.ticket?.dbName" size="small" type="success" effect="plain" style="font-weight: 600;">
              🗃️ 目标数据库: {{ ticketDetail.ticket.dbName }}
            </el-tag>
          </div>
          <div class="sql-viewer-header-right">
            <el-input
              v-model="sqlViewerSearch"
              placeholder="在 SQL 中快速定位关键字..."
              size="small"
              clearable
              :prefix-icon="Search"
              style="width: 240px;"
            />
            <el-button size="small" :icon="CopyDocument" @click="copyAllSql">复制全部 SQL</el-button>
            <el-button size="small" type="success" :icon="Download" @click="downloadSubmittedSql">下载 SQL (.sql)</el-button>
          </div>
        </div>
      </template>

      <div class="fullscreen-sql-content-body">
        <div class="sql-viewer-stats-bar">
          <span>💾 脚本大小: {{ (ticketDetail?.detail?.sqlText || '').length }} 字符</span>
          <span>⏱️ 提交时间: {{ ticketDetail?.ticket?.createTime || '-' }}</span>
          <span>👤 申请人: {{ cleanApplicantName(ticketDetail?.ticket?.applicantName || ticketDetail?.ticket?.applicantIdCard) }}</span>
          <span>🏷️ 变更类型: {{ getTicketTypeLabel(ticketDetail?.ticket?.type) }}</span>
          <span>🗂️ 业务资源组: {{ getInstanceResourceGroup(ticketDetail?.ticket || {}) }}</span>
        </div>

        <div class="sql-code-editor-container">
          <div class="sql-line-numbers">
            <div v-for="n in submittedSqlLines.length" :key="n" class="sql-line-num">{{ n }}</div>
          </div>
          <div class="sql-code-lines">
            <div
              v-for="(line, idx) in submittedSqlLines"
              :key="idx"
              class="sql-code-line"
              :class="{ 'highlight-search': sqlViewerSearch && line.toLowerCase().includes(sqlViewerSearch.toLowerCase()) }"
            >
              {{ line || ' ' }}
            </div>
          </div>
        </div>
      </div>
    </el-dialog>

    <!-- 审批加急催办弹窗 (企业微信 / 钉钉多渠道触达) -->
    <el-dialog
      v-model="urgeDialogVisible"
      title="⚡ 审批加急催办 (企业微信 / 钉钉多渠道触达)"
      width="520px"
      append-to-body
      destroy-on-close
    >
      <div class="urge-dialog-body">
        <el-alert
          type="warning"
          show-icon
          :closable="false"
          title="系统将向当前节点所有候选审批人员发送即时工作通知，提醒尽快审阅与执行。"
          style="margin-bottom: 16px;"
        />

        <el-form label-position="top">
          <el-form-item label="当前待审批节点">
            <el-input :model-value="activeUrgeNode?.nodeName || '审批节点'" disabled />
          </el-form-item>

          <el-form-item label="接收催办通知的候选审批人">
            <div style="display: flex; flex-wrap: wrap; gap: 6px; width: 100%;">
              <el-tag
                v-for="app in (activeUrgeNode?.eligibleApprovers || [])"
                :key="app"
                type="success"
                effect="dark"
              >
                <el-icon style="margin-right: 2px;"><User /></el-icon>{{ app }}
              </el-tag>
            </div>
          </el-form-item>

          <el-form-item label="即时触达渠道">
            <div style="display: flex; gap: 8px; flex-wrap: wrap;">
              <el-tag type="success">🟢 企业微信应用工作消息</el-tag>
              <el-tag type="primary">🔵 阿里钉钉群/工作通知</el-tag>
              <el-tag type="success" style="background: rgba(16,185,129,0.1); border-color: #10b981; color: #047857;">🟣 字节飞书 / Lark 交互卡片</el-tag>
              <el-tag type="warning">🟡 站内待办审批提醒</el-tag>
            </div>
          </el-form-item>

          <el-form-item label="催办留言说明 (可选)">
            <el-input
              v-model="urgeReason"
              type="textarea"
              :rows="3"
              placeholder="请输入加急原因（例如：生产发布在即，核心业务依赖该 SQL 变更，请组长与 DBA 尽快审阅...）"
              maxlength="200"
              show-word-limit
            />
          </el-form-item>
        </el-form>
      </div>

      <template #footer>
        <div class="dialog-footer" style="display: flex; justify-content: flex-end; gap: 10px;">
          <el-button @click="urgeDialogVisible = false">取消</el-button>
          <el-button
            type="warning"
            :loading="urgeLoading"
            :icon="Bell"
            @click="handleConfirmUrge"
          >
            立即发送催办通知
          </el-button>
        </div>
      </template>
    </el-dialog>

    <!-- 1. 主动终止工单弹窗 -->
    <el-dialog
      v-model="terminateDialogVisible"
      title="🛑 主动终止工单流程"
      width="500px"
      append-to-body
      destroy-on-close
    >
      <div>
        <el-alert
          type="error"
          show-icon
          :closable="false"
          title="终止工单后，当前工单的审批流将立即作废并停止后续流式/线下执行！所有人均可根据业务需要随时终止工单。"
          style="margin-bottom: 16px;"
        />
        <el-form label-position="top">
          <el-form-item label="终止原因说明" required>
            <el-input
              v-model="terminateReason"
              type="textarea"
              :rows="3"
              placeholder="请输入终止原因（例如：业务需求调整不再需要变更、发现 SQL 逻辑有误需作废、重复提交等...）"
              maxlength="200"
              show-word-limit
            />
          </el-form-item>
        </el-form>
      </div>
      <template #footer>
        <div class="dialog-footer" style="display: flex; justify-content: flex-end; gap: 10px;">
          <el-button @click="terminateDialogVisible = false">取消</el-button>
          <el-button
            type="danger"
            :loading="terminateLoading"
            @click="handleConfirmTerminate"
          >
            确认终止工单
          </el-button>
        </div>
      </template>
    </el-dialog>


  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ElMessage, ElMessageBox, ElNotification } from 'element-plus'
import {
  ArrowLeft, CopyDocument, Download, Connection, Check,
  Loading, Close, UserFilled, User, ChatLineSquare, Stamp,
  Document, Tickets, EditPen, VideoPlay, Clock, Tools, Finished, Refresh,
  FullScreen, Search, Bell, RefreshLeft, Share
} from '@element-plus/icons-vue'
import request from '../utils/request'
import { downloadTicketAttachment } from '../api/ticket'
import { useUserStore } from '../store/user'

const userStore = useUserStore()

const route = useRoute()
const router = useRouter()
const searchQuery = ref('')
const ticketDetail = ref<any>(null)
const refreshLoading = ref(false)
let pollTimer: any = null

// SQL 分页与全屏查看状态
const sqlCurrentPage = ref(1)
const sqlPageSize = ref(20)
const sqlViewerVisible = ref(false)
const sqlViewerSearch = ref('')

// 审批相关表单
const approvalExecutionMode = ref<'IMMEDIATE' | 'SCHEDULED' | 'CANARY_BATCH' | 'MANUAL_DBA'>('IMMEDIATE')
const approvalScheduledTime = ref('')
const approvalBatchSize = ref(500)
const approvalIntervalMs = ref(100)
const approvalComment = ref('影响行数校验无误，同意发布上线')
const approveLoading = ref(false)
const rejectLoading = ref(false)

// DBA 线下执行反馈表单
const dbaFeedbackForm = ref({
  status: 'SUCCESS',
  affectRows: 1,
  durationMs: 120,
  feedbackNotes: '已通过专用客户端工具执行完毕，线上数据校验一致，无异常告警。'
})
const feedbackLoading = ref(false)
const executeNowLoading = ref(false)

const handleBack = () => {
  router.push('/ticket-list')
}

const openInNewWindow = () => {
  const id = route.params.id || '1'
  const href = router.resolve(`/ticket/${id}`).href
  window.open(href, '_blank')
}

const fetchTicketDetail = async () => {
  try {
    const id = route.params.id || '1'
    const response: any = await request.get(`/v1/ticket/${id}/detail`)
    ticketDetail.value = response.data

    if (!ticketDetail.value || !ticketDetail.value.ticket) {
      throw new Error('No data')
    }

    if (ticketDetail.value.detail?.affectRowsEstimate) {
      dbaFeedbackForm.value.affectRows = ticketDetail.value.detail.affectRowsEstimate
    }

    // 同时拉取工单操作日志
    fetchOperationLogs(id as string)

    // 检查是否需要启动轮询
    startPollingIfNeeded()
  } catch (error) {
    ElMessage.error('获取详情失败，请检查工单 ID 或权限')
    ticketDetail.value = {
      ticket: {},
      detail: {},
      flowNodes: []
    }
  }
}

// 工单操作日志
const operationLogs = ref<any[]>([])

const fetchOperationLogs = async (id: string) => {
  try {
    const res: any = await request.get(`/v1/ticket/${id}/logs`)
    operationLogs.value = res.data || []
  } catch (e) {
    operationLogs.value = []
  }
}

const getOperationTypeLabel = (type: string): string => {
  const map: Record<string, string> = {
    SUBMIT: '提交工单',
    APPROVE: '审批通过',
    REJECT: '审批驳回',
    SCHEDULED: '定时排期',
    RESCHEDULE: '定时任务调整',
    CANARY_BATCH: '灰度分批',
    MANUAL_DBA: '转DBA线下',
    EXECUTE: '立即触发',
    ENGINE_EXECUTE: '流式引擎执行',
    EXECUTE_SUCCESS: '执行成功',
    EXECUTE_FAIL: '执行失败',
    DBA_FEEDBACK: 'DBA反馈结果',
    ARCHIVE: '合规归档',
    REVOKE: '工单撤回'
  }
  return map[type] || type
}

const getLogItemType = (type: string): '' | 'primary' | 'success' | 'warning' | 'danger' | 'info' => {
  if (type === 'SUBMIT') return 'primary'
  if (type === 'APPROVE' || type === 'EXECUTE_SUCCESS' || type === 'DBA_FEEDBACK' || type === 'ARCHIVE') return 'success'
  if (type === 'ENGINE_EXECUTE') return 'primary'
  if (type === 'REJECT' || type === 'EXECUTE_FAIL') return 'danger'
  if (type === 'SCHEDULED' || type === 'CANARY_BATCH') return 'warning'
  return 'info'
}

const handleManualRefresh = async () => {
  refreshLoading.value = true
  try {
    await fetchTicketDetail()
    ElMessage.success('状态已刷新')
  } finally {
    refreshLoading.value = false
  }
}

const startPollingIfNeeded = () => {
  if (pollTimer) {
    clearInterval(pollTimer)
    pollTimer = null
  }

  const currentStatus = ticketDetail.value?.ticket?.status
  if (currentStatus === 'APPROVED' || currentStatus === 'EXECUTING') {
    let attempts = 0
    pollTimer = setInterval(async () => {
      attempts++
      try {
        const id = route.params.id || '1'
        const res: any = await request.get(`/v1/ticket/${id}/detail`)
        ticketDetail.value = res.data
        const st = res.data?.ticket?.status
        if (st === 'EXECUTED' || st === 'FAILED' || st === 'REJECTED' || attempts >= 10) {
          clearInterval(pollTimer)
          pollTimer = null
        }
      } catch (e) {
        clearInterval(pollTimer)
        pollTimer = null
      }
    }, 1500)
  }
}

const flowNodes = computed(() => {
  if (ticketDetail.value?.flowNodes && Array.isArray(ticketDetail.value.flowNodes)) {
    return ticketDetail.value.flowNodes
  }
  return []
})

const flowDisplayMode = ref<'bpmn' | 'timeline' | 'pipeline'>('bpmn')

const isDetailHighRiskRoute = computed(() => {
  if (ticketDetail.value?.gatewayDecision?.isHighRisk !== undefined) {
    return !!ticketDetail.value.gatewayDecision.isHighRisk
  }
  const hasDdlSql = sqlList.value.some((item: any) => item.isDdl)
  const totalRows = getActualAffectRowsDisplay()
  return hasDdlSql || Number(totalRows) > 1000 || ticketDetail.value?.ticket?.type === 'DDL_CHANGE'
})

const activeNodeInfo = computed(() => {
  return flowNodes.value.find((n: any) => n.status === 'ACTIVE') || null
})

const parsedCustomFields = computed(() => {
  const raw = ticketDetail.value?.detail?.customFieldValues
  if (!raw) return null
  try {
    const obj = JSON.parse(raw)
    return (obj && typeof obj === 'object') ? obj : null
  } catch (e) {
    return null
  }
})

const formatStandardRoleText = (roleDesc: string) => {
  if (!roleDesc) return '审批责任人'
  if (roleDesc.includes('DEV_LEAD') || roleDesc.includes('开发组长') || roleDesc.includes('组长')) {
    return '🛡️ 业务开发组长'
  }
  if (roleDesc.includes('DBA') || roleDesc.includes('数据库管理员')) {
    return '💾 核心数据库管理员'
  }
  if (roleDesc.includes('AUDITOR') || roleDesc.includes('审计')) {
    return '🔍 安全合规审计员'
  }
  if (roleDesc.includes('ADMIN') || roleDesc.includes('超级管理员') || roleDesc.includes('管理员')) {
    return '👑 系统超级管理员'
  }
  if (roleDesc.includes('DEV') || roleDesc.includes('研发') || roleDesc.includes('开发') || roleDesc.includes('发起人')) {
    return '💻 研发工程师'
  }
  if (roleDesc.includes('调度引擎') || roleDesc.includes('流式') || roleDesc.includes('引擎')) {
    return '⚙️ 安全流式调度引擎'
  }
  return roleDesc
}

const getRoleBadgeClass = (roleDesc: string) => {
  if (!roleDesc) return 'role-default'
  if (roleDesc.includes('DBA') || roleDesc.includes('数据库管理员')) return 'role-dba'
  if (roleDesc.includes('ADMIN') || roleDesc.includes('超级管理员')) return 'role-admin'
  if (roleDesc.includes('DEV_LEAD') || roleDesc.includes('开发组长') || roleDesc.includes('组长')) return 'role-lead'
  if (roleDesc.includes('AUDITOR') || roleDesc.includes('审计')) return 'role-auditor'
  if (roleDesc.includes('DEV') || roleDesc.includes('研发') || roleDesc.includes('发起人')) return 'role-dev'
  return 'role-engine'
}

// 催办相关状态与方法
const isTicketAuditing = computed(() => {
  const status = ticketDetail.value?.ticket?.status
  return ['AUDITING', 'PENDING_APPROVAL', 'MANUAL_PROCESSING'].includes(status || '')
})

const urgeDialogVisible = ref(false)
const urgeLoading = ref(false)
const urgeReason = ref('')
const urgeCooldown = ref(0)
const activeUrgeNode = ref<any>(null)
let urgeTimer: any = null

const handleOpenUrgeDialog = (node?: any) => {
  if (urgeCooldown.value > 0) {
    ElMessage.warning(`您刚已发起过催办，请等待 ${urgeCooldown.value} 秒后再次催办`)
    return
  }
  if (node && node.nodeName) {
    activeUrgeNode.value = node
  } else {
    activeUrgeNode.value = {
      nodeName: ticketDetail.value?.gatewayDecision?.targetNodeName || '当前审批节点',
      eligibleApprovers: ticketDetail.value?.gatewayDecision?.eligibleApprovers || ['当前节点审批人']
    }
  }
  urgeReason.value = '生产上线窗口临近，劳烦领导尽快协助审批！'
  urgeDialogVisible.value = true
}

const handleConfirmUrge = async () => {
  const ticketId = route.params.id
  if (!ticketId) return

  urgeLoading.value = true
  try {
    const res: any = await request.post(`/v1/ticket/${ticketId}/urge`, {
      reason: urgeReason.value
    })
    ElMessage.success(res.data?.message || '已成功向当前节点候选审批人发送企微/钉钉加急催办通知！')
    urgeDialogVisible.value = false

    // 启动 60 秒冷却倒计时防频繁刷屏
    urgeCooldown.value = 60
    if (urgeTimer) clearInterval(urgeTimer)
    urgeTimer = setInterval(() => {
      if (urgeCooldown.value > 0) {
        urgeCooldown.value--
      } else {
        clearInterval(urgeTimer)
      }
    }, 1000)

    // 重新加载工单详情以呈现催办流水日志
    await fetchTicketDetail()
  } catch (error: any) {
    ElMessage.error(error.message || '催办请求失败，请稍后重试')
  } finally {
    urgeLoading.value = false
  }
}

const canApproveCurrentTicket = computed(() => {
  return !!ticketDetail.value?.canApprove
})

const isFinalApprovalStep = computed(() => {
  if (ticketDetail.value?.isFinalApprovalStep !== undefined) {
    return !!ticketDetail.value.isFinalApprovalStep
  }
  if (ticketDetail.value?.gatewayDecision?.isFinalApprovalStep !== undefined) {
    return !!ticketDetail.value.gatewayDecision.isFinalApprovalStep
  }
  return true
})

const currentUserName = computed(() => {
  return ticketDetail.value?.currentUserName || '当前用户'
})

const currentOperatorDesc = computed(() => {
  if (ticketDetail.value?.isAdmin) {
    return `当前操作人: ${currentUserName.value} (系统管理员 👑 拥有全节点直接审批特权)`
  }
  return `当前操作人: ${currentUserName.value} (${ticketDetail.value?.currentUserRole || '开发人员'})`
})

const getExecModeBtnLabel = (mode: string) => {
  if (mode === 'SCHEDULED') return '定时计划执行'
  if (mode === 'CANARY_BATCH') return '灰度分批执行'
  if (mode === 'MANUAL_DBA') return '转 DBA 线下工具'
  return '立即流式执行'
}

const handleApproveTicket = async () => {
  if (isFinalApprovalStep.value && approvalExecutionMode.value === 'SCHEDULED' && !approvalScheduledTime.value) {
    ElMessage.warning('请选择计划执行时间')
    return
  }

  try {
    if (isFinalApprovalStep.value) {
      let modeText = '【立即流式执行】'
      if (approvalExecutionMode.value === 'SCHEDULED') {
        modeText = `【定时计划执行：${approvalScheduledTime.value}】`
      } else if (approvalExecutionMode.value === 'CANARY_BATCH') {
        modeText = `【灰度分批执行：每批 ${approvalBatchSize.value} 行，间隔 ${approvalIntervalMs.value} ms】`
      } else if (approvalExecutionMode.value === 'MANUAL_DBA') {
        modeText = '【转由 DBA 工具线下执行并反馈结果】'
      }

      await ElMessageBox.confirm(`确认终审通过此工单？执行方式为：${modeText}`, '工单终审确认', {
        confirmButtonText: '确认终审通过并下发',
        cancelButtonText: '取消',
        type: 'success'
      })
    } else {
      const nextStage = ticketDetail.value?.gatewayDecision?.nextStageName || '下一级审批人'
      await ElMessageBox.confirm(`确认初审通过此工单？审核通过后流程将流转推进至【${nextStage}】进行复核。`, '阶段初审确认', {
        confirmButtonText: '确认推进',
        cancelButtonText: '取消',
        type: 'primary'
      })
    }

    approveLoading.value = true
    const id = route.params.id || '1'
    const res: any = await request.post(`/v1/ticket/${id}/approve`, {
      executionMode: isFinalApprovalStep.value ? approvalExecutionMode.value : 'IMMEDIATE',
      scheduledTime: isFinalApprovalStep.value ? approvalScheduledTime.value : null,
      batchSize: isFinalApprovalStep.value ? approvalBatchSize.value : null,
      intervalMs: isFinalApprovalStep.value ? approvalIntervalMs.value : null,
      comment: approvalComment.value
    })

    if (res.data && res.data.message && (res.data.message.includes('流转') || res.data.message.includes('推进') || res.data.message.includes('待'))) {
      ElNotification({
        title: '阶段节点审批通过',
        message: res.data.message,
        type: 'success',
        duration: 5000
      })
    } else if (res.data && res.data.success) {
      ElNotification({
        title: '终审通过并已执行',
        message: res.data.message || '已在目标数据库成功执行完毕！',
        type: 'success',
        duration: 5000
      })
    } else if (res.data && !res.data.success) {
      ElNotification({
        title: '执行异常',
        message: res.data.message || '执行遇到错误，请排查审计日志',
        type: 'error',
        duration: 6000
      })
    } else {
      ElMessage.success('工单审批通过，流程已成功推进！')
    }

    await fetchTicketDetail()
  } catch (err: any) {
    if (err !== 'cancel') {
      ElMessage.error(err.response?.data?.message || '审批失败')
    }
  } finally {
    approveLoading.value = false
  }
}

const handleRejectTicket = async () => {
  try {
    const { value: reason } = await ElMessageBox.prompt('请输入驳回工单的原因说明：', '驳回工单确认', {
      confirmButtonText: '确认驳回',
      cancelButtonText: '取消',
      inputPattern: /^.+$/,
      inputErrorMessage: '驳回理由不能为空',
      type: 'warning'
    })

    rejectLoading.value = true
    const id = route.params.id || '1'
    await request.post(`/v1/ticket/${id}/reject`, { comment: reason })
    ElMessage.warning('工单已驳回')
    await fetchTicketDetail()
  } catch (err: any) {
    if (err !== 'cancel') {
      ElMessage.error(err.response?.data?.message || '驳回失败')
    }
  } finally {
    rejectLoading.value = false
  }
}

// 提交 DBA 线下执行反馈
const handleSubmitDbaFeedback = async () => {
  if (!dbaFeedbackForm.value.feedbackNotes.trim()) {
    ElMessage.warning('请填写 DBA 执行日志与验证备注')
    return
  }

  try {
    await ElMessageBox.confirm('确认提交 DBA 线下执行反馈并归档此工单？', '反馈提交确认', {
      confirmButtonText: '确认提交归档',
      cancelButtonText: '取消',
      type: 'primary'
    })

    feedbackLoading.value = true
    const id = route.params.id || '1'
    await request.post(`/v1/ticket/${id}/feedback`, {
      status: dbaFeedbackForm.value.status,
      affectRows: dbaFeedbackForm.value.affectRows,
      durationMs: dbaFeedbackForm.value.durationMs,
      feedbackNotes: dbaFeedbackForm.value.feedbackNotes
    })

    ElMessage.success('DBA 执行反馈已提交，工单已成功归档！')
    await fetchTicketDetail()
  } catch (err: any) {
    if (err !== 'cancel') {
      ElMessage.error(err.response?.data?.message || '提交反馈失败')
    }
  } finally {
    feedbackLoading.value = false
  }
}

// 立即手动触发流式执行
const handleExecuteNow = async () => {
  try {
    await ElMessageBox.confirm('确认立即手动触发流式执行？系统将连接目标数据库自动下发语句。', '立即执行确认', {
      confirmButtonText: '立即执行',
      cancelButtonText: '取消',
      type: 'warning'
    })

    executeNowLoading.value = true
    const id = route.params.id || '1'
    const res: any = await request.post(`/v1/ticket/${id}/execute-now`)

    if (res.data && res.data.success) {
      ElNotification({
        title: '流式执行成功',
        message: res.data.message || '已在目标数据库成功执行完毕！',
        type: 'success',
        duration: 5000
      })
    } else if (res.data && !res.data.success) {
      ElNotification({
        title: '执行异常',
        message: res.data.message || '执行遇到错误',
        type: 'error',
        duration: 6000
      })
    } else {
      ElMessage.success('已触发流式执行！')
    }

    await fetchTicketDetail()
  } catch (err: any) {
    if (err !== 'cancel') {
      ElMessage.error(err.response?.data?.message || '触发执行失败')
    }
  } finally {
    executeNowLoading.value = false
  }
}

// 重新指定定时执行时间逻辑
const rescheduleDialogVisible = ref(false)
const rescheduleSubmitting = ref(false)
const rescheduleForm = ref({
  scheduledTime: '',
  comment: ''
})

const disablePastDate = (time: Date) => {
  return time.getTime() < Date.now() - 8.64e7
}

const quickTimePresets = computed(() => {
  const now = new Date()
  const formatDate = (d: Date) => {
    const pad = (n: number) => String(n).padStart(2, '0')
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
  }

  // 1. 今晚 23:30
  const tonight = new Date(now)
  tonight.setHours(23, 30, 0, 0)
  if (tonight.getTime() <= now.getTime()) {
    tonight.setDate(tonight.getDate() + 1)
  }

  // 2. 明日凌晨 02:00 (核心低峰)
  const tomorrowDawn = new Date(now)
  tomorrowDawn.setDate(tomorrowDawn.getDate() + 1)
  tomorrowDawn.setHours(2, 0, 0, 0)

  // 3. 明日清晨 05:00
  const tomorrowMorning = new Date(now)
  tomorrowMorning.setDate(tomorrowMorning.getDate() + 1)
  tomorrowMorning.setHours(5, 0, 0, 0)

  // 4. 明日夜间 23:00
  const tomorrowNight = new Date(now)
  tomorrowNight.setDate(tomorrowNight.getDate() + 1)
  tomorrowNight.setHours(23, 0, 0, 0)

  return [
    { label: '🌙 今晚低峰 23:30', value: formatDate(tonight) },
    { label: '⏰ 明日凌晨 02:00 (推荐)', value: formatDate(tomorrowDawn) },
    { label: '🌅 明日清晨 05:00', value: formatDate(tomorrowMorning) },
    { label: '📅 明日夜间 23:00', value: formatDate(tomorrowNight) }
  ]
})

const applyQuickTime = (val: string) => {
  rescheduleForm.value.scheduledTime = val
}

const timeRemainingHint = computed(() => {
  if (!rescheduleForm.value.scheduledTime) return ''
  try {
    const target = new Date(rescheduleForm.value.scheduledTime).getTime()
    const now = Date.now()
    const diff = target - now
    if (diff <= 0) return '已到达或已过所选时间，将立即触发调度'
    const hours = Math.floor(diff / (1000 * 60 * 60))
    const minutes = Math.floor((diff % (1000 * 60 * 60)) / (1000 * 60))
    return `预计将于 ${hours > 0 ? hours + ' 小时 ' : ''}${minutes} 分钟后由安全调度引擎自动流式执行`
  } catch (e) {
    return ''
  }
})

const handleOpenRescheduleDialog = () => {
  let curTime = ''
  const win = ticketDetail.value?.ticket?.executionWindow || ''
  const match = win.match(/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}/)
  if (match) {
    curTime = match[0]
  }
  rescheduleForm.value = {
    scheduledTime: curTime,
    comment: ''
  }
  rescheduleDialogVisible.value = true
}

const handleSubmitReschedule = async () => {
  if (!rescheduleForm.value.scheduledTime) {
    ElMessage.warning('请选择新的计划执行时间')
    return
  }
  rescheduleSubmitting.value = true
  try {
    const id = route.params.id || '1'
    await request.post(`/v1/ticket/${id}/reschedule`, {
      scheduledTime: rescheduleForm.value.scheduledTime,
      comment: rescheduleForm.value.comment
    })
    ElMessage.success('计划执行时间已成功更新！')
    rescheduleDialogVisible.value = false
    await fetchTicketDetail()
  } catch (err: any) {
    ElMessage.error(err.response?.data?.message || '更新执行时间失败')
  } finally {
    rescheduleSubmitting.value = false
  }
}

const copyAllSql = () => {
  if (!ticketDetail.value?.detail?.sqlText) {
    ElMessage.warning('暂无 SQL 内容可复制')
    return
  }
  navigator.clipboard.writeText(ticketDetail.value.detail.sqlText)
  ElMessage.success('完整 SQL 脚本已复制至剪贴板，可在客户端工具中直接粘贴执行！')
}

const ticketInfoList = computed(() => {
  if (!ticketDetail.value || !ticketDetail.value.ticket) return []
  const t = ticketDetail.value.ticket
  const d = ticketDetail.value.detail || {}
  return [
    {
      ticketId: t.id,
      applicant: t.applicantIdCard || '-',
      applicantName: t.applicantName || '',
      instance: t.instanceId || '-',
      dbName: t.dbName || '',
      createTime: t.createTime || '',
      type: t.type || 'SQL_AUDIT',
      affectRowsEstimate: d.affectRowsEstimate !== undefined ? d.affectRowsEstimate : '-',
      status: t.status || '-'
    }
  ]
})

const cleanApplicantName = (name?: string) => {
  if (!name) return '系统'
  return name.replace(/[\(（].*?[\)）]/g, '').trim() || name
}

const getAvatarInitial = (name?: string) => {
  const clean = cleanApplicantName(name)
  return clean ? clean.charAt(0) : '用'
}

const parseExpectedRows = (sql: string) => {
  const lines = sql.split('\n')
  for (const line of lines) {
    const trimmed = line.trim()
    const match = trimmed.match(/^--(?:(?:\s*(?:预期|预计)?影响行数|expect|expected|count)[:：]?\s*|\s+)(\d+)/i)
    if (match) {
      return parseInt(match[1], 10)
    }
    const blockMatch = trimmed.match(/^\/\*(?:(?:\s*(?:预期|预计)?影响行数|expect|expected|count)[:：]?\s*|\s+)(\d+)\s*\*\//i)
    if (blockMatch) {
      return parseInt(blockMatch[1], 10)
    }
    const inlineMatch = trimmed.match(/--(?:(?:\s*(?:预期|预计)?影响行数|expect|expected|count)[:：]?\s*|\s+)(\d+)/i)
    if (inlineMatch) {
      return parseInt(inlineMatch[1], 10)
    }
  }
  return null
}

const determineType = (sql: string) => {
  const clean = sql.replace(/\/\*[\s\S]*?\*\//g, '').replace(/--.*?(\r?\n|$)/g, '').trim()
  if (!clean) return 'EMPTY'
  const firstWord = clean.split(/\s+/)[0].toUpperCase()
  return firstWord
}

const sqlList = computed(() => {
  if (!ticketDetail.value || !ticketDetail.value.detail || !ticketDetail.value.detail.sqlText) return []
  const text = ticketDetail.value.detail.sqlText
  const rawSegments = text.split(';').map((s: string) => s.trim()).filter((s: string) => s.length > 0)

  const parsedItems: any[] = []
  let pendingExpectedRows: number | null = null

  for (let i = 0; i < rawSegments.length; i++) {
    const raw = rawSegments[i]
    const cleanSql = raw.replace(/\/\*[\s\S]*?\*\//g, '').replace(/--.*?(\r?\n|$)/g, '').trim()
    const expRows = parseExpectedRows(raw)

    if (!cleanSql) {
      // 纯注释片段（如单独的 "-- 1" 或注释行）
      if (expRows !== null) {
        if (parsedItems.length > 0 && parsedItems[parsedItems.length - 1].expectedRows === null) {
          parsedItems[parsedItems.length - 1].expectedRows = expRows
        } else {
          pendingExpectedRows = expRows
        }
      }
      continue
    }

    const type = determineType(raw)
    const isDml = ['INSERT', 'UPDATE', 'DELETE', 'REPLACE'].includes(type)
    const isDdl = ['CREATE', 'ALTER', 'DROP', 'TRUNCATE', 'RENAME', 'GRANT', 'REVOKE'].includes(type)
    const isDql = ['SELECT', 'SHOW', 'DESC', 'EXPLAIN'].includes(type)

    let finalExpRows = expRows !== null ? expRows : pendingExpectedRows
    pendingExpectedRows = null

    // 如果工单全局设置了预估行数且当前是唯一 DML 且未在 SQL 注释中指定
    if (finalExpRows === null && isDml && ticketDetail.value.ticket?.affectRowsEstimate) {
      finalExpRows = ticketDetail.value.ticket.affectRowsEstimate
    }

    let dryRunRows: number | null = null
    if (isDml) {
      if (ticketDetail.value.detail?.actualAffectRows !== undefined && ticketDetail.value.detail?.actualAffectRows !== null) {
        dryRunRows = ticketDetail.value.detail.actualAffectRows
      } else if (ticketDetail.value.ticket?.affectRowsEstimate !== undefined && ticketDetail.value.ticket?.affectRowsEstimate !== null) {
        dryRunRows = ticketDetail.value.ticket.affectRowsEstimate
      } else if (finalExpRows !== null) {
        dryRunRows = finalExpRows
      } else {
        dryRunRows = 1
      }
    } else if (isDdl) {
      dryRunRows = 0
    }

    parsedItems.push({
      id: parsedItems.length + 1,
      sqlContent: raw,
      type: type,
      isDml: isDml,
      isDdl: isDdl,
      isDql: isDql,
      expectedRows: finalExpRows,
      dryRunRows: dryRunRows,
      status: '通过'
    })
  }

  return parsedItems
})

const filteredSqlList = computed(() => {
  if (!searchQuery.value) return sqlList.value
  const q = searchQuery.value.toLowerCase().trim()
  return sqlList.value.filter((item: any) =>
    item.sqlContent.toLowerCase().includes(q) || (item.type && item.type.toLowerCase().includes(q))
  )
})

const pagedSqlList = computed(() => {
  const start = (sqlCurrentPage.value - 1) * sqlPageSize.value
  return filteredSqlList.value.slice(start, start + sqlPageSize.value)
})

watch(() => searchQuery.value, () => {
  sqlCurrentPage.value = 1
})

const submittedSqlLines = computed(() => {
  const text = ticketDetail.value?.detail?.sqlText || ''
  return text ? text.split('\n') : []
})

const getActualAffectRowsDisplay = () => {
  if (!ticketDetail.value) return 0

  // 优先：将 sqlList 中所有 DML 语句的预执行行数累加，得到准确总影响行数
  const dmlItems = sqlList.value.filter((item: any) => item.isDml && item.dryRunRows !== null)
  if (dmlItems.length > 0) {
    const total = dmlItems.reduce((sum: number, item: any) => sum + (item.dryRunRows || 0), 0)
    if (total > 0) return total
  }

  // 降级：从后端返回的字段中读取汇总值
  if (ticketDetail.value.executionInfo?.actualAffectRows !== undefined && ticketDetail.value.executionInfo?.actualAffectRows !== null) {
    return ticketDetail.value.executionInfo.actualAffectRows
  }
  if (ticketDetail.value.detail?.affectRowsEstimate !== undefined && ticketDetail.value.detail?.affectRowsEstimate !== null) {
    return ticketDetail.value.detail.affectRowsEstimate
  }
  if (ticketDetail.value.detail?.actualAffectRows !== undefined && ticketDetail.value.detail?.actualAffectRows !== null) {
    return ticketDetail.value.detail.actualAffectRows
  }
  if (ticketDetail.value.ticket?.affectRowsEstimate !== undefined && ticketDetail.value.ticket?.affectRowsEstimate !== null) {
    return ticketDetail.value.ticket.affectRowsEstimate
  }
  return 0
}

const openSqlViewerDialog = () => {
  const ticketId = ticketDetail.value?.ticket?.id || 'ticket'
  const sqlText = ticketDetail.value?.detail?.sqlText || ''
  const dbName = ticketDetail.value?.ticket?.dbName || 'default'
  const applicant = cleanApplicantName(ticketDetail.value?.ticket?.applicantName || ticketDetail.value?.ticket?.applicantIdCard)
  const createTime = ticketDetail.value?.ticket?.createTime || '-'
  const ticketType = getTicketTypeLabel(ticketDetail.value?.ticket?.type)
  const instanceName = getInstanceName(ticketDetail.value?.ticket?.instanceId || '')

  const lines = sqlText.split('\n')
  const lineHtml = lines.map((line: string, idx: number) =>
    `<div class="line"><span class="ln">${idx + 1}</span><span class="lc">${
      line.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;') || '&nbsp;'
    }</span></div>`
  ).join('')

  const html = `<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>工单 #${ticketId} — SQL 完整脚本详情</title>
<style>
  *{box-sizing:border-box;margin:0;padding:0}
  body{
    font-family:-apple-system,BlinkMacSystemFont,'Segoe UI','PingFang SC','Hiragino Sans GB','Microsoft YaHei',sans-serif;
    background:#f4f6f8;
    color:#1e293b;
    display:flex;
    flex-direction:column;
    height:100vh;
    -webkit-font-smoothing:antialiased;
  }
  .toolbar{
    display:flex;
    align-items:center;
    gap:10px;
    padding:10px 20px;
    background:#ffffff;
    border-bottom:1px solid #e2e8f0;
    box-shadow:0 1px 3px rgba(0,0,0,0.04);
    flex-shrink:0;
    flex-wrap:wrap;
  }
  .toolbar-title{
    font-size:15px;
    font-weight:700;
    color:#0f172a;
    display:flex;
    align-items:center;
    gap:6px;
    margin-right:6px;
  }
  .meta-tag{
    font-size:11.5px;
    font-weight:600;
    padding:3px 9px;
    border-radius:6px;
    display:inline-flex;
    align-items:center;
    gap:4px;
  }
  .tag-schema{background:#ecfdf5;color:#047857;border:1px solid #a7f3d0}
  .tag-user{background:#eff6ff;color:#1d4ed8;border:1px solid #bfdbfe}
  .tag-type{background:#f8fafc;color:#475569;border:1px solid #cbd5e1}
  .tag-inst{background:#fffbeb;color:#b45309;border:1px solid #fde68a}
  .spacer{flex:1}
  .actions{display:flex;align-items:center;gap:8px}
  .search-box{
    padding:6px 12px;
    border-radius:6px;
    border:1px solid #cbd5e1;
    background:#f8fafc;
    color:#1e293b;
    font-size:12.5px;
    width:220px;
    outline:none;
    transition:all 0.2s;
  }
  .search-box:focus{
    background:#ffffff;
    border-color:#3b82f6;
    box-shadow:0 0 0 3px rgba(59,130,246,0.15);
  }
  .search-box::placeholder{color:#94a3b8}
  .btn{
    display:inline-flex;
    align-items:center;
    gap:5px;
    font-size:12.5px;
    font-weight:500;
    padding:6px 14px;
    border-radius:6px;
    cursor:pointer;
    transition:all 0.2s;
    border:1px solid transparent;
  }
  .btn-default{
    background:#ffffff;
    border-color:#cbd5e1;
    color:#334155;
  }
  .btn-default:hover{
    background:#f1f5f9;
    border-color:#94a3b8;
    color:#0f172a;
  }
  .btn-primary{
    background:#2563eb;
    color:#ffffff;
  }
  .btn-primary:hover{
    background:#1d4ed8;
  }
  .stats-bar{
    display:flex;
    gap:18px;
    padding:8px 20px;
    background:#f8fafc;
    border-bottom:1px solid #e2e8f0;
    font-size:12px;
    color:#64748b;
    flex-shrink:0;
    flex-wrap:wrap;
  }
  .stats-item{display:flex;align-items:center;gap:4px}
  .code-container{
    flex:1;
    overflow:auto;
    background:#ffffff;
    display:flex;
    flex-direction:column;
  }
  .line{
    display:flex;
    align-items:baseline;
    min-height:22px;
    font-size:13px;
    line-height:1.65;
    font-family:Consolas, 'Fira Code', Menlo, Monaco, monospace;
    transition:background 0.15s;
  }
  .line:hover{background:#f1f5f9}
  .line.hl{background:#fef08a;box-shadow:inset 3px 0 0 #eab308}
  .ln{
    width:58px;
    min-width:58px;
    text-align:right;
    padding-right:16px;
    color:#94a3b8;
    background:#f8fafc;
    border-right:1px solid #edf2f7;
    user-select:none;
    font-size:12px;
  }
  .lc{
    flex:1;
    padding-left:14px;
    padding-right:20px;
    white-space:pre;
    color:#1e293b;
    word-break:break-all;
  }
  .lc-comment{color:#15803d;font-style:italic}
  .toast{
    position:fixed;
    top:16px;
    left:50%;
    transform:translateX(-50%);
    background:#0f172a;
    color:#ffffff;
    padding:8px 16px;
    border-radius:6px;
    font-size:13px;
    box-shadow:0 4px 12px rgba(0,0,0,0.15);
    opacity:0;
    transition:opacity 0.2s, top 0.2s;
    pointer-events:none;
    z-index:999;
  }
  .toast.show{opacity:1;top:24px}
</style>
</head>
<body>
<div class="toolbar">
  <div class="toolbar-title">📄 工单 #<b>${ticketId}</b> SQL 脚本详情</div>
  <span class="meta-tag tag-schema">🗃️ ${dbName}</span>
  <span class="meta-tag tag-user">👤 ${applicant}</span>
  <span class="meta-tag tag-type">${ticketType}</span>
  <span class="meta-tag tag-inst">${instanceName}</span>
  <div class="spacer"></div>
  <div class="actions">
    <input class="search-box" id="search" placeholder="🔍 搜索 SQL 关键字..." oninput="doSearch(this.value)">
    <button class="btn btn-default" onclick="copyAll()">📋 复制全部 SQL</button>
    <button class="btn btn-primary" onclick="downloadSql()">⬇ 下载 .sql 脚本</button>
  </div>
</div>
<div class="stats-bar">
  <span class="stats-item">⏱️ 提交时间: <b>${createTime}</b></span>
  <span class="stats-item">📏 脚本规模: <b>${lines.length}</b> 行 · <b>${sqlText.length}</b> 字符</span>
  <span class="stats-item">🌿 视图模式: <b>清新护眼宽屏模式</b></span>
</div>
<div class="code-container" id="codeWrap">${lineHtml}</div>
<div id="toast" class="toast"></div>

<script>
const rawSql = ${JSON.stringify(sqlText)};
function showToast(msg){
  const t = document.getElementById('toast');
  t.innerText = msg;
  t.classList.add('show');
  setTimeout(()=>t.classList.remove('show'), 2000);
}
function copyAll(){
  if(navigator.clipboard && navigator.clipboard.writeText){
    navigator.clipboard.writeText(rawSql).then(()=>showToast('✅ 完整 SQL 脚本已复制至剪贴板')).catch(()=>fallbackCopy());
  } else {
    fallbackCopy();
  }
}
function fallbackCopy(){
  const t = document.createElement('textarea');
  t.value = rawSql;
  document.body.appendChild(t);
  t.select();
  document.execCommand('copy');
  document.body.removeChild(t);
  showToast('✅ 完整 SQL 脚本已复制至剪贴板');
}
function downloadSql(){
  const blob = new Blob([rawSql], {type:'text/plain;charset=utf-8'});
  const a = document.createElement('a');
  a.href = URL.createObjectURL(blob);
  a.download = 'ticket_${ticketId}_submitted_sql.sql';
  a.click();
  showToast('⬇ SQL 文件已开始下载');
}
function doSearch(q){
  const lines = document.querySelectorAll('.line');
  lines.forEach(el => {
    const text = el.querySelector('.lc').textContent;
    if(q && text.toLowerCase().includes(q.toLowerCase())){
      el.classList.add('hl');
    } else {
      el.classList.remove('hl');
    }
  });
  if(q){
    const first = document.querySelector('.line.hl');
    if(first) first.scrollIntoView({behavior:'smooth', block:'center'});
  }
}
<\/script>
</body>
</html>`

  const blob = new Blob([html], { type: 'text/html;charset=utf-8' })
  const url = URL.createObjectURL(blob)
  const win = window.open(url, `sql_viewer_ticket_${ticketId}`, '')
  if (win) {
    win.addEventListener('load', () => URL.revokeObjectURL(url), { once: true })
  }
}

const downloadSubmittedSql = async () => {
  const text = ticketDetail.value?.detail?.sqlText
  const ticketId = ticketDetail.value?.ticket?.id || 'ticket'
  const filename = `ticket_${ticketId}_submitted_sql_${new Date().toISOString().slice(0, 10)}.sql`

  if (text) {
    const header = `-- ================================================================\n-- 工单编号: #${ticketId}\n-- 目标数据库实例: ${getInstanceName(ticketDetail.value?.ticket?.instanceId || '')}\n-- 目标数据库: ${ticketDetail.value?.ticket?.dbName || 'default'}\n-- 申请人: ${cleanApplicantName(ticketDetail.value?.ticket?.applicantName || ticketDetail.value?.ticket?.applicantIdCard)}\n-- 提交时间: ${ticketDetail.value?.ticket?.createTime || new Date().toLocaleString()}\n-- 说明: 原始提交执行 SQL 脚本\n-- ================================================================\n\n`
    const blob = new Blob([header + text], { type: 'text/plain;charset=utf-8' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = filename
    document.body.appendChild(a)
    a.click()
    document.body.removeChild(a)
    URL.revokeObjectURL(url)
    ElMessage.success(`提交 SQL 文件【${filename}】已开始下载`)
  } else if (ticketDetail.value?.detail?.attachmentOssKey) {
    await downloadAttachment()
  } else {
    ElMessage.warning('暂无提交的 SQL 内容可供下载')
  }
}

const getNodeStatusLabel = (status: string) => {
  switch (status) {
    case 'COMPLETED': return '已完成'
    case 'ACTIVE': return '当前进行中'
    case 'REJECTED': return '已驳回'
    default: return '等待中'
  }
}

const getNodeStatusTagType = (status: string) => {
  switch (status) {
    case 'COMPLETED': return 'success'
    case 'ACTIVE': return 'warning'
    case 'REJECTED': return 'danger'
    default: return 'info'
  }
}

// 终止工单相关状态与方法
const terminateDialogVisible = ref(false)
const terminateReason = ref('')
const terminateLoading = ref(false)

const handleOpenTerminateDialog = () => {
  terminateReason.value = ''
  terminateDialogVisible.value = true
}

const handleConfirmTerminate = async () => {
  const ticketId = route.params.id
  if (!ticketId) return
  if (!terminateReason.value.trim()) {
    ElMessage.warning('请填写终止原因说明')
    return
  }

  terminateLoading.value = true
  try {
    await request.post(`/v1/ticket/${ticketId}/terminate`, {
      reason: terminateReason.value.trim()
    })
    ElMessage.success('工单流程已成功终止！')
    terminateDialogVisible.value = false
    await fetchTicketDetail()
  } catch (error: any) {
    ElMessage.error(error.message || '终止工单失败')
  } finally {
    terminateLoading.value = false
  }
}

const isCurrentUserApplicant = computed(() => {
  if (!userStore.userInfo || !ticketDetail.value?.ticket) return false
  const myIdCard = userStore.userInfo.idCard
  const myUsername = userStore.userInfo.username
  const myRealName = userStore.userInfo.realName
  const app = ticketDetail.value.ticket.applicantIdCard || ticketDetail.value.ticket.applicantName || ''
  return app === myIdCard || app === myUsername || app === myRealName
})

const handleWithdrawTicket = async () => {
  const ticketId = ticketDetail.value?.ticket?.id || route.params.id
  if (!ticketId) return
  try {
    await ElMessageBox.confirm(
      `确定要撤回工单 #${ticketId} 吗？\n撤回后当前审批流将立即作废，并将自动载入该工单历史数据返回创建页面供您再次编辑。`,
      '撤回工单确认',
      {
        confirmButtonText: '确认撤回并编辑',
        cancelButtonText: '取消',
        type: 'warning'
      }
    )
    await request.post(`/v1/ticket/${ticketId}/withdraw`)
    ElMessage.success('工单已成功撤回，正在为您加载原数据进入编辑模式...')
    router.push({ path: '/ticket-create', query: { fromTicketId: ticketId } })
  } catch (err: any) {
    if (err !== 'cancel') {
      ElMessage.error(err.message || '撤回工单失败')
    }
  }
}

// 复制 / 再来一单方法
const handleCloneTicket = () => {
  const ticketId = ticketDetail.value?.ticket?.id || route.params.id
  if (ticketId) {
    router.push({ path: '/ticket-create', query: { fromTicketId: ticketId } })
  }
}

const getStatusType = (status: string) => {
  switch (status) {
    case 'APPROVED': return 'primary'
    case 'EXECUTED': return 'success'
    case 'WAITING_EXECUTION': return 'warning'
    case 'MANUAL_PROCESSING': return 'warning'
    case 'REJECTED': return 'danger'
    case 'FAILED': return 'danger'
    case 'TERMINATED': return 'info'
    default: return 'info'
  }
}

const getStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    'AUDITING': '审批中',
    'APPROVED': '已审批待执行',
    'WAITING_EXECUTION': '定时计划执行中',
    'MANUAL_PROCESSING': '转DBA工具执行中',
    'EXECUTED': '已执行归档',
    'REJECTED': '已驳回',
    'FAILED': '执行失败',
    'TERMINATED': '已终止'
  }
  return map[status] || status || '-'
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

const instances = ref<any[]>([])

const loadInstances = async () => {
  try {
    const res: any = await request.get('/v1/instance/list')
    instances.value = Array.isArray(res.data) ? res.data : []
  } catch (e) {
    // ignore
  }
}

const isRollbackExpanded = ref(false)
const isLogDetailsExpanded = ref(false)

const isRollbackLong = computed(() => {
  const text = ticketDetail.value?.detail?.rollbackSqlText || ''
  const lineCount = text.split('\n').length
  return lineCount > 6 || text.length > 300
})

const getInstanceObj = (id: number | string) => {
  return instances.value.find(i => String(i.id) === String(id)) || null
}

const getInstanceName = (id: number | string) => {
  const found = getInstanceObj(id)
  return found ? `${found.name} (${found.env || 'PROD'} · ${found.dbType || 'mysql'})` : `数据库实例 #${id}`
}

const getInstanceResourceGroup = (row: any) => {
  if (row.resourceGroup) return row.resourceGroup
  if (ticketDetail.value?.ticket?.resourceGroup) return ticketDetail.value.ticket.resourceGroup
  const inst = getInstanceObj(row.instance || ticketDetail.value?.ticket?.instanceId)
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

const getInstanceDeptOrSystem = (row: any) => {
  const inst = getInstanceObj(row.instance || ticketDetail.value?.ticket?.instanceId)
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
  const rg = getInstanceResourceGroup(row)
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

const formatExecutionWindow = (win?: string) => {
  if (!win) return '立即流式执行'
  if (win.includes('scheduled:') || win.includes('计划定时执行:')) {
    const time = win.replace('scheduled:', '').replace('计划定时执行:', '').trim()
    return `🕒 计划预约执行时间：${time}`
  }
  if (win.includes('immediate')) return '⚡ 审批通过后立即流式执行'
  if (win.includes('canary')) return `🚦 灰度分批流式执行 (${win})`
  if (win.includes('manual')) return '🛠️ 转交核心 DBA 线下安全执行'
  return win
}

const formatTimeShort = (timeStr?: string) => {
  if (!timeStr) return ''
  const parts = timeStr.split(' ')
  return parts.length > 1 ? parts[1] : timeStr
}

const handleDownloadRollbackSql = () => {
  const sql = ticketDetail.value?.detail?.rollbackSqlText
  if (!sql) {
    ElMessage.warning('暂无回滚 SQL 脚本可下载')
    return
  }
  const ticketId = ticketDetail.value?.ticket?.id || 'ticket'
  const filename = `ticket_${ticketId}_rollback_${new Date().toISOString().slice(0, 10)}.sql`
  const header = `-- ================================================================\n-- 工单编号: #${ticketId}\n-- 目标数据库: ${ticketDetail.value?.ticket?.dbName || 'default'}\n-- 导出时间: ${new Date().toLocaleString()}\n-- 说明: 生产应急回滚与数据备份脚本\n-- ================================================================\n\n`
  const blob = new Blob([header + sql], { type: 'text/plain;charset=utf-8' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = filename
  document.body.appendChild(a)
  a.click()
  document.body.removeChild(a)
  URL.revokeObjectURL(url)
  ElMessage.success(`回滚脚本文件【${filename}】已开始下载`)
}

const copyRollbackSql = () => {
  if (!ticketDetail.value?.detail?.rollbackSqlText) {
    ElMessage.warning('暂无回滚 SQL 可复制')
    return
  }
  navigator.clipboard.writeText(ticketDetail.value.detail.rollbackSqlText)
  ElMessage.success('数据回滚与备份 SQL 脚本已复制至剪贴板！')
}

const downloadRollbackAttachment = async () => {
  try {
    const id = route.params.id || '1'
    const url = await downloadTicketAttachment(id as string)
    const link = document.createElement('a')
    link.href = url
    link.target = '_blank'
    link.download = ticketDetail.value?.detail?.rollbackOssKey || 'rollback_backup.sql'
    document.body.appendChild(link)
    link.click()
    document.body.removeChild(link)
  } catch (error) {
    ElMessage.error('获取回滚附件下载链接失败')
  }
}

const downloadAttachment = async () => {
  try {
    const id = route.params.id || '1'
    const url = await downloadTicketAttachment(id as string)
    const link = document.createElement('a')
    link.href = url
    link.target = '_blank'
    link.download = ticketDetail.value?.detail?.attachmentOssKey || 'backup.sql'
    document.body.appendChild(link)
    link.click()
    document.body.removeChild(link)
  } catch (error) {
    ElMessage.error('获取附件下载链接失败')
  }
}

watch(() => route.params.id, (newId) => {
  if (newId) {
    fetchTicketDetail()
  }
})

onMounted(() => {
  loadInstances()
  fetchTicketDetail()
})

onUnmounted(() => {
  if (pollTimer) {
    clearInterval(pollTimer)
    pollTimer = null
  }
})
</script>

<style scoped>
.ticket-detail {
  width: 100%;
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

/* ==================== 粘性顶栏 (所有角色常驻关键上下文) ==================== */
.top-sticky-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: 10px;
  background: #ffffff;
  padding: 10px 18px;
  border-radius: 8px;
  border: 1px solid #e4e7ed;
  position: sticky;
  top: 8px;
  z-index: 100;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.08);
  backdrop-filter: blur(8px);
  background: rgba(255, 255, 255, 0.96);
}

.sticky-bar-left {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.sticky-bar-right {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.ticket-id-badge {
  display: flex;
  align-items: center;
}

.ticket-id-label {
  font-size: 14px;
  font-weight: 800;
  color: #1e3a8a;
  font-family: Consolas, monospace;
  letter-spacing: 0.5px;
}

.ticket-meta-inline {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
}

.meta-tag {
  font-size: 11px;
}

.meta-sep {
  color: #cbd5e1;
  font-size: 13px;
}

.meta-text {
  font-size: 12.5px;
  color: #475569;
  font-weight: 500;
  white-space: nowrap;
}

.status-badge-group {
  display: flex;
  align-items: center;
  gap: 6px;
}

.actions-left {
  display: flex;
  align-items: center;
  gap: 10px;
}

.actions-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

.status-tip-text {
  font-size: 11px;
  color: #94a3b8;
  font-weight: 500;
}

.main-status-tag {
  font-size: 12px;
  font-weight: 700;
  padding: 3px 10px;
}

/* ==================== 1. 审批流全景追踪卡片 ==================== */
.workflow-card {
  border-radius: 8px;
  border: none;
  background: linear-gradient(135deg, #f0f9ff 0%, #f8fafc 100%);
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.06);
}

.workflow-card :deep(.el-card__header) {
  background: linear-gradient(90deg, #1e3a8a 0%, #1d4ed8 100%);
  border-radius: 8px 8px 0 0;
  padding: 12px 18px;
}

.workflow-card :deep(.el-card__header) .title-text {
  color: #ffffff;
}

.workflow-card :deep(.el-card__header) .title-icon {
  color: #93c5fd;
}

.workflow-card :deep(.el-card__header) .card-header-flex {
  /* inherits flex from card-header-flex */
}

/* 智能排他网关决策横幅 */
.gateway-decision-banner {
  background: linear-gradient(135deg, #eff6ff 0%, #f0fdf4 100%);
  border: 1px solid #bfdbfe;
  border-radius: 8px;
  padding: 10px 16px;
  margin-bottom: 16px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: 12px;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.03);
}

.gateway-banner-left {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  color: #1e293b;
  line-height: 1.5;
}

.gw-icon {
  font-size: 16px;
  color: #2563eb;
  flex-shrink: 0;
}

.gw-text code {
  background: #e2e8f0;
  color: #0f172a;
  padding: 2px 6px;
  border-radius: 4px;
  font-family: 'Fira Code', Consolas, monospace;
  font-size: 12px;
}

.gateway-banner-right {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.curr-approver-label {
  font-size: 12.5px;
  font-weight: 600;
  color: #475569;
}

.approver-pill-group {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.curr-approver-tag {
  font-weight: 600;
  font-size: 11.5px;
  border-radius: 6px;
}

/* BPMN 2.0 流程图主画布 */
.bpmn-flow-visual-box {
  display: flex;
  flex-direction: column;
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

.bpmn-stage-header-track {
  display: grid;
  grid-template-columns: 142px 44px 168px 68px 305px 68px 186px 44px 132px;
  align-items: center;
  margin-bottom: 10px;
  padding-bottom: 8px;
  border-bottom: 1px solid rgba(203, 213, 225, 0.75);
  min-width: 1157px;
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

.bpmn-stage-grid {
  display: grid;
  grid-template-columns: 142px 44px 168px 68px 305px 68px 186px 44px 132px;
  align-items: center;
  min-width: 1157px;
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

.bpmn-flow-dash-anim {
  stroke-dasharray: 6, 4;
  animation: bpmnFlowDashMove 0.85s linear infinite;
}

@keyframes bpmnFlowDashMove {
  from { stroke-dashoffset: 20; }
  to { stroke-dashoffset: 0; }
}

.card-header-flex {
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
}

.header-title-group {
  display: flex;
  align-items: center;
  gap: 8px;
}

.title-icon {
  font-size: 18px;
  color: #409EFF;
}

.title-text {
  font-size: 15px;
  font-weight: 700;
  color: #303133;
}

.sub-text {
  font-size: 12px;
  color: #909399;
  margin-left: 6px;
}

.workflow-routing-info-banner {
  display: flex;
  align-items: center;
  gap: 8px;
  background: #f0f9ff;
  border: 1px solid #bae6fd;
  border-radius: 6px;
  padding: 8px 14px;
  margin-bottom: 12px;
  font-size: 12px;
  color: #1e293b;
}

.workflow-routing-info-banner.is-pinned-banner {
  background: #fffbeb;
  border-color: #fde68a;
}

.trigger-condition-text {
  color: #64748b;
  margin-left: 6px;
}

/* 流程横向轨道 */
.flow-track-container {
  display: flex;
  gap: 14px;
  overflow-x: auto;
  padding: 10px 4px 14px 4px;
}

.flow-node-item {
  flex: 1;
  min-width: 220px;
  position: relative;
  display: flex;
  flex-direction: column;
}

.node-icon-wrapper {
  display: flex;
  align-items: center;
  margin-bottom: 10px;
  position: relative;
}

.node-badge {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  background: #909399;
  color: #ffffff;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: bold;
  font-size: 13px;
  z-index: 2;
  transition: all 0.3s;
}

/* ==================== 现代化极简审批与执行流水线 ==================== */
.flow-pipeline-container {
  display: flex;
  align-items: stretch;
  gap: 8px;
  overflow-x: auto;
  padding: 8px 4px 16px 4px;
  scrollbar-width: thin;
  scrollbar-color: #cbd5e1 transparent;
}

.flow-pipeline-container::-webkit-scrollbar {
  height: 5px;
}
.flow-pipeline-container::-webkit-scrollbar-thumb {
  background: #cbd5e1;
  border-radius: 3px;
}

.pipeline-node-wrapper {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1;
  min-width: 220px;
  max-width: 300px;
}

.pipeline-node-wrapper.is-end-wrapper {
  flex: 0.9;
  min-width: 190px;
  max-width: 260px;
}

.pipeline-node-card {
  background: #ffffff;
  border: 1px solid #e2e8f0;
  border-radius: 10px;
  padding: 12px 14px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  flex: 1;
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.05);
  transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  position: relative;
  border-left-width: 3px;
  overflow: hidden;
}

/* 完成状态卡片顶部彩条 */
.pipeline-node-card::before {
  content: '';
  position: absolute;
  top: 0; left: 0; right: 0;
  height: 3px;
  border-radius: 10px 10px 0 0;
  background: transparent;
  transition: background 0.3s;
}

.pipeline-node-card.node-state-completed::before {
  background: linear-gradient(90deg, #16a34a, #86efac);
}

.pipeline-node-card.node-state-active::before {
  background: linear-gradient(90deg, #2563eb, #60a5fa);
}

.pipeline-node-card.node-state-rejected::before {
  background: linear-gradient(90deg, #dc2626, #fca5a5);
}

.pipeline-node-card:hover {
  transform: translateY(-3px);
  box-shadow: 0 8px 20px rgba(0, 0, 0, 0.1);
}

.pipeline-node-card.node-state-completed {
  border-color: #86efac;
  border-left-color: #16a34a;
  background: linear-gradient(160deg, #f0fdf4 0%, #ffffff 100%);
}

.pipeline-node-card.node-state-active {
  border-color: #60a5fa;
  border-left-color: #2563eb;
  background: linear-gradient(160deg, #eff6ff 0%, #ffffff 100%);
  box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.2), 0 4px 12px rgba(59, 130, 246, 0.12);
}

.pipeline-node-card.node-state-rejected {
  border-color: #fca5a5;
  border-left-color: #dc2626;
  background: linear-gradient(160deg, #fef2f2 0%, #ffffff 100%);
}

.pipeline-node-card.node-state-pending {
  border-color: #e2e8f0;
  border-left-color: #cbd5e1;
  background: #f8fafc;
  opacity: 0.75;
}

.node-head-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

/* 状态标签紧贴标题右侧 */
.node-status-inline {
  margin-left: auto;
  flex-shrink: 0;
}

.node-badge-circle {
  width: 26px;
  height: 26px;
  border-radius: 50%;
  background: #94a3b8;
  color: #ffffff;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  font-weight: 800;
  flex-shrink: 0;
  box-shadow: 0 1px 3px rgba(0,0,0,0.15);
}

.node-state-completed .node-badge-circle {
  background: linear-gradient(135deg, #16a34a, #22c55e);
  box-shadow: 0 2px 6px rgba(22, 163, 74, 0.35);
}

.node-state-active .node-badge-circle {
  background: linear-gradient(135deg, #2563eb, #60a5fa);
  box-shadow: 0 2px 6px rgba(37, 99, 235, 0.35);
  animation: pulse-glow 2s infinite;
}

.node-state-rejected .node-badge-circle {
  background: linear-gradient(135deg, #dc2626, #ef4444);
  box-shadow: 0 2px 6px rgba(220, 38, 38, 0.3);
}

.node-title-text {
  font-size: 13px;
  font-weight: 700;
  color: #1e293b;
  line-height: 1.3;
  flex: 1;
  min-width: 0;
}

.node-role-row {
  display: flex;
  align-items: center;
}

.role-badge {
  font-size: 11px;
  font-weight: 600;
  padding: 2px 8px;
  border-radius: 4px;
}

.role-dev {
  background: #f1f5f9;
  color: #334155;
  border: 1px solid #cbd5e1;
}

.role-lead {
  background: #eff6ff;
  color: #1d4ed8;
  border: 1px solid #bfdbfe;
}

.role-dba {
  background: #fef3c7;
  color: #b45309;
  border: 1px solid #fde68a;
}

.role-auditor {
  background: #ecfdf5;
  color: #047857;
  border: 1px solid #a7f3d0;
}

.role-admin {
  background: #fef2f2;
  color: #b91c1c;
  border: 1px solid #fecaca;
}

.role-engine {
  background: #f8fafc;
  color: #059669;
  border: 1px solid #86efac;
}

.node-target-approver {
  display: flex;
  flex-direction: column;
  gap: 2px;
  background: rgba(248, 250, 252, 0.8);
  border: 1px solid #f1f5f9;
  border-radius: 6px;
  padding: 5px 9px;
}

.node-state-active .node-target-approver {
  background: rgba(239, 246, 255, 0.7);
  border-color: #bfdbfe;
}

.node-target-approver .lbl {
  font-size: 10px;
  color: #64748b;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.3px;
}

.node-target-approver .val-box {
  display: flex;
  align-items: center;
  flex: 1;
}

.approvers-tag-wrap {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.approver-name-tag {
  font-size: 11.5px;
  font-weight: 600;
  border-radius: 4px;
}

.node-urge-box {
  margin-top: 4px;
  display: flex;
  justify-content: flex-start;
}

.urge-btn {
  font-size: 11.5px;
  font-weight: 600;
  border-radius: 4px;
  padding: 4px 10px;
  background: #fffbeb !important;
  border-color: #fde68a !important;
  color: #d97706 !important;
  transition: all 0.2s ease;
}

.urge-btn:hover {
  background: #fef3c7 !important;
  border-color: #f59e0b !important;
  color: #b45309 !important;
  transform: translateY(-1px);
}

.node-target-approver .val-name {
  font-size: 12.5px;
  font-weight: 700;
  color: #1e293b;
}

.node-record-details {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding-top: 6px;
  border-top: 1px dashed #e2e8f0;
  font-size: 11px;
}

.detail-row {
  display: flex;
  justify-content: space-between;
  color: #475569;
}

.detail-lbl {
  color: #94a3b8;
}

.detail-val {
  font-weight: 600;
}

.detail-comment-row {
  display: flex;
  align-items: flex-start;
  background: #f0fdf4;
  border: 1px solid #bbf7d0;
  border-radius: 4px;
  padding: 4px 8px;
  color: #15803d;
  font-size: 11px;
  margin-top: 2px;
}

.comment-text {
  line-height: 1.4;
}

/* 平滑连线与箭头 */
.pipeline-connector-line-wrapper {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: #94a3b8;
  flex-shrink: 0;
  gap: 2px;
}

.pipeline-line {
  display: none;
}

.pipeline-arrow {
  font-size: 20px;
  color: #cbd5e1;
  font-weight: 400;
  line-height: 1;
}

.end-node-desc {
  font-size: 11.5px;
  color: #64748b;
  line-height: 1.4;
}

/* ==================== 2. 审批操作控制台 ==================== */
.approval-action-card {
  border: 1px solid #faecd8;
  background: #fdf6ec;
}

.approval-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
}

.user-perm-info {
  display: flex;
  align-items: center;
}

.perm-title {
  font-size: 15px;
  font-weight: 700;
  color: #303133;
}

.perm-subtip {
  font-size: 12px;
  color: #909399;
}

/* ==================== 现代化执行方式卡片单选网格 ==================== */
.intermediate-stage-banner {
  background: linear-gradient(135deg, #eff6ff 0%, #dbeafe 100%);
  border: 1px solid #bfdbfe;
  border-radius: 10px;
  padding: 14px 18px;
  margin-bottom: 16px;
  box-shadow: 0 2px 6px rgba(37, 99, 235, 0.06);
}

.stage-banner-header {
  display: flex;
  align-items: flex-start;
  gap: 12px;
}

.stage-icon-col {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  border-radius: 8px;
  background: #ffffff;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08);
  flex-shrink: 0;
}

.stage-text-col {
  flex: 1;
}

.stage-main-title {
  font-size: 14px;
  font-weight: 700;
  color: #1e3a8a;
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 6px;
}

.stage-sub-desc {
  font-size: 12px;
  color: #3b82f6;
  line-height: 1.6;
  margin-top: 4px;
}

.exec-card-group-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 12px;
  width: 100%;
  margin-top: 6px;
}

.exec-mode-card {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 12px 14px;
  background: #f8fafc;
  border: 1.5px solid #e2e8f0;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s ease;
  position: relative;
  user-select: none;
}

.exec-mode-card:hover {
  background: #ffffff;
  border-color: #93c5fd;
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(59, 130, 246, 0.08);
}

.exec-mode-card.is-selected {
  background: linear-gradient(180deg, #eff6ff 0%, #ffffff 100%);
  border-color: #3b82f6;
  box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.2);
}

.card-radio-indicator {
  width: 16px;
  height: 16px;
  border-radius: 50%;
  border: 1.5px solid #cbd5e1;
  display: flex;
  align-items: center;
  justify-content: center;
  margin-top: 2px;
  flex-shrink: 0;
  transition: all 0.2s ease;
}

.exec-mode-card.is-selected .card-radio-indicator {
  border-color: #3b82f6;
  background: #3b82f6;
}

.inner-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #ffffff;
  opacity: 0;
  transition: all 0.2s ease;
}

.exec-mode-card.is-selected .inner-dot {
  opacity: 1;
}

.card-icon-col {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: 6px;
  flex-shrink: 0;
}

.icon-immediate {
  background: #eff6ff;
  color: #2563eb;
}

.icon-scheduled {
  background: #fffbeb;
  color: #d97706;
}

.icon-batch {
  background: #f0fdf4;
  color: #16a34a;
}

.icon-dba {
  background: #fdf2f8;
  color: #db2777;
}

.card-text-col {
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.card-title {
  font-size: 13.5px;
  font-weight: 700;
  color: #1e293b;
  display: flex;
  align-items: center;
  gap: 6px;
}

.badge-tag {
  font-size: 10px;
  font-weight: 700;
  background: #dbeafe;
  color: #1d4ed8;
  padding: 1px 5px;
  border-radius: 3px;
}

.card-desc {
  font-size: 11.5px;
  color: #64748b;
  line-height: 1.4;
}

/* 内联参数配置盒子 */
.inline-mode-config-box {
  margin-top: 12px;
  padding: 14px 16px;
  border-radius: 8px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  animation: fadeIn 0.25s ease;
}

.box-scheduled {
  background: #fffbeb;
  border: 1px solid #fde68a;
}

.box-batch {
  background: #f0fdf4;
  border: 1px solid #bbf7d0;
}

.config-title-row {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  font-weight: 700;
  color: #1e293b;
}

.config-control-row {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 16px;
}

.control-item {
  display: flex;
  align-items: center;
  gap: 8px;
}

.ctrl-label {
  font-size: 12.5px;
  font-weight: 600;
  color: #334155;
}

.sub-hint {
  font-size: 12px;
  color: #64748b;
}

@keyframes fadeIn {
  from { opacity: 0; transform: translateY(-4px); }
  to { opacity: 1; transform: translateY(0); }
}

.tip-immediate {
  background: #ecf5ff;
  border: 1px solid #d9ecff;
  color: #409EFF;
}

.tip-scheduled {
  background: #fdf6ec;
  border: 1px solid #faecd8;
  color: #E6A23C;
}

.tip-dba {
  background: #f4f4f5;
  border: 1px solid #e9e9eb;
  color: #606266;
}

.approval-btns-row {
  display: flex;
  justify-content: flex-end;
  gap: 14px;
  margin-top: 14px;
}

.no-perm-tip {
  margin-top: 10px;
  font-size: 12px;
  color: #909399;
  text-align: right;
}

/* ==================== 3. DBA 线下执行反馈面板 ==================== */
.dba-feedback-card {
  border: 1px solid #d9ecff;
  background: #f4f8fc;
}

.dba-feedback-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  width: 100%;
}

.dba-feedback-body {
  background: #ffffff;
  padding: 16px;
  border-radius: 4px;
  border: 1px solid #ebeef5;
}

.dba-action-btns {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  margin-top: 12px;
}

/* ==================== 4. 定时计划执行等待卡片 ==================== */
.scheduled-wait-card {
  border: 1px solid #faecd8;
  background: #fdf6ec;
}

.scheduled-box-flex {
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: 16px;
  padding: 6px 0;
}

.scheduled-left {
  display: flex;
  align-items: center;
  gap: 14px;
}

.clock-icon {
  font-size: 32px;
  color: #E6A23C;
}

.scheduled-title {
  font-size: 15px;
  font-weight: 700;
  color: #303133;
  margin-bottom: 4px;
}

.scheduled-desc {
  font-size: 13px;
  color: #606266;
}

/* ==================== 5. 执行结果与防篡改报告 ==================== */
.exec-result-card {
  border: 1px solid #e1f3d8;
  background: #fafdf8;
}

.result-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 14px;
  background: #ffffff;
  padding: 16px;
  border-radius: 4px;
  border: 1px solid #ebeef5;
}

.result-metric-item {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.m-label {
  font-size: 12px;
  color: #909399;
}

.m-val {
  font-size: 20px;
  font-weight: 700;
  color: #303133;
  font-family: monospace;
}

.m-val.highlight {
  color: #67C23A;
}

.m-val small {
  font-size: 13px;
  font-weight: normal;
  color: #606266;
}

.m-val-text {
  font-size: 14px;
  font-weight: 600;
  color: #409EFF;
}

/* ==================== 通用卡片与 SQL 清单 ==================== */
.section-card-wrapper {
  width: 100%;
}

.section-title-wrap {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 14px;
  font-weight: 700;
  color: #303133;
}

.execution-desc {
  background-color: #f8f9fa;
  padding: 12px 16px;
  border-radius: 4px;
  min-height: 40px;
  color: #606266;
  font-size: 13px;
  line-height: 1.6;
}

.table-actions-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  width: 100%;
  flex-wrap: wrap;
  gap: 8px;
}

.right-actions {
  display: flex;
  align-items: center;
  gap: 10px;
}

.sql-code-snippet {
  margin: 0;
  font-family: Consolas, 'Fira Code', Monaco, monospace;
  font-size: 12px;
  white-space: pre-wrap;
  background: #f8f9fa;
  padding: 6px 8px;
  border-radius: 4px;
}

.rollback-box {
  padding: 8px;
}

.rollback-snippet {
  background: #f0fdf4;
  border: 1px dashed #86efac;
  color: #166534;
}

.rollback-file-box {
  padding: 8px;
}

.gateway-decision-banner {
  margin: 0 16px 16px 16px;
  background: #f0fdf4;
  border: 1px solid #bbf7d0;
  border-radius: 6px;
  padding: 10px 14px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
}

.gateway-decision-banner.high-risk {
  background: #fef2f2;
  border-color: #fecaca;
}

.banner-left {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  color: #334155;
  flex-wrap: wrap;
}

.banner-icon {
  font-size: 16px;
  color: #3b82f6;
}

.banner-title {
  font-weight: 700;
  color: #0f172a;
}

.banner-detail {
  color: #475569;
}

.branch-highlight {
  font-weight: 600;
}

.branch-highlight.red {
  color: #dc2626;
}

.branch-highlight.green {
  color: #16a34a;
}

.custom-fields-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: 16px;
  background: #f8fafc;
  padding: 16px;
  border-radius: 6px;
  border: 1px solid #e2e8f0;
}

.cf-item {
  display: flex;
  align-items: center;
  gap: 8px;
}

.cf-label {
  font-size: 13px;
  font-weight: 600;
  color: #475569;
  white-space: nowrap;
}

.cf-val {
  font-size: 13px;
  font-weight: 600;
  color: #0f172a;
}

.font-mono {
  font-family: monospace;
}

.reschedule-modal-body {
  padding: 4px 6px;
}

.reschedule-tip-banner {
  display: flex;
  align-items: center;
  gap: 12px;
  background: #f8fafc;
  border: 1px solid #e2e8f0;
  border-left: 4px solid #f59e0b;
  padding: 12px 14px;
  border-radius: 6px;
  margin-bottom: 16px;
}

.reschedule-tip-banner .el-icon {
  font-size: 24px;
  color: #f59e0b;
}

.quick-preset-section {
  background: #fffbeb;
  border: 1px dashed #fde68a;
  padding: 12px 14px;
  border-radius: 6px;
  margin-bottom: 16px;
}

.quick-preset-title {
  font-size: 12px;
  font-weight: 600;
  color: #92400e;
  margin-bottom: 8px;
}

.quick-chips-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.quick-time-chip {
  background: #ffffff !important;
  border-color: #fcd34d !important;
  color: #78350f !important;
  font-size: 11.5px !important;
  font-weight: 600 !important;
  border-radius: 4px !important;
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
}

.quick-time-chip:hover {
  background: #fef3c7 !important;
  border-color: #f59e0b !important;
  transform: translateY(-1px);
}

.time-remaining-badge {
  margin-top: 6px;
  font-size: 12px;
  color: #0369a1;
  background: #f0f9ff;
  border: 1px solid #bae6fd;
  padding: 4px 10px;
  border-radius: 4px;
  display: inline-block;
  font-weight: 500;
}

/* 回滚代码块长文本折叠容器 */
.rollback-box-container {
  position: relative;
  background: #1e293b;
  border-radius: 6px;
  overflow: hidden;
}

.rollback-code-wrapper {
  transition: all 0.3s ease;
}

.rollback-code-wrapper.is-collapsed {
  max-height: 180px;
  overflow: hidden;
}

.rollback-expand-bar {
  background: linear-gradient(180deg, rgba(30, 41, 59, 0.7) 0%, #0f172a 100%);
  padding: 8px 12px;
  text-align: center;
  border-top: 1px solid #334155;
  cursor: pointer;
}

/* 横向流水线摘要条 */
.horizontal-log-summary-bar {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 14px 16px;
  background: #f8fafc;
  border-radius: 8px;
  border: 1px solid #e2e8f0;
  overflow-x: auto;
  flex-wrap: nowrap;
}

.summary-log-node {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-shrink: 0;
}

.node-icon-badge {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 15px;
  background: #ffffff;
  border: 1px solid #cbd5e1;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.05);
}

.node-icon-badge.badge-primary {
  border-color: #93c5fd;
  background: #eff6ff;
}

.node-icon-badge.badge-success {
  border-color: #86efac;
  background: #f0fdf4;
}

.node-icon-badge.badge-warning {
  border-color: #fde047;
  background: #fefce8;
}

.node-icon-badge.badge-danger {
  border-color: #fca5a5;
  background: #fef2f2;
}

.node-summary-text {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.node-action-title {
  font-size: 13px;
  font-weight: 700;
  color: #1e293b;
}

.node-operator-sub {
  font-size: 11px;
  color: #64748b;
}

.node-connector-arrow {
  font-size: 14px;
  color: #94a3b8;
  margin: 0 6px;
}

.full-log-timeline-wrapper {
  margin-top: 14px;
}

/* 全屏 SQL 查看器样式 */
.full-sql-viewer-dialog :deep(.el-dialog__header) {
  padding: 14px 20px;
  margin-right: 0;
  border-bottom: 1px solid #e2e8f0;
  background: #f8fafc;
}

.full-sql-viewer-dialog :deep(.el-dialog__body) {
  padding: 0;
  height: calc(100vh - 65px);
  display: flex;
  flex-direction: column;
  background: #0f172a;
}

.sql-viewer-header-wrap {
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: 12px;
}

.sql-viewer-header-left {
  display: flex;
  align-items: center;
  gap: 10px;
}

.dialog-title-text {
  font-size: 16px;
  font-weight: 700;
  color: #0f172a;
}

.sql-viewer-header-right {
  display: flex;
  align-items: center;
  gap: 10px;
}

.fullscreen-sql-content-body {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}

.sql-viewer-stats-bar {
  display: flex;
  align-items: center;
  gap: 20px;
  padding: 8px 18px;
  background: #1e293b;
  border-bottom: 1px solid #334155;
  color: #94a3b8;
  font-size: 12px;
  font-family: monospace;
}

.sql-code-editor-container {
  flex: 1;
  display: flex;
  overflow: auto;
  background: #0b1120;
  font-family: 'JetBrains Mono', 'Fira Code', Consolas, Monaco, monospace;
  font-size: 13px;
  line-height: 1.6;
}

.sql-line-numbers {
  padding: 14px 0;
  width: 55px;
  text-align: right;
  background: #0f172a;
  border-right: 1px solid #1e293b;
  color: #475569;
  user-select: none;
  flex-shrink: 0;
}

.sql-line-num {
  padding: 0 10px;
  height: 21px;
  line-height: 21px;
}

.sql-code-lines {
  flex: 1;
  padding: 14px 18px;
  color: #e2e8f0;
  overflow-x: auto;
  white-space: pre;
}

.sql-code-line {
  height: 21px;
  line-height: 21px;
}

.sql-code-line.highlight-search {
  background: rgba(234, 179, 8, 0.25);
  color: #fef08a;
  border-radius: 2px;
  font-weight: 700;
}
</style>
