# WMDB 完美数据库平台 - 生产服务器部署与运维操作手册

> **部署目标服务器**：`<SERVER_IP>` (CentOS 7 Linux)  
> **访问协议与入口**：`http://<SERVER_IP>` (直接通过公网 IP 访问，默认 80 端口)  
> **更新时间**：2026-09-04  

---

## 一、 快速使用与访问入口

### 1. 系统在线访问地址
- **前端 Web 界面**：[http://<SERVER_IP>](http://<SERVER_IP>)
- **后端 API 代理**：[http://<SERVER_IP>/api/](http://<SERVER_IP>/api/)

### 2. 内置角色测试账号与密码
| 登录账号 (Account) | 真实姓名 | 所属角色 | 默认密码 | 角色权限与使用场景 |
| :--- | :--- | :--- | :--- | :--- |
| **`testadmin1`** | **王总** | **ADMIN (系统超级管理员)** | `123456` | 具备全功能特权：全局参数/安全策略/审批流设计/多源实例/直接终审 |
| **`testadmin2`** | **张伟** | **DEV_LEAD (业务开发组长)** | `123456` | 具备初审权限：业务变更审核、所属资源组工单管理 |
| **`testadmin3`** | **赵工** | **DBA (核心安全运维)** | `123456` | 具备复核/终审权限：SQL执行窗口控制、灰度分批/线下执行确认 |
| **`admin`** | **管理员** | **ADMIN** | `123456` | 系统内置通用管理特权账号 |

---

## 二、 部署架构与容器矩阵

系统采用高性能的 **Docker + Nginx 反向代理 + Spring Boot 3 容器化** 架构，与服务器原有业务相互隔离、互不干扰：

```
                              [ 用户浏览器 / 终端 ]
                                        │
                                        │ (HTTP 80 端口)
                                        ▼
                 ┌──────────────────────────────────────────────┐
                 │          Nginx 容器 (nginxtest)              │
                 │      监听 80 端口 (支持 IP 访问与域名隔离)        │
                 └──────────────┬────────────────────────┬──────┘
                                │                        │
                  (静态页面 /)  │                        │ (/api/ 代理)
                                ▼                        ▼
                 ┌────────────────────────┐    ┌────────────────────────┐
                 │   前端静态资源 (Vue3)   │    │  后端容器 (wmdb-backend)│
                 │ /usr/share/nginx/html/ │    │      (端口 8088)       │
                 │          wmdb          │    │   JDK 17 + SpringBoot  │
                 └────────────────────────┘    └────────────┬───────────┘
                                                            │
                                    ┌───────────────────────┴───────────────────────┐
                                    ▼                                               ▼
                         ┌───────────────────────┐                     ┌────────────────────────┐
                         │  Redis 缓存容器        │                     │   生产 RDS MySQL 8     │
                         │ (wmdb-redis: 6379)    │                     │ (rm-uf6ab...aliyuncs)  │
                         └───────────────────────┘                     └────────────────────────┘
```

### 运行容器一览
| 容器名称 | 基础镜像 | 映射端口 | 容器说明 |
| :--- | :--- | :--- | :--- |
| **`nginxtest`** | `nginx:latest` | `0.0.0.0:80->80`<br>`0.0.0.0:443->443` | 负责静态前端托管（单页路由 History 支持）与 `/api/` 转发 |
| **`wmdb-backend`** | `wmdb-backend:latest`<br>*(基于 eclipse-temurin:17-jre-alpine)* | `0.0.0.0:8088->8088` | Spring Boot 3 + Flowable 审批引擎核心服务 |
| **`wmdb-redis`** | `redis:7-alpine` | `0.0.0.0:6379->6379` | 会话缓存与频次限制 |

---

## 三、 从零开始部署全流程

### 步骤 1：本地构建与打包

#### 1.1 前端项目打包 (Vue3 + Vite + Element Plus)
在本地 `frontend/` 目录下执行：
```bash
cd frontend
npm run build
```
打包生成后的文件位于 `frontend/dist/`，将其压缩打包为 `frontend_dist.zip` 便于远端快速传输：
```powershell
Compress-Archive -Path frontend\dist\* -DestinationPath frontend_dist.zip -Force
```

#### 1.2 后端项目打包 (Spring Boot 3 + Java 17)
在本地 `backend/` 目录下执行 Maven 编译打包：
```bash
cd backend
mvn clean package -DskipTests=true
```
编译成功后生成：`backend/target/wmdb-backend-1.0.0-SNAPSHOT.jar`。

---

### 步骤 2：上传发布包至远程服务器
通过 SFTP / SCP 将文件传送到服务器的 `/opt/wmdb/` 目录：
```bash
# 在本地终端执行（也可以使用 Xftp / FinalShell 拖拽上传）
scp frontend_dist.zip root@<SERVER_IP>:/opt/wmdb/
scp backend/target/wmdb-backend-1.0.0-SNAPSHOT.jar root@<SERVER_IP>:/opt/wmdb/wmdb-backend.jar
```

---

### 步骤 3：服务器端解压前端静态资源
登录服务器执行：
```bash
# 确保 Nginx 挂载目录存在并解压
mkdir -p /opt/docker/nginx/html/wmdb
unzip -o /opt/wmdb/frontend_dist.zip -d /opt/docker/nginx/html/wmdb/
```

---

### 步骤 4：启动与构建后端容器

#### 4.1 启动 Redis 依赖容器
```bash
docker rm -f wmdb-redis || true
docker run -d --name wmdb-redis --restart always -p 6379:6379 redis:7-alpine
```

#### 4.2 编写后端 Dockerfile (`/opt/wmdb/Dockerfile`)
```dockerfile
FROM eclipse-temurin:17-jre-alpine
WORKDIR /app
COPY wmdb-backend.jar app.jar
EXPOSE 8088
ENV JAVA_OPTS="-Xms256m -Xmx600m -Dfile.encoding=UTF-8 -Duser.timezone=Asia/Shanghai"
ENTRYPOINT ["sh", "-c", "java $JAVA_OPTS -jar app.jar --server.port=8088 --spring.redis.host=172.17.0.1 --spring.redis.port=6379"]
```

#### 4.3 构建后端镜像并启动
```bash
docker rm -f wmdb-backend || true
docker build -t wmdb-backend:latest /opt/wmdb
docker run -d --name wmdb-backend --restart always -p 8088:8088 wmdb-backend:latest
```

---

### 步骤 5：配置 Nginx 代理与重载
编辑 `/opt/docker/nginx/conf/nginx.conf`，在 `http` 块内加入默认 IP 访问配置：

```nginx
# 🟢 WMDB 完美数据库管理系统 (直接通过 IP:80 访问)
server {
    listen       80 default_server;
    server_name  <SERVER_IP> _;

    root   /usr/share/nginx/html/wmdb;
    index  index.html index.htm;

    # 前端单页面应用 HTML5 History 模式支持
    location / {
        try_files $uri $uri/ /index.html;
    }

    # 后端 API 反向代理至 Spring Boot 容器
    location /api/ {
        proxy_pass http://172.17.0.1:8088/api/;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        client_max_body_size 100m;
        proxy_connect_timeout 60s;
        proxy_send_timeout 180s;
        proxy_read_timeout 180s;
    }

    error_page   500 502 503 504  /50x.html;
    location = /50x.html {
        root   /usr/share/nginx/html;
    }
}
```

配置保存后测试并重载 Nginx：
```bash
docker exec nginxtest nginx -t
docker exec nginxtest nginx -s reload
```

---

## 四、 常见运维与排障命令

### 1. 查看后端运行状态与日志
```bash
# 实时跟踪后端最新日志
docker logs -f --tail 100 wmdb-backend

# 检查后端端口连通性
curl -I http://127.0.0.1:8088/api/v1/auth/login
```

### 2. 重启服务容器
```bash
# 重启后端服务
docker restart wmdb-backend

# 重启 Redis
docker restart wmdb-redis

# 重载 Nginx 配置
docker exec nginxtest nginx -s reload
```

### 3. 一键热更新发版脚本 (`/opt/wmdb/deploy.sh`)
服务器已放置自动化热更新脚本，后续代码更新后只需执行：
```bash
cd /opt/wmdb
bash deploy.sh
```

---

## 五、 验证检查清单

- [x] **页面直访验证**：浏览器打开 `http://<SERVER_IP>`，页面正常渲染 Vue 3 界面。
- [x] **用户登录验证**：输入 `testadmin1` / `123456`，成功获取 JWT Token 并跳转控制台首页。
- [x] **数据连通验证**：工单列表拉取阿里云 RDS 数据库真实历史工单（60+条）。
- [x] **接口代理验证**：前端发起 `/api/v1/...` 请求返回 HTTP 200，无跨域报错。
- [x] **原有服务不受影响**：服务器原有的 `toolso.cn` HTTPS 站点正常运行。

---

## 六、 多数据库与国产数据库（达梦/金仓/高斯/TiDB/OceanBase/Oracle）部署与自动化执行实战手册

系统采用**配置驱动与方言智能嗅探架构**（`DatabaseAutoInitializerRunner`），底层具备完善的多源异构数据方言支持，可在服务启动部署时根据环境变量或配置文件**全自动识别数据库类型并精准执行对应方言的 DDL/DML 初始化脚本**。

---

### 1. 核心架构与自动化建表机制

```text
启动 Spring Boot (wmdb-backend)
           │
           ▼
[DatabaseAutoInitializerRunner (@Order(1))]
           │
   嗅探 spring.datasource.url 与 JDBC MetaData
           │
   ┌───────┴────────────────────────┬─────────────────────────┬──────────────────────┐
   ▼                                ▼                         ▼                      ▼
【达梦 DM8】                   【人大金仓 KES】            【华为 openGauss】      【MySQL / TiDB】
classpath:db/dameng/schema.sql   classpath:db/kingbase/    classpath:db/opengauss/ classpath:db/mysql/
   │                                │                         │                      │
   └────────────────────────────────┼─────────────────────────┴──────────────────────┘
                                    │
                                    ▼
                         执行方言脚本 (建表与索引)
                                    │
                                    ▼
[TestDataInitializerRunner (@Order(2))]：注入角色/8大资源组/真实工单数据
                                    │
                                    ▼
                           平台正常提供服务！
```

---

### 2. 数据库类型配置与环境变量清单

在 `application.yml` 或 Docker 环境变量中，支持以下核心参数：

| 配置键 | 对应环境变量 | 默认值 | 可选值 / 说明 |
| :--- | :--- | :--- | :--- |
| `wmdb.database.auto-init` | `WMDB_DB_AUTO_INIT` | `true` | `true` (开启启动时自动建表), `false` (关闭) |
| `wmdb.database.type` | `WMDB_DB_TYPE` | `auto` | `auto` (智能嗅探)、`mysql`、`dameng`、`kingbase`、`opengauss`、`oracle`、`tidb`、`oceanbase` |
| `wmdb.database.continue-on-error` | `WMDB_DB_CONTINUE_ON_ERROR` | `true` | `true` (遇到已存在的表自动忽略，保障多次部署幂等性) |
| `spring.datasource.url` | `WMDB_DB_URL` | 云 RDS 地址 | 目标数据库 JDBC 连接串 |
| `spring.datasource.username` | `WMDB_DB_USERNAME` | `root` | 数据库账号 |
| `spring.datasource.password` | `WMDB_DB_PASSWORD` | - | 数据库密码 |
| `spring.datasource.driver-class-name`| `WMDB_DB_DRIVER` | `com.mysql.cj.jdbc.Driver` | 对应数据库的 JDBC 驱动全类名 |

---

### 3. 各数据库实战部署配方 (Recipes)

#### 配方 A：MySQL 8.0 / 阿里云 RDS / 腾讯云 CDB（标准推荐）
- **驱动全类名**：`com.mysql.cj.jdbc.Driver`
- **默认执行脚本**：`classpath:db/mysql/schema.sql`
- **Docker 部署运行命令**：
  ```bash
  docker run -d --name wmdb-backend \
    --restart always \
    -p 8088:8088 \
    -e WMDB_DB_URL="jdbc:mysql://192.168.1.50:3306/wmdb?useSSL=false&allowPublicKeyRetrieval=true&serverTimezone=Asia/Shanghai&characterEncoding=utf8" \
    -e WMDB_DB_USERNAME="wmdb_user" \
    -e WMDB_DB_PASSWORD="YourPassword123++" \
    -e WMDB_DB_DRIVER="com.mysql.cj.jdbc.Driver" \
    -e WMDB_DB_TYPE="mysql" \
    wmdb-backend:latest
  ```

---

#### 配方 B：国产达梦数据库 (DaMeng DM8 / DM7)
- **驱动依赖与类名**：`dm.jdbc.driver.DmDriver`（系统已预置 `com.dameng:DmJdbcDriver18:8.1.3.140`）
- **执行方言脚本**：`classpath:db/dameng/schema.sql`（已针对达梦语法适配 `IDENTITY(1,1)`、`VARCHAR`、大写表名与注释语法）
- **JDBC 连接串格式**：`jdbc:dm://<host>:5236/<DATABASE_NAME>?schema=SYSDBA&compatibleMode=mysql`
- **Docker 部署运行命令**：
  ```bash
  docker run -d --name wmdb-backend \
    --restart always \
    -p 8088:8088 \
    -e WMDB_DB_URL="jdbc:dm://192.168.1.60:5236/DAMENG?schema=SYSDBA" \
    -e WMDB_DB_USERNAME="SYSDBA" \
    -e WMDB_DB_PASSWORD="SYSDBA_PASSWORD" \
    -e WMDB_DB_DRIVER="dm.jdbc.driver.DmDriver" \
    -e WMDB_DB_TYPE="dameng" \
    wmdb-backend:latest
  ```
- **启动成功日志特征**：
  ```text
  [DB-AUTO-INIT] 📌 底层驱动检测产品: 【DM DBMS】
  [DB-AUTO-INIT] 🇨🇳 识别为国产【达梦数据库 (DaMeng DM)】引擎
  [DB-AUTO-INIT] 📂 正在加载并执行初始化脚本: 【classpath:db/dameng/schema.sql】
  [DB-AUTO-INIT] 🌟 脚本【schema.sql】批量执行完毕！
  ```

---

#### 配方 C：国产人大金仓 (KingbaseES KES V8R3 / V8R6 / V9)
- **驱动依赖与类名**：`com.kingbase8.Driver`（系统已预置 `cn.com.kingbase:kingbase8:8.6.0`）
- **执行方言脚本**：`classpath:db/kingbase/schema.sql`（支持金仓 `BIGINT GENERATED BY DEFAULT AS IDENTITY` 自增序列与序列级幂等）
- **JDBC 连接串格式**：`jdbc:kingbase8://<host>:54321/<DATABASE_NAME>?currentSchema=public`
- **Docker 部署运行命令**：
  ```bash
  docker run -d --name wmdb-backend \
    --restart always \
    -p 8088:8088 \
    -e WMDB_DB_URL="jdbc:kingbase8://192.168.1.61:54321/wmdb?currentSchema=public" \
    -e WMDB_DB_USERNAME="system" \
    -e WMDB_DB_PASSWORD="KingbasePassword++" \
    -e WMDB_DB_DRIVER="com.kingbase8.Driver" \
    -e WMDB_DB_TYPE="kingbase" \
    wmdb-backend:latest
  ```
- **启动成功日志特征**：
  ```text
  [DB-AUTO-INIT] 📌 底层驱动检测产品: 【KingbaseES】
  [DB-AUTO-INIT] 🇨🇳 识别为国产【人大金仓 (KingbaseES)】引擎
  [DB-AUTO-INIT] 📂 正在加载并执行初始化脚本: 【classpath:db/kingbase/schema.sql】
  [DB-AUTO-INIT] 🌟 脚本【schema.sql】批量执行完毕！
  ```

---

#### 配方 D：国产华为 openGauss / MogDB / 统信 UOS
- **驱动依赖与类名**：`org.opengauss.Driver`（兼容 `org.postgresql.Driver`）
- **执行方言脚本**：`classpath:db/opengauss/schema.sql`（适配 openGauss `SERIAL / BIGSERIAL`，`VARCHAR` 规范与约束）
- **JDBC 连接串格式**：`jdbc:opengauss://<host>:5432/<DATABASE_NAME>?currentSchema=public`
- **Docker 部署运行命令**：
  ```bash
  docker run -d --name wmdb-backend \
    --restart always \
    -p 8088:8088 \
    -e WMDB_DB_URL="jdbc:opengauss://192.168.1.62:5432/wmdb?currentSchema=public" \
    -e WMDB_DB_USERNAME="opengauss" \
    -e WMDB_DB_PASSWORD="OpenGaussPassword++" \
    -e WMDB_DB_DRIVER="org.opengauss.Driver" \
    -e WMDB_DB_TYPE="opengauss" \
    wmdb-backend:latest
  ```

---

#### 配方 E：企业商业数据库 Oracle 12c / 19c / 21c
- **驱动依赖与类名**：`oracle.jdbc.OracleDriver`（系统已引入 `com.oracle.database.jdbc:ojdbc8`）
- **执行方言脚本**：`classpath:db/oracle/schema.sql`（支持 `NUMBER(20) GENERATED ALWAYS AS IDENTITY` 或触发器序列建表）
- **JDBC 连接串格式**：`jdbc:oracle:thin:@//<host>:1521/<SERVICE_NAME>`
- **Docker 部署运行命令**：
  ```bash
  docker run -d --name wmdb-backend \
    --restart always \
    -p 8088:8088 \
    -e WMDB_DB_URL="jdbc:oracle:thin:@//192.168.1.70:1521/ORCLPDB1" \
    -e WMDB_DB_USERNAME="wmdb_user" \
    -e WMDB_DB_PASSWORD="OraclePassword123" \
    -e WMDB_DB_DRIVER="oracle.jdbc.OracleDriver" \
    -e WMDB_DB_TYPE="oracle" \
    wmdb-backend:latest
  ```

---

#### 配方 F：国产云原生分布式数据库 TiDB / OceanBase
- **驱动依赖与类名**：`com.mysql.cj.jdbc.Driver`
- **说明**：TiDB 与 OceanBase (MySQL 模式) 与 MySQL 协议 100% 兼容，系统自动执行 `classpath:db/mysql/schema.sql`。
- **JDBC 连接串格式**：`jdbc:mysql://<tidb-host>:4000/wmdb?useSSL=false`
- **Docker 部署运行命令**：
  ```bash
  docker run -d --name wmdb-backend \
    --restart always \
    -p 8088:8088 \
    -e WMDB_DB_URL="jdbc:mysql://192.168.1.80:4000/wmdb?useSSL=false&allowMultiQueries=true" \
    -e WMDB_DB_USERNAME="root" \
    -e WMDB_DB_PASSWORD="TiDBPassword++" \
    -e WMDB_DB_TYPE="tidb" \
    wmdb-backend:latest
  ```

---

### 4. 离线/自定义 SQL 脚本扩展指引

如果企业内部有定制字段或特殊表空间需求，可在代码结构中的对应目录下直接替换或追加脚本：

```text
backend/src/main/resources/db/
├── dameng/
│   └── schema.sql       # 达梦数据库全量表结构与索引定义
├── kingbase/
│   └── schema.sql       # 人大金仓表结构与序列定义
├── opengauss/
│   └── schema.sql       # 华为 openGauss 表结构定义
├── oracle/
│   └── schema.sql       # Oracle 表结构与序列定义
└── mysql/
    └── schema.sql       # MySQL / TiDB / OceanBase 表结构定义
```

> **提示**：系统在每次启动时会检查并执行脚本，内置 `continue-on-error: true`，对于已存在的表会自动跳过，无需担心破坏已有业务数据。

