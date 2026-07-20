//! Text-based PDF import using lopdf.
//! Encrypted/password-protected PDFs are rejected. Scanning-based PDFs
//! with insufficient text return OcrRequired.

use crate::{
    DocumentStats, ImportError, ImportRequest, ImportedChapter, ImportedDocument, NovelFormat,
    SourceAnchor, SourceDescriptor, SourceLocator,
};

/// Public entry point matching the `plain_text::import` contract.
pub(crate) fn import(
    request: ImportRequest<'_>,
    format: NovelFormat,
) -> Result<ImportedDocument, ImportError> {
    let chapters = import_pdf(request.bytes, request.source_name)?;
    let total_chars: usize = chapters.iter().map(|c| c.text.chars().count()).sum();
    Ok(ImportedDocument {
        source: SourceDescriptor {
            display_name: request.source_name.to_owned(),
            format,
            media_type: Some("application/pdf".to_owned()),
            text_encoding: None,
        },
        stats: DocumentStats {
            chapter_count: chapters.len(),
            line_count: chapters.iter().map(|c| c.text.lines().count()).sum(),
            character_count: total_chars,
            decoded_utf8_bytes: total_chars,
        },
        chapters,
        warnings: Vec::new(),
    })
}

pub(crate) fn import_pdf(
    bytes: &[u8],
    source_name: &str,
) -> Result<Vec<ImportedChapter>, ImportError> {
    let doc = lopdf::Document::load_mem(bytes).map_err(|e| {
        if format!("{e}").contains("password") || format!("{e}").contains("encrypt") {
            ImportError::Protected {
                source_name: source_name.to_owned(),
                detail: "PDF 已加密或受密码保护".into(),
            }
        } else {
            ImportError::Corrupt {
                source_name: source_name.to_owned(),
                detail: format!("无法解析 PDF：{e}"),
            }
        }
    })?;

    let mut total_text = String::new();
    let mut chapter_idx = 0usize;
    let page_count = doc.get_pages().len();

    for (page_num, page_id) in doc.get_pages() {
        let page_num_usize = page_num as usize;
        if let Ok(text) = doc.extract_text(&[page_num as u32]) {
            let trimmed = text.trim().to_owned();
            if !trimmed.is_empty() {
                if !total_text.is_empty() {
                    total_text.push('\n');
                }
                total_text.push_str(&trimmed);
            }
        }
    }

    let text = total_text.trim().to_owned();
    if text.is_empty() {
        return Err(ImportError::OcrRequired {
            source_name: source_name.to_owned(),
            detail: "PDF 无可提取的文本层，可能需要 OCR".into(),
        });
    }

    // Create chapters: one per page with text, or single chapter for short PDFs
    let mut chapters = Vec::new();
    if page_count <= 3 {
        chapter_idx = 1;
        chapters.push(ImportedChapter {
            index: 1,
            title: source_name.to_owned(),
            text: text.clone(),
            anchor: SourceAnchor {
                source_name: source_name.to_owned(),
                format: NovelFormat::Pdf,
                chapter_index: Some(1),
                chapter_title: Some(format!("{source_name} (全 {page_count} 页)")),
                locator: SourceLocator::TextRange {
                    line_start: 1,
                    line_end: text.lines().count().max(1),
                    decoded_byte_start: 0,
                    decoded_byte_end: text.len(),
                },
            },
            heading_anchor: None,
        });
    } else {
        for (page_num, page_id) in doc.get_pages() {
            if let Ok(page_text) = doc.extract_text(&[page_num as u32]) {
                let trimmed = page_text.trim().to_owned();
                if !trimmed.is_empty() {
                    chapter_idx += 1;
                    chapters.push(ImportedChapter {
                        index: chapter_idx,
                        title: format!("第 {page_num} 页"),
                        text: trimmed.clone(),
                        anchor: SourceAnchor {
                            source_name: source_name.to_owned(),
                            format: NovelFormat::Pdf,
                            chapter_index: Some(chapter_idx),
                            chapter_title: Some(format!("第 {page_num} 页")),
                            locator: SourceLocator::TextRange {
                                line_start: 1,
                                line_end: trimmed.lines().count().max(1),
                                decoded_byte_start: 0,
                                decoded_byte_end: trimmed.len(),
                            },
                        },
                        heading_anchor: None,
                    });
                }
            }
        }
    }

    if chapters.is_empty() {
        return Err(ImportError::OcrRequired {
            source_name: source_name.to_owned(),
            detail: "PDF 无可提取的文本层".into(),
        });
    }

    Ok(chapters)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_bytes_rejected() {
        assert!(import_pdf(b"", "empty.pdf").is_err());
    }

    #[test]
    fn garbage_bytes_rejected() {
        assert!(import_pdf(b"not a pdf file", "bad.pdf").is_err());
    }
}
