package com.wmdb.service;

import com.baomidou.mybatisplus.core.conditions.query.QueryWrapper;
import com.wmdb.exception.BusinessException;
import com.wmdb.mapper.DataMaskingRuleMapper;
import com.wmdb.model.DataMaskingRule;
import com.wmdb.model.MaskingAlgorithmDTO;
import com.wmdb.model.MaskingPreviewRequestDTO;
import jakarta.annotation.PostConstruct;
import lombok.RequiredArgsConstructor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.stereotype.Service;

import javax.sql.DataSource;
import java.sql.Connection;
import java.sql.DatabaseMetaData;
import java.sql.ResultSet;
import java.sql.Statement;
import java.util.*;

/**
 * 数据脱敏规则业务服务
 *
 * @author wm
 */
@Slf4j
@Service
@RequiredArgsConstructor
public class DataMaskingRuleService {

    private final DataMaskingRuleMapper dataMaskingRuleMapper;
    private final MaskingEngine maskingEngine;
    private final DataSource dataSource;

    private static final List<MaskingAlgorithmDTO> SUPPORTED_ALGORITHMS = List.of(
            MaskingAlgorithmDTO.builder().type("NONE").name("不脱敏 (原始明文)").description("保持原始数据库明文显示").sampleOriginal("13800000001").sampleMasked("13800000001").build(),
            MaskingAlgorithmDTO.builder().type("PHONE").name("手机号码脱敏").description("保留前3后4，中间4位掩码 (138****0001)").sampleOriginal("13800000001").sampleMasked("138****0001").build(),
            MaskingAlgorithmDTO.builder().type("ID_CARD").name("身份证号脱敏").description("保留前4后4，中间10位掩码").sampleOriginal("310101199001011234").sampleMasked("3101**********1234").build(),
            MaskingAlgorithmDTO.builder().type("NAME").name("中文姓名脱敏").description("2字留首位，3字及以上留首尾字符").sampleOriginal("张小三").sampleMasked("张*三").build(),
            MaskingAlgorithmDTO.builder().type("EMAIL").name("电子邮箱脱敏").description("保留邮箱前缀首尾及域名后缀").sampleOriginal("zhangsan@qq.com").sampleMasked("z****n@qq.com").build(),
            MaskingAlgorithmDTO.builder().type("BANK_CARD").name("银行卡号脱敏").description("保留前6后4，中间连续掩码").sampleOriginal("6222021234567890").sampleMasked("622202******7890").build(),
            MaskingAlgorithmDTO.builder().type("ADDRESS").name("详细地址脱敏").description("隐藏后半段门牌号与楼栋信息").sampleOriginal("北京市海淀区中关村南大街1号院").sampleMasked("北京市海淀区中关村南大街****").build(),
            MaskingAlgorithmDTO.builder().type("PASSWORD").name("密码/全掩码").description("所有字符统一转换为 ******").sampleOriginal("MySecretPwd123").sampleMasked("******").build(),
            MaskingAlgorithmDTO.builder().type("CUSTOM_REGEX").name("自定义正则表达式").description("按自定义正则与替换串精准脱敏").sampleOriginal("ABC123456XYZ").sampleMasked("ABC****XYZ").build()
    );

    @PostConstruct
    public void initTable() {
        try (Connection conn = dataSource.getConnection()) {
            DatabaseMetaData md = conn.getMetaData();
            String catalog = conn.getCatalog();
            String schema = conn.getSchema();

            boolean exists = false;
            try (ResultSet rs = md.getTables(catalog, schema, "data_masking_rule", new String[]{"TABLE"})) {
                if (rs.next()) {
                    exists = true;
                }
            }

            if (!exists) {
                try (Statement stmt = conn.createStatement()) {
                    String createSql = "CREATE TABLE IF NOT EXISTS data_masking_rule (" +
                            "id BIGINT NOT NULL AUTO_INCREMENT PRIMARY KEY," +
                            "tenant_id VARCHAR(50) NOT NULL DEFAULT '1'," +
                            "instance_id BIGINT NOT NULL," +
                            "db_name VARCHAR(100) NOT NULL," +
                            "table_name VARCHAR(100) NOT NULL," +
                            "column_name VARCHAR(100) NOT NULL," +
                            "column_type VARCHAR(100)," +
                            "column_comment VARCHAR(255)," +
                            "rule_type VARCHAR(50) NOT NULL," +
                            "custom_regex VARCHAR(255)," +
                            "custom_replacement VARCHAR(255)," +
                            "status INT NOT NULL DEFAULT 1," +
                            "description VARCHAR(500)," +
                            "create_time DATETIME," +
                            "update_time DATETIME," +
                            "UNIQUE KEY uk_table_column (tenant_id, instance_id, db_name, table_name, column_name)" +
                            ") ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;";
                    stmt.execute(createSql);
                    log.info("Created table data_masking_rule successfully.");
                }
            }
        } catch (Exception e) {
            log.warn("Init data_masking_rule table exception: {}", e.getMessage());
        }
    }

    public List<MaskingAlgorithmDTO> listSupportedAlgorithms() {
        return SUPPORTED_ALGORITHMS;
    }

    public List<DataMaskingRule> listRules(Long instanceId, String dbName, String tableName) {
        QueryWrapper<DataMaskingRule> qw = new QueryWrapper<>();
        if (instanceId != null) {
            qw.eq("instance_id", instanceId);
        }
        if (dbName != null && !dbName.trim().isEmpty()) {
            qw.eq("db_name", dbName.trim());
        }
        if (tableName != null && !tableName.trim().isEmpty()) {
            qw.eq("table_name", tableName.trim());
        }
        qw.orderByDesc("id");
        return dataMaskingRuleMapper.selectList(qw);
    }

    public List<DataMaskingRule> listAllRules(String keyword) {
        QueryWrapper<DataMaskingRule> qw = new QueryWrapper<>();
        if (keyword != null && !keyword.trim().isEmpty()) {
            qw.like("table_name", keyword.trim())
                    .or()
                    .like("column_name", keyword.trim())
                    .or()
                    .like("db_name", keyword.trim())
                    .or()
                    .like("rule_type", keyword.trim());
        }
        qw.orderByDesc("id");
        return dataMaskingRuleMapper.selectList(qw);
    }

    public com.wmdb.model.PageResultDTO<DataMaskingRule> pageRules(int page, int size, String keyword, Long instanceId, String dbName) {
        QueryWrapper<DataMaskingRule> qw = new QueryWrapper<>();
        if (keyword != null && !keyword.trim().isEmpty()) {
            String kw = keyword.trim();
            qw.and(w -> w.like("table_name", kw)
                    .or().like("column_name", kw)
                    .or().like("db_name", kw)
                    .or().like("rule_type", kw));
        }
        if (instanceId != null && instanceId > 0) {
            qw.eq("instance_id", instanceId);
        }
        if (dbName != null && !dbName.trim().isEmpty()) {
            qw.eq("db_name", dbName.trim());
        }
        qw.orderByDesc("id");
        com.baomidou.mybatisplus.extension.plugins.pagination.Page<DataMaskingRule> mpPage =
                new com.baomidou.mybatisplus.extension.plugins.pagination.Page<>(page > 0 ? page : 1, size > 0 ? size : 10);
        dataMaskingRuleMapper.selectPage(mpPage, qw);
        return com.wmdb.model.PageResultDTO.from(mpPage);
    }

    public void saveRules(List<DataMaskingRule> rules) {
        if (rules == null || rules.isEmpty()) return;

        Date now = new Date();
        for (DataMaskingRule rule : rules) {
            if (rule.getInstanceId() == null || rule.getDbName() == null || rule.getTableName() == null || rule.getColumnName() == null) {
                continue;
            }
            if (rule.getTenantId() == null || rule.getTenantId().isEmpty()) {
                rule.setTenantId("1");
            }
            if (rule.getStatus() == null) {
                rule.setStatus(1);
            }

            // 如果选择了 NONE，删除或更新为停用
            if ("NONE".equalsIgnoreCase(rule.getRuleType())) {
                dataMaskingRuleMapper.delete(new QueryWrapper<DataMaskingRule>()
                        .eq("instance_id", rule.getInstanceId())
                        .eq("db_name", rule.getDbName())
                        .eq("table_name", rule.getTableName())
                        .eq("column_name", rule.getColumnName()));
                continue;
            }

            DataMaskingRule existing = dataMaskingRuleMapper.selectOne(new QueryWrapper<DataMaskingRule>()
                    .eq("instance_id", rule.getInstanceId())
                    .eq("db_name", rule.getDbName())
                    .eq("table_name", rule.getTableName())
                    .eq("column_name", rule.getColumnName()));

            if (existing != null) {
                existing.setRuleType(rule.getRuleType());
                existing.setCustomRegex(rule.getCustomRegex());
                existing.setCustomReplacement(rule.getCustomReplacement());
                existing.setColumnType(rule.getColumnType());
                existing.setColumnComment(rule.getColumnComment());
                existing.setStatus(rule.getStatus());
                existing.setDescription(rule.getDescription());
                existing.setUpdateTime(now);
                dataMaskingRuleMapper.updateById(existing);
            } else {
                rule.setCreateTime(now);
                rule.setUpdateTime(now);
                dataMaskingRuleMapper.insert(rule);
            }
        }
    }

    public void deleteRule(Long id) {
        dataMaskingRuleMapper.deleteById(id);
    }

    public void toggleStatus(Long id) {
        DataMaskingRule rule = dataMaskingRuleMapper.selectById(id);
        if (rule != null) {
            rule.setStatus(rule.getStatus() != null && rule.getStatus() == 1 ? 0 : 1);
            rule.setUpdateTime(new Date());
            dataMaskingRuleMapper.updateById(rule);
        }
    }

    public String previewMasking(MaskingPreviewRequestDTO request) {
        String val = request.getSampleValue();
        if (val == null || val.trim().isEmpty()) {
            for (MaskingAlgorithmDTO alg : SUPPORTED_ALGORITHMS) {
                if (alg.getType().equalsIgnoreCase(request.getRuleType())) {
                    val = alg.getSampleOriginal();
                    break;
                }
            }
        }
        return maskingEngine.mask(val, request.getRuleType(), request.getCustomRegex(), request.getCustomReplacement());
    }

    /**
     * 获取指定实例和库下所有生效中的脱敏规则 (以 Map<columnNameLowerCase, Rule> 返回)
     */
    public Map<String, DataMaskingRule> getActiveRulesMap(Long instanceId, String dbName) {
        Map<String, DataMaskingRule> map = new HashMap<>();
        QueryWrapper<DataMaskingRule> qw = new QueryWrapper<DataMaskingRule>()
                .eq("instance_id", instanceId)
                .eq("status", 1);
        if (dbName != null && !dbName.trim().isEmpty()) {
            qw.eq("db_name", dbName.trim());
        }

        List<DataMaskingRule> list = dataMaskingRuleMapper.selectList(qw);
        for (DataMaskingRule r : list) {
            if (r.getColumnName() != null) {
                map.put(r.getColumnName().toLowerCase(), r);
                // 同时也以 tableName.columnName 索引
                if (r.getTableName() != null) {
                    map.put(r.getTableName().toLowerCase() + "." + r.getColumnName().toLowerCase(), r);
                }
            }
        }
        return map;
    }
}
