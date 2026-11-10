package com.wmdb.controller;

import com.wmdb.common.Result;
import com.wmdb.model.DataMaskingRule;
import com.wmdb.model.MaskingAlgorithmDTO;
import com.wmdb.model.MaskingPreviewRequestDTO;
import com.wmdb.service.DataMaskingRuleService;
import io.swagger.v3.oas.annotations.Operation;
import io.swagger.v3.oas.annotations.tags.Tag;
import lombok.RequiredArgsConstructor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.web.bind.annotation.*;

import java.util.List;

/**
 * 数据脱敏规则管理控制器
 *
 * @author wm
 */
@Tag(name = "数据脱敏配置", description = "提供字段级动态脱敏规则配置、算法清单、实时效果预览与规则管理")
@Slf4j
@RestController
@RequestMapping("/api/v1/masking-rule")
@RequiredArgsConstructor
public class DataMaskingRuleController {

    private final DataMaskingRuleService dataMaskingRuleService;

    @Operation(summary = "获取支持的脱敏算法清单与样例")
    @GetMapping("/algorithms")
    public Result<List<MaskingAlgorithmDTO>> listSupportedAlgorithms() {
        return Result.success(dataMaskingRuleService.listSupportedAlgorithms());
    }

    @Operation(summary = "查询指定表或实例的脱敏规则")
    @GetMapping("/list")
    public Result<List<DataMaskingRule>> listRules(@RequestParam(value = "instanceId", required = false) Long instanceId,
                                                   @RequestParam(value = "dbName", required = false) String dbName,
                                                   @RequestParam(value = "tableName", required = false) String tableName) {
        return Result.success(dataMaskingRuleService.listRules(instanceId, dbName, tableName));
    }

    @Operation(summary = "全局检索所有脱敏规则（全量兼容）")
    @GetMapping("/all")
    public Result<List<DataMaskingRule>> listAllRules(@RequestParam(value = "keyword", required = false) String keyword) {
        return Result.success(dataMaskingRuleService.listAllRules(keyword));
    }

    @Operation(summary = "分页查询数据脱敏规则列表")
    @GetMapping("/page")
    public Result<com.wmdb.model.PageResultDTO<DataMaskingRule>> pageRules(
            @RequestParam(value = "page", defaultValue = "1") int page,
            @RequestParam(value = "size", defaultValue = "10") int size,
            @RequestParam(value = "keyword", required = false) String keyword,
            @RequestParam(value = "instanceId", required = false) Long instanceId,
            @RequestParam(value = "dbName", required = false) String dbName) {
        return Result.success(dataMaskingRuleService.pageRules(page, size, keyword, instanceId, dbName));
    }

    @Operation(summary = "保存/批量保存表字段的脱敏规则")
    @PostMapping("/save")
    public Result<Void> saveRules(@RequestBody List<DataMaskingRule> rules) {
        dataMaskingRuleService.saveRules(rules);
        return Result.success(null);
    }

    @Operation(summary = "删除脱敏规则")
    @DeleteMapping("/{id:\\d+}")
    public Result<Void> deleteRule(@PathVariable("id") Long id) {
        dataMaskingRuleService.deleteRule(id);
        return Result.success(null);
    }

    @Operation(summary = "切换脱敏规则启用/停用状态")
    @PostMapping("/{id:\\d+}/toggle-status")
    public Result<Void> toggleStatus(@PathVariable("id") Long id) {
        dataMaskingRuleService.toggleStatus(id);
        return Result.success(null);
    }

    @Operation(summary = "实时测试脱敏算法转换效果")
    @PostMapping("/preview")
    public Result<String> previewMasking(@RequestBody MaskingPreviewRequestDTO request) {
        return Result.success(dataMaskingRuleService.previewMasking(request));
    }
}
