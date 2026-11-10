package com.wmdb.model;

import com.baomidou.mybatisplus.annotation.IdType;
import com.baomidou.mybatisplus.annotation.TableId;
import com.baomidou.mybatisplus.annotation.TableName;
import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

import java.util.Date;

/**
 * 数据脱敏规则实体类
 * <p>
 * 映射 data_masking_rule 表，存储字段级的动态脱敏规则配置。
 * </p>
 *
 * @author wm
 */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
@TableName("data_masking_rule")
public class DataMaskingRule {

    private String tenantId;

    /**
     * 规则 ID
     */
    @TableId(type = IdType.AUTO)
    private Long id;

    /**
     * 关联数据库实例 ID
     */
    private Long instanceId;

    /**
     * 数据库名称 (Schema)
     */
    private String dbName;

    /**
     * 数据表名称
     */
    private String tableName;

    /**
     * 字段名称
     */
    private String columnName;

    /**
     * 字段数据类型
     */
    private String columnType;

    /**
     * 字段注释
     */
    private String columnComment;

    /**
     * 脱敏规则算法类型：
     * PHONE (手机号), ID_CARD (身份证), NAME (姓名), EMAIL (邮箱),
     * BANK_CARD (银行卡), ADDRESS (地址), PASSWORD (密码全脱敏), CUSTOM_REGEX (自定义正则)
     */
    private String ruleType;

    /**
     * 自定义正则表达式（仅在 ruleType = CUSTOM_REGEX 时生效）
     */
    private String customRegex;

    /**
     * 自定义替换表达式（如 $1****$2）
     */
    private String customReplacement;

    /**
     * 规则状态：1-启用，0-禁用
     */
    private Integer status;

    /**
     * 描述说明
     */
    private String description;

    /**
     * 创建时间
     */
    private Date createTime;

    /**
     * 更新时间
     */
    private Date updateTime;
}
