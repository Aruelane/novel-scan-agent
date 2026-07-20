//! HTTP transport implementation using the system HTTP stack.
//! Built on ureq for lightweight, sync HTTP without async runtime deps.

use std::time::Duration;

use novel_core::ProviderError;

use crate::retry::{
    backoff_delay, is_non_retryable_http_status, is_retryable_http_status, RetryConfig,
};
use crate::schema::MAX_RESPONSE_BYTES;

/// Real HTTP transport backed by the platform HTTP stack.
pub struct HttpClient {
    timeout: Duration,
    retry_config: RetryConfig,
}

impl HttpClient {
    pub fn new(timeout: Duration, retry_config: RetryConfig) -> Self {
        Self {
            timeout,
            retry_config,
        }
    }

    /// POST JSON to `url` with `Bearer` auth, returning the response body bytes.
    /// Retries on retryable status codes up to `max_attempts`.
    pub fn post_json(
        &self,
        url: &str,
        bearer_token: &str,
        body: &serde_json::Value,
    ) -> Result<Vec<u8>, ProviderError> {
        let body_bytes = serde_json::to_vec(body).map_err(|e| {
            ProviderError::new(
                "SERIALIZE",
                format!("failed to serialize request: {e}"),
                false,
            )
        })?;

        let mut last_error = None;

        for attempt in 1..=self.retry_config.max_attempts {
            match self.try_request(url, bearer_token, &body_bytes) {
                Ok(bytes) => {
                    if bytes.len() > MAX_RESPONSE_BYTES {
                        return Err(ProviderError::new(
                            "RESPONSE_TOO_LARGE",
                            format!(
                                "response {} bytes exceeds limit {}",
                                bytes.len(),
                                MAX_RESPONSE_BYTES
                            ),
                            false,
                        ));
                    }
                    return Ok(bytes);
                }
                Err(e) => {
                    if !e.retryable || attempt == self.retry_config.max_attempts {
                        return Err(e);
                    }
                    last_error = Some(e);
                    let delay = backoff_delay(&self.retry_config, attempt);
                    std::thread::sleep(delay);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            ProviderError::new("HTTP", "request failed after all retries".into(), false)
        }))
    }

    fn try_request(
        &self,
        url: &str,
        bearer_token: &str,
        body: &[u8],
    ) -> Result<Vec<u8>, ProviderError> {
        let response = ureq::post(url)
            .set("Authorization", &format!("Bearer {bearer_token}"))
            .set("Content-Type", "application/json")
            .timeout(self.timeout)
            .send_bytes(body)
            .map_err(|e| {
                let retryable = matches!(&e, ureq::Error::Transport(_));
                ProviderError::new("HTTP", format!("request to {url} failed: {e}"), retryable)
            })?;

        let status = response.status();
        match status {
            200..=299 => {
                let mut bytes = Vec::new();
                response
                    .into_reader()
                    .read_to_end(&mut bytes)
                    .map_err(|e| {
                        ProviderError::new("HTTP", format!("failed to read response: {e}"), false)
                    })?;
                Ok(bytes)
            }
            code if is_non_retryable_http_status(code) => Err(ProviderError::new(
                "HTTP",
                format!("HTTP {code}: authentication or authorization error"),
                false,
            )),
            code if is_retryable_http_status(code) => Err(ProviderError::new(
                "HTTP",
                format!("HTTP {code}: retryable server error"),
                true,
            )),
            code => Err(ProviderError::new(
                "HTTP",
                format!("HTTP {code}: unexpected status"),
                false,
            )),
        }
    }
}
