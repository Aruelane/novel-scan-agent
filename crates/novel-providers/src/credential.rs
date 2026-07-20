//! Windows Credential Manager integration stub.
//! Real implementation will use the `windows-credentials` crate or
//! direct Win32 API calls. This stub documents the contract.

use crate::secret::{ResolvedSecret, SecretStore, SecretStoreError};

/// Windows Credential Manager-backed secret store.
/// Stub: real implementation requires S6 productization.
pub struct WindowsCredentialStore;

impl WindowsCredentialStore {
    pub fn new() -> Self {
        Self
    }
}

impl SecretStore for WindowsCredentialStore {
    fn resolve(&self, handle: &str) -> Result<ResolvedSecret, SecretStoreError> {
        if !crate::secret::is_valid_secret_ref(handle) {
            return Err(SecretStoreError::new("invalid secret-ref handle"));
        }
        // Stub: real impl calls CredReadW
        Err(SecretStoreError::new(
            "Windows Credential Manager not yet wired (S6 productization)",
        ))
    }

    fn store(&self, _key_material: &str, _label: &str) -> Result<String, SecretStoreError> {
        Err(SecretStoreError::new(
            "Windows Credential Manager not yet wired (S6 productization)",
        ))
    }

    fn delete(&self, _handle: &str) -> Result<(), SecretStoreError> {
        Err(SecretStoreError::new(
            "Windows Credential Manager not yet wired (S6 productization)",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_handle() {
        let store = WindowsCredentialStore;
        assert!(store.resolve("bad-handle").is_err());
    }

    #[test]
    fn valid_handle_reports_not_wired() {
        let store = WindowsCredentialStore;
        let result = store.resolve("secret-ref:test");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not yet wired"));
    }
}
