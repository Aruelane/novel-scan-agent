//! In-memory secret store for testing. Never holds real keys in CI.

use novel_providers::secret::{ResolvedSecret, SecretStore, SecretStoreError};
use std::collections::HashMap;
use std::sync::Mutex;

/// A canary secret store for testing. Not for production use.
pub struct MemoryStore {
    storage: Mutex<HashMap<String, String>>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self {
            storage: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretStore for MemoryStore {
    fn resolve(&self, handle: &str) -> Result<ResolvedSecret, SecretStoreError> {
        if !novel_providers::secret::is_valid_secret_ref(handle) {
            return Ok(ResolvedSecret::Missing);
        }
        let storage = self
            .storage
            .lock()
            .map_err(|_| SecretStoreError::new("memory store lock poisoned"))?;
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
            "secret-ref:test-{}",
            label.to_lowercase().replace(
                |c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_' && c != '.',
                "-"
            )
        );
        let mut storage = self
            .storage
            .lock()
            .map_err(|_| SecretStoreError::new("memory store lock poisoned"))?;
        storage.insert(handle.clone(), key_material.to_owned());
        Ok(handle)
    }

    fn delete(&self, handle: &str) -> Result<(), SecretStoreError> {
        let mut storage = self
            .storage
            .lock()
            .map_err(|_| SecretStoreError::new("memory store lock poisoned"))?;
        storage.remove(handle);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_store_round_trips() {
        let store = MemoryStore::new();
        let handle = store.store("test-key-123", "Test").unwrap();
        match store.resolve(&handle).unwrap() {
            ResolvedSecret::Key(k) => assert_eq!(k, "test-key-123"),
            _ => panic!("expected Key"),
        }
    }

    #[test]
    fn memory_store_delete() {
        let store = MemoryStore::new();
        let handle = store.store("key", "Test").unwrap();
        store.delete(&handle).unwrap();
        match store.resolve(&handle).unwrap() {
            ResolvedSecret::Missing => {}
            _ => panic!("expected Missing after delete"),
        }
    }
}
