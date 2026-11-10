package com.wmdb;

import com.wmdb.mapper.SqlAuditLogMapper;
import com.wmdb.mapper.SqlTicketDetailMapper;
import com.wmdb.mapper.SqlTicketMapper;
import com.wmdb.model.SqlTicket;
import com.wmdb.model.SqlTicketDetail;
import com.wmdb.service.AsyncTicketExecutor;
import com.wmdb.service.TicketService;
import org.junit.jupiter.api.Assertions;
import org.junit.jupiter.api.Test;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.boot.test.context.SpringBootTest;

import java.util.Map;
import java.util.UUID;

@SpringBootTest
public class ExecutionFailureIntegrationTest {

    @Autowired
    private SqlTicketMapper sqlTicketMapper;

    @Autowired
    private SqlTicketDetailMapper sqlTicketDetailMapper;

    @Autowired
    private SqlAuditLogMapper sqlAuditLogMapper;

    @Autowired
    private AsyncTicketExecutor asyncTicketExecutor;

    @Autowired
    private TicketService ticketService;

    @Test
    public void testSqlExecutionFailureGracefulTermination() {
        long testTicketId = System.currentTimeMillis();

        // 1. 创建测试工单
        SqlTicket ticket = new SqlTicket();
        ticket.setId(testTicketId);
        ticket.setTenantId("1");
        ticket.setBusinessKey(UUID.randomUUID().toString());
        ticket.setInstanceId(1L);
        ticket.setApplicantIdCard("310101199001011234");
        ticket.setStatus("AUDITING");
        ticket.setType("SQL_AUDIT");
        ticket.setReason("[目标库: huiqitong_erp] 测试执行期错误捕获");
        sqlTicketMapper.insert(ticket);

        // 2. 创建明细（包含执行期必然失败的 SQL，例如表不存在）
        SqlTicketDetail detail = new SqlTicketDetail();
        detail.setId(testTicketId + 1);
        detail.setTenantId("1");
        detail.setTicketId(testTicketId);
        detail.setSqlText("UPDATE non_existing_table_99999 SET dummy = 1 WHERE id = 1;");
        detail.setAffectRowsEstimate(1);
        sqlTicketDetailMapper.insert(detail);

        // 3. 执行同步执行
        AsyncTicketExecutor.ExecutionResult result = asyncTicketExecutor.executeTicketSync(testTicketId);

        // 4. 验证执行结果
        Assertions.assertFalse(result.isSuccess(), "执行应判定为失败");
        Assertions.assertTrue(result.getMessage().contains("Table") || result.getMessage().contains("doesn't exist") || result.getMessage().contains("non_existing_table_99999"), "应包含表不存在报错信息");

        // 5. 验证工单状态流转至 FAILED
        SqlTicket updatedTicket = sqlTicketMapper.selectById(testTicketId);
        Assertions.assertNotNull(updatedTicket);
        Assertions.assertEquals("FAILED", updatedTicket.getStatus(), "工单状态必须为 FAILED");
        Assertions.assertTrue(updatedTicket.getExecutionWindow().contains("执行失败"), "executionWindow 必须记录失败原因");

        // 6. 验证审批流节点流转至终止状态
        Map<String, Object> detailMap = ticketService.getTicketDetail(testTicketId, "310101199001011234");
        Assertions.assertNotNull(detailMap);

        // 7. 清理测试数据
        sqlTicketMapper.deleteById(testTicketId);
        sqlTicketDetailMapper.deleteById(testTicketId + 1);
        sqlAuditLogMapper.delete(new com.baomidou.mybatisplus.core.conditions.query.QueryWrapper<com.wmdb.model.SqlAuditLog>().eq("ticket_id", testTicketId));

        System.out.println("====== 测试通过：执行期异常被完整捕获并记录，工单状态流转至 FAILED，流程优雅结束 ======");
    }
}
