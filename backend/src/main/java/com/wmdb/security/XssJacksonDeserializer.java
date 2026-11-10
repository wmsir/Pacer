package com.wmdb.security;

import com.fasterxml.jackson.core.JsonParser;
import com.fasterxml.jackson.databind.DeserializationContext;
import com.fasterxml.jackson.databind.JsonDeserializer;
import java.io.IOException;
import java.util.regex.Pattern;

/**
 * Jackson XSS 反序列化器
 * <p>
 * 在 JSON 反序列化期间，对所有的 String 属性执行危险标签过滤，防止通过 RequestBody 注入 XSS。
 * 注意：不直接使用 htmlEscape，以避免破坏正常的数据（如单引号、双引号等在正常业务数据中出现）。
 * </p>
 *
 * @author wm
 */
public class XssJacksonDeserializer extends JsonDeserializer<String> {

    @Override
    public String deserialize(JsonParser p, DeserializationContext ctxt) throws IOException {
        String value = p.getValueAsString();
        if (value != null) {
            return stripXSS(value);
        }
        return null;
    }

    private String stripXSS(String value) {
        if (value != null) {
            // 作为一个 SQL 治理平台，由于输入中可能包含大量合法的 `<`、`>` 符号或类似 HTML 标签的字符内容，
            // 传统的 Regex XSS 过滤（尤其是全局的 Jackson 过滤）会造成极为严重的数据破坏和误伤。
            // 因此，后端反序列化时应尽可能保留原始输入以供 AST 解析，
            // 防护 XSS 的正确且安全的姿势是：
            // 1. 拦截层对非法空字符进行剥离。
            // 2. 前端展示数据时，使用 DOMPurify 或 Vue 的内置 HTML 实体编码进行安全渲染。

            value = value.replaceAll("\0", "");

            // 为了兼顾防御最基本的脚本注入而尽量不误伤 SQL，
            // 这里仅拦截特定的已知高度恶意的非 SQL 关键词组合，而不是进行正则替换破坏 SQL。
            String upperValue = value.toUpperCase();
            if (upperValue.contains("<SCRIPT>") || upperValue.contains("JAVASCRIPT:")) {
                throw new IllegalArgumentException("Potential XSS payload detected in request body.");
            }
        }
        return value;
    }
}
