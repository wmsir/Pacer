package com.wmdb.controller;

import com.wmdb.common.Result;
import com.wmdb.service.DatabaseInspectionService;
import org.springframework.web.bind.annotation.*;

import java.util.Map;

/**
 * 数据库巡检控制器
 *
 * @author wm
 */
@RestController
@RequestMapping("/api/v1/inspection")
public class DatabaseInspectionController {

    private final DatabaseInspectionService inspectionService;

    public DatabaseInspectionController(DatabaseInspectionService inspectionService) {
        this.inspectionService = inspectionService;
    }

    @PostMapping("/generate")
    public Result<Map<String, Object>> generateReport(@RequestParam("instanceId") Long instanceId) {
        return Result.success(inspectionService.generateInspectionReport(instanceId));
    }
}
