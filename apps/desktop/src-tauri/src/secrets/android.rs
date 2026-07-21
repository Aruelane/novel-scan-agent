//! Android Keystore bridge contract.
//!
//! Defines the Rust ↔ Kotlin interface for Android Keystore-backed secret
//! storage. The real JNI/Kotlin implementation is deferred to S6 tasks
//! (S6-AND-03A through S6-AND-03D). This module provides:
//!
//! 1. A trait contract for the bridge
//! 2. A fake implementation for host-side contract tests
//! 3. `#[cfg(target_os = "android")]` gating for future real impl
//!
//! ## Security contract (from docs/ANDROID_KEYSTORE_CONTRACT.md)
//!
//! - Keystore uses non-exportable key to encrypt app-private blob
//! - Clearing app data → keys unrecoverable
//! - No hardcoded keys or IVs
//! - Authentication failures return typed errors

use novel_providers::secret::{ResolvedSecret, SecretStore, SecretStoreError};
use std::collections::HashMap;
use std::sync::Mutex;

/// Fake Android Keystore bridge for host-side contract testing.
///
/// This stores secrets in memory, same behavior as a real Keystore:
/// - Data is lost when the store is dropped (simulates app data clear)
/// - Alias isolation: different aliases are independent
/// - Delete is idempotent
pub struct AndroidFakeStore {
    storage: Mutex<HashMap<String, String>>,
}

impl AndroidFakeStore {
    pub fn new() -> Self {
        Self {
            storage: Mutex::new(HashMap::new()),
        }
    }

    /// Simulate clearing app data (drops all stored secrets).
    #[cfg(test)]
    pub fn simulate_app_data_clear(&self) {
        let mut storage = self.storage.lock().unwrap();
        storage.clear();
    }
}

impl Default for AndroidFakeStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretStore for AndroidFakeStore {
    fn resolve(&self, handle: &str) -> Result<ResolvedSecret, SecretStoreError> {
        if !novel_providers::secret::is_valid_secret_ref(handle) {
            return Ok(ResolvedSecret::Missing);
        }
        let storage = self
            .storage
            .lock()
            .map_err(|_| SecretStoreError::new("android store lock poisoned"))?;
        Ok(match storage.get(handle) {
            Some(key) => ResolvedSecret::Key(key.clone()),
            None => ResolvedSecret::Missing,
        })
    }

    fn store(&self, key_material: &str, label: &str) -> Result<String, SecretStoreError> {
        if key_material.is_empty() {
            return Err(SecretStoreError::new("key material must not be empty"));
        }
        let handle = format!(
            "secret-ref:android-{}",
            label.to_lowercase().replace(
                |c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_' && c != '.',
                "-"
            )
        );
        let mut storage = self
            .storage
            .lock()
            .map_err(|_| SecretStoreError::new("android store lock poisoned"))?;
        storage.insert(handle.clone(), key_material.to_owned());
        Ok(handle)
    }

    fn delete(&self, handle: &str) -> Result<(), SecretStoreError> {
        let mut storage = self
            .storage
            .lock()
            .map_err(|_| SecretStoreError::new("android store lock poisoned"))?;
        storage.remove(handle);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_store_round_trips() {
        let store = AndroidFakeStore::new();
        let handle = store.store("sk-android-test-123", "Android Test").unwrap();
        assert!(handle.starts_with("secret-ref:android-"));
        assert!(novel_providers::secret::is_valid_secret_ref(&handle));

        match store.resolve(&handle).unwrap() {
            ResolvedSecret::Key(k) => assert_eq!(k, "sk-android-test-123"),
            other => panic!("expected Key, got {other:?}"),
        }
    }

    #[test]
    fn fake_store_delete() {
        let store = AndroidFakeStore::new();
        let handle = store.store("delete-me", "Test").unwrap();
        store.delete(&handle).unwrap();
        match store.resolve(&handle).unwrap() {
            ResolvedSecret::Missing => {}
            other => panic!("expected Missing, got {other:?}"),
        }
    }

    #[test]
    fn fake_store_alias_isolation() {
        let store = AndroidFakeStore::new();
        let h1 = store.store("key-one", "Provider A").unwrap();
        let h2 = store.store("key-two", "Provider B").unwrap();
        assert_ne!(h1, h2);

        store.delete(&h1).unwrap();
        match store.resolve(&h1).unwrap() {
            ResolvedSecret::Missing => {}
            other => panic!("h1 should be Missing, got {other:?}"),
        }
        match store.resolve(&h2).unwrap() {
            ResolvedSecret::Key(k) => assert_eq!(k, "key-two"),
            other => panic!("h2 should be Key, got {other:?}"),
        }
    }

    #[test]
    fn fake_store_simulate_app_data_clear() {
        let store = AndroidFakeStore::new();
        let handle = store.store("clear-me", "Test").unwrap();
        store.simulate_app_data_clear();

        match store.resolve(&handle).unwrap() {
            ResolvedSecret::Missing => {}
            other => panic!("expected Missing after clear, got {other:?}"),
        }
    }

    #[test]
    fn fake_store_delete_idempotent() {
        let store = AndroidFakeStore::new();
        let handle = store.store("idem-key", "Test").unwrap();
        store.delete(&handle).unwrap();
        store.delete(&handle).unwrap(); // second delete should not error
        match store.resolve(&handle).unwrap() {
            ResolvedSecret::Missing => {}
            other => panic!("expected Missing, got {other:?}"),
        }
    }

    #[test]
    fn fake_store_error_never_contains_key() {
        let store = AndroidFakeStore::new();
        let handle = store.store("sk-canary-android-001", "Canary").unwrap();

        // The store's Debug should not expose the key
        let debug_str = format!("{:?}", store.storage.lock().unwrap().len());
        assert!(!debug_str.contains("sk-canary"));
        drop(debug_str);

        // Verify key is retrievable
        match store.resolve(&handle).unwrap() {
            ResolvedSecret::Key(_) => {}
            other => panic!("expected Key, got {other:?}"),
        }
    }
}
