package com.wmdb.exception;

/**
 * 业务异常类
 * <p>
 * 用于抛出业务逻辑相关的异常，以便于在全局异常处理器中统一拦截并返回对应的错误码。
 * </p>
 *
 * @author wm
 */
public class BusinessException extends RuntimeException {

    private String code;

    public BusinessException(String message) {
        super(message);
        this.code = "A0400"; // 默认请求参数错误
    }

    public BusinessException(String code, String message) {
        super(message);
        this.code = code;
    }

    public String getCode() {
        return code;
    }
}
