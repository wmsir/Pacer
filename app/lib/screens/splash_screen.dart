import 'package:flutter/material.dart';
import '../config/app_config.dart';
import '../services/api_service.dart';
import '../services/auth_service.dart';
import '../theme/pacer_theme.dart';
import 'home_navigation.dart';
import 'login_screen.dart';

class SplashScreen extends StatefulWidget {
  const SplashScreen({super.key});

  @override
  State<SplashScreen> createState() => _SplashScreenState();
}

class _SplashScreenState extends State<SplashScreen> {
  String _statusText = '正在建立端到端加密安全通道...';

  @override
  void initState() {
    super.initState();
    _initializeApp();
  }

  Future<void> _initializeApp() async {
    // 1. 初始化用户会话
    await AuthService().initSession();

    // 2. 探针与服务可用性探活
    try {
      await ApiService().probeAllGateways();
      if (mounted) {
        setState(() {
          _statusText = '安全信道校验通过，正在进入审批中心...';
        });
      }
    } catch (_) {
      if (mounted) {
        setState(() {
          _statusText = '已开启安全沙箱离线加签模式...';
        });
      }
    }

    await Future.delayed(const Duration(milliseconds: 600));

    if (!mounted) return;

    if (AuthService().isAuthenticated) {
      if (mounted) {
        setState(() {
          _statusText = '正在唤醒指纹 / 面容生物安全解锁...';
        });
      }
      final bioSuccess = await AuthService().loginWithBiometrics();
      if (!mounted) return;
      if (bioSuccess) {
        setState(() {
          _statusText = '指纹 / 面容安全核验通过，正在进入审批中心...';
        });
        await Future.delayed(const Duration(milliseconds: 350));
        if (!mounted) return;
        Navigator.of(context).pushReplacement(
          MaterialPageRoute(builder: (_) => const HomeNavigation()),
        );
      } else {
        Navigator.of(context).pushReplacement(
          MaterialPageRoute(builder: (_) => const LoginScreen(autoBiometric: false)),
        );
      }
    } else {
      Navigator.of(context).pushReplacement(
        MaterialPageRoute(builder: (_) => const LoginScreen(autoBiometric: false)),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: PacerTheme.brandDark,
      body: SafeArea(
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 24.0, vertical: 32.0),
          child: Column(
            children: [
              const Spacer(flex: 2),
              // 稳骥品牌 Logo 区域
              ClipRRect(
                borderRadius: BorderRadius.circular(24),
                child: Container(
                  width: 96,
                  height: 96,
                  decoration: BoxDecoration(
                    boxShadow: [
                      BoxShadow(
                        color: const Color(0xFF0284C7).withValues(alpha: 0.35),
                        blurRadius: 20,
                        offset: const Offset(0, 8),
                      ),
                    ],
                  ),
                  child: Image.asset(
                    'assets/icon/app_icon.png',
                    fit: BoxFit.cover,
                  ),
                ),
              ),
              const SizedBox(height: 24),
              const Text(
                '稳骥 PacerSQL',
                style: TextStyle(
                  color: Colors.white,
                  fontSize: 26,
                  fontWeight: FontWeight.bold,
                  letterSpacing: 0.5,
                ),
              ),
              const SizedBox(height: 8),
              const Text(
                '移动风控审批与流式微批步态控制中心',
                style: TextStyle(
                  color: Color(0xFF94A3B8),
                  fontSize: 14,
                ),
              ),
              const Spacer(flex: 2),

              // 极简加载指示条
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 14),
                decoration: BoxDecoration(
                  color: const Color(0xFF1E293B),
                  borderRadius: BorderRadius.circular(12),
                  border: Border.all(color: const Color(0xFF334155)),
                ),
                child: Row(
                  children: [
                    const SizedBox(
                      width: 14,
                      height: 14,
                      child: CircularProgressIndicator(
                        strokeWidth: 2,
                        valueColor: AlwaysStoppedAnimation<Color>(Color(0xFF38BDF8)),
                      ),
                    ),
                    const SizedBox(width: 14),
                    Expanded(
                      child: Text(
                        _statusText,
                        style: const TextStyle(
                          color: Color(0xFFE2E8F0),
                          fontSize: 13,
                          fontWeight: FontWeight.w500,
                        ),
                      ),
                    ),
                  ],
                ),
              ),

              const SizedBox(height: 24),
              // 官方主站与技术支持 (仅展示官网与邮箱，不展示备案信息与运营商BGP技术细节)
              const Text(
                '官方网站: ${AppConfig.officialWebsite}',
                style: TextStyle(
                  color: Color(0xFF94A3B8),
                  fontSize: 12,
                  fontWeight: FontWeight.w500,
                ),
              ),
              const SizedBox(height: 4),
              const Text(
                '技术支持: ${AppConfig.officialEmail}',
                style: TextStyle(
                  color: Color(0xFF64748B),
                  fontSize: 11,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
