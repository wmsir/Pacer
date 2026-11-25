package com.wmdb.exception;

import com.wmdb.common.Result;
import lombok.extern.slf4j.Slf4j;
import org.springframework.http.HttpStatus;
import org.springframework.http.ResponseEntity;
import org.springframework.web.bind.annotation.ExceptionHandler;
import org.springframework.web.bind.annotation.RestControllerAdvice;

/**
 * 全局异常处理类
 *
 * @author wm
 */
@Slf4j
@RestControllerAdvice
public class GlobalExceptionHandler {

    @ExceptionHandler(Exception.class)
    public ResponseEntity<Result<Void>> handleException(Exception e) {
        log.error("系统异常: ", e);
        // B0500: 系统执行出错
        return ResponseEntity.status(HttpStatus.INTERNAL_SERVER_ERROR)
                .body(Result.error("B0500", "系统内部异常，请联系管理员"));
    }

    @ExceptionHandler(BusinessException.class)
    public ResponseEntity<Result<Void>> handleBusinessException(BusinessException e) {
        String code = e.getCode() != null ? e.getCode() : "A0400";
        HttpStatus status = HttpStatus.BAD_REQUEST;
        if ("A0220".equals(code)) {
            status = HttpStatus.UNAUTHORIZED;
        } else if ("A0301".equals(code) || "A0403".equals(code)) {
            status = HttpStatus.FORBIDDEN;
        } else if ("A0404".equals(code)) {
            status = HttpStatus.NOT_FOUND;
        }
        log.warn("业务异常: code={}, message={}", code, e.getMessage());
        return ResponseEntity.status(status)
                .body(Result.error(code, e.getMessage()));
    }

    @ExceptionHandler(RuntimeException.class)
    public ResponseEntity<Result<Void>> handleRuntimeException(RuntimeException e) {
        String msg = e.getMessage() != null ? e.getMessage() : "请求处理失败";
        if ("未登录或登录凭据已过期".equals(msg)) {
            log.warn("认证拒止: {}", msg);
            return ResponseEntity.status(HttpStatus.UNAUTHORIZED)
                    .body(Result.error("A0220", msg));
        }
        if ("账号不存在或密码错误".equals(msg)) {
            log.warn("登录校验失败: {}", msg);
            return ResponseEntity.status(HttpStatus.BAD_REQUEST)
                    .body(Result.error("A0400", msg));
        }
        log.error("运行时异常: ", e);
        // A0400: 用户请求参数错误/校验未通过
        return ResponseEntity.status(HttpStatus.BAD_REQUEST)
                .body(Result.error("A0400", msg));
    }
}
