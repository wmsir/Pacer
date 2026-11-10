package com.wmdb.model;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

/**
 * 数据库全局运行参数 DTO
 *
 * @author wm
 */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class DbVariableDTO {

    /**
     * 变量名 (如 max_connections, innodb_buffer_pool_size)
     */
    private String name;

    /**
     * 当前运行值
     */
    private String value;

    /**
     * 参数类别 (如 连接与网络, 缓冲池, 字符集, 事务与日志, 缓存)
     */
    private String category;

    /**
     * 中文描述与说明
     */
    private String description;
}
