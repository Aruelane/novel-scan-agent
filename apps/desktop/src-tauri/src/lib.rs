use novel_core::DocumentFormat;
use novel_import::{capability_registry, CapabilityStatus, FormatCapability, NovelFormat};
use serde::Serialize;
use tauri_plugin_sql::{Migration, MigrationKind};

const DATABASE_URL: &str = "sqlite:novel-scout.db";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportCapabilityDto {
    format_id: &'static str,
    label: &'static str,
    extensions: &'static [&'static str],
    media_types: &'static [&'static str],
    status: &'static str,
    detail: &'static str,
    source_locator: &'static str,
    core_document_format: &'static str,
}

impl From<&'static FormatCapability> for ImportCapabilityDto {
    fn from(capability: &'static FormatCapability) -> Self {
        Self {
            format_id: capability.format.stable_id(),
            label: capability.label,
            extensions: capability.extensions,
            media_types: capability.media_types,
            status: capability_status_id(capability.status),
            detail: capability.detail,
            source_locator: capability.source_locator,
            core_document_format: document_format_id(to_core_document_format(capability.format)),
        }
    }
}

const fn capability_status_id(status: CapabilityStatus) -> &'static str {
    match status {
        CapabilityStatus::Ready => "ready",
        CapabilityStatus::Pending => "pending",
        CapabilityStatus::Unsupported => "unsupported",
    }
}

const fn to_core_document_format(format: NovelFormat) -> DocumentFormat {
    match format {
        NovelFormat::PlainText => DocumentFormat::PlainText,
        NovelFormat::Markdown => DocumentFormat::Markdown,
        NovelFormat::Epub => DocumentFormat::Epub,
        NovelFormat::Docx => DocumentFormat::Docx,
        NovelFormat::Pdf => DocumentFormat::Pdf,
        NovelFormat::Html => DocumentFormat::Html,
        NovelFormat::Mobi => DocumentFormat::Mobi,
        NovelFormat::Azw3 => DocumentFormat::Azw3,
        NovelFormat::Zip | NovelFormat::SevenZip => DocumentFormat::Archive,
        NovelFormat::LegacyDoc => DocumentFormat::Other,
    }
}

const fn document_format_id(format: DocumentFormat) -> &'static str {
    match format {
        DocumentFormat::PlainText => "plain_text",
        DocumentFormat::Epub => "epub",
        DocumentFormat::Pdf => "pdf",
        DocumentFormat::Docx => "docx",
        DocumentFormat::Markdown => "markdown",
        DocumentFormat::Html => "html",
        DocumentFormat::Mobi => "mobi",
        DocumentFormat::Azw3 => "azw3",
        DocumentFormat::Archive => "archive",
        DocumentFormat::Other => "other",
    }
}

#[tauri::command]
fn import_capabilities() -> Vec<ImportCapabilityDto> {
    capability_registry()
        .iter()
        .map(ImportCapabilityDto::from)
        .collect()
}

fn database_migrations() -> Vec<Migration> {
    vec![Migration {
        version: 1,
        description: "initial_novel_scan_schema",
        sql: include_str!("../migrations/0001_initial.sql"),
        kind: MigrationKind::Up,
    }]
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_sql::Builder::default()
                .add_migrations(DATABASE_URL, database_migrations())
                .build(),
        )
        .invoke_handler(tauri::generate_handler![import_capabilities])
        .run(tauri::generate_context!())
        .expect("error while running 小说扫评 Agent");
}

/// Converts a 1-based imported chapter index to a 0-based core ordinal.
/// Returns `None` for index == 0 or values that overflow `u32`.
fn import_index_to_core_ordinal(index: usize) -> Option<u32> {
    index.checked_sub(1).and_then(|v| u32::try_from(v).ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::*;

    #[test]
    fn chapter_index_1_maps_to_ordinal_0() {
        assert_eq!(import_index_to_core_ordinal(1), Some(0));
    }

    #[test]
    fn chapter_index_5_maps_to_ordinal_4() {
        assert_eq!(import_index_to_core_ordinal(5), Some(4));
    }

    #[test]
    fn consecutive_chapters_have_consecutive_ordinals() {
        assert_eq!(import_index_to_core_ordinal(1), Some(0));
        assert_eq!(import_index_to_core_ordinal(2), Some(1));
        assert_eq!(import_index_to_core_ordinal(3), Some(2));
    }

    #[test]
    fn index_zero_is_rejected() {
        assert_eq!(import_index_to_core_ordinal(0), None);
    }

    #[test]
    fn u32_max_plus_one_is_rejected() {
        let big = (u32::MAX as usize).saturating_add(1);
        assert_eq!(import_index_to_core_ordinal(big), None);
    }

    #[test]
    fn u32_max_maps_correctly() {
        assert_eq!(
            import_index_to_core_ordinal(u32::MAX as usize),
            Some(u32::MAX - 1)
        );
    }

    #[test]
    fn command_reports_every_import_registry_entry() {
        let capabilities = import_capabilities();

        assert_eq!(capabilities.len(), capability_registry().len());
        assert!(capabilities.iter().any(|item| {
            item.format_id == "txt"
                && item.status == "ready"
                && item.core_document_format == "plain_text"
        }));
        assert!(capabilities.iter().any(|item| {
            item.format_id == "zip"
                && item.status == "pending"
                && item.core_document_format == "archive"
        }));
    }

    #[test]
    fn every_capability_has_a_source_locator_contract() {
        assert!(import_capabilities()
            .iter()
            .all(|item| !item.source_locator.trim().is_empty()));
    }
}
