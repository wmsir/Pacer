package com.wmdb.service;

import com.baomidou.mybatisplus.core.conditions.query.QueryWrapper;
import com.fasterxml.jackson.core.type.TypeReference;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.wmdb.exception.BusinessException;
import com.wmdb.mapper.DbInstanceMapper;
import com.wmdb.mapper.WorkflowTemplateMapper;
import com.wmdb.model.DbInstance;
import com.wmdb.model.RoutingPreviewDTO;
import com.wmdb.model.RoutingPreviewRequestDTO;
import com.wmdb.model.WorkflowTemplate;
import jakarta.annotation.PostConstruct;
import lombok.RequiredArgsConstructor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.stereotype.Service;

import javax.sql.DataSource;
import java.sql.Connection;
import java.sql.DatabaseMetaData;
import java.sql.ResultSet;
import java.sql.Statement;
import java.util.*;

/**
 * 审批流模板业务服务
 *
 * @author wm
 */
@Slf4j
@Service
@RequiredArgsConstructor
public class WorkflowTemplateService {

    private final WorkflowTemplateMapper workflowTemplateMapper;
    private final DbInstanceMapper dbInstanceMapper;
    private final com.wmdb.mapper.ResourceGroupMapper resourceGroupMapper;
    private final WorkflowService workflowService;
    private final DataSource dataSource;
    private final ObjectMapper objectMapper = new ObjectMapper();

    @PostConstruct
    public void initTableAndDefaults() {
        try (Connection conn = dataSource.getConnection()) {
            DatabaseMetaData md = conn.getMetaData();
            String catalog = conn.getCatalog();
            String schema = conn.getSchema();

            boolean exists = false;
            try (ResultSet rs = md.getTables(catalog, schema, "workflow_template", new String[]{"TABLE"})) {
                if (rs.next()) {
                    exists = true;
                }
            }

            try (Statement stmt = conn.createStatement()) {
                if (!exists) {
                    String createSql = "CREATE TABLE IF NOT EXISTS workflow_template (" +
                            "id BIGINT NOT NULL AUTO_INCREMENT PRIMARY KEY," +
                            "tenant_id VARCHAR(50) NOT NULL DEFAULT '1'," +
                            "template_name VARCHAR(100) NOT NULL," +
                            "flow_type VARCHAR(50) NOT NULL," +
                            "resource_groups TEXT," +
                            "node_config TEXT," +
                            "condition_dimension VARCHAR(50) DEFAULT 'AFFECT_ROWS'," +
                            "affect_rows_threshold INT DEFAULT 1000," +
                            "high_risk_role VARCHAR(50) DEFAULT 'DBA'," +
                            "low_risk_role VARCHAR(50) DEFAULT 'DEV_LEAD'," +
                            "spel_expression VARCHAR(255) DEFAULT '#{affectRows > 1000}'," +
                            "trigger_condition VARCHAR(255)," +
                            "default_execution_mode VARCHAR(255) DEFAULT '[\"IMMEDIATE\"]'," +
                            "status INT NOT NULL DEFAULT 1," +
                            "description VARCHAR(500)," +
                            "create_time DATETIME," +
                            "update_time DATETIME" +
                            ") ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;";
                    stmt.execute(createSql);
                    log.info("Created table workflow_template successfully.");
                } else {
                    stmt.execute("ALTER TABLE workflow_template MODIFY COLUMN default_execution_mode VARCHAR(255);");
                    try {
                        stmt.execute("ALTER TABLE workflow_template ADD COLUMN condition_dimension VARCHAR(50) DEFAULT 'AFFECT_ROWS';");
                    } catch (Exception ignored) {}
                    try {
                        stmt.execute("ALTER TABLE workflow_template ADD COLUMN affect_rows_threshold INT DEFAULT 1000;");
                    } catch (Exception ignored) {}
                    try {
                        stmt.execute("ALTER TABLE workflow_template ADD COLUMN high_risk_role VARCHAR(50) DEFAULT 'DBA';");
                    } catch (Exception ignored) {}
                    try {
                        stmt.execute("ALTER TABLE workflow_template ADD COLUMN low_risk_role VARCHAR(50) DEFAULT 'DEV_LEAD';");
                    } catch (Exception ignored) {}
                    try {
                        stmt.execute("ALTER TABLE workflow_template ADD COLUMN spel_expression VARCHAR(255) DEFAULT '#{affectRows > 1000}';");
                    } catch (Exception ignored) {}
                    try {
                        stmt.execute("ALTER TABLE workflow_template ADD COLUMN target_databases TEXT;");
                    } catch (Exception ignored) {}
                }
            }

            Date now = new Date();
            List<WorkflowTemplate> defaults = List.of(
                    WorkflowTemplate.builder()
                            .tenantId("1")
                            .templateName("DML 影响行数智能条件分支审批流")
                            .flowType("DML_CHANGE")
                            .resourceGroups("[\"车险承保资源组\",\"销管系统资源组\",\"水险财产险1000条以下\",\"默认核心业务资源组\",\"全部业务资源组通用\"]")
                            .nodeConfig("[{\"step\":1,\"nodeName\":\"影响行数智能排他网关判定\",\"role\":\"GATEWAY\",\"condition\":\"affect_rows > 1000 ? DBA : OPS\",\"branches\":[{\"condition\":\"影响行数 > 1000\",\"targetNode\":\"核心DBA安全复核\",\"role\":\"DBA\"},{\"condition\":\"影响行数 <= 1000\",\"targetNode\":\"运维/开发组长初审\",\"role\":\"DEV_LEAD\"}]},{\"step\":2,\"nodeName\":\"JDBC流式安全执行\",\"role\":\"SERVICE\"}]")
                            .triggerCondition("影响行数 > 1000 需核心 DBA 审核；影响行数 ≤ 1000 由运维/开发组长审核")
                            .defaultExecutionMode("[\"IMMEDIATE\",\"SCHEDULED\",\"CANARY_BATCH\"]")
                            .affectRowsThreshold(1000)
                            .conditionDimension("AFFECT_ROWS")
                            .highRiskRole("DBA")
                            .lowRiskRole("DEV_LEAD")
                            .spelExpression("#{affectRows > 1000 || hasDdl == true}")
                            .status(1)
                            .description("根据 SQL 预执行检测的影响行数自动触发智能排他网关分流：大于 1000 行由核心 DBA 终审，小于等于 1000 行由运维/开发组长初审")
                            .createTime(now)
                            .updateTime(now)
                            .build(),
                    WorkflowTemplate.builder()
                            .tenantId("1")
                            .templateName("高危 DDL 结构变更三级严格审批流")
                            .flowType("DDL_CHANGE")
                            .resourceGroups("[\"车险承保资源组\",\"销管系统资源组\",\"农险理赔资源组\",\"核心账务资源组\",\"全部业务资源组通用\"]")
                            .nodeConfig("[{\"step\":1,\"nodeName\":\"开发组长初审\",\"role\":\"DEV_LEAD\"},{\"step\":2,\"nodeName\":\"核心DBA技术复审\",\"role\":\"DBA\"},{\"step\":3,\"nodeName\":\"系统管理员终审\",\"role\":\"ADMIN\"}]")
                            .triggerCondition("包含 CREATE TABLE / ALTER TABLE / DROP 等结构定义变更")
                            .defaultExecutionMode("[\"SCHEDULED\",\"MANUAL_DBA\"]")
                            .status(1)
                            .description("针对可能引起锁表或高危风险的 DDL 操作进行三级强管控，需开发组长、核心 DBA、管理员逐级终审")
                            .createTime(now)
                            .updateTime(now)
                            .build(),
                    WorkflowTemplate.builder()
                            .tenantId("1")
                            .templateName("四级递进混合审批流 (节点一自动审批+后置三级人工)")
                            .flowType("SQL_AUDIT")
                            .resourceGroups("[\"车险承保资源组\",\"销管系统资源组\",\"农险理赔资源组\",\"默认核心业务资源组\",\"全部业务资源组通用\"]")
                            .nodeConfig("[{\"step\":1,\"nodeName\":\"SQL语法与安全预检网关 (系统自动审批)\",\"role\":\"SYSTEM\",\"condition\":\"auto_pass\"},{\"step\":2,\"nodeName\":\"业务开发组长初审\",\"role\":\"DEV_LEAD\"},{\"step\":3,\"nodeName\":\"核心DBA安全复核\",\"role\":\"DBA\"},{\"step\":4,\"nodeName\":\"运维安全总监终审\",\"role\":\"ADMIN\"}]")
                            .triggerCondition("第 1 节点由系统预检网关自动审批放行，第 2/3/4 节点依次由开发组长、核心 DBA、运维总监人工逐级审核")
                            .defaultExecutionMode("[\"IMMEDIATE\",\"SCHEDULED\",\"CANARY_BATCH\"]")
                            .status(1)
                            .description("包含 4 级递进审核：节点一由系统预检网关智能自动放行，节点二/三/四需人工逐级复核，适用于混合模式测试与多级管控")
                            .createTime(now)
                            .updateTime(now)
                            .build(),
                    WorkflowTemplate.builder()
                            .tenantId("1")
                            .templateName("测试环境全自动直通免审审批流")
                            .flowType("ALL")
                            .resourceGroups("[\"测试系统-测试团队-测试用途\",\"全部业务资源组通用\",\"默认核心业务资源组\"]")
                            .nodeConfig("[{\"step\":1,\"nodeName\":\"测试环境SQL预校验 (系统自动审批)\",\"role\":\"SYSTEM\",\"condition\":\"auto_pass\"},{\"step\":2,\"nodeName\":\"测试环境免审直通放行 (系统自动审批)\",\"role\":\"SYSTEM\",\"condition\":\"auto_pass\"},{\"step\":3,\"nodeName\":\"JDBC流式执行与防篡改归档\",\"role\":\"SERVICE\"}]")
                            .triggerCondition("测试与开发环境专属：提交预校验通过后全自动免审放行并立即触发流式执行与归档")
                            .defaultExecutionMode("[\"IMMEDIATE\"]")
                            .status(1)
                            .description("专为测试与研发自测环境设计的极速审批流：预校验通过后系统全自动放行并直接执行，零人工等待")
                            .createTime(now)
                            .updateTime(now)
                            .build()
            );

            // 清理删除所有不再使用的多余历史模板，仅保留这 4 个正在使用的模板
            List<String> retainedNames = defaults.stream().map(WorkflowTemplate::getTemplateName).toList();
            try {
                workflowTemplateMapper.delete(new QueryWrapper<WorkflowTemplate>().notIn("template_name", retainedNames));
                log.info("Cleaned obsolete workflow templates, retaining 4 active templates: {}", retainedNames);
            } catch (Exception e) {
                log.warn("Clean obsolete workflow templates exception: {}", e.getMessage());
            }

            for (WorkflowTemplate tpl : defaults) {
                WorkflowTemplate exist = workflowTemplateMapper.selectOne(new QueryWrapper<WorkflowTemplate>().eq("template_name", tpl.getTemplateName()));
                if (exist == null) {
                    try {
                        workflowTemplateMapper.insert(tpl);
                        log.info("Inserted default workflow template: {}", tpl.getTemplateName());
                    } catch (Exception ignored) {}
                } else {
                    exist.setFlowType(tpl.getFlowType());
                    exist.setNodeConfig(tpl.getNodeConfig());
                    exist.setTriggerCondition(tpl.getTriggerCondition());
                    exist.setDefaultExecutionMode(tpl.getDefaultExecutionMode());
                    exist.setResourceGroups(tpl.getResourceGroups());
                    exist.setDescription(tpl.getDescription());
                    exist.setStatus(1);
                    exist.setAffectRowsThreshold(tpl.getAffectRowsThreshold());
                    exist.setConditionDimension(tpl.getConditionDimension());
                    exist.setHighRiskRole(tpl.getHighRiskRole());
                    exist.setLowRiskRole(tpl.getLowRiskRole());
                    exist.setSpelExpression(tpl.getSpelExpression());
                    workflowTemplateMapper.updateById(exist);
                }
            }
            log.info("Initialized exactly 4 active default workflow templates successfully.");
        } catch (Exception e) {
            log.warn("Init workflow_template table exception: {}", e.getMessage());
        }
    }

    public List<WorkflowTemplate> listTemplates(String keyword) {
        QueryWrapper<WorkflowTemplate> qw = new QueryWrapper<>();
        if (keyword != null && !keyword.trim().isEmpty()) {
            qw.like("template_name", keyword.trim())
                    .or()
                    .like("flow_type", keyword.trim())
                    .or()
                    .like("resource_groups", keyword.trim());
        }
        qw.orderByDesc("id");
        return workflowTemplateMapper.selectList(qw);
    }

    public com.wmdb.model.PageResultDTO<WorkflowTemplate> pageTemplates(int page, int size, String keyword, String flowType) {
        QueryWrapper<WorkflowTemplate> qw = new QueryWrapper<>();
        if (keyword != null && !keyword.trim().isEmpty()) {
            String kw = keyword.trim();
            qw.and(w -> w.like("template_name", kw)
                    .or().like("flow_type", kw)
                    .or().like("resource_groups", kw)
                    .or().like("description", kw));
        }
        if (flowType != null && !flowType.trim().isEmpty()) {
            qw.eq("flow_type", flowType.trim());
        }
        qw.orderByDesc("id");
        com.baomidou.mybatisplus.extension.plugins.pagination.Page<WorkflowTemplate> mpPage =
                new com.baomidou.mybatisplus.extension.plugins.pagination.Page<>(page > 0 ? page : 1, size > 0 ? size : 10);
        workflowTemplateMapper.selectPage(mpPage, qw);
        return com.wmdb.model.PageResultDTO.from(mpPage);
    }

    public void saveTemplate(WorkflowTemplate template) {
        if (template.getTemplateName() == null || template.getTemplateName().trim().isEmpty()) {
            throw new BusinessException("A0400", "流程模板名称不能为空");
        }
        if (template.getTenantId() == null || template.getTenantId().isEmpty()) {
            template.setTenantId("1");
        }
        if (template.getStatus() == null) {
            template.setStatus(1);
        }
        if (template.getDefaultExecutionMode() == null) {
            template.setDefaultExecutionMode("IMMEDIATE");
        }
        if (template.getAffectRowsThreshold() == null || template.getAffectRowsThreshold() <= 0) {
            template.setAffectRowsThreshold(1000);
        }
        if (template.getConditionDimension() == null || template.getConditionDimension().isEmpty()) {
            template.setConditionDimension("AFFECT_ROWS");
        }
        if (template.getHighRiskRole() == null || template.getHighRiskRole().isEmpty()) {
            template.setHighRiskRole("DBA");
        }
        if (template.getLowRiskRole() == null || template.getLowRiskRole().isEmpty()) {
            template.setLowRiskRole("DEV_LEAD");
        }
        if (template.getSpelExpression() == null || template.getSpelExpression().trim().isEmpty()) {
            int th = template.getAffectRowsThreshold() != null ? template.getAffectRowsThreshold() : 1000;
            String dim = template.getConditionDimension() != null ? template.getConditionDimension() : "AFFECT_ROWS";
            if ("CHANGE_TYPE".equalsIgnoreCase(dim)) {
                template.setSpelExpression("#{hasDdl == true}");
            } else if ("COMPOSITE".equalsIgnoreCase(dim)) {
                template.setSpelExpression("#{affectRows > " + th + " || hasDdl == true}");
            } else {
                template.setSpelExpression("#{affectRows > " + th + "}");
            }
        }

        Date now = new Date();
        template.setUpdateTime(now);

        if (template.getId() == null) {
            template.setCreateTime(now);
            workflowTemplateMapper.insert(template);
        } else {
            workflowTemplateMapper.updateById(template);
        }
    }

    public void deleteTemplate(Long id) {
        workflowTemplateMapper.deleteById(id);
    }

    public void toggleStatus(Long id) {
        WorkflowTemplate template = workflowTemplateMapper.selectById(id);
        if (template != null) {
            template.setStatus(template.getStatus() != null && template.getStatus() == 1 ? 0 : 1);
            template.setUpdateTime(new Date());
            workflowTemplateMapper.updateById(template);
        }
    }

    /**
     * 根据工单创建参数预估路由唯一的审批流模板与节点链路
     */
    public RoutingPreviewDTO previewRouting(RoutingPreviewRequestDTO request) {
        if (request == null) {
            request = new RoutingPreviewRequestDTO();
        }

        DbInstance instance = request.getInstanceId() != null ? dbInstanceMapper.selectById(request.getInstanceId()) : null;
        String resourceGroup = request.getResourceGroup() != null ? request.getResourceGroup().trim() : "";
        String dbName = request.getDbName() != null ? request.getDbName().trim() : "";
        String ticketType = request.getTicketType() != null ? request.getTicketType().trim() : "SQL_AUDIT";
        int expectedRows = request.getExpectedRows() != null ? request.getExpectedRows() : 1;

        // 解析资源组的安全策略与专库审批流映射
        boolean enforceDryRun = true;
        boolean enableStep3Rollback = true;
        boolean enableStep4DryRun = true;
        String customDbWorkflowName = null;

        if (!resourceGroup.isEmpty()) {
            List<com.wmdb.model.ResourceGroup> rgList = resourceGroupMapper.selectList(
                    new QueryWrapper<com.wmdb.model.ResourceGroup>().eq("group_name", resourceGroup).last("LIMIT 1")
            );
            if (rgList != null && !rgList.isEmpty()) {
                com.wmdb.model.ResourceGroup rg = rgList.get(0);
                if (rg.getFormConfig() != null && !rg.getFormConfig().trim().isEmpty()) {
                    try {
                        com.fasterxml.jackson.databind.JsonNode rootNode = objectMapper.readTree(rg.getFormConfig());
                        if (rootNode.has("enforceDryRun")) {
                            enforceDryRun = rootNode.get("enforceDryRun").asBoolean(true);
                        }
                        if (rootNode.has("enableStep3Rollback")) {
                            enableStep3Rollback = rootNode.get("enableStep3Rollback").asBoolean(true);
                        }
                        if (rootNode.has("enableStep4DryRun")) {
                            enableStep4DryRun = rootNode.get("enableStep4DryRun").asBoolean(true);
                        }
                        if (rootNode.has("dbWorkflowMappings") && rootNode.get("dbWorkflowMappings").isObject() && !dbName.isEmpty()) {
                            com.fasterxml.jackson.databind.JsonNode mappings = rootNode.get("dbWorkflowMappings");
                            String exactKey = (request.getInstanceId() != null ? request.getInstanceId() : "") + ":" + dbName;
                            if (mappings.has(exactKey) && !mappings.get(exactKey).asText().trim().isEmpty()) {
                                customDbWorkflowName = mappings.get(exactKey).asText().trim();
                            } else if (mappings.has(dbName) && !mappings.get(dbName).asText().trim().isEmpty()) {
                                customDbWorkflowName = mappings.get(dbName).asText().trim();
                            }
                        }
                    } catch (Exception ignored) {}
                }
            }
        }

        // 0. 优先判定：测试/开发环境免审批直通执行（测试环境无需多级审批流转，预校验通过后直接自动执行）
        boolean isTestOrDev = instance != null && instance.getEnv() != null
                && ("TEST".equalsIgnoreCase(instance.getEnv()) || "DEV".equalsIgnoreCase(instance.getEnv()) || "UAT".equalsIgnoreCase(instance.getEnv()) || "SIT".equalsIgnoreCase(instance.getEnv()) || "LOCAL".equalsIgnoreCase(instance.getEnv()));

        if (isTestOrDev) {
            String envName = instance.getEnv().toUpperCase();
            RoutingPreviewDTO testDto = RoutingPreviewDTO.builder()
                    .templateId(999L)
                    .templateName("测试/开发环境免审批直通执行流")
                    .isPinned(true)
                    .routingReason("检测到目标数据库实例【" + instance.getName() + "】所属环境为【" + envName + "】测试环境，系统开启免审批直通模式，提交预校验通过后直接自动执行上线。")
                    .triggerCondition("测试环境 (env == '" + envName + "') 免审批直接执行")
                    .flowType(ticketType)
                    .highRiskRole("系统免审直通")
                    .lowRiskRole("系统免审直通")
                    .affectRowsThreshold(999999)
                    .spelExpression("#{env == 'TEST' || env == 'DEV'}")
                    .conditionDimension("ENV")
                    .enforceDryRun(enforceDryRun)
                    .enableStep3Rollback(enableStep3Rollback)
                    .enableStep4DryRun(enableStep4DryRun)
                    .nodes(List.of(
                            RoutingPreviewDTO.PreviewNodeDTO.builder().step(1).nodeName("SQL 语法与影响行数预校验").approverRole("系统预检引擎").eligibleApprovers(List.of("静态语法与事务预检")).build(),
                            RoutingPreviewDTO.PreviewNodeDTO.builder().step(2).nodeName("测试环境免审批自动放行").approverRole("系统自动放行网关").eligibleApprovers(List.of("测试环境免审直通")).build(),
                            RoutingPreviewDTO.PreviewNodeDTO.builder().step(3).nodeName("JDBC 流式安全执行并归档").approverRole("异步流式执行引擎").eligibleApprovers(List.of("执行归档引擎")).build()
                    ))
                    .build();
            return testDto;
        }

        List<WorkflowTemplate> allTemplates = workflowTemplateMapper.selectList(
                new QueryWrapper<WorkflowTemplate>().eq("status", 1).orderByDesc("id")
        );

        // 1. 优先判定：BPMN 设计器细化绑定的生效目标数据库/实例 (Custom Target Databases + 工单类型)
        for (WorkflowTemplate tpl : allTemplates) {
            if (tpl.getTargetDatabases() != null && !tpl.getTargetDatabases().isEmpty()
                    && !tpl.getTargetDatabases().equals("[\"ALL\"]") && !tpl.getTargetDatabases().equals("[]")) {
                String td = tpl.getTargetDatabases();
                String fullDbKey = (instance != null ? instance.getName() : "") + "/" + dbName;
                boolean dbMatched = (dbName != null && !dbName.isEmpty() && td.contains(dbName)) ||
                        (instance != null && instance.getName() != null && td.contains(instance.getName())) ||
                        td.contains(fullDbKey);
                boolean typeMatched = isFlowTypeMatched(tpl.getFlowType(), ticketType);
                if (dbMatched && typeMatched) {
                    String reason = "🎯 命中 BPMN 细化绑定生效范围：业务资源组【" + (resourceGroup.isEmpty() ? "全部资源组" : resourceGroup) + "】+ 工单类型【" + formatTicketTypeChinese(tpl.getFlowType()) + "】+ 目标库【" + (dbName != null ? dbName : instance.getName()) + "】";
                    RoutingPreviewDTO dto = buildRoutingPreviewDTO(tpl, true, reason, expectedRows, ticketType, request.getSqlSnippet());
                    dto.setEnforceDryRun(enforceDryRun);
                    dto.setEnableStep3Rollback(enableStep3Rollback);
                    dto.setEnableStep4DryRun(enableStep4DryRun);
                    return dto;
                }
            }
        }

        // 2. 次优先判定：BPMN 绑定的生效业务资源组 + 生效工单类型
        for (WorkflowTemplate tpl : allTemplates) {
            String rgs = tpl.getResourceGroups() != null ? tpl.getResourceGroups() : "";
            boolean isSpecificRg = !rgs.isEmpty() && !rgs.contains("全部业务资源组通用") && !rgs.contains("默认核心业务资源组");
            boolean rgMatched = !resourceGroup.isEmpty() && rgs.contains(resourceGroup);
            boolean typeMatched = isFlowTypeMatched(tpl.getFlowType(), ticketType);
            if (isSpecificRg && rgMatched && typeMatched) {
                String reason = "🎯 命中绑定生效范围：业务资源组【" + resourceGroup + "】+ 生效工单类型【" + formatTicketTypeChinese(tpl.getFlowType()) + "】";
                RoutingPreviewDTO dto = buildRoutingPreviewDTO(tpl, true, reason, expectedRows, ticketType, request.getSqlSnippet());
                dto.setEnforceDryRun(enforceDryRun);
                dto.setEnableStep3Rollback(enableStep3Rollback);
                dto.setEnableStep4DryRun(enableStep4DryRun);
                return dto;
            }
        }

        // 3. Flowable 引擎当前已挂载并部署生效的 BPMN 2.0 流程 (Active Deployed Flowable Process)
        String activeDeployed = workflowService != null ? workflowService.getLatestActiveDeployedProcessName() : null;
        if (activeDeployed != null && !activeDeployed.isEmpty()) {
            for (WorkflowTemplate tpl : allTemplates) {
                if (activeDeployed.contains(tpl.getTemplateName()) || tpl.getTemplateName().contains(activeDeployed)) {
                    String rgs = tpl.getResourceGroups() != null ? tpl.getResourceGroups() : "";
                    boolean rgMatch = resourceGroup.isEmpty() || rgs.contains(resourceGroup) || rgs.contains("全部业务资源组通用") || rgs.contains("默认核心业务资源组");
                    boolean typeMatched = isFlowTypeMatched(tpl.getFlowType(), ticketType);
                    if (rgMatch && typeMatched) {
                        String reason = "⚡ Flowable 引擎已挂载部署生效：【" + tpl.getTemplateName() + "】+ 工单类型【" + formatTicketTypeChinese(tpl.getFlowType()) + "】";
                        RoutingPreviewDTO dto = buildRoutingPreviewDTO(tpl, true, reason, expectedRows, ticketType, request.getSqlSnippet());
                        dto.setEnforceDryRun(enforceDryRun);
                        dto.setEnableStep3Rollback(enableStep3Rollback);
                        dto.setEnableStep4DryRun(enableStep4DryRun);
                        return dto;
                    }
                }
            }
        }

        // 4. 资源组中为具体数据库配置的专属审批流
        if (customDbWorkflowName != null && !customDbWorkflowName.isEmpty() && !"DEFAULT".equalsIgnoreCase(customDbWorkflowName)) {
            List<WorkflowTemplate> matchedList = workflowTemplateMapper.selectList(
                    new QueryWrapper<WorkflowTemplate>().eq("template_name", customDbWorkflowName).last("LIMIT 1")
            );
            if (matchedList != null && !matchedList.isEmpty()) {
                WorkflowTemplate dbTpl = matchedList.get(0);
                String reason = "业务资源组【" + resourceGroup + "】已为目标数据库【" + dbName + "】绑定专属审批流【" + dbTpl.getTemplateName() + "】";
                RoutingPreviewDTO dto = buildRoutingPreviewDTO(dbTpl, true, reason, expectedRows, ticketType, request.getSqlSnippet());
                dto.setEnforceDryRun(enforceDryRun);
                dto.setEnableStep3Rollback(enableStep3Rollback);
                dto.setEnableStep4DryRun(enableStep4DryRun);
                return dto;
            }
        }

        // 5. 实例级专属固定审批流 (Pinned / Fixed Workflow Template)
        if (instance != null && instance.getFixedWorkflowTemplateId() != null && instance.getFixedWorkflowTemplateId() > 0) {
            WorkflowTemplate pinnedTpl = workflowTemplateMapper.selectById(instance.getFixedWorkflowTemplateId());
            if (pinnedTpl != null && (pinnedTpl.getStatus() == null || pinnedTpl.getStatus() == 1)) {
                String reason = "目标实例【" + instance.getName() + "】已配置专属固定审批流，跳过综合决策强制生效";
                RoutingPreviewDTO dto = buildRoutingPreviewDTO(pinnedTpl, true, reason, expectedRows, ticketType, request.getSqlSnippet());
                dto.setEnforceDryRun(enforceDryRun);
                dto.setEnableStep3Rollback(enableStep3Rollback);
                dto.setEnableStep4DryRun(enableStep4DryRun);
                return dto;
            }
        }

        // 6. 动态综合智能决策 (Dynamic Multi-Dimensional Decision)

        WorkflowTemplate bestMatch = null;
        int highestScore = -1;
        String matchDetail = "";

        for (WorkflowTemplate tpl : allTemplates) {
            int score = 0;
            String rgs = tpl.getResourceGroups() != null ? tpl.getResourceGroups() : "";
            String flowType = tpl.getFlowType() != null ? tpl.getFlowType() : "ALL";
            // 变更类型匹配判定
            boolean typeMatched = isFlowTypeMatched(flowType, ticketType);
            if (!typeMatched) {
                continue;
            }
            if (flowType.contains(ticketType)) {
                score += 40;
            } else {
                score += 20;
            }

            // 资源组匹配判定
            if (!resourceGroup.isEmpty()) {
                if (rgs.contains(resourceGroup)) {
                    score += 40;
                } else if (rgs.contains("全部业务资源组通用") || rgs.contains("默认核心业务资源组")) {
                    score += 15;
                }
            } else {
                score += 10;
            }

            // 实例标签与条件判定
            if (instance != null && instance.getTags() != null) {
                String tags = instance.getTags();
                if (tags.contains("核心") || tags.contains("生产")) {
                    if (tpl.getTemplateName().contains("智能") || tpl.getTemplateName().contains("严格") || tpl.getTemplateName().contains("高危")) {
                        score += 30;
                    }
                }
                if (tags.contains("测试") && tpl.getTemplateName().contains("测试")) {
                    score += 35;
                }
            }

            // 阈值与条件判定加权
            int threshold = tpl.getAffectRowsThreshold() != null && tpl.getAffectRowsThreshold() > 0 ? tpl.getAffectRowsThreshold() : 1000;
            String dimension = tpl.getConditionDimension() != null ? tpl.getConditionDimension() : "AFFECT_ROWS";
            boolean hasDdl = "DDL_CHANGE".equalsIgnoreCase(ticketType) || (request.getSqlSnippet() != null && (request.getSqlSnippet().toUpperCase().contains("CREATE") || request.getSqlSnippet().toUpperCase().contains("ALTER") || request.getSqlSnippet().toUpperCase().contains("DROP") || request.getSqlSnippet().toUpperCase().contains("TRUNCATE")));
            boolean isHighRisk = "CHANGE_TYPE".equalsIgnoreCase(dimension) ? hasDdl : ("COMPOSITE".equalsIgnoreCase(dimension) ? (expectedRows > threshold || hasDdl) : expectedRows > threshold);

            if (isHighRisk && (tpl.getTemplateName().contains("智能") || tpl.getTemplateName().contains("网关") || tpl.getTemplateName().contains("严格"))) {
                score += 25;
            }

            if (score > highestScore) {
                highestScore = score;
                bestMatch = tpl;
                matchDetail = "综合决策自动命中：资源组【" + (resourceGroup.isEmpty() ? "通用" : resourceGroup) + "】+ 变更类型【" + ticketType + "】" +
                        (instance != null && instance.getTags() != null ? " + 实例标签" + instance.getTags() : "") +
                        (isHighRisk ? "（触发高危条件阈值 > " + threshold + " 行）" : "（符合常规低危阈值 ≤ " + threshold + " 行）");
            }
        }

        if (bestMatch == null) {
            if (!allTemplates.isEmpty()) {
                bestMatch = allTemplates.get(0);
                matchDetail = "默认兜底审批流";
            } else {
                RoutingPreviewDTO fallback = buildDefaultFallbackPreview();
                fallback.setEnforceDryRun(enforceDryRun);
                fallback.setEnableStep3Rollback(enableStep3Rollback);
                fallback.setEnableStep4DryRun(enableStep4DryRun);
                return fallback;
            }
        }

        RoutingPreviewDTO dto = buildRoutingPreviewDTO(bestMatch, false, matchDetail, expectedRows, ticketType, request.getSqlSnippet());
        dto.setEnforceDryRun(enforceDryRun);
        dto.setEnableStep3Rollback(enableStep3Rollback);
        dto.setEnableStep4DryRun(enableStep4DryRun);
        return dto;
    }

    private RoutingPreviewDTO buildRoutingPreviewDTO(WorkflowTemplate tpl, boolean isPinned, String reason, int affectRows, String ticketType, String sqlSnippet) {
        int threshold = tpl.getAffectRowsThreshold() != null && tpl.getAffectRowsThreshold() > 0 ? tpl.getAffectRowsThreshold() : 1000;
        String dimension = tpl.getConditionDimension() != null ? tpl.getConditionDimension() : "AFFECT_ROWS";
        String highRiskRole = tpl.getHighRiskRole() != null && !tpl.getHighRiskRole().isEmpty() ? tpl.getHighRiskRole() : "DBA";
        String lowRiskRole = tpl.getLowRiskRole() != null && !tpl.getLowRiskRole().isEmpty() ? tpl.getLowRiskRole() : "DEV_LEAD";

        boolean hasDdl = "DDL_CHANGE".equalsIgnoreCase(ticketType) || (sqlSnippet != null && (sqlSnippet.toUpperCase().contains("CREATE") || sqlSnippet.toUpperCase().contains("ALTER") || sqlSnippet.toUpperCase().contains("DROP") || sqlSnippet.toUpperCase().contains("TRUNCATE")));
        boolean isHighRisk = "CHANGE_TYPE".equalsIgnoreCase(dimension) ? hasDdl : ("COMPOSITE".equalsIgnoreCase(dimension) ? (affectRows > threshold || hasDdl) : affectRows > threshold);

        List<RoutingPreviewDTO.PreviewNodeDTO> nodes = new ArrayList<>();
        // 节点 1: 申请人发起
        nodes.add(RoutingPreviewDTO.PreviewNodeDTO.builder()
                .step(1)
                .nodeName("申请人发起工单")
                .role("START")
                .approverRole("工单发起人")
                .eligibleApprovers(List.of("当前登录申请人"))
                .condition("提交工单并触发审批流")
                .build());

        boolean isGatewayTemplate = (tpl.getNodeConfig() != null && tpl.getNodeConfig().contains("GATEWAY")) ||
                (tpl.getTemplateName() != null && tpl.getTemplateName().contains("智能条件分支"));

        if (isGatewayTemplate) {
            // 条件网关节点
            String gatewayCond = "AFFECT_ROWS".equalsIgnoreCase(dimension)
                    ? "affect_rows > " + threshold + " ? " + highRiskRole + " : " + lowRiskRole
                    : ("CHANGE_TYPE".equalsIgnoreCase(dimension)
                    ? "has_ddl ? " + highRiskRole + " : " + lowRiskRole
                    : "(affect_rows > " + threshold + " || has_ddl) ? " + highRiskRole + " : " + lowRiskRole);

            nodes.add(RoutingPreviewDTO.PreviewNodeDTO.builder()
                    .step(2)
                    .nodeName("智能排他网关判定 (" + ("AFFECT_ROWS".equalsIgnoreCase(dimension) ? "行数阈值 " + threshold : ("CHANGE_TYPE".equalsIgnoreCase(dimension) ? "DDL变更判定" : "复合判定")) + ")")
                    .role("GATEWAY")
                    .approverRole("BPMN 智能排他网关")
                    .eligibleApprovers(List.of("系统智能自动判定"))
                    .condition(gatewayCond)
                    .build());

            // 动态分流目标节点
            String targetRole = isHighRisk ? highRiskRole : lowRiskRole;
            String nodeName = isHighRisk
                    ? ("DBA".equalsIgnoreCase(highRiskRole) ? "核心DBA安全复核 (高危分支)" : ("ADMIN".equalsIgnoreCase(highRiskRole) ? "系统管理员终审 (高危分支)" : "双人联合复审 (高危分支)"))
                    : ("DEV_LEAD".equalsIgnoreCase(lowRiskRole) ? "业务开发组长初审 (常规分支)" : "业务运维初审 (常规分支)");

            String approverRole = "DBA".equalsIgnoreCase(targetRole) ? "核心数据库管理员 (DBA)" : ("ADMIN".equalsIgnoreCase(targetRole) ? "系统超级管理员 (ADMIN)" : ("DEV_LEAD".equalsIgnoreCase(targetRole) ? "业务开发组长 (DEV_LEAD)" : "运维管理员 (OPS)"));
            List<String> eligible = "DBA".equalsIgnoreCase(targetRole) ? List.of("核心 DBA (DBA)", "管理员 (admin)") : ("ADMIN".equalsIgnoreCase(targetRole) ? List.of("超级管理员 (ADMIN)") : List.of("开发组长 (DEV_LEAD)", "管理员 (admin)"));

            nodes.add(RoutingPreviewDTO.PreviewNodeDTO.builder()
                    .step(3)
                    .nodeName(nodeName)
                    .role(targetRole)
                    .approverRole(approverRole)
                    .eligibleApprovers(eligible)
                    .condition(isHighRisk ? "判定命中高危管控规则" : "判定命中常规放行规则")
                    .build());

            nodes.add(RoutingPreviewDTO.PreviewNodeDTO.builder()
                    .step(4)
                    .nodeName("JDBC流式安全执行")
                    .role("SERVICE")
                    .approverRole("JDBC 引擎自动化流式执行")
                    .eligibleApprovers(List.of("系统引擎全自动发布"))
                    .condition("审批通过后自动执行")
                    .build());
        } else {
            // 标准顺序节点解析
            if (tpl.getNodeConfig() != null && !tpl.getNodeConfig().trim().isEmpty()) {
                try {
                    List<Map<String, Object>> list = objectMapper.readValue(tpl.getNodeConfig(), new TypeReference<List<Map<String, Object>>>() {});
                    int stepCounter = 2;
                    for (Map<String, Object> map : list) {
                        String nodeName = (String) map.getOrDefault("nodeName", "审批节点");
                        String role = (String) map.getOrDefault("role", "DEV_LEAD");
                        String condition = (String) map.getOrDefault("condition", "");

                        String approverRole = "业务开发组长";
                        List<String> eligible = List.of("开发组长 (DEV_LEAD)", "管理员 (admin)");
                        if ("SYSTEM".equalsIgnoreCase(role)) {
                            approverRole = "系统自动化预检引擎 (自动审批)";
                            eligible = List.of("静态语法与事务预检引擎", "系统免审直通网关");
                        } else if ("DBA".equalsIgnoreCase(role)) {
                            approverRole = "核心数据库管理员 (DBA)";
                            eligible = List.of("核心 DBA (DBA)", "管理员 (admin)");
                        } else if ("ADMIN".equalsIgnoreCase(role)) {
                            approverRole = "系统超级管理员 (ADMIN)";
                            eligible = List.of("超级管理员 (ADMIN)");
                        } else if ("SERVICE".equalsIgnoreCase(role)) {
                            approverRole = "JDBC 引擎自动化流式执行";
                            eligible = List.of("系统引擎全自动发布");
                        }

                        nodes.add(RoutingPreviewDTO.PreviewNodeDTO.builder()
                                .step(stepCounter++)
                                .nodeName(nodeName)
                                .role(role)
                                .approverRole(approverRole)
                                .eligibleApprovers(eligible)
                                .condition(condition)
                                .build());
                    }
                } catch (Exception e) {
                    log.warn("Parse nodeConfig failed: {}", e.getMessage());
                }
            }

            if (nodes.size() == 1) {
                nodes.add(RoutingPreviewDTO.PreviewNodeDTO.builder()
                        .step(2)
                        .nodeName("开发组长初审")
                        .role("DEV_LEAD")
                        .approverRole("业务开发组长")
                        .eligibleApprovers(List.of("业务开发组长", "系统管理员"))
                        .build());
                nodes.add(RoutingPreviewDTO.PreviewNodeDTO.builder()
                        .step(3)
                        .nodeName("核心DBA安全复审")
                        .role("DBA")
                        .approverRole("核心数据库管理员")
                        .eligibleApprovers(List.of("核心数据库管理员", "系统管理员"))
                        .build());
            }
        }

        String spel = tpl.getSpelExpression() != null && !tpl.getSpelExpression().isEmpty()
                ? tpl.getSpelExpression()
                : ("CHANGE_TYPE".equalsIgnoreCase(dimension)
                ? "#{hasDdl == true}"
                : ("COMPOSITE".equalsIgnoreCase(dimension)
                ? "#{affectRows > " + threshold + " || hasDdl == true}"
                : "#{affectRows > " + threshold + "}"));

        return RoutingPreviewDTO.builder()
                .templateId(tpl.getId())
                .templateName(tpl.getTemplateName())
                .isPinned(isPinned)
                .isGateway(isGatewayTemplate)
                .resourceGroups(tpl.getResourceGroups())
                .targetDatabases(tpl.getTargetDatabases())
                .flowType(tpl.getFlowType())
                .routingReason(reason)
                .triggerCondition(tpl.getTriggerCondition())
                .defaultExecutionMode(tpl.getDefaultExecutionMode())
                .conditionDimension(dimension)
                .affectRowsThreshold(threshold)
                .isHighRisk(isHighRisk)
                .highRiskRole(highRiskRole)
                .lowRiskRole(lowRiskRole)
                .spelExpression(spel)
                .nodes(nodes)
                .build();
    }

    private boolean isFlowTypeMatched(String flowType, String ticketType) {
        if (flowType == null || flowType.trim().isEmpty() || "ALL".equalsIgnoreCase(flowType.trim())) {
            return true;
        }
        if (ticketType == null || ticketType.trim().isEmpty()) {
            return true;
        }
        String ft = flowType.trim();
        if (ft.contains(ticketType)) {
            return true;
        }
        if ("SQL_AUDIT".equalsIgnoreCase(ticketType)) {
            return ft.contains("SQL_AUDIT") || ft.contains("DML_CHANGE") || ft.contains("DDL_CHANGE");
        }
        if ("DML_CHANGE".equalsIgnoreCase(ticketType)) {
            return ft.contains("SQL_AUDIT") || ft.contains("DML_CHANGE");
        }
        if ("DDL_CHANGE".equalsIgnoreCase(ticketType)) {
            return ft.contains("SQL_AUDIT") || ft.contains("DDL_CHANGE");
        }
        if ("PERMISSION".equalsIgnoreCase(ticketType)) {
            return ft.contains("PERMISSION") || ft.contains("DATA_QUERY");
        }
        if ("DATA_QUERY".equalsIgnoreCase(ticketType)) {
            return ft.contains("PERMISSION") || ft.contains("DATA_QUERY");
        }
        if ("DATA_RECOVERY".equalsIgnoreCase(ticketType)) {
            return ft.contains("DATA_RECOVERY");
        }
        return false;
    }

    private String formatTicketTypeChinese(String type) {
        if (type == null || type.trim().isEmpty() || "ALL".equalsIgnoreCase(type)) return "全部工单类型通用";
        StringBuilder sb = new StringBuilder();
        String[] parts = type.replace("[", "").replace("]", "").replace("\"", "").split(",");
        for (String p : parts) {
            String clean = p.trim();
            if (clean.isEmpty()) continue;
            if (sb.length() > 0) sb.append("、");
            switch (clean.toUpperCase()) {
                case "SQL_AUDIT":
                    sb.append("SQL 变更审核 (全量)");
                    break;
                case "DML_CHANGE":
                    sb.append("DML 数据变更");
                    break;
                case "DDL_CHANGE":
                    sb.append("DDL 结构变更");
                    break;
                case "DATA_EXPORT":
                    sb.append("敏感数据导出");
                    break;
                case "PERMISSION":
                    sb.append("权限与账号申请");
                    break;
                case "DATA_QUERY":
                    sb.append("数据查询提权");
                    break;
                case "DATA_RECOVERY":
                    sb.append("应急数据修复与恢复");
                    break;
                default:
                    sb.append(clean);
                    break;
            }
        }
        return sb.length() > 0 ? sb.toString() : "全部工单类型通用";
    }

    private RoutingPreviewDTO buildDefaultFallbackPreview() {
        return RoutingPreviewDTO.builder()
                .templateId(0L)
                .templateName("标准生产变更两级审批流")
                .isPinned(false)
                .isGateway(false)
                .flowType("SQL_AUDIT")
                .routingReason("默认通用审批流")
                .triggerCondition("通用 SQL 变更")
                .defaultExecutionMode("[\"IMMEDIATE\"]")
                .affectRowsThreshold(1000)
                .conditionDimension("AFFECT_ROWS")
                .spelExpression("#{affectRows > 1000}")
                .isHighRisk(false)
                .highRiskRole("DBA")
                .lowRiskRole("DEV_LEAD")
                .nodes(List.of(
                        RoutingPreviewDTO.PreviewNodeDTO.builder()
                                .step(1)
                                .nodeName("申请人发起工单")
                                .role("START")
                                .approverRole("工单申请人")
                                .eligibleApprovers(List.of("当前登录申请人"))
                                .build(),
                        RoutingPreviewDTO.PreviewNodeDTO.builder()
                                .step(2)
                                .nodeName("开发组长初审")
                                .role("DEV_LEAD")
                                .approverRole("业务开发组长")
                                .eligibleApprovers(List.of("业务开发组长", "系统管理员"))
                                .build(),
                        RoutingPreviewDTO.PreviewNodeDTO.builder()
                                .step(3)
                                .nodeName("核心DBA安全复核")
                                .role("DBA")
                                .approverRole("核心数据库管理员")
                                .eligibleApprovers(List.of("核心数据库管理员", "系统管理员"))
                                .build()
                ))
                .build();
    }

    /**
     * 在线 SpEL 规则沙箱测试与求值
     */
    public com.wmdb.model.SpelEvaluationResultDTO evaluateSpel(com.wmdb.model.SpelEvaluationRequestDTO request) {
        if (request == null || request.getSpelExpression() == null || request.getSpelExpression().trim().isEmpty()) {
            return com.wmdb.model.SpelEvaluationResultDTO.builder()
                    .syntaxValid(false)
                    .matched(false)
                    .errorMessage("SpEL 表达式不可为空")
                    .explanation("❌ 请输入有效的 SpEL 条件表达式")
                    .build();
        }

        String rawExpr = request.getSpelExpression().trim();
        if (rawExpr.startsWith("#{") && rawExpr.endsWith("}")) {
            rawExpr = rawExpr.substring(2, rawExpr.length() - 1).trim();
        }

        try {
            org.springframework.expression.ExpressionParser parser = new org.springframework.expression.spel.standard.SpelExpressionParser();
            org.springframework.expression.Expression expression = parser.parseExpression(rawExpr);
            org.springframework.expression.spel.support.StandardEvaluationContext context = new org.springframework.expression.spel.support.StandardEvaluationContext();
            context.addPropertyAccessor(new org.springframework.context.expression.MapAccessor());

            if (request.getContext() != null) {
                context.setRootObject(request.getContext());
                for (Map.Entry<String, Object> entry : request.getContext().entrySet()) {
                    context.setVariable(entry.getKey(), entry.getValue());
                }
            }

            Object evaluated = expression.getValue(context);
            boolean isMatched = false;
            if (evaluated instanceof Boolean b) {
                isMatched = b;
            } else if (evaluated != null) {
                isMatched = true;
            }

            String explanation = isMatched
                    ? "✅ SpEL 表达式计算结果为 true（判定命中高危管控分支）"
                    : "⚡ SpEL 表达式计算结果为 false（判定命中常规低危分支）";

            return com.wmdb.model.SpelEvaluationResultDTO.builder()
                    .syntaxValid(true)
                    .matched(isMatched)
                    .result(evaluated)
                    .explanation(explanation)
                    .build();
        } catch (Exception e) {
            return com.wmdb.model.SpelEvaluationResultDTO.builder()
                    .syntaxValid(false)
                    .matched(false)
                    .errorMessage("SpEL 语法解析或求值异常: " + e.getMessage())
                    .explanation("❌ 表达式语法错误或变量缺失")
                    .build();
        }
    }
}
