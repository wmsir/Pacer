package com.wmdb.security;

import com.baomidou.mybatisplus.core.conditions.query.QueryWrapper;
import com.wmdb.exception.BusinessException;
import com.wmdb.mapper.SysUserMapper;
import com.wmdb.model.SysUser;
import lombok.RequiredArgsConstructor;
import org.springframework.security.core.Authentication;
import org.springframework.security.core.context.SecurityContextHolder;
import org.springframework.stereotype.Component;

import java.util.Locale;

/**
 * 基于已鉴权主体与数据库实时角色的 RBAC 权限守卫
 */
@Component
@RequiredArgsConstructor
public class PrivilegeGuard {

    private final SysUserMapper sysUserMapper;

    public SysUser requireAuthenticatedUser() {
        Authentication auth = SecurityContextHolder.getContext().getAuthentication();
        if (auth == null || !auth.isAuthenticated() || auth.getPrincipal() == null
                || "anonymousUser".equals(auth.getPrincipal())) {
            throw new BusinessException("A0220", "未登录或登录凭据已过期");
        }
        String subject = String.valueOf(auth.getPrincipal()).trim();
        if (subject.isEmpty()) {
            throw new BusinessException("A0220", "未登录或登录凭据已过期");
        }
        SysUser user = sysUserMapper.selectOne(new QueryWrapper<SysUser>()
                .eq("username", subject)
                .or().eq("phone", subject)
                .or().eq("id_card", subject)
                .last("LIMIT 1"));
        if (user == null) {
            throw new BusinessException("A0220", "登录账号不存在或已失效");
        }
        if (user.getStatus() != null && user.getStatus() == 0 && !"admin".equalsIgnoreCase(user.getUsername())) {
            throw new BusinessException("A0301", "账号已被冻结禁用");
        }
        return user;
    }

    public SysUser requireAdmin() {
        return requireAnyRole("ADMIN", "SUPER_ADMIN");
    }

    public SysUser requireAdminOrDba() {
        return requireAnyRole("ADMIN", "SUPER_ADMIN", "DBA");
    }

    public SysUser requireAnyRole(String... allowedRoles) {
        SysUser user = requireAuthenticatedUser();
        if ("admin".equalsIgnoreCase(user.getUsername()) || (user.getId() != null && user.getId() == 1L)) {
            return user;
        }
        String rawRole = user.getRole() != null ? user.getRole() : "";
        String[] tokens = rawRole.split("[,\\[\\]\\s\"']+");
        for (String tok : tokens) {
            if (tok == null || tok.isEmpty()) {
                continue;
            }
            String norm = tok.trim().toUpperCase(Locale.ROOT);
            for (String allowed : allowedRoles) {
                if (norm.equals(allowed)) {
                    return user;
                }
            }
        }
        throw new BusinessException("A0301", "权限不足，仅授权管理员可执行此操作");
    }
}
