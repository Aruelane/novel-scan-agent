//! Persistence contract for atomic chapter commits.
//!
//! The scanner calls these hooks at well-defined safe points. SQLite, Room, or
//! an in-memory store implement the trait. A single chapter commit is atomic:
//! checkpoint, findings, evidence, and usage are written together.

use crate::{EvidenceAnchor, Finding, ProcessedChapter, ScanCheckpoint, ScanError, UsageTotals};

/// Outcome of a single-chapter commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChapterCommit {
    pub chapter_id: String,
    pub ordinal: u32,
    pub findings_count: usize,
    pub evidence_count: usize,
}

/// Application-level persistence contract for scanner checkpoints and findings.
/// The core calls these in the right order; implementations handle the actual
/// database transaction.
pub trait ScanPersistence: Send + Sync {
    /// Atomically persist the checkpoint after a chapter completes. Must also
    /// persist all findings and evidence from that chapter within the same
    /// write transaction.
    fn commit_chapter(
        &self,
        checkpoint: &ScanCheckpoint,
        processed: &[ProcessedChapter],
        new_findings: &[Finding],
        new_evidence: &[EvidenceAnchor],
        usage: &UsageTotals,
    ) -> Result<ChapterCommit, ScanError>;

    /// Load an existing checkpoint by task ID. Returns `None` if no checkpoint
    /// exists for this task yet.
    fn load_checkpoint(&self, task_id: &str) -> Result<Option<ScanCheckpoint>, ScanError>;

    /// Persist only the checkpoint without findings (e.g., for zero-chapter
    /// pause or status-only updates).
    fn save_checkpoint(&self, checkpoint: &ScanCheckpoint) -> Result<(), ScanError>;
}

/// In-memory persistence for tests and development. Not for production use.
#[derive(Debug, Default)]
pub struct InMemoryPersistence {
    checkpoint: std::sync::Mutex<Option<ScanCheckpoint>>,
    findings: std::sync::Mutex<Vec<Finding>>,
    evidence: std::sync::Mutex<Vec<EvidenceAnchor>>,
    commits: std::sync::Mutex<Vec<ChapterCommit>>,
}

impl ScanPersistence for InMemoryPersistence {
    fn commit_chapter(
        &self,
        checkpoint: &ScanCheckpoint,
        _processed: &[ProcessedChapter],
        new_findings: &[Finding],
        new_evidence: &[EvidenceAnchor],
        _usage: &UsageTotals,
    ) -> Result<ChapterCommit, ScanError> {
        let mut cp = self.checkpoint.lock().map_err(|_| {
            ScanError::Checkpoint(crate::CheckpointStoreError::new("lock poisoned"))
        })?;
        *cp = Some(checkpoint.clone());

        {
            let mut findings = self.findings.lock().map_err(|_| {
                ScanError::Checkpoint(crate::CheckpointStoreError::new("lock poisoned"))
            })?;
            findings.extend_from_slice(new_findings);
        }
        {
            let mut evidence = self.evidence.lock().map_err(|_| {
                ScanError::Checkpoint(crate::CheckpointStoreError::new("lock poisoned"))
            })?;
            evidence.extend_from_slice(new_evidence);
        }

        let commit = ChapterCommit {
            chapter_id: checkpoint
                .processed_chapters
                .last()
                .map(|p| p.chapter_id.clone())
                .unwrap_or_default(),
            ordinal: checkpoint.next_chapter_position.saturating_sub(1) as u32,
            findings_count: new_findings.len(),
            evidence_count: new_evidence.len(),
        };

        let mut commits = self.commits.lock().map_err(|_| {
            ScanError::Checkpoint(crate::CheckpointStoreError::new("lock poisoned"))
        })?;
        commits.push(commit.clone());

        Ok(commit)
    }

    fn load_checkpoint(&self, _task_id: &str) -> Result<Option<ScanCheckpoint>, ScanError> {
        self.checkpoint
            .lock()
            .map(|g| g.clone())
            .map_err(|_| ScanError::Checkpoint(crate::CheckpointStoreError::new("lock poisoned")))
    }

    fn save_checkpoint(&self, checkpoint: &ScanCheckpoint) -> Result<(), ScanError> {
        let mut cp = self.checkpoint.lock().map_err(|_| {
            ScanError::Checkpoint(crate::CheckpointStoreError::new("lock poisoned"))
        })?;
        *cp = Some(checkpoint.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AlertLevel, Chapter, ChapterRef, DocumentFormat, EvidenceAnchor, Finding, FindingStatus,
        NovelDocument, NovelTask, ProviderStamp, RuleCategory, RuleSelection, ScanConfig,
        SourceLocator, TextSpan,
    };

    #[test]
    fn in_memory_persistence_round_trips_checkpoint() {
        let store = InMemoryPersistence::default();
        let task = NovelTask {
            id: "t1".into(),
            document_id: "d1".into(),
            status: crate::TaskStatus::Running,
            created_at_ms: 1,
            updated_at_ms: 1,
            selected_rules: vec![],
            config: ScanConfig::default(),
        };
        let document = NovelDocument::new(
            "d1",
            "Test",
            "test.txt",
            DocumentFormat::PlainText,
            vec![Chapter::new(
                "c1",
                0,
                "Ch1",
                "text",
                SourceLocator::Unknown {
                    description: "x".into(),
                },
            )],
        );

        let checkpoint = ScanCheckpoint {
            schema_version: 2,
            task_id: task.id.clone(),
            document_id: document.id.clone(),
            document_fingerprint: document.computed_fingerprint(),
            scan_profile_fingerprint: "fp1".into(),
            next_chapter_position: 1,
            status: crate::TaskStatus::Paused,
            processed_chapters: vec![ProcessedChapter {
                chapter_id: "c1".into(),
                content_hash: "h1".into(),
            }],
            findings: vec![],
            context: Default::default(),
            usage_totals: Default::default(),
            stop_reason: None,
        };

        let finding = Finding {
            id: "f1".into(),
            rule_id: "r1".into(),
            rule_version: 1,
            category: RuleCategory::Landmine,
            alert_level: AlertLevel::Medium,
            confidence_bps: 5000,
            rationale: "test".into(),
            status: FindingStatus::Suspected,
            source: ChapterRef {
                document_id: "d1".into(),
                chapter_id: "c1".into(),
                chapter_ordinal: 0,
                chapter_title: "Ch1".into(),
                locator: SourceLocator::Unknown {
                    description: "x".into(),
                },
            },
            evidence: vec![],
            verification_note: None,
            provider: ProviderStamp {
                provider_id: "p1".into(),
                model_id: "m1".into(),
            },
        };

        let commit = store
            .commit_chapter(
                &checkpoint,
                &checkpoint.processed_chapters,
                &[finding],
                &[],
                &Default::default(),
            )
            .unwrap();
        assert_eq!(commit.ordinal, 0);
        assert_eq!(commit.findings_count, 1);

        let loaded = store.load_checkpoint("t1").unwrap().unwrap();
        assert_eq!(loaded.next_chapter_position, 1);
    }
}
