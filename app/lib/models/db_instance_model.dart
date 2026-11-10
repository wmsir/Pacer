import 'package:flutter/material.dart';

enum DbHealthStatus {
  healthy,
  warning,
  down,
}

class DbInstance {
  final int id;
  final String instanceName;
  final String dbType; // MySQL 8.0, TiDB 7.1, OceanBase, DM8, openGauss
  final String env;    // PROD, STAGING, TEST
  final String host;
  final int port;
  final int qps;
  final int activeConnections;
  final int maxConnections;
  final int replicationLagMs; // 主从复制延迟
  final DbHealthStatus status;
  final String clusterRole;  // 主控写节点 / 只读从节点 / 分片集群

  const DbInstance({
    required this.id,
    required this.instanceName,
    required this.dbType,
    required this.env,
    required this.host,
    required this.port,
    required this.qps,
    required this.activeConnections,
    required this.maxConnections,
    required this.replicationLagMs,
    required this.status,
    required this.clusterRole,
  });

  factory DbInstance.fromJson(Map<String, dynamic> json) {
    final rawStatus = (json['status'] ?? 'ONLINE').toString().toUpperCase();
    DbHealthStatus parsedStatus = DbHealthStatus.healthy;
    if (rawStatus.contains('WARN')) {
      parsedStatus = DbHealthStatus.warning;
    } else if (rawStatus.contains('DOWN') || rawStatus.contains('OFFLINE') || rawStatus.contains('ERR')) {
      parsedStatus = DbHealthStatus.down;
    }

    return DbInstance(
      id: json['id'] is int ? json['id'] : int.tryParse(json['id']?.toString() ?? '1') ?? 1,
      instanceName: (json['name'] ?? json['instanceName'] ?? '生产数据库实例').toString(),
      dbType: (json['dbType'] ?? 'MySQL 8.0').toString(),
      env: (json['env'] ?? 'PROD').toString(),
      host: (json['host'] ?? '10.0.10.11').toString(),
      port: json['port'] is int ? json['port'] : int.tryParse(json['port']?.toString() ?? '3306') ?? 3306,
      qps: json['qps'] is int ? json['qps'] : 1420,
      activeConnections: json['activeConnections'] is int ? json['activeConnections'] : 86,
      maxConnections: json['maxConnections'] is int ? json['maxConnections'] : 2000,
      replicationLagMs: json['replicationLagMs'] is int ? json['replicationLagMs'] : 2,
      status: parsedStatus,
      clusterRole: (json['clusterRole'] ?? json['resourceGroup'] ?? '主控写节点 (Master)').toString(),
    );
  }

  Color get statusColor {
    switch (status) {
      case DbHealthStatus.healthy:
        return const Color(0xFF10B981);
      case DbHealthStatus.warning:
        return const Color(0xFFF59E0B);
      case DbHealthStatus.down:
        return const Color(0xFFEF4444);
    }
  }

  String get statusText {
    switch (status) {
      case DbHealthStatus.healthy:
        return '正常运行';
      case DbHealthStatus.warning:
        return '高负荷预警';
      case DbHealthStatus.down:
        return '节点不可用';
    }
  }
}
