package com.wmdb.controller;

import com.wmdb.common.Result;
import com.wmdb.service.SystemConfigService;
import io.swagger.v3.oas.annotations.Operation;
import io.swagger.v3.oas.annotations.tags.Tag;
import lombok.RequiredArgsConstructor;
import org.springframework.web.bind.annotation.*;

/**
 * 平台全局安全与审计策略控制器
 *
 * @author wm
 */
@Tag(name = "平台全局安全与审计策略配置")
@RestController
@RequestMapping("/api/v1/config")
@RequiredArgsConstructor
public class SystemConfigController {

    private final SystemConfigService systemConfigService;

    @Operation(summary = "获取全局安全策略配置")
    @GetMapping("/safety-policies")
    public Result<SystemConfigService.SafetyPolicyDTO> getSafetyPolicies() {
        return Result.success(systemConfigService.getSafetyPolicies());
    }

    @Operation(summary = "更新全局安全策略配置 (管理员/DBA权限)")
    @PostMapping("/safety-policies")
    public Result<SystemConfigService.SafetyPolicyDTO> updateSafetyPolicies(@RequestBody SystemConfigService.SafetyPolicyDTO dto) {
        systemConfigService.updateSafetyPolicies(dto);
        return Result.success(systemConfigService.getSafetyPolicies());
    }

    @Operation(summary = "获取集群执行策略配置 (默认：查询走从节点/执行走主节点)")
    @GetMapping("/execution-policy")
    public Result<SystemConfigService.ExecutionPolicyDTO> getExecutionPolicy() {
        return Result.success(systemConfigService.getExecutionPolicy());
    }

    @Operation(summary = "更新集群执行策略配置 (支持动态配置多节点路由)")
    @PostMapping("/execution-policy")
    public Result<SystemConfigService.ExecutionPolicyDTO> updateExecutionPolicy(@RequestBody SystemConfigService.ExecutionPolicyDTO dto) {
        systemConfigService.updateExecutionPolicy(dto);
        return Result.success(systemConfigService.getExecutionPolicy());
    }
}