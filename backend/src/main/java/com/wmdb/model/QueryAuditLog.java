package com.wmdb.model;

import com.baomidou.mybatisplus.annotation.IdType;
import com.baomidou.mybatisplus.annotation.TableId;
import com.baomidou.mybatisplus.annotation.TableName;
import lombok.Data;

import java.util.Date;

/**
 * 在线 SQL 查询与执行计划防抵赖审计日志实体
 *
 * @author wm
 */
@Data
@TableName("query_audit_log")
public class QueryAuditLog {

    @TableId(type = IdType.AUTO)
    private Long id;

    private String tenantId;

    /**
     * 目标实例 ID 与实例名称
     */
    private Long instanceId;
    private String instanceName;

    /**
     * 目标数据库名称 (Schema)
     */
    private String dbName;

    /**
     * 查询执行人信息
     */
    private Long userId;
    private String username;
    private String realName;

    /**
     * 操作类型 (SELECT, EXPLAIN, SHOW, DESC)
     */
    private String opType;

    /**
     * 查询 SQL 文本
     */
    private String sqlText;

    /**
     * 执行耗时（毫秒）
     */
    private Long costMs;

    /**
     * 返回记录行数
     */
    private Integer resultRows;

    /**
     * 执行状态 (SUCCESS, FAILED)
     */
    private String status;

    /**
     * 异常错误信息
     */
    private String errorMsg;

    /**
     * 客户端 IP
     */
    private String clientIp;

    /**
     * 执行时间
     */
    private Date createTime;
}
