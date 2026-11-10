<template>
  <div class="ai-sql-review-container">
    <div class="page-header-flex">
      <div>
        <h2 class="page-title">AI SQL 智能审查与架构诊断</h2>
        <div class="page-subtitle">基于大模型（DeepSeek / 通义千问 / OpenAI / 智谱 GLM）进行 SQL 性能优化、风险审计与执行计划深度解析</div>
      </div>
      <div class="header-actions">
        <el-tag size="default" type="success" effect="plain" class="model-badge">
          🤖 当前生效引擎: <b>{{ activeModelName }}</b>
        </el-tag>
        <el-button
          v-if="userStore.isAdmin || userStore.hasPermission('/ai-config')"
          type="primary"
          :icon="Setting"
          plain
          @click="router.push('/ai-config')"
        >
          ⚙️ AI 模型配置与连通测试
        </el-button>
      </div>
    </div>

    <el-row :gutter="16">
      <el-col :xs="24" :sm="24" :md="12" class="col-item">
        <el-card shadow="hover" class="box-card">
          <template #header>
            <div class="card-header">
              <span>输入区 (SQL 语句 / EXPLAIN 执行计划)</span>
            </div>
          </template>
          <div class="card-body-content">
            <el-input
              v-model="inputContent"
              type="textarea"
              :rows="12"
              placeholder="请在此粘贴您的 SQL 语句或执行计划 (EXPLAIN 输出)..."
            ></el-input>
            <div class="action-buttons">
              <el-button type="primary" :loading="loading" @click="handleAction('explain')">SQL 解释</el-button>
              <el-button type="success" :loading="loading" @click="handleAction('optimize')">SQL 优化</el-button>
              <el-button type="warning" :loading="loading" @click="handleAction('rewrite')">SQL 重写</el-button>
              <el-button type="danger" :loading="loading" @click="handleAction('risk')">风险分析</el-button>
              <el-button type="info" :loading="loading" @click="handleAction('explain-plan')">解析执行计划</el-button>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="24" :md="12" class="col-item">
        <el-card shadow="hover" class="box-card">
          <template #header>
            <div class="card-header">
              <span>AI 智能诊断结果</span>
            </div>
          </template>
          <div class="result-area" v-loading="loading">
             <div v-if="aiResult" class="markdown-body">
               <div v-if="isUnconfiguredHint" class="unconfigured-banner">
                 <div class="banner-title">⚠️ 尚未配置真实的 AI API Key</div>
                 <div class="banner-desc">{{ aiResult }}</div>
                 <el-button
                   v-if="userStore.isAdmin || userStore.hasPermission('/ai-config')"
                   type="primary"
                   size="small"
                   style="margin-top: 10px;"
                   @click="router.push('/ai-config')"
                 >
                   👉 立即前往【AI 模型配置】页面配置 API Key
                 </el-button>
               </div>
               <div v-else>{{ aiResult }}</div>
             </div>
             <el-empty v-else description="暂无分析结果，请在左侧输入 SQL 后点击上方诊断按钮" />
          </div>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useUserStore } from '../store/user'
import { Setting } from '@element-plus/icons-vue'
import request from '../utils/request'
import { ElMessage } from 'element-plus'

const router = useRouter()
const userStore = useUserStore()
const inputContent = ref('')
const aiResult = ref('')
const loading = ref(false)
const activeModelName = ref('DeepSeek (deepseek-chat)')

const isUnconfiguredHint = computed(() => {
  return aiResult.value && (aiResult.value.includes('Mock LLM Response') || aiResult.value.includes('未配置真实 API Key') || aiResult.value.includes('(AI 提示)'))
})

const fetchActiveModel = async () => {
  try {
    const res: any = await request.get('/v1/ai/config')
    if (res.data) {
      const activeId = res.data.activeProvider || 'deepseek'
      const detail = res.data.providers?.[activeId]
      if (detail) {
        activeModelName.value = `${detail.providerName} (${detail.model})`
      }
    }
  } catch (err) {
    // ignore
  }
}

const handleAction = async (action: string) => {
  if (!inputContent.value.trim()) {
    ElMessage.warning('请输入需要分析的内容')
    return
  }

  loading.value = true
  aiResult.value = ''

  try {
    const res: any = await request.post(`/v1/ai/${action}`, { sql: inputContent.value })
    aiResult.value = res.data
  } catch (error) {
    ElMessage.error('AI 服务请求失败，请检查配置')
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  fetchActiveModel()
})
</script>

<style scoped>
.ai-sql-review-container {
  width: 100%;
  box-sizing: border-box;
}

.page-header-flex {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
  flex-wrap: wrap;
  gap: 12px;
}

.page-title {
  font-size: 20px;
  font-weight: 600;
  color: #303133;
  margin: 0 0 4px 0;
}

.page-subtitle {
  font-size: 13px;
  color: #909399;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 12px;
}

.model-badge {
  font-size: 13px;
}

.col-item {
  margin-bottom: 16px;
}

.box-card {
  min-height: 480px;
  height: calc(100vh - 180px);
  display: flex;
  flex-direction: column;
}

:deep(.el-card__body) {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  padding: 16px;
}

.card-body-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

:deep(.card-body-content .el-textarea) {
  flex: 1;
  display: flex;
}

:deep(.card-body-content .el-textarea__inner) {
  height: 100% !important;
  font-family: 'Consolas', 'Courier New', monospace;
  font-size: 13px;
  resize: none;
}

.action-buttons {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.result-area {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
  background-color: #fafafa;
  border-radius: 4px;
}

.markdown-body {
  font-size: 14px;
  line-height: 1.6;
  white-space: pre-wrap;
  color: #303133;
}

.unconfigured-banner {
  background-color: #fffbe6;
  border: 1px solid #ffe58f;
  padding: 16px;
  border-radius: 6px;
}

.banner-title {
  font-size: 15px;
  font-weight: 600;
  color: #d46b08;
  margin-bottom: 8px;
}

.banner-desc {
  font-size: 13px;
  color: #595959;
  line-height: 1.5;
}
</style>
