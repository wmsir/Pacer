package com.wmdb.controller;

import com.wmdb.common.Result;
import com.wmdb.service.AiDbaService;
import lombok.Data;
import org.springframework.web.bind.annotation.*;

import java.util.Map;

/**
 * AI DBA 控制器
 *
 * @author wm
 */
@RestController
@RequestMapping("/api/v1/ai-dba")
public class AiDbaController {

    private final AiDbaService aiDbaService;

    public AiDbaController(AiDbaService aiDbaService) {
        this.aiDbaService = aiDbaService;
    }

    @PostMapping("/ask")
    public Result<String> askQuestion(@RequestBody DbaQuestionRequest request) {
        return Result.success(aiDbaService.askDatabaseQuestion(request.getQuestion()));
    }

    @GetMapping("/capacity-predict")
    public Result<Map<String, Object>> predictCapacity(@RequestParam("instanceId") Long instanceId) {
        return Result.success(aiDbaService.predictCapacity(instanceId));
    }

    @Data
    public static class DbaQuestionRequest {
        private String question;
    }
}
