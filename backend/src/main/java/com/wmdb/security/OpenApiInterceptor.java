package com.wmdb.security;

import jakarta.servlet.http.HttpServletRequest;
import jakarta.servlet.http.HttpServletResponse;
import org.springframework.stereotype.Component;
import org.springframework.web.servlet.HandlerInterceptor;

/**
 * OpenAPI 鉴权拦截器
 * <p>
 * 实现基于 AK/SK 的接口签名校验机制。
 * </p>
 *
 * @author wm
 */
@Component
public class OpenApiInterceptor implements HandlerInterceptor {

    @Override
    public boolean preHandle(HttpServletRequest request, HttpServletResponse response, Object handler) throws Exception {
        String accessKey = request.getHeader("X-Access-Key");
        String signature = request.getHeader("X-Signature");
        String timestamp = request.getHeader("X-Timestamp");

        if (accessKey == null || signature == null || timestamp == null) {
            response.setStatus(HttpServletResponse.SC_UNAUTHORIZED);
            response.setContentType("application/json;charset=UTF-8");
            response.getWriter().write("{\"code\":\"A0220\", \"message\":\"OpenAPI 缺少认证信息\"}");
            return false;
        }

        // 简单的防重放攻击（5分钟内有效）
        long reqTime;
        try {
            reqTime = Long.parseLong(timestamp);
        } catch (NumberFormatException e) {
            response.setStatus(HttpServletResponse.SC_UNAUTHORIZED);
            return false;
        }
        if (Math.abs(System.currentTimeMillis() - reqTime) > 5 * 60 * 1000) {
            response.setStatus(HttpServletResponse.SC_UNAUTHORIZED);
            response.setContentType("application/json;charset=UTF-8");
            response.getWriter().write("{\"code\":\"A0220\", \"message\":\"请求已过期\"}");
            return false;
        }

        // 生产环境应从数据库或缓存根据 accessKey 获取 secretKey
        // 为了演示安全机制，这里固定使用一个基于环境配置或默认值的 SK
        String secretKey = "wmdb-secret-key-for-" + accessKey;

        // 采用 HmacSHA256 签名，避免使用不安全的 MD5
        String payloadToSign = accessKey + timestamp;
        String expectedSign = cn.hutool.crypto.SecureUtil.hmacSha256(secretKey).digestHex(payloadToSign);

        if (!expectedSign.equalsIgnoreCase(signature)) {
            response.setStatus(HttpServletResponse.SC_UNAUTHORIZED);
            response.setContentType("application/json;charset=UTF-8");
            response.getWriter().write("{\"code\":\"A0220\", \"message\":\"OpenAPI 签名验证失败\"}");
            return false;
        }

        return true;
    }
}
