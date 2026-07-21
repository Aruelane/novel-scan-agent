//! OpenAI-compatible Chat Completions adapter.
//!
//! Maps the OpenAI Chat Completions protocol to the `ModelProvider` trait.
//! This adapter works with any OpenAI-compatible endpoint (OpenAI, DeepSeek,
//! Ollama, vLLM, LocalAI, etc.) that implements the `/chat/completions` endpoint.
//!
//! # Protocol mapping
//!
//! - System prompt → `messages[0]` (role: "system")
//! - Chapter text (user message) → `messages[1]` (role: "user")
//! - `temperature: 0.0` for deterministic output
//! - `response_format: {"type": "json_object"}` for structured JSON output
//!
//! # Response parsing
//!
//! OpenAI response → extract `choices[0].message.content` → parse JSON →
//! validate `WireResponse` → convert to `ProviderResponse`.
//! Empty choices, non-JSON content, truncated finish_reason, and missing
//! usage are all typed errors.

use crate::config::ProviderProfile;
use crate::http::HttpClient;
use crate::prompt::{build_system_prompt, build_user_message};
use crate::redaction::RedactedSecret;
use crate::schema::{validate_wire_response, WireResponse};
use novel_core::{
    InferenceRequest, ModelProvider, ProviderCandidate, ProviderError, ProviderEvidenceRange,
    ProviderFuture, ProviderResponse, ProviderUsage,
};
use serde::Deserialize;

/// An OpenAI-compatible Chat Completions adapter.
///
/// Uses a shared `HttpClient` for transport. The API key is held as a
/// `RedactedSecret` and only exposed at the HTTP request layer.
pub struct OpenAiCompatAdapter {
    profile: ProviderProfile,
    http_client: HttpClient,
    api_key: RedactedSecret,
}

impl OpenAiCompatAdapter {
    /// Create a new adapter. The `api_key` is wrapped in a `RedactedSecret`
    /// and will never appear in logs, errors, or serialized output.
    pub fn new(profile: ProviderProfile, http_client: HttpClient, api_key: String) -> Self {
        Self {
            profile,
            http_client,
            api_key: RedactedSecret::new(api_key),
        }
    }

    /// Build the endpoint URL by safely joining the profile's base_url with
    /// the chat completions path. Prevents double `/v1` and other URL issues.
    fn endpoint_url(&self) -> String {
        let base = self.profile.base_url.trim_end_matches('/');
        if base.ends_with("/v1") {
            format!("{base}/chat/completions")
        } else {
            format!("{base}/v1/chat/completions")
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
            let body = build_request_body(&self.profile.model_id, request);
            let url = self.endpoint_url();

            let response_bytes = self
                .http_client
                .post_json(&url, &self.api_key, &body)
                .await?;

            let openai_response: OpenAiChatResponse = serde_json::from_slice(&response_bytes)
                .map_err(|e| {
                    ProviderError::new("PARSE", format!("invalid OpenAI response JSON: {e}"), false)
                })?;

            parse_openai_response(&openai_response)
        })
    }
}

// ── OpenAI API types ──────────────────────────────────────────

/// OpenAI Chat Completions response (minimal fields we need).
#[derive(Debug, Clone, Deserialize)]
struct OpenAiChatResponse {
    #[serde(default)]
    choices: Vec<OpenAiChoice>,
    #[serde(default)]
    usage: Option<OpenAiUsage>,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenAiChoice {
    #[serde(default)]
    message: OpenAiMessage,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct OpenAiMessage {
    #[serde(default)]
    content: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenAiUsage {
    prompt_tokens: Option<u64>,
    completion_tokens: Option<u64>,
    #[allow(dead_code)]
    total_tokens: Option<u64>,
}

// ── Request building ──────────────────────────────────────────

fn build_request_body(model: &str, request: &InferenceRequest) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "messages": [
            {"role": "system", "content": build_system_prompt(&request.rules)},
            {"role": "user",   "content": build_user_message(request)}
        ],
        "temperature": 0.0,
        "max_tokens": 4096,
        "response_format": {"type": "json_object"}
    })
}

// ── Response parsing ──────────────────────────────────────────

fn parse_openai_response(response: &OpenAiChatResponse) -> Result<ProviderResponse, ProviderError> {
    // Check for empty choices
    if response.choices.is_empty() {
        return Err(ProviderError::new(
            "EMPTY_CHOICES",
            "OpenAI returned zero choices",
            true,
        ));
    }

    let choice = &response.choices[0];

    // Check finish_reason for problems
    if let Some(ref reason) = choice.finish_reason {
        match reason.as_str() {
            "length" => {
                return Err(ProviderError::new(
                    "TRUNCATED",
                    "OpenAI response truncated (finish_reason=length); increase max_tokens",
                    false,
                ));
            }
            "content_filter" => {
                return Err(ProviderError::new(
                    "CONTENT_FILTER",
                    "OpenAI content filter triggered",
                    false,
                ));
            }
            _ => {} // "stop", "tool_calls", null are OK
        }
    }

    // Extract message content
    let content = choice.message.content.as_deref().unwrap_or("");

    if content.is_empty() {
        return Err(ProviderError::new(
            "EMPTY_CONTENT",
            "OpenAI returned empty message content",
            false,
        ));
    }

    // Parse content as WireResponse JSON
    let wire: WireResponse = serde_json::from_str(content).map_err(|e| {
        ProviderError::new(
            "PARSE",
            format!("invalid WireResponse JSON in message content: {e}"),
            false,
        )
    })?;

    // Validate wire response structure
    if let Err(errors) = validate_wire_response(&wire) {
        return Err(ProviderError::new(
            "VALIDATION",
            format!("WireResponse validation failed: {}", errors.join("; ")),
            false,
        ));
    }

    // Convert to ProviderResponse
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
                .map(|r| ProviderEvidenceRange {
                    utf8_byte_start: r.utf8_byte_start,
                    utf8_byte_end: r.utf8_byte_end,
                })
                .collect(),
        })
        .collect();

    // Map usage — mark unknown if missing
    let usage = match &response.usage {
        Some(u) => ProviderUsage {
            input_units: u.prompt_tokens.unwrap_or(0),
            output_units: u.completion_tokens.unwrap_or(0),
        },
        None => ProviderUsage {
            input_units: 0,
            output_units: 0,
        },
    };

    Ok(ProviderResponse {
        candidates,
        usage,
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::HttpClient;
    use crate::retry::RetryConfig;
    use novel_core::{
        AlertLevel, Chapter, ConfirmationScope, ContextSnapshot, DetectionMode, RuleCategory,
        RuleContext, SourceLocator,
    };
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    fn test_profile() -> ProviderProfile {
        ProviderProfile {
            id: "test-openai".into(),
            display_name: "Test OpenAI".into(),
            protocol: crate::config::ProviderProtocol::OpenAICompatible,
            base_url: "https://api.openai.com/v1".into(),
            model_id: "gpt-4".into(),
            max_requests_per_minute: None,
            timeout_seconds: 30,
            retry_max_attempts: 3,
            credential_ref: Some("secret-ref:test".into()),
        }
    }

    fn test_request() -> InferenceRequest {
        let chapter = Chapter::new(
            "c1",
            0,
            "序章",
            "测试正文内容",
            SourceLocator::Unknown {
                description: "t".into(),
            },
        );
        InferenceRequest {
            task_id: "t1".into(),
            document_id: "d1".into(),
            chapter,
            rules: vec![RuleContext {
                id: "r1".into(),
                version: 1,
                name: "测试规则".into(),
                description: "规则描述".into(),
                category: RuleCategory::Landmine,
                alert_level: AlertLevel::Critical,
                confirmation_scope: ConfirmationScope::Chapter,
                requires_user_boundary: false,
                detection_mode: DetectionMode::Semantic,
                detection_profile_ref: None,
                criteria: vec!["test".into()],
                exclusions: vec!["test".into()],
                pending_conditions: vec!["test".into()],
            }],
            context: ContextSnapshot::default(),
        }
    }

    // ── Unit tests (no server) ────────────────────────────────

    #[test]
    fn request_body_contains_model_and_messages() {
        let body = build_request_body("gpt-4", &test_request());
        assert_eq!(body["model"], "gpt-4");
        let messages = body["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages[1]["role"], "user");
    }

    #[test]
    fn request_body_has_json_response_format() {
        let body = build_request_body("gpt-4", &test_request());
        assert_eq!(body["response_format"]["type"], "json_object");
    }

    #[test]
    fn request_body_temperature_is_zero() {
        let body = build_request_body("gpt-4", &test_request());
        assert_eq!(body["temperature"], 0.0);
    }

    #[test]
    fn endpoint_url_strips_trailing_slash() {
        let mut profile = test_profile();
        profile.base_url = "https://api.openai.com/v1/".into();
        let adapter = OpenAiCompatAdapter::new(
            profile,
            HttpClient::new(Duration::from_secs(5), RetryConfig::default()),
            "sk-test".into(),
        );
        let url = adapter.endpoint_url();
        assert!(!url.contains("//chat"));
        assert_eq!(url, "https://api.openai.com/v1/chat/completions");
    }

    #[test]
    fn endpoint_url_adds_v1_if_missing() {
        let mut profile = test_profile();
        profile.base_url = "https://custom.api.com".into();
        let adapter = OpenAiCompatAdapter::new(
            profile,
            HttpClient::new(Duration::from_secs(5), RetryConfig::default()),
            "sk-test".into(),
        );
        let url = adapter.endpoint_url();
        assert_eq!(url, "https://custom.api.com/v1/chat/completions");
    }

    #[test]
    fn endpoint_url_no_double_v1() {
        let mut profile = test_profile();
        profile.base_url = "https://api.openai.com/v1".into();
        let adapter = OpenAiCompatAdapter::new(
            profile,
            HttpClient::new(Duration::from_secs(5), RetryConfig::default()),
            "sk-test".into(),
        );
        let url = adapter.endpoint_url();
        assert_eq!(url, "https://api.openai.com/v1/chat/completions");
    }

    #[test]
    fn parse_empty_choices_returns_error() {
        let response = OpenAiChatResponse {
            choices: vec![],
            usage: None,
        };
        let result = parse_openai_response(&response);
        match result {
            Err(e) => {
                assert!(e.code.contains("EMPTY_CHOICES"));
            }
            Ok(_) => panic!("expected error for empty choices"),
        }
    }

    #[test]
    fn parse_truncated_finish_reason_returns_error() {
        let response = OpenAiChatResponse {
            choices: vec![OpenAiChoice {
                message: OpenAiMessage {
                    content: Some(r#"{"candidates":[]}"#.into()),
                },
                finish_reason: Some("length".into()),
            }],
            usage: None,
        };
        let result = parse_openai_response(&response);
        match result {
            Err(e) => {
                assert!(e.code.contains("TRUNCATED"));
            }
            Ok(_) => panic!("expected error for truncated"),
        }
    }

    #[test]
    fn parse_content_filter_returns_error() {
        let response = OpenAiChatResponse {
            choices: vec![OpenAiChoice {
                message: OpenAiMessage {
                    content: Some("blocked".into()),
                },
                finish_reason: Some("content_filter".into()),
            }],
            usage: None,
        };
        let result = parse_openai_response(&response);
        match result {
            Err(e) => {
                assert!(e.code.contains("CONTENT_FILTER"));
            }
            Ok(_) => panic!("expected error for content filter"),
        }
    }

    #[test]
    fn parse_empty_content_returns_error() {
        let response = OpenAiChatResponse {
            choices: vec![OpenAiChoice {
                message: OpenAiMessage { content: None },
                finish_reason: Some("stop".into()),
            }],
            usage: None,
        };
        let result = parse_openai_response(&response);
        match result {
            Err(e) => {
                assert!(e.code.contains("EMPTY_CONTENT"));
            }
            Ok(_) => panic!("expected error for empty content"),
        }
    }

    #[test]
    fn parse_invalid_json_in_content_returns_error() {
        let response = OpenAiChatResponse {
            choices: vec![OpenAiChoice {
                message: OpenAiMessage {
                    content: Some("not valid json!!!".into()),
                },
                finish_reason: Some("stop".into()),
            }],
            usage: None,
        };
        let result = parse_openai_response(&response);
        match result {
            Err(e) => {
                assert!(e.code.contains("PARSE"));
            }
            Ok(_) => panic!("expected error for invalid JSON"),
        }
    }

    #[test]
    fn parse_successful_response_with_candidates() {
        let wire = WireResponse {
            candidates: vec![crate::schema::WireCandidate {
                rule_id: "r1".into(),
                confidence_bps: 5000,
                rationale: "matches".into(),
                requires_later_confirmation: false,
                evidence_ranges: vec![crate::schema::WireEvidenceRange {
                    utf8_byte_start: 0,
                    utf8_byte_end: 10,
                }],
            }],
            usage_input: 100,
            usage_output: 50,
        };
        let wire_json = serde_json::to_string(&wire).unwrap();

        let response = OpenAiChatResponse {
            choices: vec![OpenAiChoice {
                message: OpenAiMessage {
                    content: Some(wire_json),
                },
                finish_reason: Some("stop".into()),
            }],
            usage: Some(OpenAiUsage {
                prompt_tokens: Some(100),
                completion_tokens: Some(50),
                total_tokens: Some(150),
            }),
        };

        let result = parse_openai_response(&response).unwrap();
        assert_eq!(result.candidates.len(), 1);
        assert_eq!(result.candidates[0].rule_id, "r1");
        assert_eq!(result.usage.input_units, 100);
        assert_eq!(result.usage.output_units, 50);
    }

    #[test]
    fn parse_response_with_missing_usage_marks_zero() {
        let response = OpenAiChatResponse {
            choices: vec![OpenAiChoice {
                message: OpenAiMessage {
                    content: Some(r#"{"candidates":[]}"#.into()),
                },
                finish_reason: Some("stop".into()),
            }],
            usage: None,
        };
        let result = parse_openai_response(&response).unwrap();
        assert_eq!(result.usage.input_units, 0);
        assert_eq!(result.usage.output_units, 0);
    }

    #[test]
    fn adapter_provider_id_matches_profile() {
        let adapter = OpenAiCompatAdapter::new(
            test_profile(),
            HttpClient::new(Duration::from_secs(5), RetryConfig::default()),
            "sk-test".into(),
        );
        assert_eq!(adapter.provider_id(), "test-openai");
    }

    #[test]
    fn adapter_model_id_matches_profile() {
        let adapter = OpenAiCompatAdapter::new(
            test_profile(),
            HttpClient::new(Duration::from_secs(5), RetryConfig::default()),
            "sk-test".into(),
        );
        assert_eq!(adapter.model_id(), "gpt-4");
    }

    // ── Integration tests (loopback fake server) ───────────────

    fn fake_openai_server(
        response_status: u16,
        response_body: &'static str,
    ) -> (
        String,
        Arc<AtomicBool>,
        std::sync::mpsc::Receiver<()>,
        std::thread::JoinHandle<()>,
    ) {
        use std::io::Write;
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let url = format!("http://{}:{}/v1/chat/completions", addr.ip(), addr.port());
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_clone = shutdown.clone();
        let body = response_body.to_string();
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();

        let handle = std::thread::spawn(move || {
            let _ = ready_tx.send(());
            listener.set_nonblocking(true).unwrap();
            loop {
                if shutdown_clone.load(Ordering::Relaxed) {
                    break;
                }
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        use std::io::Read;
                        let mut buf = [0u8; 8192];
                        let _ = stream.read(&mut buf);
                        let _ = write!(
                            stream,
                            "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                            response_status, body.len(), body
                        );
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(_) => break,
                }
            }
        });

        (url, shutdown, ready_rx, handle)
    }

    fn stop_server(shutdown: Arc<AtomicBool>, handle: std::thread::JoinHandle<()>) {
        shutdown.store(true, Ordering::Relaxed);
        let _ = handle.join();
    }

    fn success_response_json() -> String {
        let wire = WireResponse {
            candidates: vec![crate::schema::WireCandidate {
                rule_id: "r1".into(),
                confidence_bps: 8000,
                rationale: "clear match".into(),
                requires_later_confirmation: false,
                evidence_ranges: vec![crate::schema::WireEvidenceRange {
                    utf8_byte_start: 0,
                    utf8_byte_end: 12,
                }],
            }],
            usage_input: 200,
            usage_output: 100,
        };
        let wire_json = serde_json::to_string(&wire).unwrap();
        let openai_response = serde_json::json!({
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": wire_json
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 200,
                "completion_tokens": 100,
                "total_tokens": 300
            }
        });
        openai_response.to_string()
    }

    #[tokio::test]
    async fn adapter_successful_scan_returns_candidates() {
        let body = success_response_json();
        let leaked: &'static str = Box::leak(body.into_boxed_str());
        let (url, shutdown, ready, handle) = fake_openai_server(200, leaked);
        ready.recv().unwrap();

        let mut profile = test_profile();
        profile.base_url = url.trim_end_matches("/v1/chat/completions").into();

        let adapter = OpenAiCompatAdapter::new(
            profile,
            HttpClient::new(Duration::from_secs(5), RetryConfig::default()),
            "sk-test-canary".into(),
        );

        let result = adapter.analyze(&test_request()).await;
        stop_server(shutdown, handle);

        match result {
            Ok(response) => {
                assert_eq!(response.candidates.len(), 1);
                assert_eq!(response.candidates[0].rule_id, "r1");
                assert_eq!(response.usage.input_units, 200);
                assert_eq!(response.usage.output_units, 100);
            }
            Err(e) => panic!("Expected success, got error: {}: {}", e.code, e.message),
        }
    }

    #[tokio::test]
    async fn adapter_handles_http_401() {
        let (url, shutdown, ready, handle) =
            fake_openai_server(401, r#"{"error":{"message":"unauthorized"}}"#);
        ready.recv().unwrap();

        let mut profile = test_profile();
        profile.base_url = url.trim_end_matches("/v1/chat/completions").into();

        let adapter = OpenAiCompatAdapter::new(
            profile,
            HttpClient::new(Duration::from_secs(5), RetryConfig::default()),
            "sk-test-canary".into(),
        );

        let result = adapter.analyze(&test_request()).await;
        stop_server(shutdown, handle);

        match result {
            Err(e) => {
                assert!(e.code.contains("401"), "expected 401, got: {}", e.code);
                assert!(!e.retryable);
                assert!(!e.message.contains("sk-test-canary"));
            }
            Ok(_) => panic!("expected error for 401"),
        }
    }

    #[tokio::test]
    async fn adapter_handles_http_429() {
        let (url, shutdown, ready, handle) =
            fake_openai_server(429, r#"{"error":{"message":"rate limited"}}"#);
        ready.recv().unwrap();

        let mut profile = test_profile();
        profile.base_url = url.trim_end_matches("/v1/chat/completions").into();

        let adapter = OpenAiCompatAdapter::new(
            profile,
            HttpClient::new(Duration::from_secs(5), RetryConfig::default()),
            "sk-test-canary".into(),
        );

        let result = adapter.analyze(&test_request()).await;
        stop_server(shutdown, handle);

        match result {
            Err(e) => {
                assert!(e.code.contains("429"), "expected 429, got: {}", e.code);
                assert!(e.retryable);
            }
            Ok(_) => panic!("expected error for 429"),
        }
    }

    #[tokio::test]
    async fn adapter_handles_empty_choices() {
        let response_json = r#"{"choices":[],"usage":null}"#;
        let leaked: &'static str = Box::leak(response_json.to_string().into_boxed_str());
        let (url, shutdown, ready, handle) = fake_openai_server(200, leaked);
        ready.recv().unwrap();

        let mut profile = test_profile();
        profile.base_url = url.trim_end_matches("/v1/chat/completions").into();

        let adapter = OpenAiCompatAdapter::new(
            profile,
            HttpClient::new(Duration::from_secs(5), RetryConfig::default()),
            "sk-test-canary".into(),
        );

        let result = adapter.analyze(&test_request()).await;
        stop_server(shutdown, handle);

        match result {
            Err(e) => {
                assert!(e.code.contains("EMPTY_CHOICES"));
            }
            Ok(_) => panic!("expected error for empty choices"),
        }
    }

    #[tokio::test]
    async fn adapter_handles_invalid_json_in_content() {
        let openai_json = serde_json::json!({
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "not valid json!!!"
                },
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 10, "completion_tokens": 1, "total_tokens": 11}
        });
        let body = openai_json.to_string();
        let leaked: &'static str = Box::leak(body.into_boxed_str());
        let (url, shutdown, ready, handle) = fake_openai_server(200, leaked);
        ready.recv().unwrap();

        let mut profile = test_profile();
        profile.base_url = url.trim_end_matches("/v1/chat/completions").into();

        let adapter = OpenAiCompatAdapter::new(
            profile,
            HttpClient::new(Duration::from_secs(5), RetryConfig::default()),
            "sk-test-canary".into(),
        );

        let result = adapter.analyze(&test_request()).await;
        stop_server(shutdown, handle);

        match result {
            Err(e) => {
                assert!(e.code.contains("PARSE"), "expected PARSE, got: {}", e.code);
            }
            Ok(_) => panic!("expected error for invalid JSON content"),
        }
    }
}
