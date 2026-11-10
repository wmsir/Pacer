import 'package:flutter/material.dart';
import '../config/app_config.dart';
import '../services/api_service.dart';
import '../services/auth_service.dart';
import '../theme/pacer_theme.dart';
import 'home_navigation.dart';
import 'network_diagnostics_screen.dart';
import 'qr_scanner_screen.dart';

class LoginScreen extends StatefulWidget {
  final bool autoBiometric;
  const LoginScreen({super.key, this.autoBiometric = false});

  @override
  State<LoginScreen> createState() => _LoginScreenState();
}

class _LoginScreenState extends State<LoginScreen> with SingleTickerProviderStateMixin {
  late TabController _tabController;
  final TextEditingController _accountController = TextEditingController(text: 'dba_admin');
  final TextEditingController _passwordController = TextEditingController(text: 'Pacer@2026');
  final TextEditingController _phoneController = TextEditingController(text: '13800138000');
  final TextEditingController _smsController = TextEditingController(text: '8888');

  bool _obscurePassword = true;
  bool _isLoading = false;

  @override
  void initState() {
    super.initState();
    _tabController = TabController(length: 2, vsync: this);
    if (widget.autoBiometric) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        _handleBiometricLogin(isAutomatic: true);
      });
    }
  }

  @override
  void dispose() {
    _tabController.dispose();
    _accountController.dispose();
    _passwordController.dispose();
    _phoneController.dispose();
    _smsController.dispose();
    super.dispose();
  }

  Future<void> _handleAccountLogin() async {
    final account = _accountController.text.trim();
    final pwd = _passwordController.text.trim();
    if (account.isEmpty || pwd.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('请输入运维审批账号与密钥')),
      );
      return;
    }

    setState(() => _isLoading = true);
    final success = await AuthService().login(account, pwd);
    setState(() => _isLoading = false);

    if (!mounted) return;
    if (success) {
      Navigator.of(context).pushReplacement(
        MaterialPageRoute(builder: (_) => const HomeNavigation()),
      );
    } else {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('登录校验失败，请检查账号密码或网络连接')),
      );
    }
  }

  Future<void> _handleSmsLogin() async {
    final phone = _phoneController.text.trim();
    final code = _smsController.text.trim();
    if (phone.isEmpty || code.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('请输入手机号与短信验证码')),
      );
      return;
    }

    setState(() => _isLoading = true);
    final success = await AuthService().loginWithSms(phone, code);
    setState(() => _isLoading = false);

    if (!mounted) return;
    if (success) {
      Navigator.of(context).pushReplacement(
        MaterialPageRoute(builder: (_) => const HomeNavigation()),
      );
    } else {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('验证码错误或已失效')),
      );
    }
  }

  Future<void> _handleDemoLogin() async {
    setState(() => _isLoading = true);
    await AuthService().loginAsDbaDemo();
    setState(() => _isLoading = false);

    if (!mounted) return;
    Navigator.of(context).pushReplacement(
      MaterialPageRoute(builder: (_) => const HomeNavigation()),
    );
  }

  Future<void> _handleBiometricLogin({bool isAutomatic = false}) async {
    setState(() => _isLoading = true);
    final success = await AuthService().loginWithBiometrics();
    setState(() => _isLoading = false);

    if (!mounted) return;
    if (success) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
          content: Text('指纹/面容生物认证成功，已快速进入审批控制台'),
          backgroundColor: Color(0xFF10B981),
          duration: Duration(seconds: 2),
        ),
      );
      Navigator.of(context).pushReplacement(
        MaterialPageRoute(builder: (_) => const HomeNavigation()),
      );
    } else if (!isAutomatic) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('生物特征认证未通过，请使用账号密码或扫码登录')),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final currentGateway = ApiService().currentGateway;

    return Scaffold(
      backgroundColor: Colors.white,
      appBar: AppBar(
        backgroundColor: Colors.transparent,
        elevation: 0,
        actions: [
          IconButton(
            icon: const Icon(Icons.qr_code_scanner, color: Color(0xFF0F172A), size: 22),
            tooltip: '扫码登录 / 授权',
            onPressed: () {
              Navigator.of(context).push(
                MaterialPageRoute(builder: (_) => const QrScannerScreen()),
              );
            },
          ),
          TextButton.icon(
            onPressed: () {
              Navigator.of(context).push(
                MaterialPageRoute(builder: (_) => const NetworkDiagnosticsScreen()),
              );
            },
            icon: const Icon(Icons.wifi_tethering, size: 16, color: PacerTheme.accentCyan),
            label: Text(
              currentGateway.carrierTag,
              style: const TextStyle(fontSize: 12, color: PacerTheme.accentCyan),
            ),
          ),
          const SizedBox(width: 8),
        ],
      ),
      body: SafeArea(
        child: SingleChildScrollView(
          padding: const EdgeInsets.symmetric(horizontal: 24.0, vertical: 8.0),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              // 品牌头图
              Row(
                children: [
                  ClipRRect(
                    borderRadius: BorderRadius.circular(12),
                    child: SizedBox(
                      width: 48,
                      height: 48,
                      child: Image.asset(
                        'assets/icon/app_icon.png',
                        fit: BoxFit.cover,
                      ),
                    ),
                  ),
                  const SizedBox(width: 14),
                  const Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        '稳骥 PacerSQL',
                        style: TextStyle(
                          fontSize: 20,
                          fontWeight: FontWeight.bold,
                          color: Color(0xFF0F172A),
                        ),
                      ),
                      Text(
                        '移动审批决策客户端 · 企业级数据库控制中心',
                        style: TextStyle(
                          fontSize: 12,
                          color: Color(0xFF64748B),
                        ),
                      ),
                    ],
                  ),
                ],
              ),
              const SizedBox(height: 28),

              // 极速免密双通道：指纹面容 + 扫码登录 (打开即用)
              Row(
                children: [
                  Expanded(
                    child: InkWell(
                      onTap: _isLoading ? null : _handleBiometricLogin,
                      borderRadius: BorderRadius.circular(12),
                      child: Container(
                        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 14),
                        decoration: BoxDecoration(
                          gradient: const LinearGradient(
                            colors: [Color(0xFF0F172A), Color(0xFF1E3A8A)],
                            begin: Alignment.topLeft,
                            end: Alignment.bottomRight,
                          ),
                          borderRadius: BorderRadius.circular(12),
                          boxShadow: [
                            BoxShadow(
                              color: const Color(0xFF1E3A8A).withValues(alpha: 0.25),
                              blurRadius: 10,
                              offset: const Offset(0, 4),
                            ),
                          ],
                        ),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Row(
                              mainAxisAlignment: MainAxisAlignment.spaceBetween,
                              children: [
                                Container(
                                  padding: const EdgeInsets.all(7),
                                  decoration: BoxDecoration(
                                    color: const Color(0xFF0284C7).withValues(alpha: 0.25),
                                    shape: BoxShape.circle,
                                  ),
                                  child: const Icon(Icons.fingerprint, color: Color(0xFF38BDF8), size: 22),
                                ),
                                const Icon(Icons.arrow_forward_ios, color: Colors.white38, size: 12),
                              ],
                            ),
                            const SizedBox(height: 10),
                            const Text(
                              '指纹 / 面容',
                              style: TextStyle(color: Colors.white, fontSize: 14, fontWeight: FontWeight.bold),
                            ),
                            const SizedBox(height: 2),
                            const Text(
                              '生物轻触极速进台',
                              style: TextStyle(color: Color(0xFF93C5FD), fontSize: 11),
                            ),
                          ],
                        ),
                      ),
                    ),
                  ),
                  const SizedBox(width: 12),
                  Expanded(
                    child: InkWell(
                      onTap: () {
                        Navigator.of(context).push(
                          MaterialPageRoute(builder: (_) => const QrScannerScreen()),
                        );
                      },
                      borderRadius: BorderRadius.circular(12),
                      child: Container(
                        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 14),
                        decoration: BoxDecoration(
                          gradient: const LinearGradient(
                            colors: [Color(0xFF0B192C), Color(0xFF0369A1)],
                            begin: Alignment.topLeft,
                            end: Alignment.bottomRight,
                          ),
                          borderRadius: BorderRadius.circular(12),
                          boxShadow: [
                            BoxShadow(
                              color: const Color(0xFF0284C7).withValues(alpha: 0.25),
                              blurRadius: 10,
                              offset: const Offset(0, 4),
                            ),
                          ],
                        ),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Row(
                              mainAxisAlignment: MainAxisAlignment.spaceBetween,
                              children: [
                                Container(
                                  padding: const EdgeInsets.all(7),
                                  decoration: BoxDecoration(
                                    color: const Color(0xFF38BDF8).withValues(alpha: 0.25),
                                    shape: BoxShape.circle,
                                  ),
                                  child: const Icon(Icons.qr_code_scanner, color: Color(0xFF38BDF8), size: 22),
                                ),
                                const Icon(Icons.arrow_forward_ios, color: Colors.white38, size: 12),
                              ],
                            ),
                            const SizedBox(height: 10),
                            const Text(
                              '扫码极速登录',
                              style: TextStyle(color: Colors.white, fontSize: 14, fontWeight: FontWeight.bold),
                            ),
                            const SizedBox(height: 2),
                            const Text(
                              '扫描 Web 控制台码',
                              style: TextStyle(color: Color(0xFFBAE6FD), fontSize: 11),
                            ),
                          ],
                        ),
                      ),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 20),

              Row(
                children: [
                  const Expanded(child: Divider(color: Color(0xFFE2E8F0))),
                  Padding(
                    padding: const EdgeInsets.symmetric(horizontal: 12),
                    child: Text(
                      '或使用常规方式登录',
                      style: TextStyle(fontSize: 12, color: Colors.grey.shade500),
                    ),
                  ),
                  const Expanded(child: Divider(color: Color(0xFFE2E8F0))),
                ],
              ),
              const SizedBox(height: 16),

              // 登录 Tab
              Container(
                decoration: BoxDecoration(
                  color: const Color(0xFFF1F5F9),
                  borderRadius: BorderRadius.circular(8),
                ),
                child: TabBar(
                  controller: _tabController,
                  indicatorSize: TabBarIndicatorSize.tab,
                  indicator: BoxDecoration(
                    color: Colors.white,
                    borderRadius: BorderRadius.circular(8),
                    boxShadow: [
                      BoxShadow(
                        color: Colors.black.withValues(alpha: 0.05),
                        blurRadius: 4,
                        offset: const Offset(0, 1),
                      ),
                    ],
                  ),
                  labelColor: PacerTheme.primaryBlue,
                  unselectedLabelColor: const Color(0xFF64748B),
                  labelStyle: const TextStyle(fontWeight: FontWeight.w600, fontSize: 13),
                  tabs: const [
                    Tab(text: '账号密码登录'),
                    Tab(text: '手机短信验证码'),
                  ],
                ),
              ),
              const SizedBox(height: 24),

              // 表单区域
              SizedBox(
                height: 200,
                child: TabBarView(
                  controller: _tabController,
                  children: [
                    // 账号密码表单
                    Column(
                      children: [
                        TextField(
                          controller: _accountController,
                          decoration: const InputDecoration(
                            labelText: 'DBA / 审批人账号',
                            prefixIcon: Icon(Icons.person_outline, size: 20),
                          ),
                        ),
                        const SizedBox(height: 14),
                        TextField(
                          controller: _passwordController,
                          obscureText: _obscurePassword,
                          decoration: InputDecoration(
                            labelText: '安全密码 / 动态口令',
                            prefixIcon: const Icon(Icons.lock_outline, size: 20),
                            suffixIcon: IconButton(
                              icon: Icon(
                                _obscurePassword ? Icons.visibility_off : Icons.visibility,
                                size: 18,
                              ),
                              onPressed: () => setState(() => _obscurePassword = !_obscurePassword),
                            ),
                          ),
                        ),
                      ],
                    ),

                    // 手机短信表单
                    Column(
                      children: [
                        TextField(
                          controller: _phoneController,
                          keyboardType: TextInputType.phone,
                          decoration: const InputDecoration(
                            labelText: '技术审批人手机号',
                            prefixIcon: Icon(Icons.phone_iphone, size: 20),
                          ),
                        ),
                        const SizedBox(height: 14),
                        Row(
                          children: [
                            Expanded(
                              child: TextField(
                                controller: _smsController,
                                keyboardType: TextInputType.number,
                                decoration: const InputDecoration(
                                  labelText: '6位短信验证码',
                                  prefixIcon: Icon(Icons.shield_outlined, size: 20),
                                ),
                              ),
                            ),
                            const SizedBox(width: 12),
                            OutlinedButton(
                              onPressed: () {
                                ScaffoldMessenger.of(context).showSnackBar(
                                  const SnackBar(content: Text('验证码已发送至审批专线：8888')),
                                );
                              },
                              style: OutlinedButton.styleFrom(
                                padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 15),
                                side: const BorderSide(color: PacerTheme.accentCyan),
                              ),
                              child: const Text('获取验证码', style: TextStyle(fontSize: 12)),
                            ),
                          ],
                        ),
                      ],
                    ),
                  ],
                ),
              ),

              // 登录按钮
              ElevatedButton(
                onPressed: _isLoading
                    ? null
                    : () {
                        if (_tabController.index == 0) {
                          _handleAccountLogin();
                        } else {
                          _handleSmsLogin();
                        }
                      },
                child: _isLoading
                    ? const SizedBox(
                        width: 18,
                        height: 18,
                        child: CircularProgressIndicator(color: Colors.white, strokeWidth: 2),
                      )
                    : const Text('登 录 稳 骥 审 批 端', style: TextStyle(fontSize: 15)),
              ),
              const SizedBox(height: 14),

              // 快速免密体验
              OutlinedButton.icon(
                onPressed: _isLoading ? null : _handleDemoLogin,
                style: OutlinedButton.styleFrom(
                  padding: const EdgeInsets.symmetric(vertical: 12),
                  side: const BorderSide(color: Color(0xFFCBD5E1)),
                ),
                icon: const Icon(Icons.rocket_launch, size: 18, color: PacerTheme.accentCyan),
                label: const Text(
                  '一键体验首席 DBA 审批角色 (演示环境)',
                  style: TextStyle(color: Color(0xFF1E293B), fontWeight: FontWeight.w600),
                ),
              ),

              const SizedBox(height: 28),
              // 风控与安全提示
              Container(
                padding: const EdgeInsets.all(14),
                decoration: BoxDecoration(
                  color: const Color(0xFFF8FAFC),
                  borderRadius: BorderRadius.circular(8),
                  border: Border.all(color: const Color(0xFFE2E8F0)),
                ),
                child: const Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      children: [
                        Icon(Icons.verified_user, size: 16, color: Color(0xFF10B981)),
                        SizedBox(width: 6),
                        Text(
                          '企业级数据库变更安全守护 · 99.9% 生产可用性',
                          style: TextStyle(fontSize: 12, fontWeight: FontWeight.bold, color: Color(0xFF334155)),
                        ),
                      ],
                    ),
                    SizedBox(height: 6),
                    Text(
                      '• 支持专线高可用与断网离线事务加签容灾保护。\n'
                      '• 所有 SQL 均经过 AST 语义语法树预检与 Pacer 微批步态安全流控。',
                      style: TextStyle(fontSize: 11, color: Color(0xFF64748B), height: 1.4),
                    ),
                  ],
                ),
              ),

              const SizedBox(height: 24),
              // 官网与技术支持展示
              Center(
                child: Column(
                  children: [
                    const Text(
                      '官方主站: ${AppConfig.officialWebsite}',
                      style: TextStyle(color: Color(0xFF64748B), fontSize: 12, fontWeight: FontWeight.w600),
                    ),
                    const SizedBox(height: 4),
                    Text(
                      '技术支持: ${AppConfig.officialEmail}',
                      style: const TextStyle(color: Color(0xFF94A3B8), fontSize: 11),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
