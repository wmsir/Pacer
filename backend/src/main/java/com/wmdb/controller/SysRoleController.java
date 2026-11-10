package com.wmdb.controller;

import com.wmdb.common.Result;
import com.wmdb.model.SysRole;
import com.wmdb.service.SysRoleService;
import lombok.RequiredArgsConstructor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.web.bind.annotation.*;

import java.util.List;

/**
 * 角色管理控制器
 *
 * @author wm
 * @date 2023-10-25
 */
@Slf4j
@RestController
@RequestMapping("/api/v1/role")
@RequiredArgsConstructor
public class SysRoleController {

    private final SysRoleService sysRoleService;

    @GetMapping("/list")
    public Result<List<SysRole>> listRoles() {
        return Result.success(sysRoleService.listRoles());
    }

    @GetMapping("/page")
    public Result<com.wmdb.model.PageResultDTO<SysRole>> pageRoles(
            @RequestParam(value = "page", defaultValue = "1") int page,
            @RequestParam(value = "size", defaultValue = "10") int size,
            @RequestParam(value = "keyword", required = false) String keyword) {
        return Result.success(sysRoleService.pageRoles(page, size, keyword));
    }

    @PostMapping("/save")
    public Result<Void> saveRole(@RequestBody SysRole role) {
        sysRoleService.saveRole(role);
        return Result.success(null);
    }

    @DeleteMapping("/{id}")
    public Result<Void> deleteRole(@PathVariable("id") Long id) {
        sysRoleService.deleteRole(id);
        return Result.success(null);
    }

    @GetMapping("/{roleCode}/users")
    public Result<com.wmdb.model.PageResultDTO<com.wmdb.model.SysUserDTO>> getRoleUsers(
            @PathVariable("roleCode") String roleCode,
            @RequestParam(value = "page", defaultValue = "1") int page,
            @RequestParam(value = "size", defaultValue = "10") int size,
            @RequestParam(value = "keyword", required = false) String keyword) {
        return Result.success(sysRoleService.getRoleUsers(roleCode, keyword, page, size));
    }

    @GetMapping("/{roleCode}/candidate-users")
    public Result<com.wmdb.model.PageResultDTO<com.wmdb.model.SysUserDTO>> getCandidateUsersForRole(
            @PathVariable("roleCode") String roleCode,
            @RequestParam(value = "page", defaultValue = "1") int page,
            @RequestParam(value = "size", defaultValue = "10") int size,
            @RequestParam(value = "keyword", required = false) String keyword) {
        return Result.success(sysRoleService.getCandidateUsersForRole(roleCode, keyword, page, size));
    }

    @PostMapping("/{roleCode}/users/add")
    public Result<Void> addUsersToRole(
            @PathVariable("roleCode") String roleCode,
            @RequestBody java.util.Map<String, Object> body) {
        java.util.List<Long> userIds = extractUserIds(body);
        sysRoleService.addUsersToRole(roleCode, userIds);
        return Result.success(null);
    }

    @PostMapping("/{roleCode}/users/remove")
    public Result<Void> removeUsersFromRole(
            @PathVariable("roleCode") String roleCode,
            @RequestBody java.util.Map<String, Object> body) {
        java.util.List<Long> userIds = extractUserIds(body);
        sysRoleService.removeUsersFromRole(roleCode, userIds);
        return Result.success(null);
    }

    private java.util.List<Long> extractUserIds(java.util.Map<String, Object> body) {
        java.util.List<Long> result = new java.util.ArrayList<>();
        if (body == null || !body.containsKey("userIds")) {
            return result;
        }
        Object raw = body.get("userIds");
        if (raw instanceof java.util.Collection<?> col) {
            for (Object item : col) {
                if (item instanceof Number num) {
                    result.add(num.longValue());
                } else if (item != null) {
                    try {
                        result.add(Long.parseLong(item.toString().trim()));
                    } catch (Exception ignored) {}
                }
            }
        }
        return result;
    }
}
