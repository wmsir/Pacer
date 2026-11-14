/**
 * Vue Router 路由配置
 *
 * 定义系统中的所有页面跳转规则。
 */
import { createRouter, createWebHistory } from 'vue-router'
// 登录页与框架页直接打包；其余页面按需加载，避免登录页就下载 Monaco、BPMN、ECharts 等大依赖
import Login from '../views/Login.vue'
import Layout from '../views/Layout.vue'

import { useUserStore } from '../store/user'

// 路由规则表
const routes = [
  {
    path: '/',
    redirect: '/login'
  },
  {
    path: '/login',
    name: 'Login',
    component: Login
  },
  {
    path: '/',
    component: Layout,
    meta: { requiresAuth: true },
    children: [
      {
        path: 'dashboard',
        name: 'Dashboard',
        component: () => import('../views/Dashboard.vue')
      },
      // 工单中心
      {
        path: 'ticket-list',
        name: 'TicketList',
        component: () => import('../views/TicketList.vue')
      },
      {
        path: 'ticket-create',
        name: 'CreateTicket',
        component: () => import('../views/CreateTicket.vue')
      },
      {
        path: 'ticket/create',
        redirect: '/ticket-create'
      },
      {
        path: 'ticket/:id',
        name: 'TicketDetail',
        component: () => import('../views/TicketDetail.vue')
      },
      {
        path: 'ai-sql-review',
        name: 'AiSqlReview',
        component: () => import('../views/AiSqlReview.vue')
      },
      // 数据操作 & 安全
      {
        path: 'data-query',
        name: 'DataQuery',
        component: () => import('../views/DataQuery.vue')
      },
      {
        path: 'sql-query',
        redirect: '/data-query'
      },
      {
        path: 'data-masking',
        name: 'DataMaskingConfig',
        component: () => import('../views/DataMaskingConfig.vue')
      },
      {
        path: 'audit-dashboard',
        name: 'AuditDashboard',
        component: () => import('../views/AuditDashboard.vue')
      },
      // 实例管理子模块
      {
        path: 'instance-list',
        name: 'InstanceList',
        component: () => import('../views/InstanceList.vue')
      },
      {
        path: 'instance/create',
        name: 'InstanceCreate',
        component: () => import('../views/InstanceEdit.vue')
      },
      {
        path: 'instance/edit/:id',
        name: 'InstanceEdit',
        component: () => import('../views/InstanceEdit.vue')
      },
      {
        path: 'instance-databases',
        name: 'DatabaseManage',
        component: () => import('../views/DatabaseManage.vue')
      },
      {
        path: 'instance/databases',
        redirect: '/instance-databases'
      },
      {
        path: 'instance-sessions',
        name: 'SessionManage',
        component: () => import('../views/SessionManage.vue')
      },
      {
        path: 'instance/sessions',
        redirect: '/instance-sessions'
      },
      {
        path: 'instance-accounts',
        name: 'AccountManage',
        component: () => import('../views/AccountManage.vue')
      },
      {
        path: 'instance/accounts',
        redirect: '/instance-accounts'
      },
      {
        path: 'instance-config',
        name: 'InstanceParamConfig',
        component: () => import('../views/InstanceList.vue')
      },
      {
        path: 'instance-params',
        name: 'ParamConfig',
        component: () => import('../views/ParamConfig.vue')
      },
      {
        path: 'instance-global-params',
        redirect: '/instance-params'
      },
      {
        path: 'instance/params',
        redirect: '/instance-params'
      },
      // 权限与组织
      {
        path: 'resource-group-list',
        name: 'ResourceGroupList',
        component: () => import('../views/ResourceGroupList.vue')
      },
      {
        path: 'user-list',
        name: 'UserList',
        component: () => import('../views/UserList.vue')
      },
      {
        path: 'role-list',
        name: 'RoleList',
        component: () => import('../views/RoleList.vue')
      },
      // 系统与流程
      {
        path: 'workflow-designer',
        name: 'WorkflowDesigner',
        component: () => import('../views/WorkflowDesigner.vue')
      },
      {
        path: 'license',
        name: 'License',
        component: () => import('../views/License.vue')
      },
      {
        path: 'settings',
        name: 'Settings',
        component: () => import('../views/Settings.vue')
      },
      {
        path: 'ai-config',
        name: 'AiConfig',
        component: () => import('../views/AiConfig.vue')
      },
      {
        path: 'ai/config',
        redirect: '/ai-config'
      },
      {
        path: 'notification-config',
        name: 'NotificationConfig',
        component: () => import('../views/NotificationConfig.vue')
      },
      {
        path: 'notification/config',
        redirect: '/notification-config'
      },
      // 个人中心
      {
        path: 'user-profile',
        name: 'UserProfile',
        component: () => import('../views/UserProfile.vue')
      }
    ]
  },
  // 未知地址：回到登录页（已登录会再被守卫带到首页），而不是显示一片空白
  {
    path: '/:pathMatch(.*)*',
    redirect: '/login'
  }
]

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes
})

import { ElMessage } from 'element-plus'

// 全局前置路由守卫
router.beforeEach(async (to, _from, next) => {
  const userStore = useUserStore()

  if (to.meta.requiresAuth && !userStore.isAuthenticated) {
    userStore.logout()
    next({ name: 'Login' })
  } else if (to.name === 'Login' && userStore.isAuthenticated) {
    if (userStore.isAdmin || userStore.userRole?.toUpperCase() === 'ADMIN' || userStore.userRole?.toUpperCase() === 'ROLE_ADMIN') {
      next({ name: 'Dashboard' })
    } else {
      next({ name: 'CreateTicket' })
    }
  } else if (to.meta.requiresAuth && userStore.isAuthenticated) {
    // 刷新页面或从 SSO 进入时，先取一次真实的角色与权限再判断，避免按本地缓存误放行或误拦截
    if (!userStore.userInfo && !userStore.userInfoAttempted) {
      await userStore.fetchUserInfo()
    }

    // 只有超级管理员才可访问平台总览 (/dashboard)，其他角色访问时重定向至新建工单
    if (to.path === '/dashboard' && !userStore.isAdmin) {
      next({ path: '/ticket-create' })
      return
    }

    // 基础放行路径（如个人中心、工单详情页）
    if (to.path === '/dashboard' || to.path === '/user-profile' || to.path.startsWith('/ticket/') || to.path.startsWith('/instance/edit/')) {
      next()
      return
    }

    if (!userStore.hasPermission(to.path)) {
      ElMessage.warning({
        message: `您所属角色【${userStore.userRole}】无权访问【${String(to.name || to.path)}】页签功能`,
        grouping: true
      })
      // 没有新建工单权限的账号以前会在这里无限重定向，改为落到个人中心
      const fallback = userStore.isAdmin ? '/dashboard' : (userStore.hasPermission('/ticket-create') ? '/ticket-create' : '/user-profile')
      next({ path: fallback })
      return
    }
    next()
  } else {
    next()
  }
})

// 只在登录页显示 index.html 里的「主站门户 / 架构专页 / 移动审批端」悬浮条，
// 进入控制台后它会遮住表格分页和操作按钮
router.afterEach((to) => {
  document.body.classList.toggle('route-login', to.name === 'Login')
})

// 发布新版本后，旧页面里的按需加载文件名已失效：自动刷新一次拿到新版本
router.onError((error, to) => {
  const msg = String(error?.message || '')
  if (/Failed to fetch dynamically imported module|Importing a module script failed|error loading dynamically imported module/i.test(msg)) {
    const key = 'pacer_chunk_reload'
    if (sessionStorage.getItem(key) !== to.fullPath) {
      sessionStorage.setItem(key, to.fullPath)
      window.location.assign(router.resolve(to.fullPath).href)
    }
  }
})

export default router
