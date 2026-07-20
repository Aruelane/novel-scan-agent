//! Community rule source provenance ledger.
//!
//! Every rule must trace back to a verifiable source. This module defines
//! the source record types used in rule pack provenance.

use serde::{Deserialize, Serialize};

/// What kind of source was consulted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    CommunityPost,
    CommunityIndex,
    UserRequirement,
    ProjectNote,
}

/// Verification status of a source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceVerification {
    Unverified,
    MetadataOnly,
    PartiallyVerified,
    Verified,
}

/// A single source record. Mirrors the rule pack JSON `sourceCatalog` entries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceRecord {
    pub id: String,
    pub kind: SourceKind,
    pub title: String,
    #[serde(default)]
    pub uri: Option<String>,
    #[serde(default)]
    pub published_at: Option<String>,
    pub accessed_at: String,
    pub verification: SourceVerification,
    pub note: String,
}

/// A rule-to-source mapping. One rule can reference multiple sources.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleProvenance {
    pub rule_id: String,
    pub source_refs: Vec<String>,
    pub verification: SourceVerification,
    pub note: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_record_round_trips_json() {
        let record = SourceRecord {
            id: "tieba.post.1".into(),
            kind: SourceKind::CommunityPost,
            title: "测试帖".into(),
            uri: Some("https://example.com".into()),
            published_at: None,
            accessed_at: "2026-01-01".into(),
            verification: SourceVerification::MetadataOnly,
            note: "测试来源".into(),
        };
        let json = serde_json::to_string(&record).unwrap();
        let restored: SourceRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(record, restored);
    }

    #[test]
    fn unverified_is_the_default_for_missing_provenance() {
        let json = r#"{"id":"r1","source_refs":[],"verification":"unverified","note":""}"#;
        let prov: RuleProvenance = serde_json::from_str(json).unwrap();
        assert_eq!(prov.verification, SourceVerification::Unverified);
    }
}
