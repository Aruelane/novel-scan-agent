use std::{fmt, future::Future, pin::Pin};

use serde::{Deserialize, Serialize};

use crate::{Chapter, Finding};

pub type CompressionFuture<'a> =
    Pin<Box<dyn Future<Output = Result<ContextSnapshot, CompressionError>> + Send + 'a>>;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityMemory {
    pub entity_id: String,
    pub display_name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    /// Short, provider-generated state. It is context, never source evidence.
    pub state: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextSnapshot {
    pub revision: u64,
    pub processed_chapter_ids: Vec<String>,
    pub rolling_summary: String,
    #[serde(default)]
    pub entities: Vec<EntityMemory>,
    /// IDs or short descriptions that later chapters may confirm. They must
    /// never be surfaced as confirmed findings without exact source anchors.
    #[serde(default)]
    pub unresolved_candidates: Vec<String>,
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
}
