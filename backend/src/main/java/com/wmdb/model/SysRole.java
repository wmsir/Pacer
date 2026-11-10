package com.wmdb.model;

import com.baomidou.mybatisplus.annotation.IdType;
import com.baomidou.mybatisplus.annotation.TableId;
import com.baomidou.mybatisplus.annotation.TableName;
import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

/**
 * 角色实体类
 * <p>
 * 映射 sys_role 表。
 * </p>
 *
 * @author wm
 * @date 2023-10-25
 */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
@TableName("sys_role")
public class SysRole {

    private String tenantId;

    /**
     * 主键 ID
     */
    @TableId(type = IdType.AUTO)
    private Long id;

    /**
     * 角色编码
     */
    private String roleCode;

    /**
     * 角色名称
     */
    private String roleName;

    /**
     * 角色描述
     */
    private String description;

    /**
     * 允许访问的系统页签与功能权限列表（JSON 字符串格式，存储路由路径列表如 ["/dashboard", "/ticket-list", ...]）
     */
    private String permissions;

    /**
     * 当前角色包含的用户成员显示名称列表 (如 ["系统超级管理员 (admin)", "业务开发组长 (testadmin2)"])
     */
    @com.baomidou.mybatisplus.annotation.TableField(exist = false)
    private java.util.List<String> memberNames;

    /**
     * 当前角色成员总数
     */
    @com.baomidou.mybatisplus.annotation.TableField(exist = false)
    private Integer memberCount;
}
