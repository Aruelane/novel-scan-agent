//! Source-preserving novel import contracts.
//!
//! The public API deliberately distinguishes formats that are ready from formats
//! that are only planned. Callers can therefore render an honest capability list
//! instead of treating every recognised file extension as importable.
//!
//! ## Chapter ordinal mapping
//!
//! `ImportedChapter.index` uses one-based numbering (the first chapter is 1,
//! the second is 2, etc.) for human-facing display. When code needs a zero-based
//! ordinal (e.g. array indexing), subtract 1: `chapter.index - 1`.

#![forbid(unsafe_code)]

pub mod archive;
mod capability;
mod encoding;
mod error;
pub mod html;
pub mod markdown;
mod model;
mod plain_text;

pub use capability::{
    capability_for, capability_registry, detect_format, CapabilityStatus, FormatCapability,
    NovelFormat,
};
pub use encoding::{DecodedText, TextEncoding, TextEncodingHint};
pub use error::ImportError;
pub use model::{
    ChapterSplitMode, DocumentStats, ImportOptions, ImportRequest, ImportWarning,
    ImportWarningCode, ImportedChapter, ImportedDocument, SourceAnchor, SourceDescriptor,
    SourceLocator,
};

/// Imports a novel using the registered implementation for the detected format.
///
/// Known but unfinished formats return [`ImportError::PendingSupport`]. Unknown
/// formats return [`ImportError::UnsupportedFormat`]. This is intentionally more
/// explicit than a single catch-all "failed to open" result.
pub fn import_novel(request: ImportRequest<'_>) -> Result<ImportedDocument, ImportError> {
    let format = detect_format(request.source_name, request.media_type, request.bytes)?;
    let capability = capability_for(format);

    match capability.status {
        CapabilityStatus::Ready => match format {
            NovelFormat::PlainText | NovelFormat::Markdown => plain_text::import(request, format),
            _ => Err(ImportError::UnsupportedFormat {
                source_name: request.source_name.to_owned(),
                detail: format!(
                    "{} is marked ready but no importer is registered",
                    capability.label
                ),
            }),
        },
        CapabilityStatus::Pending => Err(ImportError::PendingSupport {
            format,
            detail: capability.detail.to_owned(),
        }),
        CapabilityStatus::Unsupported => Err(ImportError::UnsupportedFormat {
            source_name: request.source_name.to_owned(),
            detail: capability.detail.to_owned(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_is_explicit_about_every_requested_format() {
        let expected = [
            NovelFormat::PlainText,
            NovelFormat::Markdown,
            NovelFormat::Epub,
            NovelFormat::Docx,
            NovelFormat::Pdf,
            NovelFormat::Html,
            NovelFormat::Mobi,
            NovelFormat::Azw3,
            NovelFormat::Zip,
            NovelFormat::SevenZip,
        ];

        for format in expected {
            let capability = capability_for(format);
            assert_eq!(capability.format, format);
            assert!(!capability.extensions.is_empty());
            assert!(!capability.source_locator.is_empty());
        }

        assert_eq!(
            capability_for(NovelFormat::PlainText).status,
            CapabilityStatus::Ready
        );
        assert_eq!(
            capability_for(NovelFormat::Epub).status,
            CapabilityStatus::Pending
        );
    }

    #[test]
    fn known_but_unfinished_format_returns_pending_instead_of_fake_success() {
        let error = import_novel(ImportRequest::new("book.epub", b"PK\x03\x04")).unwrap_err();

        assert!(matches!(
            error,
            ImportError::PendingSupport {
                format: NovelFormat::Epub,
                ..
            }
        ));
    }

    #[test]
    fn unknown_binary_format_is_unsupported() {
        let error = import_novel(ImportRequest::new("book.bin", &[0, 159, 146, 150])).unwrap_err();

        assert!(matches!(error, ImportError::UnsupportedFormat { .. }));
    }

    #[test]
    fn imported_chapter_ordinals_are_one_based() {
        // When displaying chapter numbers to users, use chapter.index directly.
        // For zero-based array access, subtract 1.
        let request = ImportRequest::new(
            "demo.txt",
            b"\xef\xbb\xbf\xe7\xac\xac\xe4\xb8\x80\xe7\xab\xa0 \xe5\xbc\x80\xe5\xa4\xb4\n\xe6\xad\xa3\xe6\x96\x87\xe5\x86\x85\xe5\xae\xb9\xe3\x80\x82\n",
        );
        let doc = import_novel(request).unwrap();
        assert!(!doc.chapters.is_empty());
        // First chapter index must be 1 (one-based for display)
        assert_eq!(doc.chapters[0].index, 1);
        // Zero-based ordinal: chapter.index - 1 == 0 for the first chapter
        assert_eq!(doc.chapters[0].index - 1, 0);
    }
}
