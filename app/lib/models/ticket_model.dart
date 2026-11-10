import 'package:flutter/material.dart';

enum TicketStatus {
  pending,          // 待我审批
  waitingExecution, // 待定时计划执行
  running,          // 流式执行中
  executed,         // 执行完成 / 已过账
  rejected,         // 已驳回
  failed,           // 执行异常 / 已熔断
}

enum RiskLevel {
  low,    // 一级低危 (安全合规)
  medium, // 二级中危 (大批量更新/无WHERE)
  high,   // 三级高危 (DDL/删表/阻断)
}

class SqlTicket {
  final int id;
  final String ticketNo;
  final String title;
  final String instanceName;
  final String dbName;
  final String changeType; // DDL / DML / DCL
  final TicketStatus status;
  final RiskLevel riskLevel;
  final int riskScore; // 0 - 100
  final String submitter;
  final String submitTime;
  final String? scheduledTime;
  final String sqlPreview;
  final int expectedRows;
  final int actualRows;
  final String? rejectReason;

  const SqlTicket({
    required this.id,
    required this.ticketNo,
    required this.title,
    required this.instanceName,
    required this.dbName,
    required this.changeType,
    required this.status,
    required this.riskLevel,
    required this.riskScore,
    required this.submitter,
    required this.submitTime,
    this.scheduledTime,
    required this.sqlPreview,
    this.expectedRows = 0,
    this.actualRows = 0,
    this.rejectReason,
  });

  factory SqlTicket.fromJson(Map<String, dynamic> json) {
    TicketStatus parseStatus(String? s) {
      switch ((s ?? '').toUpperCase()) {
        case 'RUNNING':
        case 'EXECUTING':
        case 'MANUAL_PROCESSING':
          return TicketStatus.running;
        case 'EXECUTED':
        case 'SUCCESS':
          return TicketStatus.executed;
        case 'REJECTED':
        case 'TERMINATED':
          return TicketStatus.rejected;
        case 'WAITING_EXECUTION':
        case 'SCHEDULED':
        case 'APPROVED':
          return TicketStatus.waitingExecution;
        case 'FAILED':
          return TicketStatus.failed;
        case 'AUDITING':
        case 'PENDING_APPROVAL':
        case 'SUBMITTED':
        default:
          return TicketStatus.pending;
      }
    }

    RiskLevel parseRisk(int score) {
      if (score >= 80) return RiskLevel.high;
      if (score >= 40) return RiskLevel.medium;
      return RiskLevel.low;
    }

    final int expected = json['expectedRows'] is int
        ? json['expectedRows'] as int
        : (json['affectRowsEstimate'] is int
            ? json['affectRowsEstimate'] as int
            : int.tryParse(json['affectRowsEstimate']?.toString() ?? '1') ?? 1);

    final String rawType = (json['type'] ?? 'DML').toString();
    final String normalizedType = rawType.contains('DDL')
        ? 'DDL'
        : (rawType.contains('EXPORT') ? 'DQL' : 'DML');

    int score = json['riskScore'] is int ? json['riskScore'] as int : 0;
    if (score <= 0) {
      if (normalizedType == 'DDL' || expected > 1000) {
        score = 86;
      } else if (expected > 100) {
        score = 52;
      } else {
        score = 18;
      }
    }

    return SqlTicket(
      id: json['id'] is int ? json['id'] : int.tryParse(json['id']?.toString() ?? '0') ?? 0,
      ticketNo: (json['ticketNo'] ?? 'TK-2026-${(json['id'] ?? 1000).toString().padLeft(4, '0')}').toString(),
      title: (json['reason'] ?? json['title'] ?? '数据库变更工单 #${json['id']}').toString(),
      instanceName: (json['instanceName'] ?? json['dbInstance'] ?? 'MySQL-PROD-集群01').toString(),
      dbName: (json['dbName'] ?? 'db_crm_master').toString(),
      changeType: normalizedType,
      status: parseStatus(json['status']?.toString()),
      riskLevel: parseRisk(score),
      riskScore: score,
      submitter: (json['applicantName'] ?? json['applicant'] ?? json['applicantIdCard'] ?? json['submitter'] ?? '研发-李工').toString(),
      submitTime: (json['createTime'] ?? json['createdTime'] ?? json['createdAt'] ?? '2026-11-03 10:30').toString(),
      scheduledTime: json['scheduledTime']?.toString() ?? json['executionWindow']?.toString(),
      sqlPreview: (json['sqlText'] ?? '-- SQL 脚本待载入').toString(),
      expectedRows: expected,
      actualRows: json['affectRows'] is int
          ? json['affectRows'] as int
          : (json['actualAffectRows'] is int ? json['actualAffectRows'] as int : expected),
      rejectReason: json['rejectReason']?.toString(),
    );
  }

  String get statusText {
    switch (status) {
      case TicketStatus.pending:
        return '待审批';
      case TicketStatus.waitingExecution:
        return '定时待执行';
      case TicketStatus.running:
        return '流式执行中';
      case TicketStatus.executed:
        return '已执行';
      case TicketStatus.rejected:
        return '已驳回';
      case TicketStatus.failed:
        return '已熔断失败';
    }
  }

  Color get statusColor {
    switch (status) {
      case TicketStatus.pending:
        return const Color(0xFF0284C7); // 待审批天蓝
      case TicketStatus.waitingExecution:
        return const Color(0xFF8B5CF6); // 定时紫色
      case TicketStatus.running:
        return const Color(0xFFF59E0B); // 执行中橙色
      case TicketStatus.executed:
        return const Color(0xFF10B981); // 成功绿色
      case TicketStatus.rejected:
        return const Color(0xFF94A3B8); // 驳回灰
      case TicketStatus.failed:
        return const Color(0xFFEF4444); // 失败红
    }
  }

  String get riskLabel {
    switch (riskLevel) {
      case RiskLevel.high:
        return '三级高危阻断';
      case RiskLevel.medium:
        return '二级中危提醒';
      case RiskLevel.low:
        return '一级安全预检';
    }
  }

  Color get riskColor {
    switch (riskLevel) {
      case RiskLevel.high:
        return const Color(0xFFEF4444);
      case RiskLevel.medium:
        return const Color(0xFFF59E0B);
      case RiskLevel.low:
        return const Color(0xFF10B981);
    }
  }
}

class SqlTicketDetail {
  final SqlTicket ticket;
  final String fullSql;
  final String? rollbackSql;
  final List<String> astRiskRules;
  final List<String> dbaSuggestions;
  final bool dryRunPassed;
  final int dryRunAffectedRows;
  final List<ApprovalStep> timeline;

  const SqlTicketDetail({
    required this.ticket,
    required this.fullSql,
    this.rollbackSql,
    required this.astRiskRules,
    required this.dbaSuggestions,
    this.dryRunPassed = true,
    this.dryRunAffectedRows = 0,
    required this.timeline,
  });
}

class ApprovalStep {
  final String nodeName;
  final String operator;
  final String time;
  final String status;
  final String? comment;

  const ApprovalStep({
    required this.nodeName,
    required this.operator,
    required this.time,
    required this.status,
    this.comment,
  });
}
