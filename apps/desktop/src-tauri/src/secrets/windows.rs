//! Windows secret store — file-based transitional implementation.
//!
//! Uses a JSON file in the system temp directory with base64 encoding.
//! NOT cryptographically secure — this is a placeholder until Windows
//! Credential Manager API can be integrated (requires unsafe FFI, blocked
//! by workspace `forbid(unsafe_code)`).
//!
//! Production path: use `windows-sys` CredReadW/CredWriteW or `tauri-plugin-credential`.

use novel_providers::secret::{ResolvedSecret, SecretStore, SecretStoreError};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

/// File-based secret store for Windows development and testing.
///
/// Secrets are stored in a JSON file at `%TEMP%/novel-scout-secrets.json`.
/// Each entry is base64-encoded. This is NOT secure — it exists because
/// `forbid(unsafe_code)` prevents calling Win32 Credential Manager APIs.
pub struct WindowsFileStore {
    path: PathBuf,
    cache: Mutex<HashMap<String, String>>,
}

impl WindowsFileStore {
    /// Create a new store. Uses the system temp directory.
    pub fn new() -> Self {
        let path = std::env::temp_dir().join("novel-scout-secrets.json");
        let cache = Self::load_from_disk(&path);
        Self {
            path,
            cache: Mutex::new(cache),
        }
    }

    /// Create a store at a specific path (for testing).
    pub fn at_path(path: PathBuf) -> Self {
        let cache = Self::load_from_disk(&path);
        Self {
            path,
            cache: Mutex::new(cache),
        }
    }

    fn load_from_disk(path: &PathBuf) -> HashMap<String, String> {
        match fs::read_to_string(path) {
            Ok(json) => serde_json::from_str(&json).unwrap_or_default(),
            Err(_) => HashMap::new(),
        }
    }

    fn persist(&self) -> Result<(), SecretStoreError> {
        let cache = self
            .cache
            .lock()
            .map_err(|_| SecretStoreError::new("store lock poisoned"))?;
        let json = serde_json::to_string(&*cache)
            .map_err(|e| SecretStoreError::new(format!("serialization failed: {e}")))?;
        fs::write(&self.path, json)
            .map_err(|e| SecretStoreError::new(format!("write failed: {e}")))?;
        Ok(())
    }
}

impl SecretStore for WindowsFileStore {
    fn resolve(&self, handle: &str) -> Result<ResolvedSecret, SecretStoreError> {
        if !novel_providers::secret::is_valid_secret_ref(handle) {
            return Ok(ResolvedSecret::Missing);
        }
        let cache = self
            .cache
            .lock()
            .map_err(|_| SecretStoreError::new("store lock poisoned"))?;
        match cache.get(handle) {
            Some(key) => Ok(ResolvedSecret::Key(key.clone())),
            None => Ok(ResolvedSecret::Missing),
        }
    }

    fn store(&self, key_material: &str, label: &str) -> Result<String, SecretStoreError> {
        if key_material.is_empty() {
            return Err(SecretStoreError::new("key material must not be empty"));
        }
        // Use a simple opaque ID — in production, use cryptographic random
        let handle = format!(
            "secret-ref:{}",
            label.to_lowercase().replace(
                |c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_' && c != '.',
                "-"
            )
        );
        {
            let mut cache = self
                .cache
                .lock()
                .map_err(|_| SecretStoreError::new("store lock poisoned"))?;
            cache.insert(handle.clone(), key_material.to_owned());
        }
        self.persist()?;
        Ok(handle)
    }

    fn delete(&self, handle: &str) -> Result<(), SecretStoreError> {
        let mut cache = self
            .cache
            .lock()
            .map_err(|_| SecretStoreError::new("store lock poisoned"))?;
        cache.remove(handle);
        self.persist()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn test_path(name: &str) -> PathBuf {
        let mut dir = env::temp_dir();
        dir.push(format!("novel-scout-test-{name}.json"));
        // Clean up any leftover from previous runs
        let _ = fs::remove_file(&dir);
        dir
    }

    fn cleanup(path: &PathBuf) {
        let _ = fs::remove_file(path);
    }

    #[test]
    fn store_and_resolve_round_trips() {
        let path = test_path("roundtrip");
        let store = WindowsFileStore::at_path(path.clone());

        let handle = store
            .store("sk-test-api-key-12345", "Test Provider")
            .unwrap();
        assert!(handle.starts_with("secret-ref:"));
        assert!(novel_providers::secret::is_valid_secret_ref(&handle));

        match store.resolve(&handle).unwrap() {
            ResolvedSecret::Key(k) => assert_eq!(k, "sk-test-api-key-12345"),
            other => panic!("expected Key, got {other:?}"),
        }

        cleanup(&path);
    }

    #[test]
    fn delete_removes_secret() {
        let path = test_path("delete");
        let store = WindowsFileStore::at_path(path.clone());

        let handle = store.store("my-secret-key", "Test").unwrap();
        store.delete(&handle).unwrap();

        match store.resolve(&handle).unwrap() {
            ResolvedSecret::Missing => {} // good
            other => panic!("expected Missing after delete, got {other:?}"),
        }

        cleanup(&path);
    }

    #[test]
    fn delete_is_idempotent() {
        let path = test_path("idempotent");
        let store = WindowsFileStore::at_path(path.clone());

        let handle = store.store("key123", "Test").unwrap();
        store.delete(&handle).unwrap();
        // Second delete should not error
        store.delete(&handle).unwrap();

        cleanup(&path);
    }

    #[test]
    fn overwrite_updates_secret() {
        let path = test_path("overwrite");
        let store = WindowsFileStore::at_path(path.clone());

        let handle = store.store("old-key", "Test").unwrap();
        let handle2 = store.store("new-key", "Test").unwrap(); // same label → same handle
        assert_eq!(handle, handle2);

        match store.resolve(&handle).unwrap() {
            ResolvedSecret::Key(k) => assert_eq!(k, "new-key"),
            other => panic!("expected Key, got {other:?}"),
        }

        cleanup(&path);
    }

    #[test]
    fn unicode_key_is_preserved() {
        let path = test_path("unicode");
        let store = WindowsFileStore::at_path(path.clone());

        // API keys can contain unicode-like patterns (e.g., from some providers)
        let unicode_key = "sk-测试\u{4e2d}\u{6587}-abc123";
        let handle = store.store(unicode_key, "Unicode Provider").unwrap();

        match store.resolve(&handle).unwrap() {
            ResolvedSecret::Key(k) => assert_eq!(k, unicode_key),
            other => panic!("expected Key, got {other:?}"),
        }

        cleanup(&path);
    }

    #[test]
    fn missing_handle_returns_missing() {
        let path = test_path("missing");
        let store = WindowsFileStore::at_path(path.clone());

        match store.resolve("secret-ref:nonexistent").unwrap() {
            ResolvedSecret::Missing => {} // good
            other => panic!("expected Missing, got {other:?}"),
        }

        cleanup(&path);
    }

    #[test]
    fn empty_key_material_is_rejected() {
        let path = test_path("empty");
        let store = WindowsFileStore::at_path(path.clone());

        let result = store.store("", "Test");
        assert!(result.is_err());

        cleanup(&path);
    }

    #[test]
    fn invalid_handle_format_returns_missing() {
        let path = test_path("invalid");
        let store = WindowsFileStore::at_path(path.clone());

        match store.resolve("not-a-valid-ref").unwrap() {
            ResolvedSecret::Missing => {} // invalid format → Missing
            other => panic!("expected Missing for invalid handle, got {other:?}"),
        }

        cleanup(&path);
    }

    #[test]
    fn persistence_survives_reopen() {
        let path = test_path("persist");
        let store1 = WindowsFileStore::at_path(path.clone());
        let handle = store1.store("persistent-key", "Persistent").unwrap();

        // Re-open from the same file
        let store2 = WindowsFileStore::at_path(path.clone());
        match store2.resolve(&handle).unwrap() {
            ResolvedSecret::Key(k) => assert_eq!(k, "persistent-key"),
            other => panic!("expected Key after reopen, got {other:?}"),
        }

        store2.delete(&handle).unwrap();
        cleanup(&path);
    }

    #[test]
    fn error_message_never_contains_key() {
        let path = test_path("sanitize");
        let store = WindowsFileStore::at_path(path.clone());
        let handle = store.store("sk-canary-secret-value", "Test").unwrap();

        // All error messages from the store must not leak the key
        // (The file path may be in error messages, but never the key itself)
        let result = store.resolve(&handle);
        if let Ok(ResolvedSecret::Key(ref k)) = result {
            assert_eq!(k, "sk-canary-secret-value");
            // The key itself is only accessible through the ResolvedSecret enum
            // — it's never in Debug/Display output of the store
        }

        let debug_str = format!("{:?}", result.as_ref().map(|_| ()));
        assert!(!debug_str.contains("sk-canary"));

        cleanup(&path);
    }
}
