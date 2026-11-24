use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufWriter, Cursor, Seek, Write};
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
    let mut cursor = Cursor::new(Vec::new());
    write_xlsx_to_writer(&mut cursor, sheet_name, columns, rows)?;
    Ok(cursor.into_inner())
}

/// Streams tabular data directly into an `.xlsx` file on disk via a buffered ZIP writer
/// without materializing the full worksheet XML in memory, returning exported row count.
pub fn write_xlsx_file(
    path: &Path,
    sheet_name: &str,
    columns: &[String],
    rows: &[BTreeMap<String, Value>],
) -> CoreResult<usize> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = File::create(path)?;
    let mut writer = BufWriter::with_capacity(64 * 1024, file);
    write_xlsx_to_writer(&mut writer, sheet_name, columns, rows)?;
    writer.flush()?;
    Ok(rows.len())
}

fn write_xlsx_to_writer<W: Write + Seek>(
    dest: W,
    sheet_name: &str,
    columns: &[String],
    rows: &[BTreeMap<String, Value>],
) -> CoreResult<()> {
    let safe_sheet = sanitize_sheet_name(sheet_name);
    let mut escaped_sheet_attr = String::with_capacity(safe_sheet.len());
    push_xml_attr_escaped(&mut escaped_sheet_attr, &safe_sheet);
    let workbook_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <sheets>
    <sheet name="{escaped_sheet_attr}" sheetId="1" r:id="rId1"/>
  </sheets>
</workbook>"#
    );

    let mut zw = ZipWriter::new(dest);
    let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    let static_parts: [(&str, &[u8]); 5] = [
        ("[Content_Types].xml", CONTENT_TYPES_XML.as_bytes()),
        ("_rels/.rels", ROOT_RELS_XML.as_bytes()),
        ("xl/workbook.xml", workbook_xml.as_bytes()),
        ("xl/_rels/workbook.xml.rels", WORKBOOK_RELS_XML.as_bytes()),
        ("xl/styles.xml", STYLES_XML.as_bytes()),
    ];

    for (name, data) in static_parts {
        zw.start_file(name, opts)
            .map_err(|e| CoreError::ExecutionFailed(format!("创建 Excel ZIP 条目 {name} 失败: {e}")))?;
        zw.write_all(data)
            .map_err(|e| CoreError::ExecutionFailed(format!("写入 Excel 条目 {name} 失败: {e}")))?;
    }

    zw.start_file("xl/worksheets/sheet1.xml", opts)
        .map_err(|e| CoreError::ExecutionFailed(format!("创建 Excel 工作表条目失败: {e}")))?;
    write_worksheet_xml_to(&mut zw, columns, rows)
        .map_err(|e| CoreError::ExecutionFailed(format!("流式写入 Excel 工作表失败: {e}")))?;

    zw.finish()
        .map_err(|e| CoreError::ExecutionFailed(format!("完成 Excel 工作簿打包失败: {e}")))?;
    Ok(())
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

fn write_worksheet_xml_to<W: Write>(
    writer: &mut W,
    columns: &[String],
    rows: &[BTreeMap<String, Value>],
) -> std::io::Result<()> {
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
                let w = (value_preview_width(val) + 3) as f64;
                if w > widths[c_idx] {
                    widths[c_idx] = w;
                }
            }
        }
    }

    writer.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetViews>
    <sheetView workbookViewId="0">
      <pane ySplit="1" topLeftCell="A2" activePane="bottomLeft" state="frozen"/>
    </sheetView>
  </sheetViews>
"#,
    )?;

    let mut buf = String::with_capacity(columns.len().max(1) * 96 + 128);
    if !columns.is_empty() {
        buf.push_str("  <cols>\n");
        for (idx, w) in widths.iter().enumerate() {
            let clamped = w.clamp(10.0, 48.0);
            let one_based = idx + 1;
            use std::fmt::Write as _;
            let _ = writeln!(
                buf,
                r#"    <col min="{one_based}" max="{one_based}" width="{clamped:.1}" customWidth="1"/>"#
            );
        }
        buf.push_str("  </cols>\n");
        writer.write_all(buf.as_bytes())?;
    }

    writer.write_all(b"  <sheetData>\n")?;

    if !columns.is_empty() {
        buf.clear();
        buf.push_str(r#"    <row r="1" ht="20" customHeight="1">"#);
        for (c_idx, col_name) in columns.iter().enumerate() {
            append_inline_str_cell(&mut buf, &col_letters[c_idx], 1, "1", col_name);
        }
        buf.push_str("</row>\n");
        writer.write_all(buf.as_bytes())?;

        for (r_idx, row) in rows.iter().enumerate() {
            let excel_row = r_idx + 2;
            buf.clear();
            use std::fmt::Write as _;
            let _ = write!(buf, r#"    <row r="{excel_row}">"#);
            for (c_idx, col_name) in columns.iter().enumerate() {
                if let Some(val) = row.get(col_name) {
                    if !val.is_null() {
                        encode_xlsx_cell(&mut buf, &col_letters[c_idx], excel_row, val);
                    }
                }
            }
            buf.push_str("</row>\n");
            writer.write_all(buf.as_bytes())?;
        }
    }

    writer.write_all(b"  </sheetData>\n</worksheet>")?;
    Ok(())
}

fn encode_xlsx_cell(out: &mut String, col_letter: &str, excel_row: usize, val: &Value) {
    use std::fmt::Write as _;
    match val {
        Value::Null => {}
        Value::Bool(b) => {
            let _ = write!(
                out,
                r#"<c r="{col_letter}{excel_row}" t="b"><v>{}</v></c>"#,
                if *b { '1' } else { '0' }
            );
        }
        Value::Number(n) => {
            let text = n.to_string();
            if is_long_or_scientific_number(&text) {
                append_inline_str_cell(out, col_letter, excel_row, "2", &text);
            } else {
                let _ = write!(
                    out,
                    r#"<c r="{col_letter}{excel_row}" t="n"><v>{text}</v></c>"#
                );
            }
        }
        Value::String(s) => {
            // All string values (including 19-digit BIGINT strings, phone numbers,
            // zero-padded IDs) are explicitly styled with Text format (s="2") so Excel
            // never reformats them into scientific notation or strips leading zeroes.
            append_inline_str_cell(out, col_letter, excel_row, "2", s);
        }
        Value::Array(_) | Value::Object(_) => {
            let json_str = serde_json::to_string(val).unwrap_or_default();
            append_inline_str_cell(out, col_letter, excel_row, "2", &json_str);
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

fn valid_xml10_char(c: char) -> char {
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
}

fn append_inline_str_cell(
    out: &mut String,
    col_letter: &str,
    excel_row: usize,
    style_idx: &str,
    raw_text: &str,
) {
    let mut first_char: Option<char> = None;
    let mut last_char: Option<char> = None;
    for raw_c in raw_text.chars() {
        let vc = valid_xml10_char(raw_c);
        if first_char.is_none() {
            first_char = Some(vc);
        }
        last_char = Some(vc);
    }
    let preserve = first_char.is_some_and(char::is_whitespace)
        || last_char.is_some_and(char::is_whitespace);
    let space_attr = if preserve {
        r#" xml:space="preserve""#
    } else {
        ""
    };
    use std::fmt::Write as _;
    let _ = write!(
        out,
        r#"<c r="{col_letter}{excel_row}" s="{style_idx}" t="inlineStr"><is><t{space_attr}>"#
    );
    for raw_c in raw_text.chars() {
        match valid_xml10_char(raw_c) {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            ch => out.push(ch),
        }
    }
    out.push_str("</t></is></c>");
}

fn push_xml_attr_escaped(out: &mut String, text: &str) {
    for raw_c in text.chars() {
        match valid_xml10_char(raw_c) {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            ch => out.push(ch),
        }
    }
}

fn display_width(text: &str) -> usize {
    let first_line = text.lines().next().unwrap_or("");
    let mut width = 0usize;
    for ch in first_line.chars().take(64) {
        width += if ch.is_ascii() { 1 } else { 2 };
    }
    width
}

fn value_preview_width(val: &Value) -> usize {
    match val {
        Value::Null => 0,
        Value::Bool(true) => 4,
        Value::Bool(false) => 5,
        Value::Number(n) => n.to_string().len().min(64),
        Value::String(s) => display_width(s),
        Value::Array(_) | Value::Object(_) => {
            let s = serde_json::to_string(val).unwrap_or_default();
            display_width(&s)
        }
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

    #[test]
    fn test_write_xlsx_file_streams_to_disk_and_preserves_long_integers() {
        let tmp = std::env::temp_dir().join(format!(
            "pacergrip_xlsx_stream_{}.xlsx",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&tmp);

        let cols = vec!["id".to_string(), "snowflake".to_string()];
        let mut rows = Vec::with_capacity(1200);
        for i in 1..=1200 {
            let mut r = BTreeMap::new();
            r.insert("id".to_string(), json!(i));
            r.insert(
                "snowflake".to_string(),
                json!(1_892_345_678_901_230_000i64 + i as i64),
            );
            rows.push(r);
        }

        let written = write_xlsx_file(&tmp, "Orders", &cols, &rows).unwrap();
        assert_eq!(written, 1200);

        let file = File::open(&tmp).unwrap();
        let mut archive = ZipArchive::new(file).unwrap();
        let mut sheet_xml = String::new();
        archive
            .by_name("xl/worksheets/sheet1.xml")
            .unwrap()
            .read_to_string(&mut sheet_xml)
            .unwrap();
        assert!(sheet_xml.contains(
            r#"<c r="B1201" s="2" t="inlineStr"><is><t>1892345678901231200</t></is></c>"#
        ));
        let _ = std::fs::remove_file(&tmp);
    }
}
