//! Multi-provider adapter registry for the novel scan agent.
//!
//! This crate defines provider configuration, templates, HTTP transport with
//! sanitization and cancellation, and a registry of known providers. Platform
//! secret stores and Tauri commands are built in later S4 tasks.

pub mod config;
pub mod credential;
pub mod http;
pub mod openai_compat;
pub mod prompt;
pub mod redaction;
pub mod registry;
pub mod retry;
pub mod schema;
pub mod secret;
pub mod security;
