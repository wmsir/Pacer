<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from '../i18n';
import type {
  ColumnMeta,
  ConnectionProfile,
  DatabaseMeta,
  IndexMeta,
  TableMeta,
  WorkspaceProject,
} from '../api/ipc';
import IdeIcon, { type IdeIconName } from './IdeIcon.vue';

const props = defineProps<{
  projects: WorkspaceProject[];
  activeProjectId: string;
  expandedProjects: Record<string, boolean>;
  connections: ConnectionProfile[];
  activeProfileId: string;
  activeDbName: string;
  databasesByProfile: Record<string, DatabaseMeta[]>;
  tablesByKey: Record<string, TableMeta[]>;
  columnsByKey: Record<string, ColumnMeta[]>;
  indexesByKey: Record<string, IndexMeta[]>;
  expandedProfiles: Record<string, boolean>;
  expandedDatabases: Record<string, boolean>;
  expandedTables: Record<string, boolean>;
}>();

const emit = defineEmits<{
  (e: 'new-connection', projectId?: string): void;
  (e: 'new-project'): void;
  (e: 'edit-project', project: WorkspaceProject): void;
  (e: 'delete-project', projectId: string): void;
  (e: 'toggle-project', project: WorkspaceProject): void;
  (e: 'collapse-all-tree'): void;
  (e: 'move-profile-project', profile: ConnectionProfile, projectId: string): void;
  (e: 'import-dbeaver', projectId?: string): void;
  (e: 'init-demo', projectId?: string): void;
  (e: 'toggle-profile', profile: ConnectionProfile): void;
  (e: 'toggle-database', profile: ConnectionProfile, dbName: string): void;
  (e: 'toggle-table', profile: ConnectionProfile, dbName: string, tableName: string): void;
  (e: 'select-table', profile: ConnectionProfile, dbName: string, tableName: string): void;
  (e: 'query-top100', profile: ConnectionProfile, dbName: string, tableName: string): void;
  (e: 'view-ddl', profile: ConnectionProfile, dbName: string, tableName: string): void;
  (e: 'gen-select', profile: ConnectionProfile, dbName: string, tableName: string): void;
  (e: 'test-profile', profile: ConnectionProfile): void;
  (e: 'edit-profile', profile: ConnectionProfile): void;
  (e: 'delete-profile', profile: ConnectionProfile): void;
  (e: 'copy-text', text: string): void;
}>();

const { t } = useI18n();
const search = ref('');
const selectedNodeKey = ref('');

const ctxMenu = ref<{
  visible: boolean;
  x: number;
  y: number;
  kind: 'project' | 'profile' | 'table';
  project?: WorkspaceProject;
  profile?: ConnectionProfile;
  dbName?: string;
  tableName?: string;
}>({
  visible: false,
  x: 0,
  y: 0,
  kind: 'profile',
});

function engineShort(dbType: string | Record<string, string>): string {
  const raw = typeof dbType === 'string' ? dbType : Object.values(dbType)[0] ?? 'DB';
  const norm = raw.toLowerCase();
  if (norm.includes('tidb')) return 'TiDB';
  if (norm.includes('sqlite')) return 'SQLite';
  if (norm.includes('postgres')) return 'PG';
  if (norm.includes('mysql')) return 'MySQL';
  if (norm.includes('maria')) return 'MariaDB';
  if (norm.includes('oracle')) return 'Oracle';
  if (norm.includes('dameng') || norm === 'dm') return 'DM8';
  if (norm.includes('kingbase')) return 'KB';
  if (norm.includes('gauss')) return 'OG';
  if (norm.includes('oceanbase')) return 'OB';
  if (norm.includes('sqlserver')) return 'MSSQL';
  return raw.toUpperCase();
}

function engineIcon(dbType: string | Record<string, string>): IdeIconName {
  const raw = typeof dbType === 'string' ? dbType : Object.values(dbType)[0] ?? '';
  const norm = raw.toLowerCase();
  if (norm.includes('tidb')) return 'db-tidb';
  if (norm.includes('sqlite')) return 'db-sqlite';
  if (norm.includes('postgres') || norm.includes('kingbase') || norm.includes('gauss')) return 'db-postgres';
  if (norm.includes('mysql') || norm.includes('maria') || norm.includes('oceanbase')) return 'db-mysql';
  return 'db-generic';
}

function connectionsInProject(projectId: string): ConnectionProfile[] {
  const isFirst = props.projects[0]?.id === projectId;
  const list = props.connections.filter(
    (c) => c.projectId === projectId || (isFirst && !c.projectId),
  );
  const q = search.value.trim().toLowerCase();
  if (!q) return list;
  return list.filter((p) => {
    if (p.name.toLowerCase().includes(q)) return true;
    const dbs = props.databasesByProfile[p.id] ?? [];
    for (const db of dbs) {
      if (db.name.toLowerCase().includes(q)) return true;
      const tbls = props.tablesByKey[`${p.id}:${db.name}`] ?? [];
      if (tbls.some((tb) => tb.name.toLowerCase().includes(q))) return true;
    }
    return false;
  });
}

const visibleProjects = computed(() => {
  const q = search.value.trim().toLowerCase();
  if (!q) return props.projects;
  return props.projects.filter(
    (proj) =>
      proj.name.toLowerCase().includes(q) || connectionsInProject(proj.id).length > 0,
  );
});

function filterTables(profileId: string, dbName: string): TableMeta[] {
  const list = props.tablesByKey[`${profileId}:${dbName}`] ?? [];
  const q = search.value.trim().toLowerCase();
  if (!q) return list;
  const matched = list.filter((tb) => tb.name.toLowerCase().includes(q));
  return matched.length > 0 ? matched : list;
}

function openProjectContext(ev: MouseEvent, project: WorkspaceProject) {
  ev.preventDefault();
  ctxMenu.value = {
    visible: true,
    x: ev.clientX,
    y: ev.clientY,
    kind: 'project',
    project,
  };
}

function openProfileContext(ev: MouseEvent, profile: ConnectionProfile) {
  ev.preventDefault();
  ctxMenu.value = {
    visible: true,
    x: ev.clientX,
    y: ev.clientY,
    kind: 'profile',
    profile,
  };
}

function openTableContext(
  ev: MouseEvent,
  profile: ConnectionProfile,
  dbName: string,
  tableName: string,
) {
  ev.preventDefault();
  ctxMenu.value = {
    visible: true,
    x: ev.clientX,
    y: ev.clientY,
    kind: 'table',
    profile,
    dbName,
    tableName,
  };
}

function closeContext() {
  ctxMenu.value.visible = false;
}
</script>

<template>
  <div style="display: flex; flex-direction: column; width: 100%; height: 100%" @click="closeContext">
    <div class="tw-header">
      <span style="display: inline-flex; align-items: center; gap: 6px">
        <IdeIcon name="database" :size="14" color="#4D9BFF" />
        <span>{{ t('toolDatabase') }}</span>
      </span>
      <div class="tw-actions">
        <button class="icon-btn compact" :title="t('newProject')" @click="emit('new-project')">
          <IdeIcon name="project-add" :size="14" />
        </button>
        <button class="icon-btn compact" :title="t('newConnection')" @click="emit('new-connection', activeProjectId)">
          <IdeIcon name="plus" :size="14" />
        </button>
        <button class="icon-btn compact" :title="t('importDbeaver')" @click="emit('import-dbeaver', activeProjectId)">
          <IdeIcon name="import" :size="14" />
        </button>
        <button class="icon-btn compact" title="全部折叠" @click="emit('collapse-all-tree')">
          <IdeIcon name="collapse-all" :size="13" />
        </button>
        <button class="icon-btn compact" :title="t('openDemoDb')" @click="emit('init-demo', activeProjectId)">
          <IdeIcon name="bolt" :size="13" />
        </button>
      </div>
    </div>

    <div class="tw-search">
      <input
        v-model="search"
        class="field"
        placeholder="搜索项目、数据源、库名或表名..."
      />
    </div>

    <div class="tw-body">
      <div v-if="visibleProjects.length === 0" class="empty-state">
        <div>暂无匹配的项目或数据源</div>
        <div style="display: flex; gap: 6px; margin-top: 8px; flex-wrap: wrap">
          <button class="btn btn-primary btn-sm" @click="emit('new-project')">
            <IdeIcon name="project-add" :size="12" />
            {{ t('newProject') }}
          </button>
        </div>
      </div>

      <!-- Root Level: Collapsible Workspace Projects (Folder-style) -->
      <div v-for="proj in visibleProjects" :key="proj.id" style="margin-bottom: 3px">
        <div
          class="tree-row project-tree-row"
          :class="{
            'project-active': proj.id === activeProjectId,
            selected: selectedNodeKey === `proj:${proj.id}`,
          }"
          :title="proj.description || proj.name"
          @click="
            selectedNodeKey = `proj:${proj.id}`;
            emit('toggle-project', proj);
          "
          @contextmenu="openProjectContext($event, proj)"
        >
          <span class="tree-chevron">
            <IdeIcon :name="expandedProjects[proj.id] ? 'chevron-down' : 'chevron-right'" :size="12" />
          </span>
          <IdeIcon
            :name="expandedProjects[proj.id] ? 'project-open' : 'project'"
            :size="15"
            :color="proj.color || '#3574F0'"
          />
          <span class="tree-label" style="font-weight: 700">{{ proj.name }}</span>
          <span class="pill pill-neutral">{{ connectionsInProject(proj.id).length }}</span>
          <span class="tree-hover-actions" @click.stop>
            <button
              class="icon-btn compact"
              title="在此项目新建数据源"
              @click="emit('new-connection', proj.id)"
            >
              <IdeIcon name="plus" :size="12" />
            </button>
            <button
              class="icon-btn compact"
              title="编辑项目"
              @click="emit('edit-project', proj)"
            >
              <IdeIcon name="edit" :size="12" />
            </button>
          </span>
        </div>

        <!-- Project's Data Sources -->
        <div v-if="expandedProjects[proj.id]">
          <div
            v-if="connectionsInProject(proj.id).length === 0"
            style="
              padding: 4px 8px 6px 28px;
              font-size: 11px;
              color: var(--text-muted);
              display: flex;
              align-items: center;
              gap: 6px;
            "
          >
            <span>暂无数据源</span>
            <button
              class="conn-link-btn"
              style="font-size: 11px"
              @click="emit('new-connection', proj.id)"
            >
              + 添加
            </button>
          </div>

          <div v-for="p in connectionsInProject(proj.id)" :key="p.id">
            <div
              class="tree-row"
              style="padding-left: 16px"
              :class="{ selected: selectedNodeKey === `p:${p.id}` || activeProfileId === p.id }"
              @click="
                selectedNodeKey = `p:${p.id}`;
                emit('toggle-profile', p);
              "
              @contextmenu="openProfileContext($event, p)"
            >
              <span class="tree-chevron">
                <IdeIcon :name="expandedProfiles[p.id] ? 'chevron-down' : 'chevron-right'" :size="12" />
              </span>
              <IdeIcon :name="engineIcon(p.dbType)" :size="14" />
              <span class="env-dot" :class="p.env" :title="p.env" />
              <span class="tree-label" style="font-weight: 600">{{ p.name }}</span>
              <span class="tree-meta">{{ engineShort(p.dbType) }}</span>
              <span class="tree-hover-actions" @click.stop>
                <button class="icon-btn compact" :title="t('btnTest')" @click="emit('test-profile', p)">
                  <IdeIcon name="check" :size="12" />
                </button>
                <button class="icon-btn compact" :title="t('btnEdit')" @click="emit('edit-profile', p)">
                  <IdeIcon name="edit" :size="12" />
                </button>
              </span>
            </div>

            <!-- Databases / Schemas -->
            <div v-if="expandedProfiles[p.id] && databasesByProfile[p.id]">
              <div v-for="db in databasesByProfile[p.id]" :key="db.name">
                <div
                  class="tree-row"
                  style="padding-left: 32px"
                  :class="{
                    selected:
                      selectedNodeKey === `d:${p.id}:${db.name}` ||
                      (activeProfileId === p.id && activeDbName === db.name),
                  }"
                  @click="
                    selectedNodeKey = `d:${p.id}:${db.name}`;
                    emit('toggle-database', p, db.name);
                  "
                >
                  <span class="tree-chevron">
                    <IdeIcon :name="expandedDatabases[`${p.id}:${db.name}`] ? 'chevron-down' : 'chevron-right'" :size="12" />
                  </span>
                  <IdeIcon name="schema" :size="13" />
                  <span class="tree-label">{{ db.name }}</span>
                  <span class="tree-meta">{{ db.tableCount ?? '' }}</span>
                </div>

                <!-- Tables -->
                <div v-if="expandedDatabases[`${p.id}:${db.name}`]">
                  <div v-for="tbl in filterTables(p.id, db.name)" :key="tbl.name">
                    <div
                      class="tree-row"
                      style="padding-left: 48px"
                      :class="{ selected: selectedNodeKey === `t:${p.id}:${db.name}:${tbl.name}` }"
                      @click="
                        selectedNodeKey = `t:${p.id}:${db.name}:${tbl.name}`;
                        emit('select-table', p, db.name, tbl.name);
                        emit('toggle-table', p, db.name, tbl.name);
                      "
                      @dblclick="emit('query-top100', p, db.name, tbl.name)"
                      @contextmenu="openTableContext($event, p, db.name, tbl.name)"
                    >
                      <span class="tree-chevron">
                        <IdeIcon :name="expandedTables[`${p.id}:${db.name}:${tbl.name}`] ? 'chevron-down' : 'chevron-right'" :size="11" />
                      </span>
                      <IdeIcon name="table" :size="13" />
                      <span class="tree-label">{{ tbl.name }}</span>
                      <span class="tree-meta">{{ tbl.estimatedRows ?? '' }}</span>
                      <span class="tree-hover-actions" @click.stop>
                        <button
                          class="icon-btn compact"
                          :title="t('ctxQueryTop100')"
                          @click="emit('query-top100', p, db.name, tbl.name)"
                        >
                          <IdeIcon name="run" :size="11" color="var(--run-green-bright)" />
                        </button>
                        <button
                          class="icon-btn compact"
                          :title="t('ctxViewDdl')"
                          @click="emit('view-ddl', p, db.name, tbl.name)"
                        >
                          DDL
                        </button>
                      </span>
                    </div>

                    <!-- Columns & Indexes -->
                    <div v-if="expandedTables[`${p.id}:${db.name}:${tbl.name}`]">
                      <div
                        v-for="col in columnsByKey[`${p.id}:${db.name}:${tbl.name}`] ?? []"
                        :key="col.name"
                        class="tree-row"
                        style="padding-left: 66px"
                        @dblclick="emit('copy-text', col.name)"
                      >
                        <IdeIcon :name="col.isPrimaryKey ? 'key' : 'column'" :size="12" />
                        <span class="tree-label">{{ col.name }}</span>
                        <span class="tree-meta">{{ col.dataType }}</span>
                      </div>

                      <div
                        v-for="idx in indexesByKey[`${p.id}:${db.name}:${tbl.name}`] ?? []"
                        :key="`idx-${idx.name}`"
                        class="tree-row"
                        style="padding-left: 66px"
                      >
                        <IdeIcon name="index" :size="12" />
                        <span class="tree-label" style="color: var(--text-secondary)">
                          {{ idx.name }}
                        </span>
                        <span class="tree-meta">{{ idx.columns.join(',') }}</span>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Context Menu -->
    <div
      v-if="ctxMenu.visible"
      class="context-menu"
      :style="{ left: `${ctxMenu.x}px`, top: `${ctxMenu.y}px` }"
      @click.stop="closeContext"
    >
      <template v-if="ctxMenu.kind === 'project' && ctxMenu.project">
        <button class="dropdown-item" @click="emit('new-connection', ctxMenu.project.id)">
          <span>在此项目新建数据源...</span>
        </button>
        <button class="dropdown-item" @click="emit('import-dbeaver', ctxMenu.project.id)">
          <span>导入 DBeaver 连接到此项目...</span>
        </button>
        <button class="dropdown-item" @click="emit('edit-project', ctxMenu.project)">
          <span>{{ t('manageProject') }}</span>
        </button>
        <template v-if="projects.length > 1">
          <div class="dropdown-sep" />
          <button
            class="dropdown-item"
            style="color: var(--prod)"
            @click="emit('delete-project', ctxMenu.project.id)"
          >
            <span>{{ t('deleteProject') }}</span>
          </button>
        </template>
      </template>

      <template v-else-if="ctxMenu.kind === 'profile' && ctxMenu.profile">
        <button class="dropdown-item" @click="emit('test-profile', ctxMenu.profile)">
          <span>{{ t('btnTest') }}</span>
        </button>
        <button class="dropdown-item" @click="emit('edit-profile', ctxMenu.profile)">
          <span>{{ t('btnEdit') }}</span>
        </button>
        <button class="dropdown-item" @click="emit('copy-text', ctxMenu.profile.name)">
          <span>{{ t('ctxCopyName') }}</span>
        </button>
        <template v-if="projects.length > 1">
          <div class="dropdown-sep" />
          <div style="padding: 3px 10px; font-size: 10.5px; color: var(--text-secondary)">
            {{ t('moveToProject') }}
          </div>
          <button
            v-for="proj in projects.filter((pr) => pr.id !== ctxMenu.profile?.projectId)"
            :key="proj.id"
            class="dropdown-item"
            @click="emit('move-profile-project', ctxMenu.profile, proj.id)"
          >
            <span>→ {{ proj.name }}</span>
          </button>
        </template>
        <div class="dropdown-sep" />
        <button
          class="dropdown-item"
          style="color: var(--prod)"
          @click="emit('delete-profile', ctxMenu.profile)"
        >
          <span>{{ t('btnDelete') }}</span>
        </button>
      </template>

      <template v-else-if="ctxMenu.kind === 'table' && ctxMenu.profile && ctxMenu.dbName && ctxMenu.tableName">
        <button
          class="dropdown-item"
          @click="emit('query-top100', ctxMenu.profile, ctxMenu.dbName, ctxMenu.tableName)"
        >
          <span>{{ t('ctxQueryTop100') }}</span>
        </button>
        <button
          class="dropdown-item"
          @click="emit('gen-select', ctxMenu.profile, ctxMenu.dbName, ctxMenu.tableName)"
        >
          <span>{{ t('ctxGenSelect') }}</span>
        </button>
        <button
          class="dropdown-item"
          @click="emit('view-ddl', ctxMenu.profile, ctxMenu.dbName, ctxMenu.tableName)"
        >
          <span>{{ t('ctxViewDdl') }}</span>
        </button>
        <button
          class="dropdown-item"
          @click="emit('select-table', ctxMenu.profile, ctxMenu.dbName, ctxMenu.tableName)"
        >
          <span>{{ t('ctxViewStructure') }}</span>
        </button>
        <div class="dropdown-sep" />
        <button class="dropdown-item" @click="emit('copy-text', ctxMenu.tableName)">
          <span>{{ t('ctxCopyName') }}</span>
        </button>
      </template>
    </div>
  </div>
</template>
