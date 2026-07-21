//! Recovery matrix — deterministic fault injection tests.
//! Proves provider errors don't corrupt completed chapters.

use novel_core::{
    AlertLevel, Chapter, CheckpointStore, ConfirmationScope, ContextCompressor, DetectionMode,
    DeterministicContextCompressor, DeterministicTestProvider, DocumentFormat,
    InMemoryCheckpointStore, NovelDocument, NovelTask, PatternRule, RuleCategory, RuleDefinition,
    RuleSelection, ScanConfig, ScanEngine, SourceLocator, TaskStatus,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

struct FaultInjectingProvider {
    inner: Arc<dyn novel_core::ModelProvider>,
    call_count: AtomicUsize,
    fail_after_n: usize,
}

impl FaultInjectingProvider {
    fn new(inner: Arc<dyn novel_core::ModelProvider>, fail_after_n: usize) -> Self {
        Self {
            inner,
            call_count: AtomicUsize::new(0),
            fail_after_n,
        }
    }
}

impl novel_core::ModelProvider for FaultInjectingProvider {
    fn provider_id(&self) -> &str {
        self.inner.provider_id()
    }
    fn model_id(&self) -> &str {
        self.inner.model_id()
    }
    fn analyze<'a>(
        &'a self,
        req: &'a novel_core::InferenceRequest,
    ) -> novel_core::ProviderFuture<'a> {
        let n = self.call_count.fetch_add(1, Ordering::SeqCst);
        if n == self.fail_after_n {
            Box::pin(async move {
                Err(novel_core::ProviderError::new(
                    "FAULT",
                    format!("fail at {n}"),
                    true,
                ))
            })
        } else {
            self.inner.analyze(req)
        }
    }
}

fn ch(id: &str, ord: u32, title: &str, text: &str) -> Chapter {
    Chapter::new(
        id,
        ord,
        title,
        text,
        SourceLocator::Unknown {
            description: "t".into(),
        },
    )
}

fn rule(id: &str, name: &str) -> RuleDefinition {
    RuleDefinition {
        id: id.into(),
        version: 1,
        name: name.into(),
        description: format!("{name} desc"),
        category: RuleCategory::Landmine,
        default_alert_level: AlertLevel::Critical,
        confirmation_scope: ConfirmationScope::Chapter,
        requires_user_boundary: false,
        tags: vec![],
        detection_profile_ref: None,
        detection_mode: DetectionMode::Semantic,
        criteria: vec![name.into()],
        exclusions: vec![],
        pending_conditions: vec![],
    }
}

fn doc(chs: Vec<Chapter>) -> NovelDocument {
    NovelDocument::new("d1", "Test", "t.txt", DocumentFormat::PlainText, chs)
}

fn task(id: &str, rids: &[&str]) -> NovelTask {
    NovelTask {
        id: id.into(),
        document_id: "d1".into(),
        status: TaskStatus::Pending,
        created_at_ms: 1,
        updated_at_ms: 1,
        selected_rules: rids
            .iter()
            .map(|rid| RuleSelection {
                rule_id: rid.to_string(),
                category: RuleCategory::Landmine,
                alert_level: AlertLevel::Critical,
                enabled: true,
            })
            .collect(),
        config: ScanConfig {
            context_budget_chars: 5000,
            retain_unverified_candidates: true,
            rule_pack_version_snapshot: None,
            rule_pack_id_snapshot: Some("rp-1".into()),
        },
    }
}

fn dp(ps: Vec<PatternRule>) -> Arc<dyn novel_core::ModelProvider> {
    Arc::new(DeterministicTestProvider::new(ps))
}

fn comp() -> Arc<dyn ContextCompressor> {
    Arc::new(DeterministicContextCompressor {
        excerpt_chars_per_chapter: 500,
    })
}

fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
}

fn run(
    t: &NovelTask,
    d: &NovelDocument,
    rs: &[RuleDefinition],
    p: Arc<dyn novel_core::ModelProvider>,
    s: Option<&dyn CheckpointStore>,
) -> Result<novel_core::ScanBatchResult, novel_core::ScanError> {
    let e = ScanEngine::new(p, comp());
    rt().block_on(e.scan_batch(t, d, rs, None, s, 100))
}

#[test]
fn baseline_clean_scan() {
    let d = doc(vec![ch("c1", 0, "A", "hello"), ch("c2", 1, "B", "bar")]);
    let t = task("t1", &["r1"]);
    let rs = vec![rule("r1", "hello")];
    let p = dp(vec![PatternRule {
        rule_id: "r1".into(),
        phrases: vec!["hello".into()],
        rationale: "x".into(),
        confidence_bps: 9000,
    }]);
    let r = run(&t, &d, &rs, p, None).unwrap();
    assert_eq!(r.checkpoint.findings.len(), 1);
}

#[test]
fn recovery_after_fault_preserves_results() {
    let d = doc(vec![ch("c1", 0, "A", "hello"), ch("c2", 1, "B", "bar")]);
    let t = task("t2", &["r1"]);
    let rs = vec![rule("r1", "hello")];
    let pat = vec![PatternRule {
        rule_id: "r1".into(),
        phrases: vec!["hello".into()],
        rationale: "x".into(),
        confidence_bps: 9000,
    }];
    let base = run(&t, &d, &rs, dp(pat.clone()), None).unwrap();

    let s = InMemoryCheckpointStore::default();
    let fp = Arc::new(FaultInjectingProvider::new(dp(pat.clone()), 0));
    assert!(run(&t, &d, &rs, fp, Some(&s)).is_err());

    let res = run(&t, &d, &rs, dp(pat), Some(&s)).unwrap();
    assert_eq!(
        res.checkpoint.findings.len(),
        base.checkpoint.findings.len()
    );
    assert_eq!(res.checkpoint.usage_totals, base.checkpoint.usage_totals);
}

#[test]
fn resume_no_duplicate_findings() {
    let d = doc(vec![
        ch("c1", 0, "A", "aaa"),
        ch("c2", 1, "B", "hello"),
        ch("c3", 2, "C", "bbb"),
    ]);
    let t = task("t3", &["r1"]);
    let rs = vec![rule("r1", "hello")];
    let pat = vec![PatternRule {
        rule_id: "r1".into(),
        phrases: vec!["hello".into()],
        rationale: "x".into(),
        confidence_bps: 9000,
    }];
    let base = run(&t, &d, &rs, dp(pat.clone()), None).unwrap();
    assert_eq!(base.checkpoint.findings.len(), 1);

    let s = InMemoryCheckpointStore::default();
    let _ = run(
        &t,
        &d,
        &rs,
        Arc::new(FaultInjectingProvider::new(dp(pat.clone()), 1)),
        Some(&s),
    );
    assert!(s.load("t3").unwrap().is_some());

    let res = run(&t, &d, &rs, dp(pat), Some(&s)).unwrap();
    assert_eq!(
        res.checkpoint.findings.len(),
        base.checkpoint.findings.len()
    );
    let mut ids: Vec<&str> = res
        .checkpoint
        .findings
        .iter()
        .map(|f| f.id.as_str())
        .collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), res.checkpoint.findings.len());
}

#[test]
fn changed_document_rejects_resume() {
    let d1 = doc(vec![ch("c1", 0, "A", "orig"), ch("c2", 1, "B", "more")]);
    let t = task("t4", &["r1"]);
    let rs = vec![rule("r1", "orig")];
    let pat = vec![PatternRule {
        rule_id: "r1".into(),
        phrases: vec!["orig".into()],
        rationale: "x".into(),
        confidence_bps: 9000,
    }];
    let s = InMemoryCheckpointStore::default();
    let _ = run(
        &t,
        &d1,
        &rs,
        Arc::new(FaultInjectingProvider::new(dp(pat.clone()), 1)),
        Some(&s),
    );

    let d2 = doc(vec![ch("c1", 0, "A", "orig"), ch("c2", 1, "B", "CHANGED")]);
    assert!(run(&t, &d2, &rs, dp(pat), Some(&s)).is_err());
}

#[test]
fn changed_rules_reject_resume() {
    let d = doc(vec![ch("c1", 0, "A", "orig"), ch("c2", 1, "B", "more")]);
    let t = task("t5", &["r1"]);
    let rs = vec![rule("r1", "orig")];
    let pat = vec![PatternRule {
        rule_id: "r1".into(),
        phrases: vec!["orig".into()],
        rationale: "x".into(),
        confidence_bps: 9000,
    }];
    let s = InMemoryCheckpointStore::default();
    let _ = run(
        &t,
        &d,
        &rs,
        Arc::new(FaultInjectingProvider::new(dp(pat.clone()), 1)),
        Some(&s),
    );

    let t2 = task("t5", &["r1", "r2"]);
    assert!(run(&t2, &d, &rs, dp(pat), Some(&s)).is_err());
}

#[test]
fn multi_fault_eventually_completes() {
    let d = doc(vec![
        ch("c1", 0, "A", "a"),
        ch("c2", 1, "B", "b"),
        ch("c3", 2, "C", "hello"),
        ch("c4", 3, "D", "c"),
    ]);
    let t = task("t6", &["r1"]);
    let rs = vec![rule("r1", "hello")];
    let pat = vec![PatternRule {
        rule_id: "r1".into(),
        phrases: vec!["hello".into()],
        rationale: "x".into(),
        confidence_bps: 9000,
    }];
    let base = run(&t, &d, &rs, dp(pat.clone()), None).unwrap();

    let s = InMemoryCheckpointStore::default();
    let _ = run(
        &t,
        &d,
        &rs,
        Arc::new(FaultInjectingProvider::new(dp(pat.clone()), 0)),
        Some(&s),
    );
    let _ = run(
        &t,
        &d,
        &rs,
        Arc::new(FaultInjectingProvider::new(dp(pat.clone()), 2)),
        Some(&s),
    );
    let res = run(&t, &d, &rs, dp(pat), Some(&s)).unwrap();

    assert_eq!(
        res.checkpoint.findings.len(),
        base.checkpoint.findings.len()
    );
    let mut ids: Vec<&str> = res
        .checkpoint
        .findings
        .iter()
        .map(|f| f.id.as_str())
        .collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), res.checkpoint.findings.len());
}

#[test]
fn zero_max_chapters_noop() {
    let d = doc(vec![ch("c1", 0, "A", "hello")]);
    let t = task("t7", &["r1"]);
    let rs = vec![rule("r1", "hello")];
    let p = dp(vec![PatternRule {
        rule_id: "r1".into(),
        phrases: vec!["hello".into()],
        rationale: "x".into(),
        confidence_bps: 9000,
    }]);
    let e = ScanEngine::new(p, comp());
    let r = rt()
        .block_on(e.scan_batch(&t, &d, &rs, None, None::<&dyn CheckpointStore>, 0))
        .unwrap();
    assert!(r.checkpoint.findings.is_empty());
}
