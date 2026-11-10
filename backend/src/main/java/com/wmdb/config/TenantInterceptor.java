package com.wmdb.config;

import org.springframework.stereotype.Component;
import org.springframework.web.servlet.HandlerInterceptor;
import org.springframework.web.servlet.ModelAndView;

import jakarta.servlet.http.HttpServletRequest;
import jakarta.servlet.http.HttpServletResponse;

/**
 * 多租户拦截器
 * <p>
 * 从 HTTP 请求头中提取 X-Tenant-Id 并设置到上下文中。
 * </p>
 *
 * @author wm
 */
@Component
public class TenantInterceptor implements HandlerInterceptor {

    private static final String TENANT_HEADER = "X-Tenant-Id";

    @Override
    public boolean preHandle(HttpServletRequest request, HttpServletResponse response, Object handler) {
        String tenantId = request.getHeader(TENANT_HEADER);
        if (tenantId != null && !tenantId.isEmpty() && !"public".equals(tenantId)) {
            TenantContextHolder.setTenantId(tenantId);
        } else {
            // Default to '1' if 'public' or null since the actual DB tenant_id uses numeric IDs
            TenantContextHolder.setTenantId("1");
        }
        return true;
    }

    @Override
    public void postHandle(HttpServletRequest request, HttpServletResponse response, Object handler, ModelAndView modelAndView) {
        TenantContextHolder.clear();
    }
}
