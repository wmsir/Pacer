<script setup lang="ts">
/**
 * 根组件入口
 * 全局挂载了 v-watermark 防止截图泄露，并通过 ElConfigProvider 全局注入中文语言包
 */
import zhCn from 'element-plus/es/locale/lang/zh-cn'
</script>

<template>
  <el-config-provider :locale="zhCn">
    <div v-watermark class="app-root">
      <router-view></router-view>
    </div>
  </el-config-provider>
</template>

<style scoped>
.app-root {
  height: 100%;
  width: 100%;
  overflow: hidden;
  position: relative;
}
</style>
