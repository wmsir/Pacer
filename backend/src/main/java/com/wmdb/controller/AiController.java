package com.wmdb.controller;

import com.wmdb.common.Result;
import com.wmdb.model.AiModelConfigDTO;
import com.wmdb.model.AiModelConfigDTO.TestConnectionRequest;
import com.wmdb.model.AiModelConfigDTO.TestConnectionResult;
import com.wmdb.service.AiConfigService;
import com.wmdb.service.AiService;
import io.swagger.v3.oas.annotations.Operation;
import io.swagger.v3.oas.annotations.tags.Tag;
import lombok.Data;
import org.springframework.web.bind.annotation.*;

import java.util.Map;

/**
 * AI 控制器
 * <p>
 * 对外暴露 AI 模型多提供商配置、在线连通性自检以及 Text2SQL、SQL 审查、优化、重写与执行计划解释。
 * </p>
 *
 * @author wm
 * @date 2023-10-25
 */
@Tag(name = "AI 智能分析与模型配置", description = "提供大模型多提供商配置、连通性探测自检与 Text2SQL、SQL 智能调优等功能")
@RestController
@RequestMapping("/api/v1/ai")
public class AiController {

    private final AiService aiService;
    private final AiConfigService aiConfigService;

    public AiController(AiService aiService, AiConfigService aiConfigService) {
        this.aiService = aiService;
        this.aiConfigService = aiConfigService;
    }

    /**
     * 获取全量 AI 模型配置
     */
    @Operation(summary = "获取全量 AI 模型配置 (包含 API Key 脱敏)")
    @GetMapping("/config")
    public Result<AiModelConfigDTO> getAiConfig() {
        return Result.success(aiConfigService.getConfig(true));
    }

    /**
     * 保存/更新 AI 模型配置
     */
    @Operation(summary = "保存/更新 AI 模型配置与切换激活模型")
    @PostMapping("/config")
    public Result<Void> updateAiConfig(@RequestBody AiModelConfigDTO config) {
        aiConfigService.updateConfig(config);
        return Result.success(null);
    }

    /**
     * 实时连通性探测与自检
     */
    @Operation(summary = "实时探测测试大模型接口连通性与 API Key 有效性")
    @PostMapping("/test-connection")
    public Result<TestConnectionResult> testConnection(@RequestBody TestConnectionRequest request) {
        return Result.success(aiConfigService.testConnection(request));
    }

    /**
     * 快速 AI 体验沙箱对话
     */
    @Operation(summary = "AI 快速体验沙箱对话测试")
    @PostMapping("/chat")
    public Result<String> chat(@RequestBody Map<String, String> body) {
        String prompt = body.getOrDefault("prompt", "你好，请介绍一下你自己。");
        String systemPrompt = body.getOrDefault("systemPrompt", "你是一位精通企业级数据库架构与 SQL 调优的高级专家。");
        return Result.success(aiService.callLlm(systemPrompt, prompt));
    }

    /**
     * Text2SQL
     */
    @Operation(summary = "自然语言转换为 SQL")
    @PostMapping("/text2sql")
    public Result<Map<String, Object>> textToSql(@RequestBody Text2SqlRequest request) {
        return Result.success(aiService.textToSql(request.getPrompt(), request.getDbType()));
    }

    /**
     * SQL 智能优化建议
     */
    @Operation(summary = "SQL 性能诊断与优化建议")
    @PostMapping("/optimize")
    public Result<String> optimizeSql(@RequestBody OptimizeRequest request) {
        return Result.success(aiService.optimizeSql(request.getSql()));
    }

    /**
     * SQL 解释
     */
    @Operation(summary = "SQL 语义与执行逻辑通俗解释")
    @PostMapping("/explain")
    public Result<String> explainSql(@RequestBody OptimizeRequest request) {
        return Result.success(aiService.explainSql(request.getSql()));
    }

    /**
     * SQL 重写
     */
    @Operation(summary = "SQL 高性能等价重写")
    @PostMapping("/rewrite")
    public Result<String> rewriteSql(@RequestBody OptimizeRequest request) {
        return Result.success(aiService.rewriteSql(request.getSql()));
    }

    /**
     * SQL 风险分析
     */
    @Operation(summary = "SQL 安全与风险审计分析")
    @PostMapping("/risk")
    public Result<String> riskAnalyzeSql(@RequestBody OptimizeRequest request) {
        return Result.success(aiService.riskAnalyzeSql(request.getSql()));
    }

    /**
     * 执行计划解释
     */
    @Operation(summary = "执行计划 (EXPLAIN) 逐行解析与瓶颈诊断")
    @PostMapping("/explain-plan")
    public Result<String> explainExecutionPlan(@RequestBody OptimizeRequest request) {
        return Result.success(aiService.explainExecutionPlan(request.getSql()));
    }

    @Data
    public static class Text2SqlRequest {
        private String prompt;
        private String dbType;
    }

    @Data
    public static class OptimizeRequest {
        private String sql;
    }
}
