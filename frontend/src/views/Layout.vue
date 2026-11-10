<template>
  <el-container class="layout-container" :class="{ 'is-sidebar-collapsed': isCollapse }">
    <!-- 左侧现代暗黑科技风导航栏 -->
    <el-aside :width="isCollapse ? '64px' : '240px'" class="layout-aside">
      <!-- 品牌 Logo 区域 -->
      <div class="brand-header" @click="router.push('/dashboard')">
        <div class="brand-logo-icon">
          <el-icon><Platform /></el-icon>
        </div>
        <div class="brand-text-wrap" v-if="!isCollapse">
          <div class="brand-title">稳骥 PacerSQL</div>
          <div class="brand-subtitle">高可用数据库变更自治平台</div>
        </div>
      </div>

      <!-- 导航菜单 -->
      <el-scrollbar class="aside-menu-scrollbar">
        <el-menu
          :default-active="activeMenu"
          :default-openeds="isCollapse ? [] : ['/instances', '/tickets', '/data', '/perms', '/system']"
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
                <el-tag size="small" type="success" effect="plain" class="inner-menu-tag">业务线</el-tag>
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
                <el-tag size="small" type="primary" effect="plain" class="inner-menu-tag">LLM</el-tag>
              </template>
            </el-menu-item>
            <el-menu-item index="/notification-config" v-if="userStore.hasPermission('/notification-config') || userStore.hasPermission('/settings')">
              <el-icon><Bell /></el-icon>
              <template #title>
                <span>消息通知与告警</span>
                <el-tag size="small" type="success" effect="plain" class="inner-menu-tag">企微/钉钉/飞书</el-tag>
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
        <span class="collapse-text" v-if="!isCollapse">收起导航菜单</span>
      </div>
    </el-aside>

    <!-- 右侧主体内容容器 -->
    <el-container class="inner-container">
      <!-- 现代化精致 Header -->
      <el-header class="layout-header">
        <div class="header-left">
          <!-- 返回门户快捷入口 -->
          <a href="/" class="portal-return-link" title="返回 FWPT 统一门户与系统矩阵">
            <span class="portal-pill-badge">FWPT</span>
            <span>返回门户</span>
          </a>
          <!-- 面包屑导航 -->
          <el-breadcrumb separator="/" class="layout-breadcrumb">
            <el-breadcrumb-item :to="{ path: '/dashboard' }">
              <el-icon class="crumb-icon"><HomeFilled /></el-icon>
              <span>控制台</span>
            </el-breadcrumb-item>
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
            <span>搜索功能与页面...</span>
            <kbd class="shortcut-key">Ctrl K</kbd>
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
                  <el-icon color="#F56C6C" style="margin-right: 6px;"><BellFilled /></el-icon>
                  <span>待办审批工单提醒</span>
                  <el-tag size="small" type="danger" effect="dark" round style="margin-left: 8px;">
                    {{ pendingApprovals.length }} 待处理
                  </el-tag>
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

              <el-empty v-else description="暂无需要您审批的待办工单 🎉" :image-size="70" style="padding: 20px 0;" />

              <div class="n-footer" v-if="pendingApprovals.length > 0">
                <el-button type="primary" size="small" style="width: 100%;" @click="handleGoToAllTickets">
                  查看全部工单列表
                </el-button>
              </div>
            </div>
          </el-popover>

          <!-- 语言切换 -->
          <el-dropdown @command="handleLanguageChange" trigger="click">
            <div class="header-tool-btn text-tool-btn">
              <span>{{ locale === 'zh' ? '🇨🇳 中文' : '🇺🇸 English' }}</span>
              <el-icon class="el-icon--right"><ArrowDown /></el-icon>
            </div>
            <template #dropdown>
              <el-dropdown-menu class="custom-dropdown-menu">
                <el-dropdown-item command="zh" :class="{ 'is-active-lang': locale === 'zh' }">🇨🇳 简体中文</el-dropdown-item>
                <el-dropdown-item command="en" :class="{ 'is-active-lang': locale === 'en' }">🇺🇸 English</el-dropdown-item>
              </el-dropdown-menu>
            </template>
          </el-dropdown>

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
                <el-tag size="small" :type="getRoleTagType(userStore.userRole)" effect="light" class="role-pill">
                  {{ formatRoleNameZh(userStore.userRole) }}
                </el-tag>
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
      title="🚀 全局快捷功能检索"
      width="560px"
      :show-close="false"
      class="quick-search-dialog"
      align-center
    >
      <el-input
        v-model="searchKeyword"
        placeholder="输入功能名称或拼音进行快速跳转 (如 工单、资源组、AI、实例)..."
        :prefix-icon="Search"
        clearable
        size="large"
        class="search-dialog-input"
        autofocus
      />
      <div class="search-results-list">
        <div
          v-for="item in filteredQuickNavItems"
          :key="item.path"
          class="search-result-item"
          @click="handleQuickNavigate(item.path)"
        >
          <div class="s-icon">{{ item.icon }}</div>
          <div class="s-meta">
            <div class="s-title">{{ item.name }}</div>
            <div class="s-desc">{{ item.desc }}</div>
          </div>
          <el-tag size="small" type="info" effect="plain">{{ item.category }}</el-tag>
        </div>
        <el-empty v-if="filteredQuickNavItems.length === 0" description="未搜索到匹配功能，请尝试其他关键词" :image-size="60" />
      </div>
    </el-dialog>
  </el-container>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useUserStore } from '../store/user'
import { useI18n } from 'vue-i18n'
import request from '../utils/request'
import {
  ArrowDown,
  Platform,
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
  BellFilled,
  Refresh,
  SwitchButton,
  DataAnalysis,
  Cpu,
  HomeFilled,
  Fold,
  Expand
} from '@element-plus/icons-vue'

const { locale } = useI18n()
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

const getRoleTagType = (role: string) => {
  switch (role?.toUpperCase()) {
    case 'ADMIN':
    case 'ROLE_ADMIN': return 'danger'
    case 'DBA':
    case 'ROLE_DBA': return 'warning'
    case 'DEV_LEAD':
    case 'LEAD': return 'primary'
    case 'DEV':
    case 'ROLE_DEV': return 'info'
    case 'AUDITOR':
    case 'ROLE_AUDITOR': return 'success'
    case 'OPS':
    case 'ROLE_OPS': return 'success'
    default: return 'info'
  }
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
  { name: '自定义主题与水印', desc: '系统外观风格、主色调与防泄密水印', path: '/settings', icon: '🎨', category: '系统配置' }
]

const userAccessibleQuickNavItems = computed(() => {
  return quickNavItems.filter(item => userStore.isAdmin || userStore.hasPermission(item.path))
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

const handleLanguageChange = (lang: string) => {
  locale.value = lang
  localStorage.setItem('language', lang)
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
.layout-container {
  height: 100vh;
  width: 100vw;
  display: flex;
  flex-direction: row !important;
  overflow: hidden;
  background-color: #f1f5f9;
}

/* ===================================================
   1. 侧边栏：现代极简纯净、护眼明亮视觉系统
   =================================================== */
.layout-aside {
  background: #ffffff;
  flex-shrink: 0;
  height: 100%;
  display: flex;
  flex-direction: column;
  box-shadow: 2px 0 12px rgba(15, 23, 42, 0.04);
  transition: width 0.28s cubic-bezier(0.4, 0, 0.2, 1);
  z-index: 100;
  border-right: 1px solid #e2e8f0;
}

.brand-header {
  height: 64px;
  display: flex;
  align-items: center;
  padding: 0 18px;
  background: #ffffff;
  border-bottom: 1px solid #f1f5f9;
  cursor: pointer;
  transition: all 0.2s ease;
  user-select: none;
}

.brand-header:hover {
  background: #f8fafc;
}

.brand-logo-icon {
  width: 36px;
  height: 36px;
  border-radius: 10px;
  background: linear-gradient(135deg, #2563eb 0%, #38bdf8 100%);
  display: flex;
  align-items: center;
  justify-content: center;
  color: #ffffff;
  font-size: 20px;
  box-shadow: 0 4px 14px rgba(37, 99, 235, 0.25);
  flex-shrink: 0;
  transition: transform 0.25s ease;
}

.brand-header:hover .brand-logo-icon {
  transform: scale(1.05);
}

.brand-text-wrap {
  margin-left: 12px;
  overflow: hidden;
}

.brand-title {
  font-size: 15px;
  font-weight: 700;
  color: #0f172a;
  letter-spacing: 0.3px;
  line-height: 1.2;
}

.brand-subtitle {
  font-size: 11px;
  color: #64748b;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  margin-top: 3px;
  font-weight: 400;
}

.aside-menu-scrollbar {
  flex: 1;
  overflow-x: hidden;
}

.el-menu-vertical {
  border-right: none !important;
  background-color: transparent !important;
  padding: 10px 0;
}

/* 菜单项基础与悬浮效果：明亮护眼微圆角 */
:deep(.el-menu-item),
:deep(.el-sub-menu__title) {
  height: 42px;
  line-height: 42px;
  margin: 3px 10px;
  border-radius: 8px;
  color: #475569 !important;
  font-size: 13.5px;
  font-weight: 500;
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
}

:deep(.el-menu-item:hover),
:deep(.el-sub-menu__title:hover) {
  background: #f1f5f9 !important;
  color: #0f172a !important;
}

:deep(.el-menu-item:hover .el-icon),
:deep(.el-sub-menu__title:hover .el-icon) {
  color: #2563eb !important;
}

/* 激活高亮：清爽天空蓝微润胶囊底色与左指示线 */
:deep(.el-menu-item.is-active) {
  background: #eff6ff !important;
  color: #2563eb !important;
  font-weight: 600;
  position: relative;
  box-shadow: inset 3px 0 0 0 #2563eb;
}

:deep(.el-menu-item.is-active .el-icon) {
  color: #2563eb !important;
}

/* 二级菜单背景：淡雅暖灰收纳槽与平滑缩进 */
:deep(.el-sub-menu .el-menu) {
  background-color: #f8fafc !important;
  padding: 4px 6px;
  margin: 2px 8px;
  border-radius: 8px;
  border: 1px solid #f1f5f9;
}

:deep(.el-sub-menu .el-menu-item) {
  padding-left: 38px !important;
  height: 38px;
  line-height: 38px;
  font-size: 13px;
  color: #64748b !important;
  border-radius: 6px;
  margin: 2px 0;
}

:deep(.el-sub-menu .el-menu-item:hover) {
  background-color: #f1f5f9 !important;
  color: #0f172a !important;
}

:deep(.el-sub-menu .el-menu-item.is-active) {
  background-color: #e0f2fe !important;
  color: #0284c7 !important;
  font-weight: 600;
  box-shadow: none;
}

.menu-icon {
  font-size: 17px;
  margin-right: 10px;
  color: #64748b;
  transition: color 0.2s;
}

.menu-badge {
  margin-left: auto;
  transform: scale(0.85);
}

.inner-menu-tag {
  margin-left: auto;
  font-size: 10px;
  height: 18px;
  padding: 0 5px;
  line-height: 16px;
  transform: scale(0.85);
  border-radius: 4px;
}

/* 侧边栏底部折叠栏：纯白轻边界 */
.aside-footer {
  height: 46px;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  color: #64748b;
  font-size: 12.5px;
  border-top: 1px solid #f1f5f9;
  background: #ffffff;
  cursor: pointer;
  user-select: none;
  transition: all 0.2s ease;
}

.aside-footer:hover {
  background: #f8fafc;
  color: #2563eb;
}

.collapse-icon {
  font-size: 16px;
}

/* ===================================================
   2. 顶部 Header：现代高透毛玻璃卡片风格
   =================================================== */
.inner-container {
  display: flex;
  flex: 1;
  flex-direction: column !important;
  height: 100%;
  overflow: hidden;
  min-width: 0;
}

.layout-header {
  height: 64px !important;
  background-color: rgba(255, 255, 255, 0.92);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  border-bottom: 1px solid #e2e8f0;
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0 24px;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.04);
  flex-shrink: 0;
  z-index: 50;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 14px;
}

.portal-return-link {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  font-weight: 600;
  color: #3b82f6;
  background: #eff6ff;
  border: 1px solid #bfdbfe;
  padding: 3px 10px 3px 6px;
  border-radius: 20px;
  text-decoration: none;
  transition: all 0.2s ease;
}

.portal-return-link:hover {
  background: #dbeafe;
  border-color: #93c5fd;
  transform: translateY(-1px);
}

.portal-pill-badge {
  background: #2563eb;
  color: #ffffff;
  font-size: 9px;
  font-weight: 800;
  padding: 1px 5px;
  border-radius: 10px;
  letter-spacing: 0.5px;
}

.layout-breadcrumb {
  font-size: 13.5px;
}

:deep(.layout-breadcrumb .el-breadcrumb__inner) {
  display: flex;
  align-items: center;
  color: #64748b;
  font-weight: 500;
}

:deep(.layout-breadcrumb .el-breadcrumb__inner:hover) {
  color: #3b82f6;
}

.crumb-icon {
  margin-right: 4px;
  font-size: 14px;
}

.active-crumb {
  color: #0f172a !important;
  font-weight: 600;
}

.header-right {
  display: flex;
  align-items: center;
  gap: 14px;
}

/* 全局搜索快捷触发条 */
.quick-search-trigger {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  background: #f1f5f9;
  border: 1px solid #e2e8f0;
  border-radius: 20px;
  color: #64748b;
  font-size: 12.5px;
  cursor: pointer;
  transition: all 0.2s ease;
}

.quick-search-trigger:hover {
  background: #e2e8f0;
  border-color: #cbd5e1;
  color: #1e293b;
}

.shortcut-key {
  background: #ffffff;
  border: 1px solid #cbd5e1;
  border-radius: 4px;
  padding: 1px 5px;
  font-size: 11px;
  font-family: monospace;
  color: #64748b;
  box-shadow: 0 1px 1px rgba(0,0,0,0.06);
}

.header-tool-btn {
  display: flex;
  align-items: center;
  cursor: pointer;
}

.text-tool-btn {
  padding: 6px 10px;
  border-radius: 6px;
  font-size: 13px;
  color: #475569;
  font-weight: 500;
  transition: all 0.2s;
}

.text-tool-btn:hover {
  background: #f1f5f9;
  color: #3b82f6;
}

.bell-btn {
  border: 1px solid #e2e8f0;
  background: #f8fafc;
  color: #64748b;
  transition: all 0.2s;
}

.bell-btn:hover {
  background: #eff6ff;
  border-color: #3b82f6;
  color: #3b82f6;
}

.header-divider {
  width: 1px;
  height: 24px;
  background: #e2e8f0;
}

/* 用户头像胶囊 */
.user-profile-badge {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 4px 10px 4px 6px;
  border-radius: 24px;
  background: #f8fafc;
  border: 1px solid #e2e8f0;
  cursor: pointer;
  transition: all 0.2s ease;
}

.user-profile-badge:hover {
  background: #f1f5f9;
  border-color: #cbd5e1;
}

.avatar-box {
  position: relative;
  display: flex;
  align-items: center;
}

.custom-avatar {
  background: linear-gradient(135deg, #3b82f6 0%, #2563eb 100%);
  font-weight: 600;
  color: #ffffff;
}

.status-dot {
  position: absolute;
  bottom: 0;
  right: 0;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #10b981;
  border: 1.5px solid #ffffff;
}

.user-text-info {
  display: flex;
  align-items: center;
  gap: 6px;
}

.user-name {
  font-size: 13.5px;
  font-weight: 600;
  color: #1e293b;
}

.role-pill {
  font-size: 11px;
  height: 20px;
  line-height: 18px;
  padding: 0 6px;
  border-radius: 10px;
}

.arrow-icon {
  font-size: 12px;
  color: #94a3b8;
}

.custom-dropdown-menu {
  padding: 6px;
  border-radius: 8px;
  box-shadow: 0 10px 25px -5px rgba(0, 0, 0, 0.1), 0 8px 10px -6px rgba(0, 0, 0, 0.1);
}

.user-dropdown-header {
  padding: 8px 14px 12px 14px;
  border-bottom: 1px solid #f1f5f9;
  margin-bottom: 4px;
}

.u-name {
  font-size: 14px;
  font-weight: 700;
  color: #0f172a;
}

.u-role {
  font-size: 12px;
  color: #64748b;
  margin-top: 2px;
}

.logout-item {
  color: #ef4444 !important;
}

/* ===================================================
   3. 主内容区域与页面切换动效
   =================================================== */
.layout-main {
  background-color: #f8fafc;
  padding: 20px 24px;
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  box-sizing: border-box;
  min-width: 0;
  width: 100%;
}

.fade-transform-enter-active,
.fade-transform-leave-active {
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
}

.fade-transform-enter-from {
  opacity: 0;
  transform: translateY(6px);
}

.fade-transform-leave-to {
  opacity: 0;
  transform: translateY(-6px);
}

/* 通知中心列表 */
.notification-card {
  padding: 4px 0;
}

.n-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding-bottom: 10px;
  border-bottom: 1px solid #f1f5f9;
  margin-bottom: 10px;
}

.n-title {
  display: flex;
  align-items: center;
  font-weight: 600;
  font-size: 14px;
  color: #0f172a;
}

.n-body {
  max-height: 360px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.n-item {
  padding: 10px 12px;
  border-radius: 6px;
  background: #f8fafc;
  border: 1px solid #e2e8f0;
  cursor: pointer;
  transition: all 0.2s ease;
}

.n-item:hover {
  background: #eff6ff;
  border-color: #3b82f6;
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(59, 130, 246, 0.12);
}

.n-item-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 4px;
}

.n-item-type {
  font-weight: 600;
  color: #3b82f6;
  font-size: 12px;
}

.n-item-id {
  font-family: monospace;
  font-size: 12px;
  color: #475569;
  flex: 1;
  margin: 0 6px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.n-item-meta {
  font-size: 12px;
  color: #334155;
  margin-bottom: 4px;
}

.n-item-desc {
  font-size: 12px;
  color: #64748b;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  margin-bottom: 6px;
}

.n-item-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 11px;
  color: #94a3b8;
}

.n-footer {
  margin-top: 10px;
  padding-top: 8px;
  border-top: 1px solid #f1f5f9;
}

/* 全局快捷搜索弹窗 */
.quick-search-dialog :deep(.el-dialog__header) {
  padding: 16px 20px 10px 20px;
}

.quick-search-dialog :deep(.el-dialog__body) {
  padding: 10px 20px 20px 20px;
}

.search-dialog-input {
  margin-bottom: 14px;
}

.search-results-list {
  max-height: 380px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.search-result-item {
  display: flex;
  align-items: center;
  padding: 10px 12px;
  border-radius: 6px;
  background: #f8fafc;
  border: 1px solid #e2e8f0;
  cursor: pointer;
  transition: all 0.2s ease;
}

.search-result-item:hover {
  background: #eff6ff;
  border-color: #3b82f6;
  transform: translateX(3px);
}

.s-icon {
  font-size: 20px;
  margin-right: 12px;
}

.s-meta {
  flex: 1;
}

.s-title {
  font-size: 13.5px;
  font-weight: 600;
  color: #1e293b;
}

.s-desc {
  font-size: 12px;
  color: #64748b;
  margin-top: 2px;
}
</style>
