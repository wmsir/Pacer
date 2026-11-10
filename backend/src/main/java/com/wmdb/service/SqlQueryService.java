package com.wmdb.service;

import com.baomidou.dynamic.datasource.DynamicRoutingDataSource;
import com.baomidou.dynamic.datasource.creator.DataSourceProperty;
import com.baomidou.dynamic.datasource.creator.DefaultDataSourceCreator;
import com.baomidou.dynamic.datasource.toolkit.DynamicDataSourceContextHolder;
import com.wmdb.exception.BusinessException;
import com.wmdb.mapper.DbInstanceMapper;
import com.wmdb.mapper.QueryAuditLogMapper;
import com.wmdb.mapper.SysUserMapper;
import com.wmdb.model.DataMaskingRule;
import com.wmdb.model.DbInstance;
import com.wmdb.model.QueryAuditLog;
import com.wmdb.model.SqlQueryResult;
import com.wmdb.model.SysUser;
import com.wmdb.security.SmUtils;
import jakarta.annotation.PostConstruct;
import lombok.extern.slf4j.Slf4j;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.security.core.Authentication;
import org.springframework.security.core.context.SecurityContextHolder;
import org.springframework.stereotype.Service;

import javax.crypto.Cipher;
import javax.crypto.spec.SecretKeySpec;
import javax.sql.DataSource;
import java.nio.charset.StandardCharsets;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.ResultSet;
import java.sql.ResultSetMetaData;
import java.sql.Statement;
import java.util.*;

/**
 * SQL 在线安全只读查询与执行计划服务（集成动态脱敏引擎与查询审计策略）
 *
 * @author wm
 */
@Slf4j
@Service
public class SqlQueryService {

    private final DbInstanceMapper dbInstanceMapper;
    private final DataSource dataSource;
    private final DefaultDataSourceCreator dataSourceCreator;
    private final DataMaskingRuleService dataMaskingRuleService;
    private final MaskingEngine maskingEngine;
    private final QueryAuditLogMapper queryAuditLogMapper;
    private final QueryAuditConfigService queryAuditConfigService;
    private final SysUserMapper sysUserMapper;
    private final DbInstanceService dbInstanceService;

    @Value("${wmdb.db.aes-key:1234567890123456}")
    private String aesKey;

    public SqlQueryService(DbInstanceMapper dbInstanceMapper,
                            DataSource dataSource,
                            DefaultDataSourceCreator dataSourceCreator,
                            DataMaskingRuleService dataMaskingRuleService,
                            MaskingEngine maskingEngine,
                            QueryAuditLogMapper queryAuditLogMapper,
                            QueryAuditConfigService queryAuditConfigService,
                            SysUserMapper sysUserMapper,
                            DbInstanceService dbInstanceService) {
        this.dbInstanceMapper = dbInstanceMapper;
        this.dataSource = dataSource;
        this.dataSourceCreator = dataSourceCreator;
        this.dataMaskingRuleService = dataMaskingRuleService;
        this.maskingEngine = maskingEngine;
        this.queryAuditLogMapper = queryAuditLogMapper;
        this.queryAuditConfigService = queryAuditConfigService;
        this.sysUserMapper = sysUserMapper;
        this.dbInstanceService = dbInstanceService;
    }

    @PostConstruct
    public void initAuditTable() {
        try (Connection conn = dataSource.getConnection();
             Statement stmt = conn.createStatement()) {
            stmt.execute("CREATE TABLE IF NOT EXISTS query_audit_log (" +
                    "id bigint NOT NULL AUTO_INCREMENT PRIMARY KEY, " +
                    "tenant_id varchar(50) DEFAULT '1', " +
                    "instance_id bigint, " +
                    "instance_name varchar(100), " +
                    "db_name varchar(100), " +
                    "user_id bigint, " +
                    "username varchar(100), " +
                    "real_name varchar(100), " +
                    "op_type varchar(50), " +
                    "sql_text text, " +
                    "cost_ms bigint, " +
                    "result_rows int, " +
                    "status varchar(20), " +
                    "error_msg text, " +
                    "client_ip varchar(50), " +
                    "create_time datetime" +
                    ") ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;");
        } catch (Exception e) {
            log.warn("初始化 query_audit_log 表提示：{}", e.getMessage());
        }
    }

    /**
     * 执行受控的安全只读 SQL 查询，并自动应用动态数据脱敏与审计记录
     */
    public SqlQueryResult executeQuery(Long instanceId, String dbName, String sql, Integer limit) {
        return executeQueryInternal(instanceId, dbName, sql, limit, false);
    }

    /**
     * 执行 SQL 执行计划 (EXPLAIN) 分析
     */
    public SqlQueryResult executeExplain(Long instanceId, String dbName, String sql) {
        if (sql == null || sql.trim().isEmpty()) {
            throw new BusinessException("A0400", "请输入待分析执行计划的 SQL 语句");
        }
        String explainSql = sql.trim();
        if (!explainSql.toUpperCase().startsWith("EXPLAIN")) {
            explainSql = "EXPLAIN " + explainSql;
        }
        return executeQueryInternal(instanceId, dbName, explainSql, 100, true);
    }

    private SqlQueryResult executeQueryInternal(Long instanceId, String dbName, String sql, Integer limit, boolean isExplain) {
        if (instanceId == null) {
            throw new BusinessException("A0400", "请先选择目标数据库实例");
        }
        DbInstance instance = dbInstanceMapper.selectById(instanceId);
        if (instance == null) {
            throw new BusinessException("A0400", "目标数据库实例不存在");
        }

        // 校验实例是否开启了「支持查询」操作权限
        if (instance.getSupportedOps() != null && !instance.getSupportedOps().trim().isEmpty()) {
            String ops = instance.getSupportedOps();
            if (!ops.contains("支持查询")) {
                throw new BusinessException("A0403", "目标实例【" + instance.getName() + "】当前已禁用【支持查询】操作权限，严禁在线执行数据检索");
            }
        }

        if (sql == null || sql.trim().isEmpty()) {
            throw new BusinessException("A0400", "请输入待执行的 SQL 查询语句");
        }

        String trimmedSql = sql.trim();
        if (!isExplain) {
            validateReadOnlySql(trimmedSql);
        }

        int maxLimit = limit != null && limit > 0 && limit <= 1000 ? limit : 200;
        long startTime = System.currentTimeMillis();

        String password = dbInstanceService.resolvePassword(instance);
        String targetDb = (dbName != null && !dbName.trim().isEmpty()) ? dbName.trim() : "";
        String dbType = instance.getDbType() != null ? instance.getDbType().toLowerCase() : "mysql";

        // 主从读写分离智能路由：若配置了只读从库，只读 DQL 查询优先路由至只读从库执行；若未配置或从库故障则平滑回退至主库
        boolean hasSlave = instance.getReadOnlyJdbcUrl() != null && !instance.getReadOnlyJdbcUrl().trim().isEmpty();
        boolean isRoutedToSlave = hasSlave;
        String selectedRawUrl = hasSlave ? instance.getReadOnlyJdbcUrl().trim() : instance.getJdbcUrl();
        String jdbcUrl = buildJdbcUrlWithDatabase(selectedRawUrl, targetDb);

        // 脱敏规则
        Map<String, DataMaskingRule> activeRules = dataMaskingRuleService.getActiveRulesMap(instanceId, targetDb);

        List<String> columns = new ArrayList<>();
        List<Map<String, Object>> rows = new ArrayList<>();
        Map<String, String> maskedColumnsMap = new LinkedHashMap<>();
        String opType = isExplain ? "EXPLAIN" : resolveOpType(trimmedSql);
        String status = "SUCCESS";
        String errorMsg = null;

        try {
            Class.forName(dbInstanceService.getDriverClassName(dbType));
            DriverManager.setLoginTimeout(5);

            Connection conn = null;
            try {
                conn = DriverManager.getConnection(jdbcUrl, instance.getUsername(), password);
            } catch (Exception connEx) {
                // 如果是从库连接失败，触发自动降级，切换到主库重试
                if (hasSlave) {
                    log.warn("只读从库连接异常 [{}]，触发高可用读写分离自动降级，切换至主库执行: {}", jdbcUrl, connEx.getMessage());
                    isRoutedToSlave = false;
                    jdbcUrl = buildJdbcUrlWithDatabase(instance.getJdbcUrl(), targetDb);
                    conn = DriverManager.getConnection(jdbcUrl, instance.getUsername(), password);
                } else {
                    throw connEx;
                }
            }

            try (Connection activeConn = conn) {
                try {
                    activeConn.setReadOnly(true); // 驱动与数据库会话层设置只读模式
                } catch (Exception ignored) {}
                try (Statement stmt = activeConn.createStatement()) {

                // 切换具体数据库
                if (!targetDb.isEmpty()) {
                    try {
                        activeConn.setCatalog(targetDb);
                    } catch (Exception ignored) {}
                    if ("mysql".equalsIgnoreCase(dbType) || "tidb".equalsIgnoreCase(dbType) || "oceanbase".equalsIgnoreCase(dbType)) {
                        try {
                            stmt.execute("USE `" + targetDb + "`;");
                        } catch (Exception ignored) {}
                    }
                }

                // 安全防护：限制最大返回行数与查询超时（15 秒）
                stmt.setMaxRows(maxLimit);
                stmt.setQueryTimeout(15);

                try (ResultSet rs = stmt.executeQuery(trimmedSql)) {
                    ResultSetMetaData metaData = rs.getMetaData();
                    int columnCount = metaData.getColumnCount();

                    for (int i = 1; i <= columnCount; i++) {
                        String colName = metaData.getColumnLabel(i);
                        if (colName == null || colName.isEmpty()) {
                            colName = metaData.getColumnName(i);
                        }
                        columns.add(colName);
                    }

                    while (rs.next()) {
                        Map<String, Object> row = new LinkedHashMap<>();
                        for (int i = 1; i <= columnCount; i++) {
                            String colName = columns.get(i - 1);
                            Object val = rs.getObject(i);
                            String rawStr = val != null ? String.valueOf(val) : "NULL";

                            if (!isExplain) {
                                String colKey = colName.toLowerCase();
                                if (activeRules.containsKey(colKey) && !"NULL".equals(rawStr)) {
                                    DataMaskingRule rule = activeRules.get(colKey);
                                    String masked = maskingEngine.mask(rawStr, rule.getRuleType(), rule.getCustomRegex(), rule.getCustomReplacement());
                                    row.put(colName, masked);
                                    String rName = getFriendlyMaskRuleName(rule.getRuleType());
                                    maskedColumnsMap.put(colName, rName);
                                } else {
                                    row.put(colName, rawStr);
                                }
                            } else {
                                row.put(colName, rawStr);
                            }
                        }
                        rows.add(row);
                    }
                }

                long duration = System.currentTimeMillis() - startTime;
                String routeNode = isRoutedToSlave ? "SLAVE" : "MASTER";
                String routeDesc = isRoutedToSlave ? "已智能路由至只读从库 (Slave) 执行，分流主库负载" : "已路由至核心主库 (Master) 执行";

                SqlQueryResult result = SqlQueryResult.builder()
                        .success(true)
                        .databaseName(targetDb.isEmpty() ? "default" : targetDb)
                        .sql(trimmedSql)
                        .columns(columns)
                        .rows(rows)
                        .maskedColumns(maskedColumnsMap)
                        .totalRows(rows.size())
                        .durationMs(duration)
                        .routeNode(routeNode)
                        .routeDescription(routeDesc)
                        .build();

                recordAuditLogAsync(instance, targetDb, opType, trimmedSql, duration, rows.size(), "SUCCESS", null);
                return result;
            }
        }
    } catch (Exception e) {
            long duration = System.currentTimeMillis() - startTime;
            status = "FAILED";
            errorMsg = e.getMessage();
            log.warn("SQL 查询失败 (instance: {}, db: {}): {}", instance.getName(), dbName, e.getMessage());

            recordAuditLogAsync(instance, dbName, opType, trimmedSql, duration, 0, "FAILED", errorMsg);

            return SqlQueryResult.builder()
                    .success(false)
                    .databaseName(dbName != null ? dbName : "default")
                    .sql(trimmedSql)
                    .columns(new ArrayList<>())
                    .rows(new ArrayList<>())
                    .totalRows(0)
                    .durationMs(duration)
                    .errorMessage(errorMsg)
                    .routeNode(isRoutedToSlave ? "SLAVE" : "MASTER")
                    .build();
        }
    }

    private String resolveOpType(String sql) {
        if (sql == null) return "SELECT";
        String upper = sql.trim().toUpperCase();
        if (upper.startsWith("EXPLAIN")) return "EXPLAIN";
        if (upper.startsWith("SHOW")) return "SHOW";
        if (upper.startsWith("DESC")) return "DESC";
        return "SELECT";
    }

    private void recordAuditLogAsync(DbInstance instance, String dbName, String opType, String sqlText,
                                     long costMs, int resultRows, String status, String errorMsg) {
        try {
            if (!queryAuditConfigService.shouldAuditQuery(instance.getName(), dbName)) {
                return;
            }
            QueryAuditLog logEntity = new QueryAuditLog();
            logEntity.setTenantId(instance.getTenantId() != null ? instance.getTenantId() : "1");
            logEntity.setInstanceId(instance.getId());
            logEntity.setInstanceName(instance.getName());
            logEntity.setDbName(dbName != null ? dbName : "default");
            logEntity.setOpType(opType);
            logEntity.setSqlText(sqlText);
            logEntity.setCostMs(costMs);
            logEntity.setResultRows(resultRows);
            logEntity.setStatus(status);
            logEntity.setErrorMsg(errorMsg);
            logEntity.setClientIp("127.0.0.1");
            logEntity.setCreateTime(new Date());

            // 获取当前登录人
            Authentication auth = SecurityContextHolder.getContext().getAuthentication();
            if (auth != null && auth.getPrincipal() != null) {
                String principal = String.valueOf(auth.getPrincipal());
                com.baomidou.mybatisplus.core.conditions.query.QueryWrapper<SysUser> qw = new com.baomidou.mybatisplus.core.conditions.query.QueryWrapper<>();
                qw.eq("id_card", principal).or().eq("username", principal);
                SysUser user = sysUserMapper.selectOne(qw);
                if (user != null) {
                    logEntity.setUserId(user.getId());
                    logEntity.setUsername(user.getUsername());
                    logEntity.setRealName(user.getRealName());
                } else {
                    logEntity.setUsername(principal);
                    logEntity.setRealName(principal);
                }
            } else {
                logEntity.setUsername("admin");
                logEntity.setRealName("系统管理员");
            }

            queryAuditLogMapper.insert(logEntity);
        } catch (Exception e) {
            log.error("写入在线查询审计日志失败：{}", e.getMessage());
        }
    }

    private void validateReadOnlySql(String sql) {
        String upper = sql.trim().toUpperCase();
        if (upper.startsWith("INSERT") || upper.startsWith("UPDATE") || upper.startsWith("DELETE") ||
            upper.startsWith("DROP") || upper.startsWith("ALTER") || upper.startsWith("TRUNCATE") ||
            upper.startsWith("CREATE") || upper.startsWith("REPLACE") || upper.startsWith("GRANT") ||
            upper.startsWith("REVOKE")) {
            throw new BusinessException("A0403", "安全管控拦截：数据查询控制台仅支持安全只读查询（SELECT/SHOW/DESC/EXPLAIN），严禁在此直接执行 DDL/DML 修改操作！请前往工单中心提交变更审核工单。");
        }
    }

    private String resolvePassword(DbInstance instance) {
        if (instance.getPasswordCipher() == null || instance.getPasswordCipher().isEmpty()) {
            return "";
        }
        try {
            if (instance.getPasswordCipher().startsWith("SM4:")) {
                String hexKey = cn.hutool.core.util.HexUtil.encodeHexStr(aesKey.getBytes(StandardCharsets.UTF_8));
                return SmUtils.sm4Decrypt(instance.getPasswordCipher().substring(4), hexKey);
            }
            SecretKeySpec secretKey = new SecretKeySpec(aesKey.getBytes(StandardCharsets.UTF_8), "AES");
            Cipher cipher = Cipher.getInstance("AES");
            cipher.init(Cipher.DECRYPT_MODE, secretKey);
            byte[] decrypted = cipher.doFinal(Base64.getDecoder().decode(instance.getPasswordCipher()));
            return new String(decrypted, StandardCharsets.UTF_8);
        } catch (Exception e) {
            return instance.getPasswordCipher();
        }
    }

    private String buildJdbcUrlWithDatabase(String rawUrl, String dbName) {
        if (dbName == null || dbName.trim().isEmpty() || rawUrl == null) {
            return rawUrl;
        }
        int qIdx = rawUrl.indexOf('?');
        String params = qIdx > -1 ? rawUrl.substring(qIdx) : "";
        String base = qIdx > -1 ? rawUrl.substring(0, qIdx) : rawUrl;

        int lastSlash = base.lastIndexOf('/');
        if (lastSlash > "jdbc:mysql://".length()) {
            return base.substring(0, lastSlash + 1) + dbName.trim() + params;
        }
        return base + "/" + dbName.trim() + params;
    }

    private String getDriverClassName(String dbType) {
        if (dbType == null) return "com.mysql.cj.jdbc.Driver";
        switch (dbType.toLowerCase()) {
            case "oracle":
                return "oracle.jdbc.OracleDriver";
            case "postgresql":
            case "postgres":
                return "org.postgresql.Driver";
            case "sqlserver":
                return "com.microsoft.sqlserver.jdbc.SQLServerDriver";
            case "mysql":
            default:
                return "com.mysql.cj.jdbc.Driver";
        }
    }

    private String getFriendlyMaskRuleName(String ruleType) {
        if (ruleType == null) return "动态脱敏";
        switch (ruleType.toUpperCase()) {
            case "PHONE": return "手机号掩码";
            case "ID_CARD": return "身份证掩码";
            case "NAME": return "姓名掩码";
            case "EMAIL": return "邮箱掩码";
            case "BANK_CARD": return "银行卡掩码";
            case "ADDRESS": return "地址掩码";
            case "PASSWORD":
            case "FULL": return "密码/全掩码";
            case "HASH_SHA256":
            case "HASH": return "Hash哈希掩码";
            case "CUSTOM_REGEX": return "自定义正则脱敏";
            default: return ruleType + " 脱敏";
        }
    }
}
