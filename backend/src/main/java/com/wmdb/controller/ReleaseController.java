package com.wmdb.controller;

import com.wmdb.common.Result;
import com.wmdb.service.ReleaseService;
import lombok.Data;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RestController;

import java.util.Map;

/**
 * 数据库发布平台控制器
 *
 * @author wm
 */
@RestController
@RequestMapping("/api/v1/release")
public class ReleaseController {

    private final ReleaseService releaseService;

    public ReleaseController(ReleaseService releaseService) {
        this.releaseService = releaseService;
    }

    @PostMapping("/canary")
    public Result<Map<String, Object>> canaryRelease(@RequestBody ReleaseRequest request) {
        return Result.success(releaseService.canaryRelease(request.getTicketId(), request.getPercentage()));
    }

    @PostMapping("/blue-green")
    public Result<Map<String, Object>> blueGreenDeploy(@RequestBody ReleaseRequest request) {
        return Result.success(releaseService.blueGreenDeployment(request.getTicketId(), request.getTargetEnv()));
    }

    @org.springframework.web.bind.annotation.GetMapping("/topology")
    public Result<Map<String, Object>> getTopology(@org.springframework.web.bind.annotation.RequestParam("instanceId") Long instanceId) {
        return Result.success(releaseService.getReplicationTopology(instanceId));
    }

    @PostMapping("/rollback")
    public Result<Map<String, Object>> rollback(@RequestBody ReleaseRequest request) {
        return Result.success(releaseService.rollbackRelease(request.getReleaseId()));
    }

    @Data
    public static class ReleaseRequest {
        private Long ticketId;
        private int percentage;
        private String targetEnv;
        private String releaseId;
    }
}
