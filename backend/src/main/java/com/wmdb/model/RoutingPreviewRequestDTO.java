package com.wmdb.model;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

/**
 * 审批流预估路由请求参数
 *
 * @author wm
 */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class RoutingPreviewRequestDTO {
    /**
     * 目标数据库实例 ID
     */
    private Long instanceId;

    /**
     * 目标数据库名 (Schema)
     */
    private String dbName;

    /**
     * 所属业务资源组
     */
    private String resourceGroup;

    /**
     * 工单变更类型 (SQL_AUDIT, DML_CHANGE, DDL_CHANGE, DATA_EXPORT, DATA_RECOVERY)
     */
    private String ticketType;

    /**
     * 预估或预执行的影响行数
     */
    private Integer expectedRows;

    /**
     * SQL 语句片段 (用于辅助分析 DDL/DML)
     */
    private String sqlSnippet;
}
