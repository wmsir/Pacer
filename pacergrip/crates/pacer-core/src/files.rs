use std::path::Path;

use chardetng::EncodingDetector;
use encoding_rs::{Encoding, GB18030, UTF_16BE, UTF_16LE, UTF_8, WINDOWS_1252};

use crate::{CoreError, CoreResult, FileStatsDto, OpenedFileDto, WorkspaceFileEntry};

/// Opens a local text/SQL/Markdown file, auto-detecting BOM/encoding, line endings, and line stats.
pub fn open_text_file(
    path: &Path,
    encoding_override: Option<&str>,
) -> CoreResult<OpenedFileDto> {
    if !path.is_file() {
        return Err(CoreError::InvalidConfig(format!(
            "文件不存在: {}",
            path.display()
        )));
    }

    let raw_bytes = std::fs::read(path)?;
    let byte_size = raw_bytes.len() as u64;

    let (decoded, encoding_label, has_bom) = decode_bytes(&raw_bytes, encoding_override)?;
    let line_ending = detect_line_ending(&decoded);
    let normalized = decoded.replace("\r\n", "\n").replace('\r', "\n");

    let line_count = if normalized.is_empty() {
        1
    } else {
        normalized.split('\n').count()
    };
    let char_count = normalized.chars().count();

    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("untitled")
        .to_string();
    let file_kind = infer_file_kind(path);

    Ok(OpenedFileDto {
        path: path.to_string_lossy().replace('\\', "/"),
        name,
        content: normalized,
        encoding: encoding_label,
        has_bom,
        line_ending,
        line_count,
        char_count,
        byte_size,
        file_kind,
        sql_blocks: Vec::new(),
    })
}

/// Saves editor text back to disk using the requested encoding, BOM setting, and line ending.
pub fn save_text_file(
    path: &Path,
    content: &str,
    encoding: &str,
    line_ending: &str,
    with_bom: bool,
) -> CoreResult<FileStatsDto> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let normalized_lf = content.replace("\r\n", "\n").replace('\r', "\n");
    let effective_eol = if line_ending.eq_ignore_ascii_case("CRLF") {
        "CRLF"
    } else {
        "LF"
    };
    let final_text = if effective_eol == "CRLF" {
        normalized_lf.replace('\n', "\r\n")
    } else {
        normalized_lf.clone()
    };

    let (encoded_bytes, normalized_enc, effective_bom) =
        encode_text(&final_text, encoding, with_bom);

    std::fs::write(path, &encoded_bytes)?;

    let line_count = if normalized_lf.is_empty() {
        1
    } else {
        normalized_lf.split('\n').count()
    };

    Ok(FileStatsDto {
        path: path.to_string_lossy().replace('\\', "/"),
        encoding: normalized_enc,
        has_bom: effective_bom,
        line_ending: effective_eol.to_string(),
        line_count,
        char_count: normalized_lf.chars().count(),
        byte_size: encoded_bytes.len() as u64,
    })
}

/// Lists direct children of a local directory for the Pacer Files Explorer tool window.
pub fn list_workspace_files(dir_path: &Path) -> CoreResult<Vec<WorkspaceFileEntry>> {
    if !dir_path.is_dir() {
        return Err(CoreError::InvalidConfig(format!(
            "目录不存在: {}",
            dir_path.display()
        )));
    }

    let mut entries = Vec::new();
    for item in std::fs::read_dir(dir_path)? {
        let Ok(entry) = item else {
            continue;
        };
        let name = entry.file_name().to_string_lossy().into_owned();
        if matches!(
            name.as_str(),
            ".git" | "node_modules" | "target" | "dist-win" | ".idea"
        ) {
            continue;
        }
        let path = entry.path();
        let meta = entry.metadata().ok();
        let is_dir = meta.as_ref().is_some_and(|m| m.is_dir());
        let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
        let extension = if is_dir {
            None
        } else {
            path.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_ascii_lowercase())
        };
        entries.push(WorkspaceFileEntry {
            path: path.to_string_lossy().replace('\\', "/"),
            name,
            is_dir,
            size,
            extension,
        });
    }

    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase()))
    });
    Ok(entries)
}

fn decode_bytes(
    bytes: &[u8],
    encoding_override: Option<&str>,
) -> CoreResult<(String, String, bool)> {
    if let Some(override_name) = encoding_override.map(str::trim).filter(|s| !s.is_empty()) {
        let (enc, label, strip_bom) = resolve_encoding_spec(override_name);
        let (slice, has_bom) = strip_matching_bom(bytes, enc);
        let input = if strip_bom || has_bom { slice } else { bytes };
        let (decoded, _, _) = enc.decode(input);
        return Ok((decoded.into_owned(), label, has_bom));
    }

    // 1. Exact BOM detection first.
    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        let (decoded, _, _) = UTF_8.decode(rest);
        return Ok((decoded.into_owned(), "UTF-8 BOM".to_string(), true));
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        let (decoded, _, _) = UTF_16LE.decode(rest);
        return Ok((decoded.into_owned(), "UTF-16 LE".to_string(), true));
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        let (decoded, _, _) = UTF_16BE.decode(rest);
        return Ok((decoded.into_owned(), "UTF-16 BE".to_string(), true));
    }

    // 2. Strict valid UTF-8 check prevents false positives on Chinese UTF-8 files.
    if let Ok(valid_utf8) = std::str::from_utf8(bytes) {
        return Ok((valid_utf8.to_string(), "UTF-8".to_string(), false));
    }

    // 3. Fallback statistical encoding detection (GBK/GB18030, Shift_JIS, Windows-1252, etc.).
    let mut detector = EncodingDetector::new();
    detector.feed(bytes, true);
    let detected = detector.guess(None, true);
    let (decoded, _, _) = detected.decode(bytes);
    let label = friendly_encoding_name(detected);
    Ok((decoded.into_owned(), label, false))
}

fn encode_text(text: &str, encoding: &str, with_bom: bool) -> (Vec<u8>, String, bool) {
    let norm = encoding.trim().to_ascii_uppercase();
    if norm == "UTF-16 LE" || norm == "UTF-16LE" {
        let mut out = Vec::with_capacity(text.len() * 2 + 2);
        if with_bom {
            out.extend_from_slice(&[0xFF, 0xFE]);
        }
        for unit in text.encode_utf16() {
            out.extend_from_slice(&unit.to_le_bytes());
        }
        return (out, "UTF-16 LE".to_string(), with_bom);
    }
    if norm == "UTF-16 BE" || norm == "UTF-16BE" {
        let mut out = Vec::with_capacity(text.len() * 2 + 2);
        if with_bom {
            out.extend_from_slice(&[0xFE, 0xFF]);
        }
        for unit in text.encode_utf16() {
            out.extend_from_slice(&unit.to_be_bytes());
        }
        return (out, "UTF-16 BE".to_string(), with_bom);
    }
    if norm == "GBK" || norm == "GB18030" || norm == "GB2312" {
        let (cow, _, _) = GB18030.encode(text);
        return (cow.into_owned(), "GBK".to_string(), false);
    }
    if norm == "WINDOWS-1252" || norm == "ISO-8859-1" || norm == "LATIN1" {
        let (cow, _, _) = WINDOWS_1252.encode(text);
        return (cow.into_owned(), "Windows-1252".to_string(), false);
    }

    let effective_bom = with_bom || norm == "UTF-8 BOM";
    let mut out = Vec::with_capacity(text.len() + 3);
    if effective_bom {
        out.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
    }
    out.extend_from_slice(text.as_bytes());
    let label = if effective_bom {
        "UTF-8 BOM".to_string()
    } else {
        "UTF-8".to_string()
    };
    (out, label, effective_bom)
}

fn resolve_encoding_spec(name: &str) -> (&'static Encoding, String, bool) {
    match name.trim().to_ascii_uppercase().as_str() {
        "UTF-8 BOM" | "UTF8-BOM" => (UTF_8, "UTF-8 BOM".to_string(), true),
        "UTF-8" | "UTF8" => (UTF_8, "UTF-8".to_string(), false),
        "GBK" | "GB2312" | "GB18030" => (GB18030, "GBK".to_string(), false),
        "UTF-16 LE" | "UTF-16LE" | "UTF16LE" => (UTF_16LE, "UTF-16 LE".to_string(), true),
        "UTF-16 BE" | "UTF-16BE" | "UTF16BE" => (UTF_16BE, "UTF-16 BE".to_string(), true),
        "WINDOWS-1252" | "ISO-8859-1" | "LATIN1" => {
            (WINDOWS_1252, "Windows-1252".to_string(), false)
        }
        other => {
            if let Some(enc) = Encoding::for_label(other.as_bytes()) {
                (enc, friendly_encoding_name(enc), false)
            } else {
                (UTF_8, "UTF-8".to_string(), false)
            }
        }
    }
}

fn strip_matching_bom<'a>(bytes: &'a [u8], enc: &'static Encoding) -> (&'a [u8], bool) {
    if enc == UTF_8 {
        if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
            return (rest, true);
        }
    } else if enc == UTF_16LE {
        if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
            return (rest, true);
        }
    } else if enc == UTF_16BE {
        if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
            return (rest, true);
        }
    }
    (bytes, false)
}

fn friendly_encoding_name(enc: &'static Encoding) -> String {
    match enc.name() {
        "UTF-8" => "UTF-8".to_string(),
        "gb18030" | "GBK" => "GBK".to_string(),
        "UTF-16LE" => "UTF-16 LE".to_string(),
        "UTF-16BE" => "UTF-16 BE".to_string(),
        "windows-1252" => "Windows-1252".to_string(),
        other => other.to_string(),
    }
}

fn detect_line_ending(text: &str) -> String {
    let mut crlf = 0usize;
    let mut lf = 0usize;
    let bytes = text.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'\r' && bytes.get(i + 1) == Some(&b'\n') {
            crlf += 1;
            i += 2;
            continue;
        }
        if bytes[i] == b'\n' {
            lf += 1;
        }
        i += 1;
    }
    if crlf > 0 && lf > 0 {
        "MIXED".to_string()
    } else if crlf > 0 {
        "CRLF".to_string()
    } else {
        "LF".to_string()
    }
}

fn infer_file_kind(path: &Path) -> String {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "md" | "markdown" | "mdx" => "markdown".to_string(),
        "sql" | "ddl" | "dml" | "psql" | "pls" => "sql".to_string(),
        "json" => "json".to_string(),
        "xml" => "xml".to_string(),
        _ => "text".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gbk_utf8_bom_and_utf16le_file_roundtrip() {
        let dir = std::env::temp_dir().join(format!("pacergrip_file_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        // 1. GBK Chinese Markdown file with CRLF
        let gbk_path = dir.join("gbk_doc.md");
        let content = "# 中文测试文档\n\n```sql\nSELECT '你好' AS greeting;\n```\n";
        let stats = save_text_file(&gbk_path, content, "GBK", "CRLF", false).unwrap();
        assert_eq!(stats.encoding, "GBK");
        assert_eq!(stats.line_ending, "CRLF");

        let opened = open_text_file(&gbk_path, None).unwrap();
        assert_eq!(opened.encoding, "GBK");
        assert_eq!(opened.line_ending, "CRLF");
        assert_eq!(opened.file_kind, "markdown");
        assert!(opened.content.contains("中文测试文档"));

        // 2. UTF-8 BOM SQL file with LF
        let bom_path = dir.join("bom_script.sql");
        save_text_file(&bom_path, "SELECT 1;\nSELECT 2;\n", "UTF-8", "LF", true).unwrap();
        let opened_bom = open_text_file(&bom_path, None).unwrap();
        assert_eq!(opened_bom.encoding, "UTF-8 BOM");
        assert!(opened_bom.has_bom);
        assert_eq!(opened_bom.line_ending, "LF");
        assert_eq!(opened_bom.line_count, 3);

        // 3. UTF-16 LE file with BOM
        let u16_path = dir.join("unicode.sql");
        save_text_file(&u16_path, "SELECT '北京';\n", "UTF-16 LE", "CRLF", true).unwrap();
        let opened_u16 = open_text_file(&u16_path, None).unwrap();
        assert_eq!(opened_u16.encoding, "UTF-16 LE");
        assert!(opened_u16.has_bom);
        assert!(opened_u16.content.contains("北京"));

        // 4. Directory listing
        let entries = list_workspace_files(&dir).unwrap();
        assert_eq!(entries.len(), 3);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
