use std::{fmt, future::Future, pin::Pin};

use serde::{Deserialize, Serialize};

use crate::{
    AlertLevel, Chapter, ConfirmationScope, ContextSnapshot, RuleCategory, RuleDefinition,
};

pub type ProviderFuture<'a> =
    Pin<Box<dyn Future<Output = Result<ProviderResponse, ProviderError>> + Send + 'a>>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleContext {
    pub id: String,
    pub version: u32,
    pub name: String,
    pub description: String,
    pub category: RuleCategory,
    pub alert_level: AlertLevel,
    pub confirmation_scope: ConfirmationScope,
    pub requires_user_boundary: bool,
}

impl RuleContext {
    pub fn from_definition(rule: &RuleDefinition, alert_level: AlertLevel) -> Self {
        Self {
            id: rule.id.clone(),
            version: rule.version,
            name: rule.name.clone(),
            description: rule.description.clone(),
            category: rule.category,
            alert_level,
            confirmation_scope: rule.confirmation_scope,
            requires_user_boundary: rule.requires_user_boundary,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InferenceRequest {
    pub task_id: String,
    pub document_id: String,
    pub chapter: Chapter,
    pub rules: Vec<RuleContext>,
    pub context: ContextSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderEvidenceRange {
    pub utf8_byte_start: usize,
    pub utf8_byte_end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderCandidate {
    pub rule_id: String,
    pub confidence_bps: u16,
    pub rationale: String,
    /// The chapter contains a clue, but later chapters or relationship facts
    /// are required before the rule can be confirmed. The core only emits
    /// `pending_confirmation` when the supplied evidence is valid.
    #[serde(default)]
    pub requires_later_confirmation: bool,
    /// UTF-8 byte ranges in `InferenceRequest.chapter.text`. The core validates
    /// every boundary and reconstructs quotes directly from that text.
    pub evidence_ranges: Vec<ProviderEvidenceRange>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderUsage {
    /// Provider-neutral accounting units. Adapters may map these to tokens.
    pub input_units: u64,
    pub output_units: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderResponse {
    pub candidates: Vec<ProviderCandidate>,
    pub usage: ProviderUsage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

impl ProviderError {
    pub fn new(code: impl Into<String>, message: impl Into<String>, retryable: bool) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            retryable,
        }
    }
}

impl fmt::Display for ProviderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ProviderError {}

/// Runtime API adapters implement this trait. Returning a boxed future keeps
/// it object-safe, so users can switch providers without rebuilding the core.
pub trait ModelProvider: Send + Sync {
    fn provider_id(&self) -> &str;
    fn model_id(&self) -> &str;
    fn analyze<'a>(&'a self, request: &'a InferenceRequest) -> ProviderFuture<'a>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatternRule {
    pub rule_id: String,
    pub phrases: Vec<String>,
    pub rationale: String,
    pub confidence_bps: u16,
}

/// Offline provider used for deterministic tests, UI development, and smoke
/// tests. It is not presented as an AI model and requires no API credential.
#[derive(Debug, Clone)]
pub struct DeterministicTestProvider {
    provider_id: String,
    model_id: String,
    patterns: Vec<PatternRule>,
}

impl DeterministicTestProvider {
    pub fn new(patterns: Vec<PatternRule>) -> Self {
        Self {
            provider_id: "deterministic-test".to_owned(),
            model_id: "exact-pattern-v1".to_owned(),
            patterns,
        }
    }
}

impl ModelProvider for DeterministicTestProvider {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }

    fn model_id(&self) -> &str {
        &self.model_id
    }

    fn analyze<'a>(&'a self, request: &'a InferenceRequest) -> ProviderFuture<'a> {
        Box::pin(async move {
            let enabled_rule_ids = request
                .rules
                .iter()
                .map(|rule| rule.id.as_str())
                .collect::<std::collections::HashSet<_>>();
            let mut candidates = Vec::new();

            for pattern in &self.patterns {
                if !enabled_rule_ids.contains(pattern.rule_id.as_str()) {
                    continue;
                }
                for phrase in &pattern.phrases {
                    if phrase.is_empty() {
                        continue;
                    }
                    for (start, matched) in request.chapter.text.match_indices(phrase) {
                        candidates.push(ProviderCandidate {
                            rule_id: pattern.rule_id.clone(),
                            confidence_bps: pattern.confidence_bps.min(10_000),
                            rationale: pattern.rationale.clone(),
                            requires_later_confirmation: false,
                            evidence_ranges: vec![ProviderEvidenceRange {
                                utf8_byte_start: start,
                                utf8_byte_end: start + matched.len(),
                            }],
                        });
                    }
                }
            }

            Ok(ProviderResponse {
                usage: ProviderUsage {
                    input_units: request.chapter.text.chars().count() as u64,
                    output_units: candidates.len() as u64,
                },
                candidates,
            })
        })
    }
}

#[cfg(test)]
mod tests {
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
    fn deterministic_provider_returns_exact_utf8_ranges() {
        let provider = DeterministicTestProvider::new(vec![PatternRule {
            rule_id: "takeover".into(),
            phrases: vec!["接盘".into()],
            rationale: "明确词组".into(),
            confidence_bps: 9_000,
        }]);
        let chapter = Chapter::new(
            "c1",
            0,
            "第一章",
            "他说：接盘是不可能的。",
            SourceLocator::Unknown {
                description: "test".into(),
            },
        );
        let request = InferenceRequest {
            task_id: "t1".into(),
            document_id: "d1".into(),
            chapter,
            rules: vec![RuleContext {
                id: "takeover".into(),
                version: 1,
                name: "接盘".into(),
                description: String::new(),
                category: RuleCategory::Landmine,
                alert_level: AlertLevel::Critical,
                confirmation_scope: ConfirmationScope::Chapter,
                requires_user_boundary: false,
            }],
            context: ContextSnapshot::default(),
        };

        let response = ready(provider.analyze(&request)).unwrap();
        let range = &response.candidates[0].evidence_ranges[0];
        let (start, end) = (range.utf8_byte_start, range.utf8_byte_end);
        assert_eq!(&request.chapter.text[start..end], "接盘");
        assert_eq!(response.candidates.len(), 1);
    }
}
