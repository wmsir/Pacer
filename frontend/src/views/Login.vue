<template>
  <div class="login-wrapper">
    <!-- 背景流光与科技网格点缀 -->
    <div class="ambient-glow glow-1"></div>
    <div class="ambient-glow glow-2"></div>
    <div class="ambient-glow glow-3"></div>
    <div class="cyber-grid-overlay"></div>

    <!-- 顶栏微导航 -->
    <header class="top-nav-bar">
      <div class="nav-brand">
        <div class="nav-brand-logo">
          <span class="logo-emoji">🐎</span>
        </div>
        <div class="nav-brand-text">
          <span class="brand-title">稳骥 PacerSQL</span>
          <span class="brand-badge">Enterprise v2.8</span>
        </div>
      </div>
      <div class="nav-status-group">
        <router-link to="/portal" class="env-pill portal-link" style="text-decoration: none; color: inherit; display: flex; align-items: center; gap: 4px; background: rgba(59, 130, 246, 0.15); border-color: rgba(59, 130, 246, 0.3);">
          <span>🌐</span>
          <span>fwpt.cn 旗舰门户</span>
        </router-link>
        <div class="status-indicator">
          <span class="pulse-dot"></span>
          <span class="status-text">生产安全网关集群正常</span>
        </div>
        <div class="env-pill">RDS MySQL 8.0 联动在线</div>
      </div>
    </header>

    <!-- 主展示与登录双栏区域 -->
    <main class="login-main-stage">
      <!-- 左栏：产品定位与核心能力展台 -->
      <section class="hero-showcase-panel">
        <div class="hero-header">
          <div class="hero-tag">
            <span class="tag-sparkle">✨</span>
            <span>“骥”为千里良驹 · “稳”压倒一切</span>
          </div>
          <h1 class="hero-headline">
            稳骥 PacerSQL<br />
            <span class="gradient-text">企业级高可用数据库变更自治平台</span>
          </h1>
          <p class="hero-description">
            Pacer（匀速平稳赛马）之律动，直击生产高可用痛点。融合 AI 深度预检、SpEL 智能路由与灰度分批流式引擎，保障百万级核心数据库稳健运行。
          </p>
        </div>

        <!-- 4 大技术特性矩阵 -->
        <div class="feature-matrix-grid">
          <div class="feature-card">
            <div class="feature-icon-box icon-shield">
              <span>🛡️</span>
            </div>
            <div class="feature-info">
              <div class="feature-title">SQL 智能预检与安全网关</div>
              <div class="feature-desc">Dry-Run 影响行数深度推导比对，高危 DDL/DML 自动阻断与偏差告警</div>
            </div>
          </div>

          <div class="feature-card">
            <div class="feature-icon-box icon-workflow">
              <span>⛓️</span>
            </div>
            <div class="feature-info">
              <div class="feature-title">动态多级排他审批流水线</div>
              <div class="feature-desc">业务初审 ➔ 核心 DBA 复核 ➔ 安全总监终审，基于规则灵活条件分流</div>
            </div>
          </div>

          <div class="feature-card">
            <div class="feature-icon-box icon-speed">
              <span>⚡</span>
            </div>
            <div class="feature-info">
              <div class="feature-title">灰度分批与定时窗口调度</div>
              <div class="feature-desc">低峰期自动流式下发，大批量 DML 平滑提交，杜绝长事务锁表与从库延迟</div>
            </div>
          </div>

          <div class="feature-card">
            <div class="feature-icon-box icon-bell">
              <span>🔔</span>
            </div>
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
              <h2 class="card-title">欢迎登录</h2>
              <span class="card-title-badge">用户控制台</span>
            </div>
            <p class="card-subtitle">请选择身份验证方式接入数据库变更管控中心</p>
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
                  <span v-if="!loading">登 录 控 制 台</span>
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
                    <el-icon :size="32" color="#10b981"><Check /></el-icon>
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
                  <span style="color: #64748b;">授权密钥:</span>
                  <code style="font-weight: bold; color: #0284c7; font-family: monospace; font-size: 12px;">{{ qrInfo.qrKey }}</code>
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
                    ⚡ 一键模拟手机扫码确认
                  </el-button>
                </div>
              </div>
            </div>
          </div>

          <!-- 精致快速演练账号选择栏 (Horizontal Role Pills) -->
          <div class="fast-account-strip">
            <div class="strip-header">
              <span class="strip-title">🎯 快捷身份一键切换 (预置演练账号)</span>
              <span class="strip-hint">点击填入</span>
            </div>
            <div class="role-chips-grid">
              <button
                type="button"
                class="role-chip"
                :class="{ active: selectedRoleKey === 'admin' }"
                @click="fillTestRole('admin', 'testadmin1', '123456')"
              >
                <span class="role-icon">👑</span>
                <span class="role-label">超级管理员</span>
              </button>
              <button
                type="button"
                class="role-chip"
                :class="{ active: selectedRoleKey === 'lead' }"
                @click="fillTestRole('lead', 'testadmin2', '123456')"
              >
                <span class="role-icon">👔</span>
                <span class="role-label">开发组长</span>
              </button>
              <button
                type="button"
                class="role-chip"
                :class="{ active: selectedRoleKey === 'dba' }"
                @click="fillTestRole('dba', 'testadmin3', '123456')"
              >
                <span class="role-icon">🛡️</span>
                <span class="role-label">核心 DBA</span>
              </button>
              <button
                type="button"
                class="role-chip"
                :class="{ active: selectedRoleKey === 'audit' }"
                @click="fillTestRole('audit', 'test_auditor', '123456')"
              >
                <span class="role-icon">🔍</span>
                <span class="role-label">安全合规官</span>
              </button>
              <button
                type="button"
                class="role-chip"
                :class="{ active: selectedRoleKey === 'dev' }"
                @click="fillTestRole('dev', 'test_dev', '123456')"
              >
                <span class="role-icon">💻</span>
                <span class="role-label">开发工程师</span>
              </button>
            </div>
          </div>

          <!-- 底部辅助功能：注册新账号 -->
          <div class="card-footer-actions">
            <span class="footer-hint">暂无内部权限凭证？</span>
            <el-button link type="primary" class="register-link" @click="registerDialogVisible = true">
              申请注册新账号 ➔
            </el-button>
          </div>

          <div class="security-meta-bar">
            <span>🔒 国密 SM2/SM4 传输防护</span>
            <span>·</span>
            <span>TLS 1.3</span>
            <span>·</span>
            <span>延迟 &lt; 15ms</span>
          </div>
        </div>
      </section>
    </main>

    <!-- 页脚版权信息 -->
    <footer class="bottom-copyright-bar">
      <span>© 2026 稳骥 PacerSQL · 企业级高可用数据库变更自治云平台 · fwpt.cn</span>
      <span class="pipe-split">|</span>
      <span>粤ICP备2025000000号-1</span>
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
  position: relative;
  min-height: 100vh;
  width: 100%;
  background: #f8fafc;
  overflow-x: hidden;
  display: flex;
  flex-direction: column;
  color: #0f172a;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
}

/* 柔和科技流光微晕背景（低饱和轻柔明亮） */
.ambient-glow {
  position: absolute;
  border-radius: 50%;
  filter: blur(140px);
  pointer-events: none;
  z-index: 0;
}

.glow-1 {
  width: 650px;
  height: 650px;
  top: -150px;
  left: -100px;
  background: radial-gradient(circle, rgba(59, 130, 246, 0.09) 0%, rgba(59, 130, 246, 0) 70%);
}

.glow-2 {
  width: 600px;
  height: 600px;
  bottom: -100px;
  right: -80px;
  background: radial-gradient(circle, rgba(99, 102, 241, 0.08) 0%, rgba(99, 102, 241, 0) 70%);
}

.glow-3 {
  width: 500px;
  height: 500px;
  top: 40%;
  left: 45%;
  background: radial-gradient(circle, rgba(16, 185, 129, 0.06) 0%, rgba(16, 185, 129, 0) 70%);
}

/* 细腻网格轻底纹 */
.cyber-grid-overlay {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-image: 
    linear-gradient(to right, rgba(15, 23, 42, 0.025) 1px, transparent 1px),
    linear-gradient(to bottom, rgba(15, 23, 42, 0.025) 1px, transparent 1px);
  background-size: 40px 40px;
  pointer-events: none;
  z-index: 1;
}

/* 顶部品牌微导航：高通透白净玻璃 */
.top-nav-bar {
  position: relative;
  z-index: 10;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 48px;
  border-bottom: 1px solid #e2e8f0;
  background: rgba(255, 255, 255, 0.88);
  backdrop-filter: blur(16px);
}

.nav-brand {
  display: flex;
  align-items: center;
  gap: 12px;
}

.nav-brand-logo {
  width: 38px;
  height: 38px;
  border-radius: 10px;
  background: linear-gradient(135deg, #2563eb 0%, #38bdf8 100%);
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 4px 12px rgba(37, 99, 235, 0.25);
}

.logo-emoji {
  font-size: 20px;
}

.nav-brand-text {
  display: flex;
  align-items: center;
  gap: 8px;
}

.brand-title {
  font-size: 18px;
  font-weight: 700;
  letter-spacing: 0.3px;
  color: #0f172a;
}

.brand-badge {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 20px;
  background: #eff6ff;
  color: #2563eb;
  border: 1px solid #bfdbfe;
  font-weight: 600;
}

.nav-status-group {
  display: flex;
  align-items: center;
  gap: 16px;
}

.status-indicator {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12.5px;
  color: #475569;
}

.pulse-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background-color: #10b981;
  box-shadow: 0 0 0 2px rgba(16, 185, 129, 0.25);
  animation: pulse-ring 2s infinite ease-in-out;
}

@keyframes pulse-ring {
  0% { transform: scale(0.95); opacity: 0.7; }
  50% { transform: scale(1.2); opacity: 1; }
  100% { transform: scale(0.95); opacity: 0.7; }
}

.env-pill {
  font-size: 12px;
  padding: 4px 10px;
  border-radius: 6px;
  background: #f1f5f9;
  border: 1px solid #e2e8f0;
  color: #334155;
  font-weight: 500;
}

/* 主展示区域 */
.login-main-stage {
  position: relative;
  z-index: 10;
  flex: 1;
  max-width: 1320px;
  width: 100%;
  margin: 0 auto;
  padding: 36px 48px 24px;
  box-sizing: border-box;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 60px;
}

/* 左侧品牌展台：明亮大方、层次分明 */
.hero-showcase-panel {
  flex: 1.15;
  max-width: 680px;
}

.hero-tag {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 5px 12px;
  border-radius: 30px;
  background: #eff6ff;
  border: 1px solid #bfdbfe;
  color: #2563eb;
  font-size: 13px;
  font-weight: 600;
  margin-bottom: 20px;
}

.hero-headline {
  font-size: 38px;
  font-weight: 800;
  line-height: 1.25;
  margin: 0 0 16px;
  color: #0f172a;
  letter-spacing: -0.5px;
}

.gradient-text {
  background: linear-gradient(135deg, #2563eb 0%, #4f46e5 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
}

.hero-description {
  font-size: 15px;
  line-height: 1.65;
  color: #475569;
  margin: 0 0 32px;
}

/* 特性矩阵卡片：白净微润卡片 */
.feature-matrix-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 16px;
  margin-bottom: 32px;
}

.feature-card {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 16px 18px;
  background: #ffffff;
  border: 1px solid #e2e8f0;
  border-radius: 12px;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.03);
  transition: all 0.25s ease;
}

.feature-card:hover {
  background: #ffffff;
  border-color: #93c5fd;
  transform: translateY(-2px);
  box-shadow: 0 8px 24px rgba(37, 99, 235, 0.08);
}

.feature-icon-box {
  width: 38px;
  height: 38px;
  border-radius: 9px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 18px;
  flex-shrink: 0;
}

.icon-shield { background: #eff6ff; border: 1px solid #dbeafe; }
.icon-workflow { background: #f5f3ff; border: 1px solid #ede9fe; }
.icon-speed { background: #fffbeb; border: 1px solid #fef3c7; }
.icon-bell { background: #ecfdf5; border: 1px solid #d1fae5; }

.feature-title {
  font-size: 13.5px;
  font-weight: 700;
  color: #1e293b;
  margin-bottom: 4px;
}

.feature-desc {
  font-size: 12px;
  line-height: 1.5;
  color: #64748b;
}

/* 底部引擎标签 */
.db-ecosystem-footer {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
  padding-top: 20px;
  border-top: 1px solid #e2e8f0;
}

.db-eco-label {
  font-size: 12px;
  color: #64748b;
  font-weight: 600;
}

.db-badges-wrap {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.db-badge {
  font-size: 11px;
  font-weight: 600;
  padding: 3px 8px;
  border-radius: 6px;
  background: #ffffff;
  border: 1px solid #e2e8f0;
  color: #475569;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
}

/* 右侧登录卡片容器：明亮纯净高光立体 */
.login-card-panel {
  flex: 0.95;
  max-width: 480px;
  width: 100%;
}

.login-card-glass {
  background: #ffffff;
  border: 1px solid #e2e8f0;
  border-radius: 20px;
  padding: 32px 30px 26px;
  box-shadow: 
    0 20px 40px -15px rgba(15, 23, 42, 0.08),
    0 1px 3px rgba(0, 0, 0, 0.03);
}

.card-header {
  margin-bottom: 20px;
}

.card-title-row {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 6px;
}

.card-title {
  font-size: 24px;
  font-weight: 800;
  color: #0f172a;
  margin: 0;
  letter-spacing: 0.3px;
}

.card-title-badge {
  font-size: 11.5px;
  font-weight: 600;
  color: #2563eb;
  background: #eff6ff;
  border: 1px solid #bfdbfe;
  padding: 2px 8px;
  border-radius: 20px;
}

.card-subtitle {
  font-size: 13px;
  color: #64748b;
  margin: 0;
}

/* 现代胶囊选项卡：白底柔灰分段器 */
.segmented-tabs-bar {
  display: flex;
  background: #f1f5f9;
  padding: 4px;
  border-radius: 12px;
  border: 1px solid #e2e8f0;
  margin-bottom: 22px;
  gap: 4px;
}

.tab-pill-btn {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 8px 12px;
  background: transparent;
  border: none;
  border-radius: 8px;
  color: #64748b;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s ease;
}

.tab-pill-btn:hover {
  color: #0f172a;
}

.tab-pill-btn.active {
  background: #ffffff;
  color: #2563eb;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
}

.tab-content-fade {
  animation: fadeIn 0.25s ease;
}

@keyframes fadeIn {
  from { opacity: 0; transform: translateY(4px); }
  to { opacity: 1; transform: translateY(0); }
}

/* 表单定制输入框：高质感轻盈边框 */
.custom-login-form :deep(.el-form-item) {
  margin-bottom: 18px;
}

.custom-login-form :deep(.el-form-item__label) {
  color: #334155;
  font-size: 13px;
  font-weight: 600;
  padding-bottom: 6px;
}

.tech-input :deep(.el-input__wrapper) {
  background: #ffffff !important;
  border: 1px solid #d1d5db !important;
  box-shadow: none !important;
  border-radius: 10px !important;
  padding: 6px 14px;
  transition: all 0.2s;
}

.tech-input :deep(.el-input__wrapper:hover) {
  border-color: #93c5fd !important;
}

.tech-input :deep(.el-input__wrapper.is-focus) {
  border-color: #2563eb !important;
  box-shadow: 0 0 0 3px rgba(37, 99, 235, 0.15) !important;
}

.tech-input :deep(.el-input__inner) {
  color: #0f172a !important;
  font-size: 14px;
}

.tech-input :deep(.el-input__inner::placeholder) {
  color: #94a3b8 !important;
  font-size: 13px;
}

.tech-input :deep(.el-input__prefix-inner) {
  color: #2563eb;
  font-size: 16px;
}

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
  width: 120px;
  height: 42px;
  border-radius: 10px;
  font-size: 13px;
  font-weight: 600;
}

/* 提交按钮：高级品牌蓝渐变高光 */
.tech-submit-btn {
  width: 100%;
  height: 44px;
  border-radius: 10px;
  font-size: 15px;
  font-weight: 700;
  letter-spacing: 1px;
  background: linear-gradient(135deg, #2563eb 0%, #1d4ed8 100%) !important;
  border: none !important;
  box-shadow: 0 4px 14px rgba(37, 99, 235, 0.35);
  transition: all 0.2s;
}

.tech-submit-btn:hover {
  transform: translateY(-1px);
  box-shadow: 0 6px 20px rgba(37, 99, 235, 0.45);
}

.tech-submit-btn:active {
  transform: translateY(0);
}

/* 扫码登录区 */
.qr-login-container {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 4px 0 10px;
}

.qr-channel-selector {
  display: flex;
  background: #f1f5f9;
  padding: 4px;
  border-radius: 10px;
  border: 1px solid #e2e8f0;
  margin-bottom: 16px;
  width: 100%;
}

.channel-tab {
  flex: 1;
  text-align: center;
  padding: 7px 4px;
  font-size: 12px;
  font-weight: 600;
  color: #64748b;
  cursor: pointer;
  border-radius: 7px;
  transition: all 0.2s;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 4px;
}

.channel-tab.active {
  background: #ffffff;
  color: #2563eb;
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.08);
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
  border: 1px solid #e2e8f0;
  border-radius: 14px;
  background: #ffffff;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.08);
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
  background: #ffffff;
  border-radius: 7px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
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
  background: rgba(255, 255, 255, 0.95);
  color: #1e293b;
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
  color: #475569;
}

.qr-expire-tag {
  font-size: 12px;
  color: #94a3b8;
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
  margin-top: 18px;
  padding: 12px 14px;
  background: #f8fafc;
  border: 1px solid #e2e8f0;
  border-radius: 12px;
}

.strip-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}

.strip-title {
  font-size: 12px;
  font-weight: 700;
  color: #475569;
}

.strip-hint {
  font-size: 11px;
  color: #2563eb;
  font-weight: 500;
}

.role-chips-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.role-chip {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 5px 9px;
  border-radius: 6px;
  background: #ffffff;
  border: 1px solid #e2e8f0;
  color: #475569;
  font-size: 12px;
  cursor: pointer;
  transition: all 0.2s;
}

.role-chip:hover {
  background: #eff6ff;
  border-color: #93c5fd;
  color: #2563eb;
}

.role-chip.active {
  background: #dbeafe;
  border-color: #3b82f6;
  color: #1d4ed8;
  font-weight: 600;
}

.role-icon {
  font-size: 12px;
}

.role-label {
  font-size: 11.5px;
}

/* 底部辅助链接 */
.card-footer-actions {
  display: flex;
  justify-content: center;
  align-items: center;
  gap: 6px;
  margin-top: 14px;
  font-size: 12.5px;
}

.footer-hint {
  color: #64748b;
}

.register-link {
  font-size: 12.5px;
  font-weight: 600;
}

.security-meta-bar {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  margin-top: 12px;
  padding-top: 10px;
  border-top: 1px solid #f1f5f9;
  font-size: 11px;
  color: #94a3b8;
}

/* 页脚版权：清亮微边界 */
.bottom-copyright-bar {
  position: relative;
  z-index: 10;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 16px 20px;
  border-top: 1px solid #e2e8f0;
  font-size: 12px;
  color: #64748b;
  background: #ffffff;
}

.pipe-split {
  opacity: 0.4;
}

/* 响应式适配 */
@media (max-width: 1080px) {
  .login-main-stage {
    flex-direction: column;
    padding: 30px 24px;
    gap: 36px;
  }
  
  .hero-showcase-panel {
    max-width: 100%;
    text-align: center;
  }

  .hero-tag {
    margin: 0 auto 16px;
  }

  .hero-headline {
    font-size: 28px;
  }

  .feature-matrix-grid {
    text-align: left;
  }

  .db-ecosystem-footer {
    justify-content: center;
  }

  .login-card-panel {
    max-width: 480px;
  }
}

@media (max-width: 640px) {
  .top-nav-bar {
    padding: 14px 20px;
  }

  .nav-status-group {
    display: none;
  }

  .feature-matrix-grid {
    grid-template-columns: 1fr;
  }

  .login-card-glass {
    padding: 24px 18px;
  }
}
</style>
