# 稳骥 PacerSQL · Web 协同治理工作台 (`frontend`)

基于 **Vue 3.5 + TypeScript + Vite + Element Plus + Monaco Editor + BPMN.js + ECharts** 构建的企业级数据库治理 Web 前端工作台。

## 核心功能模块

- **商业化大盘总览 (`Dashboard.vue`)**：实时展示数据库健康评分、TPS/QPS、连接数、锁等待与工单统计。
- **SQL 工单中心 (`CreateTicket.vue` / `TicketList.vue` / `TicketDetail.vue`)**：支持 SQL 语法高亮、AST 风险预检、Dry Run 预执行校验、多级审批全景追踪与三种执行模式闭环。
- **只读数据查询工作台 (`DataQuery.vue` / `SqlMonacoWorkbench.vue`)**：支持库表元数据树浏览、点击表名即查、Monaco 智能补全、只读拦截与脱敏导出（CSV/JSON）。
- **BPMN 2.0 工作流设计器 (`WorkflowDesigner.vue`)**：内置 4 大标准审批流模板，支持在线可视化编排、BPMN XML 导出与一键部署。
- **多源数据库实例管理 (`InstanceList.vue` / `InstanceEdit.vue`)**：结构化连接配置、SM4 加密落库与秒级实时连通性探测。
- **AI SQL 审查与安全审计 (`AiSqlReview.vue` / `AuditDashboard.vue` / `DataMaskingConfig.vue`)**：集成 AI 优化建议、动态数据脱敏规则与防篡改审计看板。

## 本地开发与构建

```bash
# 安装依赖
npm install

# 启动本地开发服务器 (默认 http://localhost:5173)
npm run dev

# 生产环境类型检查与打包构建 (输出至 dist/)
npm run build
```
