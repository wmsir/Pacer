<template>
  <div class="login-wrapper">
    <!-- 顶栏微导航 -->
    <header class="top-nav-bar">
      <div class="nav-brand">
        <div class="nav-brand-logo" aria-hidden="true">骥</div>
        <span class="brand-title">稳骥 PacerSQL</span>
      </div>
      <router-link to="/portal" class="portal-link">返回门户</router-link>
    </header>

    <!-- 主展示与登录双栏区域 -->
    <main class="login-main-stage">
      <!-- 左栏：产品定位与核心能力展台 -->
      <section class="hero-showcase-panel">
        <div class="hero-header">
          <h1 class="hero-headline">
            让每一次数据库变更<br />都可审、可控、可回溯
          </h1>
          <p class="hero-description">
            SQL 工单、AI 预检、分级审批与分批执行，在一个平台里完成。
          </p>
        </div>

        <!-- 4 大技术特性矩阵 -->
        <div class="feature-matrix-grid">
          <div class="feature-card">
            <div class="feature-info">
              <div class="feature-title">SQL 智能预检与安全网关</div>
              <div class="feature-desc">Dry-Run 影响行数深度推导比对，高危 DDL/DML 自动阻断与偏差告警</div>
            </div>
          </div>

          <div class="feature-card">
            <div class="feature-info">
              <div class="feature-title">动态多级排他审批流水线</div>
              <div class="feature-desc">业务初审、DBA 复核、安全终审，按规则自动分流</div>
            </div>
          </div>

          <div class="feature-card">
            <div class="feature-info">
              <div class="feature-title">灰度分批与定时窗口调度</div>
              <div class="feature-desc">低峰期自动流式下发，大批量 DML 平滑提交，杜绝长事务锁表与从库延迟</div>
            </div>
          </div>

          <div class="feature-card">
            <div class="feature-info">
              <div class="feature-title">全渠道消息与加急催办总线</div>
              <div class="feature-desc">企业微信工作消息、飞书富文本互动卡片、钉钉加急群通知多端即刻触达</div>
            </div>
          </div>
        </div>

        <!-- 多数据库支持生态徽章 -->
        <div class="db-ecosystem-footer">
          <span class="db-eco-label">支持引擎生态：</span>
          <div class="db-badges-wrap">
            <span class="db-badge">MySQL 8.0</span>
            <span class="db-badge">TiDB</span>
            <span class="db-badge">OceanBase</span>
            <span class="db-badge">达梦 DM8</span>
            <span class="db-badge">人大金仓 Kingbase</span>
            <span class="db-badge">openGauss</span>
            <span class="db-badge">Oracle</span>
          </div>
        </div>
      </section>

      <!-- 右栏：轻奢毛玻璃科技登录卡片 -->
      <section class="login-card-panel">
        <div class="login-card-glass">
          <div class="card-header">
            <div class="card-title-row">
              <h2 class="card-title">登录</h2>
            </div>
          </div>

          <!-- 定制胶囊式选项卡 (Segmented Pills) -->
          <div class="segmented-tabs-bar">
            <button
              class="tab-pill-btn"
              :class="{ active: activeTab === 'account' }"
              @click="handleTabChange('account')"
            >
              <el-icon><User /></el-icon>
              <span>账号密码</span>
            </button>
            <button
              class="tab-pill-btn"
              :class="{ active: activeTab === 'phone' }"
              @click="handleTabChange('phone')"
            >
              <el-icon><Iphone /></el-icon>
              <span>手机验证码</span>
            </button>
            <button
              class="tab-pill-btn"
              :class="{ active: activeTab === 'qrcode' }"
              @click="handleTabChange('qrcode')"
            >
              <el-icon><Cellphone /></el-icon>
              <span>APP 扫码</span>
            </button>
          </div>

          <!-- Tab 1: 账号密码登录 -->
          <div v-show="activeTab === 'account'" class="tab-content-fade">
            <el-form
              ref="accountFormRef"
              :model="accountForm"
              :rules="accountRules"
              label-position="top"
              class="custom-login-form"
            >
              <el-form-item label="登录凭证账号" prop="account">
                <el-input
                  v-model="accountForm.account"
                  placeholder="用户名 / 手机号 / 企业邮箱 / 身份证号"
                  clearable
                  size="large"
                  :prefix-icon="User"
                  class="tech-input"
                />
              </el-form-item>
              <el-form-item label="安全访问密码" prop="password">
                <el-input
                  v-model="accountForm.password"
                  type="password"
                  placeholder="请输入密码（预置演练账号密码：123456）"
                  show-password
                  size="large"
                  :prefix-icon="Lock"
                  class="tech-input"
                  @keyup.enter="handleAccountLogin"
                />
              </el-form-item>
              <el-form-item style="margin-top: 24px;">
                <el-button
                  type="primary"
                  class="tech-submit-btn"
                  size="large"
                  @click="handleAccountLogin"
                  :loading="loading"
                >
                  <span v-if="!loading">登录</span>
                  <span v-else>正在验证安全凭据...</span>
                </el-button>
              </el-form-item>
            </el-form>
          </div>

          <!-- Tab 2: 手机/邮箱验证码登录 -->
          <div v-show="activeTab === 'phone'" class="tab-content-fade">
            <el-form
              ref="phoneFormRef"
              :model="phoneForm"
              :rules="phoneRules"
              label-position="top"
              class="custom-login-form"
            >
              <el-form-item label="手机号码或企业邮箱" prop="phone">
                <el-input
                  v-model="phoneForm.phone"
                  placeholder="请输入 11 位手机号或企业邮箱"
                  clearable
                  size="large"
                  :prefix-icon="Iphone"
                  class="tech-input"
                />
              </el-form-item>
              <el-form-item label="动态短信验证码" prop="code">
                <div class="code-input-row">
                  <el-input
                    v-model="phoneForm.code"
                    placeholder="请输入 6 位验证码"
                    clearable
                    size="large"
                    :prefix-icon="Key"
                    maxlength="6"
                    class="tech-input code-flex-input"
                    @keyup.enter="handlePhoneLogin"
                  />
                  <el-button
                    type="primary"
                    plain
                    size="large"
                    class="send-code-btn"
                    :disabled="countdown > 0 || sendLoading"
                    :loading="sendLoading"
                    @click="handleSendCode"
                  >
                    {{ countdown > 0 ? `${countdown}s 重发` : '获取验证码' }}
                  </el-button>
                </div>
              </el-form-item>
              <el-form-item style="margin-top: 24px;">
                <el-button
                  type="primary"
                  class="tech-submit-btn"
                  size="large"
                  @click="handlePhoneLogin"
                  :loading="loading"
                >
                  <span v-if="!loading">快速验证并登入</span>
                  <span v-else>正在核实验证码...</span>
                </el-button>
              </el-form-item>
            </el-form>
          </div>

          <!-- Tab 3: APP 扫码登录 (企微/钉钉/飞书/统一SSO) -->
          <div v-show="activeTab === 'qrcode'" class="tab-content-fade">
            <div class="qr-login-container">
              <!-- 渠道选择器 -->
              <div class="qr-channel-selector">
                <div
                  v-for="ch in channels"
                  :key="ch.key"
                  class="channel-tab"
                  :class="{ active: currentChannel === ch.key }"
                  @click="switchChannel(ch.key)"
                >
                  <span class="ch-icon">{{ ch.icon }}</span>
                  <span class="ch-label">{{ ch.name }}</span>
                </div>
              </div>

              <!-- 二维码展示核心区 -->
              <div class="qr-card-wrapper">
                <div
                  class="qr-box"
                  :class="{ 'is-expired': qrInfo?.status === 'EXPIRED' }"
                  @click="qrInfo?.status === 'EXPIRED' && refreshQr()"
                >
                  <div class="qr-code-graphic" v-if="qrInfo">
                    <svg viewBox="0 0 100 100" class="qr-svg">
                      <rect x="0" y="0" width="100" height="100" fill="#ffffff" />
                      <!-- 定位角标 1 (左上) -->
                      <rect x="8" y="8" width="26" height="26" fill="#1e293b" rx="4" />
                      <rect x="13" y="13" width="16" height="16" fill="#ffffff" rx="2" />
                      <rect x="17" y="17" width="8" height="8" fill="#1e293b" rx="1" />

                      <!-- 定位角标 2 (右上) -->
                      <rect x="66" y="8" width="26" height="26" fill="#1e293b" rx="4" />
                      <rect x="71" y="13" width="16" height="16" fill="#ffffff" rx="2" />
                      <rect x="75" y="17" width="8" height="8" fill="#1e293b" rx="1" />

                      <!-- 定位角标 3 (左下) -->
                      <rect x="8" y="66" width="26" height="26" fill="#1e293b" rx="4" />
                      <rect x="13" y="71" width="16" height="16" fill="#ffffff" rx="2" />
                      <rect x="17" y="75" width="8" height="8" fill="#1e293b" rx="1" />

                      <!-- 数据矩阵点阵 -->
                      <circle cx="42" cy="12" r="3" fill="#3b82f6" />
                      <circle cx="50" cy="18" r="3" fill="#1e293b" />
                      <circle cx="40" cy="28" r="3" fill="#1e293b" />
                      <circle cx="55" cy="30" r="3" fill="#3b82f6" />
                      <circle cx="15" cy="45" r="3" fill="#1e293b" />
                      <circle cx="26" cy="48" r="3" fill="#3b82f6" />
                      <circle cx="38" cy="42" r="3" fill="#1e293b" />
                      <circle cx="50" cy="50" r="4" fill="#2563eb" />
                      <circle cx="62" cy="42" r="3" fill="#1e293b" />
                      <circle cx="75" cy="48" r="3" fill="#3b82f6" />
                      <circle cx="85" cy="45" r="3" fill="#1e293b" />
                      <circle cx="42" cy="62" r="3" fill="#1e293b" />
                      <circle cx="55" cy="70" r="3" fill="#3b82f6" />
                      <circle cx="68" cy="72" r="3" fill="#1e293b" />
                      <circle cx="82" cy="65" r="3" fill="#1e293b" />
                      <circle cx="72" cy="85" r="3" fill="#3b82f6" />
                      <circle cx="85" cy="82" r="3" fill="#1e293b" />
                      <circle cx="45" cy="85" r="3" fill="#1e293b" />
                    </svg>
                    <div class="qr-center-logo">
                      <span>{{ getChannelIcon(currentChannel) }}</span>
                    </div>
                  </div>

                  <!-- 过期遮罩 -->
                  <div class="qr-mask expired-mask" v-if="qrInfo?.status === 'EXPIRED'">
                    <el-icon :size="28"><Refresh /></el-icon>
                    <span class="mask-text">二维码已失效</span>
                    <span class="mask-sub">点击即刻刷新</span>
                  </div>

                  <!-- 扫码成功待确认遮罩 -->
                  <div class="qr-mask scanned-mask" v-else-if="qrInfo?.status === 'SCANNED'">
                    <el-icon :size="32"><Check /></el-icon>
                    <span class="mask-text">手机端已识别</span>
                    <span class="mask-sub">请在移动终端点击确认授权</span>
                  </div>
                </div>

                <!-- 状态提示与有效倒计时 -->
                <div class="qr-status-info">
                  <div class="qr-tip-text">
                    请打开 <b>{{ getChannelName(currentChannel) }}</b> 扫一扫
                  </div>
                  <div class="qr-expire-tag" v-if="qrInfo && qrInfo.status === 'WAITING'">
                    有效时间剩余：<b>{{ qrRemainSeconds }}s</b>
                  </div>
                </div>

                <!-- 会话授权码可直接在移动端录入 -->
                <div v-if="qrInfo && qrInfo.qrKey" style="margin-top: 10px; display: flex; align-items: center; justify-content: center; gap: 8px; font-size: 11.5px; background: rgba(30,41,59,0.06); padding: 5px 12px; border-radius: 6px;">
                  <span style="color: var(--c-text-3);">授权密钥:</span>
                  <code style="font-weight: bold; color: var(--c-accent); font-family: monospace; font-size: 12px;">{{ qrInfo.qrKey }}</code>
                  <el-button link type="primary" size="small" @click="copyQrKey(qrInfo.qrKey)" style="font-size: 11px; padding: 0 4px;">复制</el-button>
                </div>

                <!-- 测试模拟扫码授权按钮 -->
                <div class="mock-scan-area">
                  <el-button
                    type="success"
                    size="small"
                    plain
                    :icon="Cellphone"
                    :loading="mockScanLoading"
                    @click="handleMockScanConfirm"
                    class="mock-btn"
                  >
                    一键模拟手机扫码确认
                  </el-button>
                </div>
              </div>
            </div>
          </div>

          <!-- 精致快速演练账号选择栏 (Horizontal Role Pills) -->
          <div class="fast-account-strip">
            <div class="strip-header">
              <span class="strip-title">演示账号</span>
              <span class="strip-hint">点击填入</span>
            </div>
            <div class="role-chips-grid">
              <button
                type="button"
                class="role-chip"
                :class="{ active: selectedRoleKey === 'admin' }"
                @click="fillTestRole('admin', 'testadmin1', '123456')"
              >
                <span class="role-label">超级管理员</span>
              </button>
              <button
                type="button"
                class="role-chip"
                :class="{ active: selectedRoleKey === 'lead' }"
                @click="fillTestRole('lead', 'testadmin2', '123456')"
              >
                <span class="role-label">开发组长</span>
              </button>
              <button
                type="button"
                class="role-chip"
                :class="{ active: selectedRoleKey === 'dba' }"
                @click="fillTestRole('dba', 'testadmin3', '123456')"
              >
                <span class="role-label">核心 DBA</span>
              </button>
              <button
                type="button"
                class="role-chip"
                :class="{ active: selectedRoleKey === 'audit' }"
                @click="fillTestRole('audit', 'test_auditor', '123456')"
              >
                <span class="role-label">安全合规官</span>
              </button>
              <button
                type="button"
                class="role-chip"
                :class="{ active: selectedRoleKey === 'dev' }"
                @click="fillTestRole('dev', 'test_dev', '123456')"
              >
                <span class="role-label">开发工程师</span>
              </button>
            </div>
          </div>

          <!-- 底部辅助功能：注册新账号 -->
          <div class="card-footer-actions">
            <span class="footer-hint">还没有账号？</span>
            <el-button link type="primary" class="register-link" @click="registerDialogVisible = true">
              申请注册
            </el-button>
          </div>

        </div>
      </section>
    </main>

    <!-- 页脚版权信息 -->
    <footer class="bottom-copyright-bar">
      <span>© {{ new Date().getFullYear() }} 稳骥 PacerSQL</span>
    </footer>

    <!-- 用户多渠道注册弹窗 -->
    <el-dialog
      title="申请注册新账号 (User Registration)"
      v-model="registerDialogVisible"
      width="540px"
      destroy-on-close
      class="custom-register-dialog"
    >
      <el-alert
        title="重要说明：姓名必填，系统支持同名同姓智能消歧（将自动匹配显示身份证后6位/手机尾号）"
        type="info"
        :closable="false"
        show-icon
        style="margin-bottom: 18px;"
      />

      <el-form
        ref="registerFormRef"
        :model="registerForm"
        :rules="registerRules"
        label-width="110px"
      >
        <el-form-item label="真实姓名" prop="realName">
          <el-input v-model="registerForm.realName" placeholder="必填，如：张伟、李明" />
        </el-form-item>

        <el-form-item label="注册认证方式" prop="registerType">
          <el-radio-group v-model="registerForm.registerType">
            <el-radio value="ACCOUNT">自定义账号</el-radio>
            <el-radio value="PHONE">手机号注册</el-radio>
            <el-radio value="EMAIL">企业邮箱注册</el-radio>
          </el-radio-group>
        </el-form-item>

        <el-form-item label="登录用户名" prop="username" v-if="registerForm.registerType === 'ACCOUNT'">
          <el-input v-model="registerForm.username" placeholder="请输入账号名（字母数字下划线组合）" />
        </el-form-item>

        <el-form-item label="手机号码" prop="phone" :required="registerForm.registerType === 'PHONE'">
          <el-input v-model="registerForm.phone" placeholder="请输入 11 位有效手机号" maxlength="11" />
        </el-form-item>

        <el-form-item label="企业工作邮箱" prop="email" :required="registerForm.registerType === 'EMAIL'">
          <el-input v-model="registerForm.email" placeholder="如：user@fwpt.cn 或企业工作邮箱" />
        </el-form-item>

        <el-form-item label="邮箱验证码" prop="code" v-if="registerForm.registerType === 'EMAIL'">
          <div style="display: flex; gap: 10px; width: 100%;">
            <el-input v-model="registerForm.code" placeholder="请输入 6 位邮箱验证码" maxlength="6" style="flex: 1;" />
            <el-button
              type="primary"
              plain
              :disabled="emailCountdown > 0 || sendEmailLoading"
              :loading="sendEmailLoading"
              @click="handleSendEmailCode"
            >
              {{ emailCountdown > 0 ? `${emailCountdown}s 重发` : '获取验证码' }}
            </el-button>
          </div>
        </el-form-item>

        <el-form-item label="身份证号" prop="idCard">
          <el-input v-model="registerForm.idCard" placeholder="选填，用于重名消歧识别（后6位展示）" maxlength="18" />
        </el-form-item>

        <el-form-item label="归属业务组" prop="resourceGroups">
          <el-select
            v-model="registerForm.resourceGroups"
            multiple
            collapse-tags
            collapse-tags-tooltip
            style="width: 100%;"
            placeholder="请选择归属业务资源组（支持多选）"
          >
            <el-option label="车险承保资源组" value="车险承保资源组" />
            <el-option label="销管系统资源组" value="销管系统资源组" />
            <el-option label="理赔服务核心组" value="理赔服务核心组" />
            <el-option label="水险财产险1000条以下" value="水险财产险1000条以下" />
            <el-option label="农险理赔资源组" value="农险理赔资源组" />
            <el-option label="风勘中心资源组" value="风勘中心资源组" />
            <el-option label="互联网车主服务与理赔快处组" value="互联网车主服务与理赔快处组" />
            <el-option label="默认核心业务资源组" value="默认核心业务资源组" />
          </el-select>
        </el-form-item>

        <el-form-item label="申请初始角色" prop="role">
          <el-select v-model="registerForm.role" style="width: 100%;">
            <el-option label="DEV - 开发工程师 (工单申请与SQL查询)" value="DEV" />
            <el-option label="DEV_LEAD - 开发组长 (初审审批人)" value="DEV_LEAD" />
            <el-option label="DBA - 数据库管理员 (安全复审与执行调度)" value="DBA" />
            <el-option label="AUDITOR - 安全审计员 (全量合规审计)" value="AUDITOR" />
          </el-select>
        </el-form-item>

        <el-form-item label="设置密码" prop="password">
          <el-input v-model="registerForm.password" type="password" placeholder="请输入不少于 6 位安全密码" show-password />
        </el-form-item>
      </el-form>

      <template #footer>
        <el-button @click="registerDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="registerLoading" @click="handleRegisterSubmit">
          立即提交并登入系统
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import { User, Lock, Iphone, Key, Refresh, Check, Cellphone } from '@element-plus/icons-vue'
import request from '../utils/request'
import { sm2 } from 'sm-crypto'
import { useUserStore } from '../store/user'

const PUBLIC_KEY = import.meta.env.VITE_SM2_PUBLIC_KEY || '04fea943c0bb2c03cefbf0e26eab00b5c7266c3fb7f47e8e80401a2b614315f2b89b0ba40eea69d3322e9942b317a7ecf8415ed7c73b026c02e3f568f0acdcc94e'

const router = useRouter()
const userStore = useUserStore()

const activeTab = ref<'account' | 'phone' | 'qrcode'>('account')
const loading = ref(false)
const sendLoading = ref(false)
const countdown = ref(0)
let timer: number | null = null

// 快速角色体验选中的 key
const selectedRoleKey = ref('admin')

// 账号密码表单
const accountFormRef = ref<FormInstance>()
const accountForm = reactive({
  account: 'testadmin1',
  password: '123456'
})

const accountRules = reactive<FormRules>({
  account: [
    { required: true, message: '请输入用户名、手机号、邮箱或身份证号', trigger: 'blur' }
  ],
  password: [
    { required: true, message: '请输入登录密码', trigger: 'blur' },
    { min: 6, message: '密码长度不能小于6位', trigger: 'blur' }
  ]
})

// 验证码登录表单
const phoneFormRef = ref<FormInstance>()
const phoneForm = reactive({
  phone: '',
  code: ''
})

const phoneRules = reactive<FormRules>({
  phone: [
    { required: true, message: '请输入手机号或邮箱', trigger: 'blur' }
  ],
  code: [
    { required: true, message: '请输入6位验证码', trigger: 'blur' },
    { len: 6, message: '验证码为6位', trigger: 'blur' }
  ]
})

// 扫码登录相关
const channels = [
  { key: 'PACER_APP', name: '稳骥移动端', icon: '📱' },
  { key: 'WECOM', name: '企业微信', icon: '💼' },
  { key: 'DINGTALK', name: '阿里钉钉', icon: '📌' },
  { key: 'FEISHU', name: '字节飞书', icon: '🕊️' },
  { key: 'SSO', name: '统一 SSO', icon: '🌐' }
]
const currentChannel = ref('PACER_APP')
const qrInfo = ref<any>(null)
const qrRemainSeconds = ref(180)
const mockScanLoading = ref(false)
let qrPollTimer: number | null = null
let qrCountdownTimer: number | null = null

const getChannelName = (key: string) => {
  const c = channels.find(item => item.key === key)
  return c ? c.name : '移动端 APP'
}

const getChannelIcon = (key: string) => {
  const c = channels.find(item => item.key === key)
  return c ? c.icon : '📱'
}

const copyQrKey = (key: string) => {
  if (navigator.clipboard) {
    navigator.clipboard.writeText(key)
    ElMessage.success('授权码已复制，可在移动审批端粘贴快速授权')
  }
}

// 统一登录成功后角色路由分发
const handleLoginSuccess = async (token: string, welcomeMsg?: string) => {
  if (welcomeMsg) {
    ElMessage.success(welcomeMsg)
  } else {
    ElMessage.success('凭据校验通过，欢迎进入系统！')
  }
  userStore.token = token
  localStorage.setItem('wmdb_token', token)
  await userStore.fetchUserInfo()

  // 只有超级管理员才跳转平台总览 (/dashboard)，其他角色默认跳转新建工单 (/ticket-create)
  if (userStore.isAdmin || userStore.userRole?.toUpperCase() === 'ADMIN' || userStore.userRole?.toUpperCase() === 'ROLE_ADMIN') {
    router.push('/dashboard')
  } else {
    router.push('/ticket-create')
  }
}

const handleTabChange = (tabName: 'account' | 'phone' | 'qrcode') => {
  activeTab.value = tabName
  if (tabName === 'qrcode') {
    refreshQr()
  } else {
    stopQrPolling()
  }
}

const switchChannel = (key: string) => {
  currentChannel.value = key
  refreshQr()
}

const refreshQr = async () => {
  stopQrPolling()
  try {
    const res: any = await request.post(`/v1/auth/qr/generate?channel=${currentChannel.value}`)
    qrInfo.value = res.data
    qrRemainSeconds.value = res.data?.expireSeconds || 180

    // 启动状态轮询与倒计时
    startQrPolling(res.data.qrKey)
  } catch (e) {
    ElMessage.error('生成二维码失败')
  }
}

const startQrPolling = (qrKey: string) => {
  stopQrPolling()

  // 1s 倒计时
  qrCountdownTimer = window.setInterval(() => {
    if (qrRemainSeconds.value > 0) {
      qrRemainSeconds.value--
    } else {
      if (qrInfo.value) qrInfo.value.status = 'EXPIRED'
      stopQrPolling()
    }
  }, 1000)

  // 2s 轮询状态
  qrPollTimer = window.setInterval(async () => {
    try {
      const res: any = await request.get('/v1/auth/qr/status', { params: { qrKey } })
      const data = res.data
      if (data) {
        qrInfo.value.status = data.status
        qrInfo.value.statusMsg = data.statusMsg

        if (data.status === 'CONFIRMED' && data.token) {
          stopQrPolling()
          await handleLoginSuccess(data.token, `扫码授权成功，欢迎【${data.user?.displayName || '用户'}】`)
        } else if (data.status === 'EXPIRED') {
          stopQrPolling()
        }
      }
    } catch (err) {
      // ignore
    }
  }, 2000)
}

const stopQrPolling = () => {
  if (qrPollTimer) {
    clearInterval(qrPollTimer)
    qrPollTimer = null
  }
  if (qrCountdownTimer) {
    clearInterval(qrCountdownTimer)
    qrCountdownTimer = null
  }
}

// 模拟扫码授权
const handleMockScanConfirm = async () => {
  if (!qrInfo.value || !qrInfo.value.qrKey) return
  mockScanLoading.value = true
  try {
    const res: any = await request.post('/v1/auth/qr/mock-scan', {
      qrKey: qrInfo.value.qrKey,
      account: 'testadmin1'
    })
    const data = res.data
    if (data && data.token) {
      await handleLoginSuccess(data.token, `模拟扫码授权成功：${data.user?.displayName || '管理员'}`)
    }
  } catch (error: any) {
    ElMessage.error(error.message || '模拟扫码失败')
  } finally {
    mockScanLoading.value = false
  }
}

// 账号密码登录
const handleAccountLogin = async () => {
  if (!accountFormRef.value) return
  await accountFormRef.value.validate(async (valid) => {
    if (!valid) return
    loading.value = true
    try {
      let encryptedPassword = accountForm.password
      try {
        const pk = PUBLIC_KEY.startsWith('04') ? PUBLIC_KEY.substring(2) : PUBLIC_KEY
        const cipher = sm2.doEncrypt(accountForm.password, pk, 1)
        encryptedPassword = cipher.startsWith('04') ? cipher : '04' + cipher
      } catch (e) {
        console.warn('SM2 encryption failed, falling back to plaintext')
      }

      const payload = {
        loginType: 'ACCOUNT_PASSWORD',
        account: accountForm.account.trim(),
        password: encryptedPassword
      }

      const response: any = await request.post('/v1/auth/login', payload)
      const token = response.data.token
      await handleLoginSuccess(token)
    } catch (error: any) {
      // handled
    } finally {
      loading.value = false
    }
  })
}

// 发送短信/邮箱验证码
const handleSendCode = async () => {
  if (!phoneForm.phone) {
    ElMessage.warning('请先输入手机号或企业邮箱')
    return
  }

  sendLoading.value = true
  try {
    const target = phoneForm.phone.trim()
    const isEmail = target.includes('@')
    const url = isEmail ? '/v1/auth/send-email-code' : '/v1/auth/send-code'
    const body = isEmail ? { email: target } : { phone: target }

    const res: any = await request.post(url, body)
    const code = res.data?.code || '123456'
    ElMessage.success(`安全验证码已发送（测试码：${code}）`)

    countdown.value = 60
    timer = window.setInterval(() => {
      countdown.value--
      if (countdown.value <= 0) {
        if (timer) clearInterval(timer)
        timer = null
      }
    }, 1000)
  } catch (error: any) {
    ElMessage.error(error.message || '发送验证码失败')
  } finally {
    sendLoading.value = false
  }
}

// 验证码快速登录 (手机/企业邮箱)
const handlePhoneLogin = async () => {
  if (!phoneFormRef.value) return
  await phoneFormRef.value.validate(async (valid) => {
    if (!valid) return
    loading.value = true
    try {
      const target = phoneForm.phone.trim()
      const payload: any = {
        loginType: 'PHONE_CODE',
        phone: target,
        code: phoneForm.code.trim()
      }
      if (target.includes('@')) {
        payload.email = target
        payload.account = target
      }

      const response: any = await request.post('/v1/auth/login', payload)
      const token = response.data.token
      await handleLoginSuccess(token)
    } catch (error: any) {
      // handled
    } finally {
      loading.value = false
    }
  })
}

// 多渠道自主注册
const registerDialogVisible = ref(false)
const registerLoading = ref(false)
const sendEmailLoading = ref(false)
const emailCountdown = ref(0)
let emailTimer: number | null = null

const registerFormRef = ref<FormInstance>()
const registerForm = reactive({
  registerType: 'ACCOUNT',
  username: '',
  realName: '',
  idCard: '',
  phone: '',
  email: '',
  code: '',
  resourceGroups: ['车险承保资源组'],
  role: 'DEV',
  password: ''
})

const registerRules = reactive<FormRules>({
  realName: [
    { required: true, message: '真实姓名必填（用于身份辨识与同名消歧）', trigger: 'blur' }
  ],
  password: [
    { required: true, message: '请输入密码', trigger: 'blur' },
    { min: 6, message: '密码不能少于6位', trigger: 'blur' }
  ]
})

const handleSendEmailCode = async () => {
  if (!registerForm.email || !/^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$/.test(registerForm.email.trim())) {
    ElMessage.warning('请输入有效的企业电子邮箱')
    return
  }
  sendEmailLoading.value = true
  try {
    const res: any = await request.post('/v1/auth/send-email-code', { email: registerForm.email.trim() })
    const code = res.data?.code || '123456'
    ElMessage.success(`安全验证码已发送至邮箱（测试码：${code}）`)
    emailCountdown.value = 60
    emailTimer = window.setInterval(() => {
      emailCountdown.value--
      if (emailCountdown.value <= 0) {
        if (emailTimer) clearInterval(emailTimer)
        emailTimer = null
      }
    }, 1000)
  } catch (error: any) {
    ElMessage.error(error.message || '发送邮箱验证码失败')
  } finally {
    sendEmailLoading.value = false
  }
}

const handleRegisterSubmit = async () => {
  if (!registerFormRef.value) return
  await registerFormRef.value.validate(async (valid) => {
    if (!valid) return
    registerLoading.value = true
    try {
      const payload = {
        ...registerForm,
        resourceGroup: (registerForm.resourceGroups || []).join(', ')
      }
      const res: any = await request.post('/v1/auth/register', payload)
      const token = res.data?.token
      if (token) {
        registerDialogVisible.value = false
        await handleLoginSuccess(token, `账号注册成功！已为【${registerForm.realName}】自动登入系统`)
      }
    } catch (error: any) {
      ElMessage.error(error.message || '注册失败')
    } finally {
      registerLoading.value = false
    }
  })
}

// 快速填入演练角色账号
const fillTestRole = (key: string, account: string, pass: string) => {
  selectedRoleKey.value = key
  activeTab.value = 'account'
  accountForm.account = account
  accountForm.password = pass
  ElMessage.info({
    message: `已自动填入角色凭据，点击「登录控制台」即可登入`,
    duration: 2000
  })
}

onMounted(async () => {
  // 1. 尝试从 URL 查询参数或 localStorage 探测 FWPT 统一通行证 (OneID SSO)
  try {
    const urlParams = new URLSearchParams(window.location.search)
    const ssoToken = urlParams.get('sso_token') || localStorage.getItem('fwpt_token') || localStorage.getItem('pacer_token')
    const userRaw = localStorage.getItem('fwpt_user')
    if (ssoToken) {
      localStorage.setItem('wmdb_token', ssoToken)
      localStorage.setItem('pacer_token', ssoToken)
      userStore.token = ssoToken
      if (userRaw) {
        try {
          const user = JSON.parse(userRaw)
          if (user && (user.username || user.realName)) {
            ElMessage.success(`🎉 检测到 FWPT 统一通行证【${user.realName || user.username}】，已为您无缝授权！`)
            // 超级管理员进入平台总览，其他角色进入新建工单
            setTimeout(() => {
              if (userStore.isAdmin || userStore.userRole?.toUpperCase() === 'ADMIN' || user.role === 'ADMIN') {
                router.push('/dashboard')
              } else {
                router.push('/ticket-create')
              }
            }, 600)
            return
          }
        } catch (e) {}
      }
    }
  } catch (err) {}

  if (activeTab.value === 'qrcode') {
    refreshQr()
  }
})

onUnmounted(() => {
  stopQrPolling()
})
</script>

<style scoped>
/* 整个页面主容器：清爽明亮白底、柔和空气感护眼 */
.login-wrapper {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
  background: var(--c-bg);
  color: var(--c-text);
}

/* 顶栏 */
.top-nav-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 20px 40px;
}
.nav-brand { display: flex; align-items: center; gap: 10px; }
.nav-brand-logo {
  width: 28px;
  height: 28px;
  border-radius: 7px;
  background: var(--c-text);
  color: var(--c-surface);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 15px;
  font-weight: 700;
}
.brand-title { font-size: 15px; font-weight: 650; letter-spacing: -0.01em; }
.portal-link { font-size: 13px; color: var(--c-text-3); text-decoration: none; }
.portal-link:hover { color: var(--c-text); }

/* 主体：左侧介绍 + 右侧登录卡 */
.login-main-stage {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: clamp(40px, 8vw, 120px);
  padding: 32px 40px 56px;
  max-width: 1160px;
  width: 100%;
  margin: 0 auto;
  box-sizing: border-box;
}
.hero-showcase-panel { flex: 1; max-width: 520px; }
.hero-headline {
  margin: 0 0 14px;
  font-size: 34px;
  line-height: 1.25;
  font-weight: 650;
  letter-spacing: -0.03em;
  color: var(--c-text);
}
.hero-description {
  margin: 0 0 32px;
  font-size: 15px;
  line-height: 1.7;
  color: var(--c-text-3);
}
.feature-matrix-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 22px 28px;
  padding-top: 24px;
  border-top: 1px solid var(--c-border);
}
.feature-title { font-size: 14px; font-weight: 600; color: var(--c-text); margin-bottom: 4px; }
.feature-desc { font-size: 13px; line-height: 1.6; color: var(--c-text-3); }
.db-ecosystem-footer {
  margin-top: 28px;
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 6px;
  font-size: 12.5px;
  color: var(--c-text-4);
}
.db-badges-wrap { display: contents; }
.db-badge { color: var(--c-text-3); }
.db-badge:not(:last-child)::after { content: "、"; color: var(--c-text-4); }

.login-card-panel { width: 400px; flex-shrink: 0; }
.login-card-glass {
  background: var(--c-surface);
  border: 1px solid var(--c-border);
  border-radius: var(--radius-lg, 14px);
  padding: 32px 32px 24px;
  box-shadow: var(--shadow-md);
}
.card-header { margin-bottom: 18px; }
.card-title { margin: 0; font-size: 20px; font-weight: 650; letter-spacing: -0.02em; }

/* 登录方式：下划线选项卡 */
.segmented-tabs-bar {
  display: flex;
  gap: 20px;
  border-bottom: 1px solid var(--c-border);
  margin-bottom: 22px;
}
.tab-pill-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 0 10px;
  margin-bottom: -1px;
  background: none;
  border: none;
  border-bottom: 2px solid transparent;
  color: var(--c-text-3);
  font: inherit;
  font-size: 13.5px;
  cursor: pointer;
}
.tab-pill-btn .el-icon { display: none; }
.tab-pill-btn:hover { color: var(--c-text); }
.tab-pill-btn.active { color: var(--c-text); font-weight: 600; border-bottom-color: var(--c-text); }
.tab-pill-btn:focus-visible { outline: 2px solid var(--c-accent); outline-offset: 2px; }

.tab-content-fade { animation: fadeIn 0.2s ease; }
@keyframes fadeIn { from { opacity: 0; } to { opacity: 1; } }

.custom-login-form :deep(.el-form-item) {
  margin-bottom: 18px;
}

.custom-login-form :deep(.el-form-item__label) {
  color: var(--c-text-2);
  font-size: 13px;
  font-weight: 600;
  padding-bottom: 6px;
}

.tech-input :deep(.el-input__wrapper) {
  border-radius: var(--radius-sm) !important;
  padding: 4px 12px;
}

.tech-input :deep(.el-input__inner) {
  color: var(--c-text) !important;
  font-size: 14px;
}

.tech-input :deep(.el-input__inner::placeholder) {
  color: var(--c-text-4) !important;
  font-size: 13px;
}

.tech-input :deep(.el-input__prefix-inner) { color: var(--c-text-4); }

.code-input-row {
  display: flex;
  gap: 10px;
  width: 100%;
}

.code-flex-input {
  flex: 1;
}

.send-code-btn {
  flex-shrink: 0;
  width: 112px;
  height: 40px;
}

.tech-submit-btn {
  width: 100%;
  height: 42px;
  font-size: 14.5px;
  font-weight: 600;
}

.qr-login-container {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 4px 0 10px;
}

.qr-channel-selector {
  display: flex;
  background: var(--c-subtle);
  padding: 4px;
  border-radius: 10px;
  border: 1px solid var(--c-border);
  margin-bottom: 16px;
  width: 100%;
}

.channel-tab {
  flex: 1;
  text-align: center;
  padding: 7px 4px;
  font-size: 12px;
  font-weight: 600;
  color: var(--c-text-3);
  cursor: pointer;
  border-radius: 7px;
  transition: all 0.2s;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 4px;
}

.channel-tab.active {
  background: var(--c-surface);
  color: var(--c-accent);
  box-shadow: var(--shadow-sm);
}

.qr-card-wrapper {
  display: flex;
  flex-direction: column;
  align-items: center;
  width: 100%;
}

.qr-box {
  width: 180px;
  height: 180px;
  padding: 10px;
  border: 1px solid var(--c-border);
  border-radius: 14px;
  background: var(--c-surface);
  box-shadow: var(--shadow-sm);
  position: relative;
  box-sizing: border-box;
}

.qr-box.is-expired {
  cursor: pointer;
}

.qr-code-graphic {
  width: 100%;
  height: 100%;
  position: relative;
}

.qr-svg {
  width: 100%;
  height: 100%;
}

.qr-center-logo {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  width: 32px;
  height: 32px;
  background: var(--c-surface);
  border-radius: 7px;
  box-shadow: var(--shadow-sm);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 18px;
}

.qr-mask {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  border-radius: 14px;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 6px;
  backdrop-filter: blur(4px);
}

.expired-mask {
  background: rgba(15, 23, 42, 0.82);
  color: #ffffff;
}

.scanned-mask {
  background: color-mix(in srgb, var(--c-surface) 95%, transparent);
  color: var(--c-text);
}

.mask-text {
  font-size: 13.5px;
  font-weight: 700;
}

.mask-sub {
  font-size: 11px;
  opacity: 0.8;
}

.qr-status-info {
  margin-top: 12px;
  text-align: center;
}

.qr-tip-text {
  font-size: 13px;
  color: var(--c-text-2);
}

.qr-expire-tag {
  font-size: 12px;
  color: var(--c-text-4);
  margin-top: 4px;
}

.mock-scan-area {
  margin-top: 12px;
}

.mock-btn {
  border-radius: 8px;
}

/* 快捷演练账号选择栏：清爽卡片色 */
.fast-account-strip {
  margin-top: 20px;
  padding-top: 16px;
  border-top: 1px dashed var(--c-border);
}
.strip-header { display: flex; justify-content: space-between; margin-bottom: 8px; }
.strip-title { font-size: 12.5px; color: var(--c-text-3); }
.strip-hint { font-size: 12px; color: var(--c-text-4); }
.role-chips-grid { display: flex; flex-wrap: wrap; gap: 6px; }
.role-chip {
  padding: 4px 10px;
  border-radius: 999px;
  background: var(--c-surface);
  border: 1px solid var(--c-border);
  color: var(--c-text-2);
  font: inherit;
  font-size: 12px;
  cursor: pointer;
}
.role-chip:hover { border-color: var(--c-border-strong); color: var(--c-text); }
.role-chip.active { border-color: var(--c-text); color: var(--c-text); font-weight: 600; }

.card-footer-actions {
  display: flex;
  justify-content: center;
  align-items: center;
  gap: 4px;
  margin-top: 18px;
  font-size: 13px;
}
.footer-hint { color: var(--c-text-3); }
.register-link { font-size: 13px; font-weight: 600; }

.bottom-copyright-bar {
  padding: 18px 20px;
  text-align: center;
  font-size: 12px;
  color: var(--c-text-4);
}

@media (max-width: 960px) {
  .login-main-stage { flex-direction: column-reverse; align-items: stretch; padding: 16px 20px 40px; gap: 40px; }
  .login-card-panel { width: 100%; max-width: 420px; margin: 0 auto; }
  .hero-showcase-panel { max-width: 420px; margin: 0 auto; }
  .hero-headline { font-size: 26px; }
}
@media (max-width: 640px) {
  .top-nav-bar { padding: 16px 20px; }
  .feature-matrix-grid { grid-template-columns: 1fr; }
  .login-card-glass { padding: 24px 20px 20px; }
}
</style>
