<script setup lang="ts">
export type IdeIconName =
  | 'project'
  | 'project-open'
  | 'project-add'
  | 'folder'
  | 'folder-open'
  | 'database'
  | 'schema'
  | 'table'
  | 'view'
  | 'column'
  | 'key'
  | 'index'
  | 'sql-file'
  | 'md-file'
  | 'text-file'
  | 'run'
  | 'run-all'
  | 'explain'
  | 'format'
  | 'plus'
  | 'import'
  | 'refresh'
  | 'edit'
  | 'trash'
  | 'copy'
  | 'save'
  | 'search'
  | 'snippets'
  | 'structure'
  | 'extensions'
  | 'sun'
  | 'moon'
  | 'locate'
  | 'reveal'
  | 'dock-left'
  | 'dock-right'
  | 'collapse-all'
  | 'minimize'
  | 'maximize'
  | 'restore'
  | 'close'
  | 'chevron-right'
  | 'chevron-down'
  | 'chevron-up'
  | 'lock'
  | 'unlock'
  | 'bolt'
  | 'check'
  | 'warning'
  | 'db-tidb'
  | 'db-mysql'
  | 'db-postgres'
  | 'db-sqlite'
  | 'db-redis'
  | 'db-generic'
  | 'leaf'
  | 'clock'
  | 'keyboard'
  | 'pin'
  | 'wrap';

withDefaults(
  defineProps<{
    name: IdeIconName;
    size?: number;
    color?: string;
  }>(),
  {
    size: 15,
    color: '',
  },
);
</script>

<template>
  <svg
    class="ide-svg-icon"
    :width="size"
    :height="size"
    viewBox="0 0 16 16"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
    :style="color ? { color } : undefined"
    aria-hidden="true"
  >
    <!-- Project Folder (Closed / Open) -->
    <template v-if="name === 'project'">
      <path
        d="M2 4.2C2 3.54 2.54 3 3.2 3H6.3L7.6 4.4H12.8C13.46 4.4 14 4.94 14 5.6V12.2C14 12.86 13.46 13.4 12.8 13.4H3.2C2.54 13.4 2 12.86 2 12.2V4.2Z"
        fill="currentColor"
        fill-opacity="0.18"
        stroke="currentColor"
        stroke-width="1.35"
        stroke-linejoin="round"
      />
      <rect x="4.5" y="7.2" width="7" height="1.4" rx="0.7" fill="currentColor" fill-opacity="0.7" />
      <rect x="4.5" y="9.8" width="4.5" height="1.4" rx="0.7" fill="currentColor" fill-opacity="0.5" />
    </template>

    <template v-else-if="name === 'project-open'">
      <path
        d="M2 4.2C2 3.54 2.54 3 3.2 3H6.3L7.5 4.3H12.4C13.06 4.3 13.6 4.84 13.6 5.5V6.5H3.3L2 12.2V4.2Z"
        fill="currentColor"
        fill-opacity="0.16"
        stroke="currentColor"
        stroke-width="1.3"
        stroke-linejoin="round"
      />
      <path
        d="M2.1 12.6L3.4 6.7H14.2L12.9 12.6C12.8 13.05 12.4 13.4 11.9 13.4H3.1C2.5 13.4 2 13.1 2.1 12.6Z"
        fill="currentColor"
        fill-opacity="0.26"
        stroke="currentColor"
        stroke-width="1.3"
        stroke-linejoin="round"
      />
    </template>

    <template v-else-if="name === 'project-add'">
      <path
        d="M2 4.2C2 3.54 2.54 3 3.2 3H6.2L7.4 4.3H12.5C13.16 4.3 13.7 4.84 13.7 5.5V8"
        stroke="currentColor"
        stroke-width="1.35"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
      <path
        d="M8.5 13H3.2C2.54 13 2 12.46 2 11.8V4.2"
        stroke="currentColor"
        stroke-width="1.35"
        stroke-linecap="round"
      />
      <path
        d="M11.8 9.2V14.2M9.3 11.7H14.3"
        stroke="#4D9BFF"
        stroke-width="1.6"
        stroke-linecap="round"
      />
    </template>

    <!-- Local Folders -->
    <template v-else-if="name === 'folder'">
      <path
        d="M1.8 4.2C1.8 3.54 2.34 3 3 3H6.2L7.5 4.4H13C13.66 4.4 14.2 4.94 14.2 5.6V12C14.2 12.66 13.66 13.2 13 13.2H3C2.34 13.2 1.8 12.66 1.8 12V4.2Z"
        fill="#E0A82E"
        fill-opacity="0.2"
        stroke="#E0A82E"
        stroke-width="1.3"
        stroke-linejoin="round"
      />
    </template>

    <template v-else-if="name === 'folder-open'">
      <path
        d="M1.8 4.2C1.8 3.54 2.34 3 3 3H6.1L7.3 4.3H12.3C12.96 4.3 13.5 4.84 13.5 5.5V6.6H3.4L1.8 12.1V4.2Z"
        fill="#E0A82E"
        fill-opacity="0.16"
        stroke="#E0A82E"
        stroke-width="1.25"
        stroke-linejoin="round"
      />
      <path
        d="M2 12.4L3.4 6.8H14.3L12.9 12.4C12.8 12.85 12.4 13.2 11.9 13.2H3C2.35 13.2 1.9 12.85 2 12.4Z"
        fill="#E0A82E"
        fill-opacity="0.28"
        stroke="#E0A82E"
        stroke-width="1.25"
        stroke-linejoin="round"
      />
    </template>

    <!-- Database / Engine Icons -->
    <template v-else-if="name === 'database' || name === 'db-generic'">
      <ellipse cx="8" cy="3.8" rx="5.2" ry="2" fill="currentColor" fill-opacity="0.16" stroke="currentColor" stroke-width="1.3" />
      <path d="M2.8 3.8V12.2C2.8 13.3 5.13 14.2 8 14.2C10.87 14.2 13.2 13.3 13.2 12.2V3.8" stroke="currentColor" stroke-width="1.3" />
      <path d="M2.8 8C2.8 9.1 5.13 10 8 10C10.87 10 13.2 9.1 13.2 8" stroke="currentColor" stroke-width="1.2" />
    </template>

    <template v-else-if="name === 'db-tidb'">
      <polygon points="8,1.6 13.8,4.9 13.8,11.1 8,14.4 2.2,11.1 2.2,4.9" fill="#E30C34" fill-opacity="0.18" stroke="#E30C34" stroke-width="1.35" stroke-linejoin="round" />
      <path d="M5.4 5.7H10.6M8 5.7V11.2" stroke="#E30C34" stroke-width="1.5" stroke-linecap="round" />
    </template>

    <template v-else-if="name === 'db-mysql'">
      <ellipse cx="8" cy="3.8" rx="5.1" ry="1.9" fill="#3B82F6" fill-opacity="0.2" stroke="#3B82F6" stroke-width="1.35" />
      <path d="M2.9 3.8V12.1C2.9 13.2 5.2 14.1 8 14.1C10.8 14.1 13.1 13.2 13.1 12.1V3.8" stroke="#3B82F6" stroke-width="1.35" />
      <path d="M2.9 8C2.9 9.05 5.2 9.9 8 9.9C10.8 9.9 13.1 9.05 13.1 8" stroke="#3B82F6" stroke-width="1.2" />
    </template>

    <template v-else-if="name === 'db-postgres'">
      <rect x="2.3" y="2.3" width="11.4" height="11.4" rx="3" fill="#6366F1" fill-opacity="0.2" stroke="#6366F1" stroke-width="1.35" />
      <path d="M5.6 10.6V5.5H8.4C9.5 5.5 10.2 6.1 10.2 7.1C10.2 8.1 9.5 8.7 8.4 8.7H5.6" stroke="#6366F1" stroke-width="1.45" stroke-linecap="round" stroke-linejoin="round" />
    </template>

    <template v-else-if="name === 'db-sqlite'">
      <ellipse cx="8" cy="3.8" rx="5.1" ry="1.9" fill="#22C55E" fill-opacity="0.2" stroke="#22C55E" stroke-width="1.35" />
      <path d="M2.9 3.8V12.1C2.9 13.2 5.2 14.1 8 14.1C10.8 14.1 13.1 13.2 13.1 12.1V3.8" stroke="#22C55E" stroke-width="1.35" />
      <path d="M2.9 8C2.9 9.05 5.2 9.9 8 9.9C10.8 9.9 13.1 9.05 13.1 8" stroke="#22C55E" stroke-width="1.2" />
    </template>

    <template v-else-if="name === 'db-redis'">
      <path d="M8 2L13.8 4.8L8 7.6L2.2 4.8L8 2Z" fill="#DC382D" fill-opacity="0.24" stroke="#DC382D" stroke-width="1.3" stroke-linejoin="round" />
      <path d="M2.2 8.2L8 11L13.8 8.2" stroke="#DC382D" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round" />
      <path d="M2.2 11.2L8 14L13.8 11.2" stroke="#DC382D" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round" />
    </template>

    <template v-else-if="name === 'schema'">
      <path d="M8 2.2L13.8 5.2L8 8.2L2.2 5.2L8 2.2Z" fill="#5B9BD5" fill-opacity="0.22" stroke="#5B9BD5" stroke-width="1.3" stroke-linejoin="round" />
      <path d="M2.2 8.4L8 11.4L13.8 8.4" stroke="#5B9BD5" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round" />
      <path d="M2.2 11.2L8 14.2L13.8 11.2" stroke="#5B9BD5" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round" />
    </template>

    <template v-else-if="name === 'table'">
      <rect x="2.2" y="2.6" width="11.6" height="10.8" rx="1.6" fill="#38BDF8" fill-opacity="0.12" stroke="#38BDF8" stroke-width="1.3" />
      <path d="M2.2 6H13.8M6.6 6V13.4M10.4 6V13.4M2.2 9.6H13.8" stroke="#38BDF8" stroke-width="1.15" />
    </template>

    <template v-else-if="name === 'view'">
      <rect x="2" y="2.6" width="12" height="10.8" rx="1.8" fill="#8B5CF6" fill-opacity="0.14" stroke="#8B5CF6" stroke-width="1.25" />
      <path d="M3.6 8C4.7 6.05 6.25 5.1 8 5.1C9.75 5.1 11.3 6.05 12.4 8C11.3 9.95 9.75 10.9 8 10.9C6.25 10.9 4.7 9.95 3.6 8Z" stroke="#8B5CF6" stroke-width="1.2" stroke-linejoin="round" />
      <circle cx="8" cy="8" r="1.4" fill="#8B5CF6" />
    </template>

    <template v-else-if="name === 'column'">
      <rect x="3" y="3.2" width="10" height="9.6" rx="1.6" stroke="currentColor" stroke-width="1.25" opacity="0.72" />
      <path d="M6.4 3.2V12.8" stroke="currentColor" stroke-width="1.2" opacity="0.72" />
    </template>

    <template v-else-if="name === 'key'">
      <circle cx="5.6" cy="6.2" r="2.6" fill="#E0A82E" fill-opacity="0.2" stroke="#E0A82E" stroke-width="1.35" />
      <path d="M7.6 8.2L13.2 13.6M10.8 11.2L12.5 9.5M9.2 9.6L10.6 8.2" stroke="#E0A82E" stroke-width="1.35" stroke-linecap="round" />
    </template>

    <template v-else-if="name === 'index' || name === 'explain' || name === 'bolt'">
      <path d="M9.2 1.8L3.6 8.8H7.8L6.8 14.2L12.4 7.2H8.2L9.2 1.8Z" fill="#A78BFA" fill-opacity="0.22" stroke="#A78BFA" stroke-width="1.3" stroke-linejoin="round" />
    </template>

    <!-- File Icons -->
    <template v-else-if="name === 'sql-file'">
      <path d="M3.2 2.2H9.6L12.8 5.4V13.8H3.2V2.2Z" fill="#38BDF8" fill-opacity="0.14" stroke="#38BDF8" stroke-width="1.3" stroke-linejoin="round" />
      <path d="M9.4 2.2V5.6H12.8" stroke="#38BDF8" stroke-width="1.25" />
      <path d="M5.2 8.4H10.8M5.2 10.9H9.2" stroke="#38BDF8" stroke-width="1.3" stroke-linecap="round" />
    </template>

    <template v-else-if="name === 'md-file'">
      <rect x="2" y="2.8" width="12" height="10.4" rx="1.8" fill="#818CF8" fill-opacity="0.18" stroke="#818CF8" stroke-width="1.3" />
      <path d="M4.3 10.4V5.6L6.4 8.2L8.5 5.6V10.4" stroke="#818CF8" stroke-width="1.35" stroke-linecap="round" stroke-linejoin="round" />
      <path d="M11.3 5.8V10.2M10 8.9L11.3 10.3L12.6 8.9" stroke="#818CF8" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round" />
    </template>

    <template v-else-if="name === 'text-file'">
      <path d="M3.2 2.2H9.6L12.8 5.4V13.8H3.2V2.2Z" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round" opacity="0.8" />
      <path d="M5.2 8.2H10.8M5.2 10.8H9.2" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" opacity="0.7" />
    </template>

    <!-- Action Icons -->
    <template v-else-if="name === 'run'">
      <path d="M4.2 2.8L12.8 8L4.2 13.2V2.8Z" fill="currentColor" stroke="currentColor" stroke-width="1.1" stroke-linejoin="round" />
    </template>

    <template v-else-if="name === 'run-all'">
      <path d="M2.4 3.2L8.2 8L2.4 12.8V3.2Z" fill="currentColor" />
      <path d="M8.4 3.2L14.2 8L8.4 12.8V3.2Z" fill="currentColor" />
    </template>

    <template v-else-if="name === 'format'">
      <path d="M2.5 3.6H13.5M2.5 6.6H10.5M4.8 9.6H13.5M4.8 12.6H9.8" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" />
    </template>

    <template v-else-if="name === 'plus'">
      <path d="M8 2.8V13.2M2.8 8H13.2" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
    </template>

    <template v-else-if="name === 'import'">
      <path d="M8 2.5V10.2M5.2 7.6L8 10.4L10.8 7.6" stroke="currentColor" stroke-width="1.45" stroke-linecap="round" stroke-linejoin="round" />
      <path d="M2.5 11.2V12.8C2.5 13.35 2.95 13.8 3.5 13.8H12.5C13.05 13.8 13.5 13.35 13.5 12.8V11.2" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" />
    </template>

    <template v-else-if="name === 'refresh'">
      <path d="M13 8A5 5 0 1 1 11.5 4.4M13 2.5V5.4H10.1" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" />
    </template>

    <template v-else-if="name === 'edit'">
      <path d="M11.2 2.4L13.6 4.8L5.4 13H3V10.6L11.2 2.4Z" stroke="currentColor" stroke-width="1.35" stroke-linejoin="round" />
    </template>

    <template v-else-if="name === 'trash'">
      <path d="M2.8 4.4H13.2M6 4.4V2.8H10V4.4M4.3 4.4L4.9 13.2H11.1L11.7 4.4" stroke="currentColor" stroke-width="1.35" stroke-linecap="round" stroke-linejoin="round" />
    </template>

    <template v-else-if="name === 'copy'">
      <rect x="5.4" y="5.2" width="7.8" height="8.4" rx="1.4" stroke="currentColor" stroke-width="1.3" />
      <path d="M3.2 10.6V3.6C3.2 2.95 3.72 2.4 4.38 2.4H10.4" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" />
    </template>

    <template v-else-if="name === 'save'">
      <path d="M3 2.5H11.2L13.5 4.8V13.5H3V2.5Z" stroke="currentColor" stroke-width="1.35" stroke-linejoin="round" />
      <path d="M5.5 2.5V6H10.2V2.5M5.2 13.5V9.5H10.8V13.5" stroke="currentColor" stroke-width="1.25" />
    </template>

    <template v-else-if="name === 'search'">
      <circle cx="7.1" cy="7.1" r="4.4" stroke="currentColor" stroke-width="1.45" />
      <path d="M10.4 10.4L13.6 13.6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
    </template>

    <template v-else-if="name === 'snippets'">
      <path d="M5.4 4.2L2.2 8L5.4 11.8M10.6 4.2L13.8 8L10.6 11.8M9 3.2L7 12.8" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" />
    </template>

    <template v-else-if="name === 'structure'">
      <rect x="2.4" y="2.4" width="4.2" height="3.2" rx="0.8" stroke="currentColor" stroke-width="1.3" />
      <rect x="9.4" y="6.4" width="4.2" height="3" rx="0.8" stroke="currentColor" stroke-width="1.3" />
      <rect x="9.4" y="11" width="4.2" height="3" rx="0.8" stroke="currentColor" stroke-width="1.3" />
      <path d="M4.5 5.6V12.5H9.4M4.5 7.9H9.4" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" />
    </template>

    <template v-else-if="name === 'extensions'">
      <path d="M6.4 2.8C6.4 2.1 7.1 1.6 7.9 1.6C8.7 1.6 9.4 2.1 9.4 2.8V4H12.2V7.2C12.9 7.2 13.5 7.8 13.5 8.6C13.5 9.4 12.9 10 12.2 10V13.2H3.6V4H6.4V2.8Z" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round" />
    </template>

    <template v-else-if="name === 'sun'">
      <circle cx="8" cy="8" r="2.8" stroke="currentColor" stroke-width="1.4" />
      <path d="M8 1.8V3.2M8 12.8V14.2M1.8 8H3.2M12.8 8H14.2M3.6 3.6L4.6 4.6M11.4 11.4L12.4 12.4M12.4 3.6L11.4 4.6M4.6 11.4L3.6 12.4" stroke="currentColor" stroke-width="1.35" stroke-linecap="round" />
    </template>

    <template v-else-if="name === 'moon'">
      <path d="M12.8 9.8A5.4 5.4 0 1 1 6.2 3.2A4.3 4.3 0 0 0 12.8 9.8Z" stroke="currentColor" stroke-width="1.4" stroke-linejoin="round" />
    </template>

    <template v-else-if="name === 'locate'">
      <circle cx="8" cy="8" r="4.2" stroke="currentColor" stroke-width="1.35" />
      <circle cx="8" cy="8" r="1.4" fill="currentColor" />
      <path d="M8 1.6V3.8M8 12.2V14.4M1.6 8H3.8M12.2 8H14.4" stroke="currentColor" stroke-width="1.35" stroke-linecap="round" />
    </template>

    <template v-else-if="name === 'reveal'">
      <path d="M6.8 3.2H3.4C2.74 3.2 2.2 3.74 2.2 4.4V12.6C2.2 13.26 2.74 13.8 3.4 13.8H11.6C12.26 13.8 12.8 13.26 12.8 12.6V9.2" stroke="currentColor" stroke-width="1.35" stroke-linecap="round" />
      <path d="M9.4 2.4H13.6V6.6M13.4 2.6L7.4 8.6" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" />
    </template>

    <template v-else-if="name === 'dock-left'">
      <rect x="2" y="2.6" width="12" height="10.8" rx="1.6" stroke="currentColor" stroke-width="1.3" />
      <rect x="2.6" y="3.2" width="4" height="9.6" rx="0.8" fill="currentColor" fill-opacity="0.45" />
      <path d="M6.6 2.6V13.4" stroke="currentColor" stroke-width="1.25" />
    </template>

    <template v-else-if="name === 'dock-right'">
      <rect x="2" y="2.6" width="12" height="10.8" rx="1.6" stroke="currentColor" stroke-width="1.3" />
      <rect x="9.4" y="3.2" width="4" height="9.6" rx="0.8" fill="currentColor" fill-opacity="0.45" />
      <path d="M9.4 2.6V13.4" stroke="currentColor" stroke-width="1.25" />
    </template>

    <template v-else-if="name === 'collapse-all'">
      <rect x="2.6" y="3.8" width="9.2" height="9.2" rx="1.4" stroke="currentColor" stroke-width="1.3" />
      <path d="M5 8.4H9.4" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" />
      <path d="M5.2 2.2H12.2C13 2.2 13.6 2.8 13.6 3.6V10.6" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" opacity="0.65" />
    </template>

    <template v-else-if="name === 'minimize'">
      <path d="M3 11.5H13" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
    </template>

    <template v-else-if="name === 'maximize'">
      <rect x="2.6" y="2.8" width="10.8" height="10.4" rx="1.4" stroke="currentColor" stroke-width="1.35" />
      <path d="M2.6 5.6H13.4" stroke="currentColor" stroke-width="1.2" />
    </template>

    <template v-else-if="name === 'restore'">
      <rect x="4.6" y="2.6" width="8.6" height="8" rx="1.2" stroke="currentColor" stroke-width="1.25" />
      <path d="M11.2 10.6V12.2C11.2 12.86 10.66 13.4 10 13.4H3.8C3.14 13.4 2.6 12.86 2.6 12.2V6.4C2.6 5.74 3.14 5.2 3.8 5.2H4.6" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" />
    </template>

    <template v-else-if="name === 'close'">
      <path d="M4.2 4.2L11.8 11.8M11.8 4.2L4.2 11.8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
    </template>

    <template v-else-if="name === 'chevron-right'">
      <path d="M6 4L10 8L6 12" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" />
    </template>

    <template v-else-if="name === 'chevron-down'">
      <path d="M4 6L8 10L12 6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" />
    </template>

    <template v-else-if="name === 'chevron-up'">
      <path d="M4.2 10L8 6.2L11.8 10" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" />
    </template>

    <template v-else-if="name === 'lock'">
      <rect x="3.4" y="7.2" width="9.2" height="6.4" rx="1.4" stroke="currentColor" stroke-width="1.35" />
      <path d="M5.4 7.2V5.3C5.4 3.85 6.55 2.7 8 2.7C9.45 2.7 10.6 3.85 10.6 5.3V7.2" stroke="currentColor" stroke-width="1.35" />
    </template>

    <template v-else-if="name === 'unlock'">
      <rect x="3.4" y="7.2" width="9.2" height="6.4" rx="1.4" stroke="currentColor" stroke-width="1.35" />
      <path d="M5.4 7.2V5.2C5.4 3.75 6.55 2.6 8 2.6C9.1 2.6 10 3.25 10.4 4.2" stroke="currentColor" stroke-width="1.35" stroke-linecap="round" />
    </template>

    <template v-else-if="name === 'check'">
      <path d="M3.2 8.4L6.4 11.6L12.8 4.6" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" />
    </template>

    <template v-else-if="name === 'warning'">
      <path d="M8 2.4L14 13.2H2L8 2.4Z" fill="#E0A82E" fill-opacity="0.18" stroke="#E0A82E" stroke-width="1.35" stroke-linejoin="round" />
      <path d="M8 6.4V9.6M8 11.4V11.6" stroke="#E0A82E" stroke-width="1.5" stroke-linecap="round" />
    </template>

    <template v-else-if="name === 'leaf'">
      <path
        d="M13.2 2.8C8.2 2.8 3.6 5.8 3.2 12.2C6.4 12.4 12.8 11.2 13.2 2.8Z"
        fill="currentColor"
        fill-opacity="0.22"
        stroke="currentColor"
        stroke-width="1.35"
        stroke-linejoin="round"
      />
      <path
        d="M2.6 13.4C5.4 10.6 8.6 7.8 11.8 4.8"
        stroke="currentColor"
        stroke-width="1.35"
        stroke-linecap="round"
      />
    </template>

    <template v-else-if="name === 'clock'">
      <circle cx="8" cy="8" r="5.5" stroke="currentColor" stroke-width="1.35" />
      <path d="M8 4.8V8.2L10.4 9.6" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" />
    </template>

    <template v-else-if="name === 'keyboard'">
      <rect x="1.8" y="3.6" width="12.4" height="8.8" rx="1.6" stroke="currentColor" stroke-width="1.3" />
      <path d="M4.2 6.4H5.4M7.2 6.4H8.4M10.2 6.4H11.6M4.2 8.8H5.8M7.6 8.8H8.8M10.6 8.8H11.6M5.2 10.7H10.8" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" />
    </template>

    <template v-else-if="name === 'pin'">
      <path d="M9.8 2.4L13.6 6.2L12 7.8L10.6 7.2L8 9.8V11.8L6.8 13L4.9 11.1L2.4 13.6M4.9 11.1L3 9.2L4.2 8H6.2L8.8 5.4L8.2 4L9.8 2.4Z" stroke="currentColor" stroke-width="1.25" stroke-linejoin="round" stroke-linecap="round" />
    </template>

    <template v-else-if="name === 'wrap'">
      <path d="M2.5 4H13.5M2.5 8H11.2C12.5 8 13.5 8.9 13.5 10.1C13.5 11.3 12.5 12.2 11.2 12.2H8.4M10 10.6L8.2 12.2L10 13.8M2.5 12.2H5.6" stroke="currentColor" stroke-width="1.35" stroke-linecap="round" stroke-linejoin="round" />
    </template>
  </svg>
</template>
