package com.wmdb.model;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

/**
 * SpEL 表达式沙箱求值响应结果
 */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class SpelEvaluationResultDTO {

    /**
     * 表达式语法是否有效合法
     */
    private Boolean syntaxValid;

    /**
     * 判定是否命中 (true/false)
     */
    private Boolean matched;

    /**
     * 原始求值结果对象
     */
    private Object result;

    /**
     * 业务判定解释说明
     */
    private String explanation;

    /**
     * 错误信息（若有语法或求值异常）
     */
    private String errorMessage;
}
