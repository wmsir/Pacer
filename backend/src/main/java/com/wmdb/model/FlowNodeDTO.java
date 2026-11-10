package com.wmdb.model;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

import java.util.ArrayList;
import java.util.List;

/**
 * 审批流节点明细数据传输对象
 * <p>
 * 用于前端可视化渲染审批步骤条、各节点当前状态及候选审批人员信息。
 * </p>
 *
 * @author wm
 */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class FlowNodeDTO {

    /**
     * 节点标识 (如: submit, leader_review, dba_review, execution, archive)
     */
    private String nodeKey;

    /**
     * 节点中文名称 (如: 提交工单, 开发组长初审, DBA安全复核, 自动化流式执行, 变更归档)
     */
    private String nodeName;

    /**
     * 节点类型 (START, USER_TASK, SERVICE_TASK, END)
     */
    private String nodeType;

    /**
     * 节点状态 (COMPLETED-已完成, ACTIVE-当前进行中/待审核, PENDING-未到达/等待中, REJECTED-已驳回)
     */
    private String status;

    /**
     * 审批角色职责描述 (如: 申请人, 开发组长 (DEV_LEAD), 数据库管理员 (DBA), 自动化执行引擎)
     */
    private String approverRole;

    /**
     * 当前节点可审批人员清单 (如: ["开发组长 (DEV_LEAD)", "管理员 (testadmin1, testadmin2 全权特权审批)"])
     */
    @Builder.Default
    private List<String> eligibleApprovers = new ArrayList<>();

    /**
     * 实际审批/操作人
     */
    private String actualApprover;

    /**
     * 节点完成/流转时间
     */
    private String finishTime;

    /**
     * 审批意见或流转说明
     */
    private String comment;
}
