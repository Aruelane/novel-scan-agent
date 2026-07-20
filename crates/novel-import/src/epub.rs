//! EPUB import via safe ZIP enumeration + HTML extraction.
//! Reuses archive.rs for ZIP safety and html.rs for content extraction.

use crate::archive::enumerate_zip;
use crate::html::import_html;
use crate::{
    DocumentStats, ImportError, ImportRequest, ImportWarning, ImportedChapter, ImportedDocument,
    NovelFormat, SourceAnchor, SourceDescriptor, SourceLocator,
};

/// Public entry point matching the `plain_text::import` contract.
pub(crate) fn import(
    request: ImportRequest<'_>,
    format: NovelFormat,
) -> Result<ImportedDocument, ImportError> {
    let source_name = request.source_name;
    let max_chapters = request.options.limits.max_chapters;

    let entries = enumerate_zip(source_name, request.bytes, 20_000_000, 100_000_000)?;

    // ── 1. Validate mimetype ──
    let mimetype_entry = entries.iter().find(|e| e.name == "mimetype");
    match mimetype_entry {
        Some(entry) => {
            let content = String::from_utf8_lossy(&entry.data);
            if !content.trim().starts_with("application/epub+zip") {
                return Err(ImportError::Corrupt {
                    source_name: source_name.to_owned(),
                    detail: format!(
                        "mimetype 文件内容不是 application/epub+zip：{}",
                        content.trim()
                    ),
                });
            }
        }
        None => {
            return Err(ImportError::Corrupt {
                source_name: source_name.to_owned(),
                detail: "缺少 mimetype 文件，不是有效的 EPUB".to_owned(),
            });
        }
    }

    // ── 2. Parse META-INF/container.xml ──
    let container_entry = entries.iter().find(|e| e.name == "META-INF/container.xml");
    let container_xml = match container_entry {
        Some(entry) => String::from_utf8_lossy(&entry.data).into_owned(),
        None => {
            return Err(ImportError::Corrupt {
                source_name: source_name.to_owned(),
                detail: "缺少 META-INF/container.xml".to_owned(),
            });
        }
    };

    let rootfile_path = extract_rootfile(&container_xml).ok_or_else(|| ImportError::Corrupt {
        source_name: source_name.to_owned(),
        detail: "container.xml 中未找到 rootfile 条目".to_owned(),
    })?;

    // ── 3. Parse OPF: manifest + spine ──
    let opf_entry = entries.iter().find(|e| e.name == rootfile_path);
    let opf_xml = match opf_entry {
        Some(entry) => String::from_utf8_lossy(&entry.data).into_owned(),
        None => {
            return Err(ImportError::Corrupt {
                source_name: source_name.to_owned(),
                detail: format!("OPF 文件 {rootfile_path} 在 ZIP 中不存在"),
            });
        }
    };

    // Extract manifest: id → href mapping
    let manifest = extract_manifest(&opf_xml);
    if manifest.is_empty() {
        return Err(ImportError::Corrupt {
            source_name: source_name.to_owned(),
            detail: "OPF manifest 为空".to_owned(),
        });
    }

    // Extract spine: ordered list of idrefs
    let spine_refs = extract_spine(&opf_xml);

    // Detect navigation resources (epub:type="nav" or properties="nav")
    let nav_ids = extract_nav_ids(&opf_xml);

    // ── 4. Collect XHTML chapters in spine order ──
    let mut chapters: Vec<ImportedChapter> = Vec::new();
    let mut warnings: Vec<ImportWarning> = Vec::new();
    let mut total_character_count = 0usize;

    // Build a lookup: entry name → data
    let entry_map: std::collections::HashMap<&str, &[u8]> = entries
        .iter()
        .map(|e| (e.name.as_str(), e.data.as_slice()))
        .collect();

    // Process spine items in order, skipping nav-only resources
    if !spine_refs.is_empty() {
        for idref in &spine_refs {
            if chapters.len() >= max_chapters {
                warnings.push(ImportWarning {
                    code: crate::ImportWarningCode::ChapterCountLimited,
                    message: format!("EPUB 章节数已达上限（{max_chapters}），剩余内容未导入"),
                    anchor: None,
                });
                break;
            }

            // Skip pure navigation resources
            if nav_ids.contains(idref) {
                continue;
            }

            if let Some(href) = manifest.get(idref) {
                // Parse fragment from href (e.g., "ch1.xhtml#section1")
                let (base_href, fragment) = split_fragment(href);
                let resolved = resolve_opf_relative(&rootfile_path, &base_href);
                if let Some(data) = entry_map.get(resolved.as_str()) {
                    let text = String::from_utf8_lossy(data);
                    let para_count = count_paragraphs(&text);
                    if let Ok(mut chs) = import_html(&text, source_name) {
                        for (chunk_i, ch) in chs.iter_mut().enumerate() {
                            ch.anchor.format = NovelFormat::Epub;
                            ch.anchor.locator = SourceLocator::EpubSpine {
                                resource: resolved.clone(),
                                fragment: fragment.clone(),
                                paragraph_index: chunk_i.saturating_add(1),
                                paragraph_count: para_count,
                            };
                            ch.index = chapters.len() + 1;
                            if let Some(ref mut ha) = ch.heading_anchor {
                                ha.format = NovelFormat::Epub;
                                ha.locator = SourceLocator::EpubSpine {
                                    resource: resolved.clone(),
                                    fragment: fragment.clone(),
                                    paragraph_index: chunk_i.saturating_add(1),
                                    paragraph_count: para_count,
                                };
                            }
                            total_character_count += ch.text.chars().count();
                        }
                        chapters.append(&mut chs);
                    }
                } else {
                    warnings.push(ImportWarning {
                        code: crate::ImportWarningCode::NoChapterHeadings,
                        message: format!("spine 项 {idref} 引用的资源 {resolved} 不存在，已跳过"),
                        anchor: None,
                    });
                }
            }
        }
    }

    // Fallback: if spine produced nothing, try all HTML/XHTML entries
    if chapters.is_empty() {
        let html_entries: Vec<_> = entries
            .iter()
            .filter(|e| {
                let name = e.name.to_lowercase();
                name.ends_with(".xhtml") || name.ends_with(".html") || name.ends_with(".htm")
            })
            .collect();

        for entry in html_entries {
            if chapters.len() >= max_chapters {
                break;
            }
            let text = String::from_utf8_lossy(&entry.data);
            if let Ok(mut chs) = import_html(&text, source_name) {
                for ch in &mut chs {
                    ch.anchor.format = NovelFormat::Epub;
                    let para_count = count_paragraphs(&text);
                    ch.anchor.locator = SourceLocator::EpubSpine {
                        resource: entry.name.clone(),
                        fragment: None,
                        paragraph_index: 1,
                        paragraph_count: para_count,
                    };
                    ch.index = chapters.len() + 1;
                    total_character_count += ch.text.chars().count();
                }
                chapters.append(&mut chs);
            }
        }
    }

    if chapters.is_empty() {
        return Err(ImportError::EmptyDocument {
            source_name: source_name.to_owned(),
        });
    }

    Ok(ImportedDocument {
        source: SourceDescriptor {
            display_name: source_name.to_owned(),
            format,
            media_type: Some("application/epub+zip".to_owned()),
            text_encoding: None,
        },
        stats: DocumentStats {
            chapter_count: chapters.len(),
            line_count: chapters.iter().map(|c| c.text.lines().count()).sum(),
            character_count: total_character_count,
            decoded_utf8_bytes: total_character_count,
        },
        chapters,
        warnings,
    })
}

/// Convenience entry point for internal callers (e.g., doesn't need full ImportRequest).
pub(crate) fn import_epub(
    bytes: &[u8],
    source_name: &str,
) -> Result<Vec<ImportedChapter>, ImportError> {
    let request = ImportRequest::new(source_name, bytes);
    let doc = import(request, NovelFormat::Epub)?;
    Ok(doc.chapters)
}

// ── OPF parsing helpers ──

fn extract_rootfile(container_xml: &str) -> Option<String> {
    // Simple extraction: look for rootfile full-path attribute
    let start = container_xml.find("<rootfile")?;
    let section = &container_xml[start..];
    let end = section.find("/>").or_else(|| section.find(">"))?;
    let tag = &section[..end];
    // Extract full-path="..."
    let attr = tag.find("full-path=\"")?;
    let after = &tag[attr + 11..];
    let close = after.find('"')?;
    let path = after[..close].to_owned();
    (!path.is_empty()).then_some(path)
}

fn extract_manifest(opf_xml: &str) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    // Find <manifest> ... </manifest>
    if let Some(manifest_start) = opf_xml.find("<manifest") {
        let after_start = &opf_xml[manifest_start..];
        if let Some(manifest_end) = after_start.find("</manifest>") {
            let manifest_section = &after_start[..manifest_end];
            // Extract item id="X" href="Y"
            for part in manifest_section.split("<item") {
                if let (Some(id), Some(href)) =
                    (extract_attr(part, "id"), extract_attr(part, "href"))
                {
                    map.insert(id, href);
                }
            }
        }
    }
    // Add idref values that map to themselves (fallback for HTML files directly referenced)
    if let Some(spine) = extract_spine_section(opf_xml) {
        for part in spine.split("itemref") {
            if let Some(idref) = extract_attr(part, "idref") {
                map.entry(idref.clone()).or_insert_with(|| {
                    // Guess .xhtml extension
                    format!("{}.xhtml", idref)
                });
            }
        }
    }
    map
}

fn extract_spine(opf_xml: &str) -> Vec<String> {
    let mut refs = Vec::new();
    if let Some(spine_section) = extract_spine_section(opf_xml) {
        for part in spine_section.split("itemref") {
            if let Some(idref) = extract_attr(part, "idref") {
                refs.push(idref);
            }
        }
    }
    refs
}

fn extract_spine_section(opf_xml: &str) -> Option<&str> {
    let start = opf_xml.find("<spine")?;
    let section = &opf_xml[start..];
    let end = section.find("</spine>")?;
    Some(&section[..end + "</spine>".len()])
}

fn extract_attr(tag: &str, attr_name: &str) -> Option<String> {
    let search = format!("{attr_name}=\"");
    let start = tag.find(&search)?;
    let after = &tag[start + search.len()..];
    let end = after.find('"')?;
    let value = after[..end].to_owned();
    (!value.is_empty()).then_some(value)
}

fn extract_nav_ids(opf_xml: &str) -> Vec<String> {
    let mut ids = Vec::new();
    // Find manifest items with properties="nav" or epub:type="nav"
    if let Some(manifest_start) = opf_xml.find("<manifest") {
        let after_start = &opf_xml[manifest_start..];
        if let Some(manifest_end) = after_start.find("</manifest>") {
            let section = &after_start[..manifest_end];
            for part in section.split("<item") {
                let is_nav =
                    part.contains("properties=\"nav\"") || part.contains("epub:type=\"nav\"");
                if is_nav {
                    if let Some(id) = extract_attr(part, "id") {
                        ids.push(id);
                    }
                }
            }
        }
    }
    ids
}

fn split_fragment(href: &str) -> (String, Option<String>) {
    if let Some(pos) = href.find('#') {
        let base = href[..pos].to_owned();
        let fragment = href[pos + 1..].to_owned();
        (
            base,
            if fragment.is_empty() {
                None
            } else {
                Some(fragment)
            },
        )
    } else {
        (href.to_owned(), None)
    }
}

fn count_paragraphs(html: &str) -> usize {
    // Count <p> opening tags
    let lower = html.to_lowercase();
    lower.matches("<p>").count()
        + lower.matches("<p ").count()
        + lower.matches("</p>").count().max(1)
}

fn resolve_opf_relative(opf_path: &str, href: &str) -> String {
    // OPF is typically in the root or OEBPS/ or content/
    // Resolve relative href against OPF directory
    if href.contains("://") || href.starts_with('/') {
        return href.to_owned();
    }
    let dir = std::path::Path::new(opf_path)
        .parent()
        .and_then(|p| p.to_str())
        .unwrap_or("");
    if dir.is_empty() {
        href.to_owned()
    } else {
        format!("{}/{}", dir, href)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import_novel;

    fn make_minimal_epub() -> Vec<u8> {
        // Build a minimal valid EPUB in memory
        use std::io::Write;

        let mut buf = Vec::new();
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));

        let options: zip::write::FileOptions<()> = zip::write::FileOptions::<()>::default()
            .compression_method(zip::CompressionMethod::Stored);

        // mimetype (must be first, uncompressed)
        zip.start_file("mimetype", options).unwrap();
        zip.write_all(b"application/epub+zip").unwrap();

        // META-INF/container.xml
        zip.start_file("META-INF/container.xml", options).unwrap();
        zip.write_all(
            b"<?xml version=\"1.0\"?><container version=\"1.0\" xmlns=\"urn:oasis:names:tc:opendocument:xmlns:container\"><rootfiles><rootfile full-path=\"content.opf\" media-type=\"application/oebps-package+xml\"/></rootfiles></container>"
        ).unwrap();

        // content.opf
        zip.start_file("content.opf", options).unwrap();
        zip.write_all(
            b"<?xml version=\"1.0\"?><package xmlns=\"http://www.idpf.org/2007/opf\" version=\"3.0\"><manifest><item id=\"ch1\" href=\"ch1.xhtml\" media-type=\"application/xhtml+xml\"/><item id=\"ch2\" href=\"ch2.xhtml\" media-type=\"application/xhtml+xml\"/></manifest><spine><itemref idref=\"ch1\"/><itemref idref=\"ch2\"/></spine></package>"
        ).unwrap();

        // ch1.xhtml
        zip.start_file("ch1.xhtml", options).unwrap();
        zip.write_all(
            b"<?xml version=\"1.0\"?><html xmlns=\"http://www.w3.org/1999/xhtml\"><body><h1>Chapter 1</h1><p>Content one.</p></body></html>"
        ).unwrap();

        // ch2.xhtml
        zip.start_file("ch2.xhtml", options).unwrap();
        zip.write_all(
            b"<?xml version=\"1.0\"?><html xmlns=\"http://www.w3.org/1999/xhtml\"><body><h1>Chapter 2</h1><p>Content two.</p></body></html>"
        ).unwrap();

        zip.finish().unwrap();
        buf
    }

    // ── Normal ──

    #[test]
    fn imports_minimal_epub_with_spine_order() {
        let epub_bytes = make_minimal_epub();
        let doc = import_novel(ImportRequest::new("test.epub", &epub_bytes)).unwrap();
        assert_eq!(doc.chapters.len(), 2);
        assert_eq!(doc.chapters[0].title, "Chapter 1");
        assert_eq!(doc.chapters[1].title, "Chapter 2");
        assert_eq!(doc.source.format, NovelFormat::Epub);
    }

    // ── Corrupt ──

    #[test]
    fn empty_zip_is_rejected() {
        let empty = [
            0x50, 0x4b, 0x05, 0x06, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let err = import_novel(ImportRequest::new("empty.epub", &empty)).unwrap_err();
        assert!(matches!(err, ImportError::EmptyDocument { .. }));
    }

    #[test]
    fn encrypted_zip_is_rejected() {
        let mut encrypted = vec![0x50, 0x4b, 0x03, 0x04];
        encrypted.extend_from_slice(&[0x14, 0x00, 0x01, 0x00]);
        encrypted.extend_from_slice(&[0x08, 0x00]);
        encrypted.extend_from_slice(&[0; 16]);
        encrypted.extend_from_slice(&[1, 0]);
        encrypted.extend_from_slice(&[0; 4]);
        encrypted.push(b'a');
        encrypted.extend_from_slice(&[
            0x50, 0x4b, 0x05, 0x06, 0, 0, 0, 0, 1, 0, 1, 0, 40, 0, 0, 0, 25, 0, 0, 0, 0, 0,
        ]);
        assert!(import_novel(ImportRequest::new("encrypted.epub", &encrypted)).is_err());
    }

    #[test]
    fn bad_mimetype_rejected() {
        use std::io::Write;
        let mut buf = Vec::new();
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        let options = zip::write::FileOptions::<()>::default()
            .compression_method(zip::CompressionMethod::Stored);
        zip.start_file("mimetype", options).unwrap();
        zip.write_all(b"text/plain").unwrap();
        zip.start_file("META-INF/container.xml", options).unwrap();
        zip.write_all(
            b"<container><rootfiles><rootfile full-path=\"x.opf\"/></rootfiles></container>",
        )
        .unwrap();
        zip.start_file("x.opf", options).unwrap();
        zip.write_all(b"<package><manifest><item id=\"c1\" href=\"c1.xhtml\" media-type=\"application/xhtml+xml\"/></manifest><spine><itemref idref=\"c1\"/></spine></package>").unwrap();
        zip.start_file("c1.xhtml", options).unwrap();
        zip.write_all(b"<html><body><p>text</p></body></html>")
            .unwrap();
        zip.finish().unwrap();

        let err = import_novel(ImportRequest::new("bad.epub", &buf)).unwrap_err();
        assert!(matches!(err, ImportError::Corrupt { .. }));
    }

    #[test]
    fn missing_container_rejected() {
        use std::io::Write;
        let mut buf = Vec::new();
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        let options = zip::write::FileOptions::<()>::default()
            .compression_method(zip::CompressionMethod::Stored);
        zip.start_file("mimetype", options).unwrap();
        zip.write_all(b"application/epub+zip").unwrap();
        // No container.xml
        zip.finish().unwrap();

        let err = import_novel(ImportRequest::new("nocontainer.epub", &buf)).unwrap_err();
        assert!(matches!(err, ImportError::Corrupt { .. }));
    }

    // ── Overlimit ──

    #[test]
    fn respects_chapter_limit() {
        use std::io::Write;
        let mut buf = Vec::new();
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        let options = zip::write::FileOptions::<()>::default()
            .compression_method(zip::CompressionMethod::Stored);
        zip.start_file("mimetype", options).unwrap();
        zip.write_all(b"application/epub+zip").unwrap();
        zip.start_file("META-INF/container.xml", options).unwrap();
        zip.write_all(
            b"<container><rootfiles><rootfile full-path=\"c.opf\"/></rootfiles></container>",
        )
        .unwrap();
        zip.start_file("c.opf", options).unwrap();
        zip.write_all(b"<package><manifest><item id=\"a\" href=\"a.xhtml\" media-type=\"application/xhtml+xml\"/><item id=\"b\" href=\"b.xhtml\" media-type=\"application/xhtml+xml\"/><item id=\"c\" href=\"c.xhtml\" media-type=\"application/xhtml+xml\"/></manifest><spine><itemref idref=\"a\"/><itemref idref=\"b\"/><itemref idref=\"c\"/></spine></package>").unwrap();
        zip.start_file("a.xhtml", options).unwrap();
        zip.write_all(b"<html><body><h1>A</h1><p>a</p></body></html>")
            .unwrap();
        zip.start_file("b.xhtml", options).unwrap();
        zip.write_all(b"<html><body><h1>B</h1><p>b</p></body></html>")
            .unwrap();
        zip.start_file("c.xhtml", options).unwrap();
        zip.write_all(b"<html><body><h1>C</h1><p>c</p></body></html>")
            .unwrap();
        zip.finish().unwrap();

        let limits = crate::model::ImportLimits {
            max_chapters: 2,
            ..crate::model::ImportLimits::default()
        };
        let options = crate::ImportOptions {
            limits,
            ..crate::ImportOptions::default()
        };
        let doc =
            import_novel(ImportRequest::new("limit.epub", &buf).with_options(options)).unwrap();
        assert_eq!(doc.chapters.len(), 2);
        assert!(doc.warnings.iter().any(|w| w.message.contains("上限")));
    }

    // ── Anchor regression ──

    #[test]
    fn epub_chapters_have_spine_locators() {
        let epub_bytes = make_minimal_epub();
        let doc = import_novel(ImportRequest::new("test.epub", &epub_bytes)).unwrap();
        assert_eq!(doc.chapters.len(), 2);
        for ch in &doc.chapters {
            assert_eq!(ch.anchor.format, NovelFormat::Epub);
            match &ch.anchor.locator {
                SourceLocator::EpubSpine { resource, .. } => {
                    assert!(resource.ends_with(".xhtml"));
                }
                _ => panic!("expected EpubSpine locator, got {:?}", ch.anchor.locator),
            }
        }
    }
}
