package com.wmdb.controller;

import com.wmdb.common.Result;
import com.wmdb.model.SysUser;
import com.wmdb.service.UserService;
import io.swagger.v3.oas.annotations.Operation;
import io.swagger.v3.oas.annotations.tags.Tag;
import lombok.RequiredArgsConstructor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.web.bind.annotation.*;

import java.util.List;
import java.util.Map;

/**
 * 用户管理控制器
 *
 * @author wm
 */
@Tag(name = "用户管理", description = "提供系统人员账号的新建、编辑、删除、启禁用及重置密码")
@Slf4j
@RestController
@RequestMapping("/api/v1/user")
@RequiredArgsConstructor
public class UserController {

    private final UserService userService;

    @Operation(summary = "查询用户列表（全量兼容）")
    @GetMapping("/list")
    public Result<List<com.wmdb.model.SysUserDTO>> listUsers(@RequestParam(value = "keyword", required = false) String keyword) {
        return Result.success(userService.listUsers(keyword));
    }

    @Operation(summary = "分页查询用户列表")
    @GetMapping("/page")
    public Result<com.wmdb.model.PageResultDTO<com.wmdb.model.SysUserDTO>> pageUsers(
            @RequestParam(value = "page", defaultValue = "1") int page,
            @RequestParam(value = "size", defaultValue = "10") int size,
            @RequestParam(value = "keyword", required = false) String keyword,
            @RequestParam(value = "role", required = false) String role,
            @RequestParam(value = "status", required = false) Integer status) {
        return Result.success(userService.pageUsers(page, size, keyword, role, status));
    }

    @Operation(summary = "保存用户（新建或编辑）")
    @PostMapping("/save")
    public Result<Void> saveUser(@RequestBody SysUser user) {
        userService.saveUser(user);
        return Result.success(null);
    }

    @Operation(summary = "删除用户")
    @DeleteMapping("/{id:\\d+}")
    public Result<Void> deleteUser(@PathVariable("id") Long id) {
        userService.deleteUser(id);
        return Result.success(null);
    }

    @Operation(summary = "切换用户状态（启用/禁用）")
    @PostMapping("/{id:\\d+}/toggle-status")
    public Result<Void> toggleStatus(@PathVariable("id") Long id) {
        userService.toggleStatus(id);
        return Result.success(null);
    }

    @Operation(summary = "重置用户密码")
    @PostMapping("/{id:\\d+}/reset-password")
    public Result<Void> resetPassword(@PathVariable("id") Long id, @RequestBody(required = false) Map<String, String> body) {
        String newPassword = body != null ? body.get("newPassword") : "123456";
        userService.resetPassword(id, newPassword);
        return Result.success(null);
    }

    @Operation(summary = "获取当前登录用户个人档案")
    @GetMapping("/profile")
    public Result<com.wmdb.model.SysUserDTO> getProfile() {
        String identifier = (String) org.springframework.security.core.context.SecurityContextHolder.getContext().getAuthentication().getPrincipal();
        return Result.success(userService.getCurrentUserProfile(identifier));
    }

    @Operation(summary = "修改当前登录用户个人资料与国内联系方式")
    @PutMapping("/profile")
    public Result<Void> updateProfile(@RequestBody com.wmdb.model.UserProfileUpdateDTO req) {
        String identifier = (String) org.springframework.security.core.context.SecurityContextHolder.getContext().getAuthentication().getPrincipal();
        userService.updateCurrentUserProfile(identifier, req);
        return Result.success(null);
    }

    @Operation(summary = "当前登录用户自主修改密码")
    @PostMapping("/change-password")
    public Result<Void> changePassword(@RequestBody com.wmdb.model.ChangePasswordRequestDTO req) {
        String identifier = (String) org.springframework.security.core.context.SecurityContextHolder.getContext().getAuthentication().getPrincipal();
        userService.changeCurrentUserPassword(identifier, req);
        return Result.success(null);
    }
}
