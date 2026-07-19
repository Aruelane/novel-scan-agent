use std::fmt;

use novel_core::{AlertLevel, ConfirmationScope, RuleCategory, RuleDefinition};
use serde::Deserialize;

// ---------------------------------------------------------------------------
// Deserialization structs matching the rule-pack JSON schema
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RulePackJson {
    #[serde(rename = "$schema")]
    pub schema_uri: Option<String>,
    pub schema_version: String,
    pub id: String,
    pub version: String,
    #[allow(dead_code)]
    pub status: Option<String>,
    #[allow(dead_code)]
    pub locale: Option<String>,
    #[allow(dead_code)]
    pub title: Option<String>,
    #[allow(dead_code)]
    pub description: Option<String>,
    pub rules: Vec<RuleJson>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleJson {
    pub id: String,
    pub version: u32,
    /// Deserialized as String so we can explicitly validate the mapping.
    pub category: String,
    pub name: String,
    pub description: String,
    pub status: String,
    pub default_enabled: bool,
    pub default_severity: String,
    pub detection: DetectionJson,
    #[serde(default)]
    pub provenance: ProvenanceJson,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectionJson {
    pub profile_ref: Option<String>,
    pub mode: Option<String>,
    #[serde(default)]
    pub criteria: Vec<String>,
    #[serde(default)]
    pub exclusions: Vec<String>,
    #[serde(default)]
    pub pending_conditions: Vec<String>,
    pub confirmation_scope: String,
    pub requires_user_boundary: bool,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProvenanceJson {
    #[allow(dead_code)]
    pub verification: Option<String>,
    #[allow(dead_code)]
    pub source_refs: Option<Vec<String>>,
    #[allow(dead_code)]
    pub note: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserConfigJson {
    #[allow(dead_code)]
    pub toggleable: bool,
    #[allow(dead_code)]
    pub severity_override: bool,
    #[allow(dead_code)]
    pub custom_boundary_allowed: bool,
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
pub struct RulePackError {
    pub message: String,
}

impl RulePackError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for RulePackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for RulePackError {}

// ---------------------------------------------------------------------------
// Domain types for loaded rules
// ---------------------------------------------------------------------------

/// A fully loaded rule including the detection text arrays that the core
/// `RuleDefinition` does not carry yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedRule {
    pub definition: RuleDefinition,
    pub criteria: Vec<String>,
    pub exclusions: Vec<String>,
    pub pending_conditions: Vec<String>,
    /// "semantic" or "manual_only"
    pub detection_mode: String,
    pub profile_ref: Option<String>,
    pub status: String,
    pub default_enabled: bool,
    pub provenance: ProvenanceJson,
}

/// A deserialized, validated rule pack ready for use by the scanner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RulePack {
    pub id: String,
    pub version: String,
    pub schema_version: String,
    pub rules: Vec<LoadedRule>,
}

// ---------------------------------------------------------------------------
// Mapping helpers
// ---------------------------------------------------------------------------

fn map_category(raw: &str) -> Result<RuleCategory, RulePackError> {
    match raw {
        "landmine" => Ok(RuleCategory::Landmine),
        "frustration" => Ok(RuleCategory::Frustration),
        other => Err(RulePackError::new(format!(
            "unknown rule category '{other}'; expected 'landmine' or 'frustration'"
        ))),
    }
}

fn map_severity(raw: &str) -> Result<AlertLevel, RulePackError> {
    match raw {
        "critical" => Ok(AlertLevel::Critical),
        "high" => Ok(AlertLevel::High),
        "medium" => Ok(AlertLevel::Medium),
        "low" => Ok(AlertLevel::Low),
        "info" => Ok(AlertLevel::Info),
        other => Err(RulePackError::new(format!(
            "unknown severity level '{other}'"
        ))),
    }
}

fn map_confirmation_scope(raw: &str) -> Result<ConfirmationScope, RulePackError> {
    match raw {
        "local" => Ok(ConfirmationScope::Local),
        "chapter" => Ok(ConfirmationScope::Chapter),
        "cross_chapter" => Ok(ConfirmationScope::CrossChapter),
        "whole_book" => Ok(ConfirmationScope::WholeBook),
        other => Err(RulePackError::new(format!(
            "unknown confirmation scope '{other}'"
        ))),
    }
}

// ---------------------------------------------------------------------------
// TryFrom<RuleJson> for RuleDefinition
// ---------------------------------------------------------------------------

impl TryFrom<RuleJson> for RuleDefinition {
    type Error = RulePackError;

    fn try_from(rule: RuleJson) -> Result<Self, Self::Error> {
        if rule.version < 1 {
            return Err(RulePackError::new(format!(
                "rule '{}' version must be at least 1",
                rule.id
            )));
        }
        let category = map_category(&rule.category)?;
        let default_alert_level = map_severity(&rule.default_severity)?;
        let confirmation_scope = map_confirmation_scope(&rule.detection.confirmation_scope)?;

        // Unverified rules must not be default-enabled.
        if rule.status != "verified" && rule.default_enabled {
            return Err(RulePackError::new(format!(
                "unverified rule '{}' has status '{}' but default_enabled is true; \
                 only verified rules may be default-enabled",
                rule.id, rule.status
            )));
        }

        Ok(RuleDefinition {
            id: rule.id,
            version: rule.version,
            name: rule.name,
            description: rule.description,
            category,
            default_alert_level,
            confirmation_scope,
            requires_user_boundary: rule.detection.requires_user_boundary,
            tags: rule.tags,
        })
    }
}

// ---------------------------------------------------------------------------
// RulePack loading
// ---------------------------------------------------------------------------

impl RulePack {
    /// Read and deserialize a rule-pack JSON file from disk.
    pub fn load_from_path(path: &str) -> Result<Self, RulePackError> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| RulePackError::new(format!("failed to read rule pack '{path}': {e}")))?;
        Self::load_from_json(&text)
    }

    pub fn load_from_json(json: &str) -> Result<Self, RulePackError> {
        let raw: RulePackJson = serde_json::from_str(json)
            .map_err(|e| RulePackError::new(format!("failed to parse rule pack JSON: {e}")))?;

        let mut rules = Vec::with_capacity(raw.rules.len());
        let mut seen_ids = std::collections::HashSet::new();
        for rule_json in raw.rules {
            if !seen_ids.insert(rule_json.id.clone()) {
                return Err(RulePackError::new(format!(
                    "duplicate rule id '{}'",
                    rule_json.id
                )));
            }
            let loaded = Self::convert_to_loaded_rule(rule_json)?;
            rules.push(loaded);
        }

        Ok(RulePack {
            id: raw.id,
            version: raw.version,
            schema_version: raw.schema_version,
            rules,
        })
    }

    fn convert_to_loaded_rule(rule: RuleJson) -> Result<LoadedRule, RulePackError> {
        let detection_mode = rule
            .detection
            .mode
            .clone()
            .unwrap_or_else(|| "semantic".to_owned());
        let criteria = rule.detection.criteria.clone();
        let exclusions = rule.detection.exclusions.clone();
        let pending_conditions = rule.detection.pending_conditions.clone();
        let profile_ref = rule.detection.profile_ref.clone();
        let status = rule.status.clone();
        let default_enabled = rule.default_enabled;
        let provenance = rule.provenance.clone();

        let definition = RuleDefinition::try_from(rule)?;

        Ok(LoadedRule {
            definition,
            criteria,
            exclusions,
            pending_conditions,
            detection_mode,
            profile_ref,
            status,
            default_enabled,
            provenance,
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_rule_json() -> RuleJson {
        RuleJson {
            id: "test.rule".into(),
            version: 1,
            category: "landmine".into(),
            name: "Test Rule".into(),
            description: "A test rule".into(),
            status: "verified".into(),
            default_enabled: true,
            default_severity: "high".into(),
            detection: DetectionJson {
                profile_ref: None,
                mode: Some("semantic".into()),
                criteria: vec!["Must have evidence".into()],
                exclusions: vec!["Rumors".into()],
                pending_conditions: vec![],
                confirmation_scope: "chapter".into(),
                requires_user_boundary: false,
            },
            provenance: ProvenanceJson {
                verification: Some("verified".into()),
                source_refs: Some(vec!["test.source".into()]),
                note: Some("test provenance".into()),
            },
            tags: vec!["test".into()],
        }
    }

    // --- Category mapping ---

    #[test]
    fn category_landmine_maps_correctly() {
        let mut rule = minimal_rule_json();
        rule.category = "landmine".into();
        let def = RuleDefinition::try_from(rule).unwrap();
        assert_eq!(def.category, RuleCategory::Landmine);
    }

    #[test]
    fn category_frustration_maps_correctly() {
        let mut rule = minimal_rule_json();
        rule.category = "frustration".into();
        let def = RuleDefinition::try_from(rule).unwrap();
        assert_eq!(def.category, RuleCategory::Frustration);
    }

    #[test]
    fn category_thunder_is_rejected() {
        let mut rule = minimal_rule_json();
        rule.category = "thunder".into();
        let err = RuleDefinition::try_from(rule).unwrap_err();
        assert!(err.message.contains("unknown rule category"));
        assert!(err.message.contains("thunder"));
    }

    #[test]
    fn category_never_thunder() {
        // Sanity check: the "thunder" string must never map to a valid category.
        assert!(map_category("thunder").is_err());
        assert!(map_category("THUNDER").is_err());
        assert!(map_category("Thunder").is_err());
    }

    // --- Severity mapping ---

    #[test]
    fn severity_maps_all_levels() {
        for (raw, expected) in [
            ("critical", AlertLevel::Critical),
            ("high", AlertLevel::High),
            ("medium", AlertLevel::Medium),
            ("low", AlertLevel::Low),
            ("info", AlertLevel::Info),
        ] {
            let mut rule = minimal_rule_json();
            rule.default_severity = raw.into();
            let def = RuleDefinition::try_from(rule).unwrap();
            assert_eq!(def.default_alert_level, expected, "mismatch for '{raw}'");
        }
    }

    #[test]
    fn invalid_severity_is_rejected() {
        let mut rule = minimal_rule_json();
        rule.default_severity = "extreme".into();
        let err = RuleDefinition::try_from(rule).unwrap_err();
        assert!(err.message.contains("extreme"));
    }

    // --- Scope mapping ---

    #[test]
    fn confirmation_scope_maps_all_variants() {
        for (raw, expected) in [
            ("local", ConfirmationScope::Local),
            ("chapter", ConfirmationScope::Chapter),
            ("cross_chapter", ConfirmationScope::CrossChapter),
            ("whole_book", ConfirmationScope::WholeBook),
        ] {
            let mut rule = minimal_rule_json();
            rule.detection.confirmation_scope = raw.into();
            let def = RuleDefinition::try_from(rule).unwrap();
            assert_eq!(def.confirmation_scope, expected, "mismatch for '{raw}'");
        }
    }

    #[test]
    fn invalid_scope_is_rejected() {
        let mut rule = minimal_rule_json();
        rule.detection.confirmation_scope = "paragraph".into();
        let err = RuleDefinition::try_from(rule).unwrap_err();
        assert!(err.message.contains("paragraph"));
    }

    // --- Unverified gate ---

    #[test]
    fn unverified_rule_must_not_be_default_enabled() {
        let mut rule = minimal_rule_json();
        rule.status = "draft".into();
        rule.default_enabled = true;
        let err = RuleDefinition::try_from(rule).unwrap_err();
        assert!(err.message.contains("unverified"));
        assert!(err.message.contains("default_enabled"));
    }

    #[test]
    fn unverified_rule_with_default_disabled_is_ok() {
        let mut rule = minimal_rule_json();
        rule.status = "seed".into();
        rule.default_enabled = false;
        let def = RuleDefinition::try_from(rule).unwrap();
        assert_eq!(def.id, "test.rule");
    }

    #[test]
    fn verified_rule_may_be_default_enabled() {
        let mut rule = minimal_rule_json();
        rule.status = "verified".into();
        rule.default_enabled = true;
        let def = RuleDefinition::try_from(rule).unwrap();
        assert!(def.tags.contains(&"test".to_owned()));
    }

    // --- requires_user_boundary ---

    #[test]
    fn requires_user_boundary_is_preserved() {
        let mut rule = minimal_rule_json();
        rule.detection.requires_user_boundary = true;
        let def = RuleDefinition::try_from(rule).unwrap();
        assert!(def.requires_user_boundary);
    }

    #[test]
    fn requires_user_boundary_false_is_preserved() {
        let mut rule = minimal_rule_json();
        rule.detection.requires_user_boundary = false;
        let def = RuleDefinition::try_from(rule).unwrap();
        assert!(!def.requires_user_boundary);
    }

    // --- Missing version ---

    #[test]
    fn version_must_be_at_least_1() {
        let mut rule = minimal_rule_json();
        rule.id = "zero.ver".into();
        rule.version = 0;
        let err = RuleDefinition::try_from(rule).unwrap_err();
        assert!(
            err.message.contains("zero.ver"),
            "error should contain rule id: {}",
            err.message
        );
        assert!(
            err.message.contains("version"),
            "error should contain 'version': {}",
            err.message
        );
        assert!(
            err.message.contains("at least 1"),
            "error should contain 'at least 1': {}",
            err.message
        );
    }

    // --- detection arrays are preserved in LoadedRule ---

    #[test]
    fn loaded_rule_preserves_detection_arrays() {
        let rule = minimal_rule_json();
        let loaded = RulePack::convert_to_loaded_rule(rule).unwrap();
        assert_eq!(loaded.criteria, vec!["Must have evidence"]);
        assert_eq!(loaded.exclusions, vec!["Rumors"]);
        assert!(loaded.pending_conditions.is_empty());
        assert_eq!(loaded.detection_mode, "semantic");
    }

    #[test]
    fn loaded_rule_preserves_multiple_criteria() {
        let mut rule = minimal_rule_json();
        rule.detection.criteria = vec!["Criterion A".into(), "Criterion B".into()];
        let loaded = RulePack::convert_to_loaded_rule(rule).unwrap();
        assert_eq!(loaded.criteria.len(), 2);
    }
}
