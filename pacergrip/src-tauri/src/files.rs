use std::path::Path;

use pacer_core::{CoreResult, FileStatsDto, OpenedFileDto, WorkspaceFileEntry};
use pacer_sql_ast::extract_markdown_sql_blocks;

/// Opens a local text/SQL/Markdown file and enriches Markdown files with executable SQL blocks.
pub fn open_text_file(
    path: &Path,
    encoding_override: Option<&str>,
) -> CoreResult<OpenedFileDto> {
    let mut dto = pacer_core::files::open_text_file(path, encoding_override)?;
    if dto.file_kind == "markdown" {
        dto.sql_blocks = extract_markdown_sql_blocks(&dto.content);
    }
    Ok(dto)
}

/// Saves editor text back to disk using the requested encoding, BOM setting, and line ending.
pub fn save_text_file(
    path: &Path,
    content: &str,
    encoding: &str,
    line_ending: &str,
    with_bom: bool,
) -> CoreResult<FileStatsDto> {
    pacer_core::files::save_text_file(path, content, encoding, line_ending, with_bom)
}

/// Lists direct children of a local directory for the Pacer Files Explorer tool window.
pub fn list_workspace_files(dir_path: &Path) -> CoreResult<Vec<WorkspaceFileEntry>> {
    pacer_core::files::list_workspace_files(dir_path)
}

/// Writes a sample Chinese Markdown SQL playbook tailored to the Demo SQLite Store.
pub fn ensure_demo_markdown_file(storage_dir: &Path) -> CoreResult<OpenedFileDto> {
    std::fs::create_dir_all(storage_dir)?;
    let path = storage_dir.join("示例查询手册.md");
    if !path.is_file() {
        let sample = r#"# Pacer SQL 运维与数据分析手册

> 提示：在 Markdown 文档中，你可以：
> 1. 直接点击每个 SQL 代码块上方的 **「▶ 执行」** 或 **「⚡ EXPLAIN」** 按钮；
> 2. 用鼠标**选中任意一段 SQL**，按 `Ctrl+Enter` 即可在当前选定的数据库中执行；
> 3. 查看底部状态栏的 **文件行数、光标行列、UTF-8/GBK 编码、CRLF/LF 行尾符**，点击编码可直接重载或转码保存。

---

## 1. 客户与订单联表查询

```sql
SELECT
  c.full_name AS 客户姓名,
  c.tier AS 客户等级,
  o.order_no AS 订单编号,
  o.total_amount AS 订单金额,
  o.status AS 订单状态,
  o.ordered_at AS 下单时间
FROM orders o
LEFT JOIN customers c ON o.customer_id = c.id
ORDER BY o.total_amount DESC;
```

## 2. 按商品类目汇总销售额与销量

```sql
SELECT
  p.category AS 商品类目,
  COUNT(DISTINCT oi.order_id) AS 订单数,
  SUM(oi.quantity) AS 总销量,
  ROUND(SUM(oi.quantity * oi.unit_price), 2) AS 销售总额
FROM order_items oi
INNER JOIN products p ON oi.product_id = p.id
GROUP BY p.category
ORDER BY 销售总额 DESC;
```

## 3. 查看 VIP 客户及其消费明细

```sql
SELECT
  c.full_name,
  c.email,
  COUNT(o.id) AS order_count,
  COALESCE(SUM(o.total_amount), 0) AS total_spent
FROM customers c
LEFT JOIN orders o ON c.id = o.customer_id
WHERE c.tier IN ('VIP', 'ENTERPRISE')
GROUP BY c.id, c.full_name, c.email;
```
"#;
        std::fs::write(&path, sample.as_bytes())?;
    }
    open_text_file(&path, None)
}
