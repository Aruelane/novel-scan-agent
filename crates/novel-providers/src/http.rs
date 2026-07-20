//! HTTP transport stub. Real HTTP will be wired in via Tauri commands.
//! The `HttpTransport` trait in openai_compat.rs defines the contract.

use crate::retry::RetryConfig;
use novel_core::ProviderError;
use std::time::Duration;

/// Stub HTTP client. Returns `ProviderError` for all requests until the Tauri
/// command bridge is implemented (S4-14).
pub struct HttpClient {
    #[allow(dead_code)]
    timeout: Duration,
    #[allow(dead_code)]
    retry_config: RetryConfig,
}

impl HttpClient {
    pub fn new(timeout: Duration, retry_config: RetryConfig) -> Self {
        Self {
            timeout,
            retry_config,
        }
    }

    pub fn post_json(
        &self,
        _url: &str,
        _bearer_token: &str,
        _body: &serde_json::Value,
    ) -> Result<Vec<u8>, ProviderError> {
        Err(ProviderError::new(
            "NOT_IMPLEMENTED",
            String::from("HTTP transport not yet implemented; will be wired via Tauri command bridge in S4-14"),
            false,
        ))
    }
}
