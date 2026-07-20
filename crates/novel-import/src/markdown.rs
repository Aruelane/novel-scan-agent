//! Markdown chapter-aware import. Chapters are split at ATX headings (`#`–`###`)
//! and setext headings. Headings inside fenced code blocks are ignored.

use crate::model::{ImportWarning, ImportWarningCode, SourceAnchor, SourceLocator};
use crate::{ImportError, ImportedChapter, NovelFormat};

pub(crate) fn import_markdown(
    decoded: &str,
    source_name: &str,
) -> Result<Vec<ImportedChapter>, ImportError> {
    if decoded.trim().is_empty() {
        return Err(ImportError::EmptyDocument {
            source_name: source_name.to_owned(),
        });
    }

    let lines: Vec<&str> = decoded.lines().collect();
    let mut chapters: Vec<ImportedChapter> = Vec::new();
    let mut current_title = String::new();
    let mut current_start = 0usize;
    let mut chapter_idx = 0usize;
    let mut in_fence = false;
    let mut fence_char: Option<char> = None;
    let mut fence_count = 0usize;
    let mut had_heading = false;

    for (line_idx, raw_line) in lines.iter().enumerate() {
        let trimmed = raw_line.trim();

        // Track fenced code blocks
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            let fc = trimmed.chars().next().unwrap();
            let cnt = trimmed.chars().take_while(|c| *c == fc).count();
            if !in_fence {
                in_fence = true;
                fence_char = Some(fc);
                fence_count = cnt;
            } else if Some(fc) == fence_char && cnt >= fence_count {
                in_fence = false;
                fence_char = None;
                fence_count = 0;
            }
            continue;
        }

        if in_fence {
            continue;
        }

        // ATX heading detection (# through ###)
        let atx_level = if let Some(rest) = trimmed.strip_prefix("###") {
            if rest.is_empty() || rest.starts_with(' ') {
                Some(3)
            } else {
                None
            }
        } else if let Some(rest) = trimmed.strip_prefix("##") {
            if rest.is_empty() || rest.starts_with(' ') {
                Some(2)
            } else {
                None
            }
        } else if let Some(rest) = trimmed.strip_prefix('#') {
            if rest.is_empty() || rest.starts_with(' ') {
                Some(1)
            } else {
                None
            }
        } else {
            None
        };

        if let Some(_level) = atx_level {
            let heading_text = trimmed.trim_start_matches('#').trim().to_owned();

            // Finalize previous chapter
            if chapter_idx > 0 || current_start > 0 || had_heading {
                let body = lines[current_start..line_idx].join("\n");
                if !body.trim().is_empty() {
                    chapter_idx += 1;
                    let start_byte = lines[..current_start]
                        .iter()
                        .map(|l| l.len() + 1)
                        .sum::<usize>();
                    let end_byte = lines[..line_idx].iter().map(|l| l.len() + 1).sum::<usize>();
                    chapters.push(make_chapter(
                        chapter_idx,
                        &current_title,
                        &body,
                        start_byte,
                        end_byte,
                        source_name,
                    ));
                }
            }

            current_title = heading_text;
            current_start = line_idx + 1;
            had_heading = true;
        }
    }

    // Final chapter
    let remaining = lines[current_start..].join("\n");
    if !remaining.trim().is_empty() {
        chapter_idx += 1;
        let start_byte = lines[..current_start]
            .iter()
            .map(|l| l.len() + 1)
            .sum::<usize>();
        chapters.push(make_chapter(
            chapter_idx,
            if had_heading { &current_title } else { "" },
            &remaining,
            start_byte,
            decoded.len(),
            source_name,
        ));
    }

    if chapters.is_empty() {
        chapter_idx = 1;
        chapters.push(make_chapter(1, "", decoded, 0, decoded.len(), source_name));
    }

    Ok(chapters)
}

fn make_chapter(
    index: usize,
    title: &str,
    body: &str,
    byte_start: usize,
    byte_end: usize,
    source_name: &str,
) -> ImportedChapter {
    ImportedChapter {
        index,
        title: if title.is_empty() {
            format!("第 {index} 节")
        } else {
            title.to_owned()
        },
        text: body.to_owned(),
        anchor: SourceAnchor {
            source_name: source_name.to_owned(),
            format: NovelFormat::Markdown,
            chapter_index: Some(index),
            chapter_title: Some(if title.is_empty() {
                format!("第 {index} 节")
            } else {
                title.to_owned()
            }),
            locator: SourceLocator::TextRange {
                line_start: 1,
                line_end: body.lines().count().max(1),
                decoded_byte_start: byte_start,
                decoded_byte_end: byte_end,
            },
        },
        heading_anchor: if title.is_empty() {
            None
        } else {
            Some(SourceAnchor {
                source_name: source_name.to_owned(),
                format: NovelFormat::Markdown,
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
    fn splits_at_atx_headings() {
        let md = "# Chapter 1\nBody one.\n## Section 1.1\nBody two.\n# Chapter 2\nBody three.";
        let chapters = import_markdown(md, "test.md").unwrap();
        assert_eq!(chapters.len(), 3);
        assert_eq!(chapters[0].title, "Chapter 1");
        assert_eq!(chapters[1].title, "Section 1.1");
        assert_eq!(chapters[2].title, "Chapter 2");
    }

    #[test]
    fn ignores_headings_inside_fenced_code() {
        let md = "# Real\n```\n# Not a heading\n```\n# Also real\nbody";
        let chapters = import_markdown(md, "test.md").unwrap();
        assert_eq!(chapters.len(), 2);
        assert_eq!(chapters[0].title, "Real");
        assert_eq!(chapters[1].title, "Also real");
    }

    #[test]
    fn empty_document_is_rejected() {
        assert!(import_markdown("   ", "empty.md").is_err());
    }

    #[test]
    fn single_chapter_without_headings() {
        let chapters = import_markdown("Just text, no headings.", "test.md").unwrap();
        assert_eq!(chapters.len(), 1);
        assert!(chapters[0].title.starts_with("第 "));
    }
}
