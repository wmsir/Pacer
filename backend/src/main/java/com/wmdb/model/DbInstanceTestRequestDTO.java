package com.wmdb.model;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

/**
 * 数据库实例连接测试请求参数 DTO
 *
 * @author wm
 */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class DbInstanceTestRequestDTO {

    /**
     * 实例 ID（可选，已保存实例直接按 ID 测试）
     */
    private Long id;

    /**
     * 实例名称
     */
    private String name;

    /**
     * 数据库类型 (mysql, dameng, oracle, postgresql, tidb, oceanbase, opengauss, kingbase)
     */
    private String dbType;

    /**
     * 数据库主机 / IP 地址
     */
    private String host;

    /**
     * 数据库端口
     */
    private Integer port;

    /**
     * 默认数据库名 (Schema)
     */
    private String databaseName;

    /**
     * 数据库账号
     */
    private String username;

    /**
     * 连接密码（明文或已加密密文）
     */
    private String password;

    /**
     * 完整 JDBC URL（可选，若提供则优先使用）
     */
    private String jdbcUrl;
}
