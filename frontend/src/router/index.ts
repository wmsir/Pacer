/**
 * Vue Router 路由配置
 *
 * 定义系统中的所有页面跳转规则。
 */
import { createRouter, createWebHistory } from 'vue-router'
import Login from '../views/Login.vue'
import Layout from '../views/Layout.vue'
import Dashboard from '../views/Dashboard.vue'
import TicketList from '../views/TicketList.vue'
import CreateTicket from '../views/CreateTicket.vue'
import DataQuery from '../views/DataQuery.vue'
import DataMaskingConfig from '../views/DataMaskingConfig.vue'
import AuditDashboard from '../views/AuditDashboard.vue'
import TicketDetail from '../views/TicketDetail.vue'
import WorkflowDesigner from '../views/WorkflowDesigner.vue'
import License from '../views/License.vue'
import AiSqlReview from '../views/AiSqlReview.vue'
import Settings from '../views/Settings.vue'
import InstanceList from '../views/InstanceList.vue'
import InstanceEdit from '../views/InstanceEdit.vue'
import DatabaseManage from '../views/DatabaseManage.vue'
import SessionManage from '../views/SessionManage.vue'
import AccountManage from '../views/AccountManage.vue'
import ParamConfig from '../views/ParamConfig.vue'
import ResourceGroupList from '../views/ResourceGroupList.vue'
import UserList from '../views/UserList.vue'
import RoleList from '../views/RoleList.vue'
import UserProfile from '../views/UserProfile.vue'
import AiConfig from '../views/AiConfig.vue'
import NotificationConfig from '../views/NotificationConfig.vue'

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
        component: Dashboard
      },
      // 工单中心
      {
        path: 'ticket-list',
        name: 'TicketList',
        component: TicketList
      },
      {
        path: 'ticket-create',
        name: 'CreateTicket',
        component: CreateTicket
      },
      {
        path: 'ticket/create',
        redirect: '/ticket-create'
      },
      {
        path: 'ticket/:id',
        name: 'TicketDetail',
        component: TicketDetail
      },
      {
        path: 'ai-sql-review',
        name: 'AiSqlReview',
        component: AiSqlReview
      },
      // 数据操作 & 安全
      {
        path: 'data-query',
        name: 'DataQuery',
        component: DataQuery
      },
      {
        path: 'sql-query',
        redirect: '/data-query'
      },
      {
        path: 'data-masking',
        name: 'DataMaskingConfig',
        component: DataMaskingConfig
      },
      {
        path: 'audit-dashboard',
        name: 'AuditDashboard',
        component: AuditDashboard
      },
      // 实例管理子模块
      {
        path: 'instance-list',
        name: 'InstanceList',
        component: InstanceList
      },
      {
        path: 'instance/create',
        name: 'InstanceCreate',
        component: InstanceEdit
      },
      {
        path: 'instance/edit/:id',
        name: 'InstanceEdit',
        component: InstanceEdit
      },
      {
        path: 'instance-databases',
        name: 'DatabaseManage',
        component: DatabaseManage
      },
      {
        path: 'instance/databases',
        redirect: '/instance-databases'
      },
      {
        path: 'instance-sessions',
        name: 'SessionManage',
        component: SessionManage
      },
      {
        path: 'instance/sessions',
        redirect: '/instance-sessions'
      },
      {
        path: 'instance-accounts',
        name: 'AccountManage',
        component: AccountManage
      },
      {
        path: 'instance/accounts',
        redirect: '/instance-accounts'
      },
      {
        path: 'instance-config',
        name: 'InstanceParamConfig',
        component: InstanceList
      },
      {
        path: 'instance-params',
        name: 'ParamConfig',
        component: ParamConfig
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
        component: ResourceGroupList
      },
      {
        path: 'user-list',
        name: 'UserList',
        component: UserList
      },
      {
        path: 'role-list',
        name: 'RoleList',
        component: RoleList
      },
      // 系统与流程
      {
        path: 'workflow-designer',
        name: 'WorkflowDesigner',
        component: WorkflowDesigner
      },
      {
        path: 'license',
        name: 'License',
        component: License
      },
      {
        path: 'settings',
        name: 'Settings',
        component: Settings
      },
      {
        path: 'ai-config',
        name: 'AiConfig',
        component: AiConfig
      },
      {
        path: 'ai/config',
        redirect: '/ai-config'
      },
      {
        path: 'notification-config',
        name: 'NotificationConfig',
        component: NotificationConfig
      },
      {
        path: 'notification/config',
        redirect: '/notification-config'
      },
      // 个人中心
      {
        path: 'user-profile',
        name: 'UserProfile',
        component: UserProfile
      }
    ]
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
      const fallback = userStore.isAdmin ? '/dashboard' : '/ticket-create'
      next({ path: fallback })
      return
    }
    next()
  } else {
    next()
  }
})

export default router
