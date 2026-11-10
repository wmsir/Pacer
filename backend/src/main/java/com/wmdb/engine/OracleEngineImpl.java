package com.wmdb.engine;

import com.alibaba.druid.sql.SQLUtils;
import com.alibaba.druid.sql.ast.SQLStatement;
import com.alibaba.druid.sql.ast.statement.SQLDeleteStatement;
import com.alibaba.druid.sql.ast.statement.SQLSelectStatement;
import com.alibaba.druid.sql.ast.statement.SQLUpdateStatement;
import com.wmdb.model.DbInstance;
import org.springframework.stereotype.Component;

import java.util.List;

/**
 * Oracle 引擎插件实现类
 * <p>
 * 实现针对 Oracle 数据库方言的 AST 语法分析与拦截校验。
 * </p>
 *
 * @author wm
 */
@Component
public class OracleEngineImpl implements DbEnginePlugin {

    @Override
    public void preCheck(String script) {
        com.alibaba.druid.DbType dbType = com.alibaba.druid.DbType.oracle;
        List<SQLStatement> statements = SQLUtils.parseStatements(script, dbType);

        for (SQLStatement stmt : statements) {
            if (stmt instanceof SQLUpdateStatement) {
                SQLUpdateStatement updateStmt = (SQLUpdateStatement) stmt;
                if (updateStmt.getWhere() == null) {
                    throw new RuntimeException("AST Error [Oracle]: UPDATE statement without WHERE clause is strictly prohibited.");
                }
            } else if (stmt instanceof SQLDeleteStatement) {
                SQLDeleteStatement deleteStmt = (SQLDeleteStatement) stmt;
                if (deleteStmt.getWhere() == null) {
                    throw new RuntimeException("AST Error [Oracle]: DELETE statement without WHERE clause is strictly prohibited.");
                }
            } else if (stmt instanceof SQLSelectStatement) {
                // Simplified basic check for "SELECT *"
                String stmtString = SQLUtils.toSQLString(stmt, dbType).toUpperCase();
                if (stmtString.contains("SELECT *")) {
                    throw new RuntimeException("AST Error [Oracle]: SELECT * is prohibited for performance reasons.");
                }
            }
        }
    }

    @Override
    public void execute(DbInstance instance, String script) {
        // Implementation for execution handled centrally via dynamic-datasource
        System.out.println("Executing Oracle script on instance: " + instance.getName());
    }
}
