package com.wmdb.security;

import jakarta.servlet.FilterChain;
import jakarta.servlet.ServletException;
import jakarta.servlet.http.HttpServletRequest;
import jakarta.servlet.http.HttpServletResponse;
import org.springframework.security.authentication.UsernamePasswordAuthenticationToken;
import org.springframework.security.core.context.SecurityContextHolder;
import org.springframework.stereotype.Component;
import org.springframework.web.filter.OncePerRequestFilter;

import java.io.IOException;
import java.util.ArrayList;

/**
 * 双模 JWT 认证过滤器（传统 OneID HS256 与统一认证中心 RS256 JWKS）
 * <p>
 * 拦截所有 HTTP 请求，校验黑名单、签名与会话有效性；
 * 如果有效，将统一映射后的用户标识封装存入 SecurityContextHolder；如果携带无效/过期/已吊销 Token，直接响应 401。
 * </p>
 *
 * @author wm
 * @date 2023-10-25
 */
@Component
public class JwtAuthenticationFilter extends OncePerRequestFilter {

    private final JwtTokenResolver jwtTokenResolver;

    public JwtAuthenticationFilter(JwtTokenResolver jwtTokenResolver) {
        this.jwtTokenResolver = jwtTokenResolver;
    }

    @Override
    protected void doFilterInternal(HttpServletRequest request, HttpServletResponse response, FilterChain filterChain)
            throws ServletException, IOException {

        final String authHeader = request.getHeader("Authorization");

        if (authHeader == null || !authHeader.regionMatches(true, 0, "Bearer ", 0, 7)) {
            filterChain.doFilter(request, response);
            return;
        }

        final String jwt = authHeader.substring(7).trim();
        try {
            JwtTokenResolver.ResolvedPrincipal resolved = jwtTokenResolver.resolve(jwt);
            if (resolved != null) {
                UsernamePasswordAuthenticationToken authToken = new UsernamePasswordAuthenticationToken(
                        resolved.getPrincipal(), null, new ArrayList<>());

                SecurityContextHolder.getContext().setAuthentication(authToken);
                filterChain.doFilter(request, response);
            } else if ("/api/v1/auth/logout".equals(request.getRequestURI())) {
                SecurityContextHolder.clearContext();
                filterChain.doFilter(request, response);
            } else {
                SecurityContextHolder.clearContext();
                response.setStatus(HttpServletResponse.SC_UNAUTHORIZED);
                response.setContentType("application/json;charset=UTF-8");
                response.getWriter().write("{\"code\":\"A0220\", \"message\":\"登录身份已过期或无效，请重新登录\"}");
            }
        } catch (Exception e) {
            SecurityContextHolder.clearContext();
            response.setStatus(HttpServletResponse.SC_UNAUTHORIZED);
            response.setContentType("application/json;charset=UTF-8");
            response.getWriter().write("{\"code\":\"A0220\", \"message\":\"登录身份验证未通过，请重新登录\"}");
        }
    }
}
