package com.wmdb.model;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

/**
 * 脱敏效果预览请求 DTO
 *
 * @author wm
 */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class MaskingPreviewRequestDTO {

    /**
     * 脱敏算法类型
     */
    private String ruleType;

    /**
     * 待脱敏测试样本值
     */
    private String sampleValue;

    /**
     * 自定义正则表达式
     */
    private String customRegex;

    /**
     * 自定义替换表达式
     */
    private String customReplacement;
}
