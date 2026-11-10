import 'package:flutter/material.dart';
import '../services/ticket_service.dart';
import '../theme/pacer_theme.dart';
import 'dashboard_screen.dart';
import 'db_instance_screen.dart';
import 'profile_screen.dart';
import 'ticket_list_screen.dart';

class HomeNavigation extends StatefulWidget {
  const HomeNavigation({super.key});

  @override
  State<HomeNavigation> createState() => _HomeNavigationState();
}

class _HomeNavigationState extends State<HomeNavigation> {
  int _currentIndex = 0;

  final List<Widget> _screens = const [
    TicketListScreen(),
    DbInstanceScreen(),
    DashboardScreen(),
    ProfileScreen(),
  ];

  @override
  void initState() {
    super.initState();
    TicketService().addListener(_onTicketUpdate);
  }

  void _onTicketUpdate() {
    if (mounted) setState(() {});
  }

  @override
  void dispose() {
    TicketService().removeListener(_onTicketUpdate);
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final pendingCount = TicketService().pendingCount;
    final isDark = Theme.of(context).brightness == Brightness.dark;
    final unselectedColor = isDark ? const Color(0xFF94A3B8) : const Color(0xFF64748B);
    final surfaceColor = isDark ? const Color(0xFF1E293B) : Colors.white;
    final borderColor = isDark ? const Color(0xFF334155) : const Color(0xFFE2E8F0);

    return LayoutBuilder(
      builder: (context, constraints) {
        final isWide = constraints.maxWidth >= 720;

        return Scaffold(
          body: isWide
              ? Row(
                  children: [
                    NavigationRail(
                      selectedIndex: _currentIndex,
                      onDestinationSelected: (index) => setState(() => _currentIndex = index),
                      backgroundColor: surfaceColor,
                      labelType: NavigationRailLabelType.all,
                      selectedIconTheme: const IconThemeData(color: PacerTheme.primaryBlue),
                      unselectedIconTheme: IconThemeData(color: unselectedColor),
                      selectedLabelTextStyle: const TextStyle(
                        color: PacerTheme.primaryBlue,
                        fontSize: 12,
                        fontWeight: FontWeight.bold,
                      ),
                      unselectedLabelTextStyle: TextStyle(
                        color: unselectedColor,
                        fontSize: 12,
                      ),
                      destinations: [
                        NavigationRailDestination(
                          icon: Badge(
                            isLabelVisible: pendingCount > 0,
                            label: Text('$pendingCount'),
                            backgroundColor: const Color(0xFFEF4444),
                            child: const Icon(Icons.assignment_turned_in_outlined),
                          ),
                          selectedIcon: Badge(
                            isLabelVisible: pendingCount > 0,
                            label: Text('$pendingCount'),
                            backgroundColor: const Color(0xFFEF4444),
                            child: const Icon(Icons.assignment_turned_in),
                          ),
                          label: const Text('待办审批'),
                        ),
                        const NavigationRailDestination(
                          icon: Icon(Icons.dns_outlined),
                          selectedIcon: Icon(Icons.dns),
                          label: Text('集群拓扑'),
                        ),
                        const NavigationRailDestination(
                          icon: Icon(Icons.analytics_outlined),
                          selectedIcon: Icon(Icons.analytics),
                          label: Text('运控大屏'),
                        ),
                        const NavigationRailDestination(
                          icon: Icon(Icons.person_outline),
                          selectedIcon: Icon(Icons.person),
                          label: Text('我的运维'),
                        ),
                      ],
                    ),
                    VerticalDivider(thickness: 1, width: 1, color: borderColor),
                    Expanded(
                      child: IndexedStack(
                        index: _currentIndex,
                        children: _screens,
                      ),
                    ),
                  ],
                )
              : IndexedStack(
                  index: _currentIndex,
                  children: _screens,
                ),
          bottomNavigationBar: isWide
              ? null
              : BottomNavigationBar(
                  currentIndex: _currentIndex,
                  onTap: (index) => setState(() => _currentIndex = index),
                  type: BottomNavigationBarType.fixed,
                  backgroundColor: surfaceColor,
                  selectedItemColor: PacerTheme.primaryBlue,
                  unselectedItemColor: unselectedColor,
                  selectedFontSize: 12,
                  unselectedFontSize: 12,
                  selectedLabelStyle: const TextStyle(fontWeight: FontWeight.bold),
                  items: [
                    BottomNavigationBarItem(
                      icon: Badge(
                        isLabelVisible: pendingCount > 0,
                        label: Text('$pendingCount'),
                        backgroundColor: const Color(0xFFEF4444),
                        child: const Icon(Icons.assignment_turned_in_outlined),
                      ),
                      activeIcon: Badge(
                        isLabelVisible: pendingCount > 0,
                        label: Text('$pendingCount'),
                        backgroundColor: const Color(0xFFEF4444),
                        child: const Icon(Icons.assignment_turned_in),
                      ),
                      label: '待办审批',
                    ),
                    const BottomNavigationBarItem(
                      icon: Icon(Icons.dns_outlined),
                      activeIcon: Icon(Icons.dns),
                      label: '集群拓扑',
                    ),
                    const BottomNavigationBarItem(
                      icon: Icon(Icons.analytics_outlined),
                      activeIcon: Icon(Icons.analytics),
                      label: '运控大屏',
                    ),
                    const BottomNavigationBarItem(
                      icon: Icon(Icons.person_outline),
                      activeIcon: Icon(Icons.person),
                      label: '我的运维',
                    ),
                  ],
                ),
        );
      },
    );
  }
}
