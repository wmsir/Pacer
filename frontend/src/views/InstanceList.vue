<template>
  <div class="instance-list-container page-container">
    <!-- 多页签工作台模式 (多任务并发切换与随时关闭) -->
    <el-tabs
      v-model="activeTab"
      type="card"
      class="instance-workbench-tabs"
      @tab-remove="handleCloseTab"
    >
      <!-- 主页签 1: 实例资产列表 -->
      <el-tab-pane name="list" :closable="false">
        <template #label>
          <div class="tab-label-item">
            <el-icon color="#409EFF"><Coin /></el-icon>
            <span>数据库实例资产 (Instances)</span>
          </div>
        </template>

        <div class="tab-inner-content">
          <div class="header-action">
            <div>
              <h2 class="page-title">数据库实例管理</h2>
              <div class="page-subtitle">配置和纳管 MySQL、达梦、PostgreSQL 等异构数据源，支持资源组授权、参数安全管控与工单扩展字段定制</div>
            </div>
            <div class="action-btns">
              <el-button type="warning" plain :icon="Share" @click="goToWorkflowDesigner">
                审批流设计中心
              </el-button>
              <el-input
                v-model="searchQuery"
                placeholder="搜索实例名称/环境/类型/资源组..."
                clearable
                style="width: 240px; margin-right: 12px; margin-left: 8px;"
                :prefix-icon="Search"
              />
              <el-button @click="fetchInstances" :icon="Refresh">刷新</el-button>
              <el-button type="primary" @click="handleOpenCreateTab" :icon="Plus">新增实例</el-button>
            </div>
          </div>

          <!-- 审批流快捷引导 Banner -->
          <div class="instance-workflow-tip-banner">
            <el-alert
              type="info"
              :closable="false"
              show-icon
              style="margin-bottom: 16px; border: 1px solid #e0e7ff; background: #f5f3ff;"
            >
              <template #title>
                <div style="display: flex; align-items: center; justify-content: space-between; width: 100%;">
                  <span style="font-weight: 600; color: #4338ca;">💡 流程设计指引：没有符合当前业务需要的审批流程？</span>
                  <el-button type="primary" link :icon="Share" @click="goToWorkflowDesigner" style="font-weight: 600;">
                    前往【审批流设计中心】新建/编辑 BPMN 审批流程 ➔
                  </el-button>
                </div>
              </template>
            </el-alert>
          </div>

          <div class="table-wrapper">
            <el-table :data="pagedInstances" border stripe style="width: 100%" v-loading="loading">
              <el-table-column prop="id" label="ID" width="70" align="center"></el-table-column>

              <el-table-column prop="name" label="实例名称" min-width="170" show-overflow-tooltip>
                <template #default="scope">
                  <div style="display: flex; align-items: center; flex-wrap: wrap; gap: 4px;">
                    <el-link type="primary" :underline="false" style="font-weight: 600;" @click="handleOpenConfigTab(scope.row)">
                      {{ scope.row.name }}
                    </el-link>
                    <el-tooltip
                      v-if="scope.row.fixedWorkflowTemplateName"
                      :content="`此实例已配置专属固定审批流【${scope.row.fixedWorkflowTemplateName}】，所有工单均走此固定流`"
                      placement="top"
                    >
                      <el-tag size="small" type="warning" effect="dark" style="font-size: 10px; height: 18px; line-height: 16px; padding: 0 4px;">
                        固定流
                      </el-tag>
                    </el-tooltip>
                  </div>
                  <div v-if="scope.row.description" style="font-size: 11px; color: #909399; line-height: 1.2; margin-top: 2px;">
                    {{ scope.row.description }}
                  </div>
                </template>
              </el-table-column>

              <el-table-column prop="dbType" label="引擎类型" width="120" align="center">
                <template #default="scope">
                  <el-tag size="small" :type="getDbTypeTagType(scope.row.dbType)">
                    {{ formatDbType(scope.row.dbType) }}
                  </el-tag>
                </template>
              </el-table-column>

              <el-table-column prop="env" label="运行环境" width="100" align="center">
                <template #default="scope">
                  <el-tag size="small" :type="scope.row.env === 'PROD' ? 'danger' : (scope.row.env === 'TEST' ? 'warning' : 'info')">
                    {{ scope.row.env }}
                  </el-tag>
                </template>
              </el-table-column>

              <!-- 关联资源组列 -->
              <el-table-column label="关联资源组" min-width="180">
                <template #default="scope">
                  <div class="tag-cell-wrap">
                    <template v-if="parseResourceGroups(scope.row.resourceGroups).length > 0">
                      <el-tag
                        v-for="(rg, idx) in parseResourceGroups(scope.row.resourceGroups).slice(0, 2)"
                        :key="idx"
                        size="small"
                        type="warning"
                        effect="plain"
                        style="margin: 2px 4px 2px 0;"
                      >
                        {{ rg }}
                      </el-tag>
                      <el-tooltip
                        v-if="parseResourceGroups(scope.row.resourceGroups).length > 2"
                        :content="parseResourceGroups(scope.row.resourceGroups).join('、')"
                        placement="top"
                      >
                        <el-tag size="small" type="info" style="cursor: pointer;">
                          +{{ parseResourceGroups(scope.row.resourceGroups).length - 2 }}
                        </el-tag>
                      </el-tooltip>
                    </template>
                    <span v-else style="color: #c0c4cc; font-size: 12px;">全员默认业务组</span>
                  </div>
                </template>
              </el-table-column>

              <!-- 实例业务标签列 -->
              <el-table-column label="实例业务标签" min-width="170">
                <template #default="scope">
                  <div class="tag-cell-wrap">
                    <template v-if="parseTags(scope.row.tags).length > 0">
                      <el-tag
                        v-for="(t, idx) in parseTags(scope.row.tags).slice(0, 2)"
                        :key="idx"
                        size="small"
                        :type="getTagColorType(t)"
                        effect="light"
                        style="margin: 2px 4px 2px 0;"
                      >
                        {{ t }}
                      </el-tag>
                      <el-tooltip
                        v-if="parseTags(scope.row.tags).length > 2"
                        :content="parseTags(scope.row.tags).join('、')"
                        placement="top"
                      >
                        <el-tag size="small" type="info" style="cursor: pointer;">
                          +{{ parseTags(scope.row.tags).length - 2 }}
                        </el-tag>
                      </el-tooltip>
                    </template>
                    <el-tag v-else size="small" type="info" effect="plain">默认业务库</el-tag>
                  </div>
                </template>
              </el-table-column>

              <!-- 支持的操作列 -->
              <el-table-column label="支持操作范围 (管控)" min-width="190">
                <template #default="scope">
                  <div class="tag-cell-wrap">
                    <template v-if="parseSupportedOps(scope.row.supportedOps).length > 0">
                      <el-tag
                        v-for="(op, idx) in parseSupportedOps(scope.row.supportedOps).slice(0, 3)"
                        :key="idx"
                        size="small"
                        :type="getOpTagType(op)"
                        style="margin: 2px 4px 2px 0;"
                      >
                        {{ op }}
                      </el-tag>
                      <el-tooltip
                        v-if="parseSupportedOps(scope.row.supportedOps).length > 3"
                        :content="parseSupportedOps(scope.row.supportedOps).join('、')"
                        placement="top"
                      >
                        <el-tag size="small" type="info" style="cursor: pointer;">
                          +{{ parseSupportedOps(scope.row.supportedOps).length - 3 }}
                        </el-tag>
                      </el-tooltip>
                    </template>
                    <template v-else>
                      <el-tag size="small" type="success">支持上线</el-tag>
                      <el-tag size="small" type="primary">支持查询</el-tag>
                    </template>
                  </div>
                </template>
              </el-table-column>

              <el-table-column prop="status" label="实例状态" width="100" align="center">
                <template #default="scope">
                  <el-tag :type="scope.row.status === 'APPROVED' ? 'success' : (scope.row.status === 'DISABLED' ? 'info' : 'warning')">
                    {{ scope.row.status === 'APPROVED' ? '启用上线' : (scope.row.status === 'DISABLED' ? '已下线禁用' : scope.row.status) }}
                  </el-tag>
                </template>
              </el-table-column>

              <el-table-column label="操作" width="310" fixed="right" align="center">
                <template #default="scope">
                  <!-- 参数与字段配置按钮 (核心高光) -->
                  <el-button
                    size="small"
                    type="primary"
                    plain
                    :icon="Setting"
                    @click="handleOpenConfigTab(scope.row)"
                  >
                    参数配置
                  </el-button>
                  <!-- 测试连接按钮 -->
                  <el-button
                    size="small"
                    type="success"
                    plain
                    :icon="Connection"
                    :loading="testingId === scope.row.id"
                    @click="handleTestRowConnection(scope.row)"
                  >
                    测试连接
                  </el-button>
                  <el-button size="small" :icon="Edit" @click="handleOpenEditTab(scope.row)">编辑</el-button>
                  <el-button size="small" type="danger" plain :icon="Delete" @click="handleDelete(scope.row)">删除</el-button>
                </template>
              </el-table-column>
            </el-table>

            <!-- 分页控制栏 -->
            <div class="pagination-bar" style="display: flex; justify-content: flex-end; margin-top: 16px;">
              <el-pagination
                v-model:current-page="currentPage"
                v-model:page-size="pageSize"
                :page-sizes="[10, 20, 50, 100]"
                :total="filteredInstances.length"
                layout="total, sizes, prev, pager, next, jumper"
                background
              />
            </div>
          </div>
        </div>
      </el-tab-pane>

      <!-- 动态页签 (支持多实例参数配置、新建与编辑) -->
      <el-tab-pane
        v-for="tab in dynamicTabs"
        :key="tab.name"
        :name="tab.name"
        :closable="true"
      >
        <template #label>
          <div class="tab-label-item">
            <el-icon :color="tab.type === 'config' ? '#409EFF' : (tab.type === 'edit' ? '#E6A23C' : '#67C23A')">
              <component :is="tab.type === 'config' ? Setting : (tab.type === 'edit' ? Edit : Plus)" />
            </el-icon>
            <span>{{ tab.title }}</span>
          </div>
        </template>

        <div class="tab-inner-content edit-tab-content">
          <!-- 企业级一体化吸顶操作导航条 -->
          <div class="edit-nav-top-bar">
            <div class="nav-left">
              <el-button :icon="ArrowLeft" @click="activeTab = 'list'" plain size="default">
                返回实例列表
              </el-button>
              <div class="nav-title-group">
                <span class="nav-main-title">
                  {{ tab.type === 'config' ? `实例参数与工单字段配置：${tab.instanceName}` : (tab.type === 'edit' ? `编辑数据库实例：${tab.instanceName}` : '注册纳管新数据库实例') }}
                </span>
                <span class="nav-sub-desc">
                  {{ tab.type === 'config' ? '配置实例执行安全参数、工单定制扩展字段与必填约束规范' : '配置实例连接串、认证凭据、支持操作与资源组授权' }}
                </span>
              </div>
            </div>
            <div class="nav-actions" style="display: flex; align-items: center; gap: 10px;">
              <!-- 参数配置快速切换目标实例 -->
              <el-select
                v-if="tab.type === 'config'"
                :model-value="tab.instanceId"
                @change="handleSwitchConfigInstance"
                placeholder="快速切换目标实例"
                style="width: 220px;"
                size="default"
              >
                <el-option
                  v-for="inst in instances"
                  :key="inst.id"
                  :label="`${inst.name} (${inst.env || 'PROD'})`"
                  :value="inst.id"
                />
              </el-select>

              <el-button @click="handleCloseTab(tab.name)" :icon="CloseBold" plain type="info">
                关闭页签
              </el-button>
              <el-button
                type="primary"
                :icon="Check"
                :loading="tab.saveLoading"
                @click="handleSaveTab(tab)"
              >
                保存配置并生效
              </el-button>
            </div>
          </div>

          <!-- ========================================================= -->
          <!-- A. 参数配置工作台 (tab.type === 'config') -->
          <!-- ========================================================= -->
          <div v-if="tab.type === 'config'" class="config-workbench-flow">
            <!-- 1. 实例核心安全与执行参数卡片 -->
            <el-card shadow="hover" class="config-card mb-16">
              <template #header>
                <div class="card-header-title">
                  <span class="step-num-badge">1</span>
                  <el-icon color="#409EFF"><Tools /></el-icon>
                  <span>实例核心执行与安全管控参数 (Execution & Safety Parameters)</span>
                </div>
              </template>

              <el-form label-position="top" :model="tab.execParams">
                <el-row :gutter="24">
                  <el-col :span="8">
                    <el-form-item label="SQL 最大执行超时时间 (秒)">
                      <el-input-number v-model="tab.execParams.queryTimeout" :min="5" :max="3600" :step="10" style="width: 100%;" />
                      <div class="form-item-tip">超过此时间系统流式引擎将主动执行 Kill 中断，防止慢查询拖垮集群</div>
                    </el-form-item>
                  </el-col>
                  <el-col :span="8">
                    <el-form-item label="单次变更最大影响行数上限 (行)">
                      <el-input-number v-model="tab.execParams.maxAffectRows" :min="100" :max="500000" :step="1000" style="width: 100%;" />
                      <div class="form-item-tip">预检校验超过此限制时，将强制升级为 DBA 严格复审或阻断发布</div>
                    </el-form-item>
                  </el-col>
                  <el-col :span="8">
                    <el-form-item label="DQL 只读查询单次最大导出上限 (行)">
                      <el-input-number v-model="tab.execParams.maxExportRows" :min="500" :max="100000" :step="1000" style="width: 100%;" />
                      <div class="form-item-tip">保护只读从库与网络带宽，防止一次性拉取海量数据导致 OOM</div>
                    </el-form-item>
                  </el-col>
                </el-row>

                <el-row :gutter="24" style="margin-top: 10px;">
                  <el-col :span="8">
                    <el-form-item label="强制事务级预执行校验 (Enforce Dry-Run)">
                      <div class="switch-row">
                        <el-switch v-model="tab.execParams.enforceDryRun" active-text="强制必须通过" inactive-text="推荐但不阻断" />
                      </div>
                      <div class="form-item-tip">提交工单前是否必须成功运行事务 Rollback Dry-Run 校验并生成报告</div>
                    </el-form-item>
                  </el-col>
                  <el-col :span="8">
                    <el-form-item label="自动备份与逆向回滚语句生成 (Auto Backup)">
                      <div class="switch-row">
                        <el-switch v-model="tab.execParams.autoBackup" active-text="开启自动逆向回滚" inactive-text="关闭" />
                      </div>
                      <div class="form-item-tip">执行前自动暂存变更前镜像，生成反向补偿 SQL 以备快速回滚</div>
                    </el-form-item>
                  </el-col>
                  <el-col :span="8">
                    <el-form-item label="连接池最大活跃连接数 (Max Active Connections)">
                      <el-input-number v-model="tab.execParams.maxActiveConn" :min="5" :max="100" :step="5" style="width: 100%;" />
                      <div class="form-item-tip">平台连接此实例的并发 Session 连接池大小管控</div>
                    </el-form-item>
                  </el-col>
                </el-row>
              </el-form>
            </el-card>

            <!-- 2. 工单可配置化扩展字段与必填约束卡片 (核心需求) -->
            <el-card shadow="hover" class="config-card mb-16">
              <template #header>
                <div class="card-header-title" style="display: flex; justify-content: space-between; align-items: center; width: 100%;">
                  <div style="display: flex; align-items: center; gap: 8px;">
                    <span class="step-num-badge">2</span>
                    <el-icon color="#409EFF"><Tickets /></el-icon>
                    <span>工单可配置化扩展字段与必填约束 (Custom Form Fields & Constraints)</span>
                  </div>
                  <div style="display: flex; gap: 8px;">
                    <el-button size="small" :icon="Plus" type="primary" plain @click="handleAddCustomField(tab)">
                      添加自定义字段
                    </el-button>
                    <el-button size="small" :icon="Document" type="info" plain @click="handleLoadPresetFields(tab)">
                      载入标准预设字段包
                    </el-button>
                  </div>
                </div>
              </template>

              <div class="field-config-table-wrapper">
                <el-table :data="tab.formFieldList" border stripe style="width: 100%">
                  <el-table-column label="序号" width="60" align="center">
                    <template #default="{ $index }">{{ $index + 1 }}</template>
                  </el-table-column>

                  <el-table-column label="字段标识 (Key)" min-width="150">
                    <template #default="{ row }">
                      <el-input v-model="row.fieldKey" size="small" placeholder="如 releaseVersion" />
                    </template>
                  </el-table-column>

                  <el-table-column label="字段显示名称" min-width="160">
                    <template #default="{ row }">
                      <el-input v-model="row.fieldName" size="small" placeholder="如 上线发布版本号" />
                    </template>
                  </el-table-column>

                  <el-table-column label="字段输入类型" width="150" align="center">
                    <template #default="{ row }">
                      <el-select v-model="row.fieldType" size="small" style="width: 100%;">
                        <el-option label="单行文本 (TEXT)" value="TEXT" />
                        <el-option label="日期年月日 (DATE)" value="DATE" />
                        <el-option label="时间窗口 (TIME_RANGE)" value="TIME_RANGE" />
                        <el-option label="多行文本 (TEXTAREA)" value="TEXTAREA" />
                        <el-option label="下拉单选 (SELECT)" value="SELECT" />
                      </el-select>
                    </template>
                  </el-table-column>

                  <el-table-column label="输入占位提示 (Placeholder)" min-width="180">
                    <template #default="{ row }">
                      <el-input v-model="row.placeholder" size="small" placeholder="填写提示说明..." />
                    </template>
                  </el-table-column>

                  <el-table-column label="是否启用" width="90" align="center">
                    <template #default="{ row }">
                      <el-switch v-model="row.enabled" size="small" />
                    </template>
                  </el-table-column>

                  <el-table-column label="必填约束" width="90" align="center">
                    <template #default="{ row }">
                      <el-switch v-model="row.required" :disabled="!row.enabled" size="small" />
                    </template>
                  </el-table-column>

                  <el-table-column label="排序/操作" width="140" align="center">
                    <template #default="{ $index }">
                      <el-button
                        size="small"
                        link
                        type="primary"
                        :disabled="$index === 0"
                        @click="handleMoveField(tab, $index, -1)"
                      >
                        上移
                      </el-button>
                      <el-button
                        size="small"
                        link
                        type="primary"
                        :disabled="$index === tab.formFieldList.length - 1"
                        @click="handleMoveField(tab, $index, 1)"
                      >
                        下移
                      </el-button>
                      <el-button
                        size="small"
                        link
                        type="danger"
                        @click="handleDeleteField(tab, $index)"
                      >
                        删除
                      </el-button>
                    </template>
                  </el-table-column>
                </el-table>
              </div>
            </el-card>

            <!-- 3. 专属固定审批流与操作管控卡片 -->
            <el-card shadow="hover" class="config-card mb-16">
              <template #header>
                <div class="card-header-title">
                  <span class="step-num-badge">3</span>
                  <el-icon color="#409EFF"><Share /></el-icon>
                  <span>专属固定审批流与操作管控标签 (Workflow & Operations)</span>
                </div>
              </template>

              <el-form label-position="top">
                <el-form-item label="绑定专属固定审批流模板">
                  <el-select
                    v-model="tab.form.fixedWorkflowTemplateName"
                    placeholder="请选择专属固定审批流（选定后归属本实例的工单强制走此流程）"
                    clearable
                    style="width: 100%;"
                  >
                    <el-option label="🎯 不固定专属流 (根据变更类型和影响行数动态智能决策)" value="" />
                    <el-option
                      v-for="wf in workflowOptions"
                      :key="wf.name"
                      :label="wf.name"
                      :value="wf.name"
                    />
                  </el-select>
                </el-form-item>

                <el-form-item label="支持操作范围管控标签 (多选)">
                  <el-checkbox-group v-model="tab.supportedOpsList">
                    <el-checkbox-button
                      v-for="op in allSupportedOpsOptions"
                      :key="op"
                      :value="op"
                    >
                      {{ op }}
                    </el-checkbox-button>
                  </el-checkbox-group>
                </el-form-item>
              </el-form>
            </el-card>
          </div>

          <!-- ========================================================= -->
          <!-- B. 实例注册与编辑工作台 (tab.type === 'edit' || 'create') -->
          <!-- ========================================================= -->
          <div v-else class="instance-form-workbench">
            <el-card shadow="hover" class="config-card mb-16">
              <template #header>
                <div class="card-header-title">
                  <el-icon color="#409EFF"><Coin /></el-icon>
                  <span>实例基础连接与凭据信息 (Basic Connection & Auth)</span>
                </div>
              </template>

              <el-form label-position="top" :model="tab.form">
                <el-row :gutter="20">
                  <el-col :span="12">
                    <el-form-item label="实例名称" required>
                      <el-input v-model="tab.form.name" placeholder="如 阿里云RDS-核心生产库" />
                    </el-form-item>
                  </el-col>
                  <el-col :span="6">
                    <el-form-item label="数据库引擎" required>
                      <el-select v-model="tab.form.dbType" style="width: 100%;">
                        <el-option label="MySQL" value="mysql" />
                        <el-option label="达梦数据库 (DM8)" value="dameng" />
                        <el-option label="PostgreSQL" value="postgresql" />
                        <el-option label="Oracle" value="oracle" />
                        <el-option label="TiDB" value="tidb" />
                        <el-option label="OceanBase" value="oceanbase" />
                        <el-option label="人大金仓 (Kingbase)" value="kingbase" />
                        <el-option label="openGauss" value="opengauss" />
                      </el-select>
                    </el-form-item>
                  </el-col>
                  <el-col :span="6">
                    <el-form-item label="运行环境" required>
                      <el-select v-model="tab.form.env" style="width: 100%;">
                        <el-option label="生产环境 (PROD)" value="PROD" />
                        <el-option label="测试环境 (TEST)" value="TEST" />
                        <el-option label="开发环境 (DEV)" value="DEV" />
                      </el-select>
                    </el-form-item>
                  </el-col>
                </el-row>

                <el-row :gutter="20">
                  <el-col :span="14">
                    <el-form-item label="主机地址 (Host / IP)" required>
                      <el-input v-model="tab.form.host" placeholder="如 your-rds-instance.mysql.rds.aliyuncs.com" />
                    </el-form-item>
                  </el-col>
                  <el-col :span="4">
                    <el-form-item label="端口号" required>
                      <el-input-number v-model="tab.form.port" :min="1" :max="65535" style="width: 100%;" />
                    </el-form-item>
                  </el-col>
                  <el-col :span="6">
                    <el-form-item label="默认数据库名 (Schema)">
                      <el-input v-model="tab.form.databaseName" placeholder="如 huiqitong_erp" />
                    </el-form-item>
                  </el-col>
                </el-row>

                <el-row :gutter="20">
                  <el-col :span="12">
                    <el-form-item label="数据库账号" required>
                      <el-input v-model="tab.form.username" placeholder="数据库登录账号" />
                    </el-form-item>
                  </el-col>
                  <el-col :span="12">
                    <el-form-item label="数据库密码" required>
                      <el-input v-model="tab.form.passwordCipher" type="password" show-password placeholder="输入密码 (系统自动执行 SM4 加密存储)" />
                    </el-form-item>
                  </el-col>
                </el-row>

                <el-form-item label="业务用途与描述说明">
                  <el-input v-model="tab.form.description" type="textarea" :rows="2" placeholder="填写该数据库实例的业务用途说明..." />
                </el-form-item>
              </el-form>
            </el-card>
          </div>
        </div>
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { ElMessage, ElMessageBox, ElNotification } from 'element-plus'
import {
  Search, Refresh, Plus, Connection, Edit, Delete, Share,
  Setting, Tools, Tickets, Document, Check, ArrowLeft, CloseBold, Coin
} from '@element-plus/icons-vue'
import request from '../utils/request'

const router = useRouter()
const route = useRoute()

interface InstanceItem {
  id: number | null
  name: string
  dbType: string
  host?: string
  port?: number
  databaseName?: string
  jdbcUrl: string
  readOnlyJdbcUrl?: string
  username: string
  passwordCipher: string
  env: string
  status?: string
  resourceGroups?: string
  tags?: string
  supportedOps?: string
  fixedWorkflowTemplateId?: number
  fixedWorkflowTemplateName?: string
  description?: string
  tenantId?: string
  instanceConfig?: string
}

interface FormFieldConfig {
  fieldKey: string
  fieldName: string
  fieldType: 'TEXT' | 'DATE' | 'TIME_RANGE' | 'TEXTAREA' | 'SELECT'
  placeholder: string
  enabled: boolean
  required: boolean
}

interface ExecParamsConfig {
  queryTimeout: number
  maxAffectRows: number
  maxExportRows: number
  enforceDryRun: boolean
  autoBackup: boolean
  maxActiveConn: number
}

interface DynamicTabItem {
  name: string
  title: string
  type: 'config' | 'edit' | 'create'
  instanceId: number | null
  instanceName: string
  saveLoading: boolean
  form: InstanceItem
  execParams: ExecParamsConfig
  formFieldList: FormFieldConfig[]
  supportedOpsList: string[]
}

const activeTab = ref('list')
const dynamicTabs = ref<DynamicTabItem[]>([])

const instances = ref<InstanceItem[]>([])
const loading = ref(false)
const testingId = ref<number | null>(null)
const searchQuery = ref('')

const currentPage = ref(1)
const pageSize = ref(10)

const workflowOptions = ref<any[]>([])
const allSupportedOpsOptions = ref([
  '支持上线', '支持查询', '支持DML变更', '支持DDL结构变更',
  '支持数据导出', '支持事务预执行', '支持数据脱敏', '支持历史回滚'
])

// 标准默认工单扩展字段预设包
const defaultInstanceFields: FormFieldConfig[] = [
  { fieldKey: 'releaseVersion', fieldName: '上线发布版本号', fieldType: 'TEXT', placeholder: '如 v2026.08.22-release-01', enabled: true, required: true },
  { fieldKey: 'releaseDate', fieldName: '计划上线日期 (年月日)', fieldType: 'DATE', placeholder: '选择执行发布日期', enabled: true, required: false },
  { fieldKey: 'executionTimeRange', fieldName: '允许执行时间窗口', fieldType: 'TIME_RANGE', placeholder: '如 02:00 ~ 04:00 (夜间低峰)', enabled: true, required: false },
  { fieldKey: 'jiraIssueKey', fieldName: '关联需求Jira单号', fieldType: 'TEXT', placeholder: '如 PROD-9821', enabled: true, required: false },
  { fieldKey: 'rollbackPlan', fieldName: '应急回滚预案说明', fieldType: 'TEXTAREA', placeholder: '简述出现异常时的回滚策略与责任人', enabled: true, required: false }
]

const goToWorkflowDesigner = () => {
  router.push('/workflow-designer')
}

const filteredInstances = computed(() => {
  if (!searchQuery.value) return instances.value
  const q = searchQuery.value.toLowerCase()
  return instances.value.filter(item =>
    (item.name && item.name.toLowerCase().includes(q)) ||
    (item.env && item.env.toLowerCase().includes(q)) ||
    (item.dbType && item.dbType.toLowerCase().includes(q)) ||
    (item.jdbcUrl && item.jdbcUrl.toLowerCase().includes(q)) ||
    (item.tags && item.tags.toLowerCase().includes(q)) ||
    (item.supportedOps && item.supportedOps.toLowerCase().includes(q)) ||
    (item.resourceGroups && item.resourceGroups.toLowerCase().includes(q))
  )
})

const pagedInstances = computed(() => {
  const start = (currentPage.value - 1) * pageSize.value
  return filteredInstances.value.slice(start, start + pageSize.value)
})

watch(() => searchQuery.value, () => {
  currentPage.value = 1
})

const parseTags = (tagStr?: string): string[] => {
  if (!tagStr) return []
  try {
    const arr = JSON.parse(tagStr)
    return Array.isArray(arr) ? arr : []
  } catch (e) {
    return tagStr.split(/[,，]/).filter(Boolean)
  }
}

const getTagColorType = (tag: string) => {
  if (tag.includes('生产') || tag.includes('敏感') || tag.includes('核心')) return 'danger'
  if (tag.includes('只读') || tag.includes('从库')) return 'primary'
  if (tag.includes('测试')) return 'warning'
  if (tag.includes('集群')) return 'success'
  return 'info'
}

const getOpTagType = (op: string) => {
  if (op.includes('上线')) return 'success'
  if (op.includes('查询')) return 'primary'
  if (op.includes('DML')) return 'warning'
  if (op.includes('DDL')) return 'danger'
  if (op.includes('导出')) return 'info'
  return 'info'
}

const parseResourceGroups = (rgStr?: string): string[] => {
  if (!rgStr) return []
  try {
    const arr = JSON.parse(rgStr)
    return Array.isArray(arr) ? arr : []
  } catch (e) {
    return rgStr.split(',').filter(Boolean)
  }
}

const parseSupportedOps = (opStr?: string): string[] => {
  if (!opStr) return []
  try {
    const arr = JSON.parse(opStr)
    return Array.isArray(arr) ? arr : []
  } catch (e) {
    return opStr.split(',').filter(Boolean)
  }
}

const formatDbType = (type: string) => {
  const map: Record<string, string> = {
    mysql: 'MySQL',
    dameng: '达梦 DM8',
    postgresql: 'PostgreSQL',
    oracle: 'Oracle',
    tidb: 'TiDB',
    oceanbase: 'OceanBase',
    kingbase: '人大金仓',
    opengauss: 'openGauss'
  }
  return map[type?.toLowerCase()] || type || 'MySQL'
}

const getDbTypeTagType = (type: string) => {
  const t = type?.toLowerCase()
  if (t === 'mysql' || t === 'tidb' || t === 'oceanbase') return 'primary'
  if (t === 'dameng' || t === 'kingbase') return 'warning'
  if (t === 'postgresql' || t === 'opengauss') return 'success'
  if (t === 'oracle') return 'danger'
  return 'info'
}

const fetchWorkflowTemplates = async () => {
  try {
    const res: any = await request.get('/v1/workflow/template/list')
    workflowOptions.value = Array.isArray(res.data) ? res.data : []
  } catch (e) {}
}

const fetchInstances = async () => {
  loading.value = true
  try {
    const res: any = await request.get('/v1/instance/list')
    instances.value = Array.isArray(res.data) ? res.data : []
    checkRouteForTab()
  } catch (error) {
    ElMessage.error('获取实例列表失败')
  } finally {
    loading.value = false
  }
}

const checkRouteForTab = () => {
  if (route.path === '/instance-config' || route.query.tab === 'config') {
    const targetId = route.query.instanceId ? Number(route.query.instanceId) : (instances.value[0]?.id || null)
    if (targetId) {
      const targetInst = instances.value.find(i => i.id === targetId) || instances.value[0]
      if (targetInst) {
        handleOpenConfigTab(targetInst)
      }
    }
  }
}

const handleSwitchConfigInstance = (newId: number) => {
  const target = instances.value.find(i => i.id === newId)
  if (target) {
    handleOpenConfigTab(target)
  }
}

// 表格操作列测试连接
const handleTestRowConnection = async (row: InstanceItem) => {
  if (!row.id) return
  testingId.value = row.id
  try {
    const res: any = await request.post(`/v1/instance/${row.id}/test-connection`)
    if (res.data?.success) {
      ElNotification({
        title: '连接测试成功',
        message: `实例【${row.name}】连接正常！\n引擎：${res.data.databaseProductName} ${res.data.databaseProductVersion || ''}\n响应耗时：${res.data.latencyMs}ms`,
        type: 'success',
        duration: 4500
      })
    } else {
      ElNotification({
        title: '连接测试失败',
        message: `实例【${row.name}】连接异常：${res.data?.errorMessage || res.data?.message || '无法连接'}`,
        type: 'error',
        duration: 6000
      })
    }
  } catch (err: any) {
    ElNotification({
      title: '连接测试异常',
      message: err.response?.data?.message || err.message || '网络请求错误',
      type: 'error',
      duration: 6000
    })
  } finally {
    testingId.value = null
  }
}

// 打开「参数与工单扩展字段配置」页签
const handleOpenConfigTab = (row: InstanceItem) => {
  const tabName = `config_${row.id}`
  const existing = dynamicTabs.value.find(t => t.name === tabName)
  if (existing) {
    activeTab.value = tabName
    return
  }

  let execParams: ExecParamsConfig = {
    queryTimeout: 60,
    maxAffectRows: 5000,
    maxExportRows: 10000,
    enforceDryRun: true,
    autoBackup: true,
    maxActiveConn: 20
  }

  let formFieldList: FormFieldConfig[] = JSON.parse(JSON.stringify(defaultInstanceFields))

  if (row.instanceConfig) {
    try {
      const parsed = JSON.parse(row.instanceConfig)
      if (parsed && typeof parsed === 'object') {
        if (parsed.execParams) execParams = { ...execParams, ...parsed.execParams }
        if (Array.isArray(parsed.formFieldList) && parsed.formFieldList.length > 0) {
          formFieldList = parsed.formFieldList
        }
      }
    } catch (e) {}
  }

  let opsList = parseSupportedOps(row.supportedOps)
  if (opsList.length === 0) {
    opsList = [...allSupportedOpsOptions.value]
  }

  const newTab: DynamicTabItem = {
    name: tabName,
    title: `⚙️ 参数配置：${row.name}`,
    type: 'config',
    instanceId: row.id,
    instanceName: row.name,
    saveLoading: false,
    form: { ...row },
    execParams,
    formFieldList,
    supportedOpsList: opsList
  }

  dynamicTabs.value.push(newTab)
  activeTab.value = tabName
}

// 打开「编辑实例」页签
const handleOpenEditTab = (row: InstanceItem) => {
  const tabName = `edit_${row.id}`
  const existing = dynamicTabs.value.find(t => t.name === tabName)
  if (existing) {
    activeTab.value = tabName
    return
  }

  const newTab: DynamicTabItem = {
    name: tabName,
    title: `✏️ 编辑：${row.name}`,
    type: 'edit',
    instanceId: row.id,
    instanceName: row.name,
    saveLoading: false,
    form: { ...row },
    execParams: {
      queryTimeout: 60,
      maxAffectRows: 5000,
      maxExportRows: 10000,
      enforceDryRun: true,
      autoBackup: true,
      maxActiveConn: 20
    },
    formFieldList: JSON.parse(JSON.stringify(defaultInstanceFields)),
    supportedOpsList: parseSupportedOps(row.supportedOps)
  }

  dynamicTabs.value.push(newTab)
  activeTab.value = tabName
}

// 打开「新增实例」页签
const handleOpenCreateTab = () => {
  const tabName = 'create_instance_tab'
  const existing = dynamicTabs.value.find(t => t.name === tabName)
  if (existing) {
    activeTab.value = tabName
    return
  }

  const newTab: DynamicTabItem = {
    name: tabName,
    title: '➕ 注册纳管新实例',
    type: 'create',
    instanceId: null,
    instanceName: '新数据库实例',
    saveLoading: false,
    form: {
      id: null,
      name: '',
      dbType: 'mysql',
      host: '',
      port: 3306,
      databaseName: '',
      jdbcUrl: '',
      readOnlyJdbcUrl: '',
      username: 'root',
      passwordCipher: '',
      env: 'PROD',
      status: 'APPROVED',
      resourceGroups: '["默认核心业务资源组"]',
      tags: '["核心生产库"]',
      supportedOps: '["支持上线","支持查询","支持DML变更"]',
      description: ''
    },
    execParams: {
      queryTimeout: 60,
      maxAffectRows: 5000,
      maxExportRows: 10000,
      enforceDryRun: true,
      autoBackup: true,
      maxActiveConn: 20
    },
    formFieldList: JSON.parse(JSON.stringify(defaultInstanceFields)),
    supportedOpsList: [...allSupportedOpsOptions.value]
  }

  dynamicTabs.value.push(newTab)
  activeTab.value = tabName
}

// 关闭指定页签
const handleCloseTab = (tabName: string) => {
  const index = dynamicTabs.value.findIndex(t => t.name === tabName)
  if (index > -1) {
    dynamicTabs.value.splice(index, 1)
    if (activeTab.value === tabName) {
      if (dynamicTabs.value.length > 0) {
        const nextTab = dynamicTabs.value[Math.max(0, index - 1)]
        activeTab.value = nextTab.name
      } else {
        activeTab.value = 'list'
      }
    }
  }
}

// 自定义字段操作
const handleAddCustomField = (tab: DynamicTabItem) => {
  tab.formFieldList.push({
    fieldKey: `customField_${Date.now().toString().slice(-4)}`,
    fieldName: '新增扩展属性',
    fieldType: 'TEXT',
    placeholder: '请输入...',
    enabled: true,
    required: false
  })
}

const handleLoadPresetFields = (tab: DynamicTabItem) => {
  tab.formFieldList = JSON.parse(JSON.stringify(defaultInstanceFields))
  ElMessage.success('已载入标准工单扩展字段预设包')
}

const handleMoveField = (tab: DynamicTabItem, index: number, step: number) => {
  const target = index + step
  if (target < 0 || target >= tab.formFieldList.length) return
  const temp = tab.formFieldList[index]
  tab.formFieldList[index] = tab.formFieldList[target]
  tab.formFieldList[target] = temp
}

const handleDeleteField = (tab: DynamicTabItem, index: number) => {
  tab.formFieldList.splice(index, 1)
}

// 保存页签配置
const handleSaveTab = async (tab: DynamicTabItem) => {
  tab.saveLoading = true
  try {
    const payload = { ...tab.form }
    payload.supportedOps = JSON.stringify(tab.supportedOpsList)

    // 打包 instanceConfig
    const configObj = {
      execParams: tab.execParams,
      formFieldList: tab.formFieldList
    }
    payload.instanceConfig = JSON.stringify(configObj)

    await request.post('/v1/instance/save', payload)
    ElMessage.success(`实例【${payload.name || '配置'}】保存并生效成功！`)

    await fetchInstances()
    handleCloseTab(tab.name)
  } catch (err: any) {
    ElMessage.error(err.response?.data?.message || err.message || '保存失败')
  } finally {
    tab.saveLoading = false
  }
}

const handleDelete = async (row: any) => {
  try {
    await ElMessageBox.confirm(`确认删除实例【${row.name}】吗？删除后不可恢复！`, '警告', {
      type: 'warning',
      confirmButtonText: '确认删除',
      cancelButtonText: '取消'
    })
    await request.delete(`/v1/instance/${row.id}`)
    ElMessage.success('删除成功')
    fetchInstances()
  } catch (error) {
    if (error !== 'cancel') {
      ElMessage.error('删除失败')
    }
  }
}

watch(() => [route.path, route.query], () => {
  if (instances.value.length > 0) {
    checkRouteForTab()
  }
})

onMounted(() => {
  fetchInstances()
  fetchWorkflowTemplates()
})
</script>

<style scoped>
.instance-list-container {
  width: 100%;
}

.instance-workbench-tabs {
  width: 100%;
}

.instance-workbench-tabs :deep(.el-tabs__content) {
  overflow: visible !important;
}

/* ==================== 一体化置顶工作台头 ==================== */
.instance-workbench-tabs > :deep(.el-tabs__header) {
  position: sticky;
  top: 0px;
  z-index: 100;
  margin-bottom: 0px;
  background: #ffffff;
  border-radius: 8px 8px 0 0;
  padding: 8px 14px 0 14px;
  border: 1px solid #e2e8f0;
  border-bottom: none;
}

.instance-workbench-tabs :deep(.el-tabs__item) {
  font-weight: 600;
  font-size: 13px;
  transition: all 0.2s ease;
  background: #f8fafc;
  margin-right: 4px;
  border-radius: 6px 6px 0 0;
  border: 1px solid #e2e8f0;
  border-bottom: none;
}

.instance-workbench-tabs :deep(.el-tabs__item.is-active) {
  background: #ffffff;
  color: #3b82f6;
  border-color: #cbd5e1;
}

.tab-label-item {
  display: flex;
  align-items: center;
  gap: 6px;
}

.tab-inner-content {
  background: transparent;
}

.header-action {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-top: 14px;
  margin-bottom: 16px;
  background: #ffffff;
  padding: 16px 20px;
  border-radius: 8px;
  border: 1px solid #e2e8f0;
  flex-wrap: wrap;
  gap: 12px;
}

.page-title {
  font-size: 18px;
  font-weight: 700;
  color: #1e293b;
  margin: 0 0 4px 0;
}

.page-subtitle {
  font-size: 12px;
  color: #64748b;
}

.action-btns {
  display: flex;
  align-items: center;
}

.table-wrapper {
  background: #ffffff;
  border-radius: 8px;
  padding: 16px;
  border: 1px solid #e2e8f0;
}

.tag-cell-wrap {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
}

/* ==================== 编辑与配置页签特有样式 ==================== */
.edit-tab-content {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.edit-nav-top-bar {
  position: sticky;
  top: 42px;
  z-index: 99;
  display: flex;
  justify-content: space-between;
  align-items: center;
  background: #ffffff;
  padding: 10px 16px;
  border-radius: 0 0 8px 8px;
  border: 1px solid #e2e8f0;
  border-top: 1px solid #f1f5f9;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.05);
  margin-bottom: 4px;
}

.nav-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.nav-title-group {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.nav-main-title {
  font-size: 15px;
  font-weight: 700;
  color: #1e293b;
}

.nav-sub-desc {
  font-size: 11.5px;
  color: #64748b;
}

.nav-actions {
  display: flex;
  align-items: center;
  gap: 10px;
}

.config-card {
  border: 1px solid #e2e8f0;
  border-radius: 8px;
  background: #ffffff;
}

.card-header-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14.5px;
  font-weight: 700;
  color: #1e293b;
}

.step-num-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  background: #3b82f6;
  color: #ffffff;
  border-radius: 50%;
  font-size: 11px;
  font-weight: 800;
}

.form-item-tip {
  font-size: 11.5px;
  color: #94a3b8;
  margin-top: 4px;
  line-height: 1.3;
}

.switch-row {
  display: flex;
  align-items: center;
  height: 32px;
}

.mb-16 {
  margin-bottom: 16px;
}
</style>
