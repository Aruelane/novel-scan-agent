//! EPUB import via safe ZIP enumeration + HTML extraction.
//! Reuses archive.rs for ZIP safety and html.rs for content extraction.

use crate::archive::enumerate_zip;
use crate::html::import_html;
use crate::{ImportError, ImportedChapter, NovelFormat};

pub(crate) fn import_epub(
    bytes: &[u8],
    source_name: &str,
) -> Result<Vec<ImportedChapter>, ImportError> {
    let entries = enumerate_zip(source_name, bytes, 20_000_000, 100_000_000)?;

    // Find the OPF file (content.opf or *.opf in root or META-INF)
    let opf_entry = entries.iter().find(|e| {
        let name = e.name.to_lowercase();
        name.ends_with(".opf") && !name.contains('/')
    });

    let mut spine_hrefs = Vec::new();
    if let Some(opf) = opf_entry {
        let opf_text = String::from_utf8_lossy(&opf.data);
        // Simple spine extraction: find <itemref idref="..." /> in <spine>
        if let Some(spine_start) = opf_text.find("<spine") {
            let spine_section = &opf_text[spine_start..];
            if let Some(spine_end) = spine_section.find("</spine>") {
                let spine = &spine_section[..spine_end];
                for part in spine.split("itemref") {
                    if let Some(idref_start) = part.find("idref=\"") {
                        let after = &part[idref_start + 7..];
                        if let Some(idref_end) = after.find('"') {
                            spine_hrefs.push(after[..idref_end].to_owned());
                        }
                    }
                }
            }
        }
    }

    // Collect XHTML/HTML chapters from spine order or all entries
    let mut chapters = Vec::new();
    let html_entries: Vec<_> = entries
        .iter()
        .filter(|e| {
            let name = e.name.to_lowercase();
            name.ends_with(".xhtml") || name.ends_with(".html") || name.ends_with(".htm")
        })
        .collect();

    // Try spine-ordered first, then fall back to all HTML entries
    if !spine_hrefs.is_empty() {
        for href in &spine_hrefs {
            let href_lower = href.to_lowercase();
            if let Some(entry) = html_entries.iter().find(|e| {
                e.name.to_lowercase().ends_with(&href_lower)
                    || e.name.to_lowercase().contains(&href_lower)
            }) {
                let text = String::from_utf8_lossy(&entry.data);
                if let Ok(mut chs) = import_html(&text, source_name) {
                    chapters.append(&mut chs);
                }
            }
        }
    }

    // Fallback: all HTML entries
    if chapters.is_empty() {
        for entry in &html_entries {
            let text = String::from_utf8_lossy(&entry.data);
            if let Ok(mut chs) = import_html(&text, source_name) {
                chapters.append(&mut chs);
            }
        }
    }

    if chapters.is_empty() {
        return Err(ImportError::EmptyDocument {
            source_name: source_name.to_owned(),
        });
    }

    // Re-index chapters
    for (i, ch) in chapters.iter_mut().enumerate() {
        ch.index = i + 1;
    }

    Ok(chapters)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_zip_is_rejected() {
        let empty = [
            0x50, 0x4b, 0x05, 0x06, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        assert!(import_epub(&empty, "empty.epub").is_err());
    }

    #[test]
    fn encrypted_zip_is_rejected() {
        // ZIP local file header with bit 0 set (encrypted)
        let mut encrypted = vec![0x50, 0x4b, 0x03, 0x04];
        encrypted.extend_from_slice(&[0x14, 0x00, 0x01, 0x00]); // version + GP flag bit 0
        encrypted.extend_from_slice(&[0x08, 0x00]); // deflate
        encrypted.extend_from_slice(&[0; 16]); // times/crc/sizes
        encrypted.extend_from_slice(&[1, 0]); // filename len
        encrypted.extend_from_slice(&[0; 4]); // extra len
        encrypted.push(b'a'); // filename
                              // EOCD
        encrypted.extend_from_slice(&[
            0x50, 0x4b, 0x05, 0x06, 0, 0, 0, 0, 1, 0, 1, 0, 40, 0, 0, 0, 25, 0, 0, 0, 0, 0,
        ]);
        assert!(import_epub(&encrypted, "encrypted.epub").is_err());
    }
}
