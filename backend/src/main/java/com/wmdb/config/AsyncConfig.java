package com.wmdb.config;

import org.springframework.context.annotation.Configuration;
import org.springframework.scheduling.annotation.EnableAsync;

/**
 * 异步任务配置类
 * <p>
 * 开启 Spring 异步方法执行能力，用于后台执行诸如大文件 SQL 流式解析与下发等耗时任务。
 * </p>
 *
 * @author wm
 */
@Configuration
@EnableAsync
public class AsyncConfig {
}
