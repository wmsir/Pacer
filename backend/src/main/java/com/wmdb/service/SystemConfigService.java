package com.wmdb.service;

import lombok.Data;
import lombok.extern.slf4j.Slf4j;
import org.springframework.stereotype.Service;

import java.util.concurrent.ConcurrentHashMap;

/**
 * 平台全局安全与审计策略配置服务
 * <p>
 * 管理员可在线动态配置：事务级预执行校验强制策略、DML/DDL 数据备份必填策略、最大查询行数限制等。
 * </p>
 *
 * @author wm
 */
@Slf4j
@Service
public class SystemConfigService {

    private final ConcurrentHashMap<String, Object> configStore = new ConcurrentHashMap<>();

    public SystemConfigService() {
        // 默认策略配置
        configStore.put("enforceDryRun", true);
        configStore.put("requireBackup", true);
        configStore.put("maxQueryRows", 1000);
        configStore.put("allowDbaManualTool", true);

        // 默认集群执行策略：查询走从节点，执行走主节点
        ExecutionPolicyDTO defaultPolicy = ExecutionPolicyDTO.builder()
                .policyMode("MASTER_WRITE_SLAVE_READ")
                .policyName("智能读写分离与高可用路由策略 (系统默认)")
                .queryRoute("WORKER_SLAVE_FIRST")
                .executeRoute("MASTER_ONLY")
                .failoverEnabled(true)
                .maxLagSeconds(5)
                .heavyQueryNode("WORKER_ONLY")
                .description("当系统扩展至多台机器时，日常 DQL 查询自动负载均衡分流到只读 Worker 从机，变更工单与审批发布锁定 Master 主机。")
                .updateTime(java.time.LocalDateTime.now().format(java.time.format.DateTimeFormatter.ofPattern("yyyy-MM-dd HH:mm:ss")))
                .build();
        configStore.put("executionPolicy", defaultPolicy);
    }

    public ExecutionPolicyDTO getExecutionPolicy() {
        Object val = configStore.get("executionPolicy");
        if (val instanceof ExecutionPolicyDTO) {
            return (ExecutionPolicyDTO) val;
        }
        return ExecutionPolicyDTO.builder()
                .policyMode("MASTER_WRITE_SLAVE_READ")
                .policyName("智能读写分离与高可用路由策略 (系统默认)")
                .queryRoute("WORKER_SLAVE_FIRST")
                .executeRoute("MASTER_ONLY")
                .failoverEnabled(true)
                .maxLagSeconds(5)
                .heavyQueryNode("WORKER_ONLY")
                .description("查询走从节点，执行走主节点")
                .updateTime(java.time.LocalDateTime.now().format(java.time.format.DateTimeFormatter.ofPattern("yyyy-MM-dd HH:mm:ss")))
                .build();
    }

    public void updateExecutionPolicy(ExecutionPolicyDTO dto) {
        if (dto != null) {
            dto.setUpdateTime(java.time.LocalDateTime.now().format(java.time.format.DateTimeFormatter.ofPattern("yyyy-MM-dd HH:mm:ss")));
            configStore.put("executionPolicy", dto);
            log.info("Updated cluster execution policy: mode={}, queryRoute={}, executeRoute={}, failover={}",
                    dto.getPolicyMode(), dto.getQueryRoute(), dto.getExecuteRoute(), dto.getFailoverEnabled());
        }
    }

    public boolean isEnforceDryRun() {
        Object val = configStore.get("enforceDryRun");
        return val instanceof Boolean ? (Boolean) val : Boolean.parseBoolean(String.valueOf(val));
    }

    public boolean isRequireBackup() {
        Object val = configStore.get("requireBackup");
        return val instanceof Boolean ? (Boolean) val : Boolean.parseBoolean(String.valueOf(val));
    }

    public int getMaxQueryRows() {
        Object val = configStore.get("maxQueryRows");
        return val instanceof Integer ? (Integer) val : 1000;
    }

    public SafetyPolicyDTO getSafetyPolicies() {
        return SafetyPolicyDTO.builder()
                .enforceDryRun(isEnforceDryRun())
                .requireBackup(isRequireBackup())
                .maxQueryRows(getMaxQueryRows())
                .build();
    }

    public void updateSafetyPolicies(SafetyPolicyDTO dto) {
        if (dto != null) {
            if (dto.getEnforceDryRun() != null) {
                configStore.put("enforceDryRun", dto.getEnforceDryRun());
            }
            if (dto.getRequireBackup() != null) {
                configStore.put("requireBackup", dto.getRequireBackup());
            }
            if (dto.getMaxQueryRows() != null) {
                configStore.put("maxQueryRows", dto.getMaxQueryRows());
            }
            log.info("Updated safety policies: enforceDryRun={}, requireBackup={}, maxQueryRows={}",
                    isEnforceDryRun(), isRequireBackup(), getMaxQueryRows());
        }
    }

    @Data
    @lombok.Builder
    @lombok.NoArgsConstructor
    @lombok.AllArgsConstructor
    public static class ExecutionPolicyDTO {
        /**
         * 策略模式代码 (如: "MASTER_WRITE_SLAVE_READ", "LEAST_LOAD_FIRST", "WORKER_COMPUTE_ISOLATION", "ROUND_ROBIN")
         */
        private String policyMode;

        /**
         * 策略名称 (如: "智能读写分离与高可用路由策略 (默认)")
         */
        private String policyName;

        /**
         * 数据查询 DQL 路由策略 (如: "WORKER_SLAVE_FIRST" 优先从节点/从库, "WORKER_ONLY" 仅从节点, "MASTER_ONLY" 仅主节点)
         */
        private String queryRoute;

        /**
         * 工单变更 DDL/DML 路由策略 (如: "MASTER_ONLY" 强制主节点/主库, "PRIMARY_COORDINATOR" 主协调机)
         */
        private String executeRoute;

        /**
         * 是否开启故障自动降级 (从库异常/延迟超标时自动回退主库)
         */
        private Boolean failoverEnabled;

        /**
         * 主从复制最大容忍延迟(秒)
         */
        private Integer maxLagSeconds;

        /**
         * 慢查询或大算力隔离节点 (如: "WORKER_ONLY", "ALL_WORKERS")
         */
        private String heavyQueryNode;

        /**
         * 策略说明
         */
        private String description;

        /**
         * 最近更新时间
         */
        private String updateTime;
    }

    @Data
    @lombok.Builder
    @lombok.NoArgsConstructor
    @lombok.AllArgsConstructor
    public static class SafetyPolicyDTO {
        private Boolean enforceDryRun;
        private Boolean requireBackup;
        private Integer maxQueryRows;
    }
}