import 'package:flutter/material.dart';
import 'package:shared_preferences/shared_preferences.dart';
import '../config/app_config.dart';
import '../services/auth_service.dart';
import '../theme/pacer_theme.dart';
import 'login_screen.dart';
import 'network_diagnostics_screen.dart';
import 'qr_scanner_screen.dart';

class ProfileScreen extends StatefulWidget {
  const ProfileScreen({super.key});

  @override
  State<ProfileScreen> createState() => _ProfileScreenState();
}

class _ProfileScreenState extends State<ProfileScreen> {
  bool _biometricEnabled = true;
  bool _pushNotifications = true;

  @override
  void initState() {
    super.initState();
    _loadPrefs();
  }

  Future<void> _loadPrefs() async {
    try {
      final sp = await SharedPreferences.getInstance();
      if (!mounted) return;
      setState(() {
        _biometricEnabled = sp.getBool('pacer_biometric_enabled') ?? true;
        _pushNotifications = sp.getBool('pacer_push_notifications') ?? true;
      });
    } catch (_) {}
  }

  Future<void> _savePref(String key, bool val) async {
    try {
      final sp = await SharedPreferences.getInstance();
      await sp.setBool(key, val);
    } catch (_) {}
  }

  @override
  Widget build(BuildContext context) {
    final user = AuthService().currentUser;

    return Scaffold(
      appBar: AppBar(
        title: const Text('我的运维账户'),
      ),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          // 用户资料名片
          Container(
            padding: const EdgeInsets.all(20),
            decoration: BoxDecoration(
              gradient: const LinearGradient(
                colors: [Color(0xFF1E3A8A), Color(0xFF0F172A)],
                begin: Alignment.topLeft,
                end: Alignment.bottomRight,
              ),
              borderRadius: BorderRadius.circular(16),
              boxShadow: [
                BoxShadow(
                  color: const Color(0xFF1E3A8A).withOpacity(0.2),
                  blurRadius: 12,
                  offset: const Offset(0, 4),
                ),
              ],
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    CircleAvatar(
                      radius: 28,
                      backgroundColor: PacerTheme.accentCyan,
                      child: Text(
                        (user?.nickname.isNotEmpty == true ? user!.nickname.substring(0, 1) : 'DBA'),
                        style: const TextStyle(fontSize: 22, fontWeight: FontWeight.bold, color: Colors.white),
                      ),
                    ),
                    const SizedBox(width: 14),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Row(
                            children: [
                              Text(
                                user?.nickname ?? '生产专家',
                                style: const TextStyle(color: Colors.white, fontSize: 17, fontWeight: FontWeight.bold),
                              ),
                              if (user?.isDemo == true) ...[
                                const SizedBox(width: 6),
                                Container(
                                  padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                                  decoration: BoxDecoration(
                                    color: const Color(0xFFF59E0B),
                                    borderRadius: BorderRadius.circular(4),
                                  ),
                                  child: const Text('DEMO 体验', style: TextStyle(color: Colors.white, fontSize: 10, fontWeight: FontWeight.bold)),
                                ),
                              ],
                            ],
                          ),
                          const SizedBox(height: 4),
                          Text(
                            user?.roleDesc ?? '首席生产数据库管理员',
                            style: const TextStyle(color: Color(0xFF93C5FD), fontSize: 12),
                          ),
                        ],
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 16),
                const Divider(color: Colors.white24, height: 1),
                const SizedBox(height: 12),
                Row(
                  children: [
                    const Icon(Icons.security, color: Color(0xFF38BDF8), size: 14),
                    const SizedBox(width: 6),
                    Expanded(
                      child: Text(
                        '授权数据范围: ${user?.dataScope ?? "全业务集群"}',
                        style: const TextStyle(color: Color(0xFFE2E8F0), fontSize: 12),
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                  ],
                ),
              ],
            ),
          ),
          const SizedBox(height: 20),

          // 安全与设置卡片
          _buildSectionHeader('安全风控与审批设置'),
          _buildSettingsGroup([
            SwitchListTile(
              title: const Text('指纹 / 面容快速签批', style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500)),
              subtitle: const Text('高危工单二次人脸或生物加签', style: TextStyle(fontSize: 11, color: Color(0xFF64748B))),
              value: _biometricEnabled,
              activeColor: PacerTheme.primaryBlue,
              onChanged: (val) {
                setState(() => _biometricEnabled = val);
                _savePref('pacer_biometric_enabled', val);
              },
            ),
            const Divider(height: 1, indent: 16),
            SwitchListTile(
              title: const Text('高危工单强提醒 (生产告警专线/钉钉)', style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500)),
              subtitle: const Text('收到待办后即时推送至移动审批端', style: TextStyle(fontSize: 11, color: Color(0xFF64748B))),
              value: _pushNotifications,
              activeColor: PacerTheme.primaryBlue,
              onChanged: (val) {
                setState(() => _pushNotifications = val);
                _savePref('pacer_push_notifications', val);
              },
            ),
          ]),
          const SizedBox(height: 16),

          // 专线网络与系统管理
          _buildSectionHeader('专线网络与授权管理'),
          _buildSettingsGroup([
            ListTile(
              leading: const Icon(Icons.qr_code_scanner, color: PacerTheme.primaryBlue, size: 20),
              title: const Text('扫码授权 Web 控制台登录', style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500)),
              subtitle: const Text('扫描桌面浏览器二维码快速授权', style: TextStyle(fontSize: 11, color: Color(0xFF64748B))),
              trailing: const Icon(Icons.chevron_right, size: 20, color: Color(0xFF94A3B8)),
              onTap: () {
                Navigator.of(context).push(
                  MaterialPageRoute(builder: (_) => const QrScannerScreen()),
                );
              },
            ),
            const Divider(height: 1, indent: 16),
            ListTile(
              leading: const Icon(Icons.wifi_tethering, color: PacerTheme.accentCyan, size: 20),
              title: const Text('高可用专线与链路诊断', style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500)),
              trailing: const Icon(Icons.chevron_right, size: 20, color: Color(0xFF94A3B8)),
              onTap: () {
                Navigator.of(context).push(
                  MaterialPageRoute(builder: (_) => const NetworkDiagnosticsScreen()),
                );
              },
            ),
            const Divider(height: 1, indent: 16),
            ListTile(
              leading: const Icon(Icons.cleaning_services_outlined, color: Color(0xFF64748B), size: 20),
              title: const Text('清理本地 AST 语法树离线缓存', style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500)),
              trailing: const Icon(Icons.chevron_right, size: 20, color: Color(0xFF94A3B8)),
              onTap: () {
                ScaffoldMessenger.of(context).showSnackBar(
                  const SnackBar(content: Text('已清除本地推演日志缓存 (已释放 12.8MB)')),
                );
              },
            ),
            const Divider(height: 1, indent: 16),
            ListTile(
              leading: const Icon(Icons.info_outline, color: Color(0xFF64748B), size: 20),
              title: const Text('关于稳骥 PacerSQL', style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500)),
              trailing: const Text(AppConfig.appVersion, style: TextStyle(fontSize: 12, color: Color(0xFF94A3B8))),
              onTap: _showAboutDialog,
            ),
          ]),
          const SizedBox(height: 24),

          // 退出登录按钮
          OutlinedButton(
            style: OutlinedButton.styleFrom(
              padding: const EdgeInsets.symmetric(vertical: 14),
              side: const BorderSide(color: Color(0xFFCBD5E1)),
            ),
            onPressed: () async {
              await AuthService().logout();
              if (!context.mounted) return;
              Navigator.of(context).pushAndRemoveUntil(
                MaterialPageRoute(builder: (_) => const LoginScreen()),
                (route) => false,
              );
            },
            child: const Text('退出登录', style: TextStyle(color: Color(0xFFEF4444), fontSize: 14, fontWeight: FontWeight.bold)),
          ),
          const SizedBox(height: 24),

          // 官方网站与支持
          Center(
            child: Column(
              children: [
                const Text(
                  '官方网站: ${AppConfig.officialWebsite}',
                  style: TextStyle(color: Color(0xFF64748B), fontSize: 12, fontWeight: FontWeight.w600),
                ),
                const SizedBox(height: 4),
                Text(
                  '官方技术专线: ${AppConfig.officialEmail}',
                  style: const TextStyle(color: Color(0xFF94A3B8), fontSize: 11),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildSectionHeader(String title) {
    return Padding(
      padding: const EdgeInsets.only(left: 4, bottom: 8),
      child: Text(
        title,
        style: const TextStyle(fontSize: 12, fontWeight: FontWeight.bold, color: Color(0xFF64748B)),
      ),
    );
  }

  Widget _buildSettingsGroup(List<Widget> children) {
    return Container(
      decoration: BoxDecoration(
        color: Colors.white,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: const Color(0xFFE2E8F0)),
      ),
      child: Column(children: children),
    );
  }

  void _showAboutDialog() {
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Row(
          children: [
            ClipRRect(
              borderRadius: BorderRadius.circular(8),
              child: SizedBox(
                width: 32,
                height: 32,
                child: Image.asset('assets/icon/app_icon.png', fit: BoxFit.cover),
              ),
            ),
            const SizedBox(width: 10),
            const Text('稳骥 PacerSQL', style: TextStyle(fontSize: 17, fontWeight: FontWeight.bold)),
          ],
        ),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Text(
              '稳骥 PacerSQL 移动审批端是面向企业级生产数据库变更的移动风控与流式微批步态控制中心。',
              style: TextStyle(fontSize: 13, color: Color(0xFF334155), height: 1.4),
            ),
            const SizedBox(height: 12),
            const Text('• 客户端版本: ${AppConfig.appVersion} (${AppConfig.buildNumber})', style: TextStyle(fontSize: 12, color: Color(0xFF64748B))),
            const Text('• 官方网站: ${AppConfig.officialWebsite}', style: TextStyle(fontSize: 12, color: Color(0xFF64748B))),
            const Text('• 联系邮箱: ${AppConfig.officialEmail}', style: TextStyle(fontSize: 12, color: Color(0xFF64748B))),
            const Text('• SLA 生产稳定性: 99.9%', style: TextStyle(fontSize: 12, color: Color(0xFF64748B))),
            const SizedBox(height: 10),
            const Text(
              '高可用专线自适应网络架构支持公网核心主站、直连节点及容灾专线自动降级与离线容灾。',
              style: TextStyle(fontSize: 11, color: Color(0xFF0284C7)),
            ),
          ],
        ),
        actions: [
          ElevatedButton(
            onPressed: () => Navigator.of(ctx).pop(),
            child: const Text('我知道了'),
          ),
        ],
      ),
    );
  }
}
