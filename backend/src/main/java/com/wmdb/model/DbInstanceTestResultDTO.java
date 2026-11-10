package com.wmdb.model;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

/**
 * 数据库实例连接测试响应结果 DTO
 *
 * @author wm
 */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class DbInstanceTestResultDTO {

    /**
     * 连接是否成功
     */
    private boolean success;

    /**
     * 网络连接与握手耗时 (毫秒)
     */
    private long latencyMs;

    /**
     * 数据库产品名称 (如 MySQL, DM DBMS, Oracle 等)
     */
    private String databaseProductName;

    /**
     * 数据库产品版本号 (如 8.0.28, 19.3.0 等)
     */
    private String databaseProductVersion;

    /**
     * 驱动名称
     */
    private String driverName;

    /**
     * 提示说明
     */
    private String message;

    /**
     * 失败详细报错原因
     */
    private String errorMessage;
}
