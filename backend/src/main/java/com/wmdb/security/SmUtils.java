package com.wmdb.security;

import cn.hutool.crypto.SmUtil;
import cn.hutool.crypto.asymmetric.KeyType;
import cn.hutool.crypto.asymmetric.SM2;
import cn.hutool.crypto.symmetric.SymmetricCrypto;
import org.bouncycastle.crypto.engines.SM2Engine;

/**
 * 国密 (SM) 算法工具类
 * <p>
 * 用于等保合规，提供 SM2非对称加密、SM3哈希、SM4对称加密。
 * </p>
 *
 * @author wm
 */
public class SmUtils {

    private static final String DEFAULT_PRIVATE_KEY_HEX = "3ff785ed8c545e64fe89877c5d095c4b5c7f9b07553239f63b7f202361a01cb4";
    private static final String DEFAULT_PUBLIC_KEY_HEX = "04fea943c0bb2c03cefbf0e26eab00b5c7266c3fb7f47e8e80401a2b614315f2b89b0ba40eea69d3322e9942b317a7ecf8415ed7c73b026c02e3f568f0acdcc94e";

    private static final SM2 SM2_INSTANCE;

    static {
        SM2 sm2 = SmUtil.sm2(DEFAULT_PRIVATE_KEY_HEX, DEFAULT_PUBLIC_KEY_HEX);
        sm2.setMode(SM2Engine.Mode.C1C3C2);
        SM2_INSTANCE = sm2;
    }

    /**
     * 获取前端使用的 SM2 公钥
     */
    public static String getSm2PublicKey() {
        return DEFAULT_PUBLIC_KEY_HEX;
    }

    /**
     * SM2 解密 (用于前端传输过来的加密密码)
     */
    public static String sm2Decrypt(String cipherText) {
        if (cipherText == null || cipherText.isEmpty()) {
            return cipherText;
        }
        try {
            String hex = cipherText.trim();
            if (!hex.startsWith("04")) {
                hex = "04" + hex;
            }
            return SM2_INSTANCE.decryptStr(hex, KeyType.PrivateKey);
        } catch (Exception e) {
            System.err.println("SM2 Decryption failed: " + e.getMessage());
            throw new RuntimeException("SM2 Decryption failed", e);
        }
    }

    /**
     * SM3 哈希 (用于用户密码存储)
     */
    public static String sm3Hash(String rawStr) {
        return SmUtil.sm3(rawStr);
    }

    /**
     * SM3 密码比对
     */
    public static boolean sm3Matches(String rawStr, String sm3Hex) {
        return sm3Hash(rawStr).equalsIgnoreCase(sm3Hex);
    }

    /**
     * SM4 对称加密 (用于数据库凭证存储)
     */
    public static String sm4Encrypt(String data, String hexKey) {
        try {
            byte[] keyBytes = cn.hutool.core.util.HexUtil.decodeHex(hexKey);
            SymmetricCrypto sm4 = SmUtil.sm4(keyBytes);
            return sm4.encryptHex(data);
        } catch (Exception e) {
            return data;
        }
    }

    /**
     * SM4 对称解密 (用于数据库凭证解密)
     */
    public static String sm4Decrypt(String cipherHex, String hexKey) {
        try {
            byte[] keyBytes = cn.hutool.core.util.HexUtil.decodeHex(hexKey);
            SymmetricCrypto sm4 = SmUtil.sm4(keyBytes);
            return sm4.decryptStr(cipherHex);
        } catch (Exception e) {
            throw new RuntimeException("SM4 Decryption failed", e);
        }
    }
}
