package com.wmdb.model;

import com.baomidou.mybatisplus.annotation.IdType;
import com.baomidou.mybatisplus.annotation.TableId;
import com.baomidou.mybatisplus.annotation.TableName;
import com.fasterxml.jackson.databind.annotation.JsonSerialize;
import com.fasterxml.jackson.databind.ser.std.ToStringSerializer;
import lombok.Data;

/**
 * SQL 工单主表实体类
 * <p>
 * 映射 sql_ticket 表，记录审批流工单基础信息，串联 Flowable 审批实例。
 * </p>
 *
 * @author wm
 * @date 2023-10-25
 */
@Data
@TableName("sql_ticket")
public class SqlTicket {

    private String tenantId;

    /**
     * 主键 ID（18位数字通过 ToStringSerializer 防止前端 JS 精度丢失）
     */
    @TableId(type = IdType.INPUT)
    @JsonSerialize(using = ToStringSerializer.class)
    private Long id;

    /**
     * 业务流水号
     */
    private String businessKey;

    /**
     * 目标数据库实例 ID
     */
    private Long instanceId;

    /**
     * 申请人身份证号码
     */
    private String applicantIdCard;

    /**
     * 工单状态（DRAFT, AUDITING, APPROVED, REJECTED, EXECUTED, FAILED）
     */
    private String status;

    /**
     * 风险等级
     */
    private String riskLevel;

    /**
     * 工单类型（SQL_AUDIT, DATA_EXPORT, PERMISSION, ACCOUNT, DB_TABLE, DATA_RECOVERY）
     */
    private String type;

    /**
     * 申请原因（非 SQL 文件类工单的业务描述）
     */
    private String reason;

    /**
     * 关联的 Flowable 流程实例 ID
     */
    private String flowInstanceId;

    /**
     * 指定的维护窗口期（如：cron 表达式或特定时间范围，为空则立即执行）
     */
    private String executionWindow;

    /**
     * 绑定的审批流模板 ID
     */
    private Long workflowTemplateId;

    /**
     * 审批流模板名称
     */
    private String workflowTemplateName;

    /**
     * 申请人真实姓名/显示名称
     */
    private String applicantName;

    /**
     * 目标 Schema / 数据库名称
     */
    private String dbName;

    /**
     * 提交时间 (yyyy-MM-dd HH:mm:ss)
     */
    private String createTime;
}
