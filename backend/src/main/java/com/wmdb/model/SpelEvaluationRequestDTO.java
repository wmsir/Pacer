package com.wmdb.model;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

import java.util.Map;

/**
 * SpEL 表达式沙箱求值请求参数
 */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class SpelEvaluationRequestDTO {

    /**
     * SpEL 条件表达式 (如: #{affectRows > 2000 || hasDdl == true})
     */
    private String spelExpression;

    /**
     * 上下文变量字典 (如: { "affectRows": 2500, "hasDdl": true, "environment": "PROD" })
     */
    private Map<String, Object> context;
}
