package com.wmdb.controller;

import com.wmdb.common.Result;
import com.wmdb.model.*;
import com.wmdb.service.DbInstanceService;
import io.swagger.v3.oas.annotations.Operation;
import io.swagger.v3.oas.annotations.tags.Tag;
import lombok.RequiredArgsConstructor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.web.bind.annotation.*;

import java.util.List;
import java.util.Map;

/**
 * 数据库实例控制器
 * <p>
 * 提供数据库实例管理、多库 (Schema) 详情与创建删除、会话 Processlist 与 Kill、账号权限运维、参数 Variables 查询、资源组绑定与连接测试。
 * </p>
 *
 * @author wm
 * @date 2023-10-25
 */
@Tag(name = "数据库实例管理", description = "提供数据库实例纳管、连接测试、库名管理、会话运维、账号管理与参数配置")
@Slf4j
@RestController
@RequestMapping("/api/v1/instance")
@RequiredArgsConstructor
public class DbInstanceController {

    private final DbInstanceService dbInstanceService;

    @Operation(summary = "获取实例列表（全量兼容）")
    @GetMapping("/list")
    public Result<List<DbInstance>> listInstances() {
        return Result.success(dbInstanceService.listInstances());
    }

    @Operation(summary = "分页查询数据库实例列表")
    @GetMapping("/page")
    public Result<com.wmdb.model.PageResultDTO<DbInstance>> pageInstances(
            @RequestParam(value = "page", defaultValue = "1") int page,
            @RequestParam(value = "size", defaultValue = "10") int size,
            @RequestParam(value = "keyword", required = false) String keyword,
            @RequestParam(value = "env", required = false) String env,
            @RequestParam(value = "dbType", required = false) String dbType,
            @RequestParam(value = "resourceGroup", required = false) String resourceGroup) {
        return Result.success(dbInstanceService.pageInstances(page, size, keyword, env, dbType, resourceGroup));
    }

    @Operation(summary = "获取所有可用资源组列表")
    @GetMapping("/resource-groups")
    public Result<List<String>> listAllResourceGroups() {
        return Result.success(dbInstanceService.listAllResourceGroups());
    }

    @Operation(summary = "获取所有支持的操作类型标签")
    @GetMapping("/tags")
    public Result<List<String>> listAllSupportedOps() {
        return Result.success(dbInstanceService.listAllSupportedOps());
    }

    @Operation(summary = "获取所有支持的操作类型列表 (别名)")
    @GetMapping("/supported-ops")
    public Result<List<String>> getSupportedOps() {
        return Result.success(dbInstanceService.listAllSupportedOps());
    }

    @Operation(summary = "获取实例标签预设列表")
    @GetMapping("/tag-presets")
    public Result<List<String>> listAllTagPresets() {
        return Result.success(dbInstanceService.listAllTagPresets());
    }

    @Operation(summary = "根据参数实时测试数据库连通性", description = "用于新建/编辑表单输入 IP、端口、账号、密码后进行实时连接探测")
    @PostMapping("/test-connection-params")
    public Result<DbInstanceTestResultDTO> testConnectionWithParams(@RequestBody DbInstanceTestRequestDTO request) {
        return Result.success(dbInstanceService.testConnectionWithParams(request));
    }

    @Operation(summary = "获取单实例详细配置")
    @GetMapping("/{id:\\d+}")
    public Result<DbInstance> getInstanceById(@PathVariable("id") Long id) {
        return Result.success(dbInstanceService.getInstanceById(id));
    }

    @Operation(summary = "测试已保存实例的连通性")
    @PostMapping("/{id:\\d+}/test-connection")
    public Result<DbInstanceTestResultDTO> testConnection(@PathVariable("id") Long id) {
        return Result.success(dbInstanceService.testConnection(id));
    }

    // ==========================================
    // 1. 数据库管理 (Database / Schema)
    // ==========================================

    @Operation(summary = "查询实例下所有数据库/Schema详细元数据")
    @GetMapping("/{id:\\d+}/databases-detail")
    public Result<List<DbSchemaDetailDTO>> listDatabasesDetail(@PathVariable("id") Long id) {
        return Result.success(dbInstanceService.listDatabasesDetail(id));
    }

    @Operation(summary = "在指定实例上创建新数据库")
    @PostMapping("/{id:\\d+}/databases/create")
    public Result<Void> createDatabase(@PathVariable("id") Long id, @RequestBody CreateDbRequestDTO request) {
        dbInstanceService.createDatabase(id, request);
        return Result.success(null);
    }

    @Operation(summary = "在指定实例上删除数据库")
    @DeleteMapping("/{id:\\d+}/databases/{dbName}")
    public Result<Void> dropDatabase(@PathVariable("id") Long id, @PathVariable("dbName") String dbName) {
        dbInstanceService.dropDatabase(id, dbName);
        return Result.success(null);
    }

    // ==========================================
    // 2. 会话管理 (Processlist & Kill)
    // ==========================================

    @Operation(summary = "查询实例当前活动会话与进程列表")
    @GetMapping("/{id:\\d+}/sessions")
    public Result<List<DbSessionDTO>> listSessions(@PathVariable("id") Long id) {
        return Result.success(dbInstanceService.listSessions(id));
    }

    @Operation(summary = "终止/Kill 指定会话进程")
    @PostMapping("/{id:\\d+}/sessions/{processId:\\d+}/kill")
    public Result<Void> killSession(@PathVariable("id") Long id, @PathVariable("processId") Long processId) {
        dbInstanceService.killSession(id, processId);
        return Result.success(null);
    }

    // ==========================================
    // 3. 数据库账号管理 (Accounts)
    // ==========================================

    @Operation(summary = "查询数据库实例账号与主机权限列表")
    @GetMapping("/{id:\\d+}/accounts")
    public Result<List<DbAccountDTO>> listAccounts(@PathVariable("id") Long id) {
        return Result.success(dbInstanceService.listAccounts(id));
    }

    @Operation(summary = "创建新数据库账号并授权")
    @PostMapping("/{id:\\d+}/accounts/create")
    public Result<Void> createAccount(@PathVariable("id") Long id, @RequestBody CreateAccountRequestDTO request) {
        dbInstanceService.createAccount(id, request);
        return Result.success(null);
    }

    @Operation(summary = "删除数据库账号")
    @DeleteMapping("/{id:\\d+}/accounts")
    public Result<Void> dropAccount(@PathVariable("id") Long id,
                                    @RequestParam("user") String user,
                                    @RequestParam(value = "host", required = false, defaultValue = "%") String host) {
        dbInstanceService.dropAccount(id, user, host);
        return Result.success(null);
    }

    @Operation(summary = "重置数据库账号密码")
    @PostMapping("/{id:\\d+}/accounts/reset-password")
    public Result<Void> resetAccountPassword(@PathVariable("id") Long id,
                                             @RequestBody Map<String, String> body) {
        String user = body.get("user");
        String host = body.getOrDefault("host", "%");
        String newPassword = body.get("newPassword");
        dbInstanceService.resetAccountPassword(id, user, host, newPassword);
        return Result.success(null);
    }

    // ==========================================
    // 4. 参数配置 (Variables)
    // ==========================================

    @Operation(summary = "查询数据库实例全局参数（全量兼容）")
    @GetMapping("/{id:\\d+}/variables")
    public Result<List<DbVariableDTO>> listVariables(@PathVariable("id") Long id,
                                                     @RequestParam(value = "keyword", required = false) String keyword) {
        return Result.success(dbInstanceService.listVariables(id, keyword));
    }

    @Operation(summary = "分页查询数据库实例全局参数")
    @GetMapping("/{id:\\d+}/variables/page")
    public Result<com.wmdb.model.PageResultDTO<DbVariableDTO>> pageVariables(
            @PathVariable("id") Long id,
            @RequestParam(value = "page", defaultValue = "1") int page,
            @RequestParam(value = "size", defaultValue = "20") int size,
            @RequestParam(value = "keyword", required = false) String keyword) {
        return Result.success(dbInstanceService.pageVariables(id, page, size, keyword));
    }

    // ==========================================
    // 5. 基础 Schema 与表结构
    // ==========================================

    @Operation(summary = "查询实例下所有可用数据库/Schema列表")
    @GetMapping("/{id:\\d+}/databases")
    public Result<List<String>> listDatabases(@PathVariable("id") Long id) {
        return Result.success(dbInstanceService.listDatabases(id));
    }

    @Operation(summary = "查询指定数据库下的数据表清单")
    @GetMapping("/{id:\\d+}/databases/{dbName}/tables")
    public Result<List<Map<String, Object>>> listTables(@PathVariable("id") Long id,
                                                        @PathVariable("dbName") String dbName) {
        return Result.success(dbInstanceService.listTables(id, dbName));
    }

    @Operation(summary = "查询指定数据表的字段结构与注释")
    @GetMapping("/{id:\\d+}/databases/{dbName}/tables/{tableName}/columns")
    public Result<List<Map<String, Object>>> getTableColumns(@PathVariable("id") Long id,
                                                             @PathVariable("dbName") String dbName,
                                                             @PathVariable("tableName") String tableName) {
        return Result.success(dbInstanceService.getTableColumns(id, dbName, tableName));
    }

    @Operation(summary = "保存数据库实例")
    @PostMapping("/save")
    public Result<Void> saveInstance(@RequestBody DbInstance instance) {
        dbInstanceService.saveInstance(instance);
        return Result.success(null);
    }

    @Operation(summary = "删除数据库实例")
    @DeleteMapping("/{id:\\d+}")
    public Result<Void> deleteInstance(@PathVariable("id") Long id) {
        dbInstanceService.deleteInstance(id);
        return Result.success(null);
    }

    @Operation(summary = "审核通过实例")
    @PostMapping("/{id:\\d+}/approve")
    public Result<Void> approveInstance(@PathVariable("id") Long id) {
        dbInstanceService.approveInstance(id);
        return Result.success(null);
    }

    @Operation(summary = "启用/禁用数据库实例")
    @PostMapping("/{id:\\d+}/toggle-status")
    public Result<Void> toggleStatus(@PathVariable("id") Long id) {
        dbInstanceService.toggleStatus(id);
        return Result.success(null);
    }
}
