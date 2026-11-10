# 稳骥 PacerSQL · 后端核心服务 (`backend`)

基于 **Spring Boot 3.2 + Java 17 + Flowable 7.0 + MyBatis-Plus** 构建的企业级数据库协同治理、SQL AST 语法树审核与自动化变更执行引擎。

## 核心能力

- **SQL AST 审核与规则引擎**：基于 Druid / JSQLParser 语法树解析，支持 DDL/DML 风险评估、黑名单拦截、影响行数注解识别与事务级预执行回滚校验（Dry Run）。
- **审批流与执行解耦**：内置 Flowable 7.0 BPMN 2.0 流程引擎，支持立即执行（`IMMEDIATE`）、定时计划窗口执行（`SCHEDULED`）与转 DBA 线下执行（`MANUAL_DBA`）。
- **多源异构与信创数据库自适应**：内置 `DatabaseAutoInitializerRunner`，支持 MySQL 8.0、达梦（DM8）、人大金仓（KingbaseES）、华为 openGauss、Oracle、TiDB、OceanBase。
- **国密与区块链审计**：支持 SM2/SM3/SM4 国密算法加解密与防篡改审计哈希链存证。
- **AI DBA 与智能审查**：支持 Text2SQL、SQL 风险解释、慢查询优化与数据库健康巡检。

## 目录结构

```text
backend/
├── src/main/java/com/wmdb/
│   ├── config/          # Spring Security、Swagger/OpenAPI、多数据源与跨域配置
│   ├── controller/      # RESTful API 控制器 (Auth, Ticket, Instance, SqlQuery, Workflow, AI 等)
│   ├── dto/             # 请求与响应数据传输对象
│   ├── entity/          # 数据库实体模型
│   ├── mapper/          # MyBatis-Plus Mapper 接口
│   ├── model/           # 领域值对象与执行结果模型
│   ├── runner/          # 多数据库方言自动初始化与演练数据加载器
│   ├── security/        # JWT、RSA、国密 SM2/SM3/SM4 与 XSS 防护组件
│   ├── service/         # 核心业务服务层
│   └── utils/           # SQL 切分与影响行数注解解析工具
├── src/main/resources/
│   ├── db/              # 多数据库方言 DDL 脚本 (mysql, dameng, kingbase, opengauss, oracle)
│   ├── processes/       # Flowable BPMN 2.0 流程定义文件
│   └── application.yml  # 服务主配置（支持环境变量覆盖）
├── Dockerfile           # 容器化构建定义
└── pom.xml              # Maven 依赖管理
```

## 本地运行与构建

```bash
# 开发模式启动（默认监听 8088 端口）
mvn spring-boot:run

# 编译打包可执行 JAR
mvn clean package -DskipTests
```

启动后可访问 Swagger 接口文档：`http://localhost:8088/swagger-ui/index.html`
