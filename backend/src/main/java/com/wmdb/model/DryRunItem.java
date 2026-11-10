package com.wmdb.model;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

/**
 * 单条 SQL 预执行校验结果明细
 *
 * @author wm
 */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class DryRunItem {

    /**
     * 语句序号
     */
    private int index;

    /**
     * 语句类型 (INSERT, UPDATE, DELETE, SELECT, DDL 等)
     */
    private String statementType;

    /**
     * 是否为 DML 语句
     */
    private boolean isDml;

    /**
     * SQL 摘要 / 语句内容
     */
    private String sqlSnippet;

    /**
     * 注解中指定的预期影响行数 (如 `-- 1`)
     */
    private Integer expectedRows;

    /**
     * 事务内实际预执行影响行数
     */
    private Integer actualRows;

    /**
     * 校验状态：MATCHED (一致), MISMATCHED (不匹配), SKIPPED (非DML), ERROR (执行异常)
     */
    private String status;

    /**
     * 校验详情说明
     */
    private String message;

    /**
     * 预执行耗时 (ms)
     */
    private long durationMs;
}
