import { reactive, ref } from 'vue';

export type ShortcutCategory = 'execution' | 'tabs' | 'files' | 'panels' | 'editor';

export type ShortcutActionId =
  | 'runCurrent'
  | 'runAll'
  | 'explainSql'
  | 'formatSql'
  | 'newConsole'
  | 'closeTab'
  | 'reopenTab'
  | 'prevTab'
  | 'nextTab'
  | 'openFile'
  | 'saveFile'
  | 'saveFileAs'
  | 'searchEverywhere'
  | 'locateFile'
  | 'openKeymap'
  | 'toggleDbPanel'
  | 'toggleFilesPanel'
  | 'toggleBottomPanel'
  | 'toggleComment'
  | 'duplicateLine'
  | 'deleteLine'
  | 'findReplace'
  | 'zoomIn'
  | 'zoomOut'
  | 'zoomReset';

export interface ShortcutActionDef {
  id: ShortcutActionId;
  category: ShortcutCategory;
  labelZh: string;
  labelEn: string;
  defaultCombo: string;
}

export const SHORTCUT_CATEGORY_LABELS: Record<
  ShortcutCategory,
  { zh: string; en: string }
> = {
  execution: { zh: '执行与查询', en: 'Execution' },
  tabs: { zh: '标签页与窗口', en: 'Tabs & Windows' },
  files: { zh: '文件与搜索', en: 'Files & Search' },
  panels: { zh: '面板切换', en: 'Tool Windows' },
  editor: { zh: '编辑与代码', en: 'Editor' },
};

export const SHORTCUT_ACTIONS: ShortcutActionDef[] = [
  {
    id: 'runCurrent',
    category: 'execution',
    labelZh: '执行当前语句 / 选中 SQL',
    labelEn: 'Execute Statement / Selection',
    defaultCombo: 'Ctrl+Enter',
  },
  {
    id: 'runAll',
    category: 'execution',
    labelZh: '执行全部脚本 / 所有 SQL 块',
    labelEn: 'Execute All Statements',
    defaultCombo: 'Ctrl+Shift+Enter',
  },
  {
    id: 'explainSql',
    category: 'execution',
    labelZh: '查看执行计划 (EXPLAIN)',
    labelEn: 'Explain Plan',
    defaultCombo: 'Ctrl+Alt+E',
  },
  {
    id: 'formatSql',
    category: 'execution',
    labelZh: '格式化 SQL 语句',
    labelEn: 'Reformat SQL',
    defaultCombo: 'Ctrl+Alt+L',
  },
  {
    id: 'newConsole',
    category: 'tabs',
    labelZh: '新建 SQL 查询控制台',
    labelEn: 'New Query Console',
    defaultCombo: 'Ctrl+N',
  },
  {
    id: 'closeTab',
    category: 'tabs',
    labelZh: '关闭当前 SQL / 文件标签页',
    labelEn: 'Close Active Editor Tab',
    defaultCombo: 'Ctrl+W',
  },
  {
    id: 'reopenTab',
    category: 'tabs',
    labelZh: '恢复刚关闭的标签页',
    labelEn: 'Reopen Closed Tab',
    defaultCombo: 'Ctrl+Shift+T',
  },
  {
    id: 'prevTab',
    category: 'tabs',
    labelZh: '切换到左侧标签页',
    labelEn: 'Select Previous Tab',
    defaultCombo: 'Alt+ArrowLeft',
  },
  {
    id: 'nextTab',
    category: 'tabs',
    labelZh: '切换到右侧标签页',
    labelEn: 'Select Next Tab',
    defaultCombo: 'Alt+ArrowRight',
  },
  {
    id: 'openFile',
    category: 'files',
    labelZh: '打开本地 .md / .sql 文件',
    labelEn: 'Open Local File',
    defaultCombo: 'Ctrl+O',
  },
  {
    id: 'saveFile',
    category: 'files',
    labelZh: '保存当前文档',
    labelEn: 'Save Current File',
    defaultCombo: 'Ctrl+S',
  },
  {
    id: 'saveFileAs',
    category: 'files',
    labelZh: '文档另存为',
    labelEn: 'Save As',
    defaultCombo: 'Ctrl+Shift+S',
  },
  {
    id: 'searchEverywhere',
    category: 'files',
    labelZh: '随处搜索 / 查找操作',
    labelEn: 'Search Everywhere',
    defaultCombo: 'Ctrl+K',
  },
  {
    id: 'locateFile',
    category: 'files',
    labelZh: '在文件树中定位当前文件',
    labelEn: 'Locate File in Tree',
    defaultCombo: 'Alt+F1',
  },
  {
    id: 'openKeymap',
    category: 'files',
    labelZh: '打开快捷键配置 (Keymap)',
    labelEn: 'Open Keymap Settings',
    defaultCombo: 'Ctrl+,',
  },
  {
    id: 'toggleDbPanel',
    category: 'panels',
    labelZh: '显示/隐藏 数据库资源管理器',
    labelEn: 'Toggle Database Tool Window',
    defaultCombo: 'Alt+1',
  },
  {
    id: 'toggleFilesPanel',
    category: 'panels',
    labelZh: '显示/隐藏 本地文件与脚本',
    labelEn: 'Toggle Files Tool Window',
    defaultCombo: 'Alt+2',
  },
  {
    id: 'toggleBottomPanel',
    category: 'panels',
    labelZh: '显示/隐藏 结果集与控制台',
    labelEn: 'Toggle Services & Results Panel',
    defaultCombo: 'Alt+4',
  },
  {
    id: 'toggleComment',
    category: 'editor',
    labelZh: '注释 / 取消注释当前行',
    labelEn: 'Toggle Line Comment',
    defaultCombo: 'Ctrl+/',
  },
  {
    id: 'duplicateLine',
    category: 'editor',
    labelZh: '复制当前行 / 选区',
    labelEn: 'Duplicate Line or Selection',
    defaultCombo: 'Ctrl+D',
  },
  {
    id: 'deleteLine',
    category: 'editor',
    labelZh: '删除当前行',
    labelEn: 'Delete Line',
    defaultCombo: 'Ctrl+Y',
  },
  {
    id: 'findReplace',
    category: 'editor',
    labelZh: '文档内查找与替换',
    labelEn: 'Find & Replace in Editor',
    defaultCombo: 'Ctrl+R',
  },
  {
    id: 'zoomIn',
    category: 'editor',
    labelZh: '放大编辑器字号',
    labelEn: 'Zoom In Editor Font',
    defaultCombo: 'Ctrl+=',
  },
  {
    id: 'zoomOut',
    category: 'editor',
    labelZh: '缩小编辑器字号',
    labelEn: 'Zoom Out Editor Font',
    defaultCombo: 'Ctrl+-',
  },
  {
    id: 'zoomReset',
    category: 'editor',
    labelZh: '重置编辑器默认字号',
    labelEn: 'Reset Editor Font Size',
    defaultCombo: 'Ctrl+0',
  },
];

export const KEYMAP_PRESETS: Record<'idea' | 'vscode', Record<ShortcutActionId, string>> = {
  idea: {
    runCurrent: 'Ctrl+Enter',
    runAll: 'Ctrl+Shift+Enter',
    explainSql: 'Ctrl+Alt+E',
    formatSql: 'Ctrl+Alt+L',
    newConsole: 'Ctrl+N',
    closeTab: 'Ctrl+W',
    reopenTab: 'Ctrl+Shift+T',
    prevTab: 'Alt+ArrowLeft',
    nextTab: 'Alt+ArrowRight',
    openFile: 'Ctrl+O',
    saveFile: 'Ctrl+S',
    saveFileAs: 'Ctrl+Shift+S',
    searchEverywhere: 'Ctrl+K',
    locateFile: 'Alt+F1',
    openKeymap: 'Ctrl+,',
    toggleDbPanel: 'Alt+1',
    toggleFilesPanel: 'Alt+2',
    toggleBottomPanel: 'Alt+4',
    toggleComment: 'Ctrl+/',
    duplicateLine: 'Ctrl+D',
    deleteLine: 'Ctrl+Y',
    findReplace: 'Ctrl+R',
    zoomIn: 'Ctrl+=',
    zoomOut: 'Ctrl+-',
    zoomReset: 'Ctrl+0',
  },
  vscode: {
    runCurrent: 'Ctrl+Enter',
    runAll: 'F5',
    explainSql: 'Ctrl+Alt+E',
    formatSql: 'Alt+Shift+F',
    newConsole: 'Ctrl+N',
    closeTab: 'Ctrl+W',
    reopenTab: 'Ctrl+Shift+T',
    prevTab: 'Ctrl+PageUp',
    nextTab: 'Ctrl+PageDown',
    openFile: 'Ctrl+O',
    saveFile: 'Ctrl+S',
    saveFileAs: 'Ctrl+Shift+S',
    searchEverywhere: 'Ctrl+P',
    locateFile: 'Alt+F1',
    openKeymap: 'Ctrl+,',
    toggleDbPanel: 'Ctrl+B',
    toggleFilesPanel: 'Ctrl+Shift+E',
    toggleBottomPanel: 'Ctrl+J',
    toggleComment: 'Ctrl+/',
    duplicateLine: 'Shift+Alt+ArrowDown',
    deleteLine: 'Ctrl+Shift+K',
    findReplace: 'Ctrl+H',
    zoomIn: 'Ctrl+=',
    zoomOut: 'Ctrl+-',
    zoomReset: 'Ctrl+0',
  },
};

function normalizeKeyToken(rawKey: string): string {
  const k = rawKey.trim();
  if (!k) return '';
  if (k === '+' || k === '=') return '=';
  if (k === '_' || k === '-') return '-';
  if (k === ' ') return 'Space';
  const low = k.toLowerCase();
  if (low === 'left' || low === 'arrowleft') return 'ArrowLeft';
  if (low === 'right' || low === 'arrowright') return 'ArrowRight';
  if (low === 'up' || low === 'arrowup') return 'ArrowUp';
  if (low === 'down' || low === 'arrowdown') return 'ArrowDown';
  if (low === 'enter' || low === 'return') return 'Enter';
  if (low === 'pageup') return 'PageUp';
  if (low === 'pagedown') return 'PageDown';
  if (low === 'tab') return 'Tab';
  if (low === 'space') return 'Space';
  if (/^f([1-9]|1[0-2])$/i.test(k)) return k.toUpperCase();
  if (k.length === 1) return k.toUpperCase();
  return k;
}

export function normalizeCombo(raw: string): string {
  const cleaned = raw.trim();
  if (!cleaned) return '';
  // Handle trailing "+" token like "Ctrl++"
  const parts = cleaned === '+' ? ['+'] : cleaned.split('+').filter((p, i, arr) => p !== '' || i === arr.length - 1);
  let hasCtrl = false;
  let hasAlt = false;
  let hasShift = false;
  let mainKey = '';

  for (const part of parts) {
    const low = part.trim().toLowerCase();
    if (!low) continue;
    if (low === 'ctrl' || low === 'control' || low === 'cmd' || low === 'meta' || low === 'mod') {
      hasCtrl = true;
    } else if (low === 'alt' || low === 'option') {
      hasAlt = true;
    } else if (low === 'shift') {
      hasShift = true;
    } else {
      mainKey = normalizeKeyToken(part);
    }
  }
  if (!mainKey) return '';
  const tokens: string[] = [];
  if (hasCtrl) tokens.push('Ctrl');
  if (hasAlt) tokens.push('Alt');
  if (hasShift) tokens.push('Shift');
  tokens.push(mainKey);
  return tokens.join('+');
}

export const IS_MAC: boolean =
  typeof navigator !== 'undefined' && /mac|darwin/i.test(navigator.platform || navigator.userAgent);

export const FILE_MANAGER_LABEL: string = IS_MAC ? '访达 (Finder)' : '系统资源管理器';

function resolveKeyTokenFromEvent(e: KeyboardEvent): string {
  if (e.code === 'Equal' || e.key === '=' || e.key === '+') return '=';
  if (e.code === 'Minus' || e.key === '-' || e.key === '_') return '-';
  if (e.code === 'Slash' || e.key === '/') return '/';
  if (e.code === 'Comma' || e.key === ',') return ',';
  if (e.code === 'Period' || e.key === '.') return '.';
  if (e.code === 'Semicolon' || e.key === ';') return ';';
  // macOS Option+letter/digit produces dead keys or accented glyphs in e.key; recover physical key from e.code.
  if (IS_MAC && e.altKey && e.code) {
    const keyMatch = /^Key([A-Z])$/.exec(e.code);
    if (keyMatch) return keyMatch[1];
    const digitMatch = /^Digit([0-9])$/.exec(e.code);
    if (digitMatch) return digitMatch[1];
  }
  return normalizeKeyToken(e.key);
}

export function eventToCombo(e: KeyboardEvent): string | null {
  if (['Control', 'Shift', 'Alt', 'Meta', 'OS'].includes(e.key)) {
    return null;
  }
  const keyToken = resolveKeyTokenFromEvent(e);
  if (!keyToken || keyToken === 'Escape' || keyToken === 'Backspace' || keyToken === 'Delete') {
    return null;
  }
  const hasMod = e.ctrlKey || e.metaKey;
  // Disallow bare single printable character or Shift+letter as global shortcut (except F1-F12)
  const isFn = /^F([1-9]|1[0-2])$/.test(keyToken);
  if (!hasMod && !e.altKey && !isFn) {
    return null;
  }
  const tokens: string[] = [];
  if (hasMod) tokens.push('Ctrl');
  if (e.altKey) tokens.push('Alt');
  if (e.shiftKey) tokens.push('Shift');
  tokens.push(keyToken);
  return tokens.join('+');
}

export function matchesCombo(e: KeyboardEvent, combo: string): boolean {
  const norm = normalizeCombo(combo);
  if (!norm) return false;
  const parts = norm.split('+');
  const expectCtrl = parts.includes('Ctrl');
  const expectAlt = parts.includes('Alt');
  const expectShift = parts.includes('Shift');
  const expectedKey = parts[parts.length - 1];

  if ((e.ctrlKey || e.metaKey) !== expectCtrl) return false;
  if (e.altKey !== expectAlt) return false;
  if (e.shiftKey !== expectShift) return false;

  return resolveKeyTokenFromEvent(e) === expectedKey;
}

export function comboToCodeMirrorKey(combo: string): string {
  const norm = normalizeCombo(combo);
  if (!norm) return '';
  const parts = norm.split('+');
  const key = parts[parts.length - 1];
  const mods: string[] = [];
  if (parts.includes('Ctrl')) mods.push('Mod');
  if (parts.includes('Alt')) mods.push('Alt');
  if (parts.includes('Shift')) mods.push('Shift');
  let cmKey = key;
  if (cmKey.length === 1 && /[A-Z]/.test(cmKey)) {
    cmKey = cmKey.toLowerCase();
  }
  mods.push(cmKey);
  return mods.join('-');
}

export function formatComboDisplay(combo: string): string {
  if (!combo) return '未设置';
  const tokens = combo
    .replace('ArrowLeft', '←')
    .replace('ArrowRight', '→')
    .replace('ArrowUp', '↑')
    .replace('ArrowDown', '↓')
    .split('+');
  if (!IS_MAC) {
    return tokens.join(' + ');
  }
  return tokens
    .map((token) => {
      if (token === 'Ctrl') return '⌘';
      if (token === 'Alt') return '⌥';
      if (token === 'Shift') return '⇧';
      return token;
    })
    .join(' + ');
}

const STORAGE_KEY = 'pacer.keymap.v1';

interface StoredKeymap {
  preset?: 'idea' | 'vscode' | 'custom';
  doubleShiftSearch?: boolean;
  bindings?: Partial<Record<ShortcutActionId, string>>;
}

function loadInitialKeymap(): {
  preset: 'idea' | 'vscode' | 'custom';
  doubleShiftSearch: boolean;
  bindings: Record<ShortcutActionId, string>;
} {
  const defaults = { ...KEYMAP_PRESETS.idea };
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) {
      return { preset: 'idea', doubleShiftSearch: true, bindings: defaults };
    }
    const parsed = JSON.parse(raw) as StoredKeymap;
    const merged = { ...defaults };
    if (parsed.bindings) {
      for (const def of SHORTCUT_ACTIONS) {
        const val = parsed.bindings[def.id];
        if (typeof val === 'string') {
          merged[def.id] = val === '' ? '' : normalizeCombo(val);
        }
      }
    }
    return {
      preset: parsed.preset === 'vscode' || parsed.preset === 'custom' ? parsed.preset : 'idea',
      doubleShiftSearch: parsed.doubleShiftSearch !== false,
      bindings: merged,
    };
  } catch {
    return { preset: 'idea', doubleShiftSearch: true, bindings: defaults };
  }
}

const initial = loadInitialKeymap();
const bindings = reactive<Record<ShortcutActionId, string>>({ ...initial.bindings });
const doubleShiftSearch = ref<boolean>(initial.doubleShiftSearch);
const activePreset = ref<'idea' | 'vscode' | 'custom'>(initial.preset);

function persistKeymap() {
  try {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        preset: activePreset.value,
        doubleShiftSearch: doubleShiftSearch.value,
        bindings: { ...bindings },
      } satisfies StoredKeymap),
    );
  } catch {
    // non-fatal
  }
}

export function useKeymap() {
  function setBinding(id: ShortcutActionId, rawCombo: string) {
    bindings[id] = rawCombo === '' ? '' : normalizeCombo(rawCombo);
    activePreset.value = 'custom';
    persistKeymap();
  }

  function clearBinding(id: ShortcutActionId) {
    bindings[id] = '';
    activePreset.value = 'custom';
    persistKeymap();
  }

  function findConflict(
    rawCombo: string,
    excludeId: ShortcutActionId,
  ): ShortcutActionDef | null {
    const norm = normalizeCombo(rawCombo);
    if (!norm) return null;
    for (const def of SHORTCUT_ACTIONS) {
      if (def.id === excludeId) continue;
      if (normalizeCombo(bindings[def.id]) === norm) {
        return def;
      }
    }
    return null;
  }

  function resetBinding(id: ShortcutActionId) {
    const def = SHORTCUT_ACTIONS.find((a) => a.id === id);
    if (!def) return;
    bindings[id] = def.defaultCombo;
    activePreset.value = 'custom';
    persistKeymap();
  }

  function applyPreset(preset: 'idea' | 'vscode') {
    const map = KEYMAP_PRESETS[preset];
    for (const def of SHORTCUT_ACTIONS) {
      bindings[def.id] = map[def.id];
    }
    activePreset.value = preset;
    persistKeymap();
  }

  function resetAllBindings() {
    applyPreset('idea');
    doubleShiftSearch.value = true;
    persistKeymap();
  }

  function setDoubleShiftSearch(enabled: boolean) {
    doubleShiftSearch.value = enabled;
    persistKeymap();
  }

  function getCombo(id: ShortcutActionId): string {
    return bindings[id] ?? '';
  }

  function getDisplay(id: ShortcutActionId): string {
    return formatComboDisplay(bindings[id] ?? '');
  }

  return {
    bindings,
    doubleShiftSearch,
    activePreset,
    setBinding,
    clearBinding,
    findConflict,
    resetBinding,
    applyPreset,
    resetAllBindings,
    setDoubleShiftSearch,
    getCombo,
    getDisplay,
  };
}
