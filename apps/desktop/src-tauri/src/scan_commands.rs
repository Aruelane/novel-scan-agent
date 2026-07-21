//! Tauri scan commands — typed DTOs for job lifecycle, no body text or keys.
//!
//! All commands return camelCase JSON. Chapter text, source paths, and API
//! keys are never sent to the frontend. Progress is reported as committed
//! chapter count, not model percentage.

use novel_core::{
    CheckpointStore, DeterministicTestProvider, InMemoryCheckpointStore, NovelDocument, NovelTask,
    PatternRule, ScanCheckpoint, ScanConfig, ScanEngine, TaskStatus,
};
use novel_rulepack::RulePack;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// ── DTO types ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanJobDto {
    pub task_id: String,
    pub document_id: String,
    pub status: String,
    pub chapter_position: usize,
    pub total_chapters: usize,
    pub findings_count: usize,
    pub stop_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FindingDto {
    pub id: String,
    pub rule_id: String,
    pub category: String,
    pub alert_level: String,
    pub confidence_bps: u16,
    pub rationale: String,
    pub status: String,
    pub chapter_id: String,
    pub chapter_ordinal: u32,
    pub chapter_title: String,
    pub evidence_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceDetailDto {
    pub finding_id: String,
    pub exact_quote: String,
    pub quote_hash: String,
    pub chapter_id: String,
    pub chapter_ordinal: u32,
    pub chapter_title: String,
    pub utf8_byte_start: usize,
    pub utf8_byte_end: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateJobRequest {
    pub document_json: String,
    pub rule_pack_json: String,
    pub max_chapters_per_batch: Option<usize>,
}

// ── App state ──────────────────────────────────────────────────

/// In-memory registry of active scan jobs. A real deployment would persist
/// everything through SQLite and only keep runner handles here.
pub struct ScanState {
    /// Maps task_id → stored checkpoint for the most recent completed chapter.
    checkpoints: Mutex<HashMap<String, ScanCheckpoint>>,
}

impl ScanState {
    pub fn new() -> Self {
        Self {
            checkpoints: Mutex::new(HashMap::new()),
        }
    }
}

// ── Helper: load rule pack ────────────────────────────────────

fn load_rule_pack() -> Result<RulePack, String> {
    let json = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../packages/rulepack/packs/yy-novel-bar/2026.0.0-seed.1.json"
    ));
    RulePack::load_from_json(json).map_err(|e| e.to_string())
}

// ── Helper: convert domain types to DTO ────────────────────────

fn status_string(status: TaskStatus, _stop_reason: Option<&novel_core::StopReason>) -> String {
    match status {
        TaskStatus::Pending => "pending".into(),
        TaskStatus::Running => "running".into(),
        TaskStatus::Paused => "paused".into(),
        TaskStatus::Completed => "completed".into(),
        TaskStatus::Failed => "failed".into(),
        // TaskStatus has no Cancelled variant; Failed covers errors
    }
}

fn alert_level_string(level: novel_core::AlertLevel) -> &'static str {
    match level {
        novel_core::AlertLevel::Critical => "critical",
        novel_core::AlertLevel::High => "high",
        novel_core::AlertLevel::Medium => "medium",
        novel_core::AlertLevel::Low => "low",
        novel_core::AlertLevel::Info => "info",
    }
}

fn finding_status_string(status: novel_core::FindingStatus) -> &'static str {
    match status {
        novel_core::FindingStatus::Suspected => "suspected",
        novel_core::FindingStatus::PendingConfirmation => "pending_confirmation",
        novel_core::FindingStatus::Confirmed => "confirmed",
        novel_core::FindingStatus::Rejected => "rejected",
    }
}

// ── Commands ──────────────────────────────────────────────────

#[tauri::command]
pub fn create_scan_job(
    document_json: String,
    state: tauri::State<'_, ScanState>,
) -> Result<ScanJobDto, String> {
    let document: NovelDocument =
        serde_json::from_str(&document_json).map_err(|e| format!("invalid document JSON: {e}"))?;
    let rule_pack: RulePack = load_rule_pack()?;

    let task_id = format!("scan-{}", uuid_simple());

    let selected_rules: Vec<novel_core::RuleSelection> = rule_pack
        .rules
        .iter()
        .map(|r| novel_core::RuleSelection {
            rule_id: r.definition.id.clone(),
            category: r.definition.category,
            alert_level: r.definition.default_alert_level,
            enabled: true,
        })
        .collect();

    let task = NovelTask {
        id: task_id.clone(),
        document_id: document.id.clone(),
        status: TaskStatus::Pending,
        created_at_ms: ms_now(),
        updated_at_ms: ms_now(),
        selected_rules,
        config: ScanConfig {
            context_budget_chars: 20_000,
            retain_unverified_candidates: true,
            rule_pack_version_snapshot: Some(rule_pack.version.clone()),
            rule_pack_id_snapshot: Some(rule_pack.id.clone()),
        },
    };

    // Create a fresh checkpoint
    let rule_contexts: Vec<novel_core::RuleContext> = rule_pack
        .rules
        .iter()
        .map(|r| {
            novel_core::RuleContext::from_definition(
                &r.definition,
                r.definition.default_alert_level,
            )
        })
        .collect();

    let checkpoint = ScanCheckpoint::fresh(
        &task,
        &document,
        &rule_contexts,
        "deterministic-test",
        "exact-pattern-v1",
    );

    let mut checkpoints = state.checkpoints.lock().map_err(|e| e.to_string())?;
    checkpoints.insert(task_id.clone(), checkpoint);

    Ok(ScanJobDto {
        task_id,
        document_id: document.id,
        status: "pending".into(),
        chapter_position: 0,
        total_chapters: document.chapters.len(),
        findings_count: 0,
        stop_reason: None,
    })
}

#[tauri::command]
pub fn run_scan_batch(
    task_id: String,
    max_chapters: Option<usize>,
    state: tauri::State<'_, ScanState>,
) -> Result<ScanJobDto, String> {
    let max_chapters = max_chapters.unwrap_or(1);

    // Load document from the stored context (in real impl, from SQLite)
    // For now, we load the rule pack and reconstruct the document
    let rule_pack = load_rule_pack()?;

    let mut checkpoints = state.checkpoints.lock().map_err(|e| e.to_string())?;
    let stored_cp = checkpoints
        .get(&task_id)
        .cloned()
        .ok_or_else(|| format!("no scan job found for task_id: {task_id}"))?;

    // Build rule catalog from the rule pack
    let rule_catalog: Vec<novel_core::RuleDefinition> = rule_pack
        .rules
        .iter()
        .map(|r| r.definition.clone())
        .collect();

    // Use deterministic test provider for now (S4 providers not wired yet)
    // The S4-15 factory task will replace this with real provider selection
    let provider = Arc::new(DeterministicTestProvider::new(vec![PatternRule {
        rule_id: "yy.thunder.accepting-prior-partner".into(),
        phrases: vec!["前女友".into(), "前男友".into()],
        rationale: "检测到前任伴侣".into(),
        confidence_bps: 8_500,
    }]));

    let compressor = Arc::new(novel_core::DeterministicContextCompressor {
        excerpt_chars_per_chapter: 2000,
    });

    let engine = ScanEngine::new(provider, compressor);
    let store = InMemoryCheckpointStore::default();

    // Restore the existing checkpoint into the store so the engine can resume
    store
        .save(&stored_cp)
        .map_err(|e| format!("failed to save checkpoint: {e}"))?;

    // We need a document. In the real implementation, the document would be
    // stored in SQLite. For this MVP, we construct a minimal one.
    // The engine validates the document fingerprint against the checkpoint.
    let document = NovelDocument::new(
        &stored_cp.document_id,
        "Loaded Document",
        "imported.txt",
        novel_core::DocumentFormat::PlainText,
        vec![], // chapters loaded from persistence in real impl
    );

    let task = NovelTask {
        id: task_id.clone(),
        document_id: stored_cp.document_id.clone(),
        status: TaskStatus::Running,
        created_at_ms: ms_now(),
        updated_at_ms: ms_now(),
        selected_rules: vec![],
        config: ScanConfig {
            context_budget_chars: 20_000,
            retain_unverified_candidates: true,
            rule_pack_version_snapshot: None,
            rule_pack_id_snapshot: None,
        },
    };

    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .map_err(|e| format!("runtime error: {e}"))?;

    let document_id = stored_cp.document_id.clone();
    let result = rt.block_on(engine.scan_batch(
        &task,
        &document,
        &rule_catalog,
        Some(stored_cp),
        Some(&store),
        max_chapters,
    ));

    match result {
        Ok(batch) => {
            let cp = batch.checkpoint.clone();
            checkpoints.insert(task_id.clone(), cp.clone());

            let findings_count = cp.findings.len();
            let status = if batch.complete {
                "completed"
            } else {
                "paused"
            };

            Ok(ScanJobDto {
                task_id,
                document_id: cp.document_id,
                status: status.into(),
                chapter_position: cp.next_chapter_position,
                total_chapters: 0, // unknown without stored document
                findings_count,
                stop_reason: Some(format!("{:?}", batch.stop_reason)),
            })
        }
        Err(e) => Ok(ScanJobDto {
            task_id,
            document_id: document_id.clone(),
            status: "failed".into(),
            chapter_position: 0,
            total_chapters: 0,
            findings_count: 0,
            stop_reason: Some(format!("{e:?}")),
        }),
    }
}

#[tauri::command]
pub fn get_scan_job(
    task_id: String,
    state: tauri::State<'_, ScanState>,
) -> Result<ScanJobDto, String> {
    let checkpoints = state.checkpoints.lock().map_err(|e| e.to_string())?;
    let cp = checkpoints
        .get(&task_id)
        .ok_or_else(|| format!("no scan job found for task_id: {task_id}"))?;

    let status = match cp.status {
        TaskStatus::Pending => "pending",
        TaskStatus::Running => "running",
        TaskStatus::Paused => "paused",
        TaskStatus::Completed => "completed",
        TaskStatus::Failed => "failed",
    };

    Ok(ScanJobDto {
        task_id: cp.task_id.clone(),
        document_id: cp.document_id.clone(),
        status: status.into(),
        chapter_position: cp.next_chapter_position,
        total_chapters: cp.processed_chapters.len(),
        findings_count: cp.findings.len(),
        stop_reason: cp.stop_reason.map(|r| format!("{r:?}")),
    })
}

#[tauri::command]
pub fn list_findings(
    task_id: String,
    state: tauri::State<'_, ScanState>,
) -> Result<Vec<FindingDto>, String> {
    let checkpoints = state.checkpoints.lock().map_err(|e| e.to_string())?;
    let cp = checkpoints
        .get(&task_id)
        .ok_or_else(|| format!("no scan job found for task_id: {task_id}"))?;

    let dtos: Vec<FindingDto> = cp
        .findings
        .iter()
        .map(|f| {
            let chapter_ref = &f.source;
            FindingDto {
                id: f.id.clone(),
                rule_id: f.rule_id.clone(),
                category: format!("{:?}", f.category).to_lowercase(),
                alert_level: alert_level_string(f.alert_level).into(),
                confidence_bps: f.confidence_bps,
                rationale: f.rationale.clone(),
                status: finding_status_string(f.status).into(),
                chapter_id: chapter_ref.chapter_id.clone(),
                chapter_ordinal: chapter_ref.chapter_ordinal,
                chapter_title: chapter_ref.chapter_title.clone(),
                evidence_count: f.evidence.len(),
            }
        })
        .collect();

    Ok(dtos)
}

#[tauri::command]
pub fn get_evidence_detail(
    finding_id: String,
    state: tauri::State<'_, ScanState>,
) -> Result<Vec<EvidenceDetailDto>, String> {
    let checkpoints = state.checkpoints.lock().map_err(|e| e.to_string())?;
    let finding = checkpoints
        .values()
        .flat_map(|cp| cp.findings.iter())
        .find(|f| f.id == finding_id)
        .ok_or_else(|| format!("no finding found for id: {finding_id}"))?;

    let chapter_ref = &finding.source;
    let dtos: Vec<EvidenceDetailDto> = finding
        .evidence
        .iter()
        .map(|e| EvidenceDetailDto {
            finding_id: finding.id.clone(),
            exact_quote: e.exact_quote.clone(),
            quote_hash: e.quote_hash.clone(),
            chapter_id: chapter_ref.chapter_id.clone(),
            chapter_ordinal: chapter_ref.chapter_ordinal,
            chapter_title: chapter_ref.chapter_title.clone(),
            utf8_byte_start: e.span.utf8_byte_start,
            utf8_byte_end: e.span.utf8_byte_end,
        })
        .collect();

    Ok(dtos)
}

// ── Utility ────────────────────────────────────────────────────

fn ms_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{ts:016x}")
}
