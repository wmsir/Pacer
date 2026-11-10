# 稳骥 PacerSQL · 移动审批与巡检终端 (`app`)

基于 **Flutter 3.x (Dart)** 构建的跨平台移动客户端（支持 Android / iOS / Windows / macOS / Linux / Web），为 DBA、研发负责人与运维架构师提供随时随地的移动工单审批、实例健康监控与网络链路诊断能力。

## 核心特性

- **移动工单即时审批**：实时查看待审 SQL 工单、AST 风险评分、预估影响行数与执行计划，支持移动端一键通过、定时执行或驳回。
- **数据库实例健康看板**：移动端实时巡检多云数据库实例在线状态、连接水位与告警指标。
- **三网高可用网关自适应**：内置多节点专线探测与网络诊断工具（`NetworkDiagnosticsScreen`），自动切换最优网关线路。
- **扫码快捷授权与生物识别**：支持 Web 工作台二维码扫码登录（`QrScannerScreen`）与移动端安全认证。

## 目录结构

```text
app/
├── lib/
│   ├── config/          # 全局应用配置与多网关节点定义 (app_config.dart)
│   ├── models/          # 工单、数据库实例与用户数据模型
│   ├── screens/         # 看板、工单列表/详情、实例监控、扫码登录与网络诊断页面
│   ├── services/        # API 通信、身份认证与集群状态服务
│   ├── theme/           # PacerSQL 视觉主题与深色模式规范
│   └── main.dart        # 应用入口
├── android/             # Android 原生工程配置 (cn.fwpt.pacer.pacer_app)
├── ios/                 # iOS 原生工程配置
├── windows/             # Windows 桌面端工程配置
├── macos/               # macOS 桌面端工程配置
├── linux/               # Linux 桌面端工程配置
└── pubspec.yaml         # Flutter 依赖与资源声明
```

## 本地开发与打包

```bash
# 拉取依赖
flutter pub get

# 调试运行
flutter run

# 构建 Android Release APK
flutter build apk --release
```
