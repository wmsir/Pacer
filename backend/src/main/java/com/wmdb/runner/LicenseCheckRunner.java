package com.wmdb.runner;

import com.wmdb.service.LicenseService;
import lombok.extern.slf4j.Slf4j;
import org.springframework.boot.CommandLineRunner;
import org.springframework.stereotype.Component;

/**
 * License 检查启动器
 * <p>
 * 在应用启动时校验商业授权，如果不通过则中断启动。
 * </p>
 *
 * @author wm
 */
@Slf4j
@Component
public class LicenseCheckRunner implements CommandLineRunner {

    private final LicenseService licenseService;

    public LicenseCheckRunner(LicenseService licenseService) {
        this.licenseService = licenseService;
    }

    @Override
    public void run(String... args) throws Exception {
        log.info("====== 正在校验系统 License ======");
        try {
            licenseService.checkLicenseStatus();
            log.info("====== License 校验通过 ======");
        } catch (Exception e) {
            log.error("====== License 校验失败，系统即将退出 ======", e);
            System.exit(1);
        }
    }
}
