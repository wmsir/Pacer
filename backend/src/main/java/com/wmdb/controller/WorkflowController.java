package com.wmdb.controller;

import com.wmdb.common.Result;
import com.wmdb.model.WorkflowTemplate;
import com.wmdb.service.TicketService;
import com.wmdb.service.WorkflowService;
import io.swagger.v3.oas.annotations.Operation;
import io.swagger.v3.oas.annotations.tags.Tag;
import lombok.Data;
import lombok.RequiredArgsConstructor;
import org.springframework.web.bind.annotation.*;

import java.util.HashMap;
import java.util.List;
import java.util.Map;

/**
 * 审批流与工作流设计控制器
 * <p>
 * 提供常用审批流程模板库、BPMN 在线部署与外部审批系统的 Webhook 回调。
 * </p>
 *
 * @author wm
 * @date 2023-10-25
 */
@Tag(name = "工作流与审批流管理", description = "提供常用审批流程模板查询、BPMN 2.0 在线部署与回调流转")
@RestController
@RequestMapping("/api/v1/workflow")
@RequiredArgsConstructor
public class WorkflowController {

    private final TicketService ticketService;
    private final WorkflowService workflowService;

    /**
     * 获取系统预置的常用审批流程模板列表
     */
    @Operation(summary = "获取常用审批流程模板库", description = "返回标准SQL变更、高危DDL复核、紧急绿色通道、敏感数据导出等预置流程模板")
    @GetMapping("/templates")
    public Result<List<com.wmdb.model.BpmnTemplateDTO>> getTemplates() {
        return Result.success(workflowService.getPresetTemplates());
    }

    /**
     * 在线部署 BPMN 工作流
     */
    @Operation(summary = "在线部署审批流程", description = "接收 BPMN 2.0 XML 字符串并挂载至 Flowable 引擎")
    @PostMapping("/deploy")
    public Result<Map<String, Object>> deployProcess(@RequestBody DeployRequest request) {
        String deploymentId = workflowService.deployBpmn(request.getProcessName(), request.getBpmnXml());
        Map<String, Object> data = new HashMap<>();
        data.put("deploymentId", deploymentId);
        data.put("processName", request.getProcessName());
        data.put("isDeployed", true);
        data.put("message", "流程定义已成功部署挂载至 Flowable 引擎");
        return Result.success(data);
    }

    /**
     * 查询流程部署状态
     */
    @Operation(summary = "查询流程部署状态", description = "获取当前流程是否已挂载部署至 Flowable 引擎")
    @GetMapping("/deploy-status")
    public Result<Map<String, Object>> getDeployStatus(@RequestParam("processName") String processName) {
        return Result.success(workflowService.getDeployStatus(processName));
    }

    /**
     * 终止 / 卸载流程定义
     */
    @Operation(summary = "终止并卸载审批流程", description = "从 Flowable 引擎中卸载并终止指定的审批流程定义")
    @PostMapping("/terminate")
    public Result<Map<String, Object>> terminateProcess(@RequestBody TerminateRequest request) {
        workflowService.terminateBpmn(request.getProcessName(), request.getDeploymentId());
        Map<String, Object> data = new HashMap<>();
        data.put("processName", request.getProcessName());
        data.put("isDeployed", false);
        data.put("message", "流程定义【" + request.getProcessName() + "】已成功终止并从 Flowable 引擎卸载！");
        return Result.success(data);
    }

    /**
     * 接收外部审批系统的回调
     */
    @Operation(summary = "审批流 Webhook 回调", description = "供外部 OA 或审批中心完成审批后回调触发工单执行")
    @PostMapping("/callback")
    public Result<String> callback(@RequestBody WorkflowCallbackRequest request) {
        if ("APPROVED".equals(request.getStatus())) {
            ticketService.approveTicket(request.getTicketId());
        } else {
            // handle rejection
        }
        return Result.success("Callback processed");
    }

    /**
     * 流程部署请求封装实体
     */
    @Data
    public static class DeployRequest {
        private String processName;
        private String bpmnXml;
    }

    /**
     * 终止流程请求实体
     */
    @Data
    public static class TerminateRequest {
        private String processName;
        private String deploymentId;
    }

    /**
     * 工作流回调请求参数实体
     */
    @Data
    public static class WorkflowCallbackRequest {
        /**
         * 工单 ID
         */
        private Long ticketId;

        /**
         * 审批状态（如：APPROVED, REJECTED）
         */
        private String status;

        /**
         * 审批意见
         */
        private String comment;
    }
}
