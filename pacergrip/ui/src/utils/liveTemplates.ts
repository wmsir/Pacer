import { reactive } from 'vue';
import { quoteIdentifier } from './sqlTools';

export type LiveTemplateCategory = 'query' | 'dml' | 'ddl' | 'clause' | 'custom';

export interface SqlLiveTemplate {
  id: string;
  abbrev: string;
  description: string;
  category: LiveTemplateCategory;
  template: string;
  enabled: boolean;
  builtin?: boolean;
}

export const LIVE_TEMPLATE_CATEGORY_LABELS: Record<
  LiveTemplateCategory,
  { zh: string; en: string }
> = {
  query: { zh: '查询 (Query)', en: 'Query' },
  dml: { zh: '增删改 (DML)', en: 'DML' },
  ddl: { zh: '建表 DDL', en: 'DDL' },
  clause: { zh: '子句片段', en: 'Clause' },
  custom: { zh: '自定义', en: 'Custom' },
};

export const BUILTIN_LIVE_TEMPLATES: SqlLiveTemplate[] = [
  {
    id: 'lt-sel',
    abbrev: 'sel',
    description: '查询全表记录 SELECT * FROM table',
    category: 'query',
    template: 'SELECT * FROM ${1:table};${0}',
    enabled: true,
    builtin: true,
  },
  {
    id: 'lt-sel100',
    abbrev: 'sel100',
    description: '查询表前 100 行',
    category: 'query',
    template: 'SELECT * FROM ${1:table} LIMIT 100;${0}',
    enabled: true,
    builtin: true,
  },
  {
    id: 'lt-selc',
    abbrev: 'selc',
    description: '统计表总行数 SELECT COUNT(*)',
    category: 'query',
    template: 'SELECT COUNT(*) FROM ${1:table};${0}',
    enabled: true,
    builtin: true,
  },
  {
    id: 'lt-selw',
    abbrev: 'selw',
    description: '带 WHERE 条件查询表记录',
    category: 'query',
    template: 'SELECT * FROM ${1:table} WHERE ${2:column} = ${3:value} LIMIT 100;${0}',
    enabled: true,
    builtin: true,
  },
  {
    id: 'lt-selwc',
    abbrev: 'selwc',
    description: '带 WHERE 条件统计行数',
    category: 'query',
    template: 'SELECT COUNT(*) FROM ${1:table} WHERE ${2:column} = ${3:value};${0}',
    enabled: true,
    builtin: true,
  },
  {
    id: 'lt-selo',
    abbrev: 'selo',
    description: '按指定列倒序查询最新记录',
    category: 'query',
    template: 'SELECT * FROM ${1:table} ORDER BY ${2:column} DESC LIMIT 100;${0}',
    enabled: true,
    builtin: true,
  },
  {
    id: 'lt-selg',
    abbrev: 'selg',
    description: '按字段分组聚合计数排行榜',
    category: 'query',
    template:
      'SELECT ${2:column}, COUNT(*) AS cnt\nFROM ${1:table}\nGROUP BY ${2:column}\nORDER BY cnt DESC\nLIMIT 100;${0}',
    enabled: true,
    builtin: true,
  },
  {
    id: 'lt-seld',
    abbrev: 'seld',
    description: '字段去重查询 SELECT DISTINCT',
    category: 'query',
    template: 'SELECT DISTINCT ${2:column} FROM ${1:table} LIMIT 100;${0}',
    enabled: true,
    builtin: true,
  },
  {
    id: 'lt-selj',
    abbrev: 'selj',
    description: '双表 LEFT JOIN 关联查询',
    category: 'query',
    template:
      'SELECT a.*, b.*\nFROM ${1:table} a\nLEFT JOIN ${2:table2} b ON a.${3:column} = b.${4:column2}\nLIMIT 100;${0}',
    enabled: true,
    builtin: true,
  },
  {
    id: 'lt-selin',
    abbrev: 'selin',
    description: 'IN 多值过滤查询',
    category: 'query',
    template: 'SELECT * FROM ${1:table} WHERE ${2:column} IN (${3:values}) LIMIT 100;${0}',
    enabled: true,
    builtin: true,
  },
  {
    id: 'lt-ins',
    abbrev: 'ins',
    description: '插入单条记录 INSERT INTO',
    category: 'dml',
    template: 'INSERT INTO ${1:table} (${2:column}) VALUES (${3:value});${0}',
    enabled: true,
    builtin: true,
  },
  {
    id: 'lt-upd',
    abbrev: 'upd',
    description: '带 WHERE 安全条件更新记录',
    category: 'dml',
    template: 'UPDATE ${1:table} SET ${2:column} = ${3:value} WHERE ${4:pk} = ${5:pk_val};${0}',
    enabled: true,
    builtin: true,
  },
  {
    id: 'lt-del',
    abbrev: 'del',
    description: '带 WHERE 安全条件删除记录',
    category: 'dml',
    template: 'DELETE FROM ${1:table} WHERE ${2:pk} = ${3:pk_val};${0}',
    enabled: true,
    builtin: true,
  },
  {
    id: 'lt-cte',
    abbrev: 'cte',
    description: 'WITH 公用表表达式 (CTE)',
    category: 'query',
    template:
      'WITH ${1:cte_name} AS (\n  SELECT * FROM ${2:table} WHERE ${3:condition}\n)\nSELECT * FROM ${1:cte_name} LIMIT 100;${0}',
    enabled: true,
    builtin: true,
  },
  {
    id: 'lt-crt',
    abbrev: 'crt',
    description: '创建标准数据表 CREATE TABLE',
    category: 'ddl',
    template:
      'CREATE TABLE ${1:table_name} (\n  id BIGINT PRIMARY KEY AUTO_INCREMENT,\n  ${2:col_name} VARCHAR(128) NOT NULL,\n  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP\n);${0}',
    enabled: true,
    builtin: true,
  },
  {
    id: 'lt-alt',
    abbrev: 'alt',
    description: '新增表字段 ALTER TABLE ADD COLUMN',
    category: 'ddl',
    template:
      "ALTER TABLE ${1:table} ADD COLUMN ${2:col_name} VARCHAR(128) DEFAULT NULL COMMENT '${3:comment}';${0}",
    enabled: true,
    builtin: true,
  },
  {
    id: 'lt-idx',
    abbrev: 'idx',
    description: '新建普通索引 CREATE INDEX',
    category: 'ddl',
    template: 'CREATE INDEX idx_${1:table}_${2:column} ON ${1:table} (${2:column});${0}',
    enabled: true,
    builtin: true,
  },
  {
    id: 'lt-lj',
    abbrev: 'lj',
    description: 'LEFT JOIN 关联子句',
    category: 'clause',
    template: 'LEFT JOIN ${1:table} ${2:alias} ON ${2:alias}.${3:column} = ${4:other}${0}',
    enabled: true,
    builtin: true,
  },
  {
    id: 'lt-ij',
    abbrev: 'ij',
    description: 'INNER JOIN 内连接子句',
    category: 'clause',
    template: 'INNER JOIN ${1:table} ${2:alias} ON ${2:alias}.${3:column} = ${4:other}${0}',
    enabled: true,
    builtin: true,
  },
  {
    id: 'lt-obd',
    abbrev: 'obd',
    description: '倒序排序 ORDER BY col DESC',
    category: 'clause',
    template: 'ORDER BY ${1:column} DESC${0}',
    enabled: true,
    builtin: true,
  },
  {
    id: 'lt-oba',
    abbrev: 'oba',
    description: '升序排序 ORDER BY col ASC',
    category: 'clause',
    template: 'ORDER BY ${1:column} ASC${0}',
    enabled: true,
    builtin: true,
  },
  {
    id: 'lt-gb',
    abbrev: 'gb',
    description: '分组聚合 GROUP BY col',
    category: 'clause',
    template: 'GROUP BY ${1:column}${0}',
    enabled: true,
    builtin: true,
  },
  {
    id: 'lt-lim',
    abbrev: 'lim',
    description: '结果集行数限制 LIMIT 100',
    category: 'clause',
    template: 'LIMIT ${1:100}${0}',
    enabled: true,
    builtin: true,
  },
  {
    id: 'lt-btw',
    abbrev: 'btw',
    description: '区间过滤 BETWEEN ... AND ...',
    category: 'clause',
    template: "BETWEEN '${1:start}' AND '${2:end}'${0}",
    enabled: true,
    builtin: true,
  },
  {
    id: 'lt-isn',
    abbrev: 'isn',
    description: '空值判断 IS NULL',
    category: 'clause',
    template: 'IS NULL${0}',
    enabled: true,
    builtin: true,
  },
  {
    id: 'lt-inn',
    abbrev: 'inn',
    description: '非空判断 IS NOT NULL',
    category: 'clause',
    template: 'IS NOT NULL${0}',
    enabled: true,
    builtin: true,
  },
  {
    id: 'lt-case',
    abbrev: 'case',
    description: '条件分支 CASE WHEN ... END',
    category: 'clause',
    template: 'CASE\n  WHEN ${1:condition} THEN ${2:result}\n  ELSE ${3:fallback}\nEND${0}',
    enabled: true,
    builtin: true,
  },
];

const RESERVED_SQL_WORDS = new Set([
  'add',
  'all',
  'alter',
  'analyze',
  'and',
  'as',
  'asc',
  'between',
  'by',
  'call',
  'cascade',
  'case',
  'change',
  'check',
  'collate',
  'column',
  'condition',
  'constraint',
  'continue',
  'convert',
  'create',
  'cross',
  'database',
  'databases',
  'default',
  'delete',
  'desc',
  'describe',
  'distinct',
  'drop',
  'else',
  'end',
  'escape',
  'exists',
  'explain',
  'false',
  'fetch',
  'for',
  'force',
  'foreign',
  'from',
  'full',
  'function',
  'grant',
  'group',
  'groups',
  'having',
  'if',
  'in',
  'index',
  'inner',
  'insert',
  'interval',
  'into',
  'is',
  'join',
  'key',
  'keys',
  'kill',
  'lag',
  'lead',
  'left',
  'like',
  'limit',
  'lines',
  'load',
  'lock',
  'match',
  'not',
  'null',
  'on',
  'option',
  'or',
  'order',
  'outer',
  'over',
  'partition',
  'primary',
  'procedure',
  'range',
  'rank',
  'read',
  'references',
  'regexp',
  'rename',
  'replace',
  'return',
  'revoke',
  'right',
  'role',
  'row',
  'rows',
  'schema',
  'schemas',
  'select',
  'separator',
  'set',
  'show',
  'sql',
  'status',
  'system',
  'table',
  'tables',
  'terminated',
  'then',
  'to',
  'trigger',
  'true',
  'union',
  'unique',
  'unlock',
  'update',
  'usage',
  'use',
  'user',
  'using',
  'values',
  'view',
  'when',
  'where',
  'while',
  'window',
  'with',
  'write',
]);

export function isSqlReservedKeyword(ident: string): boolean {
  return RESERVED_SQL_WORDS.has(ident.trim().toLowerCase());
}

export function formatCompletionIdentifier(
  ident: string,
  engineKey: string,
  embeddedInIdentifier: boolean,
): string {
  if (!ident) return '';
  if (!embeddedInIdentifier && (isSqlReservedKeyword(ident) || /[^A-Za-z0-9_$]/.test(ident))) {
    return quoteIdentifier(ident, engineKey);
  }
  return ident;
}

/**
 * Normalizes either CodeMirror (`${1:table}`, `${0}`) or JetBrains-style (`$table$`, `$END$`)
 * placeholder variables into valid CodeMirror snippet templates.
 */
export function normalizeLiveTemplate(raw: string): string {
  if (!raw) return '';
  let normalized = raw.replace(/\$\{0\}|\$END\$/g, '${}');
  normalized = normalized.replace(/(^|[^\\])\$([A-Za-z_][A-Za-z0-9_]*)\$/g, '$1${$2}');
  return normalized;
}

/** Converts snippet template placeholders into clean readable preview text. */
export function formatTemplatePreview(raw: string): string {
  return raw
    .replace(/\$\{\d*\}/g, '')
    .replace(/\$\{\d+:([^}]+)\}/g, '$1')
    .replace(/\$\{([^}]+)\}/g, '$1')
    .replace(/\$END\$/g, '')
    .replace(/\$([A-Za-z_][A-Za-z0-9_]*)\$/g, '$1')
    .trim();
}

const PLACEHOLDER_TOKENS = new Set([
  'table',
  'table2',
  'tbl',
  'table_name',
  'cte_name',
  'column',
  'column2',
  'col_name',
]);

/**
 * Extracts table names and aliases referenced in the SQL statement surrounding `cursorPos`,
 * allowing column autocompletion right after selecting a table in a template.
 */
export function extractReferencedTablesInStatement(
  docText: string,
  cursorPos: number,
): { tables: string[]; aliases: Record<string, string> } {
  const before = docText.slice(0, cursorPos);
  const after = docText.slice(cursorPos);

  const lastSemi = before.lastIndexOf(';');
  const lastFence = before.lastIndexOf('```');
  const stmtStart = Math.max(0, lastSemi >= 0 ? lastSemi + 1 : 0, lastFence >= 0 ? lastFence + 3 : 0);

  const nextSemi = after.indexOf(';');
  const nextFence = after.indexOf('```');
  let stmtEnd = docText.length;
  if (nextSemi >= 0) stmtEnd = Math.min(stmtEnd, cursorPos + nextSemi);
  if (nextFence >= 0) stmtEnd = Math.min(stmtEnd, cursorPos + nextFence);

  const stmt = docText
    .slice(stmtStart, stmtEnd)
    .replace(/--[^\n]*/g, ' ')
    .replace(/\/\*[\s\S]*?\*\//g, ' ');

  const tables: string[] = [];
  const aliases: Record<string, string> = {};
  const clauseStopwords = new Set([
    'where',
    'set',
    'values',
    'value',
    'order',
    'group',
    'limit',
    'having',
    'union',
    'left',
    'right',
    'inner',
    'outer',
    'cross',
    'full',
    'join',
    'on',
    'using',
    'select',
    'from',
    'and',
    'or',
    'as',
    'add',
  ]);

  const regex =
    /\b(?:from|join|into|update|alter\s+table)\s+(?:[`"\[]?[A-Za-z0-9_$-]+[`"\]]?\s*\.\s*)?([`"\[]?[A-Za-z0-9_$-]+[`"\]]?)(?:\s+(?:as\s+)?([A-Za-z_][A-Za-z0-9_$]*))?/gi;

  let m: RegExpExecArray | null;
  while ((m = regex.exec(stmt)) !== null) {
    const rawTable = (m[1] ?? '').replace(/^[`"\[]|[`"\]]$/g, '').trim();
    if (!rawTable || PLACEHOLDER_TOKENS.has(rawTable.toLowerCase())) continue;
    if (!tables.includes(rawTable)) tables.push(rawTable);
    const maybeAlias = (m[2] ?? '').trim();
    if (maybeAlias && !clauseStopwords.has(maybeAlias.toLowerCase())) {
      aliases[maybeAlias.toLowerCase()] = rawTable;
    }
  }

  return { tables, aliases };
}

const STORAGE_KEY = 'pacer.liveTemplates.v1';

interface StoredLiveTemplates {
  version: number;
  items: SqlLiveTemplate[];
}

function cloneBuiltins(): SqlLiveTemplate[] {
  return BUILTIN_LIVE_TEMPLATES.map((t) => ({ ...t }));
}

function loadInitialTemplates(): SqlLiveTemplate[] {
  if (typeof localStorage === 'undefined') return cloneBuiltins();
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return cloneBuiltins();
    const parsed = JSON.parse(raw) as StoredLiveTemplates | SqlLiveTemplate[];
    const items = Array.isArray(parsed) ? parsed : parsed?.items;
    if (!Array.isArray(items) || items.length === 0) return cloneBuiltins();
    return items.map((item) => ({
      id: String(item.id || `lt-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`),
      abbrev: String(item.abbrev || '').trim(),
      description: String(item.description || ''),
      category: item.category || 'custom',
      template: String(item.template || ''),
      enabled: item.enabled !== false,
      builtin: Boolean(item.builtin),
    }));
  } catch {
    return cloneBuiltins();
  }
}

const templateStore = reactive<{ items: SqlLiveTemplate[] }>({
  items: loadInitialTemplates(),
});

function persistTemplates() {
  if (typeof localStorage === 'undefined') return;
  try {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({ version: 1, items: templateStore.items } satisfies StoredLiveTemplates),
    );
  } catch {
    // non-fatal
  }
}

export function useLiveTemplates() {
  function findByAbbrev(word: string): SqlLiveTemplate | undefined {
    const target = word.trim().toLowerCase();
    if (!target) return undefined;
    return templateStore.items.find(
      (item) => item.enabled && item.abbrev.trim().toLowerCase() === target,
    );
  }

  function findConflict(abbrev: string, excludeId?: string): SqlLiveTemplate | undefined {
    const target = abbrev.trim().toLowerCase();
    if (!target) return undefined;
    return templateStore.items.find(
      (item) =>
        item.enabled &&
        item.id !== excludeId &&
        item.abbrev.trim().toLowerCase() === target,
    );
  }

  function upsertTemplate(
    draft: Omit<SqlLiveTemplate, 'id'> & { id?: string },
  ): SqlLiveTemplate {
    const cleanAbbrev = draft.abbrev.trim();
    if (draft.id) {
      const idx = templateStore.items.findIndex((t) => t.id === draft.id);
      if (idx >= 0) {
        const updated: SqlLiveTemplate = {
          ...templateStore.items[idx],
          abbrev: cleanAbbrev,
          description: draft.description.trim() || cleanAbbrev,
          category: draft.category,
          template: draft.template,
          enabled: draft.enabled,
        };
        templateStore.items[idx] = updated;
        persistTemplates();
        return updated;
      }
    }
    const created: SqlLiveTemplate = {
      id: `lt-custom-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
      abbrev: cleanAbbrev,
      description: draft.description.trim() || cleanAbbrev,
      category: draft.category || 'custom',
      template: draft.template,
      enabled: draft.enabled !== false,
      builtin: false,
    };
    templateStore.items.unshift(created);
    persistTemplates();
    return created;
  }

  function toggleTemplate(id: string) {
    const target = templateStore.items.find((t) => t.id === id);
    if (!target) return;
    target.enabled = !target.enabled;
    persistTemplates();
  }

  function deleteTemplate(id: string) {
    const idx = templateStore.items.findIndex((t) => t.id === id);
    if (idx >= 0) {
      templateStore.items.splice(idx, 1);
      persistTemplates();
    }
  }

  function resetToDefaults() {
    templateStore.items.splice(0, templateStore.items.length, ...cloneBuiltins());
    persistTemplates();
  }

  function exportJson(): string {
    return JSON.stringify(templateStore.items, null, 2);
  }

  function importJson(raw: string): { added: number; updated: number } {
    const parsed = JSON.parse(raw);
    const list: unknown[] = Array.isArray(parsed)
      ? parsed
      : Array.isArray(parsed?.items)
        ? parsed.items
        : [];
    let added = 0;
    let updated = 0;
    for (const entry of list) {
      if (!entry || typeof entry !== 'object') continue;
      const obj = entry as Partial<SqlLiveTemplate>;
      const abbrev = String(obj.abbrev ?? '').trim();
      const template = String(obj.template ?? '');
      if (!abbrev || !template) continue;
      const existing = templateStore.items.find(
        (t) => t.abbrev.trim().toLowerCase() === abbrev.toLowerCase(),
      );
      if (existing) {
        existing.description = String(obj.description || existing.description || abbrev);
        existing.template = template;
        existing.category = (obj.category as LiveTemplateCategory) || existing.category;
        existing.enabled = obj.enabled !== false;
        updated += 1;
      } else {
        templateStore.items.unshift({
          id: `lt-import-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
          abbrev,
          description: String(obj.description || abbrev),
          category: (obj.category as LiveTemplateCategory) || 'custom',
          template,
          enabled: obj.enabled !== false,
          builtin: false,
        });
        added += 1;
      }
    }
    persistTemplates();
    return { added, updated };
  }

  return {
    templates: templateStore.items,
    findByAbbrev,
    findConflict,
    upsertTemplate,
    toggleTemplate,
    deleteTemplate,
    resetToDefaults,
    exportJson,
    importJson,
  };
}
