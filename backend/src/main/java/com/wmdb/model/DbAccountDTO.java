package com.wmdb.model;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

/**
 * 数据库账号 DTO
 *
 * @author wm
 */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class DbAccountDTO {

    /**
     * 账号名称
     */
    private String user;

    /**
     * 允许连接的主机 (如 %, localhost, 192.168.%)
     */
    private String host;

    /**
     * 认证插件
     */
    private String plugin;

    /**
     * 账号锁定状态
     */
    private String accountLocked;

    /**
     * 权限概览
     */
    private String privileges;
}
