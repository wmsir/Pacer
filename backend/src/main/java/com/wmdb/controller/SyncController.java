package com.wmdb.controller;

import com.wmdb.common.Result;
import com.wmdb.service.ExternalSyncService;
import lombok.Data;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RestController;

/**
 * 组织架构同步控制器
 * <p>
 * 提供标准 Webhook 接口接收外部现成的 OA/HR 系统用户信息推送，
 * 完成权限过滤、装配并入库，实现企业级系统对接。
 * </p>
 *
 * @author wm
 */
@RestController
@RequestMapping("/api/v1/sync")
public class SyncController {

    private final ExternalSyncService externalSyncService;

    public SyncController(ExternalSyncService externalSyncService) {
        this.externalSyncService = externalSyncService;
    }

    /**
     * 接收外部用户推送 Webhook
     * <p>
     * 生产环境需结合 IP 白名单或 HMAC 校验确保该接口安全。
     * </p>
     *
     * @param request 外部推送的用户信息数据包
     * @return 同步结果
     */
    @PostMapping("/users")
    public Result<String> syncUsers(@RequestBody UserSyncRequest request) {
        externalSyncService.syncUser(
            request.getRealName(),
            request.getIdCard(),
            request.getDepartment(),
            request.getTenantId()
        );
        return Result.success("User synchronized successfully");
    }

    /**
     * 同步请求实体
     */
    @Data
    public static class UserSyncRequest {
        private String realName;
        private String idCard;
        private String department;
        private String tenantId;
    }
}
