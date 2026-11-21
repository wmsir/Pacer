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

const props = defineProps<{
  projects: WorkspaceProject[];
  activeProjectId: string;
  showAllProjects: boolean;
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
  (e: 'new-connection'): void;
  (e: 'new-project'): void;
  (e: 'toggle-show-all-projects'): void;
  (e: 'move-profile-project', profile: ConnectionProfile, projectId: string): void;
  (e: 'import-dbeaver'): void;
  (e: 'init-demo'): void;
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
  kind: 'profile' | 'table';
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
  if (norm.includes('sqlite')) return 'SQLite';
  if (norm.includes('postgres')) return 'PG';
  if (norm.includes('mysql')) return 'MySQL';
  if (norm.includes('maria')) return 'Maria';
  if (norm.includes('oracle')) return 'Ora';
  if (norm.includes('dameng')) return 'DM8';
  if (norm.includes('kingbase')) return 'KB';
  if (norm.includes('gauss')) return 'OG';
  if (norm.includes('tidb')) return 'TiDB';
  if (norm.includes('oceanbase')) return 'OB';
  if (norm.includes('sqlserver')) return 'MSSQL';
  return raw.toUpperCase();
}

const activeProject = computed(
  () => props.projects.find((p) => p.id === props.activeProjectId) ?? props.projects[0] ?? null,
);

function projectNameOf(projectId?: string | null): string {
  if (!projectId) return '';
  return props.projects.find((p) => p.id === projectId)?.name ?? '';
}

const filteredConnections = computed(() => {
  const scoped = props.showAllProjects
    ? props.connections
    : props.connections.filter(
        (p) => !p.projectId || p.projectId === props.activeProjectId,
      );
  const q = search.value.trim().toLowerCase();
  if (!q) return scoped;
  return scoped.filter((p) => {
    if (p.name.toLowerCase().includes(q)) return true;
    const dbs = props.databasesByProfile[p.id] ?? [];
    for (const db of dbs) {
      if (db.name.toLowerCase().includes(q)) return true;
      const tbls = props.tablesByKey[`${p.id}:${db.name}`] ?? [];
      if (tbls.some((tb) => tb.name.toLowerCase().includes(q))) return true;
    }
    return false;
  });
});

function filterTables(profileId: string, dbName: string): TableMeta[] {
  const list = props.tablesByKey[`${profileId}:${dbName}`] ?? [];
  const q = search.value.trim().toLowerCase();
  if (!q) return list;
  const matched = list.filter((tb) => tb.name.toLowerCase().includes(q));
  return matched.length > 0 ? matched : list;
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
  <div class="tool-window left" style="width: 100%; height: 100%" @click="closeContext">
    <div class="tw-header">
      <span>{{ t('toolDatabase') }}</span>
      <div class="tw-actions">
        <button class="icon-btn compact" :title="t('newProject')" @click="emit('new-project')">
          📦
        </button>
        <button class="icon-btn compact" :title="t('newConnection')" @click="emit('new-connection')">
          ＋
        </button>
        <button class="icon-btn compact" :title="t('importDbeaver')" @click="emit('import-dbeaver')">
          ⤓
        </button>
        <button class="icon-btn compact" :title="t('openDemoDb')" @click="emit('init-demo')">
          ⚡
        </button>
      </div>
    </div>

    <div v-if="activeProject" class="project-banner">
      <div style="display: flex; align-items: center; gap: 6px; min-width: 0">
        <span
          class="project-dot"
          :style="{ background: activeProject.color || 'var(--accent)' }"
        />
        <span
          style="font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap"
          :title="activeProject.description || activeProject.name"
        >
          {{ showAllProjects ? t('allProjects') : activeProject.name }}
        </span>
        <span class="pill pill-neutral">{{ filteredConnections.length }}</span>
      </div>
      <button
        v-if="projects.length > 1"
        class="icon-btn compact"
        :class="{ active: showAllProjects }"
        title="切换仅看当前项目或查看全部项目数据源"
        @click="emit('toggle-show-all-projects')"
      >
        {{ showAllProjects ? '仅当前' : '全部' }}
      </button>
    </div>

    <div class="tw-search">
      <input
        v-model="search"
        class="field"
        :placeholder="t('filterTreePlaceholder')"
      />
    </div>

    <div class="tw-body">
      <div v-if="filteredConnections.length === 0" class="empty-state">
        <div>暂无数据源连接</div>
        <div style="display: flex; gap: 6px; margin-top: 8px; flex-wrap: wrap">
          <button class="btn btn-primary btn-sm" @click="emit('new-connection')">
            + {{ t('newConnection') }}
          </button>
          <button class="btn btn-sm" @click="emit('init-demo')">
            {{ t('openDemoDb') }}
          </button>
        </div>
      </div>

      <div v-for="p in filteredConnections" :key="p.id">
        <div
          class="tree-row"
          :class="{ selected: selectedNodeKey === `p:${p.id}` || activeProfileId === p.id }"
          @click="
            selectedNodeKey = `p:${p.id}`;
            emit('toggle-profile', p);
          "
          @contextmenu="openProfileContext($event, p)"
        >
          <span class="tree-chevron">{{ expandedProfiles[p.id] ? '▾' : '▸' }}</span>
          <span class="env-dot" :class="p.env" :title="p.env" />
          <span class="tree-label" style="font-weight: 600">{{ p.name }}</span>
          <span v-if="showAllProjects && projectNameOf(p.projectId)" class="pill pill-neutral" style="margin-right: 2px">
            {{ projectNameOf(p.projectId) }}
          </span>
          <span class="tree-meta">{{ engineShort(p.dbType) }}</span>
          <span class="tree-hover-actions" @click.stop>
            <button class="icon-btn compact" :title="t('btnTest')" @click="emit('test-profile', p)">
              ✓
            </button>
            <button class="icon-btn compact" :title="t('btnEdit')" @click="emit('edit-profile', p)">
              ✎
            </button>
          </span>
        </div>

        <div v-if="expandedProfiles[p.id] && databasesByProfile[p.id]">
          <div v-for="db in databasesByProfile[p.id]" :key="db.name">
            <div
              class="tree-row"
              style="padding-left: 18px"
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
                {{ expandedDatabases[`${p.id}:${db.name}`] ? '▾' : '▸' }}
              </span>
              <span style="color: #5b9bd5; font-size: 11px">⛁</span>
              <span class="tree-label">{{ db.name }}</span>
              <span class="tree-meta">{{ db.tableCount ?? '' }}</span>
            </div>

            <div v-if="expandedDatabases[`${p.id}:${db.name}`]">
              <div v-for="tbl in filterTables(p.id, db.name)" :key="tbl.name">
                <div
                  class="tree-row"
                  style="padding-left: 34px"
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
                    {{ expandedTables[`${p.id}:${db.name}:${tbl.name}`] ? '▾' : '▸' }}
                  </span>
                  <span style="color: #4ec9b0; font-size: 11px">▦</span>
                  <span class="tree-label">{{ tbl.name }}</span>
                  <span class="tree-meta">{{ tbl.estimatedRows ?? '' }}</span>
                  <span class="tree-hover-actions" @click.stop>
                    <button
                      class="icon-btn compact"
                      :title="t('ctxQueryTop100')"
                      @click="emit('query-top100', p, db.name, tbl.name)"
                    >
                      ▶
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

                <div v-if="expandedTables[`${p.id}:${db.name}:${tbl.name}`]">
                  <div
                    v-for="col in columnsByKey[`${p.id}:${db.name}:${tbl.name}`] ?? []"
                    :key="col.name"
                    class="tree-row"
                    style="padding-left: 52px"
                    @dblclick="emit('copy-text', col.name)"
                  >
                    <span :style="{ color: col.isPrimaryKey ? '#e0a82e' : '#868a91', fontSize: '11px' }">
                      {{ col.isPrimaryKey ? '🔑' : '◦' }}
                    </span>
                    <span class="tree-label">{{ col.name }}</span>
                    <span class="tree-meta">{{ col.dataType }}</span>
                  </div>

                  <div
                    v-for="idx in indexesByKey[`${p.id}:${db.name}:${tbl.name}`] ?? []"
                    :key="`idx-${idx.name}`"
                    class="tree-row"
                    style="padding-left: 52px"
                  >
                    <span style="color: #a78bfa; font-size: 10px">⚡</span>
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

    <!-- Context Menu -->
    <div
      v-if="ctxMenu.visible"
      class="context-menu"
      :style="{ left: `${ctxMenu.x}px`, top: `${ctxMenu.y}px` }"
      @click.stop="closeContext"
    >
      <template v-if="ctxMenu.kind === 'profile' && ctxMenu.profile">
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

      <template v-if="ctxMenu.kind === 'table' && ctxMenu.profile && ctxMenu.dbName && ctxMenu.tableName">
        <button
          class="dropdown-item"
          @click="emit('query-top100', ctxMenu.profile, ctxMenu.dbName, ctxMenu.tableName)"
        >
          <span>{{ t('ctxQueryTop100') }}</span>
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
        <button
          class="dropdown-item"
          @click="emit('gen-select', ctxMenu.profile, ctxMenu.dbName, ctxMenu.tableName)"
        >
          <span>{{ t('ctxGenSelect') }}</span>
        </button>
        <div class="dropdown-sep" />
        <button class="dropdown-item" @click="emit('copy-text', ctxMenu.tableName)">
          <span>{{ t('ctxCopyName') }}</span>
        </button>
      </template>
    </div>
  </div>
</template>
