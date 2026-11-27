<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from '../i18n';
import type {
  ColumnMeta,
  ConnectionProfile,
  DatabaseMeta,
  ForeignKeyMeta,
  IndexMeta,
  RoutineMeta,
  TableMeta,
  WorkspaceProject,
} from '../api/ipc';
import IdeIcon, { type IdeIconName } from './IdeIcon.vue';
import { buildRedisNamespaceTree, type RedisNamespaceNode } from '../utils/commercialTools';
import { isViewTableType } from '../utils/sqlTools';

export interface ConnectionHealthInfo {
  status: 'ok' | 'error' | 'testing';
  message?: string;
  latencyMs?: number;
  checkedAt?: string;
}

export interface FavoriteTableEntry {
  profileId: string;
  dbName: string;
  tableName: string;
  isRedis?: boolean;
}

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
  foreignKeysByKey?: Record<string, ForeignKeyMeta[]>;
  routinesByKey?: Record<string, RoutineMeta[]>;
  expandedProfiles: Record<string, boolean>;
  expandedDatabases: Record<string, boolean>;
  expandedTables: Record<string, boolean>;
  connHealthByProfile?: Record<string, ConnectionHealthInfo>;
  activeConnectedIds?: string[];
  favorites?: FavoriteTableEntry[];
  batchTesting?: boolean;
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
  (e: 'export-dbeaver-profile', profile: ConnectionProfile): void;
  (e: 'export-dbeaver-project', project: WorkspaceProject): void;
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
  (e: 'duplicate-profile', profile: ConnectionProfile): void;
  (e: 'refresh-profile', profile: ConnectionProfile): void;
  (e: 'delete-profile', profile: ConnectionProfile): void;
  (e: 'refresh-database', profile: ConnectionProfile, dbName: string): void;
  (e: 'new-console-for-db', profile: ConnectionProfile, dbName: string): void;
  (e: 'create-table', profile: ConnectionProfile, dbName: string): void;
  (e: 'export-db-ddl', profile: ConnectionProfile, dbName: string): void;
  (e: 'count-table', profile: ConnectionProfile, dbName: string, tableName: string): void;
  (e: 'copy-table-columns', profile: ConnectionProfile, dbName: string, tableName: string): void;
  (e: 'gen-insert', profile: ConnectionProfile, dbName: string, tableName: string): void;
  (e: 'gen-update', profile: ConnectionProfile, dbName: string, tableName: string): void;
  (e: 'import-table-data', profile: ConnectionProfile, dbName: string, tableName: string): void;
  (e: 'gen-entity-code', profile: ConnectionProfile, dbName: string, tableName: string): void;
  (e: 'gen-mock-data', profile: ConnectionProfile, dbName: string, tableName: string): void;
  (e: 'rename-table', profile: ConnectionProfile, dbName: string, tableName: string): void;
  (e: 'truncate-table', profile: ConnectionProfile, dbName: string, tableName: string): void;
  (e: 'drop-table', profile: ConnectionProfile, dbName: string, tableName: string): void;
  (e: 'refresh-table', profile: ConnectionProfile, dbName: string, tableName: string): void;
  (e: 'test-all-connections'): void;
  (e: 'open-object-search', profile?: ConnectionProfile, dbName?: string): void;
  (e: 'open-schema-diff', profile?: ConnectionProfile, dbName?: string, tableName?: string): void;
  (e: 'open-alter-table', profile: ConnectionProfile, dbName: string, tableName: string): void;
  (e: 'open-er-diagram', profile: ConnectionProfile, dbName: string): void;
  (e: 'open-server-monitor', profile: ConnectionProfile): void;
  (e: 'open-data-diff', profile: ConnectionProfile, dbName: string, tableName: string): void;
  (e: 'open-data-transfer', profile: ConnectionProfile, dbName: string, tableName?: string): void;
  (e: 'open-data-dict', profile: ConnectionProfile, dbName: string, tableName?: string): void;
  (e: 'inspect-routine', profile: ConnectionProfile, dbName: string, routine: RoutineMeta): void;
  (e: 'dump-sql', profile: ConnectionProfile, dbName: string, tableNames: string[] | null, includeData: boolean): void;
  (e: 'run-sql-file', profile: ConnectionProfile, dbName: string): void;
  (e: 'open-redis-new-key', profile: ConnectionProfile, dbName: string): void;
  (e: 'edit-redis-key', profile: ConnectionProfile, dbName: string, keyName: string): void;
  (e: 'toggle-favorite', profile: ConnectionProfile, dbName: string, tableName: string): void;
  (e: 'disconnect-profile', profile: ConnectionProfile): void;
  (e: 'redis-prefix-action', action: 'unlink' | 'expire' | 'bigkeys', profile: ConnectionProfile, dbName: string, prefix: string): void;
  (e: 'copy-text', text: string): void;
  (e: 'minimize-panel'): void;
}>();

function connStatusClass(profileId: string): 'ok' | 'error' | 'testing' | 'unknown' {
  return props.connHealthByProfile?.[profileId]?.status ?? 'unknown';
}

function connStatusTitle(profile: ConnectionProfile): string {
  const h = props.connHealthByProfile?.[profile.id];
  if (!h) {
    return `${profile.name} (${profile.env}) · 尚未测试连接（点击右侧 ✓ 图标或展开即可检测）`;
  }
  if (h.status === 'ok') {
    const lat = h.latencyMs !== undefined ? ` · ${h.latencyMs}ms` : '';
    return `${profile.name} · 连接正常可使用${lat}${h.message ? ` (${h.message})` : ''}`;
  }
  if (h.status === 'error') {
    return `${profile.name} · 无法连接：${h.message || '请检查网络、主机端口或认证信息'}`;
  }
  return `${profile.name} · 正在测试连接...`;
}

const { t } = useI18n();
const search = ref('');
const objectFilter = ref<'all' | 'table' | 'view'>('all');
const expandedTablesGroup = ref<Record<string, boolean>>({});
const expandedViewsGroup = ref<Record<string, boolean>>({});
const expandedRoutinesGroup = ref<Record<string, boolean>>({});
const expandedRedisFolders = ref<Record<string, boolean>>({});
const redisTreeMode = ref(true);
const redisDelimiter = ref(':');
const selectedNodeKey = ref('');
const favoritesCollapsed = ref(false);

interface FlatRedisRow {
  node: RedisNamespaceNode;
  depth: number;
}

function flattenedRedisNodes(profileId: string, dbName: string): FlatRedisRow[] {
  const tree = buildRedisNamespaceTree(filterTables(profileId, dbName), redisDelimiter.value);
  const out: FlatRedisRow[] = [];
  function walk(list: RedisNamespaceNode[], depth: number) {
    for (const n of list) {
      out.push({ node: n, depth });
      if (n.isFolder && expandedRedisFolders.value[`${profileId}:${dbName}:${n.id}`]) {
        walk(n.children, depth + 1);
      }
    }
  }
  walk(tree, 0);
  return out;
}

function toggleRedisFolder(profileId: string, dbName: string, nodeId: string) {
  const k = `${profileId}:${dbName}:${nodeId}`;
  expandedRedisFolders.value[k] = !expandedRedisFolders.value[k];
}

function isTablesGroupExpanded(key: string): boolean {
  return expandedTablesGroup.value[key] !== false;
}

function toggleTablesGroup(key: string) {
  expandedTablesGroup.value[key] = !isTablesGroupExpanded(key);
}

function isViewsGroupExpanded(key: string): boolean {
  return expandedViewsGroup.value[key] !== false;
}

function toggleViewsGroup(key: string) {
  expandedViewsGroup.value[key] = !isViewsGroupExpanded(key);
}

function toggleRoutinesGroup(key: string) {
  expandedRoutinesGroup.value[key] = !expandedRoutinesGroup.value[key];
}

function isViewTable(tbl: TableMeta): boolean {
  return isViewTableType(tbl.tableType);
}

function isMaterializedView(tbl: TableMeta): boolean {
  return (tbl.tableType ?? '').toUpperCase().includes('MATERIALIZED');
}

function findTableMeta(profileId: string, dbName: string, tableName: string): TableMeta | undefined {
  const list = props.tablesByKey[`${profileId}:${dbName}`] ?? [];
  const lower = tableName.toLowerCase();
  return list.find((t) => t.name === tableName) ?? list.find((t) => t.name.toLowerCase() === lower);
}

function isViewByName(profileId: string, dbName: string, tableName: string): boolean {
  const meta = findTableMeta(profileId, dbName, tableName);
  return meta ? isViewTable(meta) : false;
}

function isFavorited(profileId: string, dbName: string, tableName: string): boolean {
  return Boolean(
    props.favorites?.some(
      (f) => f.profileId === profileId && f.dbName === dbName && f.tableName === tableName,
    ),
  );
}

function profileOfFavorite(profileId: string): ConnectionProfile | undefined {
  return props.connections.find((c) => c.id === profileId);
}

const ctxMenu = ref<{
  visible: boolean;
  x: number;
  y: number;
  kind: 'project' | 'profile' | 'database' | 'table' | 'redis-folder';
  project?: WorkspaceProject;
  profile?: ConnectionProfile;
  dbName?: string;
  tableName?: string;
  isView?: boolean;
  redisPrefix?: string;
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
  if (norm.includes('redis')) return 'Redis';
  if (norm.includes('postgres')) return 'PG';
  if (norm.includes('mysql')) return 'MySQL';
  if (norm.includes('maria')) return 'MariaDB';
  if (norm.includes('clickhouse')) return 'CH';
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
  if (norm.includes('redis')) return 'db-redis';
  if (norm.includes('postgres') || norm.includes('kingbase') || norm.includes('gauss')) return 'db-postgres';
  if (norm.includes('mysql') || norm.includes('maria') || norm.includes('oceanbase')) return 'db-mysql';
  return 'db-generic';
}

function isRedisProfile(profile?: ConnectionProfile | null): boolean {
  if (!profile) return false;
  const raw =
    typeof profile.dbType === 'string'
      ? profile.dbType
      : Object.values(profile.dbType)[0] ?? '';
  return raw.toLowerCase().includes('redis');
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

function filterBySearch(
  profileId: string,
  dbName: string,
  list: TableMeta[],
): TableMeta[] {
  const q = search.value.trim().toLowerCase();
  if (!q) return list;
  const matched = list.filter((tb) => tb.name.toLowerCase().includes(q));
  if (matched.length > 0) return matched;
  const prof = props.connections.find((c) => c.id === profileId);
  const ancestorHit =
    dbName.toLowerCase().includes(q) || Boolean(prof && prof.name.toLowerCase().includes(q));
  return ancestorHit ? list : [];
}

function filterTables(profileId: string, dbName: string): TableMeta[] {
  const rawList = props.tablesByKey[`${profileId}:${dbName}`] ?? [];
  const typeFiltered =
    objectFilter.value === 'all'
      ? rawList
      : objectFilter.value === 'view'
        ? rawList.filter((tb) => isViewTable(tb))
        : rawList.filter((tb) => !isViewTable(tb));
  return filterBySearch(profileId, dbName, typeFiltered);
}

function filterBaseTables(profileId: string, dbName: string): TableMeta[] {
  if (objectFilter.value === 'view') return [];
  const rawList = props.tablesByKey[`${profileId}:${dbName}`] ?? [];
  return filterBySearch(
    profileId,
    dbName,
    rawList.filter((tb) => !isViewTable(tb)),
  );
}

function filterViewTables(profileId: string, dbName: string): TableMeta[] {
  if (objectFilter.value === 'table') return [];
  const rawList = props.tablesByKey[`${profileId}:${dbName}`] ?? [];
  return filterBySearch(
    profileId,
    dbName,
    rawList.filter((tb) => isViewTable(tb)),
  );
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

function openDatabaseContext(ev: MouseEvent, profile: ConnectionProfile, dbName: string) {
  ev.preventDefault();
  ctxMenu.value = {
    visible: true,
    x: ev.clientX,
    y: ev.clientY,
    kind: 'database',
    profile,
    dbName,
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
    isView: !isRedisProfile(profile) && isViewByName(profile.id, dbName, tableName),
  };
}

function openRedisFolderContext(
  ev: MouseEvent,
  profile: ConnectionProfile,
  dbName: string,
  prefix: string,
) {
  ev.preventDefault();
  ctxMenu.value = {
    visible: true,
    x: ev.clientX,
    y: ev.clientY,
    kind: 'redis-folder',
    profile,
    dbName,
    redisPrefix: prefix,
  };
}

function closeContext() {
  ctxMenu.value.visible = false;
}
</script>

<template>
  <div style="display: flex; flex-direction: column; width: 100%; height: 100%" @click="closeContext">
    <div class="tw-header">
      <span class="tw-title" :title="t('toolDatabase')">
        <IdeIcon name="database" :size="14" color="#4D9BFF" />
        <span class="tw-title-text">{{ t('toolDatabase') }}</span>
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
        <button
          class="icon-btn compact"
          title="批量测试当前项目所有数据源连接健康"
          :disabled="batchTesting"
          @click="emit('test-all-connections')"
        >
          <IdeIcon name="check" :size="13" />
        </button>
        <button
          class="icon-btn compact"
          title="在数据库中全局搜索表名、字段名与 Redis Key"
          @click="emit('open-object-search')"
        >
          <IdeIcon name="search" :size="13" />
        </button>
        <button class="icon-btn compact" title="全部折叠" @click="emit('collapse-all-tree')">
          <IdeIcon name="collapse-all" :size="13" />
        </button>
        <button class="icon-btn compact" :title="t('openDemoDb')" @click="emit('init-demo', activeProjectId)">
          <IdeIcon name="bolt" :size="13" />
        </button>
        <button class="icon-btn compact" title="最小化数据库面板" @click="emit('minimize-panel')">
          <IdeIcon name="minimize" :size="13" />
        </button>
      </div>
    </div>

    <div class="tw-search">
      <input
        v-model="search"
        class="field"
        placeholder="搜索项目、数据源、库名或表名..."
      />
      <div class="tree-obj-filter-row">
        <button
          class="tree-filter-chip"
          :class="{ active: objectFilter === 'all' }"
          @click="objectFilter = 'all'"
        >
          全部
        </button>
        <button
          class="tree-filter-chip"
          :class="{ active: objectFilter === 'table' }"
          @click="objectFilter = 'table'"
        >
          表
        </button>
        <button
          class="tree-filter-chip"
          :class="{ active: objectFilter === 'view' }"
          @click="objectFilter = 'view'"
        >
          视图
        </button>
      </div>
    </div>

    <div class="tw-body">
      <!-- Pinned Favorite Tables / Keys -->
      <div v-if="favorites && favorites.length > 0" class="favorites-section">
        <div
          class="tree-row project-tree-row"
          @click="favoritesCollapsed = !favoritesCollapsed"
        >
          <span class="tree-chevron">
            <IdeIcon :name="favoritesCollapsed ? 'chevron-right' : 'chevron-down'" :size="12" />
          </span>
          <span style="font-size: 12px">⭐</span>
          <span class="tree-label" style="font-weight: 700">常用收藏</span>
          <span class="pill pill-neutral">{{ favorites.length }}</span>
        </div>
        <div v-if="!favoritesCollapsed">
          <div
            v-for="fav in favorites"
            :key="`fav-${fav.profileId}-${fav.dbName}-${fav.tableName}`"
            class="tree-row favorite-row"
            :title="`${profileOfFavorite(fav.profileId)?.name || fav.profileId} / ${fav.dbName}.${fav.tableName}`"
            @click="
              (() => {
                const prof = profileOfFavorite(fav.profileId);
                if (prof) emit('select-table', prof, fav.dbName, fav.tableName);
              })()
            "
            @dblclick="
              (() => {
                const prof = profileOfFavorite(fav.profileId);
                if (prof) emit('query-top100', prof, fav.dbName, fav.tableName);
              })()
            "
          >
            <IdeIcon
              :name="fav.isRedis ? 'key' : isViewByName(fav.profileId, fav.dbName, fav.tableName) ? 'view' : 'table'"
              :size="13"
            />
            <span class="tree-label">{{ fav.tableName }}</span>
            <span
              v-if="!fav.isRedis && isViewByName(fav.profileId, fav.dbName, fav.tableName)"
              class="tree-view-badge"
            >
              视图
            </span>
            <span class="tree-meta">{{ fav.dbName }}</span>
            <span class="tree-hover-actions" @click.stop>
              <button
                class="icon-btn compact"
                title="查询前100行 / 读取 Key"
                @click="
                  (() => {
                    const prof = profileOfFavorite(fav.profileId);
                    if (prof) emit('query-top100', prof, fav.dbName, fav.tableName);
                  })()
                "
              >
                <IdeIcon name="run" :size="11" color="var(--run-green-bright)" />
              </button>
              <button
                class="icon-btn compact"
                title="取消收藏"
                @click="
                  (() => {
                    const prof = profileOfFavorite(fav.profileId);
                    if (prof) emit('toggle-favorite', prof, fav.dbName, fav.tableName);
                  })()
                "
              >
                ×
              </button>
            </span>
          </div>
        </div>
      </div>

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
              <span
                class="conn-status-dot"
                :class="connStatusClass(p.id)"
                :title="connStatusTitle(p)"
              />
              <span class="tree-label" style="font-weight: 600">{{ p.name }}</span>
              <span
                v-if="activeConnectedIds?.includes(p.id)"
                class="pill pill-dev"
                style="font-size: 8.5px; padding: 0 4px; flex-shrink: 0"
                title="活跃连接会话与连接池已建立"
              >
                LIVE
              </span>
              <span
                v-if="p.env === 'PROD' || p.env === 'TEST'"
                class="pill"
                :class="p.env === 'PROD' ? 'pill-prod' : 'pill-test'"
                style="font-size: 9px; padding: 0 4px; flex-shrink: 0"
              >
                {{ p.env }}
              </span>
              <span class="tree-meta">{{ engineShort(p.dbType) }}</span>
              <span class="tree-hover-actions" @click.stop>
                <button class="icon-btn compact" title="刷新数据库列表" @click="emit('refresh-profile', p)">
                  <IdeIcon name="refresh" :size="12" />
                </button>
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
                  @contextmenu="openDatabaseContext($event, p, db.name)"
                >
                  <span class="tree-chevron">
                    <IdeIcon :name="expandedDatabases[`${p.id}:${db.name}`] ? 'chevron-down' : 'chevron-right'" :size="12" />
                  </span>
                  <IdeIcon name="schema" :size="13" />
                  <span class="tree-label">{{ db.name }}</span>
                  <span class="tree-meta">{{ db.tableCount ?? '' }}</span>
                  <span class="tree-hover-actions" @click.stop>
                    <button
                      class="icon-btn compact"
                      title="刷新此库表列表"
                      @click="emit('refresh-database', p, db.name)"
                    >
                      <IdeIcon name="refresh" :size="11" />
                    </button>
                    <button
                      class="icon-btn compact"
                      title="在此库新建查询控制台"
                      @click="emit('new-console-for-db', p, db.name)"
                    >
                      <IdeIcon name="plus" :size="11" />
                    </button>
                  </span>
                </div>

                <!-- Tables / Redis Namespaces -->
                <div v-if="expandedDatabases[`${p.id}:${db.name}`]">
                  <div
                    v-if="isRedisProfile(p)"
                    style="display: flex; align-items: center; gap: 4px; padding: 3px 8px 3px 46px; font-size: 10.5px; color: var(--text-secondary)"
                  >
                    <button
                      class="tree-filter-chip"
                      :class="{ active: redisTreeMode }"
                      @click="redisTreeMode = true"
                    >
                      树形
                    </button>
                    <button
                      class="tree-filter-chip"
                      :class="{ active: !redisTreeMode }"
                      @click="redisTreeMode = false"
                    >
                      平铺
                    </button>
                    <select
                      v-if="redisTreeMode"
                      v-model="redisDelimiter"
                      class="field"
                      style="width: auto; padding: 0 4px; font-size: 10.5px; height: 20px"
                      title="Redis 命名空间分隔符"
                    >
                      <option value=":">:</option>
                      <option value="/">/</option>
                      <option value=".">.</option>
                    </select>
                    <button
                      class="icon-btn compact"
                      title="扫描 Top 20 大 Key (MEMORY USAGE)"
                      @click="emit('redis-prefix-action', 'bigkeys', p, db.name, '')"
                    >
                      大Key
                    </button>
                  </div>

                  <template v-if="isRedisProfile(p) && redisTreeMode">
                    <template v-for="item in flattenedRedisNodes(p.id, db.name)" :key="item.node.id">
                      <div
                        v-if="item.node.isFolder"
                        class="tree-row"
                        :style="{ paddingLeft: `${48 + item.depth * 14}px` }"
                        @click="toggleRedisFolder(p.id, db.name, item.node.id)"
                        @contextmenu="openRedisFolderContext($event, p, db.name, item.node.prefix)"
                      >
                        <span class="tree-chevron">
                          <IdeIcon :name="expandedRedisFolders[`${p.id}:${db.name}:${item.node.id}`] ? 'chevron-down' : 'chevron-right'" :size="11" />
                        </span>
                        <IdeIcon name="folder" :size="13" color="#EAB308" />
                        <span class="tree-label" style="font-weight: 500">{{ item.node.label }}</span>
                        <span class="tree-meta">{{ item.node.keyCount }}</span>
                      </div>
                      <div
                        v-else-if="item.node.meta"
                        class="tree-row"
                        :style="{ paddingLeft: `${48 + item.depth * 14}px` }"
                        :class="{ selected: selectedNodeKey === `t:${p.id}:${db.name}:${item.node.meta.name}` }"
                        @click="
                          selectedNodeKey = `t:${p.id}:${db.name}:${item.node.meta!.name}`;
                          emit('select-table', p, db.name, item.node.meta!.name);
                          emit('toggle-table', p, db.name, item.node.meta!.name);
                        "
                        @dblclick="emit('query-top100', p, db.name, item.node.meta!.name)"
                        @contextmenu="openTableContext($event, p, db.name, item.node.meta!.name)"
                      >
                        <IdeIcon name="key" :size="12" />
                        <span class="tree-label">
                          {{ item.node.prefix ? item.node.meta.name.slice(item.node.prefix.length) : item.node.meta.name }}
                        </span>
                        <span class="tree-meta">
                          {{ [item.node.meta.tableType, item.node.meta.engine].filter(Boolean).join(' · ') }}
                        </span>
                      </div>
                    </template>
                  </template>

                  <template v-else>
                    <!-- Base Tables Group -->
                    <div
                      v-if="
                        objectFilter !== 'view' &&
                        (filterBaseTables(p.id, db.name).length > 0 ||
                          (filterViewTables(p.id, db.name).length === 0 && !search.trim()))
                      "
                    >
                      <div
                        class="tree-row tree-group-row"
                        style="padding-left: 44px"
                        @click="toggleTablesGroup(`${p.id}:${db.name}`)"
                      >
                        <span class="tree-chevron">
                          <IdeIcon
                            :name="isTablesGroupExpanded(`${p.id}:${db.name}`) ? 'chevron-down' : 'chevron-right'"
                            :size="11"
                          />
                        </span>
                        <IdeIcon name="table" :size="12" />
                        <span class="tree-label">
                          表 ({{ filterBaseTables(p.id, db.name).length }})
                        </span>
                      </div>

                      <div v-if="isTablesGroupExpanded(`${p.id}:${db.name}`)">
                        <div
                          v-if="filterBaseTables(p.id, db.name).length === 0"
                          class="tree-row"
                          style="padding-left: 60px; color: var(--text-muted); font-size: 11px"
                        >
                          <span>暂无数据表</span>
                        </div>
                        <div v-for="tbl in filterBaseTables(p.id, db.name)" :key="`tbl-${tbl.name}`">
                          <div
                            class="tree-row"
                            style="padding-left: 56px"
                            :class="{ selected: selectedNodeKey === `t:${p.id}:${db.name}:${tbl.name}` }"
                            :title="tbl.comment ? `${tbl.name} — ${tbl.comment}` : tbl.name"
                            @click="
                              selectedNodeKey = `t:${p.id}:${db.name}:${tbl.name}`;
                              emit('select-table', p, db.name, tbl.name);
                              emit('toggle-table', p, db.name, tbl.name);
                            "
                            @dblclick="emit('query-top100', p, db.name, tbl.name)"
                            @contextmenu="openTableContext($event, p, db.name, tbl.name)"
                          >
                            <span class="tree-chevron">
                              <IdeIcon
                                :name="expandedTables[`${p.id}:${db.name}:${tbl.name}`] ? 'chevron-down' : 'chevron-right'"
                                :size="11"
                              />
                            </span>
                            <IdeIcon :name="isRedisProfile(p) ? 'key' : 'table'" :size="13" />
                            <span class="tree-label">{{ tbl.name }}</span>
                            <span class="tree-meta">
                              {{
                                isRedisProfile(p)
                                  ? [tbl.tableType, tbl.engine].filter(Boolean).join(' · ')
                                  : (tbl.estimatedRows ?? '')
                              }}
                            </span>
                            <span class="tree-hover-actions" @click.stop>
                              <button
                                class="icon-btn compact"
                                :title="isRedisProfile(p) ? '读取键值 (GET / HGETALL / LRANGE ...)' : t('ctxQueryTop100')"
                                @click="emit('query-top100', p, db.name, tbl.name)"
                              >
                                <IdeIcon name="run" :size="11" color="var(--run-green-bright)" />
                              </button>
                              <button
                                class="icon-btn compact"
                                :title="isRedisProfile(p) ? '查看 Key 详细信息与命令模板' : t('ctxViewDdl')"
                                @click="emit('view-ddl', p, db.name, tbl.name)"
                              >
                                {{ isRedisProfile(p) ? 'INFO' : 'DDL' }}
                              </button>
                            </span>
                          </div>

                          <!-- Columns, Indexes & Foreign Keys -->
                          <div v-if="expandedTables[`${p.id}:${db.name}:${tbl.name}`]">
                            <div
                              v-for="col in columnsByKey[`${p.id}:${db.name}:${tbl.name}`] ?? []"
                              :key="col.name"
                              class="tree-row"
                              style="padding-left: 74px"
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
                              style="padding-left: 74px"
                            >
                              <IdeIcon name="index" :size="12" />
                              <span class="tree-label" style="color: var(--text-secondary)">
                                {{ idx.name }}
                              </span>
                              <span class="tree-meta">{{ idx.columns.join(',') }}</span>
                            </div>

                            <div
                              v-for="fk in foreignKeysByKey?.[`${p.id}:${db.name}:${tbl.name}`] ?? []"
                              :key="`fk-${fk.name}-${fk.columnName}`"
                              class="tree-row"
                              style="padding-left: 74px"
                              :title="`外键 ${fk.name}: ${fk.columnName} -> ${fk.referencedTable}.${fk.referencedColumn}`"
                            >
                              <span style="font-size: 11px; flex-shrink: 0">🔗</span>
                              <span class="tree-label" style="color: var(--accent)">
                                {{ fk.columnName }}
                              </span>
                              <span class="tree-meta">→ {{ fk.referencedTable }}.{{ fk.referencedColumn }}</span>
                            </div>
                          </div>
                        </div>
                      </div>
                    </div>

                    <!-- Dedicated Views Group (Visually separated from tables) -->
                    <div
                      v-if="
                        objectFilter !== 'table' &&
                        (filterViewTables(p.id, db.name).length > 0 || objectFilter === 'view')
                      "
                    >
                      <div
                        class="tree-row tree-group-row tree-group-view"
                        style="padding-left: 44px"
                        @click="toggleViewsGroup(`${p.id}:${db.name}`)"
                      >
                        <span class="tree-chevron">
                          <IdeIcon
                            :name="isViewsGroupExpanded(`${p.id}:${db.name}`) ? 'chevron-down' : 'chevron-right'"
                            :size="11"
                          />
                        </span>
                        <IdeIcon name="view" :size="12" />
                        <span class="tree-label">
                          视图 ({{ filterViewTables(p.id, db.name).length }})
                        </span>
                      </div>

                      <div v-if="isViewsGroupExpanded(`${p.id}:${db.name}`)">
                        <div
                          v-if="filterViewTables(p.id, db.name).length === 0"
                          class="tree-row"
                          style="padding-left: 60px; color: var(--text-muted); font-size: 11px"
                        >
                          <span>暂无视图</span>
                        </div>
                        <div v-for="tbl in filterViewTables(p.id, db.name)" :key="`view-${tbl.name}`">
                          <div
                            class="tree-row tree-row-view"
                            style="padding-left: 56px"
                            :class="{ selected: selectedNodeKey === `t:${p.id}:${db.name}:${tbl.name}` }"
                            :title="`${tbl.name}（${isMaterializedView(tbl) ? '物化视图' : '视图'} · 只读不可修改数据）${tbl.comment ? ` — ${tbl.comment}` : ''}`"
                            @click="
                              selectedNodeKey = `t:${p.id}:${db.name}:${tbl.name}`;
                              emit('select-table', p, db.name, tbl.name);
                              emit('toggle-table', p, db.name, tbl.name);
                            "
                            @dblclick="emit('query-top100', p, db.name, tbl.name)"
                            @contextmenu="openTableContext($event, p, db.name, tbl.name)"
                          >
                            <span class="tree-chevron">
                              <IdeIcon
                                :name="expandedTables[`${p.id}:${db.name}:${tbl.name}`] ? 'chevron-down' : 'chevron-right'"
                                :size="11"
                              />
                            </span>
                            <IdeIcon name="view" :size="13" />
                            <span class="tree-label">{{ tbl.name }}</span>
                            <span :class="isMaterializedView(tbl) ? 'tree-mview-badge' : 'tree-view-badge'">
                              {{ isMaterializedView(tbl) ? '物化视图' : '视图' }}
                            </span>
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
                                title="查看视图定义 (DDL)"
                                @click="emit('view-ddl', p, db.name, tbl.name)"
                              >
                                DDL
                              </button>
                            </span>
                          </div>

                          <!-- View Columns -->
                          <div v-if="expandedTables[`${p.id}:${db.name}:${tbl.name}`]">
                            <div
                              v-for="col in columnsByKey[`${p.id}:${db.name}:${tbl.name}`] ?? []"
                              :key="col.name"
                              class="tree-row"
                              style="padding-left: 74px"
                              @dblclick="emit('copy-text', col.name)"
                            >
                              <IdeIcon name="column" :size="12" />
                              <span class="tree-label">{{ col.name }}</span>
                              <span class="tree-meta">{{ col.dataType }}</span>
                            </div>
                          </div>
                        </div>
                      </div>
                    </div>
                  </template>

                  <!-- Routines / Functions / Triggers Group -->
                  <div
                    v-if="
                      !isRedisProfile(p) &&
                      (routinesByKey?.[`${p.id}:${db.name}`]?.length ?? 0) > 0
                    "
                  >
                    <div
                      class="tree-row tree-group-row"
                      style="padding-left: 44px"
                      @click="toggleRoutinesGroup(`${p.id}:${db.name}`)"
                    >
                      <span class="tree-chevron">
                        <IdeIcon
                          :name="expandedRoutinesGroup[`${p.id}:${db.name}`] ? 'chevron-down' : 'chevron-right'"
                          :size="11"
                        />
                      </span>
                      <span style="font-size: 11px">⚙️</span>
                      <span class="tree-label">
                        过程 / 函数 / 触发器 ({{ routinesByKey?.[`${p.id}:${db.name}`]?.length ?? 0 }})
                      </span>
                    </div>
                    <div v-if="expandedRoutinesGroup[`${p.id}:${db.name}`]">
                      <div
                        v-for="rt in routinesByKey?.[`${p.id}:${db.name}`] ?? []"
                        :key="`rt-${rt.kind}-${rt.name}`"
                        class="tree-row"
                        style="padding-left: 66px"
                        :title="`${rt.kind}: ${rt.name}${rt.comment ? ` — ${rt.comment}` : ''}`"
                        @click="emit('inspect-routine', p, db.name, rt)"
                      >
                        <span class="tree-routine-tag">{{ rt.kind.slice(0, 4) }}</span>
                        <span class="tree-label">{{ rt.name }}</span>
                        <span class="tree-meta">{{ rt.returnType || '' }}</span>
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
        <button class="dropdown-item" @click="emit('export-dbeaver-project', ctxMenu.project)">
          <span>📦 导出此项目数据源给 DBeaver (.dbp)...</span>
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
        <button class="dropdown-item" @click="emit('refresh-profile', ctxMenu.profile)">
          <span>刷新数据库列表</span>
        </button>
        <button class="dropdown-item" @click="emit('test-profile', ctxMenu.profile)">
          <span>{{ t('btnTest') }}</span>
        </button>
        <button
          v-if="activeConnectedIds?.includes(ctxMenu.profile.id)"
          class="dropdown-item"
          @click="emit('disconnect-profile', ctxMenu.profile)"
        >
          <span>🔌 断开连接（释放连接池/隧道）</span>
        </button>
        <button class="dropdown-item" @click="emit('open-server-monitor', ctxMenu.profile)">
          <span>📊 服务器监控与活跃会话管理...</span>
        </button>
        <button class="dropdown-item" @click="emit('edit-profile', ctxMenu.profile)">
          <span>{{ t('btnEdit') }}</span>
        </button>
        <button class="dropdown-item" @click="emit('duplicate-profile', ctxMenu.profile)">
          <span>复制数据源配置...</span>
        </button>
        <button class="dropdown-item" @click="emit('export-dbeaver-profile', ctxMenu.profile)">
          <span>📦 导出此账号配置给 DBeaver (.dbp)...</span>
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

      <template v-else-if="ctxMenu.kind === 'database' && ctxMenu.profile && ctxMenu.dbName">
        <button class="dropdown-item" @click="emit('refresh-database', ctxMenu.profile, ctxMenu.dbName)">
          <span>{{ isRedisProfile(ctxMenu.profile) ? '刷新 Key 列表 (SCAN)' : '刷新数据表列表' }}</span>
        </button>
        <button class="dropdown-item" @click="emit('new-console-for-db', ctxMenu.profile, ctxMenu.dbName)">
          <span>{{ isRedisProfile(ctxMenu.profile) ? '新建此库 Redis 命令控制台' : '新建此库 SQL 查询控制台' }}</span>
        </button>
        <button class="dropdown-item" @click="emit('open-object-search', ctxMenu.profile, ctxMenu.dbName)">
          <span>{{ isRedisProfile(ctxMenu.profile) ? '🔍 按模式检索 Key...' : '🔍 在此库中查找表/字段...' }}</span>
        </button>
        <template v-if="isRedisProfile(ctxMenu.profile)">
          <button class="dropdown-item" @click="emit('open-redis-new-key', ctxMenu.profile, ctxMenu.dbName)">
            <span>➕ 可视化新建 Redis Key...</span>
          </button>
        </template>
        <template v-else>
          <button class="dropdown-item" @click="emit('create-table', ctxMenu.profile, ctxMenu.dbName)">
            <span>✨ 可视化新建数据表...</span>
          </button>
          <button class="dropdown-item" @click="emit('open-er-diagram', ctxMenu.profile, ctxMenu.dbName)">
            <span>🗺️ 查看数据库 ER 关系图...</span>
          </button>
          <button class="dropdown-item" @click="emit('open-data-dict', ctxMenu.profile, ctxMenu.dbName)">
            <span>📘 生成并导出数据字典 (.md / .html)...</span>
          </button>
          <button class="dropdown-item" @click="emit('open-data-transfer', ctxMenu.profile, ctxMenu.dbName)">
            <span>🚚 跨库数据迁移向导...</span>
          </button>
          <div class="dropdown-sep" />
          <button class="dropdown-item" @click="emit('dump-sql', ctxMenu.profile, ctxMenu.dbName, null, false)">
            <span>💾 转储 SQL 文件（仅结构 .sql）...</span>
          </button>
          <button class="dropdown-item" @click="emit('dump-sql', ctxMenu.profile, ctxMenu.dbName, null, true)">
            <span>💾 转储 SQL 文件（结构 + 数据 .sql）...</span>
          </button>
          <button class="dropdown-item" @click="emit('export-db-ddl', ctxMenu.profile, ctxMenu.dbName)">
            <span>📄 查看全库建表 DDL（新控制台）</span>
          </button>
          <button class="dropdown-item" @click="emit('run-sql-file', ctxMenu.profile, ctxMenu.dbName)">
            <span>▶️ 运行本地 SQL 文件...</span>
          </button>
        </template>
        <div class="dropdown-sep" />
        <button class="dropdown-item" @click="emit('copy-text', ctxMenu.dbName)">
          <span>{{ isRedisProfile(ctxMenu.profile) ? '复制逻辑库名称' : '复制数据库名' }}</span>
        </button>
      </template>

      <template v-else-if="ctxMenu.kind === 'table' && ctxMenu.profile && ctxMenu.dbName && ctxMenu.tableName">
        <div
          v-if="ctxMenu.isView"
          class="dropdown-item"
          style="color: #8b5cf6; font-size: 11px; cursor: default; font-weight: 600"
        >
          <span>🔒 数据库视图（只读，不可修改数据）</span>
        </div>
        <button
          class="dropdown-item"
          @click="emit('query-top100', ctxMenu.profile, ctxMenu.dbName, ctxMenu.tableName)"
        >
          <span>{{ isRedisProfile(ctxMenu.profile) ? '读取键值内容 (GET/HGETALL/...)' : t('ctxQueryTop100') }}</span>
        </button>
        <button
          class="dropdown-item"
          @click="emit('count-table', ctxMenu.profile, ctxMenu.dbName, ctxMenu.tableName)"
        >
          <span>{{ isRedisProfile(ctxMenu.profile) ? '统计长度 / 元素数量' : ctxMenu.isView ? '统计视图总行数 (COUNT(*))' : '统计表总行数 (COUNT(*))' }}</span>
        </button>
        <button
          class="dropdown-item"
          @click="emit('view-ddl', ctxMenu.profile, ctxMenu.dbName, ctxMenu.tableName)"
        >
          <span>{{ isRedisProfile(ctxMenu.profile) ? '查看 Key 详情与操作命令' : ctxMenu.isView ? '查看视图定义 (DDL)' : t('ctxViewDdl') }}</span>
        </button>
        <button
          class="dropdown-item"
          @click="emit('select-table', ctxMenu.profile, ctxMenu.dbName, ctxMenu.tableName)"
        >
          <span>{{ isRedisProfile(ctxMenu.profile) ? '在右侧查看 Key 属性结构' : ctxMenu.isView ? '在右侧查看视图字段结构' : t('ctxViewStructure') }}</span>
        </button>
        <button
          class="dropdown-item"
          @click="emit('toggle-favorite', ctxMenu.profile, ctxMenu.dbName, ctxMenu.tableName)"
        >
          <span>
            {{
              isFavorited(ctxMenu.profile.id, ctxMenu.dbName, ctxMenu.tableName)
                ? '★ 取消常用收藏'
                : '⭐ 加入常用收藏'
            }}
          </span>
        </button>
        <template v-if="isRedisProfile(ctxMenu.profile)">
          <div class="dropdown-sep" />
          <button
            class="dropdown-item"
            @click="emit('edit-redis-key', ctxMenu.profile, ctxMenu.dbName, ctxMenu.tableName)"
          >
            <span>✏️ 可视化编辑 Key 与 TTL...</span>
          </button>
        </template>
        <template v-else>
          <div class="dropdown-sep" />
          <button
            v-if="!ctxMenu.isView"
            class="dropdown-item"
            @click="emit('open-alter-table', ctxMenu.profile, ctxMenu.dbName, ctxMenu.tableName)"
          >
            <span>🛠️ 可视化修改表结构...</span>
          </button>
          <button
            v-if="!ctxMenu.isView"
            class="dropdown-item"
            @click="emit('open-schema-diff', ctxMenu.profile, ctxMenu.dbName, ctxMenu.tableName)"
          >
            <span>⚖️ 表结构对比与同步...</span>
          </button>
          <button
            v-if="!ctxMenu.isView"
            class="dropdown-item"
            @click="emit('open-data-diff', ctxMenu.profile, ctxMenu.dbName, ctxMenu.tableName)"
          >
            <span>🧬 数据内容对比与同步 SQL...</span>
          </button>
          <button
            v-if="!ctxMenu.isView"
            class="dropdown-item"
            @click="emit('open-data-transfer', ctxMenu.profile, ctxMenu.dbName, ctxMenu.tableName)"
          >
            <span>🚚 迁移此表到其他库...</span>
          </button>
          <button
            class="dropdown-item"
            @click="emit('open-data-dict', ctxMenu.profile, ctxMenu.dbName, ctxMenu.tableName)"
          >
            <span>📘 导出数据字典...</span>
          </button>
          <button
            v-if="!ctxMenu.isView"
            class="dropdown-item"
            @click="emit('dump-sql', ctxMenu.profile, ctxMenu.dbName, [ctxMenu.tableName], true)"
          >
            <span>💾 转储此表 SQL（结构 + 数据 .sql）...</span>
          </button>
          <button
            v-if="!ctxMenu.isView"
            class="dropdown-item"
            @click="emit('import-table-data', ctxMenu.profile, ctxMenu.dbName, ctxMenu.tableName)"
          >
            <span>📥 导入 CSV / JSON 数据...</span>
          </button>
          <button
            class="dropdown-item"
            @click="emit('gen-entity-code', ctxMenu.profile, ctxMenu.dbName, ctxMenu.tableName)"
          >
            <span>💻 生成实体代码 (Java/Go/TS/Py/Rust)...</span>
          </button>
          <button
            v-if="!ctxMenu.isView"
            class="dropdown-item"
            @click="emit('gen-mock-data', ctxMenu.profile, ctxMenu.dbName, ctxMenu.tableName)"
          >
            <span>🎲 生成 10 行 Mock 测试数据</span>
          </button>
        </template>
        <div class="dropdown-sep" />
        <button
          class="dropdown-item"
          @click="emit('gen-select', ctxMenu.profile, ctxMenu.dbName, ctxMenu.tableName)"
        >
          <span>{{ isRedisProfile(ctxMenu.profile) ? '插入查询命令到控制台' : t('ctxGenSelect') }}</span>
        </button>
        <template v-if="!isRedisProfile(ctxMenu.profile)">
          <button
            v-if="!ctxMenu.isView"
            class="dropdown-item"
            @click="emit('gen-insert', ctxMenu.profile, ctxMenu.dbName, ctxMenu.tableName)"
          >
            <span>生成 INSERT 模板到编辑器</span>
          </button>
          <button
            v-if="!ctxMenu.isView"
            class="dropdown-item"
            @click="emit('gen-update', ctxMenu.profile, ctxMenu.dbName, ctxMenu.tableName)"
          >
            <span>生成 UPDATE 模板到编辑器</span>
          </button>
          <button
            class="dropdown-item"
            @click="emit('copy-table-columns', ctxMenu.profile, ctxMenu.dbName, ctxMenu.tableName)"
          >
            <span>复制全部列名（逗号分隔）</span>
          </button>
        </template>
        <div class="dropdown-sep" />
        <button
          class="dropdown-item"
          @click="emit('refresh-table', ctxMenu.profile, ctxMenu.dbName, ctxMenu.tableName)"
        >
          <span>{{ isRedisProfile(ctxMenu.profile) ? '刷新 Key 属性与 TTL' : '刷新字段与索引元数据' }}</span>
        </button>
        <button
          v-if="!ctxMenu.isView"
          class="dropdown-item"
          @click="emit('rename-table', ctxMenu.profile, ctxMenu.dbName, ctxMenu.tableName)"
        >
          <span>{{ isRedisProfile(ctxMenu.profile) ? '✏️ 重命名 Key...' : '✏️ 重命名表...' }}</span>
        </button>
        <button class="dropdown-item" @click="emit('copy-text', ctxMenu.tableName)">
          <span>{{ isRedisProfile(ctxMenu.profile) ? '复制 Key 名称' : ctxMenu.isView ? '复制视图名称' : t('ctxCopyName') }}</span>
        </button>
        <div class="dropdown-sep" />
        <button
          v-if="!isRedisProfile(ctxMenu.profile) && !ctxMenu.isView"
          class="dropdown-item"
          style="color: var(--prod)"
          @click="emit('truncate-table', ctxMenu.profile, ctxMenu.dbName, ctxMenu.tableName)"
        >
          <span>🧹 清空表数据 (TRUNCATE)</span>
        </button>
        <button
          class="dropdown-item"
          style="color: var(--prod)"
          @click="emit('drop-table', ctxMenu.profile, ctxMenu.dbName, ctxMenu.tableName)"
        >
          <span>{{ isRedisProfile(ctxMenu.profile) ? '🗑️ 删除 Key (DEL)' : ctxMenu.isView ? '🗑️ 删除视图 (DROP VIEW)' : '🗑️ 删除表 (DROP TABLE)' }}</span>
        </button>
      </template>

      <template v-else-if="ctxMenu.kind === 'redis-folder' && ctxMenu.profile && ctxMenu.dbName && ctxMenu.redisPrefix">
        <button
          class="dropdown-item"
          @click="emit('redis-prefix-action', 'bigkeys', ctxMenu.profile, ctxMenu.dbName, ctxMenu.redisPrefix)"
        >
          <span>📊 扫描此命名空间 Top 20 大 Key...</span>
        </button>
        <button
          class="dropdown-item"
          @click="emit('redis-prefix-action', 'expire', ctxMenu.profile, ctxMenu.dbName, ctxMenu.redisPrefix)"
        >
          <span>⏱️ 批量设置过期时间 (EXPIRE)...</span>
        </button>
        <button
          class="dropdown-item"
          @click="emit('copy-text', ctxMenu.redisPrefix)"
        >
          <span>复制 Key 前缀 ({{ ctxMenu.redisPrefix }})</span>
        </button>
        <div class="dropdown-sep" />
        <button
          class="dropdown-item"
          style="color: var(--prod)"
          @click="emit('redis-prefix-action', 'unlink', ctxMenu.profile, ctxMenu.dbName, ctxMenu.redisPrefix)"
        >
          <span>🗑️ 批量删除前缀 Key ({{ ctxMenu.redisPrefix }}* UNLINK)...</span>
        </button>
      </template>
    </div>
  </div>
</template>
