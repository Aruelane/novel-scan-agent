//! DOCX import via safe ZIP enumeration + XML paragraph extraction.
//! DOCX files are ZIP archives containing word/document.xml.

use crate::archive::enumerate_zip;
use crate::model::{SourceAnchor, SourceLocator};
use crate::{ImportError, ImportedChapter, NovelFormat};

pub(crate) fn import_docx(
    bytes: &[u8],
    source_name: &str,
) -> Result<Vec<ImportedChapter>, ImportError> {
    let entries = enumerate_zip(source_name, bytes, 20_000_000, 100_000_000)?;

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
    let mut chapters = Vec::new();
    let mut current = Vec::new();
    let mut current_title = String::new();
    let mut chapter_idx = 0usize;
    let mut byte_pos = 0usize;

    for (text, is_heading) in &paragraphs {
        if *is_heading && !current.is_empty() {
            chapter_idx += 1;
            let body = current.join("\n");
            chapters.push(make_docx_chapter(
                chapter_idx,
                &current_title,
                &body,
                byte_pos,
                source_name,
            ));
            byte_pos += body.len() + 1;
            current.clear();
            current_title = text.clone();
        } else if *is_heading {
            current_title = text.clone();
        }
        current.push(text.clone());
    }

    // Final chapter
    if !current.is_empty() {
        chapter_idx += 1;
        let body = current.join("\n");
        chapters.push(make_docx_chapter(
            chapter_idx,
            &current_title,
            &body,
            byte_pos,
            source_name,
        ));
    }

    if chapters.is_empty() {
        let body = paragraphs
            .iter()
            .map(|(t, _)| t.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        chapter_idx = 1;
        chapters.push(make_docx_chapter(1, "", &body, 0, source_name));
    }

    Ok(chapters)
}

fn extract_paragraphs(xml: &str) -> Vec<(String, bool)> {
    let mut result = Vec::new();
    let mut in_p = false;
    let mut in_r = false;
    let mut in_t = false;
    let mut current_text = String::new();
    let mut is_heading = false;

    // Simple tag-based parser for w:p elements
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

fn make_docx_chapter(
    index: usize,
    title: &str,
    body: &str,
    byte_start: usize,
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
                line_start: 1,
                line_end: body.lines().count().max(1),
                decoded_byte_start: byte_start,
                decoded_byte_end: byte_start + body.len(),
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
                    line_start: 1,
                    line_end: 1,
                    decoded_byte_start: byte_start,
                    decoded_byte_end: byte_start + title.len(),
                },
            })
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_paragraph_text() {
        let xml = r#"<w:document><w:body><w:p><w:r><w:t>Hello</w:t></w:r></w:p><w:p><w:r><w:t>World</w:t></w:r></w:p></w:body></w:document>"#;
        let paras = extract_paragraphs(xml);
        assert_eq!(paras.len(), 2);
        assert_eq!(paras[0].0, "Hello");
    }

    #[test]
    fn empty_docx_rejected() {
        // Minimal ZIP with no word/document.xml — would fail at enumerate_zip
        let empty_zip = [
            0x50, 0x4b, 0x05, 0x06, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        assert!(import_docx(&empty_zip, "empty.docx").is_err());
    }
}
