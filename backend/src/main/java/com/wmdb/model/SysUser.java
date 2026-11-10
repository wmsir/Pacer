package com.wmdb.model;

import com.baomidou.mybatisplus.annotation.IdType;
import com.baomidou.mybatisplus.annotation.TableId;
import com.baomidou.mybatisplus.annotation.TableName;
import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

/**
 * 系统用户实体类
 * <p>
 * 映射 sys_user 表，存储用户名、手机号、真实姓名、身份证、密码、角色及归属资源组等信息。
 * </p>
 *
 * @author wm
 * @date 2023-10-25
 */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
@TableName("sys_user")
public class SysUser {

    private String tenantId;

    /**
     * 主键 ID
     */
    @TableId(type = IdType.AUTO)
    private Long id;

    /**
     * 用户名（支持用户名登录）
     */
    private String username;

    /**
     * 手机号码（支持手机号登录）
     */
    private String phone;

    /**
     * 真实姓名
     */
    private String realName;

    /**
     * 身份证号码（系统实名认证与登录名）
     */
    private String idCard;

    /**
     * 邮箱
     */
    private String email;

    /**
     * 所属业务资源组（如：车险承保资源组）
     */
    private String resourceGroup;

    /**
     * 加密存储的密码（SM3 哈希）
     */
    private String passwordCipher;

    /**
     * BCrypt 密码哈希（兼容字段）
     */
    private String passwordHash;

    /**
     * 角色标识（如：ADMIN, DBA, DEV, DEV_LEAD, AUDITOR，支持多角色以逗号或 JSON 存储）
     */
    private String role;

    /**
     * 微信号
     */
    private String wechat;

    /**
     * 企业微信 UserID / 企微账号
     */
    private String workWechat;

    /**
     * 钉钉账号 / 手机号
     */
    private String dingtalk;

    /**
     * 飞书账号
     */
    private String feishu;

    /**
     * 所属部门 / 业务线 (如: 保险科技核心研发部)
     */
    private String department;

    /**
     * 员工工号 (如: WM-9527)
     */
    private String jobNo;

    /**
     * 消息通知渠道偏好设置 (JSON 格式)
     */
    private String notificationPrefs;

    /**
     * 用户级自定义已授权功能页签与权限列表（JSON 格式，覆盖或补充角色默认权限）
     */
    private String permissions;

    /**
     * 工单数据可见范围配置: ALL (全平台), RESOURCE_GROUP (所属资源组), SELF (仅本人参与), NULL (角色自适应)
     */
    private String ticketDataScope;

    /**
     * 状态（1-正常，0-禁用）
     */
    private Integer status;

    /**
     * 邮箱是否已核验（1-已验证，0-未验证）
     */
    private Integer emailVerified;

    /**
     * 手机短信是否已核验（1-已验证，0-未验证）
     */
    private Integer phoneVerified;
}
