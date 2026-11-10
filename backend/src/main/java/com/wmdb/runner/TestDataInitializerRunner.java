package com.wmdb.runner;

import com.baomidou.mybatisplus.core.conditions.query.QueryWrapper;
import com.wmdb.mapper.*;
import com.wmdb.model.*;
import com.wmdb.security.SmUtils;
import lombok.RequiredArgsConstructor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.boot.CommandLineRunner;
import org.springframework.core.annotation.Order;
import org.springframework.stereotype.Component;

import java.util.Date;
import java.util.List;
import java.util.UUID;

/**
 * 全系统真实业务测试数据与多角色测试账号初始化器
 * <p>
 * 保证系统启动时具备完整的真实业务演练数据：
 * 1. 各审批节点测试账号（超级管理员、开发组长初审、核心DBA运维、安全合规审查、普通开发申请人）；
 * 2. 覆盖各业务资源组、各状态生命周期（待初审、待复核、待执行反馈、已执行归档、已驳回）的真实 SQL 工单与回滚方案；
 * 3. 真实执行审计与耗时日志记录。
 * </p>
 *
 * @author wm
 */
@Slf4j
@Component
@Order(10)
@RequiredArgsConstructor
public class TestDataInitializerRunner implements CommandLineRunner {

    private final SysUserMapper sysUserMapper;
    private final SysRoleMapper sysRoleMapper;
    private final ResourceGroupMapper resourceGroupMapper;
    private final SqlTicketMapper sqlTicketMapper;
    private final SqlTicketDetailMapper sqlTicketDetailMapper;
    private final SqlAuditLogMapper sqlAuditLogMapper;
    private final DbInstanceMapper dbInstanceMapper;
    private final WorkflowTemplateMapper workflowTemplateMapper;

    // 默认标准密码 123456 的 BCrypt 哈希
    private static final String DEFAULT_PWD_HASH = "$2a$10$f1pqjyVOHcJWgUGQCmz.B.QXefiemDBivlwLMAZpBNppJdsfM4RuW";

    @Override
    public void run(String... args) {
        log.info("====== 开始初始化全系统真实业务测试数据与审批节点角色账号 ======");
        initRoles();
        initResourceGroups();
        initTestAccounts();
        initTestInstances();
        initRealisticTickets();
        log.info("====== 全系统真实业务测试数据与角色账号初始化完成 ======");
    }

    private void initRoles() {
        ensureRole("ADMIN", "系统超级管理员", "拥有全系统最高审批与全局管理特权");
        ensureRole("DEV_LEAD", "业务开发组长", "负责业务工单初审与资源组权限");
        ensureRole("DBA", "核心数据库管理员", "负责生产数据库变更复核、执行与运维");
        ensureRole("AUDITOR", "安全合规审计员", "负责数据合规审计与慢SQL审查");
        ensureRole("DEV", "开发工程师", "负责工单发起、SQL编写与查询");
    }

    private void ensureRole(String code, String name, String desc) {
        try {
            SysRole exist = sysRoleMapper.selectOne(new QueryWrapper<SysRole>().eq("role_code", code));
            if (exist == null) {
                SysRole r = SysRole.builder()
                        .tenantId("1")
                        .roleCode(code)
                        .roleName(name)
                        .description(desc)
                        .build();
                sysRoleMapper.insert(r);
            }
        } catch (Exception e) {
            log.debug("Role init note: {}", e.getMessage());
        }
    }

    private void initResourceGroups() {
        ensureGroup("车险承保资源组", "财险业务部", "testadmin2", "testadmin3", "主要负责车险承保、核心保单流转库变更");
        ensureGroup("销管系统资源组", "渠道管理部", "leader_sales", "testadmin3", "负责代理人、佣金结算与机构组织树");
        ensureGroup("理赔服务核心组", "理赔运营部", "testadmin2", "dba_master", "负责报案理赔、核赔与定损核心表结构变更");
        ensureGroup("默认核心业务资源组", "基础技术架构组", "testadmin2", "testadmin3", "全域基础核心变更保障组");
        ensureGroup("水险财产险1000条以下", "非车业务部", "leader_sales", "testadmin3", "水险与财产险日常微小变更流式通道");
        ensureGroup("农险理赔资源组", "农险运营部", "testadmin2", "dba_master", "政策性农业保险与理赔结算");
        ensureGroup("风勘中心资源组", "风险管理部", "testadmin2", "testadmin3", "现场风勘与大额标的评估数据");
        ensureGroup("互联网车主服务与理赔快处组", "车主生态部", "testadmin2", "testadmin3", "线上端小程序与车友圈业务库");
    }

    private void ensureGroup(String groupName, String deptName, String devLead, String dbaLead, String desc) {
        try {
            ResourceGroup exist = resourceGroupMapper.selectOne(new QueryWrapper<ResourceGroup>().eq("group_name", groupName));
            if (exist == null) {
                ResourceGroup rg = ResourceGroup.builder()
                        .tenantId("1")
                        .groupName(groupName)
                        .deptName(deptName)
                        .devLead(devLead)
                        .dbaLead(dbaLead)
                        .description(desc)
                        .build();
                resourceGroupMapper.insert(rg);
            }
        } catch (Exception e) {
            log.debug("Group init note: {}", e.getMessage());
        }
    }

    /**
     * 初始化各审批节点配套的专属测试账号 (统一密码 123456，支持多业务资源组归属)
     */
    private void initTestAccounts() {
        // 1. 超级管理员 (所有节点全权审批，归属全平台资源组)
        ensureUser("testadmin1", "王总", "ADMIN", "全平台最高决策组, 车险承保资源组, 销管系统资源组, 理赔服务核心组, 默认核心业务资源组", "13800000001", "310101198501010001", "admin1@wmdb.com");
        ensureUser("admin", "超级管理员", "ADMIN", "全平台最高决策组, 默认核心业务资源组", "13800000000", "0", "admin@wmdb.com");

        // 2. 开发组长初审节点测试账号 (可兼管多个业务线)
        ensureUser("testadmin2", "张伟", "DEV_LEAD", "车险承保资源组, 理赔服务核心组", "13800000002", "310101198802020002", "zhangwei_lead@wmdb.com");
        ensureUser("leader_sales", "陈敏", "DEV_LEAD", "销管系统资源组, 水险财产险1000条以下", "13800000012", "310101198805050012", "chenmin_lead@wmdb.com");

        // 3. 核心 DBA 安全复核与线下执行节点测试账号
        ensureUser("testadmin3", "赵工", "DBA", "生产DBA运维保障组, 默认核心业务资源组", "13800000003", "310101199003030003", "zhaodba@wmdb.com");
        ensureUser("dba_master", "钱工", "DBA", "生产DBA运维保障组, 农险理赔资源组", "13800000013", "310101199006060013", "qiandba@wmdb.com");

        // 4. 数据安全与合规审查节点测试账号
        ensureUser("test_auditor", "李安全", "AUDITOR", "全平台合规审计组, 风勘中心资源组", "13800000004", "310101199204040004", "lisecurity@wmdb.com");

        // 5. 普通开发人员 (工单发起与查询，支持跨项目组)
        ensureUser("test_dev", "张三", "DEV", "车险承保资源组, 销管系统资源组", "13800000005", "310101199505050005", "zhangsan_dev@wmdb.com");
        ensureUser("sales_dev", "李四", "DEV", "销管系统资源组", "13800000006", "310101199606060006", "lisi_dev@wmdb.com");
        ensureUser("claims_dev", "刘五", "DEV", "理赔服务核心组, 互联网车主服务与理赔快处组", "13800000007", "310101199707070007", "liuwu_dev@wmdb.com");
    }

    private void ensureUser(String username, String realName, String role, String resourceGroup, String phone, String idCard, String email) {
        SysUser exist = sysUserMapper.selectOne(new QueryWrapper<SysUser>().eq("username", username));
        if (exist == null) {
            SysUser user = new SysUser();
            user.setTenantId("1");
            user.setUsername(username);
            user.setRealName(realName);
            user.setRole(role);
            user.setResourceGroup(resourceGroup);
            user.setPhone(phone);
            user.setIdCard(idCard);
            user.setEmail(email);
            user.setPasswordHash(DEFAULT_PWD_HASH);
            user.setPasswordCipher(SmUtils.sm3Hash("123456"));
            user.setStatus(1);
            sysUserMapper.insert(user);
            log.info("Initialized test user: {} ({}) with role {}", username, realName, role);
        } else {
            exist.setRealName(realName);
            exist.setRole(role);
            exist.setResourceGroup(resourceGroup);
            exist.setPhone(phone);
            exist.setIdCard(idCard);
            exist.setEmail(email);
            exist.setPasswordHash(DEFAULT_PWD_HASH);
            exist.setPasswordCipher(SmUtils.sm3Hash("123456"));
            exist.setStatus(1);
            sysUserMapper.updateById(exist);
        }
    }

    /**
     * 初始化具有真实业务场景、涵盖各审批流生命周期的仿真工单数据
     */
    private void initRealisticTickets() {
        Long inst1 = 1L; // 阿里云RDS-车险与销管核心生产库
        Long count = sqlTicketMapper.selectCount(new QueryWrapper<>());
        if (count != null && count >= 8) {
            log.info("系统已有 {} 条工单数据，跳过测试工单注入", count);
            return;
        }

        // ==================== 工单 1：待开发组长初审 (DML 变更) ====================
        createTicketIfNotExist(
                1788001001001L,
                "310101199505050005", // 张三 (车险开发)
                inst1,
                "SQL_AUDIT",
                "AUDITING",
                "[目标库: huiqitong_erp] 2026续保核心配置上线 (复制续保清空配置)",
                "-- 1\nINSERT INTO typ_preference (PREFERENCEID, CREATEDATE, UPDATEDATE, DESCRIPTION, actortype, kind, name, value, profile)\nSELECT COALESCE(MAX(PREFERENCEID), 0) + 1, NOW(), NOW(), '复制续保清空配置-18080003分片1', 'System', 'copyClearFieldsConfig', 'copyClearFieldsConfig', 'plcWarrLineSupply:netPlatSendBackFlag=0', '18080003'\nFROM typ_preference\nWHERE PREFERENCEID >= 1 AND PREFERENCEID <= 99999999;",
                "-- 1\n-- 数据回滚补偿语句：恢复 typ_preference 表配置数据\nDELETE FROM typ_preference WHERE name = 'copyClearFieldsConfig' AND profile = '18080003';",
                1,
                null
        );

        // ==================== 工单 2：待 DBA 安全复核与执行 (DDL 提速索引) ====================
        createTicketIfNotExist(
                1788001002002L,
                "310101199606060006", // 李四 (销管开发)
                inst1,
                "SQL_AUDIT",
                "AUDITING",
                "[目标库: huiqitong_erp] 渠道佣金结算查询性能调优 (创建复合加速索引)",
                "-- 0\nCREATE INDEX idx_pref_name_prof ON typ_preference (name, profile, UPDATEDATE);",
                "-- 0\n-- 索引回滚方案：删除新建索引\nDROP INDEX idx_pref_name_prof ON typ_preference;",
                0,
                null
        );

        // ==================== 工单 3：已转由 DBA 专用工具线下执行 (MANUAL_PROCESSING, 待DBA反馈归档) ====================
        createTicketIfNotExist(
                1788001003003L,
                "310101199505050005", // 张三
                inst1,
                "SQL_AUDIT",
                "MANUAL_PROCESSING",
                "[目标库: huiqitong_erp] 历史承保保单大体量数据分批归档与物理表空间优化",
                "-- 100\nUPDATE typ_preference SET DESCRIPTION = '归档优化-完成' WHERE PREFERENCEID BETWEEN 1000 AND 1099;",
                "-- 100\nUPDATE typ_preference SET DESCRIPTION = '历史默认配置' WHERE PREFERENCEID BETWEEN 1000 AND 1099;",
                100,
                "转由 DBA 工具手工线下执行并反馈结果"
        );

        // ==================== 工单 4：已自动化流式执行完毕 (EXECUTED, 归档并记录审计日志) ====================
        SqlTicket t4 = createTicketIfNotExist(
                1788001004004L,
                "310101199707070007", // 刘五 (理赔核心)
                inst1,
                "SQL_AUDIT",
                "EXECUTED",
                "[目标库: huiqitong_erp] 理赔反欺诈黑名单与高风险客户配置字典更新",
                "-- 1\nUPDATE typ_preference SET value = 'antiFraudLevel=HIGH,autoAudit=0' WHERE name = 'copyClearFieldsConfig' AND profile = '18080003';",
                "-- 1\nUPDATE typ_preference SET value = 'antiFraudLevel=NORMAL,autoAudit=1' WHERE name = 'copyClearFieldsConfig' AND profile = '18080003';",
                1,
                "立即自动化流式执行成功 (耗时: 38ms)"
        );
        if (t4 != null) {
            SqlAuditLog log4 = new SqlAuditLog();
            log4.setTicketId(t4.getId());
            log4.setExecuteSql("UPDATE typ_preference SET value = 'antiFraudLevel=HIGH,autoAudit=0' WHERE name = 'copyClearFieldsConfig' AND profile = '18080003';");
            log4.setCostTimeMs(38L);
            log4.setStatus("SUCCESS");
            sqlAuditLogMapper.insert(log4);
        }

        // ==================== 工单 5：合规与敏感数据导出申请 (待合规专员审查) ====================
        createTicketIfNotExist(
                1788001005005L,
                "310101199606060006", // 李四
                inst1,
                "DATA_EXPORT",
                "AUDITING",
                "[目标库: huiqitong_erp] 2026 年中全国渠道代理人佣金结算脱敏明细数据导出",
                "SELECT profile, name, value, UPDATEDATE FROM typ_preference WHERE name LIKE '%Config%' LIMIT 500;",
                "-- 数据导出工单无回滚 SQL 脚本",
                500,
                null
        );

        // ==================== 工单 6：已驳回工单 (REJECTED, 存在高危全表更新风险) ====================
        createTicketIfNotExist(
                1788001006006L,
                "310101199707070007", // 刘五
                inst1,
                "SQL_AUDIT",
                "REJECTED",
                "[目标库: huiqitong_erp] 批量更新系统全表参数状态 (已驳回)",
                "-- 0\nUPDATE typ_preference SET DESCRIPTION = '全表危险更新测试';",
                "-- 0\n-- 无回滚",
                0,
                "[驳回原因: 严禁提交无 WHERE 范围条件的全表更新语句，存在重大生产事故风险！]"
        );
    }

    /**
     * 初始化用于测试环境免审直通与四级混合审批演练的数据库实例
     */
    private void initTestInstances() {
        try {
            // 1. 测试环境免审直通数据库实例
            DbInstance testInst = dbInstanceMapper.selectOne(new QueryWrapper<DbInstance>().eq("name", "敏捷自测与集成测试数据库 (测试环境直通)"));
            if (testInst == null) {
                testInst = new DbInstance();
                testInst.setTenantId("1");
                testInst.setName("敏捷自测与集成测试数据库 (测试环境直通)");
                testInst.setDbType("MYSQL");
                testInst.setHost("127.0.0.1");
                testInst.setPort(3306);
                testInst.setUsername("root");
                testInst.setPasswordCipher(SmUtils.sm4Encrypt("root", "1234567890abcdef1234567890abcdef"));
                testInst.setDatabaseName("huiqitong_erp");
                testInst.setJdbcUrl("jdbc:mysql://127.0.0.1:3306/huiqitong_erp?useUnicode=true&characterEncoding=utf-8&useSSL=false&serverTimezone=Asia/Shanghai&allowMultiQueries=true");
                testInst.setEnv("TEST");
                testInst.setStatus("APPROVED");
                testInst.setResourceGroups("[\"测试系统-测试团队-测试用途\",\"全部业务资源组通用\",\"默认核心业务资源组\"]");
                testInst.setTags("[\"敏捷自测\",\"测试环境\",\"免审直通\"]");
                testInst.setSupportedOps("[\"支持上线\",\"支持查询\",\"支持DML变更\",\"支持DDL结构变更\",\"支持事务预执行\",\"支持数据导出\",\"支持历史回滚\"]");
                testInst.setConnectionTunnel("DIRECT");
                dbInstanceMapper.insert(testInst);
                log.info("Initialized test environment db instance: {}", testInst.getName());
            }

            // 2. 四级审批演练专属数据库实例
            DbInstance hybridInst = dbInstanceMapper.selectOne(new QueryWrapper<DbInstance>().eq("name", "核心业务四级混合审批演练专属库"));
            if (hybridInst == null) {
                WorkflowTemplate tpl4 = workflowTemplateMapper.selectOne(new QueryWrapper<WorkflowTemplate>().eq("template_name", "四级递进混合审批流 (节点一自动审批+后置三级人工)"));
                hybridInst = new DbInstance();
                hybridInst.setTenantId("1");
                hybridInst.setName("核心业务四级混合审批演练专属库");
                hybridInst.setDbType("MYSQL");
                hybridInst.setHost("127.0.0.1");
                hybridInst.setPort(3306);
                hybridInst.setUsername("root");
                hybridInst.setPasswordCipher(SmUtils.sm4Encrypt("root", "1234567890abcdef1234567890abcdef"));
                hybridInst.setDatabaseName("huiqitong_erp");
                hybridInst.setJdbcUrl("jdbc:mysql://127.0.0.1:3306/huiqitong_erp?useUnicode=true&characterEncoding=utf-8&useSSL=false&serverTimezone=Asia/Shanghai&allowMultiQueries=true");
                hybridInst.setEnv("PROD");
                hybridInst.setStatus("APPROVED");
                hybridInst.setResourceGroups("[\"车险承保资源组\",\"销管系统资源组\",\"全部业务资源组通用\",\"默认核心业务资源组\"]");
                hybridInst.setTags("[\"核心业务\",\"四级审批\",\"严格合规\"]");
                hybridInst.setSupportedOps("[\"支持上线\",\"支持查询\",\"支持DML变更\",\"支持DDL结构变更\",\"支持事务预执行\",\"支持数据脱敏\",\"支持历史回滚\"]");
                hybridInst.setConnectionTunnel("DIRECT");
                if (tpl4 != null) {
                    hybridInst.setFixedWorkflowTemplateId(tpl4.getId());
                    hybridInst.setFixedWorkflowTemplateName(tpl4.getTemplateName());
                }
                dbInstanceMapper.insert(hybridInst);
                log.info("Initialized hybrid approval db instance: {}", hybridInst.getName());
            }

            // 确保生产主库未被固定死历史模板，以便优先遵循 BPMN 细化绑定与 Flowable 部署
            DbInstance prodMainInst = dbInstanceMapper.selectOne(new QueryWrapper<DbInstance>().eq("name", "阿里云RDS-车险与销管核心生产库"));
            if (prodMainInst != null && prodMainInst.getFixedWorkflowTemplateId() != null) {
                prodMainInst.setFixedWorkflowTemplateId(null);
                prodMainInst.setFixedWorkflowTemplateName(null);
                dbInstanceMapper.updateById(prodMainInst);
            }
        } catch (Exception e) {
            log.warn("initTestInstances exception: {}", e.getMessage());
        }
    }

    private SqlTicket createTicketIfNotExist(Long ticketId, String applicantIdCard, Long instanceId, String type,
                                             String status, String reason, String sqlText, String rollbackSql,
                                             Integer affectRows, String executionWindow) {
        SqlTicket exist = sqlTicketMapper.selectById(ticketId);
        if (exist != null) {
            return exist;
        }

        SqlTicket ticket = new SqlTicket();
        ticket.setId(ticketId);
        ticket.setBusinessKey(UUID.randomUUID().toString());
        ticket.setApplicantIdCard(applicantIdCard);
        ticket.setInstanceId(instanceId);
        ticket.setType(type);
        ticket.setStatus(status);
        ticket.setReason(reason);
        ticket.setExecutionWindow(executionWindow);
        sqlTicketMapper.insert(ticket);

        SqlTicketDetail detail = new SqlTicketDetail();
        detail.setId(ticketId + 1);
        detail.setTicketId(ticketId);
        detail.setSqlText(sqlText);
        detail.setRollbackSqlText(rollbackSql);
        detail.setAffectRowsEstimate(affectRows != null ? affectRows : 1);
        sqlTicketDetailMapper.insert(detail);

        log.info("Initialized realistic ticket #{}: {}", ticketId, reason);
        return ticket;
    }
}