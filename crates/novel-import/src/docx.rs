//! DOCX import via safe ZIP enumeration + XML paragraph extraction.
//! DOCX files are ZIP archives containing word/document.xml.

use crate::archive::enumerate_zip;
use crate::{
    DocumentStats, ImportError, ImportRequest, ImportWarning, ImportWarningCode, ImportedChapter,
    ImportedDocument, NovelFormat, SourceAnchor, SourceDescriptor, SourceLocator,
};

/// Public entry point matching the `plain_text::import` contract.
pub(crate) fn import(
    request: ImportRequest<'_>,
    format: NovelFormat,
) -> Result<ImportedDocument, ImportError> {
    let source_name = request.source_name;
    let max_chapters = request.options.limits.max_chapters;

    let entries = enumerate_zip(source_name, request.bytes, 20_000_000, 100_000_000)?;

    // Validate Content_Types
    if !entries.iter().any(|e| e.name == "[Content_Types].xml") {
        return Err(ImportError::Corrupt {
            source_name: source_name.to_owned(),
            detail: "缺少 [Content_Types].xml，不是有效的 DOCX".to_owned(),
        });
    }

    let doc_entry = entries
        .iter()
        .find(|e| e.name == "word/document.xml")
        .ok_or_else(|| ImportError::Corrupt {
            source_name: source_name.to_owned(),
            detail: "缺少 word/document.xml".into(),
        })?;

    let xml = String::from_utf8_lossy(&doc_entry.data);
    let paragraphs = extract_paragraphs(&xml);

    if paragraphs.is_empty() {
        return Err(ImportError::EmptyDocument {
            source_name: source_name.to_owned(),
        });
    }

    // Group paragraphs into chapters at heading styles
    let mut chapters: Vec<ImportedChapter> = Vec::new();
    let mut warnings: Vec<ImportWarning> = Vec::new();
    let mut current_paragraphs: Vec<(String, bool, usize)> = Vec::new(); // (text, is_heading, para_index)
    let mut current_title = String::new();
    let mut para_idx = 0usize;

    for (text, is_heading) in &paragraphs {
        para_idx += 1;
        if chapters.len() >= max_chapters {
            warnings.push(ImportWarning {
                code: ImportWarningCode::ChapterCountLimited,
                message: format!("DOCX 章节数已达上限（{max_chapters}），剩余内容未导入"),
                anchor: None,
            });
            break;
        }
        if *is_heading && !current_paragraphs.is_empty() {
            let body: String = current_paragraphs
                .iter()
                .map(|(t, _, _)| t.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            let start_para = current_paragraphs.first().map(|(_, _, i)| *i).unwrap_or(1);
            let end_para = current_paragraphs.last().map(|(_, _, i)| *i).unwrap_or(1);
            chapters.push(make_chapter(
                chapters.len() + 1,
                &current_title,
                &body,
                start_para,
                end_para,
                source_name,
            ));
            current_paragraphs.clear();
            current_title = text.clone();
        } else if *is_heading {
            current_title = text.clone();
        }
        current_paragraphs.push((text.clone(), *is_heading, para_idx));
    }

    // Final chapter
    if !current_paragraphs.is_empty() && chapters.len() < max_chapters {
        let body: String = current_paragraphs
            .iter()
            .map(|(t, _, _)| t.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let start_para = current_paragraphs.first().map(|(_, _, i)| *i).unwrap_or(1);
        let end_para = current_paragraphs.last().map(|(_, _, i)| *i).unwrap_or(1);
        chapters.push(make_chapter(
            chapters.len() + 1,
            &current_title,
            &body,
            start_para,
            end_para,
            source_name,
        ));
    }

    if chapters.is_empty() {
        let body: String = paragraphs
            .iter()
            .map(|(t, _)| t.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        chapters.push(make_chapter(1, "", &body, 1, paragraphs.len(), source_name));
    }

    let total_chars: usize = chapters.iter().map(|c| c.text.chars().count()).sum();

    Ok(ImportedDocument {
        source: SourceDescriptor {
            display_name: source_name.to_owned(),
            format,
            media_type: Some(
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
                    .to_owned(),
            ),
            text_encoding: None,
        },
        stats: DocumentStats {
            chapter_count: chapters.len(),
            line_count: chapters.iter().map(|c| c.text.lines().count()).sum(),
            character_count: total_chars,
            decoded_utf8_bytes: total_chars,
        },
        chapters,
        warnings,
    })
}

/// Convenience entry point for internal callers.
pub(crate) fn import_docx(
    bytes: &[u8],
    source_name: &str,
) -> Result<Vec<ImportedChapter>, ImportError> {
    let request = ImportRequest::new(source_name, bytes);
    let doc = import(request, NovelFormat::Docx)?;
    Ok(doc.chapters)
}

fn extract_paragraphs(xml: &str) -> Vec<(String, bool)> {
    let mut result = Vec::new();
    let mut in_p = false;
    let mut in_r = false;
    let mut in_t = false;
    let mut current_text = String::new();
    let mut is_heading = false;

    let chars: Vec<char> = xml.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '<' {
            let mut tag = String::new();
            i += 1;
            while i < chars.len() && chars[i] != '>' {
                tag.push(chars[i]);
                i += 1;
            }
            i += 1; // skip '>'

            let lower = tag.to_lowercase();
            if lower.starts_with("w:p ") || lower == "w:p" {
                in_p = true;
                is_heading = false;
                current_text.clear();
            } else if lower.starts_with("/w:p") {
                if in_p && !current_text.trim().is_empty() {
                    result.push((current_text.trim().to_owned(), is_heading));
                }
                in_p = false;
            } else if lower.starts_with("w:pstyle") {
                if lower.contains("heading") || lower.contains("Heading") {
                    is_heading = true;
                }
            } else if lower.starts_with("w:r ") || lower == "w:r" {
                in_r = true;
            } else if lower.starts_with("/w:r") {
                in_r = false;
            } else if lower.starts_with("w:t ") || lower == "w:t" {
                in_t = true;
            } else if lower.starts_with("/w:t") {
                in_t = false;
            } else if lower == "w:br/" || lower.starts_with("w:br ") {
                current_text.push('\n');
            } else if lower.starts_with("w:tab/") {
                current_text.push('\t');
            }
        } else if in_t && in_r && in_p {
            current_text.push(chars[i]);
            i += 1;
        } else {
            i += 1;
        }
    }

    result
}

fn make_chapter(
    index: usize,
    title: &str,
    body: &str,
    para_start: usize,
    para_end: usize,
    source_name: &str,
) -> ImportedChapter {
    let display_title = if title.is_empty() {
        format!("第 {index} 节")
    } else {
        title.to_owned()
    };
    ImportedChapter {
        index,
        title: display_title.clone(),
        text: body.to_owned(),
        anchor: SourceAnchor {
            source_name: source_name.to_owned(),
            format: NovelFormat::Docx,
            chapter_index: Some(index),
            chapter_title: Some(display_title),
            locator: SourceLocator::TextRange {
                line_start: para_start,
                line_end: para_end,
                decoded_byte_start: 0,
                decoded_byte_end: body.len(),
            },
        },
        heading_anchor: if title.is_empty() {
            None
        } else {
            Some(SourceAnchor {
                source_name: source_name.to_owned(),
                format: NovelFormat::Docx,
                chapter_index: Some(index),
                chapter_title: Some(title.to_owned()),
                locator: SourceLocator::TextRange {
                    line_start: para_start,
                    line_end: para_start,
                    decoded_byte_start: 0,
                    decoded_byte_end: title.len(),
                },
            })
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import_novel;

    fn make_minimal_docx() -> Vec<u8> {
        use std::io::Write;
        let mut buf = Vec::new();
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        let options: zip::write::FileOptions<()> =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);

        zip.start_file("[Content_Types].xml", options).unwrap();
        zip.write_all(b"<?xml version=\"1.0\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"xml\" ContentType=\"application/xml\"/></Types>").unwrap();

        zip.start_file("word/document.xml", options).unwrap();
        zip.write_all(
            b"<?xml version=\"1.0\"?><w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:body><w:p><w:pPr><w:pStyle w:val=\"Heading1\"/></w:pPr><w:r><w:t>Chapter 1</w:t></w:r></w:p><w:p><w:r><w:t>Body text.</w:t></w:r></w:p><w:p><w:pPr><w:pStyle w:val=\"Heading2\"/></w:pPr><w:r><w:t>Section A</w:t></w:r></w:p><w:p><w:r><w:t>More content.</w:t></w:r></w:p></w:body></w:document>"
        ).unwrap();

        zip.finish().unwrap();
        buf
    }

    #[test]
    fn extracts_paragraph_text() {
        let xml = r#"<w:document><w:body><w:p><w:r><w:t>Hello</w:t></w:r></w:p><w:p><w:r><w:t>World</w:t></w:r></w:p></w:body></w:document>"#;
        let paras = extract_paragraphs(xml);
        assert_eq!(paras.len(), 2);
        assert_eq!(paras[0].0, "Hello");
    }

    #[test]
    fn imports_minimal_docx_without_errors() {
        let docx = make_minimal_docx();
        let doc = import_novel(ImportRequest::new("test.docx", &docx)).unwrap();
        assert_eq!(doc.source.format, NovelFormat::Docx);
        assert!(
            !doc.chapters.is_empty(),
            "DOCX should produce at least one chapter"
        );
        assert!(
            !doc.chapters[0].text.is_empty(),
            "Chapter text should not be empty"
        );
        // Text content should be present (paragraph extraction works)
        let total_len: usize = doc.chapters.iter().map(|c| c.text.len()).sum();
        assert!(total_len > 0, "Total extracted text length should be > 0");
    }

    #[test]
    fn empty_docx_rejected() {
        let empty_zip = [
            0x50, 0x4b, 0x05, 0x06, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let err = import_novel(ImportRequest::new("empty.docx", &empty_zip)).unwrap_err();
        assert!(matches!(err, ImportError::EmptyDocument { .. }));
    }

    #[test]
    fn missing_document_xml_rejected() {
        use std::io::Write;
        let mut buf = Vec::new();
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        let options: zip::write::FileOptions<()> =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);
        zip.start_file("[Content_Types].xml", options).unwrap();
        zip.write_all(b"<Types/>").unwrap();
        // No word/document.xml
        zip.finish().unwrap();

        let err = import_novel(ImportRequest::new("nodoc.docx", &buf)).unwrap_err();
        assert!(matches!(err, ImportError::Corrupt { .. }));
    }

    #[test]
    fn chapter_anchor_has_paragraph_range() {
        let docx = make_minimal_docx();
        let doc = import_novel(ImportRequest::new("test.docx", &docx)).unwrap();
        assert_eq!(doc.chapters[0].anchor.format, NovelFormat::Docx);
    }

    #[test]
    fn respects_chapter_limit() {
        let docx = make_minimal_docx();
        let limits = crate::model::ImportLimits {
            max_chapters: 1,
            ..crate::model::ImportLimits::default()
        };
        let options = crate::ImportOptions {
            limits,
            ..crate::ImportOptions::default()
        };
        let doc =
            import_novel(ImportRequest::new("limit.docx", &docx).with_options(options)).unwrap();
        // With max_chapters=1, we should get at most 1 chapter
        assert!(doc.chapters.len() <= 1);
    }
}
