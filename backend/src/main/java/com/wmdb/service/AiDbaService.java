package com.wmdb.service;

import lombok.extern.slf4j.Slf4j;
import org.springframework.stereotype.Service;

import java.util.HashMap;
import java.util.Map;

/**
 * AI DBA 高阶服务
 * <p>
 * 提供数据库知识库问答、Copilot、容量预测、自动巡检报告生成等高阶 AI 能力。
 * </p>
 *
 * @author wm
 */
@Slf4j
@Service
public class AiDbaService {

    private final AiService aiService;

    public AiDbaService(AiService aiService) {
        this.aiService = aiService;
    }

    /**
     * 数据库问答 (AI Copilot / 知识库问答)
     */
    public String askDatabaseQuestion(String question) {
        log.info("AI DBA Received Question: {}", question);
        // 复用 AiService 的底层通道，封装特定的系统提示词
        String sysPrompt = "你是一个全能的 AI DBA (Database Administrator) 助手，具备深厚的 Oracle, MySQL, PostgreSQL 等数据库知识。请专业、准确地回答用户的数据库技术问题。";
        // 这里为了简化演示，直接在内部 mock
        return "模拟回答：" + question + "。您可以参考官方文档进行调优。";
    }

    /**
     * 容量预测
     */
    public Map<String, Object> predictCapacity(Long instanceId) {
        log.info("Predicting capacity for instance {}", instanceId);
        Map<String, Object> result = new HashMap<>();
        result.put("currentUsage", "75%");
        result.put("estimatedFullDate", "2024-05-15");
        result.put("suggestion", "建议在下个季度初扩容 500GB 磁盘空间。");
        return result;
    }
}
