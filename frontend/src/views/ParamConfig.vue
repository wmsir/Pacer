<template>
  <div class="param-config-container page-container">
    <!-- 顶部操作栏 -->
    <div class="header-action">
      <div class="title-area">
        <h2 class="page-title">
          <span>⚙️ 全局参数查看 (Global System Variables)</span>
        </h2>
        <div class="page-subtitle">在线检索和查看底层数据库实例的全局运行变量与系统参数（连接数、缓冲区、超时时间、日志设置等）</div>
      </div>
      <div class="action-area">
        <el-select
          v-model="selectedInstanceId"
          placeholder="请选择目标数据库实例"
          style="width: 280px; margin-right: 12px;"
          @change="fetchVariables"
        >
          <el-option
            v-for="item in instanceList"
            :key="item.id"
            :label="`${item.name} (${item.dbType || 'mysql'})`"
            :value="item.id"
          />
        </el-select>

        <el-input
          v-model="searchKeyword"
          placeholder="搜索参数名 (如 max_connections)..."
          clearable
          style="width: 260px; margin-right: 12px;"
          :prefix-icon="Search"
          @clear="fetchVariables"
          @keyup.enter="fetchVariables"
        />

        <el-button :icon="Refresh" :loading="loading" @click="fetchVariables">刷新</el-button>
      </div>
    </div>

    <!-- 参数分类过滤标签 -->
    <div class="category-tabs" v-if="selectedInstanceId">
      <el-radio-group v-model="selectedCategory" size="default">
        <el-radio-button value="ALL">全部参数 ({{ variables.length }})</el-radio-button>
        <el-radio-button value="连接与超时">连接与超时</el-radio-button>
        <el-radio-button value="缓冲与内存">缓冲与内存</el-radio-button>
        <el-radio-button value="InnoDB 引擎">InnoDB 引擎</el-radio-button>
        <el-radio-button value="日志与复制">日志与复制</el-radio-button>
        <el-radio-button value="字符集与排序">字符集与排序</el-radio-button>
      </el-radio-group>
    </div>

    <!-- 未选择实例时的空状态提示 -->
    <el-empty
      v-if="!selectedInstanceId"
      description="请在右上角下拉框中选择需要查看的数据库实例"
      style="padding: 60px 0;"
    />

    <!-- 参数列表表格 -->
    <div class="table-wrapper" v-else>
      <el-table :data="pagedVariables" border stripe style="width: 100%" v-loading="loading">
        <el-table-column type="index" label="序号" width="65" align="center" />

        <el-table-column prop="name" label="参数名称 (Variable Name)" min-width="260" show-overflow-tooltip>
          <template #default="scope">
            <span style="font-family: monospace; font-weight: 600; color: #2563eb;">{{ scope.row.name }}</span>
          </template>
        </el-table-column>

        <el-table-column prop="value" label="当前运行值 (Value)" min-width="240" show-overflow-tooltip>
          <template #default="scope">
            <span style="font-family: monospace; font-weight: 500; color: #1e293b;">{{ scope.row.value }}</span>
          </template>
        </el-table-column>

        <el-table-column prop="category" label="所属分类" width="140" align="center">
          <template #default="scope">
            <el-tag size="small" type="info" effect="plain">{{ scope.row.category }}</el-tag>
          </template>
        </el-table-column>

        <el-table-column prop="description" label="参数中文说明与调优建议" min-width="280" show-overflow-tooltip>
          <template #default="scope">
            <span style="color: #475569; font-size: 13px;">{{ scope.row.description }}</span>
          </template>
        </el-table-column>
      </el-table>

      <!-- 分页控制栏 -->
      <div class="pagination-bar" style="display: flex; justify-content: flex-end; margin-top: 16px;">
        <el-pagination
          v-model:current-page="currentPage"
          v-model:page-size="pageSize"
          :page-sizes="[10, 20, 50, 100]"
          :total="filteredVariables.length"
          layout="total, sizes, prev, pager, next, jumper"
          background
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { ElMessage } from 'element-plus'
import { Refresh, Search } from '@element-plus/icons-vue'
import request from '../utils/request'

interface VariableItem {
  name: string
  value: string
  category: string
  description: string
}

const loading = ref(false)
const instanceList = ref<any[]>([])
const selectedInstanceId = ref<number | null>(null)
const searchKeyword = ref('')
const selectedCategory = ref('ALL')
const variables = ref<VariableItem[]>([])

const currentPage = ref(1)
const pageSize = ref(20)

// 获取实例列表
const fetchInstances = async () => {
  try {
    const res: any = await request.get('/v1/instance/list')
    instanceList.value = Array.isArray(res.data) ? res.data : []
    if (instanceList.value.length > 0) {
      selectedInstanceId.value = instanceList.value[0].id
      fetchVariables()
    }
  } catch (error) {
    ElMessage.error('获取实例列表失败')
  }
}

// 客户端根据分类与搜索关键词二次过滤
const filteredVariables = computed(() => {
  let list = variables.value

  if (selectedCategory.value !== 'ALL') {
    list = list.filter(v => v.category === selectedCategory.value)
  }

  if (searchKeyword.value.trim()) {
    const kw = searchKeyword.value.trim().toLowerCase()
    list = list.filter(v =>
      (v.name && v.name.toLowerCase().includes(kw)) ||
      (v.description && v.description.toLowerCase().includes(kw)) ||
      (v.value && v.value.toLowerCase().includes(kw))
    )
  }

  return list
})

// 分页切片数据
const pagedVariables = computed(() => {
  const start = (currentPage.value - 1) * pageSize.value
  return filteredVariables.value.slice(start, start + pageSize.value)
})

watch([selectedCategory, searchKeyword], () => {
  currentPage.value = 1
})

// 查询底层全局变量
const fetchVariables = async () => {
  if (!selectedInstanceId.value) return
  loading.value = true
  try {
    const res: any = await request.get(`/v1/instance/${selectedInstanceId.value}/variables`, {
      params: { keyword: searchKeyword.value.trim() }
    })
    variables.value = Array.isArray(res.data) ? res.data : []
  } catch (error) {
    ElMessage.error('获取全局参数列表失败')
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  fetchInstances()
})
</script>

<style scoped>
.header-action {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
  flex-wrap: wrap;
  gap: 12px;
}

.page-subtitle {
  font-size: 13px;
  color: #64748b;
  margin-top: 4px;
}

.action-area {
  display: flex;
  align-items: center;
}

.category-tabs {
  margin-bottom: 16px;
}

.table-wrapper {
  background: #ffffff;
  border-radius: 8px;
  overflow-x: auto;
}
</style>
