package com.wmdb.model;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

import java.util.ArrayList;
import java.util.List;
import java.util.Map;

/**
 * SQL 在线查询返回结果封装
 *
 * @author wm
 */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class SqlQueryResult {

    /**
     * 是否执行成功
     */
    private boolean success;

    /**
     * 目标数据库名称
     */
    private String databaseName;

    /**
     * 执行的 SQL
     */
    private String sql;

    /**
     * 结果集字段列名清单
     */
    @Builder.Default
    private List<String> columns = new ArrayList<>();

    /**
     * 结果集数据行（键值对）
     */
    @Builder.Default
    private List<Map<String, Object>> rows = new ArrayList<>();

    /**
     * 结果集中已脱敏字段及其脱敏规则说明映射 (字段名 -> 规则名，如 mobile -> 手机号掩码)
     */
    @Builder.Default
    private Map<String, String> maskedColumns = new java.util.LinkedHashMap<>();

    /**
     * 返回总行数
     */
    private int totalRows;

    /**
     * 执行耗时 (ms)
     */
    private long durationMs;

    /**
     * 错误信息（如有）
     */
    private String errorMessage;

    /**
     * 读写分离路由节点类型 (MASTER 主库 / SLAVE 只读从库)
     */
    private String routeNode;

    /**
     * 路由说明 (例如: 已自动路由至只读从库 (Slave) 执行，保护核心主库)
     */
    private String routeDescription;
}
