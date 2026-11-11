// 稳骥 PacerSQL · 全局网络配置与三网自适应引擎
class AppConfig {
  static const String appName = '稳骥 PacerSQL';
  static const String appVersion = '1.0.5';
  static const String buildNumber = '20261103.1';
  static const String officialWebsite = 'https://fwpt.cn';
  static const String officialEmail = 'admin@fwpt.cn';

  // 1. 高可用专线集群网关节点 (支持生产主站、直连节点、容灾专线)
  static const List<GatewayNode> gatewayNodes = [
    GatewayNode(
      name: 'FWPT 官方主站 (生产核心集群)',
      baseUrl: 'https://fwpt.cn/pacer',
      carrierTag: '生产主线路',
      isDefault: true,
    ),
    GatewayNode(
      name: '生产专线节点 1 (直连高可用)',
      baseUrl: 'http://101.35.100.169:8088',
      carrierTag: '直连专线1',
      isDefault: false,
    ),
    GatewayNode(
      name: '生产专线节点 2 (灾备备份节点)',
      baseUrl: 'http://39.97.158.22:8088',
      carrierTag: '容灾专线2',
      isDefault: false,
    ),
  ];

  static const String defaultBaseUrl = 'https://fwpt.cn/pacer';
}

class GatewayNode {
  final String name;
  final String baseUrl;
  final String carrierTag;
  final bool isDefault;

  const GatewayNode({
    required this.name,
    required this.baseUrl,
    required this.carrierTag,
    this.isDefault = false,
  });
}
