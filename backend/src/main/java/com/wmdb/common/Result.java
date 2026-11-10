package com.wmdb.common;

import lombok.Data;

/**
 * 统一 API 响应结果包装类
 *
 * @param <T> 数据类型
 * @author wm
 */
@Data
public class Result<T> {
    private String code;
    private String message;
    private T data;

    public static <T> Result<T> success(T data) {
        Result<T> result = new Result<>();
        result.setCode("00000"); // 阿里规范：00000 表示一切 ok
        result.setMessage("操作成功");
        result.setData(data);
        return result;
    }

    public static <T> Result<T> success() {
        return success(null);
    }

    public static <T> Result<T> error(String code, String message) {
        Result<T> result = new Result<>();
        result.setCode(code);
        result.setMessage(message);
        return result;
    }
}
