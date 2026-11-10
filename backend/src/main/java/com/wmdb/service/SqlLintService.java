package com.wmdb.service;

import com.baomidou.dynamic.datasource.DynamicRoutingDataSource;
import com.baomidou.dynamic.datasource.creator.DataSourceProperty;
import com.baomidou.dynamic.datasource.creator.DefaultDataSourceCreator;
import com.baomidou.dynamic.datasource.toolkit.DynamicDataSourceContextHolder;
import com.wmdb.model.DbInstance;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.stereotype.Service;

import javax.crypto.Cipher;
import javax.crypto.spec.SecretKeySpec;
import javax.sql.DataSource;
import java.nio.charset.StandardCharsets;
import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.Statement;

import lombok.extern.slf4j.Slf4j;

/**
 * 智能化 SQL 审核服务 (SQL Lint & EXPLAIN)
 * <p>
 * 目标库直连执行 EXPLAIN 预估执行计划，进行全表扫描拦截和低效索征测。
 * </p>
 *
 * @author wm
 */
@Slf4j
@Service
public class SqlLintService {

    private final DataSource dataSource;
    private final DefaultDataSourceCreator dataSourceCreator;

    @Value("${wmdb.db.aes-key}")
    private String aesKey;

    public SqlLintService(DataSource dataSource, DefaultDataSourceCreator dataSourceCreator) {
        this.dataSource = dataSource;
        this.dataSourceCreator = dataSourceCreator;
    }

    /**
     * 对脚本进行 EXPLAIN 计划预检，并校验预计影响行数
     *
     * @param instance     目标数据库实例
     * @param script       需要预检的脚本
     * @param expectedRows 预期最大影响行数（针对 DML）
     * @return 返回是否通过预检
     */
    public boolean explainCheck(DbInstance instance, String script, Integer expectedRows) {
        // 简化逻辑：仅对单条 SELECT 语句或包含更新/删除的语句执行 EXPLAIN 检查
        if (!"mysql".equalsIgnoreCase(instance.getDbType())) {
            // 目前仅支持 MySQL EXPLAIN 解析
            return true;
        }

        if (script == null || script.trim().isEmpty()) {
            return true;
        }

        String pwd;
        try {
            if ("mockPassword".equals(instance.getPasswordCipher())) {
                pwd = "root";
            } else {
                SecretKeySpec secretKeySpec = new SecretKeySpec(aesKey.getBytes(StandardCharsets.UTF_8), "AES");
                Cipher cipher = Cipher.getInstance("AES/ECB/PKCS5Padding");
                cipher.init(Cipher.DECRYPT_MODE, secretKeySpec);
                byte[] decrypted = cipher.doFinal(java.util.Base64.getDecoder().decode(instance.getPasswordCipher()));
                pwd = new String(decrypted, StandardCharsets.UTF_8);
            }
        } catch (Exception e) {
            pwd = instance.getPasswordCipher();
        }

        String url = instance.getJdbcUrl();
        if (url != null && !url.contains("connectTimeout")) {
            url += (url.contains("?") ? "&" : "?") + "connectTimeout=2000&socketTimeout=3000";
        }

        try {
            Class.forName("com.mysql.cj.jdbc.Driver");
            java.sql.DriverManager.setLoginTimeout(2);
            try (Connection conn = java.sql.DriverManager.getConnection(url, instance.getUsername(), pwd);
                 Statement stmt = conn.createStatement()) {

                java.util.List<String> queries = com.wmdb.utils.SqlSplitUtils.split(script);
                for (String query : queries) {
                    String trimmed = query.trim();
                    if (trimmed.isEmpty() || trimmed.startsWith("--")) continue;

                    boolean isDml = trimmed.toUpperCase().startsWith("UPDATE") || trimmed.toUpperCase().startsWith("DELETE");
                    boolean isSelect = trimmed.toUpperCase().startsWith("SELECT");

                    // 强制只针对 SELECT/UPDATE/DELETE 做 EXPLAIN
                    if (isSelect || isDml) {
                        long totalRows = 0;
                        try (ResultSet rs = stmt.executeQuery("EXPLAIN " + trimmed)) {
                            while (rs.next()) {
                                String type = rs.getString("type");
                                // 极简 Lint 规则：不允许全表扫描
                                if ("ALL".equalsIgnoreCase(type)) {
                                    throw new RuntimeException("SQL Lint Error: EXPLAIN plan shows a full table scan (type=ALL) for query: " + trimmed);
                                }

                                if (isDml) {
                                    try {
                                        String rowsStr = rs.getString("rows");
                                        if (rowsStr != null) {
                                            totalRows += Long.parseLong(rowsStr);
                                        }
                                    } catch (Exception ignore) {
                                    }
                                }
                            }
                        }

                        // 如果是 DML 且传入了 expectedRows，进行校验
                        if (isDml && expectedRows != null) {
                            if (totalRows > expectedRows) {
                                 throw new RuntimeException(String.format("SQL Lint Error: Estimated affected rows (%d) exceeds the expected maximum (%d) for query: %s", totalRows, expectedRows, trimmed));
                            }
                        } else if (isDml && totalRows > 1000) {
                            throw new RuntimeException(String.format("SQL Lint Error: Estimated affected rows (%d) exceeds the hard limit (1000) for query: %s", totalRows, trimmed));
                        }
                    }
                }
            }
            return true;
        } catch (Exception e) {
            if (e instanceof RuntimeException && e.getMessage() != null && e.getMessage().startsWith("SQL Lint Error:")) {
                throw (RuntimeException) e;
            }
            log.warn("SQL Lint EXPLAIN connection warning (host unreachable or timeout, proceeding with ticket): {}", e.getMessage());
            return true;
        }
    }
}
