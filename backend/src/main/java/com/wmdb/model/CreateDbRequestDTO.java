package com.wmdb.model;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

/**
 * 创建数据库请求 DTO
 *
 * @author wm
 */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class CreateDbRequestDTO {

    /**
     * 数据库名称 (Schema)
     */
    private String dbName;

    /**
     * 字符集 (如 utf8mb4, gbk, latin1, utf8)
     */
    private String charset;

    /**
     * 排序规则 (如 utf8mb4_general_ci, utf8mb4_0900_ai_ci, gbk_chinese_ci)
     */
    private String collation;

    /**
     * 备注说明
     */
    private String comment;
}
