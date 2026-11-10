import 'package:flutter/material.dart';
import '../models/ticket_model.dart';
import '../services/cluster_service.dart';
import '../services/ticket_service.dart';
import '../theme/pacer_theme.dart';
import 'ticket_detail_screen.dart';

class DashboardScreen extends StatefulWidget {
  const DashboardScreen({super.key});

  @override
  State<DashboardScreen> createState() => _DashboardScreenState();
}

class _DashboardScreenState extends State<DashboardScreen> {
  @override
  void initState() {
    super.initState();
    TicketService().addListener(_onUpdate);
    ClusterService().addListener(_onUpdate);
    TicketService().fetchTickets();
    ClusterService().fetchInstances();
  }

  void _onUpdate() {
    if (mounted) setState(() {});
  }

  @override
  void dispose() {
    TicketService().removeListener(_onUpdate);
    ClusterService().removeListener(_onUpdate);
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final ticketService = TicketService();
    final clusterService = ClusterService();
    final metrics = clusterService.getMetrics();
    final allTickets = ticketService.tickets;
    final totalCount = allTickets.isNotEmpty ? allTickets.length : 1;
    final highRiskCount = allTickets.where((t) => t.riskLevel == RiskLevel.high).length;
    final mediumRiskCount = allTickets.where((t) => t.riskLevel == RiskLevel.medium).length;
    final lowRiskCount = allTickets.where((t) => t.riskLevel == RiskLevel.low).length;
    final blockedCount = allTickets
        .where((t) => t.status == TicketStatus.rejected || t.status == TicketStatus.failed)
        .length;

    return Scaffold(
      appBar: AppBar(
        title: const Text('稳骥运维风控大屏'),
        actions: [
          IconButton(
            icon: const Icon(Icons.refresh, size: 20),
            tooltip: '刷新实时态势',
            onPressed: () {
              TicketService().fetchTickets(forceRefresh: true);
              ClusterService().fetchInstances(forceRefresh: true);
            },
          ),
        ],
      ),
      body: RefreshIndicator(
        onRefresh: () async {
          await TicketService().fetchTickets(forceRefresh: true);
          await ClusterService().fetchInstances(forceRefresh: true);
        },
        child: SingleChildScrollView(
          physics: const AlwaysScrollableScrollPhysics(),
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // 4大核心指标卡片
              Row(
                children: [
                  Expanded(
                    child: _buildMetricTile(
                      title: '待我审批',
                      value: '${ticketService.pendingCount}',
                      unit: '单',
                      color: PacerTheme.accentCyan,
                      icon: Icons.assignment_late_outlined,
                    ),
                  ),
                  const SizedBox(width: 12),
                  Expanded(
                    child: _buildMetricTile(
                      title: '流式运行中',
                      value: '${ticketService.runningCount}',
                      unit: '个批次',
                      color: const Color(0xFFF59E0B),
                      icon: Icons.sync,
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 12),
              Row(
                children: [
                  Expanded(
                    child: _buildMetricTile(
                      title: '生产可用率',
                      value: '${metrics.sla}%',
                      unit: 'SLA 合规',
                      color: const Color(0xFF10B981),
                      icon: Icons.health_and_safety_outlined,
                    ),
                  ),
                  const SizedBox(width: 12),
                  Expanded(
                    child: _buildMetricTile(
                      title: '阻断高危操作',
                      value: '${blockedCount > 0 ? blockedCount : highRiskCount}',
                      unit: '次 (合规)',
                      color: PacerTheme.riskHigh,
                      icon: Icons.gpp_bad_outlined,
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 24),

              // AST 风险拦截分布
              Card(
                elevation: 0,
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(12),
                  side: const BorderSide(color: Color(0xFFE2E8F0)),
                ),
                child: Padding(
                  padding: const EdgeInsets.all(16),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      const Row(
                        children: [
                          Icon(Icons.pie_chart_outline, size: 18, color: PacerTheme.primaryBlue),
                          SizedBox(width: 8),
                          Text('AST 语义风控等级分布', style: TextStyle(fontSize: 14, fontWeight: FontWeight.bold, color: Color(0xFF0F172A))),
                        ],
                      ),
                      const SizedBox(height: 16),
                      _buildRiskBar('三级高危阻断 (DDL/删表)', highRiskCount, totalCount, const Color(0xFFEF4444)),
                      const SizedBox(height: 10),
                      _buildRiskBar('二级中危提醒 (大批量/分批)', mediumRiskCount, totalCount, const Color(0xFFF59E0B)),
                      const SizedBox(height: 10),
                      _buildRiskBar('一级安全合规 (Dry-Run 通过)', lowRiskCount, totalCount, const Color(0xFF10B981)),
                    ],
                  ),
                ),
              ),
              const SizedBox(height: 16),

              // Pacer 微批步态安全控速引擎状态
              Card(
                elevation: 0,
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(12),
                  side: const BorderSide(color: Color(0xFFE2E8F0)),
                ),
                child: Padding(
                  padding: const EdgeInsets.all(16),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      const Row(
                        children: [
                          Icon(Icons.speed, size: 18, color: Color(0xFF10B981)),
                          SizedBox(width: 8),
                          Text('Pacer 微批步态控速特性', style: TextStyle(fontSize: 14, fontWeight: FontWeight.bold, color: Color(0xFF0F172A))),
                        ],
                      ),
                      const SizedBox(height: 12),
                      _buildPacerFeatureItem(
                        '主从复制零延迟防护',
                        '流式微批休眠机制，保障从库 Binlog 延迟恒定 <= 10ms',
                      ),
                      _buildPacerFeatureItem(
                        '自适应锁持有窗口',
                        '每次仅锁定微批范围 (默认 1000行)，单批执行耗时 < 3ms，拒绝锁全表',
                      ),
                      _buildPacerFeatureItem(
                        '事务级断点续传与熔断',
                        '若监测到底层 QPS 突增或 CPU > 85%，自适应扩大休眠间隔或暂停下发',
                      ),
                    ],
                  ),
                ),
              ),
              const SizedBox(height: 16),

              // 实时审计日志流水 (与真实工单池实时联动，支持点击跳转详情)
              Card(
                elevation: 0,
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(12),
                  side: const BorderSide(color: Color(0xFFE2E8F0)),
                ),
                child: Padding(
                  padding: const EdgeInsets.all(16),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      const Row(
                        children: [
                          Icon(Icons.history, size: 18, color: Color(0xFF64748B)),
                          SizedBox(width: 8),
                          Text('最新运控审计事件 (点击可查看工单)', style: TextStyle(fontSize: 14, fontWeight: FontWeight.bold, color: Color(0xFF0F172A))),
                        ],
                      ),
                      const SizedBox(height: 12),
                      ...allTickets.take(5).map(
                        (t) => InkWell(
                          onTap: () {
                            Navigator.of(context).push(
                              MaterialPageRoute(builder: (_) => TicketDetailScreen(ticket: t)),
                            );
                          },
                          borderRadius: BorderRadius.circular(6),
                          child: _buildAuditEvent(
                            time: t.submitTime.length > 10 ? t.submitTime.substring(t.submitTime.length - 8) : t.submitTime,
                            action: '${t.submitter} · ${t.title} (${t.ticketNo})',
                            status: t.statusText,
                            statusColor: t.statusColor,
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildMetricTile({
    required String title,
    required String value,
    required String unit,
    required Color color,
    required IconData icon,
  }) {
    return Container(
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: Colors.white,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: const Color(0xFFE2E8F0)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Icon(icon, size: 18, color: color),
              const Spacer(),
              Container(
                width: 6,
                height: 6,
                decoration: BoxDecoration(shape: BoxShape.circle, color: color),
              ),
            ],
          ),
          const SizedBox(height: 12),
          Text(
            value,
            style: TextStyle(
              fontSize: 22,
              fontWeight: FontWeight.bold,
              color: color,
            ),
          ),
          const SizedBox(height: 4),
          Row(
            children: [
              Text(title, style: const TextStyle(fontSize: 12, color: Color(0xFF64748B))),
              const Spacer(),
              Text(unit, style: const TextStyle(fontSize: 11, color: Color(0xFF94A3B8))),
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildRiskBar(String label, int count, int total, Color color) {
    final double percent = count / total;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          mainAxisAlignment: MainAxisAlignment.spaceBetween,
          children: [
            Text(label, style: const TextStyle(fontSize: 12, color: Color(0xFF475569))),
            Text('$count 单 (${(percent * 100).toInt()}%)', style: TextStyle(fontSize: 12, fontWeight: FontWeight.bold, color: color)),
          ],
        ),
        const SizedBox(height: 6),
        ClipRRect(
          borderRadius: BorderRadius.circular(4),
          child: LinearProgressIndicator(
            value: percent,
            minHeight: 8,
            backgroundColor: const Color(0xFFF1F5F9),
            valueColor: AlwaysStoppedAnimation<Color>(color),
          ),
        ),
      ],
    );
  }

  Widget _buildPacerFeatureItem(String title, String desc) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 10),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Icon(Icons.check_circle, color: Color(0xFF10B981), size: 16),
          const SizedBox(width: 8),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(title, style: const TextStyle(fontSize: 13, fontWeight: FontWeight.bold, color: Color(0xFF1E293B))),
                const SizedBox(height: 2),
                Text(desc, style: const TextStyle(fontSize: 11, color: Color(0xFF64748B))),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildAuditEvent({
    required String time,
    required String action,
    required String status,
    Color statusColor = PacerTheme.accentCyan,
  }) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 6),
      child: Row(
        children: [
          Text(time, style: const TextStyle(fontSize: 11, fontFamily: 'monospace', color: Color(0xFF94A3B8))),
          const SizedBox(width: 8),
          Expanded(
            child: Text(
              action,
              style: const TextStyle(fontSize: 12, color: Color(0xFF334155)),
              overflow: TextOverflow.ellipsis,
            ),
          ),
          const SizedBox(width: 8),
          Text(
            status,
            style: TextStyle(fontSize: 11, color: statusColor, fontWeight: FontWeight.w600),
          ),
          const SizedBox(width: 4),
          const Icon(Icons.chevron_right, size: 14, color: Color(0xFFCBD5E1)),
        ],
      ),
    );
  }
}
