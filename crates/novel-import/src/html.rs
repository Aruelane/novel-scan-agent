//! Safe HTML text import. No script execution, no external resources.
//! Uses a simple state machine to extract body text and h1-h3 headings.

use crate::{
    encoding, DocumentStats, ImportError, ImportRequest, ImportWarning, ImportWarningCode,
    ImportedChapter, ImportedDocument, NovelFormat, SourceAnchor, SourceDescriptor, SourceLocator,
};

/// Convenience entry point for internal callers (e.g., EPUB) that already have
/// decoded text and only need chapter extraction.
pub(crate) fn import_html(
    raw_html: &str,
    source_name: &str,
) -> Result<Vec<ImportedChapter>, ImportError> {
    if raw_html.trim().is_empty() {
        return Err(ImportError::EmptyDocument {
            source_name: source_name.to_owned(),
        });
    }
    let text = strip_tags(raw_html);
    if text.trim().is_empty() {
        return Err(ImportError::EmptyDocument {
            source_name: source_name.to_owned(),
        });
    }
    let chapters = split_at_headings(&text, source_name, usize::MAX);
    if chapters.is_empty() {
        return Err(ImportError::EmptyDocument {
            source_name: source_name.to_owned(),
        });
    }
    Ok(chapters)
}

/// Public entry point matching the `plain_text::import` contract.
pub(crate) fn import(
    request: ImportRequest<'_>,
    format: NovelFormat,
) -> Result<ImportedDocument, ImportError> {
    let decoded = encoding::decode(request.bytes, request.options.encoding_hint)?;

    // Apply chapter limit before processing
    let max_chapters = request.options.limits.max_chapters;

    let text = strip_tags(&decoded.text);
    if text.trim().is_empty() {
        return Err(ImportError::EmptyDocument {
            source_name: request.source_name.to_owned(),
        });
    }

    let mut chapters = split_at_headings(&text, request.source_name, max_chapters);

    if chapters.is_empty() {
        return Err(ImportError::EmptyDocument {
            source_name: request.source_name.to_owned(),
        });
    }

    // Track if any chapter is truncated
    let truncated = chapters.len() >= max_chapters && max_chapters > 0;
    let mut warnings = Vec::new();
    if truncated {
        warnings.push(ImportWarning {
            code: ImportWarningCode::ChapterCountLimited,
            message: format!("HTML 章节数已达上限（{max_chapters}），剩余内容未导入"),
            anchor: None,
        });
    }

    Ok(ImportedDocument {
        source: SourceDescriptor {
            display_name: request.source_name.to_owned(),
            format,
            media_type: request.media_type.map(str::to_owned),
            text_encoding: Some(decoded.encoding),
        },
        stats: DocumentStats {
            chapter_count: chapters.len(),
            line_count: text.lines().count(),
            character_count: decoded.text.chars().count(),
            decoded_utf8_bytes: decoded.text.len(),
        },
        chapters,
        warnings,
    })
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
                        // closing tag in text mode — ignore
                    } else {
                        state = State::Text;
                        // Emit heading markers for h1-h3 (these are used by split_at_headings)
                        if lower == "h1" || lower.starts_with("h1 ") {
                            out.push('\n');
                            out.push_str("# ");
                        } else if lower == "h2" || lower.starts_with("h2 ") {
                            out.push('\n');
                            out.push_str("## ");
                        } else if lower == "h3" || lower.starts_with("h3 ") {
                            out.push('\n');
                            out.push_str("### ");
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

    // Collapse multiple blank lines
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

fn split_at_headings(text: &str, source_name: &str, max_chapters: usize) -> Vec<ImportedChapter> {
    let mut chapters: Vec<ImportedChapter> = Vec::new();
    let mut current_title = String::new();
    let mut current_body = String::new();
    let mut byte_offset = 0usize;
    let mut had_heading = false;
    let mut is_first_heading = true;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("# ") || trimmed.starts_with("## ") || trimmed.starts_with("### ") {
            // Save previous chapter (unless this is the very first heading and we have front matter)
            let body_not_empty = !current_body.trim().is_empty();
            let has_title = !current_title.is_empty();
            let should_flush = body_not_empty || (has_title && !is_first_heading);

            if should_flush && chapters.len() < max_chapters {
                let end = byte_offset;
                let start = end.saturating_sub(current_body.len());
                chapters.push(make_html_chapter(
                    chapters.len() + 1,
                    &current_title,
                    &current_body,
                    start,
                    end,
                    source_name,
                ));
            } else if body_not_empty && chapters.len() >= max_chapters {
                break;
            }

            current_title = trimmed.trim_start_matches('#').trim().to_owned();
            current_body = String::new();
            had_heading = true;
            if is_first_heading {
                is_first_heading = false;
            }
        } else {
            current_body.push_str(line);
            current_body.push('\n');
        }
        byte_offset += line.len() + 1; // +1 for newline
    }

    // Final chapter
    if (!current_body.trim().is_empty() || !had_heading) && chapters.len() < max_chapters {
        let end = text.len();
        let start = end.saturating_sub(current_body.len());
        let title = if had_heading {
            current_title.as_str()
        } else {
            ""
        };
        chapters.push(make_html_chapter(
            chapters.len() + 1,
            title,
            &current_body,
            start,
            end,
            source_name,
        ));
    }

    // Fallback: no headings found at all → single chapter
    if chapters.is_empty() {
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
    let line_count = body.lines().count().max(1);
    ImportedChapter {
        index,
        title: display_title.clone(),
        text: body.to_owned(),
        anchor: SourceAnchor {
            source_name: source_name.to_owned(),
            format: NovelFormat::Html,
            chapter_index: Some(index),
            chapter_title: Some(display_title.clone()),
            locator: SourceLocator::TextRange {
                line_start: 1,
                line_end: line_count,
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
                chapter_title: Some(display_title),
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
    use crate::model::ImportLimits;
    use crate::{import_novel, ImportOptions};

    // ── Normal ──

    #[test]
    fn imports_basic_html_with_h1_headings() {
        let html = "<html><body><h1>Chapter 1</h1><p>Body text.</p><h2>Section</h2><p>More.</p></body></html>";
        let doc = import_novel(ImportRequest::new("test.html", html.as_bytes())).unwrap();
        assert!(doc.chapters.len() >= 2);
        assert_eq!(doc.chapters[0].title, "Chapter 1");
        assert!(doc.chapters[0].text.contains("Body text"));
        assert_eq!(doc.source.format, NovelFormat::Html);
    }

    #[test]
    fn html_with_no_headings_returns_single_chapter() {
        let html = "<html><body><p>Just some text without headings.</p></body></html>";
        let doc = import_novel(ImportRequest::new("plain.html", html.as_bytes())).unwrap();
        assert_eq!(doc.chapters.len(), 1);
        assert!(doc.chapters[0]
            .text
            .contains("Just some text without headings"));
    }

    // ── Corrupt ──

    #[test]
    fn empty_html_rejected() {
        let err = import_novel(ImportRequest::new("empty.html", b"   ")).unwrap_err();
        assert!(matches!(err, ImportError::EmptyDocument { .. }));
    }

    #[test]
    fn html_with_only_scripts_rejected() {
        let html = "<html><script>alert('hi')</script></html>";
        let err = import_novel(ImportRequest::new("script.html", html.as_bytes())).unwrap_err();
        assert!(matches!(err, ImportError::EmptyDocument { .. }));
    }

    // ── Security ──

    #[test]
    fn strips_script_and_style_content() {
        let html =
            "<html><script>alert('xss')</script><style>body{color:red}</style><p>Safe</p></html>";
        let doc = import_novel(ImportRequest::new("xss.html", html.as_bytes())).unwrap();
        assert!(!doc.chapters[0].text.contains("alert"));
        assert!(!doc.chapters[0].text.contains("body{color:red}"));
        assert!(doc.chapters[0].text.contains("Safe"));
    }

    #[test]
    fn ignores_external_resource_references() {
        let html = "<html><link rel='stylesheet' href='http://evil.com/x.css'><p>content</p><img src='http://evil.com/x.png'></html>";
        let doc = import_novel(ImportRequest::new("external.html", html.as_bytes())).unwrap();
        assert!(!doc.chapters[0].text.contains("http://evil.com"));
        assert!(doc.chapters[0].text.contains("content"));
    }

    // ── Overlimit ──

    #[test]
    fn respects_chapter_limit() {
        let html = "<h1>A</h1><p>a</p><h1>B</h1><p>b</p><h1>C</h1><p>c</p>";
        let limits = ImportLimits {
            max_chapters: 2,
            ..ImportLimits::default()
        };
        let options = ImportOptions {
            limits,
            ..ImportOptions::default()
        };
        let doc =
            import_novel(ImportRequest::new("limited.html", html.as_bytes()).with_options(options))
                .unwrap();
        assert_eq!(doc.chapters.len(), 2);
        assert!(doc.warnings.iter().any(|w| w.message.contains("上限")));
    }

    // ── Anchor regression ──

    #[test]
    fn chapter_anchor_retains_line_range_and_byte_offsets() {
        let html = "<h1>Chapter 1</h1><p>Line one.\nLine two.</p>";
        let doc = import_novel(ImportRequest::new("anchor.html", html.as_bytes())).unwrap();
        let anchor = &doc.chapters[0].anchor;
        assert_eq!(anchor.format, NovelFormat::Html);
        assert_eq!(anchor.chapter_title.as_deref(), Some("Chapter 1"));
        // Verify text can be sliced from byte range
        let slice = &doc.chapters[0].text;
        assert!(slice.contains("Line one"));
        assert!(slice.contains("Line two"));
        // Verify line count makes sense
        if let SourceLocator::TextRange { line_end, .. } = anchor.locator {
            assert!(line_end >= 1);
        } else {
            panic!("expected TextRange locator");
        }
    }

    #[test]
    fn heading_anchor_present_for_chapters_with_headings() {
        let html = "<h1>Title</h1><p>Body</p>";
        let doc = import_novel(ImportRequest::new("head.html", html.as_bytes())).unwrap();
        assert!(doc.chapters[0].heading_anchor.is_some());
        assert_eq!(
            doc.chapters[0]
                .heading_anchor
                .as_ref()
                .unwrap()
                .chapter_title,
            Some("Title".to_owned())
        );
    }

    #[test]
    fn heading_anchor_none_for_no_heading_chapter() {
        let html = "<p>Just text</p>";
        let doc = import_novel(ImportRequest::new("nohead.html", html.as_bytes())).unwrap();
        assert!(doc.chapters[0].heading_anchor.is_none());
    }

    // ── Multi-charset ──

    #[test]
    fn handles_chinese_html() {
        let html = "<html><body><h1>第一章 相遇</h1><p>她推开了门。</p><h2>第二章 真相</h2><p>答案在这里。</p></body></html>";
        let doc = import_novel(ImportRequest::new("cn.html", html.as_bytes())).unwrap();
        assert!(doc.chapters.len() >= 2);
        assert_eq!(doc.chapters[0].title, "第一章 相遇");
        assert!(doc.chapters[0].text.contains("她推开了门"));
    }

    #[test]
    fn handles_emoji_in_html() {
        let html = "<h1>🎉 Celebration 🎉</h1><p>Fireworks! 🔥✨</p>";
        let doc = import_novel(ImportRequest::new("emoji.html", html.as_bytes())).unwrap();
        assert!(doc.chapters[0].title.contains("🎉"));
        assert!(doc.chapters[0].text.contains("🔥"));
    }
}
