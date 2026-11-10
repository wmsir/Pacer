<template>
  <div class="settings-container page-container">
    <div class="settings-header">
      <h2 class="page-title">系统与个性化主题设置</h2>
      <div class="page-subtitle">定制控制台主色系、暗黑模式、以及全站背景实名防泄密水印显隐程度与样式</div>
    </div>

    <div class="settings-grid">
      <!-- 1. 基础主题与色彩配置 -->
      <el-card shadow="hover" class="settings-card">
        <template #header>
          <div class="card-header-title">
            <el-icon color="#409EFF"><Brush /></el-icon>
            <span>基础外观与色彩 (Theme & Colors)</span>
          </div>
        </template>

        <el-form label-position="left" label-width="140px">
          <el-form-item label="系统主色调 (Primary)">
            <div class="color-picker-row">
              <el-color-picker v-model="primaryColor" @change="handleColorChange" />
              <div class="color-presets">
                <span
                  v-for="color in themePresets"
                  :key="color.value"
                  class="color-pill"
                  :style="{ backgroundColor: color.value }"
                  :title="color.label"
                  @click="selectPresetColor(color.value)"
                >
                  <span v-if="primaryColor.toLowerCase() === color.value.toLowerCase()" class="check-mark">✓</span>
                </span>
              </div>
            </div>
          </el-form-item>

          <el-form-item label="暗黑深色模式">
            <div class="switch-row">
              <el-switch v-model="isDark" @change="toggleDark" active-text="开启暗黑模式" inactive-text="标准明亮" />
            </div>
          </el-form-item>
        </el-form>
      </el-card>

      <!-- 2. 背景实名防泄密水印设置 (核心优化需求) -->
      <el-card shadow="hover" class="settings-card">
        <template #header>
          <div class="card-header-title">
            <el-icon color="#E6A23C"><Stamp /></el-icon>
            <span>背景安全水印配置 (Security Watermark Settings)</span>
            <el-tag size="small" type="success" effect="plain" style="margin-left: 8px;">即时全局生效</el-tag>
          </div>
        </template>

        <el-form label-position="left" label-width="140px">
          <el-form-item label="背景水印总开关">
            <el-switch
              v-model="watermarkEnabled"
              active-text="启用全站背景水印"
              inactive-text="关闭水印显示"
              @change="handleWatermarkChange"
            />
          </el-form-item>

          <el-form-item label="水印显隐程度 (透明度)" :disabled="!watermarkEnabled">
            <div class="slider-wrapper">
              <el-slider
                v-model="watermarkOpacityPercent"
                :min="0"
                :max="50"
                :step="1"
                :format-tooltip="(val: number) => `${val}% (${getOpacityLabel(val)})`"
                @input="handleOpacitySlider"
                @change="handleWatermarkChange"
              />
              <span class="opacity-value-tag">{{ watermarkOpacityPercent }}%</span>
            </div>

            <!-- 快捷档位预设 -->
            <div class="opacity-presets">
              <span class="preset-title">快捷档位：</span>
              <el-button
                v-for="p in opacityPresets"
                :key="p.value"
                size="small"
                plain
                :type="watermarkOpacityPercent === p.value ? 'primary' : 'info'"
                @click="setOpacityPreset(p.value)"
              >
                {{ p.label }} ({{ p.value }}%)
              </el-button>
            </div>
          </el-form-item>

          <el-form-item label="自定义水印后缀说明">
            <el-input
              v-model="watermarkCustomText"
              placeholder="可补充单位/部门/环境标记，如：内部机密 · 严禁外传"
              clearable
              style="max-width: 360px;"
              @change="handleWatermarkChange"
            />
          </el-form-item>

          <!-- 实时效果预览卡片 -->
          <el-form-item label="水印效果实时预览">
            <div class="preview-stage-box" :style="{ backgroundColor: isDark ? '#1e293b' : '#f8fafc' }">
              <div
                v-if="watermarkEnabled && watermarkOpacityPercent > 0"
                class="mock-watermark-overlay"
                :style="previewWatermarkStyle"
              ></div>
              <div class="stage-content">
                <div class="stage-card">
                  <div class="stage-title">稳骥 PacerSQL 数据库智能运维自治平台</div>
                  <div class="stage-desc">
                    这是一张示例界面卡片。调节上方【水印显隐程度】滑块，可以实时观察文字清晰度与背景干扰程度，找到最适合您屏幕的平衡点！
                  </div>
                  <div class="stage-footer">
                    <el-tag size="small" type="primary">当前状态: {{ watermarkEnabled ? `显隐度 ${watermarkOpacityPercent}% (${getOpacityLabel(watermarkOpacityPercent)})` : '已隐藏' }}</el-tag>
                  </div>
                </div>
              </div>
            </div>
          </el-form-item>
        </el-form>
      </el-card>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useDark, useToggle } from '@vueuse/core'
import { ElMessage } from 'element-plus'
import { Brush, Stamp } from '@element-plus/icons-vue'

const isDark = useDark()
const toggleDark = useToggle(isDark)

const primaryColor = ref('#409EFF')

const themePresets = [
  { label: '经典蓝 (Element)', value: '#409EFF' },
  { label: '极光绿 (Emerald)', value: '#10B981' },
  { label: '深邃紫 (Violet)', value: '#8B5CF6' },
  { label: '赤焰红 (Rose)', value: '#EF4444' },
  { label: '琥珀金 (Amber)', value: '#F59E0B' },
  { label: '商务藏青 (Navy)', value: '#1E40AF' },
  { label: '暗夜灰黑 (Slate)', value: '#475569' }
]

const handleColorChange = (color: string) => {
  if (color) {
    document.documentElement.style.setProperty('--el-color-primary', color)
    localStorage.setItem('primaryColor', color)
    ElMessage.success(`主题色已切换为 ${color}`)
  }
}

const selectPresetColor = (color: string) => {
  primaryColor.value = color
  handleColorChange(color)
}

// ==================== 水印配置管理 ====================
const watermarkEnabled = ref(true)
const watermarkOpacityPercent = ref(16) // 0 - 50 (%)
const watermarkCustomText = ref('')

const opacityPresets = [
  { label: '极浅微透', value: 6 },
  { label: '优雅低调', value: 12 },
  { label: '标准适中', value: 18 },
  { label: '清晰防摄', value: 28 },
  { label: '强力防泄', value: 42 }
]

const getOpacityLabel = (val: number) => {
  if (val === 0) return '完全隐藏'
  if (val <= 8) return '极浅'
  if (val <= 14) return '微弱'
  if (val <= 22) return '标准'
  if (val <= 32) return '清晰'
  return '高对比度'
}

const setOpacityPreset = (val: number) => {
  watermarkOpacityPercent.value = val
  handleWatermarkChange()
}

const handleOpacitySlider = () => {
  // 动态触发预览与变更
}

const handleWatermarkChange = () => {
  localStorage.setItem('wm_watermark_enabled', String(watermarkEnabled.value))
  const opacityDec = (watermarkOpacityPercent.value / 100).toFixed(2)
  localStorage.setItem('wm_watermark_opacity', opacityDec)
  localStorage.setItem('wm_watermark_custom_text', watermarkCustomText.value)

  // 派发全局事件通知全站水印图层实时更新
  window.dispatchEvent(new CustomEvent('wm-watermark-update'))
  ElMessage.success(`水印设置已保存：${watermarkEnabled.value ? `显隐度 ${watermarkOpacityPercent.value}%` : '已隐藏'}`)
}

// 预览图层样式计算
const previewWatermarkStyle = computed(() => {
  const op = watermarkOpacityPercent.value / 100
  const custom = watermarkCustomText.value ? ` · ${watermarkCustomText.value}` : ''
  const text = `张三 (管理员)_0001${custom}`

  const canvas = document.createElement('canvas')
  canvas.width = 240
  canvas.height = 130
  const ctx = canvas.getContext('2d')
  if (ctx) {
    ctx.rotate((-20 * Math.PI) / 180)
    ctx.font = '13px "PingFang SC", "Microsoft YaHei", sans-serif'
    ctx.fillStyle = `rgba(160, 160, 160, ${op})`
    ctx.textAlign = 'left'
    ctx.textBaseline = 'middle'
    ctx.fillText(text, 20, 65)
  }
  const url = canvas.toDataURL('image/png')
  return {
    backgroundImage: `url('${url}')`,
    backgroundRepeat: 'repeat'
  }
})

onMounted(() => {
  const savedColor = localStorage.getItem('primaryColor')
  if (savedColor) {
    primaryColor.value = savedColor
    document.documentElement.style.setProperty('--el-color-primary', savedColor)
  }

  const enabledStr = localStorage.getItem('wm_watermark_enabled')
  if (enabledStr !== null) {
    watermarkEnabled.value = enabledStr === 'true'
  }

  const opacityStr = localStorage.getItem('wm_watermark_opacity')
  if (opacityStr) {
    watermarkOpacityPercent.value = Math.round(parseFloat(opacityStr) * 100)
  }

  const customText = localStorage.getItem('wm_watermark_custom_text')
  if (customText) {
    watermarkCustomText.value = customText
  }
})
</script>

<style scoped>
.settings-container {
  width: 100%;
  max-width: 1080px;
  margin: 0 auto;
}

.settings-header {
  margin-bottom: 20px;
}

.page-title {
  font-size: 20px;
  font-weight: 700;
  color: #1e293b;
  margin: 0 0 6px 0;
}

.page-subtitle {
  font-size: 13px;
  color: #64748b;
}

.settings-grid {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.settings-card {
  border-radius: 8px;
  border: 1px solid #e2e8f0;
}

.card-header-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 15px;
  font-weight: 700;
  color: #1e293b;
}

.color-picker-row {
  display: flex;
  align-items: center;
  gap: 20px;
  flex-wrap: wrap;
}

.color-presets {
  display: flex;
  align-items: center;
  gap: 10px;
}

.color-pill {
  width: 24px;
  height: 24px;
  border-radius: 50%;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: transform 0.2s, box-shadow 0.2s;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
}

.color-pill:hover {
  transform: scale(1.15);
}

.check-mark {
  color: #ffffff;
  font-size: 13px;
  font-weight: bold;
}

.slider-wrapper {
  display: flex;
  align-items: center;
  gap: 16px;
  max-width: 440px;
  width: 100%;
}

.slider-wrapper :deep(.el-slider) {
  flex: 1;
}

.opacity-value-tag {
  font-weight: 700;
  font-family: monospace;
  font-size: 14px;
  color: #2563eb;
  min-width: 40px;
}

.opacity-presets {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  margin-top: 10px;
}

.preset-title {
  font-size: 12px;
  color: #94a3b8;
}

.preview-stage-box {
  position: relative;
  width: 100%;
  max-width: 580px;
  min-height: 160px;
  border: 1px dashed #cbd5e1;
  border-radius: 8px;
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 16px;
}

.mock-watermark-overlay {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
  z-index: 1;
}

.stage-content {
  position: relative;
  z-index: 2;
  width: 100%;
}

.stage-card {
  background: #ffffff;
  border: 1px solid #e2e8f0;
  border-radius: 6px;
  padding: 14px 18px;
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.05);
}

.stage-title {
  font-size: 14px;
  font-weight: 700;
  color: #0f172a;
  margin-bottom: 6px;
}

.stage-desc {
  font-size: 12px;
  color: #475569;
  line-height: 1.5;
  margin-bottom: 10px;
}

.stage-footer {
  display: flex;
  justify-content: flex-end;
}
</style>
