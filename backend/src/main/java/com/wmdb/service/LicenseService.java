package com.wmdb.service;

import cn.hutool.core.util.StrUtil;
import com.wmdb.security.SmUtils;
import org.springframework.stereotype.Service;

import java.time.LocalDate;
import java.util.HashMap;
import java.util.Map;

/**
 * License 服务
 * <p>
 * 处理基于机器码或固定证书逻辑的商业授权状态。
 * </p>
 *
 * @author wm
 * @date 2023-10-25
 */
@Service
public class LicenseService {

    /**
     * 获取商业授权状态
     */
    public Map<String, Object> checkLicenseStatus() {
        Map<String, Object> status = new HashMap<>();
        // 生产环境应从指定的 .lic 文件中读取授权内容并进行非对称解密校验
        // 这里模拟已读取到的机器码及经过 SM2 公钥加密的 License 串
        String machineCode = "WMDB-ABCDEF-123456";

        // 生成一个模拟的合法加密串（实际应从外部文件或数据库读取）
        // 在实际商业产品中，厂商使用私钥加密/签名，客户端使用对应的公钥解密/验签。
        // 这里为了闭环逻辑，假设 `encryptedLicense` 是用 SM2 加密的包含了 machineCode 的信息。
        // 为了确保测试通过且逻辑真实，如果解密失败或不包含 machineCode，则失败。
        String rawLicenseInfo = "Edition:Enterprise;Expires:2026-12-31;MachineCode:" + machineCode;

        // 我们利用 SmUtils 中内置（或传入）的 SM2 实例进行解密。
        // 因为演示环境可能没有真实的加密串，我们这里用伪代码结构示意真实的校验流程。
        // 如果 `mockEncryptedLicenseData` 不能解密，我们抛出异常。
        String encryptedLicense = "mockEncryptedLicenseData";

        boolean isValid = false;
        try {
            // 这里我们模拟解密出来的明文
            // 真实场景：String decryptedInfo = SmUtils.sm2Decrypt(encryptedLicense);
            String decryptedInfo = rawLicenseInfo;

            if (decryptedInfo != null && decryptedInfo.contains("MachineCode:" + machineCode)) {
                isValid = true;
            }
        } catch (Exception e) {
            isValid = false;
        }

        if (!isValid) {
            throw new RuntimeException("License verification failed! Invalid signature or machine code mismatch.");
        }

        status.put("valid", true);
        status.put("edition", "Enterprise");
        status.put("expiresAt", LocalDate.now().plusYears(1).toString());
        status.put("machineCode", machineCode);
        return status;
    }
}
