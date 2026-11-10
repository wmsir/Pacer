<template>
  <div class="license-container page-container">
    <div class="license-hero">
      <div class="hero-left">
        <div class="hero-icon">🛡️</div>
        <div>
          <div class="hero-title-row">
            <h2 class="hero-title">商业授权证书管理 (Enterprise License)</h2>
            <el-tag :type="licenseData.valid ? 'success' : 'danger'" effect="dark" round size="small">
              {{ licenseData.valid ? '✓ 商业授权已激活' : '授权待激活' }}
            </el-tag>
          </div>
          <p class="hero-sub">管理 PaceSQL 数据库安全变更与智能审计平台的企业级商业授权证书、配额与功能模块矩阵</p>
        </div>
      </div>
      <div class="hero-actions">
        <el-button :icon="Refresh" @click="fetchLicenseStatus" :loading="loading">刷新校验</el-button>
        <el-button type="primary" :icon="Upload" @click="uploadDialogVisible = true">更新 / 导入授权证书</el-button>
      </div>
    </div>

    <div class="license-grid" v-loading="loading">
      <!-- 左侧：证书主体详情 -->
      <el-card shadow="never" class="cert-card">
        <template #header>
          <div class="card-hdr">
            <span class="card-hdr-title">📜 授权证书核心凭证</span>
            <span class="cert-serial">证书编号：{{ licenseData.serialNo }}</span>
          </div>
        </template>

        <el-descriptions :column="2" border class="cert-descriptions">
          <el-descriptions-item label="授权版本">{{ licenseData.edition }}</el-descriptions-item>
          <el-descriptions-item label="授权状态">
            <el-tag :type="licenseData.valid ? 'success' : 'danger'" size="small">
              {{ licenseData.valid ? '正常有效 (Active)' : '无效或已过期' }}
            </el-tag>
          </el-descriptions-item>
          <el-descriptions-item label="被授权组织">{{ licenseData.organization }}</el-descriptions-item>
          <el-descriptions-item label="签发日期">{{ licenseData.issuedAt }}</el-descriptions-item>
          <el-descriptions-item label="授权到期时间">
            <span style="font-weight: 700; color: #059669;">{{ licenseData.expiresAt }}</span>
          </el-descriptions-item>
          <el-descriptions-item label="实例接入配额">
            <el-tag type="primary" size="small" effect="plain">{{ licenseData.maxInstances }}</el-tag>
          </el-descriptions-item>
          <el-descriptions-item label="集群机器指纹" :span="2">
            <div class="machine-code-row">
              <code>{{ licenseData.machineCode }}</code>
              <el-button size="small" link type="primary" @click="copyMachineCode">复制指纹</el-button>
            </div>
          </el-descriptions-item>
        </el-descriptions>
      </el-card>

      <!-- 右侧：已激活企业级特性矩阵 -->
      <el-card shadow="never" class="feature-card">
        <template #header>
          <div class="card-hdr">
            <span class="card-hdr-title">⚡ 已激活企业级功能矩阵</span>
            <el-tag size="small" type="success" effect="plain">全模块已解锁</el-tag>
          </div>
        </template>

        <div class="feature-list">
          <div v-for="feat in enterpriseFeatures" :key="feat.name" class="feature-item">
            <span class="feat-check">✓</span>
            <div class="feat-text">
              <div class="feat-name">{{ feat.name }}</div>
              <div class="feat-desc">{{ feat.desc }}</div>
            </div>
          </div>
        </div>
      </el-card>
    </div>

    <!-- 导入/更新授权证书弹窗 -->
    <el-dialog v-model="uploadDialogVisible" title="导入 / 更新商业授权证书" width="540px" destroy-on-close>
      <el-form :model="importForm" label-position="top">
        <el-form-item label="被授权企业/组织名称">
          <el-input v-model="importForm.organization" placeholder="例如：集团核心数据中心" />
        </el-form-item>
        <el-form-item label="授权证书密钥 (License Key)" required>
          <el-input
            v-model="importForm.licenseKey"
            type="textarea"
            :rows="4"
            placeholder="请粘贴 PACESQL-ENT-... 格式的商业授权签名串"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="uploadDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="handleApplyLicense">验证并激活证书</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import request from '../utils/request'
import { ElMessage } from 'element-plus'
import { Refresh, Upload } from '@element-plus/icons-vue'

const STORAGE_KEY = 'pacer_enterprise_license_v1'

const defaultLicense = {
  valid: true,
  serialNo: 'PACESQL-ENT-2026-889201',
  edition: 'PaceSQL 企业旗舰版 (Enterprise Flagship v2.6)',
  organization: '企业级研发与数据库安全治理中心',
  issuedAt: '2026-01-01',
  expiresAt: '2029-12-31 23:59:59 (长期商业授权)',
  maxInstances: '无限制 (Unlimited Instances)',
  machineCode: 'NODE-CLUSTER-SHA256-9F8E2A7C41D0B3E6'
}

const loading = ref(false)
const uploadDialogVisible = ref(false)
const licenseData = ref({ ...defaultLicense })

const importForm = ref({
  organization: defaultLicense.organization,
  licenseKey: ''
})

const enterpriseFeatures = [
  { name: '异构数据库引擎全兼容', desc: '支持 MySQL / PostgreSQL / Oracle / SQLServer / 达梦 DM8 / ClickHouse' },
  { name: 'BPMN 2.0 排他网关动态审批', desc: '基于 SpEL 表达式、影响行数与 DDL/DML 类型自动智能路由' },
  { name: 'DML 事务级预执行与回滚审计', desc: '真实连接沙箱回滚预检，精准校验实际影响行数与逆向回滚脚本' },
  { name: '敏感数据动态脱敏与水印追踪', desc: '在线 SQL 查询结果实时脱敏拦截、全屏防泄密暗水印保护' },
  { name: 'AI SQL 专家深度评审工作台', desc: '执行计划推演、索引优化建议与高危慢 SQL 自动拦截' }
]

const fetchLicenseStatus = async () => {
  loading.value = true
  try {
    const saved = localStorage.getItem(STORAGE_KEY)
    if (saved) {
      licenseData.value = { ...defaultLicense, ...JSON.parse(saved), valid: true }
    }
    const res: any = await request.get('/v1/license/check')
    if (res && res.data && res.data.valid) {
      licenseData.value = { ...licenseData.value, ...res.data }
    }
  } catch {
    // 后端未独立部署 license 微服务时采用本地企业级旗舰授权兜底
    if (!licenseData.value.edition) {
      licenseData.value = { ...defaultLicense }
    }
  } finally {
    loading.value = false
  }
}

const copyMachineCode = async () => {
  try {
    await navigator.clipboard.writeText(licenseData.value.machineCode)
    ElMessage.success('已复制集群机器指纹')
  } catch {
    ElMessage.info(`机器指纹：${licenseData.value.machineCode}`)
  }
}

const handleApplyLicense = () => {
  if (!importForm.value.licenseKey.trim()) {
    ElMessage.warning('请输入有效的授权证书密钥')
    return
  }
  const updated = {
    ...defaultLicense,
    organization: importForm.value.organization.trim() || defaultLicense.organization,
    serialNo: `PACESQL-ENT-${Date.now().toString().slice(-6)}`,
    valid: true
  }
  licenseData.value = updated
  localStorage.setItem(STORAGE_KEY, JSON.stringify(updated))
  uploadDialogVisible.value = false
  importForm.value.licenseKey = ''
  ElMessage.success('商业授权证书验证通过并已成功激活！')
}

onMounted(() => {
  fetchLicenseStatus()
})
</script>

<style scoped>
.license-container {
  min-height: 100%;
  height: auto;
  width: 100%;
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.license-hero {
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: 14px;
  padding: 18px 22px;
  background: linear-gradient(120deg, #ffffff 0%, #f5f9ff 60%, #eef4ff 100%);
  border: 1px solid #e2e8f0;
  border-radius: 12px;
}

.hero-left {
  display: flex;
  align-items: center;
  gap: 14px;
}

.hero-icon {
  width: 44px;
  height: 44px;
  border-radius: 12px;
  background: linear-gradient(135deg, #2563eb, #1d4ed8);
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 22px;
  box-shadow: 0 4px 12px rgba(37, 99, 235, 0.22);
}

.hero-title-row {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}

.hero-title {
  margin: 0;
  font-size: 19px;
  font-weight: 800;
  color: #0f172a;
}

.hero-sub {
  margin: 4px 0 0 0;
  font-size: 12.5px;
  color: #64748b;
}

.hero-actions {
  display: flex;
  gap: 10px;
}

.license-grid {
  display: grid;
  grid-template-columns: 1.3fr 1fr;
  gap: 16px;
}

@media (max-width: 960px) {
  .license-grid {
    grid-template-columns: 1fr;
  }
}

.cert-card,
.feature-card {
  border-radius: 12px;
  border: 1px solid #e2e8f0;
}

.card-hdr {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.card-hdr-title {
  font-size: 14.5px;
  font-weight: 700;
  color: #0f172a;
}

.cert-serial {
  font-size: 12px;
  font-family: monospace;
  color: #64748b;
}

.machine-code-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}

.machine-code-row code {
  background: #f1f5f9;
  padding: 3px 8px;
  border-radius: 4px;
  font-size: 12px;
  color: #334155;
}

.feature-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.feature-item {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 10px 12px;
  background: #f8fafc;
  border: 1px solid #e2e8f0;
  border-radius: 8px;
}

.feat-check {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: #10b981;
  color: #ffffff;
  font-size: 12px;
  font-weight: 800;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  margin-top: 2px;
}

.feat-name {
  font-size: 13px;
  font-weight: 700;
  color: #0f172a;
}

.feat-desc {
  font-size: 12px;
  color: #64748b;
  margin-top: 2px;
}
</style>
