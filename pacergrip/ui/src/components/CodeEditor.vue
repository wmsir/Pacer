<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue';
import {
  Compartment,
  EditorState,
  RangeSetBuilder,
  StateField,
} from '@codemirror/state';
import {
  Decoration,
  type DecorationSet,
  EditorView,
  WidgetType,
  drawSelection,
  highlightActiveLine,
  highlightActiveLineGutter,
  highlightSpecialChars,
  keymap,
  lineNumbers,
} from '@codemirror/view';
import {
  copyLineDown,
  defaultKeymap,
  deleteLine,
  history,
  historyKeymap,
  indentWithTab,
  toggleLineComment,
} from '@codemirror/commands';
import {
  LanguageDescription,
  bracketMatching,
  defaultHighlightStyle,
  foldGutter,
  foldKeymap,
  indentOnInput,
  syntaxHighlighting,
} from '@codemirror/language';
import { highlightSelectionMatches, openSearchPanel, searchKeymap } from '@codemirror/search';
import {
  autocompletion,
  closeBrackets,
  closeBracketsKeymap,
  completionKeymap,
} from '@codemirror/autocomplete';
import {
  MySQL,
  PostgreSQL,
  SQLite,
  StandardSQL,
  sql as sqlLang,
} from '@codemirror/lang-sql';
import { markdown as markdownLang } from '@codemirror/lang-markdown';
import { oneDark } from '@codemirror/theme-one-dark';

import type { MarkdownSqlBlock } from '../api/ipc';
import { extractMarkdownSqlBlocksLocal } from '../utils/sqlTools';
import { comboToCodeMirrorKey, useKeymap } from '../utils/keymap';

export interface EditorCursorTelemetry {
  line: number;
  col: number;
  lineCount: number;
  charCount: number;
  cursorOffset: number;
  selectedText: string;
  selectedChars: number;
  selectedLines: number;
}

const props = defineProps<{
  modelValue: string;
  fileKind: string;
  engineKey: string;
  theme: 'dark' | 'light' | 'eyecare';
  readOnly?: boolean;
  fontSize?: number;
  wordWrap?: boolean;
  schemaMap?: Record<string, string[]>;
  runLabel?: string;
  explainLabel?: string;
  copyConsoleLabel?: string;
}>();

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void;
  (e: 'cursor-change', telemetry: EditorCursorTelemetry): void;
  (e: 'run-shortcut', all: boolean): void;
  (e: 'explain-shortcut'): void;
  (e: 'save-shortcut'): void;
  (e: 'format-shortcut'): void;
  (e: 'run-md-block', block: MarkdownSqlBlock): void;
  (e: 'explain-md-block', block: MarkdownSqlBlock): void;
  (e: 'open-md-block-console', block: MarkdownSqlBlock): void;
}>();

const userKeymap = useKeymap();
const hostEl = ref<HTMLDivElement | null>(null);
let view: EditorView | null = null;
let syncingFromProp = false;

const langCompartment = new Compartment();
const themeCompartment = new Compartment();
const readOnlyCompartment = new Compartment();
const lensCompartment = new Compartment();
const keymapCompartment = new Compartment();
const fontCompartment = new Compartment();
const wrapCompartment = new Compartment();

class MarkdownSqlLensWidget extends WidgetType {
  constructor(
    private readonly block: MarkdownSqlBlock,
    private readonly runLabel: string,
    private readonly explainLabel: string,
    private readonly copyLabel: string,
  ) {
    super();
  }

  eq(other: MarkdownSqlLensWidget): boolean {
    return (
      other.block.index === this.block.index &&
      other.block.startLine === this.block.startLine &&
      other.block.endLine === this.block.endLine &&
      other.block.sql === this.block.sql &&
      other.runLabel === this.runLabel
    );
  }

  toDOM(): HTMLElement {
    const bar = document.createElement('div');
    bar.className = 'cm-md-sql-lens';

    const runBtn = document.createElement('button');
    runBtn.type = 'button';
    runBtn.className = 'cm-md-sql-lens-btn';
    runBtn.textContent = `▶ ${this.runLabel}`;
    runBtn.addEventListener('mousedown', (ev) => {
      ev.preventDefault();
      ev.stopPropagation();
      emit('run-md-block', this.block);
    });

    const expBtn = document.createElement('button');
    expBtn.type = 'button';
    expBtn.className = 'cm-md-sql-lens-btn secondary';
    expBtn.textContent = `⚡ ${this.explainLabel}`;
    expBtn.addEventListener('mousedown', (ev) => {
      ev.preventDefault();
      ev.stopPropagation();
      emit('explain-md-block', this.block);
    });

    const openBtn = document.createElement('button');
    openBtn.type = 'button';
    openBtn.className = 'cm-md-sql-lens-btn secondary';
    openBtn.textContent = `⧉ ${this.copyLabel}`;
    openBtn.addEventListener('mousedown', (ev) => {
      ev.preventDefault();
      ev.stopPropagation();
      emit('open-md-block-console', this.block);
    });

    const meta = document.createElement('span');
    const titlePart = this.block.title ? ` · ${this.block.title}` : '';
    meta.textContent = `${this.block.language.toUpperCase()} #${this.block.index + 1} (L${this.block.contentStartLine}-${this.block.contentEndLine})${titlePart}`;

    bar.append(runBtn, expBtn, openBtn, meta);
    return bar;
  }

  ignoreEvent(): boolean {
    return true;
  }
}

function buildMarkdownLensExtension() {
  if (props.fileKind !== 'markdown') return [];

  const runText = props.runLabel || '执行';
  const explainText = props.explainLabel || 'EXPLAIN';
  const copyText = props.copyConsoleLabel || '控制台打开';

  const computeDecorations = (state: EditorState): DecorationSet => {
    const text = state.doc.toString();
    const blocks = extractMarkdownSqlBlocksLocal(text);
    const builder = new RangeSetBuilder<Decoration>();
    for (const blk of blocks) {
      if (blk.startLine >= 1 && blk.startLine <= state.doc.lines) {
        const line = state.doc.line(blk.startLine);
        builder.add(
          line.from,
          line.from,
          Decoration.widget({
            widget: new MarkdownSqlLensWidget(blk, runText, explainText, copyText),
            block: true,
            side: -1,
          }),
        );
      }
    }
    return builder.finish();
  };

  return StateField.define<DecorationSet>({
    create(state) {
      return computeDecorations(state);
    },
    update(decos, tr) {
      if (tr.docChanged) {
        return computeDecorations(tr.state);
      }
      return decos;
    },
    provide: (f) => EditorView.decorations.from(f),
  });
}

function sqlDialectFor(engine: string) {
  const e = engine.toLowerCase();
  if (e.includes('sqlite')) return SQLite;
  if (e.includes('postgres') || e.includes('gauss') || e.includes('kingbase')) {
    return PostgreSQL;
  }
  if (e.includes('mysql') || e.includes('maria') || e.includes('tidb') || e.includes('oceanbase')) {
    return MySQL;
  }
  return StandardSQL;
}

function buildLanguageExtension() {
  const schema = props.schemaMap ?? {};
  const activeSqlSupport = sqlLang({
    dialect: sqlDialectFor(props.engineKey),
    schema,
    upperCaseKeywords: true,
  });

  if (props.fileKind === 'markdown') {
    const mdCodeLanguages = [
      LanguageDescription.of({
        name: 'SQL',
        alias: ['sql', 'ddl', 'dml', 'dm', 'dameng', 'oracle', 'plsql', 'tsql', 'mssql', 'sqlserver', 'clickhouse'],
        support: activeSqlSupport,
      }),
      LanguageDescription.of({
        name: 'MySQL',
        alias: ['mysql', 'mariadb', 'tidb', 'oceanbase'],
        support: sqlLang({ dialect: MySQL, schema, upperCaseKeywords: true }),
      }),
      LanguageDescription.of({
        name: 'PostgreSQL',
        alias: ['postgresql', 'postgres', 'pgsql', 'plpgsql', 'kingbase', 'opengauss'],
        support: sqlLang({ dialect: PostgreSQL, schema, upperCaseKeywords: true }),
      }),
      LanguageDescription.of({
        name: 'SQLite',
        alias: ['sqlite', 'sqlite3'],
        support: sqlLang({ dialect: SQLite, schema, upperCaseKeywords: true }),
      }),
    ];
    return markdownLang({ codeLanguages: mdCodeLanguages });
  }
  return activeSqlSupport;
}

function buildThemeExtension() {
  if (props.theme === 'dark') {
    return [
      oneDark,
      EditorView.theme(
        {
          '&': { backgroundColor: '#1E1F22', color: '#DFE1E5' },
          '.cm-gutters': {
            backgroundColor: '#1E1F22',
            color: '#6F737A',
            borderRight: '1px solid #323438',
          },
          '.cm-activeLine': { backgroundColor: 'rgba(255,255,255,0.035)' },
          '.cm-activeLineGutter': { backgroundColor: '#2B2D30', color: '#DFE1E5' },
          '&.cm-focused .cm-selectionBackground, ::selection': {
            backgroundColor: '#2E436E',
          },
        },
        { dark: true },
      ),
    ];
  }
  if (props.theme === 'eyecare') {
    return [
      syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
      EditorView.theme(
        {
          '&': { backgroundColor: '#CCE8CF', color: '#1C2B20' },
          '.cm-content': { caretColor: '#1C2B20' },
          '.cm-cursor, .cm-dropCursor': { borderLeftColor: '#1C2B20' },
          '.cm-gutters': {
            backgroundColor: '#C0DFC3',
            color: '#58725E',
            borderRight: '1px solid #A9CCA9',
          },
          '.cm-activeLine': { backgroundColor: 'rgba(34,122,82,0.085)' },
          '.cm-activeLineGutter': { backgroundColor: '#B4D6B8', color: '#1C2B20' },
          '&.cm-focused .cm-selectionBackground, ::selection': {
            backgroundColor: '#98CCA4',
          },
          '.cm-panels': { backgroundColor: '#C0DFC3', color: '#1C2B20' },
          '.cm-tooltip': {
            backgroundColor: '#D7EFD9',
            border: '1px solid #9FC4A4',
            color: '#1C2B20',
          },
        },
        { dark: false },
      ),
    ];
  }
  return [
    syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
    EditorView.theme(
      {
        '&': { backgroundColor: '#FFFFFF', color: '#181A1F' },
        '.cm-gutters': {
          backgroundColor: '#F7F8FA',
          color: '#818594',
          borderRight: '1px solid #E7E9EE',
        },
        '.cm-activeLine': { backgroundColor: 'rgba(53,116,240,0.055)' },
        '.cm-activeLineGutter': { backgroundColor: '#EBEEF5', color: '#181A1F' },
        '&.cm-focused .cm-selectionBackground, ::selection': {
          backgroundColor: '#D4E2FF',
        },
      },
      { dark: false },
    ),
  ];
}

function emitCursorTelemetry(state: EditorState) {
  const sel = state.selection.main;
  const lineInfo = state.doc.lineAt(sel.head);
  const selectedText = sel.empty ? '' : state.sliceDoc(sel.from, sel.to);
  const selectedLines = sel.empty
    ? 0
    : state.doc.lineAt(sel.to).number - state.doc.lineAt(sel.from).number + 1;

  emit('cursor-change', {
    line: lineInfo.number,
    col: sel.head - lineInfo.from + 1,
    lineCount: state.doc.lines,
    charCount: state.doc.length,
    cursorOffset: sel.head,
    selectedText,
    selectedChars: selectedText.length,
    selectedLines,
  });
}

function buildCustomKeymapExtension() {
  const b = userKeymap.bindings;
  const entries: Array<{ key: string; run: (v: EditorView) => boolean }> = [];
  const add = (combo: string, run: (v: EditorView) => boolean) => {
    const cmKey = comboToCodeMirrorKey(combo);
    if (cmKey) entries.push({ key: cmKey, run });
  };
  add(b.runCurrent, () => {
    emit('run-shortcut', false);
    return true;
  });
  add(b.runAll, () => {
    emit('run-shortcut', true);
    return true;
  });
  add(b.explainSql, () => {
    emit('explain-shortcut');
    return true;
  });
  add(b.saveFile, () => {
    emit('save-shortcut');
    return true;
  });
  add(b.formatSql, () => {
    emit('format-shortcut');
    return true;
  });
  add(b.toggleComment, toggleLineComment);
  add(b.duplicateLine, copyLineDown);
  add(b.deleteLine, deleteLine);
  add(b.findReplace, openSearchPanel);
  return keymap.of(entries);
}

function buildFontExtension() {
  const sz = Math.max(10, Math.min(26, props.fontSize ?? 13));
  return EditorView.theme({
    '.cm-content': { fontSize: `${sz}px` },
    '.cm-gutters': { fontSize: `${Math.max(10, sz - 1)}px` },
  });
}

function buildWrapExtension() {
  return props.wordWrap ? EditorView.lineWrapping : [];
}

onMounted(() => {
  if (!hostEl.value) return;

  const state = EditorState.create({
    doc: props.modelValue,
    extensions: [
      keymapCompartment.of(buildCustomKeymapExtension()),
      lineNumbers(),
      highlightActiveLineGutter(),
      highlightSpecialChars(),
      history(),
      foldGutter(),
      drawSelection(),
      EditorState.allowMultipleSelections.of(true),
      indentOnInput(),
      bracketMatching(),
      closeBrackets(),
      autocompletion(),
      highlightActiveLine(),
      highlightSelectionMatches(),
      keymap.of([
        ...closeBracketsKeymap,
        ...defaultKeymap,
        ...searchKeymap,
        ...historyKeymap,
        ...foldKeymap,
        ...completionKeymap,
        indentWithTab,
      ]),
      langCompartment.of(buildLanguageExtension()),
      themeCompartment.of(buildThemeExtension()),
      readOnlyCompartment.of(EditorState.readOnly.of(Boolean(props.readOnly))),
      lensCompartment.of(buildMarkdownLensExtension()),
      fontCompartment.of(buildFontExtension()),
      wrapCompartment.of(buildWrapExtension()),
      EditorView.updateListener.of((update) => {
        if (update.docChanged && !syncingFromProp) {
          emit('update:modelValue', update.state.doc.toString());
        }
        if (update.docChanged || update.selectionSet) {
          emitCursorTelemetry(update.state);
        }
      }),
    ],
  });

  view = new EditorView({
    state,
    parent: hostEl.value,
  });
  emitCursorTelemetry(state);
});

watch(
  () => props.modelValue,
  (next) => {
    if (!view) return;
    const current = view.state.doc.toString();
    if (next !== current) {
      syncingFromProp = true;
      view.dispatch({
        changes: { from: 0, to: current.length, insert: next },
      });
      syncingFromProp = false;
      emitCursorTelemetry(view.state);
    }
  },
);

watch(
  [() => props.fileKind, () => props.engineKey, () => props.schemaMap],
  () => {
    if (!view) return;
    view.dispatch({
      effects: [
        langCompartment.reconfigure(buildLanguageExtension()),
        lensCompartment.reconfigure(buildMarkdownLensExtension()),
      ],
    });
  },
  { deep: true },
);

watch(
  () => props.theme,
  () => {
    if (!view) return;
    view.dispatch({
      effects: themeCompartment.reconfigure(buildThemeExtension()),
    });
  },
);

watch(
  () => props.readOnly,
  (ro) => {
    if (!view) return;
    view.dispatch({
      effects: readOnlyCompartment.reconfigure(EditorState.readOnly.of(Boolean(ro))),
    });
  },
);

watch(
  [() => props.runLabel, () => props.explainLabel, () => props.copyConsoleLabel],
  () => {
    if (!view) return;
    view.dispatch({
      effects: lensCompartment.reconfigure(buildMarkdownLensExtension()),
    });
  },
);

watch(
  () => ({ ...userKeymap.bindings }),
  () => {
    if (!view) return;
    view.dispatch({
      effects: keymapCompartment.reconfigure(buildCustomKeymapExtension()),
    });
  },
  { deep: true },
);

watch(
  () => props.fontSize,
  () => {
    if (!view) return;
    view.dispatch({
      effects: fontCompartment.reconfigure(buildFontExtension()),
    });
  },
);

watch(
  () => props.wordWrap,
  () => {
    if (!view) return;
    view.dispatch({
      effects: wrapCompartment.reconfigure(buildWrapExtension()),
    });
  },
);

function jumpToLine(lineNum: number) {
  if (!view) return;
  const clamped = Math.max(1, Math.min(lineNum, view.state.doc.lines));
  const line = view.state.doc.line(clamped);
  view.dispatch({
    selection: { anchor: line.from },
    effects: EditorView.scrollIntoView(line.from, { y: 'center' }),
  });
  view.focus();
}

function insertText(snippet: string) {
  if (!view) return;
  const sel = view.state.selection.main;
  view.dispatch({
    changes: { from: sel.from, to: sel.to, insert: snippet },
    selection: { anchor: sel.from + snippet.length },
  });
  view.focus();
}

function focus() {
  view?.focus();
}

defineExpose({ jumpToLine, insertText, focus });

onBeforeUnmount(() => {
  view?.destroy();
  view = null;
});
</script>

<template>
  <div ref="hostEl" class="cm-host" />
</template>
