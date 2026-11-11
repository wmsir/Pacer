-- =============================================================================
-- Migration: V1.0.0__baseline_indexes.sql
-- Description: 为 WMdb (Pacer) 数据库变更管控与审计平台建立核心高频二级索引基线
-- Backward-compatible: YES (采用 MySQL 8.0 INPLACE 无锁索引创建，幂等检查表与列元数据)
-- Author: FWPT Database Architecture Team
-- Created: 2026-11-11
-- =============================================================================

-- 1. sys_user 二级索引
SET @tbl := (SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = DATABASE() AND table_name = 'sys_user');
SET @idx := (SELECT COUNT(*) FROM information_schema.statistics WHERE table_schema = DATABASE() AND table_name = 'sys_user' AND index_name = 'idx_sys_user_tenant_idcard');
SET @sqlstmt := IF(@tbl > 0 AND @idx = 0, 'ALTER TABLE `sys_user` ADD INDEX `idx_sys_user_tenant_idcard` (`tenant_id`, `id_card`) ALGORITHM=INPLACE, LOCK=NONE', 'SELECT "Skip idx_sys_user_tenant_idcard"');
PREPARE stmt FROM @sqlstmt; EXECUTE stmt; DEALLOCATE PREPARE stmt;

SET @idx := (SELECT COUNT(*) FROM information_schema.statistics WHERE table_schema = DATABASE() AND table_name = 'sys_user' AND index_name = 'idx_sys_user_tenant_phone');
SET @sqlstmt := IF(@tbl > 0 AND @idx = 0, 'ALTER TABLE `sys_user` ADD INDEX `idx_sys_user_tenant_phone` (`tenant_id`, `phone`) ALGORITHM=INPLACE, LOCK=NONE', 'SELECT "Skip idx_sys_user_tenant_phone"');
PREPARE stmt FROM @sqlstmt; EXECUTE stmt; DEALLOCATE PREPARE stmt;

-- 2. db_instance 二级索引
SET @tbl := (SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = DATABASE() AND table_name = 'db_instance');
SET @idx := (SELECT COUNT(*) FROM information_schema.statistics WHERE table_schema = DATABASE() AND table_name = 'db_instance' AND index_name = 'idx_db_inst_tenant_env');
SET @sqlstmt := IF(@tbl > 0 AND @idx = 0, 'ALTER TABLE `db_instance` ADD INDEX `idx_db_inst_tenant_env` (`tenant_id`, `env`, `status`) ALGORITHM=INPLACE, LOCK=NONE', 'SELECT "Skip idx_db_inst_tenant_env"');
PREPARE stmt FROM @sqlstmt; EXECUTE stmt; DEALLOCATE PREPARE stmt;

-- 3. sql_ticket 二级索引
SET @tbl := (SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = DATABASE() AND table_name = 'sql_ticket');
SET @idx := (SELECT COUNT(*) FROM information_schema.statistics WHERE table_schema = DATABASE() AND table_name = 'sql_ticket' AND index_name = 'idx_ticket_tenant_status');
SET @sqlstmt := IF(@tbl > 0 AND @idx = 0, 'ALTER TABLE `sql_ticket` ADD INDEX `idx_ticket_tenant_status` (`tenant_id`, `status`) ALGORITHM=INPLACE, LOCK=NONE', 'SELECT "Skip idx_ticket_tenant_status"');
PREPARE stmt FROM @sqlstmt; EXECUTE stmt; DEALLOCATE PREPARE stmt;

SET @idx := (SELECT COUNT(*) FROM information_schema.statistics WHERE table_schema = DATABASE() AND table_name = 'sql_ticket' AND index_name = 'idx_ticket_instance');
SET @sqlstmt := IF(@tbl > 0 AND @idx = 0, 'ALTER TABLE `sql_ticket` ADD INDEX `idx_ticket_instance` (`instance_id`) ALGORITHM=INPLACE, LOCK=NONE', 'SELECT "Skip idx_ticket_instance"');
PREPARE stmt FROM @sqlstmt; EXECUTE stmt; DEALLOCATE PREPARE stmt;

SET @col := (SELECT COUNT(*) FROM information_schema.columns WHERE table_schema = DATABASE() AND table_name = 'sql_ticket' AND column_name = 'applicant_id_card');
SET @idx := (SELECT COUNT(*) FROM information_schema.statistics WHERE table_schema = DATABASE() AND table_name = 'sql_ticket' AND index_name = 'idx_ticket_applicant');
SET @sqlstmt := IF(@tbl > 0 AND @col > 0 AND @idx = 0, 'ALTER TABLE `sql_ticket` ADD INDEX `idx_ticket_applicant` (`applicant_id_card`) ALGORITHM=INPLACE, LOCK=NONE', 'SELECT "Skip idx_ticket_applicant"');
PREPARE stmt FROM @sqlstmt; EXECUTE stmt; DEALLOCATE PREPARE stmt;

SET @col := (SELECT COUNT(*) FROM information_schema.columns WHERE table_schema = DATABASE() AND table_name = 'sql_ticket' AND column_name = 'submitter_id');
SET @idx := (SELECT COUNT(*) FROM information_schema.statistics WHERE table_schema = DATABASE() AND table_name = 'sql_ticket' AND index_name = 'idx_ticket_submitter');
SET @sqlstmt := IF(@tbl > 0 AND @col > 0 AND @idx = 0, 'ALTER TABLE `sql_ticket` ADD INDEX `idx_ticket_submitter` (`submitter_id`) ALGORITHM=INPLACE, LOCK=NONE', 'SELECT "Skip idx_ticket_submitter"');
PREPARE stmt FROM @sqlstmt; EXECUTE stmt; DEALLOCATE PREPARE stmt;

SET @col := (SELECT COUNT(*) FROM information_schema.columns WHERE table_schema = DATABASE() AND table_name = 'sql_ticket' AND column_name = 'create_time');
SET @idx := (SELECT COUNT(*) FROM information_schema.statistics WHERE table_schema = DATABASE() AND table_name = 'sql_ticket' AND index_name = 'idx_ticket_create_time');
SET @sqlstmt := IF(@tbl > 0 AND @col > 0 AND @idx = 0, 'ALTER TABLE `sql_ticket` ADD INDEX `idx_ticket_create_time` (`create_time`) ALGORITHM=INPLACE, LOCK=NONE', 'SELECT "Skip idx_ticket_create_time"');
PREPARE stmt FROM @sqlstmt; EXECUTE stmt; DEALLOCATE PREPARE stmt;

SET @col := (SELECT COUNT(*) FROM information_schema.columns WHERE table_schema = DATABASE() AND table_name = 'sql_ticket' AND column_name IN ('execution_mode', 'scheduled_time'));
SET @idx := (SELECT COUNT(*) FROM information_schema.statistics WHERE table_schema = DATABASE() AND table_name = 'sql_ticket' AND index_name = 'idx_ticket_sched');
SET @sqlstmt := IF(@tbl > 0 AND @col = 2 AND @idx = 0, 'ALTER TABLE `sql_ticket` ADD INDEX `idx_ticket_sched` (`status`, `execution_mode`, `scheduled_time`) ALGORITHM=INPLACE, LOCK=NONE', 'SELECT "Skip idx_ticket_sched"');
PREPARE stmt FROM @sqlstmt; EXECUTE stmt; DEALLOCATE PREPARE stmt;

-- 4. sql_ticket_detail 二级索引
SET @tbl := (SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = DATABASE() AND table_name = 'sql_ticket_detail');
SET @idx := (SELECT COUNT(*) FROM information_schema.statistics WHERE table_schema = DATABASE() AND table_name = 'sql_ticket_detail' AND index_name = 'idx_ticket_detail_ticket');
SET @sqlstmt := IF(@tbl > 0 AND @idx = 0, 'ALTER TABLE `sql_ticket_detail` ADD INDEX `idx_ticket_detail_ticket` (`ticket_id`) ALGORITHM=INPLACE, LOCK=NONE', 'SELECT "Skip idx_ticket_detail_ticket"');
PREPARE stmt FROM @sqlstmt; EXECUTE stmt; DEALLOCATE PREPARE stmt;

SET @idx := (SELECT COUNT(*) FROM information_schema.statistics WHERE table_schema = DATABASE() AND table_name = 'sql_ticket_detail' AND index_name = 'idx_ticket_detail_tenant');
SET @sqlstmt := IF(@tbl > 0 AND @idx = 0, 'ALTER TABLE `sql_ticket_detail` ADD INDEX `idx_ticket_detail_tenant` (`tenant_id`) ALGORITHM=INPLACE, LOCK=NONE', 'SELECT "Skip idx_ticket_detail_tenant"');
PREPARE stmt FROM @sqlstmt; EXECUTE stmt; DEALLOCATE PREPARE stmt;

-- 5. ticket_operation_log 二级索引
SET @tbl := (SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = DATABASE() AND table_name = 'ticket_operation_log');
SET @idx := (SELECT COUNT(*) FROM information_schema.statistics WHERE table_schema = DATABASE() AND table_name = 'ticket_operation_log' AND index_name = 'idx_ticket_oplog_ticket');
SET @sqlstmt := IF(@tbl > 0 AND @idx = 0, 'ALTER TABLE `ticket_operation_log` ADD INDEX `idx_ticket_oplog_ticket` (`ticket_id`) ALGORITHM=INPLACE, LOCK=NONE', 'SELECT "Skip idx_ticket_oplog_ticket"');
PREPARE stmt FROM @sqlstmt; EXECUTE stmt; DEALLOCATE PREPARE stmt;

SET @idx := (SELECT COUNT(*) FROM information_schema.statistics WHERE table_schema = DATABASE() AND table_name = 'ticket_operation_log' AND index_name = 'idx_ticket_oplog_tenant_time');
SET @sqlstmt := IF(@tbl > 0 AND @idx = 0, 'ALTER TABLE `ticket_operation_log` ADD INDEX `idx_ticket_oplog_tenant_time` (`tenant_id`, `created_time`) ALGORITHM=INPLACE, LOCK=NONE', 'SELECT "Skip idx_ticket_oplog_tenant_time"');
PREPARE stmt FROM @sqlstmt; EXECUTE stmt; DEALLOCATE PREPARE stmt;

-- 6. sql_audit_log 二级索引
SET @tbl := (SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = DATABASE() AND table_name = 'sql_audit_log');
SET @idx := (SELECT COUNT(*) FROM information_schema.statistics WHERE table_schema = DATABASE() AND table_name = 'sql_audit_log' AND index_name = 'idx_sql_audit_ticket');
SET @sqlstmt := IF(@tbl > 0 AND @idx = 0, 'ALTER TABLE `sql_audit_log` ADD INDEX `idx_sql_audit_ticket` (`ticket_id`) ALGORITHM=INPLACE, LOCK=NONE', 'SELECT "Skip idx_sql_audit_ticket"');
PREPARE stmt FROM @sqlstmt; EXECUTE stmt; DEALLOCATE PREPARE stmt;

SET @idx := (SELECT COUNT(*) FROM information_schema.statistics WHERE table_schema = DATABASE() AND table_name = 'sql_audit_log' AND index_name = 'idx_sql_audit_tenant_status');
SET @sqlstmt := IF(@tbl > 0 AND @idx = 0, 'ALTER TABLE `sql_audit_log` ADD INDEX `idx_sql_audit_tenant_status` (`tenant_id`, `status`) ALGORITHM=INPLACE, LOCK=NONE', 'SELECT "Skip idx_sql_audit_tenant_status"');
PREPARE stmt FROM @sqlstmt; EXECUTE stmt; DEALLOCATE PREPARE stmt;

SET @col := (SELECT COUNT(*) FROM information_schema.columns WHERE table_schema = DATABASE() AND table_name = 'sql_audit_log' AND column_name = 'created_at');
SET @idx := (SELECT COUNT(*) FROM information_schema.statistics WHERE table_schema = DATABASE() AND table_name = 'sql_audit_log' AND index_name = 'idx_sql_audit_created');
SET @sqlstmt := IF(@tbl > 0 AND @col > 0 AND @idx = 0, 'ALTER TABLE `sql_audit_log` ADD INDEX `idx_sql_audit_created` (`created_at`) ALGORITHM=INPLACE, LOCK=NONE', 'SELECT "Skip idx_sql_audit_created"');
PREPARE stmt FROM @sqlstmt; EXECUTE stmt; DEALLOCATE PREPARE stmt;

-- 7. query_audit_log 二级索引
SET @tbl := (SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = DATABASE() AND table_name = 'query_audit_log');
SET @idx := (SELECT COUNT(*) FROM information_schema.statistics WHERE table_schema = DATABASE() AND table_name = 'query_audit_log' AND index_name = 'idx_query_audit_tenant_inst');
SET @sqlstmt := IF(@tbl > 0 AND @idx = 0, 'ALTER TABLE `query_audit_log` ADD INDEX `idx_query_audit_tenant_inst` (`tenant_id`, `instance_id`) ALGORITHM=INPLACE, LOCK=NONE', 'SELECT "Skip idx_query_audit_tenant_inst"');
PREPARE stmt FROM @sqlstmt; EXECUTE stmt; DEALLOCATE PREPARE stmt;

SET @idx := (SELECT COUNT(*) FROM information_schema.statistics WHERE table_schema = DATABASE() AND table_name = 'query_audit_log' AND index_name = 'idx_query_audit_created');
SET @sqlstmt := IF(@tbl > 0 AND @idx = 0, 'ALTER TABLE `query_audit_log` ADD INDEX `idx_query_audit_created` (`created_time`) ALGORITHM=INPLACE, LOCK=NONE', 'SELECT "Skip idx_query_audit_created"');
PREPARE stmt FROM @sqlstmt; EXECUTE stmt; DEALLOCATE PREPARE stmt;

-- 8. data_masking_rule & workflow_template 二级索引
SET @tbl := (SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = DATABASE() AND table_name = 'data_masking_rule');
SET @idx := (SELECT COUNT(*) FROM information_schema.statistics WHERE table_schema = DATABASE() AND table_name = 'data_masking_rule' AND index_name = 'idx_mask_rule_tenant_status');
SET @sqlstmt := IF(@tbl > 0 AND @idx = 0, 'ALTER TABLE `data_masking_rule` ADD INDEX `idx_mask_rule_tenant_status` (`tenant_id`, `status`) ALGORITHM=INPLACE, LOCK=NONE', 'SELECT "Skip idx_mask_rule_tenant_status"');
PREPARE stmt FROM @sqlstmt; EXECUTE stmt; DEALLOCATE PREPARE stmt;

SET @tbl := (SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = DATABASE() AND table_name = 'workflow_template');
SET @idx := (SELECT COUNT(*) FROM information_schema.statistics WHERE table_schema = DATABASE() AND table_name = 'workflow_template' AND index_name = 'idx_wf_tpl_tenant_type');
SET @sqlstmt := IF(@tbl > 0 AND @idx = 0, 'ALTER TABLE `workflow_template` ADD INDEX `idx_wf_tpl_tenant_type` (`tenant_id`, `flow_type`) ALGORITHM=INPLACE, LOCK=NONE', 'SELECT "Skip idx_wf_tpl_tenant_type"');
PREPARE stmt FROM @sqlstmt; EXECUTE stmt; DEALLOCATE PREPARE stmt;
