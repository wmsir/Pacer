package com.wmdb.runner;

import lombok.extern.slf4j.Slf4j;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.boot.ApplicationArguments;
import org.springframework.boot.ApplicationRunner;
import org.springframework.core.annotation.Order;
import org.springframework.core.io.Resource;
import org.springframework.core.io.support.PathMatchingResourcePatternResolver;
import org.springframework.core.io.support.ResourcePatternResolver;
import org.springframework.jdbc.datasource.init.ResourceDatabasePopulator;
import org.springframework.stereotype.Component;

import javax.sql.DataSource;
import java.nio.charset.StandardCharsets;
import java.sql.Connection;
import java.sql.DatabaseMetaData;

/**
 * 智能多数据库方言与国产数据库全自动初始化执行器 (方案 B)
 * <p>
 * 功能特性：
 * 1. 启动时根据配置或自动嗅探 JDBC 底层驱动与数据库产品名称；
 * 2. 深度支持国产数据库：达梦 (DM8/DM7)、人大金仓 (KingbaseES)、华为 openGauss、OceanBase、PingCAP TiDB 以及 MySQL、Oracle；
 * 3. 自动定位执行对应方言目录 (classpath:db/{dbType}/schema.sql 与 data.sql)；
 * 4. 具备幂等容错性 (continue-on-error)，表已存在或对象已存在时平滑跳过，保证高可用。
 * </p>
 *
 * @author wm
 */
@Slf4j
@Component
@Order(1) // 优先级最高，确保在业务数据装载器与服务就绪之前完成建表
public class DatabaseAutoInitializerRunner implements ApplicationRunner {

    private final DataSource dataSource;
    private final ResourcePatternResolver resourceResolver = new PathMatchingResourcePatternResolver();

    @Value("${wmdb.database.auto-init:true}")
    private boolean autoInit;

    @Value("${wmdb.database.type:auto}")
    private String configuredDbType;

    @Value("${wmdb.database.continue-on-error:true}")
    private boolean continueOnError;

    public DatabaseAutoInitializerRunner(DataSource dataSource) {
        this.dataSource = dataSource;
    }

    @Override
    public void run(ApplicationArguments args) {
        if (!autoInit) {
            log.info("[DB-AUTO-INIT] 数据库自动初始化已关闭 (wmdb.database.auto-init=false)");
            return;
        }

        log.info("================================================================================");
        log.info("[DB-AUTO-INIT] 🚀 正在启动多数据库方言与国产数据引擎智能初始化组件...");

        String resolvedDbType = "mysql";
        String detectedProductName = "Unknown";
        String jdbcUrl = "Unknown";

        try (Connection conn = dataSource.getConnection()) {
            DatabaseMetaData metaData = conn.getMetaData();
            detectedProductName = metaData.getDatabaseProductName() != null ? metaData.getDatabaseProductName() : "Unknown";
            jdbcUrl = metaData.getURL() != null ? metaData.getURL() : "Unknown";

            log.info("[DB-AUTO-INIT] 📌 底层驱动检测产品: 【{}】", detectedProductName);
            log.info("[DB-AUTO-INIT] 📌 数据库连接目标URL: 【{}】", maskJdbcUrl(jdbcUrl));

            // 1. 判定数据库类型
            resolvedDbType = resolveDatabaseType(configuredDbType, detectedProductName, jdbcUrl);
            log.info("[DB-AUTO-INIT] 🎯 最终决策匹配的数据库方言目录为: 【{}】", resolvedDbType);

            // 2. 执行表结构脚本 schema.sql
            executeScriptIfExists(resolvedDbType, "schema.sql");

            // 3. 执行初始数据脚本 data.sql (如果存在)
            executeScriptIfExists(resolvedDbType, "data.sql");

            log.info("[DB-AUTO-INIT] ✅ 数据库【{}】方言初始化全流程执行完毕！", resolvedDbType);
            log.info("================================================================================");

        } catch (Exception e) {
            log.warn("[DB-AUTO-INIT] ⚠️ 数据库脚本执行产生警告 (如重复对象或约束已存在): {}", e.getMessage());
            log.info("================================================================================");
        }
    }

    /**
     * 解析数据库类型：配置优先，若为 auto 则根据 JDBC 连接元数据智能推断
     */
    private String resolveDatabaseType(String configType, String productName, String url) {
        if (configType != null && !configType.trim().isEmpty() && !"auto".equalsIgnoreCase(configType.trim())) {
            String cType = configType.trim().toLowerCase();
            log.info("[DB-AUTO-INIT] ⚙️ 检测到显式指定数据库配置: wmdb.database.type = {}", cType);
            return normalizeType(cType);
        }

        String prod = productName.toLowerCase();
        String u = url.toLowerCase();

        // 1. 国产达梦数据库 (DaMeng DM8/DM7)
        if (prod.contains("dm") || prod.contains("dameng") || u.contains(":dm:")) {
            log.info("[DB-AUTO-INIT] 🇨🇳 识别为国产【达梦数据库 (DaMeng DM)】引擎");
            return "dameng";
        }

        // 2. 国产人大金仓 (KingbaseES KES)
        if (prod.contains("kingbase") || u.contains(":kingbase")) {
            log.info("[DB-AUTO-INIT] 🇨🇳 识别为国产【人大金仓 (KingbaseES)】引擎");
            return "kingbase";
        }

        // 3. 华为 openGauss / 统信 UOS / 高斯数据库
        if (prod.contains("opengauss") || prod.contains("zenith") || u.contains(":opengauss:")) {
            log.info("[DB-AUTO-INIT] 🇨🇳 识别为国产【华为 openGauss 高斯数据库】引擎");
            return "opengauss";
        }

        // 4. OceanBase (阿里蚂蚁分布式数据库)
        if (prod.contains("oceanbase") || u.contains("oceanbase")) {
            if (u.contains(":oracle:") || prod.contains("oracle")) {
                log.info("[DB-AUTO-INIT] 🇨🇳 识别为国产【OceanBase (Oracle 模式)】引擎");
                return "oracle";
            } else {
                log.info("[DB-AUTO-INIT] 🇨🇳 识别为国产【OceanBase (MySQL 模式)】引擎");
                return "mysql";
            }
        }

        // 5. PingCAP TiDB (云原生分布式数据库)
        if (prod.contains("tidb") || u.contains("tidb")) {
            log.info("[DB-AUTO-INIT] 🇨🇳 识别为国产【PingCAP TiDB 分布式数据库】引擎");
            return "mysql"; // TiDB 语法与 MySQL 100% 兼容
        }

        // 6. Oracle
        if (prod.contains("oracle") || u.contains(":oracle:")) {
            log.info("[DB-AUTO-INIT] 🌐 识别为【Oracle】企业级数据库引擎");
            return "oracle";
        }

        // 7. PostgreSQL
        if (prod.contains("postgresql") || u.contains(":postgresql:")) {
            log.info("[DB-AUTO-INIT] 🌐 识别为【PostgreSQL】关系型数据库引擎");
            return "opengauss"; // openGauss 与 PG 脚本共享高兼容性
        }

        // 默认兜底 MySQL
        log.info("[DB-AUTO-INIT] 🌐 识别为标准【MySQL】关系型数据库引擎");
        return "mysql";
    }

    private String normalizeType(String type) {
        switch (type) {
            case "dm":
            case "dameng":
                return "dameng";
            case "kes":
            case "kingbase":
            case "kingbasees":
                return "kingbase";
            case "gauss":
            case "opengauss":
                return "opengauss";
            case "oracle":
                return "oracle";
            case "tidb":
            case "mysql":
            default:
                return "mysql";
        }
    }

    /**
     * 加载并执行对应的 SQL 脚本文件
     */
    private void executeScriptIfExists(String dbType, String scriptName) {
        String scriptPath = "classpath:db/" + dbType + "/" + scriptName;
        Resource resource = resourceResolver.getResource(scriptPath);

        // 如果对应方言目录下没有该脚本，且方言为 tidb，则回退至 mysql 目录查找
        if (!resource.exists() && "tidb".equalsIgnoreCase(dbType)) {
            scriptPath = "classpath:db/mysql/" + scriptName;
            resource = resourceResolver.getResource(scriptPath);
        }

        if (resource.exists()) {
            log.info("[DB-AUTO-INIT] 📂 正在加载并执行初始化脚本: 【{}】", scriptPath);
            ResourceDatabasePopulator populator = new ResourceDatabasePopulator();
            populator.addScript(resource);
            populator.setContinueOnError(continueOnError);
            populator.setIgnoreFailedDrops(true);
            populator.setSqlScriptEncoding(StandardCharsets.UTF_8.name());
            populator.setCommentPrefixes("--", "/*");
            populator.setSeparator(";");

            populator.execute(dataSource);
            log.info("[DB-AUTO-INIT] 🌟 脚本【{}】批量执行完毕！", scriptName);
        } else {
            log.debug("[DB-AUTO-INIT] 脚本文件不存在，忽略: {}", scriptPath);
        }
    }

    private String maskJdbcUrl(String url) {
        if (url == null) return "";
        // 对敏感信息进行简单遮罩保护
        return url.replaceAll("(?i)(password|pwd)=[^&;]+", "$1=******");
    }
}
