//! Platform secret store abstraction.
//!
//! API keys are never stored in SQLite, Rust state, or configuration files.
//! They are resolved through platform-specific secret stores using opaque
//! `secret-ref:` handles. The core never sees key material.

/// Opaque reference to a secret stored in a platform secret manager.
/// Format: `secret-ref:<opaque-id>` where `<opaque-id>` matches
/// `[a-zA-Z0-9._-]+` with a length limit of 245 characters.
pub fn is_valid_secret_ref(s: &str) -> bool {
    if !s.starts_with("secret-ref:") {
        return false;
    }
    let suffix = &s[11..];
    if suffix.is_empty() || suffix.len() > 245 {
        return false;
    }
    suffix
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
}

/// What the platform secret store returns when resolving a `secret-ref:`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedSecret {
    /// A key was found and resolved. Never log or serialize this.
    Key(String),
    /// The handle is valid but no key is stored yet.
    Missing,
    /// The platform store is unavailable (e.g., no Credential Manager on
    /// a stripped-down system).
    Unavailable,
}

/// Platform-agnostic secret store contract. Implementations use Windows
/// Credential Manager or Android Keystore.
pub trait SecretStore: Send + Sync {
    /// Resolve a `secret-ref:` handle. Returns `None` if the handle is
    /// malformed (callers should validate with `is_valid_secret_ref` first).
    fn resolve(&self, handle: &str) -> Result<ResolvedSecret, SecretStoreError>;

    /// Store a secret and return its opaque handle. The caller must have
    /// already validated that the secret is non-empty.
    fn store(&self, key_material: &str, label: &str) -> Result<String, SecretStoreError>;

    /// Remove a previously stored secret.
    fn delete(&self, handle: &str) -> Result<(), SecretStoreError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretStoreError {
    pub message: String,
}

impl SecretStoreError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for SecretStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for SecretStoreError {}

/// A canary secret store for testing. Never holds real keys.
#[derive(Debug, Default)]
pub struct CanarySecretStore {
    storage: std::sync::Mutex<std::collections::HashMap<String, String>>,
}

impl SecretStore for CanarySecretStore {
    fn resolve(&self, handle: &str) -> Result<ResolvedSecret, SecretStoreError> {
        let storage = self
            .storage
            .lock()
            .map_err(|_| SecretStoreError::new("canary store lock poisoned"))?;
        Ok(match storage.get(handle) {
            Some(key) => ResolvedSecret::Key(key.clone()),
            None => ResolvedSecret::Missing,
        })
    }

    fn store(&self, key_material: &str, label: &str) -> Result<String, SecretStoreError> {
        let handle = format!(
            "secret-ref:canary-{}",
            label.replace(' ', "-").to_lowercase()
        );
        let mut storage = self
            .storage
            .lock()
            .map_err(|_| SecretStoreError::new("canary store lock poisoned"))?;
        storage.insert(handle.clone(), key_material.to_owned());
        Ok(handle)
    }

    fn delete(&self, handle: &str) -> Result<(), SecretStoreError> {
        let mut storage = self
            .storage
            .lock()
            .map_err(|_| SecretStoreError::new("canary store lock poisoned"))?;
        storage.remove(handle);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_secret_ref_accepted() {
        assert!(is_valid_secret_ref("secret-ref:abc.123_xyz-00"));
    }

    #[test]
    fn invalid_prefix_rejected() {
        assert!(!is_valid_secret_ref("SECRET-REF:x"));
        assert!(!is_valid_secret_ref("secret:x"));
        assert!(!is_valid_secret_ref("api_key"));
    }

    #[test]
    fn empty_suffix_rejected() {
        assert!(!is_valid_secret_ref("secret-ref:"));
    }

    #[test]
    fn canary_store_round_trips() {
        let store = CanarySecretStore::default();
        let handle = store.store("test-key-12345", "Test Provider").unwrap();
        assert!(handle.starts_with("secret-ref:"));

        let resolved = store.resolve(&handle).unwrap();
        match resolved {
            ResolvedSecret::Key(k) => assert_eq!(k, "test-key-12345"),
            _ => panic!("expected Key"),
        }

        store.delete(&handle).unwrap();
        match store.resolve(&handle).unwrap() {
            ResolvedSecret::Missing => {}
            _ => panic!("expected Missing after delete"),
        }
    }
}
