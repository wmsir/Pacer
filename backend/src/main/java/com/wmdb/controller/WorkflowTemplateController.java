package com.wmdb.controller;

import com.wmdb.common.Result;
import com.wmdb.model.WorkflowTemplate;
import com.wmdb.service.WorkflowTemplateService;
import io.swagger.v3.oas.annotations.Operation;
import io.swagger.v3.oas.annotations.tags.Tag;
import lombok.RequiredArgsConstructor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.web.bind.annotation.*;

import java.util.List;

/**
 * 审批流模板控制器
 *
 * @author wm
 */
@Tag(name = "审批流模板管理", description = "提供审批流模板定义、业务资源组关联绑定与节点规则维护")
@Slf4j
@RestController
@RequestMapping("/api/v1/workflow/template")
@RequiredArgsConstructor
public class WorkflowTemplateController {

    private final WorkflowTemplateService workflowTemplateService;

    @Operation(summary = "查询审批流模板列表（全量兼容）")
    @GetMapping("/list")
    public Result<List<WorkflowTemplate>> listTemplates(@RequestParam(value = "keyword", required = false) String keyword) {
        return Result.success(workflowTemplateService.listTemplates(keyword));
    }

    @Operation(summary = "分页查询审批流模板列表")
    @GetMapping("/page")
    public Result<com.wmdb.model.PageResultDTO<WorkflowTemplate>> pageTemplates(
            @RequestParam(value = "page", defaultValue = "1") int page,
            @RequestParam(value = "size", defaultValue = "10") int size,
            @RequestParam(value = "keyword", required = false) String keyword,
            @RequestParam(value = "flowType", required = false) String flowType) {
        return Result.success(workflowTemplateService.pageTemplates(page, size, keyword, flowType));
    }

    @Operation(summary = "保存审批流模板（新建或编辑）")
    @PostMapping("/save")
    public Result<Void> saveTemplate(@RequestBody WorkflowTemplate template) {
        workflowTemplateService.saveTemplate(template);
        return Result.success(null);
    }

    @Operation(summary = "删除审批流模板")
    @DeleteMapping("/{id:\\d+}")
    public Result<Void> deleteTemplate(@PathVariable("id") Long id) {
        workflowTemplateService.deleteTemplate(id);
        return Result.success(null);
    }

    @Operation(summary = "切换审批流模板启用状态")
    @PostMapping("/{id:\\d+}/toggle-status")
    public Result<Void> toggleStatus(@PathVariable("id") Long id) {
        workflowTemplateService.toggleStatus(id);
        return Result.success(null);
    }

    @Operation(summary = "预估工单路由的审批流模板与节点链路", description = "根据资源组、工单类型、目标实例（含固定流与标签）及影响行数计算匹配的审批流")
    @PostMapping("/preview-routing")
    public Result<com.wmdb.model.RoutingPreviewDTO> previewRouting(@RequestBody com.wmdb.model.RoutingPreviewRequestDTO request) {
        return Result.success(workflowTemplateService.previewRouting(request));
    }

    @Operation(summary = "SpEL 条件规则在线沙箱测试与求值", description = "支持输入 SpEL 表达式与模拟上下文变量测试判定结果与语法合法性")
    @PostMapping("/evaluate-spel")
    public Result<com.wmdb.model.SpelEvaluationResultDTO> evaluateSpel(@RequestBody com.wmdb.model.SpelEvaluationRequestDTO request) {
        return Result.success(workflowTemplateService.evaluateSpel(request));
    }
}
