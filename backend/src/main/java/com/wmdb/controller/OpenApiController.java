package com.wmdb.controller;

import com.wmdb.common.Result;
import com.wmdb.model.SqlTicket;
import com.wmdb.service.TicketService;
import org.springframework.web.bind.annotation.*;

import java.util.Map;

/**
 * OpenAPI 控制器
 * <p>
 * 提供 RESTful API 供企业外部系统（如 DevOps, CI/CD 平台）集成。
 * </p>
 *
 * @author wm
 * @date 2023-10-25
 */
@RestController
@RequestMapping("/api/v1/openapi")
public class OpenApiController {

    private final TicketService ticketService;

    public OpenApiController(TicketService ticketService) {
        this.ticketService = ticketService;
    }

    /**
     * 获取指定工单的状态
     *
     * @param ticketId 工单 ID
     * @param applicantId 申请人身份证（鉴权校验依据）
     * @return 工单状态和概要信息
     */
    @com.wmdb.common.annotation.RateLimit(time = 60, count = 100)
    @GetMapping("/ticket/{id}/status")
    public Result<Map<String, Object>> getTicketStatus(@PathVariable("id") Long ticketId,
                                                       @RequestParam("applicantId") String applicantId) {
        Map<String, Object> detailMap = ticketService.getTicketDetail(ticketId, applicantId);
        if (detailMap == null || detailMap.get("ticket") == null) {
            return Result.error("B0500", "Ticket not found or access denied");
        }

        SqlTicket ticket = (SqlTicket) detailMap.get("ticket");

        return Result.success(Map.of(
            "ticketId", ticket.getId(),
            "status", ticket.getStatus(),
            "instanceId", ticket.getInstanceId()
        ));
    }
}
