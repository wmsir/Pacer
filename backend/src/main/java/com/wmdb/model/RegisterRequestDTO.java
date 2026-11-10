package com.wmdb.model;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

/**
 * 多渠道用户注册请求 DTO
 *
 * @author wm
 */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class RegisterRequestDTO {

    /**
     * 注册方式：ACCOUNT (用户名密码), PHONE (手机号验证码/密码), EMAIL (邮箱密码)
     */
    private String registerType;

    /**
     * 用户账号 / 登录名
     */
    private String username;

    /**
     * 用户真实姓名（必填，用于系统内身份识别与同名消歧）
     */
    private String realName;

    /**
     * 身份证号码（选填/必填，用于实名安全认证及同名消歧尾号显示）
     */
    private String idCard;

    /**
     * 手机号码
     */
    private String phone;

    /**
     * 电子邮箱
     */
    private String email;

    /**
     * 登录密码
     */
    private String password;

    /**
     * 短信/邮箱验证码 (可选)
     */
    private String code;

    /**
     * 所属业务资源组 (如：车险承保资源组、销管系统资源组)
     */
    private String resourceGroup;

    /**
     * 申请角色 (DEV 开发工程师, DEV_LEAD 开发组长, DBA 管理员, AUDITOR 审计员)
     */
    private String role;
}
