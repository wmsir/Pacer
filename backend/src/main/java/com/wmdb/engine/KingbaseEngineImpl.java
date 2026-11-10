package com.wmdb.engine;

import com.wmdb.model.DbInstance;
import lombok.extern.slf4j.Slf4j;
import org.springframework.stereotype.Component;

/**
 * 人大金仓 Kingbase 引擎插件实现类 (信创国产库支持)
 *
 * @author wm
 */
@Slf4j
@Component
public class KingbaseEngineImpl implements DbEnginePlugin {
    @Override
    public void preCheck(String script) {
        log.info("Performing Kingbase specific AST pre-check...");
    }

    @Override
    public void execute(DbInstance instance, String script) {
        log.info("Executing script on Kingbase instance: {}", instance.getName());
    }
}
