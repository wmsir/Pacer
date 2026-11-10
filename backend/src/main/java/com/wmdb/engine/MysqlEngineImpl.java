package com.wmdb.engine;

import com.alibaba.druid.sql.SQLUtils;
import com.alibaba.druid.sql.ast.SQLStatement;
import com.alibaba.druid.sql.ast.statement.SQLDeleteStatement;
import com.alibaba.druid.sql.ast.statement.SQLSelectStatement;
import com.alibaba.druid.sql.ast.statement.SQLUpdateStatement;
import com.alibaba.druid.sql.visitor.SchemaStatVisitor;
import com.wmdb.model.DbInstance;
import com.wmdb.exception.BusinessException;
import lombok.extern.slf4j.Slf4j;
import org.springframework.stereotype.Component;

import java.util.List;

/**
 * MySQL 引擎插件实现类
 * <p>
 * 采用 Alibaba Druid 解析 AST 语法树，对危险语句（如无 Where 范围的删改、Select *）进行强拦截。
 * </p>
 *
 * @author wm
 * @date 2023-10-25
 */
@Slf4j
@Component
public class MysqlEngineImpl implements DbEnginePlugin {

    /**
     * 预检 SQL 脚本
     *
     * @param script 待解析预检的 SQL
     * @throws RuntimeException 如果命中高危语法规则则抛出异常
     */
    @Override
    public void preCheck(String script) {
        com.alibaba.druid.DbType dbType = com.alibaba.druid.DbType.mysql;
        List<SQLStatement> statements = SQLUtils.parseStatements(script, dbType);

        for (SQLStatement stmt : statements) {
            if (stmt instanceof SQLUpdateStatement) {
                SQLUpdateStatement updateStmt = (SQLUpdateStatement) stmt;
                if (updateStmt.getWhere() == null) {
                    throw new BusinessException("A0400", "AST Error: 严禁执行无 WHERE 条件的 UPDATE 语句。");
                }
            } else if (stmt instanceof SQLDeleteStatement) {
                SQLDeleteStatement deleteStmt = (SQLDeleteStatement) stmt;
                if (deleteStmt.getWhere() == null) {
                    throw new BusinessException("A0400", "AST Error: 严禁执行无 WHERE 条件的 DELETE 语句。");
                }
            } else if (stmt instanceof SQLSelectStatement) {
                // A very basic check for "SELECT *"
                String stmtString = SQLUtils.toSQLString(stmt, dbType).toUpperCase();
                if (stmtString.contains("SELECT *")) {
                    throw new BusinessException("A0400", "AST Error: 严禁执行 SELECT * 查询。");
                }
            }
        }
    }

    /**
     * 模拟执行 SQL（实际可由中心化逻辑控制流式执行）
     *
     * @param instance 目标实例
     * @param script SQL 脚本
     */
    @Override
    public void execute(DbInstance instance, String script) {
        // Implementation for execution (to be handled dynamically in TicketService/Flowable callback)
        log.info("Executing script on instance: {}", instance.getName());
    }
}
