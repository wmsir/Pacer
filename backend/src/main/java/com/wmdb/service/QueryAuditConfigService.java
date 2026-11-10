package com.wmdb.service;

import com.fasterxml.jackson.core.type.TypeReference;
import com.fasterxml.jackson.databind.ObjectMapper;
import lombok.Data;
import lombok.extern.slf4j.Slf4j;
import org.springframework.stereotype.Service;

import java.io.File;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.util.*;
import java.util.concurrent.ConcurrentHashMap;

/**
 * 在线查询审计可配置化策略管理服务
 * <p>
 * 支持按数据库维度配置是否开启审计日志记录（白名单/黑名单模式），
 * 支持持久化到本地 JSON 配置文件。
 * </p>
 *
 * @author wm
 */
@Slf4j
@Service
public class QueryAuditConfigService {

    private static final String CONFIG_FILE_PATH = "config/query_audit_config.json";
    private final ObjectMapper objectMapper = new ObjectMapper();

    @Data
    public static class AuditPolicyConfig {
        /**
         * 是否默认对所有数据库开启查询审计
         */
        private boolean globalEnabled = true;

        /**
         * 数据库黑名单（在此列表中的库不记录查询审计）
         */
        private Set<String> disabledDatabases = Collections.newSetFromMap(new ConcurrentHashMap<>());

        /**
         * 数据库独立强制开启白名单
         */
        private Set<String> enabledDatabases = Collections.newSetFromMap(new ConcurrentHashMap<>());
    }

    private AuditPolicyConfig currentConfig = new AuditPolicyConfig();

    public QueryAuditConfigService() {
        loadConfig();
    }

    /**
     * 判断特定实例/数据库是否需要记录查询审计
     *
     * @param instanceName 实例名
     * @param dbName       数据库名
     * @return true-需要审计，false-跳过审计
     */
    public boolean shouldAuditQuery(String instanceName, String dbName) {
        if (dbName == null || dbName.trim().isEmpty()) {
            return currentConfig.isGlobalEnabled();
        }
        String cleanDb = dbName.trim();
        String fullKey = (instanceName != null ? instanceName.trim() + "/" : "") + cleanDb;

        // 如果在显式禁用列表中，则不审计
        if (currentConfig.getDisabledDatabases().contains(cleanDb) || currentConfig.getDisabledDatabases().contains(fullKey)) {
            return false;
        }

        // 如果在显式启用列表中，则强制审计
        if (currentConfig.getEnabledDatabases().contains(cleanDb) || currentConfig.getEnabledDatabases().contains(fullKey)) {
            return true;
        }

        return currentConfig.isGlobalEnabled();
    }

    public synchronized AuditPolicyConfig getConfig() {
        return currentConfig;
    }

    public synchronized void updateConfig(AuditPolicyConfig newConfig) {
        if (newConfig != null) {
            this.currentConfig = newConfig;
            saveConfig();
        }
    }

    private void loadConfig() {
        try {
            File file = new File(CONFIG_FILE_PATH);
            if (file.exists()) {
                byte[] bytes = Files.readAllBytes(file.toPath());
                AuditPolicyConfig loaded = objectMapper.readValue(new String(bytes, StandardCharsets.UTF_8), AuditPolicyConfig.class);
                if (loaded != null) {
                    this.currentConfig = loaded;
                }
            }
        } catch (Exception e) {
            log.warn("加载查询审计配置文件失败，使用默认全量开启策略：{}", e.getMessage());
        }
    }

    private void saveConfig() {
        try {
            File file = new File(CONFIG_FILE_PATH);
            if (!file.getParentFile().exists()) {
                file.getParentFile().mkdirs();
            }
            String json = objectMapper.writerWithDefaultPrettyPrinter().writeValueAsString(currentConfig);
            Files.write(file.toPath(), json.getBytes(StandardCharsets.UTF_8));
        } catch (Exception e) {
            log.error("保存查询审计配置文件失败：{}", e.getMessage());
        }
    }
}
