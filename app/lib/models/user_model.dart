class UserProfile {
  final String username;
  final String nickname;
  final String role;         // ADMIN, DBA, TECH_LEADER, DEVELOPER
  final String phone;
  final String email;
  final String dataScope;     // 全集群范围 / 核心业务组
  final String token;
  final bool isDemo;

  const UserProfile({
    required this.username,
    required this.nickname,
    required this.role,
    required this.phone,
    required this.email,
    required this.dataScope,
    required this.token,
    this.isDemo = false,
  });

  String get roleDesc {
    switch (role.toUpperCase()) {
      case 'ADMIN':
        return '系统超级架构师 (全权限)';
      case 'DBA':
        return '首席生产数据库管理员 (DBA)';
      case 'TECH_LEADER':
        return '技术审批主管 / 架构师';
      default:
        return '研发工程师 / 变更发起人';
    }
  }

  bool get canApprove => role.toUpperCase() == 'ADMIN' || role.toUpperCase() == 'DBA' || role.toUpperCase() == 'TECH_LEADER';
}
