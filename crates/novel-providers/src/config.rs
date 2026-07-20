use serde::{Deserialize, Serialize};

/// The wire protocol an adapter implements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderProtocol {
    OpenAiCompatible,
    AnthropicNative,
    DeterministicTest,
}

/// Non-secret provider configuration. API keys are never stored here;
/// only an opaque `secret-ref:` handle is kept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderProfile {
    pub id: String,
    pub display_name: String,
    pub protocol: ProviderProtocol,
    pub base_url: String,
    pub model_id: String,
    #[serde(default)]
    pub max_requests_per_minute: Option<u32>,
    #[serde(default)]
    pub timeout_seconds: u32,
    #[serde(default)]
    pub retry_max_attempts: u32,
    #[serde(default)]
    pub credential_ref: Option<String>,
}

impl ProviderProfile {
    pub fn validate_id(id: &str) -> bool {
        !id.is_empty()
            && id.len() <= 128
            && id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    }

    pub fn validate_url(url: &str) -> Result<(), String> {
        if url.is_empty() {
            return Err("URL must not be empty".into());
        }
        let scheme_end = url.find("://").unwrap_or(0);
        let scheme = &url[..scheme_end];
        if scheme != "https" && scheme != "http" {
            return Err("URL scheme must be https or http".into());
        }
        let rest = &url[scheme_end + 3..];
        if rest.contains('@') {
            return Err("URL must not contain userinfo".into());
        }
        if rest.contains('#') {
            return Err("URL must not contain fragment".into());
        }
        let host = rest.split('/').next().unwrap_or("");
        if host.is_empty() {
            return Err("URL must have a host".into());
        }
        Ok(())
    }
}

/// Template for a known provider that the user can instantiate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderTemplate {
    pub id: String,
    pub display_name: String,
    pub protocol: ProviderProtocol,
    pub default_base_url: String,
    pub description: String,
}

/// What a provider adapter can do. Read-only capability declaration.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCapabilities {
    pub supports_streaming: bool,
    pub supports_tool_calls: bool,
    pub max_context_chars: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_ids_accepted() {
        assert!(ProviderProfile::validate_id("openai-gpt-4"));
        assert!(ProviderProfile::validate_id("deepseek.v3"));
        assert!(ProviderProfile::validate_id("test_provider"));
    }

    #[test]
    fn empty_id_rejected() {
        assert!(!ProviderProfile::validate_id(""));
    }

    #[test]
    fn https_url_accepted() {
        assert!(ProviderProfile::validate_url("https://api.openai.com/v1").is_ok());
    }

    #[test]
    fn url_with_userinfo_rejected() {
        assert!(ProviderProfile::validate_url("https://user:pass@host.com").is_err());
    }

    #[test]
    fn profile_json_has_no_secret_fields() {
        let profile = ProviderProfile {
            id: "test".into(),
            display_name: "Test".into(),
            protocol: ProviderProtocol::OpenAiCompatible,
            base_url: "https://example.com".into(),
            model_id: "test-v1".into(),
            max_requests_per_minute: None,
            timeout_seconds: 30,
            retry_max_attempts: 3,
            credential_ref: Some("secret-ref:test".into()),
        };
        let json = serde_json::to_string(&profile).unwrap();
        assert!(!json.contains("api_key"));
        assert!(!json.contains("token"));
        // credential_ref contains the secret-ref: prefix — that's the opaque handle, not the secret
        assert!(!json.contains("sk-"));
        assert!(!json.contains("Bearer"));
    }
}
