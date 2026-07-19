use crate::{NovelFormat, TextEncoding, TextEncodingHint};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChapterSplitMode {
    /// TXT uses common novel headings; Markdown additionally uses ATX headings.
    Auto,
    /// Common headings such as “第一章”, “楔子”, and “Chapter 12”.
    NovelHeadings,
    /// Markdown `#` through `###` headings plus common novel headings.
    MarkdownHeadings,
    /// Preserve the entire input as one chapter.
    None,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ImportLimits {
    pub max_raw_bytes: usize,
    pub max_decoded_chars: usize,
    pub max_chapters: usize,
    pub max_archive_entries: usize,
    pub max_archive_entry_bytes: usize,
    pub max_archive_total_uncompressed: usize,
    pub max_xml_node_depth: usize,
    pub max_pdf_pages: usize,
}

impl Default for ImportLimits {
    fn default() -> Self {
        Self {
            max_raw_bytes: 50_000_000,
            max_decoded_chars: 20_000_000,
            max_chapters: 5_000,
            max_archive_entries: 10_000,
            max_archive_entry_bytes: 20_000_000,
            max_archive_total_uncompressed: 100_000_000,
            max_xml_node_depth: 100,
            max_pdf_pages: 5_000,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ImportOptions {
    pub encoding_hint: Option<TextEncodingHint>,
    pub chapter_split: ChapterSplitMode,
    pub limits: ImportLimits,
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            encoding_hint: None,
            chapter_split: ChapterSplitMode::Auto,
            limits: ImportLimits::default(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ImportRequest<'a> {
    pub source_name: &'a str,
    pub media_type: Option<&'a str>,
    pub bytes: &'a [u8],
    pub options: ImportOptions,
}

impl<'a> ImportRequest<'a> {
    pub fn new(source_name: &'a str, bytes: &'a [u8]) -> Self {
        Self {
            source_name,
            media_type: None,
            bytes,
            options: ImportOptions::default(),
        }
    }

    pub fn with_media_type(mut self, media_type: &'a str) -> Self {
        self.media_type = Some(media_type);
        self
    }

    pub fn with_options(mut self, options: ImportOptions) -> Self {
        self.options = options;
        self
    }
}

/// Platform-agnostic source reference. Windows paths and Android content://
/// URIs are never exposed to the frontend; only safe display names cross the
/// Tauri command boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceUri {
    /// Windows filesystem path (Rust layer only).
    WindowsPath(String),
    /// Android Storage Access Framework content URI (Rust layer only).
    AndroidContentUri(String),
    /// In-memory or test data with a display label.
    MemoryBuffer { display_name: String },
}

impl SourceUri {
    pub fn safe_display_name(&self) -> &str {
        match self {
            Self::WindowsPath(p) => std::path::Path::new(p)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown"),
            Self::AndroidContentUri(_) => "Android document",
            Self::MemoryBuffer { display_name } => display_name,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceDescriptor {
    pub display_name: String,
    pub format: NovelFormat,
    pub media_type: Option<String>,
    pub text_encoding: Option<TextEncoding>,
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceLocator {
    /// Half-open byte offsets refer to the decoded UTF-8 string, not necessarily
    /// the original file bytes (important for UTF-16 input).
    TextRange {
        line_start: usize,
        line_end: usize,
        decoded_byte_start: usize,
        decoded_byte_end: usize,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceAnchor {
    pub source_name: String,
    pub format: NovelFormat,
    pub chapter_index: Option<usize>,
    pub chapter_title: Option<String>,
    pub locator: SourceLocator,
}

impl SourceAnchor {
    pub fn citation_label(&self) -> String {
        let chapter = match (&self.chapter_index, &self.chapter_title) {
            (Some(index), Some(title)) => format!("第 {index} 节《{title}》"),
            (Some(index), None) => format!("第 {index} 节"),
            _ => "全文".to_owned(),
        };
        match self.locator {
            SourceLocator::TextRange {
                line_start,
                line_end,
                ..
            } if line_start == line_end => {
                format!("{} · {} · 第 {} 行", self.source_name, chapter, line_start)
            }
            SourceLocator::TextRange {
                line_start,
                line_end,
                ..
            } => format!(
                "{} · {} · 第 {}–{} 行",
                self.source_name, chapter, line_start, line_end
            ),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportedChapter {
    /// One-based order in the decoded document.
    pub index: usize,
    pub title: String,
    /// Exact decoded slice represented by `anchor`; line endings are preserved.
    pub text: String,
    pub anchor: SourceAnchor,
    pub heading_anchor: Option<SourceAnchor>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImportWarningCode {
    NoChapterHeadings,
    FrontMatterDetected,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportWarning {
    pub code: ImportWarningCode,
    pub message: String,
    pub anchor: Option<SourceAnchor>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentStats {
    pub chapter_count: usize,
    pub line_count: usize,
    pub character_count: usize,
    pub decoded_utf8_bytes: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportedDocument {
    pub source: SourceDescriptor,
    pub chapters: Vec<ImportedChapter>,
    pub warnings: Vec<ImportWarning>,
    pub stats: DocumentStats,
}
