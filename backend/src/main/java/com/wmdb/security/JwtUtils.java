package com.wmdb.security;

import io.jsonwebtoken.Claims;
import io.jsonwebtoken.Jwts;
import io.jsonwebtoken.security.Keys;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.stereotype.Component;

import javax.crypto.SecretKey;
import java.nio.charset.StandardCharsets;
import java.util.Date;

/**
 * JWT 工具类
 * <p>
 * 负责基于 HMAC 算法签发、解析和校验 JWT Token。
 * </p>
 *
 * @author wm
 * @date 2023-10-25
 */
@Component
public class JwtUtils {

    /**
     * JWT 签名密钥
     */
    @Value("${wmdb.jwt.secret}")
    private String secretString;

    /**
     * JWT 过期时间（毫秒）
     */
    @Value("${wmdb.jwt.expiration}")
    private long expirationTime;

    private SecretKey cachedSigningKey;

    @jakarta.annotation.PostConstruct
    public void validateSecret() {
        if (secretString == null || secretString.trim().getBytes(java.nio.charset.StandardCharsets.UTF_8).length < 32) {
            throw new IllegalStateException("CRITICAL SECURITY VIOLATION: wmdb.jwt.secret must be provided via environment variable and must be at least 32 bytes in length!");
        }
        this.cachedSigningKey = Keys.hmacShaKeyFor(secretString.getBytes(StandardCharsets.UTF_8));
    }

    /**
     * 获取用于签名的 Key
     *
     * @return SecretKey 实例
     */
    private SecretKey getSigningKey() {
        if (cachedSigningKey == null) {
            cachedSigningKey = Keys.hmacShaKeyFor(secretString.getBytes(StandardCharsets.UTF_8));
        }
        return cachedSigningKey;
    }

    /**
     * 生成 JWT Token
     *
     * @param idCard 身份证号码（作为 Subject）
     * @param realName 真实姓名
     * @return 签发的 JWT 字符串
     */
    public String generateToken(String idCard, String realName) {
        long nowMillis = System.currentTimeMillis();
        return Jwts.builder()
                .subject(idCard)
                .claim("realName", realName)
                .claim("iat_ms", nowMillis)
                .issuedAt(new Date(nowMillis))
                .expiration(new Date(nowMillis + expirationTime))
                .signWith(getSigningKey())
                .compact();
    }

    /**
     * 获取配置的 Token 有效期（毫秒）
     */
    public long getExpirationTime() {
        return expirationTime;
    }

    /**
     * 提取 Token 中所有的 Claim
     *
     * @param token JWT 字符串
     * @return Claims 对象
     */
    public Claims extractAllClaims(String token) {
        return Jwts.parser()
                .verifyWith(getSigningKey())
                .build()
                .parseSignedClaims(token)
                .getPayload();
    }

    /**
     * 提取 Token 中的身份证号码
     *
     * @param token JWT 字符串
     * @return 身份证号码
     */
    public String extractIdCard(String token) {
        return extractAllClaims(token).getSubject();
    }

    /**
     * 提取 Token 中的真实姓名
     *
     * @param token JWT 字符串
     * @return 真实姓名
     */
    public String extractRealName(String token) {
        return extractAllClaims(token).get("realName", String.class);
    }

    /**
     * 单次解析并校验 Token，无效或过期返回 null（避免过滤器重复验签）
     *
     * @param token JWT 字符串
     * @return Claims 或 null
     */
    public Claims parseClaimsIfValid(String token) {
        try {
            return extractAllClaims(token);
        } catch (Exception e) {
            return null;
        }
    }

    /**
     * 验证 Token 是否合法且未过期
     *
     * @param token JWT 字符串
     * @return 是否有效
     */
    public boolean isTokenValid(String token) {
        return parseClaimsIfValid(token) != null;
    }
}
