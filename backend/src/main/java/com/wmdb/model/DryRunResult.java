package com.wmdb.model;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

import java.util.ArrayList;
import java.util.List;

/**
 * 完整脚本预执行校验聚合结果
 *
 * @author wm
 */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class DryRunResult {

    /**
     * 是否全部校验通过（所有带注解的 DML 实际影响行数均与预期一致）
     */
    private boolean passed;

    /**
     * 语句总数
     */
    private int totalStatements;

    /**
     * DML 语句总数
     */
    private int dmlCount;

    /**
     * 预期总影响行数
     */
    private int totalExpectedRows;

    /**
     * 实际预执行总影响行数
     */
    private int totalActualRows;

    /**
     * 总结提示信息
     */
    private String summaryMessage;

    /**
     * 各条语句详细校验清单
     */
    @Builder.Default
    private List<DryRunItem> items = new ArrayList<>();
}
