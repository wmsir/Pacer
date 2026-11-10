package com.wmdb.model;

import com.baomidou.mybatisplus.annotation.IdType;
import com.baomidou.mybatisplus.annotation.TableId;
import com.baomidou.mybatisplus.annotation.TableName;
import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

/**
 * 工单操作日志实体类
 */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
@TableName("ticket_operation_log")
public class TicketOperationLog {

    @TableId(type = IdType.AUTO)
    private Long id;

    private Long ticketId;

    private String operatorIdCard;

    private String operatorName;

    /**
     * SUBMIT/APPROVE/REJECT/EXECUTE/EXECUTE_SUCCESS/EXECUTE_FAIL/DBA_FEEDBACK/REVOKE/SCHEDULED/CANARY_BATCH
     */
    private String operationType;

    private String nodeName;

    private String comment;

    private String tenantId;

    private String createdTime;
}
