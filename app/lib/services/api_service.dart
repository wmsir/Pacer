import 'dart:async';
import 'dart:convert';
import 'package:flutter/foundation.dart';
import 'package:http/http.dart' as http;
import '../config/app_config.dart';

class GatewayLatency {
  final GatewayNode node;
  final int? latencyMs;
  final bool isAvailable;

  const GatewayLatency({
    required this.node,
    this.latencyMs,
    this.isAvailable = false,
  });
}

class ApiService {
  static final ApiService _instance = ApiService._internal();
  factory ApiService() => _instance;
  ApiService._internal();

  GatewayNode _currentGateway = AppConfig.gatewayNodes.first;
  String? _authToken;
  bool _offlineMockMode = false;

  GatewayNode get currentGateway => _currentGateway;
  String? get authToken => _authToken;
  bool get isOfflineMockMode => _offlineMockMode;

  void setAuthToken(String? token) {
    _authToken = token;
  }

  void switchGateway(GatewayNode node) {
    _currentGateway = node;
    _offlineMockMode = false;
    debugPrint('已切换主网关节点至: ${node.name} (${node.baseUrl})');
  }

  void setOfflineMockMode(bool enable) {
    _offlineMockMode = enable;
  }

  Map<String, String> _getHeaders() {
    final headers = {
      'Content-Type': 'application/json; charset=UTF-8',
      'Accept': 'application/json',
      'X-Client-Platform': 'Flutter-Android',
      'X-Client-Version': AppConfig.appVersion,
    };
    if (_authToken != null && _authToken!.isNotEmpty) {
      headers['Authorization'] = 'Bearer $_authToken';
    }
    return headers;
  }

  /// 测试单个网关节点的连通延迟 (ms)
  Future<GatewayLatency> pingGateway(GatewayNode node) async {
    final stopwatch = Stopwatch()..start();
    try {
      final url = Uri.parse('${node.baseUrl}/api/v1/health');
      final response = await http.get(url).timeout(const Duration(milliseconds: 3500));
      stopwatch.stop();
      if (response.statusCode >= 200 && response.statusCode < 500) {
        return GatewayLatency(
          node: node,
          latencyMs: stopwatch.elapsedMilliseconds,
          isAvailable: true,
        );
      }
    } catch (_) {
      // 备用方式：如果 /api/v1/health 不通，尝试根路径探测
      try {
        final rootUrl = Uri.parse(node.baseUrl);
        final rootResp = await http.head(rootUrl).timeout(const Duration(milliseconds: 2500));
        stopwatch.stop();
        if (rootResp.statusCode < 500) {
          return GatewayLatency(
            node: node,
            latencyMs: stopwatch.elapsedMilliseconds,
            isAvailable: true,
          );
        }
      } catch (_) {}
    }
    stopwatch.stop();
    return GatewayLatency(
      node: node,
      latencyMs: null,
      isAvailable: false,
    );
  }

  /// 三网自适应诊断：并行探测所有网关节点
  Future<List<GatewayLatency>> probeAllGateways() async {
    final futures = AppConfig.gatewayNodes.map((node) => pingGateway(node)).toList();
    final results = await Future.wait(futures);

    // 如果当前选中的网关不可用，自动切换到第一个可用节点
    final available = results.where((r) => r.isAvailable).toList();
    if (available.isNotEmpty) {
      final currentStillOk = available.any((r) => r.node.baseUrl == _currentGateway.baseUrl);
      if (!currentStillOk) {
        // 按延迟从低到高排序，自动选择最快节点
        available.sort((a, b) => (a.latencyMs ?? 9999).compareTo(b.latencyMs ?? 9999));
        switchGateway(available.first.node);
      }
    } else {
      _offlineMockMode = true;
    }
    return results;
  }

  /// 通用 GET 请求，集成三网自动故障转移
  Future<dynamic> get(String path) async {
    if (_offlineMockMode) {
      throw Exception('当前处于离线演示模式');
    }

    // 优先尝试当前网关
    final nodesToTry = [
      _currentGateway,
      ...AppConfig.gatewayNodes.where((n) => n.baseUrl != _currentGateway.baseUrl),
    ];

    for (var node in nodesToTry) {
      try {
        final uri = Uri.parse('${node.baseUrl}$path');
        final response = await http
            .get(uri, headers: _getHeaders())
            .timeout(const Duration(milliseconds: 5000));

        if (response.statusCode >= 200 && response.statusCode < 300) {
          if (node.baseUrl != _currentGateway.baseUrl) {
            _currentGateway = node; // 自动转移
          }
          final decoded = jsonDecode(utf8.decode(response.bodyBytes));
          if (decoded is Map<String, dynamic> &&
              decoded.containsKey('code') &&
              decoded.containsKey('data')) {
            return decoded['data'];
          }
          return decoded;
        }
      } catch (e) {
        debugPrint('网关 ${node.name} 请求异常: $e，尝试备用节点...');
      }
    }

    throw Exception('三网节点均不可达，已进入本地高可用容灾保护模式');
  }

  /// 通用 POST 请求
  Future<dynamic> post(String path, Map<String, dynamic> body) async {
    if (_offlineMockMode) {
      throw Exception('当前处于离线演示模式');
    }

    final nodesToTry = [
      _currentGateway,
      ...AppConfig.gatewayNodes.where((n) => n.baseUrl != _currentGateway.baseUrl),
    ];

    for (var node in nodesToTry) {
      try {
        final uri = Uri.parse('${node.baseUrl}$path');
        final response = await http
            .post(
              uri,
              headers: _getHeaders(),
              body: jsonEncode(body),
            )
            .timeout(const Duration(milliseconds: 6000));

        if (response.statusCode >= 200 && response.statusCode < 300) {
          if (node.baseUrl != _currentGateway.baseUrl) {
            _currentGateway = node;
          }
          final decoded = jsonDecode(utf8.decode(response.bodyBytes));
          if (decoded is Map<String, dynamic> &&
              decoded.containsKey('code') &&
              decoded.containsKey('data')) {
            return decoded['data'];
          }
          return decoded;
        }
      } catch (e) {
        debugPrint('网关 ${node.name} 请求异常: $e');
      }
    }

    throw Exception('网络请求超时，已启用本地事务缓存');
  }
}
