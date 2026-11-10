package com.wmdb.model;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

import java.util.Date;

/**
 * 待办审批工单提醒 DTO
 *
 * @author wm
 */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class PendingApprovalDTO {

    /**
     * 工单 ID
     */
    private Long id;

    /**
     * 工单业务编号
     */
    private String businessKey;

    /**
     * 变更类型 (SQL_AUDIT, DML_CHANGE, DDL_CHANGE, DATA_QUERY)
     */
    private String type;

    /**
     * 数据库实例名称
     */
    private String instanceName;

    /**
     * 目标数据库 Schema
     */
    private String dbName;

    /**
     * 申请人身份证/账号
     */
    private String applicantIdCard;

    /**
     * 申请人真实姓名
     */
    private String applicantName;

    /**
     * 申请原因/工单描述
     */
    private String reason;

    /**
     * 当前审批节点 (如: 开发组长初审, DBA安全复审, 管理员终审)
     */
    private String currentNodeName;

    /**
     * 预估影响行数
     */
    private Integer affectRows;

    /**
     * 申请提交时间
     */
    private Date createTime;
}
