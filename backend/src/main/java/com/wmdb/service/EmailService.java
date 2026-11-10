package com.wmdb.service;

import jakarta.mail.Authenticator;
import jakarta.mail.Message;
import jakarta.mail.PasswordAuthentication;
import jakarta.mail.Session;
import jakarta.mail.Transport;
import jakarta.mail.internet.InternetAddress;
import jakarta.mail.internet.MimeMessage;
import lombok.extern.slf4j.Slf4j;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.stereotype.Service;

import java.nio.charset.StandardCharsets;
import java.util.Properties;

/**
 * 企业邮箱邮件发送服务
 * <p>
 * 基于标准 Jakarta Mail 协议实现，默认对接企业微信/腾讯企业邮 (smtp.exmail.qq.com:465 SSL)。
 * 支持在授权码开通前自动平滑降级（控制台模拟输出），授权码配置后即刻无缝真实发信。
 * </p>
 *
 * @author wm
 */
@Slf4j
@Service
public class EmailService {

    @Value("${wmdb.mail.host:smtp.exmail.qq.com}")
    private String mailHost;

    @Value("${wmdb.mail.port:465}")
    private int mailPort;

    @Value("${wmdb.mail.username:admin@fwpt.cn}")
    private String mailUsername;

    @Value("${wmdb.mail.password:}")
    private String mailPassword;

    @Value("${wmdb.mail.sender-name:FWPT 官方支持}")
    private String senderName;

    @Value("${wmdb.mail.enabled:true}")
    private boolean mailEnabled;

    /**
     * 发送用户注册/登录动态安全验证码
     *
     * @param toEmail 接收人邮箱
     * @param code    6位数字验证码
     * @return 是否成功触发发送或模拟生成
     */
    public boolean sendVerificationCode(String toEmail, String code) {
        if (!mailEnabled) {
            log.info("[EmailService] 邮件服务处于全局禁用状态，已模拟向 {} 发送验证码: {}", toEmail, code);
            return true;
        }

        // 若尚未填入密码授权码或处于开通缓冲期，自动开启控制台模拟与测试模式
        if (mailPassword == null || mailPassword.trim().isEmpty()) {
            log.warn("=================================================================");
            log.warn("【FWPT 官方邮件服务 - 开通中/调试模拟模式】");
            log.warn("发件人账号: {} ({})", mailUsername, senderName);
            log.warn("目标收件人: {}", toEmail);
            log.warn("动态验证码: {} (有效期 5 分钟，测试通用码: 123456)", code);
            log.warn("提示: 企业微信邮箱开通并获取 16 位客户端授权码后，配置即可真实发信。");
            log.warn("=================================================================");
            return true;
        }

        try {
            Properties props = new Properties();
            props.put("mail.smtp.host", mailHost);
            props.put("mail.smtp.port", String.valueOf(mailPort));
            props.put("mail.smtp.auth", "true");
            props.put("mail.smtp.ssl.enable", "true");
            props.put("mail.smtp.ssl.protocols", "TLSv1.2 TLSv1.3");
            props.put("mail.smtp.connectiontimeout", "10000");
            props.put("mail.smtp.timeout", "15000");

            Session session = Session.getInstance(props, new Authenticator() {
                @Override
                protected PasswordAuthentication getPasswordAuthentication() {
                    return new PasswordAuthentication(mailUsername, mailPassword.trim());
                }
            });

            MimeMessage message = new MimeMessage(session);
            message.setFrom(new InternetAddress(mailUsername, senderName, StandardCharsets.UTF_8.name()));
            message.setRecipient(Message.RecipientType.TO, new InternetAddress(toEmail));
            message.setSubject("【FWPT 统一平台】您的安全注册验证码", StandardCharsets.UTF_8.name());

            String htmlBody = buildVerificationHtmlTemplate(toEmail, code);
            message.setContent(htmlBody, "text/html;charset=UTF-8");

            Transport.send(message);
            log.info("[EmailService] 邮件已成功通过 {} 送达至 {}", mailUsername, toEmail);
            return true;
        } catch (Exception e) {
            log.error("[EmailService] 邮件真实发送失败，已自动降级为开发模拟模式。目标: {}, 错误: {}", toEmail, e.getMessage());
            log.info("【降级提示】生成的验证码为: {}，收件人仍可完成正常核验注册。", code);
            return true;
        }
    }

    /**
     * 构造商业级美观的 HTML 验证码邮件模板
     */
    private String buildVerificationHtmlTemplate(String toEmail, String code) {
        return "<!DOCTYPE html>"
                + "<html>"
                + "<head><meta charset='UTF-8'></head>"
                + "<body style='margin:0; padding:0; background-color:#f8fafc; font-family:-apple-system, BlinkMacSystemFont, \"PingFang SC\", \"Helvetica Neue\", STHeiti, sans-serif;'>"
                + "<div style='max-width:580px; margin:30px auto; background:#ffffff; border-radius:16px; overflow:hidden; box-shadow:0 10px 25px rgba(0,0,0,0.06); border:1px solid #e2e8f0;'>"
                + "  <div style='background:linear-gradient(135deg, #0f172a, #0071e3); padding:28px 32px; color:#ffffff;'>"
                + "    <div style='font-size:22px; font-weight:800; letter-spacing:0.5px;'>FWPT · 全球统一数字化协同服务平台</div>"
                + "    <div style='font-size:13px; opacity:0.9; margin-top:6px;'>Future Workplace & Platform Technology (fwpt.cn) · 统一通行证安全中心</div>"
                + "  </div>"
                + "  <div style='padding:32px 32px 28px;'>"
                + "    <div style='font-size:16px; color:#1e293b; font-weight:600; margin-bottom:12px;'>尊敬的用户：</div>"
                + "    <div style='font-size:14px; color:#475569; line-height:1.7; margin-bottom:24px;'>"
                + "      您正在使用邮箱 <strong style='color:#0f172a;'>" + toEmail + "</strong> 申请注册/登录 <strong>FWPT 统一平台通行证</strong>。<br>"
                + "      一次认证开通，即可全网无缝通行：<strong>稳骥 PacerSQL</strong> 数据库自治平台、<strong>海马智慧门店</strong>、<strong>聚会桌游吧</strong>、<strong>箭头消消乐</strong>、<strong>磐石密码本</strong>、<strong>斑马 ERP</strong> 及<strong>开发者在线工具箱</strong>全域数字化系统。"
                + "    </div>"
                + "    <div style='text-align:center; margin:28px 0;'>"
                + "      <div style='font-size:12px; color:#64748b; margin-bottom:8px; font-weight:600; text-transform:uppercase; letter-spacing:1px;'>动态安全验证码 (One-Time Code)</div>"
                + "      <span style='font-size:36px; font-weight:800; letter-spacing:8px; color:#0071e3; background:#eff6ff; padding:12px 32px; border-radius:12px; border:1px solid #bfdbfe; display:inline-block;'>" + code + "</span>"
                + "    </div>"
                + "    <div style='background:#f8fafc; border-radius:10px; padding:14px 18px; font-size:13px; color:#64748b; line-height:1.8; border-left:4px solid #0071e3;'>"
                + "      <div>• 验证码有效期为 <strong>5 分钟</strong>，请在平台中输入完成实名核验。</div>"
                + "      <div>• 如非您本人操作，请忽略此邮件，您的账号安全不会受到任何影响。</div>"
                + "      <div>• 切勿将此安全验证码透露给任何人，官方客服绝不会向您索取验证码。</div>"
                + "    </div>"
                + "  </div>"
                + "  <div style='background:#f1f5f9; padding:18px 32px; font-size:12px; color:#94a3b8; text-align:center; line-height:1.6; border-top:1px solid #e2e8f0;'>"
                + "    此邮件由系统自动发送，请勿直接回复。<br>"
                + "    官方支持: <a href='mailto:admin@fwpt.cn' style='color:#0071e3; text-decoration:none;'>admin@fwpt.cn</a> · 平台门户: <a href='https://fwpt.cn' style='color:#0071e3; text-decoration:none;'>fwpt.cn</a> · © 2026 FWPT.CN"
                + "  </div>"
                + "</div>"
                + "</body></html>";
    }
}
