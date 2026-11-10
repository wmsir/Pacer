package com.wmdb.service;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.wmdb.model.AiModelConfigDTO;
import com.wmdb.model.AiModelConfigDTO.ProviderDetail;
import com.wmdb.model.AiModelConfigDTO.TestConnectionRequest;
import com.wmdb.model.AiModelConfigDTO.TestConnectionResult;
import lombok.extern.slf4j.Slf4j;
import org.springframework.http.HttpEntity;
import org.springframework.http.HttpHeaders;
import org.springframework.http.MediaType;
import org.springframework.http.ResponseEntity;
import org.springframework.http.client.SimpleClientHttpRequestFactory;
import org.springframework.stereotype.Service;
import org.springframework.web.client.HttpClientErrorException;
import org.springframework.web.client.HttpServerErrorException;
import org.springframework.web.client.ResourceAccessException;
import org.springframework.web.client.RestTemplate;

import java.util.*;
import java.util.concurrent.ConcurrentHashMap;

/**
 * AI 大模型多提供商配置与健康诊断管理服务
 *
 * @author wm
 */
@Slf4j
@Service
public class AiConfigService {

    private final Map<String, ProviderDetail> providerStore = new ConcurrentHashMap<>();
    private volatile String activeProvider = "deepseek";
    private volatile boolean enabled = true;

    private final ObjectMapper objectMapper = new ObjectMapper();

    public AiConfigService() {
        initDefaultProviders();
    }

    private void initDefaultProviders() {
        // 1. DeepSeek (深度求索)
        providerStore.put("deepseek", ProviderDetail.builder()
                .providerId("deepseek")
                .providerName("DeepSeek (深度求索)")
                .icon("⚡")
                .endpoint("https://api.deepseek.com/chat/completions")
                .model("deepseek-chat")
                .apiKey("")
                .temperature(0.3)
                .maxTokens(4096)
                .timeoutSeconds(30)
                .presetModels(List.of("deepseek-chat", "deepseek-reasoner"))
                .officialWebsite("https://www.deepseek.com")
                .apiKeyDocUrl("https://platform.deepseek.com/api_keys")
                .description("国产高性能推理与代码生成大模型 (DeepSeek-V3 / R1)，在 SQL 调优与治理领域表现顶尖。")
                .isCustom(false)
                .build());

        // 2. 通义千问 (Qwen / 阿里百炼)
        providerStore.put("qwen", ProviderDetail.builder()
                .providerId("qwen")
                .providerName("通义千问 (Qwen / 阿里百炼)")
                .icon("☁️")
                .endpoint("https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions")
                .model("qwen-plus")
                .apiKey("")
                .temperature(0.3)
                .maxTokens(4096)
                .timeoutSeconds(30)
                .presetModels(List.of("qwen-plus", "qwen-max", "qwen-turbo", "qwen-coder-plus", "qwen2.5-coder-32b-instruct"))
                .officialWebsite("https://dashscope.aliyun.com")
                .apiKeyDocUrl("https://dashscope.console.aliyun.com/apiKey")
                .description("阿里云官方大模型，代码理解与超长上下文处理能力优异。")
                .isCustom(false)
                .build());

        // 3. OpenAI (ChatGPT / GPT-4o)
        providerStore.put("openai", ProviderDetail.builder()
                .providerId("openai")
                .providerName("OpenAI (ChatGPT / GPT-4o)")
                .icon("🌐")
                .endpoint("https://api.openai.com/v1/chat/completions")
                .model("gpt-4o-mini")
                .apiKey("")
                .temperature(0.3)
                .maxTokens(4096)
                .timeoutSeconds(30)
                .presetModels(List.of("gpt-4o", "gpt-4o-mini", "gpt-4-turbo", "gpt-3.5-turbo"))
                .officialWebsite("https://openai.com")
                .apiKeyDocUrl("https://platform.openai.com/api-keys")
                .description("全球通用大模型标杆，具备强大的逻辑推理与 SQL 解析生成能力。")
                .isCustom(false)
                .build());

        // 4. 智谱清言 (GLM / BigModel)
        providerStore.put("zhipu", ProviderDetail.builder()
                .providerId("zhipu")
                .providerName("智谱清言 (GLM-4)")
                .icon("🔮")
                .endpoint("https://open.bigmodel.cn/api/paas/v4/chat/completions")
                .model("glm-4-flash")
                .apiKey("")
                .temperature(0.3)
                .maxTokens(4096)
                .timeoutSeconds(30)
                .presetModels(List.of("glm-4-plus", "glm-4-air", "glm-4-flash", "glm-4"))
                .officialWebsite("https://open.bigmodel.cn")
                .apiKeyDocUrl("https://open.bigmodel.cn/usercenter/apikeys")
                .description("清华系智谱 AI 开放平台，提供高性价比的 GLM-4 旗舰模型。")
                .isCustom(false)
                .build());

        // 5. Moonshot Kimi (月之暗面)
        providerStore.put("moonshot", ProviderDetail.builder()
                .providerId("moonshot")
                .providerName("Moonshot Kimi (月之暗面)")
                .icon("🌙")
                .endpoint("https://api.moonshot.cn/v1/chat/completions")
                .model("moonshot-v1-8k")
                .apiKey("")
                .temperature(0.3)
                .maxTokens(4096)
                .timeoutSeconds(30)
                .presetModels(List.of("moonshot-v1-8k", "moonshot-v1-32k", "moonshot-v1-128k"))
                .officialWebsite("https://www.moonshot.cn")
                .apiKeyDocUrl("https://platform.moonshot.cn/console/api-keys")
                .description("支持超长 Prompt 上下文与复杂复杂长文本数据治理分析。")
                .isCustom(false)
                .build());

        // 6. 百度千帆 (ERNIE / 文心一言)
        providerStore.put("baidu", ProviderDetail.builder()
                .providerId("baidu")
                .providerName("百度千帆 (文心一言)")
                .icon("🐻")
                .endpoint("https://qianfan.baidubce.com/v2/chat/completions")
                .model("ernie-4.0-8k-latest")
                .apiKey("")
                .temperature(0.3)
                .maxTokens(4096)
                .timeoutSeconds(30)
                .presetModels(List.of("ernie-4.0-8k-latest", "ernie-3.5-8k", "ernie-speed-128k"))
                .officialWebsite("https://cloud.baidu.com/product/wenxinworkshop")
                .apiKeyDocUrl("https://console.bce.baidu.com/qianfan/ais/console/onlineActive")
                .description("百度文心大模型企业级平台，中文领域合规与语义理解能力全面。")
                .isCustom(false)
                .build());

        // 7. Ollama (本地私有化大模型)
        providerStore.put("ollama", ProviderDetail.builder()
                .providerId("ollama")
                .providerName("Ollama (本地私有化部署)")
                .icon("🦙")
                .endpoint("http://localhost:11434/v1/chat/completions")
                .model("qwen2.5-coder:7b")
                .apiKey("ollama-local")
                .temperature(0.3)
                .maxTokens(4096)
                .timeoutSeconds(45)
                .presetModels(List.of("qwen2.5-coder:7b", "deepseek-r1:7b", "deepseek-r1:14b", "llama3.1:8b", "codellama"))
                .officialWebsite("https://ollama.com")
                .apiKeyDocUrl("https://ollama.com/download")
                .description("企业内网私有化本地部署，离线运行无需外网，数据绝对保密。")
                .isCustom(false)
                .build());

        // 8. 自定义大模型 (OpenAI-Compatible Custom LLM)
        providerStore.put("custom", ProviderDetail.builder()
                .providerId("custom")
                .providerName("自定义大模型 (OpenAI-Compatible)")
                .icon("⚙️")
                .endpoint("https://api.your-custom-llm.com/v1/chat/completions")
                .model("custom-llm")
                .apiKey("")
                .temperature(0.3)
                .maxTokens(4096)
                .timeoutSeconds(30)
                .presetModels(List.of("custom-llm", "vllm-model", "azure-openai", "oneapi-proxy"))
                .officialWebsite("")
                .apiKeyDocUrl("")
                .description("兼容 OpenAI 协议的任意私有网关 (vLLM, OneAPI, Azure OpenAI, LocalAI, Dify 等)。")
                .isCustom(true)
                .build());
    }

    /**
     * 获取全量配置（可控制是否对 API Key 进行脱敏）
     */
    public AiModelConfigDTO getConfig(boolean maskApiKey) {
        Map<String, ProviderDetail> copy = new LinkedHashMap<>();
        for (Map.Entry<String, ProviderDetail> entry : providerStore.entrySet()) {
            ProviderDetail d = entry.getValue();
            String key = d.getApiKey();
            boolean hasKey = key != null && !key.trim().isEmpty() && !"mock-api-key".equalsIgnoreCase(key.trim());
            
            String displayKey = key;
            if (maskApiKey && hasKey) {
                displayKey = maskKey(key);
            }

            copy.put(entry.getKey(), ProviderDetail.builder()
                    .providerId(d.getProviderId())
                    .providerName(d.getProviderName())
                    .icon(d.getIcon())
                    .endpoint(d.getEndpoint())
                    .model(d.getModel())
                    .apiKey(displayKey)
                    .hasApiKey(hasKey)
                    .temperature(d.getTemperature())
                    .maxTokens(d.getMaxTokens())
                    .timeoutSeconds(d.getTimeoutSeconds())
                    .presetModels(d.getPresetModels())
                    .officialWebsite(d.getOfficialWebsite())
                    .apiKeyDocUrl(d.getApiKeyDocUrl())
                    .description(d.getDescription())
                    .isCustom(d.getIsCustom())
                    .build());
        }

        return AiModelConfigDTO.builder()
                .activeProvider(activeProvider)
                .enabled(enabled)
                .providers(copy)
                .build();
    }

    /**
     * 保存/更新模型配置
     */
    public void updateConfig(AiModelConfigDTO newConfig) {
        if (newConfig == null) return;

        if (newConfig.getEnabled() != null) {
            this.enabled = newConfig.getEnabled();
        }

        if (newConfig.getActiveProvider() != null && !newConfig.getActiveProvider().trim().isEmpty()) {
            this.activeProvider = newConfig.getActiveProvider().trim();
        }

        if (newConfig.getProviders() != null) {
            for (Map.Entry<String, ProviderDetail> entry : newConfig.getProviders().entrySet()) {
                String pId = entry.getKey();
                ProviderDetail incoming = entry.getValue();
                ProviderDetail existing = providerStore.get(pId);

                if (existing != null) {
                    // 如果传入的 key 是掩码格式（如包含 ****），保留原有真实的 key
                    String finalKey = incoming.getApiKey();
                    if (finalKey != null && finalKey.contains("****")) {
                        finalKey = existing.getApiKey();
                    } else if (finalKey == null) {
                        finalKey = existing.getApiKey();
                    }

                    existing.setEndpoint(incoming.getEndpoint());
                    existing.setModel(incoming.getModel());
                    existing.setApiKey(finalKey);
                    if (incoming.getTemperature() != null) existing.setTemperature(incoming.getTemperature());
                    if (incoming.getMaxTokens() != null) existing.setMaxTokens(incoming.getMaxTokens());
                    if (incoming.getTimeoutSeconds() != null) existing.setTimeoutSeconds(incoming.getTimeoutSeconds());
                } else {
                    // 新增自定义提供商
                    providerStore.put(pId, incoming);
                }
            }
        }

        log.info("AI 大模型配置更新完成，当前激活模型: {}", activeProvider);
    }

    /**
     * 获取当前生效的提供商配置
     */
    public ProviderDetail getActiveProviderDetail() {
        ProviderDetail detail = providerStore.get(activeProvider);
        if (detail == null) {
            detail = providerStore.get("deepseek");
        }
        return detail;
    }

    /**
     * 连通性测试与自检
     */
    public TestConnectionResult testConnection(TestConnectionRequest req) {
        long startTime = System.currentTimeMillis();

        if (req == null || req.getEndpoint() == null || req.getEndpoint().trim().isEmpty()) {
            return TestConnectionResult.builder()
                    .success(false)
                    .latencyMs(0L)
                    .message("测试失败: 接口端点 (Endpoint) 不能为空")
                    .errorDetails("请检查 API Endpoint 配置")
                    .build();
        }

        String endpoint = req.getEndpoint().trim();
        String model = (req.getModel() != null && !req.getModel().trim().isEmpty()) ? req.getModel().trim() : "gpt-3.5-turbo";

        // 获取真实的 API Key
        String apiKey = req.getApiKey();
        if (apiKey != null && apiKey.contains("****")) {
            ProviderDetail exist = providerStore.get(req.getProviderId());
            if (exist != null) {
                apiKey = exist.getApiKey();
            }
        }
        if (apiKey == null) apiKey = "";

        // 如果是本地 Ollama，没有传 key 则使用默认标识
        if ("ollama".equalsIgnoreCase(req.getProviderId()) && apiKey.trim().isEmpty()) {
            apiKey = "ollama-local";
        }

        int timeoutSec = (req.getTimeoutSeconds() != null && req.getTimeoutSeconds() > 0) ? req.getTimeoutSeconds() : 15;

        // 构造 RestTemplate
        SimpleClientHttpRequestFactory factory = new SimpleClientHttpRequestFactory();
        factory.setConnectTimeout(timeoutSec * 1000);
        factory.setReadTimeout(timeoutSec * 1000);
        RestTemplate testRestTemplate = new RestTemplate(factory);

        HttpHeaders headers = new HttpHeaders();
        headers.setContentType(MediaType.APPLICATION_JSON);
        if (!apiKey.trim().isEmpty()) {
            headers.setBearerAuth(apiKey.trim());
        }

        // 构造标准的探测 prompt
        Map<String, Object> requestBody = new HashMap<>();
        requestBody.put("model", model);
        requestBody.put("messages", List.of(
                Map.of("role", "system", "content", "你是一个 AI 连通性检测助手。"),
                Map.of("role", "user", "content", "请回复【WMDB_AI_PONG】并附带你当前运行的模型名称和简短欢迎语。")
        ));
        requestBody.put("temperature", req.getTemperature() != null ? req.getTemperature() : 0.3);
        requestBody.put("max_tokens", req.getMaxTokens() != null ? Math.min(req.getMaxTokens(), 200) : 100);

        try {
            HttpEntity<Map<String, Object>> entity = new HttpEntity<>(requestBody, headers);
            ResponseEntity<String> response = testRestTemplate.postForEntity(endpoint, entity, String.class);
            long latency = System.currentTimeMillis() - startTime;

            if (response.getStatusCode().is2xxSuccessful() && response.getBody() != null) {
                JsonNode root = objectMapper.readTree(response.getBody());
                JsonNode choices = root.path("choices");
                String reply = "";
                if (choices.isArray() && choices.size() > 0) {
                    reply = choices.get(0).path("message").path("content").asText();
                } else {
                    reply = response.getBody();
                }

                return TestConnectionResult.builder()
                        .success(true)
                        .latencyMs(latency)
                        .model(model)
                        .responseText(reply.trim())
                        .message("🎉 AI 模型接口连通性测试通过！响应延迟 " + latency + "ms")
                        .build();
            } else {
                return TestConnectionResult.builder()
                        .success(false)
                        .latencyMs(System.currentTimeMillis() - startTime)
                        .message("测试异常: 服务端返回 HTTP " + response.getStatusCode())
                        .errorDetails(response.getBody())
                        .build();
            }
        } catch (HttpClientErrorException e) {
            long latency = System.currentTimeMillis() - startTime;
            int status = e.getStatusCode().value();
            String tip = "HTTP " + status;
            if (status == 401) {
                tip = "认证失败 (401 Unauthorized)：API Key 无效、已过期或未填写。请核对 Key 是否正确。";
            } else if (status == 403) {
                tip = "拒绝访问 (403 Forbidden)：当前账号权限不足、余额不足或触发 IP 白名单限制。";
            } else if (status == 404) {
                tip = "端点不存在 (404 Not Found)：请检查 Endpoint URL 路径是否正确（需包含 /v1/chat/completions 等完整路径）。";
            } else if (status == 429) {
                tip = "超出配额或限流 (429 Too Many Requests)：当前模型调用频次超限或账户 Token 额度已用尽。";
            }
            return TestConnectionResult.builder()
                    .success(false)
                    .latencyMs(latency)
                    .message(tip)
                    .errorDetails(e.getResponseBodyAsString())
                    .build();
        } catch (HttpServerErrorException e) {
            return TestConnectionResult.builder()
                    .success(false)
                    .latencyMs(System.currentTimeMillis() - startTime)
                    .message("大模型服务端内部错误 (HTTP " + e.getStatusCode().value() + ")，请稍后重试。")
                    .errorDetails(e.getResponseBodyAsString())
                    .build();
        } catch (ResourceAccessException e) {
            String msg = e.getMessage();
            String tip = "网络连接失败: 无法连接至 " + endpoint;
            if (msg != null && msg.contains("timed out")) {
                tip = "请求超时 (> " + timeoutSec + "s)，请检查大模型端点网络连通性或代理配置。";
            } else if (msg != null && msg.contains("Connection refused")) {
                tip = "连接被拒绝 (Connection Refused)，请确认本地 Ollama/vLLM 服务是否已在后台启动。";
            }
            return TestConnectionResult.builder()
                    .success(false)
                    .latencyMs(System.currentTimeMillis() - startTime)
                    .message(tip)
                    .errorDetails(msg)
                    .build();
        } catch (Exception e) {
            return TestConnectionResult.builder()
                    .success(false)
                    .latencyMs(System.currentTimeMillis() - startTime)
                    .message("连通性测试失败: " + e.getMessage())
                    .errorDetails(e.toString())
                    .build();
        }
    }

    private String maskKey(String key) {
        if (key == null || key.length() <= 8) return "sk-****";
        return key.substring(0, 4) + "****" + key.substring(key.length() - 4);
    }
}
