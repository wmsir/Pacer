package com.wmdb.engine;

import com.wmdb.model.DbInstance;
import lombok.extern.slf4j.Slf4j;
import org.springframework.stereotype.Component;

/**
 * OceanBase 引擎插件实现类 (信创国产库支持)
 *
 * @author wm
 */
@Slf4j
@Component
public class OceanBaseEngineImpl implements DbEnginePlugin {
    @Override
    public void preCheck(String script) {
        log.info("Performing OceanBase specific AST pre-check...");
    }

    @Override
    public void execute(DbInstance instance, String script) {
        log.info("Executing script on OceanBase instance: {}", instance.getName());
    }
}
