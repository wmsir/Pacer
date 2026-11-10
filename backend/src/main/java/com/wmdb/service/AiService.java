package com.wmdb.service;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.wmdb.model.AiModelConfigDTO.ProviderDetail;
import lombok.extern.slf4j.Slf4j;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.http.HttpEntity;
import org.springframework.http.HttpHeaders;
import org.springframework.http.MediaType;
import org.springframework.http.ResponseEntity;
import org.springframework.http.client.SimpleClientHttpRequestFactory;
import org.springframework.stereotype.Service;
import org.springframework.web.client.HttpClientErrorException;
import org.springframework.web.client.HttpServerErrorException;
import org.springframework.web.client.RestTemplate;

import java.util.HashMap;
import java.util.List;
import java.util.Map;

/**
 * AI 智能治理服务
 * <p>
 * 提供 Text2SQL、自动巡检、异常预测、智能审查等核心 AI Agent 功能。
 * 底层动态对接并复用 AiConfigService 配置的活跃大模型通道（如 DeepSeek / Qwen / OpenAI / GLM / Ollama / 自定义模型）。
 * </p>
 *
 * @author wm
 */
@Slf4j
@Service
public class AiService {

    @Value("${wmdb.ai.api-key:mock-api-key}")
    private String defaultApiKey;

    @Value("${wmdb.ai.endpoint:https://api.deepseek.com/chat/completions}")
    private String defaultApiEndpoint;

    @Value("${wmdb.ai.model:deepseek-chat}")
    private String defaultModelName;

    private final AiConfigService aiConfigService;
    private final ObjectMapper objectMapper = new ObjectMapper();

    public AiService(AiConfigService aiConfigService) {
        this.aiConfigService = aiConfigService;
    }

    /**
     * 基础大模型调用通道 (动态读取当前激活的模型与 API Key)
     */
    public String callLlm(String systemPrompt, String userMessage) {
        ProviderDetail provider = aiConfigService.getActiveProviderDetail();
        
        String endpoint = (provider != null && provider.getEndpoint() != null && !provider.getEndpoint().isEmpty()) 
                ? provider.getEndpoint() : defaultApiEndpoint;
        String model = (provider != null && provider.getModel() != null && !provider.getModel().isEmpty()) 
                ? provider.getModel() : defaultModelName;
        String apiKey = (provider != null && provider.getApiKey() != null) ? provider.getApiKey().trim() : "";

        // 如果用户在 yml 中配置了非 mock 的 key 且配置中心为空，则降级使用 yml
        if (apiKey.isEmpty() && !"mock-api-key".equals(defaultApiKey)) {
            apiKey = defaultApiKey;
        }

        boolean isOllama = provider != null && "ollama".equalsIgnoreCase(provider.getProviderId());

        // 如果没有配置有效的 API Key 且不是本地免密 Ollama，给出友好提示
        if (!isOllama && (apiKey.isEmpty() || "mock-api-key".equals(apiKey))) {
            return "{\"content\": \"(AI 提示) 当前大模型【" + (provider != null ? provider.getProviderName() : "DeepSeek") + "】尚未配置真实 API Key。请前往【系统与流程 -> AI 模型配置】页面配置 API Key 并测试连通性，即可激活实时大模型智能治理能力。\"}";
        }

        int timeoutSec = (provider != null && provider.getTimeoutSeconds() != null && provider.getTimeoutSeconds() > 0) 
                ? provider.getTimeoutSeconds() : 30;

        SimpleClientHttpRequestFactory factory = new SimpleClientHttpRequestFactory();
        factory.setConnectTimeout(timeoutSec * 1000);
        factory.setReadTimeout(timeoutSec * 1000);
        RestTemplate restTemplate = new RestTemplate(factory);

        HttpHeaders headers = new HttpHeaders();
        headers.setContentType(MediaType.APPLICATION_JSON);
        if (!apiKey.isEmpty()) {
            headers.setBearerAuth(apiKey);
        }

        Map<String, Object> requestBody = new HashMap<>();
        requestBody.put("model", model);
        requestBody.put("messages", List.of(
                Map.of("role", "system", "content", systemPrompt),
                Map.of("role", "user", "content", userMessage)
        ));
        if (provider != null && provider.getTemperature() != null) {
            requestBody.put("temperature", provider.getTemperature());
        }
        if (provider != null && provider.getMaxTokens() != null) {
            requestBody.put("max_tokens", provider.getMaxTokens());
        }

        try {
            HttpEntity<Map<String, Object>> request = new HttpEntity<>(requestBody, headers);
            ResponseEntity<String> response = restTemplate.postForEntity(endpoint, request, String.class);
            JsonNode root = objectMapper.readTree(response.getBody());
            JsonNode choices = root.path("choices");
            if (choices.isArray() && choices.size() > 0) {
                return choices.get(0).path("message").path("content").asText();
            }
            return response.getBody();
        } catch (HttpClientErrorException e) {
            int code = e.getStatusCode().value();
            log.warn("大模型调用返回客户端异常 HTTP {}: {}", code, e.getResponseBodyAsString());
            if (code == 401) {
                return "【AI 调用异常 401】API Key 认证失败或已失效，请在【AI 模型配置】页面重新配置有效的 Key。";
            } else if (code == 429) {
                return "【AI 调用异常 429】大模型调用频次超限或账户余额不足，请稍后重试或更换模型。";
            }
            return "【AI 调用异常】HTTP " + code + ": " + e.getMessage();
        } catch (HttpServerErrorException e) {
            log.error("大模型服务端内部异常 HTTP {}: {}", e.getStatusCode().value(), e.getResponseBodyAsString());
            return "【AI 远端异常】大模型平台服务端暂时不可用 (HTTP " + e.getStatusCode().value() + ")，请稍后重试。";
        } catch (Exception e) {
            log.error("LLM 调用失败: {}", e.getMessage());
            return "【AI 服务连接失败】" + e.getMessage() + "。请检查【AI 模型配置】中的端点与网络连通性。";
        }
    }

    /**
     * Text2SQL: 将自然语言转换为 SQL
     */
    public Map<String, Object> textToSql(String prompt, String dbType) {
        String sysPrompt = "你是一个精通 " + dbType + " 的高级数据库工程师。请将用户的自然语言意图转换为 SQL 语句。请严格以 JSON 格式返回，包含 'sql' 和 'explanation' 两个字段。";
        String response = callLlm(sysPrompt, prompt);

        // Fallback if unconfigured
        if (response.contains("(AI 提示)") || response.contains("未配置真实 API Key")) {
            Map<String, Object> result = new HashMap<>();
            result.put("sql", "SELECT * FROM dual; /* 请在【AI 模型配置】中配置真实 API Key */");
            result.put("explanation", response);
            return result;
        }

        try {
            // 提取可能被 markdown 代码块包裹的 JSON
            String jsonStr = response.trim();
            if (jsonStr.startsWith("```json")) {
                jsonStr = jsonStr.substring(7);
            }
            if (jsonStr.startsWith("```")) {
                jsonStr = jsonStr.substring(3);
            }
            if (jsonStr.endsWith("```")) {
                jsonStr = jsonStr.substring(0, jsonStr.length() - 3);
            }
            return objectMapper.readValue(jsonStr.trim(), Map.class);
        } catch (Exception e) {
            return Map.of("sql", "", "explanation", response);
        }
    }

    /**
     * 智能 SQL 优化建议
     */
    public String optimizeSql(String sql) {
        String sysPrompt = "你是一位资深的 DBA 专家。请对以下 SQL 进行深度性能审查，指出其中可能存在的全表扫描、隐式转换、索引失效、死锁风险或不规范写法，并给出优化后的 SQL 语句与建议。";
        return callLlm(sysPrompt, sql);
    }

    /**
     * SQL 解释
     */
    public String explainSql(String sql) {
        String sysPrompt = "你是一位数据库技术专家。请通俗易懂地解释以下 SQL 语句的业务意图、涉及的表和字段以及执行逻辑。";
        return callLlm(sysPrompt, sql);
    }

    /**
     * SQL 重写
     */
    public String rewriteSql(String sql) {
        String sysPrompt = "你是一位数据库开发与架构专家。请将以下 SQL 重写为更加规范、执行性能更好且语义完全等价的 SQL 语句，并附带重写理由。只返回重写后的 SQL 和简短说明。";
        return callLlm(sysPrompt, sql);
    }

    /**
     * SQL 风险分析
     */
    public String riskAnalyzeSql(String sql) {
        String sysPrompt = "你是一位数据库安全与合规专家。请对以下 SQL 进行严格的安全与风险审计，重点分析是否存在 SQL 注入风险、越权访问、删除全表、无 WHERE 条件 UPDATE/DELETE、大表 DDL 锁表等高危操作，并给出安全评级与防范建议。";
        return callLlm(sysPrompt, sql);
    }

    /**
     * 执行计划解释
     */
    public String explainExecutionPlan(String planStr) {
        String sysPrompt = "你是一位数据库性能调优专家。用户会提供一段数据库执行计划（EXPLAIN 输出），请逐行通俗地解释其执行过程，指出其中的性能瓶颈（如 Using filesort, Using temporary, type=ALL 全表扫描等），并给出针对性的索引添加或 SQL 改造建议。";
        return callLlm(sysPrompt, planStr);
    }

    /**
     * 异常诊断与故障分析
     */
    public String diagnoseError(String errorLog) {
        String sysPrompt = "你是一个数据库故障排查专家。请分析这段数据库错误日志，直接指出根本原因（如死锁、OOM、主从同步延迟、网络分区等），并给出实操性强的排查与解决建议。";
        return callLlm(sysPrompt, errorLog);
    }

    /**
     * 数据库巡检 (AI DBA)
     */
    public String inspectDatabase(String metricsJson) {
        String sysPrompt = "你是一个专业 AI DBA。以下是一份数据库实时监控指标（连接数、CPU、QPS/TPS、慢查询等）。请进行深度分析，评估当前健康度，并预测未来是否存在容量瓶颈。";
        return callLlm(sysPrompt, metricsJson);
    }
}
