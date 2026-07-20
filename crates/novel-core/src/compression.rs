use std::{fmt, future::Future, pin::Pin};

use serde::{Deserialize, Serialize};

use crate::{stable_fingerprint, Chapter, Finding};

pub type CompressionFuture<'a> =
    Pin<Box<dyn Future<Output = Result<ContextSnapshot, CompressionError>> + Send + 'a>>;

/// Current schema version for `ContextSnapshot`. Old snapshots default to 0
/// and are rejected on resume so they are never silently misinterpreted.
pub const CONTEXT_SNAPSHOT_SCHEMA_VERSION: u32 = 1;

// ── Memory ledger types ──────────────────────────────────────────────
// These are inference memories, not evidence. Every confirmed /
// pending_confirmation finding must reference exact source anchors
// reconstructed from `NovelDocument.chapters[*].text`.

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityMemory {
    pub entity_id: String,
    pub display_name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    /// Short, provider-generated state. It is context, never source evidence.
    pub state: String,
    /// Chapter where this entity was last updated.
    #[serde(default)]
    pub last_seen_chapter_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationshipMemory {
    pub relationship_id: String,
    pub entity_a_id: String,
    pub entity_b_id: String,
    pub relation_type: String,
    pub state: String,
    #[serde(default)]
    pub last_seen_chapter_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventMemory {
    pub event_id: String,
    pub description: String,
    /// Participating entity IDs.
    #[serde(default)]
    pub participant_ids: Vec<String>,
    /// "open" or "resolved".
    pub status: String,
    #[serde(default)]
    pub last_seen_chapter_id: Option<String>,
}

/// Memory ledger for unresolved findings. The finding ID is the unique identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnresolvedMemory {
    pub finding_id: String,
    pub rule_id: String,
    pub clue: String,
    pub source_chapter_id: String,
    pub last_seen_revision: u64,
}

/// Generates a stable, deterministic memory ID from a prefix and seed material.
pub fn memory_id(prefix: &str, seed: &str) -> String {
    format!("{prefix}_{}", stable_fingerprint(seed.as_bytes()))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextSnapshot {
    pub schema_version: u32,
    pub revision: u64,
    pub processed_chapter_ids: Vec<String>,
    pub rolling_summary: String,
    #[serde(default)]
    pub entities: Vec<EntityMemory>,
    #[serde(default)]
    pub relationships: Vec<RelationshipMemory>,
    #[serde(default)]
    pub events: Vec<EventMemory>,
    #[serde(default)]
    pub unresolved_memories: Vec<UnresolvedMemory>,
    /// Stable finding IDs that later chapters may confirm. Each ID must
    /// correspond to a Suspected or PendingConfirmation finding. They must
    /// never be surfaced as confirmed findings without exact source anchors.
    /// DEPRECATED: replaced by `unresolved_memories`. Kept for backward
    /// compatibility during schema migration; new code should write both.
    #[serde(default)]
    pub unresolved_candidates: Vec<String>,
}

impl Default for ContextSnapshot {
    fn default() -> Self {
        Self {
            schema_version: CONTEXT_SNAPSHOT_SCHEMA_VERSION,
            revision: 0,
            processed_chapter_ids: Vec::new(),
            rolling_summary: String::new(),
            entities: Vec::new(),
            relationships: Vec::new(),
            events: Vec::new(),
            unresolved_memories: Vec::new(),
            unresolved_candidates: Vec::new(),
        }
    }
}

impl ContextSnapshot {
    /// Returns true if this snapshot was produced by an older (pre-v1) schema
    /// that lacks the structured memory types.
    pub fn is_legacy(&self) -> bool {
        self.schema_version == 0
    }

    /// Detect duplicate IDs in a list of memories. Returns the first duplicate
    /// found, or `None` if all IDs are unique.
    pub fn first_duplicate_entity_id(entities: &[EntityMemory]) -> Option<&str> {
        let mut seen = std::collections::HashSet::new();
        for e in entities {
            if !seen.insert(e.entity_id.as_str()) {
                return Some(&e.entity_id);
            }
        }
        None
    }

    pub fn first_duplicate_relationship_id(rels: &[RelationshipMemory]) -> Option<&str> {
        let mut seen = std::collections::HashSet::new();
        for r in rels {
            if !seen.insert(r.relationship_id.as_str()) {
                return Some(&r.relationship_id);
            }
        }
        None
    }

    pub fn first_duplicate_event_id(events: &[EventMemory]) -> Option<&str> {
        let mut seen = std::collections::HashSet::new();
        for ev in events {
            if !seen.insert(ev.event_id.as_str()) {
                return Some(&ev.event_id);
            }
        }
        None
    }
}

#[derive(Debug, Clone)]
pub struct CompressionRequest<'a> {
    pub previous: &'a ContextSnapshot,
    pub chapter: &'a Chapter,
    pub chapter_findings: &'a [Finding],
    pub budget_chars: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompressionError {
    pub message: String,
    pub retryable: bool,
}

impl CompressionError {
    pub fn new(message: impl Into<String>, retryable: bool) -> Self {
        Self {
            message: message.into(),
            retryable,
        }
    }
}

impl fmt::Display for CompressionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for CompressionError {}

pub trait ContextCompressor: Send + Sync {
    fn compress<'a>(&'a self, request: CompressionRequest<'a>) -> CompressionFuture<'a>;
}

/// A predictable compressor for development and offline operation. Production
/// adapters can replace it with a provider-backed structured summarizer while
/// preserving the same checkpoint format.
#[derive(Debug, Clone)]
pub struct DeterministicContextCompressor {
    pub excerpt_chars_per_chapter: usize,
}

impl Default for DeterministicContextCompressor {
    fn default() -> Self {
        Self {
            excerpt_chars_per_chapter: 160,
        }
    }
}

impl ContextCompressor for DeterministicContextCompressor {
    fn compress<'a>(&'a self, request: CompressionRequest<'a>) -> CompressionFuture<'a> {
        Box::pin(async move {
            let excerpt = take_chars(&request.chapter.text, self.excerpt_chars_per_chapter);
            let addition = format!("[{}] {}", request.chapter.title, excerpt.trim());
            let combined = if request.previous.rolling_summary.is_empty() {
                addition
            } else {
                format!("{}\n{}", request.previous.rolling_summary, addition)
            };

            let mut next = (*request.previous).clone();
            next.revision = next.revision.saturating_add(1);
            if !next
                .processed_chapter_ids
                .iter()
                .any(|id| id == &request.chapter.id)
            {
                next.processed_chapter_ids.push(request.chapter.id.clone());
            }
            next.rolling_summary = tail_chars(&combined, request.budget_chars);
            for finding_id in request
                .chapter_findings
                .iter()
                .filter(|finding| {
                    matches!(
                        finding.status,
                        crate::FindingStatus::Suspected | crate::FindingStatus::PendingConfirmation
                    )
                })
                .map(|finding| &finding.id)
            {
                if !next
                    .unresolved_candidates
                    .iter()
                    .any(|existing| existing == finding_id)
                {
                    next.unresolved_candidates.push(finding_id.clone());
                }
            }
            Ok(next)
        })
    }
}

fn take_chars(value: &str, limit: usize) -> String {
    value.chars().take(limit).collect()
}

fn tail_chars(value: &str, limit: usize) -> String {
    if limit == 0 {
        return String::new();
    }
    let count = value.chars().count();
    if count <= limit {
        return value.to_owned();
    }
    value.chars().skip(count - limit).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        future::Future,
        sync::Arc,
        task::{Context, Poll, Wake, Waker},
    };

    use super::*;
    use crate::SourceLocator;

    struct NoopWake;
    impl Wake for NoopWake {
        fn wake(self: Arc<Self>) {}
    }

    fn ready<F: Future>(future: F) -> F::Output {
        let waker = Waker::from(Arc::new(NoopWake));
        let mut context = Context::from_waker(&waker);
        let mut future = Box::pin(future);
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => output,
            Poll::Pending => panic!("deterministic future unexpectedly yielded"),
        }
    }

    #[test]
    fn deterministic_compression_respects_unicode_character_budget() {
        let compressor = DeterministicContextCompressor {
            excerpt_chars_per_chapter: 20,
        };
        let chapter = Chapter::new(
            "c1",
            0,
            "章",
            "一二三四五六七八九十",
            SourceLocator::Unknown {
                description: "test".into(),
            },
        );
        let previous = ContextSnapshot::default();
        let snapshot = ready(compressor.compress(CompressionRequest {
            previous: &previous,
            chapter: &chapter,
            chapter_findings: &[],
            budget_chars: 6,
        }))
        .unwrap();

        assert_eq!(snapshot.rolling_summary.chars().count(), 6);
        assert_eq!(snapshot.processed_chapter_ids, vec!["c1"]);
    }

    // ── S3-01: memory ledger types ──

    #[test]
    fn context_snapshot_default_has_current_schema_version() {
        let snapshot = ContextSnapshot::default();
        assert_eq!(snapshot.schema_version, CONTEXT_SNAPSHOT_SCHEMA_VERSION);
        assert!(!snapshot.is_legacy());
    }

    #[test]
    fn legacy_schema_is_detected() {
        let mut snapshot = ContextSnapshot::default();
        snapshot.schema_version = 0;
        assert!(snapshot.is_legacy());
    }

    #[test]
    fn context_snapshot_json_round_trip() {
        let mut snapshot = ContextSnapshot::default();
        snapshot.entities.push(EntityMemory {
            entity_id: "ent-1".into(),
            display_name: "Alice".into(),
            aliases: vec!["A".into()],
            state: "active".into(),
            last_seen_chapter_id: Some("ch1".into()),
        });
        snapshot.relationships.push(RelationshipMemory {
            relationship_id: "rel-1".into(),
            entity_a_id: "ent-1".into(),
            entity_b_id: "ent-2".into(),
            relation_type: "ally".into(),
            state: "strong".into(),
            last_seen_chapter_id: Some("ch3".into()),
        });
        snapshot.events.push(EventMemory {
            event_id: "evt-1".into(),
            description: "battle".into(),
            participant_ids: vec!["ent-1".into(), "ent-2".into()],
            status: "open".into(),
            last_seen_chapter_id: Some("ch2".into()),
        });
        snapshot.unresolved_memories.push(UnresolvedMemory {
            finding_id: "f-1".into(),
            rule_id: "takeover".into(),
            clue: "identity".into(),
            source_chapter_id: "ch1".into(),
            last_seen_revision: 3,
        });

        let json = serde_json::to_string(&snapshot).unwrap();
        let restored: ContextSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(snapshot, restored);
    }

    #[test]
    fn old_json_missing_schema_version_becomes_zero() {
        let old_json = r#"{"revision":0,"processed_chapter_ids":[],"rolling_summary":"","entities":[],"unresolved_candidates":[]}"#;
        let restored: ContextSnapshot = serde_json::from_str(old_json).unwrap();
        assert_eq!(restored.schema_version, 0);
        assert!(restored.is_legacy());
    }

    #[test]
    fn duplicate_entity_id_is_detected() {
        let entities = vec![
            EntityMemory {
                entity_id: "dup".into(),
                ..Default::default()
            },
            EntityMemory {
                entity_id: "dup".into(),
                ..Default::default()
            },
        ];
        assert_eq!(
            ContextSnapshot::first_duplicate_entity_id(&entities),
            Some("dup")
        );
    }

    #[test]
    fn unique_entity_ids_have_no_duplicate() {
        let entities = vec![
            EntityMemory {
                entity_id: "a".into(),
                ..Default::default()
            },
            EntityMemory {
                entity_id: "b".into(),
                ..Default::default()
            },
        ];
        assert_eq!(ContextSnapshot::first_duplicate_entity_id(&entities), None);
    }

    #[test]
    fn memory_id_is_deterministic() {
        let a = memory_id("ent", "seed");
        let b = memory_id("ent", "seed");
        assert_eq!(a, b);
    }
}
