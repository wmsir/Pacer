package com.wmdb.service;

import lombok.extern.slf4j.Slf4j;
import org.springframework.stereotype.Component;

import java.util.regex.Matcher;
import java.util.regex.Pattern;

/**
 * 数据脱敏算法核心引擎
 * <p>
 * 提供手机号、身份证、姓名、邮箱、银行卡、住址、密码全掩码及自定义正则等高性能脱敏算法。
 * </p>
 *
 * @author wm
 */
@Slf4j
@Component
public class MaskingEngine {

    /**
     * 根据脱敏规则对原始数据进行脱敏处理
     *
     * @param rawVal            原始数据
     * @param ruleType          脱敏规则类型
     * @param customRegex       自定义正则表达式 (可选)
     * @param customReplacement 自定义替换表达式 (可选)
     * @return 脱敏后的安全字符串
     */
    public String mask(String rawVal, String ruleType, String customRegex, String customReplacement) {
        if (rawVal == null || rawVal.isEmpty() || "NULL".equalsIgnoreCase(rawVal)) {
            return rawVal;
        }

        if (ruleType == null || ruleType.trim().isEmpty() || "NONE".equalsIgnoreCase(ruleType)) {
            return rawVal;
        }

        try {
            switch (ruleType.toUpperCase()) {
                case "PHONE":
                    return maskPhone(rawVal);
                case "ID_CARD":
                    return maskIdCard(rawVal);
                case "NAME":
                    return maskName(rawVal);
                case "EMAIL":
                    return maskEmail(rawVal);
                case "BANK_CARD":
                    return maskBankCard(rawVal);
                case "ADDRESS":
                    return maskAddress(rawVal);
                case "PASSWORD":
                case "FULL":
                    return maskFull(rawVal);
                case "HASH_SHA256":
                case "HASH":
                    return maskHash(rawVal);
                case "CUSTOM_REGEX":
                    return maskCustomRegex(rawVal, customRegex, customReplacement);
                default:
                    return rawVal;
            }
        } catch (Exception e) {
            log.warn("脱敏处理异常, ruleType={}, val={}, err={}", ruleType, rawVal, e.getMessage());
            return rawVal;
        }
    }

    /**
     * 哈希杂凑脱敏 (SHA-256 单向不可逆摘要)
     */
    public String maskHash(String val) {
        if (val == null || val.isEmpty()) return val;
        try {
            java.security.MessageDigest md = java.security.MessageDigest.getInstance("SHA-256");
            byte[] hash = md.digest(val.getBytes(java.nio.charset.StandardCharsets.UTF_8));
            StringBuilder hexString = new StringBuilder();
            for (byte b : hash) {
                String hex = Integer.toHexString(0xff & b);
                if (hex.length() == 1) hexString.append('0');
                hexString.append(hex);
            }
            return hexString.substring(0, 16) + "...[HASH]";
        } catch (Exception e) {
            return "HASH_" + val.hashCode();
        }
    }

    /**
     * 手机号码脱敏 (保留前3后4，中间4位掩码)
     * 13800000001 -> 138****0001
     */
    public String maskPhone(String phone) {
        if (phone == null || phone.length() < 7) {
            return maskPartial(phone, 1, 1);
        }
        if (phone.length() == 11) {
            return phone.substring(0, 3) + "****" + phone.substring(7);
        }
        int keepPrefix = Math.min(3, phone.length() / 3);
        int keepSuffix = Math.min(4, phone.length() / 3);
        int maskLen = phone.length() - keepPrefix - keepSuffix;
        return phone.substring(0, keepPrefix) + "*".repeat(Math.max(1, maskLen)) + phone.substring(phone.length() - keepSuffix);
    }

    /**
     * 身份证号码脱敏 (保留前4后4，中间10位掩码)
     * 310101199001011234 -> 3101**********1234
     */
    public String maskIdCard(String idCard) {
        if (idCard == null || idCard.length() < 8) {
            return maskPartial(idCard, 2, 2);
        }
        if (idCard.length() == 18) {
            return idCard.substring(0, 4) + "**********" + idCard.substring(14);
        }
        if (idCard.length() == 15) {
            return idCard.substring(0, 3) + "*********" + idCard.substring(12);
        }
        int keep = Math.min(4, idCard.length() / 4);
        int maskLen = idCard.length() - (keep * 2);
        return idCard.substring(0, keep) + "*".repeat(Math.max(1, maskLen)) + idCard.substring(idCard.length() - keep);
    }

    /**
     * 姓名脱敏
     * 2字: 张三 -> 张*
     * 3字: 张小三 -> 张*三
     * 4字及以上: 诸葛孔明 -> 诸**明
     */
    public String maskName(String name) {
        if (name == null || name.isEmpty()) return name;
        int len = name.length();
        if (len == 1) return "*";
        if (len == 2) return name.charAt(0) + "*";
        if (len == 3) return name.charAt(0) + "*" + name.charAt(2);
        return name.charAt(0) + "*".repeat(len - 2) + name.charAt(len - 1);
    }

    /**
     * 电子邮箱脱敏
     * zhangsan@qq.com -> z****n@qq.com
     */
    public String maskEmail(String email) {
        if (email == null || !email.contains("@")) {
            return maskPartial(email, 1, 1);
        }
        int atIdx = email.indexOf('@');
        String name = email.substring(0, atIdx);
        String domain = email.substring(atIdx);

        if (name.length() <= 2) {
            return name.charAt(0) + "****" + domain;
        } else {
            return name.charAt(0) + "****" + name.charAt(name.length() - 1) + domain;
        }
    }

    /**
     * 银行卡号脱敏 (保留前6后4，中间掩码)
     * 6222021234567890 -> 622202******7890
     */
    public String maskBankCard(String card) {
        if (card == null || card.length() < 10) {
            return maskPartial(card, 4, 4);
        }
        int keepPrefix = 6;
        int keepSuffix = 4;
        if (card.length() <= 10) {
            keepPrefix = 2;
            keepSuffix = 2;
        }
        int maskLen = card.length() - keepPrefix - keepSuffix;
        return card.substring(0, keepPrefix) + "*".repeat(Math.max(1, maskLen)) + card.substring(card.length() - keepSuffix);
    }

    /**
     * 地址脱敏 (隐藏后半段门牌号)
     * 北京市海淀区中关村南大街1号院 -> 北京市海淀区中关村南大街****
     */
    public String maskAddress(String address) {
        if (address == null || address.length() <= 6) {
            return address + "****";
        }
        int keepLen = (int) Math.ceil(address.length() * 0.6);
        return address.substring(0, keepLen) + "****";
    }

    /**
     * 密码或完全脱敏
     */
    public String maskFull(String val) {
        return "******";
    }

    /**
     * 自定义正则替换脱敏
     */
    public String maskCustomRegex(String val, String regex, String replacement) {
        if (regex == null || regex.isEmpty()) {
            return val;
        }
        String rep = (replacement != null) ? replacement : "****";
        try {
            return val.replaceAll(regex, rep);
        } catch (Exception e) {
            log.warn("Regex replacement failed: regex={}, rep={}, err={}", regex, rep, e.getMessage());
            return val;
        }
    }

    private String maskPartial(String str, int prefix, int suffix) {
        if (str == null) return "";
        int len = str.length();
        if (len <= prefix + suffix) {
            return "*".repeat(len);
        }
        int maskLen = len - prefix - suffix;
        return str.substring(0, prefix) + "*".repeat(maskLen) + str.substring(len - suffix);
    }
}
