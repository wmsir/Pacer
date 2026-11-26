package com.wmdb.security;

import com.baomidou.mybatisplus.core.conditions.query.QueryWrapper;
import com.wmdb.mapper.SysUserMapper;
import com.wmdb.model.SysUser;
import com.wmdb.security.AuthCenterJwtVerifier.AuthCenterClaims;
import io.jsonwebtoken.Claims;
import lombok.RequiredArgsConstructor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.stereotype.Service;

import java.util.*;

/**
 * 双模 JWT 令牌统一解析器（兼容传统 OneID HS256 与统一认证中心 RS256 JWKS）
 * <p>
 * 在单一位置完成黑名单拦截、alg=none 拒绝、验签、用户级吊销时间校验、账号禁用拦截，
 * 并将认证中心身份映射为本地 sys_user 标识。
 * </p>
 */
@Slf4j
@Service
@RequiredArgsConstructor
public class JwtTokenResolver {

    private final JwtUtils jwtUtils;
    private final AuthCenterJwtVerifier authCenterJwtVerifier;
    private final JwtBlacklistService jwtBlacklistService;
    private final SysUserMapper sysUserMapper;

    public String extractBearerToken(String authHeader) {
        if (authHeader == null) {
            return null;
        }
        String trimmed = authHeader.trim();
        if (trimmed.regionMatches(true, 0, "Bearer ", 0, 7)) {
            String token = trimmed.substring(7).trim();
            return token.isEmpty() ? null : token;
        }
        return null;
    }

    /**
     * 解析并校验 JWT（HS256 或 RS256），不通过则返回 null
     */
    public ResolvedPrincipal resolve(String rawToken) {
        if (rawToken == null || rawToken.trim().isEmpty()) {
            return null;
        }
        String token = rawToken.trim();
        if (jwtBlacklistService.isBlacklisted(token)) {
            return null;
        }

        Map<String, Object> header = authCenterJwtVerifier.parseUnverifiedHeader(token);
        if (header == null) {
            return null;
        }
        String alg = String.valueOf(header.get("alg")).trim();
        if ("RS256".equals(alg)) {
            return resolveAuthCenterToken(token);
        }
        if (alg.startsWith("HS")) {
            return resolveLegacyHsToken(token);
        }
        return null;
    }

    private ResolvedPrincipal resolveAuthCenterToken(String token) {
        AuthCenterClaims ac = authCenterJwtVerifier.verify(token);
        if (ac == null) {
            return null;
        }
        if (ac.getJti() != null && jwtBlacklistService.isJtiBlacklisted(ac.getJti())) {
            return null;
        }

        SysUser sysUser = resolveSysUserForAuthCenter(ac);
        if (sysUser != null && sysUser.getStatus() != null && sysUser.getStatus() == 0) {
            return null;
        }

        Set<String> revocableIds = new LinkedHashSet<>();
        addNonBlank(revocableIds, ac.getSubject());
        addNonBlank(revocableIds, ac.getUsername());
        addNonBlank(revocableIds, ac.getEmail());
        addNonBlank(revocableIds, ac.getPhone());
        if (ac.getLegacyOneidId() != null) {
            revocableIds.add(String.valueOf(ac.getLegacyOneidId()));
        }
        if (sysUser != null) {
            revocableIds.addAll(JwtBlacklistService.collectUserIdentifiers(sysUser));
        }

        if (jwtBlacklistService.isUserTokenRevoked(revocableIds, ac.getIssuedAt(), null)) {
            return null;
        }

        String principal = pickCanonicalPrincipal(sysUser, ac);
        if (principal == null || principal.isEmpty()) {
            return null;
        }
        String realName = (sysUser != null && sysUser.getRealName() != null && !sysUser.getRealName().trim().isEmpty())
                ? sysUser.getRealName().trim()
                : ac.getName();

        return new ResolvedPrincipal(
                principal,
                realName,
                "RS256",
                ac.getJti(),
                ac.getSid(),
                ac.getSubject(),
                ac.getIssuedAt(),
                ac.getExpiresAt(),
                sysUser
        );
    }

    private ResolvedPrincipal resolveLegacyHsToken(String token) {
        Claims claims = jwtUtils.parseClaimsIfValid(token);
        if (claims == null) {
            return null;
        }
        String subject = claims.getSubject();
        if (subject == null || subject.trim().isEmpty()) {
            return null;
        }
        subject = subject.trim();

        SysUser sysUser = findSysUserByIdentifier(subject);
        if (sysUser != null && sysUser.getStatus() != null && sysUser.getStatus() == 0) {
            return null;
        }

        Set<String> revocableIds = new LinkedHashSet<>();
        revocableIds.add(subject);
        if (sysUser != null) {
            revocableIds.addAll(JwtBlacklistService.collectUserIdentifiers(sysUser));
        }

        Long iatMs = null;
        Object iatMsObj = claims.get("iat_ms");
        if (iatMsObj instanceof Number n) {
            iatMs = n.longValue();
        }
        if (jwtBlacklistService.isUserTokenRevoked(revocableIds, claims.getIssuedAt(), iatMs)) {
            return null;
        }

        String realName = claims.get("realName", String.class);
        if ((realName == null || realName.trim().isEmpty()) && sysUser != null) {
            realName = sysUser.getRealName();
        }

        return new ResolvedPrincipal(
                subject,
                realName,
                "HS256",
                claims.getId(),
                null,
                null,
                claims.getIssuedAt(),
                claims.getExpiration(),
                sysUser
        );
    }

    private SysUser resolveSysUserForAuthCenter(AuthCenterClaims ac) {
        if (ac.getLegacyOneidId() != null && ac.getLegacyOneidId() > 0L) {
            SysUser byId = sysUserMapper.selectById(ac.getLegacyOneidId());
            if (byId != null) {
                return byId;
            }
        }
        if (ac.getUsername() != null) {
            SysUser byUsername = sysUserMapper.selectOne(new QueryWrapper<SysUser>()
                    .eq("username", ac.getUsername())
                    .last("LIMIT 1"));
            if (byUsername != null) {
                return byUsername;
            }
        }
        if (ac.getEmail() != null) {
            SysUser byEmail = sysUserMapper.selectOne(new QueryWrapper<SysUser>()
                    .eq("email", ac.getEmail())
                    .last("LIMIT 1"));
            if (byEmail != null) {
                return byEmail;
            }
        }
        if (ac.getPhone() != null) {
            SysUser byPhone = sysUserMapper.selectOne(new QueryWrapper<SysUser>()
                    .eq("phone", ac.getPhone())
                    .last("LIMIT 1"));
            if (byPhone != null) {
                return byPhone;
            }
        }
        return null;
    }

    private SysUser findSysUserByIdentifier(String identifier) {
        if (identifier == null || identifier.trim().isEmpty()) {
            return null;
        }
        String trimmed = identifier.trim();
        SysUser user = sysUserMapper.selectOne(new QueryWrapper<SysUser>()
                .eq("username", trimmed)
                .last("LIMIT 1"));
        if (user == null) {
            user = sysUserMapper.selectOne(new QueryWrapper<SysUser>()
                    .eq("phone", trimmed)
                    .last("LIMIT 1"));
        }
        if (user == null && !"0".equals(trimmed)) {
            user = sysUserMapper.selectOne(new QueryWrapper<SysUser>()
                    .eq("id_card", trimmed)
                    .last("LIMIT 1"));
        }
        return user;
    }

    private static String pickCanonicalPrincipal(SysUser sysUser, AuthCenterClaims ac) {
        if (sysUser != null) {
            if (sysUser.getIdCard() != null && !sysUser.getIdCard().trim().isEmpty() && !"0".equals(sysUser.getIdCard().trim())) {
                return sysUser.getIdCard().trim();
            }
            if (sysUser.getUsername() != null && !sysUser.getUsername().trim().isEmpty()) {
                return sysUser.getUsername().trim();
            }
            if (sysUser.getPhone() != null && !sysUser.getPhone().trim().isEmpty()) {
                return sysUser.getPhone().trim();
            }
        }
        if (ac.getUsername() != null && !ac.getUsername().isEmpty()) {
            return ac.getUsername();
        }
        if (ac.getPhone() != null && !ac.getPhone().isEmpty()) {
            return ac.getPhone();
        }
        return ac.getSubject();
    }

    public boolean isAdmin(ResolvedPrincipal resolved) {
        if (resolved == null) {
            return false;
        }
        SysUser user = resolved.getSysUser();
        if (user != null) {
            if (user.getStatus() != null && user.getStatus() == 0) {
                return false;
            }
            if ("admin".equalsIgnoreCase(user.getUsername())) {
                return true;
            }
            if (user.getRole() != null) {
                for (String roleToken : user.getRole().split(",")) {
                    if ("ADMIN".equalsIgnoreCase(roleToken.trim())) {
                        return true;
                    }
                }
            }
            return false;
        }
        return "admin".equalsIgnoreCase(resolved.getPrincipal());
    }

    private static void addNonBlank(Set<String> set, String value) {
        if (value != null && !value.trim().isEmpty()) {
            set.add(value.trim());
        }
    }

    public static class ResolvedPrincipal {
        private final String principal;
        private final String realName;
        private final String tokenType;
        private final String jti;
        private final String sid;
        private final String authUserId;
        private final Date issuedAt;
        private final Date expiresAt;
        private final SysUser sysUser;

        public ResolvedPrincipal(
                String principal,
                String realName,
                String tokenType,
                String jti,
                String sid,
                String authUserId,
                Date issuedAt,
                Date expiresAt,
                SysUser sysUser) {
            this.principal = principal;
            this.realName = realName;
            this.tokenType = tokenType;
            this.jti = jti;
            this.sid = sid;
            this.authUserId = authUserId;
            this.issuedAt = issuedAt;
            this.expiresAt = expiresAt;
            this.sysUser = sysUser;
        }

        public String getPrincipal() {
            return principal;
        }

        public String getRealName() {
            return realName;
        }

        public String getTokenType() {
            return tokenType;
        }

        public String getJti() {
            return jti;
        }

        public String getSid() {
            return sid;
        }

        public String getAuthUserId() {
            return authUserId;
        }

        public Date getIssuedAt() {
            return issuedAt;
        }

        public Date getExpiresAt() {
            return expiresAt;
        }

        public SysUser getSysUser() {
            return sysUser;
        }
    }
}
