package com.wmdb.model;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

/**
 * 脱敏算法描述 DTO
 *
 * @author wm
 */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class MaskingAlgorithmDTO {

    /**
     * 算法代码 (PHONE, ID_CARD, NAME, EMAIL, BANK_CARD, ADDRESS, PASSWORD, CUSTOM_REGEX)
     */
    private String type;

    /**
     * 算法中文名称
     */
    private String name;

    /**
     * 适用场景说明
     */
    private String description;

    /**
     * 样例原始值
     */
    private String sampleOriginal;

    /**
     * 样例脱敏后值
     */
    private String sampleMasked;
}
