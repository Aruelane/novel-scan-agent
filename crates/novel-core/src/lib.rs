#![forbid(unsafe_code)]

//! Shared, platform-independent novel scanning primitives.
//!
//! The crate intentionally knows nothing about API keys, HTTP clients, local
//! files, Tauri, Windows, or Android. Applications provide those capabilities
//! through adapters while the same scan and evidence rules run everywhere.

pub mod compression;
pub mod model;
pub mod provider;
pub mod scanner;

pub use compression::{
    memory_id, CompressionError, CompressionFuture, CompressionRequest, ContextCompressor,
    ContextSnapshot, DeterministicContextCompressor, EntityMemory, EventMemory, RelationshipMemory,
    UnresolvedMemory, CONTEXT_SNAPSHOT_SCHEMA_VERSION,
};
pub use model::*;
pub use provider::{
    DeterministicTestProvider, InferenceRequest, ModelProvider, PatternRule, ProviderCandidate,
    ProviderError, ProviderEvidenceRange, ProviderFuture, ProviderResponse, ProviderUsage,
    RuleContext,
};
pub use scanner::{
    CheckpointStore, CheckpointStoreError, InMemoryCheckpointStore, ProcessedChapter,
    ScanBatchResult, ScanCheckpoint, ScanEngine, ScanError,
};
