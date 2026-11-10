-- ================================================================
-- WMdb 智能云平台 - 数据库全量基础表结构定义 (MySQL / TiDB / OceanBase-MySQL)
-- 适用引擎：MySQL 5.7+ / MySQL 8.0+ / PingCAP TiDB / OceanBase MySQL Mode
-- ================================================================

-- 1. 系统角色定义表
CREATE TABLE IF NOT EXISTS sys_role (
  id bigint NOT NULL AUTO_INCREMENT PRIMARY KEY,
  tenant_id varchar(50) NOT NULL,
  role_code varchar(50) NOT NULL,
  role_name varchar(100) NOT NULL,
  description varchar(255),
  UNIQUE KEY uk_tenant_role (tenant_id, role_code)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- 2. 系统用户实名与鉴权表
CREATE TABLE IF NOT EXISTS sys_user (
  id bigint NOT NULL AUTO_INCREMENT PRIMARY KEY,
  tenant_id varchar(50) NOT NULL,
  username varchar(100) NOT NULL,
  phone varchar(50),
  real_name varchar(100),
  id_card varchar(50),
  email varchar(100),
  resource_group varchar(255),
  password_cipher varchar(255),
  password_hash varchar(255),
  role varchar(100) DEFAULT 'DEV',
  wechat varchar(100),
  work_wechat varchar(100),
  dingtalk varchar(100),
  feishu varchar(100),
  status int DEFAULT 1,
  avatar_url varchar(255),
  created_at datetime DEFAULT CURRENT_TIMESTAMP,
  updated_at datetime DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  UNIQUE KEY uk_tenant_username (tenant_id, username)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- 3. 业务资源组与审批责任人映射表
CREATE TABLE IF NOT EXISTS resource_group (
  id bigint NOT NULL AUTO_INCREMENT PRIMARY KEY,
  tenant_id varchar(50) NOT NULL,
  group_name varchar(100) NOT NULL,
  dept_name varchar(100),
  dev_lead varchar(100),
  dba_lead varchar(100),
  description varchar(255),
  created_at datetime DEFAULT CURRENT_TIMESTAMP,
  updated_at datetime DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  UNIQUE KEY uk_tenant_group_name (tenant_id, group_name)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- 4. 数据库纳管多数据源实例配置表
CREATE TABLE IF NOT EXISTS db_instance (
  id bigint NOT NULL AUTO_INCREMENT PRIMARY KEY,
  tenant_id varchar(50) NOT NULL,
  name varchar(100),
  db_type varchar(50),
  jdbc_url varchar(500),
  read_only_jdbc_url varchar(500),
  username varchar(100),
  password_cipher varchar(255),
  env varchar(20),
  status varchar(20) DEFAULT 'APPROVED',
  UNIQUE KEY uk_tenant_name (tenant_id, name)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- 5. SQL 变更审批工单主表
CREATE TABLE IF NOT EXISTS sql_ticket (
  id bigint NOT NULL AUTO_INCREMENT PRIMARY KEY,
  tenant_id varchar(50) NOT NULL,
  instance_id bigint NOT NULL,
  type varchar(20) NOT NULL,
  reason varchar(255),
  sql_text text,
  file_url varchar(255),
  status varchar(20) NOT NULL,
  submitter_id bigint,
  submitter_id_card varchar(50),
  submit_time datetime,
  process_instance_id varchar(64),
  db_name varchar(100),
  applicant_name varchar(100),
  applicant_id_card varchar(50),
  workflow_template_id bigint,
  workflow_template_name varchar(100),
  execution_mode varchar(50) DEFAULT 'IMMEDIATE',
  scheduled_time datetime,
  batch_size int,
  interval_ms int,
  affect_rows_estimate int,
  actual_affect_rows int,
  create_time datetime DEFAULT CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- 6. SQL 变更工单大文本与附件详情表
CREATE TABLE IF NOT EXISTS sql_ticket_detail (
  id bigint NOT NULL PRIMARY KEY,
  tenant_id varchar(50) NOT NULL,
  ticket_id bigint NOT NULL,
  sql_text text,
  attachment_oss_key varchar(255),
  affect_rows_estimate int,
  actual_affect_rows int
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- 7. 工单操作流水与审批催办审计表
CREATE TABLE IF NOT EXISTS ticket_operation_log (
  id bigint NOT NULL AUTO_INCREMENT PRIMARY KEY,
  ticket_id bigint NOT NULL,
  operator_id_card varchar(50),
  operator_name varchar(100),
  operation_type varchar(50),
  node_name varchar(100),
  comment text,
  tenant_id varchar(50) NOT NULL,
  created_time varchar(50)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- 8. SQL 执行审计与回滚日志表
CREATE TABLE IF NOT EXISTS sql_audit_log (
  id bigint NOT NULL AUTO_INCREMENT PRIMARY KEY,
  tenant_id varchar(50) NOT NULL,
  ticket_id bigint,
  execute_sql text,
  cost_time_ms bigint,
  status varchar(20),
  error_trace text,
  previous_hash varchar(255),
  current_hash varchar(255),
  rollback_sql text,
  created_at datetime DEFAULT CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- 9. 数据库查询审计日志表
CREATE TABLE IF NOT EXISTS query_audit_log (
  id bigint NOT NULL AUTO_INCREMENT PRIMARY KEY,
  tenant_id varchar(50) NOT NULL,
  instance_id bigint,
  db_name varchar(100),
  query_sql text,
  cost_time_ms bigint,
  affect_rows bigint,
  operator_name varchar(100),
  created_time datetime DEFAULT CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- 10. 动态脱敏规则表
CREATE TABLE IF NOT EXISTS data_masking_rule (
  id bigint NOT NULL AUTO_INCREMENT PRIMARY KEY,
  tenant_id varchar(50) NOT NULL,
  rule_name varchar(100),
  column_pattern varchar(100),
  algorithm varchar(50),
  params varchar(255),
  status int DEFAULT 1,
  created_at datetime DEFAULT CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- 11. 动态审批流模板配置表
CREATE TABLE IF NOT EXISTS workflow_template (
  id bigint NOT NULL AUTO_INCREMENT PRIMARY KEY,
  tenant_id varchar(50) NOT NULL,
  template_name varchar(100) NOT NULL,
  flow_type varchar(50),
  resource_groups text,
  target_databases text,
  node_config text,
  condition_expression varchar(255),
  is_pinned int DEFAULT 0,
  description varchar(255),
  created_at datetime DEFAULT CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- 基础初始化种子数据 (MySQL)
INSERT IGNORE INTO sys_role (id, tenant_id, role_code, role_name, description) VALUES (1, '1', 'ADMIN', '系统管理员', '拥有系统最高权限');
INSERT IGNORE INTO sys_role (id, tenant_id, role_code, role_name, description) VALUES (2, '1', 'DBA', '数据库管理员', '负责数据库运维及审批');
INSERT IGNORE INTO sys_role (id, tenant_id, role_code, role_name, description) VALUES (3, '1', 'DEV', '开发工程师', '负责提交SQL工单');
INSERT IGNORE INTO sys_role (id, tenant_id, role_code, role_name, description) VALUES (4, '1', 'DEV_LEAD', '业务开发组长', '负责业务工单初审');
INSERT IGNORE INTO sys_role (id, tenant_id, role_code, role_name, description) VALUES (5, '1', 'AUDITOR', '安全审计员', '负责全平台合规审计');
