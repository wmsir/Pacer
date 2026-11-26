package com.wmdb.security;

import com.fasterxml.jackson.core.type.TypeReference;
import com.fasterxml.jackson.databind.ObjectMapper;
import io.jsonwebtoken.Claims;
import io.jsonwebtoken.Jwts;
import lombok.extern.slf4j.Slf4j;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.jdbc.BadSqlGrammarException;
import org.springframework.jdbc.core.JdbcTemplate;
import org.springframework.stereotype.Component;

import java.math.BigInteger;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.nio.charset.StandardCharsets;
import java.security.KeyFactory;
import java.security.interfaces.RSAPublicKey;
import java.security.spec.RSAPublicKeySpec;
import java.time.Duration;
import java.util.*;
import java.util.concurrent.ConcurrentHashMap;

/**
 * 统一认证中心 (Go auth-center) RS256 JWKS 验签与会话校验器
 * <p>
 * 负责拉取并缓存 auth-center JWKS 公钥，严格校验 RS256 签名、kid、iss、exp、nbf、aud，
 * 并结合 fwpt_auth.auth_session 与 fwpt_auth.auth_user 校验会话未被吊销、账号处于启用状态。
 * </p>
 */
@Slf4j
@Component
public class AuthCenterJwtVerifier {

    private static final TypeReference<Map<String, Object>> MAP_TYPE = new TypeReference<>() {};

    private final JdbcTemplate jdbcTemplate;
    private final ObjectMapper objectMapper;
    private final HttpClient httpClient;

    @Value("${wmdb.auth-center.enabled:true}")
    private boolean enabled;

    @Value("${wmdb.auth-center.issuer:https://fwpt.cn/auth}")
    private String expectedIssuer;

    @Value("${wmdb.auth-center.jwks-url:http://172.17.0.1:8092/auth/.well-known/jwks.json}")
    private String jwksUrl;

    @Value("${wmdb.auth-center.allowed-audiences:fwpt-all,fwpt-portal,pacer,wmdb}")
    private String allowedAudiencesRaw;

    @Value("${wmdb.auth-center.jwks-cache-seconds:300}")
    private long jwksCacheSeconds;

    @Value("${wmdb.auth-center.jwks-refresh-cooldown-seconds:30}")
    private long jwksRefreshCooldownSeconds;

    private volatile Map<String, RSAPublicKey> jwksCache = new ConcurrentHashMap<>();
    private volatile long lastFetchMillis = 0L;
    private volatile long lastAttemptMillis = 0L;
    private final Object refreshLock = new Object();

    public AuthCenterJwtVerifier(JdbcTemplate jdbcTemplate, ObjectMapper objectMapper) {
        this.jdbcTemplate = jdbcTemplate;
        this.objectMapper = objectMapper;
        this.httpClient = HttpClient.newBuilder()
                .connectTimeout(Duration.ofSeconds(3))
                .build();
    }

    /**
     * 安全解析未验签的 JWT Header（拒绝 alg=none 或格式非法令牌）
     */
    public Map<String, Object> parseUnverifiedHeader(String jwt) {
        if (jwt == null) {
            return null;
        }
        String trimmed = jwt.trim();
        String[] parts = trimmed.split("\\.", -1);
        if (parts.length != 3 || parts[0].isEmpty() || parts[1].isEmpty() || parts[2].isEmpty()) {
            return null;
        }
        try {
            byte[] headerBytes = Base64.getUrlDecoder().decode(parts[0]);
            Map<String, Object> header = objectMapper.readValue(new String(headerBytes, StandardCharsets.UTF_8), MAP_TYPE);
            Object algObj = header.get("alg");
            if (!(algObj instanceof String alg) || alg.trim().isEmpty() || "none".equalsIgnoreCase(alg.trim())) {
                return null;
            }
            return header;
        } catch (Exception e) {
            return null;
        }
    }

    /**
     * 验证统一认证中心签发的 RS256 Access Token 并解析认证声明
     */
    public AuthCenterClaims verify(String jwt) {
        if (!enabled || jwt == null || jwt.trim().isEmpty()) {
            return null;
        }
        Map<String, Object> header = parseUnverifiedHeader(jwt);
        if (header == null) {
            return null;
        }
        String alg = String.valueOf(header.get("alg")).trim();
        if (!"RS256".equals(alg)) {
            return null;
        }
        Object kidObj = header.get("kid");
        if (!(kidObj instanceof String kid) || kid.trim().isEmpty()) {
            log.warn("[AuthCenterJwtVerifier] Rejected RS256 token without kid header");
            return null;
        }
        kid = kid.trim();

        RSAPublicKey publicKey = resolvePublicKey(kid);
        if (publicKey == null) {
            log.warn("[AuthCenterJwtVerifier] Unknown JWKS kid={}", kid);
            return null;
        }

        Claims claims;
        try {
            claims = Jwts.parser()
                    .verifyWith(publicKey)
                    .clockSkewSeconds(15)
                    .build()
                    .parseSignedClaims(jwt.trim())
                    .getPayload();
        } catch (Exception e) {
            log.debug("[AuthCenterJwtVerifier] RS256 signature/time verification failed: {}", e.getMessage());
            return null;
        }

        if (!isValidIssuer(claims.getIssuer())) {
            log.warn("[AuthCenterJwtVerifier] Issuer mismatch: expected={}, actual={}", expectedIssuer, claims.getIssuer());
            return null;
        }
        if (claims.getExpiration() == null) {
            log.warn("[AuthCenterJwtVerifier] Rejected RS256 token without exp claim");
            return null;
        }
        String subject = claims.getSubject();
        if (subject == null || subject.trim().isEmpty()) {
            log.warn("[AuthCenterJwtVerifier] Rejected RS256 token without sub claim");
            return null;
        }
        subject = subject.trim();

        if (!isValidAudience(claims.getAudience())) {
            log.warn("[AuthCenterJwtVerifier] Audience mismatch for sub={}: aud={}", subject, claims.getAudience());
            return null;
        }

        String jti = claims.getId();
        String sid = stringClaim(claims, "sid");
        Long legacyOneidId = longClaim(claims, "legacy_oneid_id");
        String username = stringClaim(claims, "username");
        String email = stringClaim(claims, "email");
        String phone = stringClaim(claims, "phone");
        String name = stringClaim(claims, "name");
        String role = stringClaim(claims, "role");

        if (jdbcTemplate != null) {
            try {
                if (sid != null && !sid.isEmpty()) {
                    List<Map<String, Object>> sessionRows = jdbcTemplate.queryForList(
                            "SELECT revoked, expires_at FROM fwpt_auth.auth_session WHERE sid = ? LIMIT 1",
                            sid
                    );
                    if (!sessionRows.isEmpty()) {
                        Map<String, Object> sess = sessionRows.get(0);
                        Object revokedObj = sess.get("revoked");
                        if (revokedObj instanceof Number n && n.intValue() != 0) {
                            log.info("[AuthCenterJwtVerifier] Rejected token for revoked sid={}", sid);
                            return null;
                        }
                        Object expObj = sess.get("expires_at");
                        if (isSessionExpired(expObj)) {
                            log.info("[AuthCenterJwtVerifier] Rejected token for expired sid={}", sid);
                            return null;
                        }
                    }
                }

                List<Map<String, Object>> userRows = jdbcTemplate.queryForList(
                        "SELECT legacy_oneid_id, username, display_name, email, phone, role, status FROM fwpt_auth.auth_user WHERE id = ? LIMIT 1",
                        subject
                );
                if (!userRows.isEmpty()) {
                    Map<String, Object> row = userRows.get(0);
                    Object statusObj = row.get("status");
                    if (statusObj instanceof Number n && n.intValue() != 1) {
                        log.info("[AuthCenterJwtVerifier] Rejected token for disabled auth_user id={}", subject);
                        return null;
                    }
                    if (legacyOneidId == null && row.get("legacy_oneid_id") instanceof Number n) {
                        legacyOneidId = n.longValue();
                    }
                    if (isBlank(username)) {
                        username = asTrimmedString(row.get("username"));
                    }
                    if (isBlank(name)) {
                        name = asTrimmedString(row.get("display_name"));
                    }
                    if (isBlank(email)) {
                        email = asTrimmedString(row.get("email"));
                    }
                    if (isBlank(phone)) {
                        phone = asTrimmedString(row.get("phone"));
                    }
                    if (isBlank(role)) {
                        role = asTrimmedString(row.get("role"));
                    }
                }
            } catch (BadSqlGrammarException ex) {
                log.debug("[AuthCenterJwtVerifier] fwpt_auth lookup skipped (table unavailable): {}", ex.getMessage());
            } catch (Exception ex) {
                log.warn("[AuthCenterJwtVerifier] fwpt_auth session/user check failed closed: {}", ex.getMessage());
                return null;
            }
        }

        return new AuthCenterClaims(
                subject,
                jti,
                sid,
                claims.getIssuedAt(),
                claims.getExpiration(),
                legacyOneidId,
                username,
                email,
                phone,
                name,
                role
        );
    }

    private RSAPublicKey resolvePublicKey(String kid) {
        long now = System.currentTimeMillis();
        long ttlMs = Math.max(jwksCacheSeconds, 10L) * 1000L;
        if (jwksCache.isEmpty() || (now - lastFetchMillis) > ttlMs) {
            refreshJwks(false);
        }
        RSAPublicKey key = jwksCache.get(kid);
        if (key == null) {
            long cooldownMs = Math.max(jwksRefreshCooldownSeconds, 5L) * 1000L;
            if ((now - lastAttemptMillis) > cooldownMs) {
                refreshJwks(true);
                key = jwksCache.get(kid);
            }
        }
        return key;
    }

    private void refreshJwks(boolean force) {
        synchronized (refreshLock) {
            long now = System.currentTimeMillis();
            long ttlMs = Math.max(jwksCacheSeconds, 10L) * 1000L;
            long cooldownMs = Math.max(jwksRefreshCooldownSeconds, 5L) * 1000L;
            if (!force && !jwksCache.isEmpty() && (now - lastFetchMillis) <= ttlMs) {
                return;
            }
            if ((now - lastAttemptMillis) < cooldownMs && !jwksCache.isEmpty()) {
                return;
            }
            lastAttemptMillis = now;

            if (isBlank(jwksUrl)) {
                return;
            }
            try {
                HttpRequest request = HttpRequest.newBuilder()
                        .uri(URI.create(jwksUrl.trim()))
                        .timeout(Duration.ofSeconds(5))
                        .header("Accept", "application/json")
                        .GET()
                        .build();
                HttpResponse<String> response = httpClient.send(request, HttpResponse.BodyHandlers.ofString(StandardCharsets.UTF_8));
                if (response.statusCode() != 200 || isBlank(response.body())) {
                    log.warn("[AuthCenterJwtVerifier] JWKS fetch returned HTTP {}", response.statusCode());
                    return;
                }
                Map<String, Object> body = objectMapper.readValue(response.body(), MAP_TYPE);
                Object keysObj = body.get("keys");
                if (!(keysObj instanceof List<?> keysList)) {
                    log.warn("[AuthCenterJwtVerifier] JWKS payload missing keys array");
                    return;
                }
                Map<String, RSAPublicKey> parsed = new ConcurrentHashMap<>();
                for (Object item : keysList) {
                    if (!(item instanceof Map<?, ?> rawMap)) {
                        continue;
                    }
                    String kid = asTrimmedString(rawMap.get("kid"));
                    String kty = asTrimmedString(rawMap.get("kty"));
                    String use = asTrimmedString(rawMap.get("use"));
                    String alg = asTrimmedString(rawMap.get("alg"));
                    String n = asTrimmedString(rawMap.get("n"));
                    String e = asTrimmedString(rawMap.get("e"));
                    if (isBlank(kid) || !"RSA".equalsIgnoreCase(kty)) {
                        continue;
                    }
                    if (!isBlank(use) && !"sig".equalsIgnoreCase(use)) {
                        continue;
                    }
                    if (!isBlank(alg) && !"RS256".equalsIgnoreCase(alg)) {
                        continue;
                    }
                    if (isBlank(n) || isBlank(e)) {
                        continue;
                    }
                    RSAPublicKey pub = buildRsaPublicKey(n, e);
                    if (pub != null) {
                        parsed.put(kid, pub);
                    }
                }
                if (!parsed.isEmpty()) {
                    jwksCache = parsed;
                    lastFetchMillis = now;
                }
            } catch (Exception ex) {
                log.warn("[AuthCenterJwtVerifier] Failed to refresh JWKS from {}: {}", jwksUrl, ex.getMessage());
            }
        }
    }

    private static RSAPublicKey buildRsaPublicKey(String nB64Url, String eB64Url) {
        try {
            byte[] nBytes = Base64.getUrlDecoder().decode(nB64Url);
            byte[] eBytes = Base64.getUrlDecoder().decode(eB64Url);
            if (nBytes.length < 128 || eBytes.length == 0) {
                return null;
            }
            BigInteger modulus = new BigInteger(1, nBytes);
            BigInteger exponent = new BigInteger(1, eBytes);
            KeyFactory kf = KeyFactory.getInstance("RSA");
            return (RSAPublicKey) kf.generatePublic(new RSAPublicKeySpec(modulus, exponent));
        } catch (Exception ex) {
            return null;
        }
    }

    private boolean isValidIssuer(String actualIssuer) {
        if (isBlank(expectedIssuer)) {
            return true;
        }
        if (isBlank(actualIssuer)) {
            return false;
        }
        return normalizeIssuer(expectedIssuer).equals(normalizeIssuer(actualIssuer));
    }

    private static String normalizeIssuer(String issuer) {
        String s = issuer.trim();
        while (s.endsWith("/")) {
            s = s.substring(0, s.length() - 1);
        }
        return s;
    }

    private boolean isValidAudience(Set<String> tokenAudience) {
        if (isBlank(allowedAudiencesRaw)) {
            return true;
        }
        if (tokenAudience == null || tokenAudience.isEmpty()) {
            return false;
        }
        for (String allowed : allowedAudiencesRaw.split(",")) {
            String candidate = allowed.trim();
            if (!candidate.isEmpty() && tokenAudience.contains(candidate)) {
                return true;
            }
        }
        return false;
    }

    private static String stringClaim(Claims claims, String name) {
        Object val = claims.get(name);
        return asTrimmedString(val);
    }

    private static Long longClaim(Claims claims, String name) {
        Object val = claims.get(name);
        if (val instanceof Number n) {
            return n.longValue();
        }
        if (val instanceof String s && !s.trim().isEmpty()) {
            try {
                return Long.parseLong(s.trim());
            } catch (NumberFormatException ignored) {
                return null;
            }
        }
        return null;
    }

    private static String asTrimmedString(Object val) {
        if (val == null) {
            return null;
        }
        String s = String.valueOf(val).trim();
        return s.isEmpty() ? null : s;
    }

    private static boolean isSessionExpired(Object expObj) {
        long now = System.currentTimeMillis();
        if (expObj instanceof Date expDate) {
            return expDate.getTime() < now;
        }
        if (expObj instanceof java.time.LocalDateTime ldt) {
            return ldt.atZone(java.time.ZoneId.systemDefault()).toInstant().toEpochMilli() < now;
        }
        if (expObj instanceof java.time.Instant inst) {
            return inst.toEpochMilli() < now;
        }
        return false;
    }

    private static boolean isBlank(String val) {
        return val == null || val.trim().isEmpty();
    }

    public static class AuthCenterClaims {
        private final String subject;
        private final String jti;
        private final String sid;
        private final Date issuedAt;
        private final Date expiresAt;
        private final Long legacyOneidId;
        private final String username;
        private final String email;
        private final String phone;
        private final String name;
        private final String role;

        public AuthCenterClaims(
                String subject,
                String jti,
                String sid,
                Date issuedAt,
                Date expiresAt,
                Long legacyOneidId,
                String username,
                String email,
                String phone,
                String name,
                String role) {
            this.subject = subject;
            this.jti = jti;
            this.sid = sid;
            this.issuedAt = issuedAt;
            this.expiresAt = expiresAt;
            this.legacyOneidId = legacyOneidId;
            this.username = username;
            this.email = email;
            this.phone = phone;
            this.name = name;
            this.role = role;
        }

        public String getSubject() {
            return subject;
        }

        public String getJti() {
            return jti;
        }

        public String getSid() {
            return sid;
        }

        public Date getIssuedAt() {
            return issuedAt;
        }

        public Date getExpiresAt() {
            return expiresAt;
        }

        public Long getLegacyOneidId() {
            return legacyOneidId;
        }

        public String getUsername() {
            return username;
        }

        public String getEmail() {
            return email;
        }

        public String getPhone() {
            return phone;
        }

        public String getName() {
            return name;
        }

        public String getRole() {
            return role;
        }
    }
}
