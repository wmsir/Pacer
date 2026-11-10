package com.wmdb.controller;

import com.wmdb.common.Result;
import com.wmdb.service.LicenseService;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RestController;

import java.util.Map;

/**
 * 商业授权控制器
 * <p>
 * 提供 License 状态检查接口。
 * </p>
 *
 * @author wm
 * @date 2023-10-25
 */
@RestController
@RequestMapping("/api/v1/license")
public class LicenseController {

    private final LicenseService licenseService;

    public LicenseController(LicenseService licenseService) {
        this.licenseService = licenseService;
    }

    /**
     * 校验 License
     *
     * @return 授权状态信息
     */
    @GetMapping("/check")
    public Result<Map<String, Object>> checkLicense() {
        return Result.success(licenseService.checkLicenseStatus());
    }
}
