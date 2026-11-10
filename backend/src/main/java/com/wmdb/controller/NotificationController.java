package com.wmdb.controller;

import com.wmdb.common.Result;
import com.wmdb.model.NotificationConfigDTO;
import com.wmdb.service.NotificationConfigService;
import io.swagger.v3.oas.annotations.Operation;
import io.swagger.v3.oas.annotations.tags.Tag;
import lombok.RequiredArgsConstructor;
import org.springframework.web.bind.annotation.*;

/**
 * 消息通知与告警通道配置控制器
 *
 * @author wm
 */
@Tag(name = "消息通知与告警配置")
@RestController
@RequestMapping("/api/v1/notification")
@RequiredArgsConstructor
public class NotificationController {

    private final NotificationConfigService notificationConfigService;

    @Operation(summary = "获取当前消息通知通道与告警配置")
    @GetMapping("/config")
    public Result<NotificationConfigDTO> getConfig() {
        return Result.success(notificationConfigService.getConfig());
    }

    @Operation(summary = "保存更新消息通知通道与告警配置 (管理员/DBA权限)")
    @PostMapping("/config")
    public Result<NotificationConfigDTO> saveConfig(@RequestBody NotificationConfigDTO config) {
        notificationConfigService.saveConfig(config);
        return Result.success(notificationConfigService.getConfig());
    }

    @Operation(summary = "在线测试指定通道 (企业微信/钉钉/电话语音外呼)")
    @PostMapping("/test-channel")
    public Result<NotificationConfigDTO.TestChannelResponse> testChannel(@RequestBody NotificationConfigDTO.TestChannelRequest request) {
        return Result.success(notificationConfigService.testChannel(request));
    }
}
