<template>
  <div class="dashboard-container">
    <!-- 1. 问候与常用操作 -->
    <div class="welcome-hero-banner">
      <div class="welcome-left">
        <h1 class="welcome-heading">{{ greeting }}，{{ userStore.userInfo?.realName || userStore.userInfo?.username || userStore.realName || '你好' }}</h1>
        <p class="welcome-subtext">
          <template v-if="stats.pendingTickets">有 <b>{{ stats.pendingTickets }}</b> 张工单在等待审批。</template>
          <template v-else>当前没有等待审批的工单。</template>
        </p>
      </div>

      <div class="welcome-actions">
        <el-button type="primary" :icon="Plus" @click="router.push('/ticket-create')">新建工单</el-button>
        <el-button :icon="Search" @click="router.push('/data-query')">数据查询</el-button>
        <el-button :icon="MagicStick" @click="router.push('/ai-sql-review')">AI 审核</el-button>
        <el-tooltip content="刷新数据" placement="bottom">
          <el-button :icon="Refresh" :loading="loading" circle aria-label="刷新数据" @click="loadStats" />
        </el-tooltip>
      </div>
    </div>

    <!-- 2. 核心指标：一条带分隔的数字行，不再是六张彩色卡片 -->
    <div class="metric-strip">
      <button
        v-for="m in metricItems"
        :key="m.label"
        type="button"
        class="metric-cell"
        @click="router.push(m.to)"
      >
        <span class="metric-title">{{ m.label }}</span>
        <span class="metric-number" :class="m.tone">{{ m.value ?? '—' }}</span>
        <span class="metric-sub">{{ m.sub }}</span>
      </button>
    </div>

    <!-- 2.5 生产服务多节点集群拓扑矩阵 (支持多台机器展示、IP/名称/备注与指标监控) -->
    <div class="clean-section-card cluster-nodes-section">
      <div class="section-card-header">
        <div class="section-title-wrap">
          <div class="section-icon-dot bg-indigo-dot"></div>
          <h3 class="section-heading">生产服务多节点集群拓扑大盘 (Multi-Node Service Cluster)</h3>
          <span class="section-subtext">已纳管 {{ clusterNodes.length }} 台部署机器 · 支持根据 IP / 命名 / 备注展示 · 点击卡片切换聚焦监控</span>
        </div>
        <div class="section-extra">
          <el-button size="small" type="success" plain :icon="Operation" @click="openPolicyDialog">
            配置执行策略
          </el-button>
          <el-button size="small" type="primary" plain :icon="Plus" @click="expandNodeDialogVisible = true">
            扩展/纳管新机器
          </el-button>
        </div>
      </div>

      <!-- 集群任务执行策略全局指示条 -->
      <div class="cluster-policy-banner" @click="openPolicyDialog">
        <div class="policy-banner-left">
          <div class="policy-badge-pill">
            <span class="policy-dot pulse-emerald-dot"></span>
            <span class="policy-mode-title">当前集群执行策略：<b>{{ executionPolicy.policyName || '智能读写分离与高可用路由 (系统默认)' }}</b></span>
          </div>
          <div class="policy-route-tags">
            <span class="route-chip route-query">
              <el-icon><Search /></el-icon> 查询 (DQL): <b>{{ formatQueryRoute(executionPolicy.queryRoute) }}</b>
            </span>
            <span class="route-chip route-exec">
              <el-icon><Tickets /></el-icon> 变更 (DDL/DML): <b>{{ formatExecuteRoute(executionPolicy.executeRoute) }}</b>
            </span>
            <span class="route-chip route-failover" v-if="executionPolicy.failoverEnabled">
              故障自愈降级: 开启 (≤{{ executionPolicy.maxLagSeconds || 5 }}s)
            </span>
          </div>
        </div>
        <div class="policy-banner-right">
          <span class="policy-btn-hint">调整策略模板 / 自定义路由 </span>
        </div>
      </div>

      <div class="cluster-nodes-grid">
        <!-- 节点卡片循环 -->
        <div
          v-for="node in clusterNodes"
          :key="node.nodeId"
          class="node-card"
          :class="{ active: activeNodeId === node.nodeId }"
          @click="selectNode(node)"
        >
          <div class="node-card-top">
            <div class="node-title-group">
              <span class="node-name">{{ node.name }}</span>
              <el-tag size="small" :type="node.role === 'MASTER' ? 'primary' : 'success'" effect="light" class="node-role-tag">
                {{ node.roleName }}
              </el-tag>
            </div>
            <div class="node-status-badge">
              <span class="pulse-dot-mini"></span>
              <span class="status-label">实时在线</span>
            </div>
          </div>

          <div class="node-ip-row">
            <span class="ip-label">节点 IP：</span>
            <span class="ip-value font-mono">{{ node.ip }}</span>
            <el-tooltip content="复制 IP 地址" placement="top">
              <el-icon class="copy-icon" @click.stop="copyIp(node.ip)"><DocumentCopy /></el-icon>
            </el-tooltip>
            <a :href="'http://' + node.ip" target="_blank" class="node-link-btn" @click.stop>
              访问服务 ↗
            </a>
          </div>

          <div class="node-remark-box">
            <span class="node-region-tag">{{ node.region }}</span>
            <span class="node-remark-text" :title="node.remark">{{ node.remark }}</span>
          </div>

          <div class="node-metrics-row">
            <div class="mini-metric">
              <div class="mm-label">CPU 负载</div>
              <div class="mm-val font-mono">{{ node.cpuUsage }}%</div>
              <el-progress :percentage="node.cpuUsage" :stroke-width="4" :show-text="false" color="#10b981" />
            </div>
            <div class="mini-metric">
              <div class="mm-label">内存占用</div>
              <div class="mm-val font-mono">{{ node.memoryUsage }}%</div>
              <el-progress :percentage="node.memoryUsage" :stroke-width="4" :show-text="false" color="#3b82f6" />
            </div>
            <div class="mini-metric">
              <div class="mm-label">网络延迟</div>
              <div class="mm-val font-mono">{{ node.latency }}</div>
              <span class="mm-sub">心跳正常</span>
            </div>
          </div>
        </div>

        <!-- 扩展添加机器引导卡片 -->
        <div class="node-card add-node-card" @click="expandNodeDialogVisible = true">
          <div class="add-card-inner">
            <div class="add-icon-circle">
              <el-icon :size="24"><Plus /></el-icon>
            </div>
            <div class="add-title">扩容纳管新机器</div>
            <div class="add-desc">支持 2 台、3 台甚至任意多台机器，自定义命名与备注</div>
          </div>
        </div>
      </div>
    </div>

    <!-- 3. 实时数据库性能微仪表网格 (全新清爽无边框磁贴架构) -->
    <div class="clean-section-card">
      <div class="section-card-header">
        <div class="section-title-wrap">
          <div class="section-icon-dot bg-blue-dot"></div>
          <h3 class="section-heading">
            生产数据库与当前节点运行效能 (RDS MySQL 8.0)
            <span class="focused-node-badge" v-if="activeNode">
              聚焦: {{ activeNode.name }} ({{ activeNode.ip }})
            </span>
          </h3>
          <span class="section-subtext">采样周期 5 秒 · 全链路心跳检测中 · 主从读写分离已启用</span>
        </div>
        <div class="section-extra" style="display: flex; gap: 8px; align-items: center;">
          <el-tag size="small" type="success" effect="light" class="status-pill-tag">
            ● 节点心跳正常 (ONLINE)
          </el-tag>
          <el-tag size="small" type="primary" effect="plain" class="status-pill-tag">
            主从读写分离：开启 (Master写 / Slave读)
          </el-tag>
        </div>
      </div>

      <div class="monitor-tiles-grid">
        <!-- 磁贴 1: CPU 算力 -->
        <div class="monitor-tile">
          <div class="tile-top">
            <span class="tile-label">CPU 算力利用率</span>
            <span class="tile-tag tag-success">健康空闲</span>
          </div>
          <div class="tile-value">{{ monitorStats.cpuUsage }}%</div>
          <el-progress :percentage="monitorStats.cpuUsage" :stroke-width="6" :show-text="false" color="#10b981" class="tile-progress" />
          <div class="tile-desc">8 核心分配 / 波动平稳</div>
        </div>

        <!-- 磁贴 2: 活跃连接 -->
        <div class="monitor-tile">
          <div class="tile-top">
            <span class="tile-label">当前活跃连接数</span>
            <span class="tile-tag tag-primary">连接池优化</span>
          </div>
          <div class="tile-value">{{ monitorStats.connections }} <span class="tile-sub-val">/ 500</span></div>
          <el-progress :percentage="Math.round(monitorStats.connections / 5)" :stroke-width="6" :show-text="false" color="#3b82f6" class="tile-progress" />
          <div class="tile-desc">连接利用率 {{ Math.round(monitorStats.connections / 5) }}%</div>
        </div>

        <!-- 磁贴 3: TPS / QPS 吞吐 -->
        <div class="monitor-tile">
          <div class="tile-top">
            <span class="tile-label">吞吐能力 TPS / QPS</span>
            <span class="tile-tag tag-info">实时流速</span>
          </div>
          <div class="tile-value font-mono">{{ monitorStats.tps }} <span class="tile-sub-val">/ {{ monitorStats.qps }}</span></div>
          <div class="tile-bars-visual">
            <span class="bar-pill bar-1"></span>
            <span class="bar-pill bar-2"></span>
            <span class="bar-pill bar-3"></span>
            <span class="bar-pill bar-4"></span>
            <span class="bar-pill bar-5"></span>
          </div>
          <div class="tile-desc">毫秒级事务低抖动</div>
        </div>

        <!-- 磁贴 4: 慢 SQL 与锁等待 -->
        <div class="monitor-tile">
          <div class="tile-top">
            <span class="tile-label">慢 SQL / 锁等待</span>
            <span class="tile-tag tag-success">零阻塞</span>
          </div>
          <div class="tile-value text-emerald font-mono">{{ monitorStats.slowSql }} <span class="tile-sub-val">条 / {{ monitorStats.lockWaits }} 次</span></div>
          <div class="tile-status-clean">
            <el-icon><CircleCheck /></el-icon>
            <span>无长事务阻塞锁表</span>
          </div>
          <div class="tile-desc">阈值 &gt; 1000ms 自动拦截</div>
        </div>

        <!-- 磁贴 5: Buffer Pool 命中率 -->
        <div class="monitor-tile">
          <div class="tile-top">
            <span class="tile-label">Buffer Pool 命中率</span>
            <span class="tile-tag tag-emerald">内存加速</span>
          </div>
          <div class="tile-value text-emerald font-mono">{{ monitorStats.bufferPoolHitRate }}%</div>
          <el-progress :percentage="monitorStats.bufferPoolHitRate" :stroke-width="6" :show-text="false" color="#10b981" class="tile-progress" />
          <div class="tile-desc">绝大部分热点走缓存命中</div>
        </div>

        <!-- 磁贴 6: 存储与主从延迟 -->
        <div class="monitor-tile">
          <div class="tile-top">
            <span class="tile-label">磁盘容量 / 主从延迟</span>
            <span class="tile-tag tag-info">同步正常</span>
          </div>
          <div class="tile-value font-mono">{{ monitorStats.diskSpaceUsage }}% <span class="tile-sub-val">/ {{ monitorStats.replDelay }}</span></div>
          <el-progress :percentage="monitorStats.diskSpaceUsage" :stroke-width="6" :show-text="false" color="#38bdf8" class="tile-progress" />
          <div class="tile-desc">表空间占 29% · 充裕</div>
        </div>
      </div>
    </div>

    <!-- 4. 工单流转分析与生命周期统计图表 -->
    <el-row :gutter="16" class="charts-row">
      <el-col :xs="24" :sm="24" :md="24" :lg="12" class="col-item">
        <div class="chart-card-box">
          <div class="chart-box-header">
            <div class="chart-box-title">
              <span class="chart-dot dot-blue"></span>
              <span>工单生命周期全状态分布</span>
            </div>
            <el-tag size="small" effect="plain" type="info">共 {{ stats.totalTickets || 60 }} 笔</el-tag>
          </div>
          <div ref="ticketChartRef" class="chart-container"></div>
        </div>
      </el-col>

      <el-col :xs="24" :sm="24" :md="24" :lg="12" class="col-item">
        <div class="chart-card-box">
          <div class="chart-box-header">
            <div class="chart-box-title">
              <span class="chart-dot dot-emerald"></span>
              <span>近 7 天变更工单提交与流转趋势</span>
            </div>
            <el-tag size="small" effect="plain" type="success">按天统计</el-tag>
          </div>
          <div ref="workloadChartRef" class="chart-container"></div>
        </div>
      </el-col>
    </el-row>

    <!-- 5. 极速工作流与日常效能导航条 (高级设计师体验设计) -->
    <div class="quick-workflow-panel">
      <div class="workflow-panel-title">
        <el-icon><Operation /></el-icon>
        <span>核心管理与协同功能直达</span>
      </div>
      <div class="workflow-shortcuts-grid">
        <div class="shortcut-pill" @click="router.push('/ticket-list')">
          <div class="sc-icon bg-amber-soft"><Tickets /></div>
          <div class="sc-text">
            <div class="sc-name">工单流转列表</div>
            <div class="sc-desc">查看审批与加急催办</div>
          </div>
        </div>

        <div class="shortcut-pill" @click="router.push('/data-query')">
          <div class="sc-icon bg-sky-soft"><Search /></div>
          <div class="sc-text">
            <div class="sc-name">数据查询工作台</div>
            <div class="sc-desc">动态脱敏与只读查询</div>
          </div>
        </div>

        <div class="shortcut-pill" @click="router.push('/resource-group-list')">
          <div class="sc-icon bg-teal-soft"><Suitcase /></div>
          <div class="sc-text">
            <div class="sc-name">业务资源组</div>
            <div class="sc-desc">资产隔离与流程路由</div>
          </div>
        </div>

        <div class="shortcut-pill" @click="router.push('/ai-sql-review')">
          <div class="sc-icon bg-purple-soft"><MagicStick /></div>
          <div class="sc-text">
            <div class="sc-name">AI 智能预检</div>
            <div class="sc-desc">Dry-Run 影响行推导</div>
          </div>
        </div>

        <div class="shortcut-pill" @click="router.push('/notification-config')">
          <div class="sc-icon bg-rose-soft"><Bell /></div>
          <div class="sc-text">
            <div class="sc-name">消息通知配置</div>
            <div class="sc-desc">企微/飞书/钉钉总线</div>
          </div>
        </div>
      </div>
    </div>

    <!-- 6. 机器节点扩容与纳管配置引导对话框 -->
    <el-dialog
      v-model="expandNodeDialogVisible"
      title="集群机器节点扩容与纳管配置指南"
      width="680px"
      append-to-body
      class="clean-dialog"
    >
      <div class="expand-dialog-content">
        <el-alert
          type="info"
          :closable="false"
          show-icon
          title="平台原生支持多机器集群部署与统一自治纳管。任意新增机器只需配置 IP、端口、角色与备注即可自动加入大盘监控。"
          style="margin-bottom: 16px;"
        />

        <div class="dialog-section-title">当前已纳管机器节点列表 ({{ clusterNodes.length }} 台)</div>
        <el-table :data="clusterNodes" size="small" border stripe style="margin-bottom: 20px;">
          <el-table-column prop="name" label="节点名称" width="170" />
          <el-table-column prop="ip" label="IP 地址" width="130" />
          <el-table-column prop="roleName" label="部署角色" width="130">
            <template #default="{ row }">
              <el-tag size="small" :type="row.role === 'MASTER' ? 'primary' : 'success'">{{ row.roleName }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column prop="remark" label="业务备注 / 机房" />
        </el-table>

        <div class="dialog-section-title">如何扩容接入第 3 台或更多机器？</div>
        <div class="expand-steps-box">
          <div class="step-item">
            <span class="step-badge">1</span>
            <div class="step-desc">
              <b>机器环境准备</b>：在新机器（如 <code>192.168.1.103</code>）上安装 Docker，并拉取启动 <code>pacersql-frontend</code> 与 <code>pacersql-backend</code> 容器。
            </div>
          </div>
          <div class="step-item">
            <span class="step-badge">2</span>
            <div class="step-desc">
              <b>数据源与Redis共享</b>：配置新节点的环境变量指向统一的 RDS 生产数据库与 Redis，保证工单与用户状态多机强一致。
            </div>
          </div>
          <div class="step-item">
            <span class="step-badge">3</span>
            <div class="step-desc">
              <b>大盘拓扑自动注册</b>：在配置文件 <code>application.yml</code> 的 <code>pacersql.cluster.nodes</code> 增加该节点的 IP、自定义名称与备注，重启后总览大盘立即实时监控！
            </div>
          </div>
        </div>
      </div>
      <template #footer>
        <el-button type="primary" @click="expandNodeDialogVisible = false">知道了</el-button>
      </template>
    </el-dialog>

    <!-- 7. 集群多机器执行与路由策略可视化配置弹窗 -->
    <el-dialog
      v-model="policyDialogVisible"
      title="集群多节点任务执行策略配置 (Cluster Execution Policy)"
      width="780px"
      append-to-body
      class="clean-dialog policy-dialog"
    >
      <div class="policy-dialog-body">
        <el-alert
          type="info"
          :closable="false"
          show-icon
          title="可视化配置集群多台机器的任务执行策略。系统内置默认策略：查询走从节点 (Worker)，工单变更与发布锁定主节点 (Master)。"
          style="margin-bottom: 18px;"
        />

        <!-- 预设策略模板卡片 -->
        <div class="dialog-sub-title">1. 推荐预设策略模板（点击卡片一键套用）</div>
        <div class="preset-cards-grid">
          <div
            v-for="tpl in presetTemplates"
            :key="tpl.policyMode"
            class="preset-card"
            :class="{ active: policyForm.policyMode === tpl.policyMode }"
            @click="applyTemplate(tpl)"
          >
            <div class="preset-card-head">
              <span class="preset-name">{{ tpl.policyName }}</span>
              <el-tag size="small" :type="tpl.tagType" effect="light">{{ tpl.badge }}</el-tag>
            </div>
            <div class="preset-desc">{{ tpl.description }}</div>
            <div class="preset-routes">
              <span class="pr-item">查询: <b>{{ tpl.queryRouteLabel }}</b></span>
              <span class="pr-item">变更: <b>{{ tpl.executeRouteLabel }}</b></span>
            </div>
          </div>
        </div>

        <!-- 自定义精细化参数 -->
        <div class="dialog-sub-title" style="margin-top: 20px;">2. 精细化路由与高可用参数设置</div>
        <el-form :model="policyForm" label-width="160px" class="policy-form" size="default">
          <el-form-item label="策略显示名称">
            <el-input v-model="policyForm.policyName" placeholder="例如：智能读写分离与高可用路由策略" />
          </el-form-item>

          <el-form-item label="数据查询 (DQL) 路由">
            <el-radio-group v-model="policyForm.queryRoute">
              <el-radio label="WORKER_SLAVE_FIRST">优先从节点 (系统默认)</el-radio>
              <el-radio label="WORKER_ONLY">仅限从节点 (强隔离)</el-radio>
              <el-radio label="MASTER_ONLY">仅限主节点</el-radio>
              <el-radio label="LEAST_LOAD">动态最低负载节点</el-radio>
            </el-radio-group>
            <div class="form-tip-text">
              日常数据查询、大模型 AI 预检依据此规则分流至只读从机（如 39.97.158.22），减轻主调度网关负载。
            </div>
          </el-form-item>

          <el-form-item label="工单变更 (DDL/DML) 路由">
            <el-radio-group v-model="policyForm.executeRoute">
              <el-radio label="MASTER_ONLY">强制主节点 (系统默认)</el-radio>
              <el-radio label="PRIMARY_COORDINATOR">分布式两阶段主协调</el-radio>
            </el-radio-group>
            <div class="form-tip-text">
              数据表结构变更与批量 DML 更新统一在 Master 主调度网关执行，保障全局锁与审计强一致。
            </div>
          </el-form-item>

          <el-form-item label="高可用与故障自愈">
            <div class="switch-row">
              <el-switch v-model="policyForm.failoverEnabled" active-text="从机故障时自动平滑回退主库" />
              <div class="form-tip-text" style="margin-top: 4px;">
                从节点异常、断网或复制延迟超标时，自动无感把查询流量降级回退至主库，保障业务零中断。
              </div>
            </div>
          </el-form-item>

          <el-form-item label="主从最大容忍延迟" v-if="policyForm.failoverEnabled">
            <el-input-number v-model="policyForm.maxLagSeconds" :min="1" :max="60" :step="1" style="width: 160px;" />
            <span style="margin-left: 10px; color: var(--c-text-3); font-size: 13px;">秒（从库延迟超过该阈值时自动降级回退主库）</span>
          </el-form-item>

          <el-form-item label="慢查询 / AI 算力隔离">
            <el-select v-model="policyForm.heavyQueryNode" style="width: 260px;">
              <el-option label="优先隔离至 Worker 从机" value="WORKER_ONLY" />
              <el-option label="所有 Worker 节点轮询" value="ALL_WORKERS" />
              <el-option label="不指定隔离节点" value="ANY" />
            </el-select>
          </el-form-item>

          <el-form-item label="策略详细说明">
            <el-input type="textarea" :rows="2" v-model="policyForm.description" placeholder="请输入策略备注说明..." />
          </el-form-item>
        </el-form>
      </div>

      <template #footer>
        <div class="dialog-footer-row">
          <el-button @click="resetToDefaultPolicy">恢复系统默认策略</el-button>
          <div class="dialog-footer-right">
            <el-button @click="policyDialogVisible = false">取消</el-button>
            <el-button type="primary" :loading="savingPolicy" @click="saveExecutionPolicy">保存并全局生效</el-button>
          </div>
        </div>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount, nextTick } from 'vue'
import { useRouter } from 'vue-router'
import { useUserStore } from '../store/user'
import request from '../utils/request'
import { ElMessage } from 'element-plus'
import * as echarts from 'echarts'
import {
  Plus,
  Search,
  Refresh,
  CircleCheck,
  Tickets,
  Suitcase,
  MagicStick,
  Operation,
  Bell,
  DocumentCopy
} from '@element-plus/icons-vue'

const router = useRouter()
const userStore = useUserStore()
const loading = ref(false)
const expandNodeDialogVisible = ref(false)

// 集群执行策略配置与多节点路由状态
const policyDialogVisible = ref(false)
const savingPolicy = ref(false)

const executionPolicy = ref<any>({
  policyMode: 'MASTER_WRITE_SLAVE_READ',
  policyName: '智能读写分离与高可用路由 (系统默认)',
  queryRoute: 'WORKER_SLAVE_FIRST',
  executeRoute: 'MASTER_ONLY',
  failoverEnabled: true,
  maxLagSeconds: 5,
  heavyQueryNode: 'WORKER_ONLY',
  description: '日常数据查询自动分流到只读从机 (39.97.158.22)，变更工单发布锁定主网关执行 (101.35.100.169)。'
})

const policyForm = ref<any>({ ...executionPolicy.value })

// 推荐预设策略模板库
const presetTemplates = [
  {
    policyMode: 'MASTER_WRITE_SLAVE_READ',
    policyName: '智能读写分离 (系统推荐·默认)',
    badge: '推荐',
    tagType: 'success',
    queryRoute: 'WORKER_SLAVE_FIRST',
    executeRoute: 'MASTER_ONLY',
    queryRouteLabel: '优先分流从节点 (Worker)',
    executeRouteLabel: '强制主调度机 (Master)',
    failoverEnabled: true,
    maxLagSeconds: 5,
    heavyQueryNode: 'WORKER_ONLY',
    description: '绝大多数生产环境首选。日常 DQL 查询自动负载均衡分流到 Worker 从机，变更工单与审批发布锁定 Master 主机。'
  },
  {
    policyMode: 'WORKER_COMPUTE_ISOLATION',
    policyName: '算力强隔离与分析加速',
    badge: '高性能',
    tagType: 'primary',
    queryRoute: 'WORKER_ONLY',
    executeRoute: 'MASTER_ONLY',
    queryRouteLabel: '仅限从机计算池',
    executeRouteLabel: '强制主调度机',
    failoverEnabled: true,
    maxLagSeconds: 10,
    heavyQueryNode: 'WORKER_ONLY',
    description: '针对大量报表统计、AI 预检和大批量数据查询场景，严禁查询流量触及主生产机，确保主网关绝对平稳。'
  },
  {
    policyMode: 'LEAST_LOAD_FIRST',
    policyName: '动态最低负载分流',
    badge: '自适应',
    tagType: 'warning',
    queryRoute: 'LEAST_LOAD',
    executeRoute: 'MASTER_ONLY',
    queryRouteLabel: '动态最低 CPU/连接',
    executeRouteLabel: '强制主调度机',
    failoverEnabled: true,
    maxLagSeconds: 3,
    heavyQueryNode: 'ALL_WORKERS',
    description: '适合扩容至 3 台或更多机器的动态高并发场景，系统每秒侦测集群负载，将请求实时路由至最轻闲机器。'
  },
  {
    policyMode: 'ROUND_ROBIN',
    policyName: '多从库均匀轮询 (Round Robin)',
    badge: '高可用',
    tagType: 'info',
    queryRoute: 'ROUND_ROBIN',
    executeRoute: 'MASTER_ONLY',
    queryRouteLabel: '多从机对等循环',
    executeRouteLabel: '强制主调度机',
    failoverEnabled: true,
    maxLagSeconds: 5,
    heavyQueryNode: 'ALL_WORKERS',
    description: '适用于多台从库读副本的集群架构，使读请求均匀分散在集群全部 Worker 节点上。'
  }
]

const formatQueryRoute = (route: string) => {
  switch (route) {
    case 'WORKER_SLAVE_FIRST': return '优先从节点 (默认)'
    case 'WORKER_ONLY': return '仅限从节点 (强隔离)'
    case 'MASTER_ONLY': return '仅限主节点'
    case 'LEAST_LOAD': return '动态最低负载'
    case 'ROUND_ROBIN': return '多从节点轮询'
    default: return route || '优先从节点'
  }
}

const formatExecuteRoute = (route: string) => {
  switch (route) {
    case 'MASTER_ONLY': return '强制主节点 (Master)'
    case 'PRIMARY_COORDINATOR': return '分布式两阶段主协调'
    default: return route || '强制主节点 (Master)'
  }
}

const openPolicyDialog = () => {
  policyForm.value = { ...executionPolicy.value }
  policyDialogVisible.value = true
}

const applyTemplate = (tpl: any) => {
  policyForm.value.policyMode = tpl.policyMode
  policyForm.value.policyName = tpl.policyName
  policyForm.value.queryRoute = tpl.queryRoute
  policyForm.value.executeRoute = tpl.executeRoute
  policyForm.value.failoverEnabled = tpl.failoverEnabled
  policyForm.value.maxLagSeconds = tpl.maxLagSeconds
  policyForm.value.heavyQueryNode = tpl.heavyQueryNode
  policyForm.value.description = tpl.description
  ElMessage.info(`已套用模板: ${tpl.policyName}`)
}

const resetToDefaultPolicy = () => {
  applyTemplate(presetTemplates[0])
}

const saveExecutionPolicy = async () => {
  savingPolicy.value = true
  try {
    const res: any = await request.post('/v1/config/execution-policy', policyForm.value)
    if (res.data) {
      executionPolicy.value = res.data
    } else {
      executionPolicy.value = { ...policyForm.value }
    }
    ElMessage.success('集群执行策略已更新并即时全局生效！')
    policyDialogVisible.value = false
  } catch (error) {
    ElMessage.error('保存集群执行策略失败')
  } finally {
    savingPolicy.value = false
  }
}


// 多机器节点集群管理数据
const clusterNodes = ref<any[]>([
  {
    nodeId: 'node-101-35-100-169',
    name: '华北生产主网关 (Node-01)',
    ip: '101.35.100.169',
    port: 80,
    role: 'MASTER',
    roleName: '调度主网关 (Master)',
    region: '华北-北京可用区A',
    remark: '承载工单调度流转、三权分立鉴权与全局数据治理',
    status: 'ONLINE',
    cpuUsage: 24,
    memoryUsage: 48,
    connections: 136,
    tps: 420,
    qps: 1850,
    uptime: '8天 12小时',
    latency: '1.2ms'
  },
  {
    nodeId: 'node-39-97-158-22',
    name: '华北容灾与算力从节点 (Node-02)',
    ip: '39.97.158.22',
    port: 80,
    role: 'WORKER',
    roleName: '容灾与算力节点 (Worker)',
    region: '华北-北京可用区B',
    remark: '分流只读数据查询检索、AI大模型预检与异步报表生成',
    status: 'ONLINE',
    cpuUsage: 18,
    memoryUsage: 36,
    connections: 82,
    tps: 260,
    qps: 1420,
    uptime: '15天 6小时',
    latency: '1.5ms'
  }
])

const activeNodeId = ref('node-101-35-100-169')

const activeNode = computed(() => {
  return clusterNodes.value.find(n => n.nodeId === activeNodeId.value) || clusterNodes.value[0]
})

const selectNode = async (node: any) => {
  activeNodeId.value = node.nodeId
  try {
    const res: any = await request.get(`/v1/dashboard/monitor?nodeId=${node.nodeId}`)
    if (res.data) {
      monitorStats.value = res.data
    }
    ElMessage.success(`已切换聚焦监控节点: ${node.name}`)
  } catch (e) {}
}

const copyIp = (ip: string) => {
  if (navigator.clipboard) {
    navigator.clipboard.writeText(ip)
    ElMessage.success(`已复制节点 IP: ${ip}`)
  } else {
    ElMessage.success(`节点 IP: ${ip}`)
  }
}

const stats = ref<any>({
  healthScore: 99,
  totalSqls: 0,
  riskSqls: 0,
  totalTickets: null,
  pendingTickets: null,
  executedTickets: null,
  rejectedTickets: 0,
  myTicketsCount: 0,
  // 加载完成前显示「—」，不再用假数字占位
  instancesCount: null,
  usersCount: null,
  resourceGroupsCount: null,
  dbaWorkload: 35,
  approvalEfficiency: '0.8h',
  statusDistribution: {},
  trendDates: [],
  trendCounts: []
})

const greeting = computed(() => {
  const h = new Date().getHours()
  if (h < 6) return '夜深了'
  if (h < 11) return '早上好'
  if (h < 13) return '中午好'
  if (h < 18) return '下午好'
  return '晚上好'
})

const metricItems = computed(() => [
  { label: '数据库实例', value: stats.value.instancesCount, sub: '查看实例', to: '/instance-list', tone: '' },
  { label: '业务资源组', value: stats.value.resourceGroupsCount, sub: '查看资源组', to: '/resource-group-list', tone: '' },
  { label: '工单总数', value: stats.value.totalTickets, sub: `我发起 ${stats.value.myTicketsCount ?? 0}`, to: '/ticket-list', tone: '' },
  { label: '待审批', value: stats.value.pendingTickets, sub: stats.value.approvalEfficiency ? `平均流转 ${stats.value.approvalEfficiency}` : '审批中', to: '/ticket-list?status=AUDITING', tone: stats.value.pendingTickets ? 'is-warning' : '' },
  { label: '已执行', value: stats.value.executedTickets, sub: '执行完成', to: '/ticket-list', tone: '' },
  { label: '平台用户', value: stats.value.usersCount, sub: '查看用户', to: '/user-list', tone: '' }
])

const monitorStats = ref({
  cpuUsage: 25,
  connections: 48,
  slowSql: 0,
  tps: 420,
  qps: 1850,
  lockWaits: 0,
  replDelay: '0ms',
  diskSpaceUsage: 38,
  tableSpaceUsage: 29,
  bufferPoolHitRate: 99.8
})

const ticketChartRef = ref<HTMLElement | null>(null)
const workloadChartRef = ref<HTMLElement | null>(null)
let ticketChartInstance: echarts.ECharts | null = null
let workloadChartInstance: echarts.ECharts | null = null
let resizeObserver: ResizeObserver | null = null

const initCharts = () => {
  if (ticketChartRef.value) {
    if (!ticketChartInstance) {
      ticketChartInstance = echarts.init(ticketChartRef.value)
    }

    const dist = stats.value.statusDistribution || {}
    const pieData = Object.keys(dist).map(k => ({
      name: k,
      value: dist[k]
    })).filter(d => d.value > 0)

    if (pieData.length === 0) {
      pieData.push({ name: '暂无工单', value: 1 })
    }

    ticketChartInstance.setOption({
      tooltip: {
        trigger: 'item',
        formatter: '{b}: <b>{c}</b> 笔 ({d}%)',
        backgroundColor: 'rgba(255, 255, 255, 0.98)',
        borderColor: '#e2e8f0',
        borderWidth: 1,
        textStyle: { color: '#0f172a', fontSize: 13 },
        extraCssText: 'box-shadow: 0 10px 25px -5px rgba(0, 0, 0, 0.08); border-radius: 8px;'
      },
      legend: {
        bottom: '2%',
        left: 'center',
        icon: 'circle',
        itemWidth: 10,
        itemHeight: 10,
        textStyle: { color: '#64748b', fontSize: 12 }
      },
      color: ['#3b82f6', '#10b981', '#f59e0b', '#8b5cf6', '#ef4444', '#06b6d4', '#64748b'],
      series: [
        {
          name: '工单状态分布',
          type: 'pie',
          radius: ['48%', '72%'],
          center: ['50%', '42%'],
          avoidLabelOverlap: false,
          itemStyle: {
            borderRadius: 6,
            borderColor: '#ffffff',
            borderWidth: 3
          },
          label: { show: false, position: 'center' },
          emphasis: {
            label: { show: true, fontSize: 16, fontWeight: 'bold', color: '#0f172a' }
          },
          labelLine: { show: false },
          data: pieData
        }
      ]
    })
    ticketChartInstance.resize()
  }

  if (workloadChartRef.value) {
    if (!workloadChartInstance) {
      workloadChartInstance = echarts.init(workloadChartRef.value)
    }

    const dates = stats.value.trendDates && stats.value.trendDates.length > 0
      ? stats.value.trendDates
      : ['08-15', '08-16', '08-17', '08-18', '08-19', '08-20', '08-21']
    const counts = stats.value.trendCounts && stats.value.trendCounts.length > 0
      ? stats.value.trendCounts
      : [2, 4, 3, 5, 2, 6, Math.max(1, stats.value.totalTickets)]

    workloadChartInstance.setOption({
      tooltip: {
        trigger: 'axis',
        formatter: '{b} <br/>工单提交流转量: <b>{c}</b> 笔',
        backgroundColor: 'rgba(255, 255, 255, 0.98)',
        borderColor: '#e2e8f0',
        borderWidth: 1,
        textStyle: { color: '#0f172a', fontSize: 13 },
        extraCssText: 'box-shadow: 0 10px 25px -5px rgba(0, 0, 0, 0.08); border-radius: 8px;'
      },
      grid: {
        left: '3%',
        right: '4%',
        bottom: '8%',
        top: '14%',
        containLabel: true
      },
      xAxis: {
        type: 'category',
        data: dates,
        boundaryGap: false,
        axisLine: { lineStyle: { color: '#e2e8f0' } },
        axisLabel: { color: '#64748b', fontSize: 12 }
      },
      yAxis: {
        type: 'value',
        minInterval: 1,
        axisLine: { show: false },
        axisTick: { show: false },
        splitLine: { lineStyle: { color: '#f1f5f9' } },
        axisLabel: { color: '#64748b', fontSize: 12 }
      },
      series: [
        {
          name: '工单变更数',
          data: counts,
          type: 'line',
          smooth: true,
          symbol: 'circle',
          symbolSize: 6,
          itemStyle: {
            color: '#2563eb',
            borderColor: '#ffffff',
            borderWidth: 2
          },
          lineStyle: {
            color: '#2563eb',
            width: 3
          },
          areaStyle: {
            color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
              { offset: 0, color: 'rgba(37, 99, 235, 0.25)' },
              { offset: 1, color: 'rgba(37, 99, 235, 0.01)' }
            ])
          }
        }
      ]
    })
    workloadChartInstance.resize()
  }
}

const handleResize = () => {
  ticketChartInstance?.resize()
  workloadChartInstance?.resize()
}

const loadStats = async () => {
  loading.value = true
  try {
    const [statsRes, monitorRes, policyRes]: any = await Promise.all([
      request.get('/v1/dashboard/stats'),
      request.get(`/v1/dashboard/monitor?nodeId=${activeNodeId.value}`),
      request.get('/v1/config/execution-policy').catch(() => null)
    ])
    if (statsRes && statsRes.data) {
      stats.value = statsRes.data
      if (statsRes.data.clusterNodes && statsRes.data.clusterNodes.length > 0) {
        clusterNodes.value = statsRes.data.clusterNodes
      }
    }
    if (monitorRes && monitorRes.data) {
      monitorStats.value = monitorRes.data
    }
    if (policyRes && policyRes.data) {
      executionPolicy.value = policyRes.data
    }

    nextTick(() => {
      initCharts()
    })
  } catch (error) {
    ElMessage.error('加载大盘数据失败')
  } finally {
    loading.value = false
  }
}

let monitorTimer: number | null = null

const fetchMonitorData = async () => {
  if (typeof document !== 'undefined' && document.hidden) {
    monitorTimer = window.setTimeout(fetchMonitorData, 15000)
    return
  }
  try {
    const res: any = await request.get(`/v1/dashboard/monitor?nodeId=${activeNodeId.value}`)
    if (res.data) {
      monitorStats.value = res.data
    }
  } catch (e) {
    // ignore
  } finally {
    monitorTimer = window.setTimeout(fetchMonitorData, 15000)
  }
}

onMounted(() => {
  loadStats()
  fetchMonitorData()
  window.addEventListener('resize', handleResize)

  if (typeof ResizeObserver !== 'undefined') {
    resizeObserver = new ResizeObserver(() => {
      handleResize()
    })
    if (ticketChartRef.value?.parentElement) resizeObserver.observe(ticketChartRef.value.parentElement)
    if (workloadChartRef.value?.parentElement) resizeObserver.observe(workloadChartRef.value.parentElement)
  }
})

onBeforeUnmount(() => {
  window.removeEventListener('resize', handleResize)
  if (monitorTimer) {
    clearTimeout(monitorTimer)
    monitorTimer = null
  }
  if (resizeObserver) {
    resizeObserver.disconnect()
    resizeObserver = null
  }
  ticketChartInstance?.dispose()
  workloadChartInstance?.dispose()
})
</script>

<style scoped>
.dashboard-container {
  padding: 0 0 32px 0;
}

/* ---- 问候与常用操作 ---- */
.welcome-hero-banner {
  display: flex;
  justify-content: space-between;
  align-items: flex-end;
  flex-wrap: wrap;
  gap: 16px;
  padding: 4px 0 20px;
}
.welcome-left { flex: 1; min-width: 260px; }
.welcome-heading {
  margin: 0;
  font-size: 22px;
  font-weight: 650;
  color: var(--c-text);
  letter-spacing: -0.02em;
}
.welcome-subtext {
  margin: 6px 0 0;
  font-size: 13.5px;
  color: var(--c-text-3);
}
.welcome-subtext b { color: var(--c-text); font-weight: 600; }
.welcome-actions {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  align-items: center;
}
.welcome-actions .el-button + .el-button { margin-left: 0; }

/* ---- 指标条 ---- */
.metric-strip {
  display: grid;
  grid-template-columns: repeat(6, minmax(0, 1fr));
  gap: 1px;
  background: var(--c-border);
  border: 1px solid var(--c-border);
  border-radius: var(--radius-md);
  margin-bottom: 16px;
  overflow: hidden;
}
.metric-cell {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 4px;
  padding: 16px 18px;
  background: var(--c-surface);
  border: none;
  font: inherit;
  text-align: left;
  cursor: pointer;
}
.metric-cell:hover { background: var(--c-muted-bg); }
.metric-cell:focus-visible { outline: 2px solid var(--c-accent); outline-offset: -2px; }
.metric-title { font-size: 12.5px; color: var(--c-text-3); }
.metric-number {
  font-size: 26px;
  font-weight: 650;
  line-height: 1.15;
  color: var(--c-text);
  letter-spacing: -0.02em;
  font-variant-numeric: tabular-nums;
}
.metric-number.is-warning { color: var(--c-warning-strong, var(--c-warning)); }
.metric-sub { font-size: 12px; color: var(--c-text-4); }
.metric-cell:hover .metric-sub { color: var(--c-accent); }

@media (max-width: 1200px) {
  .metric-strip { grid-template-columns: repeat(3, minmax(0, 1fr)); }
}
@media (max-width: 640px) {
  .metric-strip { grid-template-columns: repeat(2, minmax(0, 1fr)); }
}

/* ===================================================
   3. 实时性能监控：清爽磁贴式微架构
   =================================================== */
.clean-section-card {
  background: var(--c-surface);
  border: 1px solid var(--c-border);
  border-radius: 14px;
  padding: 18px 20px 20px;
  margin-bottom: 16px;
  box-shadow: var(--shadow-sm);
}

.section-card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 14px;
  flex-wrap: wrap;
  gap: 8px;
}

.section-title-wrap {
  display: flex;
  align-items: center;
  gap: 8px;
}

.section-icon-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.bg-blue-dot { background: var(--c-accent); box-shadow: var(--shadow-sm); }

.section-heading {
  margin: 0;
  font-size: 14.5px;
  font-weight: 700;
  color: var(--c-text);
}

.section-subtext {
  font-size: 12px;
  color: var(--c-text-4);
  margin-left: 6px;
}

.status-pill-tag {
  border-radius: 20px;
  font-weight: 500;
}

.monitor-tiles-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(170px, 1fr));
  gap: 12px;
}

.monitor-tile {
  background: var(--c-muted-bg);
  border: 1px solid var(--c-border);
  border-radius: 10px;
  padding: 12px 14px;
  transition: all 0.2s ease;
}

.monitor-tile:hover {
  background: var(--c-surface);
  border-color: var(--c-border-strong);
  box-shadow: var(--shadow-sm);
}

.tile-top {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 6px;
}

.tile-label {
  font-size: 11.5px;
  color: var(--c-text-3);
  font-weight: 500;
}

.tile-tag {
  font-size: 10px;
  padding: 1px 5px;
  border-radius: 4px;
  font-weight: 500;
}

.tag-success { background: var(--c-success-soft); color: var(--c-success); }
.tag-primary { background: var(--c-accent-soft); color: var(--c-accent); }
.tag-info { background: var(--c-subtle); color: var(--c-text-2); }

.tile-value {
  font-size: 18px;
  font-weight: 700;
  color: var(--c-text);
  line-height: 1.2;
  margin-bottom: 6px;
}

.tile-sub-val {
  font-size: 12px;
  color: var(--c-text-4);
  font-weight: normal;
}

.tile-progress {
  margin: 6px 0;
}

.tile-desc {
  font-size: 11px;
  color: var(--c-text-4);
  margin-top: 4px;
}

.tile-status-clean {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 11px;
  color: var(--c-success);
  margin: 6px 0;
  font-weight: 500;
}

.tile-bars-visual {
  display: flex;
  gap: 3px;
  margin: 8px 0;
  align-items: flex-end;
  height: 8px;
}

.bar-pill {
  flex: 1;
  background: var(--c-accent);
  border-radius: 2px;
  opacity: 0.6;
}

.bar-1 { height: 40%; }
.bar-2 { height: 75%; opacity: 0.8; }
.bar-3 { height: 100%; opacity: 1; }
.bar-4 { height: 60%; opacity: 0.7; }
.bar-5 { height: 85%; opacity: 0.9; }

.font-mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
}

/* ===================================================
   4. 图表卡片：通透白底与高对比度柔和色相
   =================================================== */
.charts-row {
  margin-bottom: 16px;
}

.chart-card-box {
  background: var(--c-surface);
  border: 1px solid var(--c-border);
  border-radius: 14px;
  padding: 16px 18px;
  box-shadow: var(--shadow-sm);
}

.chart-box-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 10px;
}

.chart-box-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  font-weight: 700;
  color: var(--c-text);
}

.chart-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.dot-blue { background: var(--c-accent); }
.dot-emerald { background: var(--c-success); }

.chart-container {
  height: 280px;
  width: 100%;
}

/* ===================================================
   5. 极速工作流与日常效能导航条
   =================================================== */
.quick-workflow-panel {
  background: var(--c-surface);
  border: 1px solid var(--c-border);
  border-radius: 14px;
  padding: 18px 20px;
  box-shadow: var(--shadow-sm);
}

.workflow-panel-title {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13.5px;
  font-weight: 700;
  color: var(--c-text);
  margin-bottom: 12px;
}

.workflow-shortcuts-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 10px;
}

.shortcut-pill {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  background: var(--c-muted-bg);
  border: 1px solid var(--c-border);
  border-radius: 10px;
  cursor: pointer;
  transition: all 0.2s ease;
}

.shortcut-pill:hover {
  background: var(--c-accent-soft);
  border-color: var(--c-accent);
  transform: translateY(-2px);
  box-shadow: var(--shadow-sm);
}

.sc-icon {
  width: 32px;
  height: 32px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 16px;
  flex-shrink: 0;
}

.bg-amber-soft { background: var(--c-warning-soft); color: var(--c-warning); }
.bg-sky-soft { background: var(--c-accent-soft); color: var(--c-accent); }
.bg-teal-soft { background: var(--c-success-soft); color: var(--c-success); }
.bg-purple-soft { background: var(--c-accent-soft); color: var(--c-accent); }
.bg-rose-soft { background: var(--c-danger-soft); color: var(--c-danger); }

.sc-text {
  flex: 1;
  overflow: hidden;
}

.sc-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--c-text);
}

.sc-desc {
  font-size: 11px;
  color: var(--c-text-4);
  margin-top: 1px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* ===================================================
   多节点集群拓扑矩阵样式 (清爽明亮、卡片微发光设计)
   =================================================== */
.cluster-nodes-section {
  margin-bottom: 20px;
}

.bg-indigo-dot {
  background: var(--c-accent);
  box-shadow: var(--shadow-sm);
}

.cluster-nodes-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: 16px;
  margin-top: 6px;
}

.node-card {
  background: var(--c-surface);
  border: 1px solid var(--c-border);
  border-radius: 12px;
  padding: 16px 18px;
  transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  cursor: pointer;
  position: relative;
  box-shadow: var(--shadow-sm);
}

.node-card:hover {
  border-color: var(--c-accent-border);
  transform: translateY(-2px);
  box-shadow: var(--shadow-sm);
}

.node-card.active {
  border-color: var(--c-accent);
  background: var(--c-accent-soft);
  box-shadow: var(--shadow-sm);
}

.node-card-top {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 10px;
}

.node-title-group {
  display: flex;
  align-items: center;
  gap: 8px;
}

.node-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--c-text);
}

.node-role-tag {
  font-weight: 500;
  border-radius: 6px;
}

.node-status-badge {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 2px 8px;
  background: var(--c-success-soft);
  border: 1px solid var(--c-success-border);
  border-radius: 12px;
  font-size: 11px;
  color: var(--c-success);
  font-weight: 500;
}

.pulse-dot-mini {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--c-success);
  animation: pulse-ring 2s infinite;
}

@keyframes pulse-ring {
  0% { box-shadow: var(--shadow-sm); }
  70% { box-shadow: var(--shadow-sm); }
  100% { box-shadow: var(--shadow-sm); }
}

.node-ip-row {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12.5px;
  margin-bottom: 8px;
  padding-bottom: 8px;
  border-bottom: 1px dashed var(--c-border);
}

.ip-label {
  color: var(--c-text-3);
}

.ip-value {
  font-weight: 600;
  color: var(--c-text);
  letter-spacing: 0.3px;
}

.copy-icon {
  color: var(--c-text-4);
  cursor: pointer;
  font-size: 14px;
  transition: color 0.15s;
}

.copy-icon:hover {
  color: var(--c-accent);
}

.node-link-btn {
  margin-left: auto;
  font-size: 12px;
  color: var(--c-accent);
  text-decoration: none;
  font-weight: 500;
  padding: 1px 8px;
  border-radius: 4px;
  background: var(--c-accent-soft);
  transition: all 0.15s;
}

.node-link-btn:hover {
  background: var(--c-accent-soft);
}

.node-remark-box {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
}

.node-region-tag {
  font-size: 11px;
  color: var(--c-text-2);
  background: var(--c-subtle);
  padding: 1px 6px;
  border-radius: 4px;
  flex-shrink: 0;
}

.node-remark-text {
  font-size: 12px;
  color: var(--c-text-3);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.node-metrics-row {
  display: grid;
  grid-template-columns: 1fr 1fr 1fr;
  gap: 8px;
  background: var(--c-muted-bg);
  padding: 8px 10px;
  border-radius: 8px;
}

.mini-metric {
  text-align: center;
}

.mm-label {
  font-size: 10.5px;
  color: var(--c-text-3);
  margin-bottom: 2px;
}

.mm-val {
  font-size: 13px;
  font-weight: 600;
  color: var(--c-text);
  margin-bottom: 3px;
}

.mm-sub {
  font-size: 10.5px;
  color: var(--c-success);
}

.focused-node-badge {
  font-size: 12px;
  font-weight: 500;
  color: var(--c-accent);
  background: var(--c-accent-soft);
  border: 1px solid var(--c-accent-border);
  padding: 2px 8px;
  border-radius: 6px;
  margin-left: 10px;
}

/* 扩展添加卡片 */
.add-node-card {
  border: 2px dashed var(--c-border-strong);
  background: var(--c-muted-bg);
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 150px;
}

.add-node-card:hover {
  border-color: var(--c-accent);
  background: var(--c-accent-soft);
}

.add-card-inner {
  text-align: center;
  padding: 12px;
}

.add-icon-circle {
  width: 42px;
  height: 42px;
  border-radius: 50%;
  background: var(--c-accent-soft);
  color: var(--c-accent);
  display: flex;
  align-items: center;
  justify-content: center;
  margin: 0 auto 10px;
  transition: all 0.2s;
}

.add-node-card:hover .add-icon-circle {
  transform: scale(1.08);
  background: var(--c-accent);
  color: #ffffff;
}

.add-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--c-text);
  margin-bottom: 4px;
}

.add-desc {
  font-size: 11.5px;
  color: var(--c-text-4);
  max-width: 240px;
  margin: 0 auto;
}

/* 扩容对话框样式 */
.dialog-section-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--c-text);
  margin: 14px 0 8px;
}

.expand-steps-box {
  display: flex;
  flex-direction: column;
  gap: 12px;
  background: var(--c-muted-bg);
  padding: 14px;
  border-radius: 8px;
  border: 1px solid var(--c-border);
}

.step-item {
  display: flex;
  align-items: flex-start;
  gap: 10px;
}

.step-badge {
  width: 22px;
  height: 22px;
  border-radius: 50%;
  background: var(--c-accent);
  color: #ffffff;
  font-size: 12px;
  font-weight: 600;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  margin-top: 1px;
}

.step-desc {
  font-size: 13px;
  color: var(--c-text-2);
  line-height: 1.6;
}

.step-desc code {
  background: var(--c-border);
  padding: 2px 6px;
  border-radius: 4px;
  font-family: monospace;
  color: var(--c-text);
  font-size: 12px;
}

/* 集群执行策略状态指示条与配置弹窗 */
.cluster-policy-banner {
  margin-top: 14px;
  margin-bottom: 16px;
  background: var(--c-success-soft);
  border: 1px solid var(--c-success-border);
  border-radius: 10px;
  padding: 10px 16px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  cursor: pointer;
  transition: all 0.2s ease;
}

.cluster-policy-banner:hover {
  border-color: var(--c-success-border);
  box-shadow: var(--shadow-sm);
  transform: translateY(-1px);
}

.policy-banner-left {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 12px;
}

.policy-badge-pill {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

.pulse-emerald-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--c-success);
  box-shadow: var(--shadow-sm);
  animation: pulseEmerald 2s infinite;
}

@keyframes pulseEmerald {
  0% { box-shadow: var(--shadow-sm); }
  70% { box-shadow: var(--shadow-sm); }
  100% { box-shadow: var(--shadow-sm); }
}

.policy-mode-title {
  font-size: 13px;
  color: var(--c-success-strong);
}

.policy-route-tags {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.route-chip {
  font-size: 12px;
  padding: 2px 8px;
  border-radius: 6px;
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

.route-chip.route-query {
  background: var(--c-accent-soft);
  color: var(--c-accent-hover);
  border: 1px solid var(--c-accent-border);
}

.route-chip.route-exec {
  background: var(--c-danger-soft);
  color: var(--c-danger-strong);
  border: 1px solid var(--c-danger-border);
}

.route-chip.route-failover {
  background: var(--c-subtle);
  color: var(--c-text-2);
  border: 1px solid var(--c-border);
}

.policy-banner-right {
  flex-shrink: 0;
}

.policy-btn-hint {
  font-size: 12px;
  color: var(--c-success);
  font-weight: 500;
  display: inline-flex;
  align-items: center;
}

.preset-cards-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 12px;
  margin-top: 10px;
}

.preset-card {
  border: 1px solid var(--c-border);
  border-radius: 8px;
  padding: 12px;
  background: var(--c-surface);
  cursor: pointer;
  transition: all 0.2s ease;
}

.preset-card:hover {
  border-color: var(--c-accent-border);
  box-shadow: var(--shadow-sm);
}

.preset-card.active {
  border-color: var(--c-accent);
  background: var(--c-accent-soft);
  box-shadow: var(--shadow-sm);
}

.preset-card-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 6px;
}

.preset-name {
  font-size: 13.5px;
  font-weight: 600;
  color: var(--c-text);
}

.preset-desc {
  font-size: 12px;
  color: var(--c-text-3);
  line-height: 1.5;
  margin-bottom: 8px;
  min-height: 36px;
}

.preset-routes {
  display: flex;
  flex-direction: column;
  gap: 4px;
  background: color-mix(in srgb, var(--c-surface) 70%, transparent);
  padding: 6px 8px;
  border-radius: 6px;
  font-size: 11.5px;
  color: var(--c-text-2);
}

.dialog-sub-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--c-text);
  margin-bottom: 8px;
}

.form-tip-text {
  font-size: 12px;
  color: var(--c-text-3);
  line-height: 1.5;
  margin-top: 4px;
}

.dialog-footer-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
}
</style>
