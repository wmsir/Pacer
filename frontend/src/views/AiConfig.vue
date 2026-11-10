<template>
  <div class="ai-config-container page-container">
    <!-- 顶部标题与全局状态卡片 -->
    <div class="header-action">
      <div class="title-area">
        <h2 class="page-title">
          <span class="title-icon">🤖</span>
          AI 大模型集成与多提供商配置 (LLM Model Providers)
        </h2>
        <div class="page-subtitle">
          支持接入 DeepSeek、通义千问、OpenAI、智谱清言、Moonshot Kimi、百度千帆、Ollama 本地大模型及自定义 OpenAI 兼容接口，配置完成后可一键检测连通性。
        </div>
      </div>
      <div class="action-area">
        <el-button :icon="Refresh" :loading="loading" @click="fetchConfig">刷新配置</el-button>
        <el-button type="primary" :icon="Check" :loading="saving" @click="handleSaveConfig">
          保存全部配置
        </el-button>
      </div>
    </div>

    <!-- 全局激活大模型状态栏 -->
    <el-card shadow="never" class="active-status-card">
      <div class="status-content">
        <div class="status-left">
          <div class="pulse-indicator" :class="{ active: currentActiveProvider?.hasApiKey || currentActiveProvider?.providerId === 'ollama' }"></div>
          <div class="status-info">
            <div class="status-title">
              当前系统激活大模型：
              <span class="active-name">{{ currentActiveProvider?.providerName || 'DeepSeek' }}</span>
              <el-tag size="small" effect="dark" type="success" style="margin-left: 8px;">
                {{ currentActiveProvider?.model || 'deepseek-chat' }}
              </el-tag>
              <el-tag
                size="small"
                :type="currentActiveProvider?.hasApiKey || currentActiveProvider?.providerId === 'ollama' ? 'success' : 'warning'"
                effect="plain"
                style="margin-left: 8px;"
              >
                {{ currentActiveProvider?.hasApiKey || currentActiveProvider?.providerId === 'ollama' ? '✅ 已配置 API Key' : '⚠️ 尚未配置 API Key' }}
              </el-tag>
            </div>
            <div class="status-desc">
              全站 Text2SQL、SQL 性能审查、自动重写、安全风险分析与执行计划解释将默认路由至该模型。
            </div>
          </div>
        </div>

        <div class="status-right">
          <span class="switch-label">切换生效提供商：</span>
          <el-select v-model="selectedActiveId" style="width: 260px;" @change="handleActiveChange">
            <el-option
              v-for="p in providerList"
              :key="p.providerId"
              :label="`${p.icon} ${p.providerName}`"
              :value="p.providerId"
            >
              <div style="display: flex; justify-content: space-between; align-items: center;">
                <span>{{ p.icon }} {{ p.providerName }}</span>
                <el-tag size="small" :type="p.hasApiKey || p.providerId === 'ollama' ? 'success' : 'info'" effect="plain">
                  {{ p.hasApiKey || p.providerId === 'ollama' ? '已配置' : '待配置' }}
                </el-tag>
              </div>
            </el-option>
          </el-select>
        </div>
      </div>
    </el-card>

    <!-- 主流大模型提供商矩阵卡片 -->
    <div class="provider-matrix-section">
      <div class="section-title">
        <span>主流大模型预设与自定义网关</span>
        <small>（点击卡片即可在下方编辑参数与执行实时连通性测试）</small>
      </div>

      <el-row :gutter="14" class="matrix-row">
        <el-col
          v-for="p in providerList"
          :key="p.providerId"
          :xs="24" :sm="12" :md="8" :lg="6"
          class="matrix-col"
        >
          <div
            class="provider-card"
            :class="{
              selected: currentEditId === p.providerId,
              'is-active': selectedActiveId === p.providerId
            }"
            @click="currentEditId = p.providerId"
          >
            <div class="card-top">
              <div class="p-icon">{{ p.icon }}</div>
              <div class="p-meta">
                <div class="p-name">{{ p.providerName }}</div>
                <div class="p-model">{{ p.model }}</div>
              </div>
            </div>

            <div class="card-desc">{{ p.description }}</div>

            <div class="card-bottom">
              <el-tag
                size="small"
                :type="p.hasApiKey || p.providerId === 'ollama' ? 'success' : 'info'"
                effect="plain"
              >
                {{ p.hasApiKey || p.providerId === 'ollama' ? '● Key 已就绪' : '○ 待填 Key' }}
              </el-tag>

              <el-tag v-if="selectedActiveId === p.providerId" size="small" type="primary" effect="dark">
                ★ 运行中
              </el-tag>
              <el-button
                v-else
                size="small"
                type="primary"
                link
                @click.stop="setAsActive(p.providerId)"
              >
                设为当前激活
              </el-button>
            </div>
          </div>
        </el-col>
      </el-row>
    </div>

    <!-- 选中模型的配置与健康自检区域 -->
    <div class="config-detail-section" v-if="editProvider">
      <el-row :gutter="20">
        <!-- 左侧：参数配置表单 -->
        <el-col :xs="24" :lg="14">
          <el-card shadow="hover" class="form-card">
            <template #header>
              <div class="card-header-flex">
                <div class="header-title">
                  <span class="icon">{{ editProvider.icon }}</span>
                  <span class="name">{{ editProvider.providerName }} - 详细配置</span>
                  <el-tag v-if="editProvider.isCustom" size="small" type="warning" effect="plain">自定义协议</el-tag>
                </div>
                <div class="header-links">
                  <el-link
                    v-if="editProvider.apiKeyDocUrl"
                    :href="editProvider.apiKeyDocUrl"
                    target="_blank"
                    type="primary"
                    :underline="false"
                    style="margin-right: 12px; font-size: 13px;"
                  >
                    🔑 获取 API Key
                  </el-link>
                  <el-link
                    v-if="editProvider.officialWebsite"
                    :href="editProvider.officialWebsite"
                    target="_blank"
                    type="info"
                    :underline="false"
                    style="font-size: 13px;"
                  >
                    🌐 官方文档
                  </el-link>
                </div>
              </div>
            </template>

            <el-form label-position="top" class="detail-form">
              <!-- API Endpoint -->
              <el-form-item label="接口请求端点 (API Base URL / Endpoint)">
                <el-input
                  v-model="editProvider.endpoint"
                  placeholder="https://api.example.com/v1/chat/completions"
                  clearable
                >
                  <template #prepend>POST</template>
                </el-input>
                <div class="form-hint">需兼容 OpenAI Chat Completions 标准协议接口规范。</div>
              </el-form-item>

              <!-- Model Name -->
              <el-form-item label="模型名称 (Model Identifier)">
                <el-select
                  v-model="editProvider.model"
                  filterable
                  allow-create
                  default-first-option
                  placeholder="请选择预置模型或直接输入自定义模型名称"
                  style="width: 100%;"
                >
                  <el-option
                    v-for="m in editProvider.presetModels || []"
                    :key="m"
                    :label="m"
                    :value="m"
                  />
                </el-select>
                <div class="form-hint">支持下拉选择官方推荐型号，也可直接输入自建私有模型标识（如 <code>deepseek-reasoner</code>、<code>qwen-coder-plus</code> 等）。</div>
              </el-form-item>

              <!-- API Key -->
              <el-form-item label="API 鉴权密钥 (API Key / Token)">
                <el-input
                  v-model="editProvider.apiKey"
                  :type="showApiKey ? 'text' : 'password'"
                  placeholder="sk-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
                  clearable
                >
                  <template #append>
                    <el-button @click="showApiKey = !showApiKey">
                      {{ showApiKey ? '隐藏' : '显示' }}
                    </el-button>
                  </template>
                </el-input>
                <div class="form-hint">
                  密钥将加密保存；输入新 Key 保存后即时生效。Ollama 等本地私有化部署若免密可留空或填 <code>ollama-local</code>。
                </div>
              </el-form-item>

              <!-- 高级调节参数 -->
              <el-collapse class="advanced-collapse">
                <el-collapse-item title="⚙️ 高级超参数设置 (Temperature / Max Tokens / Timeout)" name="1">
                  <el-row :gutter="16">
                    <el-col :span="12">
                      <el-form-item label="采样温度 Temperature (0.0 ~ 1.0)">
                        <div class="slider-row">
                          <el-slider
                            v-model="editProvider.temperature"
                            :min="0.0"
                            :max="1.0"
                            :step="0.05"
                            style="flex: 1; margin-right: 12px;"
                          />
                          <span class="param-val">{{ editProvider.temperature }}</span>
                        </div>
                        <div class="form-sub-hint">SQL 调优推荐 0.1~0.3（更严谨精准），创意解释推荐 0.7。</div>
                      </el-form-item>
                    </el-col>

                    <el-col :span="12">
                      <el-form-item label="最大生成 Tokens (Max Tokens)">
                        <el-input-number
                          v-model="editProvider.maxTokens"
                          :min="512"
                          :max="16384"
                          :step="512"
                          style="width: 100%;"
                        />
                        <div class="form-sub-hint">控制单次 SQL 诊断或执行计划分析的最大输出长度。</div>
                      </el-form-item>
                    </el-col>
                  </el-row>

                  <el-form-item label="接口超时时间 (Timeout Seconds)">
                    <el-input-number
                      v-model="editProvider.timeoutSeconds"
                      :min="5"
                      :max="120"
                      :step="5"
                      style="width: 200px;"
                    />
                    <span style="margin-left: 8px; color: #909399; font-size: 13px;">秒 (建议 30 秒)</span>
                  </el-form-item>
                </el-collapse-item>
              </el-collapse>
            </el-form>
          </el-card>
        </el-col>

        <!-- 右侧：连通性测试与自检面板 + 实时沙箱 -->
        <el-col :xs="24" :lg="10">
          <!-- 1. 连通性测试面板 -->
          <el-card shadow="hover" class="test-card">
            <template #header>
              <div class="card-header-flex">
                <span class="header-title">
                  <el-icon color="#409EFF"><Connection /></el-icon>
                  连通性自检与健康探测
                </span>
                <el-button
                  type="primary"
                  :icon="Lightning"
                  :loading="testing"
                  @click="handleTestConnection"
                >
                  测试连接 / Ping
                </el-button>
              </div>
            </template>

            <div class="test-content">
              <div class="test-intro">
                向配置的 <b>{{ editProvider.providerName }}</b> 端点发送轻量级探测握手报文，验证 API Key 有效性、鉴权状态与网络延迟。
              </div>

              <!-- 测试进行中 -->
              <div v-if="testing" class="testing-state">
                <el-icon class="is-loading" style="font-size: 28px; color: #409EFF;"><Loading /></el-icon>
                <div class="testing-text">正在向大模型端点发起测试握手，请稍候...</div>
              </div>

              <!-- 测试结果展示 -->
              <div v-else-if="testResult" class="test-result-box" :class="{ success: testResult.success, error: !testResult.success }">
                <div class="result-header">
                  <span class="r-badge" :class="testResult.success ? 'r-success' : 'r-error'">
                    {{ testResult.success ? '✅ 测试通过 (Connected)' : '❌ 连接失败 (Failed)' }}
                  </span>
                  <span v-if="testResult.latencyMs" class="r-latency">
                    ⏱️ 延迟: <b>{{ testResult.latencyMs }} ms</b>
                  </span>
                </div>

                <div class="result-message">{{ testResult.message }}</div>

                <div v-if="testResult.responseText" class="result-response">
                  <div class="response-label">🤖 模型实际响应内容：</div>
                  <div class="response-text">{{ testResult.responseText }}</div>
                </div>

                <div v-if="testResult.errorDetails" class="result-error-detail">
                  <div class="error-label">📋 错误诊断详情：</div>
                  <pre class="error-pre">{{ testResult.errorDetails }}</pre>
                </div>
              </div>

              <!-- 未测试默认占位 -->
              <div v-else class="test-placeholder">
                <el-icon style="font-size: 36px; color: #C0C4CC; margin-bottom: 8px;"><Cpu /></el-icon>
                <div>尚未发起连通性自检</div>
                <small style="color: #909399;">点击右上角「测试连接」按钮开始检测</small>
              </div>
            </div>
          </el-card>

          <!-- 2. AI 快速体验沙箱 -->
          <el-card shadow="hover" class="sandbox-card" style="margin-top: 16px;">
            <template #header>
              <div class="card-header-flex">
                <span class="header-title">
                  <el-icon color="#67C23A"><ChatDotRound /></el-icon>
                  AI 实时体验沙箱 (Quick Sandbox)
                </span>
                <el-button size="small" type="success" :loading="chatLoading" @click="handleSandboxChat">
                  发送体验
                </el-button>
              </div>
            </template>

            <div class="sandbox-body">
              <el-input
                v-model="sandboxPrompt"
                type="textarea"
                :rows="3"
                placeholder="输入一条 SQL 诊断需求，例如：请为 SELECT * FROM t_policy_order WHERE user_id = 123 AND status = 'ACTIVE' 给出索引优化建议。"
              />
              <div v-if="sandboxReply" class="sandbox-reply">
                <div class="reply-header">🤖 AI 诊断回复：</div>
                <div class="reply-content markdown-body">{{ sandboxReply }}</div>
              </div>
            </div>
          </el-card>
        </el-col>
      </el-row>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { Refresh, Check, Connection, Lightning, Loading, Cpu, ChatDotRound } from '@element-plus/icons-vue'
import request from '../utils/request'

interface ProviderDetail {
  providerId: string
  providerName: string
  icon: string
  endpoint: string
  model: string
  apiKey: string
  hasApiKey?: boolean
  temperature: number
  maxTokens: number
  timeoutSeconds: number
  presetModels?: string[]
  officialWebsite?: string
  apiKeyDocUrl?: string
  description?: string
  isCustom?: boolean
}

const loading = ref(false)
const saving = ref(false)
const testing = ref(false)
const chatLoading = ref(false)
const showApiKey = ref(false)

const selectedActiveId = ref('deepseek')
const currentEditId = ref('deepseek')
const providersMap = ref<Record<string, ProviderDetail>>({})

const testResult = ref<any>(null)
const sandboxPrompt = ref('请审查这条 SQL：SELECT * FROM t_policy_order WHERE user_id = 8848 ORDER BY create_time DESC LIMIT 10 并给出优化建议。')
const sandboxReply = ref('')

const providerList = computed(() => {
  return Object.values(providersMap.value)
})

const currentActiveProvider = computed(() => {
  return providersMap.value[selectedActiveId.value] || providersMap.value['deepseek']
})

const editProvider = computed(() => {
  return providersMap.value[currentEditId.value]
})

const fetchConfig = async () => {
  loading.value = true
  try {
    const res: any = await request.get('/v1/ai/config')
    if (res.data) {
      selectedActiveId.value = res.data.activeProvider || 'deepseek'
      providersMap.value = res.data.providers || {}
      if (!providersMap.value[currentEditId.value]) {
        currentEditId.value = selectedActiveId.value
      }
    }
  } catch (err: any) {
    ElMessage.error('获取 AI 模型配置失败')
  } finally {
    loading.value = false
  }
}

const handleActiveChange = (val: string) => {
  currentEditId.value = val
}

const setAsActive = (pId: string) => {
  selectedActiveId.value = pId
  currentEditId.value = pId
  ElMessage.info(`已切换当前激活模型为【${providersMap.value[pId]?.providerName}】，点击右上角「保存全部配置」即可永久生效。`)
}

const handleSaveConfig = async () => {
  saving.value = true
  try {
    const payload = {
      activeProvider: selectedActiveId.value,
      enabled: true,
      providers: providersMap.value
    }
    await request.post('/v1/ai/config', payload)
    ElMessage.success('🎉 AI 模型全局配置与激活通道已成功保存！全站智能治理即刻生效。')
    fetchConfig()
  } catch (err: any) {
    ElMessage.error(err.response?.data?.message || '保存配置失败')
  } finally {
    saving.value = false
  }
}

const handleTestConnection = async () => {
  if (!editProvider.value) return
  testing.value = true
  testResult.value = null

  try {
    const reqPayload = {
      providerId: editProvider.value.providerId,
      endpoint: editProvider.value.endpoint,
      model: editProvider.value.model,
      apiKey: editProvider.value.apiKey,
      temperature: editProvider.value.temperature,
      maxTokens: editProvider.value.maxTokens,
      timeoutSeconds: editProvider.value.timeoutSeconds
    }

    const res: any = await request.post('/v1/ai/test-connection', reqPayload)
    testResult.value = res.data
    if (res.data?.success) {
      ElMessage.success(res.data.message || '连通性测试通过！')
    } else {
      ElMessage.warning(res.data?.message || '连通性测试未通过，请检查错误提示')
    }
  } catch (err: any) {
    testResult.value = {
      success: false,
      latencyMs: 0,
      message: '测试请求异常: ' + (err.response?.data?.message || err.message),
      errorDetails: err.toString()
    }
    ElMessage.error('连通性测试请求失败')
  } finally {
    testing.value = false
  }
}

const handleSandboxChat = async () => {
  if (!sandboxPrompt.value.trim()) {
    ElMessage.warning('请输入体验内容')
    return
  }
  chatLoading.value = true
  sandboxReply.value = ''
  try {
    const res: any = await request.post('/v1/ai/chat', {
      prompt: sandboxPrompt.value.trim(),
      systemPrompt: '你是一位顶级的企业级 DBA 专家与 SQL 架构师。'
    })
    sandboxReply.value = res.data
  } catch (err: any) {
    ElMessage.error('体验对话请求失败，请先检查模型配置与连通性')
  } finally {
    chatLoading.value = false
  }
}

onMounted(() => {
  fetchConfig()
})
</script>

<style scoped>
.ai-config-container {
  padding: 20px;
}

.header-action {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.page-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 20px;
  font-weight: 600;
  color: #303133;
  margin: 0 0 6px 0;
}

.title-icon {
  font-size: 22px;
}

.page-subtitle {
  font-size: 13px;
  color: #909399;
}

/* 激活状态卡片 */
.active-status-card {
  background: linear-gradient(135deg, #f0f7ff 0%, #ffffff 100%);
  border: 1px solid #d0e4ff;
  border-radius: 8px;
  margin-bottom: 24px;
}

.status-content {
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: 16px;
}

.status-left {
  display: flex;
  align-items: center;
  gap: 14px;
}

.pulse-indicator {
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background-color: #e6a23c;
  box-shadow: 0 0 0 0 rgba(230, 162, 60, 0.7);
  animation: pulse 2s infinite;
}

.pulse-indicator.active {
  background-color: #67c23a;
  box-shadow: 0 0 0 0 rgba(103, 194, 58, 0.7);
}

@keyframes pulse {
  0% {
    transform: scale(0.95);
    box-shadow: 0 0 0 0 rgba(103, 194, 58, 0.7);
  }
  70% {
    transform: scale(1);
    box-shadow: 0 0 0 8px rgba(103, 194, 58, 0);
  }
  100% {
    transform: scale(0.95);
    box-shadow: 0 0 0 0 rgba(103, 194, 58, 0);
  }
}

.status-title {
  font-size: 15px;
  font-weight: 600;
  color: #303133;
  display: flex;
  align-items: center;
}

.active-name {
  color: #409eff;
  font-weight: 700;
  font-size: 16px;
  margin-left: 4px;
}

.status-desc {
  font-size: 12px;
  color: #909399;
  margin-top: 4px;
}

.status-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

.switch-label {
  font-size: 13px;
  color: #606266;
  font-weight: 500;
}

/* 提供商矩阵 */
.provider-matrix-section {
  margin-bottom: 24px;
}

.section-title {
  font-size: 15px;
  font-weight: 600;
  color: #303133;
  margin-bottom: 12px;
  display: flex;
  align-items: baseline;
  gap: 8px;
}

.section-title small {
  font-size: 12px;
  color: #909399;
  font-weight: normal;
}

.matrix-row {
  margin-bottom: -14px;
}

.matrix-col {
  margin-bottom: 14px;
}

.provider-card {
  background: #ffffff;
  border: 1px solid #e4e7ed;
  border-radius: 8px;
  padding: 14px;
  cursor: pointer;
  transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  display: flex;
  flex-direction: column;
  height: 100%;
  box-sizing: border-box;
}

.provider-card:hover {
  border-color: #409eff;
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(64, 158, 255, 0.12);
}

.provider-card.selected {
  border-color: #409eff;
  background-color: #f5f9ff;
}

.provider-card.is-active {
  border-color: #67c23a;
  box-shadow: 0 0 0 1px #67c23a;
}

.card-top {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 8px;
}

.p-icon {
  font-size: 24px;
  line-height: 1;
}

.p-meta {
  flex: 1;
  overflow: hidden;
}

.p-name {
  font-size: 14px;
  font-weight: 600;
  color: #303133;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.p-model {
  font-size: 12px;
  font-family: monospace;
  color: #909399;
}

.card-desc {
  font-size: 12px;
  color: #606266;
  line-height: 1.4;
  margin-bottom: 12px;
  flex: 1;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.card-bottom {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-top: auto;
}

/* 详情配置与测试 */
.config-detail-section {
  margin-top: 20px;
}

.form-card, .test-card, .sandbox-card {
  border-radius: 8px;
}

.card-header-flex {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.header-title {
  font-size: 15px;
  font-weight: 600;
  color: #303133;
  display: flex;
  align-items: center;
  gap: 6px;
}

.form-hint {
  font-size: 12px;
  color: #909399;
  line-height: 1.4;
  margin-top: 4px;
}

.form-sub-hint {
  font-size: 11px;
  color: #a8abb2;
  margin-top: 2px;
}

.slider-row {
  display: flex;
  align-items: center;
}

.param-val {
  font-family: monospace;
  font-weight: 600;
  color: #409eff;
}

.advanced-collapse {
  margin-top: 16px;
  border-radius: 6px;
  border: 1px solid #ebeef5;
}

:deep(.advanced-collapse .el-collapse-item__header) {
  padding-left: 12px;
  font-weight: 600;
  font-size: 13px;
  background-color: #fafafa;
}

:deep(.advanced-collapse .el-collapse-item__content) {
  padding: 14px 12px 0 12px;
}

/* 测试面板 */
.test-intro {
  font-size: 13px;
  color: #606266;
  line-height: 1.5;
  margin-bottom: 14px;
}

.testing-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 24px 0;
  gap: 10px;
}

.testing-text {
  font-size: 13px;
  color: #409eff;
}

.test-placeholder {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 30px 0;
  color: #909399;
  font-size: 13px;
  gap: 4px;
}

.test-result-box {
  padding: 14px;
  border-radius: 6px;
  font-size: 13px;
}

.test-result-box.success {
  background-color: #f0f9eb;
  border: 1px solid #c2e7b0;
}

.test-result-box.error {
  background-color: #fef0f0;
  border: 1px solid #fbc4c4;
}

.result-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}

.r-badge {
  font-weight: 600;
}

.r-success {
  color: #67c23a;
}

.r-error {
  color: #f56c6c;
}

.r-latency {
  font-size: 12px;
  color: #606266;
}

.result-message {
  font-weight: 500;
  color: #303133;
  margin-bottom: 8px;
}

.result-response {
  margin-top: 10px;
  background: #ffffff;
  padding: 10px;
  border-radius: 4px;
  border: 1px solid #e1f3d8;
}

.response-label {
  font-size: 12px;
  color: #67c23a;
  font-weight: 600;
  margin-bottom: 4px;
}

.response-text {
  font-size: 13px;
  color: #303133;
  line-height: 1.5;
  white-space: pre-wrap;
}

.result-error-detail {
  margin-top: 10px;
  background: #ffffff;
  padding: 10px;
  border-radius: 4px;
  border: 1px solid #fde2e2;
}

.error-label {
  font-size: 12px;
  color: #f56c6c;
  font-weight: 600;
  margin-bottom: 4px;
}

.error-pre {
  margin: 0;
  font-size: 11px;
  color: #909399;
  white-space: pre-wrap;
  word-break: break-all;
  font-family: monospace;
}

/* 沙箱 */
.sandbox-body {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.sandbox-reply {
  background-color: #fafafa;
  border: 1px solid #ebeef5;
  border-radius: 6px;
  padding: 12px;
}

.reply-header {
  font-size: 12px;
  font-weight: 600;
  color: #409eff;
  margin-bottom: 6px;
}

.reply-content {
  font-size: 13px;
  color: #303133;
  line-height: 1.6;
  white-space: pre-wrap;
}
</style>
