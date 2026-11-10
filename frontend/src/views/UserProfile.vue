<template>
  <div class="user-profile-container page-container">
    <!-- 顶部个人名片 -->
    <el-card shadow="hover" class="profile-hero-card">
      <div class="hero-content">
        <div class="hero-avatar-wrap">
          <el-avatar :size="72" class="hero-avatar">
            {{ userProfile.realName ? userProfile.realName.substring(0, 1) : (userProfile.username ? userProfile.username.substring(0, 1) : '用') }}
          </el-avatar>
          <div class="avatar-badge">
            <el-icon><Check /></el-icon>
          </div>
        </div>
        <div class="hero-info">
          <div class="hero-name-row">
            <h2 class="hero-name">{{ userProfile.realName || userProfile.username }}</h2>
            <el-tag type="success" size="small" effect="dark" round>
              <el-icon style="margin-right: 4px;"><CircleCheckFilled /></el-icon>实名已认证
            </el-tag>
            <span class="hero-username">(@{{ userProfile.username }})</span>
          </div>
          <div class="hero-meta-row">
            <span v-if="userProfile.department" class="meta-item">
              <el-icon><OfficeBuilding /></el-icon>{{ userProfile.department }}
            </span>
            <span v-if="userProfile.jobNo" class="meta-item">
              <el-icon><Postcard /></el-icon>工号: {{ userProfile.jobNo }}
            </span>
            <span class="meta-item">
              <el-icon><User /></el-icon>系统角色:
              <el-tag
                v-for="r in (userProfile.roles || [userProfile.role || 'DEV'])"
                :key="r"
                size="small"
                :type="getRoleTagType(r)"
                style="margin-left: 4px;"
              >
                {{ formatRoleNameZh(r) }}
              </el-tag>
            </span>
          </div>
        </div>
        <div class="hero-actions">
          <el-button type="primary" :icon="Refresh" @click="loadUserProfile" :loading="loading">刷新档案</el-button>
        </div>
      </div>
    </el-card>

    <el-row :gutter="20" style="margin-top: 20px;">
      <!-- 左侧：国内常用联系方式与个人资料 -->
      <el-col :xs="24" :lg="15">
        <el-card shadow="hover" class="form-card">
          <template #header>
            <div class="card-title-row">
              <div class="card-title">
                <el-icon color="#409EFF"><UserFilled /></el-icon>
                <span>常用联系方式与个人信息维护</span>
              </div>
              <el-tag size="small" type="info">自主维护 · 即时生效</el-tag>
            </div>
          </template>

          <el-form ref="profileFormRef" :model="form" :rules="profileRules" label-width="115px" v-loading="loading">
            <el-divider content-position="left"><span class="divider-title">🏢 部门与员工工号</span></el-divider>
            <el-row :gutter="16">
              <el-col :xs="24" :sm="12">
                <el-form-item label="所属部门/产线">
                  <el-input v-model="form.department" placeholder="如: 财险核心研发部 / 运维支持中心" clearable />
                </el-form-item>
              </el-col>
              <el-col :xs="24" :sm="12">
                <el-form-item label="员工工号">
                  <el-input v-model="form.jobNo" placeholder="如: WM-88029" clearable />
                </el-form-item>
              </el-col>
            </el-row>

            <el-divider content-position="left"><span class="divider-title">📱 手机与电子邮箱</span></el-divider>
            <el-row :gutter="16">
              <el-col :xs="24" :sm="12">
                <el-form-item label="真实姓名" prop="realName">
                  <el-input v-model="form.realName" placeholder="用于工单申请与审批流转姓名展示" />
                </el-form-item>
              </el-col>
              <el-col :xs="24" :sm="12">
                <el-form-item label="手机号码" prop="phone">
                  <el-input v-model="form.phone" placeholder="用于短信接收验证码与高危告警" clearable />
                </el-form-item>
              </el-col>
            </el-row>

            <el-row :gutter="16">
              <el-col :xs="24" :sm="12">
                <el-form-item label="电子邮箱" prop="email">
                  <el-input v-model="form.email" placeholder="如: zhangsan@company.com" clearable />
                </el-form-item>
              </el-col>
              <el-col :xs="24" :sm="12">
                <el-form-item label="身份证号码">
                  <el-input :model-value="userProfile.idCard || '-'" disabled placeholder="实名认证信息（不可修改）" />
                </el-form-item>
              </el-col>
            </el-row>

            <el-divider content-position="left"><span class="divider-title">💬 微信 / 企业微信 / 钉钉 / 飞书</span></el-divider>
            <el-row :gutter="16">
              <el-col :xs="24" :sm="12">
                <el-form-item label="微信账号">
                  <el-input v-model="form.wechat" placeholder="个人微信号 (用于工单协同沟通)" clearable />
                </el-form-item>
              </el-col>
              <el-col :xs="24" :sm="12">
                <el-form-item label="企业微信">
                  <el-input v-model="form.workWechat" placeholder="企微 UserID 或企微账号 (接收应用推送)" clearable />
                </el-form-item>
              </el-col>
            </el-row>

            <el-row :gutter="16">
              <el-col :xs="24" :sm="12">
                <el-form-item label="钉钉账号">
                  <el-input v-model="form.dingtalk" placeholder="钉钉号或绑定手机号 (接收工作通知)" clearable />
                </el-form-item>
              </el-col>
              <el-col :xs="24" :sm="12">
                <el-form-item label="飞书账号">
                  <el-input v-model="form.feishu" placeholder="飞书邮箱或手机号" clearable />
                </el-form-item>
              </el-col>
            </el-row>

            <div style="text-align: right; margin-top: 10px;">
              <el-button type="primary" :icon="Check" :loading="savingProfile" @click="handleSaveProfile">
                保存资料修改
              </el-button>
            </div>
          </el-form>
        </el-card>

        <!-- 消息通知渠道偏好 -->
        <el-card shadow="hover" class="form-card" style="margin-top: 20px;">
          <template #header>
            <div class="card-title-row">
              <div class="card-title">
                <el-icon color="#E6A23C"><Bell /></el-icon>
                <span>工单流转与安全告警通知偏好</span>
              </div>
            </div>
          </template>

          <div class="notification-pref-grid">
            <div class="pref-item">
              <div class="pref-left">
                <div class="pref-name">🟢 企业微信工作消息提醒</div>
                <div class="pref-desc">当您提交的工单审批通过、被驳回或需要您审批时，通过企业微信应用推送</div>
              </div>
              <el-switch v-model="notifPrefs.wecom" active-text="开启" inactive-text="关闭" />
            </div>

            <div class="pref-item">
              <div class="pref-left">
                <div class="pref-name">🔵 钉钉工作通知提醒</div>
                <div class="pref-desc">工单状态流转、高危 SQL 拦截与定时执行结果推送到钉钉</div>
              </div>
              <el-switch v-model="notifPrefs.dingtalk" active-text="开启" inactive-text="关闭" />
            </div>

            <div class="pref-item">
              <div class="pref-left">
                <div class="pref-name">📧 电子邮箱邮件通知</div>
                <div class="pref-desc">每日待办工单汇总与工单归档审计凭证发送至绑定邮箱</div>
              </div>
              <el-switch v-model="notifPrefs.email" active-text="开启" inactive-text="关闭" />
            </div>

            <div class="pref-item">
              <div class="pref-left">
                <div class="pref-name">📱 手机短信紧急告警</div>
                <div class="pref-desc">生产库高危操作拦截、定时流式执行失败等紧急事件短信通知</div>
              </div>
              <el-switch v-model="notifPrefs.sms" active-text="开启" inactive-text="关闭" />
            </div>
          </div>

          <div style="text-align: right; margin-top: 14px;">
            <el-button type="warning" plain :icon="Check" :loading="savingProfile" @click="handleSaveProfile">
              保存通知偏好
            </el-button>
          </div>
        </el-card>
      </el-col>

      <!-- 右侧：组织权限只读视图 & 修改密码 -->
      <el-col :xs="24" :lg="9">
        <!-- 组织与权限全景 -->
        <el-card shadow="hover" class="form-card">
          <template #header>
            <div class="card-title-row">
              <div class="card-title">
                <el-icon color="#67C23A"><Lock /></el-icon>
                <span>组织架构与数据权限视图</span>
              </div>
            </div>
          </template>

          <div class="scope-info-box">
            <div class="scope-header">
              <span class="scope-label">🛡️ 当前工单数据可见范围：</span>
              <el-tag :type="getDataScopeTagType(dataScopeInfo.scope)" effect="dark" size="small">
                {{ dataScopeInfo.scopeName || '所属资源组工单' }}
              </el-tag>
            </div>
            <div class="scope-desc">{{ dataScopeInfo.description || '仅展示所属资源组及本人发起的工单数据' }}</div>
          </div>

          <div class="perm-section">
            <div class="perm-title">🏢 归属业务资源组 ({{ (userProfile.resourceGroups || []).length }}个)</div>
            <div class="perm-tags">
              <template v-if="(userProfile.resourceGroups || []).length > 0">
                <el-tag
                  v-for="rg in userProfile.resourceGroups"
                  :key="rg"
                  size="small"
                  type="warning"
                  effect="plain"
                  style="margin-right: 6px; margin-bottom: 6px;"
                >
                  {{ rg }}
                </el-tag>
              </template>
              <span v-else style="color: #94a3b8; font-size: 13px;">暂未分配业务资源组</span>
            </div>
          </div>

          <div class="perm-section" style="margin-top: 14px;">
            <div class="perm-title">👑 系统角色与特权</div>
            <div class="perm-tags">
              <el-tag
                v-for="r in (userProfile.roles || [userProfile.role || 'DEV'])"
                :key="r"
                size="small"
                :type="getRoleTagType(r)"
                effect="dark"
                style="margin-right: 6px; margin-bottom: 6px;"
              >
                {{ formatRoleNameZh(r) }}
              </el-tag>
            </div>
          </div>
        </el-card>

        <!-- 修改登录密码 -->
        <el-card shadow="hover" class="form-card" style="margin-top: 20px;">
          <template #header>
            <div class="card-title-row">
              <div class="card-title">
                <el-icon color="#F56C6C"><Key /></el-icon>
                <span>修改登录密码 (SM3 国密加密)</span>
              </div>
            </div>
          </template>

          <el-form ref="pwdFormRef" :model="pwdForm" :rules="pwdRules" label-width="95px">
            <el-form-item label="原登录密码" prop="oldPassword">
              <el-input
                v-model="pwdForm.oldPassword"
                type="password"
                show-password
                placeholder="请输入当前原密码"
              />
            </el-form-item>

            <el-form-item label="新登录密码" prop="newPassword">
              <el-input
                v-model="pwdForm.newPassword"
                type="password"
                show-password
                placeholder="不少于 6 位字母/数字组合"
              />
            </el-form-item>

            <el-form-item label="确认新密码" prop="confirmPassword">
              <el-input
                v-model="pwdForm.confirmPassword"
                type="password"
                show-password
                placeholder="请再次输入新密码"
              />
            </el-form-item>

            <div style="text-align: right; margin-top: 10px;">
              <el-button type="danger" :icon="Check" :loading="changingPwd" @click="handleChangePassword">
                确认修改密码
              </el-button>
            </div>
          </el-form>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import request from '../utils/request'
import { useUserStore } from '../store/user'
import {
  User,
  UserFilled,
  Lock,
  Key,
  Bell,
  Refresh,
  Check,
  CircleCheckFilled,
  OfficeBuilding,
  Postcard
} from '@element-plus/icons-vue'

const userStore = useUserStore()
const loading = ref(false)
const savingProfile = ref(false)
const changingPwd = ref(false)

const userProfile = ref<any>({})
const dataScopeInfo = ref<any>({
  scope: 'RESOURCE_GROUP',
  scopeName: '所属业务资源组工单',
  description: '仅展示归属业务资源组相关及本人发起的工单'
})

const form = reactive({
  realName: '',
  phone: '',
  email: '',
  wechat: '',
  workWechat: '',
  dingtalk: '',
  feishu: '',
  department: '',
  jobNo: ''
})

const notifPrefs = reactive({
  wecom: true,
  dingtalk: false,
  email: true,
  sms: true
})

const profileRules = {
  realName: [{ required: true, message: '真实姓名不能为空', trigger: 'blur' }]
}

const pwdForm = reactive({
  oldPassword: '',
  newPassword: '',
  confirmPassword: ''
})

const pwdRules = {
  oldPassword: [{ required: true, message: '请输入原登录密码', trigger: 'blur' }],
  newPassword: [
    { required: true, message: '请输入新密码', trigger: 'blur' },
    { min: 6, message: '新密码不能少于 6 位', trigger: 'blur' }
  ],
  confirmPassword: [
    { required: true, message: '请再次输入新密码', trigger: 'blur' },
    {
      validator: (_rule: any, value: string, callback: any) => {
        if (value !== pwdForm.newPassword) {
          callback(new Error('两次输入的新密码不一致'))
        } else {
          callback()
        }
      },
      trigger: 'blur'
    }
  ]
}

const profileFormRef = ref()
const pwdFormRef = ref()

const getRoleTagType = (role: string) => {
  if (!role) return 'info'
  const r = role.toUpperCase()
  if (r.includes('ADMIN')) return 'danger'
  if (r.includes('DBA')) return 'warning'
  if (r.includes('LEAD')) return 'primary'
  if (r.includes('AUDITOR') || r.includes('AUDIT')) return 'success'
  if (r.includes('OPS')) return 'success'
  return 'info'
}

const formatRoleNameZh = (role: string) => {
  if (!role) return '研发工程师'
  const r = role.toUpperCase().trim()
  if (r === 'ADMIN' || r === 'ROLE_ADMIN' || r.includes('ADMIN')) return '系统超级管理员'
  if (r === 'DBA' || r === 'ROLE_DBA' || r.includes('DBA')) return '核心 DBA'
  if (r === 'DEV_LEAD' || r === 'LEAD' || r.includes('LEAD')) return '开发组长 / 架构师'
  if (r === 'AUDITOR' || r === 'SECURITY_AUDITOR' || r.includes('AUDIT')) return '安全合规审计员'
  if (r === 'OPS' || r === 'ROLE_OPS' || r.includes('OPS')) return '运维工程师'
  if (r === 'DEV' || r === 'DEVELOPER' || r === 'ROLE_DEV') return '研发工程师'
  return role
}

const getDataScopeTagType = (scope: string) => {
  if (scope === 'ALL') return 'danger'
  if (scope === 'RESOURCE_GROUP') return 'warning'
  return 'primary'
}

const loadUserProfile = async () => {
  loading.value = true
  try {
    const res: any = await request.get('/v1/user/profile')
    if (res.data) {
      userProfile.value = res.data
      form.realName = res.data.realName || ''
      form.phone = res.data.phone || ''
      form.email = res.data.email || ''
      form.wechat = res.data.wechat || ''
      form.workWechat = res.data.workWechat || ''
      form.dingtalk = res.data.dingtalk || ''
      form.feishu = res.data.feishu || ''
      form.department = res.data.department || ''
      form.jobNo = res.data.jobNo || ''

      if (res.data.notificationPrefs) {
        try {
          const parsed = JSON.parse(res.data.notificationPrefs)
          Object.assign(notifPrefs, parsed)
        } catch (e) {}
      }
    }

    // 获取工单数据可见范围说明
    const scopeRes: any = await request.get('/v1/ticket/data-scope')
    if (scopeRes.data) {
      dataScopeInfo.value = scopeRes.data
    }
  } catch (err: any) {
    ElMessage.error(err.message || '加载用户档案失败')
  } finally {
    loading.value = false
  }
}

const handleSaveProfile = async () => {
  if (!profileFormRef.value) return
  await profileFormRef.value.validate(async (valid: boolean) => {
    if (!valid) return
    savingProfile.value = true
    try {
      const payload = {
        ...form,
        notificationPrefs: JSON.stringify(notifPrefs)
      }
      await request.put('/v1/user/profile', payload)
      ElMessage.success('个人档案及国内联系方式已成功更新！')
      await userStore.fetchUserInfo()
      await loadUserProfile()
    } catch (err: any) {
      ElMessage.error(err.message || '保存资料失败')
    } finally {
      savingProfile.value = false
    }
  })
}

const handleChangePassword = async () => {
  if (!pwdFormRef.value) return
  await pwdFormRef.value.validate(async (valid: boolean) => {
    if (!valid) return
    changingPwd.value = true
    try {
      await request.post('/v1/user/change-password', pwdForm)
      ElMessage.success('登录密码已成功修改，请牢记新密码！')
      pwdForm.oldPassword = ''
      pwdForm.newPassword = ''
      pwdForm.confirmPassword = ''
      if (pwdFormRef.value) pwdFormRef.value.resetFields()
    } catch (err: any) {
      ElMessage.error(err.message || '修改密码失败')
    } finally {
      changingPwd.value = false
    }
  })
}

onMounted(() => {
  loadUserProfile()
})
</script>

<style scoped>
.user-profile-container {
  padding: 16px 20px 40px;
}

.profile-hero-card {
  border-radius: 10px;
  background: linear-gradient(135deg, #ffffff 0%, #f8fafc 100%);
  border: 1px solid #e2e8f0;
}

.hero-content {
  display: flex;
  align-items: center;
  gap: 20px;
}

.hero-avatar-wrap {
  position: relative;
}

.hero-avatar {
  background: linear-gradient(135deg, #3b82f6 0%, #1d4ed8 100%);
  color: #ffffff;
  font-size: 28px;
  font-weight: 700;
  box-shadow: 0 4px 12px rgba(59, 130, 246, 0.3);
}

.avatar-badge {
  position: absolute;
  bottom: 0;
  right: 0;
  width: 22px;
  height: 22px;
  background: #10b981;
  color: #fff;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  border: 2px solid #fff;
}

.hero-info {
  flex: 1;
}

.hero-name-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.hero-name {
  margin: 0;
  font-size: 22px;
  font-weight: 700;
  color: #0f172a;
}

.hero-username {
  color: #64748b;
  font-size: 14px;
}

.hero-meta-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 16px;
  margin-top: 8px;
  color: #475569;
  font-size: 13px;
}

.meta-item {
  display: flex;
  align-items: center;
  gap: 4px;
}

.form-card {
  border-radius: 10px;
  border: 1px solid #e2e8f0;
}

.card-title-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.card-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 700;
  font-size: 15px;
  color: #1e293b;
}

.divider-title {
  font-size: 13px;
  font-weight: 600;
  color: #64748b;
}

.notification-pref-grid {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.pref-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 14px;
  background: #f8fafc;
  border: 1px solid #e2e8f0;
  border-radius: 8px;
}

.pref-name {
  font-weight: 600;
  font-size: 14px;
  color: #1e293b;
  margin-bottom: 2px;
}

.pref-desc {
  font-size: 12px;
  color: #64748b;
}

.scope-info-box {
  background: #eff6ff;
  border: 1px solid #bfdbfe;
  border-radius: 8px;
  padding: 14px;
  margin-bottom: 16px;
}

.scope-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 6px;
}

.scope-label {
  font-weight: 700;
  font-size: 13px;
  color: #1e3a8a;
}

.scope-desc {
  font-size: 12px;
  color: #3b82f6;
  line-height: 1.5;
}

.perm-section {
  padding: 6px 0;
}

.perm-title {
  font-size: 13px;
  font-weight: 600;
  color: #475569;
  margin-bottom: 8px;
}
</style>
