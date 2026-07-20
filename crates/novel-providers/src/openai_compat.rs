//! OpenAI-compatible adapter contract.
//!
//! Maps the OpenAI Chat Completions protocol to the novel-core `ModelProvider`
//! trait. This adapter does NOT make HTTP requests — it only defines the
//! request/response mapping. The actual HTTP transport is in a separate module.

use novel_core::{
    InferenceRequest, ModelProvider, ProviderCandidate, ProviderError, ProviderEvidenceRange,
    ProviderFuture, ProviderResponse, ProviderUsage, RuleContext,
};

use crate::config::ProviderProfile;
use crate::prompt::{build_system_prompt, build_user_message};
use crate::schema::WireResponse;
use crate::secret::SecretStore;

/// An OpenAI-compatible adapter. Configured via `ProviderProfile` but does
/// not contain API key material — keys are resolved at request time via the
/// `SecretStore`.
pub struct OpenAiCompatAdapter {
    profile: ProviderProfile,
    secret_store: std::sync::Arc<dyn SecretStore>,
    http_client: std::sync::Arc<dyn HttpTransport>,
}

impl OpenAiCompatAdapter {
    pub fn new(
        profile: ProviderProfile,
        secret_store: std::sync::Arc<dyn SecretStore>,
        http_client: std::sync::Arc<dyn HttpTransport>,
    ) -> Self {
        Self {
            profile,
            secret_store,
            http_client,
        }
    }
}

impl ModelProvider for OpenAiCompatAdapter {
    fn provider_id(&self) -> &str {
        &self.profile.id
    }

    fn model_id(&self) -> &str {
        &self.profile.model_id
    }

    fn analyze<'a>(&'a self, request: &'a InferenceRequest) -> ProviderFuture<'a> {
        Box::pin(async move {
            let key = resolve_key(&*self.secret_store, self.profile.credential_ref.as_deref())?;

            let body = build_openai_request(&self.profile.model_id, request);
            let response_bytes = self
                .http_client
                .post_json(
                    &format!(
                        "{}/chat/completions",
                        self.profile.base_url.trim_end_matches('/')
                    ),
                    &key,
                    &body,
                )
                .await?;

            let wire: WireResponse = serde_json::from_slice(&response_bytes).map_err(|e| {
                ProviderError::new("PARSE", format!("invalid response JSON: {e}"), false)
            })?;

            let candidates: Vec<ProviderCandidate> = wire
                .candidates
                .into_iter()
                .map(|wc| ProviderCandidate {
                    rule_id: wc.rule_id,
                    confidence_bps: wc.confidence_bps.min(10_000),
                    rationale: wc.rationale,
                    requires_later_confirmation: wc.requires_later_confirmation,
                    evidence_ranges: wc
                        .evidence_ranges
                        .into_iter()
                        .map(ProviderEvidenceRange::from)
                        .collect(),
                })
                .collect();

            Ok(ProviderResponse {
                candidates,
                usage: ProviderUsage {
                    input_units: wire.usage_input,
                    output_units: wire.usage_output,
                },
                ..Default::default()
            })
        })
    }
}

fn resolve_key(
    store: &dyn SecretStore,
    credential_ref: Option<&str>,
) -> Result<String, ProviderError> {
    let handle = credential_ref.ok_or_else(|| {
        ProviderError::new(
            "NO_CREDENTIAL",
            "no secret-ref configured for this provider".into(),
            false,
        )
    })?;

    match store.resolve(handle).map_err(|e| {
        ProviderError::new("SECRET_STORE", format!("secret store error: {e}"), false)
    })? {
        crate::secret::ResolvedSecret::Key(k) => Ok(k),
        crate::secret::ResolvedSecret::Missing => Err(ProviderError::new(
            "NO_CREDENTIAL",
            "credential not configured".into(),
            false,
        )),
        crate::secret::ResolvedSecret::Unavailable => Err(ProviderError::new(
            "SECRET_STORE",
            "platform secret store unavailable".into(),
            false,
        )),
    }
}

fn build_openai_request(model: &str, request: &InferenceRequest) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "messages": [
            {"role": "system", "content": build_system_prompt(&request.rules)},
            {"role": "user",   "content": build_user_message(request)}
        ],
        "temperature": 0.0,
        "max_tokens": 4096
    })
}

// ── HTTP transport contract (no actual HTTP yet) ────────────────

/// Minimal HTTP transport contract. Real implementations use `reqwest` or
/// platform-native HTTP clients. This keeps novel-core free of HTTP deps.
pub trait HttpTransport: Send + Sync {
    fn post_json<'a>(
        &'a self,
        url: &str,
        bearer_token: &str,
        body: &serde_json::Value,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Vec<u8>, ProviderError>> + Send + 'a>,
    >;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openai_request_contains_model_and_messages() {
        let chapter = novel_core::Chapter::new(
            "c1",
            0,
            "序章",
            "测试正文",
            novel_core::SourceLocator::Unknown {
                description: "t".into(),
            },
        );
        let request = InferenceRequest {
            task_id: "t1".into(),
            document_id: "d1".into(),
            chapter,
            rules: vec![],
            context: Default::default(),
        };
        let body = build_openai_request("gpt-4", &request);
        assert_eq!(body["model"], "gpt-4");
        assert_eq!(body["messages"].as_array().unwrap().len(), 2);
    }
}
