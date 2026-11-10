import 'package:flutter/material.dart';
import 'config/app_config.dart';
import 'screens/splash_screen.dart';
import 'theme/pacer_theme.dart';

void main() {
  WidgetsFlutterBinding.ensureInitialized();
  runApp(const PacerApp());
}

class PacerApp extends StatelessWidget {
  const PacerApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: AppConfig.appName,
      debugShowCheckedModeBanner: false,
      theme: PacerTheme.lightTheme,
      darkTheme: PacerTheme.darkTheme,
      themeMode: ThemeMode.light,
      home: const SplashScreen(),
    );
  }
}
