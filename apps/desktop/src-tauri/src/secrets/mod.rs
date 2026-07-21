//! Platform secret store implementations.
//!
//! The `SecretStore` trait (defined in novel-providers) is implemented for
//! each platform. Windows uses a file-based store (Credential Manager
//! requires unsafe FFI which is blocked by workspace `forbid(unsafe_code)`).
//! The file store is a transitional solution; production should use the
//! Windows Credential Manager API via a safe wrapper crate.

pub mod windows;

#[cfg(test)]
pub mod memory;
