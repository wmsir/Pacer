/**
 * 系统功能页签与权限定义清单
 *
 * 规范定义系统中的 6 大核心模块与 18 个子页签的路由、名称、图标与说明。
 */

export interface PermissionNode {
  id: string // 路由路径或权限唯一标识 (如 '/dashboard', '/ticket-list')
  name: string // 菜单/页签中文名称
  icon?: string
  description?: string
  children?: PermissionNode[]
}

export const SYSTEM_PERMISSION_TREE: PermissionNode[] = [
  {
    id: '/dashboard',
    name: '平台总览 (Dashboard)',
    icon: 'Odometer',
    description: '系统整体运行概览、执行统计、待办审批与拦截监控大盘'
  },
  {
    id: '/tickets',
    name: '工单中心 (Ticket Center)',
    icon: 'Document',
    description: '工单申请、审批流转、AI 审查与工单生命周期追踪',
    children: [
      { id: '/ticket-list', name: '工单列表', description: '查看、检索与筛选系统历史与进行中工单' },
      { id: '/ticket-create', name: '新建 SQL 工单', description: '提交 DML/DDL 变更工单与执行事务级预校验 (Dry-Run)' },
      { id: '/ai-sql-review', name: 'AI 智能审核', description: '基于大模型的 SQL 性能诊断与安全规则扫描' }
    ]
  },
  {
    id: '/data',
    name: '数据操作与安全 (Data & Security)',
    icon: 'Search',
    description: '线上数据查询、脱敏规则配置与 SQL 审计合规大盘',
    children: [
      { id: '/data-query', name: '数据查询控制台', description: '在线 SQL 安全查询与大结果集分页浏览' },
      { id: '/data-masking', name: '动态脱敏配置', description: '手机号、身份证、银行卡等脱敏规则策略配置' },
      { id: '/audit-dashboard', name: 'SQL 审计大盘', description: '全站 SQL 操作审计日志、拦截监控与合规看板' }
    ]
  },
  {
    id: '/instances',
    name: '实例与资源管理 (Instances & Resources)',
    icon: 'Coin',
    description: '数据库实例纳管、业务资源组划分、会话监控与核心管控',
    children: [
      { id: '/instance-list', name: '实例列表', description: '数据库实例注册、连通性探测与环境分配' },
      { id: '/resource-group-list', name: '业务资源组', description: '业务线与实例绑定划分、多资源组统一管理' },
      { id: '/instance-databases', name: '数据库管理', description: 'Schema 数据库清单与字符集空间管理' },
      { id: '/instance-accounts', name: '账号权限管理', description: '数据库原生用户账号与特权授权管理' },
      { id: '/instance-sessions', name: '会话监控与强杀', description: '活跃连接监控、慢查询排查与会话强杀' },
      { id: '/instance-params', name: '全局参数查看', description: '数据库全局运行变量 (SHOW VARIABLES) 与平台安全策略' },
      { id: '/instance-config', name: '安全参数策略', description: '实例执行安全参数、工单定制扩展字段与必填约束配置' }
    ]
  },
  {
    id: '/perms',
    name: '用户与权限 (Users & Permissions)',
    icon: 'UserFilled',
    description: '人员组织架构分配与系统角色功能权限矩阵配置',
    children: [
      { id: '/user-list', name: '用户管理', description: '系统人员账号、姓名标识、组织分配与状态维护' },
      { id: '/role-list', name: '角色与权限', description: '系统角色定义与页签功能权限矩阵配置' }
    ]
  },
  {
    id: '/system',
    name: '系统与配置 (System & Configuration)',
    icon: 'Operation',
    description: '工作流引擎 BPMN 可视化设计、AI 模型接入、授权证书与系统主题',
    children: [
      { id: '/workflow-designer', name: '流程设计与模板', description: 'BPMN 2.0 拓扑编排、排他网关条件路由与模板绑定' },
      { id: '/ai-config', name: 'AI 模型配置', description: '主流大模型提供商（DeepSeek、通义千问、OpenAI、智谱、文心、Kimi、Ollama）与自定义大模型接入及连通自检' },
      { id: '/notification-config', name: '消息通知与告警配置', description: '企业微信工作消息 (WSDL/SOAP)、阿里钉钉群机器人、紧急电话智能语音外呼及推送频次策略配置' },
      { id: '/license', name: '授权证书', description: '商业版证书鉴权、授权节点数与到期状态' },
      { id: '/settings', name: '自定义主题与水印', description: '系统外观风格、主色调与界面个性化防泄密水印定制' }
    ]
  }
]

/**
 * 获取所有具体子页签的路由路径清单（扁平数组）
 */
export const getAllTabKeys = (): string[] => {
  const keys: string[] = []
  const traverse = (nodes: PermissionNode[]) => {
    nodes.forEach(node => {
      if (node.children && node.children.length > 0) {
        traverse(node.children)
      } else {
        keys.push(node.id)
      }
    })
  }
  traverse(SYSTEM_PERMISSION_TREE)
  return keys
}

export interface PermissionSummaryResult {
  isAll: boolean
  totalCount: number
  modules: { name: string; count: number; total: number }[]
}

/**
 * 解析角色权限 JSON 字符串并统计模块页签归属与全量标识
 */
export const getPermissionSummary = (permsStr: string): PermissionSummaryResult => {
  if (!permsStr || permsStr === '[]' || permsStr === '""') {
    return { isAll: false, totalCount: 0, modules: [] }
  }
  try {
    const parsed = typeof permsStr === 'string' ? JSON.parse(permsStr) : permsStr
    if (Array.isArray(parsed)) {
      if (parsed.includes('*')) {
        return { isAll: true, totalCount: getAllTabKeys().length, modules: [] }
      }
      const permSet = new Set(parsed)
      const modules: { name: string; count: number; total: number }[] = []
      let totalCount = 0

      SYSTEM_PERMISSION_TREE.forEach(moduleNode => {
        if (moduleNode.children && moduleNode.children.length > 0) {
          const childKeys = moduleNode.children.map(c => c.id)
          const matched = childKeys.filter(k => permSet.has(k))
          if (matched.length > 0) {
            modules.push({
              name: moduleNode.name.split(' ')[0],
              count: matched.length,
              total: childKeys.length
            })
            totalCount += matched.length
          }
        } else {
          if (permSet.has(moduleNode.id)) {
            modules.push({
              name: moduleNode.name.split(' ')[0],
              count: 1,
              total: 1
            })
            totalCount += 1
          }
        }
      })

      return { isAll: false, totalCount, modules }
    }
  } catch (e) {
    if (permsStr.includes('*')) {
      return { isAll: true, totalCount: getAllTabKeys().length, modules: [] }
    }
  }
  return { isAll: false, totalCount: 0, modules: [] }
}

/**
 * 预置角色标准页签权限包定义
 */
export const PRESET_PERMISSION_PACKAGES: Record<string, { name: string; icon: string; desc: string; paths: string[] }> = {
  ADMIN: {
    name: '超级管理员全权包',
    icon: '👑',
    desc: '拥有平台全部 18 个功能页签与特权管理权限',
    paths: ['*']
  },
  DBA: {
    name: '核心 DBA 运维管理包',
    icon: '💾',
    desc: '涵盖工单中心、数据查询、全量实例管理、业务资源组、BPMN 流程设计与 AI 模型配置',
    paths: [
      '/dashboard',
      '/ticket-list',
      '/ticket-create',
      '/ai-sql-review',
      '/data-query',
      '/data-masking',
      '/audit-dashboard',
      '/instance-list',
      '/resource-group-list',
      '/instance-databases',
      '/instance-accounts',
      '/instance-sessions',
      '/instance-params',
      '/instance-config',
      '/workflow-designer',
      '/ai-config',
      '/notification-config'
    ]
  },
  DEV_LEAD: {
    name: '研发主管/开发组长审批包',
    icon: '🛡️',
    desc: '涵盖工单初审、SQL 创建、数据查询、资源组与审批流查看',
    paths: [
      '/dashboard',
      '/ticket-list',
      '/ticket-create',
      '/ai-sql-review',
      '/data-query',
      '/data-masking',
      '/resource-group-list',
      '/workflow-designer'
    ]
  },
  DEV: {
    name: '开发工程师自主包',
    icon: '💻',
    desc: '拥有个人工单提交、预执行校验、AI 诊断与数据查询控制台',
    paths: [
      '/ticket-list',
      '/ticket-create',
      '/ai-sql-review',
      '/data-query'
    ]
  },
  AUDITOR: {
    name: '安全与合规审计员包',
    icon: '🔍',
    desc: '涵盖 SQL 变更审计、敏感数据脱敏查看、平台审计日志与工单合规核验',
    paths: [
      '/dashboard',
      '/ticket-list',
      '/audit-dashboard',
      '/data-masking'
    ]
  },
  OPS: {
    name: 'SRE / 系统运维保障包',
    icon: '⚡',
    desc: '涵盖实例列表、会话排查、参数查看与系统监控',
    paths: [
      '/dashboard',
      '/instance-list',
      '/resource-group-list',
      '/instance-sessions',
      '/instance-params'
    ]
  }
}
