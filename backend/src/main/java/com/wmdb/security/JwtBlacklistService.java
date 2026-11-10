package com.wmdb.security;

import org.springframework.stereotype.Service;

import java.util.concurrent.ConcurrentHashMap;
import java.util.Map;
import java.util.concurrent.atomic.AtomicInteger;

/**
 * JWT 黑名单服务
 *
 * @author wm
 */
@Service
public class JwtBlacklistService {

    // 使用 ConcurrentHashMap 替代 HashSet，实现基本的线程安全。
    // Value 存储到期时间的时间戳 (毫秒)。
    private final Map<String, Long> blacklist = new ConcurrentHashMap<>();

    // 默认过期时间 (例如：24小时，与 Token 有效期匹配)
    private static final long EXPIRATION_MILLIS = 24 * 60 * 60 * 1000;

    // 使用计数器降低清理频率，避免高并发下每次都全表扫描
    private final AtomicInteger cleanupCounter = new AtomicInteger(0);

    /**
     * 将 Token 加入黑名单
     */
    public void addToBlacklist(String token) {
        if (cleanupCounter.incrementAndGet() >= 1000) {
            cleanUp();
            cleanupCounter.set(0);
        }

        // 生产环境中，最好能从 token 内部解析出具体的 exp 时间，
        // 这里为了演示防御 OOM 的逻辑，直接按固定时长加上当前时间。
        blacklist.put(token, System.currentTimeMillis() + EXPIRATION_MILLIS);
    }

    /**
     * 判断 Token 是否在黑名单中
     */
    public boolean isBlacklisted(String token) {
        Long expirationTime = blacklist.get(token);
        if (expirationTime == null) {
            return false;
        }
        if (System.currentTimeMillis() > expirationTime) {
            // 已过期的黑名单条目可以移除
            blacklist.remove(token);
            return false;
        }
        return true;
    }

    /**
     * 清理已过期的黑名单记录以防止 OOM 内存泄漏
     */
    private void cleanUp() {
        long now = System.currentTimeMillis();
        blacklist.entrySet().removeIf(entry -> now > entry.getValue());
    }
}
