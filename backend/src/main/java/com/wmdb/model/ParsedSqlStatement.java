package com.wmdb.model;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

/**
 * 解析后的单条 SQL 语句封装
 *
 * @author wm
 */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class ParsedSqlStatement {

    /**
     * 语句在脚本中的序号（从 1 开始）
     */
    private int index;

    /**
     * 原始 SQL 块（包含前面的注释说明）
     */
    private String rawSql;

    /**
     * 纯净可执行 SQL（去除影响行数等特殊注释）
     */
    private String executableSql;

    /**
     * 提取出的预期影响行数（若有 `-- 1` 或 `-- 影响行数: 1`）
     */
    private Integer expectedAffectedRows;

    /**
     * 语句类型：INSERT / UPDATE / DELETE / REPLACE / SELECT / DDL / OTHER
     */
    private String statementType;

    /**
     * 是否为 DML 变更语句
     */
    private boolean isDml;
}
