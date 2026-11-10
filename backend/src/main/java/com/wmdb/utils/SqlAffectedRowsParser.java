package com.wmdb.utils;

import com.wmdb.model.ParsedSqlStatement;

import java.util.ArrayList;
import java.util.List;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/**
 * SQL DML 预期影响行数注解解析器
 * <p>
 * 支持识别以下格式的预期影响行数注释：
 * <ul>
 *     <li><code>-- 1</code></li>
 *     <li><code>-- 10</code></li>
 *     <li><code>-- 影响行数: 1</code></li>
 *     <li><code>-- 预期影响行数: 1</code></li>
 *     <li><code>-- expect: 1</code> / <code>-- count: 1</code></li>
 *     <li><code>/* 1 *&#47;</code></li>
 * </ul>
 * </p>
 *
 * @author wm
 */
public class SqlAffectedRowsParser {

    // 匹配注释行中的预期影响行数数字
    private static final Pattern NUMERIC_COMMENT_PATTERN = Pattern.compile(
            "^\\s*--(?:\\s*(?:(?:预期|预计)?影响行数|expect|expected|count)[:：]?\\s*|\\s+)(\\d+)(?:\\s*rows?)?\\s*$",
            Pattern.CASE_INSENSITIVE
    );

    private static final Pattern BLOCK_COMMENT_PATTERN = Pattern.compile(
            "^\\s*/\\*\\s*(?:(?:预期|预计)?影响行数[:：]?\\s*|expect[:：]?\\s*)?(\\d+)\\s*\\*/\\s*$",
            Pattern.CASE_INSENSITIVE
    );

    /**
     * 解析完整 SQL 脚本中的所有语句及预期影响行数
     *
     * @param script 包含多条 SQL 的脚本
     * @return 解析后的语句列表
     */
    public static List<ParsedSqlStatement> parseScript(String script) {
        List<ParsedSqlStatement> result = new ArrayList<>();
        if (script == null || script.trim().isEmpty()) {
            return result;
        }

        List<String> rawStatements = SqlSplitUtils.split(script);
        Integer pendingExpectedRows = null;

        for (String rawStmt : rawStatements) {
            String trimmed = rawStmt.trim();
            if (trimmed.isEmpty()) {
                continue;
            }

            Integer expectedRows = extractExpectedRows(trimmed);
            String executableSql = extractExecutableSql(trimmed);
            String statementType = determineStatementType(executableSql);

            // 如果该语句段纯粹是注释或空白，不属于有效 SQL 语句
            if ("EMPTY".equals(statementType) || executableSql.isEmpty()) {
                if (expectedRows != null) {
                    if (!result.isEmpty() && result.get(result.size() - 1).getExpectedAffectedRows() == null) {
                        // 关联到上一条语句（如 UPDATE ... ; -- 1）
                        result.get(result.size() - 1).setExpectedAffectedRows(expectedRows);
                    } else {
                        // 暂存给下一条语句（如 -- 1 \n ; UPDATE ...）
                        pendingExpectedRows = expectedRows;
                    }
                }
                continue;
            }

            Integer finalExpected = expectedRows != null ? expectedRows : pendingExpectedRows;
            pendingExpectedRows = null;

            boolean isDml = isDmlType(statementType);

            result.add(ParsedSqlStatement.builder()
                    .index(result.size() + 1)
                    .rawSql(trimmed)
                    .executableSql(executableSql)
                    .expectedAffectedRows(finalExpected)
                    .statementType(statementType)
                    .isDml(isDml)
                    .build());
        }

        return result;
    }

    /**
     * 从语句文本中提取预期影响行数（支持头部、尾部及中间行注释）
     */
    public static Integer extractExpectedRows(String statementText) {
        String[] lines = statementText.split("\r?\n");
        for (String line : lines) {
            String trimmedLine = line.trim();
            if (trimmedLine.isEmpty()) continue;

            Matcher m1 = NUMERIC_COMMENT_PATTERN.matcher(trimmedLine);
            if (m1.matches()) {
                try {
                    return Integer.parseInt(m1.group(1));
                } catch (NumberFormatException ignored) {}
            }

            Matcher m2 = BLOCK_COMMENT_PATTERN.matcher(trimmedLine);
            if (m2.matches()) {
                try {
                    return Integer.parseInt(m2.group(1));
                } catch (NumberFormatException ignored) {}
            }

            // 也支持行尾追加的行内注释，如: UPDATE ... WHERE ... -- 1
            int inlineIdx = trimmedLine.indexOf("--");
            if (inlineIdx >= 0) {
                String inlineComment = trimmedLine.substring(inlineIdx).trim();
                Matcher m3 = NUMERIC_COMMENT_PATTERN.matcher(inlineComment);
                if (m3.matches()) {
                    try {
                        return Integer.parseInt(m3.group(1));
                    } catch (NumberFormatException ignored) {}
                }
            }
        }
        return null;
    }

    /**
     * 提取纯净的可执行 SQL（保留语句主体）
     */
    public static String extractExecutableSql(String statementText) {
        String[] lines = statementText.split("\r?\n");
        StringBuilder sb = new StringBuilder();
        boolean inHeader = true;

        for (String line : lines) {
            String trimmedLine = line.trim();
            if (inHeader) {
                // 跳过开头的预期行数等特殊注释行，使 SQL 更干净
                if (trimmedLine.startsWith("--") || trimmedLine.startsWith("/*")) {
                    Matcher m1 = NUMERIC_COMMENT_PATTERN.matcher(trimmedLine);
                    Matcher m2 = BLOCK_COMMENT_PATTERN.matcher(trimmedLine);
                    if (m1.matches() || m2.matches()) {
                        continue;
                    }
                } else if (!trimmedLine.isEmpty()) {
                    inHeader = false;
                }
            }
            sb.append(line).append("\n");
        }

        String result = sb.toString().trim();
        return result.isEmpty() ? statementText.trim() : result;
    }

    private static final java.util.Set<String> VALID_SQL_VERBS = java.util.Set.of(
            "SELECT", "INSERT", "UPDATE", "DELETE", "REPLACE",
            "CREATE", "ALTER", "DROP", "TRUNCATE", "RENAME",
            "SHOW", "DESC", "DESCRIBE", "EXPLAIN", "USE", "SET",
            "GRANT", "REVOKE", "LOCK", "UNLOCK", "START", "BEGIN", "COMMIT", "ROLLBACK", "SAVEPOINT",
            "ANALYZE", "OPTIMIZE", "CHECK", "REPAIR", "CALL", "DO", "HANDLER", "LOAD", "WITH",
            "KILL", "FLUSH", "RESET", "CHANGE", "STOP", "SOURCE", "COMMENT", "FLASHBACK", "PURGE",
            "MERGE", "UPSERT"
    );

    /**
     * 判断 SQL 语句类型，若非合法标准 SQL 动词则返回 UNKNOWN
     */
    public static String determineStatementType(String sql) {
        String clean = sql.replaceAll("/\\*.*?\\*/", "").replaceAll("--.*?(\r?\n|$)", "").trim();
        if (clean.isEmpty()) return "EMPTY";

        String firstWord = clean.split("\\s+")[0].replaceAll("[^a-zA-Z0-9_]", "").toUpperCase();
        if (VALID_SQL_VERBS.contains(firstWord)) {
            return firstWord;
        }
        return "UNKNOWN";
    }

    /**
     * 是否为 DML 变更语句
     */
    public static boolean isDmlType(String type) {
        return "INSERT".equalsIgnoreCase(type)
                || "UPDATE".equalsIgnoreCase(type)
                || "DELETE".equalsIgnoreCase(type)
                || "REPLACE".equalsIgnoreCase(type);
    }

    private static final Pattern TABLE_EXTRACT_PATTERN = Pattern.compile(
            "\\b(?:FROM|INTO|UPDATE|TABLE|TRUNCATE)\\s+(?:IF\\s+EXISTS\\s+|IF\\s+NOT\\s+EXISTS\\s+)?([`'\"\\w.]+)",
            Pattern.CASE_INSENSITIVE
    );

    /**
     * 提取 SQL 中涉及的目标表名
     */
    public static java.util.Set<String> extractTableNames(String sql) {
        java.util.Set<String> tables = new java.util.HashSet<>();
        if (sql == null || sql.trim().isEmpty()) return tables;
        String clean = sql.replaceAll("/\\*.*?\\*/", "").replaceAll("--.*?(\r?\n|$)", " ");
        Matcher m = TABLE_EXTRACT_PATTERN.matcher(clean);
        while (m.find()) {
            String rawTable = m.group(1);
            if (rawTable != null) {
                rawTable = rawTable.replaceAll("[`'\"]", "").trim();
                if (rawTable.contains(".")) {
                    rawTable = rawTable.substring(rawTable.lastIndexOf('.') + 1);
                }
                if (!rawTable.isEmpty() && !VALID_SQL_VERBS.contains(rawTable.toUpperCase())
                        && !rawTable.equalsIgnoreCase("SELECT") && !rawTable.equalsIgnoreCase("WHERE")
                        && !rawTable.equalsIgnoreCase("SET") && !rawTable.equalsIgnoreCase("JOIN")) {
                    tables.add(rawTable.toLowerCase());
                }
            }
        }
        return tables;
    }

    @lombok.Data
    @lombok.Builder
    @lombok.NoArgsConstructor
    @lombok.AllArgsConstructor
    public static class RollbackValidationResult {
        private boolean valid;
        private boolean matched;
        private String message;
        private java.util.Set<String> execTables;
        private java.util.Set<String> rollbackTables;
    }

    /**
     * 严格校验回滚 SQL 文本的合法性以及与执行脚本的目标表关联一致性
     */
    public static RollbackValidationResult validateRollbackSql(String execSql, String rollbackSql) {
        if (rollbackSql == null || rollbackSql.trim().isEmpty()) {
            return RollbackValidationResult.builder().valid(true).matched(true).message("未提供回滚 SQL").build();
        }

        List<ParsedSqlStatement> rollbackStmts = parseScript(rollbackSql);
        for (ParsedSqlStatement st : rollbackStmts) {
            if ("UNKNOWN".equals(st.getStatementType())) {
                String snippet = st.getRawSql().length() > 30 ? st.getRawSql().substring(0, 30) + "..." : st.getRawSql();
                return RollbackValidationResult.builder()
                        .valid(false)
                        .matched(false)
                        .message(String.format("回滚方案中包含非 SQL 文本【%s】，必须填写合法的真实 SQL 语句！", snippet))
                        .build();
            }
        }

        java.util.Set<String> execTables = extractTableNames(execSql);
        java.util.Set<String> rollbackTables = extractTableNames(rollbackSql);

        if (!execTables.isEmpty() && !rollbackTables.isEmpty()) {
            java.util.Set<String> common = new java.util.HashSet<>(execTables);
            common.retainAll(rollbackTables);
            if (common.isEmpty()) {
                return RollbackValidationResult.builder()
                        .valid(false)
                        .matched(false)
                        .execTables(execTables)
                        .rollbackTables(rollbackTables)
                        .message(String.format("回滚方案涉及的表【%s】与执行脚本涉及的表【%s】完全不符，疑似非真实回滚 SQL！", String.join(", ", rollbackTables), String.join(", ", execTables)))
                        .build();
            }
        }

        return RollbackValidationResult.builder()
                .valid(true)
                .matched(true)
                .execTables(execTables)
                .rollbackTables(rollbackTables)
                .message("回滚方案语法及目标表关联校验通过")
                .build();
    }
}
