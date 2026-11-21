<script setup lang="ts">
import type { WorkspaceFileEntry } from '../api/ipc';
import { FILE_MANAGER_LABEL } from '../utils/keymap';
import { formatBytes } from '../utils/sqlTools';
import IdeIcon from './IdeIcon.vue';

defineOptions({ name: 'FileTreeNode' });

const props = defineProps<{
  entry: WorkspaceFileEntry;
  depth: number;
  expandedDirs: Record<string, boolean>;
  childrenByDir: Record<string, WorkspaceFileEntry[]>;
  activeFilePath: string | null;
  filterText: string;
}>();

const emit = defineEmits<{
  (e: 'toggle-dir', path: string): void;
  (e: 'open-file', path: string): void;
  (e: 'reveal-path', path: string): void;
}>();

function iconForEntry(entry: WorkspaceFileEntry) {
  if (entry.isDir) {
    return props.expandedDirs[entry.path] ? 'folder-open' : 'folder';
  }
  const ext = (entry.extension ?? '').toLowerCase();
  if (ext === 'md' || ext === 'markdown' || ext === 'mdx') return 'md-file';
  if (ext === 'sql' || ext === 'ddl' || ext === 'dml' || ext === 'psql') return 'sql-file';
  return 'text-file';
}

function normalizedEqual(a?: string | null, b?: string | null): boolean {
  if (!a || !b) return false;
  return a.replace(/\\/g, '/').toLowerCase() === b.replace(/\\/g, '/').toLowerCase();
}

function visibleChildren(dirPath: string): WorkspaceFileEntry[] {
  const list = props.childrenByDir[dirPath] ?? [];
  const q = props.filterText.trim().toLowerCase();
  if (!q) return list;
  return list.filter((child) => child.isDir || child.name.toLowerCase().includes(q));
}
</script>

<template>
  <div>
    <div
      class="tree-row"
      :class="{ selected: !entry.isDir && normalizedEqual(entry.path, activeFilePath) }"
      :style="{ paddingLeft: `${6 + depth * 15}px` }"
      :data-file-path="entry.path"
      :title="entry.path"
      @click="entry.isDir ? emit('toggle-dir', entry.path) : emit('open-file', entry.path)"
    >
      <span class="tree-chevron">
        <IdeIcon
          v-if="entry.isDir"
          :name="expandedDirs[entry.path] ? 'chevron-down' : 'chevron-right'"
          :size="12"
        />
      </span>
      <IdeIcon :name="iconForEntry(entry)" :size="14" />
      <span class="tree-label">{{ entry.name }}</span>
      <span v-if="!entry.isDir" class="tree-meta">{{ formatBytes(entry.size) }}</span>
      <span class="tree-hover-actions" @click.stop>
        <button
          class="icon-btn compact"
          :title="`在${FILE_MANAGER_LABEL}中定位`"
          @click="emit('reveal-path', entry.path)"
        >
          <IdeIcon name="reveal" :size="12" />
        </button>
      </span>
    </div>

    <div v-if="entry.isDir && expandedDirs[entry.path]">
      <FileTreeNode
        v-for="child in visibleChildren(entry.path)"
        :key="child.path"
        :entry="child"
        :depth="depth + 1"
        :expanded-dirs="expandedDirs"
        :children-by-dir="childrenByDir"
        :active-file-path="activeFilePath"
        :filter-text="filterText"
        @toggle-dir="emit('toggle-dir', $event)"
        @open-file="emit('open-file', $event)"
        @reveal-path="emit('reveal-path', $event)"
      />
      <div
        v-if="(childrenByDir[entry.path] ?? []).length === 0"
        style="font-size: 11px; color: var(--text-muted); padding: 2px 6px"
        :style="{ paddingLeft: `${26 + depth * 15}px` }"
      >
        （空目录）
      </div>
    </div>
  </div>
</template>
