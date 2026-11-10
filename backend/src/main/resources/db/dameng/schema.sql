-- ================================================================
-- WMdb 智能云平台 - 国产达梦数据库 (DaMeng DM8 / DM7) 全量表结构
-- 适用引擎：达梦数据库 DM8 / DM7 / DM Enterprise Edition
-- ================================================================

-- 1. 系统角色定义表
CREATE TABLE IF NOT EXISTS sys_role (
  id BIGINT IDENTITY(1, 1) PRIMARY KEY,
  tenant_id VARCHAR(50) NOT NULL,
  role_code VARCHAR(50) NOT NULL,
  role_name VARCHAR(100) NOT NULL,
  description VARCHAR(255),
  CONSTRAINT uk_dm_role UNIQUE (tenant_id, role_code)
);

-- 2. 系统用户实名与鉴权表
CREATE TABLE IF NOT EXISTS sys_user (
  id BIGINT IDENTITY(1, 1) PRIMARY KEY,
  tenant_id VARCHAR(50) NOT NULL,
  username VARCHAR(100) NOT NULL,
  phone VARCHAR(50),
  real_name VARCHAR(100),
  id_card VARCHAR(50),
  email VARCHAR(100),
  resource_group VARCHAR(255),
  password_cipher VARCHAR(255),
  password_hash VARCHAR(255),
  role VARCHAR(100) DEFAULT 'DEV',
  wechat VARCHAR(100),
  work_wechat VARCHAR(100),
  dingtalk VARCHAR(100),
  feishu VARCHAR(100),
  status INT DEFAULT 1,
  avatar_url VARCHAR(255),
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT uk_dm_user UNIQUE (tenant_id, username)
);

-- 3. 业务资源组与审批责任人映射表
CREATE TABLE IF NOT EXISTS resource_group (
  id BIGINT IDENTITY(1, 1) PRIMARY KEY,
  tenant_id VARCHAR(50) NOT NULL,
  group_name VARCHAR(100) NOT NULL,
  dept_name VARCHAR(100),
  dev_lead VARCHAR(100),
  dba_lead VARCHAR(100),
  description VARCHAR(255),
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT uk_dm_group UNIQUE (tenant_id, group_name)
);

-- 4. 数据库纳管多数据源实例配置表
CREATE TABLE IF NOT EXISTS db_instance (
  id BIGINT IDENTITY(1, 1) PRIMARY KEY,
  tenant_id VARCHAR(50) NOT NULL,
  name VARCHAR(100),
  db_type VARCHAR(50),
  jdbc_url VARCHAR(500),
  read_only_jdbc_url VARCHAR(500),
  username VARCHAR(100),
  password_cipher VARCHAR(255),
  env VARCHAR(20),
  status VARCHAR(20) DEFAULT 'APPROVED',
  CONSTRAINT uk_dm_inst UNIQUE (tenant_id, name)
);

-- 5. SQL 变更审批工单主表
CREATE TABLE IF NOT EXISTS sql_ticket (
  id BIGINT IDENTITY(1, 1) PRIMARY KEY,
  tenant_id VARCHAR(50) NOT NULL,
  instance_id BIGINT NOT NULL,
  type VARCHAR(20) NOT NULL,
  reason VARCHAR(255),
  sql_text TEXT,
  file_url VARCHAR(255),
  status VARCHAR(20) NOT NULL,
  submitter_id BIGINT,
  submitter_id_card VARCHAR(50),
  submit_time DATETIME,
  process_instance_id VARCHAR(64),
  db_name VARCHAR(100),
  applicant_name VARCHAR(100),
  applicant_id_card VARCHAR(50),
  workflow_template_id BIGINT,
  workflow_template_name VARCHAR(100),
  execution_mode VARCHAR(50) DEFAULT 'IMMEDIATE',
  scheduled_time DATETIME,
  batch_size INT,
  interval_ms INT,
  affect_rows_estimate INT,
  actual_affect_rows INT,
  create_time DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 6. SQL 变更工单详情表
CREATE TABLE IF NOT EXISTS sql_ticket_detail (
  id BIGINT NOT NULL PRIMARY KEY,
  tenant_id VARCHAR(50) NOT NULL,
  ticket_id BIGINT NOT NULL,
  sql_text TEXT,
  attachment_oss_key VARCHAR(255),
  affect_rows_estimate INT,
  actual_affect_rows INT
);

-- 7. 工单操作流水审计表
CREATE TABLE IF NOT EXISTS ticket_operation_log (
  id BIGINT IDENTITY(1, 1) PRIMARY KEY,
  ticket_id BIGINT NOT NULL,
  operator_id_card VARCHAR(50),
  operator_name VARCHAR(100),
  operation_type VARCHAR(50),
  node_name VARCHAR(100),
  comment TEXT,
  tenant_id VARCHAR(50) NOT NULL,
  created_time VARCHAR(50)
);

-- 8. SQL 执行审计与回滚日志表
CREATE TABLE IF NOT EXISTS sql_audit_log (
  id BIGINT IDENTITY(1, 1) PRIMARY KEY,
  tenant_id VARCHAR(50) NOT NULL,
  ticket_id BIGINT,
  execute_sql TEXT,
  cost_time_ms BIGINT,
  status VARCHAR(20),
  error_trace TEXT,
  previous_hash VARCHAR(255),
  current_hash VARCHAR(255),
  rollback_sql TEXT,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 9. 数据库查询审计日志表
CREATE TABLE IF NOT EXISTS query_audit_log (
  id BIGINT IDENTITY(1, 1) PRIMARY KEY,
  tenant_id VARCHAR(50) NOT NULL,
  instance_id BIGINT,
  db_name VARCHAR(100),
  query_sql TEXT,
  cost_time_ms BIGINT,
  affect_rows BIGINT,
  operator_name VARCHAR(100),
  created_time DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 10. 动态脱敏规则表
CREATE TABLE IF NOT EXISTS data_masking_rule (
  id BIGINT IDENTITY(1, 1) PRIMARY KEY,
  tenant_id VARCHAR(50) NOT NULL,
  rule_name VARCHAR(100),
  column_pattern VARCHAR(100),
  algorithm VARCHAR(50),
  params VARCHAR(255),
  status INT DEFAULT 1,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 11. 动态审批流模板配置表
CREATE TABLE IF NOT EXISTS workflow_template (
  id BIGINT IDENTITY(1, 1) PRIMARY KEY,
  tenant_id VARCHAR(50) NOT NULL,
  template_name VARCHAR(100) NOT NULL,
  flow_type VARCHAR(50),
  resource_groups TEXT,
  target_databases TEXT,
  node_config TEXT,
  condition_expression VARCHAR(255),
  is_pinned INT DEFAULT 0,
  description VARCHAR(255),
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
