import { defineStore } from 'pinia'
import { jwtDecode } from 'jwt-decode'
import request from '../utils/request'

const getInitialToken = () => {
  try {
    const searchParams = new URLSearchParams(window.location.search)
    const hashQuery = window.location.hash.includes('?') ? window.location.hash.split('?')[1] : ''
    const hashParams = new URLSearchParams(hashQuery)
    const sso = searchParams.get('sso_token') || hashParams.get('sso_token')
    if (sso) {
      localStorage.setItem('wmdb_token', sso)
      localStorage.setItem('fwpt_token', sso)
      return sso
    }
  } catch (e) {}
  return localStorage.getItem('wmdb_token') || localStorage.getItem('fwpt_token') || ''
}

export const useUserStore = defineStore('user', {
  state: () => ({
    token: getInitialToken(),
    userRole: localStorage.getItem('wmdb_role') || 'ADMIN',
    permissions: JSON.parse(localStorage.getItem('wmdb_permissions') || '["*"]') as string[],
    userInfo: null as any
  }),
  getters: {
    isAuthenticated: (state) => {
      if (!state.token) return false
      try {
        const decoded: any = jwtDecode(state.token)
        if (decoded.exp && decoded.exp * 1000 < Date.now()) {
          return false
        }
        return true
      } catch (e) {
        return false
      }
    },
    realName: (state) => {
      if (state.userInfo?.realName) {
        return state.userInfo.realName
      }
      if (!state.token) return ''
      try {
        const decoded: any = jwtDecode(state.token)
        return decoded.realName || decoded.sub || ''
      } catch (e) {
        return ''
      }
    },
    isAdmin: (state) => {
      return state.userRole === 'ADMIN' || state.permissions.includes('*')
    }
  },
  actions: {
    setToken(token: string) {
      this.token = token
      localStorage.setItem('wmdb_token', token)
      this.fetchUserInfo()
    },
    logout() {
      this.token = ''
      this.userRole = 'DEV'
      this.permissions = []
      this.userInfo = null
      localStorage.removeItem('wmdb_token')
      localStorage.removeItem('wmdb_role')
      localStorage.removeItem('wmdb_permissions')
    },
    async fetchUserInfo() {
      if (!this.token) return
      try {
        const res: any = await request.get('/v1/auth/user-info')
        if (res.data) {
          this.userInfo = res.data
          this.userRole = res.data.role || 'DEV'
          this.permissions = Array.isArray(res.data.permissions) ? res.data.permissions : ['*']
          localStorage.setItem('wmdb_role', this.userRole)
          localStorage.setItem('wmdb_permissions', JSON.stringify(this.permissions))
        }
      } catch (e) {
        // fallback
      }
    },
    hasPermission(path: string): boolean {
      if (this.isAdmin) return true
      if (this.permissions.includes('*')) return true
      return this.permissions.includes(path)
    }
  }
})
