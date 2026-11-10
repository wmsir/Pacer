package com.wmdb.config;

import com.baomidou.mybatisplus.annotation.DbType;
import com.baomidou.mybatisplus.extension.plugins.MybatisPlusInterceptor;
import com.baomidou.mybatisplus.extension.plugins.inner.PaginationInnerInterceptor;
import com.baomidou.mybatisplus.extension.plugins.inner.TenantLineInnerInterceptor;
import com.baomidou.mybatisplus.extension.plugins.handler.TenantLineHandler;
import net.sf.jsqlparser.expression.Expression;
import net.sf.jsqlparser.expression.StringValue;
import org.mybatis.spring.annotation.MapperScan;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;

/**
 * MyBatis-Plus 配置类
 * <p>
 * 集成多租户字段隔离插件与物理分页插件，默认租户 ID 为 1，支持异步线程透传。
 * </p>
 *
 * @author wm
 */
@Configuration
@MapperScan("com.wmdb.mapper")
public class MybatisPlusConfig {

    @Bean
    public MybatisPlusInterceptor mybatisPlusInterceptor() {
        MybatisPlusInterceptor interceptor = new MybatisPlusInterceptor();
        // 1. 多租户插件
        interceptor.addInnerInterceptor(new TenantLineInnerInterceptor(new TenantLineHandler() {
            @Override
            public Expression getTenantId() {
                String tenantId = TenantContextHolder.getTenantId();
                if (tenantId == null || tenantId.isEmpty() || "public".equalsIgnoreCase(tenantId)) {
                    tenantId = "1";
                }
                return new StringValue(tenantId);
            }

            @Override
            public boolean ignoreTable(String tableName) {
                // 用户表与基础角色表全局共享，不强制租户隔离
                return "sys_user".equalsIgnoreCase(tableName) || "sys_role".equalsIgnoreCase(tableName);
            }
        }));
        // 2. 物理分页插件
        interceptor.addInnerInterceptor(new PaginationInnerInterceptor(DbType.MYSQL));
        return interceptor;
    }
}
