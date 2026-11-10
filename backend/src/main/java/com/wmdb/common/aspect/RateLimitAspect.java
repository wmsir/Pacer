package com.wmdb.common.aspect;

import com.wmdb.common.annotation.RateLimit;
import jakarta.servlet.http.HttpServletRequest;
import org.aspectj.lang.ProceedingJoinPoint;
import org.aspectj.lang.annotation.Around;
import org.aspectj.lang.annotation.Aspect;
import org.aspectj.lang.reflect.MethodSignature;
import org.springframework.data.redis.core.RedisTemplate;
import org.springframework.data.redis.core.script.DefaultRedisScript;
import org.springframework.stereotype.Component;
import org.springframework.web.context.request.RequestContextHolder;
import org.springframework.web.context.request.ServletRequestAttributes;

import java.util.Collections;

/**
 * API 速率限制切面
 *
 * @author wm
 */
@Aspect
@Component
public class RateLimitAspect {

    private final RedisTemplate<String, Object> redisTemplate;

    public RateLimitAspect(RedisTemplate<String, Object> redisTemplate) {
        this.redisTemplate = redisTemplate;
    }

    @Around("@annotation(rateLimit)")
    public Object around(ProceedingJoinPoint joinPoint, RateLimit rateLimit) throws Throwable {
        HttpServletRequest request = ((ServletRequestAttributes) RequestContextHolder.currentRequestAttributes()).getRequest();

        // Use IP for rate limit key for anonymous endpoints (like login)
        String ip = request.getRemoteAddr();
        MethodSignature signature = (MethodSignature) joinPoint.getSignature();
        String methodName = signature.getDeclaringTypeName() + "." + signature.getName();

        String key = "rate_limit:" + methodName + ":" + ip;

        long limitTime = rateLimit.time();
        int limitCount = rateLimit.count();

        String luaScript =
                "local key = KEYS[1] " +
                "local limit = tonumber(ARGV[1]) " +
                "local expire_time = tonumber(ARGV[2]) " +
                "local current = tonumber(redis.call('get', key) or '0') " +
                "if current + 1 > limit then " +
                "  return 0 " +
                "else " +
                "  redis.call('incr', key) " +
                "  if current == 0 then " +
                "    redis.call('expire', key, expire_time) " +
                "  end " +
                "  return 1 " +
                "end";

        DefaultRedisScript<Long> redisScript = new DefaultRedisScript<>(luaScript, Long.class);

        try {
            Long result = redisTemplate.execute(redisScript, Collections.singletonList(key), limitCount, limitTime);
            if (result == null || result == 0L) {
                throw new RuntimeException("操作过于频繁，请稍后再试！"); // Translates to: Operations are too frequent, please try again later!
            }
        } catch (Exception e) {
            if (e.getMessage() != null && e.getMessage().contains("操作过于频繁")) {
                throw e;
            }
            // If redis fails or script fails, we fallback to allow for architecture demonstration,
            // to avoid blocking the whole app if redis isn't running properly locally.
            System.err.println("Rate limit check failed (Redis might be down): " + e.getMessage());
        }

        return joinPoint.proceed();
    }
}
