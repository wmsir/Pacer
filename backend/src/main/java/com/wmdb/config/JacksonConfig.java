package com.wmdb.config;

import com.fasterxml.jackson.databind.module.SimpleModule;
import com.wmdb.security.XssJacksonDeserializer;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;
import org.springframework.http.converter.json.Jackson2ObjectMapperBuilder;

/**
 * Jackson 配置
 * <p>
 * 注册 XSS 反序列化器，用于处理 @RequestBody 的 JSON XSS 过滤。
 * </p>
 *
 * @author wm
 */
@Configuration
public class JacksonConfig {

    @Bean
    public SimpleModule xssModule() {
        SimpleModule module = new SimpleModule();
        // 注册对 String 类型的 XSS 反序列化处理
        module.addDeserializer(String.class, new XssJacksonDeserializer());
        return module;
    }
}
