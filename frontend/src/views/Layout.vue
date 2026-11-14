<template>
  <el-container class="layout-container" :class="{ 'is-sidebar-collapsed': isCollapse }">
    <!-- 左侧现代暗黑科技风导航栏 -->
    <el-aside :width="isCollapse ? '64px' : '240px'" class="layout-aside">
      <!-- 品牌 Logo 区域 -->
      <div class="brand-header" @click="router.push('/dashboard')">
        <div class="brand-logo-icon" aria-hidden="true">骥</div>
        <div class="brand-text-wrap" v-if="!isCollapse">
          <div class="brand-title">稳骥 PacerSQL</div>
        </div>
      </div>

      <!-- 导航菜单 -->
      <el-scrollbar class="aside-menu-scrollbar">
        <el-menu
          :default-active="activeMenu"
          :default-openeds="isCollapse ? [] : defaultOpenGroups"
          :collapse="isCollapse"
          class="el-menu-vertical"
          router
          :collapse-transition="false"
        >
          <!-- 1. 平台总览 (仅超级管理员/管理角色可见) -->
          <el-menu-item index="/dashboard" v-if="userStore.isAdmin || userStore.hasPermission('/dashboard')">
            <el-icon class="menu-icon"><Odometer /></el-icon>
            <template #title>
              <span class="menu-title">平台总览</span>
              <el-tag size="small" effect="dark" type="danger" class="menu-badge" v-if="pendingApprovals.length > 0">
                {{ pendingApprovals.length }}
              </el-tag>
            </template>
          </el-menu-item>

          <!-- 2. 工单中心 -->
          <el-sub-menu index="/tickets" v-if="hasAnyPermission(['/ticket-list', '/ticket-create', '/ai-sql-review'])">
            <template #title>
              <el-icon class="menu-icon"><Document /></el-icon>
              <span>工单中心</span>
            </template>
            <el-menu-item index="/ticket-list" v-if="userStore.hasPermission('/ticket-list')">
              <el-icon><Tickets /></el-icon>
              <template #title><span>工单列表</span></template>
            </el-menu-item>
            <el-menu-item index="/ticket-create" v-if="userStore.hasPermission('/ticket-create')">
              <el-icon><EditPen /></el-icon>
              <template #title><span>新建工单</span></template>
            </el-menu-item>
            <el-menu-item index="/ai-sql-review" v-if="userStore.hasPermission('/ai-sql-review')">
              <el-icon><MagicStick /></el-icon>
              <template #title><span>AI 智能审核</span></template>
            </el-menu-item>
          </el-sub-menu>

          <!-- 3. 数据操作与安全 -->
          <el-sub-menu index="/data" v-if="hasAnyPermission(['/data-query', '/data-masking', '/audit-dashboard'])">
            <template #title>
              <el-icon class="menu-icon"><Search /></el-icon>
              <span>数据操作与安全</span>
            </template>
            <el-menu-item index="/data-query" v-if="userStore.hasPermission('/data-query')">
              <el-icon><Search /></el-icon>
              <template #title><span>数据查询控制台</span></template>
            </el-menu-item>
            <el-menu-item index="/data-masking" v-if="userStore.hasPermission('/data-masking')">
              <el-icon><Hide /></el-icon>
              <template #title><span>动态脱敏配置</span></template>
            </el-menu-item>
            <el-menu-item index="/audit-dashboard" v-if="userStore.hasPermission('/audit-dashboard')">
              <el-icon><DataAnalysis /></el-icon>
              <template #title><span>SQL 审计大盘</span></template>
            </el-menu-item>
          </el-sub-menu>

          <!-- 4. 实例与资源管理 (业务资源组统一归纳于此，操作路径最顺畅) -->
          <el-sub-menu index="/instances" v-if="hasAnyPermission(['/instance-list', '/resource-group-list', '/instance-databases', '/instance-accounts', '/instance-sessions', '/instance-params', '/instance-config'])">
            <template #title>
              <el-icon class="menu-icon"><Coin /></el-icon>
              <span>实例与资源管理</span>
            </template>
            <el-menu-item index="/instance-list" v-if="userStore.hasPermission('/instance-list')">
              <el-icon><List /></el-icon>
              <template #title><span>实例列表</span></template>
            </el-menu-item>
            <el-menu-item index="/resource-group-list" v-if="userStore.hasPermission('/resource-group-list')">
              <el-icon><Suitcase /></el-icon>
              <template #title>
                <span>业务资源组</span>
              </template>
            </el-menu-item>
            <el-menu-item index="/instance-databases" v-if="userStore.hasPermission('/instance-databases')">
              <el-icon><FolderOpened /></el-icon>
              <template #title><span>数据库管理</span></template>
            </el-menu-item>
            <el-menu-item index="/instance-accounts" v-if="userStore.hasPermission('/instance-accounts')">
              <el-icon><User /></el-icon>
              <template #title><span>账号权限管理</span></template>
            </el-menu-item>
            <el-menu-item index="/instance-sessions" v-if="userStore.hasPermission('/instance-sessions')">
              <el-icon><DataLine /></el-icon>
              <template #title><span>会话监控与强杀</span></template>
            </el-menu-item>
            <el-menu-item index="/instance-params" v-if="userStore.hasPermission('/instance-params')">
              <el-icon><Monitor /></el-icon>
              <template #title><span>全局参数查看</span></template>
            </el-menu-item>
            <el-menu-item index="/instance-config" v-if="userStore.hasPermission('/instance-config') || userStore.hasPermission('/instance-list')">
              <el-icon><Setting /></el-icon>
              <template #title><span>安全参数策略</span></template>
            </el-menu-item>
          </el-sub-menu>

          <!-- 5. 用户与权限组织 -->
          <el-sub-menu index="/perms" v-if="hasAnyPermission(['/user-list', '/role-list'])">
            <template #title>
              <el-icon class="menu-icon"><UserFilled /></el-icon>
              <span>用户与权限</span>
            </template>
            <el-menu-item index="/user-list" v-if="userStore.hasPermission('/user-list')">
              <el-icon><User /></el-icon>
              <template #title><span>用户管理</span></template>
            </el-menu-item>
            <el-menu-item index="/role-list" v-if="userStore.hasPermission('/role-list')">
              <el-icon><Lock /></el-icon>
              <template #title><span>角色与权限</span></template>
            </el-menu-item>
          </el-sub-menu>

          <!-- 6. 系统与配置 -->
          <el-sub-menu index="/system" v-if="hasAnyPermission(['/workflow-designer', '/ai-config', '/notification-config', '/license', '/settings'])">
            <template #title>
              <el-icon class="menu-icon"><Operation /></el-icon>
              <span>系统与配置</span>
            </template>
            <el-menu-item index="/workflow-designer" v-if="userStore.hasPermission('/workflow-designer')">
              <el-icon><Share /></el-icon>
              <template #title><span>流程设计与模板</span></template>
            </el-menu-item>
            <el-menu-item index="/ai-config" v-if="userStore.hasPermission('/ai-config') || userStore.hasPermission('/settings')">
              <el-icon><Cpu /></el-icon>
              <template #title>
                <span>AI 模型配置</span>
              </template>
            </el-menu-item>
            <el-menu-item index="/notification-config" v-if="userStore.hasPermission('/notification-config') || userStore.hasPermission('/settings')">
              <el-icon><Bell /></el-icon>
              <template #title>
                <span>消息通知与告警</span>
              </template>
            </el-menu-item>
            <el-menu-item index="/license" v-if="userStore.hasPermission('/license')">
              <el-icon><Key /></el-icon>
              <template #title><span>授权证书</span></template>
            </el-menu-item>
            <el-menu-item index="/settings" v-if="userStore.hasPermission('/settings')">
              <el-icon><Brush /></el-icon>
              <template #title><span>自定义主题与水印</span></template>
            </el-menu-item>
          </el-sub-menu>
        </el-menu>
      </el-scrollbar>

      <!-- 侧边栏底部折叠切换器 -->
      <div class="aside-footer" @click="isCollapse = !isCollapse">
        <el-icon class="collapse-icon">
          <Fold v-if="!isCollapse" />
          <Expand v-else />
        </el-icon>
        <span class="collapse-text" v-if="!isCollapse">收起</span>
      </div>
    </el-aside>

    <!-- 右侧主体内容容器 -->
    <el-container class="inner-container">
      <!-- 现代化精致 Header -->
      <el-header class="layout-header">
        <div class="header-left">
          <!-- 返回门户快捷入口 -->
          <a href="/" class="portal-return-link" title="返回 FWPT 统一门户">
            <el-icon><Back /></el-icon>
            <span>门户</span>
          </a>
          <span class="header-split" aria-hidden="true"></span>
          <!-- 面包屑导航 -->
          <el-breadcrumb separator="/" class="layout-breadcrumb">
            <el-breadcrumb-item v-if="currentModuleTitle">
              <span>{{ currentModuleTitle }}</span>
            </el-breadcrumb-item>
            <el-breadcrumb-item>
              <span class="active-crumb">{{ currentPageTitle }}</span>
            </el-breadcrumb-item>
          </el-breadcrumb>
        </div>

        <div class="header-right">
          <!-- 快捷全局功能导航搜索 -->
          <div class="quick-search-trigger" @click="handleOpenSearchDialog">
            <el-icon><Search /></el-icon>
            <span>搜索页面</span>
            <kbd class="shortcut-key">{{ shortcutLabel }}</kbd>
          </div>

          <!-- 待办审批提醒通知中心 -->
          <el-popover
            placement="bottom-end"
            :width="380"
            trigger="click"
            popper-class="notification-popover"
          >
            <template #reference>
              <div class="header-tool-btn" :class="{ 'has-badge': pendingApprovals.length > 0 }">
                <el-badge :value="pendingApprovals.length" :hidden="pendingApprovals.length === 0" :max="99">
                  <el-button circle class="bell-btn">
                    <el-icon :size="17"><Bell /></el-icon>
                  </el-button>
                </el-badge>
              </div>
            </template>

            <!-- 待办审批列表弹窗内容 -->
            <div class="notification-card">
              <div class="n-header">
                <div class="n-title">
                  <span>待我审批</span>
                  <span class="n-count" v-if="pendingApprovals.length">{{ pendingApprovals.length }}</span>
                </div>
                <el-button link type="primary" size="small" :icon="Refresh" @click="fetchPendingApprovals">刷新</el-button>
              </div>

              <div class="n-body" v-if="pendingApprovals.length > 0">
                <div
                  v-for="item in pendingApprovals"
                  :key="item.id"
                  class="n-item"
                  @click="handleGoToApproval(item.id)"
                >
                  <div class="n-item-top">
                    <span class="n-item-type">[{{ formatType(item.type) }}]</span>
                    <span class="n-item-id">#{{ item.id }} {{ item.businessKey }}</span>
                    <el-tag size="small" type="warning" effect="plain" class="n-item-node">
                      {{ item.currentNodeName || '待初审/复审' }}
                    </el-tag>
                  </div>
                  <div class="n-item-meta">
                    <span>目标库：<b>{{ item.instanceName }} / {{ item.dbName }}</b></span>
                  </div>
                  <div class="n-item-desc" :title="item.reason">
                    {{ item.reason || '无描述' }}
                  </div>
                  <div class="n-item-footer">
                    <span class="n-item-applicant">申请人：{{ item.applicantName || item.applicantIdCard }}</span>
                    <span class="n-item-time">{{ formatTime(item.createTime) }}</span>
                  </div>
                </div>
              </div>

              <el-empty v-else description="没有待你审批的工单" :image-size="64" style="padding: 20px 0;" />

              <div class="n-footer" v-if="pendingApprovals.length > 0">
                <el-button type="primary" size="small" style="width: 100%;" @click="handleGoToAllTickets">
                  查看全部工单列表
                </el-button>
              </div>
            </div>
          </el-popover>

          <!-- 分隔线 -->
          <div class="header-divider"></div>

          <!-- 用户头像与角色信息 -->
          <el-dropdown @command="handleUserCommand" trigger="click">
            <div class="user-profile-badge">
              <div class="avatar-box">
                <el-avatar size="small" class="custom-avatar">
                  {{ userRealName ? userRealName.substring(0, 1) : '管' }}
                </el-avatar>
                <span class="status-dot"></span>
              </div>
              <div class="user-text-info">
                <span class="user-name">{{ userRealName }}</span>
                <span class="role-pill">{{ formatRoleNameZh(userStore.userRole) }}</span>
              </div>
              <el-icon class="arrow-icon"><ArrowDown /></el-icon>
            </div>
            <template #dropdown>
              <el-dropdown-menu class="custom-dropdown-menu">
                <div class="user-dropdown-header">
                  <div class="u-name">{{ userRealName }}</div>
                  <div class="u-role">权限身份：{{ formatRoleNameZh(userStore.userRole) }}</div>
                </div>
                <el-dropdown-item command="profile">
                  <el-icon><User /></el-icon>个人中心 / 账号设置
                </el-dropdown-item>
                <el-dropdown-item command="ai-config" v-if="userStore.isAdmin || userStore.hasPermission('/ai-config')">
                  <el-icon><Cpu /></el-icon>AI 模型与配置中心
                </el-dropdown-item>
                <el-dropdown-item command="notification-config" v-if="userStore.isAdmin || userStore.hasPermission('/notification-config')">
                  <el-icon><Bell /></el-icon>消息通知与告警配置
                </el-dropdown-item>
                <el-dropdown-item command="settings" v-if="userStore.isAdmin || userStore.hasPermission('/settings')">
                  <el-icon><Brush /></el-icon>个性化外观与主题
                </el-dropdown-item>
                <el-dropdown-item command="logout" divided class="logout-item">
                  <el-icon><SwitchButton /></el-icon>退出登录
                </el-dropdown-item>
              </el-dropdown-menu>
            </template>
          </el-dropdown>
        </div>
      </el-header>

      <!-- 主视图路由展示区域 -->
      <el-main class="layout-main">
        <router-view v-slot="{ Component }">
          <transition name="fade-transform" mode="out-in">
            <component :is="Component" />
          </transition>
        </router-view>
      </el-main>
    </el-container>

    <!-- 全局快捷跳转搜索弹窗 (Ctrl + K) -->
    <el-dialog
      v-model="searchDialogVisible"
      title="跳转到页面"
      width="560px"
      :show-close="false"
      class="quick-search-dialog"
      align-center
    >
      <el-input
        v-model="searchKeyword"
        placeholder="输入页面名称，如 工单、实例、脱敏；回车打开第一项"
        :prefix-icon="Search"
        clearable
        size="large"
        class="search-dialog-input"
        autofocus
        @keydown.enter.prevent="handleQuickNavigateFirst"
      />
      <div class="search-results-list">
        <div
          v-for="item in filteredQuickNavItems"
          :key="item.path"
          class="search-result-item"
          @click="handleQuickNavigate(item.path)"
        >
          <div class="s-meta">
            <div class="s-title">{{ item.name }}</div>
            <div class="s-desc">{{ item.desc }}</div>
          </div>
          <el-tag size="small" type="info" effect="plain">{{ item.category }}</el-tag>
        </div>
        <el-empty v-if="filteredQuickNavItems.length === 0" description="没有匹配的页面" :image-size="60" />
      </div>
    </el-dialog>
  </el-container>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useUserStore } from '../store/user'
import request from '../utils/request'
import {
  ArrowDown,
  Odometer,
  Document,
  Tickets,
  EditPen,
  MagicStick,
  Search,
  Coin,
  List,
  DataLine,
  FolderOpened,
  User,
  Setting,
  Monitor,
  UserFilled,
  Suitcase,
  Lock,
  Operation,
  Share,
  Key,
  Brush,
  Hide,
  Bell,
  Refresh,
  SwitchButton,
  DataAnalysis,
  Cpu,
  Fold,
  Expand,
  Back
} from '@element-plus/icons-vue'

const router = useRouter()
const route = useRoute()
const userStore = useUserStore()

const isCollapse = ref(false)
const searchDialogVisible = ref(false)
const searchKeyword = ref('')

const hasAnyPermission = (paths: string[]) => {
  if (userStore.isAdmin) return true
  return paths.some(p => userStore.hasPermission(p))
}

const formatRoleNameZh = (role: string) => {
  if (!role) return '研发工程师'
  const r = role.toUpperCase().trim()
  if (r === 'ADMIN' || r === 'ROLE_ADMIN' || r.includes('ADMIN')) return '系统管理员'
  if (r === 'DBA' || r === 'ROLE_DBA' || r.includes('DBA')) return '核心 DBA'
  if (r === 'DEV_LEAD' || r === 'LEAD' || r.includes('LEAD')) return '开发组长'
  if (r === 'AUDITOR' || r === 'SECURITY_AUDITOR' || r.includes('AUDIT')) return '合规审计员'
  if (r === 'OPS' || r === 'ROLE_OPS' || r.includes('OPS')) return '运维工程师'
  if (r === 'DEV' || r === 'DEVELOPER' || r === 'ROLE_DEV') return '研发工程师'
  return role
}

const pendingApprovals = ref<any[]>([])
let pollTimer: number | null = null

const activeMenu = computed(() => {
  const path = route.path
  if (path.startsWith('/instance/edit') || path === '/instance/create') {
    return '/instance-list'
  }
  return path
})

const userRealName = computed(() => userStore.realName || '管理员')

// 面包屑标题映射
const routeTitleMap: Record<string, { module: string; page: string }> = {
  '/dashboard': { module: '概览', page: '平台总览' },
  '/ticket-list': { module: '工单中心', page: '工单列表' },
  '/ticket-create': { module: '工单中心', page: '新建工单' },
  '/ai-sql-review': { module: '工单中心', page: 'AI 智能审核' },
  '/data-query': { module: '数据操作与安全', page: '数据查询控制台' },
  '/data-masking': { module: '数据操作与安全', page: '动态脱敏配置' },
  '/audit-dashboard': { module: '数据操作与安全', page: 'SQL 审计大盘' },
  '/instance-list': { module: '实例与资源管理', page: '实例列表' },
  '/resource-group-list': { module: '实例与资源管理', page: '业务资源组' },
  '/instance-databases': { module: '实例与资源管理', page: '数据库管理' },
  '/instance-accounts': { module: '实例与资源管理', page: '账号权限管理' },
  '/instance-sessions': { module: '实例与资源管理', page: '会话监控与强杀' },
  '/instance-params': { module: '实例与资源管理', page: '全局参数查看' },
  '/instance-config': { module: '实例与资源管理', page: '安全参数策略' },
  '/user-list': { module: '用户与权限', page: '用户管理' },
  '/role-list': { module: '用户与权限', page: '角色与权限' },
  '/workflow-designer': { module: '系统与配置', page: '流程设计与模板' },
  '/ai-config': { module: '系统与配置', page: 'AI 模型配置' },
  '/notification-config': { module: '系统与配置', page: '消息通知与告警配置' },
  '/license': { module: '系统与配置', page: '授权证书' },
  '/settings': { module: '系统与配置', page: '自定义主题与水印' },
  '/user-profile': { module: '个人中心', page: '个人设置' }
}

const currentModuleTitle = computed(() => {
  const item = routeTitleMap[route.path]
  return item ? item.module : ''
})

const currentPageTitle = computed(() => {
  const item = routeTitleMap[route.path]
  if (item) return item.page
  if (route.path.startsWith('/ticket/')) return '工单详情与审批'
  return route.name ? String(route.name) : '当前页面'
})

// 全局快捷搜索导航项
const quickNavItems = [
  { name: '平台总览 (Dashboard)', desc: '系统运行指标、工单趋势与拦截统计', path: '/dashboard', icon: '📊', category: '平台总览' },
  { name: '工单列表', desc: '检索历史变更、追踪审批进度', path: '/ticket-list', icon: '📋', category: '工单中心' },
  { name: '新建 SQL 工单', desc: '提交 DML/DDL 变更并执行 Dry-Run 预检', path: '/ticket-create', icon: '✍️', category: '工单中心' },
  { name: 'AI 智能审核', desc: '大模型 SQL 性能诊断、重写与风险审计', path: '/ai-sql-review', icon: '🤖', category: '工单中心' },
  { name: '数据查询控制台', desc: '安全 Web SQL 查询与分页浏览', path: '/data-query', icon: '🔍', category: '数据与安全' },
  { name: '动态脱敏配置', desc: '手机号、身份证、银行卡等脱敏规则策略', path: '/data-masking', icon: '🛡️', category: '数据与安全' },
  { name: 'SQL 审计大盘', desc: '操作流水日志、合规看板与风险拦截', path: '/audit-dashboard', icon: '📈', category: '数据与安全' },
  { name: '实例列表', desc: '数据库实例注册、连通性探测', path: '/instance-list', icon: '🗄️', category: '实例与资源' },
  { name: '业务资源组', desc: '业务线与实例绑定划分、多组归属', path: '/resource-group-list', icon: '💼', category: '实例与资源' },
  { name: '会话管理 (Kill Session)', desc: '活跃连接监控、慢查询排查与会话强杀', path: '/instance-sessions', icon: '⚡', category: '实例与资源' },
  { name: '数据库管理 (Schema)', desc: '库清单、字符集与数据空间容量', path: '/instance-databases', icon: '📁', category: '实例与资源' },
  { name: '账号管理 (Accounts)', desc: '原生数据库账号与权限配置', path: '/instance-accounts', icon: '👤', category: '实例与资源' },
  { name: '全局参数查看 (Variables)', desc: 'MySQL 系统全局参数与平台策略', path: '/instance-params', icon: '⚙️', category: '实例与资源' },
  { name: '流程设计与模板 (BPMN)', desc: '可视化工作流拓扑编排与模板绑定', path: '/workflow-designer', icon: '🔀', category: '系统配置' },
  { name: 'AI 模型配置', desc: 'DeepSeek、Qwen、OpenAI、Ollama 接入配置与连通自检', path: '/ai-config', icon: '🧠', category: '系统配置' },
  { name: '消息通知与告警配置', desc: '企业微信工作消息、阿里钉钉机器人、电话语音外呼与频次策略', path: '/notification-config', icon: '🔔', category: '系统配置' },
  { name: '用户管理', desc: '系统人员账号维护与组织分配', path: '/user-list', icon: '👥', category: '用户与权限' },
  { name: '角色权限', desc: '系统角色定义与页签权限矩阵', path: '/role-list', icon: '🔐', category: '用户与权限' },
  { name: '自定义主题与水印', desc: '系统外观风格、主色调与防泄密水印', path: '/settings', icon: '🎨', category: '系统配置' },
  { name: '安全参数策略', desc: '实例级执行策略与安全阈值', path: '/instance-config', icon: '', category: '实例与资源' },
  { name: '授权证书', desc: '查看与更新商业授权', path: '/license', icon: '', category: '系统配置' },
  { name: '个人中心', desc: '个人资料、联系方式与修改密码', path: '/user-profile', icon: '', category: '个人' }
]

const userAccessibleQuickNavItems = computed(() => {
  return quickNavItems.filter(item => item.path === '/user-profile' || userStore.isAdmin || userStore.hasPermission(item.path))
})

const filteredQuickNavItems = computed(() => {
  const baseList = userAccessibleQuickNavItems.value
  if (!searchKeyword.value.trim()) return baseList
  const q = searchKeyword.value.trim().toLowerCase()
  return baseList.filter(item => 
    item.name.toLowerCase().includes(q) || 
    item.desc.toLowerCase().includes(q) ||
    item.category.toLowerCase().includes(q)
  )
})

const handleOpenSearchDialog = () => {
  searchKeyword.value = ''
  searchDialogVisible.value = true
}

const handleQuickNavigateFirst = () => {
  const first = filteredQuickNavItems.value[0]
  if (first) handleQuickNavigate(first.path)
}

const shortcutLabel = /Mac|iPhone|iPad/i.test(navigator.platform || navigator.userAgent) ? '⌘K' : 'Ctrl K'

// 侧栏只展开当前页面所在的分组，菜单不再一眼望不到头
const menuGroups: Record<string, string[]> = {
  '/tickets': ['/ticket-list', '/ticket-create', '/ai-sql-review', '/ticket/'],
  '/data': ['/data-query', '/data-masking', '/audit-dashboard'],
  '/instances': ['/instance-list', '/resource-group-list', '/instance-databases', '/instance-accounts', '/instance-sessions', '/instance-params', '/instance-config', '/instance/'],
  '/perms': ['/user-list', '/role-list'],
  '/system': ['/workflow-designer', '/ai-config', '/notification-config', '/license', '/settings']
}
const defaultOpenGroups = computed(() => {
  const path = route.path
  const hit = Object.keys(menuGroups).find(key => menuGroups[key].some(p => (p.endsWith('/') ? path.startsWith(p) : path === p)))
  return hit ? [hit] : ['/tickets']
})

const handleQuickNavigate = (path: string) => {
  searchDialogVisible.value = false
  router.push(path)
}

const handleKeyDown = (e: KeyboardEvent) => {
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
    e.preventDefault()
    searchDialogVisible.value = !searchDialogVisible.value
  }
}

const fetchPendingApprovals = async () => {
  if (!userStore.token) return
  if (typeof document !== 'undefined' && document.hidden) return
  try {
    const res: any = await request.get('/v1/ticket/pending-approvals')
    pendingApprovals.value = Array.isArray(res.data) ? res.data : []
  } catch (error) {
    // ignore
  }
}

const handleResponsiveSidebar = () => {
  if (window.innerWidth < 1024 && !isCollapse.value) {
    isCollapse.value = true
  }
}

const handleGoToApproval = (ticketId: number | string) => {
  router.push(`/ticket/${ticketId}`)
}

const handleGoToAllTickets = () => {
  router.push({ path: '/ticket-list', query: { status: 'AUDITING' } })
}

const formatType = (type: string) => {
  const map: Record<string, string> = {
    SQL_AUDIT: 'SQL 变更审核',
    DML_CHANGE: 'DML 数据变更',
    DDL_CHANGE: 'DDL 结构变更',
    DATA_EXPORT: '敏感数据导出',
    PERMISSION: '权限申请',
    ACCOUNT: '账号申请',
    DB_TABLE: '库表申请',
    DATA_RECOVERY: '应急数据恢复',
    DATA_QUERY: '数据查询'
  }
  return map[type] || type || 'SQL 变更审核'
}

const formatTime = (timeStr: any) => {
  if (!timeStr) return ''
  const str = String(timeStr).trim()
  if (str.includes('-') && str.includes(':')) {
    return str
  }
  if (str.length >= 14 && (str.startsWith('2025') || str.startsWith('2026') || str.startsWith('2027'))) {
    const y = str.substring(0, 4)
    const m = str.substring(4, 6)
    const d = str.substring(6, 8)
    const h = str.substring(8, 10)
    const mi = str.substring(10, 12)
    const s = str.substring(12, 14)
    return `${y}-${m}-${d} ${h}:${mi}:${s}`
  }
  const num = Number(str)
  if (num && num > 1000000000000) {
    const d = new Date(num)
    const year = d.getFullYear()
    const month = String(d.getMonth() + 1).padStart(2, '0')
    const day = String(d.getDate()).padStart(2, '0')
    const hours = String(d.getHours()).padStart(2, '0')
    const minutes = String(d.getMinutes()).padStart(2, '0')
    const seconds = String(d.getSeconds()).padStart(2, '0')
    return `${year}-${month}-${day} ${hours}:${minutes}:${seconds}`
  }
  return str
}

const handleLogout = () => {
  userStore.logout()
  router.push('/login')
}

const handleUserCommand = (cmd: string) => {
  if (cmd === 'profile') {
    router.push('/user-profile')
  } else if (cmd === 'ai-config') {
    router.push('/ai-config')
  } else if (cmd === 'notification-config') {
    router.push('/notification-config')
  } else if (cmd === 'settings') {
    router.push('/settings')
  } else if (cmd === 'logout') {
    handleLogout()
  }
}

onMounted(() => {
  userStore.fetchUserInfo()
  handleResponsiveSidebar()
  fetchPendingApprovals()
  pollTimer = window.setInterval(fetchPendingApprovals, 15000)
  window.addEventListener('keydown', handleKeyDown)
  window.addEventListener('resize', handleResponsiveSidebar)
})

onUnmounted(() => {
  if (pollTimer) {
    clearInterval(pollTimer)
    pollTimer = null
  }
  window.removeEventListener('keydown', handleKeyDown)
  window.removeEventListener('resize', handleResponsiveSidebar)
})
</script>

<style scoped>
/* ============ 框架：侧栏 + 顶栏 + 内容区（极简） ============ */
.layout-container {
  height: 100vh;
  width: 100%;
  overflow: hidden;
  background: var(--c-bg);
}

/* ---- 侧栏 ---- */
.layout-aside {
  height: 100vh;
  display: flex;
  flex-direction: column;
  background: var(--c-surface);
  border-right: 1px solid var(--c-border);
  transition: width 0.2s ease;
  overflow: hidden;
}
.brand-header {
  height: 56px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 0 18px;
  cursor: pointer;
}
.is-sidebar-collapsed .brand-header { justify-content: center; padding: 0; }
.brand-logo-icon {
  width: 28px;
  height: 28px;
  flex-shrink: 0;
  border-radius: 7px;
  background: var(--c-text);
  color: var(--c-surface);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 15px;
  font-weight: 700;
  line-height: 1;
}
.brand-text-wrap { min-width: 0; }
.brand-title {
  font-size: 15px;
  font-weight: 650;
  color: var(--c-text);
  letter-spacing: -0.01em;
  white-space: nowrap;
}

.aside-menu-scrollbar { flex: 1; min-height: 0; }
.el-menu-vertical {
  --el-menu-bg-color: transparent;
  --el-menu-hover-bg-color: var(--c-subtle);
  --el-menu-text-color: var(--c-text-2);
  --el-menu-active-color: var(--c-text);
  --el-menu-item-height: 36px;
  --el-menu-sub-item-height: 34px;
  --el-menu-base-level-padding: 18px;
  --el-menu-level-padding: 14px;
  border-right: none;
  padding: 4px 10px 16px;
}
.el-menu-vertical:not(.el-menu--collapse) { width: 100%; }
:deep(.el-menu-item),
:deep(.el-sub-menu__title) {
  border-radius: var(--radius-sm);
  font-size: 13.5px;
  margin-bottom: 1px;
}
:deep(.el-sub-menu__title) {
  font-weight: 500;
  color: var(--c-text-2);
}
:deep(.el-sub-menu__title:hover),
:deep(.el-menu-item:hover) { background: var(--c-subtle); }
:deep(.el-sub-menu .el-menu) { background: transparent; }
:deep(.el-sub-menu .el-menu-item) {
  padding-left: 42px !important;
  color: var(--c-text-3);
}
:deep(.el-menu-item.is-active) {
  background: var(--c-subtle);
  color: var(--c-text);
  font-weight: 600;
}
:deep(.el-menu-item .el-icon),
:deep(.el-sub-menu__title .el-icon) {
  color: var(--c-text-4);
  font-size: 16px;
}
:deep(.el-menu-item.is-active .el-icon) { color: var(--c-accent); }
:deep(.el-sub-menu .el-menu-item .el-icon) { display: none; }
:deep(.el-menu--collapse .el-sub-menu .el-menu-item .el-icon) { display: inline-flex; }
.menu-icon { margin-right: 10px; }
.menu-badge {
  margin-left: auto;
  height: 18px;
  min-width: 18px;
  padding: 0 5px;
  border-radius: 9px;
  font-size: 11px;
  line-height: 18px;
  background: var(--c-danger);
  color: #fff;
  border: none;
}

.aside-footer {
  height: 44px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 22px;
  border-top: 1px solid var(--c-border);
  color: var(--c-text-3);
  font-size: 13px;
  cursor: pointer;
}
.is-sidebar-collapsed .aside-footer { justify-content: center; padding: 0; }
.aside-footer:hover { color: var(--c-text); }
.collapse-icon { font-size: 16px; }

/* ---- 顶栏 ---- */
.inner-container {
  height: 100vh;
  min-width: 0;
  flex-direction: column;
  overflow: hidden;
}
.layout-header {
  height: 56px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 0 24px;
  background: var(--c-surface);
  border-bottom: 1px solid var(--c-border);
}
.header-left {
  display: flex;
  align-items: center;
  gap: 14px;
  min-width: 0;
}
.portal-return-link {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 13px;
  color: var(--c-text-3);
  text-decoration: none;
  white-space: nowrap;
}
.portal-return-link:hover { color: var(--c-text); }
.header-split {
  width: 1px;
  height: 16px;
  background: var(--c-border);
}
.layout-breadcrumb { font-size: 13px; white-space: nowrap; }
:deep(.layout-breadcrumb .el-breadcrumb__inner) { color: var(--c-text-3); font-weight: 400; }
:deep(.layout-breadcrumb .el-breadcrumb__separator) { color: var(--c-text-4); margin: 0 8px; }
.active-crumb { color: var(--c-text); font-weight: 600; }

.header-right {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}
.quick-search-trigger {
  height: 32px;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 8px 0 10px;
  min-width: 200px;
  border: 1px solid var(--c-border);
  border-radius: var(--radius-sm);
  background: var(--c-surface);
  color: var(--c-text-4);
  font-size: 13px;
  cursor: pointer;
}
.quick-search-trigger:hover { border-color: var(--c-border-strong); color: var(--c-text-3); }
.shortcut-key {
  margin-left: auto;
  font-size: 11px;
  padding: 1px 5px;
  border-radius: 4px;
  border: 1px solid var(--c-border);
  color: var(--c-text-3);
  background: var(--c-muted-bg);
}
.header-tool-btn { display: flex; align-items: center; }
.bell-btn {
  width: 32px;
  height: 32px;
  border: none;
  background: transparent;
  color: var(--c-text-3);
}
.bell-btn:hover { background: var(--c-subtle); color: var(--c-text); }
:deep(.el-badge__content) { border: none; height: 16px; line-height: 16px; padding: 0 5px; font-size: 11px; }
.header-divider {
  width: 1px;
  height: 18px;
  background: var(--c-border);
  margin: 0 4px;
}
.user-profile-badge {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 3px 6px 3px 3px;
  border-radius: var(--radius-sm);
  cursor: pointer;
}
.user-profile-badge:hover { background: var(--c-subtle); }
.avatar-box { position: relative; display: flex; }
.custom-avatar {
  --el-avatar-size: 28px;
  background: var(--c-subtle);
  color: var(--c-text);
  font-weight: 600;
  font-size: 13px;
}
.status-dot { display: none; }
.user-text-info {
  display: flex;
  flex-direction: column;
  line-height: 1.25;
}
.user-name { font-size: 13px; font-weight: 600; color: var(--c-text); }
.role-pill { font-size: 11.5px; color: var(--c-text-3); }
.arrow-icon { color: var(--c-text-4); font-size: 12px; }

.custom-dropdown-menu { min-width: 200px; }
.user-dropdown-header {
  padding: 10px 16px 8px;
  border-bottom: 1px solid var(--c-border);
  margin-bottom: 4px;
}
.u-name { font-size: 14px; font-weight: 600; color: var(--c-text); }
.u-role { font-size: 12px; color: var(--c-text-3); margin-top: 2px; }
.logout-item { color: var(--c-danger); }

/* ---- 内容区 ---- */
.layout-main {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 24px 28px 40px;
  background: var(--c-bg);
}
.fade-transform-enter-active,
.fade-transform-leave-active { transition: opacity 0.15s ease; }
.fade-transform-enter-from,
.fade-transform-leave-to { opacity: 0; }

/* ---- 待办审批弹出层 ---- */
.notification-card { margin: -4px; }
.n-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 4px 10px;
  border-bottom: 1px solid var(--c-border);
}
.n-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  font-weight: 600;
  color: var(--c-text);
}
.n-count {
  font-size: 12px;
  font-weight: 600;
  color: var(--c-danger);
}
.n-body { max-height: 380px; overflow-y: auto; }
.n-item {
  padding: 12px 4px;
  border-bottom: 1px solid var(--c-border);
  cursor: pointer;
}
.n-item:last-child { border-bottom: none; }
.n-item:hover { background: var(--c-muted-bg); }
.n-item-top {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
}
.n-item-type { color: var(--c-text-3); }
.n-item-id {
  font-weight: 600;
  color: var(--c-text);
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.n-item-node { flex-shrink: 0; }
.n-item-meta,
.n-item-desc,
.n-item-footer {
  font-size: 12.5px;
  color: var(--c-text-3);
  margin-top: 4px;
}
.n-item-meta b { color: var(--c-text-2); font-weight: 500; }
.n-item-desc {
  color: var(--c-text-2);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.n-item-footer { display: flex; justify-content: space-between; }
.n-footer { padding-top: 10px; border-top: 1px solid var(--c-border); }

/* ---- 跳转搜索 ---- */
.quick-search-dialog :deep(.el-dialog__header) { padding-bottom: 8px; }
.quick-search-dialog :deep(.el-dialog__body) { padding-top: 4px; }
.search-dialog-input { margin-bottom: 8px; }
.search-results-list {
  max-height: 380px;
  overflow-y: auto;
  margin: 0 -8px;
}
.search-result-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 9px 10px;
  border-radius: var(--radius-sm);
  cursor: pointer;
}
.search-result-item:hover,
.search-result-item:first-child { background: var(--c-subtle); }
.search-results-list:hover .search-result-item:first-child:not(:hover) { background: transparent; }
.s-meta { flex: 1; min-width: 0; }
.s-title { font-size: 14px; font-weight: 500; color: var(--c-text); }
.s-desc {
  font-size: 12.5px;
  color: var(--c-text-3);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

@media (max-width: 1100px) {
  .quick-search-trigger { min-width: 0; }
  .quick-search-trigger > span { display: none; }
  .user-text-info { display: none; }
}
@media (max-width: 760px) {
  .layout-header { padding: 0 12px; }
  .layout-main { padding: 16px 12px 32px; }
  .layout-breadcrumb, .header-split { display: none; }
}
</style>
