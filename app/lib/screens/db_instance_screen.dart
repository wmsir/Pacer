import 'package:flutter/material.dart';
import '../models/db_instance_model.dart';
import '../services/cluster_service.dart';
import '../theme/pacer_theme.dart';

class DbInstanceScreen extends StatefulWidget {
  const DbInstanceScreen({super.key});

  @override
  State<DbInstanceScreen> createState() => _DbInstanceScreenState();
}

class _DbInstanceScreenState extends State<DbInstanceScreen> {
  String _selectedFilter = '全部';
  final List<String> _filters = ['全部', 'TiDB', 'MySQL', 'OceanBase', '信创国产库'];

  @override
  void initState() {
    super.initState();
    ClusterService().fetchInstances();
    ClusterService().addListener(_onServiceUpdate);
  }

  void _onServiceUpdate() {
    if (mounted) setState(() {});
  }

  @override
  void dispose() {
    ClusterService().removeListener(_onServiceUpdate);
    super.dispose();
  }

  List<DbInstance> _getFilteredInstances() {
    final list = ClusterService().instances;
    if (_selectedFilter == '全部') return list;
    if (_selectedFilter == 'TiDB') return list.where((i) => i.dbType.contains('TiDB')).toList();
    if (_selectedFilter == 'MySQL') return list.where((i) => i.dbType.contains('MySQL')).toList();
    if (_selectedFilter == 'OceanBase') return list.where((i) => i.dbType.contains('OceanBase')).toList();
    if (_selectedFilter == '信创国产库') {
      return list.where((i) => i.dbType.contains('DM8') || i.dbType.contains('openGauss')).toList();
    }
    return list;
  }

  @override
  Widget build(BuildContext context) {
    final instances = _getFilteredInstances();
    final metrics = ClusterService().getMetrics();

    return Scaffold(
      appBar: AppBar(
        title: const Text('多云数据库集群拓扑'),
        actions: [
          IconButton(
            icon: const Icon(Icons.refresh),
            onPressed: () => ClusterService().fetchInstances(forceRefresh: true),
          ),
        ],
      ),
      body: Column(
        children: [
          // 顶部汇总统计卡片
          Container(
            padding: const EdgeInsets.all(16),
            color: PacerTheme.primaryBlue,
            child: Row(
              mainAxisAlignment: MainAxisAlignment.spaceAround,
              children: [
                _buildSummaryItem('集群总吞吐', '${metrics.totalQps}', 'QPS'),
                Container(width: 1, height: 28, color: Colors.white24),
                _buildSummaryItem('在线连接', '${metrics.totalConnections}', '条'),
                Container(width: 1, height: 28, color: Colors.white24),
                _buildSummaryItem('熔断拦截', '${metrics.breakerCount}', '次'),
                Container(width: 1, height: 28, color: Colors.white24),
                _buildSummaryItem('SLA 稳定性', '${metrics.sla}%', '合规'),
              ],
            ),
          ),

          // 筛选分类
          Container(
            color: Colors.white,
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
            child: SingleChildScrollView(
              scrollDirection: Axis.horizontal,
              child: Row(
                children: _filters.map((filter) {
                  final isSelected = _selectedFilter == filter;
                  return Padding(
                    padding: const EdgeInsets.only(right: 8.0),
                    child: ChoiceChip(
                      label: Text(filter),
                      selected: isSelected,
                      selectedColor: PacerTheme.primaryBlue.withOpacity(0.12),
                      labelStyle: TextStyle(
                        fontSize: 12,
                        fontWeight: isSelected ? FontWeight.bold : FontWeight.normal,
                        color: isSelected ? PacerTheme.primaryBlue : const Color(0xFF64748B),
                      ),
                      onSelected: (selected) {
                        if (selected) setState(() => _selectedFilter = filter);
                      },
                    ),
                  );
                }).toList(),
              ),
            ),
          ),
          const Divider(height: 1, color: Color(0xFFE2E8F0)),

          // 实例卡片列表
          Expanded(
            child: ListView.separated(
              padding: const EdgeInsets.all(16),
              itemCount: instances.length,
              separatorBuilder: (_, __) => const SizedBox(height: 12),
              itemBuilder: (context, index) {
                final inst = instances[index];
                return _buildInstanceCard(inst);
              },
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildSummaryItem(String label, String value, String unit) {
    return Column(
      children: [
        Text(label, style: const TextStyle(color: Color(0xFF93C5FD), fontSize: 11)),
        const SizedBox(height: 4),
        Row(
          crossAxisAlignment: CrossAxisAlignment.baseline,
          textBaseline: TextBaseline.alphabetic,
          children: [
            Text(
              value,
              style: const TextStyle(
                color: Colors.white,
                fontSize: 18,
                fontWeight: FontWeight.bold,
              ),
            ),
            const SizedBox(width: 2),
            Text(unit, style: const TextStyle(color: Colors.white70, fontSize: 10)),
          ],
        ),
      ],
    );
  }

  Widget _buildInstanceCard(DbInstance inst) {
    final connPercent = (inst.activeConnections / inst.maxConnections).clamp(0.0, 1.0);

    return Card(
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
            // 实例名与状态
            Row(
              children: [
                Container(
                  width: 10,
                  height: 10,
                  decoration: BoxDecoration(
                    shape: BoxShape.circle,
                    color: inst.statusColor,
                  ),
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    inst.instanceName,
                    style: const TextStyle(
                      fontSize: 15,
                      fontWeight: FontWeight.bold,
                      color: Color(0xFF0F172A),
                    ),
                  ),
                ),
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                  decoration: BoxDecoration(
                    color: inst.env == 'PROD' ? const Color(0xFFFEE2E2) : const Color(0xFFE0F2FE),
                    borderRadius: BorderRadius.circular(4),
                  ),
                  child: Text(
                    inst.env,
                    style: TextStyle(
                      fontSize: 10,
                      fontWeight: FontWeight.bold,
                      color: inst.env == 'PROD' ? const Color(0xFFDC2626) : const Color(0xFF0284C7),
                    ),
                  ),
                ),
              ],
            ),
            const SizedBox(height: 8),

            // 架构角色与引擎
            Row(
              children: [
                Text(
                  inst.dbType,
                  style: const TextStyle(fontSize: 12, fontWeight: FontWeight.w600, color: PacerTheme.accentCyan),
                ),
                const Text(' · ', style: TextStyle(color: Color(0xFFCBD5E1))),
                Text(
                  inst.clusterRole,
                  style: const TextStyle(fontSize: 12, color: Color(0xFF64748B)),
                ),
              ],
            ),
            const SizedBox(height: 12),
            const Divider(height: 1, color: Color(0xFFF1F5F9)),
            const SizedBox(height: 12),

            // 指标栅格
            Row(
              children: [
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      const Text('实时 QPS', style: TextStyle(fontSize: 11, color: Color(0xFF94A3B8))),
                      const SizedBox(height: 2),
                      Text('${inst.qps}', style: const TextStyle(fontSize: 14, fontWeight: FontWeight.bold, color: Color(0xFF1E293B))),
                    ],
                  ),
                ),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      const Text('主从复制延迟', style: TextStyle(fontSize: 11, color: Color(0xFF94A3B8))),
                      const SizedBox(height: 2),
                      Text(
                        '${inst.replicationLagMs} ms',
                        style: TextStyle(
                          fontSize: 14,
                          fontWeight: FontWeight.bold,
                          color: inst.replicationLagMs > 10 ? PacerTheme.riskMedium : const Color(0xFF10B981),
                        ),
                      ),
                    ],
                  ),
                ),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text('连接数 (${inst.activeConnections}/${inst.maxConnections})', style: const TextStyle(fontSize: 11, color: Color(0xFF94A3B8))),
                      const SizedBox(height: 4),
                      ClipRRect(
                        borderRadius: BorderRadius.circular(3),
                        child: LinearProgressIndicator(
                          value: connPercent,
                          backgroundColor: const Color(0xFFE2E8F0),
                          valueColor: AlwaysStoppedAnimation<Color>(
                            connPercent > 0.8 ? PacerTheme.riskHigh : PacerTheme.primaryBlue,
                          ),
                          minHeight: 6,
                        ),
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}
