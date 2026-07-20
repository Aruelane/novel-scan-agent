//! Multi-provider adapter registry for the novel scan agent.
//!
//! This crate defines provider configuration, templates, and a registry of
//! known providers. It does NOT make HTTP requests or access platform secret
//! stores. Actual API adapters and secret management are built in later S4 tasks.

pub mod anthropic;
pub mod config;
pub mod http;
pub mod openai_compat;
pub mod prompt;
pub mod registry;
pub mod retry;
pub mod schema;
pub mod secret;
