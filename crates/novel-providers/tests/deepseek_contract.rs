//! DeepSeek contract tests — verify the DeepSeek template works correctly
//! through the generic OpenAI-compatible adapter without calling real APIs.
//!
//! DeepSeek uses the OpenAI Chat Completions wire format with these specifics:
//! - Base URL: `https://api.deepseek.com/v1`
//! - x-api-key header OR Bearer token (we use Bearer for OpenAI compatibility)
//! - Supports streaming but NOT tool_calls
//! - Model IDs are user-editable; template suggests but doesn't enforce
//!
//! All tests use loopback fake servers. No real API keys or network calls.

use novel_core::{
    AlertLevel, Chapter, ConfirmationScope, ContextSnapshot, DetectionMode, InferenceRequest,
    ModelProvider, RuleCategory, RuleContext, SourceLocator,
};
use novel_providers::config::{ProviderProfile, ProviderProtocol};
use novel_providers::http::HttpClient;
use novel_providers::openai_compat::OpenAiCompatAdapter;
use novel_providers::registry::{builtin_templates, default_capabilities};
use novel_providers::retry::RetryConfig;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

// ── Helpers ────────────────────────────────────────────────────

fn deepseek_profile() -> ProviderProfile {
    let template = builtin_templates()
        .into_iter()
        .find(|t| t.id == "deepseek")
        .expect("deepseek template must exist");

    ProviderProfile {
        id: "my-deepseek".into(),
        display_name: "My DeepSeek".into(),
        protocol: template.protocol,
        base_url: template.default_base_url,
        model_id: "deepseek-chat".into(),
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

/// Fake HTTP server that echoes back a successful DeepSeek-style response.
fn fake_server(
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
    // DeepSeek uses the same /v1/chat/completions path
    let url = format!("http://{}:{}/v1/chat/completions", addr.ip(), addr.port());
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

/// Build a successful DeepSeek-style response with candidates.
fn success_response() -> String {
    let wire = novel_providers::schema::WireResponse {
        candidates: vec![novel_providers::schema::WireCandidate {
            rule_id: "r1".into(),
            confidence_bps: 8000,
            rationale: "clear match in DeepSeek test".into(),
            requires_later_confirmation: false,
            evidence_ranges: vec![novel_providers::schema::WireEvidenceRange {
                utf8_byte_start: 0,
                utf8_byte_end: 12,
            }],
        }],
        usage_input: 150,
        usage_output: 80,
    };
    let wire_json = serde_json::to_string(&wire).unwrap();
    serde_json::json!({
        "id": "deepseek-chatcmpl-test",
        "object": "chat.completion",
        "created": 1700000000,
        "model": "deepseek-chat",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": wire_json
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": 150,
            "completion_tokens": 80,
            "total_tokens": 230
        }
    })
    .to_string()
}

fn empty_candidates_response() -> String {
    let wire = novel_providers::schema::WireResponse {
        candidates: vec![],
        usage_input: 50,
        usage_output: 10,
    };
    let wire_json = serde_json::to_string(&wire).unwrap();
    serde_json::json!({
        "id": "deepseek-chatcmpl-test",
        "object": "chat.completion",
        "model": "deepseek-chat",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": wire_json
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": 50,
            "completion_tokens": 10,
            "total_tokens": 60
        }
    })
    .to_string()
}

fn adapter_for_url(base_url: &str) -> OpenAiCompatAdapter {
    let mut profile = deepseek_profile();
    profile.base_url = base_url.into();
    OpenAiCompatAdapter::new(
        profile,
        HttpClient::new(Duration::from_secs(5), RetryConfig::default()),
        "sk-deepseek-canary".into(),
    )
}

// ── Template & capability tests ──────────────────────────────

#[test]
fn deepseek_template_exists_with_correct_defaults() {
    let templates = builtin_templates();
    let ds = templates
        .iter()
        .find(|t| t.id == "deepseek")
        .expect("deepseek template must exist");

    assert_eq!(ds.protocol, ProviderProtocol::DeepSeek);
    assert_eq!(ds.default_base_url, "https://api.deepseek.com/v1");
    assert!(ds.display_name.contains("DeepSeek"));
    assert!(ds.description.contains("OpenAI Chat Completions"));
}

#[test]
fn deepseek_capabilities_no_tool_calls() {
    let caps = default_capabilities(ProviderProtocol::DeepSeek);
    assert!(caps.supports_streaming);
    assert!(
        !caps.supports_tool_calls,
        "DeepSeek does not support tool_calls"
    );
    assert_eq!(caps.max_context_chars, Some(128_000));
}

#[test]
fn deepseek_template_is_production_visible() {
    let prod = novel_providers::registry::production_templates_owned();
    assert!(prod.iter().any(|t| t.id == "deepseek"));
}

#[test]
fn deepseek_profile_constructs_from_template() {
    let profile = deepseek_profile();
    assert_eq!(profile.protocol, ProviderProtocol::DeepSeek);
    assert!(ProviderProfile::validate_id(&profile.id));
    assert!(ProviderProfile::validate_url(&profile.base_url).is_ok());
    // DeepSeek default model is user-editable; we set a suggestion
    assert_eq!(profile.model_id, "deepseek-chat");
}

#[test]
fn deepseek_profile_has_no_hardcoded_api_key() {
    let profile = deepseek_profile();
    let json = serde_json::to_string(&profile).unwrap();
    assert!(!json.contains("sk-"));
    assert!(!json.contains("Bearer"));
    assert!(!json.contains("api_key"));
}

// ── Adapter integration tests (fake server) ──────────────────

#[tokio::test]
async fn deepseek_adapter_successful_scan() {
    let body = success_response();
    let (url, shutdown, ready, handle) = fake_server(200, body);
    ready.recv().unwrap();

    let base = url.trim_end_matches("/v1/chat/completions");
    let adapter = adapter_for_url(base);
    let result = adapter.analyze(&test_request()).await;
    stop_server(shutdown, handle);

    let response = result.expect("DeepSeek scan should succeed");
    assert_eq!(response.candidates.len(), 1);
    assert_eq!(response.candidates[0].rule_id, "r1");
    assert_eq!(response.usage.input_units, 150);
    assert_eq!(response.usage.output_units, 80);
}

#[tokio::test]
async fn deepseek_adapter_empty_candidates_is_valid() {
    let body = empty_candidates_response();
    let (url, shutdown, ready, handle) = fake_server(200, body);
    ready.recv().unwrap();

    let base = url.trim_end_matches("/v1/chat/completions");
    let adapter = adapter_for_url(base);
    let result = adapter.analyze(&test_request()).await;
    stop_server(shutdown, handle);

    let response = result.expect("empty candidates should be valid");
    assert_eq!(response.candidates.len(), 0);
}

#[tokio::test]
async fn deepseek_adapter_handles_401() {
    let body = r#"{"error":{"message":"Authentication Fails"}}"#.to_string();
    let (url, shutdown, ready, handle) = fake_server(401, body);
    ready.recv().unwrap();

    let base = url.trim_end_matches("/v1/chat/completions");
    let adapter = adapter_for_url(base);
    let result = adapter.analyze(&test_request()).await;
    stop_server(shutdown, handle);

    match result {
        Err(e) => {
            assert!(e.code.contains("401"), "expected 401, got: {}", e.code);
            assert!(!e.retryable);
            assert!(!e.message.contains("sk-deepseek-canary"));
        }
        Ok(_) => panic!("expected 401 error"),
    }
}

#[tokio::test]
async fn deepseek_adapter_handles_429() {
    let body = r#"{"error":{"message":"Rate limit"}}"#.to_string();
    let (url, shutdown, ready, handle) = fake_server(429, body);
    ready.recv().unwrap();

    let base = url.trim_end_matches("/v1/chat/completions");
    let adapter = adapter_for_url(base);
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
async fn deepseek_adapter_handles_invalid_json_in_content() {
    let body = serde_json::json!({
        "id": "deepseek-test",
        "object": "chat.completion",
        "model": "deepseek-chat",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": "not valid json!!!"
            },
            "finish_reason": "stop"
        }],
        "usage": {"prompt_tokens": 10, "completion_tokens": 1, "total_tokens": 11}
    })
    .to_string();

    let (url, shutdown, ready, handle) = fake_server(200, body);
    ready.recv().unwrap();

    let base = url.trim_end_matches("/v1/chat/completions");
    let adapter = adapter_for_url(base);
    let result = adapter.analyze(&test_request()).await;
    stop_server(shutdown, handle);

    match result {
        Err(e) => {
            assert!(e.code.contains("PARSE"), "expected PARSE, got: {}", e.code);
        }
        Ok(_) => panic!("expected PARSE error"),
    }
}

#[tokio::test]
async fn deepseek_adapter_handles_empty_choices() {
    let body = r#"{"id":"ds-test","object":"chat.completion","model":"deepseek-chat","choices":[],"usage":null}"#.to_string();
    let (url, shutdown, ready, handle) = fake_server(200, body);
    ready.recv().unwrap();

    let base = url.trim_end_matches("/v1/chat/completions");
    let adapter = adapter_for_url(base);
    let result = adapter.analyze(&test_request()).await;
    stop_server(shutdown, handle);

    match result {
        Err(e) => {
            assert!(e.code.contains("EMPTY_CHOICES"));
        }
        Ok(_) => panic!("expected EMPTY_CHOICES error"),
    }
}

#[tokio::test]
async fn deepseek_adapter_handles_500() {
    let body = r#"{"error":{"message":"Internal error"}}"#.to_string();
    let (url, shutdown, ready, handle) = fake_server(500, body);
    ready.recv().unwrap();

    let base = url.trim_end_matches("/v1/chat/completions");
    let adapter = adapter_for_url(base);
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
async fn deepseek_adapter_provider_identity_stable() {
    let adapter = adapter_for_url("https://api.deepseek.com/v1");
    assert_eq!(adapter.provider_id(), "my-deepseek");
    assert_eq!(adapter.model_id(), "deepseek-chat");
}

#[tokio::test]
async fn deepseek_error_never_leaks_api_key() {
    // Intentionally bad response to trigger error that could echo request data
    let body = r#"{"error":{"message":"Invalid API key: sk-deepseek-canary"}}"#.to_string();
    let (url, shutdown, ready, handle) = fake_server(401, body);
    ready.recv().unwrap();

    let base = url.trim_end_matches("/v1/chat/completions");
    let adapter = adapter_for_url(base);
    let result = adapter.analyze(&test_request()).await;
    stop_server(shutdown, handle);

    match result {
        Err(e) => {
            let msg = format!("{}", e);
            assert!(
                !msg.contains("sk-deepseek-canary"),
                "API key leaked in error: {}",
                msg
            );
        }
        Ok(_) => {}
    }
}

#[tokio::test]
async fn deepseek_usage_missing_is_zero_not_guessed() {
    let wire = novel_providers::schema::WireResponse {
        candidates: vec![],
        usage_input: 0,
        usage_output: 0,
    };
    let wire_json = serde_json::to_string(&wire).unwrap();
    let body = serde_json::json!({
        "id": "ds-test",
        "object": "chat.completion",
        "model": "deepseek-chat",
        "choices": [{
            "index": 0,
            "message": { "role": "assistant", "content": wire_json },
            "finish_reason": "stop"
        }]
        // intentionally omit "usage" field
    })
    .to_string();

    let (url, shutdown, ready, handle) = fake_server(200, body);
    ready.recv().unwrap();

    let base = url.trim_end_matches("/v1/chat/completions");
    let adapter = adapter_for_url(base);
    let result = adapter.analyze(&test_request()).await;
    stop_server(shutdown, handle);

    let response = result.expect("missing usage should be valid");
    assert_eq!(response.usage.input_units, 0);
    assert_eq!(response.usage.output_units, 0);
}
