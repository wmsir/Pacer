use std::path::{Component, Path, PathBuf};

use chardetng::EncodingDetector;
use encoding_rs::{Encoding, GB18030, UTF_16BE, UTF_16LE, UTF_8, WINDOWS_1252};

use crate::{CoreError, CoreResult, FileStatsDto, OpenedFileDto, WorkspaceFileEntry};

pub const MAX_EDITOR_FILE_BYTES: u64 = 16 * 1024 * 1024;

/// Rejects empty paths, NUL bytes, and parent directory traversal (`..`) components.
pub fn reject_traversal_components(path: &Path) -> CoreResult<()> {
    let raw = path.as_os_str().to_string_lossy();
    if raw.trim().is_empty() {
        return Err(CoreError::InvalidConfig("路径不能为空".to_string()));
    }
    if raw.contains('\0') {
        return Err(CoreError::InvalidConfig("路径包含非法空字符".to_string()));
    }
    for comp in path.components() {
        if matches!(comp, Component::ParentDir) {
            return Err(CoreError::PermissionDenied(format!(
                "安全限制：拒绝包含 '..' 目录遍历的路径: {}",
                path.display()
            )));
        }
    }
    Ok(())
}

/// Canonicalizes an existing file or directory path after rejecting traversal components.
pub fn canonicalize_existing_path(path: &Path) -> CoreResult<PathBuf> {
    reject_traversal_components(path)?;
    std::fs::canonicalize(path).map_err(|e| {
        CoreError::InvalidConfig(format!("无法解析目标路径 ({}): {e}", path.display()))
    })
}

/// Canonicalizes a path whose leaf (or newly created subdirectories) may not exist yet by
/// canonicalizing the nearest existing ancestor and validating all trailing components.
pub fn canonicalize_target_path(path: &Path, must_exist: bool) -> CoreResult<PathBuf> {
    if must_exist {
        return canonicalize_existing_path(path);
    }
    reject_traversal_components(path)?;
    if path.exists() {
        return canonicalize_existing_path(path);
    }

    let mut suffix_parts: Vec<std::ffi::OsString> = Vec::new();
    let mut cursor = path;
    loop {
        if cursor.exists() {
            let base = canonicalize_existing_path(cursor)?;
            let mut out = base;
            for part in suffix_parts.iter().rev() {
                out.push(part);
            }
            return Ok(out);
        }
        let Some(name) = cursor.file_name() else {
            break;
        };
        suffix_parts.push(name.to_os_string());
        let Some(parent) = cursor.parent() else {
            break;
        };
        if parent.as_os_str().is_empty() {
            // Relative path under current directory: resolve `.` as base.
            let cwd = std::env::current_dir().map_err(|e| {
                CoreError::InvalidConfig(format!("无法解析当前工作目录: {e}"))
            })?;
            let base = canonicalize_existing_path(&cwd)?;
            let mut out = base;
            for part in suffix_parts.iter().rev() {
                out.push(part);
            }
            return Ok(out);
        }
        cursor = parent;
    }

    Err(CoreError::InvalidConfig(format!(
        "无法解析目标父目录: {}",
        path.display()
    )))
}

fn strip_verbatim_prefix(path: &Path) -> String {
    let s = path.to_string_lossy().replace('\\', "/");
    if let Some(rest) = s.strip_prefix("//?/UNC/") {
        format!("//{rest}")
    } else if let Some(rest) = s.strip_prefix("//?/") {
        rest.to_string()
    } else {
        s
    }
}

/// Checks whether `canonical_target` lies within any canonicalized `canonical_roots`
/// (case-insensitive on Windows to match NTFS volume semantics).
pub fn is_path_within_any_root(canonical_target: &Path, canonical_roots: &[PathBuf]) -> bool {
    let target_norm = strip_verbatim_prefix(canonical_target);
    for root in canonical_roots {
        let root_norm = strip_verbatim_prefix(root);
        let root_trimmed = root_norm.trim_end_matches('/');
        if root_trimmed.is_empty() {
            continue;
        }
        #[cfg(target_os = "windows")]
        {
            if target_norm.eq_ignore_ascii_case(root_trimmed) {
                return true;
            }
            let prefix = format!("{}/", root_trimmed.to_ascii_lowercase());
            if target_norm.to_ascii_lowercase().starts_with(&prefix) {
                return true;
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            if target_norm == root_trimmed {
                return true;
            }
            let prefix = format!("{root_trimmed}/");
            if target_norm.starts_with(&prefix) {
                return true;
            }
        }
    }
    false
}

/// Canonicalizes `path` and enforces that it resides under one of `allowed_roots`.
pub fn ensure_path_within_roots(
    path: &Path,
    allowed_roots: &[PathBuf],
    must_exist: bool,
) -> CoreResult<PathBuf> {
    let canonical_target = canonicalize_target_path(path, must_exist)?;
    let canonical_roots: Vec<PathBuf> = allowed_roots
        .iter()
        .filter_map(|r| canonicalize_existing_path(r).ok())
        .collect();
    if !is_path_within_any_root(&canonical_target, &canonical_roots) {
        return Err(CoreError::PermissionDenied(format!(
            "安全限制：拒绝访问未授权目录之外的路径 ({})",
            path.display()
        )));
    }
    Ok(canonical_target)
}

/// Ensures a file is small enough to edit safely in the browser WebView editor.
pub fn validate_editor_file_size(byte_size: u64) -> CoreResult<()> {
    if byte_size > MAX_EDITOR_FILE_BYTES {
        let mb = byte_size as f64 / (1024.0 * 1024.0);
        return Err(CoreError::InvalidConfig(format!(
            "文件体积过大 ({mb:.1} MB，编辑器安全上限 16 MB)，直接载入可能导致内存飙升。若是大型 SQL 脚本，请改用「运行本地 SQL 文件」流式执行。"
        )));
    }
    Ok(())
}

/// Opens a local text/SQL/Markdown file, auto-detecting BOM/encoding, line endings, and line stats.
pub fn open_text_file(
    path: &Path,
    encoding_override: Option<&str>,
) -> CoreResult<OpenedFileDto> {
    let canonical_path = canonicalize_existing_path(path)?;
    let meta = std::fs::metadata(&canonical_path)?;
    if !meta.is_file() {
        return Err(CoreError::InvalidConfig(format!(
            "文件不存在或不是常规文件: {}",
            path.display()
        )));
    }

    validate_editor_file_size(meta.len())?;

    let raw_bytes = std::fs::read(&canonical_path)?;
    let byte_size = raw_bytes.len() as u64;
    validate_editor_file_size(byte_size)?;

    let (decoded, encoding_label, has_bom) = decode_bytes(&raw_bytes, encoding_override)?;
    drop(raw_bytes);
    let line_ending = detect_line_ending(&decoded);
    let normalized = if decoded.contains('\r') {
        decoded.replace("\r\n", "\n").replace('\r', "\n")
    } else {
        decoded
    };

    let line_count = if normalized.is_empty() {
        1
    } else {
        normalized.split('\n').count()
    };
    let char_count = normalized.chars().count();

    let name = canonical_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("untitled")
        .to_string();
    let file_kind = infer_file_kind(&canonical_path);

    Ok(OpenedFileDto {
        path: strip_verbatim_prefix(&canonical_path),
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
    let canonical_path = canonicalize_target_path(path, false)?;
    if let Some(parent) = canonical_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let normalized_lf = if content.contains('\r') {
        content.replace("\r\n", "\n").replace('\r', "\n")
    } else {
        content.to_string()
    };
    let effective_eol = if line_ending.eq_ignore_ascii_case("CRLF") {
        "CRLF"
    } else {
        "LF"
    };
    let (encoded_bytes, normalized_enc, effective_bom) = if effective_eol == "CRLF" {
        let final_text = normalized_lf.replace('\n', "\r\n");
        encode_text(&final_text, encoding, with_bom)
    } else {
        encode_text(&normalized_lf, encoding, with_bom)
    };

    std::fs::write(&canonical_path, &encoded_bytes)?;

    let line_count = if normalized_lf.is_empty() {
        1
    } else {
        normalized_lf.split('\n').count()
    };

    Ok(FileStatsDto {
        path: strip_verbatim_prefix(&canonical_path),
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
    let canonical_dir = canonicalize_existing_path(dir_path)?;
    if !canonical_dir.is_dir() {
        return Err(CoreError::InvalidConfig(format!(
            "目录不存在: {}",
            dir_path.display()
        )));
    }

    let mut entries = Vec::new();
    for item in std::fs::read_dir(&canonical_dir)? {
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
            path: strip_verbatim_prefix(&path),
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

    #[test]
    fn test_validate_editor_file_size_blocks_oversized_files() {
        assert!(validate_editor_file_size(1024).is_ok());
        assert!(validate_editor_file_size(MAX_EDITOR_FILE_BYTES).is_ok());
        assert!(validate_editor_file_size(MAX_EDITOR_FILE_BYTES + 1).is_err());
    }

    #[test]
    fn test_path_canonicalization_blocks_traversal_and_escape() {
        let base = std::env::temp_dir().join(format!("pacergrip_path_guard_{}", std::process::id()));
        let allowed = base.join("allowed");
        let outside = base.join("outside");
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&allowed).unwrap();
        std::fs::create_dir_all(&outside).unwrap();

        let inside_file = allowed.join("query.sql");
        std::fs::write(&inside_file, b"SELECT 1;").unwrap();
        let outside_file = outside.join("secret.txt");
        std::fs::write(&outside_file, b"secret").unwrap();

        assert!(ensure_path_within_roots(&inside_file, &[allowed.clone()], true).is_ok());
        assert!(ensure_path_within_roots(&allowed.join("new_sub/export.csv"), &[allowed.clone()], false).is_ok());

        let traversal = allowed.join("../outside/secret.txt");
        assert!(ensure_path_within_roots(&traversal, &[allowed.clone()], true).is_err());
        assert!(ensure_path_within_roots(&outside_file, &[allowed], true).is_err());

        let _ = std::fs::remove_dir_all(&base);
    }
}
