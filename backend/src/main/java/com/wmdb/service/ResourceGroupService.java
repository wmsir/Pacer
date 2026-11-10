package com.wmdb.service;

import com.baomidou.mybatisplus.core.conditions.query.QueryWrapper;
import com.wmdb.exception.BusinessException;
import com.wmdb.mapper.ResourceGroupMapper;
import com.wmdb.model.ResourceGroup;
import jakarta.annotation.PostConstruct;
import lombok.RequiredArgsConstructor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.stereotype.Service;

import javax.sql.DataSource;
import java.sql.Connection;
import java.sql.DatabaseMetaData;
import java.sql.ResultSet;
import java.sql.Statement;
import java.util.Date;
import java.util.HashSet;
import java.util.List;
import java.util.Set;

/**
 * 资源组业务服务
 *
 * @author wm
 */
@Slf4j
@Service
@RequiredArgsConstructor
public class ResourceGroupService {

    private final ResourceGroupMapper resourceGroupMapper;
    private final DataSource dataSource;

    @PostConstruct
    public void initTableAndDefaultData() {
        try (Connection conn = dataSource.getConnection()) {
            DatabaseMetaData md = conn.getMetaData();
            String catalog = conn.getCatalog();
            String schema = conn.getSchema();

            boolean exists = false;
            try (ResultSet rs = md.getTables(catalog, schema, "resource_group", new String[]{"TABLE"})) {
                if (rs.next()) {
                    exists = true;
                }
            }

            try (Statement stmt = conn.createStatement()) {
                if (!exists) {
                    String createTableSql = "CREATE TABLE IF NOT EXISTS resource_group (" +
                            "id BIGINT NOT NULL AUTO_INCREMENT PRIMARY KEY," +
                            "tenant_id VARCHAR(50) NOT NULL," +
                            "group_name VARCHAR(100) NOT NULL," +
                            "dept_name VARCHAR(100)," +
                            "dev_lead VARCHAR(100)," +
                            "dba_lead VARCHAR(100)," +
                            "description VARCHAR(500)," +
                            "workflow_templates TEXT," +
                            "form_config TEXT," +
                            "status INT DEFAULT 1," +
                            "create_time DATETIME," +
                            "UNIQUE KEY uk_tenant_group (tenant_id, group_name)" +
                            ") ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;";
                    stmt.execute(createTableSql);
                    log.info("Created table resource_group successfully.");
                } else {
                    try {
                        stmt.execute("ALTER TABLE resource_group ADD COLUMN workflow_templates TEXT;");
                    } catch (Exception ignored) {}
                    try {
                        stmt.execute("ALTER TABLE resource_group ADD COLUMN form_config TEXT;");
                    } catch (Exception ignored) {}
                }
            }

            // 清理并修复历史乱码数据
            try (Statement stmt = conn.createStatement()) {
                // 删除含有双重编码乱码标志的旧记录
                stmt.execute("DELETE FROM resource_group WHERE group_name LIKE '%Ã%' OR group_name LIKE '%ï%' OR group_name LIKE '%?%';");
            } catch (Exception ignored) {}

            String defaultFormConfig = "[{\"fieldKey\":\"releaseVersion\",\"fieldName\":\"上线发布版本号\",\"fieldType\":\"TEXT\",\"placeholder\":\"请输入本次发布的版本号（例如：V20260822.01）\",\"enabled\":true,\"required\":true},{\"fieldKey\":\"releaseDate\",\"fieldName\":\"上线执行日期 (年月日)\",\"fieldType\":\"DATE\",\"placeholder\":\"请选择计划上线年月日\",\"enabled\":true,\"required\":true},{\"fieldKey\":\"executionTimeRange\",\"fieldName\":\"允许执行时间窗口 (年月日 时分秒)\",\"fieldType\":\"DATETIME_RANGE\",\"placeholder\":\"选择允许变更执行的起止日期时间段\",\"enabled\":true,\"required\":false,\"defaultRange\":[\"2026-08-22 00:00:00\",\"2026-08-23 06:00:00\"]},{\"fieldKey\":\"demandNo\",\"fieldName\":\"关联需求/任务编号\",\"fieldType\":\"TEXT\",\"placeholder\":\"请输入 JIRA / 禅道任务单号（例如：REQ-2026-001）\",\"enabled\":true,\"required\":false}]";

            try (Statement stmt = conn.createStatement()) {
                stmt.execute("UPDATE resource_group SET form_config = '" + defaultFormConfig + "' WHERE form_config LIKE '%\"TIME_RANGE\"%' OR form_config IS NULL OR form_config = '';");
            } catch (Exception ignored) {}

            // 初始化预置干净标准数据
            Long count = resourceGroupMapper.selectCount(new QueryWrapper<>());
            if (count == null || count < 5) {
                List<ResourceGroup> defaults = List.of(
                        ResourceGroup.builder().tenantId("1").groupName("车险承保资源组").deptName("车险事业部").devLead("张伟 (车险开发组长)").dbaLead("赵DBA (核心数据库架构师)").description("负责车险核心交易、保单批改与渠道对接业务").workflowTemplates("[\"标准生产 SQL 变更审批流\",\"高危 DDL 结构变更双人复核流程\"]").formConfig(defaultFormConfig).status(1).createTime(new Date()).build(),
                        ResourceGroup.builder().tenantId("1").groupName("销管系统资源组").deptName("渠道销售部").devLead("陈敏 (销管开发组长)").dbaLead("赵DBA (核心数据库架构师)").description("负责代理人佣金结算、渠道层级与保单销管业务").workflowTemplates("[\"标准生产 SQL 变更审批流\",\"高危 DDL 结构变更双人复核流程\"]").formConfig(defaultFormConfig).status(1).createTime(new Date()).build(),
                        ResourceGroup.builder().tenantId("1").groupName("理赔服务核心组").deptName("理赔运营中心").devLead("刘五 (理赔核心开发)").dbaLead("赵DBA (核心数据库架构师)").description("车险与人身险快速定损、智能核赔与反欺诈结算业务").workflowTemplates("[\"标准生产 SQL 变更审批流\"]").formConfig(defaultFormConfig).status(1).createTime(new Date()).build(),
                        ResourceGroup.builder().tenantId("1").groupName("水险财产险1000条以下").deptName("财险事业部").devLead("王组长").dbaLead("赵DBA (核心数据库架构师)").description("水险、财产险小批量常规数据变更与定期调账").workflowTemplates("[\"标准生产 SQL 变更审批流\"]").formConfig(defaultFormConfig).status(1).createTime(new Date()).build(),
                        ResourceGroup.builder().tenantId("1").groupName("农险理赔资源组").deptName("农业保险部").devLead("孙组长").dbaLead("钱DBA (资深运维专家)").description("涉农政策性保险理赔与农户直赔数据流转").workflowTemplates("[\"高危 DDL 结构变更双人复核流程\"]").formConfig(defaultFormConfig).status(1).createTime(new Date()).build(),
                        ResourceGroup.builder().tenantId("1").groupName("风勘中心资源组").deptName("风控精算部").devLead("李安全 (数据合规官)").dbaLead("钱DBA (资深运维专家)").description("风险勘测中台、防灾防损及敏感数据分析").workflowTemplates("[\"敏感数据导出与脱敏审批流\"]").formConfig(defaultFormConfig).status(1).createTime(new Date()).build(),
                        ResourceGroup.builder().tenantId("1").groupName("互联网车主服务与理赔快处组").deptName("数字科技事业部").devLead("周组长").dbaLead("核心DBA").description("车主服务App、线上快速理赔与互联网生态对接").workflowTemplates("[\"标准生产 SQL 变更审批流\",\"生产紧急变更极速放行通道\"]").formConfig(defaultFormConfig).status(1).createTime(new Date()).build(),
                        ResourceGroup.builder().tenantId("1").groupName("默认核心业务资源组").deptName("基础架构部").devLead("技术负责人").dbaLead("赵DBA (核心数据库架构师)").description("平台默认兜底通用业务资源组与公共基础架构").workflowTemplates("[\"标准生产 SQL 变更审批流\",\"生产紧急变更极速放行通道\"]").formConfig(defaultFormConfig).status(1).createTime(new Date()).build()
                );
                for (ResourceGroup rg : defaults) {
                    try {
                        ResourceGroup exist = resourceGroupMapper.selectOne(new QueryWrapper<ResourceGroup>().eq("group_name", rg.getGroupName()));
                        if (exist == null) {
                            resourceGroupMapper.insert(rg);
                        } else {
                            rg.setId(exist.getId());
                            if (exist.getFormConfig() == null || exist.getFormConfig().isEmpty()) {
                                rg.setFormConfig(defaultFormConfig);
                            }
                            resourceGroupMapper.updateById(rg);
                        }
                    } catch (Exception ignored) {}
                }
                log.info("Initialized default clean resource groups successfully.");
            }
        } catch (Exception e) {
            log.warn("Init resource_group table exception: {}", e.getMessage());
        }
    }

    public List<ResourceGroup> listResourceGroups(String keyword) {
        QueryWrapper<ResourceGroup> qw = new QueryWrapper<>();
        if (keyword != null && !keyword.trim().isEmpty()) {
            qw.like("group_name", keyword.trim())
                    .or()
                    .like("dept_name", keyword.trim())
                    .or()
                    .like("dev_lead", keyword.trim());
        }
        qw.orderByDesc("id");
        return resourceGroupMapper.selectList(qw);
    }

    public com.wmdb.model.PageResultDTO<ResourceGroup> pageResourceGroups(int page, int size, String keyword) {
        QueryWrapper<ResourceGroup> qw = new QueryWrapper<>();
        if (keyword != null && !keyword.trim().isEmpty()) {
            String kw = keyword.trim();
            qw.and(w -> w.like("group_name", kw)
                    .or().like("dept_name", kw)
                    .or().like("dev_lead", kw)
                    .or().like("dba_lead", kw)
                    .or().like("description", kw));
        }
        qw.orderByDesc("id");
        com.baomidou.mybatisplus.extension.plugins.pagination.Page<ResourceGroup> mpPage =
                new com.baomidou.mybatisplus.extension.plugins.pagination.Page<>(page > 0 ? page : 1, size > 0 ? size : 10);
        resourceGroupMapper.selectPage(mpPage, qw);
        return com.wmdb.model.PageResultDTO.from(mpPage);
    }

    public void saveResourceGroup(ResourceGroup group) {
        if (group.getGroupName() == null || group.getGroupName().trim().isEmpty()) {
            throw new BusinessException("A0400", "资源组名称不能为空");
        }
        if (group.getTenantId() == null || group.getTenantId().isEmpty()) {
            group.setTenantId("1");
        }
        if (group.getStatus() == null) {
            group.setStatus(1);
        }

        if (group.getId() == null) {
            group.setCreateTime(new Date());
            resourceGroupMapper.insert(group);
        } else {
            resourceGroupMapper.updateById(group);
        }
    }

    public void deleteResourceGroup(Long id) {
        resourceGroupMapper.deleteById(id);
    }
}
