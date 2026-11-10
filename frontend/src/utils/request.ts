import axios from 'axios'
import { ElMessage } from 'element-plus'
import { useUserStore } from '../store/user'
import router from '../router'

// 创建 axios 实例
const request = axios.create({
  baseURL: '/pacer/api',
  timeout: 30000 // 请求超时时间增加至 30 秒（适应 SQL 执行、AI 分析与多库探测）
})

// request 拦截器
request.interceptors.request.use(
  config => {
    const userStore = useUserStore()
    if (userStore.token) {
      config.headers['Authorization'] = `Bearer ${userStore.token}` // 让每个请求携带自定义 token
    }

    // 规范化 URL，防止重复添加前缀导致 404
    if (config.url && config.url.startsWith('/pacer/api/')) {
      config.url = config.url.substring(10)
    } else if (config.url && config.url.startsWith('/api/')) {
      config.url = config.url.substring(4)
    }

    // 注入多租户上下文，生产环境应从登录信息或域名解析中动态获取
    const tenantId = localStorage.getItem('tenantId') || 'public'
    config.headers['X-Tenant-Id'] = tenantId

    return config
  },
  error => {
    console.error('Request interceptor error:', error)
    return Promise.reject(error)
  }
)

// response 拦截器
request.interceptors.response.use(
  response => {
    const res = response.data

    // 如果返回的 code 是 A0220 或 A0200，说明登录失效
    if (res.code === 'A0220' || res.code === 'A0200') {
      const userStore = useUserStore()
      userStore.logout()
      ElMessage.error(res.message || '登录身份已过期，请重新登录')
      router.push('/login')
      return Promise.reject(new Error(res.message || '登录已过期'))
    }

    // 如果返回的 code 不是 00000 或 200，说明业务接口有异常
    if (res.code && res.code !== 200 && res.code !== '00000') {
      ElMessage({
        message: res.message || 'Error',
        type: 'error',
        duration: 5 * 1000
      })
      return Promise.reject(new Error(res.message || 'Error'))
    } else {
      // 成功则直接返回内部 data（处理统一 Response Result 的解包）
      return res.data !== undefined ? res : response
    }
  },
  error => {
    console.error('Response interceptor error:', error)

    let message = error.message
    if (error.code === 'ECONNABORTED' || (error.message && error.message.includes('timeout'))) {
      message = '网络请求超时（>30s），请检查后端服务运行状态或目标数据库连通性'
    } else if (error.response) {
      const status = error.response.status
      if (error.response.data && error.response.data.message) {
        message = error.response.data.message
      }

      if (status === 401) {
        message = '登录身份已失效或未登录，请重新登录'
        const userStore = useUserStore()
        userStore.logout()
        router.push('/login')
      } else if (status === 403) {
        const userStore = useUserStore()
        if (!userStore.isAuthenticated) {
          message = '未登录或登录已失效，请重新登录'
          userStore.logout()
          router.push('/login')
        } else {
          message = error.response.data?.message || '当前账号权限不足，拒绝访问'
        }
      } else if (status === 500) {
        message = error.response.data?.message || '系统内部异常'
      } else if (status === 400) {
        message = error.response.data?.message || '请求参数有误或操作被拦截'
      }
    }

    ElMessage({
      message: message,
      type: 'error',
      duration: 4 * 1000
    })
    return Promise.reject(error)
  }
)

export default request
