package com.wmdb.controller;

import com.baomidou.mybatisplus.core.conditions.query.QueryWrapper;
import com.wmdb.common.Result;
import com.wmdb.mapper.SysUserMapper;
import com.wmdb.model.SysUser;
import com.wmdb.security.JwtUtils;
import com.wmdb.security.SmUtils;
import io.swagger.v3.oas.annotations.Operation;
import io.swagger.v3.oas.annotations.tags.Tag;
import jakarta.servlet.http.HttpServletRequest;
import lombok.Data;
import lombok.RequiredArgsConstructor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.jdbc.core.JdbcTemplate;
import org.springframework.web.bind.annotation.*;

import java.sql.Timestamp;
import java.time.LocalDate;
import java.time.LocalDateTime;
import java.time.format.DateTimeFormatter;
import java.util.*;

/**
 * FWPT 超级管理中心控制器
 * <p>
 * 提供全平台注册用户全息名册、跨系统在线使用轨迹与审计日志大屏数据接口。
 * </p>
 *
 * @author wm
 */
@Tag(name = "全平台超级管理中心", description = "提供用户总数查询、各系统使用记录在线轨迹、用户生命周期管控等特权接口")
@Slf4j
@RestController
@RequestMapping("/api/v1/admin")
@RequiredArgsConstructor
public class AdminController {

    private final SysUserMapper sysUserMapper;
    private final JdbcTemplate jdbcTemplate;
    private final JwtUtils jwtUtils;

    private static final DateTimeFormatter DATE_TIME_FORMATTER = DateTimeFormatter.ofPattern("yyyy-MM-dd HH:mm:ss");

    /**
     * 校验请求是否具备超级管理员权限
     */
    private boolean checkAdminPrivilege(String authHeader) {
        if (authHeader == null || authHeader.trim().isEmpty()) {
            return false;
        }
        String token = authHeader.replace("Bearer ", "").trim();
        try {
            if (!jwtUtils.isTokenValid(token)) {
                return false;
            }
            String subject = jwtUtils.extractIdCard(token);
            SysUser user = sysUserMapper.selectOne(new QueryWrapper<SysUser>()
                    .eq("username", subject)
                    .last("LIMIT 1"));
            if (user == null && subject != null && !subject.isEmpty() && !"0".equals(subject)) {
                user = sysUserMapper.selectOne(new QueryWrapper<SysUser>()
                        .eq("id_card", subject)
                        .last("LIMIT 1"));
            }
            if (user != null) {
                return "ADMIN".equalsIgnoreCase(user.getRole()) || "admin".equalsIgnoreCase(user.getUsername());
            }
            return "admin".equalsIgnoreCase(subject);
        } catch (Exception e) {
            log.warn("[AdminController] 鉴权解析失败: {}", e.getMessage());
            return false;
        }
    }

    /**
     * 1. 获取全平台注册与各系统使用概览大屏指标
     */
    @Operation(summary = "全平台数据大屏概览", description = "获取总注册用户、今日新增、各系统使用热度统计")
    @GetMapping("/overview")
    public Result<Map<String, Object>> getOverview(@RequestHeader(value = "Authorization", required = false) String authHeader) {
        if (!checkAdminPrivilege(authHeader)) {
            return Result.error("A0301", "权限不足，仅超级管理员可查阅全平台管理大屏");
        }

        Map<String, Object> data = new HashMap<>();

        // 1. 用户总量
        Long totalUsers = sysUserMapper.selectCount(new QueryWrapper<>());
        data.put("totalUsers", totalUsers != null ? totalUsers : 0);

        // 2. 今日新增用户 (MySQL created_at >= 今天 00:00:00)
        String todayStart = LocalDate.now().toString() + " 00:00:00";
        Long todayUsers = 0L;
        try {
            todayUsers = jdbcTemplate.queryForObject(
                    "SELECT COUNT(*) FROM sys_user WHERE created_at >= ?", Long.class, todayStart);
        } catch (Exception ignored) {
            todayUsers = sysUserMapper.selectCount(new QueryWrapper<SysUser>().ge("created_at", todayStart));
        }
        data.put("todayUsers", todayUsers != null ? todayUsers : 0);

        // 3. 统计各系统使用与访问轨迹记录
        Map<String, Long> systemStats = new LinkedHashMap<>();
        systemStats.put("稳骥 PacerSQL", 0L);
        systemStats.put("海马智慧门店", 0L);
        systemStats.put("聚会桌游吧", 0L);
        systemStats.put("箭头消消乐", 0L);
        systemStats.put("磐石密码本", 0L);
        systemStats.put("斑马 ERP", 0L);
        systemStats.put("开发者工具箱", 0L);

        Long totalTraces = 0L;
        try {
            List<Map<String, Object>> list = jdbcTemplate.queryForList(
                    "SELECT resource_type, COUNT(*) as cnt FROM sys_audit_log GROUP BY resource_type");
            for (Map<String, Object> row : list) {
                String type = String.valueOf(row.get("resource_type"));
                Long cnt = ((Number) row.get("cnt")).longValue();
                totalTraces += cnt;
                if (systemStats.containsKey(type)) {
                    systemStats.put(type, cnt);
                } else if (type != null && !type.equals("null")) {
                    systemStats.put(type, cnt);
                }
            }
        } catch (Exception e) {
            log.warn("[AdminController] 获取审计日志统计异常: {}", e.getMessage());
        }
        data.put("totalTraces", totalTraces);
        data.put("systemStats", systemStats);

        // 4. 活跃用户数估算
        Long activeUsers = 0L;
        try {
            activeUsers = jdbcTemplate.queryForObject(
                    "SELECT COUNT(DISTINCT username) FROM sys_audit_log WHERE created_at >= ?", Long.class, todayStart);
        } catch (Exception ignored) {}
        if (activeUsers == null || activeUsers == 0) {
            activeUsers = Math.max(1L, (todayUsers != null ? todayUsers : 0L) + 1);
        }
        data.put("activeUsers", activeUsers);

        // 5. 验证渠道统计 (邮箱核验 / 手机短信核验)
        Long emailVerifiedCount = 0L;
        Long phoneVerifiedCount = 0L;
        try {
            emailVerifiedCount = jdbcTemplate.queryForObject(
                    "SELECT COUNT(*) FROM sys_user WHERE email_verified = 1", Long.class);
            phoneVerifiedCount = jdbcTemplate.queryForObject(
                    "SELECT COUNT(*) FROM sys_user WHERE phone_verified = 1", Long.class);
        } catch (Exception ignored) {}
        data.put("emailVerifiedCount", emailVerifiedCount != null ? emailVerifiedCount : 0L);
        data.put("phoneVerifiedCount", phoneVerifiedCount != null ? phoneVerifiedCount : 0L);

        return Result.success(data);
    }

    /**
     * 2. 查询全平台注册用户明细表（支持模糊搜索与系统轨迹聚合）
     */
    @Operation(summary = "全平台注册用户全息名册", description = "按关键字检索用户名、真实姓名、邮箱、手机号及在各系统的使用概览")
    @GetMapping("/users")
    public Result<Map<String, Object>> listUsers(
            @RequestParam(value = "keyword", required = false) String keyword,
            @RequestParam(value = "status", required = false) Integer status,
            @RequestParam(value = "role", required = false) String role,
            @RequestParam(value = "verifyType", required = false) String verifyType,
            @RequestParam(value = "page", defaultValue = "1") Integer page,
            @RequestParam(value = "size", defaultValue = "20") Integer size,
            @RequestHeader(value = "Authorization", required = false) String authHeader) {

        if (!checkAdminPrivilege(authHeader)) {
            return Result.error("A0301", "权限不足，仅超级管理员可查阅全平台用户列表");
        }

        QueryWrapper<SysUser> qw = new QueryWrapper<>();
        if (keyword != null && !keyword.trim().isEmpty()) {
            String kw = keyword.trim();
            qw.and(wrapper -> wrapper.like("username", kw)
                    .or().like("real_name", kw)
                    .or().like("email", kw)
                    .or().like("phone", kw));
        }
        if (status != null) {
            qw.eq("status", status);
        }
        if (role != null && !role.trim().isEmpty()) {
            qw.eq("role", role.trim());
        }
        if (verifyType != null && !verifyType.trim().isEmpty()) {
            String vt = verifyType.trim().toUpperCase();
            if ("EMAIL".equals(vt)) {
                qw.eq("email_verified", 1);
            } else if ("PHONE".equals(vt)) {
                qw.eq("phone_verified", 1);
            } else if ("BOTH".equals(vt)) {
                qw.eq("email_verified", 1).eq("phone_verified", 1);
            } else if ("UNVERIFIED".equals(vt)) {
                qw.and(w -> w.isNull("email_verified").or().eq("email_verified", 0))
                  .and(w -> w.isNull("phone_verified").or().eq("phone_verified", 0));
            }
        }
        qw.orderByDesc("id");

        List<SysUser> userList = sysUserMapper.selectList(qw);
        int total = userList.size();

        // 内存分页
        int fromIndex = Math.max(0, (page - 1) * size);
        int toIndex = Math.min(total, fromIndex + size);
        List<SysUser> pagedUsers = (fromIndex < total) ? userList.subList(fromIndex, toIndex) : Collections.emptyList();

        List<Map<String, Object>> resultList = new ArrayList<>();
        for (SysUser u : pagedUsers) {
            Map<String, Object> map = new LinkedHashMap<>();
            map.put("id", u.getId());
            map.put("username", u.getUsername());
            map.put("realName", u.getRealName() != null ? u.getRealName() : u.getUsername());
            map.put("email", u.getEmail() != null ? u.getEmail() : "未绑定");
            map.put("phone", u.getPhone() != null ? u.getPhone() : "未绑定");
            map.put("role", u.getRole() != null ? u.getRole() : "DEV");
            map.put("department", u.getDepartment() != null ? u.getDepartment() : (u.getResourceGroup() != null ? u.getResourceGroup() : "默认业务组"));
            map.put("status", u.getStatus() != null ? u.getStatus() : 1);

            boolean emailV = u.getEmailVerified() != null && u.getEmailVerified() == 1;
            boolean phoneV = u.getPhoneVerified() != null && u.getPhoneVerified() == 1;
            map.put("emailVerified", emailV);
            map.put("phoneVerified", phoneV);
            String vDesc;
            if (emailV && phoneV) {
                vDesc = "双重已核验";
            } else if (emailV) {
                vDesc = "邮箱已核验";
            } else if (phoneV) {
                vDesc = "短信已核验";
            } else {
                vDesc = "系统内置/免验";
            }
            map.put("verifyChannel", vDesc);

            // 查询该用户在各系统的使用摘要
            Map<String, Object> traceSummary = getUserTraceSummary(u.getUsername());
            map.put("traces", traceSummary);

            resultList.add(map);
        }

        Map<String, Object> res = new HashMap<>();
        res.put("total", total);
        res.put("page", page);
        res.put("size", size);
        res.put("list", resultList);
        return Result.success(res);
    }

    /**
     * 获取指定用户的系统使用摘要
     */
    private Map<String, Object> getUserTraceSummary(String username) {
        Map<String, Object> summary = new HashMap<>();
        if (username == null || username.isEmpty()) {
            return summary;
        }
        try {
            List<Map<String, Object>> list = jdbcTemplate.queryForList(
                    "SELECT resource_type, COUNT(*) as cnt, MAX(created_at) as last_time FROM sys_audit_log WHERE username = ? GROUP BY resource_type",
                    username);
            List<String> systems = new ArrayList<>();
            long totalOps = 0;
            String latestTime = null;
            for (Map<String, Object> row : list) {
                String sys = String.valueOf(row.get("resource_type"));
                long cnt = ((Number) row.get("cnt")).longValue();
                totalOps += cnt;
                systems.add(sys);
                Object lt = row.get("last_time");
                if (lt != null) {
                    String timeStr = lt.toString();
                    if (latestTime == null || timeStr.compareTo(latestTime) > 0) {
                        latestTime = timeStr;
                    }
                }
            }
            summary.put("accessedSystems", systems);
            summary.put("totalOps", totalOps);
            summary.put("lastActiveTime", latestTime != null ? latestTime : "暂无交互");
        } catch (Exception ignored) {
            summary.put("accessedSystems", Collections.emptyList());
            summary.put("totalOps", 0L);
            summary.put("lastActiveTime", "暂无交互");
        }
        return summary;
    }

    /**
     * 3. 查询全系统详细操作流与在线轨迹日志
     */
    @Operation(summary = "全系统在线使用轨迹流", description = "按用户名、系统名检索每一次跨系统访问、游戏互动、SQL治理等真实动作记录")
    @GetMapping("/user-logs")
    public Result<Map<String, Object>> listUserLogs(
            @RequestParam(value = "username", required = false) String username,
            @RequestParam(value = "systemName", required = false) String systemName,
            @RequestParam(value = "page", defaultValue = "1") Integer page,
            @RequestParam(value = "size", defaultValue = "30") Integer size,
            @RequestHeader(value = "Authorization", required = false) String authHeader) {

        if (!checkAdminPrivilege(authHeader)) {
            return Result.error("A0301", "权限不足，仅超级管理员可查阅审计与轨迹明细");
        }

        StringBuilder sql = new StringBuilder("SELECT id, username, resource_type, operation, details, ip_address, result, created_at FROM sys_audit_log WHERE 1=1");
        List<Object> params = new ArrayList<>();

        if (username != null && !username.trim().isEmpty()) {
            sql.append(" AND username = ?");
            params.add(username.trim());
        }
        if (systemName != null && !systemName.trim().isEmpty()) {
            sql.append(" AND resource_type = ?");
            params.add(systemName.trim());
        }

        sql.append(" ORDER BY id DESC LIMIT ? OFFSET ?");
        params.add(size);
        params.add(Math.max(0, (page - 1) * size));

        List<Map<String, Object>> logs = Collections.emptyList();
        try {
            logs = jdbcTemplate.queryForList(sql.toString(), params.toArray());
        } catch (Exception e) {
            log.error("[AdminController] 查询轨迹日志失败: {}", e.getMessage());
        }

        Map<String, Object> res = new HashMap<>();
        res.put("page", page);
        res.put("size", size);
        res.put("list", logs);
        return Result.success(res);
    }

    /**
     * 4. 记录各端跨系统交互动作与使用轨迹（公开上报接口）
     */
    @Operation(summary = "跨系统使用轨迹上报", description = "各子系统与客户端漫游时上报当前模块进入与核心操作")
    @PostMapping("/record-trace")
    public Result<Map<String, String>> recordTrace(@RequestBody RecordTraceRequest req, HttpServletRequest request) {
        if (req.getUsername() == null || req.getUsername().trim().isEmpty()) {
            return Result.success(Map.of("status", "skipped", "message", "匿名访问暂不上报"));
        }

        String clientIp = request.getHeader("X-Forwarded-For");
        if (clientIp == null || clientIp.isEmpty()) {
            clientIp = request.getRemoteAddr();
        } else if (clientIp.contains(",")) {
            clientIp = clientIp.split(",")[0].trim();
        }

        String sysName = (req.getSystemName() != null && !req.getSystemName().trim().isEmpty())
                ? req.getSystemName().trim() : "FWPT 统一平台";
        String operation = (req.getAction() != null && !req.getAction().trim().isEmpty())
                ? req.getAction().trim() : "ENTER_SYSTEM";
        String details = req.getDetails() != null ? req.getDetails() : ("用户进入了 " + sysName);

        try {
            jdbcTemplate.update(
                    "INSERT INTO sys_audit_log (tenant_id, username, resource_type, operation, details, ip_address, result, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                    1L, req.getUsername().trim(), sysName, operation, details, clientIp, "SUCCESS", new Timestamp(System.currentTimeMillis())
            );
        } catch (Exception e) {
            log.warn("[AdminController] 记录轨迹日志写入失败: {}", e.getMessage());
        }

        return Result.success(Map.of("status", "recorded"));
    }

    /**
     * 5. 切换用户状态（启用 / 禁用）
     */
    @Operation(summary = "更改用户状态", description = "超级管理员冻结或解冻账号")
    @PostMapping("/user/toggle-status")
    public Result<String> toggleStatus(
            @RequestBody Map<String, Object> req,
            @RequestHeader(value = "Authorization", required = false) String authHeader) {

        if (!checkAdminPrivilege(authHeader)) {
            return Result.error("A0301", "权限不足，仅超级管理员可操作用户状态");
        }

        Long userId = req.get("userId") != null ? Long.valueOf(req.get("userId").toString()) : null;
        Integer status = req.get("status") != null ? Integer.valueOf(req.get("status").toString()) : 1;
        if (userId == null) {
            return Result.error("A0400", "缺少用户 ID");
        }

        SysUser user = sysUserMapper.selectById(userId);
        if (user == null) {
            return Result.error("A0404", "指定用户不存在");
        }

        if ("admin".equalsIgnoreCase(user.getUsername())) {
            return Result.error("A0403", "超级管理员主账号不允许被禁用");
        }

        user.setStatus(status);
        sysUserMapper.updateById(user);

        return Result.success(status == 1 ? "账号已成功启用" : "账号已成功冻结禁用");
    }

    /**
     * 6. 重置用户密码
     */
    @Operation(summary = "重置用户密码", description = "超级管理员重置指定用户的访问密码")
    @PostMapping("/user/reset-password")
    public Result<String> resetPassword(
            @RequestBody Map<String, Object> req,
            @RequestHeader(value = "Authorization", required = false) String authHeader) {

        if (!checkAdminPrivilege(authHeader)) {
            return Result.error("A0301", "权限不足，仅超级管理员可重置密码");
        }

        Long userId = req.get("userId") != null ? Long.valueOf(req.get("userId").toString()) : null;
        String newPassword = req.get("newPassword") != null ? req.get("newPassword").toString().trim() : "123456";

        if (userId == null) {
            return Result.error("A0400", "缺少用户 ID");
        }
        if (newPassword.length() < 6) {
            return Result.error("A0400", "新密码长度不能少于 6 位");
        }

        SysUser user = sysUserMapper.selectById(userId);
        if (user == null) {
            return Result.error("A0404", "指定用户不存在");
        }

        String hash = SmUtils.sm3Hash(newPassword);
        user.setPasswordCipher(hash);
        user.setPasswordHash(hash);
        sysUserMapper.updateById(user);

        return Result.success("用户 " + user.getUsername() + " 密码已成功重置");
    }

    /**
     * 7. 超级管理员删除指定用户
     */
    @Operation(summary = "删除指定用户", description = "超级管理员永久删除指定的非核心用户通行证账号")
    @PostMapping("/user/delete")
    public Result<String> deleteUser(
            @RequestBody Map<String, Object> req,
            @RequestHeader(value = "Authorization", required = false) String authHeader) {

        if (!checkAdminPrivilege(authHeader)) {
            return Result.error("A0301", "权限不足，仅超级管理员可删除用户账号");
        }

        Long userId = req.get("userId") != null ? Long.valueOf(req.get("userId").toString()) : null;
        if (userId == null) {
            return Result.error("A0400", "缺少目标用户 ID");
        }

        SysUser user = sysUserMapper.selectById(userId);
        if (user == null) {
            return Result.error("A0404", "指定用户不存在或已被删除");
        }

        if ("admin".equalsIgnoreCase(user.getUsername()) || (user.getId() != null && user.getId() == 1L)) {
            return Result.error("A0403", "超级管理员主账号为系统基础，禁止删除");
        }

        String targetName = user.getUsername();
        sysUserMapper.deleteById(userId);

        try {
            jdbcTemplate.update(
                    "INSERT INTO sys_audit_log (tenant_id, username, resource_type, operation, details, ip_address, result, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                    1L, "admin", "超级管理中心", "DELETE_USER", "管理员永久删除了用户账号: " + targetName + " (ID: " + userId + ")", "127.0.0.1", "SUCCESS", new Timestamp(System.currentTimeMillis())
            );
        } catch (Exception ignored) {}

        return Result.success("用户【" + targetName + "】账号已成功永久删除");
    }

    @Data
    public static class RecordTraceRequest {
        private String username;
        private String systemName;
        private String action;
        private String details;
    }
}
