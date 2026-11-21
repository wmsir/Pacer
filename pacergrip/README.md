# PacerGrip

**PacerGrip** 是一款基于 **Rust + Tauri v2 Native + Vue 3 + CodeMirror 6** 打造的商业级独立桌面数据库 IDE，深度复刻 **JetBrains DataGrip New UI** 交互体系，默认简体中文界面（支持中英文一键切换），同时完整保留扩展插槽（`pacer-core::extension` 与可选 `pacer-client`），可随时与 [`wmsir/Pacer`](https://github.com/wmsir/Pacer) 数据库治理平台无缝联动。

## 核心商业化特性

1. **JetBrains DataGrip New UI 级桌面工作台**
   - **默认简体中文 + 英文切换**：全界面菜单、工具栏、状态栏、弹窗与快捷键参考默认简体中文（`zh-CN`），支持右上角一键切换英文（`en-US`）。
   - **Darcula 深色 & IntelliJ 浅色双主题**：顶部菜单栏 + 居中 DataGrip Run Widget（数据源、Schema、执行、执行全部、EXPLAIN、SQL 格式化、行数上限）、左右 38px 工具窗口侧边条（Database / Files / Snippets / Structure / Extensions）、可拖拽调整宽高的分栏布局、环境色顶部边框标签页（`DEV` 绿 / `TEST` 琥珀 / `PROD` 红）。
   - **随处搜索（`Ctrl+K`）**：一键快速检索数据源、数据表、打开的标签页、Markdown SQL 块与常用命令。

2. **Markdown (`.md`) & SQL (`.sql`) 原生文档集成与选中执行**
   - **本地文件与目录挂载**：原生系统文件/目录选择弹窗（`Ctrl+O` 打开，左侧 Files Explorer 挂载本地目录与最近文件记录）。
   - **选中即执行（`Ctrl+Enter`）**：在 `.md`、`.sql` 或任意文档中框选一段 SQL，按 `Ctrl+Enter` 或点击顶栏高亮的「执行选中 SQL」按钮，直接发往当前配置的数据源执行。
   - **Markdown CodeLens 内联执行条**：自动识别文档内所有 ```` ```sql ```` / ```` ```mysql ```` / ```` ```postgresql ```` / ```` ```sqlite ```` 代码块，在每个代码块上方渲染 DataGrip 风格内联执行栏（`▶ 执行` / `⚡ EXPLAIN` / `⧉ 控制台打开`）。
   - **安全分栏预览**：支持「仅源码 / 分栏预览 / 仅预览」三种视图（零 `v-html` 注入风险），预览卡片内置可执行 SQL 按钮，并提供「文档 SQL 块」大纲快速跳转。
   - **一键示例文档**：内置「打开示例 Markdown SQL 手册」（`示例查询手册.md`），搭配内置 SQLite 演示库开箱即用。

3. **专业级文件状态栏与多编码无乱码支持**
   - 24px JetBrains 底部状态栏实时显示：`行 X, 列 Y`、`已选 N 字符 (M 行)`、`共 N 行`、文件字节大小、`CRLF / LF`（点击切换）、字符编码（点击切换）、缩进、SQL/Markdown 方言及只读保护开关。
   - 后端使用纯 Rust `encoding_rs` + `chardetng` 实现精确编码检测与无损回写，支持 **`UTF-8`、`UTF-8 BOM`、`GBK (GB18030)`、`UTF-16 LE`、`UTF-16 BE`、`Windows-1252`**，支持“以此编码重新解码文件”解决中文旧脚本乱码，以及“切换保存编码（`Ctrl+S` 生效）”。

4. **商业级 SQL 编辑器、结果集网格与安全防线**
   - **CodeMirror 6 数据库编辑器**：方言感知高亮（MySQL / PostgreSQL / SQLite / StandardSQL，且同步生效于 Markdown SQL 块内部）、动态表名与字段名自动补全、查找替换（`Ctrl+F`）、代码折叠与 SQL 格式化（`Ctrl+Alt+L`）。
   - **多结果集服务面板**：支持多语句顺序执行生成多个结果标签页、列头点击排序、关键字实时过滤、**行列转置（Transpose）**、**单元格值检视器（JSON 自动格式化）**、一键导出 **CSV / JSON / Markdown 表格 / `INSERT INTO` SQL**、控制台执行日志、建表 DDL、执行历史回放与 AST 静态检查面板。
   - **表结构与索引检视器**：跨 SQLite / MySQL / PostgreSQL 展示字段类型、主键、可空性及索引明细。
   - **高危 SQL 二次确认拦截**：内置 `sqlparser-rs` AST 静态分析器，自动识别 `DROP` / `TRUNCATE` / 缺 `WHERE` 的 `UPDATE`/`DELETE` 以及针对 `PROD` 数据源的写入变更，执行前弹窗强制确认。

5. **DBeaver 连接全兼容 & URL 双向绑定配置面板**
   - 完整复刻专业级数据源配置弹窗：`Name` + `Color` + `Add comment`、`General / Options / SSH/SSL / Schemas / Advanced` 五标签页、`default / Unix Socket / URL only` 三模式切换、`Host / Port / User / Password / Save (Forever / Until restart / Never) / Database` 与 `URL (Overrides settings above)` 实时双向绑定，以及底栏内联 **`测试连接 (Test Connection)`**（实时展示服务端版本如 `TiDB 6.5.5` 与延迟）和 `OK / Cancel / Apply`。
   - **DBeaver 连接导入与导出 (`pacer-dbeaver-io`)**：支持一键自动扫描本机 DBeaver 工作区（`%APPDATA%\DBeaverData\workspace6\...`）、导入 `data-sources.json` / `.dbp` 项目包 / `.dbeaver` 目录 / 旧版 `.dbeaver-data-sources.xml` / 粘贴文本，并支持可选解密读取 `credentials-config.json` 已保存密码；可无密码安全导出回 DBeaver `data-sources.json`。
   - 完整保留 `pacer-core::extension`（`GovernanceExtension` / `AiSqlExtension`）与 `pacer-client`（`--features pacer-extension`）。

## 工作区结构

```text
Pacer/
├── backend/                   # wmsir/Pacer Java/Spring Boot 3 服务端
├── frontend/                  # wmsir/Pacer Vue 3 治理控制台
├── app/                       # wmsir/Pacer Flutter 移动端
└── pacergrip/                 # Rust + Tauri v2 Native DataGrip 桌面 IDE
    ├── crates/
    │   ├── pacer-core/        # ConnectionProfile, DbBackend trait, 文件/索引 DTO 与扩展插槽
    │   ├── pacer-drivers/     # 原生异步 sqlx 驱动 (SQLite, MySQL/MariaDB/TiDB/OceanBase, PostgreSQL/openGauss/Kingbase)
    │   ├── pacer-dbeaver-io/  # DBeaver data-sources.json / .dbp / credentials-config.json / XML 导入导出与 URL 解析
    │   ├── pacer-sql-ast/     # sqlparser-rs AST 拆分、光标语句提取、Markdown SQL 块提取、格式化与风险检测
    │   └── pacer-client/      # 可选 wmsir/Pacer REST SDK（由 Cargo feature `pacer-extension` 控制）
    ├── src-tauri/             # Tauri v2 原生宿主（连接管理、流式查询、多编码文件读写、系统弹窗）
    └── ui/                    # Vue 3 + TypeScript + CodeMirror 6 + Vite DataGrip New UI 桌面工作台
```

## 本地构建与验证

```bash
# 1. 构建前端资源
cd ui && npm install && npm run build
cd ..

# 2. 运行全部 Rust 单元测试与 SQLite 集成测试（含多编码文件与 Markdown SQL 块测试）
cargo test --workspace --exclude pacergrip-app
cargo test -p pacergrip-app --lib

# 3. 严格 Clippy 校验（默认独立模式 & 可选 pacer-extension 特性）
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p pacergrip-app --features pacer-extension --all-targets -- -D warnings

# 4. 启动本地桌面端开发模式
.\ui\node_modules\.bin\tauri.cmd dev
```

## GitHub Actions 自动编译 Windows `.exe`

仓库内置 [`../.github/workflows/build-pacergrip-windows.yml`](../.github/workflows/build-pacergrip-windows.yml) 与 [`../.github/workflows/release-pacergrip.yml`](../.github/workflows/release-pacergrip.yml)，在 `windows-latest` 上编译生产级 Windows 64 位二进制：

- **`PacerGrip-windows-x64-portable.exe`**：绿色单文件便携版（无需安装，双击即用）
- **`PacerGrip_*_x64-setup.exe`**：Windows 安装包（支持简中/英文安装界面，按当前用户安装并自动引导安装缺失的 WebView2 Runtime）
- **`SHA256SUMS.txt`**：对应二进制 SHA-256 校验和
