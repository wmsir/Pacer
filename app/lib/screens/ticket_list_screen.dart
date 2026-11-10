import 'package:flutter/material.dart';
import '../models/ticket_model.dart';
import '../services/api_service.dart';
import '../services/ticket_service.dart';
import '../theme/pacer_theme.dart';
import 'network_diagnostics_screen.dart';
import 'ticket_detail_screen.dart';

class TicketListScreen extends StatefulWidget {
  const TicketListScreen({super.key});

  @override
  State<TicketListScreen> createState() => _TicketListScreenState();
}

class _TicketListScreenState extends State<TicketListScreen> with SingleTickerProviderStateMixin {
  late TabController _tabController;
  final TextEditingController _searchController = TextEditingController();
  RiskLevel? _selectedRisk;

  final List<String> _tabs = ['待我审批', '全部工单', '定时待执行', '执行中', '已执行', '已驳回', '已熔断/失败'];

  @override
  void initState() {
    super.initState();
    _tabController = TabController(length: _tabs.length, vsync: this);
    _tabController.addListener(() {
      setState(() {});
    });
    TicketService().fetchTickets();
    TicketService().addListener(_onServiceUpdate);
  }

  void _onServiceUpdate() {
    if (mounted) setState(() {});
  }

  @override
  void dispose() {
    _tabController.dispose();
    _searchController.dispose();
    TicketService().removeListener(_onServiceUpdate);
    super.dispose();
  }

  List<SqlTicket> _getFilteredTickets() {
    final all = TicketService().tickets;
    final currentTab = _tabs[_tabController.index];
    final query = _searchController.text.trim().toLowerCase();

    return all.where((ticket) {
      // 1. 状态过滤
      if (currentTab == '待我审批' && ticket.status != TicketStatus.pending) return false;
      if (currentTab == '定时待执行' && ticket.status != TicketStatus.waitingExecution) return false;
      if (currentTab == '执行中' && ticket.status != TicketStatus.running) return false;
      if (currentTab == '已执行' && ticket.status != TicketStatus.executed) return false;
      if (currentTab == '已驳回' && ticket.status != TicketStatus.rejected) return false;
      if (currentTab == '已熔断/失败' && ticket.status != TicketStatus.failed) return false;

      // 2. 风险等级过滤
      if (_selectedRisk != null && ticket.riskLevel != _selectedRisk) return false;

      // 3. 搜索匹配
      if (query.isNotEmpty) {
        final matchesNo = ticket.ticketNo.toLowerCase().contains(query);
        final matchesTitle = ticket.title.toLowerCase().contains(query);
        final matchesDb = ticket.dbName.toLowerCase().contains(query);
        final matchesSubmitter = ticket.submitter.toLowerCase().contains(query);
        if (!matchesNo && !matchesTitle && !matchesDb && !matchesSubmitter) return false;
      }

      return true;
    }).toList();
  }

  @override
  Widget build(BuildContext context) {
    final filtered = _getFilteredTickets();
    final currentGateway = ApiService().currentGateway;
    final pendingCount = TicketService().pendingCount;

    return Scaffold(
      appBar: AppBar(
        title: const Text('稳骥 PacerSQL 审批台'),
        actions: [
          TextButton.icon(
            onPressed: () {
              Navigator.of(context).push(
                MaterialPageRoute(builder: (_) => const NetworkDiagnosticsScreen()),
              );
            },
            icon: const Icon(Icons.hub_outlined, size: 16, color: PacerTheme.accentCyan),
            label: Text(
              currentGateway.carrierTag,
              style: const TextStyle(fontSize: 12, color: PacerTheme.accentCyan),
            ),
          ),
          const SizedBox(width: 8),
        ],
        bottom: PreferredSize(
          preferredSize: const Size.fromHeight(48),
          child: TabBar(
            controller: _tabController,
            isScrollable: true,
            tabAlignment: TabAlignment.start,
            labelColor: PacerTheme.primaryBlue,
            unselectedLabelColor: const Color(0xFF64748B),
            indicatorColor: PacerTheme.primaryBlue,
            indicatorWeight: 3,
            labelStyle: const TextStyle(fontWeight: FontWeight.bold, fontSize: 14),
            tabs: _tabs.map((tab) {
              if (tab == '待我审批' && pendingCount > 0) {
                return Tab(
                  child: Row(
                    children: [
                      Text(tab),
                      const SizedBox(width: 6),
                      Container(
                        padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                        decoration: BoxDecoration(
                          color: const Color(0xFFEF4444),
                          borderRadius: BorderRadius.circular(10),
                        ),
                        child: Text(
                          '$pendingCount',
                          style: const TextStyle(color: Colors.white, fontSize: 11, fontWeight: FontWeight.bold),
                        ),
                      ),
                    ],
                  ),
                );
              }
              return Tab(text: tab);
            }).toList(),
          ),
        ),
      ),
      body: Column(
        children: [
          // 搜索与风险快速筛选栏
          Container(
            color: Colors.white,
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
            child: Column(
              children: [
                TextField(
                  controller: _searchController,
                  onChanged: (_) => setState(() {}),
                  decoration: InputDecoration(
                    hintText: '搜索工单号、库名、标题或申请人',
                    hintStyle: const TextStyle(fontSize: 13, color: Color(0xFF94A3B8)),
                    prefixIcon: const Icon(Icons.search, size: 20, color: Color(0xFF64748B)),
                    suffixIcon: _searchController.text.isNotEmpty
                        ? IconButton(
                            icon: const Icon(Icons.clear, size: 18),
                            onPressed: () {
                              _searchController.clear();
                              setState(() {});
                            },
                          )
                        : null,
                    contentPadding: const EdgeInsets.symmetric(vertical: 8),
                  ),
                ),
                const SizedBox(height: 8),
                Row(
                  children: [
                    const Text('风险等级:', style: TextStyle(fontSize: 12, color: Color(0xFF64748B))),
                    const SizedBox(width: 8),
                    _buildRiskChip('全部', null),
                    const SizedBox(width: 6),
                    _buildRiskChip('三级高危', RiskLevel.high, const Color(0xFFEF4444)),
                    const SizedBox(width: 6),
                    _buildRiskChip('二级中危', RiskLevel.medium, const Color(0xFFF59E0B)),
                    const SizedBox(width: 6),
                    _buildRiskChip('一级安全', RiskLevel.low, const Color(0xFF10B981)),
                  ],
                ),
              ],
            ),
          ),
          const Divider(height: 1, color: Color(0xFFE2E8F0)),

          // 工单列表区域
          Expanded(
            child: RefreshIndicator(
              onRefresh: () => TicketService().fetchTickets(forceRefresh: true),
              child: filtered.isEmpty
                  ? ListView(
                      physics: const AlwaysScrollableScrollPhysics(),
                      children: const [
                        SizedBox(height: 120),
                        Center(
                          child: Column(
                            children: [
                              Icon(Icons.inbox_outlined, size: 54, color: Color(0xFFCBD5E1)),
                              SizedBox(height: 12),
                              Text('暂无符合条件的工单', style: TextStyle(color: Color(0xFF94A3B8))),
                            ],
                          ),
                        ),
                      ],
                    )
                  : ListView.separated(
                      padding: const EdgeInsets.all(16),
                      itemCount: filtered.length,
                      separatorBuilder: (_, __) => const SizedBox(height: 12),
                      itemBuilder: (context, index) {
                        final ticket = filtered[index];
                        return _buildTicketCard(ticket);
                      },
                    ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildRiskChip(String label, RiskLevel? level, [Color? color]) {
    final isSelected = _selectedRisk == level;
    return InkWell(
      onTap: () => setState(() => _selectedRisk = level),
      borderRadius: BorderRadius.circular(12),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
        decoration: BoxDecoration(
          color: isSelected
              ? (color ?? PacerTheme.primaryBlue).withOpacity(0.12)
              : const Color(0xFFF1F5F9),
          borderRadius: BorderRadius.circular(12),
          border: Border.all(
            color: isSelected ? (color ?? PacerTheme.primaryBlue) : const Color(0xFFE2E8F0),
          ),
        ),
        child: Text(
          label,
          style: TextStyle(
            fontSize: 11,
            fontWeight: isSelected ? FontWeight.bold : FontWeight.normal,
            color: isSelected ? (color ?? PacerTheme.primaryBlue) : const Color(0xFF64748B),
          ),
        ),
      ),
    );
  }

  Widget _buildTicketCard(SqlTicket ticket) {
    return Card(
      elevation: 0,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(12),
        side: BorderSide(
          color: ticket.riskLevel == RiskLevel.high
              ? const Color(0xFFFCA5A5)
              : const Color(0xFFE2E8F0),
          width: ticket.riskLevel == RiskLevel.high ? 1.5 : 1,
        ),
      ),
      child: InkWell(
        onTap: () {
          Navigator.of(context).push(
            MaterialPageRoute(
              builder: (_) => TicketDetailScreen(ticket: ticket),
            ),
          );
        },
        borderRadius: BorderRadius.circular(12),
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // 顶部流水号与状态标签
              Row(
                children: [
                  Container(
                    padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                    decoration: BoxDecoration(
                      color: PacerTheme.primaryBlue.withOpacity(0.08),
                      borderRadius: BorderRadius.circular(4),
                    ),
                    child: Text(
                      ticket.changeType,
                      style: const TextStyle(
                        fontSize: 11,
                        fontWeight: FontWeight.bold,
                        color: PacerTheme.primaryBlue,
                      ),
                    ),
                  ),
                  const SizedBox(width: 8),
                  Text(
                    ticket.ticketNo,
                    style: const TextStyle(
                      fontSize: 12,
                      fontWeight: FontWeight.w600,
                      color: Color(0xFF64748B),
                    ),
                  ),
                  const Spacer(),
                  Container(
                    padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
                    decoration: BoxDecoration(
                      color: ticket.statusColor.withOpacity(0.12),
                      borderRadius: BorderRadius.circular(12),
                    ),
                    child: Text(
                      ticket.statusText,
                      style: TextStyle(
                        fontSize: 11,
                        fontWeight: FontWeight.bold,
                        color: ticket.statusColor,
                      ),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 10),

              // 工单标题
              Text(
                ticket.title,
                maxLines: 2,
                overflow: TextOverflow.ellipsis,
                style: const TextStyle(
                  fontSize: 15,
                  fontWeight: FontWeight.bold,
                  color: Color(0xFF0F172A),
                  height: 1.3,
                ),
              ),
              const SizedBox(height: 12),

              // 实例与库名
              Row(
                children: [
                  const Icon(Icons.dns_outlined, size: 14, color: Color(0xFF64748B)),
                  const SizedBox(width: 4),
                  Expanded(
                    child: Text(
                      '${ticket.instanceName} / ${ticket.dbName}',
                      style: const TextStyle(fontSize: 12, color: Color(0xFF475569)),
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 12),
              const Divider(height: 1, color: Color(0xFFF1F5F9)),
              const SizedBox(height: 10),

              // 底部元信息：申请人、提交时间、AST 风险分数
              Row(
                children: [
                  const Icon(Icons.person_outline, size: 14, color: Color(0xFF94A3B8)),
                  const SizedBox(width: 4),
                  Text(
                    ticket.submitter,
                    style: const TextStyle(fontSize: 12, color: Color(0xFF64748B)),
                  ),
                  const Spacer(),
                  // AST 风险徽章
                  Container(
                    padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
                    decoration: BoxDecoration(
                      color: ticket.riskColor.withOpacity(0.1),
                      borderRadius: BorderRadius.circular(6),
                      border: Border.all(color: ticket.riskColor.withOpacity(0.4)),
                    ),
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Icon(Icons.shield, size: 12, color: ticket.riskColor),
                        const SizedBox(width: 4),
                        Text(
                          'AST 风控 ${ticket.riskScore}分',
                          style: TextStyle(
                            fontSize: 11,
                            fontWeight: FontWeight.bold,
                            color: ticket.riskColor,
                          ),
                        ),
                      ],
                    ),
                  ),
                ],
              ),

              if (ticket.scheduledTime != null && ticket.status == TicketStatus.waitingExecution) ...[
                const SizedBox(height: 8),
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
                  decoration: BoxDecoration(
                    color: const Color(0xFFFAF5FF),
                    borderRadius: BorderRadius.circular(6),
                    border: Border.all(color: const Color(0xFFE9D5FF)),
                  ),
                  child: Row(
                    children: [
                      const Icon(Icons.schedule, size: 14, color: Color(0xFF8B5CF6)),
                      const SizedBox(width: 6),
                      Text(
                        '计划执行窗口: ${ticket.scheduledTime}',
                        style: const TextStyle(fontSize: 11, color: Color(0xFF7C3AED), fontWeight: FontWeight.w500),
                      ),
                    ],
                  ),
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }
}
