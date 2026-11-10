# 稳骥 PacerSQL 运维与开发脚本 (`scripts/`)

本目录存放本地开发启停与远端自动化部署脚本：

| 脚本名称 | 运行环境 | 用途说明 |
| :--- | :--- | :--- |
| [`start.bat`](start.bat) | Windows | 本地一键并行启动后端 Spring Boot 服务（`8088` 端口）与前端 Vite 开发服务（`5173` 端口），就绪后自动打开浏览器 |
| [`stop.bat`](stop.bat) | Windows | 本地一键停止后端（`8088` / `8080`）与前端（`5173`）占用的服务进程 |
| [`deploy.sh`](deploy.sh) | Linux / macOS / WSL | 基于环境变量注入的无硬编码远程构建与发布脚本（详见 [`../docs/DEPLOYMENT_GUIDE.md`](../docs/DEPLOYMENT_GUIDE.md)） |
