use std::{
    collections::HashMap,
    fmt,
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};

use crate::{
    allowed_transition, stable_fingerprint, AlertLevel, CandidateDisposition, Chapter, ChapterRef,
    CompressionError, CompressionRequest, ConfirmationScope, ContextCompressor, ContextSnapshot,
    DetectionMode, EvidenceAnchor, Finding, FindingStatus, InferenceRequest, ModelProvider,
    NovelDocument, NovelTask, ProviderCandidate, ProviderCandidateUpdate, ProviderError,
    ProviderStamp, RuleCategory, RuleContext, RuleDefinition, RuleSelection, StopReason,
    TaskStatus, TextSpan, UsageTotals, CONTEXT_SNAPSHOT_SCHEMA_VERSION,
};

pub const CHECKPOINT_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessedChapter {
    pub chapter_id: String,
    pub content_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanCheckpoint {
    pub schema_version: u32,
    pub task_id: String,
    pub document_id: String,
    pub document_fingerprint: String,
    /// Covers rule categories, versions, user alert-level overrides, and scan
    /// config.
    /// A changed profile starts a new scan instead of mixing incompatible runs.
    pub scan_profile_fingerprint: String,
    /// Position in `NovelDocument.chapters`, independent of display ordinals.
    pub next_chapter_position: usize,
    pub status: TaskStatus,
    pub processed_chapters: Vec<ProcessedChapter>,
    pub findings: Vec<Finding>,
    pub context: ContextSnapshot,
    #[serde(default)]
    pub usage_totals: UsageTotals,
    /// Why the scan was last stopped. `None` for legacy checkpoints.
    #[serde(default)]
    pub stop_reason: Option<StopReason>,
}

impl ScanCheckpoint {
    fn fresh(
        task: &NovelTask,
        document: &NovelDocument,
        rules: &[RuleContext],
        provider_id: &str,
        model_id: &str,
    ) -> Self {
        Self {
            schema_version: CHECKPOINT_SCHEMA_VERSION,
            task_id: task.id.clone(),
            document_id: document.id.clone(),
            document_fingerprint: document.computed_fingerprint(),
            scan_profile_fingerprint: scan_profile_fingerprint(task, rules, provider_id, model_id),
            next_chapter_position: 0,
            status: TaskStatus::Running,
            processed_chapters: Vec::new(),
            findings: Vec::new(),
            context: ContextSnapshot::default(),
            usage_totals: UsageTotals::default(),
            stop_reason: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckpointStoreError {
    pub message: String,
}

impl CheckpointStoreError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for CheckpointStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for CheckpointStoreError {}

/// SQLite, Room, or another app-owned persistence layer implements this small
/// interface. The engine saves after every completed chapter.
pub trait CheckpointStore: Send + Sync {
    fn load(&self, task_id: &str) -> Result<Option<ScanCheckpoint>, CheckpointStoreError>;
    fn save(&self, checkpoint: &ScanCheckpoint) -> Result<(), CheckpointStoreError>;
}

#[derive(Debug, Default)]
pub struct InMemoryCheckpointStore {
    checkpoints: Mutex<HashMap<String, ScanCheckpoint>>,
}

impl CheckpointStore for InMemoryCheckpointStore {
    fn load(&self, task_id: &str) -> Result<Option<ScanCheckpoint>, CheckpointStoreError> {
        let checkpoints = self
            .checkpoints
            .lock()
            .map_err(|_| CheckpointStoreError::new("checkpoint lock poisoned"))?;
        Ok(checkpoints.get(task_id).cloned())
    }

    fn save(&self, checkpoint: &ScanCheckpoint) -> Result<(), CheckpointStoreError> {
        let mut checkpoints = self
            .checkpoints
            .lock()
            .map_err(|_| CheckpointStoreError::new("checkpoint lock poisoned"))?;
        checkpoints.insert(checkpoint.task_id.clone(), checkpoint.clone());
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanBatchResult {
    pub checkpoint: ScanCheckpoint,
    pub chapters_scanned: usize,
    pub new_findings: usize,
    pub complete: bool,
    pub stop_reason: StopReason,
}

#[derive(Debug)]
pub enum ScanError {
    InvalidInput(String),
    ResumeMismatch(String),
    Provider(ProviderError),
    Compression(CompressionError),
    Checkpoint(CheckpointStoreError),
}

impl fmt::Display for ScanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(message) => write!(formatter, "invalid scan input: {message}"),
            Self::ResumeMismatch(message) => write!(formatter, "cannot resume scan: {message}"),
            Self::Provider(error) => write!(formatter, "provider failed: {error}"),
            Self::Compression(error) => write!(formatter, "context compression failed: {error}"),
            Self::Checkpoint(error) => write!(formatter, "checkpoint failed: {error}"),
        }
    }
}

impl std::error::Error for ScanError {}

impl From<ProviderError> for ScanError {
    fn from(value: ProviderError) -> Self {
        Self::Provider(value)
    }
}

impl From<CompressionError> for ScanError {
    fn from(value: CompressionError) -> Self {
        Self::Compression(value)
    }
}

impl From<CheckpointStoreError> for ScanError {
    fn from(value: CheckpointStoreError) -> Self {
        Self::Checkpoint(value)
    }
}

pub struct ScanEngine {
    provider: Arc<dyn ModelProvider>,
    compressor: Arc<dyn ContextCompressor>,
}

impl ScanEngine {
    pub fn new(provider: Arc<dyn ModelProvider>, compressor: Arc<dyn ContextCompressor>) -> Self {
        Self {
            provider,
            compressor,
        }
    }

    /// Scans up to `max_chapters` and persists a checkpoint after each chapter.
    /// Passing zero is a valid no-op, useful when an app is being paused.
    pub async fn scan_batch(
        &self,
        task: &NovelTask,
        document: &NovelDocument,
        rule_catalog: &[RuleDefinition],
        explicit_checkpoint: Option<ScanCheckpoint>,
        checkpoint_store: Option<&dyn CheckpointStore>,
        max_chapters: usize,
    ) -> Result<ScanBatchResult, ScanError> {
        validate_task_document(task, document)?;
        let rules = resolve_rules(&task.selected_rules, rule_catalog)?;
        let stored_checkpoint = match (explicit_checkpoint, checkpoint_store) {
            (Some(checkpoint), _) => Some(checkpoint),
            (None, Some(store)) => store.load(&task.id)?,
            (None, None) => None,
        };
        let provider_id = self.provider.provider_id();
        let model_id = self.provider.model_id();
        let mut checkpoint = stored_checkpoint.unwrap_or_else(|| {
            ScanCheckpoint::fresh(task, document, &rules, provider_id, model_id)
        });
        validate_checkpoint(&checkpoint, task, document, &rules, provider_id, model_id)?;

        checkpoint.status = TaskStatus::Running;
        let starting_findings = checkpoint.findings.len();
        let mut chapters_scanned = 0;

        while checkpoint.next_chapter_position < document.chapters.len()
            && chapters_scanned < max_chapters
        {
            let chapter = &document.chapters[checkpoint.next_chapter_position];
            validate_chapter_hash(chapter)?;
            let request = crate::InferenceRequest {
                task_id: task.id.clone(),
                document_id: document.id.clone(),
                chapter: chapter.clone(),
                rules: rules.clone(),
                context: checkpoint.context.clone(),
            };
            let response = self.provider.analyze(&request).await?;
            checkpoint.usage_totals = checkpoint.usage_totals.add(response.usage);
            let chapter_findings =
                self.materialize_findings(task, document, chapter, &rules, response.candidates)?;

            let mut next_context = self
                .compressor
                .compress(CompressionRequest {
                    previous: &checkpoint.context,
                    chapter,
                    chapter_findings: &chapter_findings,
                    budget_chars: task.config.context_budget_chars,
                    memory_delta: &crate::compression::EMPTY_MEMORY_DELTA,
                })
                .await?;
            // These fields are checkpoint invariants, even when a provider-backed
            // compressor forgets to maintain them.
            next_context.revision = checkpoint.context.revision.saturating_add(1);
            next_context.processed_chapter_ids = checkpoint
                .processed_chapters
                .iter()
                .map(|processed| processed.chapter_id.clone())
                .chain(std::iter::once(chapter.id.clone()))
                .collect();

            checkpoint.context = next_context;
            checkpoint.findings.extend(chapter_findings);
            checkpoint.processed_chapters.push(ProcessedChapter {
                chapter_id: chapter.id.clone(),
                content_hash: chapter.computed_content_hash(),
            });
            checkpoint.next_chapter_position += 1;
            chapters_scanned += 1;
            checkpoint.status = if checkpoint.next_chapter_position == document.chapters.len() {
                TaskStatus::Completed
            } else {
                TaskStatus::Paused
            };

            if let Some(store) = checkpoint_store {
                store.save(&checkpoint)?;
            }
        }

        let complete = checkpoint.next_chapter_position == document.chapters.len();
        checkpoint.status = if complete {
            TaskStatus::Completed
        } else {
            TaskStatus::Paused
        };
        if let Some(store) = checkpoint_store {
            // Also persists status for zero-sized batches and already-complete resumes.
            store.save(&checkpoint)?;
        }

        let stop_reason = if complete {
            StopReason::Completed
        } else {
            StopReason::UserPaused
        };
        checkpoint.stop_reason = Some(stop_reason);
        Ok(ScanBatchResult {
            new_findings: checkpoint.findings.len() - starting_findings,
            checkpoint,
            chapters_scanned,
            complete,
            stop_reason,
        })
    }

    fn materialize_findings(
        &self,
        task: &NovelTask,
        document: &NovelDocument,
        chapter: &Chapter,
        rules: &[RuleContext],
        candidates: Vec<ProviderCandidate>,
    ) -> Result<Vec<Finding>, ScanError> {
        let classification_by_rule = rules
            .iter()
            .map(|rule| {
                (
                    rule.id.as_str(),
                    (
                        rule.version,
                        rule.category,
                        rule.alert_level,
                        rule.confirmation_scope,
                        rule.requires_user_boundary,
                    ),
                )
            })
            .collect::<HashMap<_, _>>();
        let source = ChapterRef::from_chapter(&document.id, chapter);

        let raw = candidates
            .into_iter()
            .filter_map(|candidate| {
                let (
                    rule_version,
                    category,
                    alert_level,
                    confirmation_scope,
                    requires_user_boundary,
                ) = *classification_by_rule.get(candidate.rule_id.as_str())?;
                let mut evidence = Vec::new();
                let mut invalid_ranges = 0usize;
                for range in &candidate.evidence_ranges {
                    let start = range.utf8_byte_start;
                    let end = range.utf8_byte_end;
                    match chapter.text.get(start..end) {
                        Some(quote) if !quote.is_empty() => evidence.push(EvidenceAnchor {
                            source: source.clone(),
                            span: TextSpan::from_valid_range(&chapter.text, start, end),
                            exact_quote: quote.to_owned(),
                            quote_hash: stable_fingerprint(quote.as_bytes()),
                            chapter_content_hash: chapter.computed_content_hash(),
                        }),
                        _ => invalid_ranges += 1,
                    }
                }

                let confirmed = !candidate.evidence_ranges.is_empty()
                    && invalid_ranges == 0
                    && evidence.len() == candidate.evidence_ranges.len();
                if !confirmed && !task.config.retain_unverified_candidates {
                    return None;
                }
                let mut status = if confirmed && candidate.requires_later_confirmation {
                    FindingStatus::PendingConfirmation
                } else if confirmed {
                    FindingStatus::Confirmed
                } else {
                    FindingStatus::Suspected
                };

                // Scope gate: rules scoped to cross-chapter or whole-book
                // context cannot be confirmed during single-pass S1 scanning.
                // Without cross-chapter re-verification they are capped at
                // pending_confirmation even when the evidence is valid.
                // Rules that require user boundaries are also capped in S1
                // because the boundary UI is not yet implemented.
                if status == FindingStatus::Confirmed {
                    match confirmation_scope {
                        ConfirmationScope::CrossChapter | ConfirmationScope::WholeBook => {
                            status = FindingStatus::PendingConfirmation;
                        }
                        ConfirmationScope::Local | ConfirmationScope::Chapter => {}
                    }
                    if requires_user_boundary {
                        status = FindingStatus::PendingConfirmation;
                    }
                }
                let verification_note = match status {
                    FindingStatus::PendingConfirmation => Some(
                        "exact clue found; later chapters or relationship facts are required"
                            .to_owned(),
                    ),
                    FindingStatus::Suspected => Some(format!(
                        "{} evidence range(s) missing or invalid; requires source verification",
                        invalid_ranges
                            + if candidate.evidence_ranges.is_empty() {
                                1
                            } else {
                                0
                            }
                    )),
                    FindingStatus::Confirmed | FindingStatus::Rejected => None,
                };
                let id_material = format!(
                    "{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{:?}",
                    task.id,
                    chapter.id,
                    candidate.rule_id,
                    rule_version,
                    category_tag(category),
                    alert_level_tag(alert_level),
                    finding_status_tag(status),
                    candidate.rationale,
                    candidate.evidence_ranges
                );

                Some(Finding {
                    id: format!("finding_{}", stable_fingerprint(id_material.as_bytes())),
                    rule_id: candidate.rule_id,
                    rule_version,
                    category,
                    alert_level,
                    confidence_bps: candidate.confidence_bps.min(10_000),
                    rationale: candidate.rationale,
                    status,
                    source: source.clone(),
                    evidence,
                    verification_note,
                    provider: ProviderStamp {
                        provider_id: self.provider.provider_id().to_owned(),
                        model_id: self.provider.model_id().to_owned(),
                    },
                })
            })
            .collect::<Vec<_>>();

        // Deduplicate by stable finding ID. Same ID + identical content → keep
        // first. Same ID + different content → error.
        let mut deduped: std::collections::BTreeMap<String, Finding> =
            std::collections::BTreeMap::new();
        for finding in raw {
            if let Some(existing) = deduped.get(&finding.id) {
                if existing != &finding {
                    return Err(ScanError::InvalidInput(format!(
                        "conflicting candidates produce same finding id '{}'",
                        finding.id
                    )));
                }
            } else {
                deduped.insert(finding.id.clone(), finding);
            }
        }
        Ok(deduped.into_values().collect())
    }

    /// Splits a chapter into bounded windows, scans each, and merges the results.
    /// When the chapter fits within `window_chars`, it is scanned as a single
    /// window; otherwise it is split on safe UTF-8 boundaries.
    async fn scan_chapter_windows(
        &self,
        task: &NovelTask,
        document: &NovelDocument,
        chapter: &Chapter,
        rules: &[RuleContext],
        context: &ContextSnapshot,
        window_chars: usize,
    ) -> Result<Vec<Finding>, ScanError> {
        let windows = crate::chunk_text(&chapter.text, window_chars);
        let mut all_findings = Vec::new();

        for window_text in windows {
            // Create a temporary chapter-like window with a sub-range context
            let start_byte = window_text.as_ptr() as usize - chapter.text.as_ptr() as usize;
            let end_byte = start_byte + window_text.len();

            let window_chapter = Chapter::new(
                format!("{}/{}", chapter.id, start_byte),
                chapter.ordinal,
                format!("{} [{}-{}]", chapter.title, start_byte, end_byte),
                window_text.to_owned(),
                chapter.locator.clone(),
            );

            let request = InferenceRequest {
                task_id: task.id.clone(),
                document_id: document.id.clone(),
                chapter: window_chapter,
                rules: rules.to_vec(),
                context: context.clone(),
            };

            let response = self.provider.analyze(&request).await?;
            let window_findings =
                self.materialize_findings(task, document, chapter, rules, response.candidates)?;
            all_findings.extend(window_findings);
        }

        // Deduplicate across windows by finding ID
        let mut deduped: std::collections::BTreeMap<String, Finding> =
            std::collections::BTreeMap::new();
        for finding in all_findings {
            if let Some(existing) = deduped.get(&finding.id) {
                if existing != &finding {
                    return Err(ScanError::InvalidInput(format!(
                        "conflicting window results for finding id '{}'",
                        finding.id
                    )));
                }
            } else {
                deduped.insert(finding.id.clone(), finding);
            }
        }
        Ok(deduped.into_values().collect())
    }

    /// Apply provider candidate updates to checkpoint findings. Only Suspected and
    /// PendingConfirmation findings can be updated. Confirmed and Rejected findings
    /// are terminal.
    fn apply_candidate_updates(
        findings: &mut Vec<Finding>,
        updates: &[ProviderCandidateUpdate],
        _chapter: &Chapter,
    ) -> Result<(), ScanError> {
        for update in updates {
            let finding = findings
                .iter_mut()
                .find(|f| f.id == update.finding_id)
                .ok_or_else(|| {
                    ScanError::InvalidInput(format!(
                        "candidate update references unknown finding '{}'",
                        update.finding_id
                    ))
                })?;

            let new_status = match update.disposition {
                CandidateDisposition::KeepPending => FindingStatus::PendingConfirmation,
                CandidateDisposition::Confirm => FindingStatus::Confirmed,
                CandidateDisposition::Reject => FindingStatus::Rejected,
            };

            if !allowed_transition(finding.status, new_status) {
                return Err(ScanError::InvalidInput(format!(
                    "candidate update for '{}' cannot transition from {:?} to {:?}",
                    update.finding_id, finding.status, new_status
                )));
            }

            finding.status = new_status;
            if let Some(rationale) = &update.rationale {
                finding.rationale = rationale.clone();
            }
            if new_status == FindingStatus::Rejected {
                finding.verification_note = Some("rejected by provider update".into());
            }
        }
        Ok(())
    }
}

fn validate_task_document(task: &NovelTask, document: &NovelDocument) -> Result<(), ScanError> {
    if task.document_id != document.id {
        return Err(ScanError::InvalidInput(format!(
            "task document '{}' does not match '{}'",
            task.document_id, document.id
        )));
    }
    if task.config.context_budget_chars == 0 {
        return Err(ScanError::InvalidInput(
            "context_budget_chars must be greater than zero".into(),
        ));
    }
    if document.fingerprint != document.computed_fingerprint() {
        return Err(ScanError::InvalidInput(
            "document fingerprint is stale; re-import before scanning".into(),
        ));
    }
    let mut chapter_ids = std::collections::HashSet::new();
    for chapter in &document.chapters {
        validate_chapter_hash(chapter)?;
        if !chapter_ids.insert(chapter.id.as_str()) {
            return Err(ScanError::InvalidInput(format!(
                "duplicate chapter id '{}'",
                chapter.id
            )));
        }
    }
    Ok(())
}

fn validate_chapter_hash(chapter: &Chapter) -> Result<(), ScanError> {
    if chapter.content_hash != chapter.computed_content_hash() {
        return Err(ScanError::InvalidInput(format!(
            "chapter '{}' content hash is stale",
            chapter.id
        )));
    }
    Ok(())
}

fn validate_checkpoint(
    checkpoint: &ScanCheckpoint,
    task: &NovelTask,
    document: &NovelDocument,
    rules: &[RuleContext],
    provider_id: &str,
    model_id: &str,
) -> Result<(), ScanError> {
    if checkpoint.schema_version != CHECKPOINT_SCHEMA_VERSION {
        return Err(ScanError::ResumeMismatch(format!(
            "unsupported checkpoint schema {}",
            checkpoint.schema_version
        )));
    }
    if checkpoint.task_id != task.id || checkpoint.document_id != document.id {
        return Err(ScanError::ResumeMismatch(
            "checkpoint belongs to another task or document".into(),
        ));
    }
    if checkpoint.document_fingerprint != document.computed_fingerprint() {
        return Err(ScanError::ResumeMismatch(
            "source document changed after checkpoint".into(),
        ));
    }
    if checkpoint.scan_profile_fingerprint
        != scan_profile_fingerprint(task, rules, provider_id, model_id)
    {
        return Err(ScanError::ResumeMismatch(
            "provider/model identity, selected rules, categories, alert levels, rule versions, or scan config changed"
                .into(),
        ));
    }
    if checkpoint.next_chapter_position > document.chapters.len()
        || checkpoint.processed_chapters.len() != checkpoint.next_chapter_position
    {
        return Err(ScanError::ResumeMismatch(
            "checkpoint chapter position is inconsistent".into(),
        ));
    }
    for (position, processed) in checkpoint.processed_chapters.iter().enumerate() {
        let Some(chapter) = document.chapters.get(position) else {
            return Err(ScanError::ResumeMismatch(
                "processed chapter is absent from source".into(),
            ));
        };
        if processed.chapter_id != chapter.id
            || processed.content_hash != chapter.computed_content_hash()
        {
            return Err(ScanError::ResumeMismatch(format!(
                "processed chapter at position {position} changed"
            )));
        }
    }
    let expected_context_ids = checkpoint
        .processed_chapters
        .iter()
        .map(|processed| processed.chapter_id.as_str())
        .collect::<Vec<_>>();
    let actual_context_ids = checkpoint
        .context
        .processed_chapter_ids
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if expected_context_ids != actual_context_ids {
        return Err(ScanError::ResumeMismatch(
            "context snapshot does not match processed chapters".into(),
        ));
    }
    validate_persisted_findings(
        &checkpoint.findings,
        document,
        &checkpoint.processed_chapters,
        rules,
        provider_id,
        model_id,
    )?;
    if checkpoint.context.schema_version != CONTEXT_SNAPSHOT_SCHEMA_VERSION {
        return Err(ScanError::ResumeMismatch(format!(
            "context snapshot schema version {} != {}",
            checkpoint.context.schema_version, CONTEXT_SNAPSHOT_SCHEMA_VERSION
        )));
    }
    validate_unresolved_candidates(
        &checkpoint.context.unresolved_candidates,
        &checkpoint.findings,
    )?;
    Ok(())
}

fn validate_unresolved_candidates(
    unresolved_ids: &[String],
    findings: &[Finding],
) -> Result<(), ScanError> {
    let findings_by_id: HashMap<&str, &Finding> =
        findings.iter().map(|f| (f.id.as_str(), f)).collect();

    let mut seen = std::collections::HashSet::new();
    for id in unresolved_ids {
        if !seen.insert(id.as_str()) {
            return Err(ScanError::ResumeMismatch(format!(
                "duplicate unresolved candidate '{}'",
                id
            )));
        }
        let finding = findings_by_id.get(id.as_str()).ok_or_else(|| {
            ScanError::ResumeMismatch(format!("unknown unresolved candidate '{}'", id))
        })?;
        if !matches!(
            finding.status,
            FindingStatus::Suspected | FindingStatus::PendingConfirmation
        ) {
            return Err(ScanError::ResumeMismatch(format!(
                "unresolved candidate '{}' has status {:?}, expected suspected or pending_confirmation",
                id, finding.status
            )));
        }
    }

    // Every Suspected/PendingConfirmation finding must appear in unresolved_ids.
    for finding in findings {
        if matches!(
            finding.status,
            FindingStatus::Suspected | FindingStatus::PendingConfirmation
        ) && !seen.contains(finding.id.as_str())
        {
            return Err(ScanError::ResumeMismatch(format!(
                "finding '{}' is {:?} but missing from unresolved candidates",
                finding.id, finding.status
            )));
        }
    }

    Ok(())
}

fn validate_persisted_findings(
    findings: &[Finding],
    document: &NovelDocument,
    processed_chapters: &[ProcessedChapter],
    rules: &[RuleContext],
    provider_id: &str,
    model_id: &str,
) -> Result<(), ScanError> {
    let rule_by_id: HashMap<&str, &RuleContext> =
        rules.iter().map(|r| (r.id.as_str(), r)).collect();

    let processed_chapter_ids: std::collections::HashSet<&str> = processed_chapters
        .iter()
        .map(|chapter| chapter.chapter_id.as_str())
        .collect();

    let mut finding_ids = std::collections::HashSet::new();

    for finding in findings {
        if !finding_ids.insert(finding.id.as_str()) {
            return Err(ScanError::ResumeMismatch(format!(
                "duplicate finding id '{}'",
                finding.id
            )));
        }

        if finding.confidence_bps > 10_000 {
            return Err(ScanError::ResumeMismatch(format!(
                "finding '{}' confidence_bps {} exceeds limit 10000",
                finding.id, finding.confidence_bps
            )));
        }

        let resolved = rule_by_id.get(finding.rule_id.as_str()).ok_or_else(|| {
            ScanError::ResumeMismatch(format!(
                "finding '{}' references unknown or unselected rule '{}'",
                finding.id, finding.rule_id
            ))
        })?;

        if finding.rule_version != resolved.version {
            return Err(ScanError::ResumeMismatch(format!(
                "finding '{}' rule_version {} does not match selected version {}",
                finding.id, finding.rule_version, resolved.version
            )));
        }
        if finding.category != resolved.category {
            return Err(ScanError::ResumeMismatch(format!(
                "finding '{}' category {:?} does not match selected {:?}",
                finding.id, finding.category, resolved.category
            )));
        }
        if finding.alert_level != resolved.alert_level {
            return Err(ScanError::ResumeMismatch(format!(
                "finding '{}' alert_level {:?} does not match selected {:?}",
                finding.id, finding.alert_level, resolved.alert_level
            )));
        }
        if finding.provider.provider_id != provider_id {
            return Err(ScanError::ResumeMismatch(format!(
                "finding '{}' provider_id '{}' does not match current '{}'",
                finding.id, finding.provider.provider_id, provider_id
            )));
        }
        if finding.provider.model_id != model_id {
            return Err(ScanError::ResumeMismatch(format!(
                "finding '{}' model_id '{}' does not match current '{}'",
                finding.id, finding.provider.model_id, model_id
            )));
        }
        // Re-apply the scope/boundary gates that were enforced during scan.
        if finding.status == FindingStatus::Confirmed {
            if matches!(
                resolved.confirmation_scope,
                ConfirmationScope::CrossChapter | ConfirmationScope::WholeBook
            ) {
                return Err(ScanError::ResumeMismatch(format!(
                    "finding '{}' is Confirmed but confirmation_scope is {:?}",
                    finding.id, resolved.confirmation_scope
                )));
            }
            if resolved.requires_user_boundary {
                return Err(ScanError::ResumeMismatch(format!(
                    "finding '{}' is Confirmed but rule requires user boundary",
                    finding.id
                )));
            }
        }
        if !processed_chapter_ids.contains(finding.source.chapter_id.as_str()) {
            return Err(ScanError::ResumeMismatch(format!(
                "finding '{}' references unprocessed chapter '{}'",
                finding.id, finding.source.chapter_id
            )));
        }
        let chapter = document
            .chapters
            .iter()
            .find(|chapter| chapter.id == finding.source.chapter_id)
            .ok_or_else(|| {
                ScanError::ResumeMismatch(format!(
                    "finding '{}' references a missing chapter",
                    finding.id
                ))
            })?;
        let expected_source = ChapterRef::from_chapter(&document.id, chapter);
        if finding.source != expected_source {
            return Err(ScanError::ResumeMismatch(format!(
                "finding '{}' chapter source changed",
                finding.id
            )));
        }
        if matches!(
            finding.status,
            FindingStatus::PendingConfirmation | FindingStatus::Confirmed
        ) && finding.evidence.is_empty()
        {
            return Err(ScanError::ResumeMismatch(format!(
                "finding '{}' status {:?} requires evidence but has none",
                finding.id, finding.status
            )));
        }
        for anchor in &finding.evidence {
            if anchor.source != expected_source
                || anchor.chapter_content_hash != chapter.computed_content_hash()
            {
                return Err(ScanError::ResumeMismatch(format!(
                    "finding '{}' evidence source changed",
                    finding.id
                )));
            }
            let Some(quote) = chapter
                .text
                .get(anchor.span.utf8_byte_start..anchor.span.utf8_byte_end)
            else {
                return Err(ScanError::ResumeMismatch(format!(
                    "finding '{}' evidence range is out of bounds",
                    finding.id
                )));
            };
            let expected_span = TextSpan::from_valid_range(
                &chapter.text,
                anchor.span.utf8_byte_start,
                anchor.span.utf8_byte_end,
            );
            if quote.is_empty()
                || quote != anchor.exact_quote
                || anchor.quote_hash != stable_fingerprint(quote.as_bytes())
                || anchor.span != expected_span
            {
                return Err(ScanError::ResumeMismatch(format!(
                    "finding '{}' evidence no longer matches original text",
                    finding.id
                )));
            }
        }
    }
    Ok(())
}

fn scan_profile_fingerprint(
    task: &NovelTask,
    rules: &[RuleContext],
    provider_id: &str,
    model_id: &str,
) -> String {
    let mut selected = task
        .selected_rules
        .iter()
        .map(|selection| {
            format!(
                "{}:{}:{}:{}",
                selection.rule_id,
                category_tag(selection.category),
                alert_level_tag(selection.alert_level),
                selection.enabled
            )
        })
        .collect::<Vec<_>>();
    selected.sort_unstable();
    let mut resolved = rules
        .iter()
        .map(|rule| {
            format!(
                "{}:{}:{}:{}:{}:{}:{}:{}:mode={}:profile={:?}:criteria_len={}\0{}\0excl_len={}\0{}\0pend_len={}\0{}",
                rule.id,
                rule.version,
                rule.name,
                rule.description,
                category_tag(rule.category),
                alert_level_tag(rule.alert_level),
                scope_tag(rule.confirmation_scope),
                rule.requires_user_boundary,
                mode_tag(rule.detection_mode),
                rule.detection_profile_ref,
                rule.criteria.len(),
                rule.criteria.join("\0"),
                rule.exclusions.len(),
                rule.exclusions.join("\0"),
                rule.pending_conditions.len(),
                rule.pending_conditions.join("\0"),
            )
        })
        .collect::<Vec<_>>();
    resolved.sort_unstable();
    let profile = format!(
        "selected={selected:?}\0resolved={resolved:?}\0budget={}\0retain={}\0rule_pack={:?}\0rule_pack_ver={:?}\0provider_len={}\0provider={}\0model_len={}\0model={}",
        task.config.context_budget_chars,
        task.config.retain_unverified_candidates,
        task.config.rule_pack_id_snapshot,
        task.config.rule_pack_version_snapshot,
        provider_id.len(),
        provider_id,
        model_id.len(),
        model_id,
    );
    stable_fingerprint(profile.as_bytes())
}

const fn category_tag(category: RuleCategory) -> &'static str {
    match category {
        RuleCategory::Landmine => "landmine",
        RuleCategory::Frustration => "frustration",
    }
}

const fn alert_level_tag(alert_level: AlertLevel) -> &'static str {
    match alert_level {
        AlertLevel::Critical => "critical",
        AlertLevel::High => "high",
        AlertLevel::Medium => "medium",
        AlertLevel::Low => "low",
        AlertLevel::Info => "info",
    }
}

const fn mode_tag(mode: DetectionMode) -> &'static str {
    match mode {
        DetectionMode::Semantic => "semantic",
        DetectionMode::ManualOnly => "manual_only",
    }
}

const fn scope_tag(scope: ConfirmationScope) -> &'static str {
    match scope {
        ConfirmationScope::Local => "local",
        ConfirmationScope::Chapter => "chapter",
        ConfirmationScope::CrossChapter => "cross_chapter",
        ConfirmationScope::WholeBook => "whole_book",
    }
}

const fn finding_status_tag(status: FindingStatus) -> &'static str {
    match status {
        FindingStatus::Suspected => "suspected",
        FindingStatus::PendingConfirmation => "pending_confirmation",
        FindingStatus::Confirmed => "confirmed",
        FindingStatus::Rejected => "rejected",
    }
}

fn resolve_rules(
    selections: &[RuleSelection],
    catalog: &[RuleDefinition],
) -> Result<Vec<RuleContext>, ScanError> {
    let mut catalog_ids = std::collections::HashSet::new();
    for rule in catalog {
        if !catalog_ids.insert(rule.id.as_str()) {
            return Err(ScanError::InvalidInput(format!(
                "duplicate rule id '{}' in catalog",
                rule.id
            )));
        }
    }
    let mut selection_ids = std::collections::HashSet::new();
    for selection in selections {
        if !selection_ids.insert(selection.rule_id.as_str()) {
            return Err(ScanError::InvalidInput(format!(
                "rule '{}' is selected more than once",
                selection.rule_id
            )));
        }
    }
    let rules_by_id = catalog
        .iter()
        .map(|rule| (rule.id.as_str(), rule))
        .collect::<HashMap<_, _>>();
    selections
        .iter()
        .filter(|selection| selection.enabled)
        .map(|selection| {
            let rule = rules_by_id.get(selection.rule_id.as_str()).ok_or_else(|| {
                ScanError::InvalidInput(format!(
                    "selected rule '{}' is absent from catalog",
                    selection.rule_id
                ))
            })?;
            if selection.category != rule.category {
                return Err(ScanError::InvalidInput(format!(
                    "selected rule '{}' category does not match catalog",
                    selection.rule_id
                )));
            }
            Ok(RuleContext::from_definition(rule, selection.alert_level))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::{
        future::Future,
        sync::Arc,
        task::{Context, Poll, Wake, Waker},
    };

    use super::*;
    use crate::{
        AlertLevel, DeterministicContextCompressor, DeterministicTestProvider, DocumentFormat,
        PatternRule, ProviderEvidenceRange, ProviderFuture, ProviderResponse, RuleCategory,
        ScanConfig, SourceLocator,
    };

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

    fn rule() -> RuleDefinition {
        RuleDefinition {
            id: "takeover".into(),
            version: 1,
            name: "接盘".into(),
            description: "测试规则".into(),
            category: RuleCategory::Frustration,
            default_alert_level: AlertLevel::Low,
            confirmation_scope: ConfirmationScope::Local,
            requires_user_boundary: false,
            tags: vec!["relationship".into()],
            detection_profile_ref: None,
            detection_mode: DetectionMode::Semantic,
            criteria: vec![],
            exclusions: vec![],
            pending_conditions: vec![],
        }
    }

    fn task(document_id: &str) -> NovelTask {
        NovelTask {
            id: "task-1".into(),
            document_id: document_id.into(),
            status: TaskStatus::Pending,
            created_at_ms: 1,
            updated_at_ms: 1,
            selected_rules: vec![RuleSelection {
                rule_id: "takeover".into(),
                category: RuleCategory::Frustration,
                alert_level: AlertLevel::Critical,
                enabled: true,
            }],
            config: ScanConfig {
                context_budget_chars: 200,
                retain_unverified_candidates: true,
                rule_pack_id_snapshot: None,
                rule_pack_version_snapshot: None,
            },
        }
    }

    fn chapter(id: &str, ordinal: u32, text: &str) -> Chapter {
        Chapter::new(
            id,
            ordinal,
            format!("第{}章", ordinal + 1),
            text,
            SourceLocator::PlainText {
                line_start: ordinal + 1,
                line_end: ordinal + 1,
            },
        )
    }

    fn engine() -> ScanEngine {
        ScanEngine::new(
            Arc::new(DeterministicTestProvider::new(vec![PatternRule {
                rule_id: "takeover".into(),
                phrases: vec!["接盘".into()],
                rationale: "文本出现明确模式".into(),
                confidence_bps: 9_500,
            }])),
            Arc::new(DeterministicContextCompressor::default()),
        )
    }

    #[test]
    fn vertical_slice_confirms_only_exact_source_evidence() {
        let document = NovelDocument::new(
            "book-1",
            "测试书",
            "book.txt",
            DocumentFormat::PlainText,
            vec![chapter("c1", 0, "开头\n主角拒绝接盘。")],
        );
        let result = ready(engine().scan_batch(
            &task(&document.id),
            &document,
            &[rule()],
            None,
            None,
            usize::MAX,
        ))
        .unwrap();

        assert!(result.complete);
        assert_eq!(result.new_findings, 1);
        let finding = &result.checkpoint.findings[0];
        assert_eq!(finding.rule_version, 1);
        assert_eq!(finding.category, RuleCategory::Frustration);
        assert_eq!(finding.alert_level, AlertLevel::Critical);
        assert_eq!(finding.status, FindingStatus::Confirmed);
        assert_eq!(finding.source.chapter_title, "第1章");
        assert_eq!(finding.evidence[0].exact_quote, "接盘");
        assert_eq!(finding.evidence[0].span.line_start, 2);
        assert_eq!(
            &document.chapters[0].text
                [finding.evidence[0].span.utf8_byte_start..finding.evidence[0].span.utf8_byte_end],
            finding.evidence[0].exact_quote
        );
    }

    #[test]
    fn persisted_checkpoint_resumes_without_duplicate_findings() {
        let document = NovelDocument::new(
            "book-1",
            "测试书",
            "book.txt",
            DocumentFormat::PlainText,
            vec![
                chapter("c1", 0, "第一次接盘"),
                chapter("c2", 1, "第二次接盘"),
            ],
        );
        let task = task(&document.id);
        let store = InMemoryCheckpointStore::default();
        let first =
            ready(engine().scan_batch(&task, &document, &[rule()], None, Some(&store), 1)).unwrap();
        assert!(!first.complete);
        assert_eq!(first.checkpoint.next_chapter_position, 1);
        assert_eq!(first.checkpoint.findings.len(), 1);

        // No explicit checkpoint: the engine restores it through the store.
        let resumed =
            ready(engine().scan_batch(&task, &document, &[rule()], None, Some(&store), 10))
                .unwrap();
        assert!(resumed.complete);
        assert_eq!(resumed.chapters_scanned, 1);
        assert_eq!(resumed.checkpoint.findings.len(), 2);
        assert_eq!(resumed.checkpoint.context.processed_chapter_ids.len(), 2);
        assert_ne!(
            resumed.checkpoint.findings[0].id,
            resumed.checkpoint.findings[1].id
        );
    }

    struct InvalidEvidenceProvider;

    impl ModelProvider for InvalidEvidenceProvider {
        fn provider_id(&self) -> &str {
            "invalid-test"
        }

        fn model_id(&self) -> &str {
            "invalid-v1"
        }

        fn analyze<'a>(&'a self, _request: &'a crate::InferenceRequest) -> ProviderFuture<'a> {
            Box::pin(async {
                Ok(ProviderResponse {
                    candidates: vec![ProviderCandidate {
                        rule_id: "takeover".into(),
                        confidence_bps: 10_000,
                        rationale: "fabricated range".into(),
                        requires_later_confirmation: false,
                        evidence_ranges: vec![ProviderEvidenceRange {
                            utf8_byte_start: 1,
                            utf8_byte_end: 10_000,
                        }],
                    }],
                    ..Default::default()
                })
            })
        }
    }

    #[test]
    fn invalid_model_offsets_remain_suspected_with_chapter_source() {
        let document = NovelDocument::new(
            "book-1",
            "测试书",
            "book.txt",
            DocumentFormat::PlainText,
            vec![chapter("c1", 0, "没有对应原文")],
        );
        let engine = ScanEngine::new(
            Arc::new(InvalidEvidenceProvider),
            Arc::new(DeterministicContextCompressor::default()),
        );
        let result =
            ready(engine.scan_batch(&task(&document.id), &document, &[rule()], None, None, 1))
                .unwrap();
        let finding = &result.checkpoint.findings[0];

        assert_eq!(finding.status, FindingStatus::Suspected);
        assert!(finding.evidence.is_empty());
        assert_eq!(finding.source.chapter_id, "c1");
        assert!(finding.verification_note.is_some());
    }

    struct PendingEvidenceProvider;

    impl ModelProvider for PendingEvidenceProvider {
        fn provider_id(&self) -> &str {
            "pending-test"
        }

        fn model_id(&self) -> &str {
            "pending-v1"
        }

        fn analyze<'a>(&'a self, request: &'a crate::InferenceRequest) -> ProviderFuture<'a> {
            Box::pin(async move {
                let start = request.chapter.text.find("身份").unwrap();
                Ok(ProviderResponse {
                    candidates: vec![ProviderCandidate {
                        rule_id: "takeover".into(),
                        confidence_bps: 8_000,
                        rationale: "身份需要后文揭晓".into(),
                        requires_later_confirmation: true,
                        evidence_ranges: vec![ProviderEvidenceRange {
                            utf8_byte_start: start,
                            utf8_byte_end: start + "身份".len(),
                        }],
                    }],
                    ..Default::default()
                })
            })
        }
    }

    #[test]
    fn exact_clue_that_needs_later_context_stays_pending_confirmation() {
        let document = NovelDocument::new(
            "book-1",
            "测试书",
            "book.txt",
            DocumentFormat::PlainText,
            vec![chapter("c1", 0, "身份尚未揭晓")],
        );
        let engine = ScanEngine::new(
            Arc::new(PendingEvidenceProvider),
            Arc::new(DeterministicContextCompressor::default()),
        );
        let result =
            ready(engine.scan_batch(&task(&document.id), &document, &[rule()], None, None, 1))
                .unwrap();
        let finding = &result.checkpoint.findings[0];

        assert_eq!(finding.status, FindingStatus::PendingConfirmation);
        assert_eq!(finding.evidence[0].exact_quote, "身份");
        assert!(result
            .checkpoint
            .context
            .unresolved_candidates
            .contains(&finding.id));
    }

    #[test]
    fn rule_category_snapshot_must_match_the_catalog() {
        let document = NovelDocument::new(
            "book-1",
            "测试书",
            "book.txt",
            DocumentFormat::PlainText,
            vec![chapter("c1", 0, "普通正文")],
        );
        let mut mismatched_task = task(&document.id);
        mismatched_task.selected_rules[0].category = RuleCategory::Landmine;

        let error =
            ready(engine().scan_batch(&mismatched_task, &document, &[rule()], None, None, 1))
                .unwrap_err();

        assert!(matches!(error, ScanError::InvalidInput(_)));
    }

    #[test]
    fn resume_rejects_a_changed_alert_level() {
        let document = NovelDocument::new(
            "book-1",
            "测试书",
            "book.txt",
            DocumentFormat::PlainText,
            vec![chapter("c1", 0, "接盘")],
        );
        let original_task = task(&document.id);
        let first = ready(engine().scan_batch(&original_task, &document, &[rule()], None, None, 0))
            .unwrap();
        let mut changed_task = original_task;
        changed_task.selected_rules[0].alert_level = AlertLevel::Info;

        let error = ready(engine().scan_batch(
            &changed_task,
            &document,
            &[rule()],
            Some(first.checkpoint),
            None,
            1,
        ))
        .unwrap_err();

        assert!(matches!(error, ScanError::ResumeMismatch(_)));
    }

    #[test]
    fn resume_rejects_a_changed_document() {
        let original = NovelDocument::new(
            "book-1",
            "测试书",
            "book.txt",
            DocumentFormat::PlainText,
            vec![chapter("c1", 0, "接盘")],
        );
        let task = task(&original.id);
        let first = ready(engine().scan_batch(&task, &original, &[rule()], None, None, 0)).unwrap();
        let changed = NovelDocument::new(
            "book-1",
            "测试书",
            "book.txt",
            DocumentFormat::PlainText,
            vec![chapter("c1", 0, "文本已经改变")],
        );

        let error =
            ready(engine().scan_batch(&task, &changed, &[rule()], Some(first.checkpoint), None, 1))
                .unwrap_err();
        assert!(matches!(error, ScanError::ResumeMismatch(_)));
    }

    #[test]
    fn resume_rejects_tampered_exact_evidence() {
        let document = NovelDocument::new(
            "book-1",
            "测试书",
            "book.txt",
            DocumentFormat::PlainText,
            vec![chapter("c1", 0, "这里发生接盘")],
        );
        let task = task(&document.id);
        let mut completed = ready(engine().scan_batch(&task, &document, &[rule()], None, None, 1))
            .unwrap()
            .checkpoint;
        completed.findings[0].evidence[0].span.utf8_byte_end -= 1;

        let error =
            ready(engine().scan_batch(&task, &document, &[rule()], Some(completed), None, 0))
                .unwrap_err();
        assert!(matches!(error, ScanError::ResumeMismatch(_)));
    }

    #[test]
    fn zero_context_budget_is_rejected_before_scanning() {
        let document = NovelDocument::new(
            "book-1",
            "测试书",
            "book.txt",
            DocumentFormat::PlainText,
            vec![chapter("c1", 0, "第1章正文")],
        );
        let mut invalid_task = task(&document.id);
        invalid_task.config.context_budget_chars = 0;
        let store = InMemoryCheckpointStore::default();

        let error =
            ready(engine().scan_batch(&invalid_task, &document, &[rule()], None, Some(&store), 1))
                .unwrap_err();

        assert!(matches!(error, ScanError::InvalidInput(_)));
        let message = format!("{error}");
        assert!(
            message.contains("context_budget_chars"),
            "error message missing 'context_budget_chars': {message}"
        );
        assert!(
            message.contains("greater than zero"),
            "error message missing 'greater than zero': {message}"
        );
        assert!(store.load(&invalid_task.id).unwrap().is_none());
    }

    struct IdentityProvider {
        provider_id: String,
        model_id: String,
    }

    impl ModelProvider for IdentityProvider {
        fn provider_id(&self) -> &str {
            &self.provider_id
        }

        fn model_id(&self) -> &str {
            &self.model_id
        }

        fn analyze<'a>(&'a self, _request: &'a crate::InferenceRequest) -> ProviderFuture<'a> {
            Box::pin(async {
                Ok(ProviderResponse {
                    candidates: vec![],
                    ..Default::default()
                })
            })
        }
    }

    #[test]
    fn resume_requires_same_provider_and_model_identity() {
        let document = NovelDocument::new(
            "book-1",
            "测试书",
            "book.txt",
            DocumentFormat::PlainText,
            vec![chapter("c1", 0, "普通正文")],
        );
        let scan_task = task(&document.id);

        let provider_a_model_a = ScanEngine::new(
            Arc::new(IdentityProvider {
                provider_id: "provider-a".into(),
                model_id: "model-a".into(),
            }),
            Arc::new(DeterministicContextCompressor::default()),
        );
        let provider_b_model_a = ScanEngine::new(
            Arc::new(IdentityProvider {
                provider_id: "provider-b".into(),
                model_id: "model-a".into(),
            }),
            Arc::new(DeterministicContextCompressor::default()),
        );
        let provider_a_model_b = ScanEngine::new(
            Arc::new(IdentityProvider {
                provider_id: "provider-a".into(),
                model_id: "model-b".into(),
            }),
            Arc::new(DeterministicContextCompressor::default()),
        );

        // provider-a/model-a creates checkpoint
        let checkpoint_a =
            ready(provider_a_model_a.scan_batch(&scan_task, &document, &[rule()], None, None, 0))
                .unwrap()
                .checkpoint;

        // Same provider/model resumes successfully
        let resume_ok = ready(provider_a_model_a.scan_batch(
            &scan_task,
            &document,
            &[rule()],
            Some(checkpoint_a.clone()),
            None,
            0,
        ));
        assert!(resume_ok.is_ok(), "same provider/model should resume ok");

        // provider-b/model-a: must fail
        let resume_b = ready(provider_b_model_a.scan_batch(
            &scan_task,
            &document,
            &[rule()],
            Some(checkpoint_a.clone()),
            None,
            0,
        ));
        assert!(
            matches!(resume_b, Err(ScanError::ResumeMismatch(_))),
            "different provider should be rejected"
        );

        // provider-a/model-b: must fail
        let resume_c = ready(provider_a_model_b.scan_batch(
            &scan_task,
            &document,
            &[rule()],
            Some(checkpoint_a.clone()),
            None,
            0,
        ));
        assert!(
            matches!(resume_c, Err(ScanError::ResumeMismatch(_))),
            "different model should be rejected"
        );
    }

    #[test]
    fn finding_snapshots_rule_version_and_id_depends_on_it() {
        let document = NovelDocument::new(
            "book-1",
            "测试书",
            "book.txt",
            DocumentFormat::PlainText,
            vec![chapter("c1", 0, "接盘")],
        );
        let scan_task = task(&document.id);

        let rule_v1 = RuleDefinition {
            version: 1,
            ..rule()
        };
        let rule_v2 = RuleDefinition {
            version: 2,
            ..rule()
        };

        let result_v1 =
            ready(engine().scan_batch(&scan_task, &document, &[rule_v1.clone()], None, None, 1))
                .unwrap();
        let result_v2 =
            ready(engine().scan_batch(&scan_task, &document, &[rule_v2.clone()], None, None, 1))
                .unwrap();

        let f1 = &result_v1.checkpoint.findings[0];
        let f2 = &result_v2.checkpoint.findings[0];
        assert_eq!(f1.rule_version, 1);
        assert_eq!(f2.rule_version, 2);
        assert_ne!(
            f1.id, f2.id,
            "different versions must produce different finding IDs"
        );
    }

    struct PresetProvider {
        candidates: Vec<ProviderCandidate>,
    }

    impl ModelProvider for PresetProvider {
        fn provider_id(&self) -> &str {
            "preset"
        }

        fn model_id(&self) -> &str {
            "preset-v1"
        }

        fn analyze<'a>(&'a self, _request: &'a crate::InferenceRequest) -> ProviderFuture<'a> {
            Box::pin(async {
                Ok(ProviderResponse {
                    candidates: self.candidates.clone(),
                    ..Default::default()
                })
            })
        }
    }

    #[test]
    fn duplicate_provider_candidates_are_deduplicated() {
        let document = NovelDocument::new(
            "book-1",
            "测试书",
            "book.txt",
            DocumentFormat::PlainText,
            vec![chapter("c1", 0, "这里发生接盘")],
        );
        let scan_task = task(&document.id);
        let candidate = ProviderCandidate {
            rule_id: "takeover".into(),
            confidence_bps: 9_000,
            rationale: "重复候选".into(),
            requires_later_confirmation: false,
            evidence_ranges: vec![ProviderEvidenceRange {
                utf8_byte_start: 12, // "接盘" in "这里发生接盘"
                utf8_byte_end: 18,
            }],
        };
        let engine = ScanEngine::new(
            Arc::new(PresetProvider {
                candidates: vec![candidate.clone(), candidate.clone()],
            }),
            Arc::new(DeterministicContextCompressor::default()),
        );

        let result =
            ready(engine.scan_batch(&scan_task, &document, &[rule()], None, None, 1)).unwrap();
        assert_eq!(result.new_findings, 1);
        assert_eq!(result.checkpoint.findings.len(), 1);
    }

    #[test]
    fn conflicting_candidates_with_same_stable_id_are_rejected() {
        let document = NovelDocument::new(
            "book-1",
            "测试书",
            "book.txt",
            DocumentFormat::PlainText,
            vec![chapter("c1", 0, "这里发生接盘")],
        );
        let scan_task = task(&document.id);
        let start = 12u32;
        let end = 18u32;
        let range = ProviderEvidenceRange {
            utf8_byte_start: start as usize,
            utf8_byte_end: end as usize,
        };
        let candidate_a = ProviderCandidate {
            rule_id: "takeover".into(),
            confidence_bps: 9_000,
            rationale: "相同ID原料".into(),
            requires_later_confirmation: false,
            evidence_ranges: vec![range.clone()],
        };
        let candidate_b = ProviderCandidate {
            rule_id: "takeover".into(),
            confidence_bps: 5_000, // different confidence, same ID material
            rationale: "相同ID原料".into(),
            requires_later_confirmation: false,
            evidence_ranges: vec![range],
        };
        let engine = ScanEngine::new(
            Arc::new(PresetProvider {
                candidates: vec![candidate_a, candidate_b],
            }),
            Arc::new(DeterministicContextCompressor::default()),
        );
        let store = InMemoryCheckpointStore::default();

        let error =
            ready(engine.scan_batch(&scan_task, &document, &[rule()], None, Some(&store), 1))
                .unwrap_err();
        assert!(matches!(error, ScanError::InvalidInput(_)));
        let message = format!("{error}");
        assert!(
            message.contains("conflicting candidates"),
            "error should mention conflicting candidates: {message}"
        );
        assert!(store.load(&scan_task.id).unwrap().is_none());
    }

    #[test]
    fn resume_rejects_duplicate_finding_ids() {
        let document = NovelDocument::new(
            "book-1",
            "测试书",
            "book.txt",
            DocumentFormat::PlainText,
            vec![chapter("c1", 0, "接盘")],
        );
        let scan_task = task(&document.id);
        let mut checkpoint =
            ready(engine().scan_batch(&scan_task, &document, &[rule()], None, None, 1))
                .unwrap()
                .checkpoint;
        let dup = checkpoint.findings[0].clone();
        checkpoint.findings.push(dup);

        let error =
            ready(engine().scan_batch(&scan_task, &document, &[rule()], Some(checkpoint), None, 0))
                .unwrap_err();
        assert!(matches!(error, ScanError::ResumeMismatch(_)));
        assert!(format!("{error}").contains("duplicate finding id"));
    }

    #[test]
    fn resume_rejects_out_of_range_confidence() {
        let document = NovelDocument::new(
            "book-1",
            "测试书",
            "book.txt",
            DocumentFormat::PlainText,
            vec![chapter("c1", 0, "接盘")],
        );
        let scan_task = task(&document.id);
        let mut checkpoint =
            ready(engine().scan_batch(&scan_task, &document, &[rule()], None, None, 1))
                .unwrap()
                .checkpoint;
        checkpoint.findings[0].confidence_bps = 10_001;

        let error =
            ready(engine().scan_batch(&scan_task, &document, &[rule()], Some(checkpoint), None, 0))
                .unwrap_err();
        assert!(matches!(error, ScanError::ResumeMismatch(_)));
        let msg = format!("{error}");
        assert!(
            msg.contains("confidence_bps"),
            "msg missing confidence_bps: {msg}"
        );
        assert!(msg.contains("10000"), "msg missing 10000: {msg}");
    }

    #[test]
    fn resume_rejects_tampered_rule_snapshot() {
        let document = NovelDocument::new(
            "book-1",
            "测试书",
            "book.txt",
            DocumentFormat::PlainText,
            vec![chapter("c1", 0, "接盘")],
        );
        let scan_task = task(&document.id);
        let base = ready(engine().scan_batch(&scan_task, &document, &[rule()], None, None, 1))
            .unwrap()
            .checkpoint;

        // unknown rule_id
        let mut cp = base.clone();
        cp.findings[0].rule_id = "nonexistent".into();
        let error = ready(engine().scan_batch(&scan_task, &document, &[rule()], Some(cp), None, 0))
            .unwrap_err();
        assert!(matches!(error, ScanError::ResumeMismatch(_)));
        assert!(format!("{error}").contains("nonexistent"));

        // wrong rule_version
        let mut cp = base.clone();
        cp.findings[0].rule_version = 999;
        let error = ready(engine().scan_batch(&scan_task, &document, &[rule()], Some(cp), None, 0))
            .unwrap_err();
        assert!(matches!(error, ScanError::ResumeMismatch(_)));
        assert!(format!("{error}").contains("rule_version"));

        // wrong category
        let mut cp = base.clone();
        cp.findings[0].category = RuleCategory::Landmine;
        let error = ready(engine().scan_batch(&scan_task, &document, &[rule()], Some(cp), None, 0))
            .unwrap_err();
        assert!(matches!(error, ScanError::ResumeMismatch(_)));
        assert!(format!("{error}").contains("category"));

        // wrong alert_level
        let mut cp = base.clone();
        cp.findings[0].alert_level = AlertLevel::Info;
        let error = ready(engine().scan_batch(&scan_task, &document, &[rule()], Some(cp), None, 0))
            .unwrap_err();
        assert!(matches!(error, ScanError::ResumeMismatch(_)));
        assert!(format!("{error}").contains("alert_level"));
    }

    #[test]
    fn resume_rejects_tampered_provider_stamp() {
        let document = NovelDocument::new(
            "book-1",
            "测试书",
            "book.txt",
            DocumentFormat::PlainText,
            vec![chapter("c1", 0, "接盘")],
        );
        let scan_task = task(&document.id);
        let base = ready(engine().scan_batch(&scan_task, &document, &[rule()], None, None, 1))
            .unwrap()
            .checkpoint;

        // wrong provider_id
        let mut cp = base.clone();
        cp.findings[0].provider.provider_id = "other-provider".into();
        let error = ready(engine().scan_batch(&scan_task, &document, &[rule()], Some(cp), None, 0))
            .unwrap_err();
        assert!(matches!(error, ScanError::ResumeMismatch(_)));
        assert!(format!("{error}").contains("provider_id"));

        // wrong model_id
        let mut cp = base.clone();
        cp.findings[0].provider.model_id = "other-model".into();
        let error = ready(engine().scan_batch(&scan_task, &document, &[rule()], Some(cp), None, 0))
            .unwrap_err();
        assert!(matches!(error, ScanError::ResumeMismatch(_)));
        assert!(format!("{error}").contains("model_id"));
    }

    #[test]
    fn resume_rejects_finding_from_unprocessed_chapter() {
        let document = NovelDocument::new(
            "book-1",
            "测试书",
            "book.txt",
            DocumentFormat::PlainText,
            vec![chapter("c1", 0, "接盘"), chapter("c2", 1, "接盘")],
        );
        let scan_task = task(&document.id);
        // Scan only chapter 1
        let mut checkpoint =
            ready(engine().scan_batch(&scan_task, &document, &[rule()], None, None, 1))
                .unwrap()
                .checkpoint;
        // checkpoint.processed_chapters only has c1

        // Rewrite finding to point to chapter 2 with matching evidence
        let target = &document.chapters[1]; // c2
        let target_text = target.text.clone();
        let target_hash = stable_fingerprint(target_text.as_bytes());
        let span = TextSpan::from_valid_range(&target.text, 0, target.text.len());
        let quote = &target.text[span.utf8_byte_start..span.utf8_byte_end];

        checkpoint.findings[0].source = ChapterRef::from_chapter(&document.id, target);
        checkpoint.findings[0].evidence[0].source = ChapterRef::from_chapter(&document.id, target);
        checkpoint.findings[0].evidence[0].chapter_content_hash = target_hash.clone();
        checkpoint.findings[0].evidence[0].exact_quote = quote.to_owned();
        checkpoint.findings[0].evidence[0].quote_hash = stable_fingerprint(quote.as_bytes());
        checkpoint.findings[0].evidence[0].span = span;

        let error =
            ready(engine().scan_batch(&scan_task, &document, &[rule()], Some(checkpoint), None, 0))
                .unwrap_err();
        assert!(matches!(error, ScanError::ResumeMismatch(_)));
        let msg = format!("{error}");
        assert!(
            msg.contains("unprocessed chapter"),
            "msg missing 'unprocessed chapter': {msg}"
        );
        assert!(msg.contains("c2"), "msg missing chapter id c2: {msg}");
    }

    #[test]
    fn resume_rejects_confirmed_or_pending_without_evidence() {
        let document = NovelDocument::new(
            "book-1",
            "测试书",
            "book.txt",
            DocumentFormat::PlainText,
            vec![chapter("c1", 0, "接盘")],
        );
        let scan_task = task(&document.id);
        let base = ready(engine().scan_batch(&scan_task, &document, &[rule()], None, None, 1))
            .unwrap()
            .checkpoint;

        // Confirmed with empty evidence
        let mut cp = base.clone();
        cp.findings[0].status = FindingStatus::Confirmed;
        cp.findings[0].evidence.clear();
        let error = ready(engine().scan_batch(&scan_task, &document, &[rule()], Some(cp), None, 0))
            .unwrap_err();
        assert!(matches!(error, ScanError::ResumeMismatch(_)));
        let msg = format!("{error}");
        assert!(msg.contains("Confirmed"), "msg missing 'Confirmed': {msg}");
        assert!(msg.contains("evidence"), "msg missing 'evidence': {msg}");

        // PendingConfirmation with empty evidence
        let mut cp = base.clone();
        cp.findings[0].status = FindingStatus::PendingConfirmation;
        cp.findings[0].evidence.clear();
        let error = ready(engine().scan_batch(&scan_task, &document, &[rule()], Some(cp), None, 0))
            .unwrap_err();
        assert!(matches!(error, ScanError::ResumeMismatch(_)));
        let msg = format!("{error}");
        assert!(
            msg.contains("PendingConfirmation"),
            "msg missing 'PendingConfirmation': {msg}"
        );
        assert!(msg.contains("evidence"), "msg missing 'evidence': {msg}");
    }

    #[test]
    fn resume_allows_suspected_without_evidence() {
        let document = NovelDocument::new(
            "book-1",
            "测试书",
            "book.txt",
            DocumentFormat::PlainText,
            vec![chapter("c1", 0, "没有对应原文")],
        );
        let scan_task = task(&document.id);
        let engine = ScanEngine::new(
            Arc::new(InvalidEvidenceProvider),
            Arc::new(DeterministicContextCompressor::default()),
        );
        let checkpoint = ready(engine.scan_batch(&scan_task, &document, &[rule()], None, None, 1))
            .unwrap()
            .checkpoint;
        let finding = &checkpoint.findings[0];
        assert_eq!(finding.status, FindingStatus::Suspected);
        assert!(finding.evidence.is_empty());

        // Resume with same engine identity should succeed
        let result = ready(engine.scan_batch(
            &scan_task,
            &document,
            &[rule()],
            Some(checkpoint.clone()),
            None,
            0,
        ))
        .unwrap();
        let resumed = &result.checkpoint.findings[0];
        assert_eq!(resumed.status, FindingStatus::Suspected);
        assert!(resumed.evidence.is_empty());
    }

    // ── DS-11: scope/boundary gate on resume ──

    fn rule_with_scope(scope: ConfirmationScope) -> RuleDefinition {
        RuleDefinition {
            confirmation_scope: scope,
            requires_user_boundary: false,
            ..rule()
        }
    }

    fn rule_with_boundary() -> RuleDefinition {
        RuleDefinition {
            confirmation_scope: ConfirmationScope::Local,
            requires_user_boundary: true,
            ..rule()
        }
    }

    #[test]
    fn resume_reapplies_confirmation_scope_gate() {
        let document = NovelDocument::new(
            "book-1",
            "测试书",
            "book.txt",
            DocumentFormat::PlainText,
            vec![chapter("c1", 0, "接盘")],
        );
        let scan_task = task(&document.id);

        // CrossChapter: legitimately produces PendingConfirmation, tamper to Confirmed
        let r_cross = rule_with_scope(ConfirmationScope::CrossChapter);
        let mut cp_cross =
            ready(engine().scan_batch(&scan_task, &document, &[r_cross.clone()], None, None, 1))
                .unwrap()
                .checkpoint;
        cp_cross.findings[0].status = FindingStatus::Confirmed;
        let error = ready(engine().scan_batch(
            &scan_task,
            &document,
            &[r_cross.clone()],
            Some(cp_cross),
            None,
            0,
        ))
        .unwrap_err();
        assert!(matches!(error, ScanError::ResumeMismatch(_)));
        assert!(format!("{error}").contains("confirmation_scope"));

        // WholeBook: same pattern
        let r_whole = rule_with_scope(ConfirmationScope::WholeBook);
        let mut cp_whole =
            ready(engine().scan_batch(&scan_task, &document, &[r_whole.clone()], None, None, 1))
                .unwrap()
                .checkpoint;
        cp_whole.findings[0].status = FindingStatus::Confirmed;
        let error = ready(engine().scan_batch(
            &scan_task,
            &document,
            &[r_whole.clone()],
            Some(cp_whole),
            None,
            0,
        ))
        .unwrap_err();
        assert!(matches!(error, ScanError::ResumeMismatch(_)));
        assert!(format!("{error}").contains("confirmation_scope"));

        // Local: legitimate Confirmed should resume ok
        let r_local = rule_with_scope(ConfirmationScope::Local);
        let cp_local =
            ready(engine().scan_batch(&scan_task, &document, &[r_local.clone()], None, None, 1))
                .unwrap()
                .checkpoint;
        assert_eq!(cp_local.findings[0].status, FindingStatus::Confirmed);
        let resume = ready(engine().scan_batch(
            &scan_task,
            &document,
            &[r_local.clone()],
            Some(cp_local),
            None,
            0,
        ));
        assert!(resume.is_ok(), "Local Confirmed should resume ok");

        // Chapter: legitimate Confirmed should resume ok
        let r_chapter = rule_with_scope(ConfirmationScope::Chapter);
        let cp_chapter =
            ready(engine().scan_batch(&scan_task, &document, &[r_chapter.clone()], None, None, 1))
                .unwrap()
                .checkpoint;
        assert_eq!(cp_chapter.findings[0].status, FindingStatus::Confirmed);
        let resume = ready(engine().scan_batch(
            &scan_task,
            &document,
            &[r_chapter.clone()],
            Some(cp_chapter),
            None,
            0,
        ));
        assert!(resume.is_ok(), "Chapter Confirmed should resume ok");
    }

    #[test]
    fn resume_reapplies_user_boundary_gate() {
        let document = NovelDocument::new(
            "book-1",
            "测试书",
            "book.txt",
            DocumentFormat::PlainText,
            vec![chapter("c1", 0, "接盘")],
        );
        let scan_task = task(&document.id);
        let r = rule_with_boundary();
        let mut checkpoint =
            ready(engine().scan_batch(&scan_task, &document, &[r.clone()], None, None, 1))
                .unwrap()
                .checkpoint;
        // requires_user_boundary=true should produce PendingConfirmation
        assert_eq!(
            checkpoint.findings[0].status,
            FindingStatus::PendingConfirmation
        );
        // Tamper to Confirmed
        checkpoint.findings[0].status = FindingStatus::Confirmed;
        let error = ready(engine().scan_batch(
            &scan_task,
            &document,
            &[r.clone()],
            Some(checkpoint),
            None,
            0,
        ))
        .unwrap_err();
        assert!(matches!(error, ScanError::ResumeMismatch(_)));
        assert!(format!("{error}").contains("user boundary"));
    }

    // ── DS-12: unresolved_candidates integrity ──

    #[test]
    fn resume_rejects_unknown_or_duplicate_unresolved_candidates() {
        let document = NovelDocument::new(
            "book-1",
            "测试书",
            "book.txt",
            DocumentFormat::PlainText,
            vec![chapter("c1", 0, "身份尚未揭晓")],
        );
        let scan_task = task(&document.id);
        let engine = ScanEngine::new(
            Arc::new(PendingEvidenceProvider),
            Arc::new(DeterministicContextCompressor::default()),
        );
        let base = ready(engine.scan_batch(&scan_task, &document, &[rule()], None, None, 1))
            .unwrap()
            .checkpoint;

        // unknown unresolved candidate
        let mut cp = base.clone();
        cp.context
            .unresolved_candidates
            .push("finding_unknown".into());
        let error = ready(engine.scan_batch(&scan_task, &document, &[rule()], Some(cp), None, 0))
            .unwrap_err();
        assert!(matches!(error, ScanError::ResumeMismatch(_)));
        assert!(format!("{error}").contains("unknown unresolved candidate"));

        // duplicate unresolved candidate
        let mut cp = base.clone();
        let existing = cp.context.unresolved_candidates[0].clone();
        cp.context.unresolved_candidates.push(existing);
        let error = ready(engine.scan_batch(&scan_task, &document, &[rule()], Some(cp), None, 0))
            .unwrap_err();
        assert!(matches!(error, ScanError::ResumeMismatch(_)));
        assert!(format!("{error}").contains("duplicate unresolved candidate"));
    }

    #[test]
    fn resume_rejects_missing_or_resolved_unresolved_candidate() {
        let document = NovelDocument::new(
            "book-1",
            "测试书",
            "book.txt",
            DocumentFormat::PlainText,
            vec![chapter("c1", 0, "身份尚未揭晓")],
        );
        let scan_task = task(&document.id);
        let engine = ScanEngine::new(
            Arc::new(PendingEvidenceProvider),
            Arc::new(DeterministicContextCompressor::default()),
        );
        let base = ready(engine.scan_batch(&scan_task, &document, &[rule()], None, None, 1))
            .unwrap()
            .checkpoint;

        // missing from unresolved_candidates
        let mut cp = base.clone();
        cp.context.unresolved_candidates.clear();
        let error = ready(engine.scan_batch(&scan_task, &document, &[rule()], Some(cp), None, 0))
            .unwrap_err();
        assert!(matches!(error, ScanError::ResumeMismatch(_)));
        assert!(format!("{error}").contains("missing from unresolved candidates"));

        // unresolved candidate pointing to Confirmed finding (use Local scope rule
        // so the scope gate doesn't reject first)
        let r_local = rule_with_scope(ConfirmationScope::Local);
        let engine2 = ScanEngine::new(
            Arc::new(PendingEvidenceProvider),
            Arc::new(DeterministicContextCompressor::default()),
        );
        let mut cp2 =
            ready(engine2.scan_batch(&scan_task, &document, &[r_local.clone()], None, None, 1))
                .unwrap()
                .checkpoint;
        cp2.findings[0].status = FindingStatus::Confirmed;
        let error =
            ready(engine2.scan_batch(&scan_task, &document, &[r_local], Some(cp2), None, 0))
                .unwrap_err();
        // May be scope/boundary or unresolved status — both are ResumeMismatch
        // We check for unresolved candidate status error
        assert!(matches!(error, ScanError::ResumeMismatch(_)));
        let msg = format!("{error}");
        assert!(
            msg.contains("suspected or pending_confirmation")
                || msg.contains("unresolved candidate"),
            "msg should mention status constraint: {msg}"
        );
    }

    #[test]
    fn resume_accepts_consistent_unresolved_candidates() {
        let document = NovelDocument::new(
            "book-1",
            "测试书",
            "book.txt",
            DocumentFormat::PlainText,
            vec![chapter("c1", 0, "身份尚未揭晓")],
        );
        let scan_task = task(&document.id);

        // PendingConfirmation with unresolved
        let engine = ScanEngine::new(
            Arc::new(PendingEvidenceProvider),
            Arc::new(DeterministicContextCompressor::default()),
        );
        let cp = ready(engine.scan_batch(&scan_task, &document, &[rule()], None, None, 1))
            .unwrap()
            .checkpoint;
        assert!(!cp.context.unresolved_candidates.is_empty());
        let resume =
            ready(engine.scan_batch(&scan_task, &document, &[rule()], Some(cp.clone()), None, 0));
        assert!(
            resume.is_ok(),
            "consistent PendingConfirmation should resume ok"
        );

        // Suspected without evidence but with unresolved
        let engine2 = ScanEngine::new(
            Arc::new(InvalidEvidenceProvider),
            Arc::new(DeterministicContextCompressor::default()),
        );
        let cp2 = ready(engine2.scan_batch(&scan_task, &document, &[rule()], None, None, 1))
            .unwrap()
            .checkpoint;
        assert!(!cp2.context.unresolved_candidates.is_empty());
        assert_eq!(cp2.findings[0].status, FindingStatus::Suspected);
        let resume =
            ready(engine2.scan_batch(&scan_task, &document, &[rule()], Some(cp2.clone()), None, 0));
        assert!(resume.is_ok(), "consistent Suspected should resume ok");
        let resumed = &resume.unwrap().checkpoint;
        assert!(!resumed.context.unresolved_candidates.is_empty());
        assert!(resumed
            .context
            .unresolved_candidates
            .contains(&resumed.findings[0].id));
    }
}
