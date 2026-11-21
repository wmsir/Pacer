<script setup lang="ts">
import { computed, nextTick, ref } from 'vue';
import { useI18n } from '../i18n';
import type { WorkspaceFileEntry } from '../api/ipc';
import IdeIcon from './IdeIcon.vue';
import FileTreeNode from './FileTreeNode.vue';

const props = defineProps<{
  mountedDir: string;
  rootEntries: WorkspaceFileEntry[];
  childrenByDir: Record<string, WorkspaceFileEntry[]>;
  expandedDirs: Record<string, boolean>;
  recentFiles: string[];
  activeFilePath: string | null;
  dockSide: 'left' | 'right';
  showDemoManual?: boolean;
}>();

const emit = defineEmits<{
  (e: 'mount-folder'): void;
  (e: 'open-dir', dirPath: string): void;
  (e: 'toggle-dir', dirPath: string): void;
  (e: 'collapse-all-dirs'): void;
  (e: 'open-file-dialog'): void;
  (e: 'open-file', path: string): void;
  (e: 'locate-active-file'): void;
  (e: 'reveal-path', path: string): void;
  (e: 'toggle-dock-side'): void;
  (e: 'open-demo-md'): void;
  (e: 'refresh-dir'): void;
}>();

const { t } = useI18n();
const filterText = ref('');
const bodyEl = ref<HTMLDivElement | null>(null);

const filteredRootEntries = computed(() => {
  const q = filterText.value.trim().toLowerCase();
  if (!q) return props.rootEntries;
  return props.rootEntries.filter((e) => e.isDir || e.name.toLowerCase().includes(q));
});

function parentDir(dir: string): string | null {
  const normalized = dir.replace(/\\/g, '/').replace(/\/+$/, '');
  const idx = normalized.lastIndexOf('/');
  if (idx <= 0) return null;
  if (idx === 2 && normalized[1] === ':') return `${normalized.slice(0, 3)}`;
  return normalized.slice(0, idx);
}

function baseName(path: string): string {
  const normalized = path.replace(/\\/g, '/');
  return normalized.split('/').pop() ?? path;
}

async function scrollToActiveFile(targetPath: string) {
  await nextTick();
  if (!bodyEl.value) return;
  const norm = targetPath.replace(/\\/g, '/').toLowerCase();
  const rows = bodyEl.value.querySelectorAll<HTMLElement>('[data-file-path]');
  for (const row of rows) {
    if ((row.dataset.filePath ?? '').replace(/\\/g, '/').toLowerCase() === norm) {
      row.scrollIntoView({ behavior: 'smooth', block: 'center' });
      break;
    }
  }
}

defineExpose({ scrollToActiveFile });
</script>

<template>
  <div style="display: flex; flex-direction: column; height: 100%; width: 100%">
    <div class="tw-header">
      <span style="display: inline-flex; align-items: center; gap: 6px">
        <IdeIcon name="folder-open" :size="14" />
        <span>{{ t('toolFiles') }}</span>
      </span>
      <div class="tw-actions">
        <button
          class="icon-btn compact"
          :disabled="!activeFilePath"
          title="在文件树中定位当前打开的文件"
          @click="emit('locate-active-file')"
        >
          <IdeIcon name="locate" :size="13" />
        </button>
        <button
          class="icon-btn compact"
          :disabled="!activeFilePath && !mountedDir"
          title="在系统资源管理器中定位文件/目录"
          @click="emit('reveal-path', activeFilePath || mountedDir)"
        >
          <IdeIcon name="reveal" :size="13" />
        </button>
        <button
          class="icon-btn compact"
          title="全部折叠目录树"
          @click="emit('collapse-all-dirs')"
        >
          <IdeIcon name="collapse-all" :size="13" />
        </button>
        <button
          class="icon-btn compact"
          :title="t('openFile')"
          @click="emit('open-file-dialog')"
        >
          <IdeIcon name="sql-file" :size="13" />
        </button>
        <button
          class="icon-btn compact"
          :title="t('openFolder')"
          @click="emit('mount-folder')"
        >
          <IdeIcon name="folder" :size="13" />
        </button>
        <button
          v-if="mountedDir"
          class="icon-btn compact"
          :title="t('btnRefresh')"
          @click="emit('refresh-dir')"
        >
          <IdeIcon name="refresh" :size="13" />
        </button>
        <button
          class="icon-btn compact"
          :title="dockSide === 'right' ? '将本地文件面板移至左侧' : '将本地文件面板移至右侧'"
          @click="emit('toggle-dock-side')"
        >
          <IdeIcon :name="dockSide === 'right' ? 'dock-left' : 'dock-right'" :size="13" />
        </button>
      </div>
    </div>

    <div class="tw-search">
      <input v-model="filterText" class="field" placeholder="筛选树形文件 (.md / .sql)..." />
    </div>

    <div ref="bodyEl" class="tw-body">
      <div style="display: flex; gap: 6px; padding: 4px 2px 8px; flex-wrap: wrap">
        <button class="btn btn-primary btn-sm" @click="emit('open-file-dialog')">
          <IdeIcon name="sql-file" :size="12" />
          打开 .md / .sql
        </button>
        <button v-if="showDemoManual !== false" class="btn btn-sm" @click="emit('open-demo-md')">
          <IdeIcon name="md-file" :size="12" />
          示例手册
        </button>
      </div>

      <template v-if="mountedDir">
        <div
          style="
            display: flex;
            align-items: center;
            justify-content: space-between;
            gap: 4px;
            padding: 4px 6px;
            font-size: 11px;
            color: var(--text-secondary);
            border-bottom: 1px solid var(--border-subtle);
            margin-bottom: 4px;
          "
        >
          <span
            :title="mountedDir"
            style="
              display: inline-flex;
              align-items: center;
              gap: 5px;
              overflow: hidden;
              text-overflow: ellipsis;
              white-space: nowrap;
              font-weight: 600;
              color: var(--text-primary);
            "
          >
            <IdeIcon name="folder-open" :size="13" />
            <span style="overflow: hidden; text-overflow: ellipsis">{{ baseName(mountedDir) || mountedDir }}</span>
          </span>
          <button
            v-if="parentDir(mountedDir)"
            class="icon-btn compact"
            title="切换到上一级目录"
            @click="emit('open-dir', parentDir(mountedDir)!)"
          >
            ↑ {{ t('upFolder') }}
          </button>
        </div>

        <FileTreeNode
          v-for="entry in filteredRootEntries"
          :key="entry.path"
          :entry="entry"
          :depth="0"
          :expanded-dirs="expandedDirs"
          :children-by-dir="childrenByDir"
          :active-file-path="activeFilePath"
          :filter-text="filterText"
          @toggle-dir="emit('toggle-dir', $event)"
          @open-file="emit('open-file', $event)"
          @reveal-path="emit('reveal-path', $event)"
        />
      </template>

      <div v-else class="empty-state" style="padding: 10px 6px">
        {{ t('emptyFilesHint') }}
        <div style="margin-top: 8px">
          <button class="btn btn-sm" @click="emit('mount-folder')">
            <IdeIcon name="folder-open" :size="13" />
            {{ t('openFolder') }}
          </button>
        </div>
      </div>

      <div v-if="recentFiles.length > 0" style="margin-top: 12px">
        <div
          style="
            font-size: 11px;
            font-weight: 700;
            color: var(--text-secondary);
            padding: 6px 6px 4px;
            border-top: 1px solid var(--border-subtle);
          "
        >
          {{ t('recentFiles') }}
        </div>
        <div
          v-for="rf in recentFiles"
          :key="rf"
          class="tree-row"
          :class="{ selected: activeFilePath && rf.toLowerCase() === activeFilePath.toLowerCase() }"
          :title="rf"
          @click="emit('open-file', rf)"
        >
          <IdeIcon :name="rf.toLowerCase().endsWith('.md') ? 'md-file' : 'sql-file'" :size="14" />
          <span class="tree-label">{{ baseName(rf) }}</span>
          <span class="tree-hover-actions" @click.stop>
            <button
              class="icon-btn compact"
              title="在系统资源管理器中定位"
              @click="emit('reveal-path', rf)"
            >
              <IdeIcon name="reveal" :size="12" />
            </button>
          </span>
        </div>
      </div>
    </div>
  </div>
</template>
