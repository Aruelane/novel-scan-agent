//! Secret redaction and sanitization.
//!
//! All sensitive values (API keys, bearer tokens) are wrapped in `RedactedSecret`,
//! which prevents accidental exposure through Debug, Display, or Serialize.
//! Sanitization functions strip secrets from error messages before they leave
//! the HTTP boundary.

use zeroize::Zeroize;

/// A sensitive value (API key, token, password) that:
/// - Cannot be serialized (no `Serialize` impl)
/// - Only shows `[REDACTED]` in Debug/Display output
/// - Zeroizes its memory on drop
///
/// The inner value is only accessible via `expose()`, which should only be
/// called at the last layer (constructing the HTTP Authorization header).
#[derive(Clone)]
pub struct RedactedSecret(String);

impl RedactedSecret {
    /// Wrap a secret value. The input is moved into the wrapper and will be
    /// zeroized on drop.
    pub fn new(secret: String) -> Self {
        Self(secret)
    }

    /// Expose the raw secret. ONLY call this at the HTTP request layer when
    /// constructing the Authorization header. Never log, serialize, or store
    /// the returned value.
    pub(crate) fn expose(&self) -> &str {
        &self.0
    }

    /// Returns true if the secret is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl std::fmt::Debug for RedactedSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RedactedSecret([REDACTED])")
    }
}

impl std::fmt::Display for RedactedSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl Drop for RedactedSecret {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

// Explicitly NOT implementing Serialize for RedactedSecret.
// The inner value must never appear in JSON, logs, or wire format.

/// Replace every occurrence of `secret` in `text` with `[REDACTED]`.
/// Returns the original string unchanged if the secret is empty.
pub fn sanitize_string(text: &str, secret: &str) -> String {
    if secret.is_empty() {
        return text.to_string();
    }
    text.replace(secret, "[REDACTED]")
}

/// Truncate a string to `max_len` bytes, appending "..." if truncated.
/// Uses char boundaries to avoid splitting multi-byte UTF-8 characters.
pub fn truncate_utf8(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        return s;
    }
    let mut end = max_len;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// Sanitize a response body string by removing the secret and truncating.
pub fn sanitize_body(body: &str, secret: &str, max_len: usize) -> String {
    let sanitized = sanitize_string(body, secret);
    if sanitized.len() <= max_len {
        sanitized
    } else {
        format!("{}...", truncate_utf8(&sanitized, max_len))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacted_secret_shows_redacted_in_debug() {
        let secret = RedactedSecret::new("sk-test-12345".into());
        let debug_str = format!("{:?}", secret);
        assert!(debug_str.contains("[REDACTED]"));
        assert!(!debug_str.contains("sk-test"));
    }

    #[test]
    fn redacted_secret_shows_redacted_in_display() {
        let secret = RedactedSecret::new("sk-test-12345".into());
        let display_str = format!("{}", secret);
        assert_eq!(display_str, "[REDACTED]");
    }

    #[test]
    fn expose_returns_original() {
        let secret = RedactedSecret::new("sk-test-12345".into());
        assert_eq!(secret.expose(), "sk-test-12345");
    }

    #[test]
    fn empty_secret_is_empty() {
        let secret = RedactedSecret::new(String::new());
        assert!(secret.is_empty());
    }

    #[test]
    fn sanitize_replaces_secret() {
        let result = sanitize_string("Error: key=sk-abc123 not found", "sk-abc123");
        assert!(!result.contains("sk-abc123"));
        assert!(result.contains("[REDACTED]"));
    }

    #[test]
    fn sanitize_empty_secret_is_noop() {
        let original = "Error: something went wrong";
        let result = sanitize_string(original, "");
        assert_eq!(result, original);
    }

    #[test]
    fn sanitize_no_match_returns_unchanged() {
        let original = "Error: connection refused";
        let result = sanitize_string(original, "sk-abc123");
        assert_eq!(result, original);
    }

    #[test]
    fn truncate_utf8_preserves_char_boundaries() {
        // "hello" = 5 bytes, all ASCII
        assert_eq!(truncate_utf8("hello", 3), "hel");
        // "héllo" = 6 bytes (é is 2 bytes: 0xC3 0xA9): h(1) é(2) l(1) l(1) o(1)
        // Truncating at byte 3 gives "hé" since byte 3 is a char boundary ('l')
        let s = "héllo";
        assert_eq!(s.len(), 6);
        let t = truncate_utf8(s, 3);
        assert_eq!(t, "hé"); // bytes 0-2 = h + é
    }

    #[test]
    fn truncate_no_shortening_if_under_limit() {
        assert_eq!(truncate_utf8("hello", 10), "hello");
    }

    #[test]
    fn sanitize_body_truncates_long_content() {
        let long = "a".repeat(200);
        let result = sanitize_body(&long, "no-match", 100);
        assert!(result.len() <= 103); // 100 + "..."
        assert!(result.ends_with("..."));
    }

    #[test]
    fn sanitize_body_redacts_and_truncates() {
        let body = format!("Response with key=sk-secret-123 and {}", "x".repeat(500));
        let result = sanitize_body(&body, "sk-secret-123", 200);
        assert!(!result.contains("sk-secret-123"));
        assert!(result.contains("[REDACTED]"));
    }
}
