use serde::{Deserialize, Serialize};

/// The community-defined bucket a rule belongs to. This is independent from
/// how prominently an individual user wants the app to surface a match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleCategory {
    Landmine,
    Frustration,
}

/// How far the engine must search before a rule can be confirmed.
/// Cross-chapter and whole-book scopes are capped at `pending_confirmation`
/// during S1 single-pass scanning; cross-chapter re-verification is planned
/// for a later milestone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfirmationScope {
    Local,
    Chapter,
    CrossChapter,
    WholeBook,
}

/// User-facing alert strength. The stable JSON representation mirrors the
/// rule pack and persistence layers; UI clients may render it as a 1..=5
/// scale via [`AlertLevel::ui_scale`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertLevel {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

impl AlertLevel {
    /// Converts the front-end 1..=5 setting to the provider-neutral domain
    /// value. Values outside that range are rejected instead of clamped.
    pub const fn from_ui_scale(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::Info),
            2 => Some(Self::Low),
            3 => Some(Self::Medium),
            4 => Some(Self::High),
            5 => Some(Self::Critical),
            _ => None,
        }
    }

    pub const fn ui_scale(self) -> u8 {
        match self {
            Self::Info => 1,
            Self::Low => 2,
            Self::Medium => 3,
            Self::High => 4,
            Self::Critical => 5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DetectionMode {
    Semantic,
    ManualOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleDefinition {
    pub id: String,
    pub version: u32,
    pub name: String,
    pub description: String,
    pub category: RuleCategory,
    pub default_alert_level: AlertLevel,
    pub confirmation_scope: ConfirmationScope,
    /// When true the rule requires user-specified personal boundaries before
    /// a finding can be confirmed. In S1 (without boundary UI) this caps
    /// confirmed findings at pending_confirmation.
    #[serde(default)]
    pub requires_user_boundary: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub detection_profile_ref: Option<String>,
    pub detection_mode: DetectionMode,
    #[serde(default)]
    pub criteria: Vec<String>,
    #[serde(default)]
    pub exclusions: Vec<String>,
    #[serde(default)]
    pub pending_conditions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleSelection {
    pub rule_id: String,
    /// Snapshot of the rule-pack category. Resolution rejects a mismatch so a
    /// changed rule pack cannot silently reclassify an in-progress scan.
    pub category: RuleCategory,
    pub alert_level: AlertLevel,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

const fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Running,
    Paused,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanConfig {
    /// Character budget for the rolling context snapshot, not a model token
    /// count. Provider adapters may apply a stricter token budget.
    pub context_budget_chars: usize,
    /// Invalid or missing evidence never becomes a confirmed finding. When
    /// this flag is true it is retained as a suspected item for human review.
    pub retain_unverified_candidates: bool,
    /// Snapshot of the rule pack identity at scan start. Included in the
    /// checkpoint fingerprint so resume can reject a changed rule pack.
    #[serde(default)]
    pub rule_pack_id_snapshot: Option<String>,
    #[serde(default)]
    pub rule_pack_version_snapshot: Option<String>,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            context_budget_chars: 8_000,
            retain_unverified_candidates: true,
            rule_pack_id_snapshot: None,
            rule_pack_version_snapshot: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NovelTask {
    pub id: String,
    pub document_id: String,
    pub status: TaskStatus,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub selected_rules: Vec<RuleSelection>,
    #[serde(default)]
    pub config: ScanConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentFormat {
    PlainText,
    Epub,
    Pdf,
    Docx,
    Markdown,
    Html,
    Mobi,
    Azw3,
    Archive,
    Other,
}

/// A format-aware locator supplied by an importer. It is data-only so it can
/// cross a Tauri command boundary and be persisted by both desktop and mobile
/// implementations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SourceLocator {
    PlainText {
        line_start: u32,
        line_end: u32,
    },
    Epub {
        href: String,
        fragment: Option<String>,
    },
    Pdf {
        page_start: u32,
        page_end: u32,
    },
    Docx {
        paragraph_start: u32,
        paragraph_end: u32,
    },
    Markdown {
        line_start: u32,
        line_end: u32,
    },
    Html {
        resource: String,
        selector: Option<String>,
    },
    EbookResource {
        format: String,
        resource: String,
    },
    ArchiveEntry {
        archive_name: String,
        entry_name: String,
        inner: Box<SourceLocator>,
    },
    /// Used for Android Storage Access Framework URIs and equivalent desktop
    /// document handles without teaching the core how to open either.
    PlatformDocument {
        uri: String,
        display_name: String,
    },
    Unknown {
        description: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chapter {
    pub id: String,
    /// Zero-based order after import. Display layers may render it one-based.
    pub ordinal: u32,
    pub title: String,
    pub text: String,
    pub locator: SourceLocator,
    /// Stable FNV-1a fingerprint of the imported chapter text.
    pub content_hash: String,
}

impl Chapter {
    pub fn new(
        id: impl Into<String>,
        ordinal: u32,
        title: impl Into<String>,
        text: impl Into<String>,
        locator: SourceLocator,
    ) -> Self {
        let text = text.into();
        let content_hash = stable_fingerprint(text.as_bytes());
        Self {
            id: id.into(),
            ordinal,
            title: title.into(),
            text,
            locator,
            content_hash,
        }
    }

    pub fn computed_content_hash(&self) -> String {
        stable_fingerprint(self.text.as_bytes())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NovelDocument {
    pub id: String,
    pub title: String,
    pub source_name: String,
    pub format: DocumentFormat,
    pub chapters: Vec<Chapter>,
    /// Includes chapter order, IDs, and content, allowing resume to reject a
    /// changed source instead of silently attaching stale evidence.
    pub fingerprint: String,
}

impl NovelDocument {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        source_name: impl Into<String>,
        format: DocumentFormat,
        chapters: Vec<Chapter>,
    ) -> Self {
        let mut document = Self {
            id: id.into(),
            title: title.into(),
            source_name: source_name.into(),
            format,
            chapters,
            fingerprint: String::new(),
        };
        document.fingerprint = document.computed_fingerprint();
        document
    }

    pub fn computed_fingerprint(&self) -> String {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(self.id.as_bytes());
        bytes.push(0xff);
        bytes.extend_from_slice(self.title.as_bytes());
        bytes.push(0xfc);
        bytes.extend_from_slice(self.source_name.as_bytes());
        bytes.push(0xfb);
        bytes.extend_from_slice(format!("{:?}", self.format).as_bytes());
        bytes.push(0xfa);
        for chapter in &self.chapters {
            bytes.extend_from_slice(chapter.id.as_bytes());
            bytes.push(0xfe);
            bytes.extend_from_slice(chapter.ordinal.to_string().as_bytes());
            bytes.push(0xf9);
            bytes.extend_from_slice(chapter.title.as_bytes());
            bytes.push(0xf8);
            bytes.extend_from_slice(format!("{:?}", chapter.locator).as_bytes());
            bytes.push(0xf7);
            bytes.extend_from_slice(chapter.text.as_bytes());
            bytes.push(0xfd);
        }
        stable_fingerprint(&bytes)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChapterRef {
    pub document_id: String,
    pub chapter_id: String,
    pub chapter_ordinal: u32,
    pub chapter_title: String,
    pub locator: SourceLocator,
}

impl ChapterRef {
    pub fn from_chapter(document_id: &str, chapter: &Chapter) -> Self {
        Self {
            document_id: document_id.to_owned(),
            chapter_id: chapter.id.clone(),
            chapter_ordinal: chapter.ordinal,
            chapter_title: chapter.title.clone(),
            locator: chapter.locator.clone(),
        }
    }
}

/// Byte offsets are explicit because Rust strings and persisted imported text
/// are UTF-8. Line numbers are one-based and intended for human display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextSpan {
    pub utf8_byte_start: usize,
    pub utf8_byte_end: usize,
    pub line_start: u32,
    pub line_end: u32,
}

impl TextSpan {
    pub fn from_valid_range(text: &str, start: usize, end: usize) -> Self {
        debug_assert!(text.get(start..end).is_some());
        let end_position = if end > start { end - 1 } else { end };
        Self {
            utf8_byte_start: start,
            utf8_byte_end: end,
            line_start: line_number_at(text, start),
            line_end: line_number_at(text, end_position),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceAnchor {
    pub source: ChapterRef,
    pub span: TextSpan,
    pub exact_quote: String,
    pub quote_hash: String,
    pub chapter_content_hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingStatus {
    Suspected,
    PendingConfirmation,
    Confirmed,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderStamp {
    pub provider_id: String,
    pub model_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub rule_id: String,
    pub rule_version: u32,
    pub category: RuleCategory,
    pub alert_level: AlertLevel,
    /// Integer basis points (0..=10_000) avoid platform-dependent float JSON.
    pub confidence_bps: u16,
    pub rationale: String,
    pub status: FindingStatus,
    /// Always identifies at least the chapter, including for suspected items.
    pub source: ChapterRef,
    /// Confirmed findings have one or more exact anchors reconstructed from the
    /// original chapter, never copied from model output.
    pub evidence: Vec<EvidenceAnchor>,
    pub verification_note: Option<String>,
    pub provider: ProviderStamp,
}

/// Returns the largest valid UTF-8 byte boundary ≤ `position` in `text`.
/// Never splits multi-byte code points. Returns 0 for empty or 0-position.
pub fn safe_utf8_boundary(text: &str, position: usize) -> usize {
    let position = position.min(text.len());
    if position == 0 {
        return 0;
    }
    let bytes = text.as_bytes();
    let mut boundary = position;
    // Walk back at most 3 bytes to find a non-continuation byte
    let steps = (position - 1).saturating_sub(3.min(position));
    for offset in (steps..=position - 1).rev() {
        if bytes[offset] & 0xC0 != 0x80 {
            boundary = offset + 1;
            break;
        }
    }
    boundary.min(position)
}

/// Splits `text` into chunks of at most `max_chars` Unicode scalar characters,
/// each starting and ending on a valid UTF-8 byte boundary.
pub fn chunk_text(text: &str, max_chars: usize) -> Vec<&str> {
    let mut chunks = Vec::new();
    let mut start = 0usize;
    let mut char_count = 0usize;

    for (byte_offset, _ch) in text.char_indices() {
        char_count += 1;
        if char_count > max_chars {
            let boundary = safe_utf8_boundary(text, byte_offset);
            chunks.push(&text[start..boundary]);
            start = boundary;
            char_count = 1;
        }
    }
    if start < text.len() {
        chunks.push(&text[start..]);
    }

    chunks
}

pub(crate) fn line_number_at(text: &str, byte_position: usize) -> u32 {
    let clamped = byte_position.min(text.len());
    1 + text.as_bytes()[..clamped]
        .iter()
        .filter(|byte| **byte == b'\n')
        .count() as u32
}

/// Stable, dependency-free FNV-1a fingerprint. It is an identity/checkpoint
/// guard, not a cryptographic integrity primitive.
pub fn stable_fingerprint(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_span_uses_utf8_bytes_and_human_lines() {
        let text = "序章\n他选择了接盘。\n尾声";
        let start = text.find("接盘").unwrap();
        let end = start + "接盘".len();
        let span = TextSpan::from_valid_range(text, start, end);

        assert_eq!(&text[span.utf8_byte_start..span.utf8_byte_end], "接盘");
        assert_eq!(span.line_start, 2);
        assert_eq!(span.line_end, 2);
    }

    #[test]
    fn document_fingerprint_changes_with_source_text() {
        let chapter = Chapter::new(
            "c1",
            0,
            "第一章",
            "原文",
            SourceLocator::PlainText {
                line_start: 1,
                line_end: 1,
            },
        );
        let original = NovelDocument::new(
            "book",
            "书",
            "book.txt",
            DocumentFormat::PlainText,
            vec![chapter.clone()],
        );
        let mut changed_chapter = chapter;
        changed_chapter.text.push('改');
        let changed = NovelDocument::new(
            "book",
            "书",
            "book.txt",
            DocumentFormat::PlainText,
            vec![changed_chapter],
        );

        assert_ne!(original.fingerprint, changed.fingerprint);
    }

    #[test]
    fn alert_levels_round_trip_the_front_end_scale() {
        let levels = [
            AlertLevel::Info,
            AlertLevel::Low,
            AlertLevel::Medium,
            AlertLevel::High,
            AlertLevel::Critical,
        ];

        for (index, level) in levels.into_iter().enumerate() {
            let scale = (index + 1) as u8;
            assert_eq!(level.ui_scale(), scale);
            assert_eq!(AlertLevel::from_ui_scale(scale), Some(level));
        }
        assert_eq!(AlertLevel::from_ui_scale(0), None);
        assert_eq!(AlertLevel::from_ui_scale(6), None);
    }

    #[test]
    fn safe_boundary_preserves_ascii_positions() {
        let text = "hello world";
        assert_eq!(safe_utf8_boundary(text, 5), 5);
        assert_eq!(safe_utf8_boundary(text, 0), 0);
        assert_eq!(safe_utf8_boundary(text, 999), text.len());
    }

    #[test]
    fn safe_boundary_does_not_split_multi_byte_utf8() {
        // "é" is 2 bytes: 0xC3 0xA9
        let text = "abcéfg";
        // byte position 4 is right after 'c', at start of 'é'
        assert_eq!(safe_utf8_boundary(text, 4), 4);
        // byte position 5 is in the middle of 'é' (the continuation byte 0xA9)
        let boundary = safe_utf8_boundary(text, 5);
        assert_eq!(&text[boundary..], "éfg");
    }

    #[test]
    fn chunk_text_respects_max_chars() {
        let text = "1234567890"; // 10 ASCII chars
        let chunks = chunk_text(text, 4);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0], "1234");
        assert_eq!(chunks[1], "5678");
        assert_eq!(chunks[2], "90");
    }

    #[test]
    fn category_alert_level_and_all_finding_states_use_snake_case_json() {
        assert_eq!(
            serde_json::to_string(&RuleCategory::Landmine).unwrap(),
            "\"landmine\""
        );
        assert_eq!(
            serde_json::to_string(&AlertLevel::Critical).unwrap(),
            "\"critical\""
        );
        let states = [
            (FindingStatus::Suspected, "\"suspected\""),
            (
                FindingStatus::PendingConfirmation,
                "\"pending_confirmation\"",
            ),
            (FindingStatus::Confirmed, "\"confirmed\""),
            (FindingStatus::Rejected, "\"rejected\""),
        ];
        for (state, expected) in states {
            let json = serde_json::to_string(&state).unwrap();
            assert_eq!(json, expected);
            assert_eq!(serde_json::from_str::<FindingStatus>(&json).unwrap(), state);
        }
    }
}
