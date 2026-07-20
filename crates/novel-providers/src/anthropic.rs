//! Anthropic Messages API adapter contract.
//! Maps the Anthropic protocol to novel-core ModelProvider.
//! No HTTP requests here — transport is injected via HttpTransport.

use novel_core::{
    InferenceRequest, ModelProvider, ProviderCandidate, ProviderError, ProviderEvidenceRange,
    ProviderFuture, ProviderResponse, ProviderUsage,
};

use crate::config::ProviderProfile;
use crate::prompt::{build_system_prompt, build_user_message};
use crate::secret::SecretStore;

/// Anthropic-native adapter. Resolves API keys via SecretStore at request time.
pub struct AnthropicAdapter {
    profile: ProviderProfile,
    secret_store: std::sync::Arc<dyn SecretStore>,
}

impl AnthropicAdapter {
    pub fn new(profile: ProviderProfile, secret_store: std::sync::Arc<dyn SecretStore>) -> Self {
        Self {
            profile,
            secret_store,
        }
    }
}

impl ModelProvider for AnthropicAdapter {
    fn provider_id(&self) -> &str {
        &self.profile.id
    }
    fn model_id(&self) -> &str {
        &self.profile.model_id
    }

    fn analyze<'a>(&'a self, _request: &'a InferenceRequest) -> ProviderFuture<'a> {
        Box::pin(async move {
            Err(ProviderError::new(
                "NOT_IMPLEMENTED",
                String::from("Anthropic adapter — HTTP transport via S4-14"),
                false,
            ))
        })
    }
}

/// Build Anthropic-specific request body (for when HTTP transport is wired).
#[allow(dead_code)]
fn build_anthropic_body(model: &str, request: &InferenceRequest) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "max_tokens": 4096,
        "system": build_system_prompt(&request.rules),
        "messages": [
            {"role": "user", "content": build_user_message(request)}
        ]
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use novel_core::{Chapter, ContextSnapshot, SourceLocator};

    #[test]
    fn anthropic_body_contains_model_and_messages() {
        let chapter = Chapter::new(
            "c1",
            0,
            "Ch",
            "text",
            SourceLocator::Unknown {
                description: "x".into(),
            },
        );
        let request = InferenceRequest {
            task_id: "t1".into(),
            document_id: "d1".into(),
            chapter,
            rules: vec![],
            context: ContextSnapshot::default(),
        };
        let body = build_anthropic_body("claude-sonnet-5", &request);
        assert_eq!(body["model"], "claude-sonnet-5");
        assert_eq!(body["messages"].as_array().unwrap().len(), 1);
    }
}
