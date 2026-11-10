package com.wmdb.model;

import com.baomidou.mybatisplus.annotation.IdType;
import com.baomidou.mybatisplus.annotation.TableId;
import com.baomidou.mybatisplus.annotation.TableName;
import lombok.Data;

/**
 * SQL 执行防抵赖日志实体类
 * <p>
 * 映射 sql_audit_log 表，记录执行详情和耗时，采用哈希链机制防止日志篡改。
 * </p>
 *
 * @author wm
 * @date 2023-10-25
 */
@Data
@TableName("sql_audit_log")
public class SqlAuditLog {

    private String tenantId;

    /**
     * 主键 ID
     */
    @TableId(type = IdType.AUTO)
    private Long id;

    /**
     * 关联的工单 ID
     */
    private Long ticketId;

    /**
     * 实际执行的 SQL 语句
     */
    private String executeSql;

    /**
     * 执行耗时（毫秒）
     */
    private Long costTimeMs;

    /**
     * 执行状态（SUCCESS, FAILED）
     */
    private String status;

    /**
     * 错误堆栈信息
     */
    private String errorTrace;

    /**
     * 上一条日志的 Hash 值（防篡改设计）
     */
    private String previousHash;

    /**
     * 当前日志的 Hash 值
     */
    private String currentHash;

    /**
     * 影子重放（Flashback）生成的回滚 SQL 语句
     */
    private String rollbackSql;
}
