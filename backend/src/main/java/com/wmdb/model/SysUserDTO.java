package com.wmdb.model;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

/**
 * 系统用户响应 DTO
 *
 * @author wm
 */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class SysUserDTO {

    private Long id;
    private String tenantId;
    private String username;
    private String realName;

    /**
     * 智能消歧显示名称（如重名时显示 "张伟 (尾号: 011234)"）
     */
    private String displayName;

    private String idCard;
    private String phone;
    private String email;
    private String role;
    /**
     * 所属角色列表（支持配置多角色）
     */
    private java.util.List<String> roles;
    private String resourceGroup;
    /**
     * 所属业务资源组列表（支持一个用户归属多个资源组）
     */
    private java.util.List<String> resourceGroups;
    /**
     * 用户生效的全部功能页签权限列表
     */
    private java.util.List<String> permissions;
    /**
     * 用户独立自定义页签权限（若未单独自定义则为空）
     */
    private java.util.List<String> customPermissions;
    private String wechat;
    private String workWechat;
    private String dingtalk;
    private String feishu;
    private String department;
    private String jobNo;
    private String notificationPrefs;
    private String ticketDataScope;
    private Integer status;
}
