package com.wmdb.model;

import com.baomidou.mybatisplus.annotation.IdType;
import com.baomidou.mybatisplus.annotation.TableId;
import com.baomidou.mybatisplus.annotation.TableName;
import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

import java.util.Date;

/**
 * 审批流模板实体类
 * <p>
 * 映射 workflow_template 表，存储审批流模板定义及其绑定的业务资源组。
 * </p>
 *
 * @author wm
 */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
@TableName("workflow_template")
public class WorkflowTemplate {

    private String tenantId;

    /**
     * 模板 ID
     */
    @TableId(type = IdType.AUTO)
    private Long id;

    /**
     * 模板名称 (如: 标准DML两级审批流, 高危DDL三级审批流)
     */
    private String templateName;

    /**
     * 适用变更类型 (DML_CHANGE, DDL_CHANGE, SQL_AUDIT, DATA_QUERY, ALL)
     */
    private String flowType;

    /**
     * 绑定的业务资源组列表 (JSON 数组字符串，如 '["车险承保资源组","销管系统资源组"]')
     */
    private String resourceGroups;

    /**
     * 绑定的具体生效数据库列表 (JSON 数组字符串，如 '["ALL"]' 或 '["car_prod_mysql/db_policy"]')
     */
    private String targetDatabases;

    /**
     * 审批节点定义配置 (JSON 数组字符串，存储节点序号、节点名称、审批角色)
     */
    private String nodeConfig;

    /**
     * 智能条件判定维度 (AFFECT_ROWS 影响行数, CHANGE_TYPE 变更类型, COMPOSITE 复合规则)
     */
    private String conditionDimension;

    /**
     * 影响行数判定阈值（如 1000、2000、5000）
     */
    private Integer affectRowsThreshold;

    /**
     * 高危分支审批角色（如 DBA, ADMIN, DBA_ADMIN）
     */
    private String highRiskRole;

    /**
     * 常规低危分支审批角色（如 DEV_LEAD, OPS）
     */
    private String lowRiskRole;

    /**
     * 智能网关 SpEL 条件表达式 (如: #{affectRows > 1000}, #{affectRows > 2000 || hasDdl == true})
     */
    private String spelExpression;

    /**
     * 触发与生效条件说明 (如: 影响行数 > 500 或 DDL 语句)
     */
    private String triggerCondition;

    /**
     * 审批通过后的默认执行策略 (IMMEDIATE 立即执行, SCHEDULED 定时执行, MANUAL_DBA 转DBA线下工具)
     */
    private String defaultExecutionMode;

    /**
     * 状态：1-启用，0-停用
     */
    private Integer status;

    /**
     * 模板业务描述
     */
    private String description;

    /**
     * 创建时间
     */
    private Date createTime;

    /**
     * 更新时间
     */
    private Date updateTime;
}
