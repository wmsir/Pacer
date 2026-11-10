package com.wmdb.engine;

import com.wmdb.model.DbInstance;

/**
 * 数据库引擎插件接口
 * <p>
 * 提供策略模式 SPI 标准接口，所有异构数据库引擎均需实现此接口，
 * 以保证系统架构防腐和高扩展性。
 * </p>
 *
 * @author wm
 * @date 2023-10-25
 */
public interface DbEnginePlugin {

    /**
     * 预检 SQL 脚本（如 AST 分析拦截）
     *
     * @param script 待预检的 SQL 脚本字符串
     */
    void preCheck(String script);

    /**
     * 执行 SQL 脚本
     *
     * @param instance 目标数据库实例信息
     * @param script 待执行的 SQL 脚本字符串
     */
    void execute(DbInstance instance, String script);
}
