//! Anthropic native Messages API adapter.
//!
//! Maps the Anthropic Messages API to the `ModelProvider` trait.
//! This is a distinct protocol from OpenAI Chat Completions — different
//! headers, request shape, and response parsing.
//!
//! # Protocol differences from OpenAI
//!
//! | Feature         | Anthropic Messages          | OpenAI Chat Completions   |
//! |-----------------|-----------------------------|---------------------------|
//! | Auth header     | `x-api-key`                 | `Authorization: Bearer`   |
//! | Version header  | `anthropic-version`         | not required              |
//! | System prompt   | top-level `system` field    | `messages[0] role=system` |
//! | Response content| `content` array of blocks   | `choices[0].message`      |
//! | Usage fields    | `input_tokens`/`output_tokens` | `prompt_tokens`/`completion_tokens` |
//! | Stop reason     | `end_turn`/`max_tokens`/etc | `stop`/`length`/etc       |
//]
//! Anthropic API docs: https://docs.anthropic.com/en/api/messages (accessed 2026-07)

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

/// Required Anthropic API version header value.
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Anthropic native Messages API adapter.
///
/// Uses the same `HttpClient` and `RedactedSecret` infrastructure as the
/// OpenAI adapter, but with a completely different wire mapping.
pub struct AnthropicAdapter {
    profile: ProviderProfile,
    http_client: HttpClient,
    api_key: RedactedSecret,
}

impl AnthropicAdapter {
    pub fn new(profile: ProviderProfile, http_client: HttpClient, api_key: String) -> Self {
        Self {
            profile,
            http_client,
            api_key: RedactedSecret::new(api_key),
        }
    }

    fn endpoint_url(&self) -> String {
        let base = self.profile.base_url.trim_end_matches('/');
        format!("{base}/messages")
    }
}

impl ModelProvider for AnthropicAdapter {
    fn provider_id(&self) -> &str {
        &self.profile.id
    }

    fn model_id(&self) -> &str {
        &self.profile.model_id
    }

    fn analyze<'a>(&'a self, request: &'a InferenceRequest) -> ProviderFuture<'a> {
        Box::pin(async move {
            let body = build_anthropic_request(&self.profile.model_id, request);
            let url = self.endpoint_url();

            let response_bytes = self
                .http_client
                .post_json_with_headers(
                    &url,
                    &[
                        ("x-api-key", self.api_key.expose()),
                        ("anthropic-version", ANTHROPIC_VERSION),
                    ],
                    &body,
                )
                .await?;

            let anthropic_response: AnthropicMessageResponse =
                serde_json::from_slice(&response_bytes).map_err(|e| {
                    ProviderError::new(
                        "PARSE",
                        format!("invalid Anthropic response JSON: {e}"),
                        false,
                    )
                })?;

            parse_anthropic_response(&anthropic_response)
        })
    }
}

// ── Anthropic API types ────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
struct AnthropicMessageResponse {
    #[serde(default)]
    content: Vec<AnthropicContentBlock>,
    #[serde(default)]
    stop_reason: Option<String>,
    #[serde(default)]
    usage: Option<AnthropicUsage>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
enum AnthropicContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    #[allow(dead_code)]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(other)]
    #[allow(dead_code)]
    Unknown,
}

#[derive(Debug, Clone, Deserialize)]
struct AnthropicUsage {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
}

// ── Request building ──────────────────────────────────────────

fn build_anthropic_request(model: &str, request: &InferenceRequest) -> serde_json::Value {
    let system_prompt = build_system_prompt(&request.rules);
    let user_message = build_user_message(request);

    serde_json::json!({
        "model": model,
        "max_tokens": 4096,
        "system": system_prompt,
        "messages": [
            {"role": "user", "content": user_message}
        ]
    })
}

// ── Response parsing ──────────────────────────────────────────

fn parse_anthropic_response(
    response: &AnthropicMessageResponse,
) -> Result<ProviderResponse, ProviderError> {
    // Check stop_reason for problems
    if let Some(ref reason) = response.stop_reason {
        match reason.as_str() {
            "max_tokens" => {
                return Err(ProviderError::new(
                    "TRUNCATED",
                    "Anthropic response truncated (stop_reason=max_tokens); increase max_tokens",
                    false,
                ));
            }
            "refusal" => {
                return Err(ProviderError::new(
                    "CONTENT_FILTER",
                    "Anthropic refused the request",
                    false,
                ));
            }
            _ => {} // "end_turn", "stop_sequence", "tool_use" are OK
        }
    }

    // Extract text from content blocks
    let text_blocks: Vec<&str> = response
        .content
        .iter()
        .filter_map(|block| match block {
            AnthropicContentBlock::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect();

    if text_blocks.is_empty() {
        return Err(ProviderError::new(
            "EMPTY_CONTENT",
            "Anthropic returned no text content blocks",
            false,
        ));
    }

    // Use the first text block (should contain our WireResponse JSON)
    let content = text_blocks[0];

    // Parse content as WireResponse JSON
    let wire: WireResponse = serde_json::from_str(content).map_err(|e| {
        ProviderError::new(
            "PARSE",
            format!("invalid WireResponse JSON in Anthropic content: {e}"),
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
            input_units: u.input_tokens.unwrap_or(0),
            output_units: u.output_tokens.unwrap_or(0),
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
    use crate::config::{ProviderProfile, ProviderProtocol};
    use crate::http::HttpClient;
    use crate::retry::RetryConfig;
    use novel_core::{Chapter, ContextSnapshot, SourceLocator};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    fn test_profile() -> ProviderProfile {
        ProviderProfile {
            id: "test-anthropic".into(),
            display_name: "Test Anthropic".into(),
            protocol: ProviderProtocol::AnthropicNative,
            base_url: "https://api.anthropic.com/v1".into(),
            model_id: "claude-sonnet-4-6".into(),
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
            "测试正文",
            SourceLocator::Unknown {
                description: "t".into(),
            },
        );
        InferenceRequest {
            task_id: "t1".into(),
            document_id: "d1".into(),
            chapter,
            rules: vec![],
            context: ContextSnapshot::default(),
        }
    }

    // ── Unit tests ──────────────────────────────────────────

    #[test]
    fn endpoint_url_appends_messages() {
        let adapter = AnthropicAdapter::new(
            test_profile(),
            HttpClient::new(Duration::from_secs(5), RetryConfig::default()),
            "sk-ant-test".into(),
        );
        assert_eq!(
            adapter.endpoint_url(),
            "https://api.anthropic.com/v1/messages"
        );
    }

    #[test]
    fn endpoint_url_handles_trailing_slash() {
        let mut profile = test_profile();
        profile.base_url = "https://api.anthropic.com/v1/".into();
        let adapter = AnthropicAdapter::new(
            profile,
            HttpClient::new(Duration::from_secs(5), RetryConfig::default()),
            "sk-ant-test".into(),
        );
        assert_eq!(
            adapter.endpoint_url(),
            "https://api.anthropic.com/v1/messages"
        );
    }

    #[test]
    fn request_has_system_field_not_system_message() {
        let body = build_anthropic_request("claude-sonnet-4-6", &test_request());
        // System prompt is at top level, not in messages array
        assert!(body["system"].is_string());
        assert!(!body["system"].as_str().unwrap().is_empty());
        // Messages should only have user role
        let messages = body["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0]["role"], "user");
    }

    #[test]
    fn request_has_max_tokens() {
        let body = build_anthropic_request("claude-sonnet-4-6", &test_request());
        assert_eq!(body["max_tokens"], 4096);
    }

    #[test]
    fn request_has_model() {
        let body = build_anthropic_request("claude-sonnet-4-6", &test_request());
        assert_eq!(body["model"], "claude-sonnet-4-6");
    }

    #[test]
    fn parse_refusal_stop_reason_returns_error() {
        let response = AnthropicMessageResponse {
            content: vec![],
            stop_reason: Some("refusal".into()),
            usage: None,
        };
        let result = parse_anthropic_response(&response);
        match result {
            Err(e) => assert!(e.code.contains("CONTENT_FILTER")),
            Ok(_) => panic!("expected error for refusal"),
        }
    }

    #[test]
    fn parse_max_tokens_stop_reason_returns_error() {
        let response = AnthropicMessageResponse {
            content: vec![],
            stop_reason: Some("max_tokens".into()),
            usage: None,
        };
        let result = parse_anthropic_response(&response);
        match result {
            Err(e) => assert!(e.code.contains("TRUNCATED")),
            Ok(_) => panic!("expected error for max_tokens"),
        }
    }

    #[test]
    fn parse_empty_content_returns_error() {
        let response = AnthropicMessageResponse {
            content: vec![],
            stop_reason: Some("end_turn".into()),
            usage: None,
        };
        let result = parse_anthropic_response(&response);
        match result {
            Err(e) => assert!(e.code.contains("EMPTY_CONTENT")),
            Ok(_) => panic!("expected error for empty content"),
        }
    }

    #[test]
    fn parse_successful_response() {
        let wire = WireResponse {
            candidates: vec![crate::schema::WireCandidate {
                rule_id: "r1".into(),
                confidence_bps: 7000,
                rationale: "anthropic match".into(),
                requires_later_confirmation: false,
                evidence_ranges: vec![crate::schema::WireEvidenceRange {
                    utf8_byte_start: 0,
                    utf8_byte_end: 8,
                }],
            }],
            usage_input: 200,
            usage_output: 100,
        };
        let wire_json = serde_json::to_string(&wire).unwrap();

        let response = AnthropicMessageResponse {
            content: vec![AnthropicContentBlock::Text { text: wire_json }],
            stop_reason: Some("end_turn".into()),
            usage: Some(AnthropicUsage {
                input_tokens: Some(200),
                output_tokens: Some(100),
            }),
        };

        let result = parse_anthropic_response(&response).unwrap();
        assert_eq!(result.candidates.len(), 1);
        assert_eq!(result.candidates[0].rule_id, "r1");
        assert_eq!(result.usage.input_units, 200);
        assert_eq!(result.usage.output_units, 100);
    }

    #[test]
    fn parse_missing_usage_is_zero() {
        let wire = WireResponse {
            candidates: vec![],
            usage_input: 0,
            usage_output: 0,
        };
        let wire_json = serde_json::to_string(&wire).unwrap();

        let response = AnthropicMessageResponse {
            content: vec![AnthropicContentBlock::Text { text: wire_json }],
            stop_reason: Some("end_turn".into()),
            usage: None,
        };

        let result = parse_anthropic_response(&response).unwrap();
        assert_eq!(result.usage.input_units, 0);
        assert_eq!(result.usage.output_units, 0);
    }

    #[test]
    fn parse_invalid_json_content_returns_error() {
        let response = AnthropicMessageResponse {
            content: vec![AnthropicContentBlock::Text {
                text: "not valid json!!!".into(),
            }],
            stop_reason: Some("end_turn".into()),
            usage: None,
        };
        let result = parse_anthropic_response(&response);
        match result {
            Err(e) => assert!(e.code.contains("PARSE")),
            Ok(_) => panic!("expected PARSE error"),
        }
    }

    #[test]
    fn adapter_identity_matches_profile() {
        let adapter = AnthropicAdapter::new(
            test_profile(),
            HttpClient::new(Duration::from_secs(5), RetryConfig::default()),
            "sk-ant-test".into(),
        );
        assert_eq!(adapter.provider_id(), "test-anthropic");
        assert_eq!(adapter.model_id(), "claude-sonnet-4-6");
    }

    // ── Integration tests (fake server) ────────────────────

    fn fake_anthropic_server(
        status: u16,
        response_body: String,
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
        let url = format!("http://{}:{}/v1/messages", addr.ip(), addr.port());
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_clone = shutdown.clone();
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
                            status, response_body.len(), response_body
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

    fn success_response() -> String {
        let wire = WireResponse {
            candidates: vec![crate::schema::WireCandidate {
                rule_id: "r1".into(),
                confidence_bps: 8000,
                rationale: "anthropic test match".into(),
                requires_later_confirmation: false,
                evidence_ranges: vec![crate::schema::WireEvidenceRange {
                    utf8_byte_start: 0,
                    utf8_byte_end: 6,
                }],
            }],
            usage_input: 100,
            usage_output: 50,
        };
        let wire_json = serde_json::to_string(&wire).unwrap();
        serde_json::json!({
            "id": "msg_test",
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet-4-6",
            "content": [{"type": "text", "text": wire_json}],
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 100, "output_tokens": 50}
        })
        .to_string()
    }

    #[tokio::test]
    async fn anthropic_adapter_successful_scan() {
        let body = success_response();
        let (url, shutdown, ready, handle) = fake_anthropic_server(200, body);
        ready.recv().unwrap();

        let mut profile = test_profile();
        profile.base_url = url.trim_end_matches("/v1/messages").into();

        let adapter = AnthropicAdapter::new(
            profile,
            HttpClient::new(Duration::from_secs(5), RetryConfig::default()),
            "sk-ant-canary".into(),
        );
        let result = adapter.analyze(&test_request()).await;
        stop_server(shutdown, handle);

        let response = result.expect("Anthropic scan should succeed");
        assert_eq!(response.candidates.len(), 1);
        assert_eq!(response.candidates[0].rule_id, "r1");
        assert_eq!(response.usage.input_units, 100);
        assert_eq!(response.usage.output_units, 50);
    }

    #[tokio::test]
    async fn anthropic_adapter_handles_401() {
        let body = r#"{"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key"}}"#.to_string();
        let (url, shutdown, ready, handle) = fake_anthropic_server(401, body);
        ready.recv().unwrap();

        let mut profile = test_profile();
        profile.base_url = url.trim_end_matches("/v1/messages").into();

        let adapter = AnthropicAdapter::new(
            profile,
            HttpClient::new(Duration::from_secs(5), RetryConfig::default()),
            "sk-ant-canary".into(),
        );
        let result = adapter.analyze(&test_request()).await;
        stop_server(shutdown, handle);

        match result {
            Err(e) => {
                assert!(e.code.contains("401"));
                assert!(!e.retryable);
                assert!(!e.message.contains("sk-ant-canary"));
            }
            Ok(_) => panic!("expected 401 error"),
        }
    }

    #[tokio::test]
    async fn anthropic_adapter_handles_429() {
        let body =
            r#"{"type":"error","error":{"type":"rate_limit_error","message":"rate limited"}}"#
                .to_string();
        let (url, shutdown, ready, handle) = fake_anthropic_server(429, body);
        ready.recv().unwrap();

        let mut profile = test_profile();
        profile.base_url = url.trim_end_matches("/v1/messages").into();

        let adapter = AnthropicAdapter::new(
            profile,
            HttpClient::new(Duration::from_secs(5), RetryConfig::default()),
            "sk-ant-canary".into(),
        );
        let result = adapter.analyze(&test_request()).await;
        stop_server(shutdown, handle);

        match result {
            Err(e) => {
                assert!(e.code.contains("429"));
                assert!(e.retryable);
            }
            Ok(_) => panic!("expected 429 error"),
        }
    }

    #[tokio::test]
    async fn anthropic_adapter_handles_500() {
        let body =
            r#"{"type":"error","error":{"type":"server_error","message":"internal"}}"#.to_string();
        let (url, shutdown, ready, handle) = fake_anthropic_server(500, body);
        ready.recv().unwrap();

        let mut profile = test_profile();
        profile.base_url = url.trim_end_matches("/v1/messages").into();

        let adapter = AnthropicAdapter::new(
            profile,
            HttpClient::new(Duration::from_secs(5), RetryConfig::default()),
            "sk-ant-canary".into(),
        );
        let result = adapter.analyze(&test_request()).await;
        stop_server(shutdown, handle);

        match result {
            Err(e) => {
                assert!(e.code.contains("500"));
                assert!(e.retryable);
            }
            Ok(_) => panic!("expected 500 error"),
        }
    }

    #[tokio::test]
    async fn anthropic_error_never_leaks_api_key() {
        let body = r#"{"type":"error","error":{"type":"authentication_error","message":"invalid key: sk-ant-canary"}}"#.to_string();
        let (url, shutdown, ready, handle) = fake_anthropic_server(401, body);
        ready.recv().unwrap();

        let mut profile = test_profile();
        profile.base_url = url.trim_end_matches("/v1/messages").into();

        let adapter = AnthropicAdapter::new(
            profile,
            HttpClient::new(Duration::from_secs(5), RetryConfig::default()),
            "sk-ant-canary".into(),
        );
        let result = adapter.analyze(&test_request()).await;
        stop_server(shutdown, handle);

        match result {
            Err(e) => {
                let msg = format!("{}", e);
                assert!(
                    !msg.contains("sk-ant-canary"),
                    "API key leaked in error: {}",
                    msg
                );
            }
            Ok(_) => {}
        }
    }
}
