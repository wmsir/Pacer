package com.wmdb.config;

import io.swagger.v3.oas.models.Components;
import io.swagger.v3.oas.models.OpenAPI;
import io.swagger.v3.oas.models.info.Contact;
import io.swagger.v3.oas.models.info.Info;
import io.swagger.v3.oas.models.info.License;
import io.swagger.v3.oas.models.security.SecurityRequirement;
import io.swagger.v3.oas.models.security.SecurityScheme;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;

/**
 * Swagger / OpenAPI 3 配置类
 * <p>
 * 配置 API 基础信息、文档版本及基于 JWT Bearer Token 的接口鉴权能力。
 * </p>
 *
 * @author wm
 */
@Configuration
public class OpenApiConfig {

    private static final String SECURITY_SCHEME_NAME = "BearerAuth";

    @Bean
    public OpenAPI customOpenAPI() {
        return new OpenAPI()
                .info(new Info()
                        .title("wmDB 完美数据库 - 接口文档与管理中心")
                        .description("wmDB 企业级智能数据库运维与安全审计平台 RESTful API 文档。\n\n"
                                + "### 核心功能模块：\n"
                                + "- **用户认证服务**：多模式登录（用户名密码 / 手机验证码 / 身份证实名）\n"
                                + "- **SQL 工单与审核流转**：工单提交、AI 智能 SQL 审查、Flowable 审批流转\n"
                                + "- **数据库实例管理**：多数据源配置、脱敏凭证存储与健康监控\n"
                                + "- **OpenAPI 开放能力**：外部 CI/CD 及运维系统对接\n\n"
                                + "### 测试凭据：\n"
                                + "- **管理员账号**：`testadmin1`，密码：`123456`\n"
                                + "- **测试手机号**：`13800000001`，通用测试验证码：`123456`")
                        .version("v2.0.0")
                        .contact(new Contact().name("wmDB 技术架构组").email("5815209@qq.com"))
                        .license(new License().name("Apache 2.0").url("https://springdoc.org")))
                // 配置全局 JWT Token 认证组件
                .addSecurityItem(new SecurityRequirement().addList(SECURITY_SCHEME_NAME))
                .components(new Components()
                        .addSecuritySchemes(SECURITY_SCHEME_NAME,
                                new SecurityScheme()
                                        .name(SECURITY_SCHEME_NAME)
                                        .type(SecurityScheme.Type.HTTP)
                                        .scheme("bearer")
                                        .bearerFormat("JWT")
                                        .description("请输入登录成功后获取的 JWT Token (无需输入 Bearer 前缀)")));
    }
}
