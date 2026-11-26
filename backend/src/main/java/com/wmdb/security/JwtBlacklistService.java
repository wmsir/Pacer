package com.wmdb.security;

import com.wmdb.model.SysUser;
import lombok.extern.slf4j.Slf4j;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.data.redis.core.StringRedisTemplate;
import org.springframework.stereotype.Service;

import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.util.*;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicInteger;

/**
 * JWT 黑名单与会话吊销服务
 * <p>
 * 支持单个 Token / jti 按剩余过期时间入黑名单，以及按用户维度记录全局吊销时间戳（密码变更、禁用、删除账号），
 * 同时清理 Redis 中的遗留登录态与统一认证中心 SSO 会话缓存。
 * </p>
 */
@Slf4j
@Service
public class JwtBlacklistService {

    private static final long DEFAULT_EXPIRATION_MILLIS = 24L * 60 * 60 * 1000;
    private static final long MIN_BLACKLIST_TTL_MILLIS = 60L * 1000;
    private static final String REDIS_BLACKLIST_PREFIX = "jwt_blacklist:";
    private static final String REDIS_USER_REVOKED_PREFIX = "jwt_revoked_before:";
    private static final String REDIS_LOGIN_TOKEN_PREFIX = "login_token:";
    private static final String REDIS_AUTH_SSO_PREFIX = "auth:sso:";

    private final Map<String, Long> blacklist = new ConcurrentHashMap<>();
    private final Map<String, Long> userRevokedBefore = new ConcurrentHashMap<>();
    private final AtomicInteger cleanupCounter = new AtomicInteger(0);
    private final StringRedisTemplate stringRedisTemplate;

    public JwtBlacklistService() {
        this(null);
    }

    @Autowired
    public JwtBlacklistService(@Autowired(required = false) StringRedisTemplate stringRedisTemplate) {
        this.stringRedisTemplate = stringRedisTemplate;
    }

    /**
     * 将令牌加入黑名单（使用默认 24 小时有效期）
     */
    public void addToBlacklist(String token) {
        addToBlacklist(token, null);
    }

    /**
     * 将令牌加入黑名单并按其剩余过期时间设置 TTL
     */
    public void addToBlacklist(String token, Date expiresAt) {
        if (token == null || token.trim().isEmpty()) {
            return;
        }
        addRawKeyToBlacklist(sha256Hex(token.trim()), expiresAt);
    }

    /**
     * 将统一认证中心 Access Token 的 jti 加入黑名单
     */
    public void blacklistJti(String jti, Date expiresAt) {
        if (jti == null || jti.trim().isEmpty()) {
            return;
        }
        addRawKeyToBlacklist(sha256Hex("jti:" + jti.trim()), expiresAt);
    }

    /**
     * 判断原始令牌是否在黑名单内
     */
    public boolean isBlacklisted(String token) {
        if (token == null || token.trim().isEmpty()) {
            return false;
        }
        return isKeyBlacklisted(sha256Hex(token.trim()));
    }

    /**
     * 判断 jti 是否在黑名单内
     */
    public boolean isJtiBlacklisted(String jti) {
        if (jti == null || jti.trim().isEmpty()) {
            return false;
        }
        return isKeyBlacklisted(sha256Hex("jti:" + jti.trim()));
    }

    /**
     * 吊销指定用户的所有已签发令牌，并删除其本地/Redis 登录缓存
     */
    public void revokeUserTokens(SysUser user) {
        if (user == null) {
            return;
        }
        Set<String> identifiers = collectUserIdentifiers(user);
        revokeUserTokens(identifiers);
        for (String id : identifiers) {
            revokeLoginTokenRedis(id);
        }
    }

    /**
     * 按标识集合记录用户全局令牌吊销时间戳
     */
    public void revokeUserTokens(Collection<String> identifiers) {
        if (identifiers == null || identifiers.isEmpty()) {
            return;
        }
        long revokedAtMillis = System.currentTimeMillis();
        for (String raw : identifiers) {
            if (raw == null) {
                continue;
            }
            String normalized = raw.trim();
            if (normalized.isEmpty()) {
                continue;
            }
            String keyHash = sha256Hex("uid:" + normalized);
            userRevokedBefore.merge(keyHash, revokedAtMillis, Math::max);
            if (stringRedisTemplate != null) {
                try {
                    stringRedisTemplate.opsForValue().set(
                            REDIS_USER_REVOKED_PREFIX + keyHash,
                            String.valueOf(revokedAtMillis),
                            DEFAULT_EXPIRATION_MILLIS,
                            TimeUnit.MILLISECONDS
                    );
                } catch (Exception ex) {
                    log.debug("Failed to persist user token revocation in Redis: {}", ex.getMessage());
                }
            }
        }
        maybeCleanUp();
    }

    /**
     * 判断在 issuedAt 时刻签发的令牌是否已被用户级吊销策略作废
     */
    public boolean isUserTokenRevoked(Collection<String> identifiers, Date issuedAt, Long issuedAtMillisClaim) {
        if (identifiers == null || identifiers.isEmpty()) {
            return false;
        }
        Long tokenIssuedAtMs = null;
        if (issuedAtMillisClaim != null && issuedAtMillisClaim > 0L) {
            tokenIssuedAtMs = issuedAtMillisClaim;
        } else if (issuedAt != null) {
            tokenIssuedAtMs = issuedAt.getTime();
        }

        long now = System.currentTimeMillis();
        for (String raw : identifiers) {
            if (raw == null) {
                continue;
            }
            String normalized = raw.trim();
            if (normalized.isEmpty()) {
                continue;
            }
            String keyHash = sha256Hex("uid:" + normalized);
            Long revokedAtMs = userRevokedBefore.get(keyHash);
            if (revokedAtMs != null && (now - revokedAtMs > DEFAULT_EXPIRATION_MILLIS)) {
                userRevokedBefore.remove(keyHash, revokedAtMs);
                revokedAtMs = null;
            }
            if (stringRedisTemplate != null) {
                try {
                    String redisVal = stringRedisTemplate.opsForValue().get(REDIS_USER_REVOKED_PREFIX + keyHash);
                    if (redisVal != null && !redisVal.isEmpty()) {
                        long redisTs = Long.parseLong(redisVal);
                        revokedAtMs = (revokedAtMs == null) ? redisTs : Math.max(revokedAtMs, redisTs);
                        userRevokedBefore.merge(keyHash, redisTs, Math::max);
                    }
                } catch (Exception ex) {
                    log.debug("Failed to read user token revocation from Redis: {}", ex.getMessage());
                }
            }
            if (revokedAtMs != null) {
                if (tokenIssuedAtMs == null || tokenIssuedAtMs <= revokedAtMs) {
                    return true;
                }
            }
        }
        return false;
    }

    /**
     * 删除统一认证中心 SSO Redis 会话键
     */
    public void revokeSsoSessionRedis(String sid) {
        if (stringRedisTemplate == null || sid == null || sid.trim().isEmpty()) {
            return;
        }
        try {
            stringRedisTemplate.delete(REDIS_AUTH_SSO_PREFIX + sid.trim());
        } catch (Exception ex) {
            log.debug("Failed to delete auth:sso session from Redis: {}", ex.getMessage());
        }
    }

    /**
     * 删除 wmdb 登录态 Redis 缓存键
     */
    public void revokeLoginTokenRedis(String identifier) {
        if (stringRedisTemplate == null || identifier == null || identifier.trim().isEmpty()) {
            return;
        }
        try {
            stringRedisTemplate.delete(REDIS_LOGIN_TOKEN_PREFIX + identifier.trim());
        } catch (Exception ex) {
            log.debug("Failed to delete login_token from Redis: {}", ex.getMessage());
        }
    }

    public static Set<String> collectUserIdentifiers(SysUser user) {
        Set<String> ids = new LinkedHashSet<>();
        if (user == null) {
            return ids;
        }
        if (user.getId() != null) {
            ids.add(String.valueOf(user.getId()));
        }
        addIfNonBlank(ids, user.getUsername());
        if (user.getIdCard() != null && !user.getIdCard().trim().isEmpty() && !"0".equals(user.getIdCard().trim())) {
            ids.add(user.getIdCard().trim());
        }
        addIfNonBlank(ids, user.getPhone());
        addIfNonBlank(ids, user.getEmail());
        return ids;
    }

    private static void addIfNonBlank(Set<String> target, String value) {
        if (value != null && !value.trim().isEmpty()) {
            target.add(value.trim());
        }
    }

    private void addRawKeyToBlacklist(String keyHash, Date expiresAt) {
        long now = System.currentTimeMillis();
        long ttlMillis = DEFAULT_EXPIRATION_MILLIS;
        if (expiresAt != null) {
            ttlMillis = Math.max(expiresAt.getTime() - now, MIN_BLACKLIST_TTL_MILLIS);
        }
        long expireAtMillis = now + ttlMillis;
        blacklist.merge(keyHash, expireAtMillis, Math::max);

        if (stringRedisTemplate != null) {
            try {
                stringRedisTemplate.opsForValue().set(
                        REDIS_BLACKLIST_PREFIX + keyHash,
                        "1",
                        ttlMillis,
                        TimeUnit.MILLISECONDS
                );
            } catch (Exception ex) {
                log.debug("Failed to store token blacklist entry in Redis: {}", ex.getMessage());
            }
        }
        maybeCleanUp();
    }

    private boolean isKeyBlacklisted(String keyHash) {
        long now = System.currentTimeMillis();
        Long expireAt = blacklist.get(keyHash);
        if (expireAt != null) {
            if (now <= expireAt) {
                return true;
            }
            blacklist.remove(keyHash, expireAt);
        }
        if (stringRedisTemplate != null) {
            try {
                Boolean exists = stringRedisTemplate.hasKey(REDIS_BLACKLIST_PREFIX + keyHash);
                if (Boolean.TRUE.equals(exists)) {
                    return true;
                }
            } catch (Exception ex) {
                log.debug("Failed to check token blacklist entry in Redis: {}", ex.getMessage());
            }
        }
        return false;
    }

    private void maybeCleanUp() {
        if (cleanupCounter.incrementAndGet() % 100 == 0) {
            cleanUp();
        }
    }

    private void cleanUp() {
        long now = System.currentTimeMillis();
        blacklist.entrySet().removeIf(entry -> now > entry.getValue());
        userRevokedBefore.entrySet().removeIf(entry -> (now - entry.getValue()) > DEFAULT_EXPIRATION_MILLIS);
    }

    private static String sha256Hex(String input) {
        try {
            MessageDigest md = MessageDigest.getInstance("SHA-256");
            byte[] digest = md.digest(input.getBytes(StandardCharsets.UTF_8));
            StringBuilder sb = new StringBuilder(digest.length * 2);
            for (byte b : digest) {
                sb.append(Character.forDigit((b >> 4) & 0xF, 16));
                sb.append(Character.forDigit(b & 0xF, 16));
            }
            return sb.toString();
        } catch (Exception e) {
            return Integer.toHexString(input.hashCode());
        }
    }
}
