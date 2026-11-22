use std::collections::BTreeMap;
use std::io::{Cursor, Write};
use std::path::Path;

use serde_json::Value;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use pacer_core::{CoreError, CoreResult};

const CONTENT_TYPES_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
  <Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
  <Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>
</Types>"#;

const ROOT_RELS_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
</Relationships>"#;

const WORKBOOK_RELS_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
</Relationships>"#;

const STYLES_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <fonts count="2">
    <font>
      <sz val="11"/>
      <name val="Calibri"/>
      <family val="2"/>
    </font>
    <font>
      <b/>
      <sz val="11"/>
      <color rgb="FF1E293B"/>
      <name val="Calibri"/>
      <family val="2"/>
    </font>
  </fonts>
  <fills count="3">
    <fill><patternFill patternType="none"/></fill>
    <fill><patternFill patternType="gray125"/></fill>
    <fill>
      <patternFill patternType="solid">
        <fgColor rgb="FFEAF1FB"/>
        <bgColor indexed="64"/>
      </patternFill>
    </fill>
  </fills>
  <borders count="2">
    <border><left/><right/><top/><bottom/><diagonal/></border>
    <border>
      <left/>
      <right/>
      <top/>
      <bottom style="thin"><color rgb="FFCBD5E1"/></bottom>
      <diagonal/>
    </border>
  </borders>
  <cellStyleXfs count="1">
    <xf numFmtId="0" fontId="0" fillId="0" borderId="0"/>
  </cellStyleXfs>
  <cellXfs count="3">
    <xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>
    <xf numFmtId="0" fontId="1" fillId="2" borderId="1" xfId="0" applyFont="1" applyFill="1" applyBorder="1" applyAlignment="1">
      <alignment vertical="center"/>
    </xf>
    <xf numFmtId="49" fontId="0" fillId="0" borderId="0" xfId="0" applyNumberFormat="1"/>
  </cellXfs>
  <cellStyles count="1">
    <cellStyle name="Normal" xfId="0" builtinId="0"/>
  </cellStyles>
</styleSheet>"#;

/// Builds a genuine OOXML `.xlsx` workbook in memory.
///
/// - Strings and integers with >= 11 digits (e.g. phone numbers, order IDs, Snowflake IDs,
///   19-digit `BIGINT` values) are encoded explicitly as Excel Text cells (`s="2"` with
///   `numFmtId="49"` `@` and `t="inlineStr"`), completely preventing scientific notation
///   (`1.89E+18`) and floating-point precision loss.
pub fn build_xlsx_bytes(
    sheet_name: &str,
    columns: &[String],
    rows: &[BTreeMap<String, Value>],
) -> CoreResult<Vec<u8>> {
    let safe_sheet = sanitize_sheet_name(sheet_name);
    let workbook_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <sheets>
    <sheet name="{}" sheetId="1" r:id="rId1"/>
  </sheets>
</workbook>"#,
        xml_escape_attr(&safe_sheet)
    );

    let sheet_xml = build_worksheet_xml(columns, rows);

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zw = ZipWriter::new(&mut cursor);
        let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

        let parts: [(&str, &[u8]); 6] = [
            ("[Content_Types].xml", CONTENT_TYPES_XML.as_bytes()),
            ("_rels/.rels", ROOT_RELS_XML.as_bytes()),
            ("xl/workbook.xml", workbook_xml.as_bytes()),
            ("xl/_rels/workbook.xml.rels", WORKBOOK_RELS_XML.as_bytes()),
            ("xl/styles.xml", STYLES_XML.as_bytes()),
            ("xl/worksheets/sheet1.xml", sheet_xml.as_bytes()),
        ];

        for (name, data) in parts {
            zw.start_file(name, opts)
                .map_err(|e| CoreError::ExecutionFailed(format!("创建 Excel ZIP 条目 {name} 失败: {e}")))?;
            zw.write_all(data)
                .map_err(|e| CoreError::ExecutionFailed(format!("写入 Excel 条目 {name} 失败: {e}")))?;
        }

        zw.finish()
            .map_err(|e| CoreError::ExecutionFailed(format!("完成 Excel 工作簿打包失败: {e}")))?;
    }

    Ok(cursor.into_inner())
}

/// Writes tabular data to an `.xlsx` file on disk and returns the exported data row count.
pub fn write_xlsx_file(
    path: &Path,
    sheet_name: &str,
    columns: &[String],
    rows: &[BTreeMap<String, Value>],
) -> CoreResult<usize> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let bytes = build_xlsx_bytes(sheet_name, columns, rows)?;
    std::fs::write(path, bytes)?;
    Ok(rows.len())
}

fn sanitize_sheet_name(raw: &str) -> String {
    let mut cleaned: String = raw
        .trim()
        .chars()
        .filter(|c| !matches!(c, '\\' | '/' | '?' | '*' | '[' | ']' | ':'))
        .collect();
    while cleaned.starts_with('\'') {
        cleaned.remove(0);
    }
    while cleaned.ends_with('\'') {
        cleaned.pop();
    }
    let truncated: String = cleaned.trim().chars().take(31).collect();
    if truncated.is_empty() {
        "Sheet1".to_string()
    } else {
        truncated
    }
}

/// Converts a 0-based column index into an Excel column reference (`0 -> A`, `25 -> Z`, `26 -> AA`).
pub fn col_index_to_excel_letters(mut index: usize) -> String {
    let mut rev = Vec::new();
    loop {
        let rem = index % 26;
        rev.push((b'A' + rem as u8) as char);
        if index < 26 {
            break;
        }
        index = index / 26 - 1;
    }
    rev.into_iter().rev().collect()
}

fn build_worksheet_xml(columns: &[String], rows: &[BTreeMap<String, Value>]) -> String {
    let col_letters: Vec<String> = (0..columns.len())
        .map(col_index_to_excel_letters)
        .collect();

    // Auto-fit column widths (sampling up to first 500 rows for speed).
    let mut widths: Vec<f64> = columns
        .iter()
        .map(|c| (display_width(c) + 4) as f64)
        .collect();

    for row in rows.iter().take(500) {
        for (c_idx, col) in columns.iter().enumerate() {
            if let Some(val) = row.get(col) {
                let s = value_preview_str(val);
                let w = (display_width(&s) + 3) as f64;
                if w > widths[c_idx] {
                    widths[c_idx] = w;
                }
            }
        }
    }

    let mut out = String::with_capacity(2048 + rows.len() * columns.len().max(1) * 40);
    out.push_str(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetViews>
    <sheetView workbookViewId="0">
      <pane ySplit="1" topLeftCell="A2" activePane="bottomLeft" state="frozen"/>
    </sheetView>
  </sheetViews>
"#,
    );

    if !columns.is_empty() {
        out.push_str("  <cols>\n");
        for (idx, w) in widths.iter().enumerate() {
            let clamped = w.clamp(10.0, 48.0);
            let one_based = idx + 1;
            use std::fmt::Write as _;
            let _ = writeln!(
                out,
                r#"    <col min="{one_based}" max="{one_based}" width="{clamped:.1}" customWidth="1"/>"#
            );
        }
        out.push_str("  </cols>\n");
    }

    out.push_str("  <sheetData>\n");

    if !columns.is_empty() {
        out.push_str(r#"    <row r="1" ht="20" customHeight="1">"#);
        for (c_idx, col_name) in columns.iter().enumerate() {
            let cell_ref = format!("{}1", col_letters[c_idx]);
            append_inline_str_cell(&mut out, &cell_ref, "1", col_name);
        }
        out.push_str("</row>\n");

        for (r_idx, row) in rows.iter().enumerate() {
            let excel_row = r_idx + 2;
            use std::fmt::Write as _;
            let _ = write!(out, r#"    <row r="{excel_row}">"#);
            for (c_idx, col_name) in columns.iter().enumerate() {
                if let Some(val) = row.get(col_name) {
                    if !val.is_null() {
                        let cell_ref = format!("{}{}", col_letters[c_idx], excel_row);
                        encode_xlsx_cell(&mut out, &cell_ref, val);
                    }
                }
            }
            out.push_str("</row>\n");
        }
    }

    out.push_str("  </sheetData>\n</worksheet>");
    out
}

fn encode_xlsx_cell(out: &mut String, cell_ref: &str, val: &Value) {
    match val {
        Value::Null => {}
        Value::Bool(b) => {
            use std::fmt::Write as _;
            let _ = write!(
                out,
                r#"<c r="{cell_ref}" t="b"><v>{}</v></c>"#,
                if *b { '1' } else { '0' }
            );
        }
        Value::Number(n) => {
            let text = n.to_string();
            if is_long_or_scientific_number(&text) {
                append_inline_str_cell(out, cell_ref, "2", &text);
            } else {
                use std::fmt::Write as _;
                let _ = write!(out, r#"<c r="{cell_ref}" t="n"><v>{text}</v></c>"#);
            }
        }
        Value::String(s) => {
            // All string values (including 19-digit BIGINT strings, phone numbers,
            // zero-padded IDs) are explicitly styled with Text format (s="2") so Excel
            // never reformats them into scientific notation or strips leading zeroes.
            append_inline_str_cell(out, cell_ref, "2", s);
        }
        Value::Array(_) | Value::Object(_) => {
            let json_str = serde_json::to_string(val).unwrap_or_default();
            append_inline_str_cell(out, cell_ref, "2", &json_str);
        }
    }
}

/// Returns true if a numeric string should be written as Text in Excel to prevent
/// scientific notation (`1.89E+18`) or precision truncation.
/// Excel switches General numbers to scientific notation around >= 11-12 digits.
fn is_long_or_scientific_number(num_str: &str) -> bool {
    if num_str.contains('e') || num_str.contains('E') {
        return true;
    }
    let unsigned = num_str.strip_prefix('-').unwrap_or(num_str);
    let digit_count = unsigned.bytes().filter(u8::is_ascii_digit).count();
    digit_count >= 11
}

fn append_inline_str_cell(out: &mut String, cell_ref: &str, style_idx: &str, raw_text: &str) {
    let cleaned = sanitize_xml10_text(raw_text);
    let preserve = cleaned.starts_with(char::is_whitespace) || cleaned.ends_with(char::is_whitespace);
    let space_attr = if preserve { r#" xml:space="preserve""# } else { "" };
    let escaped = xml_escape_text(&cleaned);
    use std::fmt::Write as _;
    let _ = write!(
        out,
        r#"<c r="{cell_ref}" s="{style_idx}" t="inlineStr"><is><t{space_attr}>{escaped}</t></is></c>"#
    );
}

fn sanitize_xml10_text(raw: &str) -> String {
    raw.chars()
        .map(|c| {
            let code = c as u32;
            let valid = code == 0x9
                || code == 0xA
                || code == 0xD
                || (0x20..=0xD7FF).contains(&code)
                || (0xE000..=0xFFFD).contains(&code)
                || (0x10000..=0x10FFFF).contains(&code);
            if valid {
                c
            } else {
                ' '
            }
        })
        .collect()
}

fn xml_escape_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(ch),
        }
    }
    out
}

fn xml_escape_attr(text: &str) -> String {
    let cleaned = sanitize_xml10_text(text);
    let mut out = String::with_capacity(cleaned.len());
    for ch in cleaned.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}

fn display_width(text: &str) -> usize {
    let first_line = text.lines().next().unwrap_or("");
    let mut width = 0usize;
    for ch in first_line.chars().take(64) {
        width += if ch.is_ascii() { 1 } else { 2 };
    }
    width
}

fn value_preview_str(val: &Value) -> String {
    match val {
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(_) | Value::Object(_) => serde_json::to_string(val).unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use serde_json::json;
    use zip::ZipArchive;

    use super::*;

    #[test]
    fn test_col_index_to_excel_letters() {
        assert_eq!(col_index_to_excel_letters(0), "A");
        assert_eq!(col_index_to_excel_letters(25), "Z");
        assert_eq!(col_index_to_excel_letters(26), "AA");
        assert_eq!(col_index_to_excel_letters(27), "AB");
        assert_eq!(col_index_to_excel_letters(701), "ZZ");
        assert_eq!(col_index_to_excel_letters(702), "AAA");
    }

    #[test]
    fn test_xlsx_prevents_scientific_notation_for_long_integers_and_keeps_ooxml_valid() {
        let columns = vec![
            "id".to_string(),
            "snowflake_str".to_string(),
            "snowflake_num".to_string(),
            "mobile".to_string(),
            "zero_padded".to_string(),
            "amount".to_string(),
            "xml_note".to_string(),
        ];

        let mut row = BTreeMap::new();
        row.insert("id".to_string(), json!(42));
        row.insert("snowflake_str".to_string(), json!("1892345678901234567"));
        row.insert(
            "snowflake_num".to_string(),
            json!(9007199254740993i64),
        );
        row.insert("mobile".to_string(), json!(13800138000i64));
        row.insert("zero_padded".to_string(), json!("00123456"));
        row.insert("amount".to_string(), json!(199.5));
        row.insert("xml_note".to_string(), json!("  <script>&\"test\"\0  "));

        let bytes = build_xlsx_bytes("Users/Data:Export*1", &columns, &[row]).unwrap();
        let mut archive = ZipArchive::new(Cursor::new(bytes)).unwrap();

        let mut names = Vec::new();
        for i in 0..archive.len() {
            names.push(archive.by_index(i).unwrap().name().to_string());
        }
        assert!(names.contains(&"[Content_Types].xml".to_string()));
        assert!(names.contains(&"xl/workbook.xml".to_string()));
        assert!(names.contains(&"xl/styles.xml".to_string()));
        assert!(names.contains(&"xl/worksheets/sheet1.xml".to_string()));

        let mut sheet_xml = String::new();
        archive
            .by_name("xl/worksheets/sheet1.xml")
            .unwrap()
            .read_to_string(&mut sheet_xml)
            .unwrap();

        // Short numeric values stay numeric
        assert!(sheet_xml.contains(r#"<c r="A2" t="n"><v>42</v></c>"#));
        assert!(sheet_xml.contains(r#"<c r="F2" t="n"><v>199.5</v></c>"#));

        // 11+ digit numbers & strings are written explicitly with text style (s="2") and inlineStr
        assert!(sheet_xml.contains(
            r#"<c r="B2" s="2" t="inlineStr"><is><t>1892345678901234567</t></is></c>"#
        ));
        assert!(sheet_xml.contains(
            r#"<c r="C2" s="2" t="inlineStr"><is><t>9007199254740993</t></is></c>"#
        ));
        assert!(sheet_xml.contains(
            r#"<c r="D2" s="2" t="inlineStr"><is><t>13800138000</t></is></c>"#
        ));
        assert!(sheet_xml.contains(
            r#"<c r="E2" s="2" t="inlineStr"><is><t>00123456</t></is></c>"#
        ));
        assert!(!sheet_xml.contains("E+"));
        assert!(!sheet_xml.contains("e+"));

        // XML special chars & NUL control byte sanitized cleanly
        assert!(sheet_xml.contains(r#"xml:space="preserve""#));
        assert!(sheet_xml.contains("&lt;script&gt;&amp;\"test\"   </t>"));
        assert!(!sheet_xml.contains('\0'));

        let mut wb_xml = String::new();
        archive
            .by_name("xl/workbook.xml")
            .unwrap()
            .read_to_string(&mut wb_xml)
            .unwrap();
        assert!(wb_xml.contains(r#"name="UsersDataExport1""#));
    }
}
