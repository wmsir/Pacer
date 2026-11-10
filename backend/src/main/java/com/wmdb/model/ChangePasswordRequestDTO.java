package com.wmdb.model;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

/**
 * 个人修改登录密码请求 DTO
 *
 * @author wm
 */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class ChangePasswordRequestDTO {

    /**
     * 原密码（明文或 SM2 密文）
     */
    private String oldPassword;

    /**
     * 新密码（明文或 SM2 密文）
     */
    private String newPassword;

    /**
     * 确认新密码
     */
    private String confirmPassword;
}
