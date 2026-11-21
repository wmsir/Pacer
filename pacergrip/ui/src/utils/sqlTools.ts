import type { MarkdownSqlBlock } from '../api/ipc';

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
  | { kind: 'paragraph'; text: string }
  | { kind: 'sql-code'; block: MarkdownSqlBlock }
  | { kind: 'code'; language: string; text: string };

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
      !lines[i].trimStart().startsWith('~~~')
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

  const usablePks = opts.primaryKeys.filter((pk) => opts.columns.includes(pk));
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
  if (e.includes('tidb')) {
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
