<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from '../i18n';
import type { WorkspaceFileEntry } from '../api/ipc';
import { formatBytes } from '../utils/sqlTools';

const props = defineProps<{
  mountedDir: string;
  entries: WorkspaceFileEntry[];
  recentFiles: string[];
  showDemoManual?: boolean;
}>();

const emit = defineEmits<{
  (e: 'mount-folder'): void;
  (e: 'open-dir', dirPath: string): void;
  (e: 'open-file-dialog'): void;
  (e: 'open-file', path: string): void;
  (e: 'open-demo-md'): void;
  (e: 'refresh-dir'): void;
}>();

const { t } = useI18n();
const filterText = ref('');

const filteredEntries = computed(() => {
  const q = filterText.value.trim().toLowerCase();
  if (!q) return props.entries;
  return props.entries.filter((e) => e.name.toLowerCase().includes(q));
});

function parentDir(dir: string): string | null {
  const normalized = dir.replace(/\\/g, '/').replace(/\/+$/, '');
  const idx = normalized.lastIndexOf('/');
  if (idx <= 0) return null;
  if (idx === 2 && normalized[1] === ':') return `${normalized.slice(0, 3)}`;
  return normalized.slice(0, idx);
}

function fileIcon(entry: WorkspaceFileEntry): string {
  if (entry.isDir) return '📁';
  switch (entry.extension) {
    case 'md':
    case 'markdown':
      return '📝';
    case 'sql':
    case 'ddl':
    case 'dml':
      return '🗄️';
    case 'json':
    case 'xml':
      return '⚙';
    default:
      return '📄';
  }
}

function baseName(p: string): string {
  const clean = p.replace(/\\/g, '/');
  return clean.slice(clean.lastIndexOf('/') + 1);
}
</script>

<template>
  <div class="tool-window left" style="width: 100%; height: 100%">
    <div class="tw-header">
      <span>{{ t('toolFiles') }}</span>
      <div class="tw-actions">
        <button class="icon-btn compact" :title="t('openFile')" @click="emit('open-file-dialog')">
          📄
        </button>
        <button class="icon-btn compact" :title="t('openFolder')" @click="emit('mount-folder')">
          📁
        </button>
        <button
          v-if="mountedDir"
          class="icon-btn compact"
          :title="t('btnRefresh')"
          @click="emit('refresh-dir')"
        >
          ↻
        </button>
      </div>
    </div>

    <div class="tw-search">
      <input v-model="filterText" class="field" placeholder="筛选文件 (.md / .sql)..." />
    </div>

    <div class="tw-body">
      <div style="display: flex; gap: 6px; padding: 4px 2px 8px; flex-wrap: wrap">
        <button class="btn btn-primary btn-sm" @click="emit('open-file-dialog')">
          打开 .md / .sql
        </button>
        <button v-if="showDemoManual !== false" class="btn btn-sm" @click="emit('open-demo-md')">
          示例 .md 手册
        </button>
      </div>

      <template v-if="mountedDir">
        <div
          style="
            display: flex;
            align-items: center;
            justify-content: space-between;
            padding: 4px 6px;
            font-size: 11px;
            color: var(--text-secondary);
            border-bottom: 1px solid var(--border-subtle);
            margin-bottom: 4px;
          "
        >
          <span :title="mountedDir" style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap">
            {{ mountedDir }}
          </span>
          <button
            v-if="parentDir(mountedDir)"
            class="icon-btn compact"
            @click="emit('open-dir', parentDir(mountedDir)!)"
          >
            ↑ {{ t('upFolder') }}
          </button>
        </div>

        <div
          v-for="entry in filteredEntries"
          :key="entry.path"
          class="tree-row"
          @click="entry.isDir ? emit('open-dir', entry.path) : emit('open-file', entry.path)"
        >
          <span>{{ fileIcon(entry) }}</span>
          <span class="tree-label">{{ entry.name }}</span>
          <span v-if="!entry.isDir" class="tree-meta">{{ formatBytes(entry.size) }}</span>
        </div>
      </template>

      <div v-else class="empty-state" style="padding: 10px 6px">
        {{ t('emptyFilesHint') }}
        <div style="margin-top: 8px">
          <button class="btn btn-sm" @click="emit('mount-folder')">
            📁 {{ t('openFolder') }}
          </button>
        </div>
      </div>

      <div v-if="recentFiles.length > 0" style="margin-top: 12px">
        <div
          style="
            font-size: 11px;
            font-weight: 700;
            color: var(--text-secondary);
            padding: 4px 6px;
            border-top: 1px solid var(--border-subtle);
          "
        >
          {{ t('recentFiles') }}
        </div>
        <div
          v-for="rf in recentFiles"
          :key="rf"
          class="tree-row"
          :title="rf"
          @click="emit('open-file', rf)"
        >
          <span>{{ rf.toLowerCase().endsWith('.md') ? '📝' : '🗄️' }}</span>
          <span class="tree-label">{{ baseName(rf) }}</span>
        </div>
      </div>
    </div>
  </div>
</template>
