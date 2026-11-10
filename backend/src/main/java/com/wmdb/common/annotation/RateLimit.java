package com.wmdb.common.annotation;

import java.lang.annotation.*;

/**
 * API 速率限制注解
 *
 * @author wm
 */
@Target(ElementType.METHOD)
@Retention(RetentionPolicy.RUNTIME)
@Documented
public @interface RateLimit {

    /**
     * 限制的时间窗口（秒）
     * 默认 60 秒
     */
    long time() default 60;

    /**
     * 时间窗口内允许的最大请求次数
     * 默认 10 次
     */
    int count() default 10;
}
