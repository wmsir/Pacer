package com.wmdb.model;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

/**
 * 个人资料修改请求 DTO
 *
 * @author wm
 */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class UserProfileUpdateDTO {

    /**
     * 真实姓名
     */
    private String realName;

    /**
     * 手机号码
     */
    private String phone;

    /**
     * 邮箱
     */
    private String email;

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
     * 所属部门 / 业务线
     */
    private String department;

    /**
     * 员工工号
     */
    private String jobNo;

    /**
     * 消息通知渠道偏好设置 (JSON)
     */
    private String notificationPrefs;
}
