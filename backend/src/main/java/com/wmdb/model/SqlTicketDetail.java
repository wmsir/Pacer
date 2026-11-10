package com.wmdb.model;

import com.baomidou.mybatisplus.annotation.IdType;
import com.baomidou.mybatisplus.annotation.TableId;
import com.baomidou.mybatisplus.annotation.TableName;
import com.fasterxml.jackson.databind.annotation.JsonSerialize;
import com.fasterxml.jackson.databind.ser.std.ToStringSerializer;
import lombok.Data;

/**
 * SQL 工单明细与附件表实体类
 * <p>
 * 映射 sql_ticket_detail 表，存储 SQL 文本内容或超大文件摘要及附件 OSS 路径。
 * </p>
 *
 * @author wm
 * @date 2023-10-25
 */
@Data
@TableName("sql_ticket_detail")
public class SqlTicketDetail {

    private String tenantId;

    /**
     * 主键 ID
     */
    @TableId(type = IdType.INPUT)
    @JsonSerialize(using = ToStringSerializer.class)
    private Long id;

    /**
     * 关联的工单主表 ID
     */
    @JsonSerialize(using = ToStringSerializer.class)
    private Long ticketId;

    /**
     * SQL 文本（长度在阈值内全量存储，超出阈值存摘要）
     */
    private String sqlText;

    /**
     * 超大文件在 MinIO/OSS 中的 Key
     */
    private String attachmentOssKey;

    /**
     * 预计影响行数（通过 AST 分析得出）
     */
    private Integer affectRowsEstimate;

    /**
     * 数据回滚 / 备份 SQL 脚本内容
     */
    private String rollbackSqlText;

    /**
     * 数据回滚 / 备份附件在 MinIO/OSS 中的 Key
     */
    private String rollbackOssKey;

    /**
     * 业务资源组自定义工单扩展字段值 (JSON 字符串，如上线版本号、年月日、执行窗口等)
     */
    private String customFieldValues;
}
