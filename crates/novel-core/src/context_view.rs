//! Strictly bounded context view derived from a full checkpoint snapshot.
//! The view sent to the model must never exceed the configured character budget,
//! unlike the full `ContextSnapshot` which grows with every chapter.

use serde::{Deserialize, Serialize};

use crate::{
    compression::{EntityMemory, EventMemory, RelationshipMemory, UnresolvedMemory},
    Finding,
};

/// The bounded view sent to the model. It is derived from the full
/// `ContextSnapshot` but MUST fit within `context_budget_chars` Unicode
/// scalar characters (not tokens).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextView {
    pub revision: u64,
    pub rolling_summary: String,
    pub entity_count: usize,
    pub relationship_count: usize,
    pub event_count: usize,
    pub unresolved_count: usize,
    /// Active unresolved finding IDs carried over from the snapshot.
    pub unresolved_ids: Vec<String>,
}

impl ContextView {
    /// Build a bounded view from the full snapshot. The `budget_chars` is a
    /// hard cap on the `rolling_summary` character count.
    pub fn from_snapshot(snapshot: &crate::ContextSnapshot, budget_chars: usize) -> Self {
        Self {
            revision: snapshot.revision,
            rolling_summary: tail_chars(&snapshot.rolling_summary, budget_chars),
            entity_count: snapshot.entities.len(),
            relationship_count: snapshot.relationships.len(),
            event_count: snapshot.events.len(),
            unresolved_count: snapshot.unresolved_candidates.len(),
            unresolved_ids: snapshot.unresolved_candidates.clone(),
        }
    }

    /// The total character count of the rolling summary. Must always be
    /// `<= budget_chars`.
    pub fn summary_char_count(&self) -> usize {
        self.rolling_summary.chars().count()
    }
}

/// Returns up to `max_chars` Unicode scalar characters from the end of `text`.
fn tail_chars(text: &str, max_chars: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    let skip = chars.len().saturating_sub(max_chars);
    chars.into_iter().skip(skip).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ContextSnapshot;

    #[test]
    fn budget_respected() {
        let mut snapshot = ContextSnapshot::default();
        snapshot.rolling_summary = "ABCDEFGHIJ".to_owned(); // 10 chars
        let view = ContextView::from_snapshot(&snapshot, 6);
        assert_eq!(view.rolling_summary.len(), 6);
        assert_eq!(view.rolling_summary, "EFGHIJ");
    }

    #[test]
    fn empty_snapshot_produces_empty_view() {
        let snapshot = ContextSnapshot::default();
        let view = ContextView::from_snapshot(&snapshot, 100);
        assert_eq!(view.summary_char_count(), 0);
    }

    #[test]
    fn summary_never_exceeds_budget() {
        let mut snapshot = ContextSnapshot::default();
        snapshot.rolling_summary = "a".repeat(5000);
        let view = ContextView::from_snapshot(&snapshot, 200);
        assert!(view.summary_char_count() <= 200);
    }

    #[test]
    fn unresolved_ids_are_preserved() {
        let mut snapshot = ContextSnapshot::default();
        snapshot.unresolved_candidates = vec!["f1".into(), "f2".into()];
        let view = ContextView::from_snapshot(&snapshot, 100);
        assert_eq!(view.unresolved_ids, vec!["f1", "f2"]);
        assert_eq!(view.unresolved_count, 2);
    }
}
