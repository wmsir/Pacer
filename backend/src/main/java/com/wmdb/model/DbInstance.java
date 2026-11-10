package com.wmdb.model;

import com.baomidou.mybatisplus.annotation.IdType;
import com.baomidou.mybatisplus.annotation.TableId;
import com.baomidou.mybatisplus.annotation.TableName;
import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

/**
 * 数据库实例实体类
 * <p>
 * 映射 db_instance 表，存储纳管的目标数据库实例连接信息、资源组绑定、操作支持范围及安全凭证。
 * </p>
 *
 * @author wm
 * @date 2023-10-25
 */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
@TableName("db_instance")
public class DbInstance {

    private String tenantId;

    /**
     * 主键 ID
     */
    @TableId(type = IdType.AUTO)
    private Long id;

    /**
     * 实例名称
     */
    private String name;

    /**
     * 数据库类型（如 mysql, dameng, postgresql, oracle, tidb, oceanbase, kingbase, opengauss）
     */
    private String dbType;

    /**
     * 主机 / IP 地址
     */
    private String host;

    /**
     * 端口号
     */
    private Integer port;

    /**
     * 默认数据库名 (Schema)
     */
    private String databaseName;

    /**
     * JDBC 连接串
     */
    private String jdbcUrl;

    /**
     * 只读从库 JDBC 连接串（可选，用于 DQL 读写分离）
     */
    private String readOnlyJdbcUrl;

    /**
     * 数据库账号
     */
    private String username;

    /**
     * 密码（SM4 加密存储）
     */
    private String passwordCipher;

    /**
     * 所属环境（DEV, TEST, PROD）
     */
    private String env;

    /**
     * 状态（AUDITING, APPROVED, DISABLED）
     */
    private String status;

    /**
     * 关联资源组 (JSON / 逗号列表，如 ["车险承保资源组", "销管系统资源组"])
     */
    private String resourceGroups;

    /**
     * 实例业务标签 (如 ["核心生产库", "高可用集群", "敏感数据资产"])
     */
    private String tags;

    /**
     * 支持的操作标签 (如 ["支持上线", "支持查询", "支持DML变更", "支持DDL结构变更", "支持数据导出", "支持事务预执行", "支持数据脱敏", "支持历史回滚"])
     */
    private String supportedOps;

    /**
     * 连接隧道 (DIRECT 直连, SSH_TUNNEL SSH代理)
     */
    private String connectionTunnel;

    /**
     * 专属固定审批流模板 ID（若配置，则该实例所有工单强制走此专属固定流，跳过动态综合决策）
     */
    private Long fixedWorkflowTemplateId;

    /**
     * 专属固定审批流模板名称
     */
    private String fixedWorkflowTemplateName;

    /**
     * 实例描述 / 业务用途说明
     */
    private String description;

    /**
     * 实例参数配置与可配置化工单扩展字段（JSON 格式，包含 SQL 超时、最大影响行数、是否强制事务、customFields 扩展字段与必填约束等）
     */
    private String instanceConfig;
}
