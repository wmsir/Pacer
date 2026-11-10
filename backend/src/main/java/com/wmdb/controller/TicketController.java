package com.wmdb.controller;

import com.wmdb.common.Result;
import com.wmdb.exception.BusinessException;
import com.wmdb.model.DryRunResult;
import com.wmdb.model.SqlQueryResult;
import com.wmdb.model.SqlTicket;
import com.wmdb.model.SqlTicketDetail;
import com.wmdb.service.SqlQueryService;
import com.wmdb.service.TicketService;
import io.minio.GetPresignedObjectUrlArgs;
import io.minio.MinioClient;
import io.minio.http.Method;
import io.swagger.v3.oas.annotations.Operation;
import io.swagger.v3.oas.annotations.tags.Tag;
import lombok.Data;
import lombok.extern.slf4j.Slf4j;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.security.core.context.SecurityContextHolder;
import org.springframework.web.bind.annotation.*;
import org.springframework.web.multipart.MultipartFile;

import java.util.List;
import java.util.Map;
import java.util.concurrent.TimeUnit;

/**
 * 工单管理与 SQL 控制台控制器
 * <p>
 * 提供工单提交、三种执行模式审批（立即/定时/转DBA工具）、DBA 线下执行反馈、手动触发执行、驳回、指定数据库的 SQL 影响行数预执行校验 (Dry Run)、安全只读 SQL 查询控制台、详情获取、附件下载等接口。
 * </p>
 *
 * @author wm
 * @date 2023-10-25
 */
@Tag(name = "SQL 工单管理与执行流转", description = "包含工单创建、DML 影响行数事务级预执行校验 (Dry-Run)、三种模式审批通过、DBA线下工具执行反馈、手动立即触发、驳回、SQL 在线安全查询、详情查询与附件下载")
@Slf4j
@RestController
@RequestMapping("/api/v1/ticket")
public class TicketController {

    private final TicketService ticketService;
    private final SqlQueryService sqlQueryService;

    @Value("${wmdb.minio.endpoint:http://localhost:9000}")
    private String endpoint;

    @Value("${wmdb.minio.access-key:minioadmin}")
    private String accessKey;

    @Value("${wmdb.minio.secret-key:minioadmin}")
    private String secretKey;

    @Value("${wmdb.minio.bucket:wmdb-sql-bucket}")
    private String bucketName;

    public TicketController(TicketService ticketService, SqlQueryService sqlQueryService) {
        this.ticketService = ticketService;
        this.sqlQueryService = sqlQueryService;
    }

    /**
     * SQL 在线只读查询控制台
     */
    @Operation(summary = "SQL 在线只读安全查询", description = "支持在指定数据库实例与库上执行 SELECT/SHOW/DESC/EXPLAIN 查询，自带 MaxRows 保护与只读拦截")
    @PostMapping("/query")
    public Result<SqlQueryResult> executeQuery(@RequestBody QueryRequest request) {
        SqlQueryResult result = sqlQueryService.executeQuery(request.getInstanceId(), request.getDbName(), request.getSql(), request.getLimit());
        return Result.success(result);
    }

    /**
     * SQL 执行计划 (EXPLAIN) 分析
     */
    @Operation(summary = "SQL 执行计划 (EXPLAIN) 分析", description = "生成 SQL 的执行计划并进行性能分析与索引建议")
    @PostMapping("/explain")
    public Result<SqlQueryResult> executeExplain(@RequestBody QueryRequest request) {
        SqlQueryResult result = sqlQueryService.executeExplain(request.getInstanceId(), request.getDbName(), request.getSql());
        return Result.success(result);
    }

    /**
     * 指定数据库 DML 影响行数事务级预执行校验 (Dry-Run)
     */
    @Operation(summary = "SQL 影响行数预执行校验", description = "在目标数据库实例与指定 Schema 事务内模拟预执行，识别 `-- 1` 等影响行数注释并进行一致性比对，自动 ROLLBACK 回滚")
    @PostMapping("/dry-run")
    public Result<DryRunResult> dryRun(@RequestParam("instanceId") Long instanceId,
                                       @RequestParam(value = "dbName", required = false) String dbName,
                                       @RequestParam(value = "sqlText", required = false) String sqlText,
                                       @RequestParam(value = "file", required = false) MultipartFile file) {
        DryRunResult result = ticketService.dryRun(instanceId, dbName, sqlText, file);
        return Result.success(result);
    }

    /**
     * 提交 SQL 审核工单
     */
    @Operation(summary = "提交 SQL 工单", description = "自动在指定数据库执行 AST 校验及 DML 影响行数一致性预检，校验通过后创建工单并挂载审批流程")
    @PostMapping("/submit")
    public Result<SqlTicket> submitTicket(@RequestParam("instanceId") Long instanceId,
                                          @RequestParam(value = "dbName", required = false) String dbName,
                                          @RequestParam(value = "type", defaultValue = "SQL_AUDIT") String type,
                                          @RequestParam(value = "reason", required = false) String reason,
                                          @RequestParam(value = "sqlText", required = false) String sqlText,
                                          @RequestParam(value = "expectedRows", required = false) Integer expectedRows,
                                          @RequestParam(value = "file", required = false) MultipartFile file,
                                          @RequestParam(value = "rollbackSqlText", required = false) String rollbackSqlText,
                                          @RequestParam(value = "rollbackFile", required = false) MultipartFile rollbackFile,
                                          @RequestParam(value = "customFieldValues", required = false) String customFieldValues) throws Exception {
        String idCard = (String) SecurityContextHolder.getContext().getAuthentication().getPrincipal();
        SqlTicket ticket = ticketService.submitTicket(idCard, instanceId, dbName, type, reason, sqlText, file, expectedRows, rollbackSqlText, rollbackFile, customFieldValues);
        return Result.success(ticket);
    }

    /**
     * 审批通过工单（支持 立即执行 / 定时执行 / 转DBA工具执行）
     */
    @Operation(summary = "审批通过工单", description = "支持选择三种执行模式：IMMEDIATE(立即执行)、SCHEDULED(定时执行)、MANUAL_DBA(转DBA工具手工执行)")
    @PostMapping("/{id}/approve")
    public Result<com.wmdb.service.AsyncTicketExecutor.ExecutionResult> approveTicket(@PathVariable("id") Long id,
                                                                                      @RequestBody(required = false) ApprovalActionRequest request) {
        String operatorIdCard = (String) SecurityContextHolder.getContext().getAuthentication().getPrincipal();
        String executionMode = (request != null && request.getExecutionMode() != null) ? request.getExecutionMode() : "IMMEDIATE";
        String scheduledTime = (request != null) ? request.getScheduledTime() : null;
        Integer batchSize = (request != null) ? request.getBatchSize() : null;
        Integer intervalMs = (request != null) ? request.getIntervalMs() : null;
        String comment = (request != null && request.getComment() != null) ? request.getComment() : "审批通过";

        com.wmdb.service.AsyncTicketExecutor.ExecutionResult result = ticketService.approveTicket(id, operatorIdCard, executionMode, scheduledTime, batchSize, intervalMs, comment);
        return Result.success(result);
    }

    /**
     * DBA 线下工具执行反馈与归档
     */
    @Operation(summary = "DBA 线下工具执行反馈", description = "DBA线下通过客户端工具执行完毕后，提交执行结果与日志反馈，完成工单流转归档")
    @PostMapping("/{id}/feedback")
    public Result<Void> submitDbaFeedback(@PathVariable("id") Long id,
                                          @RequestBody DbaFeedbackRequest request) {
        String operatorIdCard = (String) SecurityContextHolder.getContext().getAuthentication().getPrincipal();
        ticketService.submitDbaFeedback(
                id,
                operatorIdCard,
                request.getStatus(),
                request.getAffectRows(),
                request.getDurationMs(),
                request.getFeedbackNotes()
        );
        return Result.success(null);
    }

    /**
     * 立即触发工单流式执行
     */
    @Operation(summary = "立即触发执行", description = "针对定时执行或等待中的工单，管理员/DBA 可手动提前立即触发流式执行")
    @PostMapping("/{id}/execute-now")
    public Result<com.wmdb.service.AsyncTicketExecutor.ExecutionResult> executeNow(@PathVariable("id") Long id) {
        String operatorIdCard = (String) SecurityContextHolder.getContext().getAuthentication().getPrincipal();
        com.wmdb.service.AsyncTicketExecutor.ExecutionResult result = ticketService.executeNow(id, operatorIdCard);
        return Result.success(result);
    }

    /**
     * 驳回工单
     */
    @Operation(summary = "驳回工单", description = "审核人员驳回工单，并填写驳回原因")
    @PostMapping("/{id}/reject")
    public Result<Void> rejectTicket(@PathVariable("id") Long id,
                                     @RequestBody(required = false) ApprovalActionRequest request) {
        String operatorIdCard = (String) SecurityContextHolder.getContext().getAuthentication().getPrincipal();
        String reason = (request != null && request.getComment() != null) ? request.getComment() : "审核未通过，驳回修改";
        ticketService.rejectTicket(id, operatorIdCard, reason);
        return Result.success(null);
    }

    /**
     * 工单加急催办（向当前审批人发送企微/钉钉加急提醒）
     */
    @Operation(summary = "工单加急催办", description = "向当前节点候选审批人员发送企业微信、钉钉等多渠道加急催办提醒")
    @PostMapping("/{id}/urge")
    public Result<java.util.Map<String, Object>> urgeTicket(@PathVariable("id") Long id,
                                                            @RequestBody(required = false) java.util.Map<String, String> body) {
        String operatorIdCard = (String) SecurityContextHolder.getContext().getAuthentication().getPrincipal();
        String reason = body != null ? body.get("reason") : null;
        java.util.Map<String, Object> result = ticketService.urgeTicket(id, operatorIdCard, reason);
        return Result.success(result);
    }

    /**
     * 获取用户工单列表（全量兼容接口）
     */
    @Operation(summary = "获取当前用户工单列表")
    @GetMapping("/list")
    public Result<List<SqlTicket>> listTickets() {
        String currentIdCard = (String) SecurityContextHolder.getContext().getAuthentication().getPrincipal();
        return Result.success(ticketService.listUserTickets(currentIdCard));
    }

    /**
     * 分页多维条件检索用户工单列表
     */
    @Operation(summary = "分页查询当前用户工单列表", description = "支持状态、视角、实例、工单类型、关键字分页筛选与数据权限隔离")
    @GetMapping("/page")
    public Result<com.wmdb.model.PageResultDTO<SqlTicket>> pageTickets(
            @RequestParam(value = "page", defaultValue = "1") int page,
            @RequestParam(value = "size", defaultValue = "10") int size,
            @RequestParam(value = "status", required = false) String status,
            @RequestParam(value = "userPerspective", required = false) String userPerspective,
            @RequestParam(value = "instanceId", required = false) Long instanceId,
            @RequestParam(value = "type", required = false) String type,
            @RequestParam(value = "keyword", required = false) String keyword) {
        String currentIdCard = (String) SecurityContextHolder.getContext().getAuthentication().getPrincipal();
        return Result.success(ticketService.pageUserTickets(currentIdCard, page, size, status, userPerspective, instanceId, type, keyword));
    }

    /**
     * 获取当前用户生效的工单数据权限范围信息
     */
    @Operation(summary = "获取当前用户生效的工单数据权限范围信息")
    @GetMapping("/data-scope")
    public Result<Map<String, Object>> getDataScope() {
        String currentIdCard = (String) SecurityContextHolder.getContext().getAuthentication().getPrincipal();
        return Result.success(ticketService.getDataScopeInfo(currentIdCard));
    }

    /**
     * 获取工单详情
     */

    @Operation(summary = "获取工单详情及 SQL 明细")
    @GetMapping("/{id}/detail")
    public Result<Map<String, Object>> getTicketDetail(@PathVariable("id") Long id) {
        String currentIdCard = (String) SecurityContextHolder.getContext().getAuthentication().getPrincipal();
        Map<String, Object> detail = ticketService.getTicketDetail(id, currentIdCard);
        if (detail == null || detail.get("ticket") == null) {
            throw new BusinessException("A0403", "拒绝访问或工单不存在");
        }
        return Result.success(detail);
    }

    /**
     * 重新调整定时计划执行时间 (Reschedule)
     */
    @Operation(summary = "调整定时执行时间 (Reschedule)", description = "针对处于定时计划执行中 (WAITING_EXECUTION) 的工单，重新指定执行维护窗口")
    @PostMapping("/{id}/reschedule")
    public Result<String> rescheduleTicket(@PathVariable("id") Long id, @RequestBody RescheduleRequest request) {
        String currentIdCard = (String) SecurityContextHolder.getContext().getAuthentication().getPrincipal();
        ticketService.rescheduleTicket(id, currentIdCard, request.getScheduledTime(), request.getComment());
        return Result.success("定时计划执行时间已成功更新");
    }

    /**
     * 获取附件下载预签名链接
     */
    @Operation(summary = "获取 SQL 附件临时防盗链下载 URL")
    @GetMapping("/{id}/download-url")
    public Result<Map<String, String>> getDownloadUrl(@PathVariable("id") Long id) throws Exception {
        String currentIdCard = (String) SecurityContextHolder.getContext().getAuthentication().getPrincipal();
        Map<String, Object> detailMap = ticketService.getTicketDetail(id, currentIdCard);
        if (detailMap == null || detailMap.get("detail") == null) {
            throw new BusinessException("A0403", "拒绝访问或工单不存在");
        }

        SqlTicketDetail detail = (SqlTicketDetail) detailMap.get("detail");
        String objectKey = detail.getAttachmentOssKey();
        if (objectKey == null) {
            throw new BusinessException("A0404", "此工单没有附件");
        }

        MinioClient minioClient = MinioClient.builder()
                .endpoint(endpoint)
                .credentials(accessKey, secretKey)
                .build();

        String url = minioClient.getPresignedObjectUrl(
                GetPresignedObjectUrlArgs.builder()
                        .method(Method.GET)
                        .bucket(bucketName)
                        .object(objectKey)
                        .expiry(5, TimeUnit.MINUTES)
                        .build());

        return Result.success(Map.of("url", url));
    }

    /**
     * 获取当前登录人待审批工单提醒列表
     */
    @Operation(summary = "获取待审批工单提醒列表", description = "用于系统右上角待办铃铛提醒，支持查看当前人员需要审批的工单清单及待办总数")
    @GetMapping("/pending-approvals")
    public Result<List<com.wmdb.model.PendingApprovalDTO>> getPendingApprovals() {
        String currentIdCard = (String) SecurityContextHolder.getContext().getAuthentication().getPrincipal();
        return Result.success(ticketService.getPendingApprovals(currentIdCard));
    }

    /**
     * 在线查询请求体
     */
    @Data
    public static class QueryRequest {
        private Long instanceId;
        private String dbName;
        private String sql;
        private Integer limit;
    }

    /**
     * 审批/驳回请求体
     */
    @Data
    public static class ApprovalActionRequest {
        private String executionMode; // IMMEDIATE, SCHEDULED, CANARY_BATCH, MANUAL_DBA
        private String scheduledTime; // e.g. 2026-08-20 02:00:00
        private Integer batchSize;    // e.g. 500
        private Integer intervalMs;   // e.g. 100
        private String comment;
    }

    /**
     * DBA 线下工具执行结果反馈请求体
     */
    @Data
    public static class DbaFeedbackRequest {
        private String status; // SUCCESS, FAILED
        private Integer affectRows;
        private Long durationMs;
        private String feedbackNotes;
    }

    /**
     * 定时计划执行时间重调度请求体
     */
    @Data
    public static class RescheduleRequest {
        private String scheduledTime;
        private String comment;
    }

    /**
     * 主动终止工单（所有人均可终止进行中的工单，终止审批与执行流程）
     */
    @Operation(summary = "终止工单", description = "操作人主动终止工单，停止后续审批与执行")
    @PostMapping("/{id}/terminate")
    public Result<SqlTicket> terminateTicket(@PathVariable("id") Long id,
                                             @RequestBody(required = false) TerminateRequest request) {
        String operatorIdCard = (String) SecurityContextHolder.getContext().getAuthentication().getPrincipal();
        String reason = (request != null && request.getReason() != null) ? request.getReason() : "业务需要终止工单";
        SqlTicket ticket = ticketService.terminateTicket(id, operatorIdCard, reason);
        return Result.success(ticket);
    }

    /**
     * 申请人撤回工单（返回编辑状态并终止流程）
     */
    @Operation(summary = "撤回工单", description = "工单申请人撤回处于待审批状态的工单，作废审批流并返回重新编辑")
    @PostMapping("/{id}/withdraw")
    public Result<SqlTicket> withdrawTicket(@PathVariable("id") Long id) {
        String operatorIdCard = (String) SecurityContextHolder.getContext().getAuthentication().getPrincipal();
        SqlTicket ticket = ticketService.withdrawTicket(id, operatorIdCard);
        return Result.success(ticket);
    }

    /**
     * 修改并重新提交工单（针对被驳回 REJECTED、已终止 TERMINATED 或失败 FAILED 的工单修改内容重新发起）
     */
    @Operation(summary = "修改并重新提交工单", description = "针对被驳回或已终止的工单，修改 SQL 脚本或回滚方案后重新发起审批流")
    @PostMapping("/{id}/resubmit")
    public Result<SqlTicket> resubmitTicket(@PathVariable("id") Long id,
                                            @RequestBody ResubmitRequest request) {
        String operatorIdCard = (String) SecurityContextHolder.getContext().getAuthentication().getPrincipal();
        SqlTicket ticket = ticketService.resubmitTicket(
                id,
                operatorIdCard,
                request != null ? request.getSqlText() : "",
                request != null ? request.getRollbackSqlText() : "",
                request != null ? request.getReason() : "",
                request != null ? request.getExpectedRows() : null,
                request != null ? request.getCustomFieldValues() : null
        );
        return Result.success(ticket);
    }

    /**
     * 查询工单操作日志（全流程节点追踪时间线）
     */
    @Operation(summary = "查询工单操作日志", description = "返回工单全生命周期各节点的操作记录（操作人、操作时间、节点名称、备注）")
    @GetMapping("/{id}/logs")
    public Result<List<com.wmdb.model.TicketOperationLog>> getTicketLogs(@PathVariable("id") Long id) {
        List<com.wmdb.model.TicketOperationLog> logs = ticketService.getTicketLogs(id);
        return Result.success(logs);
    }

    /**
     * 工单终止请求体
     */
    @Data
    public static class TerminateRequest {
        private String reason;
    }

    /**
     * 工单重新提交请求体
     */
    @Data
    public static class ResubmitRequest {
        private String sqlText;
        private String rollbackSqlText;
        private String reason;
        private Integer expectedRows;
        private String customFieldValues;
    }
}
