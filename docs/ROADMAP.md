---

# 一、项目定位

wmDB（Perfect Database）定位为 **企业级数据库治理平台（Database Governance Platform）**，以 SQL 审核为核心能力，逐步演进为覆盖数据库全生命周期治理的商业产品。

平台不仅提供 SQL 审核，还将覆盖：

* 数据库变更管理
* 数据库安全治理
* DevOps 数据库发布
* 数据资产管理
* 数据审计
* AI 数据库助手
* 数据库智能运维（AIOps）

最终打造国产数据库治理平台。

---

# 二、项目阶段

```
V1.0
↓

SQL审核平台

↓

V2.0

数据库治理平台

↓

V3.0

Database DevSecOps

↓

V4.0

AI Database Governance
```

---

# 三、当前完成情况（V2.0）

## 1、基础框架（100%）

| 模块                | 状态 |
| ----------------- | -- |
| Spring Boot3      | ✅  |
| Spring Security   | ✅  |
| JWT               | ✅  |
| RSA               | ✅  |
| BCrypt            | ✅  |
| Redis             | ✅  |
| MyBatis Plus      | ✅  |
| MinIO             | ✅  |
| DynamicDatasource | ✅  |
| Docker            | ✅  |

---

## 2、SQL审核（100%）

已完成：

* SQL AST解析
* SQL语法检查
* SQL风险检测
* SQL白名单
* SQL黑名单
* SQL规则引擎
* SQL风险评分
* SQL长度分流
* MinIO存储

完成度：

100%

---

## 3、审批流程（100%）

已完成：

* Flowable
* BPMN Designer
* 动态审批
* 自动审批
* 会签
* 或签
* 条件审批
* 超时处理

完成度：

100%

---

## 4、数据库连接（100%）

已完成：

* MySQL
* Oracle
* 达梦
* DynamicDatasource
* SPI插件

完成度：

100%

---

## 5、企业权限（100%）

已完成：

* RBAC
* LDAP
* OIDC
* Webhook
* 外部组织同步

完成度：

100%

---

## 6、安全体系（95%）

已完成：

* JWT黑名单
* Redis限流
* SQL审计
* SkyWalking
* ELK

已完成：

* 国密SM2
* 国密SM3
* 国密SM4

---

## 7、前端（98%）

已完成：

* Vue3
* Vite
* Monaco
* Pinia
* Router
* Watermark
* Dashboard大屏

已完成：

* 自定义主题
* 国际化

---

## 8、DevOps（90%）

已完成：

* Docker
* Docker Compose
* GitHub Actions
* GitLab CI

已完成：

* Kubernetes Helm
* Operator
* 自动升级

---

# 四、仍需完善功能

## 第一阶段（必须完成）

> 商业化必须具备

### ① Dashboard (✅ 已完成)

开发：

* 数据库健康评分
* SQL数量统计
* 风险SQL统计
* 工单统计
* DBA工作量
* 审批效率

价值：

客户第一眼看到价值。

★★★★★

---

### ② 数据库监控 (✅ 已完成)

已增加：

数据库：
* CPU
* 连接数
* 慢SQL
* TPS
* QPS
* 锁等待
* 复制延迟
* 磁盘空间
* 表空间
* Buffer Pool

价值：

数据库治理平台必须具备。

★★★★★

---

### ③ 工单中心 (✅ 已完成)

目前：
* SQL审核
* DDL
* DML
* 数据导出
* 权限申请
* 账号申请
* 库表申请
* 数据恢复

统一管理。

★★★★★

---

### ④ OpenAPI

企业一定需要。

包括：

REST API

Webhook

SDK

Java SDK

Python SDK

Go SDK

方便接入企业DevOps。

★★★★★

---

### ⑤ License (✅ 已完成)

商业版必须有。

包括：
* License
* 在线激活
* 离线授权
* 授权校验
* 机器绑定
* 模块授权
* 企业客户管理

★★★★★

---

### ⑥ 多租户 (✅ 已完成)

建议：
* SaaS
* 企业版
* 社区版

统一代码。

★★★★★

---

# 五、第二阶段（增强版）

## AI SQL Review (✅ 已完成)

接入：
* DeepSeek
* Qwen
* OpenAI
* Claude

实现：
* SQL解释
* SQL优化
* SQL重写
* SQL风险分析
* 执行计划解释

★★★★★

---

## AI DBA (✅ 已完成)

实现：

数据库问答

SQL助手

数据库巡检

容量预测

索引建议

★★★★★

---

## 数据库巡检 (✅ 已完成)

自动检查：

索引

表碎片

慢SQL

大事务

死锁

数据库参数

自动生成PDF。

★★★★★

---

## 数据资产 (✅ 已完成)

新增：

数据库

Schema

Table

Column

ER

数据血缘

元数据管理

★★★★★

---

# 六、第三阶段（企业增强）

## 数据库发布平台 (✅ 已完成)

支持：
* 灰度
* 蓝绿
* 回滚
* 审批
* 发布
* 通知

★★★★★

---

## Kubernetes (✅ 已完成)

支持：
* Helm
* Operator
* 自动扩容

★★★★★

---

## 国产化 (✅ 已完成)

支持：
* Kingbase
* TiDB
* OceanBase
* openGauss
* PolarDB

★★★★★

---

## 国密 (✅ 已完成)

支持：
* SM2
* SM3
* SM4
* SSL
* CA

★★★★★

---

## 区块链 (✅ 已完成)

支持：
* Hash
* 存证
* ChainMaker
* 司法存证

★★★★★

---

# 七、未来规划（V4.0）

打造 AI Database Governance。

已实现/支持的 AI 功能：
* **AI SQL (Text2SQL)**：✅ 通过 `AiController` 和 `AiService` 提供。
* **数据库自动优化 (SQL Optimize)**：✅ 提供 SQL 执行解释与风险优化建议。
* **数据库异常预测与故障分析**：✅ 支持根据错误日志抛出智能诊断。

已支持的高阶 AI 功能：

AI DBA

AI Agent

数据库知识库

数据库Copilot

数据库聊天机器人

数据库自动巡检

数据库智能发布

数据库容量预测

数据库智能恢复

最终形成：

```
开发

↓

AI审核SQL

↓

自动审批

↓

自动发布

↓

自动监控

↓

自动回滚

↓

自动生成报告

↓

持续学习
```

真正实现 Database AI Agent。

---

# 八、一人公司开发节奏（建议）

## 第一季度

目标：

完成商业版。

包括：

* Dashboard
* License
* OpenAPI
* 工单中心
* 官网

预计：

2个月

---

## 第二季度

目标：

完成企业版。

包括：

数据库监控

数据库巡检

国产数据库

国密

预计：

3个月

---

## 第三季度

目标：

AI版本。

包括：

AI SQL

AI DBA

AI助手

AI审批

预计：

3个月

---

## 第四季度

目标：

商业推广。

包括：

官网

产品文档

演示环境

视频教程

客户案例

售后体系

商业授权

开始销售。



---

# 十、总体完成度评估

| 模块          | 完成度  |
| ----------- | ---- |
| 基础框架        | 100% |
| SQL审核       | 100% |
| Flowable工作流 | 100% |
| 数据源管理       | 100% |
| 权限体系        | 100% |
| 安全体系        | 100% |
| DevOps      | 90%  |
| 前端工程        | 100% |
| 商业授权        | 100% |
| Dashboard   | 100% |
| 数据库监控       | 100% |
| 工单中心        | 100% |
| OpenAPI     | 100% |
| 多租户         | 100% |
| AI能力        | 100% |
| 国密适配        | 100% |

---

# 十一、商业化优先级（MoSCoW）

## Must Have（必须完成）

* License 授权系统
* Dashboard 可视化
* 工单中心
* OpenAPI
* 安装部署文档
* 用户手册
* 运维手册
* 企业官网
* 演示环境
* Docker 一键部署

## Should Have（建议完成）

* 数据库监控
* 数据库巡检
* 国产数据库适配
* 多租户
* Helm 部署
* SDK

## Could Have（可择机实现）

* AI SQL 审核
* AI DBA
* 区块链存证
* 数据血缘分析
* 数据资产管理

## Won't Have（暂不投入）

* 自研数据库
* 自研工作流引擎
* 自研配置中心
* 自研消息队列
* 自研 ORM 框架
* 自研监控平台

---

# 十二、最终产品矩阵规划

| 产品版本         | 定位    | 目标客户            | 实现状态 |
| ------------ | ----- | --------------- | ---- |
| Community    | 开源社区版 | 个人开发者、小团队       | ✅ 已实现 (核心SQL审核与基础流程100%) |
| Professional | 专业版   | 中小企业            | ✅ 已实现 (Dashboard与工单管理100%) |
| Enterprise   | 企业版   | 大中型企业           | ✅ 已实现 (Kubernetes、监控增强等支持100%) |
| Government   | 信创版   | 政府、央企、国企        | ✅ 已实现 (国密与国产化数据库适配100%) |
| SaaS Cloud   | 云平台版  | SaaS 客户         | ✅ 已实现 (多租户架构100%) |
| AI Edition   | 智能版   | 对数据库智能治理有高要求的企业 | 🟡 部分实现 (已完成AI SQL、巡检，但未实现全自动Database AI Agent pipeline及持续学习闭环) |

**总体目标：以 SQL 审核为切入点，逐步演进为覆盖数据库开发、安全、审批、发布、监控、审计和 AI 运维的企业级数据库治理平台，实现可持续商业化运营。**
