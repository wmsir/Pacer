package com.wmdb.config;

import com.wmdb.security.OpenApiInterceptor;
import org.springframework.context.annotation.Configuration;
import org.springframework.web.servlet.config.annotation.InterceptorRegistry;
import org.springframework.web.servlet.config.annotation.WebMvcConfigurer;

/**
 * Web 配置
 * <p>
 * 注册自定义拦截器（例如多租户拦截器、OpenAPI鉴权拦截器）。
 * </p>
 *
 * @author wm
 */
@Configuration
public class WebConfig implements WebMvcConfigurer {

    private final TenantInterceptor tenantInterceptor;
    private final OpenApiInterceptor openApiInterceptor;

    public WebConfig(TenantInterceptor tenantInterceptor, OpenApiInterceptor openApiInterceptor) {
        this.tenantInterceptor = tenantInterceptor;
        this.openApiInterceptor = openApiInterceptor;
    }

    @Override
    public void addInterceptors(InterceptorRegistry registry) {
        registry.addInterceptor(tenantInterceptor).addPathPatterns("/api/**");
        registry.addInterceptor(openApiInterceptor).addPathPatterns("/api/v1/openapi/**");
    }
}
