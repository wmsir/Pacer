package com.wmdb.service;

import org.springframework.stereotype.Service;
import java.util.regex.Pattern;

/**
 * 动态数据脱敏服务 (Data Masking)
 * <p>
 * 对 DQL 查询结果集进行正则表达式匹配，识别并脱敏手机号、身份证、邮箱等敏感信息。
 * </p>
 *
 * @author wm
 */
@Service
public class DataMaskingService {

    // 简单手机号匹配
    private static final Pattern PHONE_PATTERN = Pattern.compile("1[3-9]\\d{9}");
    // 简单身份证号匹配 (15/18位)
    private static final Pattern IDCARD_PATTERN = Pattern.compile("(\\d{14}[0-9xX])|(\\d{17}[0-9xX])");
    // 简单邮箱匹配
    private static final Pattern EMAIL_PATTERN = Pattern.compile("[a-zA-Z0-9_-]+@[a-zA-Z0-9_-]+(\\.[a-zA-Z0-9_-]+)+");

    /**
     * 对字符串内容进行通用脱敏
     *
     * @param content 原始字符串（如查询结果集中的某个字段）
     * @return 脱敏后的字符串
     */
    public String mask(String content) {
        if (content == null || content.isEmpty()) {
            return content;
        }

        String masked = content;
        // 脱敏手机号 (保留前3后4)
        masked = PHONE_PATTERN.matcher(masked).replaceAll(m -> {
            String s = m.group();
            return s.substring(0, 3) + "****" + s.substring(7);
        });

        // 脱敏身份证号 (保留前6后4)
        masked = IDCARD_PATTERN.matcher(masked).replaceAll(m -> {
            String s = m.group();
            if (s.length() == 15) {
                return s.substring(0, 6) + "*****" + s.substring(11);
            } else {
                return s.substring(0, 6) + "********" + s.substring(14);
            }
        });

        // 脱敏邮箱 (保留首字母和域名)
        masked = EMAIL_PATTERN.matcher(masked).replaceAll(m -> {
            String s = m.group();
            int atIndex = s.indexOf("@");
            if (atIndex > 1) {
                return s.substring(0, 1) + "***" + s.substring(atIndex);
            }
            return s;
        });

        return masked;
    }
}
