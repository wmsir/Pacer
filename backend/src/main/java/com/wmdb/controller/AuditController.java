package com.wmdb.controller;

import com.wmdb.common.Result;
import com.wmdb.service.AuditService;
import io.swagger.v3.oas.annotations.Operation;
import io.swagger.v3.oas.annotations.tags.Tag;
import jakarta.servlet.http.HttpServletResponse;
import lombok.RequiredArgsConstructor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.web.bind.annotation.*;

import java.util.Map;

/**
 * SQL 审计与合规中心控制器
 *
 * @author wm
 */
@Tag(name = "SQL 审计与安全合规中心", description = "提供 SQL 审计日志检索、安全合规大屏聚合统计及审计报表一键导出功能")
@Slf4j
@RestController
@RequestMapping("/api/v1/audit")
@RequiredArgsConstructor
public class AuditController {

    private final AuditService auditService;

    @Operation(summary = "多维度检索 SQL 审计日志列表")
    @GetMapping("/logs")
    public Result<Map<String, Object>> listAuditLogs(
            @RequestParam(value = "keyword", required = false) String keyword,
            @RequestParam(value = "status", required = false) String status,
            @RequestParam(value = "ticketId", required = false) Long ticketId,
            @RequestParam(value = "minDuration", required = false) Long minDuration,
            @RequestParam(value = "maxDuration", required = false) Long maxDuration,
            @RequestParam(value = "page", defaultValue = "1") Integer page,
            @RequestParam(value = "size", defaultValue = "20") Integer size) {
        return Result.success(auditService.listAuditLogs(keyword, status, ticketId, minDuration, maxDuration, page, size));
    }

    @Operation(summary = "获取企业数据安全与 SQL 合规分析大屏指标")
    @GetMapping("/dashboard-stats")
    public Result<Map<String, Object>> getDashboardStats() {
        return Result.success(auditService.getDashboardStats());
    }

    @Operation(summary = "一键导出 SQL 审计日志为标准 CSV 文件")
    @GetMapping("/export")
    public void exportAuditLogs(
            @RequestParam(value = "keyword", required = false) String keyword,
            @RequestParam(value = "status", required = false) String status,
            @RequestParam(value = "ticketId", required = false) Long ticketId,
            HttpServletResponse response) {
        auditService.exportAuditLogs(keyword, status, ticketId, response);
    }

    @Operation(summary = "多维度检索在线只读查询与 EXPLAIN 审计日志")
    @GetMapping("/query-logs")
    public Result<Map<String, Object>> listQueryAuditLogs(
            @RequestParam(value = "keyword", required = false) String keyword,
            @RequestParam(value = "opType", required = false) String opType,
            @RequestParam(value = "dbName", required = false) String dbName,
            @RequestParam(value = "status", required = false) String status,
            @RequestParam(value = "username", required = false) String username,
            @RequestParam(value = "minDuration", required = false) Long minDuration,
            @RequestParam(value = "maxDuration", required = false) Long maxDuration,
            @RequestParam(value = "page", defaultValue = "1") Integer page,
            @RequestParam(value = "size", defaultValue = "20") Integer size) {
        return Result.success(auditService.listQueryAuditLogs(keyword, opType, dbName, status, username, minDuration, maxDuration, page, size));
    }

    @Operation(summary = "获取在线查询审计策略配置")
    @GetMapping("/query-config")
    public Result<com.wmdb.service.QueryAuditConfigService.AuditPolicyConfig> getQueryAuditConfig() {
        return Result.success(auditService.getQueryAuditConfig());
    }

    @Operation(summary = "更新在线查询审计策略配置")
    @PostMapping("/query-config")
    public Result<String> updateQueryAuditConfig(@RequestBody com.wmdb.service.QueryAuditConfigService.AuditPolicyConfig config) {
        auditService.updateQueryAuditConfig(config);
        return Result.success("查询审计策略已成功更新生效");
    }
}

