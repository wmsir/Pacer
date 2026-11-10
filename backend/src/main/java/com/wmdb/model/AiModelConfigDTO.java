package com.wmdb.model;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

import java.util.List;
import java.util.Map;

/**
 * AI 大模型多提供商配置与测试 DTO
 *
 * @author wm
 */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class AiModelConfigDTO {

    /**
     * 当前系统全局激活生效的提供商 ID (如 deepseek, qwen, openai, zhipu, custom, ollama)
     */
    private String activeProvider;

    /**
     * 是否启用 AI 智能分析总开关
     */
    private Boolean enabled;

    /**
     * 各大模型提供商的具体配置映射表
     */
    private Map<String, ProviderDetail> providers;

    @Data
    @Builder
    @NoArgsConstructor
    @AllArgsConstructor
    public static class ProviderDetail {
        private String providerId;
        private String providerName;
        private String icon;
        private String endpoint;
        private String model;
        private String apiKey;
        private Boolean hasApiKey; // 是否已配置 API Key (脱敏用)
        private Double temperature;
        private Integer maxTokens;
        private Integer timeoutSeconds;
        private List<String> presetModels;
        private String officialWebsite;
        private String apiKeyDocUrl;
        private String description;
        private Boolean isCustom;
    }

    /**
     * 连通性测试请求参数
     */
    @Data
    @NoArgsConstructor
    @AllArgsConstructor
    public static class TestConnectionRequest {
        private String providerId;
        private String endpoint;
        private String model;
        private String apiKey;
        private Double temperature;
        private Integer maxTokens;
        private Integer timeoutSeconds;
    }

    /**
     * 连通性测试响应结果
     */
    @Data
    @Builder
    @NoArgsConstructor
    @AllArgsConstructor
    public static class TestConnectionResult {
        private Boolean success;
        private Long latencyMs;
        private String responseText;
        private String model;
        private String message;
        private String errorDetails;
    }
}
