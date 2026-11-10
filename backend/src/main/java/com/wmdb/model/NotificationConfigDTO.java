package com.wmdb.model;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

import java.util.List;

/**
 * 消息通知与告警配置 DTO (包含企业微信、钉钉、飞书与紧急电话语音外呼)
 *
 * @author wm
 */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class NotificationConfigDTO {

    /**
     * 🟢 企业微信配置 (支持 WebService WSDL / SOAP 与 Webhook)
     */
    private WechatConfig wechat;

    /**
     * 🔵 阿里钉钉配置 (支持 Webhook 机器人与自建应用工作通知)
     */
    private DingtalkConfig dingtalk;

    /**
     * 🟣 字节飞书 / Lark 配置 (支持自定义群机器人 Webhook 与交互卡片)
     */
    private FeishuConfig feishu;

    /**
     * 📱 移动短信服务配置 (支持 阿里云、腾讯云、华为云、自建 HTTP 短信网关)
     */
    private SmsConfig sms;

    /**
     * 📞 紧急电话语音外呼配置 (支持阿里云语音 / 腾讯云语音 / 自建 SIP 语音网关)
     */
    private VoiceCallConfig voiceCall;

    /**
     * ⚙️ 全局推送频次与事件策略
     */
    private PolicyConfig policy;

    @Data
    @Builder
    @NoArgsConstructor
    @AllArgsConstructor
    public static class WechatConfig {
        private Boolean enabled;
        private String mode; // WEBSERVICE 或 WEBHOOK
        private String wsdlEndpoint; // 例如 http://9.0.17.52:8083/wechat-wbs/services/ExternalDeptMessageService?wsdl
        private String sysId; // APP_CONFIG sys_id
        private String sysIdPass; // APP_CONFIG sys_id_pass
        private String sysFlag; // APP_CONFIG sys_flag (例如 1)
        private String webhookUrl; // 企微群机器人 Webhook
        private Integer retryTimes; // 失败重试次数 (默认 3)
        private Integer frequencyLimit; // 单人推送频次上限 (条/分钟，0表示不限)
    }

    @Data
    @Builder
    @NoArgsConstructor
    @AllArgsConstructor
    public static class DingtalkConfig {
        private Boolean enabled;
        private String webhookUrl; // 钉钉机器人 Webhook (https://oapi.dingtalk.com/robot/send?access_token=...)
        private String secret; // 加签秘钥 (SEC...)
        private Boolean atAll; // 是否 @所有人
        private Integer frequencyLimit; // 单群频次限制 (条/分钟)
    }

    @Data
    @Builder
    @NoArgsConstructor
    @AllArgsConstructor
    public static class FeishuConfig {
        private Boolean enabled;
        private String mode; // WEBHOOK (自定义群机器人) 或 APP (自建应用)
        private String webhookUrl; // 飞书群机器人 Webhook (https://open.feishu.cn/open-apis/bot/v2/hook/...)
        private String secret; // 安全设置加签秘钥 (签名校验 Secret)
        private String appId; // 飞书自建应用 App ID (cli_xxx)
        private String appSecret; // 飞书自建应用 App Secret
        private Integer frequencyLimit; // 单群频次限制 (条/分钟)
    }

    @Data
    @Builder
    @NoArgsConstructor
    @AllArgsConstructor
    public static class SmsConfig {
        /**
         * 通道启用状态
         */
        private Boolean enabled;

        /**
         * 短信服务商: ALIYUN (阿里巴巴/阿里云短信), TENCENT (腾讯云短信), HUAWEI (华为云短信), CUSTOM_HTTP (自建网关)
         */
        private String provider;

        /**
         * 短信签名 (例如: wmDB云平台、企业数据治理)
         */
        private String signName;

        /**
         * 通用审批与催办通知模版 Code (如 阿里云 SMS_283910243 或 腾讯云 1892014)
         */
        private String templateCode;

        // --- 阿里巴巴 (阿里云短信) 配置参数 ---
        private String aliyunAccessKeyId;
        private String aliyunAccessKeySecret;
        private String aliyunRegionId; // 默认 cn-hangzhou

        // --- 腾讯云短信 配置参数 ---
        private String tencentSecretId;
        private String tencentSecretKey;
        private String tencentSdkAppId; // 短信 SDKAppID (如 1400xxxxxx)
        private String tencentRegion;   // 默认 ap-guangzhou

        // --- 华为云 / 自建 HTTP 短信网关 ---
        private String customApiEndpoint; // 自建网关 API 端点 URL
        private String customApiKey;      // 认证鉴权 Token / Header

        /**
         * 单人单日短信发送上限 (条/人/天，默认 20 条，防刷保护)
         */
        private Integer dailyLimitPerUser;

        /**
         * 失败自动重试次数 (默认 2)
         */
        private Integer retryTimes;
    }

    @Data
    @Builder
    @NoArgsConstructor
    @AllArgsConstructor
    public static class VoiceCallConfig {
        private Boolean enabled;
        private String provider; // ALIYUN (阿里云语音), TENCENT (腾讯云), SIP_GATEWAY (自建SIP网关)
        private String endpoint; // 网关地址或 API Endpoint
        private String accessKeyId; // 访问凭证 Key
        private String accessKeySecret; // 访问凭证 Secret
        private String templateCode; // 语音外呼模版 Code (TTS)
        private String calledShowNumber; // 外呼显示主叫号码
        private List<String> emergencyContacts; // 紧急联系人电话列表 (用于 P0/高危告警外呼)
        private List<String> triggerEvents; // 触发事件: TICKET_FAILED_P0, INSTANCE_DOWN, BLOCKED_SESSION
    }

    @Data
    @Builder
    @NoArgsConstructor
    @AllArgsConstructor
    public static class PolicyConfig {
        private Boolean notifyOnSubmit; // 工单提交时通知审批人
        private Boolean notifyOnAudited; // 审批通过/驳回通知申请人
        private Boolean notifyOnExecuted; // 执行成功通知
        private Boolean notifyOnFailed; // 执行失败通知
        private Boolean notifyOnRiskIntercept; // 高危 SQL 拦截告警
        private Boolean quietHoursEnabled; // 是否开启夜间静默期
        private String quietHoursStart; // 22:00
        private String quietHoursEnd; // 08:00

        // --- 三大提醒方式分类场景使用策略矩阵 (Category Usage Matrix) ---
        @Builder.Default
        private Boolean dailyNotifyUseIm = true;      // 1. 日常工单通知走协作即时通讯 (企微/钉钉/飞书)
        @Builder.Default
        private Boolean urgeNotifyUseSms = true;      // 2. 加急催办/待办超时同步发送短信 (SMS)
        @Builder.Default
        private Boolean emergencyUseVoiceCall = true; // 3. P0 严重故障/实例宕机触发紧急电话语音直拨 (Voice Call)
        @Builder.Default
        private Boolean failedNotifyUseSms = false;   // 4. 工单执行失败/高危拦截同步发送短信 (SMS)
    }

    @Data
    public static class TestChannelRequest {
        private String channel; // WECHAT, DINGTALK, FEISHU, SMS, VOICE_CALL
        private String target; // 接收人工号/ERP、手机号或群目标
        private String message; // 测试消息内容
        private String provider; // 短信厂商 (针对 SMS: ALIYUN, TENCENT 等)
    }

    @Data
    public static class TestChannelResponse {
        private Boolean success;
        private Long latencyMs;
        private String message;
        private String rawResponse;
    }
}
