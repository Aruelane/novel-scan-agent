//! Platform secret store implementations.
//!
//! The `SecretStore` trait (defined in novel-providers) is implemented for
//! each platform. Windows uses a file-based store, Android has a bridge
//! contract with fake for testing (real JNI/Kotlin in S6).

pub mod android;
pub mod windows;

#[cfg(test)]
pub mod memory;
