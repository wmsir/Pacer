import type {
  ColumnMeta,
  ForeignKeyMeta,
  IndexMeta,
  RoutineMeta,
  TableMeta,
} from '../api/ipc';
import { formatSqlLiteral, getEngineDisplayName, quoteIdentifier } from './sqlTools';

// ==================== 1. Server-Side Pagination & Filter SQL Builder ====================

function qualifyTableRef(
  tableName: string,
  dbName: string | undefined,
  engineKey: string,
): string {
  const q = (id: string) => quoteIdentifier(id, engineKey);
  if (dbName && dbName.trim() && dbName !== 'main' && dbName !== 'default') {
    return `${q(dbName.trim())}.${q(tableName)}`;
  }
  return q(tableName);
}

function cleanClauseFragment(raw: string | undefined, keywordPrefix: RegExp): string {
  const trimmed = (raw ?? '').trim().replace(/;+\s*$/, '');
  return trimmed.replace(keywordPrefix, '').trim();
}

export function buildTablePageQuerySql(opts: {
  dbName?: string;
  tableName: string;
  engineKey: string;
  whereClause?: string;
  orderByClause?: string;
  limit: number;
  offset: number;
}): string {
  const tableRef = qualifyTableRef(opts.tableName, opts.dbName, opts.engineKey);
  const where = cleanClauseFragment(opts.whereClause, /^WHERE\s+/i);
  const orderBy = cleanClauseFragment(opts.orderByClause, /^ORDER\s+BY\s+/i);
  const limit = Math.max(1, Math.floor(opts.limit || 100));
  const offset = Math.max(0, Math.floor(opts.offset || 0));
  const eng = opts.engineKey.toLowerCase();

  let sql = `SELECT * FROM ${tableRef}`;
  if (where) sql += ` WHERE ${where}`;
  if (orderBy) {
    sql += ` ORDER BY ${orderBy}`;
  } else if (eng.includes('sqlserver') || eng.includes('mssql')) {
    sql += ' ORDER BY (SELECT NULL)';
  }

  if (eng.includes('sqlserver') || eng.includes('mssql') || eng.includes('oracle')) {
    sql += ` OFFSET ${offset} ROWS FETCH NEXT ${limit} ROWS ONLY`;
  } else {
    sql += offset > 0 ? ` LIMIT ${limit} OFFSET ${offset}` : ` LIMIT ${limit}`;
  }
  return sql;
}

export function buildTableCountQuerySql(opts: {
  dbName?: string;
  tableName: string;
  engineKey: string;
  whereClause?: string;
}): string {
  const tableRef = qualifyTableRef(opts.tableName, opts.dbName, opts.engineKey);
  const where = cleanClauseFragment(opts.whereClause, /^WHERE\s+/i);
  return where
    ? `SELECT COUNT(*) AS total_rows FROM ${tableRef} WHERE ${where}`
    : `SELECT COUNT(*) AS total_rows FROM ${tableRef}`;
}

export function appendWhereCondition(
  existingWhere: string,
  colName: string,
  operator: '=' | '!=' | 'IS NULL' | 'IS NOT NULL' | 'LIKE',
  value: unknown,
  engineKey: string,
): string {
  const colRef = quoteIdentifier(colName, engineKey);
  let predicate = '';
  if (operator === 'IS NULL' || operator === 'IS NOT NULL' || value === null || value === undefined) {
    predicate = operator === '!=' || operator === 'IS NOT NULL'
      ? `${colRef} IS NOT NULL`
      : `${colRef} IS NULL`;
  } else if (operator === 'LIKE') {
    const rawStr = String(value);
    predicate = `${colRef} LIKE ${formatSqlLiteral(`%${rawStr}%`)}`;
  } else {
    predicate = `${colRef} ${operator} ${formatSqlLiteral(value)}`;
  }

  const prev = cleanClauseFragment(existingWhere, /^WHERE\s+/i);
  if (!prev) return predicate;
  const wrappedPrev = /\sOR\s/i.test(prev) && !(prev.startsWith('(') && prev.endsWith(')'))
    ? `(${prev})`
    : prev;
  return `${wrappedPrev} AND ${predicate}`;
}

// ==================== 2. Visual Execution Plan Parser ====================

export type ExplainNodeSeverity = 'critical' | 'warn' | 'ok' | 'info';

export interface VisualExplainNode {
  id: string;
  depth: number;
  title: string;
  subtitle?: string;
  tableName?: string;
  accessType?: string;
  indexUsed?: string | null;
  possibleKeys?: string | null;
  estimatedRows?: number | null;
  costLabel?: string | null;
  filteredPct?: number | null;
  extra?: string | null;
  severity: ExplainNodeSeverity;
  tips: string[];
}

export interface VisualExplainPlan {
  engineFamily: 'mysql' | 'postgres' | 'sqlite' | 'generic';
  health: 'good' | 'warn' | 'critical';
  headline: string;
  totalEstimatedRows: number;
  criticalCount: number;
  warningCount: number;
  nodes: VisualExplainNode[];
  summaryTips: string[];
}

function getFieldCI(row: Record<string, unknown>, candidates: string[]): unknown {
  const entries = Object.entries(row);
  for (const cand of candidates) {
    const low = cand.toLowerCase();
    for (const [k, v] of entries) {
      if (k.toLowerCase() === low) return v;
    }
  }
  return undefined;
}

export function isExplainResult(columns: string[], sql: string): boolean {
  const cleanSql = sql.trim().replace(/^(--[^\n]*\n|\s)+/, '').toUpperCase();
  if (cleanSql.startsWith('EXPLAIN')) return true;
  const colsLower = columns.map((c) => c.toLowerCase());
  if (colsLower.includes('query plan')) return true;
  if (colsLower.includes('select_type') && colsLower.includes('possible_keys')) return true;
  if (colsLower.includes('id') && colsLower.includes('parent') && colsLower.includes('detail')) return true;
  return false;
}

export function parseExplainPlanVisual(
  columns: string[],
  rows: Record<string, unknown>[],
  engineKey: string,
): VisualExplainPlan {
  const colsLower = columns.map((c) => c.toLowerCase());
  const eng = engineKey.toLowerCase();

  // Case A: PostgreSQL / openGauss / Kingbase (`QUERY PLAN`)
  if (colsLower.includes('query plan') || eng.includes('postgres') || eng.includes('kingbase') || eng.includes('gauss')) {
    const planLines: string[] = [];
    for (const r of rows) {
      const val = getFieldCI(r, ['QUERY PLAN', 'plan', columns[0] ?? 'QUERY PLAN']);
      if (val !== null && val !== undefined) {
        for (const sub of String(val).split(/\r?\n/)) {
          if (sub.trim()) planLines.push(sub);
        }
      }
    }

    const nodes: VisualExplainNode[] = [];
    let totalRows = 0;
    let criticalCount = 0;
    let warningCount = 0;

    for (let i = 0; i < planLines.length; i++) {
      const raw = planLines[i];
      const trimmed = raw.trim();
      const arrowIdx = raw.indexOf('->');
      const indentCols = arrowIdx >= 0 ? arrowIdx : raw.search(/\S/);
      const depth =
        arrowIdx >= 0
          ? Math.max(1, Math.min(8, Math.floor((arrowIdx + 4) / 6)))
          : Math.max(0, Math.min(8, Math.floor(Math.max(0, indentCols) / 4)));
      const cleanLine = trimmed.replace(/^->\s*/, '');

      // Ignore sub-detail properties like "Filter: ...", "Buffers: ...", "Planning Time: ..."
      if (/^(Filter|Index Cond|Hash Cond|Recheck Cond|Sort Key|Join Filter|Output|Buffers|Planning|Execution|Workers):/i.test(cleanLine) && nodes.length > 0) {
        const last = nodes[nodes.length - 1];
        last.extra = last.extra ? `${last.extra} | ${cleanLine}` : cleanLine;
        continue;
      }

      const costMatch = /\(cost=([\d.]+\.\.[\d.]+)\s+rows=(\d+)/i.exec(cleanLine);
      const actualMatch = /\(actual time=([\d.]+\.\.[\d.]+)\s+rows=(\d+)/i.exec(cleanLine);
      const rowsEst = actualMatch
        ? Number(actualMatch[2])
        : costMatch
          ? Number(costMatch[2])
          : null;
      if (rowsEst !== null && Number.isFinite(rowsEst)) {
        totalRows = Math.max(totalRows, rowsEst);
      }

      const opPart = cleanLine.split('(')[0].trim();
      const onMatch = /\bon\s+([A-Za-z0-9_."]+)/i.exec(opPart);
      const tips: string[] = [];
      let severity: ExplainNodeSeverity = 'ok';

      if (/^Seq Scan\b/i.test(opPart)) {
        if ((rowsEst ?? 0) >= 1000) {
          severity = 'critical';
          criticalCount++;
          tips.push('检测到大数据量顺序扫描 (Seq Scan)，建议检查 WHERE/JOIN 字段并建立 B-Tree 索引');
        } else {
          severity = 'warn';
          warningCount++;
          tips.push('全表顺序扫描 (Seq Scan)，数据量增长后可能成为瓶颈');
        }
      } else if (/\bSort\b/i.test(opPart) && /Disk/i.test(cleanLine)) {
        severity = 'warn';
        warningCount++;
        tips.push('出现排序操作，可考虑复合索引有序扫描以消除额外 Sort 开销');
      } else if (/\bIndex/i.test(opPart)) {
        severity = 'ok';
      }

      nodes.push({
        id: `pg-${i}`,
        depth,
        title: opPart || cleanLine,
        tableName: onMatch ? onMatch[1] : undefined,
        accessType: opPart.split(/\s+on\s+/i)[0],
        estimatedRows: rowsEst,
        costLabel: costMatch ? `cost=${costMatch[1]}` : null,
        extra: actualMatch ? `actual=${actualMatch[1]}ms (rows=${actualMatch[2]})` : null,
        severity,
        tips,
      });
    }

    const health = criticalCount > 0 ? 'critical' : warningCount > 0 ? 'warn' : 'good';
    return {
      engineFamily: 'postgres',
      health,
      headline:
        criticalCount > 0
          ? `发现 ${criticalCount} 处高代价顺序扫描，建议补充索引`
          : warningCount > 0
            ? `共 ${nodes.length} 个计划节点，存在 ${warningCount} 项需关注操作`
            : `执行计划良好，共 ${nodes.length} 个计划节点（已充分利用索引）`,
      totalEstimatedRows: totalRows,
      criticalCount,
      warningCount,
      nodes,
      summaryTips: Array.from(new Set(nodes.flatMap((n) => n.tips))),
    };
  }

  // Case B: SQLite (`id`, `parent`, `detail`)
  if (colsLower.includes('parent') && colsLower.includes('detail')) {
    const parentMap = new Map<number, number>();
    const depthOf = (id: number, visited = new Set<number>()): number => {
      if (visited.has(id)) return 0;
      visited.add(id);
      const p = parentMap.get(id);
      if (p === undefined || p <= 0) return 0;
      return 1 + depthOf(p, visited);
    };
    for (const r of rows) {
      const id = Number(getFieldCI(r, ['id']) ?? 0);
      const parent = Number(getFieldCI(r, ['parent']) ?? 0);
      parentMap.set(id, parent);
    }

    let warningCount = 0;
    const nodes: VisualExplainNode[] = rows.map((r, idx) => {
      const id = Number(getFieldCI(r, ['id']) ?? idx);
      const detail = String(getFieldCI(r, ['detail']) ?? '');
      const depth = depthOf(id);
      const upper = detail.toUpperCase();
      const tips: string[] = [];
      let severity: ExplainNodeSeverity = 'ok';
      let accessType = 'PLAN';

      if (upper.startsWith('SCAN ')) {
        severity = 'warn';
        warningCount++;
        accessType = 'SCAN (全表扫)';
        tips.push('SQLite 执行了全表 SCAN，可为过滤/关联列添加 INDEX 以转为 SEARCH');
      } else if (upper.includes('SEARCH ')) {
        accessType = upper.includes('COVERING INDEX') ? 'COVERING INDEX' : 'INDEX SEARCH';
        severity = 'ok';
      } else if (upper.includes('USE TEMP B-TREE')) {
        severity = 'warn';
        warningCount++;
        accessType = 'TEMP B-TREE';
        tips.push('使用临时 B-Tree 排序或分组，可通过覆盖索引消除');
      }

      const tblMatch = /(?:SCAN|SEARCH)\s+([^\s(]+)/i.exec(detail);
      return {
        id: `sqlite-${id}-${idx}`,
        depth,
        title: detail,
        tableName: tblMatch ? tblMatch[1] : undefined,
        accessType,
        severity,
        tips,
      };
    });

    return {
      engineFamily: 'sqlite',
      health: warningCount > 0 ? 'warn' : 'good',
      headline:
        warningCount > 0
          ? `SQLite 查询计划含 ${warningCount} 处全表扫描或临时排序`
          : `SQLite 查询计划良好，已命中索引或主键`,
      totalEstimatedRows: 0,
      criticalCount: 0,
      warningCount,
      nodes,
      summaryTips: Array.from(new Set(nodes.flatMap((n) => n.tips))),
    };
  }

  // Case C: MySQL / MariaDB / TiDB / OceanBase tabular EXPLAIN
  if (colsLower.includes('select_type') || colsLower.includes('type') || colsLower.includes('table')) {
    let totalRows = 0;
    let criticalCount = 0;
    let warningCount = 0;

    const nodes: VisualExplainNode[] = rows.map((r, idx) => {
      const stepId = getFieldCI(r, ['id']) ?? idx + 1;
      const selectType = String(getFieldCI(r, ['select_type']) ?? '');
      const tableName = String(getFieldCI(r, ['table']) ?? '(derived)');
      const accessType = String(getFieldCI(r, ['type']) ?? '').trim();
      const possibleKeys = getFieldCI(r, ['possible_keys']);
      const keyUsed = getFieldCI(r, ['key']);
      const estRowsRaw = Number(getFieldCI(r, ['rows']) ?? 0);
      const estRows = Number.isFinite(estRowsRaw) ? estRowsRaw : null;
      const filteredRaw = Number(getFieldCI(r, ['filtered']));
      const filteredPct = Number.isFinite(filteredRaw) ? filteredRaw : null;
      const extra = getFieldCI(r, ['Extra', 'extra']);
      const extraStr = extra ? String(extra) : null;

      if (estRows && estRows > 0) {
        totalRows += estRows;
      }

      const tips: string[] = [];
      let severity: ExplainNodeSeverity = 'ok';
      const typeLow = accessType.toLowerCase();

      if (typeLow === 'all') {
        severity = 'critical';
        criticalCount++;
        tips.push(`表 ${tableName} 触发 ALL 全表扫描，请检查 WHERE 条件是否缺失索引或发生隐式类型转换`);
      } else if (typeLow === 'index') {
        severity = 'warn';
        warningCount++;
        tips.push(`表 ${tableName} 执行了全索引扫描 (type=index)，请核查索引最左前缀是否命中`);
      }

      if (extraStr) {
        if (/using filesort/i.test(extraStr)) {
          if (severity === 'ok') {
            severity = 'warn';
            warningCount++;
          }
          tips.push('出现 Using filesort（文件排序），可通过调整 ORDER BY 复合索引顺序优化');
        }
        if (/using temporary/i.test(extraStr)) {
          severity = 'critical';
          criticalCount++;
          tips.push('出现 Using temporary（临时表），常见于 GROUP BY / DISTINCT 未走索引');
        }
      }

      return {
        id: `mysql-${stepId}-${idx}`,
        depth: selectType === 'SIMPLE' || selectType === 'PRIMARY' ? 0 : 1,
        title: `${selectType ? `[${selectType}] ` : ''}${tableName}`,
        tableName,
        accessType: accessType || 'UNKNOWN',
        indexUsed: keyUsed ? String(keyUsed) : null,
        possibleKeys: possibleKeys ? String(possibleKeys) : null,
        estimatedRows: estRows,
        filteredPct,
        extra: extraStr,
        severity,
        tips,
      };
    });

    const health = criticalCount > 0 ? 'critical' : warningCount > 0 ? 'warn' : 'good';
    return {
      engineFamily: 'mysql',
      health,
      headline:
        criticalCount > 0
          ? `发现 ${criticalCount} 项高风险扫描（预估扫描约 ${totalRows} 行）`
          : warningCount > 0
            ? `共 ${nodes.length} 步操作，含 ${warningCount} 项可优化提示`
            : `执行计划健康，均命中高选择性索引（预估 ${totalRows} 行）`,
      totalEstimatedRows: totalRows,
      criticalCount,
      warningCount,
      nodes,
      summaryTips: Array.from(new Set(nodes.flatMap((n) => n.tips))),
    };
  }

  // Fallback generic EXPLAIN view
  return {
    engineFamily: 'generic',
    health: 'good',
    headline: `共返回 ${rows.length} 行执行计划输出`,
    totalEstimatedRows: 0,
    criticalCount: 0,
    warningCount: 0,
    nodes: rows.map((r, idx) => ({
      id: `gen-${idx}`,
      depth: 0,
      title: Object.values(r)
        .filter((v) => v !== null && v !== undefined)
        .map((v) => String(v))
        .join(' | '),
      severity: 'info',
      tips: [],
    })),
    summaryTips: [],
  };
}

// ==================== 3. Data Comparison & Sync Engine ====================

export type DataDiffKind = 'added' | 'removed' | 'modified' | 'same';

export interface DataDiffEntry {
  kind: DataDiffKind;
  keyDisplay: string;
  sourceRow: Record<string, unknown> | null;
  targetRow: Record<string, unknown> | null;
  changedColumns: string[];
}

export interface DataDiffSummary {
  keyColumns: string[];
  compareColumns: string[];
  allColumns: string[];
  entries: DataDiffEntry[];
  addedCount: number;
  removedCount: number;
  modifiedCount: number;
  sameCount: number;
}

function normalizeCellCompare(v: unknown): string {
  if (v === null || v === undefined) return '__PACER_NULL__';
  if (typeof v === 'object') return `json:${JSON.stringify(v)}`;
  return `${typeof v}:${String(v)}`;
}

function buildRowCompositeKey(row: Record<string, unknown>, keyCols: string[]): string {
  return JSON.stringify(keyCols.map((k) => row[k] ?? null));
}

export function computeDataDiff(opts: {
  sourceColumns: string[];
  sourceRows: Record<string, unknown>[];
  targetColumns: string[];
  targetRows: Record<string, unknown>[];
  keyColumns: string[];
  compareColumns?: string[];
}): DataDiffSummary {
  const allColumns = Array.from(new Set([...opts.sourceColumns, ...opts.targetColumns]));
  const keyColumns = opts.keyColumns.length > 0 ? opts.keyColumns : allColumns.slice(0, 1);
  const compareColumns = opts.compareColumns && opts.compareColumns.length > 0
    ? opts.compareColumns
    : allColumns.filter((c) => !keyColumns.includes(c));

  const targetMap = new Map<string, Record<string, unknown>>();
  for (const row of opts.targetRows) {
    const key = buildRowCompositeKey(row, keyColumns);
    if (!targetMap.has(key)) {
      targetMap.set(key, row);
    }
  }

  const seenKeys = new Set<string>();
  const entries: DataDiffEntry[] = [];
  let addedCount = 0;
  let removedCount = 0;
  let modifiedCount = 0;
  let sameCount = 0;

  for (const src of opts.sourceRows) {
    const key = buildRowCompositeKey(src, keyColumns);
    seenKeys.add(key);
    const keyDisplay = keyColumns.map((c) => `${c}=${String(src[c] ?? 'NULL')}`).join(', ');
    const tgt = targetMap.get(key);
    if (!tgt) {
      addedCount++;
      entries.push({
        kind: 'added',
        keyDisplay,
        sourceRow: src,
        targetRow: null,
        changedColumns: [],
      });
      continue;
    }

    const changedColumns: string[] = [];
    for (const col of compareColumns) {
      if (normalizeCellCompare(src[col]) !== normalizeCellCompare(tgt[col])) {
        changedColumns.push(col);
      }
    }

    if (changedColumns.length > 0) {
      modifiedCount++;
      entries.push({
        kind: 'modified',
        keyDisplay,
        sourceRow: src,
        targetRow: tgt,
        changedColumns,
      });
    } else {
      sameCount++;
      entries.push({
        kind: 'same',
        keyDisplay,
        sourceRow: src,
        targetRow: tgt,
        changedColumns: [],
      });
    }
  }

  for (const [key, tgt] of targetMap.entries()) {
    if (seenKeys.has(key)) continue;
    removedCount++;
    const keyDisplay = keyColumns.map((c) => `${c}=${String(tgt[c] ?? 'NULL')}`).join(', ');
    entries.push({
      kind: 'removed',
      keyDisplay,
      sourceRow: null,
      targetRow: tgt,
      changedColumns: [],
    });
  }

  return {
    keyColumns,
    compareColumns,
    allColumns,
    entries,
    addedCount,
    removedCount,
    modifiedCount,
    sameCount,
  };
}

export function generateDataDiffSyncSql(
  diff: DataDiffSummary,
  opts: {
    targetTable: string;
    targetDb?: string;
    engineKey: string;
    includeInserts: boolean;
    includeUpdates: boolean;
    includeDeletes: boolean;
  },
): string[] {
  const q = (id: string) => quoteIdentifier(id, opts.engineKey);
  const tableRef = qualifyTableRef(opts.targetTable || 'target_table', opts.targetDb, opts.engineKey);
  const stmts: string[] = [];

  for (const entry of diff.entries) {
    if (entry.kind === 'added' && opts.includeInserts && entry.sourceRow) {
      const cols = diff.allColumns.filter((c) => entry.sourceRow && c in entry.sourceRow);
      const colList = cols.map(q).join(', ');
      const valList = cols.map((c) => formatSqlLiteral(entry.sourceRow?.[c])).join(', ');
      stmts.push(`INSERT INTO ${tableRef} (${colList}) VALUES (${valList});`);
    } else if (entry.kind === 'modified' && opts.includeUpdates && entry.sourceRow && entry.targetRow) {
      const setList = entry.changedColumns
        .map((c) => `${q(c)} = ${formatSqlLiteral(entry.sourceRow?.[c])}`)
        .join(', ');
      const whereList = diff.keyColumns
        .map((k) => {
          const val = entry.targetRow?.[k];
          return val === null || val === undefined
            ? `${q(k)} IS NULL`
            : `${q(k)} = ${formatSqlLiteral(val)}`;
        })
        .join(' AND ');
      if (setList && whereList) {
        stmts.push(`UPDATE ${tableRef} SET ${setList} WHERE ${whereList};`);
      }
    } else if (entry.kind === 'removed' && opts.includeDeletes && entry.targetRow) {
      const whereList = diff.keyColumns
        .map((k) => {
          const val = entry.targetRow?.[k];
          return val === null || val === undefined
            ? `${q(k)} IS NULL`
            : `${q(k)} = ${formatSqlLiteral(val)}`;
        })
        .join(' AND ');
      if (whereList) {
        stmts.push(`DELETE FROM ${tableRef} WHERE ${whereList};`);
      }
    }
  }
  return stmts;
}

// ==================== 4. Cross-Engine DDL Translator for Data Transfer ====================

function translateColumnType(rawType: string, targetEngineKey: string): string {
  const t = rawType.trim().toUpperCase();
  const target = targetEngineKey.toLowerCase();

  if (target.includes('postgres') || target.includes('kingbase') || target.includes('gauss')) {
    if (t.includes('BIGINT')) return 'BIGINT';
    if (t.includes('TINYINT(1)') || t === 'BOOL' || t === 'BOOLEAN') return 'BOOLEAN';
    if (t.includes('INT')) return 'INTEGER';
    if (t.includes('DECIMAL') || t.includes('NUMERIC')) return t;
    if (t.includes('DOUBLE') || t.includes('FLOAT') || t === 'REAL') return 'DOUBLE PRECISION';
    if (t.includes('DATETIME') || t.includes('TIMESTAMP')) return 'TIMESTAMP';
    if (t === 'DATE') return 'DATE';
    if (t.includes('JSON')) return 'JSONB';
    if (t.includes('BLOB') || t.includes('BINARY') || t.includes('BYTEA')) return 'BYTEA';
    if (t.includes('TEXT')) return 'TEXT';
    if (t.includes('VARCHAR') || t.includes('CHAR')) return t;
    return 'TEXT';
  }

  if (target.includes('sqlite')) {
    if (t.includes('INT') || t.includes('BOOL')) return 'INTEGER';
    if (t.includes('FLOAT') || t.includes('DOUBLE') || t.includes('REAL') || t.includes('DECIMAL') || t.includes('NUMERIC')) {
      return 'REAL';
    }
    if (t.includes('BLOB') || t.includes('BYTEA') || t.includes('BINARY')) return 'BLOB';
    return 'TEXT';
  }

  // Default MySQL / MariaDB / TiDB / OceanBase
  if (t.includes('BIGINT') || t === 'INT8') return 'BIGINT';
  if (t.includes('INT')) return 'INT';
  if (t === 'BOOLEAN' || t === 'BOOL') return 'TINYINT(1)';
  if (t.includes('DOUBLE PRECISION')) return 'DOUBLE';
  if (t.includes('DECIMAL') || t.includes('NUMERIC')) return t;
  if (t.includes('JSONB') || t === 'JSON') return 'JSON';
  if (t.includes('TIMESTAMP')) return 'DATETIME';
  if (t.includes('BYTEA') || t.includes('BLOB')) return 'LONGBLOB';
  if (t.includes('TEXT') || t === 'ANY') return 'TEXT';
  if (t.includes('VARCHAR') || t.includes('CHAR')) return t;
  return 'TEXT';
}

export function buildTransferCreateTableSql(opts: {
  targetDb?: string;
  tableName: string;
  columns: ColumnMeta[];
  targetEngineKey: string;
}): string {
  const q = (id: string) => quoteIdentifier(id, opts.targetEngineKey);
  const tableRef = qualifyTableRef(opts.tableName, opts.targetDb, opts.targetEngineKey);
  const pks = opts.columns.filter((c) => c.isPrimaryKey).map((c) => q(c.name));
  const defs = opts.columns.map((c) => {
    const mapped = translateColumnType(c.dataType, opts.targetEngineKey);
    const nullClause = !c.nullable || c.isPrimaryKey ? ' NOT NULL' : '';
    return `  ${q(c.name)} ${mapped}${nullClause}`;
  });
  if (pks.length > 0) {
    defs.push(`  PRIMARY KEY (${pks.join(', ')})`);
  }
  return `CREATE TABLE IF NOT EXISTS ${tableRef} (\n${defs.join(',\n')}\n);`;
}

// ==================== 5. Data Dictionary Generator (Markdown & HTML) ====================

export interface DictionaryTableEntry {
  meta: TableMeta;
  columns: ColumnMeta[];
  indexes: IndexMeta[];
  foreignKeys: ForeignKeyMeta[];
}

export interface DataDictionaryInput {
  profileName: string;
  engineKey: string;
  dbName: string;
  generatedAt: string;
  tables: DictionaryTableEntry[];
  routines?: RoutineMeta[];
}

function escapeMdCell(raw: string | null | undefined): string {
  return (raw ?? '')
    .replace(/\|/g, '\\|')
    .replace(/\r?\n/g, ' ')
    .trim();
}

export function escapeHtml(raw: string | null | undefined): string {
  return (raw ?? '')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

export function generateDataDictionaryMarkdown(input: DataDictionaryInput): string {
  const engineDisplay = getEngineDisplayName(input.engineKey);
  const lines: string[] = [
    `# 数据库数据字典 — ${input.dbName}`,
    '',
    `- **数据源**: ${input.profileName} (${engineDisplay})`,
    `- **数据库 / Schema**: \`${input.dbName}\``,
    `- **包含对象数**: ${input.tables.length} 张表/视图${input.routines?.length ? ` · ${input.routines.length} 个函数/存储过程/触发器` : ''}`,
    `- **生成时间**: ${input.generatedAt}`,
    '',
    '## 目录清单',
    '',
    '| # | 表名 | 类型 | 预估行数 | 引擎 | 说明 |',
    '| :- | :--- | :--- | ---: | :--- | :--- |',
  ];

  input.tables.forEach((t, idx) => {
    lines.push(
      `| ${idx + 1} | [\`${t.meta.name}\`](#${t.meta.name.toLowerCase().replace(/[^a-z0-9_-]/g, '-')}) | ${escapeMdCell(t.meta.tableType || 'TABLE')} | ${t.meta.estimatedRows ?? '-'} | ${escapeMdCell(t.meta.engine || engineDisplay)} | ${escapeMdCell(t.meta.comment || '-')} |`,
    );
  });

  lines.push('');

  for (const tbl of input.tables) {
    lines.push(`## \`${tbl.meta.name}\``);
    if (tbl.meta.comment) {
      lines.push('', `> ${tbl.meta.comment}`);
    }
    lines.push(
      '',
      '| 字段名 | 数据类型 | 主键 | 可空 | 默认值 | 注释 |',
      '| :--- | :--- | :-: | :-: | :--- | :--- |',
    );
    for (const c of tbl.columns) {
      lines.push(
        `| \`${escapeMdCell(c.name)}\` | \`${escapeMdCell(c.dataType)}\` | ${c.isPrimaryKey ? 'PK' : ''} | ${c.nullable ? 'YES' : 'NO'} | ${escapeMdCell(c.defaultValue ?? '')} | ${escapeMdCell(c.comment ?? '')} |`,
      );
    }

    if (tbl.indexes.length > 0) {
      lines.push('', '**索引清单:**');
      for (const idx of tbl.indexes) {
        lines.push(
          `- \`${idx.name}\` (${idx.unique ? 'UNIQUE' : 'INDEX'}${idx.indexType ? ` / ${idx.indexType}` : ''}): \`${idx.columns.join(', ')}\``,
        );
      }
    }

    if (tbl.foreignKeys.length > 0) {
      lines.push('', '**外键约束:**');
      for (const fk of tbl.foreignKeys) {
        lines.push(
          `- \`${fk.name}\`: \`${fk.columnName}\` → \`${fk.referencedTable}.${fk.referencedColumn}\`${fk.onDelete ? ` (ON DELETE ${fk.onDelete})` : ''}`,
        );
      }
    }
    lines.push('');
  }

  if (input.routines && input.routines.length > 0) {
    lines.push('## 函数 / 存储过程 / 触发器', '');
    lines.push('| 名称 | 类型 | 返回类型 | 说明 |');
    lines.push('| :--- | :--- | :--- | :--- |');
    for (const r of input.routines) {
      lines.push(
        `| \`${escapeMdCell(r.name)}\` | ${escapeMdCell(r.kind)} | ${escapeMdCell(r.returnType ?? '-')} | ${escapeMdCell(r.comment ?? '-')} |`,
      );
    }
    lines.push('');
  }

  return lines.join('\n');
}

export function generateDataDictionaryHtml(input: DataDictionaryInput): string {
  const engineDisplay = getEngineDisplayName(input.engineKey);
  const navLinks = input.tables
    .map(
      (t, idx) =>
        `<a class="nav-item" data-name="${escapeHtml(t.meta.name.toLowerCase())}" href="#tbl-${idx}"><span>${escapeHtml(t.meta.name)}</span><small>${t.columns.length} 列</small></a>`,
    )
    .join('\n');

  const sections = input.tables
    .map((tbl, idx) => {
      const colRows = tbl.columns
        .map(
          (c) => `<tr>
            <td class="mono fw">${escapeHtml(c.name)}</td>
            <td class="mono type">${escapeHtml(c.dataType)}</td>
            <td class="tc">${c.isPrimaryKey ? '<span class="badge pk">PK</span>' : ''}</td>
            <td class="tc">${c.nullable ? 'YES' : '<span class="badge nn">NO</span>'}</td>
            <td class="mono">${escapeHtml(c.defaultValue ?? '')}</td>
            <td>${escapeHtml(c.comment ?? '')}</td>
          </tr>`,
        )
        .join('');

      const idxBlock =
        tbl.indexes.length > 0
          ? `<div class="sub-box"><strong>索引</strong><ul>${tbl.indexes
              .map(
                (i) =>
                  `<li><code>${escapeHtml(i.name)}</code> (${escapeHtml(i.unique ? 'UNIQUE' : 'INDEX')}) — <code>${escapeHtml(i.columns.join(', '))}</code></li>`,
              )
              .join('')}</ul></div>`
          : '';

      const fkBlock =
        tbl.foreignKeys.length > 0
          ? `<div class="sub-box"><strong>外键关联</strong><ul>${tbl.foreignKeys
              .map(
                (f) =>
                  `<li><code>${escapeHtml(f.columnName)}</code> → <code>${escapeHtml(f.referencedTable)}.${escapeHtml(f.referencedColumn)}</code> ${escapeHtml(f.onDelete ? `[ON DELETE ${f.onDelete}]` : '')}</li>`,
              )
              .join('')}</ul></div>`
          : '';

      return `<section class="card tbl-section" id="tbl-${idx}" data-name="${escapeHtml(tbl.meta.name.toLowerCase())}">
        <div class="card-head">
          <h2>${escapeHtml(tbl.meta.name)}</h2>
          <span class="meta">${escapeHtml(tbl.meta.tableType || 'TABLE')} · ${tbl.columns.length} 字段${tbl.meta.estimatedRows != null ? ` · ~${tbl.meta.estimatedRows} 行` : ''}</span>
        </div>
        ${tbl.meta.comment ? `<p class="desc">${escapeHtml(tbl.meta.comment)}</p>` : ''}
        <table>
          <thead><tr><th>字段名</th><th>类型</th><th>主键</th><th>可空</th><th>默认值</th><th>说明</th></tr></thead>
          <tbody>${colRows}</tbody>
        </table>
        ${idxBlock}${fkBlock}
      </section>`;
    })
    .join('\n');

  const routineSection =
    input.routines && input.routines.length > 0
      ? `<section class="card" id="routines-sec">
          <div class="card-head"><h2>函数 / 存储过程 / 触发器 (${input.routines.length})</h2></div>
          <table>
            <thead><tr><th>名称</th><th>类型</th><th>返回值</th><th>备注</th></tr></thead>
            <tbody>${input.routines
              .map(
                (r) =>
                  `<tr><td class="mono fw">${escapeHtml(r.name)}</td><td>${escapeHtml(r.kind)}</td><td class="mono">${escapeHtml(r.returnType ?? '-')}</td><td>${escapeHtml(r.comment ?? '-')}</td></tr>`,
              )
              .join('')}</tbody>
          </table>
        </section>`
      : '';

  return `<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8" />
<meta name="viewport" content="width=device-width, initial-scale=1.0" />
<title>数据字典 - ${escapeHtml(input.dbName)}</title>
<style>
  :root { --bg:#f8fafc; --panel:#ffffff; --border:#e2e8f0; --text:#0f172a; --muted:#64748b; --accent:#2563eb; }
  * { box-sizing:border-box; }
  body { margin:0; font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,sans-serif; background:var(--bg); color:var(--text); display:flex; min-height:100vh; }
  aside { width:260px; background:var(--panel); border-right:1px solid var(--border); padding:16px; position:sticky; top:0; height:100vh; overflow:auto; flex-shrink:0; }
  aside h1 { font-size:15px; margin:0 0 6px; }
  aside .sub { font-size:12px; color:var(--muted); margin-bottom:12px; }
  aside input { width:100%; padding:7px 10px; border:1px solid var(--border); border-radius:6px; margin-bottom:10px; font-size:13px; }
  .nav-item { display:flex; justify-content:space-between; align-items:center; padding:6px 8px; border-radius:6px; text-decoration:none; color:var(--text); font-size:12.5px; }
  .nav-item:hover { background:#eff6ff; color:var(--accent); }
  .nav-item small { color:var(--muted); font-size:11px; }
  main { flex:1; padding:24px 32px; max-width:1180px; }
  .hero { margin-bottom:20px; }
  .hero h1 { margin:0 0 6px; font-size:22px; }
  .hero p { margin:0; font-size:13px; color:var(--muted); }
  .card { background:var(--panel); border:1px solid var(--border); border-radius:10px; padding:16px 20px; margin-bottom:18px; box-shadow:0 1px 2px rgba(15,23,42,.04); }
  .card-head { display:flex; justify-content:space-between; align-items:baseline; border-bottom:1px solid var(--border); padding-bottom:8px; margin-bottom:10px; }
  .card-head h2 { margin:0; font-size:16px; }
  .meta { font-size:12px; color:var(--muted); }
  .desc { margin:0 0 10px; padding:6px 10px; background:#f1f5f9; border-left:3px solid var(--accent); font-size:13px; }
  table { width:100%; border-collapse:collapse; font-size:13px; }
  th, td { text-align:left; padding:7px 10px; border-bottom:1px solid var(--border); }
  th { background:#f8fafc; color:#475569; font-weight:600; }
  .mono { font-family:ui-monospace,SFMono-Regular,Consolas,monospace; font-size:12px; }
  .fw { font-weight:600; }
  .type { color:#0369a1; }
  .tc { text-align:center; width:56px; }
  .badge { display:inline-block; padding:1px 6px; border-radius:4px; font-size:11px; font-weight:600; }
  .badge.pk { background:#fef3c7; color:#b45309; }
  .badge.nn { background:#fee2e2; color:#b91c1c; }
  .sub-box { margin-top:10px; font-size:12.5px; color:#334155; }
  .sub-box ul { margin:4px 0 0; padding-left:18px; }
  @media print { aside { display:none; } main { padding:10px; } .card { break-inside:avoid; } }
</style>
</head>
<body>
  <aside>
    <h1>${escapeHtml(input.dbName)}</h1>
    <div class="sub">${escapeHtml(input.profileName)} · ${escapeHtml(engineDisplay)}</div>
    <input id="filterBox" type="search" placeholder="筛选表名..." />
    <nav id="navList">${navLinks}</nav>
  </aside>
  <main>
    <div class="hero">
      <h1>数据库数据字典 — ${escapeHtml(input.dbName)}</h1>
      <p>数据源: ${escapeHtml(input.profileName)} (${escapeHtml(engineDisplay)}) · ${input.tables.length} 个表/视图 · 生成于 ${escapeHtml(input.generatedAt)}</p>
    </div>
    ${sections}
    ${routineSection}
  </main>
  <script>
    const box = document.getElementById('filterBox');
    box && box.addEventListener('input', () => {
      const q = box.value.trim().toLowerCase();
      document.querySelectorAll('.nav-item, .tbl-section').forEach((el) => {
        const name = el.getAttribute('data-name') || '';
        el.style.display = !q || name.includes(q) ? '' : 'none';
      });
    });
  </script>
</body>
</html>`;
}

// ==================== 4. Value Decoders, Matrix Paste, Redis Tree, Filter & Diff ====================

export function decodeBase64Safe(input: string): { ok: boolean; text: string; error?: string } {
  const clean = input.trim();
  if (!clean) return { ok: true, text: '' };
  try {
    const normalized = clean.replace(/-/g, '+').replace(/_/g, '/');
    const pad = normalized.length % 4 === 0 ? '' : '='.repeat(4 - (normalized.length % 4));
    const bin = atob(normalized + pad);
    const bytes = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
    return { ok: true, text: new TextDecoder('utf-8', { fatal: false }).decode(bytes) };
  } catch (err) {
    return { ok: false, text: '', error: String(err) };
  }
}

export function encodeBase64Safe(input: string): string {
  const bytes = new TextEncoder().encode(input);
  let bin = '';
  for (let i = 0; i < bytes.length; i++) bin += String.fromCharCode(bytes[i]);
  return btoa(bin);
}

export function decodeUrlSafe(input: string): { ok: boolean; text: string; error?: string } {
  try {
    return { ok: true, text: decodeURIComponent(input.replace(/\+/g, '%20')) };
  } catch (err) {
    return { ok: false, text: '', error: String(err) };
  }
}

export function decodeHexToUtf8(input: string): { ok: boolean; text: string; bytes: Uint8Array; error?: string } {
  const clean = input.trim().replace(/^0x/i, '').replace(/[\s:_-]+/g, '');
  if (!clean || clean.length % 2 !== 0 || !/^[0-9a-fA-F]+$/.test(clean)) {
    return { ok: false, text: '', bytes: new Uint8Array(), error: '非合法偶数长度十六进制串' };
  }
  const bytes = new Uint8Array(clean.length / 2);
  for (let i = 0; i < bytes.length; i++) {
    bytes[i] = parseInt(clean.slice(i * 2, i * 2 + 2), 16);
  }
  return {
    ok: true,
    bytes,
    text: new TextDecoder('utf-8', { fatal: false }).decode(bytes),
  };
}

export function formatHexDump(input: string, maxBytes = 512): string {
  const hexAttempt = decodeHexToUtf8(input);
  const bytes = hexAttempt.ok
    ? hexAttempt.bytes.slice(0, maxBytes)
    : new TextEncoder().encode(input).slice(0, maxBytes);
  const lines: string[] = [];
  for (let offset = 0; offset < bytes.length; offset += 16) {
    const chunk = bytes.slice(offset, offset + 16);
    const addr = offset.toString(16).padStart(8, '0');
    const hexParts: string[] = [];
    let ascii = '';
    for (let i = 0; i < 16; i++) {
      if (i < chunk.length) {
        const b = chunk[i];
        hexParts.push(b.toString(16).padStart(2, '0').toUpperCase());
        ascii += b >= 0x20 && b <= 0x7e ? String.fromCharCode(b) : '.';
      } else {
        hexParts.push('  ');
        ascii += ' ';
      }
    }
    lines.push(`${addr}  ${hexParts.slice(0, 8).join(' ')}  ${hexParts.slice(8).join(' ')}  |${ascii}|`);
  }
  return lines.join('\n');
}

export function decodeJwtToken(input: string): {
  valid: boolean;
  header?: string;
  payload?: string;
  signature?: string;
  expHint?: string;
} {
  const parts = input.trim().split('.');
  if (parts.length !== 3 || parts.some((p) => !p)) return { valid: false };
  const h = decodeBase64Safe(parts[0]);
  const p = decodeBase64Safe(parts[1]);
  if (!h.ok || !p.ok) return { valid: false };
  try {
    const hObj = JSON.parse(h.text) as Record<string, unknown>;
    const pObj = JSON.parse(p.text) as Record<string, unknown>;
    let expHint: string | undefined;
    if (typeof pObj.exp === 'number' && Number.isFinite(pObj.exp)) {
      const dt = new Date(pObj.exp * 1000);
      const expired = dt.getTime() < Date.now();
      expHint = `${dt.toLocaleString()} (${expired ? '已过期' : '有效期内'})`;
    }
    return {
      valid: true,
      header: JSON.stringify(hObj, null, 2),
      payload: JSON.stringify(pObj, null, 2),
      signature: parts[2],
      expHint,
    };
  } catch {
    return { valid: false };
  }
}

export function detectImageDataUri(raw: string): string | null {
  const trimmed = raw.trim();
  if (!trimmed) return null;
  if (/^data:image\/(png|jpeg|jpg|gif|webp);base64,[A-Za-z0-9+/=]+$/i.test(trimmed)) {
    return trimmed;
  }
  const hexRes = decodeHexToUtf8(trimmed);
  if (!hexRes.ok || hexRes.bytes.length < 12) return null;
  const b = hexRes.bytes;
  let mime = '';
  if (b[0] === 0x89 && b[1] === 0x50 && b[2] === 0x4e && b[3] === 0x47) mime = 'image/png';
  else if (b[0] === 0xff && b[1] === 0xd8 && b[2] === 0xff) mime = 'image/jpeg';
  else if (b[0] === 0x47 && b[1] === 0x49 && b[2] === 0x46 && b[3] === 0x38) mime = 'image/gif';
  else if (b[0] === 0x52 && b[1] === 0x49 && b[2] === 0x46 && b[3] === 0x46 && b[8] === 0x57 && b[9] === 0x45) mime = 'image/webp';
  if (!mime) return null;
  let bin = '';
  for (let i = 0; i < b.length; i++) bin += String.fromCharCode(b[i]);
  return `data:${mime};base64,${btoa(bin)}`;
}

export function parseSpreadsheetPasteMatrix(text: string): string[][] {
  const norm = text.replace(/\r\n/g, '\n').replace(/\r/g, '\n');
  const trimmed = norm.endsWith('\n') ? norm.slice(0, -1) : norm;
  if (!trimmed) return [];
  const rows: string[][] = [];
  let row: string[] = [];
  let cell = '';
  let inQuotes = false;

  for (let i = 0; i < trimmed.length; i++) {
    const ch = trimmed[i];
    if (inQuotes) {
      if (ch === '"' && trimmed[i + 1] === '"') {
        cell += '"';
        i += 1;
      } else if (ch === '"') {
        inQuotes = false;
      } else {
        cell += ch;
      }
      continue;
    }
    if (ch === '"' && cell.length === 0) {
      inQuotes = true;
    } else if (ch === '\t') {
      row.push(cell);
      cell = '';
    } else if (ch === '\n') {
      row.push(cell);
      rows.push(row);
      row = [];
      cell = '';
    } else {
      cell += ch;
    }
  }
  row.push(cell);
  rows.push(row);
  return rows;
}

export interface RedisNamespaceNode {
  id: string;
  label: string;
  prefix: string;
  isFolder: boolean;
  keyCount: number;
  fullKey?: string;
  meta?: TableMeta;
  children: RedisNamespaceNode[];
}

export function buildRedisNamespaceTree(
  tables: TableMeta[],
  delimiter = ':',
): RedisNamespaceNode[] {
  interface MutableNode {
    id: string;
    label: string;
    prefix: string;
    isFolder: boolean;
    keyCount: number;
    fullKey?: string;
    meta?: TableMeta;
    folderMap: Map<string, MutableNode>;
    leaves: MutableNode[];
  }

  const rootFolders = new Map<string, MutableNode>();
  const rootLeaves: MutableNode[] = [];
  const delim = delimiter || ':';

  for (const tbl of tables) {
    const keyName = tbl.name;
    const parts = keyName.split(delim);
    if (parts.length <= 1) {
      rootLeaves.push({
        id: `k:${keyName}`,
        label: keyName,
        prefix: '',
        isFolder: false,
        keyCount: 1,
        fullKey: keyName,
        meta: tbl,
        folderMap: new Map(),
        leaves: [],
      });
      continue;
    }

    let curMap = rootFolders;
    let curLeaves = rootLeaves;
    let runningPrefix = '';
    for (let i = 0; i < parts.length; i++) {
      const seg = parts[i] || '(empty)';
      const isLast = i === parts.length - 1;
      if (isLast) {
        curLeaves.push({
          id: `k:${keyName}`,
          label: keyName,
          prefix: runningPrefix,
          isFolder: false,
          keyCount: 1,
          fullKey: keyName,
          meta: tbl,
          folderMap: new Map(),
          leaves: [],
        });
      } else {
        runningPrefix = runningPrefix ? `${runningPrefix}${delim}${seg}` : seg;
        let folder = curMap.get(seg);
        if (!folder) {
          folder = {
            id: `f:${runningPrefix}`,
            label: seg,
            prefix: `${runningPrefix}${delim}`,
            isFolder: true,
            keyCount: 0,
            folderMap: new Map(),
            leaves: [],
          };
          curMap.set(seg, folder);
        }
        folder.keyCount += 1;
        curMap = folder.folderMap;
        curLeaves = folder.leaves;
      }
    }
  }

  function finalize(folders: Map<string, MutableNode>, leaves: MutableNode[]): RedisNamespaceNode[] {
    const out: RedisNamespaceNode[] = [];
    const sortedFolders = [...folders.values()].sort((a, b) => a.label.localeCompare(b.label));
    for (const f of sortedFolders) {
      out.push({
        id: f.id,
        label: f.label,
        prefix: f.prefix,
        isFolder: true,
        keyCount: f.keyCount,
        children: finalize(f.folderMap, f.leaves),
      });
    }
    for (const l of leaves) {
      out.push({
        id: l.id,
        label: l.label,
        prefix: l.prefix,
        isFolder: false,
        keyCount: 1,
        fullKey: l.fullKey,
        meta: l.meta,
        children: [],
      });
    }
    return out;
  }

  return finalize(rootFolders, rootLeaves);
}

export type ColumnFilterOp = '=' | '!=' | 'contains' | 'not_contains' | '>' | '<' | 'is_null' | 'not_null';

export function matchesColumnFilter(rawVal: unknown, op: ColumnFilterOp, needle: string): boolean {
  const isNull = rawVal === null || rawVal === undefined;
  if (op === 'is_null') return isNull;
  if (op === 'not_null') return !isNull;
  if (isNull) return op === '!=' || op === 'not_contains';
  const text = typeof rawVal === 'object' ? JSON.stringify(rawVal) : String(rawVal);
  const lhsLow = text.toLowerCase();
  const rhsLow = needle.toLowerCase();
  if (op === 'contains') return lhsLow.includes(rhsLow);
  if (op === 'not_contains') return !lhsLow.includes(rhsLow);
  if (op === '=') return lhsLow === rhsLow;
  if (op === '!=') return lhsLow !== rhsLow;
  const ln = Number(text);
  const rn = Number(needle);
  if (Number.isFinite(ln) && Number.isFinite(rn) && needle.trim() !== '') {
    return op === '>' ? ln > rn : ln < rn;
  }
  return op === '>' ? text > needle : text < needle;
}

export function computeWatchResultDiff(
  prevRows: Array<Record<string, unknown>>,
  nextRows: Array<Record<string, unknown>>,
  columns: string[],
): {
  addedIndices: Set<number>;
  changedCells: Set<string>;
  addedCount: number;
  changedCount: number;
} {
  const addedIndices = new Set<number>();
  const changedCells = new Set<string>();
  let changedCount = 0;

  for (let r = 0; r < nextRows.length; r++) {
    if (r >= prevRows.length) {
      addedIndices.add(r);
      continue;
    }
    const prev = prevRows[r];
    const next = nextRows[r];
    for (const col of columns) {
      const a = prev[col] === null || prev[col] === undefined ? '' : typeof prev[col] === 'object' ? JSON.stringify(prev[col]) : String(prev[col]);
      const b = next[col] === null || next[col] === undefined ? '' : typeof next[col] === 'object' ? JSON.stringify(next[col]) : String(next[col]);
      if (a !== b) {
        changedCells.add(`${r}:${col}`);
        changedCount += 1;
      }
    }
  }
  return {
    addedIndices,
    changedCells,
    addedCount: addedIndices.size,
    changedCount,
  };
}
