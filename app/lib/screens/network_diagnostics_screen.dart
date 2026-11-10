import 'package:flutter/material.dart';
import '../config/app_config.dart';
import '../services/api_service.dart';
import '../theme/pacer_theme.dart';

class NetworkDiagnosticsScreen extends StatefulWidget {
  const NetworkDiagnosticsScreen({super.key});

  @override
  State<NetworkDiagnosticsScreen> createState() => _NetworkDiagnosticsScreenState();
}

class _NetworkDiagnosticsScreenState extends State<NetworkDiagnosticsScreen> {
  List<GatewayLatency> _latencies = [];
  bool _isProbing = false;

  @override
  void initState() {
    super.initState();
    _runDiagnostic();
  }

  Future<void> _runDiagnostic() async {
    setState(() => _isProbing = true);
    final results = await ApiService().probeAllGateways();
    if (mounted) {
      setState(() {
        _latencies = results;
        _isProbing = false;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final current = ApiService().currentGateway;

    return Scaffold(
      appBar: AppBar(
        title: const Text('高可用专线与链路诊断'),
      ),
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // 当前活跃网关卡片
            Container(
              padding: const EdgeInsets.all(16),
              decoration: BoxDecoration(
                gradient: const LinearGradient(
                  colors: [PacerTheme.primaryBlue, Color(0xFF0F172A)],
                  begin: Alignment.topLeft,
                  end: Alignment.bottomRight,
                ),
                borderRadius: BorderRadius.circular(12),
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      const Icon(Icons.wifi_tethering, color: Color(0xFF38BDF8), size: 20),
                      const SizedBox(width: 8),
                      const Text(
                        '当前通信主通道',
                        style: TextStyle(color: Color(0xFF94A3B8), fontSize: 12),
                      ),
                      const Spacer(),
                      Container(
                        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
                        decoration: BoxDecoration(
                          color: const Color(0xFF10B981).withOpacity(0.2),
                          borderRadius: BorderRadius.circular(12),
                          border: Border.all(color: const Color(0xFF10B981)),
                        ),
                        child: const Row(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            Icon(Icons.check_circle, color: Color(0xFF10B981), size: 10),
                            SizedBox(width: 4),
                            Text('活跃中', style: TextStyle(color: Color(0xFF10B981), fontSize: 10, fontWeight: FontWeight.bold)),
                          ],
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 12),
                  Text(
                    current.name,
                    style: const TextStyle(color: Colors.white, fontSize: 16, fontWeight: FontWeight.bold),
                  ),
                  const SizedBox(height: 4),
                  Text(
                    '${current.carrierTag} · ${current.baseUrl}',
                    style: const TextStyle(color: Color(0xFF94A3B8), fontSize: 12),
                  ),
                ],
              ),
            ),
            const SizedBox(height: 20),

            // 测速与刷新按钮
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                const Text(
                  '多节点连通性与往返延迟 (RTT)',
                  style: TextStyle(fontSize: 14, fontWeight: FontWeight.bold, color: Color(0xFF1E293B)),
                ),
                ElevatedButton.icon(
                  onPressed: _isProbing ? null : _runDiagnostic,
                  style: ElevatedButton.styleFrom(
                    padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                  ),
                  icon: _isProbing
                      ? const SizedBox(
                          width: 12,
                          height: 12,
                          child: CircularProgressIndicator(strokeWidth: 2, color: Colors.white),
                        )
                      : const Icon(Icons.refresh, size: 14),
                  label: Text(_isProbing ? '测速中...' : '一键三网测速', style: const TextStyle(fontSize: 12)),
                ),
              ],
            ),
            const SizedBox(height: 12),

            // 节点列表
            ...AppConfig.gatewayNodes.map((node) {
              final isCurrent = node.baseUrl == current.baseUrl;
              final latObj = _latencies.firstWhere(
                (l) => l.node.baseUrl == node.baseUrl,
                orElse: () => GatewayLatency(node: node, latencyMs: null, isAvailable: false),
              );

              Color statusColor;
              String statusText;
              if (latObj.latencyMs == null) {
                statusColor = const Color(0xFF94A3B8);
                statusText = '未测速 / 离线';
              } else if (latObj.latencyMs! < 100) {
                statusColor = const Color(0xFF10B981);
                statusText = '${latObj.latencyMs} ms (极速)';
              } else if (latObj.latencyMs! < 300) {
                statusColor = const Color(0xFFF59E0B);
                statusText = '${latObj.latencyMs} ms (良好)';
              } else {
                statusColor = const Color(0xFFEF4444);
                statusText = '${latObj.latencyMs} ms (高延迟)';
              }

              return Card(
                elevation: 0,
                margin: const EdgeInsets.only(bottom: 12),
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(12),
                  side: BorderSide(
                    color: isCurrent ? PacerTheme.primaryBlue : const Color(0xFFE2E8F0),
                    width: isCurrent ? 1.5 : 1,
                  ),
                ),
                child: Padding(
                  padding: const EdgeInsets.all(14),
                  child: Row(
                    children: [
                      Container(
                        padding: const EdgeInsets.all(8),
                        decoration: BoxDecoration(
                          color: (isCurrent ? PacerTheme.primaryBlue : const Color(0xFF64748B)).withOpacity(0.1),
                          shape: BoxShape.circle,
                        ),
                        child: Icon(
                          Icons.router_outlined,
                          color: isCurrent ? PacerTheme.primaryBlue : const Color(0xFF64748B),
                          size: 20,
                        ),
                      ),
                      const SizedBox(width: 12),
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Row(
                              children: [
                                Text(
                                  node.name,
                                  style: const TextStyle(fontSize: 14, fontWeight: FontWeight.bold, color: Color(0xFF1E293B)),
                                ),
                                if (isCurrent) ...[
                                  const SizedBox(width: 6),
                                  Container(
                                    padding: const EdgeInsets.symmetric(horizontal: 5, vertical: 1),
                                    decoration: BoxDecoration(
                                      color: PacerTheme.primaryBlue,
                                      borderRadius: BorderRadius.circular(4),
                                    ),
                                    child: const Text('当前', style: TextStyle(color: Colors.white, fontSize: 9, fontWeight: FontWeight.bold)),
                                  ),
                                ],
                              ],
                            ),
                            const SizedBox(height: 2),
                            Text(
                              '${node.carrierTag} · ${node.baseUrl}',
                              style: const TextStyle(fontSize: 11, color: Color(0xFF64748B)),
                            ),
                          ],
                        ),
                      ),
                      Column(
                        crossAxisAlignment: CrossAxisAlignment.end,
                        children: [
                          Text(
                            statusText,
                            style: TextStyle(
                              fontSize: 12,
                              fontWeight: FontWeight.bold,
                              color: statusColor,
                            ),
                          ),
                          if (!isCurrent) ...[
                            const SizedBox(height: 4),
                            InkWell(
                              onTap: () {
                                ApiService().switchGateway(node);
                                setState(() {});
                                ScaffoldMessenger.of(context).showSnackBar(
                                  SnackBar(content: Text('已切换主专线网关至: ${node.name}')),
                                );
                              },
                              child: const Text(
                                '切换为主要',
                                style: TextStyle(fontSize: 11, color: PacerTheme.accentCyan, fontWeight: FontWeight.bold),
                              ),
                            ),
                          ],
                        ],
                      ),
                    ],
                  ),
                ),
              );
            }),

            const SizedBox(height: 16),
            // 三网容灾架构说明
            Container(
              padding: const EdgeInsets.all(14),
              decoration: BoxDecoration(
                color: const Color(0xFFF8FAFC),
                borderRadius: BorderRadius.circular(10),
                border: Border.all(color: const Color(0xFFE2E8F0)),
              ),
              child: const Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Icon(Icons.shield_outlined, size: 16, color: PacerTheme.primaryBlue),
                      SizedBox(width: 6),
                      Text('稳骥高可用全链路容灾策略', style: TextStyle(fontSize: 12, fontWeight: FontWeight.bold, color: Color(0xFF1E293B))),
                    ],
                  ),
                  SizedBox(height: 8),
                  Text(
                    '1. 【智能路由】默认通过生产核心集群专线通道直连，零延迟同步变更。\n'
                    '2. 【超时自动重试】单节点网络波动（如信号弱或临时抖动）时，客户端将在 3.5s 内平滑下沉至生产专线 IP。\n'
                    '3. 【双机房容灾】支持华北生产主节点与异地容灾备份节点秒级热备。\n'
                    '4. 【本地审批缓存】在全网断开极端场景下，自动启用本地签名缓存，恢复连通后自动批量过账。',
                    style: TextStyle(fontSize: 11, color: Color(0xFF475569), height: 1.5),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}
