/**
 * 整个前端应用的入口文件
 *
 * 负责实例化 Vue 应用，加载路由、UI 组件库（ElementPlus）并注册全局指令（如水印防盗）。
 */
import { createApp } from 'vue'
import './style.css'
import App from './App.vue'
import router from './router'
import { createPinia } from 'pinia'
import ElementPlus from 'element-plus'
import 'element-plus/dist/index.css'
import zhCn from 'element-plus/es/locale/lang/zh-cn'
import { watermark } from './directives/watermark'
import i18n from './i18n'

const app = createApp(App)

// 挂载 Pinia 状态管理
app.use(createPinia())

// 挂载路由模块
app.use(router)

// 挂载 Element Plus 组件库 (配置全量中文语言包)
app.use(ElementPlus, {
  locale: zhCn
})

// 挂载 i18n 国际化
app.use(i18n)

// 注册全局自定义防盗水印指令
app.directive('watermark', watermark)

// 启动并挂载到 #app
app.mount('#app')
