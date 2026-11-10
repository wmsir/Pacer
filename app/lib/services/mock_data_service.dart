import '../models/ticket_model.dart';
import '../models/db_instance_model.dart';

class MockDataService {
  static List<SqlTicket> getMockTickets() {
    return [
      const SqlTicket(
        id: 1089,
        ticketNo: 'TK-20261016-001',
        title: '【高危紧急】双11核心订单分库扩容添加索引与归档冷数据',
        instanceName: 'TiDB-金融核心分布式集群-01',
        dbName: 'trade_order_cluster',
        changeType: 'DDL',
        status: TicketStatus.pending,
        riskLevel: RiskLevel.high,
        riskScore: 88,
        submitter: '订单架构组-张建峰',
        submitTime: '2026-10-16 14:22:15',
        scheduledTime: '2026-10-16 23:00:00',
        sqlPreview: 'ALTER TABLE `t_order_main` ADD INDEX `idx_create_status` (`created_at`, `order_status`);\n'
            'DELETE FROM `t_order_history_bak` WHERE `created_at` < "2025-01-01" LIMIT 50000;',
        expectedRows: 50000,
        actualRows: 0,
      ),
      const SqlTicket(
        id: 1088,
        ticketNo: 'TK-20261016-002',
        title: '连锁零售会员积分双倍活动配置批量调价更新',
        instanceName: 'MySQL8-华东生产集群 (主控写节点)',
        dbName: 'seahorse_member_db',
        changeType: 'DML',
        status: TicketStatus.pending,
        riskLevel: RiskLevel.medium,
        riskScore: 65,
        submitter: '营销系统-林青青',
        submitTime: '2026-10-16 14:10:02',
        sqlPreview: 'UPDATE `member_points_rule` SET `multiplier` = 2.0, `updated_time` = NOW() '
            'WHERE `channel` IN ("POS_STORE", "MINI_APP") AND `status` = 1;',
        expectedRows: 120,
        actualRows: 0,
      ),
      const SqlTicket(
        id: 1087,
        ticketNo: 'TK-20261016-003',
        title: 'WM-ERP 仓库物料 SKU 条码唯一性约束修复',
        instanceName: 'OceanBase-供应链仓储多租户集群',
        dbName: 'wm_inventory_master',
        changeType: 'DDL',
        status: TicketStatus.waitingExecution,
        riskLevel: RiskLevel.medium,
        riskScore: 52,
        submitter: '仓储架构-陈明',
        submitTime: '2026-10-16 11:35:40',
        scheduledTime: '2026-10-16 22:30:00',
        sqlPreview: 'ALTER TABLE `inv_sku_barcode` ADD UNIQUE KEY `uk_barcode_wh` (`barcode`, `warehouse_id`);',
        expectedRows: 0,
        actualRows: 0,
      ),
      const SqlTicket(
        id: 1086,
        ticketNo: 'TK-20261016-004',
        title: '财务月度核算汇总表字段扩容与历史对账单生成',
        instanceName: '达梦 DM8-信创合规生产库',
        dbName: 'finance_settlement_dm',
        changeType: 'DML',
        status: TicketStatus.running,
        riskLevel: RiskLevel.low,
        riskScore: 18,
        submitter: '财务中台-孙晓丽',
        submitTime: '2026-10-16 09:15:20',
        sqlPreview: 'INSERT INTO `fin_month_stat` (`month_str`, `total_amount`, `store_count`)\n'
            'SELECT "2026-09", SUM(`pay_amount`), COUNT(DISTINCT `store_id`) FROM `fin_settle_log`;',
        expectedRows: 1,
        actualRows: 1,
      ),
      const SqlTicket(
        id: 1085,
        ticketNo: 'TK-20261015-089',
        title: '用户行为分析日志临时表清理 (Pacer 灰度流式分批)',
        instanceName: 'MySQL8-华东生产集群 (主控写节点)',
        dbName: 'user_analytics_prod',
        changeType: 'DML',
        status: TicketStatus.executed,
        riskLevel: RiskLevel.low,
        riskScore: 12,
        submitter: '数据平台-赵磊',
        submitTime: '2026-10-15 20:00:10',
        sqlPreview: 'DELETE FROM `stat_page_views_temp` WHERE `log_date` < "2026-08-01";',
        expectedRows: 86500,
        actualRows: 86500,
      ),
      const SqlTicket(
        id: 1084,
        ticketNo: 'TK-20261015-056',
        title: '客户资料动态脱敏规则更新 (驳回：影响全量查询性能)',
        instanceName: 'openGauss-政企安全分布式专区',
        dbName: 'crm_customer_sec',
        changeType: 'DCL',
        status: TicketStatus.rejected,
        riskLevel: RiskLevel.high,
        riskScore: 92,
        submitter: '安全合规-钱伟',
        submitTime: '2026-10-15 16:40:00',
        sqlPreview: 'GRANT ALL PRIVILEGES ON `crm_customer_sec`.* TO "third_party_audit"@"%";',
        expectedRows: 0,
        actualRows: 0,
        rejectReason: '违反安全基线规定：禁止向第三方审计账号开放全局写权限与明文未脱敏字段！请收敛权限粒度至视图层。',
      ),
    ];
  }

  static SqlTicketDetail getMockDetail(SqlTicket ticket) {
    if (ticket.id == 1089) {
      return SqlTicketDetail(
        ticket: ticket,
        fullSql: '-- =========================================================================\n'
            '-- 稳骥 PacerSQL AST 智能语法预检推演报告 (Dry-Run)\n'
            '-- 工单流水号: TK-20261016-001 | 目标实例: TiDB-金融核心分布式集群-01\n'
            '-- =========================================================================\n\n'
            'USE `trade_order_cluster`;\n\n'
            '-- 1. 添加高频组合查询索引 (预估阻塞时长: 0 ms)\n'
            'ALTER TABLE `t_order_main` \n'
            '  ADD INDEX `idx_create_status` (`created_at`, `order_status`);\n\n'
            '-- 2. 归档历史冷数据 (启用 Pacer 微批步态控制，每次 500 行，休眠 25ms)\n'
            'DELETE FROM `t_order_history_bak` \n'
            '  WHERE `created_at` < "2025-01-01" \n'
            '  LIMIT 50000; -- expected_rows: 50000\n',
        rollbackSql: '-- 稳骥 PacerSQL 自动生成逆向回滚脚本\n'
            'USE `trade_order_cluster`;\n\n'
            '-- 回滚索引\n'
            'ALTER TABLE `t_order_main` DROP INDEX `idx_create_status`;\n\n'
            '-- 历史归档数据回滚（请从冷备存储桶 s3://pacer-cold-backup/20261016/ 执行还原）\n',
        astRiskRules: [
          '【高危阻断】DELETE 语句影响行数预估达到 50,000 行，属于大批量数据变更，强制启用 Pacer 灰度切片步态。',
          '【中危告警】t_order_main 表记录数超过 5,000 万行，添加复合索引可能引起从库复制延迟，建议在凌晨低峰维护窗口执行。',
          '【安全合规】目标库开启了 SpEL 动态分流，该写操作将收敛至集群主节点排他事务管道。',
        ],
        dbaSuggestions: [
          '建议将执行模式设置为 SCHEDULED (定时维护窗口)，推荐窗口：2026-10-16 23:30 ~ 04:00。',
          '步态设置建议：微批切片大小 2,000 行，微批休眠间隔 30ms，以保证生产在线高频读写零抖动。',
          '经 Dry-Run 预执行事务探测，预期影响行数 (50,000) 与实际事务推导 (50,000) 100% 吻合。',
        ],
        dryRunPassed: true,
        dryRunAffectedRows: 50000,
        timeline: [
          const ApprovalStep(
            nodeName: '工单创建与 AST 预检',
            operator: '订单架构组-张建峰',
            time: '2026-10-16 14:22:15',
            status: '通过',
            comment: '完成自动化 AST 语法树比对与影响行数 Dry-Run 预检。',
          ),
          const ApprovalStep(
            nodeName: '技术负责人审核',
            operator: '架构师-王工',
            time: '2026-10-16 14:35:10',
            status: '已同意',
            comment: '方案合理，同意在晚间低峰期执行。转 DBA 终审并配置步态。',
          ),
          const ApprovalStep(
            nodeName: '生产 DBA 终审决策',
            operator: '当前待办 (你)',
            time: '等待审批中',
            status: '待审批',
            comment: '请核实定时执行窗口与 Pacer 切片休眠参数。',
          ),
        ],
      );
    }

    // 通用默认详情
    return SqlTicketDetail(
      ticket: ticket,
      fullSql: ticket.sqlPreview,
      rollbackSql: '-- 自动生成回滚语句预置块\n-- 视业务主键反向补偿\n',
      astRiskRules: [
        '【AST 语义扫描】语法合规，未包含高危全表扫描或 DROP 语句。',
        '【执行行数校验】预期影响行数在安全阈值可控范围内。',
      ],
      dbaSuggestions: [
        '推荐直接使用 IMMEDIATE (立即执行) 模式。',
        '该操作预计执行耗时 < 150ms。',
      ],
      dryRunPassed: true,
      dryRunAffectedRows: ticket.expectedRows,
      timeline: [
        ApprovalStep(
          nodeName: '工单创建',
          operator: ticket.submitter,
          time: ticket.submitTime,
          status: '通过',
          comment: '提交变更申请',
        ),
        const ApprovalStep(
          nodeName: '生产主管审批',
          operator: '当前审批席 (你)',
          time: '待决策',
          status: '待审批',
        ),
      ],
    );
  }

  static List<DbInstance> getMockInstances() {
    return [
      const DbInstance(
        id: 1,
        instanceName: 'TiDB-金融核心分布式集群-01',
        dbType: 'TiDB 7.1',
        env: 'PROD',
        host: '10.0.12.88',
        port: 4000,
        qps: 18500,
        activeConnections: 420,
        maxConnections: 5000,
        replicationLagMs: 0,
        status: DbHealthStatus.healthy,
        clusterRole: '多节点分布式 Raft',
      ),
      const DbInstance(
        id: 2,
        instanceName: 'MySQL8-华东生产集群 (主控写节点)',
        dbType: 'MySQL 8.0.34',
        env: 'PROD',
        host: '101.35.100.169',
        port: 9306,
        qps: 4200,
        activeConnections: 185,
        maxConnections: 1200,
        replicationLagMs: 2,
        status: DbHealthStatus.healthy,
        clusterRole: 'SpEL 排他写主控',
      ),
      const DbInstance(
        id: 3,
        instanceName: 'MySQL8-华东生产集群 (只读从节点01)',
        dbType: 'MySQL 8.0.34',
        env: 'PROD',
        host: '39.97.158.22',
        port: 9306,
        qps: 12800,
        activeConnections: 680,
        maxConnections: 2000,
        replicationLagMs: 8,
        status: DbHealthStatus.healthy,
        clusterRole: '只读报表加速从库',
      ),
      const DbInstance(
        id: 4,
        instanceName: 'OceanBase-供应链仓储多租户集群',
        dbType: 'OceanBase 4.2',
        env: 'PROD',
        host: '10.0.25.102',
        port: 2881,
        qps: 9200,
        activeConnections: 310,
        maxConnections: 3000,
        replicationLagMs: 0,
        status: DbHealthStatus.healthy,
        clusterRole: '三副本 Paxos 共识',
      ),
      const DbInstance(
        id: 5,
        instanceName: '达梦 DM8-信创金融生产库',
        dbType: 'DM8 国产信创',
        env: 'PROD',
        host: '10.0.30.15',
        port: 5236,
        qps: 2100,
        activeConnections: 95,
        maxConnections: 800,
        replicationLagMs: 15,
        status: DbHealthStatus.healthy,
        clusterRole: '国密 SM4 专线主库',
      ),
      const DbInstance(
        id: 6,
        instanceName: 'openGauss-政企安全分布式专区',
        dbType: 'openGauss 5.0',
        env: 'STAGING',
        host: '10.0.40.8',
        port: 5432,
        qps: 1540,
        activeConnections: 45,
        maxConnections: 500,
        replicationLagMs: 5,
        status: DbHealthStatus.healthy,
        clusterRole: '预发布同步环境',
      ),
    ];
  }
}
