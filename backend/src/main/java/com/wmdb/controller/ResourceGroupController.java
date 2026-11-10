package com.wmdb.controller;

import com.wmdb.common.Result;
import com.wmdb.model.ResourceGroup;
import com.wmdb.service.ResourceGroupService;
import io.swagger.v3.oas.annotations.Operation;
import io.swagger.v3.oas.annotations.tags.Tag;
import lombok.RequiredArgsConstructor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.web.bind.annotation.*;

import java.util.List;

/**
 * 资源组管理控制器
 *
 * @author wm
 */
@Tag(name = "资源组管理", description = "提供业务资源组的新建、编辑、删除及列表查询")
@Slf4j
@RestController
@RequestMapping("/api/v1/resource-group")
@RequiredArgsConstructor
public class ResourceGroupController {

    private final ResourceGroupService resourceGroupService;

    @Operation(summary = "查询资源组列表（全量兼容）")
    @GetMapping("/list")
    public Result<List<ResourceGroup>> listResourceGroups(@RequestParam(value = "keyword", required = false) String keyword) {
        return Result.success(resourceGroupService.listResourceGroups(keyword));
    }

    @Operation(summary = "分页查询资源组列表")
    @GetMapping("/page")
    public Result<com.wmdb.model.PageResultDTO<ResourceGroup>> pageResourceGroups(
            @RequestParam(value = "page", defaultValue = "1") int page,
            @RequestParam(value = "size", defaultValue = "10") int size,
            @RequestParam(value = "keyword", required = false) String keyword) {
        return Result.success(resourceGroupService.pageResourceGroups(page, size, keyword));
    }

    @Operation(summary = "保存资源组（新建或修改）")
    @PostMapping("/save")
    public Result<Void> saveResourceGroup(@RequestBody ResourceGroup group) {
        resourceGroupService.saveResourceGroup(group);
        return Result.success(null);
    }

    @Operation(summary = "删除资源组")
    @DeleteMapping("/{id}")
    public Result<Void> deleteResourceGroup(@PathVariable("id") Long id) {
        resourceGroupService.deleteResourceGroup(id);
        return Result.success(null);
    }
}
