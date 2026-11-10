package com.wmdb.model;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

/**
 * 创建/修改数据库账号请求 DTO
 *
 * @author wm
 */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class CreateAccountRequestDTO {

    /**
     * 账号名称
     */
    private String username;

    /**
     * 允许连接的主机 (默认 %)
     */
    private String host;

    /**
     * 连接密码
     */
    private String password;

    /**
     * 授权数据库名称 (% 代表全部)
     */
    private String databaseName;

    /**
     * 权限类型 (ALL, SELECT_ONLY, DML_DQL, DDL_DML_DQL)
     */
    private String privilegeType;
}
