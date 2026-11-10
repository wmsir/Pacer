package com.wmdb.model;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

/**
 * 扫码登录会话 DTO
 *
 * @author wm
 */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class QrLoginDTO {

    /**
     * 二维码唯一标识 Key
     */
    private String qrKey;

    /**
     * 扫码接入渠道：WECOM (企业微信), DINGTALK (钉钉), FEISHU (飞书), SSO (统一单点登录)
     */
    private String channel;

    /**
     * 渠道名称（如 "企业微信扫码", "钉钉移动办公扫码"）
     */
    private String channelName;

    /**
     * 二维码原始链接 / OAuth 跳转地址
     */
    private String qrUrl;

    /**
     * 二维码内置 Base64/SVG 图像或内容数据
     */
    private String qrContent;

    /**
     * 状态：WAITING (等待扫码), SCANNED (已扫码待确认), CONFIRMED (已确认登录成功), EXPIRED (已过期)
     */
    private String status;

    /**
     * 状态文字提示
     */
    private String statusMsg;

    /**
     * 有效期倒计时（秒）
     */
    private Long expireSeconds;

    /**
     * 确认登录后签发的 JWT Token
     */
    private String token;

    /**
     * 登录成功的用户信息
     */
    private SysUserDTO user;
}
