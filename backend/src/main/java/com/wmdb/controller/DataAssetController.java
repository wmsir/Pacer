package com.wmdb.controller;

import com.wmdb.common.Result;
import com.wmdb.service.DataAssetService;
import org.springframework.web.bind.annotation.*;

import java.util.List;
import java.util.Map;

/**
 * 数据资产控制器
 *
 * @author wm
 */
@RestController
@RequestMapping("/api/v1/asset")
public class DataAssetController {

    private final DataAssetService dataAssetService;

    public DataAssetController(DataAssetService dataAssetService) {
        this.dataAssetService = dataAssetService;
    }

    @GetMapping("/overview")
    public Result<Map<String, Object>> getOverview() {
        return Result.success(dataAssetService.getAssetOverview());
    }

    @GetMapping("/lineage")
    public Result<List<Map<String, String>>> getLineage(@RequestParam("tableName") String tableName) {
        return Result.success(dataAssetService.analyzeDataLineage(tableName));
    }
}
