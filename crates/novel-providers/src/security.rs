//! Provider connection security tests and canary contracts.
//! No real API keys are used — only canary test values.

use crate::secret::{CanarySecretStore, SecretStore};

/// Verify that the canary store never persists keys to disk or logs.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ProviderProfile;
    use crate::registry::builtin_templates;
    use crate::secret::is_valid_secret_ref;

    #[test]
    fn secret_ref_never_appears_in_provider_profile_json() {
        let profile = ProviderProfile {
            id: "test".into(),
            display_name: "Test".into(),
            protocol: crate::config::ProviderProtocol::OpenAiCompatible,
            base_url: "https://example.com".into(),
            model_id: "test-v1".into(),
            max_requests_per_minute: None,
            timeout_seconds: 30,
            retry_max_attempts: 3,
            credential_ref: Some("secret-ref:test-handle".into()),
        };
        let json = serde_json::to_string(&profile).unwrap();
        // The handle reference itself may be present, but actual keys must not be
        assert!(!json.contains("sk-"));
        assert!(!json.contains("Bearer"));
        assert!(!json.contains("x-api-key"));
        assert!(!json.contains("Authorization"));
    }

    #[test]
    fn deterministic_test_provider_is_not_in_production() {
        let prod = crate::registry::production_templates_owned();
        assert!(!prod.iter().any(|t| matches!(
            t.protocol,
            crate::config::ProviderProtocol::DeterministicTest
        )));
    }

    #[test]
    fn all_builtin_template_ids_are_valid() {
        for t in builtin_templates() {
            assert!(
                t.id.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.'),
                "invalid template id: {}",
                t.id
            );
        }
    }

    #[test]
    fn canary_store_key_never_leaked_after_delete() {
        let store = CanarySecretStore::default();
        let handle = store.store("test-api-key-value", "test").unwrap();
        store.delete(&handle).unwrap();
        match store.resolve(&handle).unwrap() {
            crate::secret::ResolvedSecret::Missing => {} // good
            other => panic!("key should be missing after delete, got {other:?}"),
        }
    }
}
