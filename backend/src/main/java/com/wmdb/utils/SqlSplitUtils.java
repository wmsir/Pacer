package com.wmdb.utils;

import java.util.ArrayList;
import java.util.List;

/**
 * SQL 语句拆分工具类
 * <p>
 * 处理字符串内部的分号，防止 naive split(";") 导致的错误。
 * </p>
 *
 * @author wm
 */
public class SqlSplitUtils {

    /**
     * 安全地按照分号拆分 SQL 语句，忽略字符串字面量中的分号
     */
    public static List<String> split(String script) {
        List<String> statements = new ArrayList<>();
        if (script == null || script.trim().isEmpty()) {
            return statements;
        }

        StringBuilder currentStmt = new StringBuilder();
        boolean inString = false;
        char stringChar = 0;

        for (int i = 0; i < script.length(); i++) {
            char c = script.charAt(i);

            if (inString) {
                if (c == stringChar) {
                    // Check for escaped quotes (e.g., '')
                    if (i + 1 < script.length() && script.charAt(i + 1) == stringChar) {
                        currentStmt.append(c);
                        currentStmt.append(script.charAt(i + 1));
                        i++; // skip next quote
                    } else {
                        inString = false;
                        currentStmt.append(c);
                    }
                } else {
                    currentStmt.append(c);
                }
            } else {
                if (c == '\'' || c == '"') {
                    inString = true;
                    stringChar = c;
                    currentStmt.append(c);
                } else if (c == ';') {
                    String stmt = currentStmt.toString().trim();
                    if (!stmt.isEmpty()) {
                        statements.add(stmt);
                    }
                    currentStmt.setLength(0);
                } else {
                    currentStmt.append(c);
                }
            }
        }

        String lastStmt = currentStmt.toString().trim();
        if (!lastStmt.isEmpty()) {
            statements.add(lastStmt);
        }

        return statements;
    }
}
