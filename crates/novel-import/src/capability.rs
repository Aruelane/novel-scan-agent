use crate::ImportError;

/// File/container formats recognised by the import boundary.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum NovelFormat {
    PlainText,
    Markdown,
    Epub,
    Docx,
    Pdf,
    Html,
    Mobi,
    Azw3,
    Zip,
    SevenZip,
    LegacyDoc,
}

impl NovelFormat {
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::PlainText => "txt",
            Self::Markdown => "markdown",
            Self::Epub => "epub",
            Self::Docx => "docx",
            Self::Pdf => "pdf",
            Self::Html => "html",
            Self::Mobi => "mobi",
            Self::Azw3 => "azw3",
            Self::Zip => "zip",
            Self::SevenZip => "7z",
            Self::LegacyDoc => "doc",
        }
    }
}

/// Product-facing support status. `Pending` is not importable yet.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CapabilityStatus {
    Ready,
    Pending,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FormatCapability {
    pub format: NovelFormat,
    pub label: &'static str,
    pub extensions: &'static [&'static str],
    pub media_types: &'static [&'static str],
    pub status: CapabilityStatus,
    pub detail: &'static str,
    /// The evidence locator that a future importer must preserve.
    pub source_locator: &'static str,
}

const CAPABILITIES: &[FormatCapability] = &[
    FormatCapability {
        format: NovelFormat::PlainText,
        label: "纯文本",
        extensions: &["txt"],
        media_types: &["text/plain"],
        status: CapabilityStatus::Ready,
        detail: "支持 UTF-8（含 BOM）和带 BOM 的 UTF-16；GBK/GB18030 会明确提示尚待解码器支持",
        source_locator: "章节 + 原文行号 + 解码后 UTF-8 字节范围",
    },
    FormatCapability {
        format: NovelFormat::Markdown,
        label: "Markdown",
        extensions: &["md", "markdown", "mdown", "mkd"],
        media_types: &["text/markdown", "text/x-markdown"],
        status: CapabilityStatus::Ready,
        detail: "支持 Markdown 标题和常见网文章节标题切分",
        source_locator: "标题/章节 + 原文行号 + 解码后 UTF-8 字节范围",
    },
    FormatCapability {
        format: NovelFormat::Epub,
        label: "EPUB",
        extensions: &["epub"],
        media_types: &["application/epub+zip"],
        status: CapabilityStatus::Pending,
        detail: "已识别格式，解析 spine、目录与 EPUB CFI 的导入器将在后续阶段接入",
        source_locator: "spine 项 + 资源路径 + EPUB CFI/段落",
    },
    FormatCapability {
        format: NovelFormat::Docx,
        label: "Word DOCX",
        extensions: &["docx"],
        media_types: &["application/vnd.openxmlformats-officedocument.wordprocessingml.document"],
        status: CapabilityStatus::Pending,
        detail: "已识别格式，标题样式、段落与页眉脚注解析将在后续阶段接入",
        source_locator: "标题层级 + 段落序号",
    },
    FormatCapability {
        format: NovelFormat::Pdf,
        label: "PDF",
        extensions: &["pdf"],
        media_types: &["application/pdf"],
        status: CapabilityStatus::Pending,
        detail: "已识别格式；文本层解析优先，扫描版 OCR 将作为独立能力标注，避免伪装成可读取",
        source_locator: "页码 + 文本块/坐标范围",
    },
    FormatCapability {
        format: NovelFormat::Html,
        label: "HTML",
        extensions: &["html", "htm", "xhtml"],
        media_types: &["text/html", "application/xhtml+xml"],
        status: CapabilityStatus::Pending,
        detail: "已识别格式，正文抽取、标题层级和 DOM 锚点将在后续阶段接入",
        source_locator: "标题 + DOM 路径/文本范围",
    },
    FormatCapability {
        format: NovelFormat::Mobi,
        label: "MOBI",
        extensions: &["mobi", "prc"],
        media_types: &["application/x-mobipocket-ebook"],
        status: CapabilityStatus::Pending,
        detail: "已识别格式，PalmDB/MOBI 内容与目录解析将在后续阶段接入",
        source_locator: "内容节 + 电子书 location",
    },
    FormatCapability {
        format: NovelFormat::Azw3,
        label: "Kindle AZW3",
        extensions: &["azw3", "azw"],
        media_types: &["application/vnd.amazon.ebook"],
        status: CapabilityStatus::Pending,
        detail: "已识别格式；仅计划支持无 DRM 文件，加密文件会明确拒绝",
        source_locator: "内容节 + Kindle location",
    },
    FormatCapability {
        format: NovelFormat::Zip,
        label: "ZIP 压缩包",
        extensions: &["zip"],
        media_types: &["application/zip", "application/x-zip-compressed"],
        status: CapabilityStatus::Pending,
        detail: "已识别容器，后续将安全枚举文件并把条目路径叠加到内部格式锚点",
        source_locator: "压缩包条目路径 + 内部文档锚点",
    },
    FormatCapability {
        format: NovelFormat::SevenZip,
        label: "7Z 压缩包",
        extensions: &["7z"],
        media_types: &["application/x-7z-compressed"],
        status: CapabilityStatus::Pending,
        detail: "已识别容器，后续将安全枚举文件并明确处理密码包和资源上限",
        source_locator: "压缩包条目路径 + 内部文档锚点",
    },
    FormatCapability {
        format: NovelFormat::LegacyDoc,
        label: "旧版 Word DOC",
        extensions: &["doc"],
        media_types: &["application/msword"],
        status: CapabilityStatus::Unsupported,
        detail: "旧版二进制 DOC 当前不支持；请另存为 DOCX、TXT 或 PDF",
        source_locator: "不适用",
    },
];

pub fn capability_registry() -> &'static [FormatCapability] {
    CAPABILITIES
}

pub fn capability_for(format: NovelFormat) -> &'static FormatCapability {
    CAPABILITIES
        .iter()
        .find(|capability| capability.format == format)
        .expect("every NovelFormat must have a capability entry")
}

/// Detects a format without claiming that it can already be imported.
///
/// Detection order:
/// 1. Strong binary signatures (ZIP header, PDF header, 7Z header, BOOKMOBI) —
///    these win over any extension or media type.
/// 2. After ZIP is detected, the extension/media type sub-classifies it:
///    - `.epub` / `application/epub+zip` → Epub
///    - `.docx` / `application/vnd.openxmlformats-officedocument.wordprocessingml.document` → Docx
///    - everything else → Zip
/// 3. Media-type lookup (for non-ZIP binaries whose signature didn't match).
/// 4. Extension lookup (for non-ZIP binaries).
/// 5. Heuristic text detection (HTML, decodable text).
pub fn detect_format(
    source_name: &str,
    media_type: Option<&str>,
    bytes: &[u8],
) -> Result<NovelFormat, ImportError> {
    // ── 1. Strong binary signatures ──

    let is_zip_container = bytes.starts_with(&[0x50, 0x4b, 0x03, 0x04])
        || bytes.starts_with(&[0x50, 0x4b, 0x05, 0x06])
        || bytes.starts_with(&[0x50, 0x4b, 0x07, 0x08]);

    if bytes.starts_with(b"%PDF-") {
        return Ok(NovelFormat::Pdf);
    }
    if bytes.starts_with(&[0x37, 0x7a, 0xbc, 0xaf, 0x27, 0x1c]) {
        return Ok(NovelFormat::SevenZip);
    }
    if bytes.len() >= 68 && &bytes[60..68] == b"BOOKMOBI" {
        return Ok(match extension(source_name).as_deref() {
            Some("azw") | Some("azw3") => NovelFormat::Azw3,
            _ => NovelFormat::Mobi,
        });
    }

    // ── 2. ZIP container subclassification ──

    if is_zip_container {
        // Check media type first
        if let Some(media_type) = media_type {
            let bare_media_type = media_type
                .split(';')
                .next()
                .unwrap_or(media_type)
                .trim()
                .to_ascii_lowercase();
            match bare_media_type.as_str() {
                "application/epub+zip" => return Ok(NovelFormat::Epub),
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => {
                    return Ok(NovelFormat::Docx)
                }
                _ => {}
            }
        }

        // Then check extension
        if let Some(ext) = extension(source_name) {
            match ext.as_str() {
                "epub" => return Ok(NovelFormat::Epub),
                "docx" => return Ok(NovelFormat::Docx),
                _ => {}
            }
        }

        // ZIP bytes but no EPUB/DOCX extension or media type → honest Zip
        return Ok(NovelFormat::Zip);
    }

    // ── 3. Media-type lookup (non-ZIP) ──

    if let Some(media_type) = media_type {
        let bare_media_type = media_type
            .split(';')
            .next()
            .unwrap_or(media_type)
            .trim()
            .to_ascii_lowercase();
        if let Some(format) = CAPABILITIES.iter().find_map(|capability| {
            capability
                .media_types
                .iter()
                .any(|candidate| *candidate == bare_media_type)
                .then_some(capability.format)
        }) {
            return Ok(format);
        }
    }

    // ── 4. Extension lookup (non-ZIP) ──

    if let Some(extension) = extension(source_name) {
        if let Some(format) = CAPABILITIES.iter().find_map(|capability| {
            capability
                .extensions
                .iter()
                .any(|candidate| *candidate == extension)
                .then_some(capability.format)
        }) {
            return Ok(format);
        }
    }

    // ── 5. Heuristic text detection ──

    let trimmed = trim_ascii_prefix(bytes);
    if starts_ascii_case_insensitive(trimmed, b"<!doctype html")
        || starts_ascii_case_insensitive(trimmed, b"<html")
    {
        return Ok(NovelFormat::Html);
    }

    if looks_like_decodable_text(bytes) {
        return Ok(NovelFormat::PlainText);
    }

    Err(ImportError::UnsupportedFormat {
        source_name: source_name.to_owned(),
        detail: "无法识别文件格式；当前不会把未知二进制文件当作文本读取".to_owned(),
    })
}

fn extension(source_name: &str) -> Option<String> {
    let file_name = source_name
        .rsplit(|character| matches!(character, '/' | '\\'))
        .next()
        .unwrap_or(source_name)
        .split(|character| matches!(character, '?' | '#'))
        .next()
        .unwrap_or(source_name);
    let (_, extension) = file_name.rsplit_once('.')?;
    (!extension.is_empty()).then(|| extension.to_ascii_lowercase())
}

fn trim_ascii_prefix(mut bytes: &[u8]) -> &[u8] {
    if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
        bytes = &bytes[3..];
    }
    while bytes
        .first()
        .is_some_and(|byte| matches!(*byte, b' ' | b'\t' | b'\r' | b'\n'))
    {
        bytes = &bytes[1..];
    }
    bytes
}

fn starts_ascii_case_insensitive(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.len() >= needle.len() && haystack[..needle.len()].eq_ignore_ascii_case(needle)
}

fn looks_like_decodable_text(bytes: &[u8]) -> bool {
    if bytes.starts_with(&[0xef, 0xbb, 0xbf])
        || bytes.starts_with(&[0xff, 0xfe])
        || bytes.starts_with(&[0xfe, 0xff])
    {
        return true;
    }
    let Ok(text) = std::str::from_utf8(bytes) else {
        return false;
    };
    let character_count = text.chars().count();
    if character_count == 0 {
        return true;
    }
    let suspicious_controls = text
        .chars()
        .filter(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
        .count();
    suspicious_controls.saturating_mul(100) <= character_count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_case_insensitive_extensions_and_parameterized_media_types() {
        assert_eq!(
            detect_format("BOOK.MarkDown", None, b"# title").unwrap(),
            NovelFormat::Markdown
        );
        assert_eq!(
            detect_format(
                "download",
                Some("text/plain; charset=utf-8"),
                "第一章".as_bytes()
            )
            .unwrap(),
            NovelFormat::PlainText
        );
    }

    #[test]
    fn strong_binary_signature_wins_over_a_misleading_extension() {
        assert_eq!(
            detect_format("not-really.txt", None, b"%PDF-1.7").unwrap(),
            NovelFormat::Pdf
        );
    }

    #[test]
    fn detects_html_without_an_extension() {
        assert_eq!(
            detect_format("download", None, b"  <!DOCTYPE HTML><html></html>").unwrap(),
            NovelFormat::Html
        );
    }

    #[test]
    fn detects_an_unlabelled_zip_without_treating_it_as_text() {
        assert_eq!(
            detect_format("download", None, b"PK\x05\x06\0\0\0\0").unwrap(),
            NovelFormat::Zip
        );
    }

    #[test]
    fn rejects_nul_heavy_utf8_binary_data() {
        let error = detect_format("download", None, b"\0\0\0\0binary").unwrap_err();
        assert!(matches!(error, ImportError::UnsupportedFormat { .. }));
    }

    #[test]
    fn zip_bytes_with_misleading_txt_extension_is_zip_not_plaintext() {
        // ZIP header + ".txt" extension → Zip, not PlainText
        assert_eq!(
            detect_format("misleading.txt", None, b"PK\x03\x04\0\0\0\0").unwrap(),
            NovelFormat::Zip
        );
    }

    #[test]
    fn zip_bytes_with_text_plain_media_type_is_zip() {
        // ZIP header + text/plain media type → Zip
        assert_eq!(
            detect_format("download", Some("text/plain"), b"PK\x03\x04\0\0\0\0").unwrap(),
            NovelFormat::Zip
        );
    }

    #[test]
    fn zip_bytes_with_epub_extension_is_epub() {
        assert_eq!(
            detect_format("book.epub", None, b"PK\x03\x04\0\0\0\0").unwrap(),
            NovelFormat::Epub
        );
    }

    #[test]
    fn zip_bytes_with_docx_extension_is_docx() {
        assert_eq!(
            detect_format("book.docx", None, b"PK\x03\x04\0\0\0\0").unwrap(),
            NovelFormat::Docx
        );
    }

    #[test]
    fn pdf_bytes_with_txt_extension_still_pdf() {
        // Strong binary signature (PDF) wins over misleading .txt extension
        assert_eq!(
            detect_format("not-really.txt", None, b"%PDF-1.4 rest").unwrap(),
            NovelFormat::Pdf
        );
    }
}
