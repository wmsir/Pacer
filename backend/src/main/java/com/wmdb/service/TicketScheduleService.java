package com.wmdb.service;

import com.baomidou.mybatisplus.core.conditions.query.QueryWrapper;
import com.wmdb.mapper.SqlTicketMapper;
import com.wmdb.model.SqlTicket;
import lombok.extern.slf4j.Slf4j;
import org.springframework.scheduling.annotation.Scheduled;
import org.springframework.stereotype.Service;

import java.time.LocalDateTime;
import java.time.format.DateTimeFormatter;
import java.util.List;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/**
 * 工单自动化定时调度与窗口执行服务
 * <p>
 * 定时轮询 WAITING_EXECUTION 状态的工单，比对设定维护窗口时间，到期自动调用执行引擎流式执行并推送完成通知。
 * </p>
 *
 * @author wm
 */
@Slf4j
@Service
public class TicketScheduleService {

    private final SqlTicketMapper sqlTicketMapper;
    private final AsyncTicketExecutor asyncTicketExecutor;

    private static final Pattern TIME_PATTERN = Pattern.compile("(\\d{4}-\\d{2}-\\d{2}\\s+\\d{2}:\\d{2}(?::\\d{2})?)");
    private static final DateTimeFormatter FMT_SEC = DateTimeFormatter.ofPattern("yyyy-MM-dd HH:mm:ss");
    private static final DateTimeFormatter FMT_MIN = DateTimeFormatter.ofPattern("yyyy-MM-dd HH:mm");

    public TicketScheduleService(SqlTicketMapper sqlTicketMapper, AsyncTicketExecutor asyncTicketExecutor) {
        this.sqlTicketMapper = sqlTicketMapper;
        this.asyncTicketExecutor = asyncTicketExecutor;
    }

    /**
     * 每 15 秒轮询一次待定时执行队列
     */
    @Scheduled(fixedDelay = 15000)
    public void scanAndExecuteScheduledTickets() {
        try {
            List<SqlTicket> waitingTickets = sqlTicketMapper.selectList(
                    new QueryWrapper<SqlTicket>()
                            .eq("status", "WAITING_EXECUTION")
                            .orderByAsc("id")
            );

            if (waitingTickets == null || waitingTickets.isEmpty()) {
                return;
            }

            LocalDateTime now = LocalDateTime.now();

            for (SqlTicket ticket : waitingTickets) {
                if (isTimeDue(ticket.getExecutionWindow(), now)) {
                    log.info("⏰ Scheduled ticket #{} execution window arrived: [{}]. Triggering execution now...",
                            ticket.getId(), ticket.getExecutionWindow());
                    try {
                        asyncTicketExecutor.executeTicketSync(ticket.getId());
                    } catch (Exception e) {
                        log.error("Failed to execute scheduled ticket #{}: {}", ticket.getId(), e.getMessage(), e);
                    }
                }
            }
        } catch (Exception e) {
            log.warn("Scheduled ticket scanner encountered an issue: {}", e.getMessage());
        }
    }

    private boolean isTimeDue(String executionWindow, LocalDateTime now) {
        if (executionWindow == null || executionWindow.trim().isEmpty()) {
            return false;
        }
        Matcher matcher = TIME_PATTERN.matcher(executionWindow);
        if (matcher.find()) {
            String timeStr = matcher.group(1).trim();
            try {
                LocalDateTime targetTime;
                if (timeStr.length() == 16) {
                    targetTime = LocalDateTime.parse(timeStr, FMT_MIN);
                } else {
                    targetTime = LocalDateTime.parse(timeStr, FMT_SEC);
                }
                return !now.isBefore(targetTime);
            } catch (Exception ignored) {
            }
        }
        // 若没有包含具体时间，或格式为“计划维护窗口”，默认保留由人工触发或定时触发
        return false;
    }
}
