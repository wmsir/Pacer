package com.wmdb.config;

/**
 * 租户上下文持有者
 * <p>
 * 使用 InheritableThreadLocal 存储当前请求的租户标识，支撑 SaaS 多租户基础架构及异步线程上下文透传。
 * </p>
 *
 * @author wm
 */
public class TenantContextHolder {

    private static final ThreadLocal<String> CONTEXT = new InheritableThreadLocal<>();

    public static void setTenantId(String tenantId) {
        CONTEXT.set(tenantId);
    }

    public static String getTenantId() {
        String id = CONTEXT.get();
        return (id != null && !id.isEmpty()) ? id : "1";
    }

    public static void clear() {
        CONTEXT.remove();
    }
}
