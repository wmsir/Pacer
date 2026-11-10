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
  process_instance_id varchar(64)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;

CREATE TABLE IF NOT EXISTS sql_ticket_detail (
  id bigint NOT NULL PRIMARY KEY,
  tenant_id varchar(50) NOT NULL,
  ticket_id bigint NOT NULL,
  sql_text text,
  attachment_oss_key varchar(255),
  affect_rows_estimate int
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE IF NOT EXISTS db_instance (
  id bigint NOT NULL AUTO_INCREMENT PRIMARY KEY,
  tenant_id varchar(50) NOT NULL,
  name varchar(100),
  db_type varchar(50),
  jdbc_url varchar(255),
  read_only_jdbc_url varchar(255),
  username varchar(100),
  password_cipher varchar(255),
  env varchar(20),
  status varchar(20) DEFAULT 'APPROVED',
  UNIQUE KEY uk_tenant_name (tenant_id, name)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

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
  rollback_sql text
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE IF NOT EXISTS sys_role (
  id bigint NOT NULL AUTO_INCREMENT PRIMARY KEY,
  tenant_id varchar(50) NOT NULL,
  role_code varchar(50) NOT NULL,
  role_name varchar(100) NOT NULL,
  description varchar(255),
  UNIQUE KEY uk_tenant_role (tenant_id, role_code)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

INSERT IGNORE INTO db_instance (id, tenant_id, name, db_type, jdbc_url, username, password_cipher, env, status)
VALUES (1, '1', '默认RDS测试数据库', 'mysql', 'jdbc:mysql://127.0.0.1:3306/huiqitong_erp?useSSL=false&allowPublicKeyRetrieval=true', 'root', 'CHANGE_ME_IN_PROD', 'PROD', 'APPROVED');

INSERT IGNORE INTO sys_role (id, tenant_id, role_code, role_name, description) VALUES (1, '1', 'ADMIN', '系统管理员', '拥有系统最高权限');
INSERT IGNORE INTO sys_role (id, tenant_id, role_code, role_name, description) VALUES (2, '1', 'DBA', '数据库管理员', '负责数据库运维及审批');
INSERT IGNORE INTO sys_role (id, tenant_id, role_code, role_name, description) VALUES (3, '1', 'DEV', '开发工程师', '负责提交SQL工单');
