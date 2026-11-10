package com.wmdb.model;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

import java.util.List;

/**
 * 审批流预估路由结果与节点链路 DTO
 *
 * @author wm
 */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class RoutingPreviewDTO {

    /**
     * 模板 ID
     */
    private Long templateId;

    /**
     * 模板名称
     */
    private String templateName;

    /**
     * 是否命中专属固定审批流 (true: 数据库实例指定固定流，false: 综合决策动态匹配)
     */
    private Boolean isPinned;

    /**
     * 是否为条件网关分支流 (true: SpEL 双分支网关, false: 顺序多级递进审批流)
     */
    private Boolean isGateway;

    /**
     * 绑定的生效业务资源组
     */
    private String resourceGroups;

    /**
     * 绑定的生效目标数据库范围
     */
    private String targetDatabases;

    /**
     * 适用变更类型
     */
    private String flowType;

    /**
     * 路由判定原因与依据描述
     */
    private String routingReason;

    /**
     * 触发条件说明
     */
    private String triggerCondition;

    /**
     * 审批通过后的默认执行模式
     */
    private String defaultExecutionMode;

    /**
     * 智能条件判定维度 (AFFECT_ROWS, CHANGE_TYPE, COMPOSITE)
     */
    private String conditionDimension;

    /**
     * 配置的影响行数阈值 (如 1000, 2000, 5000)
     */
    private Integer affectRowsThreshold;

    /**
     * 本次预执行是否触发高危分支
     */
    private Boolean isHighRisk;

    /**
     * 高危分支审批角色
     */
    private String highRiskRole;

    /**
     * 常规低危分支审批角色
     */
    private String lowRiskRole;

    /**
     * 智能网关 SpEL 条件表达式
     */
    private String spelExpression;

    /**
     * 是否强制预执行校验 (true: 必须通过才可提交, false: 推荐校验/允许跳过)
     */
    private Boolean enforceDryRun;

    /**
     * 是否启用第 3 步「数据回滚方案与补偿脚本」
     */
    private Boolean enableStep3Rollback;

    /**
     * 是否启用第 4 步「事务级预执行校验 (Dry-Run)」
     */
    private Boolean enableStep4DryRun;

    /**
     * 流程节点链表
     */
    private List<PreviewNodeDTO> nodes;

    @Data
    @Builder
    @NoArgsConstructor
    @AllArgsConstructor
    public static class PreviewNodeDTO {
        private Integer step;
        private String nodeName;
        private String role;
        private String approverRole;
        private List<String> eligibleApprovers;
        private String condition;
    }
}
