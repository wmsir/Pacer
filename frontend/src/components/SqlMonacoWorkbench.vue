<template>
  <div class="sql-monaco-workbench" :class="{ 'is-fullscreen': isFullscreen, 'is-dark': isDarkTheme }">
    <!-- 工具栏头部 -->
    <div class="workbench-toolbar">
      <div class="toolbar-left">
        <el-radio-group v-model="editorThemeMode" size="small" @change="handleThemeChange">
          <el-radio-button value="auto">🌓 跟随系统</el-radio-button>
          <el-radio-button value="light">☀️ 浅色模式</el-radio-button>
          <el-radio-button value="dark">🌙 深色模式</el-radio-button>
        </el-radio-group>

        <el-divider direction="vertical" />

        <el-button size="small" type="primary" plain :icon="MagicStick" @click="handleFormatSql">
          规范化 SQL (大写/分号)
        </el-button>
        <el-button size="small" type="success" plain :icon="DocumentAdd" @click="handleInsertSafetyAnnotation">
          插入安全注释 (-- 1)
        </el-button>
        <el-button size="small" type="warning" plain :icon="View" @click="diffDialogVisible = true">
          并排对比 (原 SQL / 改后 SQL)
        </el-button>
      </div>

      <div class="toolbar-right">
        <!-- 风险计数徽标 -->
        <div class="risk-badge-group">
          <el-tag v-if="riskSummary.high > 0" size="small" type="danger" effect="dark" class="risk-pill">
            🚨 高危 {{ riskSummary.high }}
          </el-tag>
          <el-tag v-if="riskSummary.medium > 0" size="small" type="warning" effect="dark" class="risk-pill">
            ⚠️ 中危 {{ riskSummary.medium }}
          </el-tag>
          <el-tag v-if="riskSummary.low > 0" size="small" type="info" effect="plain" class="risk-pill">
            💡 建议 {{ riskSummary.low }}
          </el-tag>
          <el-tag v-if="riskSummary.total === 0 && modelValue.trim()" size="small" type="success" effect="light" class="risk-pill">
            ✅ 语法审核安全
          </el-tag>
        </div>

        <el-divider direction="vertical" />

        <el-button
          size="small"
          :type="isFullscreen ? 'warning' : 'default'"
          :icon="isFullscreen ? ScaleToOriginal : FullScreen"
          @click="toggleFullscreen"
        >
          {{ isFullscreen ? '还原 (Esc)' : '全屏' }}
        </el-button>
      </div>
    </div>

    <!-- 编辑器与右侧诊断主体分栏 -->
    <div class="workbench-main">
      <!-- 左侧 Monaco Editor 容器 -->
      <div class="editor-col">
        <div ref="editorContainerRef" class="monaco-host-container"></div>
      </div>

      <!-- 右侧 风险诊断与修改建议面板 -->
      <div class="diagnostics-col" :class="{ 'collapsed': isPanelCollapsed }">
        <div class="panel-header">
          <div class="panel-title-wrap">
            <span class="panel-title">🛡️ 风险诊断与修改建议</span>
            <span class="panel-count" v-if="riskItems.length > 0">({{ riskItems.length }})</span>
          </div>
          <div class="panel-actions">
            <el-button size="small" link @click="isPanelCollapsed = !isPanelCollapsed">
              {{ isPanelCollapsed ? '展开 ➔' : '收起 ✕' }}
            </el-button>
          </div>
        </div>

        <div v-if="!isPanelCollapsed" class="panel-body">
          <!-- 风险级别过滤 -->
          <div class="filter-row" v-if="riskItems.length > 0">
            <el-radio-group v-model="filterRiskLevel" size="small">
              <el-radio-button value="ALL">全部 ({{ riskItems.length }})</el-radio-button>
              <el-radio-button value="HIGH" v-if="riskSummary.high > 0">高危</el-radio-button>
              <el-radio-button value="MEDIUM" v-if="riskSummary.medium > 0">中危</el-radio-button>
              <el-radio-button value="LOW" v-if="riskSummary.low > 0">建议</el-radio-button>
            </el-radio-group>
          </div>

          <!-- 风险列表 -->
          <div v-if="filteredRiskItems.length > 0" class="risk-item-list">
            <div
              v-for="(item, idx) in filteredRiskItems"
              :key="idx"
              class="risk-card"
              :class="`risk-card-${item.level.toLowerCase()}`"
              @click="jumpToLine(item.line, item.col)"
            >
              <div class="risk-card-top">
                <div class="risk-card-tag-wrap">
                  <el-tag
                    size="small"
                    :type="item.level === 'HIGH' ? 'danger' : (item.level === 'MEDIUM' ? 'warning' : 'info')"
                    effect="dark"
                  >
                    {{ item.levelText }}
                  </el-tag>
                  <span class="risk-line-num">第 <b>{{ item.line }}</b> 行</span>
                </div>
                <el-button size="small" type="primary" link @click.stop="jumpToLine(item.line, item.col)">
                  定位 ➔
                </el-button>
              </div>

              <div class="risk-code-snippet">
                <code>{{ item.snippet }}</code>
              </div>

              <div class="risk-reason-box">
                <span class="lbl-reason">【原因】</span>
                <span class="text-reason">{{ item.reason }}</span>
              </div>

              <div class="risk-suggestion-box">
                <span class="lbl-sug">【建议】</span>
                <span class="text-sug">{{ item.suggestion }}</span>
              </div>

              <div class="risk-card-actions" v-if="item.autoFix">
                <el-button size="small" type="success" plain @click.stop="applyAutoFix(item)">
                  ✨ 一键采纳建议
                </el-button>
              </div>
            </div>
          </div>

          <!-- 空状态 -->
          <div v-else class="empty-risk-state">
            <el-empty
              v-if="!modelValue.trim()"
              description="请输入 SQL 语句，系统将自动进行静态风险诊断与规范检查"
              :image-size="60"
            />
            <div v-else class="safe-congratulations">
              <div class="safe-icon">🎉</div>
              <div class="safe-title">未发现高危语法隐患</div>
              <div class="safe-desc">当前编写的 SQL 脚本符合防全表变更与语法合规规范，可正常发起预执行校验！</div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 底部状态栏 -->
    <div class="workbench-statusbar">
      <div class="sb-left">
        <span class="sb-item">行数: <b>{{ editorStats.lineCount }}</b></span>
        <span class="sb-item">字符数: <b>{{ editorStats.charCount }}</b></span>
        <span class="sb-item">语句数: <b>{{ editorStats.stmtCount }}</b> (DML: {{ editorStats.dmlCount }})</span>
      </div>
      <div class="sb-right">
        <span class="sb-tip">💡 在编辑器中风险行已用红/黄/蓝波浪线标识，鼠标悬浮可查看详细审核规则</span>
      </div>
    </div>

    <!-- 并排对比 Dialog: 原 SQL vs 改后/规范化 SQL -->
    <el-dialog
      v-model="diffDialogVisible"
      title="原 SQL / 改后规范化 SQL 并排对比审计"
      width="86%"
      destroy-on-close
      append-to-body
      class="sql-diff-dialog"
    >
      <div class="diff-dialog-header-hint">
        <span>已自动对原始 SQL 进行了安全加固（关键字大写规范化、分号补齐、DML 预期行数标记与危险语句校验）。请比对两栏差异：</span>
      </div>

      <div class="side-by-side-diff-grid">
        <!-- 左侧: 原 SQL -->
        <div class="diff-side-col diff-original">
          <div class="side-title-bar original-bar">
            <span>📄 原提交 SQL (Original)</span>
            <el-tag size="small" type="info">原始输入</el-tag>
          </div>
          <div class="code-view-box">
            <div class="code-line-row" v-for="(line, idx) in originalLines" :key="idx">
              <span class="line-no">{{ idx + 1 }}</span>
              <pre class="line-content" :class="{ 'is-diff-removed': isLineDifferent(idx) }">{{ line || ' ' }}</pre>
            </div>
          </div>
        </div>

        <!-- 右侧: 改后规范化 SQL -->
        <div class="diff-side-col diff-modified">
          <div class="side-title-bar modified-bar">
            <span>✨ 改后/规范化 SQL (Modified & Safe)</span>
            <el-tag size="small" type="success">安全加固建议</el-tag>
          </div>
          <div class="code-view-box">
            <div class="code-line-row" v-for="(line, idx) in modifiedLines" :key="idx">
              <span class="line-no">{{ idx + 1 }}</span>
              <pre class="line-content" :class="{ 'is-diff-added': isLineDifferent(idx) }">{{ line || ' ' }}</pre>
            </div>
          </div>
        </div>
      </div>

      <template #footer>
        <div class="diff-footer">
          <el-button @click="diffDialogVisible = false">取消比对</el-button>
          <el-button type="primary" :icon="Check" @click="applyModifiedSql">
            采纳并替换为改后规范化 SQL
          </el-button>
        </div>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onBeforeUnmount, nextTick } from 'vue'
import * as monaco from 'monaco-editor'
import {
  MagicStick,
  DocumentAdd,
  View,
  FullScreen,
  ScaleToOriginal,
  Check
} from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'

const props = defineProps<{
  modelValue: string
  disabled?: boolean
  height?: string
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void
  (e: 'stats-change', stats: any): void
}>()

const editorContainerRef = ref<HTMLElement | null>(null)
let editorInstance: monaco.editor.IStandaloneCodeEditor | null = null

const isFullscreen = ref(false)
const isPanelCollapsed = ref(false)
const filterRiskLevel = ref('ALL')
const diffDialogVisible = ref(false)
const editorThemeMode = ref<'auto' | 'light' | 'dark'>('auto')
const isDarkTheme = ref(false)

// 风险项定义
export interface RiskItem {
  line: number
  col: number
  endCol?: number
  level: 'HIGH' | 'MEDIUM' | 'LOW'
  levelText: string
  snippet: string
  reason: string
  suggestion: string
  severity: monaco.MarkerSeverity
  autoFix?: () => void
}

const riskItems = ref<RiskItem[]>([])

const riskSummary = computed(() => {
  const high = riskItems.value.filter(i => i.level === 'HIGH').length
  const medium = riskItems.value.filter(i => i.level === 'MEDIUM').length
  const low = riskItems.value.filter(i => i.level === 'LOW').length
  return { high, medium, low, total: riskItems.value.length }
})

const filteredRiskItems = computed(() => {
  if (filterRiskLevel.value === 'ALL') return riskItems.value
  return riskItems.value.filter(i => i.level === filterRiskLevel.value)
})

// 统计信息
const editorStats = computed(() => {
  const text = props.modelValue || ''
  const lines = text.split('\n')
  const lineCount = lines.length
  const charCount = text.length

  const statements = text
    .split(';')
    .map(s => s.trim())
    .filter(s => s.length > 0)

  let dmlCount = 0
  statements.forEach(stmt => {
    const s = stmt.toUpperCase()
    if (s.startsWith('INSERT') || s.startsWith('UPDATE') || s.startsWith('DELETE')) {
      dmlCount++
    }
  })

  return {
    lineCount,
    charCount,
    stmtCount: statements.length,
    dmlCount,
    riskCount: riskItems.value.length
  }
})

// 比对行
const originalLines = computed(() => (props.modelValue || '').split('\n'))
const modifiedText = computed(() => generateStandardizedSql(props.modelValue || ''))
const modifiedLines = computed(() => modifiedText.value.split('\n'))

const isLineDifferent = (idx: number) => {
  const orig = originalLines.value[idx] ?? ''
  const mod = modifiedLines.value[idx] ?? ''
  return orig.trim() !== mod.trim()
}

// 主题定义与切换
const initMonacoThemes = () => {
  monaco.editor.defineTheme('pacer-light', {
    base: 'vs',
    inherit: true,
    rules: [
      { token: 'keyword', foreground: '0284c7', fontStyle: 'bold' },
      { token: 'comment', foreground: '64748b', fontStyle: 'italic' },
      { token: 'string', foreground: '059669' },
      { token: 'number', foreground: 'd97706' }
    ],
    colors: {
      'editor.background': '#ffffff',
      'editor.lineHighlightBackground': '#f1f5f9',
      'editorLineNumber.foreground': '#94a3b8',
      'editorLineNumber.activeForeground': '#0284c7',
      'editorGutter.background': '#f8fafc'
    }
  })

  monaco.editor.defineTheme('pacer-dark', {
    base: 'vs-dark',
    inherit: true,
    rules: [
      { token: 'keyword', foreground: '38bdf8', fontStyle: 'bold' },
      { token: 'comment', foreground: '94a3b8', fontStyle: 'italic' },
      { token: 'string', foreground: '34d399' },
      { token: 'number', foreground: 'fbbf24' }
    ],
    colors: {
      'editor.background': '#0f172a',
      'editor.lineHighlightBackground': '#1e293b',
      'editorLineNumber.foreground': '#475569',
      'editorLineNumber.activeForeground': '#38bdf8',
      'editorGutter.background': '#0b0f19'
    }
  })
}

const checkSystemDark = () => {
  const isHtmlDark = document.documentElement.classList.contains('dark')
  const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches
  return isHtmlDark || prefersDark
}

const applyTheme = () => {
  let isDark = false
  if (editorThemeMode.value === 'auto') {
    isDark = checkSystemDark()
  } else {
    isDark = editorThemeMode.value === 'dark'
  }
  isDarkTheme.value = isDark
  monaco.editor.setTheme(isDark ? 'pacer-dark' : 'pacer-light')
}

const handleThemeChange = () => {
  applyTheme()
}

// 静态 SQL 风险审查引擎 (根据规则标定高/中/低三级风险与行号)
const analyzeSqlRisks = (sql: string) => {
  const items: RiskItem[] = []
  if (!sql.trim()) {
    riskItems.value = []
    if (editorInstance) {
      const model = editorInstance.getModel()
      if (model) monaco.editor.setModelMarkers(model, 'pacer-risk', [])
    }
    return
  }

  const lines = sql.split('\n')

  lines.forEach((lineText, idx) => {
    const lineNum = idx + 1
    const trimmed = lineText.trim()
    const upper = trimmed.toUpperCase()

    // 忽略纯注释或空行
    if (!trimmed || trimmed.startsWith('--') || trimmed.startsWith('#') || trimmed.startsWith('/*')) {
      return
    }

    // 1. 高危检查 (HIGH - 红色 Error)
    if (upper.includes('DROP TABLE') || upper.includes('DROP DATABASE')) {
      items.push({
        line: lineNum,
        col: 1,
        endCol: lineText.length + 1,
        level: 'HIGH',
        levelText: '高风险 (拦截)',
        snippet: trimmed.slice(0, 50),
        reason: '检测到破坏性 DROP 指令，将彻底销毁物理表/库，不可恢复！',
        suggestion: '严禁在线执行 DROP。建议将物理表更名为 _bak 备份表，或提交 DBA 离线归档流程。',
        severity: monaco.MarkerSeverity.Error
      })
    } else if (upper.includes('TRUNCATE TABLE') || upper.startsWith('TRUNCATE ')) {
      items.push({
        line: lineNum,
        col: 1,
        endCol: lineText.length + 1,
        level: 'HIGH',
        levelText: '高风险 (拦截)',
        snippet: trimmed.slice(0, 50),
        reason: '检测到 TRUNCATE 指令，将重置数据页并绕过行级事务日志，数据无法恢复！',
        suggestion: '如需清空数据，请使用带 WHERE 条件与 LIMIT 的分批 DELETE 语句。',
        severity: monaco.MarkerSeverity.Error
      })
    } else if (upper.startsWith('DELETE ') && !upper.includes('WHERE')) {
      items.push({
        line: lineNum,
        col: 1,
        endCol: lineText.length + 1,
        level: 'HIGH',
        levelText: '高风险 (无条件删除)',
        snippet: trimmed.slice(0, 50),
        reason: 'DELETE 语句缺少 WHERE 过滤子句，执行将清空整张表全部行数据！',
        suggestion: '必须补充明确的 WHERE 条件主键范围，并在上方添加 -- 1 安全行数注释。',
        severity: monaco.MarkerSeverity.Error
      })
    } else if (upper.startsWith('UPDATE ') && upper.includes('SET') && !upper.includes('WHERE')) {
      items.push({
        line: lineNum,
        col: 1,
        endCol: lineText.length + 1,
        level: 'HIGH',
        levelText: '高风险 (无条件更新)',
        snippet: trimmed.slice(0, 50),
        reason: 'UPDATE 语句缺少 WHERE 过滤子句，执行将全表字段覆盖更新！',
        suggestion: '必须严格指定 WHERE 匹配字段；单条更新务必加上主键 ID。',
        severity: monaco.MarkerSeverity.Error
      })
    } else if (upper.includes('ALTER TABLE') && upper.includes('DROP COLUMN')) {
      items.push({
        line: lineNum,
        col: 1,
        endCol: lineText.length + 1,
        level: 'HIGH',
        levelText: '高风险 (删除字段)',
        snippet: trimmed.slice(0, 50),
        reason: '线上删除表字段容易引发存量应用读取报 Unknown column 错误！',
        suggestion: '需分步执行：先将应用层涉及字段的代码下线，并确认无依赖后再执行 DDL。',
        severity: monaco.MarkerSeverity.Error
      })
    }

    // 2. 中危检查 (MEDIUM - 黄色 Warning)
    else if (upper.includes('SELECT *') || upper.startsWith('SELECT *')) {
      items.push({
        line: lineNum,
        col: lineText.indexOf('*') + 1 || 1,
        endCol: lineText.length + 1,
        level: 'MEDIUM',
        levelText: '中风险 (全字段查询)',
        snippet: trimmed.slice(0, 50),
        reason: '使用 SELECT * 会加载大字段与无用列，浪费带宽并导致无法使用覆盖索引。',
        suggestion: '列出显式所需的字段名称清单 (如 id, name, status)。',
        severity: monaco.MarkerSeverity.Warning
      })
    } else if (upper.includes('ALTER TABLE') && upper.includes('ADD') && !upper.includes('ALGORITHM')) {
      items.push({
        line: lineNum,
        col: 1,
        endCol: lineText.length + 1,
        level: 'MEDIUM',
        levelText: '中风险 (大表锁表隐患)',
        snippet: trimmed.slice(0, 50),
        reason: '大表加字段未显式指定 ALGORITHM=INPLACE 可能触发 Copy Table 导致锁表。',
        suggestion: '建议在末尾指定 ALGORITHM=INPLACE, LOCK=NONE，或使用 gh-ost / pt-osc 变更。',
        severity: monaco.MarkerSeverity.Warning
      })
    } else if ((upper.startsWith('UPDATE') || upper.startsWith('DELETE')) && !upper.includes('--')) {
      // 检查前一行或自身是否包含 -- 预期影响行数
      const prevLine = idx > 0 ? lines[idx - 1].trim() : ''
      if (!prevLine.startsWith('--')) {
        items.push({
          line: lineNum,
          col: 1,
          endCol: lineText.length + 1,
          level: 'MEDIUM',
          levelText: '中风险 (未指定影响行)',
          snippet: trimmed.slice(0, 50),
          reason: 'DML 语句缺少预期影响行数声明注释，无法激活 PacerSQL 事务级精准防爆保护。',
          suggestion: '请在语句上方增加 "-- 预期行数" (例如: -- 1) 注释。',
          severity: monaco.MarkerSeverity.Warning
        })
      }
    }

    // 3. 低危建议 (LOW - 蓝色/绿色 Info)
    else if (/(select|insert|update|delete|from|where|into|values)\s/i.test(trimmed) && /[a-z]/.test(trimmed.slice(0, 6))) {
      items.push({
        line: lineNum,
        col: 1,
        endCol: 10,
        level: 'LOW',
        levelText: '低风险 (书写规范)',
        snippet: trimmed.slice(0, 40),
        reason: 'SQL 关键字使用了小写，不符合企业级标准 SQL 规范。',
        suggestion: '建议将 SELECT, INSERT, UPDATE, DELETE 等关键字统一转为大写。',
        severity: monaco.MarkerSeverity.Info,
        autoFix: () => handleFormatSql()
      })
    }
  })

  riskItems.value = items

  // 在 Monaco Editor 中标定风险行波浪线与 Gutter
  if (editorInstance) {
    const model = editorInstance.getModel()
    if (model) {
      const markers: monaco.editor.IMarkerData[] = items.map(item => ({
        startLineNumber: item.line,
        startColumn: item.col,
        endLineNumber: item.line,
        endColumn: item.endCol || (model.getLineMaxColumn(item.line)),
        message: `[${item.levelText}] ${item.reason} ➔ 建议：${item.suggestion}`,
        severity: item.severity
      }))
      monaco.editor.setModelMarkers(model, 'pacer-risk', markers)
    }
  }
}

// 跳转到行
const jumpToLine = (line: number, col: number) => {
  if (!editorInstance) return
  editorInstance.revealLineInCenter(line)
  editorInstance.setPosition({ lineNumber: line, column: col || 1 })
  editorInstance.focus()
}

// 一键采纳建议
const applyAutoFix = (item: RiskItem) => {
  if (item.autoFix) {
    item.autoFix()
  } else {
    ElMessage.info('已为您聚焦定位该行，请根据修改建议修正')
    jumpToLine(item.line, item.col)
  }
}

// 插入安全注释
const handleInsertSafetyAnnotation = () => {
  const current = props.modelValue || ''
  if (!current.trim()) {
    const sample = `-- 1 (声明预期影响 1 行)\nUPDATE users SET status = 'ACTIVE' WHERE id = 1001;`
    emit('update:modelValue', sample)
    if (editorInstance) editorInstance.setValue(sample)
    ElMessage.success('已插入安全注释范例')
    return
  }

  // 为开头的 DML 补充 -- 1
  const lines = current.split('\n')
  const newLines: string[] = []
  lines.forEach(l => {
    const up = l.trim().toUpperCase()
    if ((up.startsWith('UPDATE') || up.startsWith('DELETE') || up.startsWith('INSERT')) && !newLines[newLines.length - 1]?.trim().startsWith('--')) {
      newLines.push('-- 1')
    }
    newLines.push(l)
  })
  const res = newLines.join('\n')
  emit('update:modelValue', res)
  if (editorInstance) editorInstance.setValue(res)
  ElMessage.success('已为 DML 语句自动补齐安全预期行数注释 (-- 1)')
}

// 规范化 SQL (大写 + 补齐分号)
const generateStandardizedSql = (raw: string) => {
  if (!raw.trim()) return ''
  const keywords = [
    'select', 'insert', 'update', 'delete', 'from', 'where', 'into', 'values',
    'set', 'and', 'or', 'join', 'left', 'right', 'inner', 'on', 'group', 'by',
    'order', 'asc', 'desc', 'limit', 'offset', 'having', 'as', 'create', 'table',
    'alter', 'drop', 'add', 'column', 'primary', 'key', 'not', 'null', 'default'
  ]

  let lines = raw.split('\n')
  return lines.map(line => {
    let modified = line
    // 替换独立关键字为大写
    keywords.forEach(kw => {
      const reg = new RegExp(`\\b${kw}\\b`, 'gi')
      modified = modified.replace(reg, kw.toUpperCase())
    })
    // 如果是 SQL 语句行且无分号结尾
    const tr = modified.trim()
    if (tr && !tr.startsWith('--') && !tr.startsWith('#') && !tr.startsWith('/*') && !tr.endsWith(';')) {
      modified = modified + ';'
    }
    return modified
  }).join('\n')
}

const handleFormatSql = () => {
  const formatted = generateStandardizedSql(props.modelValue || '')
  if (formatted) {
    emit('update:modelValue', formatted)
    if (editorInstance) editorInstance.setValue(formatted)
    ElMessage.success('已自动规范化 SQL：关键字转为标准大写并补齐分号')
  }
}

// 采纳比对结果
const applyModifiedSql = () => {
  const mod = modifiedText.value
  emit('update:modelValue', mod)
  if (editorInstance) editorInstance.setValue(mod)
  diffDialogVisible.value = false
  ElMessage.success('已采纳改后规范化 SQL 并应用到表单！')
}

// 全屏切换
const toggleFullscreen = () => {
  isFullscreen.value = !isFullscreen.value
  nextTick(() => {
    editorInstance?.layout()
  })
}

// 键盘快捷键监听
const handleKeyDown = (e: KeyboardEvent) => {
  if (e.key === 'Escape' && isFullscreen.value) {
    toggleFullscreen()
  }
}

onMounted(() => {
  window.addEventListener('keydown', handleKeyDown)
  initMonacoThemes()

  nextTick(() => {
    if (!editorContainerRef.value) return

    editorInstance = monaco.editor.create(editorContainerRef.value, {
      value: props.modelValue || '',
      language: 'sql',
      theme: 'pacer-light',
      automaticLayout: true,
      fontSize: 13.5,
      lineNumbers: 'on',
      fontFamily: "'JetBrains Mono', 'Fira Code', 'Consolas', 'Courier New', monospace",
      minimap: { enabled: false },
      scrollBeyondLastLine: false,
      tabSize: 2,
      wordWrap: 'on',
      folding: true,
      lineDecorationsWidth: 10,
      renderLineHighlight: 'all'
    })

    applyTheme()

    editorInstance.onDidChangeModelContent(() => {
      const val = editorInstance?.getValue() || ''
      emit('update:modelValue', val)
      analyzeSqlRisks(val)
      emit('stats-change', editorStats.value)
    })

    analyzeSqlRisks(props.modelValue || '')
    emit('stats-change', editorStats.value)
  })
})

watch(() => props.modelValue, (newVal) => {
  if (editorInstance && editorInstance.getValue() !== newVal) {
    editorInstance.setValue(newVal || '')
    analyzeSqlRisks(newVal || '')
  }
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', handleKeyDown)
  if (editorInstance) {
    editorInstance.dispose()
    editorInstance = null
  }
})
</script>

<style scoped>
.sql-monaco-workbench {
  display: flex;
  flex-direction: column;
  border: 1px solid #e2e8f0;
  border-radius: 8px;
  overflow: hidden;
  background: #ffffff;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.05);
  transition: all 0.2s ease;
}

.sql-monaco-workbench.is-dark {
  border-color: #334155;
  background: #0f172a;
}

.sql-monaco-workbench.is-fullscreen {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  z-index: 2999;
  border-radius: 0;
}

/* 顶部工具栏 */
.workbench-toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 12px;
  background: #f8fafc;
  border-bottom: 1px solid #e2e8f0;
  flex-wrap: wrap;
  gap: 8px;
}

.is-dark .workbench-toolbar {
  background: #1e293b;
  border-bottom-color: #334155;
}

.toolbar-left, .toolbar-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

.risk-badge-group {
  display: flex;
  align-items: center;
  gap: 6px;
}

.risk-pill {
  font-weight: 600;
  font-size: 11px;
}

/* 主体分栏 */
.workbench-main {
  display: flex;
  height: 440px;
  position: relative;
}

.is-fullscreen .workbench-main {
  height: calc(100vh - 84px);
}

.editor-col {
  flex: 1;
  height: 100%;
  position: relative;
  min-width: 0;
}

.monaco-host-container {
  width: 100%;
  height: 100%;
}

/* 右侧风险建议面板 */
.diagnostics-col {
  width: 360px;
  border-left: 1px solid #e2e8f0;
  background: #fdfdfd;
  display: flex;
  flex-direction: column;
  transition: width 0.2s ease;
  overflow: hidden;
}

.is-dark .diagnostics-col {
  border-left-color: #334155;
  background: #111827;
}

.diagnostics-col.collapsed {
  width: 44px;
}

.panel-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 12px;
  background: #f1f5f9;
  border-bottom: 1px solid #e2e8f0;
}

.is-dark .panel-header {
  background: #1e293b;
  border-bottom-color: #334155;
}

.panel-title {
  font-size: 12.5px;
  font-weight: 700;
  color: #1e293b;
}

.is-dark .panel-title {
  color: #e2e8f0;
}

.panel-count {
  font-size: 12px;
  color: #dc2626;
  font-weight: 700;
  margin-left: 4px;
}

.panel-body {
  flex: 1;
  overflow-y: auto;
  padding: 10px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.filter-row {
  margin-bottom: 4px;
}

.risk-item-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.risk-card {
  padding: 10px;
  border-radius: 6px;
  border: 1px solid #e2e8f0;
  background: #ffffff;
  cursor: pointer;
  transition: all 0.15s ease;
}

.is-dark .risk-card {
  background: #1e293b;
  border-color: #334155;
}

.risk-card:hover {
  transform: translateY(-1px);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
}

.risk-card-high {
  border-left: 4px solid #ef4444;
  background: #fff5f5;
}
.is-dark .risk-card-high {
  background: rgba(239, 68, 68, 0.12);
}

.risk-card-medium {
  border-left: 4px solid #f59e0b;
  background: #fffbeb;
}
.is-dark .risk-card-medium {
  background: rgba(245, 158, 11, 0.12);
}

.risk-card-low {
  border-left: 4px solid #0284c7;
  background: #f0f9ff;
}
.is-dark .risk-card-low {
  background: rgba(2, 132, 199, 0.12);
}

.risk-card-top {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 6px;
}

.risk-card-tag-wrap {
  display: flex;
  align-items: center;
  gap: 6px;
}

.risk-line-num {
  font-size: 11.5px;
  color: #475569;
}

.is-dark .risk-line-num {
  color: #94a3b8;
}

.risk-code-snippet {
  background: rgba(0, 0, 0, 0.04);
  padding: 4px 6px;
  border-radius: 4px;
  font-size: 11px;
  font-family: monospace;
  color: #1e293b;
  margin-bottom: 6px;
  word-break: break-all;
}

.is-dark .risk-code-snippet {
  background: rgba(255, 255, 255, 0.06);
  color: #f1f5f9;
}

.risk-reason-box, .risk-suggestion-box {
  font-size: 12px;
  line-height: 1.5;
  margin-top: 4px;
}

.lbl-reason {
  color: #dc2626;
  font-weight: 700;
}

.lbl-sug {
  color: #059669;
  font-weight: 700;
}

.text-reason, .text-sug {
  color: #334155;
}

.is-dark .text-reason, .is-dark .text-sug {
  color: #cbd5e1;
}

.risk-card-actions {
  margin-top: 8px;
  text-align: right;
}

.empty-risk-state {
  padding: 20px 10px;
  text-align: center;
}

.safe-congratulations {
  padding: 24px 12px;
  background: #f0fdf4;
  border: 1px dashed #86efac;
  border-radius: 8px;
  text-align: center;
}

.is-dark .safe-congratulations {
  background: rgba(22, 101, 52, 0.2);
  border-color: #15803d;
}

.safe-icon {
  font-size: 32px;
  margin-bottom: 6px;
}

.safe-title {
  font-size: 14px;
  font-weight: 700;
  color: #166534;
  margin-bottom: 4px;
}

.is-dark .safe-title {
  color: #86efac;
}

.safe-desc {
  font-size: 12px;
  color: #15803d;
  line-height: 1.5;
}

.is-dark .safe-desc {
  color: #bbf7d0;
}

/* 状态栏 */
.workbench-statusbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 6px 12px;
  background: #f8fafc;
  border-top: 1px solid #e2e8f0;
  font-size: 11.5px;
  color: #64748b;
}

.is-dark .workbench-statusbar {
  background: #1e293b;
  border-top-color: #334155;
  color: #94a3b8;
}

.sb-left {
  display: flex;
  gap: 12px;
}

.sb-tip {
  font-size: 11px;
}

/* 并排比对 Grid */
.diff-dialog-header-hint {
  padding: 8px 12px;
  background: #eff6ff;
  border-radius: 6px;
  color: #1e40af;
  font-size: 12.5px;
  margin-bottom: 12px;
}

.side-by-side-diff-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
  border: 1px solid #cbd5e1;
  border-radius: 6px;
  overflow: hidden;
  height: 480px;
}

.diff-side-col {
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: #ffffff;
}

.side-title-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 12px;
  font-size: 13px;
  font-weight: 600;
  border-bottom: 1px solid #cbd5e1;
}

.original-bar {
  background: #f8fafc;
  color: #334155;
}

.modified-bar {
  background: #f0fdf4;
  color: #166534;
}

.code-view-box {
  flex: 1;
  overflow-y: auto;
  font-family: 'JetBrains Mono', Consolas, monospace;
  font-size: 12px;
  padding: 4px 0;
  background: #fafafa;
}

.code-line-row {
  display: flex;
  line-height: 20px;
  min-height: 20px;
}

.line-no {
  width: 40px;
  text-align: right;
  padding-right: 10px;
  color: #94a3b8;
  user-select: none;
  background: #f1f5f9;
  border-right: 1px solid #e2e8f0;
}

.line-content {
  flex: 1;
  margin: 0;
  padding: 0 8px;
  white-space: pre-wrap;
  word-break: break-all;
  font-family: inherit;
}

.is-diff-removed {
  background: #fee2e2;
  color: #991b1b;
}

.is-diff-added {
  background: #dcfce7;
  color: #166534;
  font-weight: 600;
}

.diff-footer {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}
</style>
