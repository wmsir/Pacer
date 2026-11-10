package com.wmdb.model;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

/**
 * 数据库详细元数据 DTO
 *
 * @author wm
 */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class DbSchemaDetailDTO {

    /**
     * 数据库名称 (Schema)
     */
    private String dbName;

    /**
     * 表数量
     */
    private int tableCount;

    /**
     * 数据总占用大小 (MB)
     */
    private double dataSizeMB;

    /**
     * 默认字符集 (如 utf8mb4)
     */
    private String charset;

    /**
     * 默认排序规则 (如 utf8mb4_general_ci)
     */
    private String collation;

    /**
     * 是否为系统保留库
     */
    private boolean isSystem;

    /**
     * 备注 / 描述
     */
    private String comment;
}
