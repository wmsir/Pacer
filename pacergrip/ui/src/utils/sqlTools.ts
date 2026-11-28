import type {
  ColumnMeta,
  ConnectionProfile,
  DirectConfig,
  IndexMeta,
  MarkdownSqlBlock,
} from '../api/ipc';

const SQL_FENCE_LANGS = new Set([
  'sql',
  'mysql',
  'mariadb',
  'postgresql',
  'postgres',
  'pgsql',
  'plpgsql',
  'sqlite',
  'sqlite3',
  'oracle',
  'plsql',
  'tsql',
  'mssql',
  'sqlserver',
  'dm',
  'dameng',
  'kingbase',
  'opengauss',
  'tidb',
  'oceanbase',
  'clickhouse',
  'ddl',
  'dml',
]);

/** Fast client-side parser mirroring Rust `extract_markdown_sql_blocks` for live CodeLens updates. */
export function extractMarkdownSqlBlocksLocal(markdown: string): MarkdownSqlBlock[] {
  const lines = markdown.split(/\r?\n/);
  const blocks: MarkdownSqlBlock[] = [];
  let lastHeading: string | null = null;

  for (let i = 0; i < lines.length; ) {
    const trimmed = lines[i].trimStart();
    if (trimmed.startsWith('#')) {
      const heading = trimmed.replace(/^#+\s*/, '').trim();
      if (heading) lastHeading = heading;
      i += 1;
      continue;
    }

    const fenceMatch = trimmed.match(/^(`{3,}|~{3,})(.*)$/);
    if (fenceMatch) {
      const fence = fenceMatch[1];
      const fenceChar = fence[0];
      const info = fenceMatch[2].trim();
      const lang = info.split(/[\s,{]/)[0]?.toLowerCase() ?? '';
      const startLine = i + 1;
      const contentStartLine = i + 2;
      const body: string[] = [];
      let j = i + 1;

      while (j < lines.length) {
        const closeLine = lines[j].trim();
        if (
          closeLine.startsWith(fenceChar.repeat(fence.length)) &&
          closeLine.replace(new RegExp(`^\\${fenceChar}+`), '').trim() === ''
        ) {
          break;
        }
        body.push(lines[j]);
        j += 1;
      }

      const endLine = Math.min(j + 1, lines.length);
      const contentEndLine = body.length === 0 ? contentStartLine : contentStartLine + body.length - 1;

      if (SQL_FENCE_LANGS.has(lang)) {
        const sql = body.join('\n').trim();
        if (sql) {
          blocks.push({
            index: blocks.length,
            language: lang,
            startLine,
            endLine,
            contentStartLine,
            contentEndLine,
            sql,
            title: lastHeading,
          });
        }
      }
      i = j + 1;
      continue;
    }
    i += 1;
  }

  return blocks;
}

export type MarkdownSegment =
  | { kind: 'heading'; level: number; text: string; line: number }
  | { kind: 'blockquote'; text: string }
  | { kind: 'hr' }
  | { kind: 'list'; items: string[] }
  | { kind: 'table'; headers: string[]; rows: string[][] }
  | { kind: 'paragraph'; text: string }
  | { kind: 'sql-code'; block: MarkdownSqlBlock }
  | { kind: 'code'; language: string; text: string };

function isMarkdownTableSeparator(line: string): boolean {
  const s = line.trim();
  return s.includes('|') && /^\|?(?:\s*:?-{2,}:?\s*\|)+\s*(?::?-{2,}:?\s*\|?)?$/.test(s);
}

function parseMarkdownTableRow(line: string): string[] {
  let s = line.trim();
  if (s.startsWith('|')) s = s.slice(1);
  if (s.endsWith('|') && !s.endsWith('\\|')) s = s.slice(0, -1);
  const cells: string[] = [];
  let cur = '';
  for (let k = 0; k < s.length; k += 1) {
    if (s[k] === '\\' && s[k + 1] === '|') {
      cur += '|';
      k += 1;
      continue;
    }
    if (s[k] === '|') {
      cells.push(cur.trim());
      cur = '';
      continue;
    }
    cur += s[k];
  }
  cells.push(cur.trim());
  return cells;
}

/** Safe XSS-free Markdown block tokenizer used by the split Preview pane (no v-html). */
export function parseMarkdownSegments(markdown: string): MarkdownSegment[] {
  const lines = markdown.split(/\r?\n/);
  const segments: MarkdownSegment[] = [];
  let sqlIndex = 0;
  let lastHeading: string | null = null;
  let i = 0;

  while (i < lines.length) {
    const raw = lines[i];
    const trimmed = raw.trim();

    if (!trimmed) {
      i += 1;
      continue;
    }

    const headingMatch = trimmed.match(/^(#{1,6})\s+(.*)$/);
    if (headingMatch) {
      lastHeading = headingMatch[2].trim();
      segments.push({
        kind: 'heading',
        level: headingMatch[1].length,
        text: lastHeading,
        line: i + 1,
      });
      i += 1;
      continue;
    }

    if (/^(-{3,}|\*{3,}|_{3,})$/.test(trimmed)) {
      segments.push({ kind: 'hr' });
      i += 1;
      continue;
    }

    const fenceMatch = raw.trimStart().match(/^(`{3,}|~{3,})(.*)$/);
    if (fenceMatch) {
      const fence = fenceMatch[1];
      const fenceChar = fence[0];
      const lang = fenceMatch[2].trim().split(/[\s,{]/)[0]?.toLowerCase() ?? '';
      const startLine = i + 1;
      const contentStartLine = i + 2;
      const body: string[] = [];
      let j = i + 1;
      while (j < lines.length) {
        const closeLine = lines[j].trim();
        if (
          closeLine.startsWith(fenceChar.repeat(fence.length)) &&
          closeLine.replace(new RegExp(`^\\${fenceChar}+`), '').trim() === ''
        ) {
          break;
        }
        body.push(lines[j]);
        j += 1;
      }
      const endLine = Math.min(j + 1, lines.length);
      const contentEndLine = body.length === 0 ? contentStartLine : contentStartLine + body.length - 1;
      const text = body.join('\n').trim();

      if (SQL_FENCE_LANGS.has(lang) && text) {
        segments.push({
          kind: 'sql-code',
          block: {
            index: sqlIndex++,
            language: lang,
            startLine,
            endLine,
            contentStartLine,
            contentEndLine,
            sql: text,
            title: lastHeading,
          },
        });
      } else {
        segments.push({ kind: 'code', language: lang || 'text', text });
      }
      i = j + 1;
      continue;
    }

    if (trimmed.startsWith('>')) {
      const quoteLines: string[] = [];
      while (i < lines.length && lines[i].trim().startsWith('>')) {
        quoteLines.push(lines[i].trim().replace(/^>\s?/, ''));
        i += 1;
      }
      segments.push({ kind: 'blockquote', text: quoteLines.join('\n') });
      continue;
    }

    if (trimmed.includes('|') && i + 1 < lines.length && isMarkdownTableSeparator(lines[i + 1])) {
      const headers = parseMarkdownTableRow(raw);
      const colCount = Math.max(1, headers.length);
      const tableRows: string[][] = [];
      i += 2;
      while (i < lines.length) {
        const rowRaw = lines[i];
        const rowTrim = rowRaw.trim();
        if (
          !rowTrim ||
          !rowTrim.includes('|') ||
          rowTrim.startsWith('```') ||
          rowTrim.startsWith('~~~')
        ) {
          break;
        }
        const cells = parseMarkdownTableRow(rowRaw);
        while (cells.length < colCount) cells.push('');
        tableRows.push(cells.slice(0, colCount));
        i += 1;
      }
      segments.push({ kind: 'table', headers, rows: tableRows });
      continue;
    }

    if (/^([-*+]|\d+\.)\s+/.test(trimmed)) {
      const items: string[] = [];
      while (i < lines.length && /^([-*+]|\d+\.)\s+/.test(lines[i].trim())) {
        items.push(lines[i].trim().replace(/^([-*+]|\d+\.)\s+/, ''));
        i += 1;
      }
      segments.push({ kind: 'list', items });
      continue;
    }

    const paraLines: string[] = [raw];
    i += 1;
    while (
      i < lines.length &&
      lines[i].trim() !== '' &&
      !lines[i].trim().startsWith('#') &&
      !lines[i].trim().startsWith('>') &&
      !lines[i].trimStart().startsWith('```') &&
      !lines[i].trimStart().startsWith('~~~') &&
      !/^(-{3,}|\*{3,}|_{3,})$/.test(lines[i].trim()) &&
      !(lines[i].includes('|') && i + 1 < lines.length && isMarkdownTableSeparator(lines[i + 1]))
    ) {
      paraLines.push(lines[i]);
      i += 1;
    }
    segments.push({ kind: 'paragraph', text: paraLines.join('\n') });
  }

  return segments;
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
}

export function estimateUtf8Bytes(text: string): number {
  return new TextEncoder().encode(text).length;
}

export function countLines(text: string): number {
  if (!text) return 1;
  return text.split('\n').length;
}

export function toCsv(columns: string[], rows: Record<string, unknown>[]): string {
  const esc = (v: unknown) => {
    if (v === null || v === undefined) return '';
    const s = typeof v === 'object' ? JSON.stringify(v) : String(v);
    return /[",\n\r]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
  };
  return [
    columns.map(esc).join(','),
    ...rows.map((r) => columns.map((c) => esc(r[c])).join(',')),
  ].join('\n');
}

export function toMarkdownTable(columns: string[], rows: Record<string, unknown>[]): string {
  if (columns.length === 0) return '';
  const esc = (v: unknown) => {
    if (v === null || v === undefined) return 'NULL';
    const s = typeof v === 'object' ? JSON.stringify(v) : String(v);
    return s.replace(/\|/g, '\\|').replace(/\r?\n/g, ' ');
  };
  const header = `| ${columns.map(esc).join(' | ')} |`;
  const sep = `| ${columns.map(() => '---').join(' | ')} |`;
  const body = rows.map((r) => `| ${columns.map((c) => esc(r[c])).join(' | ')} |`);
  return [header, sep, ...body].join('\n');
}

export function toInsertSql(
  tableName: string,
  columns: string[],
  rows: Record<string, unknown>[],
  engineKey = 'mysql',
): string {
  const safeTable = quoteIdentifier(tableName || 'target_table', engineKey);
  const colList = columns.map((c) => quoteIdentifier(c, engineKey)).join(', ');
  const sqlVal = (v: unknown): string => {
    if (v === null || v === undefined) return 'NULL';
    if (typeof v === 'number') return Number.isFinite(v) ? String(v) : 'NULL';
    if (typeof v === 'boolean') return v ? 'TRUE' : 'FALSE';
    const str = typeof v === 'object' ? JSON.stringify(v) : String(v);
    return `'${str.replace(/'/g, "''")}'`;
  };
  return rows
    .map(
      (r) =>
        `INSERT INTO ${safeTable} (${colList}) VALUES (${columns.map((c) => sqlVal(r[c])).join(', ')});`,
    )
    .join('\n');
}

export function downloadTextFile(filename: string, content: string, mime: string) {
  const blob = new Blob([content], { type: mime });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}

/**
 * Extracts a single target `[db.]table` from a simple `SELECT ... FROM ...` query.
 * Returns `null` for multi-table JOINs or UNIONs so callers can fall back to manual table selection.
 */
export function extractSingleTableTarget(sql: string): { db?: string; table: string } | null {
  const cleaned = sql
    .replace(/--[^\n]*/g, ' ')
    .replace(/\/\*[\s\S]*?\*\//g, ' ')
    .trim();
  if (!/^select\b/i.test(cleaned)) return null;
  if (/\b(join|union)\b/i.test(cleaned)) return null;

  const fromMatch = cleaned.match(
    /\bfrom\s+(?:([`"\[]?[A-Za-z0-9_$-]+[`"\]]?)\s*\.\s*)?([`"\[]?[A-Za-z0-9_$-]+[`"\]]?)(?:\s|$|,|;)/i,
  );
  if (!fromMatch) return null;

  // Reject comma-joins (`FROM a, b`)
  const afterFrom = cleaned.slice((fromMatch.index ?? 0) + fromMatch[0].length).trim();
  if (fromMatch[0].trimEnd().endsWith(',')) return null;
  if (/^(?:as\s+)?[A-Za-z0-9_$-]+\s*,/i.test(afterFrom)) return null;

  const wasQuoted = /^[`"\[]/.test(fromMatch[2]);
  const stripQuotes = (s: string) => s.replace(/^[`"\[]|[`"\]]$/g, '');
  const db = fromMatch[1] ? stripQuotes(fromMatch[1]) : undefined;
  const table = stripQuotes(fromMatch[2]);
  if (!table || (!wasQuoted && /^(select|where|group|order|limit|having|values)$/i.test(table))) {
    return null;
  }
  return { db, table };
}

export function quoteIdentifier(ident: string, engineKey: string): string {
  const eng = engineKey.toLowerCase();
  if (eng.includes('sqlserver') || eng.includes('mssql')) {
    return `[${ident.replace(/\]/g, ']]')}]`;
  }
  if (eng.includes('postgres') || eng.includes('kingbase') || eng.includes('gauss') || eng.includes('oracle') || eng.includes('dameng')) {
    return `"${ident.replace(/"/g, '""')}"`;
  }
  return `\`${ident.replace(/`/g, '``')}\``;
}

export function formatSqlLiteral(val: unknown): string {
  if (val === null || val === undefined) return 'NULL';
  if (typeof val === 'number') return Number.isFinite(val) ? String(val) : 'NULL';
  if (typeof val === 'boolean') return val ? 'TRUE' : 'FALSE';
  const str = typeof val === 'object' ? JSON.stringify(val) : String(val);
  return `'${str.replace(/'/g, "''")}'`;
}

export interface GridMutationOptions {
  tableName: string;
  dbName?: string;
  engineKey: string;
  columns: string[];
  primaryKeys: string[];
  originalRows: Record<string, unknown>[];
  modifiedCells: Record<number, Record<string, unknown>>;
  deletedRowIndices: number[];
  insertedRows: Array<Record<string, unknown>>;
}

export interface GridMutationPlan {
  statements: string[];
  usedPrimaryKey: boolean;
  keyColumns: string[];
  warning: string | null;
}

/**
 * Generates dialect-accurate `UPDATE` / `INSERT` / `DELETE` statements from inline grid edits.
 * Automatically anchors `WHERE` clauses on Primary Key columns when available, falling back to
 * all original row columns (with `IS NULL` handling) and emitting a clear warning.
 */
export function generateGridMutationSql(opts: GridMutationOptions): GridMutationPlan {
  const q = (id: string) => quoteIdentifier(id, opts.engineKey);
  const qualifiedTable =
    opts.dbName && opts.dbName !== 'main' && opts.dbName !== 'default'
      ? `${q(opts.dbName)}.${q(opts.tableName)}`
      : q(opts.tableName);

  const usablePks = opts.primaryKeys
    .map((pk) => opts.columns.find((col) => col.toLowerCase() === pk.toLowerCase()))
    .filter((col): col is string => Boolean(col));
  const usedPrimaryKey = usablePks.length > 0;
  const keyColumns = usedPrimaryKey ? usablePks : opts.columns;
  const warning = usedPrimaryKey
    ? null
    : '当前结果集未包含表主键，WHERE 条件将使用该行全部原始列值进行精确匹配，请仔细核对后再执行。';

  const buildWhere = (origRow: Record<string, unknown>) =>
    keyColumns
      .map((col) => {
        const v = origRow[col];
        return v === null || v === undefined ? `${q(col)} IS NULL` : `${q(col)} = ${formatSqlLiteral(v)}`;
      })
      .join(' AND ');

  const statements: string[] = [];
  const deletedSet = new Set(opts.deletedRowIndices);

  // 1. UPDATE statements for modified non-deleted rows
  for (const [idxStr, changes] of Object.entries(opts.modifiedCells)) {
    const rowIdx = Number(idxStr);
    if (deletedSet.has(rowIdx)) continue;
    const origRow = opts.originalRows[rowIdx];
    if (!origRow) continue;
    const changedEntries = Object.entries(changes).filter(([col, newVal]) => {
      const oldVal = origRow[col];
      return newVal !== oldVal;
    });
    if (changedEntries.length === 0) continue;

    const setClause = changedEntries
      .map(([col, val]) => `${q(col)} = ${formatSqlLiteral(val)}`)
      .join(', ');
    const whereClause = buildWhere(origRow);
    statements.push(`UPDATE ${qualifiedTable} SET ${setClause} WHERE ${whereClause};`);
  }

  // 2. INSERT statements for newly added rows
  for (const newRow of opts.insertedRows) {
    const presentCols = opts.columns.filter((col) => newRow[col] !== undefined && newRow[col] !== '');
    if (presentCols.length === 0) continue;
    const colList = presentCols.map(q).join(', ');
    const valList = presentCols.map((c) => formatSqlLiteral(newRow[c])).join(', ');
    statements.push(`INSERT INTO ${qualifiedTable} (${colList}) VALUES (${valList});`);
  }

  // 3. DELETE statements for marked rows
  for (const rowIdx of [...deletedSet].sort((a, b) => a - b)) {
    const origRow = opts.originalRows[rowIdx];
    if (!origRow) continue;
    statements.push(`DELETE FROM ${qualifiedTable} WHERE ${buildWhere(origRow)};`);
  }

  return { statements, usedPrimaryKey, keyColumns, warning };
}

export interface SingleCellUpdatePlan {
  sql: string;
  usedPrimaryKey: boolean;
  keyColumns: string[];
  warning: string | null;
}

/** Generates a single-row UPDATE statement for immediate inline cell confirmation. */
export function generateSingleCellUpdatePlan(opts: {
  tableName: string;
  dbName?: string;
  engineKey: string;
  columns: string[];
  primaryKeys: string[];
  originalRow: Record<string, unknown>;
  targetColumn: string;
  newValue: unknown;
}): SingleCellUpdatePlan | null {
  const plan = generateGridMutationSql({
    tableName: opts.tableName,
    dbName: opts.dbName,
    engineKey: opts.engineKey,
    columns: opts.columns,
    primaryKeys: opts.primaryKeys,
    originalRows: [opts.originalRow],
    modifiedCells: { 0: { [opts.targetColumn]: opts.newValue } },
    deletedRowIndices: [],
    insertedRows: [],
  });
  if (plan.statements.length === 0) return null;
  return {
    sql: plan.statements[0],
    usedPrimaryKey: plan.usedPrimaryKey,
    keyColumns: plan.keyColumns,
    warning: plan.warning,
  };
}

export interface SqlSnippet {
  id: string;
  title: string;
  category: string;
  sql: string;
}

export const BUILTIN_SNIPPETS: SqlSnippet[] = [
  {
    id: 'snip-join-agg',
    title: '多表聚合统计模板',
    category: 'DQL',
    sql: `SELECT\n  a.category,\n  COUNT(*) AS total_cnt,\n  SUM(a.amount) AS total_amount\nFROM table_a a\nLEFT JOIN table_b b ON a.id = b.a_id\nWHERE a.created_at >= '2026-01-01'\nGROUP BY a.category\nORDER BY total_amount DESC\nLIMIT 50;`,
  },
  {
    id: 'snip-cte-page',
    title: 'CTE 窗口函数去重分页',
    category: 'DQL',
    sql: `WITH ranked AS (\n  SELECT\n    t.*,\n    ROW_NUMBER() OVER (PARTITION BY t.customer_id ORDER BY t.id DESC) AS rn\n  FROM orders t\n)\nSELECT *\nFROM ranked\nWHERE rn = 1\nLIMIT 100;`,
  },
  {
    id: 'snip-create-table',
    title: '标准业务表 DDL 模板',
    category: 'DDL',
    sql: `CREATE TABLE IF NOT EXISTS biz_record (\n  id INTEGER PRIMARY KEY AUTOINCREMENT,\n  biz_no VARCHAR(64) NOT NULL UNIQUE,\n  status VARCHAR(32) NOT NULL DEFAULT 'ACTIVE',\n  payload TEXT,\n  created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP\n);`,
  },
  {
    id: 'snip-safe-update',
    title: '带影响行数注释的安全更新',
    category: 'DML',
    sql: `-- 影响行数: 1\nUPDATE orders\nSET status = 'PAID'\nWHERE order_no = 'ORD-2026-1003';`,
  },
  {
    id: 'snip-sqlite-tables',
    title: 'SQLite 元数据与索引排查',
    category: '运维',
    sql: `SELECT type, name, tbl_name, sql\nFROM sqlite_master\nWHERE name NOT LIKE 'sqlite_%'\nORDER BY type, name;`,
  },
  {
    id: 'snip-mysql-process',
    title: 'MySQL 活跃会话与锁排查',
    category: '运维',
    sql: `SELECT id, user, host, db, command, time, state, info\nFROM information_schema.PROCESSLIST\nWHERE command <> 'Sleep'\nORDER BY time DESC;`,
  },
];

// ==================== Parameterized SQL, Selection -> IN, Case & Unix Timestamp Helpers ====================

export interface SqlParamToken {
  name: string;
  rawSamples: string[];
}

export type SqlParamValueMode = 'auto' | 'string' | 'raw';

export interface SqlParamValueSpec {
  value: string;
  mode: SqlParamValueMode;
}

interface SqlParamMatch {
  start: number;
  end: number;
  name: string;
  raw: string;
}

function scanSqlParamMatches(sql: string): SqlParamMatch[] {
  const matches: SqlParamMatch[] = [];
  let i = 0;
  const n = sql.length;

  while (i < n) {
    const ch = sql[i];
    const next = sql[i + 1] ?? '';

    if (ch === '-' && next === '-') {
      i += 2;
      while (i < n && sql[i] !== '\n') i += 1;
      continue;
    }
    if (ch === '#' && next !== '{') {
      i += 1;
      while (i < n && sql[i] !== '\n') i += 1;
      continue;
    }
    if (ch === '/' && next === '*') {
      i += 2;
      while (i < n && !(sql[i] === '*' && sql[i + 1] === '/')) i += 1;
      i += 2;
      continue;
    }
    if (ch === "'" || ch === '"' || ch === '`') {
      const q = ch;
      i += 1;
      while (i < n) {
        if (q !== '`' && sql[i] === '\\') {
          i += 2;
          continue;
        }
        if (sql[i] === q) {
          if (sql[i + 1] === q) {
            i += 2;
            continue;
          }
          i += 1;
          break;
        }
        i += 1;
      }
      continue;
    }

    if ((ch === '$' || ch === '#') && next === '{') {
      const m = /^([A-Za-z_][A-Za-z0-9_]*)\}/.exec(sql.slice(i + 2));
      if (m) {
        const raw = sql.slice(i, i + 2 + m[0].length);
        matches.push({ start: i, end: i + raw.length, name: m[1], raw });
        i += raw.length;
        continue;
      }
    }

    if (ch === ':') {
      const prev = i > 0 ? sql[i - 1] : '';
      if (next !== '=' && next !== ':' && prev !== ':' && !/[A-Za-z0-9_]/.test(prev)) {
        const m = /^([A-Za-z_][A-Za-z0-9_]*)/.exec(sql.slice(i + 1));
        if (m) {
          const raw = `:${m[1]}`;
          matches.push({ start: i, end: i + raw.length, name: m[1], raw });
          i += raw.length;
          continue;
        }
      }
    }

    i += 1;
  }

  return matches;
}

export function extractSqlParameters(sql: string): SqlParamToken[] {
  const byName = new Map<string, string[]>();
  for (const m of scanSqlParamMatches(sql)) {
    const samples = byName.get(m.name);
    if (samples) {
      if (!samples.includes(m.raw)) samples.push(m.raw);
    } else {
      byName.set(m.name, [m.raw]);
    }
  }
  return Array.from(byName.entries()).map(([name, rawSamples]) => ({ name, rawSamples }));
}

function formatSqlParamValue(spec: SqlParamValueSpec | undefined): string {
  const raw = spec?.value ?? '';
  const mode = spec?.mode ?? 'auto';
  if (mode === 'raw') {
    return raw;
  }
  if (mode === 'string') {
    return `'${raw.replace(/'/g, "''")}'`;
  }
  const trimmed = raw.trim();
  if (trimmed === '' || /^null$/i.test(trimmed)) return 'NULL';
  if (/^(?:true|false)$/i.test(trimmed)) return trimmed.toUpperCase();
  if (/^-?(?:0|[1-9]\d*)(?:\.\d+)?$/.test(trimmed)) return trimmed;
  return `'${raw.replace(/'/g, "''")}'`;
}

export function substituteSqlParameters(
  sql: string,
  values: Record<string, SqlParamValueSpec>,
): string {
  const matches = scanSqlParamMatches(sql);
  if (matches.length === 0) return sql;
  let out = '';
  let cursor = 0;
  for (const m of matches) {
    out += sql.slice(cursor, m.start);
    out += formatSqlParamValue(values[m.name]);
    cursor = m.end;
  }
  out += sql.slice(cursor);
  return out;
}

export function convertSelectionToSqlIn(rawText: string, forceStrings = false): string {
  const parts = rawText.split(/[\r\n,\t]+/);
  const seen = new Set<string>();
  const formatted: string[] = [];

  for (const part of parts) {
    let cleaned = part.trim();
    if (!cleaned) continue;
    if (
      (cleaned.startsWith("'") && cleaned.endsWith("'") && cleaned.length >= 2) ||
      (cleaned.startsWith('"') && cleaned.endsWith('"') && cleaned.length >= 2)
    ) {
      cleaned = cleaned.slice(1, -1).trim();
    }
    if (!cleaned) continue;
    const isPlainNumber = /^-?(?:0|[1-9]\d*)(?:\.\d+)?$/.test(cleaned);
    const literal = !forceStrings && isPlainNumber ? cleaned : `'${cleaned.replace(/'/g, "''")}'`;
    if (!seen.has(literal)) {
      seen.add(literal);
      formatted.push(literal);
    }
  }

  return formatted.length > 0 ? `(${formatted.join(', ')})` : '()';
}

export function transformSqlTextCase(sql: string, mode: 'upper' | 'lower'): string {
  let out = '';
  let i = 0;
  const n = sql.length;

  while (i < n) {
    const ch = sql[i];
    const next = sql[i + 1] ?? '';

    if (ch === '-' && next === '-') {
      const start = i;
      i += 2;
      while (i < n && sql[i] !== '\n') i += 1;
      out += sql.slice(start, i);
      continue;
    }
    if (ch === '/' && next === '*') {
      const start = i;
      i += 2;
      while (i < n && !(sql[i] === '*' && sql[i + 1] === '/')) i += 1;
      i = Math.min(n, i + 2);
      out += sql.slice(start, i);
      continue;
    }
    if (ch === "'" || ch === '"' || ch === '`') {
      const q = ch;
      const start = i;
      i += 1;
      while (i < n) {
        if (q !== '`' && sql[i] === '\\') {
          i += 2;
          continue;
        }
        if (sql[i] === q) {
          if (sql[i + 1] === q) {
            i += 2;
            continue;
          }
          i += 1;
          break;
        }
        i += 1;
      }
      out += sql.slice(start, i);
      continue;
    }

    out += mode === 'upper' ? ch.toUpperCase() : ch.toLowerCase();
    i += 1;
  }

  return out;
}

export function decodeUnixTimestampHint(val: unknown): string | null {
  if (val === null || val === undefined || typeof val === 'boolean' || typeof val === 'object') {
    return null;
  }
  const str = String(val).trim();
  if (!/^-?\d+(?:\.\d+)?$/.test(str)) return null;
  const rawNum = Number(str);
  if (!Number.isFinite(rawNum)) return null;
  const intVal = Math.trunc(rawNum);

  let ms: number | null = null;
  let unit = '';
  if (intVal >= 946_684_800 && intVal <= 4_102_444_800) {
    ms = intVal * 1000;
    unit = '秒';
  } else if (intVal >= 946_684_800_000 && intVal <= 4_102_444_800_000) {
    ms = intVal;
    unit = '毫秒';
  } else {
    return null;
  }

  const dt = new Date(ms);
  if (Number.isNaN(dt.getTime())) return null;
  const pad = (v: number) => String(v).padStart(2, '0');
  const formatted = `${dt.getFullYear()}-${pad(dt.getMonth() + 1)}-${pad(dt.getDate())} ${pad(
    dt.getHours(),
  )}:${pad(dt.getMinutes())}:${pad(dt.getSeconds())}`;
  return `${formatted} (Unix ${unit})`;
}

// ==================== Dialect-Aware Execution Plan Generator ====================

export type ExplainPlanMode = 'standard' | 'analyze' | 'verbose';

export interface ExplainPlanModeOption {
  mode: ExplainPlanMode;
  label: string;
  shortSyntax: string;
  executesQuery: boolean;
  description: string;
}

export interface ExplainPlanResult {
  engineKey: string;
  dialectLabel: string;
  mode: ExplainPlanMode;
  modeLabel: string;
  headerComment: string;
  sql: string;
  fullScript: string;
  executesQuery: boolean;
  canRunDirectly: boolean;
}

export function getEngineDisplayName(engineKey: string): string {
  const e = (engineKey || 'mysql').toLowerCase();
  if (e.includes('tidb')) return 'TiDB';
  if (e.includes('mariadb')) return 'MariaDB';
  if (e.includes('oceanbase')) return 'OceanBase';
  if (e.includes('mysql')) return 'MySQL';
  if (e.includes('kingbase')) return 'KingbaseES';
  if (e.includes('gauss')) return 'openGauss';
  if (e.includes('postgres') || e === 'pg') return 'PostgreSQL';
  if (e.includes('sqlite')) return 'SQLite';
  if (e.includes('oracle')) return 'Oracle';
  if (e.includes('dameng') || e === 'dm') return '达梦 DM8';
  if (e.includes('sqlserver') || e.includes('mssql')) return 'SQL Server';
  if (e.includes('clickhouse')) return 'ClickHouse';
  if (e.includes('redis')) return 'Redis';
  return engineKey || 'SQL';
}

function stripLeadingSqlComments(sql: string): string {
  let s = sql.trim();
  while (s.length > 0) {
    if (s.startsWith('--')) {
      const nl = s.indexOf('\n');
      s = nl >= 0 ? s.slice(nl + 1).trimStart() : '';
      continue;
    }
    if (s.startsWith('/*')) {
      const close = s.indexOf('*/');
      s = close >= 0 ? s.slice(close + 2).trimStart() : '';
      continue;
    }
    break;
  }
  return s.trim();
}

export function isWriteOrDdlStatement(sql: string): boolean {
  const cleaned = stripLeadingSqlComments(sql);
  return /^(insert|update|delete|replace|merge|drop|alter|truncate|create|grant|revoke)\b/i.test(
    cleaned,
  );
}

/**
 * Strips leading `EXPLAIN ...` prefixes or multi-statement execution-plan wrappers (Oracle
 * `EXPLAIN PLAN FOR ... SELECT * FROM TABLE(DBMS_XPLAN.DISPLAY...)`, SQL Server `SET SHOWPLAN_*`)
 * so generating a new plan or switching dialects never creates `EXPLAIN EXPLAIN ...`.
 */
export function stripExistingExplainPrefix(rawSql: string): string {
  let s = stripLeadingSqlComments(rawSql).replace(/;+\s*$/, '').trim();

  for (let pass = 0; pass < 3; pass += 1) {
    const before = s;

    // SQL Server SET SHOWPLAN_TEXT / SHOWPLAN_XML / STATISTICS PROFILE wrappers
    s = s
      .replace(/^SET\s+(?:SHOWPLAN_TEXT|SHOWPLAN_XML|STATISTICS\s+PROFILE)\s+ON\s*;?\s*(?:GO\s*)?/i, '')
      .replace(/\s*(?:GO\s*)?SET\s+(?:SHOWPLAN_TEXT|SHOWPLAN_XML|STATISTICS\s+PROFILE)\s+OFF\s*;?\s*$/i, '')
      .trim();

    // Oracle trailing DBMS_XPLAN.DISPLAY query
    s = s
      .replace(/;?\s*SELECT\s+\*\s+FROM\s+TABLE\s*\(\s*DBMS_XPLAN\.DISPLAY[\s\S]*?\)\s*;?\s*$/i, '')
      .trim();

    // Leading EXPLAIN variants across supported dialects
    const patterns = [
      /^EXPLAIN\s+QUERY\s+PLAN\b\s*/i,
      /^EXPLAIN\s+PLAN\s+indexes\s*=\s*\d+(?:\s*,\s*[a-z_]+\s*=\s*\d+)*\b\s*/i,
      /^EXPLAIN\s+PLAN\s+FOR\b\s*/i,
      /^EXPLAIN\s+FORMAT\s*=\s*(?:'[^']*'|"[^"]*"|[A-Za-z_]+)\b\s*/i,
      /^EXPLAIN\s*\([^)]*\)\s*/i,
      /^EXPLAIN\s+(?:ANALYZE|EXTENDED|OUTLINE|PIPELINE|AST|FOR)\b\s*/i,
      /^EXPLAIN\b\s*/i,
      /^ANALYZE\b\s+(?=(?:SELECT|WITH|\())/i,
    ];
    for (const pat of patterns) {
      if (pat.test(s)) {
        s = s.replace(pat, '').trim();
        break;
      }
    }

    s = stripLeadingSqlComments(s).replace(/;+\s*$/, '').trim();
    if (s === before) break;
  }

  return s;
}

export function getExplainModesForEngine(engineKey: string): {
  dialectLabel: string;
  canRunDirectly: boolean;
  options: ExplainPlanModeOption[];
} {
  const e = (engineKey || 'mysql').toLowerCase();
  const dialectLabel = getEngineDisplayName(engineKey);

  if (e.includes('tidb')) {
    return {
      dialectLabel,
      canRunDirectly: true,
      options: [
        {
          mode: 'standard',
          label: 'TiDB 标准执行计划',
          shortSyntax: 'EXPLAIN',
          executesQuery: false,
          description: '查看 TiDB 优化器估算的分布式算子、索引扫描与 TiKV/TiFlash 路径',
        },
        {
          mode: 'analyze',
          label: 'TiDB 实际运行剖析',
          shortSyntax: 'EXPLAIN ANALYZE',
          executesQuery: true,
          description: '实际执行查询并返回真实 actRows、execution info 与 TiKV 耗时统计',
        },
        {
          mode: 'verbose',
          label: 'TiDB 详细 Cost 计划',
          shortSyntax: "EXPLAIN FORMAT='verbose'",
          executesQuery: false,
          description: '包含估算 Cost、内存与网络开销的详细计划',
        },
      ],
    };
  }

  if (e.includes('oceanbase')) {
    return {
      dialectLabel,
      canRunDirectly: true,
      options: [
        {
          mode: 'standard',
          label: 'OceanBase 标准执行计划',
          shortSyntax: 'EXPLAIN',
          executesQuery: false,
          description: '展示算子树、表扫描与估算行数及代价',
        },
        {
          mode: 'analyze',
          label: 'OceanBase 扩展计划',
          shortSyntax: 'EXPLAIN EXTENDED',
          executesQuery: false,
          description: '展示分区裁剪、表达式改写、输出列与完整 Cost Breakdown（不实际执行）',
        },
        {
          mode: 'verbose',
          label: 'OceanBase Outline 计划',
          shortSyntax: 'EXPLAIN OUTLINE',
          executesQuery: false,
          description: '生成带 Hint Outline 调优指令的完整执行计划',
        },
      ],
    };
  }

  if (e.includes('mariadb')) {
    return {
      dialectLabel,
      canRunDirectly: true,
      options: [
        {
          mode: 'standard',
          label: 'MariaDB 标准执行计划',
          shortSyntax: 'EXPLAIN',
          executesQuery: false,
          description: '标准表格式优化器执行计划（type / possible_keys / key / rows）',
        },
        {
          mode: 'analyze',
          label: 'MariaDB 真实运行剖析',
          shortSyntax: 'ANALYZE',
          executesQuery: true,
          description: '实际运行并返回 r_rows、r_filtered 与循环次数对比',
        },
        {
          mode: 'verbose',
          label: 'MariaDB JSON 详情计划',
          shortSyntax: 'EXPLAIN FORMAT=JSON',
          executesQuery: false,
          description: '包含精确 Cost 与索引条件细节的 JSON 计划',
        },
      ],
    };
  }

  if (e.includes('mysql')) {
    return {
      dialectLabel,
      canRunDirectly: true,
      options: [
        {
          mode: 'standard',
          label: 'MySQL 标准执行计划',
          shortSyntax: 'EXPLAIN',
          executesQuery: false,
          description: '标准表格式执行计划（type / key / key_len / rows / Extra）',
        },
        {
          mode: 'analyze',
          label: 'MySQL 实际耗时剖析 (8.0+)',
          shortSyntax: 'EXPLAIN ANALYZE',
          executesQuery: true,
          description: '实际执行查询并输出每个算子的真实耗时、循环与实际返回行数',
        },
        {
          mode: 'verbose',
          label: 'MySQL 树状 Cost 计划',
          shortSyntax: 'EXPLAIN FORMAT=TREE',
          executesQuery: false,
          description: '树状展示连接顺序与优化器估算 Cost（不执行查询）',
        },
      ],
    };
  }

  if (e.includes('postgres') || e.includes('kingbase') || e.includes('gauss') || e === 'pg') {
    return {
      dialectLabel,
      canRunDirectly: true,
      options: [
        {
          mode: 'standard',
          label: `${dialectLabel} 详细估算计划`,
          shortSyntax: 'EXPLAIN (COSTS, VERBOSE)',
          executesQuery: false,
          description: '不执行查询，展示节点 Cost、预估行数、目标输出列与 Schema',
        },
        {
          mode: 'analyze',
          label: `${dialectLabel} 实际执行 + 缓存统计`,
          shortSyntax: 'EXPLAIN (ANALYZE, BUFFERS)',
          executesQuery: true,
          description: '实际执行语句，返回真实耗时与共享缓存块命中/磁盘读取统计',
        },
        {
          mode: 'verbose',
          label: `${dialectLabel} JSON 结构化计划`,
          shortSyntax: 'EXPLAIN (FORMAT JSON)',
          executesQuery: false,
          description: '输出完整 JSON 计划，便于结构化比对节点成本',
        },
      ],
    };
  }

  if (e.includes('sqlite')) {
    return {
      dialectLabel,
      canRunDirectly: true,
      options: [
        {
          mode: 'standard',
          label: 'SQLite 高层索引与扫描计划',
          shortSyntax: 'EXPLAIN QUERY PLAN',
          executesQuery: false,
          description: '直观展示 SCAN TABLE / SEARCH USING INDEX 与临时排序信息',
        },
        {
          mode: 'verbose',
          label: 'SQLite VDBE 字节码指令',
          shortSyntax: 'EXPLAIN',
          executesQuery: false,
          description: '输出 SQLite 虚拟机底层字节码指令清单',
        },
      ],
    };
  }

  if (e.includes('oracle')) {
    return {
      dialectLabel,
      canRunDirectly: false,
      options: [
        {
          mode: 'standard',
          label: 'Oracle 标准 DBMS_XPLAN',
          shortSyntax: 'DBMS_XPLAN.DISPLAY',
          executesQuery: false,
          description: '生成 EXPLAIN PLAN FOR 与 DBMS_XPLAN.DISPLAY 双段查询脚本文档',
        },
        {
          mode: 'analyze',
          label: 'Oracle 运行时统计格式',
          shortSyntax: "DISPLAY(..., 'ALLSTATS LAST')",
          executesQuery: false,
          description: '带 ALLSTATS LAST 格式参数的 DBMS_XPLAN 展示语句',
        },
        {
          mode: 'verbose',
          label: 'Oracle 全量详情计划',
          shortSyntax: "DISPLAY(..., 'ALL')",
          executesQuery: false,
          description: '包含投影列、谓词过滤与 Outline 的全量 DBMS_XPLAN 展示',
        },
      ],
    };
  }

  if (e.includes('dameng') || e === 'dm') {
    return {
      dialectLabel,
      canRunDirectly: false,
      options: [
        {
          mode: 'standard',
          label: '达梦 DM8 标准执行计划',
          shortSyntax: 'EXPLAIN',
          executesQuery: false,
          description: '生成达梦 DM8 EXPLAIN 执行计划语句',
        },
        {
          mode: 'verbose',
          label: '达梦 DM8 EXPLAIN FOR 计划',
          shortSyntax: 'EXPLAIN FOR',
          executesQuery: false,
          description: '生成达梦 DM8 EXPLAIN FOR 执行计划语句',
        },
      ],
    };
  }

  if (e.includes('sqlserver') || e.includes('mssql')) {
    return {
      dialectLabel,
      canRunDirectly: false,
      options: [
        {
          mode: 'standard',
          label: 'SQL Server 文本估算计划',
          shortSyntax: 'SET SHOWPLAN_TEXT ON',
          executesQuery: false,
          description: '不执行查询，开启 SHOWPLAN_TEXT 返回预估计划',
        },
        {
          mode: 'analyze',
          label: 'SQL Server 实际执行计划',
          shortSyntax: 'SET STATISTICS PROFILE ON',
          executesQuery: true,
          description: '实际执行查询并附带每个算子的真实 Rows / Executes 统计',
        },
        {
          mode: 'verbose',
          label: 'SQL Server XML 详细计划',
          shortSyntax: 'SET SHOWPLAN_XML ON',
          executesQuery: false,
          description: '生成完整 XML 格式估算执行计划（可保存为 .sqlplan）',
        },
      ],
    };
  }

  if (e.includes('clickhouse')) {
    return {
      dialectLabel,
      canRunDirectly: false,
      options: [
        {
          mode: 'standard',
          label: 'ClickHouse 索引裁剪计划',
          shortSyntax: 'EXPLAIN PLAN indexes = 1',
          executesQuery: false,
          description: '查看 MergeTree 分区与主键粒度 Granule 裁剪效果',
        },
        {
          mode: 'analyze',
          label: 'ClickHouse 并行执行管线',
          shortSyntax: 'EXPLAIN PIPELINE',
          executesQuery: false,
          description: '查看多线程流式计算 Pipeline 算子拓扑（不实际跑全表）',
        },
        {
          mode: 'verbose',
          label: 'ClickHouse 抽象语法树',
          shortSyntax: 'EXPLAIN AST',
          executesQuery: false,
          description: '查看查询解析重写后的 AST 结构',
        },
      ],
    };
  }

  if (e.includes('redis')) {
    return {
      dialectLabel: 'Redis',
      canRunDirectly: true,
      options: [
        {
          mode: 'standard',
          label: 'Redis Key 诊断 (MEMORY / ENCODING / TTL)',
          shortSyntax: 'MEMORY USAGE',
          executesQuery: false,
          description: '检查当前命令目标 Key 的数据类型、TTL 过期时间、内部编码与内存占用',
        },
        {
          mode: 'verbose',
          label: 'Redis 实例状态总览',
          shortSyntax: 'INFO',
          executesQuery: false,
          description: '查看 Redis 内存占用、命中率、客户端连接与各 db 键空间统计',
        },
      ],
    };
  }

  return {
    dialectLabel,
    canRunDirectly: true,
    options: [
      {
        mode: 'standard',
        label: `${dialectLabel} 标准执行计划`,
        shortSyntax: 'EXPLAIN',
        executesQuery: false,
        description: '生成标准 EXPLAIN 语句',
      },
      {
        mode: 'analyze',
        label: `${dialectLabel} 执行剖析`,
        shortSyntax: 'EXPLAIN ANALYZE',
        executesQuery: true,
        description: '生成 EXPLAIN ANALYZE 语句',
      },
    ],
  };
}

/**
 * Generates dialect-specific execution plan SQL for the selected database engine and mode.
 */
export function buildExplainPlanSql(
  rawSql: string,
  engineKey: string,
  mode: ExplainPlanMode = 'standard',
): ExplainPlanResult {
  const inner = stripExistingExplainPrefix(rawSql);
  const { dialectLabel, canRunDirectly, options } = getExplainModesForEngine(engineKey);
  const chosenOption =
    options.find((o) => o.mode === mode) ?? options[0];
  const resolvedMode = chosenOption.mode;
  const e = (engineKey || 'mysql').toLowerCase();

  let sql = '';
  if (e.includes('redis')) {
    if (resolvedMode === 'verbose') {
      sql = 'INFO';
    } else {
      const firstCmd = splitRedisCommands(rawSql)[0] ?? rawSql.trim();
      const argMatch = firstCmd.match(/^\S+\s+("(?:\\.|[^"])*"|'(?:\\.|[^'])*'|\S+)/);
      const targetKey = argMatch?.[1];
      sql = targetKey ? `MEMORY USAGE ${targetKey}` : 'INFO keyspace';
    }
  } else if (e.includes('tidb')) {
    if (resolvedMode === 'analyze') sql = `EXPLAIN ANALYZE ${inner};`;
    else if (resolvedMode === 'verbose') sql = `EXPLAIN FORMAT = 'verbose' ${inner};`;
    else sql = `EXPLAIN ${inner};`;
  } else if (e.includes('oceanbase')) {
    if (resolvedMode === 'analyze') sql = `EXPLAIN EXTENDED ${inner};`;
    else if (resolvedMode === 'verbose') sql = `EXPLAIN OUTLINE ${inner};`;
    else sql = `EXPLAIN ${inner};`;
  } else if (e.includes('mariadb')) {
    if (resolvedMode === 'analyze') sql = `ANALYZE ${inner};`;
    else if (resolvedMode === 'verbose') sql = `EXPLAIN FORMAT = JSON ${inner};`;
    else sql = `EXPLAIN ${inner};`;
  } else if (e.includes('mysql')) {
    if (resolvedMode === 'analyze') sql = `EXPLAIN ANALYZE ${inner};`;
    else if (resolvedMode === 'verbose') sql = `EXPLAIN FORMAT = TREE ${inner};`;
    else sql = `EXPLAIN ${inner};`;
  } else if (e.includes('postgres') || e.includes('kingbase') || e.includes('gauss') || e === 'pg') {
    if (resolvedMode === 'analyze') sql = `EXPLAIN (ANALYZE, BUFFERS)\n${inner};`;
    else if (resolvedMode === 'verbose') sql = `EXPLAIN (FORMAT JSON, COSTS, VERBOSE)\n${inner};`;
    else sql = `EXPLAIN (COSTS, VERBOSE)\n${inner};`;
  } else if (e.includes('sqlite')) {
    if (resolvedMode === 'verbose') sql = `EXPLAIN\n${inner};`;
    else sql = `EXPLAIN QUERY PLAN\n${inner};`;
  } else if (e.includes('oracle')) {
    if (resolvedMode === 'analyze') {
      sql = `EXPLAIN PLAN FOR\n${inner};\nSELECT * FROM TABLE(DBMS_XPLAN.DISPLAY(NULL, NULL, 'ALLSTATS LAST'));`;
    } else if (resolvedMode === 'verbose') {
      sql = `EXPLAIN PLAN FOR\n${inner};\nSELECT * FROM TABLE(DBMS_XPLAN.DISPLAY(NULL, NULL, 'ALL'));`;
    } else {
      sql = `EXPLAIN PLAN FOR\n${inner};\nSELECT * FROM TABLE(DBMS_XPLAN.DISPLAY);`;
    }
  } else if (e.includes('dameng') || e === 'dm') {
    sql = resolvedMode === 'standard' ? `EXPLAIN ${inner};` : `EXPLAIN FOR ${inner};`;
  } else if (e.includes('sqlserver') || e.includes('mssql')) {
    if (resolvedMode === 'analyze') {
      sql = `SET STATISTICS PROFILE ON;\n${inner};\nSET STATISTICS PROFILE OFF;`;
    } else if (resolvedMode === 'verbose') {
      sql = `SET SHOWPLAN_XML ON;\nGO\n${inner};\nGO\nSET SHOWPLAN_XML OFF;`;
    } else {
      sql = `SET SHOWPLAN_TEXT ON;\nGO\n${inner};\nGO\nSET SHOWPLAN_TEXT OFF;`;
    }
  } else if (e.includes('clickhouse')) {
    if (resolvedMode === 'analyze') sql = `EXPLAIN PIPELINE\n${inner};`;
    else if (resolvedMode === 'verbose') sql = `EXPLAIN AST\n${inner};`;
    else sql = `EXPLAIN PLAN indexes = 1\n${inner};`;
  } else {
    sql = resolvedMode === 'analyze' ? `EXPLAIN ANALYZE ${inner};` : `EXPLAIN ${inner};`;
  }

  const lines = [`-- [${dialectLabel}] ${chosenOption.label} (${chosenOption.shortSyntax})`];
  if (chosenOption.executesQuery) {
    lines.push('-- ⚠ 注意：该模式会实际执行 SQL 以采集真实耗时与行数，写操作/生产库请谨慎运行');
  }
  const headerComment = lines.join('\n');
  const fullScript = `${headerComment}\n${sql}\n`;

  return {
    engineKey,
    dialectLabel,
    mode: resolvedMode,
    modeLabel: chosenOption.shortSyntax,
    headerComment,
    sql,
    fullScript,
    executesQuery: chosenOption.executesQuery,
    canRunDirectly,
  };
}

// ==================== Schema & Object Pattern Filter + SSH Command Helpers ====================

function globToRegex(glob: string): RegExp {
  const escaped = glob.replace(/[.+^${}()|[\]\\]/g, '\\$&').replace(/\*/g, '.*').replace(/\?/g, '.');
  return new RegExp(`^${escaped}$`, 'i');
}

/**
 * Matches a schema, database, table, or view name against comma/whitespace-separated glob patterns.
 * Supports positive rules (`app_*`, `all`, `*`, `default`), exclusion rules (`-tmp_*`, `-*_bak`),
 * and optional object-kind prefixes (`table:*`, `view:*`, `schema:*`). Empty pattern matches all.
 */
export function matchesFilterPattern(
  name: string,
  pattern?: string | null,
  defaultName?: string | null,
): boolean {
  const raw = (pattern ?? '').trim();
  if (!raw) return true;

  const tokens = raw
    .split(/[\s,;]+/)
    .map((t) => t.trim())
    .filter(Boolean);
  if (tokens.length === 0) return true;

  const includes: RegExp[] = [];
  const excludes: RegExp[] = [];
  let hasAll = false;

  for (const rawToken of tokens) {
    const isExclude = rawToken.startsWith('-') && rawToken.length > 1;
    let body = isExclude ? rawToken.slice(1).trim() : rawToken;
    body = body.replace(/^(?:table|view|schema):/i, '').trim();
    if (!body) continue;

    const lower = body.toLowerCase();
    if (!isExclude && (lower === 'all' || lower === '*')) {
      hasAll = true;
      continue;
    }
    if (!isExclude && lower === 'default') {
      if (defaultName && defaultName.trim()) {
        includes.push(globToRegex(defaultName.trim()));
      }
      continue;
    }
    if (isExclude) {
      excludes.push(globToRegex(body));
    } else {
      includes.push(globToRegex(body));
    }
  }

  for (const ex of excludes) {
    if (ex.test(name)) return false;
  }
  if (hasAll || includes.length === 0) return true;
  return includes.some((inc) => inc.test(name));
}

export function buildSshTunnelCommand(
  ssh: {
    host: string;
    port?: number | string | null;
    username?: string | null;
    keyFile?: string | null;
    localPort?: number | string | null;
  },
  targetHost: string,
  targetPort: number | string,
): string {
  const host = (ssh.host || 'bastion.example.com').trim();
  const sshPort = Number(ssh.port) || 22;
  const user = (ssh.username || 'ops').trim();
  const localPort = Number(ssh.localPort) || Number(targetPort) || 13306;
  const dbHost = (targetHost || '127.0.0.1').trim();
  const dbPort = Number(targetPort) || 3306;
  const keyPart = ssh.keyFile?.trim() ? ` -i "${ssh.keyFile.trim()}"` : '';
  const portPart = sshPort !== 22 ? ` -p ${sshPort}` : '';
  return `ssh -N -L ${localPort}:${dbHost}:${dbPort}${keyPart}${portPart} ${user}@${host}`;
}

// ==================== High-Frequency Productivity Tools (v0.9.4) ====================

export function toCsvWithBom(columns: string[], rows: Record<string, unknown>[]): string {
  return `\uFEFF${toCsv(columns, rows)}`;
}

export function toTsv(columns: string[], rows: Record<string, unknown>[]): string {
  if (columns.length === 0) return '';
  const esc = (v: unknown) => {
    if (v === null || v === undefined) return '';
    const s = typeof v === 'object' ? JSON.stringify(v) : String(v);
    return s.replace(/[\t\r\n]+/g, ' ');
  };
  return [
    columns.map(esc).join('\t'),
    ...rows.map((r) => columns.map((c) => esc(r[c])).join('\t')),
  ].join('\n');
}

export function toUpdateSql(
  tableName: string,
  columns: string[],
  primaryKeys: string[],
  rows: Record<string, unknown>[],
  engineKey = 'mysql',
): string {
  if (columns.length === 0 || rows.length === 0) return '';
  const q = (id: string) => quoteIdentifier(id, engineKey);
  const safeTable = q(tableName || 'target_table');
  const usablePks = primaryKeys.filter((pk) => columns.includes(pk));
  const keyCols = usablePks.length > 0 ? usablePks : [columns[0]];
  const setCols = columns.filter((c) => !keyCols.includes(c));
  const effectiveSetCols = setCols.length > 0 ? setCols : columns;

  return rows
    .map((r) => {
      const setClause = effectiveSetCols
        .map((c) => `${q(c)} = ${formatSqlLiteral(r[c])}`)
        .join(', ');
      const whereClause = keyCols
        .map((k) => {
          const val = r[k];
          return val === null || val === undefined ? `${q(k)} IS NULL` : `${q(k)} = ${formatSqlLiteral(val)}`;
        })
        .join(' AND ');
      return `UPDATE ${safeTable} SET ${setClause} WHERE ${whereClause};`;
    })
    .join('\n');
}

export function detectCsvDelimiter(raw: string): string {
  const firstLine = raw.replace(/^\uFEFF/, '').split(/\r?\n/).find((l) => l.trim().length > 0) ?? '';
  let commas = 0;
  let tabs = 0;
  let semis = 0;
  let inQuotes = false;
  for (let i = 0; i < firstLine.length; i += 1) {
    const ch = firstLine[i];
    if (ch === '"') {
      inQuotes = !inQuotes;
      continue;
    }
    if (!inQuotes) {
      if (ch === ',') commas += 1;
      else if (ch === '\t') tabs += 1;
      else if (ch === ';') semis += 1;
    }
  }
  if (tabs > commas && tabs >= semis) return '\t';
  if (semis > commas && commas === 0 && tabs === 0) return ';';
  return ',';
}

export function parseCsvText(
  raw: string,
  opts: { delimiter?: string; hasHeader?: boolean } = {},
): { headers: string[]; rows: string[][]; delimiter: string } {
  const text = raw.replace(/^\uFEFF/, '');
  const delimiter = opts.delimiter || detectCsvDelimiter(text);
  const hasHeader = opts.hasHeader !== false;
  const matrix: string[][] = [];
  let row: string[] = [];
  let field = '';
  let inQuotes = false;

  for (let i = 0; i < text.length; i += 1) {
    const ch = text[i];
    if (inQuotes) {
      if (ch === '"') {
        if (text[i + 1] === '"') {
          field += '"';
          i += 1;
        } else {
          inQuotes = false;
        }
      } else {
        field += ch;
      }
      continue;
    }

    if (ch === '"') {
      inQuotes = true;
    } else if (ch === delimiter) {
      row.push(field);
      field = '';
    } else if (ch === '\r') {
      if (text[i + 1] === '\n') i += 1;
      row.push(field);
      if (row.length > 1 || row[0] !== '') matrix.push(row);
      row = [];
      field = '';
    } else if (ch === '\n') {
      row.push(field);
      if (row.length > 1 || row[0] !== '') matrix.push(row);
      row = [];
      field = '';
    } else {
      field += ch;
    }
  }

  if (field.length > 0 || row.length > 0) {
    row.push(field);
    if (row.length > 1 || row[0] !== '') matrix.push(row);
  }

  if (matrix.length === 0) {
    return { headers: [], rows: [], delimiter };
  }

  const maxWidth = matrix.reduce((m, r) => Math.max(m, r.length), 0);
  if (!hasHeader) {
    const headers = Array.from({ length: maxWidth }, (_, idx) => `col_${idx + 1}`);
    return { headers, rows: matrix, delimiter };
  }

  const used = new Set<string>();
  const headers = matrix[0].map((rawName, idx) => {
    const base = rawName.trim() || `col_${idx + 1}`;
    let name = base;
    let suffix = 2;
    while (used.has(name)) {
      name = `${base}_${suffix++}`;
    }
    used.add(name);
    return name;
  });
  return { headers, rows: matrix.slice(1), delimiter };
}

export function parseImportData(
  raw: string,
  format: 'auto' | 'csv' | 'json' = 'auto',
  opts: { delimiter?: string; hasHeader?: boolean } = {},
): {
  detectedFormat: 'csv' | 'json';
  headers: string[];
  rows: Record<string, unknown>[];
  delimiter?: string;
} {
  const cleaned = raw.replace(/^\uFEFF/, '').trim();
  if (!cleaned) return { detectedFormat: 'csv', headers: [], rows: [], delimiter: ',' };

  const shouldTryJson =
    format === 'json' ||
    (format === 'auto' && (cleaned.startsWith('[') || cleaned.startsWith('{')));

  if (shouldTryJson) {
    const parsed = JSON.parse(cleaned) as unknown;
    let arr: unknown[] = [];
    if (Array.isArray(parsed)) {
      arr = parsed;
    } else if (parsed && typeof parsed === 'object') {
      const rec = parsed as Record<string, unknown>;
      const nested = [rec.data, rec.rows, rec.records, rec.items].find((v) => Array.isArray(v));
      arr = Array.isArray(nested) ? nested : [parsed];
    }
    const objRows = arr.filter((r): r is Record<string, unknown> => Boolean(r && typeof r === 'object'));
    const headers: string[] = [];
    const seen = new Set<string>();
    for (const r of objRows.slice(0, 200)) {
      for (const k of Object.keys(r)) {
        if (!seen.has(k)) {
          seen.add(k);
          headers.push(k);
        }
      }
    }
    return { detectedFormat: 'json', headers, rows: objRows };
  }

  const csv = parseCsvText(cleaned, opts);
  const rows: Record<string, unknown>[] = csv.rows.map((cells) => {
    const obj: Record<string, unknown> = {};
    for (let i = 0; i < csv.headers.length; i += 1) {
      obj[csv.headers[i]] = cells[i] ?? '';
    }
    return obj;
  });
  return { detectedFormat: 'csv', headers: csv.headers, rows, delimiter: csv.delimiter };
}

function isNumericSqlType(dataType?: string): boolean {
  if (!dataType) return false;
  return /\b(int|bigint|smallint|tinyint|mediumint|decimal|numeric|float|double|real|number|serial)\b/i.test(
    dataType,
  );
}

function isBooleanSqlType(dataType?: string): boolean {
  if (!dataType) return false;
  return /\b(bool|boolean|bit)\b/i.test(dataType);
}

function isTextSqlType(dataType?: string): boolean {
  if (!dataType) return false;
  return /\b(char|varchar|text|clob|string|json|uuid|enum)\b/i.test(dataType);
}

function formatImportCellValue(val: unknown, targetType?: string): string {
  if (val === null || val === undefined) return 'NULL';
  if (typeof val === 'number') return Number.isFinite(val) ? String(val) : 'NULL';
  if (typeof val === 'boolean') return val ? 'TRUE' : 'FALSE';
  if (typeof val === 'object') return `'${JSON.stringify(val).replace(/'/g, "''")}'`;

  const s = String(val);
  if (s === '') return 'NULL';
  const upper = s.trim().toUpperCase();
  if (upper === 'NULL') return 'NULL';

  if (isBooleanSqlType(targetType)) {
    if (upper === 'TRUE' || s === '1') return 'TRUE';
    if (upper === 'FALSE' || s === '0') return 'FALSE';
  }
  if (!isTextSqlType(targetType) && /^-?(?:0|[1-9]\d*)(?:\.\d+)?$/.test(s.trim())) {
    if (isNumericSqlType(targetType) || s.trim().length <= 15) {
      return s.trim();
    }
  }
  return `'${s.replace(/'/g, "''")}'`;
}

export function buildBatchInsertStatements(opts: {
  tableName: string;
  dbName?: string;
  engineKey: string;
  targetColumns: string[];
  targetColumnTypes?: Record<string, string>;
  rows: Array<Record<string, unknown>>;
  batchSize?: number;
  ignoreDuplicates?: boolean;
  mode?: 'insert' | 'ignore' | 'upsert';
  conflictKeys?: string[];
}): string[] {
  if (opts.targetColumns.length === 0 || opts.rows.length === 0) return [];
  const q = (id: string) => quoteIdentifier(id, opts.engineKey);
  const qualified =
    opts.dbName && opts.dbName !== 'main' && opts.dbName !== 'default'
      ? `${q(opts.dbName)}.${q(opts.tableName)}`
      : q(opts.tableName);
  const colList = opts.targetColumns.map(q).join(', ');
  const eng = (opts.engineKey || 'mysql').toLowerCase();
  const isPgFamily = eng.includes('postgres') || eng.includes('kingbase') || eng.includes('gauss') || eng === 'pg';
  const isSqlite = eng.includes('sqlite');
  const isMysqlFamily =
    eng.includes('mysql') || eng.includes('tidb') || eng.includes('maria') || eng.includes('oceanbase');

  const mode = opts.mode ?? (opts.ignoreDuplicates ? 'ignore' : 'insert');
  let prefix = 'INSERT INTO';
  let suffix = '';

  if (mode === 'ignore') {
    if (isSqlite) prefix = 'INSERT OR IGNORE INTO';
    else if (isMysqlFamily) prefix = 'INSERT IGNORE INTO';
    else if (isPgFamily) suffix = ' ON CONFLICT DO NOTHING';
  } else if (mode === 'upsert') {
    const keys = (opts.conflictKeys ?? []).filter((k) => opts.targetColumns.includes(k));
    const effectiveKeys = keys.length > 0 ? keys : [opts.targetColumns[0]];
    const updateCols = opts.targetColumns.filter((c) => !effectiveKeys.includes(c));
    if (isMysqlFamily) {
      if (updateCols.length > 0) {
        suffix = ` ON DUPLICATE KEY UPDATE ${updateCols.map((c) => `${q(c)} = VALUES(${q(c)})`).join(', ')}`;
      } else {
        prefix = 'INSERT IGNORE INTO';
      }
    } else if (isPgFamily || isSqlite) {
      const conflictClause = effectiveKeys.map(q).join(', ');
      if (updateCols.length > 0) {
        suffix = ` ON CONFLICT (${conflictClause}) DO UPDATE SET ${updateCols.map((c) => `${q(c)} = EXCLUDED.${q(c)}`).join(', ')}`;
      } else {
        suffix = ` ON CONFLICT (${conflictClause}) DO NOTHING`;
      }
    }
  }

  const batchSize = Math.max(1, opts.batchSize ?? 100);
  const out: string[] = [];

  for (let start = 0; start < opts.rows.length; start += batchSize) {
    const slice = opts.rows.slice(start, start + batchSize);
    const tuples = slice.map((r) => {
      const vals = opts.targetColumns.map((col) =>
        formatImportCellValue(r[col], opts.targetColumnTypes?.[col]),
      );
      return `(${vals.join(', ')})`;
    });
    out.push(`${prefix} ${qualified} (${colList}) VALUES\n${tuples.join(',\n')}${suffix};`);
  }

  return out;
}

export type EntityCodeLang = 'java' | 'go' | 'typescript' | 'python' | 'rust';

export interface EntityColumnInput {
  name: string;
  dataType: string;
  nullable: boolean;
  isPrimaryKey: boolean;
  comment?: string | null;
  defaultValue?: string | null;
}

function toPascalCase(raw: string): string {
  return (
    raw
      .split(/[^A-Za-z0-9]+/)
      .filter(Boolean)
      .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
      .join('') || 'EntityModel'
  );
}

function toCamelCase(raw: string): string {
  const pascal = toPascalCase(raw);
  return pascal.charAt(0).toLowerCase() + pascal.slice(1);
}

function toSnakeCase(raw: string): string {
  return raw
    .replace(/([a-z0-9])([A-Z])/g, '$1_$2')
    .replace(/[^A-Za-z0-9]+/g, '_')
    .toLowerCase()
    .replace(/^_+|_+$/g, '') || 'field';
}

function classifySqlType(
  rawType: string,
): 'bool' | 'bigint' | 'int' | 'decimal' | 'float' | 'date' | 'datetime' | 'blob' | 'json' | 'string' {
  const t = (rawType || '').toLowerCase();
  if (/\btinyint\s*\(\s*1\s*\)|\bbool|\bboolean|\bbit\b/.test(t)) return 'bool';
  if (/\bbigint|\bint8\b/.test(t)) return 'bigint';
  if (/\bint|\bsmallint|\btinyint|\bmediumint|\bserial|\binteger/.test(t)) return 'int';
  if (/\bdecimal|\bnumeric|\bnumber/.test(t)) return 'decimal';
  if (/\bfloat|\bdouble|\breal/.test(t)) return 'float';
  if (/\bdatetime|\btimestamp/.test(t)) return 'datetime';
  if (/\bdate\b/.test(t)) return 'date';
  if (/\bblob|\bbytea|\bbinary|\bvarbinary/.test(t)) return 'blob';
  if (/\bjson/.test(t)) return 'json';
  return 'string';
}

export function generateEntityCode(
  tableName: string,
  columns: EntityColumnInput[],
  lang: EntityCodeLang,
  opts: {
    camelCase?: boolean;
    withOrmTags?: boolean;
    withComments?: boolean;
    pythonStyle?: 'dataclass' | 'pydantic';
  } = {},
): { filename: string; code: string } {
  const className = toPascalCase(tableName);
  const camelCase = opts.camelCase !== false;
  const withOrmTags = opts.withOrmTags !== false;
  const withComments = opts.withComments !== false;

  if (lang === 'java') {
    const usesDecimal = columns.some((c) => classifySqlType(c.dataType) === 'decimal');
    const usesTime = columns.some((c) => {
      const k = classifySqlType(c.dataType);
      return k === 'datetime' || k === 'date';
    });
    const imports = ['import java.io.Serializable;', 'import lombok.Data;'];
    if (withOrmTags) {
      imports.push(
        'import com.baomidou.mybatisplus.annotation.IdType;',
        'import com.baomidou.mybatisplus.annotation.TableField;',
        'import com.baomidou.mybatisplus.annotation.TableId;',
        'import com.baomidou.mybatisplus.annotation.TableName;',
      );
    }
    if (usesDecimal) imports.push('import java.math.BigDecimal;');
    if (usesTime) {
      imports.push('import java.time.LocalDate;', 'import java.time.LocalDateTime;');
    }

    const fields = columns.map((c) => {
      const k = classifySqlType(c.dataType);
      const jType =
        k === 'bool'
          ? 'Boolean'
          : k === 'bigint'
            ? 'Long'
            : k === 'int'
              ? 'Integer'
              : k === 'decimal'
                ? 'BigDecimal'
                : k === 'float'
                  ? 'Double'
                  : k === 'datetime'
                    ? 'LocalDateTime'
                    : k === 'date'
                      ? 'LocalDate'
                      : k === 'blob'
                        ? 'byte[]'
                        : 'String';
      const fName = camelCase ? toCamelCase(c.name) : toSnakeCase(c.name);
      const lines: string[] = [];
      if (withComments && c.comment) {
        lines.push(`    /** ${c.comment} */`);
      }
      if (withOrmTags) {
        if (c.isPrimaryKey) {
          lines.push(`    @TableId(value = "${c.name}", type = IdType.AUTO)`);
        } else {
          lines.push(`    @TableField("${c.name}")`);
        }
      }
      lines.push(`    private ${jType} ${fName};`);
      return lines.join('\n');
    });

    const headerAnns = ['@Data'];
    if (withOrmTags) headerAnns.push(`@TableName("${tableName}")`);
    return {
      filename: `${className}.java`,
      code: `${imports.join('\n')}\n\n${headerAnns.join('\n')}\npublic class ${className} implements Serializable {\n\n${fields.join('\n\n')}\n}\n`,
    };
  }

  if (lang === 'go') {
    const usesTime = columns.some((c) => {
      const k = classifySqlType(c.dataType);
      return k === 'datetime' || k === 'date';
    });
    const header = usesTime ? 'package model\n\nimport "time"\n\n' : 'package model\n\n';
    const fields = columns.map((c) => {
      const k = classifySqlType(c.dataType);
      const base =
        k === 'bool'
          ? 'bool'
          : k === 'bigint'
            ? 'int64'
            : k === 'int'
              ? 'int'
              : k === 'decimal' || k === 'float'
                ? 'float64'
                : k === 'datetime' || k === 'date'
                  ? 'time.Time'
                  : k === 'blob'
                    ? '[]byte'
                    : 'string';
      const goType = c.nullable && !c.isPrimaryKey && base !== '[]byte' ? `*${base}` : base;
      const goName = toPascalCase(c.name);
      const jsonKey = camelCase ? toCamelCase(c.name) : toSnakeCase(c.name);
      const gormParts = [`column:${c.name}`];
      if (c.isPrimaryKey) gormParts.push('primaryKey');
      const tag = withOrmTags ? ` \`gorm:"${gormParts.join(';')}" json:"${jsonKey}"\`` : '';
      const comment = withComments && c.comment ? ` // ${c.comment}` : '';
      return `\t${goName} ${goType}${tag}${comment}`;
    });
    const tableFn = withOrmTags
      ? `\n\nfunc (${className}) TableName() string {\n\treturn "${tableName}"\n}`
      : '';
    return {
      filename: `${toSnakeCase(tableName)}.go`,
      code: `${header}type ${className} struct {\n${fields.join('\n')}\n}${tableFn}\n`,
    };
  }

  if (lang === 'typescript') {
    const fields = columns.map((c) => {
      const k = classifySqlType(c.dataType);
      const base =
        k === 'bool'
          ? 'boolean'
          : k === 'int' || k === 'bigint' || k === 'float'
            ? 'number'
            : k === 'json'
              ? 'Record<string, unknown>'
              : 'string';
      const tsType = c.nullable && !c.isPrimaryKey ? `${base} | null` : base;
      const fName = camelCase ? toCamelCase(c.name) : c.name;
      const comment = withComments && c.comment ? `  /** ${c.comment} */\n` : '';
      return `${comment}  ${fName}: ${tsType};`;
    });
    return {
      filename: `${className}.ts`,
      code: `export interface ${className} {\n${fields.join('\n')}\n}\n`,
    };
  }

  if (lang === 'python') {
    const pydantic = opts.pythonStyle !== 'dataclass';
    const imports = ['from typing import Optional'];
    const usesDateTime = columns.some((c) => classifySqlType(c.dataType) === 'datetime');
    const usesDate = columns.some((c) => classifySqlType(c.dataType) === 'date');
    const usesDecimal = columns.some((c) => classifySqlType(c.dataType) === 'decimal');
    if (usesDateTime) imports.push('from datetime import datetime');
    if (usesDate) imports.push('from datetime import date');
    if (usesDecimal) imports.push('from decimal import Decimal');
    imports.push(pydantic ? 'from pydantic import BaseModel, Field' : 'from dataclasses import dataclass');

    const fields = columns.map((c) => {
      const k = classifySqlType(c.dataType);
      const base =
        k === 'bool'
          ? 'bool'
          : k === 'int' || k === 'bigint'
            ? 'int'
            : k === 'decimal'
              ? 'Decimal'
              : k === 'float'
                ? 'float'
                : k === 'datetime'
                  ? 'datetime'
                  : k === 'date'
                    ? 'date'
                    : k === 'json'
                      ? 'dict'
                      : k === 'blob'
                        ? 'bytes'
                        : 'str';
      const pyType = c.nullable && !c.isPrimaryKey ? `Optional[${base}]` : base;
      const fName = camelCase ? toCamelCase(c.name) : toSnakeCase(c.name);
      const def =
        pydantic && withOrmTags
          ? c.nullable
            ? ` = Field(default=None${c.comment ? `, description="${c.comment.replace(/"/g, '\\"')}"` : ''})`
            : c.comment
              ? ` = Field(..., description="${c.comment.replace(/"/g, '\\"')}")`
              : ''
          : c.nullable
            ? ' = None'
            : '';
      const comment = withComments && c.comment && (!pydantic || !withOrmTags) ? `  # ${c.comment}` : '';
      return `    ${fName}: ${pyType}${def}${comment}`;
    });

    const clsHeader = pydantic
      ? `class ${className}(BaseModel):`
      : `@dataclass\nclass ${className}:`;
    return {
      filename: `${toSnakeCase(tableName)}.py`,
      code: `${imports.join('\n')}\n\n\n${clsHeader}\n${fields.join('\n')}\n`,
    };
  }

  // Rust
  const fields = columns.map((c) => {
    const k = classifySqlType(c.dataType);
    const base =
      k === 'bool'
        ? 'bool'
        : k === 'bigint'
          ? 'i64'
          : k === 'int'
            ? 'i32'
            : k === 'float'
              ? 'f64'
              : k === 'json'
                ? 'serde_json::Value'
                : k === 'blob'
                  ? 'Vec<u8>'
                  : 'String';
    const rsType = c.nullable && !c.isPrimaryKey ? `Option<${base}>` : base;
    const fName = toSnakeCase(c.name);
    const lines: string[] = [];
    if (withComments && c.comment) lines.push(`    /// ${c.comment}`);
    lines.push(`    pub ${fName}: ${rsType},`);
    return lines.join('\n');
  });
  const derives = withOrmTags
    ? `#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]\n${camelCase ? '#[serde(rename_all = "camelCase")]\n' : ''}`
    : '#[derive(Debug, Clone)]\n';
  return {
    filename: `${toSnakeCase(tableName)}.rs`,
    code: `${derives}pub struct ${className} {\n${fields.join('\n')}\n}\n`,
  };
}

export function generateMockInsertSql(
  tableName: string,
  dbName: string | undefined,
  columns: EntityColumnInput[],
  engineKey: string,
  rowCount = 10,
): string {
  if (columns.length === 0) return '';
  const targetCols = columns
    .filter((c) => !(c.isPrimaryKey && /\bint|\bbigint/i.test(c.dataType)))
    .map((c) => c);
  const activeCols = targetCols.length > 0 ? targetCols : columns;
  const rows: Array<Record<string, unknown>> = [];

  for (let i = 1; i <= rowCount; i += 1) {
    const row: Record<string, unknown> = {};
    for (const c of activeCols) {
      const nameLow = c.name.toLowerCase();
      const k = classifySqlType(c.dataType);
      if (nameLow.includes('email')) {
        row[c.name] = `user${i}@example.com`;
      } else if (nameLow.includes('phone') || nameLow.includes('mobile')) {
        row[c.name] = `1380000${String(1000 + i).slice(-4)}`;
      } else if (nameLow.includes('status') || nameLow.includes('state')) {
        row[c.name] = k === 'string' ? (i % 2 === 0 ? 'ACTIVE' : 'PENDING') : i % 2;
      } else if (k === 'bool') {
        row[c.name] = i % 2 === 1;
      } else if (k === 'bigint' || k === 'int') {
        row[c.name] = i * 10;
      } else if (k === 'decimal' || k === 'float') {
        row[c.name] = Number((i * 19.5).toFixed(2));
      } else if (k === 'date') {
        row[c.name] = `2026-01-${String((i % 28) + 1).padStart(2, '0')}`;
      } else if (k === 'datetime') {
        row[c.name] = `2026-01-${String((i % 28) + 1).padStart(2, '0')} 10:${String(i % 60).padStart(2, '0')}:00`;
      } else if (k === 'json') {
        row[c.name] = `{"seq":${i},"mock":true}`;
      } else {
        row[c.name] = `${c.name}_sample_${i}`;
      }
    }
    rows.push(row);
  }

  const targetColumnTypes: Record<string, string> = {};
  for (const c of activeCols) targetColumnTypes[c.name] = c.dataType;

  const stmts = buildBatchInsertStatements({
    tableName,
    dbName,
    engineKey,
    targetColumns: activeCols.map((c) => c.name),
    targetColumnTypes,
    rows,
    batchSize: rowCount,
    ignoreDuplicates: false,
  });
  return `-- 自动生成 ${rowCount} 行 Mock 测试数据 (${tableName})\n${stmts.join('\n')}\n`;
}

export interface CreateTableColumnDraft {
  name: string;
  dataType: string;
  length: string;
  nullable: boolean;
  primaryKey: boolean;
  autoIncrement: boolean;
  defaultValue: string;
  comment: string;
}

export interface CreateTableIndexDraft {
  name: string;
  columns: string;
  unique: boolean;
}

export interface CreateTableSpec {
  dbName?: string;
  tableName: string;
  comment?: string;
  columns: CreateTableColumnDraft[];
  indexes: CreateTableIndexDraft[];
}

export function generateCreateTableDdl(spec: CreateTableSpec, engineKey: string): string {
  const q = (id: string) => quoteIdentifier(id, engineKey);
  const rawTable = spec.tableName.trim() || 'new_table';
  const qualifiedTable =
    spec.dbName && spec.dbName !== 'main' && spec.dbName !== 'default'
      ? `${q(spec.dbName)}.${q(rawTable)}`
      : q(rawTable);
  const eng = (engineKey || 'mysql').toLowerCase();
  const isSqlite = eng.includes('sqlite');
  const isPgFamily = eng.includes('postgres') || eng.includes('kingbase') || eng.includes('gauss') || eng === 'pg';
  const isMysqlFamily =
    eng.includes('mysql') || eng.includes('tidb') || eng.includes('maria') || eng.includes('oceanbase');
  const isOracleOrDm = eng.includes('oracle') || eng.includes('dameng') || eng === 'dm';
  const isMsSql = eng.includes('sqlserver') || eng.includes('mssql');
  const isClickHouse = eng.includes('clickhouse');

  const pkCols = spec.columns.filter((c) => c.primaryKey && c.name.trim());
  const colDefs: string[] = [];

  for (const col of spec.columns) {
    const colName = col.name.trim();
    if (!colName) continue;
    let typeExpr = (col.dataType.trim() || 'VARCHAR').toUpperCase();
    if (col.length.trim() && !typeExpr.includes('(')) {
      typeExpr = `${typeExpr}(${col.length.trim()})`;
    }
    if (isPgFamily && col.autoIncrement) {
      typeExpr = typeExpr.includes('BIG') ? 'BIGSERIAL' : 'SERIAL';
    } else if (isSqlite && col.autoIncrement) {
      typeExpr = 'INTEGER';
    }

    const parts = [q(colName), typeExpr];
    if (!col.nullable || col.primaryKey) {
      if (!(isSqlite && col.autoIncrement && pkCols.length === 1)) {
        parts.push('NOT NULL');
      }
    }
    if (col.autoIncrement) {
      if (isMysqlFamily) parts.push('AUTO_INCREMENT');
      else if (isSqlite && pkCols.length === 1 && col.primaryKey) parts.push('PRIMARY KEY AUTOINCREMENT');
      else if (isMsSql) parts.push('IDENTITY(1,1)');
      else if (isOracleOrDm) parts.push('GENERATED BY DEFAULT AS IDENTITY');
    }
    if (col.defaultValue.trim() && !(isPgFamily && col.autoIncrement)) {
      const def = col.defaultValue.trim();
      const literal =
        /^(?:CURRENT_TIMESTAMP|NOW\(\)|NULL|TRUE|FALSE|-?\d+(?:\.\d+)?)$/i.test(def) || def.startsWith("'")
          ? def
          : `'${def.replace(/'/g, "''")}'`;
      parts.push(`DEFAULT ${literal}`);
    }
    if (isMysqlFamily && col.comment.trim()) {
      parts.push(`COMMENT '${col.comment.trim().replace(/'/g, "''")}'`);
    }
    colDefs.push(`  ${parts.join(' ')}`);
  }

  const sqliteInlinePk = isSqlite && pkCols.length === 1 && pkCols[0].autoIncrement;
  if (pkCols.length > 0 && !sqliteInlinePk && !isClickHouse) {
    colDefs.push(`  PRIMARY KEY (${pkCols.map((c) => q(c.name.trim())).join(', ')})`);
  }

  let footer = ';';
  if (isMysqlFamily) {
    const tblComment = spec.comment?.trim()
      ? ` COMMENT='${spec.comment.trim().replace(/'/g, "''")}'`
      : '';
    footer = ` ENGINE=InnoDB DEFAULT CHARSET=utf8mb4${tblComment};`;
  } else if (isClickHouse) {
    const orderCols = pkCols.length > 0 ? pkCols.map((c) => q(c.name.trim())).join(', ') : 'tuple()';
    footer = ` ENGINE = MergeTree() ORDER BY (${orderCols});`;
  }

  const statements = [`CREATE TABLE ${qualifiedTable} (\n${colDefs.join(',\n')}\n)${footer}`];

  if (isPgFamily || isOracleOrDm) {
    if (spec.comment?.trim()) {
      statements.push(
        `COMMENT ON TABLE ${qualifiedTable} IS '${spec.comment.trim().replace(/'/g, "''")}';`,
      );
    }
    for (const c of spec.columns) {
      if (c.name.trim() && c.comment.trim()) {
        statements.push(
          `COMMENT ON COLUMN ${qualifiedTable}.${q(c.name.trim())} IS '${c.comment.trim().replace(/'/g, "''")}';`,
        );
      }
    }
  }

  for (const idx of spec.indexes) {
    const idxName = idx.name.trim();
    const cols = idx.columns
      .split(',')
      .map((s) => s.trim())
      .filter(Boolean);
    if (!idxName || cols.length === 0) continue;
    const kw = idx.unique ? 'CREATE UNIQUE INDEX' : 'CREATE INDEX';
    statements.push(`${kw} ${q(idxName)} ON ${qualifiedTable} (${cols.map(q).join(', ')});`);
  }

  return statements.join('\n');
}

export function buildRenameTableSql(
  dbName: string | undefined,
  oldName: string,
  newName: string,
  engineKey: string,
): string {
  if (isRedisEngineKey(engineKey)) {
    return `RENAME ${quoteRedisArg(oldName)} ${quoteRedisArg(newName)}`;
  }
  const q = (id: string) => quoteIdentifier(id, engineKey);
  const hasDb = Boolean(dbName && dbName !== 'main' && dbName !== 'default');
  const fullOld = hasDb ? `${q(dbName!)}.${q(oldName)}` : q(oldName);
  const fullNew = hasDb ? `${q(dbName!)}.${q(newName)}` : q(newName);
  const eng = (engineKey || 'mysql').toLowerCase();
  if (eng.includes('mysql') || eng.includes('tidb') || eng.includes('maria') || eng.includes('oceanbase')) {
    return `RENAME TABLE ${fullOld} TO ${fullNew};`;
  }
  return `ALTER TABLE ${fullOld} RENAME TO ${q(newName)};`;
}

export function buildTruncateTableSql(
  dbName: string | undefined,
  tableName: string,
  engineKey: string,
): string {
  const q = (id: string) => quoteIdentifier(id, engineKey);
  const hasDb = Boolean(dbName && dbName !== 'main' && dbName !== 'default');
  const full = hasDb ? `${q(dbName!)}.${q(tableName)}` : q(tableName);
  if ((engineKey || '').toLowerCase().includes('sqlite')) {
    return `DELETE FROM ${full}`;
  }
  return `TRUNCATE TABLE ${full}`;
}

export function buildDropTableSql(
  dbName: string | undefined,
  tableName: string,
  engineKey: string,
): string {
  if (isRedisEngineKey(engineKey)) {
    return `DEL ${quoteRedisArg(tableName)}`;
  }
  const q = (id: string) => quoteIdentifier(id, engineKey);
  const hasDb = Boolean(dbName && dbName !== 'main' && dbName !== 'default');
  const full = hasDb ? `${q(dbName!)}.${q(tableName)}` : q(tableName);
  return `DROP TABLE ${full}`;
}

export function buildDropViewSql(
  dbName: string | undefined,
  viewName: string,
  engineKey: string,
  isMaterialized = false,
): string {
  const q = (id: string) => quoteIdentifier(id, engineKey);
  const hasDb = Boolean(dbName && dbName !== 'main' && dbName !== 'default');
  const full = hasDb ? `${q(dbName!)}.${q(viewName)}` : q(viewName);
  return isMaterialized ? `DROP MATERIALIZED VIEW IF EXISTS ${full}` : `DROP VIEW IF EXISTS ${full}`;
}

export function isViewTableType(tableType?: string | null): boolean {
  return (tableType ?? '').toUpperCase().includes('VIEW');
}

export interface DmlTargetInfo {
  action: 'UPDATE' | 'INSERT' | 'DELETE' | 'TRUNCATE' | 'ALTER';
  db?: string;
  table: string;
}

export function extractDmlTargetTable(sql: string): DmlTargetInfo | null {
  const cleaned = sql
    .replace(/--[^\n]*/g, ' ')
    .replace(/\/\*[\s\S]*?\*\//g, ' ')
    .trim();
  if (!cleaned) return null;

  const ident = '([`"\\[]?[A-Za-z0-9_$-]+[`"\\]]?)';
  const qualified = `(?:${ident}\\s*\\.\\s*)?${ident}`;
  const stripQuotes = (s: string) => s.replace(/^[`"\[]|[`"\]]$/g, '');

  const matchTarget = (
    regex: RegExp,
    action: DmlTargetInfo['action'],
  ): DmlTargetInfo | null => {
    const m = cleaned.match(regex);
    if (!m) return null;
    const db = m[1] ? stripQuotes(m[1]) : undefined;
    const table = stripQuotes(m[2]);
    if (!table) return null;
    return { action, db, table };
  };

  return (
    matchTarget(
      new RegExp(`^UPDATE\\s+(?:LOW_PRIORITY\\s+)?(?:IGNORE\\s+)?(?:ONLY\\s+)?${qualified}(?=\\s|$|;)`, 'i'),
      'UPDATE',
    ) ??
    matchTarget(
      new RegExp(
        `^(?:INSERT|REPLACE)\\s+(?:LOW_PRIORITY\\s+|DELAYED\\s+|HIGH_PRIORITY\\s+)?(?:IGNORE\\s+)?(?:INTO\\s+)?${qualified}(?=\\s|\\(|$|;)`,
        'i',
      ),
      'INSERT',
    ) ??
    matchTarget(
      new RegExp(
        `^DELETE\\s+(?:LOW_PRIORITY\\s+)?(?:QUICK\\s+)?(?:IGNORE\\s+)?FROM\\s+(?:ONLY\\s+)?${qualified}(?=\\s|$|;)`,
        'i',
      ),
      'DELETE',
    ) ??
    matchTarget(
      new RegExp(`^TRUNCATE\\s+(?:TABLE\\s+)?${qualified}(?=\\s|$|;)`, 'i'),
      'TRUNCATE',
    ) ??
    matchTarget(
      new RegExp(`^ALTER\\s+TABLE\\s+(?:IF\\s+EXISTS\\s+)?(?:ONLY\\s+)?${qualified}(?=\\s|$|;)`, 'i'),
      'ALTER',
    )
  );
}

export function formatViewMutationError(rawError: string, tableName?: string): string | null {
  const err = (rawError || '').trim();
  if (!err) return null;
  const viewErrPattern =
    /not updatable|target table .* of the (?:update|insert|delete) is not updatable|cannot (?:update|insert into|delete from) view|views that do not select from a single table|cannot modify .* because it is a view|view or function .* is not updatable|data manipulation operation not legal on this view|cannot modify a column which maps to a non key-preserved table|table .* is a view|error\s+1288/i;
  if (!viewErrPattern.test(err)) return null;
  return `「${tableName || '当前对象'}」是数据库视图，不能修改数据（${err}）`;
}

export function minifySqlText(sql: string): string {
  let out = '';
  let i = 0;
  const n = sql.length;
  let lastWasSpace = true;

  while (i < n) {
    const ch = sql[i];
    const next = sql[i + 1] ?? '';

    if (ch === '-' && next === '-') {
      while (i < n && sql[i] !== '\n') i += 1;
      continue;
    }
    if (ch === '/' && next === '*') {
      i += 2;
      while (i < n && !(sql[i] === '*' && sql[i + 1] === '/')) i += 1;
      i += 2;
      continue;
    }
    if (ch === "'" || ch === '"' || ch === '`') {
      const q = ch;
      const start = i;
      i += 1;
      while (i < n) {
        if (sql[i] === '\\' && q !== '"') {
          i += 2;
          continue;
        }
        if (sql[i] === q) {
          if (sql[i + 1] === q) {
            i += 2;
            continue;
          }
          i += 1;
          break;
        }
        i += 1;
      }
      out += sql.slice(start, i);
      lastWasSpace = false;
      continue;
    }
    if (/\s/.test(ch)) {
      if (!lastWasSpace) {
        out += ' ';
        lastWasSpace = true;
      }
      i += 1;
      continue;
    }
    out += ch;
    lastWasSpace = false;
    i += 1;
  }

  return out.trim();
}

export function escapeSqlAsCodeString(sql: string, style: 'java' | 'json'): string {
  const trimmed = sql.trim();
  if (style === 'json') {
    return JSON.stringify(trimmed);
  }
  const lines = trimmed.split(/\r?\n/);
  if (lines.length === 1) {
    return `"${lines[0].replace(/\\/g, '\\\\').replace(/"/g, '\\"')}"`;
  }
  return lines
    .map((line, idx) => {
      const esc = line.replace(/\\/g, '\\\\').replace(/"/g, '\\"');
      const suffix = idx < lines.length - 1 ? ' " +' : ';';
      return `"${esc}${suffix}`;
    })
    .join('\n');
}

// ==================== Redis Command & Key Helpers ====================

export function isRedisEngineKey(engineKey?: string | null): boolean {
  return (engineKey ?? '').toLowerCase().includes('redis');
}

export function quoteRedisArg(arg: string): string {
  if (!arg || /[\s"'\x00-\x1f]/.test(arg)) {
    return `"${arg.replace(/\\/g, '\\\\').replace(/"/g, '\\"')}"`;
  }
  return arg;
}

export function buildRedisInspectCommand(
  key: string,
  keyType?: string | null,
  limit = 100,
): string {
  const qk = quoteRedisArg(key);
  const stop = Math.max(0, Math.floor(limit) - 1);
  const kind = (keyType || '').toLowerCase();
  if (kind === 'hash') return `HGETALL ${qk}`;
  if (kind === 'list') return `LRANGE ${qk} 0 ${stop}`;
  if (kind === 'set') return `SMEMBERS ${qk}`;
  if (kind === 'zset') return `ZRANGE ${qk} 0 ${stop} WITHSCORES`;
  if (kind === 'stream') return `XRANGE ${qk} - + COUNT ${Math.max(1, Math.floor(limit))}`;
  return `GET ${qk}`;
}

export function buildRedisCountCommand(key: string, keyType?: string | null): string {
  const qk = quoteRedisArg(key);
  const kind = (keyType || '').toLowerCase();
  if (kind === 'hash') return `HLEN ${qk}`;
  if (kind === 'list') return `LLEN ${qk}`;
  if (kind === 'set') return `SCARD ${qk}`;
  if (kind === 'zset') return `ZCARD ${qk}`;
  if (kind === 'stream') return `XLEN ${qk}`;
  return `STRLEN ${qk}`;
}

function cleanRedisCommandLine(raw: string): string {
  const trimmed = raw.trim();
  if (!trimmed || trimmed.startsWith('--') || trimmed.startsWith('#') || trimmed.startsWith('//')) {
    return '';
  }
  let inSingle = false;
  let inDouble = false;
  let escaped = false;
  let trailingSemi = false;
  for (let i = 0; i < trimmed.length; i += 1) {
    const ch = trimmed[i];
    if (escaped) {
      escaped = false;
      trailingSemi = false;
      continue;
    }
    if ((inSingle || inDouble) && ch === '\\') {
      escaped = true;
      trailingSemi = false;
      continue;
    }
    if (ch === '"' && !inSingle) {
      inDouble = !inDouble;
      trailingSemi = false;
      continue;
    }
    if (ch === "'" && !inDouble) {
      inSingle = !inSingle;
      trailingSemi = false;
      continue;
    }
    trailingSemi = !inSingle && !inDouble && ch === ';';
  }
  return (trailingSemi ? trimmed.slice(0, -1) : trimmed).trim();
}

export function splitRedisCommands(content: string): string[] {
  const out: string[] = [];
  for (const line of content.split(/\r?\n/)) {
    const cmd = cleanRedisCommandLine(line);
    if (cmd) out.push(cmd);
  }
  return out;
}

export function extractRedisCommandAtLine(
  content: string,
  lineNumber1Based: number,
): string | null {
  const lines = content.split(/\r?\n/);
  if (lines.length === 0) return null;
  const idx = Math.max(0, Math.min(lines.length - 1, lineNumber1Based - 1));
  const cur = cleanRedisCommandLine(lines[idx]);
  if (cur) return cur;
  for (let i = idx - 1; i >= 0; i -= 1) {
    const prev = cleanRedisCommandLine(lines[i]);
    if (prev) return prev;
  }
  for (let i = idx + 1; i < lines.length; i += 1) {
    const next = cleanRedisCommandLine(lines[i]);
    if (next) return next;
  }
  return null;
}

// ==================== v0.9.6 Productivity & Schema Tools ====================

function isMysqlLikeEngine(engineKey: string): boolean {
  const eng = (engineKey || '').toLowerCase();
  return (
    eng.includes('mysql') ||
    eng.includes('maria') ||
    eng.includes('tidb') ||
    eng.includes('oceanbase')
  );
}

function isPgLikeEngine(engineKey: string): boolean {
  const eng = (engineKey || '').toLowerCase();
  return eng.includes('postgres') || eng.includes('kingbase') || eng.includes('gauss');
}

function isSqliteLikeEngine(engineKey: string): boolean {
  return (engineKey || '').toLowerCase().includes('sqlite');
}

function qualifyTargetTable(
  dbName: string | undefined,
  tableName: string,
  engineKey: string,
): string {
  const q = (id: string) => quoteIdentifier(id, engineKey);
  const cleanDb = (dbName ?? '').trim();
  const skipDb = !cleanDb || cleanDb === 'default' || (isSqliteLikeEngine(engineKey) && cleanDb === 'main');
  return skipDb ? q(tableName) : `${q(cleanDb)}.${q(tableName)}`;
}

function normalizeDefaultExpr(val?: string | null): string {
  return (val ?? '').trim().replace(/^'+|'+$/g, '');
}

function buildColumnSpecFragment(
  col: {
    name: string;
    dataType: string;
    nullable: boolean;
    defaultValue?: string | null;
    comment?: string | null;
  },
  engineKey: string,
): string {
  const q = (id: string) => quoteIdentifier(id, engineKey);
  const parts = [q(col.name), col.dataType.trim() || 'VARCHAR(255)'];
  parts.push(col.nullable ? 'NULL' : 'NOT NULL');
  const def = (col.defaultValue ?? '').trim();
  if (def !== '') {
    const upperDef = def.toUpperCase();
    const isKeywordOrNum =
      upperDef === 'NULL' ||
      upperDef === 'TRUE' ||
      upperDef === 'FALSE' ||
      upperDef.startsWith('CURRENT_TIMESTAMP') ||
      upperDef.startsWith('NOW(') ||
      /^-?\d+(\.\d+)?$/.test(def) ||
      (def.startsWith("'") && def.endsWith("'"));
    parts.push(`DEFAULT ${isKeywordOrNum ? def : formatSqlLiteral(def)}`);
  }
  if (isMysqlLikeEngine(engineKey) && col.comment?.trim()) {
    parts.push(`COMMENT ${formatSqlLiteral(col.comment.trim())}`);
  }
  return parts.join(' ');
}

function buildCreateIndexStatement(
  dbName: string | undefined,
  tableName: string,
  idx: { name: string; unique: boolean; columns: string[] },
  engineKey: string,
): string {
  const q = (id: string) => quoteIdentifier(id, engineKey);
  const fullTable = qualifyTargetTable(dbName, tableName, engineKey);
  const colList = idx.columns.map((c) => q(c)).join(', ');
  const uniq = idx.unique ? 'UNIQUE ' : '';
  return `CREATE ${uniq}INDEX ${q(idx.name)} ON ${fullTable} (${colList});`;
}

function buildDropIndexStatement(
  dbName: string | undefined,
  tableName: string,
  indexName: string,
  engineKey: string,
): string {
  const q = (id: string) => quoteIdentifier(id, engineKey);
  if (isMysqlLikeEngine(engineKey)) {
    const fullTable = qualifyTargetTable(dbName, tableName, engineKey);
    return `DROP INDEX ${q(indexName)} ON ${fullTable};`;
  }
  const cleanDb = (dbName ?? '').trim();
  const skipDb = !cleanDb || cleanDb === 'default' || (isSqliteLikeEngine(engineKey) && cleanDb === 'main');
  return skipDb ? `DROP INDEX ${q(indexName)};` : `DROP INDEX ${q(cleanDb)}.${q(indexName)};`;
}

// ---- 1. Visual Table Schema Diff & Sync DDL ----

export type SchemaDiffStatus = 'same' | 'added' | 'removed' | 'modified';

export interface SchemaDiffColumnItem {
  name: string;
  status: SchemaDiffStatus;
  source?: ColumnMeta;
  target?: ColumnMeta;
  changes: string[];
}

export interface SchemaDiffIndexItem {
  name: string;
  status: SchemaDiffStatus;
  source?: IndexMeta;
  target?: IndexMeta;
  changes: string[];
}

export interface TableSchemaDiffResult {
  columnDiffs: SchemaDiffColumnItem[];
  indexDiffs: SchemaDiffIndexItem[];
  syncStatements: string[];
  warnings: string[];
  stats: {
    added: number;
    removed: number;
    modified: number;
    same: number;
    indexChanges: number;
  };
}

export function computeTableSchemaDiff(opts: {
  sourceDb?: string;
  sourceTable: string;
  sourceColumns: ColumnMeta[];
  sourceIndexes: IndexMeta[];
  targetDb?: string;
  targetTable: string;
  targetColumns: ColumnMeta[];
  targetIndexes: IndexMeta[];
  targetEngineKey: string;
}): TableSchemaDiffResult {
  const engineKey = opts.targetEngineKey || 'mysql';
  const fullTarget = qualifyTargetTable(opts.targetDb, opts.targetTable, engineKey);
  const q = (id: string) => quoteIdentifier(id, engineKey);

  const targetColMap = new Map<string, ColumnMeta>();
  for (const col of opts.targetColumns) {
    targetColMap.set(col.name.toLowerCase(), col);
  }
  const seenTargetCols = new Set<string>();

  const columnDiffs: SchemaDiffColumnItem[] = [];
  const syncStatements: string[] = [];
  const warnings: string[] = [];

  let added = 0;
  let removed = 0;
  let modified = 0;
  let same = 0;

  for (const src of opts.sourceColumns) {
    const key = src.name.toLowerCase();
    const tgt = targetColMap.get(key);
    if (!tgt) {
      added += 1;
      columnDiffs.push({
        name: src.name,
        status: 'added',
        source: src,
        changes: [`目标表缺少字段 (${src.dataType})`],
      });
      syncStatements.push(
        `ALTER TABLE ${fullTarget} ADD COLUMN ${buildColumnSpecFragment(src, engineKey)};`,
      );
      continue;
    }

    seenTargetCols.add(key);
    const changes: string[] = [];
    if (src.dataType.trim().toLowerCase() !== tgt.dataType.trim().toLowerCase()) {
      changes.push(`类型: ${tgt.dataType} -> ${src.dataType}`);
    }
    if (src.nullable !== tgt.nullable) {
      changes.push(`可空: ${tgt.nullable ? 'NULL' : 'NOT NULL'} -> ${src.nullable ? 'NULL' : 'NOT NULL'}`);
    }
    if (normalizeDefaultExpr(src.defaultValue) !== normalizeDefaultExpr(tgt.defaultValue)) {
      changes.push(`默认值: ${tgt.defaultValue ?? '无'} -> ${src.defaultValue ?? '无'}`);
    }
    if ((src.comment ?? '').trim() !== (tgt.comment ?? '').trim()) {
      changes.push(`注释变更`);
    }

    if (changes.length === 0) {
      same += 1;
      columnDiffs.push({ name: src.name, status: 'same', source: src, target: tgt, changes: [] });
    } else {
      modified += 1;
      columnDiffs.push({ name: src.name, status: 'modified', source: src, target: tgt, changes });
      if (isMysqlLikeEngine(engineKey)) {
        syncStatements.push(
          `ALTER TABLE ${fullTarget} MODIFY COLUMN ${buildColumnSpecFragment(src, engineKey)};`,
        );
      } else if (isPgLikeEngine(engineKey)) {
        if (src.dataType.trim().toLowerCase() !== tgt.dataType.trim().toLowerCase()) {
          syncStatements.push(
            `ALTER TABLE ${fullTarget} ALTER COLUMN ${q(src.name)} TYPE ${src.dataType};`,
          );
        }
        if (src.nullable !== tgt.nullable) {
          syncStatements.push(
            `ALTER TABLE ${fullTarget} ALTER COLUMN ${q(src.name)} ${src.nullable ? 'DROP NOT NULL' : 'SET NOT NULL'};`,
          );
        }
        if (normalizeDefaultExpr(src.defaultValue) !== normalizeDefaultExpr(tgt.defaultValue)) {
          const def = (src.defaultValue ?? '').trim();
          syncStatements.push(
            def
              ? `ALTER TABLE ${fullTarget} ALTER COLUMN ${q(src.name)} SET DEFAULT ${def};`
              : `ALTER TABLE ${fullTarget} ALTER COLUMN ${q(src.name)} DROP DEFAULT;`,
          );
        }
      } else {
        warnings.push(`字段 ${src.name} 存在差异，当前方言 (${engineKey}) 建议通过重建表迁移修改列定义。`);
        syncStatements.push(
          `-- [${engineKey}] 字段 ${src.name} 差异 (${changes.join('; ')}) 请按需手动调整`,
        );
      }
    }
  }

  for (const tgt of opts.targetColumns) {
    if (!seenTargetCols.has(tgt.name.toLowerCase())) {
      removed += 1;
      columnDiffs.push({
        name: tgt.name,
        status: 'removed',
        target: tgt,
        changes: ['源表不存在此字段（同步将删除该列）'],
      });
      warnings.push(`⚠️ 删除目标字段 ${tgt.name} 属于破坏性变更，请确认该列数据不再需要。`);
      syncStatements.push(`ALTER TABLE ${fullTarget} DROP COLUMN ${q(tgt.name)};`);
    }
  }

  const isPrimaryIdx = (idx: IndexMeta) =>
    idx.name.toUpperCase() === 'PRIMARY' || idx.indexType?.toUpperCase() === 'PRIMARY';
  const srcIndexes = opts.sourceIndexes.filter((i) => !isPrimaryIdx(i));
  const tgtIndexes = opts.targetIndexes.filter((i) => !isPrimaryIdx(i));
  const tgtIdxMap = new Map<string, IndexMeta>();
  for (const idx of tgtIndexes) {
    tgtIdxMap.set(idx.name.toLowerCase(), idx);
  }
  const seenTgtIdx = new Set<string>();
  const indexDiffs: SchemaDiffIndexItem[] = [];
  let indexChanges = 0;

  for (const srcIdx of srcIndexes) {
    const key = srcIdx.name.toLowerCase();
    const tgtIdx = tgtIdxMap.get(key);
    if (!tgtIdx) {
      indexChanges += 1;
      indexDiffs.push({
        name: srcIdx.name,
        status: 'added',
        source: srcIdx,
        changes: [`新增${srcIdx.unique ? '唯一' : '普通'}索引 (${srcIdx.columns.join(', ')})`],
      });
      syncStatements.push(buildCreateIndexStatement(opts.targetDb, opts.targetTable, srcIdx, engineKey));
      continue;
    }
    seenTgtIdx.add(key);
    const changes: string[] = [];
    if (srcIdx.unique !== tgtIdx.unique) {
      changes.push(`唯一性: ${tgtIdx.unique ? 'UNIQUE' : 'NORMAL'} -> ${srcIdx.unique ? 'UNIQUE' : 'NORMAL'}`);
    }
    if (srcIdx.columns.join(',').toLowerCase() !== tgtIdx.columns.join(',').toLowerCase()) {
      changes.push(`索引列: (${tgtIdx.columns.join(', ')}) -> (${srcIdx.columns.join(', ')})`);
    }
    if (changes.length === 0) {
      indexDiffs.push({ name: srcIdx.name, status: 'same', source: srcIdx, target: tgtIdx, changes: [] });
    } else {
      indexChanges += 1;
      indexDiffs.push({ name: srcIdx.name, status: 'modified', source: srcIdx, target: tgtIdx, changes });
      syncStatements.push(buildDropIndexStatement(opts.targetDb, opts.targetTable, srcIdx.name, engineKey));
      syncStatements.push(buildCreateIndexStatement(opts.targetDb, opts.targetTable, srcIdx, engineKey));
    }
  }

  for (const tgtIdx of tgtIndexes) {
    if (!seenTgtIdx.has(tgtIdx.name.toLowerCase())) {
      indexChanges += 1;
      indexDiffs.push({
        name: tgtIdx.name,
        status: 'removed',
        target: tgtIdx,
        changes: ['源表无此索引（同步将删除）'],
      });
      syncStatements.push(buildDropIndexStatement(opts.targetDb, opts.targetTable, tgtIdx.name, engineKey));
    }
  }

  return {
    columnDiffs,
    indexDiffs,
    syncStatements,
    warnings,
    stats: { added, removed, modified, same, indexChanges },
  };
}

// ---- 2. Visual Alter Table DDL Builder ----

export interface AlterColumnDraft {
  originalName?: string;
  name: string;
  dataType: string;
  nullable: boolean;
  defaultValue: string;
  comment: string;
  isNew?: boolean;
  deleted?: boolean;
}

export interface AlterIndexDraft {
  originalName?: string;
  name: string;
  unique: boolean;
  columns: string;
  isNew?: boolean;
  deleted?: boolean;
}

export function buildAlterTableDdl(opts: {
  dbName?: string;
  tableName: string;
  originalColumns: ColumnMeta[];
  draftColumns: AlterColumnDraft[];
  originalIndexes: IndexMeta[];
  draftIndexes: AlterIndexDraft[];
  engineKey: string;
}): { statements: string[]; warnings: string[] } {
  const engineKey = opts.engineKey || 'mysql';
  const fullTable = qualifyTargetTable(opts.dbName, opts.tableName, engineKey);
  const q = (id: string) => quoteIdentifier(id, engineKey);

  const origColMap = new Map<string, ColumnMeta>();
  for (const col of opts.originalColumns) {
    origColMap.set(col.name, col);
  }

  const statements: string[] = [];
  const warnings: string[] = [];

  for (const draft of opts.draftColumns) {
    const cleanName = draft.name.trim();
    if (draft.isNew) {
      if (draft.deleted || !cleanName) continue;
      statements.push(
        `ALTER TABLE ${fullTable} ADD COLUMN ${buildColumnSpecFragment(
          {
            name: cleanName,
            dataType: draft.dataType,
            nullable: draft.nullable,
            defaultValue: draft.defaultValue,
            comment: draft.comment,
          },
          engineKey,
        )};`,
      );
      continue;
    }

    const origKey = draft.originalName ?? cleanName;
    const orig = origColMap.get(origKey);
    if (!orig) continue;

    if (draft.deleted) {
      warnings.push(`⚠️ 将永久删除字段 ${orig.name} 及其历史数据。`);
      statements.push(`ALTER TABLE ${fullTable} DROP COLUMN ${q(orig.name)};`);
      continue;
    }

    const renamed = Boolean(cleanName && cleanName !== orig.name);
    const typeChanged = draft.dataType.trim().toLowerCase() !== orig.dataType.trim().toLowerCase();
    const nullChanged = draft.nullable !== orig.nullable;
    const defChanged = normalizeDefaultExpr(draft.defaultValue) !== normalizeDefaultExpr(orig.defaultValue);
    const commentChanged = draft.comment.trim() !== (orig.comment ?? '').trim();

    if (!renamed && !typeChanged && !nullChanged && !defChanged && !commentChanged) {
      continue;
    }

    if (isMysqlLikeEngine(engineKey)) {
      const spec = buildColumnSpecFragment(
        {
          name: cleanName || orig.name,
          dataType: draft.dataType,
          nullable: draft.nullable,
          defaultValue: draft.defaultValue,
          comment: draft.comment,
        },
        engineKey,
      );
      if (renamed) {
        statements.push(`ALTER TABLE ${fullTable} CHANGE COLUMN ${q(orig.name)} ${spec};`);
      } else {
        statements.push(`ALTER TABLE ${fullTable} MODIFY COLUMN ${spec};`);
      }
    } else if (isPgLikeEngine(engineKey)) {
      let curCol = orig.name;
      if (renamed) {
        statements.push(`ALTER TABLE ${fullTable} RENAME COLUMN ${q(orig.name)} TO ${q(cleanName)};`);
        curCol = cleanName;
      }
      if (typeChanged) {
        statements.push(`ALTER TABLE ${fullTable} ALTER COLUMN ${q(curCol)} TYPE ${draft.dataType.trim()};`);
      }
      if (nullChanged) {
        statements.push(
          `ALTER TABLE ${fullTable} ALTER COLUMN ${q(curCol)} ${draft.nullable ? 'DROP NOT NULL' : 'SET NOT NULL'};`,
        );
      }
      if (defChanged) {
        const def = draft.defaultValue.trim();
        statements.push(
          def
            ? `ALTER TABLE ${fullTable} ALTER COLUMN ${q(curCol)} SET DEFAULT ${def};`
            : `ALTER TABLE ${fullTable} ALTER COLUMN ${q(curCol)} DROP DEFAULT;`,
        );
      }
      if (commentChanged) {
        statements.push(
          `COMMENT ON COLUMN ${fullTable}.${q(curCol)} IS ${formatSqlLiteral(draft.comment.trim())};`,
        );
      }
    } else {
      if (renamed && !typeChanged && !nullChanged && !defChanged) {
        statements.push(`ALTER TABLE ${fullTable} RENAME COLUMN ${q(orig.name)} TO ${q(cleanName)};`);
      } else {
        warnings.push(`字段 ${orig.name} 的类型/约束更改需注意当前引擎 (${engineKey}) 语法兼容性。`);
        statements.push(
          `ALTER TABLE ${fullTable} ALTER COLUMN ${q(cleanName || orig.name)} TYPE ${draft.dataType.trim()};`,
        );
      }
    }
  }

  for (const idxDraft of opts.draftIndexes) {
    const cleanIdxName = idxDraft.name.trim();
    const cols = idxDraft.columns
      .split(',')
      .map((s) => s.trim())
      .filter(Boolean);

    if (idxDraft.isNew) {
      if (idxDraft.deleted || !cleanIdxName || cols.length === 0) continue;
      statements.push(
        buildCreateIndexStatement(
          opts.dbName,
          opts.tableName,
          { name: cleanIdxName, unique: idxDraft.unique, columns: cols },
          engineKey,
        ),
      );
      continue;
    }

    const origIdxName = idxDraft.originalName ?? cleanIdxName;
    if (idxDraft.deleted && origIdxName) {
      statements.push(buildDropIndexStatement(opts.dbName, opts.tableName, origIdxName, engineKey));
    }
  }

  return { statements, warnings };
}

// ---- 3. Interactive ER Diagram Relationship Inference & Layout ----

export interface ErTableInput {
  tableName: string;
  comment?: string | null;
  estimatedRows?: number | null;
  columns: ColumnMeta[];
}

export interface ErNodeLayout {
  tableName: string;
  comment?: string | null;
  estimatedRows?: number | null;
  columns: ColumnMeta[];
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface ErEdge {
  id: string;
  fromTable: string;
  fromColumn: string;
  toTable: string;
  toColumn: string;
  inferredReason: string;
}

export interface ErDiagramGraph {
  nodes: ErNodeLayout[];
  edges: ErEdge[];
}

function singularizeToken(stem: string): string[] {
  const lower = stem.toLowerCase();
  const variants = new Set<string>([lower]);
  if (lower.endsWith('ies') && lower.length > 3) {
    variants.add(`${lower.slice(0, -3)}y`);
  }
  if (lower.endsWith('es') && lower.length > 3) {
    variants.add(lower.slice(0, -2));
  }
  if (lower.endsWith('s') && lower.length > 2) {
    variants.add(lower.slice(0, -1));
  }
  return Array.from(variants);
}

export function inferErRelationships(tables: ErTableInput[]): ErDiagramGraph {
  const tableLookup = new Map<string, ErTableInput>();
  const stemToTable = new Map<string, string>();

  for (const tbl of tables) {
    tableLookup.set(tbl.tableName.toLowerCase(), tbl);
    const bareName = tbl.tableName.includes('_')
      ? tbl.tableName.replace(/^(t|tb|tbl|sys|biz|app)_/i, '')
      : tbl.tableName;
    for (const v of [...singularizeToken(tbl.tableName), ...singularizeToken(bareName)]) {
      if (!stemToTable.has(v)) {
        stemToTable.set(v, tbl.tableName);
      }
    }
  }

  const edges: ErEdge[] = [];
  const edgeKeys = new Set<string>();

  for (const tbl of tables) {
    for (const col of tbl.columns) {
      const colLower = col.name.toLowerCase();
      let stem: string | null = null;
      if (colLower === 'parent_id') {
        stem = tbl.tableName.toLowerCase();
      } else if (colLower.endsWith('_id') && colLower.length > 3) {
        stem = colLower.slice(0, -3);
      } else if (colLower.endsWith('_uuid') && colLower.length > 5) {
        stem = colLower.slice(0, -5);
      }
      if (!stem) continue;

      let targetTableName: string | undefined;
      for (const cand of singularizeToken(stem)) {
        targetTableName = stemToTable.get(cand);
        if (targetTableName) break;
      }
      if (!targetTableName) continue;
      if (targetTableName === tbl.tableName && colLower !== 'parent_id') continue;

      const targetTbl = tableLookup.get(targetTableName.toLowerCase());
      if (!targetTbl) continue;
      const pkCol =
        targetTbl.columns.find((c) => c.isPrimaryKey)?.name ??
        targetTbl.columns.find((c) => c.name.toLowerCase() === 'id')?.name ??
        targetTbl.columns[0]?.name;
      if (!pkCol) continue;

      const edgeKey = `${tbl.tableName}.${col.name}->${targetTableName}.${pkCol}`;
      if (edgeKeys.has(edgeKey)) continue;
      edgeKeys.add(edgeKey);
      edges.push({
        id: edgeKey,
        fromTable: tbl.tableName,
        fromColumn: col.name,
        toTable: targetTableName,
        toColumn: pkCol,
        inferredReason: `${col.name} -> ${targetTableName}.${pkCol}`,
      });
    }
  }

  const cardWidth = 250;
  const colGap = 72;
  const rowGap = 48;
  const colsPerRow = tables.length <= 4 ? 2 : tables.length <= 9 ? 3 : 4;
  const startX = 48;
  const startY = 48;

  const nodes: ErNodeLayout[] = [];
  let rowMaxHeight = 0;
  let curY = startY;

  for (let i = 0; i < tables.length; i += 1) {
    const tbl = tables[i];
    const gridCol = i % colsPerRow;
    if (gridCol === 0 && i > 0) {
      curY += rowMaxHeight + rowGap;
      rowMaxHeight = 0;
    }
    const visibleColCount = Math.min(Math.max(tbl.columns.length, 1), 10);
    const height = 38 + visibleColCount * 22 + (tbl.columns.length > 10 ? 22 : 10);
    rowMaxHeight = Math.max(rowMaxHeight, height);

    nodes.push({
      tableName: tbl.tableName,
      comment: tbl.comment,
      estimatedRows: tbl.estimatedRows,
      columns: tbl.columns,
      x: startX + gridCol * (cardWidth + colGap),
      y: curY,
      width: cardWidth,
      height,
    });
  }

  return { nodes, edges };
}

// ---- 4. Redis Visual Key-Value & TTL Command Builder ----

export type RedisEditableType = 'string' | 'hash' | 'list' | 'set' | 'zset';

export interface RedisKeyEditorDraft {
  originalKey?: string;
  key: string;
  keyType: RedisEditableType;
  stringValue?: string;
  hashEntries?: Array<{ field: string; value: string }>;
  listItems?: string[];
  setMembers?: string[];
  zsetMembers?: Array<{ member: string; score: string }>;
  ttlSeconds: number | null;
  isNew?: boolean;
}

export function buildRedisSaveCommands(draft: RedisKeyEditorDraft): string[] {
  const key = draft.key.trim();
  if (!key) return [];
  const qk = quoteRedisArg(key);
  const commands: string[] = [];
  const orig = draft.originalKey?.trim();

  if (!draft.isNew && orig && orig !== key) {
    commands.push(`RENAME ${quoteRedisArg(orig)} ${qk}`);
  }

  if (draft.keyType === 'string') {
    commands.push(`SET ${qk} ${quoteRedisArg(draft.stringValue ?? '')}`);
  } else {
    if (!draft.isNew) {
      commands.push(`DEL ${qk}`);
    }
    if (draft.keyType === 'hash') {
      const valid = (draft.hashEntries ?? []).filter((e) => e.field.trim() !== '');
      if (valid.length > 0) {
        const args = valid.flatMap((e) => [quoteRedisArg(e.field), quoteRedisArg(e.value)]);
        commands.push(`HSET ${qk} ${args.join(' ')}`);
      }
    } else if (draft.keyType === 'list') {
      const valid = (draft.listItems ?? []).filter((v) => v !== '');
      if (valid.length > 0) {
        commands.push(`RPUSH ${qk} ${valid.map((v) => quoteRedisArg(v)).join(' ')}`);
      }
    } else if (draft.keyType === 'set') {
      const valid = (draft.setMembers ?? []).filter((v) => v !== '');
      if (valid.length > 0) {
        commands.push(`SADD ${qk} ${valid.map((v) => quoteRedisArg(v)).join(' ')}`);
      }
    } else if (draft.keyType === 'zset') {
      const valid = (draft.zsetMembers ?? []).filter((e) => e.member.trim() !== '');
      if (valid.length > 0) {
        const args = valid.flatMap((e) => {
          const scoreNum = Number(e.score);
          const safeScore = Number.isFinite(scoreNum) ? String(scoreNum) : '0';
          return [safeScore, quoteRedisArg(e.member)];
        });
        commands.push(`ZADD ${qk} ${args.join(' ')}`);
      }
    }
  }

  if (typeof draft.ttlSeconds === 'number' && Number.isFinite(draft.ttlSeconds)) {
    const ttl = Math.floor(draft.ttlSeconds);
    if (ttl > 0) {
      commands.push(`EXPIRE ${qk} ${ttl}`);
    } else if (ttl === -1 && !draft.isNew) {
      commands.push(`PERSIST ${qk}`);
    }
  }

  return commands;
}

// ---- 5. ResultGrid Quick Chart Dataset Helpers ----

export type QuickChartType = 'bar' | 'line' | 'pie';

export interface ChartDataPoint {
  label: string;
  value: number;
}

export interface ChartDatasetResult {
  points: ChartDataPoint[];
  min: number;
  max: number;
  sum: number;
  totalRows: number;
}

function parseChartNumeric(raw: unknown): number | null {
  if (typeof raw === 'number') {
    return Number.isFinite(raw) ? raw : null;
  }
  if (typeof raw === 'string') {
    const trimmed = raw.trim();
    if (!trimmed) return null;
    if (/^-?\d+(\.\d+)?$/.test(trimmed)) {
      const num = Number(trimmed);
      return Number.isFinite(num) ? num : null;
    }
  }
  return null;
}

export function detectChartColumns(
  columns: string[],
  rows: Array<Record<string, unknown>>,
): {
  labelCandidates: string[];
  numericColumns: string[];
  defaultLabelCol: string;
  defaultValueCol: string;
} {
  const sample = rows.slice(0, 100);
  const numericColumns: string[] = [];
  const labelCandidates: string[] = [];

  for (const col of columns) {
    let nonNull = 0;
    let numericCount = 0;
    for (const row of sample) {
      const v = row[col];
      if (v === null || v === undefined || v === '') continue;
      nonNull += 1;
      if (parseChartNumeric(v) !== null) numericCount += 1;
    }
    if (nonNull > 0 && numericCount === nonNull) {
      numericColumns.push(col);
    } else {
      labelCandidates.push(col);
    }
  }

  const preferredValue =
    numericColumns.find((c) => !/^id$/i.test(c) && !/_id$/i.test(c)) ?? numericColumns[0] ?? '';
  const defaultLabelCol =
    labelCandidates[0] ?? columns.find((c) => c !== preferredValue) ?? columns[0] ?? '';

  return {
    labelCandidates: labelCandidates.length > 0 ? labelCandidates : columns,
    numericColumns,
    defaultLabelCol,
    defaultValueCol: preferredValue,
  };
}

export function buildChartDataFromRows(
  rows: Array<Record<string, unknown>>,
  labelCol: string,
  valueCol: string,
  maxPoints = 30,
): ChartDatasetResult {
  const cap = Math.max(1, maxPoints);
  const slice = rows.slice(0, cap);
  const points: ChartDataPoint[] = [];
  let min = Number.POSITIVE_INFINITY;
  let max = Number.NEGATIVE_INFINITY;
  let sum = 0;

  for (let i = 0; i < slice.length; i += 1) {
    const row = slice[i];
    const num = parseChartNumeric(row[valueCol]);
    if (num === null) continue;
    const rawLabel = row[labelCol];
    const label =
      rawLabel === null || rawLabel === undefined || rawLabel === ''
        ? `#${i + 1}`
        : String(rawLabel);
    points.push({ label, value: num });
    if (num < min) min = num;
    if (num > max) max = num;
    sum += num;
  }

  if (points.length === 0) {
    return { points: [], min: 0, max: 0, sum: 0, totalRows: rows.length };
  }
  return { points, min, max, sum, totalRows: rows.length };
}

export function isConnectionProfileReadOnly(prof: ConnectionProfile | undefined | null): boolean {
  if (!prof) return false;
  const cfg = (prof.mode?.config ?? {}) as Partial<DirectConfig>;
  return Boolean(cfg.readOnly) || !prof.capabilities.canDml;
}

// ==================== 2D Virtualized Result Grid Layout Helpers ====================

export interface VirtualRange {
  start: number;
  end: number;
  beforeSize: number;
  afterSize: number;
  totalSize: number;
}

/**
 * Estimates fixed pixel column widths from column names and the first 25 sampled rows
 * so wide tables (70–300+ columns) render deterministically with `table-layout: fixed`.
 */
export function computeGridColumnWidths(
  columns: string[],
  rows: Array<Record<string, unknown>>,
  overrides?: Record<string, number>,
): number[] {
  const sample = rows.slice(0, 25);
  return columns.map((col) => {
    const manual = overrides?.[col];
    if (typeof manual === 'number' && Number.isFinite(manual) && manual > 0) {
      return Math.round(manual);
    }
    let maxChars = Math.min(36, col.length + 3);
    for (let i = 0; i < sample.length; i += 1) {
      const val = sample[i]?.[col];
      if (val === null || val === undefined) {
        if (maxChars < 6) maxChars = 6;
        continue;
      }
      const len =
        typeof val === 'object' ? Math.min(44, JSON.stringify(val).length) : String(val).length;
      if (len > maxChars) {
        maxChars = Math.min(44, len);
      }
      if (maxChars >= 44) break;
    }
    return Math.max(88, Math.min(280, Math.round(maxChars * 7.6 + 28)));
  });
}

export function computeFixedVirtualRange(
  scrollOffset: number,
  viewportSize: number,
  itemSize: number,
  totalCount: number,
  overscan = 8,
): VirtualRange {
  if (totalCount <= 0 || itemSize <= 0) {
    return { start: 0, end: 0, beforeSize: 0, afterSize: 0, totalSize: 0 };
  }
  const totalSize = totalCount * itemSize;
  const safeOffset = Math.max(0, scrollOffset);
  const safeViewport = Math.max(itemSize, viewportSize);
  const firstVisible = Math.floor(safeOffset / itemSize);
  const lastVisible = Math.ceil((safeOffset + safeViewport) / itemSize);
  const start = Math.max(0, Math.min(totalCount - 1, firstVisible - overscan));
  const end = Math.max(start + 1, Math.min(totalCount, lastVisible + overscan));
  return {
    start,
    end,
    beforeSize: start * itemSize,
    afterSize: Math.max(0, (totalCount - end) * itemSize),
    totalSize,
  };
}

export function computeVariableVirtualRange(
  scrollOffset: number,
  viewportSize: number,
  sizes: number[],
  overscan = 5,
): VirtualRange {
  const totalCount = sizes.length;
  if (totalCount === 0) {
    return { start: 0, end: 0, beforeSize: 0, afterSize: 0, totalSize: 0 };
  }
  const offsets = new Array<number>(totalCount + 1);
  offsets[0] = 0;
  for (let i = 0; i < totalCount; i += 1) {
    offsets[i + 1] = offsets[i] + Math.max(1, sizes[i] ?? 120);
  }
  const totalSize = offsets[totalCount];
  const safeStartPx = Math.max(0, scrollOffset);
  const safeEndPx = Math.max(safeStartPx + 1, safeStartPx + Math.max(1, viewportSize));

  // Binary search first index whose right edge exceeds safeStartPx
  let lo = 0;
  let hi = totalCount - 1;
  while (lo < hi) {
    const mid = (lo + hi) >> 1;
    if (offsets[mid + 1] <= safeStartPx) {
      lo = mid + 1;
    } else {
      hi = mid;
    }
  }
  const firstVisible = lo;

  // Binary search last index whose left edge is before safeEndPx
  lo = firstVisible;
  hi = totalCount - 1;
  while (lo < hi) {
    const mid = (lo + hi + 1) >> 1;
    if (offsets[mid] < safeEndPx) {
      lo = mid;
    } else {
      hi = mid - 1;
    }
  }
  const lastVisible = lo;

  const start = Math.max(0, firstVisible - overscan);
  const end = Math.min(totalCount, lastVisible + overscan + 1);
  const beforeSize = offsets[start];
  const afterSize = Math.max(0, totalSize - offsets[end]);

  return { start, end, beforeSize, afterSize, totalSize };
}


