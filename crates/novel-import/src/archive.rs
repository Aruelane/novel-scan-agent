//! Memory-only safe ZIP enumeration. Rejects path traversal, absolute
//! paths, and DRM/encrypted entries. Used by EPUB and DOCX importers.

use std::io::{Cursor, Read};

use crate::ImportError;

pub struct ArchiveEntry {
    pub name: String,
    pub data: Vec<u8>,
}

pub fn enumerate_zip(
    source_name: &str,
    bytes: &[u8],
    max_entry_bytes: usize,
    max_total: usize,
) -> Result<Vec<ArchiveEntry>, ImportError> {
    let cursor = Cursor::new(bytes);
    let mut archive =
        zip::ZipArchive::new(cursor).map_err(|e| ImportError::Corrupt {
            source_name: source_name.to_owned(),
            detail: format!("无法打开 ZIP 容器：{e}"),
        })?;

    let entry_count = archive.len();
    if entry_count == 0 {
        return Err(ImportError::EmptyDocument {
            source_name: source_name.to_owned(),
        });
    }

    let mut entries = Vec::with_capacity(entry_count);
    let mut total_uncompressed = 0usize;

    for idx in 0..entry_count {
        let mut entry = archive.by_index(idx).map_err(|e| ImportError::Corrupt {
            source_name: source_name.to_owned(),
            detail: format!("无法读取 ZIP 条目 {idx}：{e}"),
        })?;

        let name = entry.name().to_owned();

        // Reject directory traversal
        if name.contains("..")
            || name.contains('\\')
            || name.starts_with('/')
            || name.chars().any(|c| c == ':' && name.contains(":\\"))
        {
            return Err(ImportError::Corrupt {
                source_name: source_name.to_owned(),
                detail: format!("ZIP 条目 '{name}' 包含不安全的路径"),
            });
        }

        if entry.is_dir() {
            continue;
        }

        // Reject encrypted entries
        if entry.version_needed() & 1 != 0 {
            return Err(ImportError::Protected {
                source_name: source_name.to_owned(),
                detail: format!("ZIP 条目 '{name}' 已加密"),
            });
        }

        let size = entry.size() as usize;
        if size > max_entry_bytes {
            return Err(ImportError::LimitExceeded {
                source_name: source_name.to_owned(),
                limit: "单条目大小".to_owned(),
                detail: format!("条目 '{name}' 声明 {size} 字节，超过上限 {max_entry_bytes}"),
            });
        }

        total_uncompressed += size;
        if total_uncompressed > max_total {
            return Err(ImportError::LimitExceeded {
                source_name: source_name.to_owned(),
                limit: "总展开大小".to_owned(),
                detail: format!("超过上限 {max_total} 字节"),
            });
        }

        let mut data = vec![0u8; size];
        entry
            .read_exact(&mut data)
            .map_err(|e| ImportError::Corrupt {
                source_name: source_name.to_owned(),
                detail: format!("读取 ZIP 条目 '{name}' 失败：{e}"),
            })?;

        entries.push(ArchiveEntry { name, data });
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_zip_is_rejected() {
        // Minimal empty ZIP file (EOCD only)
        let empty_zip = [
            0x50, 0x4b, 0x05, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let result = enumerate_zip("empty.zip", &empty_zip, 1_000_000, 10_000_000);
        assert!(result.is_err());
    }
}
