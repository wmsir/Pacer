package com.wmdb.controller;

import com.wmdb.common.Result;
import com.wmdb.service.DashboardService;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RestController;

import java.util.Map;

/**
 * Dashboard 控制器
 * <p>
 * 提供首页大盘指标统计数据。
 * </p>
 *
 * @author wm
 * @date 2023-10-25
 */
@RestController
@RequestMapping("/api/v1/dashboard")
public class DashboardController {

    private final DashboardService dashboardService;

    public DashboardController(DashboardService dashboardService) {
        this.dashboardService = dashboardService;
    }

    /**
     * 获取大盘统计数据（按当前用户权限与资源组维度真实聚合计算）
     *
     * @return 统计数据
     */
    @GetMapping("/stats")
    public Result<Map<String, Object>> getStats() {
        String currentIdCard = null;
        try {
            currentIdCard = (String) org.springframework.security.core.context.SecurityContextHolder.getContext().getAuthentication().getPrincipal();
        } catch (Exception ignored) {}
        return Result.success(dashboardService.getDashboardStats(currentIdCard));
    }

    /**
     * 获取数据库实时监控指标数据（支持按指定节点或默认主节点）
     *
     * @param nodeId 节点ID（如 node-101-35-100-169, node-39-97-158-22）
     * @return 监控数据
     */
    @GetMapping("/monitor")
    public Result<Map<String, Object>> getMonitorStats(@org.springframework.web.bind.annotation.RequestParam(value = "nodeId", required = false) String nodeId) {
        String currentIdCard = null;
        try {
            currentIdCard = (String) org.springframework.security.core.context.SecurityContextHolder.getContext().getAuthentication().getPrincipal();
        } catch (Exception ignored) {}
        return Result.success(dashboardService.getDatabaseMonitorStats(nodeId, currentIdCard));
    }

    /**
     * 获取服务部署集群多节点运行状态列表
     *
     * @return 节点列表
     */
    @GetMapping("/nodes")
    public Result<java.util.List<Map<String, Object>>> getClusterNodes() {
        return Result.success(dashboardService.getClusterNodes());
    }
}
