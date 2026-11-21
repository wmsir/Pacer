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
): string {
  const safeTable = tableName || 'target_table';
  const colList = columns.join(', ');
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

  const stripQuotes = (s: string) => s.replace(/^[`"\[]|[`"\]]$/g, '');
  const db = fromMatch[1] ? stripQuotes(fromMatch[1]) : undefined;
  const table = stripQuotes(fromMatch[2]);
  if (!table || /^(select|where|group|order|limit|having|values)$/i.test(table)) {
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
