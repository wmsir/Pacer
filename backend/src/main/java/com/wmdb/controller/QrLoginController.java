package com.wmdb.controller;

import com.wmdb.common.Result;
import com.wmdb.common.annotation.RateLimit;
import com.wmdb.model.QrLoginDTO;
import com.wmdb.service.QrLoginService;
import io.swagger.v3.oas.annotations.Operation;
import io.swagger.v3.oas.annotations.tags.Tag;
import lombok.Data;
import lombok.RequiredArgsConstructor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.web.bind.annotation.*;

/**
 * 国内主流 APP 扫码登录控制器
 * <p>
 * 统一提供企业微信 (WeCom)、阿里钉钉 (DingTalk)、字节飞书 (Feishu) 及企业统一 SSO 扫码登录。
 * </p>
 *
 * @author wm
 */
@Tag(name = "移动办公 APP 扫码登录", description = "支持企业微信、阿里钉钉、字节飞书与统一SSO扫码登录、状态轮询与开放平台回调")
@Slf4j
@RestController
@RequestMapping("/api/v1/auth/qr")
@RequiredArgsConstructor
public class QrLoginController {

    private final QrLoginService qrLoginService;

    /**
     * 生成扫码登录会话与二维码数据
     */
    @Operation(summary = "生成扫码登录二维码", description = "支持渠道：WECOM (企业微信), DINGTALK (钉钉), FEISHU (飞书), SSO (统一单点登录)")
    @RateLimit(time = 60, count = 30)
    @PostMapping("/generate")
    public Result<QrLoginDTO> generateQr(@RequestParam(value = "channel", defaultValue = "WECOM") String channel) {
        QrLoginDTO dto = qrLoginService.generateQr(channel);
        return Result.success(dto);
    }

    /**
     * 轮询扫码会话状态
     */
    @Operation(summary = "轮询扫码状态", description = "获取当前二维码状态（WAITING / SCANNED / CONFIRMED / EXPIRED）")
    @GetMapping("/status")
    public Result<QrLoginDTO> checkStatus(@RequestParam("qrKey") String qrKey) {
        QrLoginDTO dto = qrLoginService.checkStatus(qrKey);
        return Result.success(dto);
    }

    /**
     * 模拟移动端扫码与确认授权（开发与测试环境快速验证）
     */
    @Operation(summary = "模拟移动端扫码授权", description = "模拟员工在移动端扫码并点击确认授权，直接完成登录")
    @PostMapping("/mock-scan")
    public Result<QrLoginDTO> mockScan(@RequestBody MockScanRequest req) {
        QrLoginDTO dto = qrLoginService.mockScanAndConfirm(req.getQrKey(), req.getAccount());
        return Result.success(dto);
    }

    /**
     * 企业开放平台 Webhook 回调预留
     */
    @Operation(summary = "开放平台 OAuth2 回调接口", description = "接收企业微信/钉钉/飞书的 OAuth 授权回调")
    @GetMapping("/callback/{channel}")
    public Result<String> handleCallback(@PathVariable("channel") String channel,
                                         @RequestParam("code") String code,
                                         @RequestParam("state") String state) {
        String msg = qrLoginService.handleOAuthCallback(channel, code, state);
        return Result.success(msg);
    }

    @Data
    public static class MockScanRequest {
        private String qrKey;
        private String account;
    }
}
