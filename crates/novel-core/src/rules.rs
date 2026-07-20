//! Rule engineering contracts — invariants, concept aggregation, and preset models.

use serde::{Deserialize, Serialize};

use crate::{AlertLevel, RuleCategory, RuleDefinition};

/// Rule invariant checks that must hold before a rule can be used in scanning.
pub fn validate_rule_invariants(rule: &RuleDefinition) -> Result<(), String> {
    if rule.id.is_empty() {
        return Err("rule id must not be empty".into());
    }
    if rule.version < 1 {
        return Err("rule version must be >= 1".into());
    }
    if rule.name.is_empty() {
        return Err("rule name must not be empty".into());
    }
    if rule.criteria.is_empty() && rule.detection_mode != crate::DetectionMode::ManualOnly {
        return Err(format!("rule '{}' has empty criteria", rule.id));
    }
    Ok(())
}

/// A user-editable preset for a set of rules with alert overrides.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RulePreset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub rule_overrides: Vec<RuleOverride>,
}

/// Per-rule override within a preset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleOverride {
    pub rule_id: String,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub alert_level: Option<AlertLevel>,
    #[serde(default)]
    pub note: Option<String>,
}

/// Merge three layers: pack defaults → preset overrides → per-scan user overrides.
/// Later layers override earlier ones.
pub fn merge_rule_config(
    default_enabled: bool,
    default_alert: AlertLevel,
    preset: Option<&RuleOverride>,
    user: Option<&RuleOverride>,
) -> (bool, AlertLevel) {
    let enabled = user
        .and_then(|u| u.enabled)
        .or_else(|| preset.and_then(|p| p.enabled))
        .unwrap_or(default_enabled);
    let alert = user
        .and_then(|u| u.alert_level)
        .or_else(|| preset.and_then(|p| p.alert_level))
        .unwrap_or(default_alert);
    (enabled, alert)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DetectionMode;

    fn test_rule() -> RuleDefinition {
        RuleDefinition {
            id: "r1".into(),
            version: 1,
            name: "Test".into(),
            description: "desc".into(),
            category: RuleCategory::Landmine,
            default_alert_level: AlertLevel::Medium,
            confirmation_scope: crate::ConfirmationScope::Chapter,
            requires_user_boundary: false,
            tags: vec![],
            detection_profile_ref: None,
            detection_mode: DetectionMode::Semantic,
            criteria: vec!["c1".into()],
            exclusions: vec!["e1".into()],
            pending_conditions: vec!["p1".into()],
        }
    }

    #[test]
    fn valid_rule_passes_invariants() {
        assert!(validate_rule_invariants(&test_rule()).is_ok());
    }

    #[test]
    fn empty_id_fails_invariants() {
        let mut r = test_rule();
        r.id = String::new();
        assert!(validate_rule_invariants(&r).is_err());
    }

    #[test]
    fn empty_criteria_fails_for_semantic() {
        let mut r = test_rule();
        r.criteria.clear();
        assert!(validate_rule_invariants(&r).is_err());
    }

    #[test]
    fn preset_overrides_default() {
        let preset = RuleOverride {
            rule_id: "r1".into(),
            enabled: Some(false),
            alert_level: None,
            note: None,
        };
        let (enabled, alert) = merge_rule_config(true, AlertLevel::Medium, Some(&preset), None);
        assert!(!enabled);
        assert_eq!(alert, AlertLevel::Medium);
    }

    #[test]
    fn user_overrides_preset() {
        let preset = RuleOverride {
            rule_id: "r1".into(),
            enabled: Some(false),
            alert_level: None,
            note: None,
        };
        let user = RuleOverride {
            rule_id: "r1".into(),
            enabled: Some(true),
            alert_level: Some(AlertLevel::Critical),
            note: None,
        };
        let (enabled, alert) =
            merge_rule_config(true, AlertLevel::Medium, Some(&preset), Some(&user));
        assert!(enabled);
        assert_eq!(alert, AlertLevel::Critical);
    }
}
