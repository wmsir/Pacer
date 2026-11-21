<script setup lang="ts">
import { computed, nextTick, ref } from 'vue';
import { useI18n } from '../i18n';
import type { WorkspaceFileEntry } from '../api/ipc';
import { FILE_MANAGER_LABEL } from '../utils/keymap';
import IdeIcon from './IdeIcon.vue';
import FileTreeNode from './FileTreeNode.vue';

const props = defineProps<{
  mountedDir: string;
  rootExpanded: boolean;
  rootEntries: WorkspaceFileEntry[];
  childrenByDir: Record<string, WorkspaceFileEntry[]>;
  expandedDirs: Record<string, boolean>;
  recentFiles: string[];
  showRecentFiles: boolean;
  activeFilePath: string | null;
  dockSide: 'left' | 'right';
  workspaceDir?: string;
  showDemoManual?: boolean;
}>();

const emit = defineEmits<{
  (e: 'mount-folder'): void;
  (e: 'open-workspace-dir'): void;
  (e: 'toggle-root-dir'): void;
  (e: 'close-mounted-dir'): void;
  (e: 'toggle-dir', dirPath: string): void;
  (e: 'collapse-all-dirs'): void;
  (e: 'open-file-dialog'): void;
  (e: 'open-file', path: string): void;
  (e: 'locate-active-file'): void;
  (e: 'reveal-path', path: string): void;
  (e: 'toggle-dock-side'): void;
  (e: 'toggle-show-recent'): void;
  (e: 'remove-recent-file', path: string): void;
  (e: 'clear-recent-files'): void;
  (e: 'open-demo-md'): void;
  (e: 'refresh-dir'): void;
}>();

const { t } = useI18n();
const filterText = ref('');
const recentCollapsed = ref<boolean>(localStorage.getItem('pacer.recentFilesCollapsed') === '1');
const bodyEl = ref<HTMLDivElement | null>(null);

const filteredRootEntries = computed(() => {
  const q = filterText.value.trim().toLowerCase();
  if (!q) return props.rootEntries;
  return props.rootEntries.filter((e) => e.isDir || e.name.toLowerCase().includes(q));
});

function toggleRecentCollapsed() {
  recentCollapsed.value = !recentCollapsed.value;
  localStorage.setItem('pacer.recentFilesCollapsed', recentCollapsed.value ? '1' : '0');
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
      <span class="tw-title" :title="t('toolFiles')">
        <IdeIcon name="folder-open" :size="14" />
        <span class="tw-title-text">{{ t('toolFiles') }}</span>
      </span>
      <div class="tw-actions">
        <button
          class="icon-btn compact"
          :class="{ active: showRecentFiles }"
          :title="showRecentFiles ? '隐藏最近打开的文件' : '显示最近打开的文件'"
          @click="emit('toggle-show-recent')"
        >
          <IdeIcon name="clock" :size="13" />
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
          :title="dockSide === 'right' ? '将文件面板移至左侧' : '将文件面板移至右侧'"
          @click="emit('toggle-dock-side')"
        >
          <IdeIcon :name="dockSide === 'right' ? 'dock-left' : 'dock-right'" :size="13" />
        </button>
      </div>
    </div>

    <div class="file-toolbar-strip">
      <div style="display: flex; align-items: center; gap: 2px">
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
          v-if="workspaceDir"
          class="icon-btn compact"
          :title="`打开 Pacer 工作空间目录 (${workspaceDir})`"
          @click="emit('open-workspace-dir')"
        >
          <IdeIcon name="project" :size="13" color="var(--accent)" />
        </button>
        <button
          v-if="mountedDir"
          class="icon-btn compact"
          :title="t('btnRefresh')"
          @click="emit('refresh-dir')"
        >
          <IdeIcon name="refresh" :size="13" />
        </button>
      </div>

      <div style="display: flex; align-items: center; gap: 2px">
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
          :title="`在${FILE_MANAGER_LABEL}中定位文件/目录`"
          @click="emit('reveal-path', activeFilePath || mountedDir)"
        >
          <IdeIcon name="reveal" :size="13" />
        </button>
      </div>
    </div>

    <div class="tw-search">
      <input v-model="filterText" class="field" placeholder="筛选树形文件 (.md / .sql)..." />
    </div>

    <div ref="bodyEl" class="tw-body">
      <template v-if="mountedDir">
        <!-- Root Mounted Folder Row (Collapsible Pure Tree, No "Up" Button) -->
        <div
          class="tree-row"
          style="padding-left: 6px; font-weight: 600"
          :title="mountedDir"
          @click="emit('toggle-root-dir')"
        >
          <span class="tree-chevron">
            <IdeIcon :name="rootExpanded ? 'chevron-down' : 'chevron-right'" :size="12" />
          </span>
          <IdeIcon :name="rootExpanded ? 'folder-open' : 'folder'" :size="14" />
          <span class="tree-label">{{ baseName(mountedDir) || mountedDir }}</span>
          <span class="tree-hover-actions" @click.stop>
            <button
              class="icon-btn compact"
              :title="t('btnRefresh')"
              @click="emit('refresh-dir')"
            >
              <IdeIcon name="refresh" :size="12" />
            </button>
            <button
              class="icon-btn compact"
              :title="`在${FILE_MANAGER_LABEL}中定位文件夹`"
              @click="emit('reveal-path', mountedDir)"
            >
              <IdeIcon name="reveal" :size="12" />
            </button>
            <button
              class="icon-btn compact"
              title="关闭当前文件夹"
              @click="emit('close-mounted-dir')"
            >
              <IdeIcon name="close" :size="12" />
            </button>
          </span>
        </div>

        <div v-if="rootExpanded">
          <FileTreeNode
            v-for="entry in filteredRootEntries"
            :key="entry.path"
            :entry="entry"
            :depth="1"
            :expanded-dirs="expandedDirs"
            :children-by-dir="childrenByDir"
            :active-file-path="activeFilePath"
            :filter-text="filterText"
            @toggle-dir="emit('toggle-dir', $event)"
            @open-file="emit('open-file', $event)"
            @reveal-path="emit('reveal-path', $event)"
          />
          <div
            v-if="filteredRootEntries.length === 0"
            style="font-size: 11px; color: var(--text-muted); padding: 2px 6px 4px 34px"
          >
            （空目录）
          </div>
        </div>
      </template>

      <div v-else class="empty-state" style="padding: 10px 6px">
        {{ t('emptyFilesHint') }}
        <div style="display: flex; gap: 6px; margin-top: 8px; flex-wrap: wrap">
          <button class="btn btn-primary btn-sm" @click="emit('open-file-dialog')">
            <IdeIcon name="sql-file" :size="12" />
            打开 .md / .sql
          </button>
          <button class="btn btn-sm" @click="emit('mount-folder')">
            <IdeIcon name="folder-open" :size="13" />
            {{ t('openFolder') }}
          </button>
          <button v-if="workspaceDir" class="btn btn-sm" @click="emit('open-workspace-dir')">
            <IdeIcon name="project" :size="12" />
            工作空间
          </button>
          <button v-if="showDemoManual !== false" class="btn btn-sm" @click="emit('open-demo-md')">
            <IdeIcon name="md-file" :size="12" />
            示例手册
          </button>
        </div>
      </div>

      <div v-if="showRecentFiles && recentFiles.length > 0" style="margin-top: 10px">
        <div class="recent-files-header" @click="toggleRecentCollapsed">
          <span style="display: inline-flex; align-items: center; gap: 4px">
            <IdeIcon :name="recentCollapsed ? 'chevron-right' : 'chevron-down'" :size="12" />
            <span>{{ t('recentFiles') }}</span>
            <span class="pill pill-neutral" style="font-size: 10px; padding: 0 5px">
              {{ recentFiles.length }}
            </span>
          </span>
          <span style="display: inline-flex; align-items: center; gap: 2px" @click.stop>
            <button
              class="icon-btn compact"
              title="清空最近打开记录"
              @click="emit('clear-recent-files')"
            >
              <IdeIcon name="trash" :size="12" />
            </button>
            <button
              class="icon-btn compact"
              title="隐藏最近打开的文件"
              @click="emit('toggle-show-recent')"
            >
              <IdeIcon name="close" :size="12" />
            </button>
          </span>
        </div>

        <template v-if="!recentCollapsed">
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
                :title="`在${FILE_MANAGER_LABEL}中定位`"
                @click="emit('reveal-path', rf)"
              >
                <IdeIcon name="reveal" :size="12" />
              </button>
              <button
                class="icon-btn compact"
                title="从最近记录中移除"
                @click="emit('remove-recent-file', rf)"
              >
                <IdeIcon name="close" :size="12" />
              </button>
            </span>
          </div>
        </template>
      </div>
    </div>
  </div>
</template>
