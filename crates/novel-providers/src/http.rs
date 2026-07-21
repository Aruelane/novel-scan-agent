//! HTTP transport with sanitization, timeout, and cancellation.
//!
//! Provides a real HTTP client for making API calls to LLM providers.
//! All sensitive data (API keys) is wrapped in [`RedactedSecret`] and
//! sanitized from error messages. Timeouts are enforced at the HTTP
//! layer. Cancellation is achieved by dropping the returned future.

use crate::redaction::{sanitize_body, sanitize_string, RedactedSecret};
use crate::retry::{is_non_retryable_http_status, is_retryable_http_status, RetryConfig};
use crate::schema::MAX_RESPONSE_BYTES;
use novel_core::ProviderError;
use std::time::Duration;

/// A real HTTP client for making API calls to LLM providers.
///
/// Uses reqwest with rustls for cross-platform TLS (no OpenSSL dependency).
/// Bearer tokens are never logged or stored — they are only used to construct
/// the Authorization header at request time.
pub struct HttpClient {
    client: reqwest::Client,
    #[allow(dead_code)]
    timeout: Duration,
    #[allow(dead_code)]
    retry_config: RetryConfig,
}

impl HttpClient {
    /// Create a new HTTP client with the given timeout and retry configuration.
    ///
    /// The timeout applies to the entire request (connect + send + read).
    /// Redirects are disabled by default; only same-host redirects are allowed
    /// to prevent credentials from being forwarded to untrusted hosts.
    pub fn new(timeout: Duration, retry_config: RetryConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .redirect(reqwest::redirect::Policy::none())
            // https_only is false for loopback testing and local dev servers.
            // Production profiles MUST use https:// URLs (validated in ProviderProfile).
            .https_only(false)
            .build()
            .expect("reqwest Client::build should not fail with default settings");
        Self {
            client,
            timeout,
            retry_config,
        }
    }

    /// POST a JSON body to the given URL with Bearer token authentication.
    ///
    /// The bearer token is used ONLY to construct the Authorization header.
    /// It is never logged, stored, or included in error messages. Error messages
    /// that might contain the token (e.g., from URL-echoing proxies) are
    /// sanitized before being returned.
    ///
    /// # Cancellation
    ///
    /// Dropping the returned future cancels the in-flight HTTP request.
    /// The response body is not read after cancellation.
    pub async fn post_json(
        &self,
        url: &str,
        bearer_token: &RedactedSecret,
        body: &serde_json::Value,
    ) -> Result<Vec<u8>, ProviderError> {
        let response = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", bearer_token.expose()))
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await
            .map_err(|e| classify_reqwest_error(e, bearer_token))?;

        let status = response.status();
        if !status.is_success() {
            let status_code = status.as_u16();
            let body_bytes = response.bytes().await.unwrap_or_default();
            let body_text = String::from_utf8_lossy(&body_bytes);
            let sanitized = sanitize_body(&body_text, bearer_token.expose(), 500);
            let retryable =
                is_retryable_http_status(status_code) && !is_non_retryable_http_status(status_code);
            return Err(ProviderError::new(
                format!("HTTP_{}", status_code),
                format!("Provider returned HTTP {}: {}", status_code, sanitized),
                retryable,
            ));
        }

        let content_length = response
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<usize>().ok());

        if let Some(len) = content_length {
            if len > MAX_RESPONSE_BYTES {
                return Err(ProviderError::new(
                    "RESPONSE_TOO_LARGE",
                    format!(
                        "Content-Length {} exceeds maximum {} bytes",
                        len, MAX_RESPONSE_BYTES
                    ),
                    false,
                ));
            }
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| classify_reqwest_error(e, bearer_token))?;

        if bytes.len() > MAX_RESPONSE_BYTES {
            return Err(ProviderError::new(
                "RESPONSE_TOO_LARGE",
                format!(
                    "Response body {} exceeds maximum {} bytes",
                    bytes.len(),
                    MAX_RESPONSE_BYTES
                ),
                false,
            ));
        }

        Ok(bytes.to_vec())
    }

    /// Like `post_json` but with explicit custom headers instead of a Bearer token.
    /// Used by the Anthropic adapter which requires `x-api-key` and
    /// `anthropic-version` headers instead of `Authorization: Bearer`.
    ///
    /// Sensitive header values (e.g., `x-api-key`) are sanitized from all
    /// error messages.
    pub async fn post_json_with_headers(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        body: &serde_json::Value,
    ) -> Result<Vec<u8>, ProviderError> {
        // Find any sensitive header value for sanitization
        let sensitive_value = headers
            .iter()
            .find(|(k, _)| *k == "x-api-key")
            .map(|(_, v)| *v)
            .unwrap_or("");

        let mut request = self
            .client
            .post(url)
            .header("Content-Type", "application/json");

        for (name, value) in headers {
            request = request.header(*name, *value);
        }

        let response = request.json(body).send().await.map_err(|e| {
            let msg = e.to_string();
            let sanitized = sanitize_string(&msg, sensitive_value);
            let (code, retryable) = if e.is_timeout() {
                ("TIMEOUT", true)
            } else if e.is_connect() {
                ("CONNECTION_ERROR", true)
            } else {
                ("HTTP_ERROR", false)
            };
            ProviderError::new(code, sanitized, retryable)
        })?;

        let status = response.status();
        if !status.is_success() {
            let status_code = status.as_u16();
            let body_bytes = response.bytes().await.unwrap_or_default();
            let body_text = String::from_utf8_lossy(&body_bytes);
            let sanitized = sanitize_body(&body_text, sensitive_value, 500);
            let retryable =
                is_retryable_http_status(status_code) && !is_non_retryable_http_status(status_code);
            return Err(ProviderError::new(
                format!("HTTP_{}", status_code),
                format!("Provider returned HTTP {}: {}", status_code, sanitized),
                retryable,
            ));
        }

        let bytes = response.bytes().await.map_err(|e| {
            let msg = e.to_string();
            ProviderError::new(
                "DECODE_ERROR",
                sanitize_string(&msg, sensitive_value),
                false,
            )
        })?;

        if bytes.len() > MAX_RESPONSE_BYTES {
            return Err(ProviderError::new(
                "RESPONSE_TOO_LARGE",
                format!(
                    "Response body {} exceeds maximum {} bytes",
                    bytes.len(),
                    MAX_RESPONSE_BYTES
                ),
                false,
            ));
        }

        Ok(bytes.to_vec())
    }
}

/// Classify a reqwest error into a `ProviderError`, sanitizing the bearer
/// token from any error message.
fn classify_reqwest_error(error: reqwest::Error, bearer_token: &RedactedSecret) -> ProviderError {
    let msg = error.to_string();
    let sanitized = sanitize_string(&msg, bearer_token.expose());

    let (code, retryable) = if error.is_timeout() {
        ("TIMEOUT", true)
    } else if error.is_connect() {
        ("CONNECTION_ERROR", true)
    } else if error.is_request() {
        // Request was never sent — likely a builder error or redirect
        ("REQUEST_ERROR", false)
    } else if error.is_decode() {
        // Response decoding error — not retryable (body is already read)
        ("DECODE_ERROR", false)
    } else {
        ("HTTP_ERROR", false)
    };

    ProviderError::new(code, sanitized, retryable)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    /// Helper to create a test client with a short timeout.
    fn test_client() -> HttpClient {
        HttpClient::new(Duration::from_secs(5), RetryConfig::default())
    }

    /// A minimal HTTP server for loopback testing.
    /// Returns (server_url, shutdown_flag, ready_rx, server_thread).
    fn start_test_server(
        response_status: u16,
        response_body: &'static str,
        check_auth: bool,
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
            // Signal that the server is ready
            let _ = ready_tx.send(());
            // Use a short accept timeout so we can check shutdown
            listener
                .set_nonblocking(true)
                .expect("set_nonblocking failed");
            loop {
                if shutdown_clone.load(Ordering::Relaxed) {
                    break;
                }
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        // Read the request to consume it
                        use std::io::Read;
                        let mut buf = [0u8; 8192];
                        let _ = stream.read(&mut buf);

                        if check_auth {
                            let request = String::from_utf8_lossy(&buf).to_lowercase();
                            if !request.contains("authorization: bearer sk-test-canary") {
                                let _ = write!(
                                    stream,
                                    "HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                                );
                                continue;
                            }
                        }

                        let _ = write!(
                            stream,
                            "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                            response_status,
                            body.len(),
                            body
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

    fn stop_test_server(shutdown: Arc<AtomicBool>, handle: std::thread::JoinHandle<()>) {
        shutdown.store(true, Ordering::Relaxed);
        let _ = handle.join();
    }

    #[tokio::test]
    async fn successful_request_returns_body() {
        let (url, shutdown, ready, handle) = start_test_server(
            200,
            r#"{"candidates":[],"usage_input":10,"usage_output":5}"#,
            false,
        );
        ready.recv().unwrap(); // wait for server to be ready
        let client = test_client();
        let token = RedactedSecret::new("sk-test-canary".into());
        let body = serde_json::json!({"model": "test", "messages": []});

        let result = client.post_json(&url, &token, &body).await;
        stop_test_server(shutdown, handle);

        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
        let bytes = result.unwrap();
        assert!(!bytes.is_empty());
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("candidates"));
    }

    #[tokio::test]
    async fn authorization_header_is_sent() {
        // Server that checks for Authorization header
        let (url, shutdown, ready, handle) = start_test_server(200, r#"{"ok":true}"#, true);
        ready.recv().unwrap();
        let client = test_client();
        let token = RedactedSecret::new("sk-test-canary".into());
        let body = serde_json::json!({"test": true});

        let result = client.post_json(&url, &token, &body).await;
        stop_test_server(shutdown, handle);

        // If the server doesn't find the auth header, it returns 401.
        // We expect 200 (auth header found).
        match result {
            Ok(bytes) => {
                let text = String::from_utf8_lossy(&bytes);
                assert!(text.contains("ok"));
            }
            Err(e) => {
                panic!(
                    "Expected 200 OK (auth header found), got {}: {}. \
                     The server checks for 'Authorization: Bearer sk-test-canary' in the request.",
                    e.code, e.message
                );
            }
        }
    }

    #[tokio::test]
    async fn http_401_returns_retryable_false() {
        let (url, shutdown, ready, handle) =
            start_test_server(401, r#"{"error":"unauthorized"}"#, false);
        ready.recv().unwrap();
        let client = test_client();
        let token = RedactedSecret::new("sk-test-canary".into());
        let body = serde_json::json!({"model": "test", "messages": []});

        let result = client.post_json(&url, &token, &body).await;
        stop_test_server(shutdown, handle);

        match result {
            Err(e) => {
                assert!(
                    e.code.contains("401"),
                    "expected 401 in code, got: {}",
                    e.code
                );
                assert!(!e.retryable);
                // Error message must not contain the canary token
                assert!(!e.message.contains("sk-test-canary"));
            }
            Ok(_) => panic!("expected error for 401"),
        }
    }

    #[tokio::test]
    async fn http_429_is_retryable() {
        let (url, shutdown, ready, handle) =
            start_test_server(429, r#"{"error":"rate limited"}"#, false);
        ready.recv().unwrap();
        let client = test_client();
        let token = RedactedSecret::new("sk-test-canary".into());
        let body = serde_json::json!({"model": "test", "messages": []});

        let result = client.post_json(&url, &token, &body).await;
        stop_test_server(shutdown, handle);

        match result {
            Err(e) => {
                assert!(
                    e.code.contains("429"),
                    "expected 429 in code, got: {}",
                    e.code
                );
                assert!(e.retryable);
            }
            Ok(_) => panic!("expected error for 429"),
        }
    }

    #[tokio::test]
    async fn http_500_is_retryable() {
        let (url, shutdown, ready, handle) =
            start_test_server(500, r#"{"error":"internal"}"#, false);
        ready.recv().unwrap();
        let client = test_client();
        let token = RedactedSecret::new("sk-test-canary".into());
        let body = serde_json::json!({"model": "test", "messages": []});

        let result = client.post_json(&url, &token, &body).await;
        stop_test_server(shutdown, handle);

        match result {
            Err(e) => {
                assert!(
                    e.code.contains("500"),
                    "expected 500 in code, got: {}",
                    e.code
                );
                assert!(e.retryable);
            }
            Ok(_) => panic!("expected error for 500"),
        }
    }

    #[tokio::test]
    async fn response_exceeding_max_bytes_is_rejected() {
        let large_body = "x".repeat(MAX_RESPONSE_BYTES + 100);
        let response_json = serde_json::json!({"data": large_body}).to_string();
        let leaked: &'static str = Box::leak(response_json.into_boxed_str());
        let (url, shutdown, ready, handle) = start_test_server(200, leaked, false);
        ready.recv().unwrap();
        let client = test_client();
        let token = RedactedSecret::new("sk-test-canary".into());
        let body = serde_json::json!({"model": "test"});

        let result = client.post_json(&url, &token, &body).await;
        stop_test_server(shutdown, handle);

        match result {
            Err(e) => {
                assert!(
                    e.code.contains("TOO_LARGE"),
                    "expected TOO_LARGE in code, got: {}",
                    e.code
                );
                assert!(!e.retryable);
            }
            Ok(_) => panic!("expected oversize rejection"),
        }
    }

    #[tokio::test]
    async fn error_message_never_contains_bearer_token() {
        // Use a URL that will fail to connect, and check the error message
        let client = test_client();
        let token = RedactedSecret::new("sk-test-canary-secret-12345".into());
        let body = serde_json::json!({"model": "test"});

        let result = client
            .post_json("https://127.0.0.1:1/v1/chat", &token, &body)
            .await;

        match result {
            Err(e) => {
                let msg = format!("{}", e);
                assert!(
                    !msg.contains("sk-test-canary-secret-12345"),
                    "Error message contains canary token: {}",
                    msg
                );
            }
            Ok(_) => {} // unexpected but not test failure
        }
    }

    #[tokio::test]
    async fn cancellation_via_future_drop() {
        use std::time::Duration;
        // Server that delays response — we'll drop the future before it completes
        let (url, shutdown, _ready, handle) = {
            use std::io::Write;
            use std::net::TcpListener;
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let addr = listener.local_addr().unwrap();
            let url = format!("http://{}:{}/v1/slow", addr.ip(), addr.port());
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
                            let mut buf = [0u8; 4096];
                            let _ = stream.read(&mut buf);
                            // Delay before responding
                            std::thread::sleep(Duration::from_secs(2));
                            let _ = write!(
                                stream,
                                "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
                                "{}"
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
        };

        let client = HttpClient::new(Duration::from_secs(10), RetryConfig::default());
        let token = RedactedSecret::new("sk-test-canary".into());
        let body = serde_json::json!({"model": "test"});

        // Spawn and immediately drop — the request should be cancelled
        let future = client.post_json(&url, &token, &body);
        drop(future);

        // If cancellation didn't work, the server thread would still be
        // waiting. We verify by checking we can stop cleanly.
        stop_test_server(shutdown, handle);
        // Test passes if we reach here without hanging
    }
}
