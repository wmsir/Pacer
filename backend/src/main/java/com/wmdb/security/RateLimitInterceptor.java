package com.wmdb.security;

import com.wmdb.exception.BusinessException;
import org.springframework.stereotype.Component;
import org.springframework.web.servlet.HandlerInterceptor;

import jakarta.servlet.http.HttpServletRequest;
import jakarta.servlet.http.HttpServletResponse;

/**
 * Redis 限流拦截器模拟
 *
 * @author wm
 */
@Component
public class RateLimitInterceptor implements HandlerInterceptor {
    @Override
    public boolean preHandle(HttpServletRequest request, HttpServletResponse response, Object handler) {
        // 模拟 Redis 限流检查
        // if (redisTemplate.opsForValue().increment(request.getRemoteAddr()) > limit) throw Exception;
        return true;
    }
}
