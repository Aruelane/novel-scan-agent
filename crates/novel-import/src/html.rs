//! Safe HTML text import. No script execution, no external resources.
//! Uses a simple state machine to extract body text and h1-h3 headings.

use crate::model::{SourceAnchor, SourceLocator};
use crate::{ImportError, ImportedChapter, NovelFormat};

pub(crate) fn import_html(
    raw_html: &str,
    source_name: &str,
) -> Result<Vec<ImportedChapter>, ImportError> {
    let text = strip_tags(raw_html);
    if text.trim().is_empty() {
        return Err(ImportError::EmptyDocument {
            source_name: source_name.to_owned(),
        });
    }

    // Split at h1-h3 headings found during strip_tags
    let chapters = split_at_headings(&text, source_name);
    if chapters.is_empty() {
        return Err(ImportError::EmptyDocument {
            source_name: source_name.to_owned(),
        });
    }
    Ok(chapters)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Text,
    Tag,
    Script,
    Style,
}

fn strip_tags(html: &str) -> String {
    let mut out = String::new();
    let mut state = State::Text;
    let mut tag = String::new();
    let mut depth = 0u32;

    for ch in html.chars() {
        match state {
            State::Text => {
                if ch == '<' {
                    state = State::Tag;
                    tag.clear();
                } else {
                    out.push(ch);
                }
            }
            State::Tag => {
                if ch == '>' {
                    let lower = tag.to_lowercase();
                    if lower.starts_with("script") || lower == "script" {
                        state = State::Script;
                    } else if lower.starts_with("style") || lower == "style" {
                        state = State::Style;
                    } else if lower.starts_with("/script")
                        || lower == "/script"
                        || lower.starts_with("/style")
                        || lower == "/style"
                    {
                        // closing tag in text mode
                    } else {
                        state = State::Text;
                        // Emit heading markers for h1-h3
                        if lower == "h1" || lower.starts_with("h1 ") {
                            out.push_str("\n# ");
                        } else if lower == "h2" || lower.starts_with("h2 ") {
                            out.push_str("\n## ");
                        } else if lower == "h3" || lower.starts_with("h3 ") {
                            out.push_str("\n### ");
                        } else if lower == "p" || lower.starts_with("p ") || lower == "/p" {
                            out.push('\n');
                        } else if lower == "br" || lower.starts_with("br ") {
                            out.push('\n');
                        } else if lower == "li" || lower.starts_with("li ") || lower == "/li" {
                            out.push_str("\n- ");
                        } else if lower.starts_with("/h") {
                            out.push('\n');
                        }
                    }
                } else {
                    tag.push(ch);
                }
            }
            State::Script => {
                if ch == '<' {
                    tag.clear();
                } else if ch == '>' && tag.to_lowercase() == "/script" {
                    state = State::Text;
                } else if ch != '>' {
                    tag.push(ch);
                }
            }
            State::Style => {
                if ch == '<' {
                    tag.clear();
                } else if ch == '>' && tag.to_lowercase() == "/style" {
                    state = State::Text;
                } else if ch != '>' {
                    tag.push(ch);
                }
            }
        }
    }

    // Clean up: collapse multiple blank lines
    let mut result = String::new();
    let mut blank = false;
    for line in out.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !blank {
                result.push('\n');
                blank = true;
            }
        } else {
            result.push_str(trimmed);
            result.push('\n');
            blank = false;
        }
    }
    result
}

fn split_at_headings(text: &str, source_name: &str) -> Vec<ImportedChapter> {
    let mut chapters = Vec::new();
    let mut current_title = String::new();
    let mut current_body = String::new();
    let mut byte_offset = 0usize;
    let mut chapter_idx = 0usize;
    let mut had_heading = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("# ") || trimmed.starts_with("## ") || trimmed.starts_with("### ") {
            // Save previous chapter
            if !current_body.trim().is_empty() || !current_title.is_empty() {
                chapter_idx += 1;
                let end = byte_offset + current_body.len();
                chapters.push(make_html_chapter(
                    chapter_idx,
                    &current_title,
                    &current_body,
                    byte_offset - current_body.len(),
                    end,
                    source_name,
                ));
            }
            current_title = trimmed.trim_start_matches('#').trim().to_owned();
            current_body = String::new();
            had_heading = true;
        } else {
            current_body.push_str(line);
            current_body.push('\n');
        }
        byte_offset += line.len() + 1;
    }

    // Final chapter
    if !current_body.trim().is_empty() || !had_heading {
        chapter_idx += 1;
        let end = text.len();
        let start = end.saturating_sub(current_body.len());
        let title = if had_heading { &current_title } else { "" };
        chapters.push(make_html_chapter(
            chapter_idx,
            title,
            &current_body,
            start,
            end,
            source_name,
        ));
    }

    if chapters.is_empty() {
        chapter_idx = 1;
        chapters.push(make_html_chapter(1, "", text, 0, text.len(), source_name));
    }

    chapters
}

fn make_html_chapter(
    index: usize,
    title: &str,
    body: &str,
    byte_start: usize,
    byte_end: usize,
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
            format: NovelFormat::Html,
            chapter_index: Some(index),
            chapter_title: Some(display_title),
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
                format: NovelFormat::Html,
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
    fn strips_script_and_style() {
        let html = "<html><script>alert('xss')</script><style>body{}</style><p>Hello</p></html>";
        let text = strip_tags(html);
        assert!(!text.contains("alert"));
        assert!(!text.contains("body{}"));
        assert!(text.contains("Hello"));
    }

    #[test]
    fn extracts_h1_headings() {
        let html = "<h1>Chapter 1</h1><p>Body text</p><h2>Section</h2><p>More</p>";
        let chapters = import_html(html, "test.html").unwrap();
        assert!(chapters.len() >= 1);
        assert_eq!(chapters[0].title, "Chapter 1");
    }

    #[test]
    fn empty_html_rejected() {
        assert!(import_html("   ", "empty.html").is_err());
    }

    #[test]
    fn plain_text_returns_single_chapter() {
        let chapters = import_html("Just text, no tags.", "test.html").unwrap();
        assert_eq!(chapters.len(), 1);
    }
}
