package com.wmdb.controller;

import com.wmdb.common.Result;
import com.wmdb.common.annotation.RateLimit;
import com.wmdb.service.AuthService;
import io.swagger.v3.oas.annotations.Operation;
import io.swagger.v3.oas.annotations.tags.Tag;
import jakarta.servlet.http.HttpServletRequest;
import lombok.Data;
import org.springframework.jdbc.core.JdbcTemplate;
import org.springframework.web.bind.annotation.*;

import java.sql.Timestamp;
import java.util.HashMap;
import java.util.Map;

/**
 * 认证控制器
 * <p>
 * 提供账号密码登录、手机验证码登录及短信验证码发送 API 接口。
 * </p>
 *
 * @author wm
 * @date 2023-10-25
 */
@Tag(name = "用户认证与授权", description = "提供手机号验证码登录、用户名/身份证/手机号密码登录及短信发送")
@RestController
@RequestMapping("/api/v1/auth")
public class AuthController {

    private final AuthService authService;
    private final JdbcTemplate jdbcTemplate;

    /**
     * 构造函数注入 AuthService 与 JdbcTemplate
     */
    public AuthController(AuthService authService, JdbcTemplate jdbcTemplate) {
        this.authService = authService;
        this.jdbcTemplate = jdbcTemplate;
    }

    /**
     * 发送手机短信验证码
     *
     * @param sendCodeRequest 手机号封装
     * @return 包含发送状态与测试验证码的响应实体
     */
    @Operation(summary = "发送手机短信验证码", description = "输入11位手机号获取短信验证码（测试通用码：123456）")
    @RateLimit(time = 60, count = 10) // 1分钟内限制发送10次
    @PostMapping("/send-code")
    public Result<Map<String, String>> sendCode(@RequestBody SendCodeRequest sendCodeRequest) {
        String code = authService.sendSmsCode(sendCodeRequest.getPhone());
        Map<String, String> response = new HashMap<>();
        response.put("phone", sendCodeRequest.getPhone());
        response.put("code", code); // 开发/测试环境返回，便于调试
        response.put("message", "验证码发送成功");
        return Result.success(response);
    }

    /**
     * 发送企业邮箱动态安全验证码
     *
     * @param req 邮箱请求封装
     * @return 响应结果
     */
    @Operation(summary = "发送企业邮箱安全验证码", description = "向指定邮箱发送6位动态安全验证码")
    @RateLimit(time = 60, count = 10)
    @PostMapping("/send-email-code")
    public Result<Map<String, String>> sendEmailCode(@RequestBody SendEmailCodeRequest req) {
        authService.sendEmailCode(req.getEmail());
        Map<String, String> response = new HashMap<>();
        response.put("email", req.getEmail());
        response.put("message", "安全验证码已发送至您的邮箱，请注意查收并在5分钟内完成核查");
        return Result.success(response);
    }

    /**
     * 用户统一登录接口（支持 账号密码登录 与 手机验证码登录）
     *
     * @param loginRequest 登录请求参数封装
     * @return 包含 Token 的响应实体
     */
    @Operation(summary = "用户登录认证", description = "支持账号密码登录（用户名/身份证号/手机号/企业邮箱）与手机验证码快速登录")
    @RateLimit(time = 60, count = 10)
    @PostMapping("/login")
    public Result<Map<String, Object>> login(@RequestBody LoginRequest loginRequest, HttpServletRequest request) {
        Map<String, Object> response;
        String loggedUsername;

        // 判断登录类型：手机验证码登录
        if ("PHONE_CODE".equalsIgnoreCase(loginRequest.getLoginType()) ||
                (loginRequest.getPhone() != null && !loginRequest.getPhone().isEmpty() && loginRequest.getCode() != null && !loginRequest.getCode().isEmpty())) {
            String token = authService.loginByPhone(loginRequest.getPhone(), loginRequest.getCode());
            response = new HashMap<>();
            response.put("token", token);
            response.put("phone", loginRequest.getPhone());
            loggedUsername = loginRequest.getPhone();
        } else {
            // 账号密码登录（兼容 account, idCard, username, email, phone 传参）
            String account = loginRequest.getAccount();
            if (account == null || account.isEmpty()) {
                account = loginRequest.getEmail();
            }
            if (account == null || account.isEmpty()) {
                account = loginRequest.getIdCard();
            }
            if (account == null || account.isEmpty()) {
                account = loginRequest.getUsername();
            }
            if (account == null || account.isEmpty()) {
                account = loginRequest.getPhone();
            }
            response = authService.loginFull(account, loginRequest.getPassword());
            loggedUsername = response.get("username") != null ? String.valueOf(response.get("username")) : account;
        }

        // 自动记录全平台跨系统登录审计轨迹，供超级管理员大屏实时检视
        recordAuthAuditTrace(loggedUsername, loginRequest.getSystemName(), "USER_LOGIN", request);

        return Result.success(response);
    }

    /**
     * 多渠道用户自主注册
     */
    @Operation(summary = "用户多渠道注册", description = "支持手机号、邮箱、用户名注册，强制录入姓名，支持指定资源组")
    @RateLimit(time = 60, count = 10)
    @PostMapping("/register")
    public Result<Map<String, String>> register(@RequestBody com.wmdb.model.RegisterRequestDTO registerRequest, HttpServletRequest request) {
        String token = authService.register(registerRequest);
        String regUser = registerRequest.getUsername() != null && !registerRequest.getUsername().isEmpty()
                ? registerRequest.getUsername()
                : (registerRequest.getEmail() != null ? registerRequest.getEmail() : registerRequest.getPhone());
        recordAuthAuditTrace(regUser, null, "USER_REGISTER", request);

        Map<String, String> response = new HashMap<>();
        response.put("token", token);
        response.put("message", "注册成功并已自动登录");
        return Result.success(response);
    }

    /**
     * 通过邮箱验证码重置密码
     */
    @Operation(summary = "通过邮箱验证码重置密码", description = "校验邮箱及验证码后更新账号密码")
    @RateLimit(time = 60, count = 10)
    @PostMapping("/reset-password")
    public Result<Map<String, String>> resetPassword(@RequestBody ResetPasswordRequest req, HttpServletRequest request) {
        if (req == null) {
            throw new com.wmdb.exception.BusinessException("A0400", "请求参数不能为空");
        }
        authService.resetPasswordWithEmail(req.getEmail(), req.getCode(), req.getNewPassword());
        recordAuthAuditTrace(req.getEmail(), null, "RESET_PASSWORD", request);

        Map<String, String> response = new HashMap<>();
        response.put("message", "密码重置成功，请使用新密码重新登录");
        return Result.success(response);
    }

    /**
     * 自动识别终端来源系统并写入 sys_audit_log
     */
    private void recordAuthAuditTrace(String username, String explicitSystem, String operation, HttpServletRequest request) {
        if (username == null || username.trim().isEmpty()) return;
        try {
            String ua = request != null && request.getHeader("User-Agent") != null ? request.getHeader("User-Agent") : "";
            String ref = request != null && request.getHeader("Referer") != null ? request.getHeader("Referer") : "";
            String sysName = explicitSystem;
            if (sysName == null || sysName.trim().isEmpty()) {
                if (ua.contains("PartyGameServer") || ref.contains("party")) {
                    sysName = "聚会桌游吧";
                } else if (ua.contains("SeahorseBack") || ref.contains("seahorse")) {
                    sysName = "海马智慧门店";
                } else if (ref.contains("arrow")) {
                    sysName = "箭头消消乐";
                } else if (ref.contains("vault") || ua.contains("Dart")) {
                    sysName = "磐石密码本";
                } else if (ref.contains("pacer")) {
                    sysName = "稳骥 PacerSQL";
                } else if (ref.contains("wm-erp") || ref.contains("zebra")) {
                    sysName = "斑马 ERP";
                } else if (ref.contains("admin.html")) {
                    sysName = "超级管理中心";
                } else {
                    sysName = "FWPT 统一门户";
                }
            }
            String clientIp = request != null ? request.getHeader("X-Forwarded-For") : null;
            if (clientIp == null || clientIp.isEmpty()) {
                clientIp = request != null ? request.getRemoteAddr() : "127.0.0.1";
            } else if (clientIp.contains(",")) {
                clientIp = clientIp.split(",")[0].trim();
            }
            String details = "USER_REGISTER".equals(operation)
                    ? ("新用户在【" + sysName + "】完成统一通行证注册并自动登录")
                    : ("用户通过统一通行证认证登录【" + sysName + "】");
            jdbcTemplate.update(
                    "INSERT INTO sys_audit_log (tenant_id, username, resource_type, operation, details, ip_address, result, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                    1L, username.trim(), sysName, operation, details, clientIp, "SUCCESS", new Timestamp(System.currentTimeMillis())
            );
        } catch (Exception ignored) {}
    }

    /**
     * 获取当前登录用户画像、角色与已授权的系统页签功能权限列表
     */
    @Operation(summary = "获取当前登录用户信息及页签权限", description = "解析 JWT Token 并返回用户名、角色、组织及可访问的页签路由清单")
    @GetMapping("/user-info")
    public Result<Map<String, Object>> getUserInfo(@RequestHeader(value = "Authorization", required = false) String authHeader) {
        return Result.success(authService.getCurrentUserInfo(authHeader));
    }

    /**
     * 发送短信验证码请求封装类
     */
    @Data
    public static class SendCodeRequest {
        private String phone;
    }

    /**
     * 发送邮箱验证码请求封装类
     */
    @Data
    public static class SendEmailCodeRequest {
        private String email;
    }

    /**
     * 登录请求参数封装类
     */
    @Data
    public static class LoginRequest {
        /**
         * 登录模式：ACCOUNT_PASSWORD (默认) 或 PHONE_CODE
         */
        private String loginType;

        /**
         * 统一登录账号（用户名、身份证、手机号或企业邮箱）
         */
        private String account;

        /**
         * 企业邮箱（兼容传参）
         */
        private String email;

        /**
         * 身份证号码（兼容旧接口）
         */
        private String idCard;

        /**
         * 用户名（兼容传参）
         */
        private String username;

        /**
         * 手机号
         */
        private String phone;

        /**
         * 密码（SM2 加密或明文）
         */
        private String password;

        /**
         * 短信或邮箱验证码
         */
        private String code;

        /**
         * 可选：来源业务系统名称
         */
        private String systemName;
    }

    /**
     * 重置密码请求封装类
     */
    @Data
    public static class ResetPasswordRequest {
        private String email;
        private String code;
        private String newPassword;
    }
}
