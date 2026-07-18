use crate::{
    encoding, ChapterSplitMode, DocumentStats, ImportError, ImportRequest, ImportWarning,
    ImportWarningCode, ImportedChapter, ImportedDocument, NovelFormat, SourceAnchor,
    SourceDescriptor, SourceLocator,
};

#[derive(Clone, Copy, Debug)]
struct LineSpan<'a> {
    number: usize,
    start: usize,
    content_end: usize,
    full_end: usize,
    content: &'a str,
}

#[derive(Clone, Debug)]
struct Heading {
    line_index: usize,
    title: String,
}

pub(crate) fn import(
    request: ImportRequest<'_>,
    format: NovelFormat,
) -> Result<ImportedDocument, ImportError> {
    let decoded = encoding::decode(request.bytes, request.options.encoding_hint)?;
    if decoded.text.trim().is_empty() {
        return Err(ImportError::EmptyDocument {
            source_name: request.source_name.to_owned(),
        });
    }

    let lines = line_spans(&decoded.text);
    let split_mode = match (request.options.chapter_split, format) {
        (ChapterSplitMode::Auto, NovelFormat::Markdown) => ChapterSplitMode::MarkdownHeadings,
        (ChapterSplitMode::Auto, _) => ChapterSplitMode::NovelHeadings,
        (mode, _) => mode,
    };
    let headings = find_headings(&lines, split_mode);
    let mut warnings = Vec::new();
    let chapters = if headings.is_empty() {
        if split_mode != ChapterSplitMode::None {
            warnings.push(ImportWarning {
                code: ImportWarningCode::NoChapterHeadings,
                message: "没有识别到章节标题，已将全文作为一个可引用分段".to_owned(),
                anchor: None,
            });
        }
        vec![chapter_from_range(
            request.source_name,
            format,
            &decoded.text,
            &lines,
            1,
            "全文".to_owned(),
            0,
            decoded.text.len(),
            None,
        )]
    } else {
        split_chapters(
            request.source_name,
            format,
            &decoded.text,
            &lines,
            &headings,
            &mut warnings,
        )
    };

    Ok(ImportedDocument {
        source: SourceDescriptor {
            display_name: request.source_name.to_owned(),
            format,
            media_type: request.media_type.map(str::to_owned),
            text_encoding: Some(decoded.encoding),
        },
        stats: DocumentStats {
            chapter_count: chapters.len(),
            line_count: lines.len(),
            character_count: decoded.text.chars().count(),
            decoded_utf8_bytes: decoded.text.len(),
        },
        chapters,
        warnings,
    })
}

fn line_spans(text: &str) -> Vec<LineSpan<'_>> {
    let mut lines = Vec::new();
    let mut start = 0;
    for (newline, character) in text.char_indices() {
        if character != '\n' {
            continue;
        }
        let raw_content = &text[start..newline];
        let content = raw_content.strip_suffix('\r').unwrap_or(raw_content);
        lines.push(LineSpan {
            number: lines.len() + 1,
            start,
            content_end: start + content.len(),
            full_end: newline + 1,
            content,
        });
        start = newline + 1;
    }
    if start < text.len() {
        let raw_content = &text[start..];
        let content = raw_content.strip_suffix('\r').unwrap_or(raw_content);
        lines.push(LineSpan {
            number: lines.len() + 1,
            start,
            content_end: start + content.len(),
            full_end: text.len(),
            content,
        });
    }
    lines
}

fn find_headings(lines: &[LineSpan<'_>], split_mode: ChapterSplitMode) -> Vec<Heading> {
    if split_mode == ChapterSplitMode::None {
        return Vec::new();
    }

    lines
        .iter()
        .enumerate()
        .filter_map(|(line_index, line)| {
            heading_title(line.content, split_mode).map(|title| Heading { line_index, title })
        })
        .collect()
}

fn heading_title(line: &str, split_mode: ChapterSplitMode) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.chars().count() > 80 {
        return None;
    }

    if split_mode == ChapterSplitMode::MarkdownHeadings {
        if let Some(title) = markdown_heading(trimmed) {
            return Some(title);
        }
    }

    if is_chinese_numbered_heading(trimmed)
        || is_english_chapter_heading(trimmed)
        || is_special_heading(trimmed)
    {
        return Some(trimmed.to_owned());
    }

    None
}

fn markdown_heading(line: &str) -> Option<String> {
    let hash_count = line.bytes().take_while(|byte| *byte == b'#').count();
    if !(1..=3).contains(&hash_count) || line.as_bytes().get(hash_count) != Some(&b' ') {
        return None;
    }
    let title = line[hash_count..].trim().trim_end_matches('#').trim();
    (!title.is_empty()).then(|| title.to_owned())
}

fn is_chinese_numbered_heading(line: &str) -> bool {
    let Some(after_prefix) = line.strip_prefix('第') else {
        return false;
    };
    let Some((marker_index, _)) = after_prefix
        .char_indices()
        .find(|(_, character)| matches!(character, '章' | '回' | '节' | '卷' | '部' | '篇'))
    else {
        return false;
    };
    let number = after_prefix[..marker_index].trim();
    !number.is_empty()
        && number.chars().count() <= 12
        && number.chars().all(|character| {
            character.is_ascii_digit()
                || matches!(
                    character,
                    '零' | '〇'
                        | '一'
                        | '二'
                        | '三'
                        | '四'
                        | '五'
                        | '六'
                        | '七'
                        | '八'
                        | '九'
                        | '十'
                        | '百'
                        | '千'
                        | '万'
                        | '两'
                        | '上'
                        | '中'
                        | '下'
                )
        })
}

fn is_english_chapter_heading(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    let Some(rest) = lower.strip_prefix("chapter") else {
        return false;
    };
    let rest = rest.trim_start();
    let token = rest
        .split(|character: char| character.is_whitespace() || matches!(character, ':' | '-' | '.'))
        .next()
        .unwrap_or_default();
    !token.is_empty()
        && token
            .chars()
            .all(|character| character.is_ascii_digit() || "ivxlcdm".contains(character))
}

fn is_special_heading(line: &str) -> bool {
    const EXACT: &[&str] = &["序", "序章", "楔子", "引子", "前言", "尾声", "后记", "终章"];
    EXACT.contains(&line)
        || line == "番外"
        || line.starts_with("番外：")
        || line.starts_with("番外:")
        || line.strip_prefix("番外").is_some_and(|rest| {
            !rest.is_empty() && rest.chars().all(|character| character.is_ascii_digit())
        })
}

#[allow(clippy::too_many_arguments)]
fn split_chapters(
    source_name: &str,
    format: NovelFormat,
    text: &str,
    lines: &[LineSpan<'_>],
    headings: &[Heading],
    warnings: &mut Vec<ImportWarning>,
) -> Vec<ImportedChapter> {
    let mut chapters = Vec::new();
    let first_heading_start = lines[headings[0].line_index].start;
    if !text[..first_heading_start].trim().is_empty() {
        let chapter = chapter_from_range(
            source_name,
            format,
            text,
            lines,
            1,
            "正文前".to_owned(),
            0,
            first_heading_start,
            None,
        );
        warnings.push(ImportWarning {
            code: ImportWarningCode::FrontMatterDetected,
            message: "首个章节标题之前存在文本，已单独保留为“正文前”以免丢失来源".to_owned(),
            anchor: Some(chapter.anchor.clone()),
        });
        chapters.push(chapter);
    }

    for (heading_position, heading) in headings.iter().enumerate() {
        let start = lines[heading.line_index].start;
        let end = headings
            .get(heading_position + 1)
            .map(|next| lines[next.line_index].start)
            .unwrap_or(text.len());
        let index = chapters.len() + 1;
        let heading_line = lines[heading.line_index];
        let heading_anchor = anchor(
            source_name,
            format,
            Some(index),
            Some(heading.title.clone()),
            heading_line.number,
            heading_line.number,
            heading_line.start,
            heading_line.content_end,
        );
        chapters.push(chapter_from_range(
            source_name,
            format,
            text,
            lines,
            index,
            heading.title.clone(),
            start,
            end,
            Some(heading_anchor),
        ));
    }

    chapters
}

#[allow(clippy::too_many_arguments)]
fn chapter_from_range(
    source_name: &str,
    format: NovelFormat,
    text: &str,
    lines: &[LineSpan<'_>],
    index: usize,
    title: String,
    start: usize,
    end: usize,
    heading_anchor: Option<SourceAnchor>,
) -> ImportedChapter {
    let (line_start, line_end) = line_range(lines, start, end);
    let chapter_anchor = anchor(
        source_name,
        format,
        Some(index),
        Some(title.clone()),
        line_start,
        line_end,
        start,
        end,
    );
    ImportedChapter {
        index,
        title,
        text: text[start..end].to_owned(),
        anchor: chapter_anchor,
        heading_anchor,
    }
}

fn line_range(lines: &[LineSpan<'_>], start: usize, end: usize) -> (usize, usize) {
    let first = lines
        .iter()
        .find(|line| line.full_end > start)
        .map(|line| line.number)
        .unwrap_or(1);
    let last = lines
        .iter()
        .rev()
        .find(|line| line.start < end)
        .map(|line| line.number)
        .unwrap_or(first);
    (first, last)
}

#[allow(clippy::too_many_arguments)]
fn anchor(
    source_name: &str,
    format: NovelFormat,
    chapter_index: Option<usize>,
    chapter_title: Option<String>,
    line_start: usize,
    line_end: usize,
    decoded_byte_start: usize,
    decoded_byte_end: usize,
) -> SourceAnchor {
    SourceAnchor {
        source_name: source_name.to_owned(),
        format,
        chapter_index,
        chapter_title,
        locator: SourceLocator::TextRange {
            line_start,
            line_end,
            decoded_byte_start,
            decoded_byte_end,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{import_novel, ImportOptions, TextEncoding};

    #[test]
    fn splits_utf8_chinese_novel_and_keeps_exact_line_anchors() {
        let text =
            "书名：测试\r\n\r\n第一章 相遇\r\n她推开了门。\r\n\r\n第二章 真相\r\n答案在这里。";
        let document = import_novel(ImportRequest::new("测试.txt", text.as_bytes())).unwrap();

        assert_eq!(document.source.text_encoding, Some(TextEncoding::Utf8));
        assert_eq!(document.chapters.len(), 3);
        assert_eq!(document.chapters[0].title, "正文前");
        assert_eq!(document.chapters[1].title, "第一章 相遇");
        assert_eq!(document.chapters[2].title, "第二章 真相");
        assert!(document.chapters[1].text.starts_with("第一章 相遇\r\n"));
        assert_eq!(
            document.chapters[1].anchor.locator,
            SourceLocator::TextRange {
                line_start: 3,
                line_end: 5,
                decoded_byte_start: text.find("第一章").unwrap(),
                decoded_byte_end: text.find("第二章").unwrap(),
            }
        );
        assert_eq!(
            document.chapters[2].anchor.citation_label(),
            "测试.txt · 第 3 节《第二章 真相》 · 第 6–7 行"
        );
    }

    #[test]
    fn markdown_uses_headings_and_preserves_the_heading_source() {
        let text = "# 书名\n简介\n\n## 第一章\n正文\n\n### 第二章 #\n结尾";
        let document = import_novel(ImportRequest::new("novel.md", text.as_bytes())).unwrap();

        assert_eq!(
            document
                .chapters
                .iter()
                .map(|chapter| chapter.title.as_str())
                .collect::<Vec<_>>(),
            vec!["书名", "第一章", "第二章"]
        );
        let heading = document.chapters[1].heading_anchor.as_ref().unwrap();
        assert_eq!(
            heading.locator,
            SourceLocator::TextRange {
                line_start: 4,
                line_end: 4,
                decoded_byte_start: text.find("## 第一章").unwrap(),
                decoded_byte_end: text.find("## 第一章").unwrap() + "## 第一章".len(),
            }
        );
    }

    #[test]
    fn can_disable_chapter_splitting() {
        let text = "第一章\n正文\n第二章\n结尾";
        let options = ImportOptions {
            chapter_split: ChapterSplitMode::None,
            ..ImportOptions::default()
        };
        let document =
            import_novel(ImportRequest::new("novel.txt", text.as_bytes()).with_options(options))
                .unwrap();

        assert_eq!(document.chapters.len(), 1);
        assert_eq!(document.chapters[0].title, "全文");
        assert_eq!(document.chapters[0].text, text);
        assert!(document.warnings.is_empty());
    }

    #[test]
    fn imports_utf16_and_tracks_decoded_offsets_instead_of_claiming_file_offsets() {
        let text = "第一章\n正文";
        let mut bytes = vec![0xff, 0xfe];
        for unit in text.encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        let document = import_novel(ImportRequest::new("novel.txt", &bytes)).unwrap();

        assert_eq!(document.source.text_encoding, Some(TextEncoding::Utf16Le));
        assert_eq!(document.chapters[0].text, text);
        assert_eq!(document.stats.decoded_utf8_bytes, text.len());
    }

    #[test]
    fn recognises_common_heading_variants_without_splitting_normal_prose() {
        let text = "楔子\n开场\nChapter IV - Return\n内容\n她说第一章很好看，但这不是标题。";
        let document = import_novel(ImportRequest::new("novel.txt", text.as_bytes())).unwrap();

        assert_eq!(document.chapters.len(), 2);
        assert_eq!(document.chapters[0].title, "楔子");
        assert_eq!(document.chapters[1].title, "Chapter IV - Return");
    }
}
