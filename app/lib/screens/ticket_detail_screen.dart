import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import '../models/ticket_model.dart';
import '../services/auth_service.dart';
import '../services/ticket_service.dart';
import '../theme/pacer_theme.dart';

class TicketDetailScreen extends StatefulWidget {
  final SqlTicket ticket;

  const TicketDetailScreen({super.key, required this.ticket});

  @override
  State<TicketDetailScreen> createState() => _TicketDetailScreenState();
}

class _TicketDetailScreenState extends State<TicketDetailScreen> with SingleTickerProviderStateMixin {
  late TabController _tabController;
  SqlTicketDetail? _detail;
  bool _isLoading = true;

  @override
  void initState() {
    super.initState();
    _tabController = TabController(length: 3, vsync: this);
    _loadDetail();
  }

  Future<void> _loadDetail() async {
    setState(() => _isLoading = true);
    final detail = await TicketService().getTicketDetail(widget.ticket.id);
    if (mounted) {
      setState(() {
        _detail = detail;
        _isLoading = false;
      });
    }
  }

  @override
  void dispose() {
    _tabController.dispose();
    super.dispose();
  }

  void _showRejectDialog() {
    final reasonController = TextEditingController();
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('驳回变更工单', style: TextStyle(fontSize: 16, fontWeight: FontWeight.bold)),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Text('请填写具体的风控违规原因或修改建议：', style: TextStyle(fontSize: 13, color: Color(0xFF64748B))),
            const SizedBox(height: 10),
            TextField(
              controller: reasonController,
              maxLines: 3,
              decoration: const InputDecoration(
                hintText: '如：未命中复合索引、批量操作缺少限制、请收敛权限等',
                hintStyle: TextStyle(fontSize: 12),
              ),
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(ctx).pop(),
            child: const Text('取消'),
          ),
          ElevatedButton(
            style: ElevatedButton.styleFrom(backgroundColor: PacerTheme.riskHigh),
            onPressed: () async {
              final reason = reasonController.text.trim();
              if (reason.isEmpty) return;
              Navigator.of(ctx).pop();
              final success = await TicketService().rejectTicket(
                ticketId: widget.ticket.id,
                reason: reason,
              );
              if (mounted && success) {
                ScaffoldMessenger.of(context).showSnackBar(
                  const SnackBar(content: Text('已驳回工单，并触发告警通知申请人')),
                );
                Navigator.of(context).pop();
              }
            },
            child: const Text('确认驳回'),
          ),
        ],
      ),
    );
  }

  void _showUrgeDialog() {
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('生产工单催办', style: TextStyle(fontSize: 16, fontWeight: FontWeight.bold)),
        content: const Text(
          '将向当前审批席、生产 DBA 及架构专家发送多渠道加急短信与钉钉告警，确定催办？',
          style: TextStyle(fontSize: 13, color: Color(0xFF475569)),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(ctx).pop(),
            child: const Text('取消'),
          ),
          ElevatedButton(
            onPressed: () async {
              Navigator.of(ctx).pop();
              await TicketService().urgeTicket(widget.ticket.id);
              if (mounted) {
                ScaffoldMessenger.of(context).showSnackBar(
                  const SnackBar(content: Text('催办指令已通过高可用专线发出，当前排队优先级已提升')),
                );
              }
            },
            child: const Text('发送催办'),
          ),
        ],
      ),
    );
  }

  void _showApprovalSheet() {
    String executionMode = 'SCHEDULED';
    int batchSize = 1000;
    int intervalMs = 20;
    final commentController = TextEditingController(text: '同意执行，已配置 Pacer 微批步态安全控速');
    final now = DateTime.now();
    DateTime scheduledDate = DateTime(now.year, now.month, now.day, 23, 30);
    if (scheduledDate.isBefore(now)) {
      scheduledDate = scheduledDate.add(const Duration(days: 1));
    }

    String formatDateTime(DateTime dt) {
      return '${dt.year}-${dt.month.toString().padLeft(2, '0')}-${dt.day.toString().padLeft(2, '0')} '
          '${dt.hour.toString().padLeft(2, '0')}:${dt.minute.toString().padLeft(2, '0')}:00';
    }

    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      backgroundColor: Colors.transparent,
      builder: (ctx) => StatefulBuilder(
        builder: (sheetCtx, setSheetState) {
          return Container(
            height: MediaQuery.of(sheetCtx).size.height * 0.78,
            decoration: const BoxDecoration(
              color: Colors.white,
              borderRadius: BorderRadius.vertical(top: Radius.circular(20)),
            ),
            padding: const EdgeInsets.all(20),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                // 顶部把手与标题
                Center(
                  child: Container(
                    width: 40,
                    height: 4,
                    decoration: BoxDecoration(
                      color: const Color(0xFFCBD5E1),
                      borderRadius: BorderRadius.circular(2),
                    ),
                  ),
                ),
                const SizedBox(height: 16),
                Row(
                  children: [
                    const Icon(Icons.verified, color: PacerTheme.accentCyan, size: 22),
                    const SizedBox(width: 8),
                    const Text(
                      'Pacer 生产审批与步态配置',
                      style: TextStyle(fontSize: 17, fontWeight: FontWeight.bold, color: Color(0xFF0F172A)),
                    ),
                    const Spacer(),
                    IconButton(
                      icon: const Icon(Icons.close, size: 20),
                      onPressed: () => Navigator.of(ctx).pop(),
                    ),
                  ],
                ),
                const Divider(height: 20),

                Expanded(
                  child: ListView(
                    children: [
                      // 执行模式选择
                      const Text(
                        '1. 选择执行模式 (Execution Mode)',
                        style: TextStyle(fontSize: 13, fontWeight: FontWeight.bold, color: Color(0xFF334155)),
                      ),
                      const SizedBox(height: 8),
                      Row(
                        children: [
                          _buildModeOption(
                            label: '定时低峰窗口',
                            value: 'SCHEDULED',
                            icon: Icons.alarm,
                            selected: executionMode == 'SCHEDULED',
                            onTap: () => setSheetState(() => executionMode = 'SCHEDULED'),
                          ),
                          const SizedBox(width: 8),
                          _buildModeOption(
                            label: '立即流式执行',
                            value: 'IMMEDIATE',
                            icon: Icons.flash_on,
                            selected: executionMode == 'IMMEDIATE',
                            onTap: () => setSheetState(() => executionMode = 'IMMEDIATE'),
                          ),
                          const SizedBox(width: 8),
                          _buildModeOption(
                            label: '线下 DBA 托管',
                            value: 'MANUAL_DBA',
                            icon: Icons.shield,
                            selected: executionMode == 'MANUAL_DBA',
                            onTap: () => setSheetState(() => executionMode = 'MANUAL_DBA'),
                          ),
                        ],
                      ),

                      if (executionMode == 'SCHEDULED') ...[
                        const SizedBox(height: 14),
                        Container(
                          padding: const EdgeInsets.all(12),
                          decoration: BoxDecoration(
                            color: const Color(0xFFFAF5FF),
                            borderRadius: BorderRadius.circular(10),
                            border: Border.all(color: const Color(0xFFE9D5FF)),
                          ),
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Row(
                                children: [
                                  const Icon(Icons.calendar_month, color: Color(0xFF8B5CF6), size: 18),
                                  const SizedBox(width: 8),
                                  Expanded(
                                    child: Text(
                                      '计划执行窗口: ${formatDateTime(scheduledDate)}',
                                      style: const TextStyle(
                                        fontSize: 12.5,
                                        color: Color(0xFF6D28D9),
                                        fontWeight: FontWeight.bold,
                                      ),
                                    ),
                                  ),
                                  TextButton.icon(
                                    style: TextButton.styleFrom(
                                      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                                      minimumSize: Size.zero,
                                      tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                                    ),
                                    onPressed: () async {
                                      final pickedDate = await showDatePicker(
                                        context: sheetCtx,
                                        initialDate: scheduledDate,
                                        firstDate: DateTime.now(),
                                        lastDate: DateTime.now().add(const Duration(days: 30)),
                                      );
                                      if (pickedDate == null || !sheetCtx.mounted) return;
                                      final pickedTime = await showTimePicker(
                                        context: sheetCtx,
                                        initialTime: TimeOfDay.fromDateTime(scheduledDate),
                                      );
                                      if (pickedTime == null) return;
                                      setSheetState(() {
                                        scheduledDate = DateTime(
                                          pickedDate.year,
                                          pickedDate.month,
                                          pickedDate.day,
                                          pickedTime.hour,
                                          pickedTime.minute,
                                        );
                                      });
                                    },
                                    icon: const Icon(Icons.edit_calendar, size: 15, color: Color(0xFF7C3AED)),
                                    label: const Text('自定义时间', style: TextStyle(fontSize: 12, color: Color(0xFF7C3AED))),
                                  ),
                                ],
                              ),
                              const SizedBox(height: 8),
                              Wrap(
                                spacing: 8,
                                children: [
                                  ActionChip(
                                    label: const Text('🌙 今晚 23:30', style: TextStyle(fontSize: 11)),
                                    visualDensity: VisualDensity.compact,
                                    onPressed: () {
                                      final n = DateTime.now();
                                      setSheetState(() {
                                        scheduledDate = DateTime(n.year, n.month, n.day, 23, 30);
                                      });
                                    },
                                  ),
                                  ActionChip(
                                    label: const Text('⏰ 次日凌晨 02:00', style: TextStyle(fontSize: 11)),
                                    visualDensity: VisualDensity.compact,
                                    onPressed: () {
                                      final n = DateTime.now().add(const Duration(days: 1));
                                      setSheetState(() {
                                        scheduledDate = DateTime(n.year, n.month, n.day, 2, 0);
                                      });
                                    },
                                  ),
                                  ActionChip(
                                    label: const Text('🌅 次日清晨 05:00', style: TextStyle(fontSize: 11)),
                                    visualDensity: VisualDensity.compact,
                                    onPressed: () {
                                      final n = DateTime.now().add(const Duration(days: 1));
                                      setSheetState(() {
                                        scheduledDate = DateTime(n.year, n.month, n.day, 5, 0);
                                      });
                                    },
                                  ),
                                ],
                              ),
                            ],
                          ),
                        ),
                      ],

                      const SizedBox(height: 20),
                      // Pacer 微批步态控制调节
                      Container(
                        padding: const EdgeInsets.all(14),
                        decoration: BoxDecoration(
                          color: const Color(0xFFF0FDF4),
                          borderRadius: BorderRadius.circular(10),
                          border: Border.all(color: const Color(0xFFBBF7D0)),
                        ),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            const Row(
                              children: [
                                Icon(Icons.tune, color: Color(0xFF10B981), size: 18),
                                SizedBox(width: 6),
                                Text(
                                  '2. 稳骥 Pacer 微批步态安全控速 (零抖动引擎)',
                                  style: TextStyle(fontSize: 13, fontWeight: FontWeight.bold, color: Color(0xFF065F46)),
                                ),
                              ],
                            ),
                            const SizedBox(height: 12),
                            // 单批切片大小
                            Row(
                              mainAxisAlignment: MainAxisAlignment.spaceBetween,
                              children: [
                                const Text('单批数据量 (Rows/Batch):', style: TextStyle(fontSize: 12, color: Color(0xFF374151))),
                                Text('$batchSize 行/批', style: const TextStyle(fontSize: 13, fontWeight: FontWeight.bold, color: PacerTheme.primaryBlue)),
                              ],
                            ),
                            Slider(
                              value: batchSize.toDouble(),
                              min: 500,
                              max: 5000,
                              divisions: 9,
                              activeColor: const Color(0xFF10B981),
                              onChanged: (val) {
                                setSheetState(() => batchSize = val.toInt());
                              },
                            ),

                            // 微批休眠间隔
                            Row(
                              mainAxisAlignment: MainAxisAlignment.spaceBetween,
                              children: [
                                const Text('批次休眠间隔 (Interval):', style: TextStyle(fontSize: 12, color: Color(0xFF374151))),
                                Text('$intervalMs ms', style: const TextStyle(fontSize: 13, fontWeight: FontWeight.bold, color: PacerTheme.primaryBlue)),
                              ],
                            ),
                            Slider(
                              value: intervalMs.toDouble(),
                              min: 10,
                              max: 100,
                              divisions: 9,
                              activeColor: const Color(0xFF10B981),
                              onChanged: (val) {
                                setSheetState(() => intervalMs = val.toInt());
                              },
                            ),

                            const Divider(color: Color(0xFFBBF7D0), height: 16),
                            Text(
                              '💡 步态推演预估: 吞吐约 ${(batchSize / (intervalMs + 5) * 1000).toInt()} 行/秒，锁持有耗时 < 3ms，主从复制延迟增量 ~0ms。',
                              style: const TextStyle(fontSize: 11, color: Color(0xFF047857)),
                            ),
                          ],
                        ),
                      ),

                      const SizedBox(height: 20),
                      // DBA 审批意见
                      const Text(
                        '3. 审批决策批注',
                        style: TextStyle(fontSize: 13, fontWeight: FontWeight.bold, color: Color(0xFF334155)),
                      ),
                      const SizedBox(height: 8),
                      TextField(
                        controller: commentController,
                        maxLines: 2,
                        decoration: const InputDecoration(
                          hintText: '输入审批批注...',
                          hintStyle: TextStyle(fontSize: 12),
                        ),
                      ),
                    ],
                  ),
                ),

                const SizedBox(height: 12),
                ElevatedButton.icon(
                  onPressed: () async {
                    Navigator.of(ctx).pop();
                    // 1. 触发指纹/面容生物免密加签核验
                    final bioPassed = await AuthService().loginWithBiometrics();
                    if (!bioPassed) {
                      if (mounted) {
                        ScaffoldMessenger.of(context).showSnackBar(
                          const SnackBar(content: Text('生物特征加签未通过，已取消审批操作')),
                        );
                      }
                      return;
                    }

                    final formattedDate = formatDateTime(scheduledDate);
                    final success = await TicketService().approveTicket(
                      ticketId: widget.ticket.id,
                      executionMode: executionMode,
                      scheduledTime: executionMode == 'SCHEDULED' ? formattedDate : null,
                      batchSize: batchSize,
                      intervalMs: intervalMs,
                      comment: commentController.text.trim(),
                    );
                    if (!mounted) return;
                    if (success) {
                      ScaffoldMessenger.of(context).showSnackBar(
                        const SnackBar(
                          content: Text('✓ 指纹/面容加签成功！工单已批准，微批步态已同步集群'),
                          backgroundColor: Color(0xFF10B981),
                          duration: Duration(seconds: 3),
                        ),
                      );
                      Navigator.of(context).pop();
                    }
                  },
                  icon: const Icon(Icons.fingerprint, size: 20),
                  label: const Center(
                    child: Text('指纹 / 面容免密加签并批准', style: TextStyle(fontSize: 15, fontWeight: FontWeight.bold)),
                  ),
                ),
              ],
            ),
          );
        },
      ),
    );
  }

  Widget _buildModeOption({
    required String label,
    required String value,
    required IconData icon,
    required bool selected,
    required VoidCallback onTap,
  }) {
    return Expanded(
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(8),
        child: Container(
          padding: const EdgeInsets.symmetric(vertical: 10, horizontal: 6),
          decoration: BoxDecoration(
            color: selected ? PacerTheme.primaryBlue.withOpacity(0.08) : const Color(0xFFF8FAFC),
            borderRadius: BorderRadius.circular(8),
            border: Border.all(
              color: selected ? PacerTheme.primaryBlue : const Color(0xFFE2E8F0),
              width: selected ? 1.5 : 1,
            ),
          ),
          child: Column(
            children: [
              Icon(icon, size: 20, color: selected ? PacerTheme.primaryBlue : const Color(0xFF64748B)),
              const SizedBox(height: 4),
              Text(
                label,
                textAlign: TextAlign.center,
                style: TextStyle(
                  fontSize: 11,
                  fontWeight: selected ? FontWeight.bold : FontWeight.normal,
                  color: selected ? PacerTheme.primaryBlue : const Color(0xFF475569),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final ticket = widget.ticket;

    return Scaffold(
      appBar: AppBar(
        title: Text(ticket.ticketNo),
        actions: [
          IconButton(
            icon: const Icon(Icons.share_outlined),
            onPressed: () {
              Clipboard.setData(ClipboardData(text: '稳骥工单 ${ticket.ticketNo} - ${ticket.title}'));
              ScaffoldMessenger.of(context).showSnackBar(
                const SnackBar(content: Text('工单摘要已复制至剪贴板')),
              );
            },
          ),
        ],
      ),
      body: _isLoading
          ? const Center(child: CircularProgressIndicator())
          : Column(
              children: [
                // 1. 顶部基础信息与 AST 风控得分面板
                Container(
                  color: Colors.white,
                  padding: const EdgeInsets.all(16),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        children: [
                          Container(
                            padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                            decoration: BoxDecoration(
                              color: PacerTheme.primaryBlue.withOpacity(0.1),
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
                          Expanded(
                            child: Text(
                              ticket.title,
                              style: const TextStyle(
                                fontSize: 16,
                                fontWeight: FontWeight.bold,
                                color: Color(0xFF0F172A),
                              ),
                            ),
                          ),
                        ],
                      ),
                      const SizedBox(height: 12),

                      // 目标数据库
                      Row(
                        children: [
                          const Icon(Icons.dns, size: 14, color: Color(0xFF64748B)),
                          const SizedBox(width: 6),
                          Text(
                            '${ticket.instanceName} / ${ticket.dbName}',
                            style: const TextStyle(fontSize: 13, color: Color(0xFF334155), fontWeight: FontWeight.w500),
                          ),
                        ],
                      ),
                      const SizedBox(height: 12),

                      // AST 风控预检条
                      Container(
                        padding: const EdgeInsets.all(12),
                        decoration: BoxDecoration(
                          color: ticket.riskColor.withOpacity(0.08),
                          borderRadius: BorderRadius.circular(8),
                          border: Border.all(color: ticket.riskColor.withOpacity(0.3)),
                        ),
                        child: Row(
                          children: [
                            Container(
                              padding: const EdgeInsets.all(8),
                              decoration: BoxDecoration(
                                color: ticket.riskColor.withOpacity(0.15),
                                shape: BoxShape.circle,
                              ),
                              child: Icon(Icons.shield, color: ticket.riskColor, size: 24),
                            ),
                            const SizedBox(width: 12),
                            Expanded(
                              child: Column(
                                crossAxisAlignment: CrossAxisAlignment.start,
                                children: [
                                  Row(
                                    children: [
                                      Text(
                                        'AST 风控评分: ${ticket.riskScore} / 100',
                                        style: TextStyle(
                                          fontSize: 13,
                                          fontWeight: FontWeight.bold,
                                          color: ticket.riskColor,
                                        ),
                                      ),
                                      const SizedBox(width: 8),
                                      Text(
                                        ticket.riskLabel,
                                        style: TextStyle(
                                          fontSize: 11,
                                          fontWeight: FontWeight.w600,
                                          color: ticket.riskColor,
                                        ),
                                      ),
                                    ],
                                  ),
                                  const SizedBox(height: 2),
                                  Text(
                                    'Dry-Run 推演: 影响预估 ${ticket.expectedRows} 行 | 语法合规通过',
                                    style: const TextStyle(fontSize: 11, color: Color(0xFF475569)),
                                  ),
                                ],
                              ),
                            ),
                          ],
                        ),
                      ),
                    ],
                  ),
                ),

                // 2. Tab 切换
                Container(
                  color: Colors.white,
                  child: TabBar(
                    controller: _tabController,
                    labelColor: PacerTheme.primaryBlue,
                    unselectedLabelColor: const Color(0xFF64748B),
                    indicatorColor: PacerTheme.primaryBlue,
                    tabs: const [
                      Tab(text: 'SQL 变更脚本'),
                      Tab(text: '逆向回滚脚本'),
                      Tab(text: '风控与审批轨迹'),
                    ],
                  ),
                ),
                const Divider(height: 1, color: Color(0xFFE2E8F0)),

                // 3. Tab 内容
                Expanded(
                  child: TabBarView(
                    controller: _tabController,
                    children: [
                      // SQL 变更脚本
                      _buildSqlViewer(_detail?.fullSql ?? ticket.sqlPreview, 'SQL 变更'),

                      // 逆向回滚脚本
                      _buildSqlViewer(
                        _detail?.rollbackSql ?? '-- 未配置自动回滚脚本',
                        '回滚脚本',
                      ),

                      // 风控规则与审批流转
                      _buildAuditAndTimelineView(),
                    ],
                  ),
                ),

                // 4. 底部审批操作栏
                if (ticket.status == TicketStatus.pending)
                  Container(
                    padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
                    decoration: BoxDecoration(
                      color: Colors.white,
                      boxShadow: [
                        BoxShadow(
                          color: Colors.black.withOpacity(0.06),
                          blurRadius: 8,
                          offset: const Offset(0, -2),
                        ),
                      ],
                    ),
                    child: SafeArea(
                      child: Row(
                        children: [
                          OutlinedButton(
                            onPressed: _showUrgeDialog,
                            style: OutlinedButton.styleFrom(
                              side: const BorderSide(color: Color(0xFFCBD5E1)),
                              padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
                            ),
                            child: const Text('催办', style: TextStyle(color: Color(0xFF475569))),
                          ),
                          const SizedBox(width: 8),
                          OutlinedButton(
                            onPressed: _showRejectDialog,
                            style: OutlinedButton.styleFrom(
                              side: const BorderSide(color: Color(0xFFFCA5A5)),
                              padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
                            ),
                            child: const Text('驳回', style: TextStyle(color: Color(0xFFEF4444))),
                          ),
                          const SizedBox(width: 8),
                          Expanded(
                            child: ElevatedButton.icon(
                              onPressed: _showApprovalSheet,
                              icon: const Icon(Icons.check_circle_outline, size: 18),
                              label: const Text('审批通过 / 配置步态'),
                            ),
                          ),
                        ],
                      ),
                    ),
                  ),
              ],
            ),
    );
  }

  Widget _buildSqlViewer(String sqlText, String typeName) {
    return Container(
      color: const Color(0xFF0F172A),
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              const Icon(Icons.code, color: Color(0xFF38BDF8), size: 16),
              const SizedBox(width: 6),
              Text(
                '$typeName (只读推演视图)',
                style: const TextStyle(color: Color(0xFF94A3B8), fontSize: 12, fontWeight: FontWeight.bold),
              ),
              const Spacer(),
              InkWell(
                onTap: () {
                  Clipboard.setData(ClipboardData(text: sqlText));
                  ScaffoldMessenger.of(context).showSnackBar(
                    SnackBar(content: Text('$typeName 已复制到剪贴板')),
                  );
                },
                child: const Row(
                  children: [
                    Icon(Icons.copy, size: 14, color: Color(0xFF38BDF8)),
                    SizedBox(width: 4),
                    Text('复制文本', style: TextStyle(color: Color(0xFF38BDF8), fontSize: 12)),
                  ],
                ),
              ),
            ],
          ),
          const SizedBox(height: 12),
          const Divider(color: Color(0xFF334155), height: 1),
          const SizedBox(height: 12),
          Expanded(
            child: SingleChildScrollView(
              child: SelectableText(
                sqlText,
                style: const TextStyle(
                  fontFamily: 'monospace',
                  color: Color(0xFFE2E8F0),
                  fontSize: 13,
                  height: 1.5,
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildAuditAndTimelineView() {
    final rules = _detail?.astRiskRules ?? [];
    final suggestions = _detail?.dbaSuggestions ?? [];
    final steps = _detail?.timeline ?? [];
    final isHighRisk = widget.ticket.riskLevel == RiskLevel.high ||
        widget.ticket.changeType == 'DDL' ||
        widget.ticket.expectedRows > 1000;

    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        // 0. BPMN 2.0 智能条件分支全景看板 (移动端 5 阶段卡片)
        _buildMobileBpmnFlowCard(isHighRisk),
        const SizedBox(height: 18),

        // AST 命中的风控规则
        const Text('AST 语义扫描命中规则', style: TextStyle(fontSize: 14, fontWeight: FontWeight.bold, color: Color(0xFF0F172A))),
        const SizedBox(height: 8),
        ...rules.map((rule) => Container(
              margin: const EdgeInsets.only(bottom: 8),
              padding: const EdgeInsets.all(10),
              decoration: BoxDecoration(
                color: const Color(0xFFFEF2F2),
                borderRadius: BorderRadius.circular(6),
                border: Border.all(color: const Color(0xFFFECACA)),
              ),
              child: Text(rule, style: const TextStyle(fontSize: 12, color: Color(0xFF991B1B), height: 1.4)),
            )),

        const SizedBox(height: 16),
        // DBA 决策建议
        const Text('Pacer 智能决策建议', style: TextStyle(fontSize: 14, fontWeight: FontWeight.bold, color: Color(0xFF0F172A))),
        const SizedBox(height: 8),
        ...suggestions.map((sug) => Container(
              margin: const EdgeInsets.only(bottom: 8),
              padding: const EdgeInsets.all(10),
              decoration: BoxDecoration(
                color: const Color(0xFFF0FDF4),
                borderRadius: BorderRadius.circular(6),
                border: Border.all(color: const Color(0xFFBBF7D0)),
              ),
              child: Text(sug, style: const TextStyle(fontSize: 12, color: Color(0xFF166534), height: 1.4)),
            )),

        const SizedBox(height: 16),
        // 审批轨迹 Timeline
        const Text('工单审批与流转轨迹', style: TextStyle(fontSize: 14, fontWeight: FontWeight.bold, color: Color(0xFF0F172A))),
        const SizedBox(height: 12),
        ...steps.map((step) => Padding(
              padding: const EdgeInsets.only(bottom: 16),
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Column(
                    children: [
                      Container(
                        width: 12,
                        height: 12,
                        decoration: const BoxDecoration(
                          shape: BoxShape.circle,
                          color: PacerTheme.accentCyan,
                        ),
                      ),
                      Container(
                        width: 2,
                        height: 40,
                        color: const Color(0xFFCBD5E1),
                      ),
                    ],
                  ),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Row(
                          children: [
                            Text(step.nodeName, style: const TextStyle(fontSize: 13, fontWeight: FontWeight.bold, color: Color(0xFF1E293B))),
                            const Spacer(),
                            Text(step.time, style: const TextStyle(fontSize: 11, color: Color(0xFF94A3B8))),
                          ],
                        ),
                        const SizedBox(height: 2),
                        Text('经办人: ${step.operator} · 状态: ${step.status}', style: const TextStyle(fontSize: 12, color: Color(0xFF64748B))),
                        if (step.comment != null) ...[
                          const SizedBox(height: 4),
                          Text('意见: ${step.comment}', style: const TextStyle(fontSize: 12, color: Color(0xFF334155))),
                        ],
                      ],
                    ),
                  ),
                ],
              ),
            )),
      ],
    );
  }

  Widget _buildMobileBpmnFlowCard(bool isHighRisk) {
    return Container(
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        color: Colors.white,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: const Color(0xFFCBD5E1)),
        boxShadow: [
          BoxShadow(
            color: Colors.black.withValues(alpha: 0.03),
            blurRadius: 8,
            offset: const Offset(0, 2),
          ),
        ],
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Container(
                padding: const EdgeInsets.all(5),
                decoration: BoxDecoration(
                  color: const Color(0xFFEFF6FF),
                  borderRadius: BorderRadius.circular(6),
                ),
                child: const Icon(Icons.account_tree_outlined, size: 16, color: Color(0xFF2563EB)),
              ),
              const SizedBox(width: 8),
              const Expanded(
                child: Text(
                  'BPMN 2.0 智能条件分支流转图',
                  style: TextStyle(fontSize: 13.5, fontWeight: FontWeight.bold, color: Color(0xFF0F172A)),
                ),
              ),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
                decoration: BoxDecoration(
                  color: isHighRisk ? const Color(0xFFFEF2F2) : const Color(0xFFEFF6FF),
                  borderRadius: BorderRadius.circular(12),
                  border: Border.all(
                    color: isHighRisk ? const Color(0xFFFCA5A5) : const Color(0xFF93C5FD),
                  ),
                ),
                child: Text(
                  isHighRisk ? '🔥 命中左分支 (DBA复核)' : '⚡ 命中右分支 (组长初审)',
                  style: TextStyle(
                    fontSize: 10.5,
                    fontWeight: FontWeight.bold,
                    color: isHighRisk ? const Color(0xFFDC2626) : const Color(0xFF1D4ED8),
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),
          Container(
            width: double.infinity,
            padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
            decoration: BoxDecoration(
              color: const Color(0xFFFFFBEB),
              borderRadius: BorderRadius.circular(6),
              border: Border.all(color: const Color(0xFFFDE68A)),
            ),
            child: Text(
              'SpEL 排他网关: #{affectRows > 1000 || hasDdl == true} (当前预估 ${widget.ticket.expectedRows} 行)',
              style: const TextStyle(fontSize: 11, fontFamily: 'monospace', color: Color(0xFFB45309)),
            ),
          ),
          const SizedBox(height: 12),
          // 5阶段节点流转展示
          Row(
            children: [
              _buildStagePill('01 发起', '已提交', const Color(0xFF10B981), true),
              _buildStageArrow(),
              _buildStagePill('02 网关', isHighRisk ? 'TRUE' : 'FALSE', const Color(0xFFD97706), true),
              _buildStageArrow(),
              Expanded(
                child: Column(
                  children: [
                    Container(
                      width: double.infinity,
                      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
                      decoration: BoxDecoration(
                        color: isHighRisk ? const Color(0xFFFEF2F2) : const Color(0xFFF8FAFC),
                        borderRadius: BorderRadius.circular(8),
                        border: Border.all(
                          color: isHighRisk ? const Color(0xFFDC2626) : const Color(0xFFCBD5E1),
                          width: isHighRisk ? 1.5 : 1,
                        ),
                      ),
                      child: Text(
                        '🔥 核心DBA安全复核',
                        textAlign: TextAlign.center,
                        style: TextStyle(
                          fontSize: 10.5,
                          fontWeight: isHighRisk ? FontWeight.bold : FontWeight.normal,
                          color: isHighRisk ? const Color(0xFFB91C1C) : const Color(0xFF94A3B8),
                        ),
                      ),
                    ),
                    const SizedBox(height: 4),
                    Container(
                      width: double.infinity,
                      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
                      decoration: BoxDecoration(
                        color: !isHighRisk ? const Color(0xFFEFF6FF) : const Color(0xFFF8FAFC),
                        borderRadius: BorderRadius.circular(8),
                        border: Border.all(
                          color: !isHighRisk ? const Color(0xFF2563EB) : const Color(0xFFCBD5E1),
                          width: !isHighRisk ? 1.5 : 1,
                        ),
                      ),
                      child: Text(
                        '⚡ 开发组长初审',
                        textAlign: TextAlign.center,
                        style: TextStyle(
                          fontSize: 10.5,
                          fontWeight: !isHighRisk ? FontWeight.bold : FontWeight.normal,
                          color: !isHighRisk ? const Color(0xFF1D4ED8) : const Color(0xFF94A3B8),
                        ),
                      ),
                    ),
                  ],
                ),
              ),
              _buildStageArrow(),
              _buildStagePill(
                '04 执行',
                widget.ticket.status == TicketStatus.executed ? '已完成' : 'JDBC流式',
                const Color(0xFF059669),
                widget.ticket.status == TicketStatus.executed || widget.ticket.status == TicketStatus.running,
              ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildStagePill(String step, String label, Color color, bool active) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 8),
      decoration: BoxDecoration(
        color: active ? color.withValues(alpha: 0.1) : const Color(0xFFF8FAFC),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: active ? color : const Color(0xFFCBD5E1)),
      ),
      child: Column(
        children: [
          Text(step, style: TextStyle(fontSize: 9.5, fontWeight: FontWeight.bold, color: active ? color : const Color(0xFF64748B))),
          const SizedBox(height: 2),
          Text(label, style: TextStyle(fontSize: 10.5, fontWeight: FontWeight.w600, color: active ? const Color(0xFF0F172A) : const Color(0xFF94A3B8))),
        ],
      ),
    );
  }

  Widget _buildStageArrow() {
    return const Padding(
      padding: EdgeInsets.symmetric(horizontal: 3),
      child: Icon(Icons.arrow_forward, size: 13, color: Color(0xFF94A3B8)),
    );
  }
}
