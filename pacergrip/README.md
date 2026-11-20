# PacerGrip

**PacerGrip** is a standalone DataGrip-inspired native desktop database IDE built with **Rust + Tauri v2 Native + Vue 3**. It supports native SQL execution (SQLite, MySQL/MariaDB/TiDB/OceanBase, PostgreSQL/openGauss/Kingbase), DataGrip connection XML/`.idea`/`.zip` import & export, schema browsing, table DDL inspection, multi-tab SQL consoles, and query history — while keeping clean extension hooks (`pacer-core::extension` + optional `pacer-client`) ready for future governance/platform integrations such as [`wmsir/Pacer`](https://github.com/wmsir/Pacer).

## Workspace Layout

```text
Pacer/
├── backend/                   # wmsir/Pacer Java/Spring Boot 3 backend
├── frontend/                  # wmsir/Pacer Vue 3 governance console
├── app/                       # wmsir/Pacer Flutter mobile app
└── pacergrip/                 # Rust + Tauri v2 Native DataGrip Desktop IDE
    ├── crates/
    │   ├── pacer-core/        # ConnectionProfile, DbBackend trait & extension hooks (GovernanceExtension / AiSqlExtension)
    │   ├── pacer-drivers/     # Native async SQL drivers via sqlx (SQLite, MySQL/MariaDB/TiDB/OceanBase, PostgreSQL/openGauss/Kingbase)
    │   ├── pacer-datagrip-io/ # DataGrip XML / .idea / sshConfigs.xml / .zip parser & exporter + fingerprint matcher
    │   ├── pacer-sql-ast/     # sqlparser-rs AST splitter, DQL vs DML/DDL classifier & risk detector
    │   └── pacer-client/      # Optional wmsir/Pacer REST SDK & proxy backend (behind Cargo feature `pacer-extension`)
    ├── src-tauri/             # Tauri v2 Native host (connection persistence, streaming Channel<QueryChunk>, ExtensionRegistry)
    └── ui/                    # Vue 3 + TypeScript + Vite 3-pane DataGrip-style desktop workbench
```

## Capabilities

1. **Native Database Drivers (`pacer-drivers`)**
   - **SQLite**: embedded native driver + one-click **Demo SQLite Store** sample database (`customers`, `products`, `orders`, `order_items`).
   - **MySQL / MariaDB / TiDB / OceanBase**: native async `sqlx` driver supporting schemas, tables, columns, `SHOW CREATE TABLE`, `EXPLAIN`, and streamed result chunks.
   - **PostgreSQL / openGauss / KingbaseES**: native async `sqlx` driver supporting schemas, tables, primary keys, synthesized `CREATE TABLE` DDL, `EXPLAIN`, and `SET search_path`.
   - Passwords are kept strictly in-memory for the session and never written into `connections.json` or exported DataGrip XML.

2. **DataGrip Connection Compatibility (`pacer-datagrip-io`)**
   - Import from DataGrip Clipboard XML, `.idea/dataSources.xml` + `dataSources.local.xml` + `sshConfigs.xml`, or JetBrains Settings `.zip` archives.
   - Export connections back to DataGrip clipboard XML without leaking credentials.

3. **Preserved Extension Architecture (`pacer-core::extension`)**
   - `ExtensionRegistry` holds optional `GovernanceExtension` (`before_execute`, `report_audit`) and `AiSqlExtension` (`text_to_sql`, `optimize_sql`) plugins.
   - `execute_sql_cmd` automatically consults `check_before_execute` and emits `AuditEvent` records, and `pacer-client` remains wired behind `pacergrip-app`'s optional `pacer-extension` Cargo feature.

## Build & Verification

```bash
# 1. Build Vue 3 frontend
cd ui && npm install && npm run build
cd ..

# 2. Run Rust unit & SQLite integration tests
cargo test --workspace --exclude pacergrip-app

# 3. Check Tauri v2 host (default standalone mode & optional pacer-extension feature)
cargo check --workspace
cargo check -p pacergrip-app --features pacer-extension

# 4. Run Desktop IDE in dev mode
.\ui\node_modules\.bin\tauri.cmd dev
```

## GitHub Actions 编译 Windows `.exe`

仓库已内置 [`../.github/workflows/build-pacergrip-windows.yml`](../.github/workflows/build-pacergrip-windows.yml)，推送到 `wmsir/Pacer` 后会在 `windows-latest` 上自动编译出可直接运行的 Windows 64 位可执行文件：

- **`PacerGrip-windows-x64-portable.exe`**：绿色单文件便携版（无需安装，双击即用）
- **`PacerGrip_*_x64-setup.exe`**：Windows 安装包（支持简中/英文安装界面，按当前用户安装并自动引导安装缺失的 WebView2 Runtime）
- **`SHA256SUMS.txt`**：对应二进制校验和

### 获取编译产物

1. **每次推送 / 手动触发**：打开 GitHub 仓库的 **Actions → Build PacerGrip Windows Executable → 最新一次运行 → Artifacts**，下载 `PacerGrip-windows-x64` 压缩包即可直接运行 `.exe`。
2. **发布正式 GitHub Release**：推送 `pacergrip-v*` 标签（由 [`../.github/workflows/release-pacergrip.yml`](../.github/workflows/release-pacergrip.yml) 自动编译并创建 Release）：
   ```bash
   git tag -a pacergrip-v0.1.0 -m "PacerGrip v0.1.0"
   git push origin pacergrip-v0.1.0
   ```
