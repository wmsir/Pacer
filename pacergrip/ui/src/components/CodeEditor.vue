<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue';
import {
  Compartment,
  EditorSelection,
  EditorState,
  Prec,
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
  acceptCompletion,
  autocompletion,
  closeBrackets,
  closeBracketsKeymap,
  closeCompletion,
  completionKeymap,
  completionStatus,
  hasNextSnippetField,
  hasPrevSnippetField,
  nextSnippetField,
  pickedCompletion,
  prevSnippetField,
  snippet,
  startCompletion,
  type Completion,
  type CompletionContext,
  type CompletionResult,
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
import {
  extractReferencedTablesInStatement,
  formatCompletionIdentifier,
  formatTemplatePreview,
  normalizeLiveTemplate,
  useLiveTemplates,
  type SqlLiveTemplate,
} from '../utils/liveTemplates';

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
  (e: 'ensure-schema-loaded'): void;
  (e: 'table-picked', tableName: string): void;
}>();

const userKeymap = useKeymap();
const liveTemplates = useLiveTemplates();
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

const TABLE_PLACEHOLDER_RE = /^(table\d*|tbl\d*|table_name)$/i;
const COLUMN_PLACEHOLDER_RE = /^(column\d*|col\d*|col_name|columns|pk)$/i;

function isCursorInSqlContext(state: EditorState, pos: number): boolean {
  if (props.fileKind !== 'markdown') return true;
  const lineNum = state.doc.lineAt(pos).number;
  const blocks = extractMarkdownSqlBlocksLocal(state.doc.toString());
  return blocks.some((b) => lineNum >= b.contentStartLine && lineNum <= b.contentEndLine);
}

function applyIdentifierCompletion(
  v: EditorView,
  completion: Completion,
  from: number,
  to: number,
  rawName: string,
  isTable: boolean,
) {
  const trSpec =
    v.state.selection.ranges.length > 1
      ? v.state.changeByRange((range) => {
          const prevChar = range.from > 0 ? v.state.sliceDoc(range.from - 1, range.from) : '';
          const embedded = /[A-Za-z0-9_$]/.test(prevChar);
          const text = formatCompletionIdentifier(rawName, props.engineKey, embedded);
          return {
            changes: { from: range.from, to: range.to, insert: text },
            range: EditorSelection.cursor(range.from + text.length),
          };
        })
      : (() => {
          const prevChar = from > 0 ? v.state.sliceDoc(from - 1, from) : '';
          const embedded = /[A-Za-z0-9_$]/.test(prevChar);
          const text = formatCompletionIdentifier(rawName, props.engineKey, embedded);
          return {
            changes: { from, to, insert: text },
            selection: EditorSelection.cursor(from + text.length),
          };
        })();

  v.dispatch({
    ...trSpec,
    annotations: pickedCompletion.of(completion),
  });

  if (isTable) {
    emit('table-picked', rawName);
  }
}

function maybeTriggerPlaceholderCompletion(v: EditorView) {
  const sel = v.state.selection.main;
  if (sel.empty || !isCursorInSqlContext(v.state, sel.from)) return;
  const selectedText = v.state.sliceDoc(sel.from, sel.to).trim();
  if (TABLE_PLACEHOLDER_RE.test(selectedText)) {
    if (Object.keys(props.schemaMap ?? {}).length === 0) {
      emit('ensure-schema-loaded');
    }
    startCompletion(v);
  } else if (COLUMN_PLACEHOLDER_RE.test(selectedText)) {
    startCompletion(v);
  }
}

function expandLiveTemplate(v: EditorView, tpl: SqlLiveTemplate, from: number, to: number) {
  closeCompletion(v);
  const normalized = normalizeLiveTemplate(tpl.template);
  snippet(normalized)(v, null, from, to);
  v.focus();
  window.setTimeout(() => {
    if (view === v) {
      maybeTriggerPlaceholderCompletion(v);
    }
  }, 0);
}

function buildTableCompletions(schema: Record<string, string[]>, boostBase = 90): Completion[] {
  return Object.keys(schema).map((tblName, idx) => {
    const colCount = schema[tblName]?.length ?? 0;
    return {
      label: tblName,
      detail: colCount > 0 ? `数据表 · ${colCount} 列` : '当前库数据表',
      type: 'class',
      boost: Math.max(10, boostBase - Math.min(idx, 20) * 0.1),
      apply: (v, completion, from, to) =>
        applyIdentifierCompletion(v, completion, from, to, tblName, true),
    };
  });
}

function buildColumnCompletionsForTables(
  schema: Record<string, string[]>,
  preferredTables: string[],
  boostBase = 85,
): Completion[] {
  const seen = new Set<string>();
  const options: Completion[] = [];
  const schemaEntries = Object.entries(schema);
  const findColsFor = (tblQuery: string) => {
    const exact = schema[tblQuery];
    if (exact) return { table: tblQuery, cols: exact };
    const low = tblQuery.toLowerCase();
    const match = schemaEntries.find(([k]) => k.toLowerCase() === low);
    return match ? { table: match[0], cols: match[1] } : null;
  };

  for (const tbl of preferredTables) {
    const resolved = findColsFor(tbl);
    if (!resolved) continue;
    for (const col of resolved.cols) {
      const key = `${resolved.table}.${col}`;
      if (seen.has(key)) continue;
      seen.add(key);
      options.push({
        label: col,
        detail: `${resolved.table} 字段`,
        type: 'property',
        boost: boostBase,
        apply: (v, completion, from, to) =>
          applyIdentifierCompletion(v, completion, from, to, col, false),
      });
    }
  }

  if (options.length === 0) {
    for (const [tblName, cols] of schemaEntries) {
      for (const col of cols) {
        if (seen.has(`${tblName}.${col}`)) continue;
        seen.add(`${tblName}.${col}`);
        options.push({
          label: col,
          detail: `${tblName} 字段`,
          type: 'property',
          boost: boostBase - 15,
          apply: (v, completion, from, to) =>
            applyIdentifierCompletion(v, completion, from, to, col, false),
        });
      }
    }
  }
  return options;
}

function pacerSqlCompletionSource(context: CompletionContext): CompletionResult | null {
  if (!isCursorInSqlContext(context.state, context.pos)) return null;
  const schema = props.schemaMap ?? {};
  const tableNames = Object.keys(schema);
  const sel = context.state.selection.main;
  const docText = context.state.doc.toString();

  // 1. Active placeholder selected from Live Template (`table` / `column`)
  if (!sel.empty) {
    const placeholder = context.state.sliceDoc(sel.from, sel.to).trim();
    if (TABLE_PLACEHOLDER_RE.test(placeholder) && tableNames.length > 0) {
      return {
        from: sel.from,
        to: sel.to,
        options: buildTableCompletions(schema, 95),
        filter: false,
      };
    }
    if (COLUMN_PLACEHOLDER_RE.test(placeholder)) {
      const { tables } = extractReferencedTablesInStatement(docText, sel.from);
      const colOpts = buildColumnCompletionsForTables(schema, tables, 95);
      if (colOpts.length > 0) {
        return {
          from: sel.from,
          to: sel.to,
          options: colOpts,
          filter: false,
        };
      }
    }
    return null;
  }

  // 2. Qualified `alias.` or `table.` column completion
  const qualifierMatch = context.matchBefore(/(?:[`"]?([A-Za-z0-9_$]+)[`"]?)\.\s*([A-Za-z0-9_$]*)$/);
  if (qualifierMatch) {
    const inner = /(?:[`"]?([A-Za-z0-9_$]+)[`"]?)\.\s*([A-Za-z0-9_$]*)$/.exec(
      qualifierMatch.text,
    );
    if (inner) {
      const targetAliasOrTable = inner[1];
      const typedCol = inner[2] ?? '';
      const { aliases } = extractReferencedTablesInStatement(docText, context.pos);
      const resolvedTable = aliases[targetAliasOrTable.toLowerCase()] ?? targetAliasOrTable;
      const colOpts = buildColumnCompletionsForTables(schema, [resolvedTable], 90);
      if (colOpts.length > 0) {
        return {
          from: context.pos - typedCol.length,
          options: colOpts,
          validFor: /^[A-Za-z0-9_$]*$/,
        };
      }
    }
  }

  // 3. Table completion directly after FROM / JOIN / INTO / UPDATE / ALTER TABLE / TRUNCATE
  const tableCtx = context.matchBefore(
    /\b(?:from|join|into|update|table|truncate)\s+([`"']?[A-Za-z0-9_$]*)$/i,
  );
  if (tableCtx && tableNames.length > 0) {
    const m = /\b(?:from|join|into|update|table|truncate)\s+([`"']?[A-Za-z0-9_$]*)$/i.exec(
      tableCtx.text,
    );
    const typedRaw = m?.[1] ?? '';
    const typedClean = typedRaw.replace(/^[`"']/, '');
    if (typedClean.length > 0 || context.explicit) {
      return {
        from: context.pos - typedRaw.length,
        options: buildTableCompletions(schema, 85),
        validFor: /^[`"']?[A-Za-z0-9_$]*$/,
      };
    }
  }

  // 4. SQL Live Templates + Columns of tables referenced in current statement
  const word = context.matchBefore(/[A-Za-z_][A-Za-z0-9_]*/);
  if (!word && !context.explicit) return null;

  const fromPos = word ? word.from : context.pos;
  const prevChar = fromPos > 0 ? context.state.sliceDoc(fromPos - 1, fromPos) : '';
  if (prevChar === '.' || prevChar === "'" || prevChar === '"' || prevChar === '`') {
    return null;
  }

  const typed = word ? word.text.toLowerCase() : '';
  const options: Completion[] = [];

  for (const tpl of liveTemplates.templates) {
    if (!tpl.enabled) continue;
    const abbrLow = tpl.abbrev.toLowerCase();
    if (!typed || abbrLow.startsWith(typed)) {
      options.push({
        label: tpl.abbrev,
        detail: `${tpl.description} [Tab]`,
        info: formatTemplatePreview(tpl.template),
        type: 'template',
        boost: abbrLow === typed ? 99 : 70,
        apply: (v, _completion, from, to) => expandLiveTemplate(v, tpl, from, to),
      });
    }
  }

  const { tables } = extractReferencedTablesInStatement(docText, context.pos);
  if (tables.length > 0) {
    options.push(...buildColumnCompletionsForTables(schema, tables, 55));
  }

  if (options.length === 0) return null;
  return {
    from: fromPos,
    options,
    validFor: /^[A-Za-z0-9_]*$/,
  };
}

function tryHandleEditorTab(v: EditorView): boolean {
  const state = v.state;
  const sel = state.selection.main;

  // 1. Live Template abbreviation right before cursor (`sel` + Tab -> expands template + opens table picker)
  if (sel.empty && isCursorInSqlContext(state, sel.head)) {
    const lineInfo = state.doc.lineAt(sel.head);
    const lineBefore = state.sliceDoc(lineInfo.from, sel.head);
    const wordMatch = /([A-Za-z_][A-Za-z0-9_]*)$/.exec(lineBefore);
    if (wordMatch) {
      const word = wordMatch[1];
      const wordFrom = sel.head - word.length;
      const beforeWord = state.sliceDoc(Math.max(0, wordFrom - 36), wordFrom);
      const isQualifiedOrQuoted =
        beforeWord.endsWith('.') ||
        beforeWord.endsWith('`') ||
        beforeWord.endsWith('"') ||
        beforeWord.endsWith("'");
      const afterTableKw = /\b(?:from|join|into|update|table|truncate)\s+$/i.test(beforeWord);
      if (!isQualifiedOrQuoted && !afterTableKw) {
        const matchedTpl = liveTemplates.findByAbbrev(word);
        if (matchedTpl) {
          expandLiveTemplate(v, matchedTpl, wordFrom, sel.head);
          return true;
        }
      }
    }
  }

  // 2. Active autocomplete item -> accept
  if (completionStatus(state) === 'active') {
    if (acceptCompletion(v)) {
      return true;
    }
  }

  // 3. Active snippet placeholder -> advance & auto-pop next table/column completions
  if (hasNextSnippetField(state)) {
    if (nextSnippetField(v)) {
      window.setTimeout(() => {
        if (view === v) {
          maybeTriggerPlaceholderCompletion(v);
        }
      }, 0);
      return true;
    }
  }

  return false;
}

function tryHandleEditorShiftTab(v: EditorView): boolean {
  if (hasPrevSnippetField(v.state) && prevSnippetField(v)) {
    window.setTimeout(() => {
      if (view === v) {
        maybeTriggerPlaceholderCompletion(v);
      }
    }, 0);
    return true;
  }
  return false;
}

onMounted(() => {
  if (!hostEl.value) return;

  const state = EditorState.create({
    doc: props.modelValue,
    extensions: [
      keymapCompartment.of(buildCustomKeymapExtension()),
      Prec.high(
        keymap.of([
          { key: 'Tab', run: tryHandleEditorTab },
          { key: 'Shift-Tab', run: tryHandleEditorShiftTab },
        ]),
      ),
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
      autocompletion({
        activateOnTyping: true,
        maxRenderedOptions: 60,
      }),
      EditorState.languageData.of(() => [{ autocomplete: pacerSqlCompletionSource }]),
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
    if (view.hasFocus && !view.state.selection.main.empty) {
      const selected = view.state.sliceDoc(
        view.state.selection.main.from,
        view.state.selection.main.to,
      ).trim();
      if (TABLE_PLACEHOLDER_RE.test(selected) || COLUMN_PLACEHOLDER_RE.test(selected)) {
        window.setTimeout(() => {
          if (view) startCompletion(view);
        }, 0);
      }
    }
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

function replaceSelection(text: string) {
  if (!view) return;
  const sel = view.state.selection.main;
  view.dispatch({
    changes: { from: sel.from, to: sel.to, insert: text },
    selection: { anchor: sel.from, head: sel.from + text.length },
  });
  view.focus();
}

function expandTemplate(tpl: SqlLiveTemplate) {
  if (!view) return;
  const sel = view.state.selection.main;
  expandLiveTemplate(view, tpl, sel.from, sel.to);
}

defineExpose({ jumpToLine, insertText, focus, replaceSelection, expandTemplate });

onBeforeUnmount(() => {
  view?.destroy();
  view = null;
});
</script>

<template>
  <div ref="hostEl" class="cm-host" />
</template>
