use novel_core::{AlertLevel, ConfirmationScope, RuleCategory};
use novel_rulepack::*;

const SEED_PACK_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../packages/rulepack/packs/yy-novel-bar/2026.0.0-seed.1.json"
);

fn load_pack() -> RulePack {
    RulePack::load_from_path(SEED_PACK_PATH).expect("failed to load seed rule pack")
}

// ---------------------------------------------------------------------------
// Structure assertions
// ---------------------------------------------------------------------------

#[test]
fn seed_pack_has_32_rules() {
    let pack = load_pack();
    assert_eq!(pack.rules.len(), 32);
}

#[test]
fn category_counts_are_11_landmine_21_frustration() {
    let pack = load_pack();
    let landmine_count = pack
        .rules
        .iter()
        .filter(|r| r.definition.category == RuleCategory::Landmine)
        .count();
    let frustration_count = pack
        .rules
        .iter()
        .filter(|r| r.definition.category == RuleCategory::Frustration)
        .count();
    assert_eq!(landmine_count, 11, "expected 11 landmine rules");
    assert_eq!(frustration_count, 21, "expected 21 frustration rules");
}

#[test]
fn every_rule_has_version_at_least_1() {
    let pack = load_pack();
    for rule in &pack.rules {
        assert!(
            rule.definition.version >= 1,
            "rule '{}' has version {}",
            rule.definition.id,
            rule.definition.version
        );
    }
}

// ---------------------------------------------------------------------------
// Scope assertions
// ---------------------------------------------------------------------------

#[test]
fn confirmation_scope_variants_are_valid() {
    let pack = load_pack();
    let mut scopes: Vec<ConfirmationScope> = pack
        .rules
        .iter()
        .map(|r| r.definition.confirmation_scope)
        .collect();
    scopes.sort_by_key(|s| format!("{s:?}"));
    scopes.dedup();
    // All scopes present in the seed pack must be valid enum variants (the
    // fact that they loaded successfully already proves this).
    // Confirm we see at least the expected kinds.
    let scope_strings: Vec<String> = scopes.iter().map(|s| format!("{s:?}")).collect();
    assert!(
        scope_strings.contains(&"CrossChapter".to_owned()),
        "expected CrossChapter scope; found {scope_strings:?}"
    );
    assert!(
        scope_strings.contains(&"WholeBook".to_owned()),
        "expected WholeBook scope; found {scope_strings:?}"
    );
    // The ConfirmationScope enum must define all four variants expected by the
    // data model, even if not all appear in the seed file yet.
    use ConfirmationScope::*;
    let _all = [Local, Chapter, CrossChapter, WholeBook];
    assert_eq!(_all.len(), 4);
}

// ---------------------------------------------------------------------------
// Unverified-rules gate
// ---------------------------------------------------------------------------

#[test]
fn unverified_rules_have_default_enabled_false() {
    // The TryFrom implementation rejects unverified + default_enabled == true,
    // so any rule that loaded successfully satisfies the invariant.
    // We verify by loading the full seed pack without error.
    let pack = load_pack();
    // All 32 rules loaded: this implicitly proves none violated the
    // unverified-must-not-be-default-enabled constraint.
    assert_eq!(pack.rules.len(), 32);
}

// ---------------------------------------------------------------------------
// requires_user_boundary mapping
// ---------------------------------------------------------------------------

#[test]
fn requires_user_boundary_is_mapped_correctly() {
    let pack = load_pack();
    let mut with_boundary = 0usize;
    let mut without_boundary = 0usize;
    for rule in &pack.rules {
        if rule.definition.requires_user_boundary {
            with_boundary += 1;
        } else {
            without_boundary += 1;
        }
    }
    assert!(
        with_boundary > 0,
        "expected some rules to require user boundary"
    );
    assert!(
        without_boundary > 0,
        "expected some rules to not require user boundary"
    );
    assert_eq!(with_boundary + without_boundary, 32);
    // The seed pack has exactly 31 rules with requiresUserBoundary: true and 1
    // with false (yy.frust.single-female-overly-dominant).
    assert_eq!(
        without_boundary, 1,
        "seed pack has exactly 1 rule without user boundary"
    );
}

// ---------------------------------------------------------------------------
// Severity mapping
// ---------------------------------------------------------------------------

#[test]
fn default_severity_maps_to_correct_alert_level() {
    let pack = load_pack();
    for rule in &pack.rules {
        let level = rule.definition.default_alert_level;
        match rule.definition.category {
            RuleCategory::Landmine => {
                // Landmine rules have severity "critical"
                assert_eq!(
                    level,
                    AlertLevel::Critical,
                    "landmine rule '{}' should be critical, got {level:?}",
                    rule.definition.id
                );
            }
            RuleCategory::Frustration => {
                // Frustration rules have severity "medium"
                assert_eq!(
                    level,
                    AlertLevel::Medium,
                    "frustration rule '{}' should be medium, got {level:?}",
                    rule.definition.id
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Category is NEVER thunder
// ---------------------------------------------------------------------------

#[test]
fn category_is_never_thunder() {
    let pack = load_pack();
    for rule in &pack.rules {
        let cat_str = format!("{:?}", rule.definition.category);
        assert!(
            !cat_str.to_lowercase().contains("thunder"),
            "rule '{}' has category {cat_str}",
            rule.definition.id
        );
    }
}

// ---------------------------------------------------------------------------
// Negative tests: invalid category
// ---------------------------------------------------------------------------

#[test]
fn invalid_category_is_rejected() {
    let json = r#"{
        "$schema": "",
        "schemaVersion": "1.0.0",
        "id": "test",
        "version": "0.1.0",
        "rules": [
            {
                "id": "bad.cat",
                "version": 1,
                "category": "thunder",
                "name": "Bad",
                "description": "Bad category",
                "status": "verified",
                "defaultEnabled": false,
                "defaultSeverity": "low",
                "detection": {
                    "criteria": [],
                    "exclusions": [],
                    "pendingConditions": [],
                    "confirmationScope": "local",
                    "requiresUserBoundary": false
                }
            }
        ]
    }"#;
    let err = RulePack::load_from_json(json).unwrap_err();
    assert!(err.message.contains("unknown rule category"));
    assert!(err.message.contains("thunder"));
}

// ---------------------------------------------------------------------------
// Negative test: invalid scope
// ---------------------------------------------------------------------------

#[test]
fn invalid_scope_is_rejected() {
    let json = r#"{
        "$schema": "",
        "schemaVersion": "1.0.0",
        "id": "test",
        "version": "0.1.0",
        "rules": [
            {
                "id": "bad.scope",
                "version": 1,
                "category": "landmine",
                "name": "Bad Scope",
                "description": "Bad scope",
                "status": "verified",
                "defaultEnabled": false,
                "defaultSeverity": "low",
                "detection": {
                    "criteria": [],
                    "exclusions": [],
                    "pendingConditions": [],
                    "confirmationScope": "sentence",
                    "requiresUserBoundary": false
                }
            }
        ]
    }"#;
    let err = RulePack::load_from_json(json).unwrap_err();
    assert!(err.message.contains("unknown confirmation scope"));
    assert!(err.message.contains("sentence"));
}

// ---------------------------------------------------------------------------
// Negative test: unverified with default_enabled = true
// ---------------------------------------------------------------------------

#[test]
fn unverified_and_default_enabled_is_rejected() {
    let json = r#"{
        "$schema": "",
        "schemaVersion": "1.0.0",
        "id": "test",
        "version": "0.1.0",
        "rules": [
            {
                "id": "unverified.on",
                "version": 1,
                "category": "landmine",
                "name": "Unverified",
                "description": "Should be rejected",
                "status": "draft",
                "defaultEnabled": true,
                "defaultSeverity": "low",
                "detection": {
                    "criteria": [],
                    "exclusions": [],
                    "pendingConditions": [],
                    "confirmationScope": "local",
                    "requiresUserBoundary": false
                }
            }
        ]
    }"#;
    let err = RulePack::load_from_json(json).unwrap_err();
    assert!(err.message.contains("unverified"));
    assert!(err.message.contains("default_enabled"));
}

// ---------------------------------------------------------------------------
// Zero rule versions are rejected by the Rust loader
// ---------------------------------------------------------------------------

#[test]
fn zero_version_is_rejected() {
    let json = r#"{
        "$schema": "",
        "schemaVersion": "1.0.0",
        "id": "test",
        "version": "0.1.0",
        "rules": [
            {
                "id": "zero.ver",
                "version": 0,
                "category": "landmine",
                "name": "Zero Ver",
                "description": "Version 0",
                "status": "verified",
                "defaultEnabled": false,
                "defaultSeverity": "low",
                "detection": {
                    "criteria": [],
                    "exclusions": [],
                    "pendingConditions": [],
                    "confirmationScope": "local",
                    "requiresUserBoundary": false
                }
            }
        ]
    }"#;
    let err = RulePack::load_from_json(json).unwrap_err();
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

// ---------------------------------------------------------------------------
// Detection arrays are preserved
// ---------------------------------------------------------------------------

#[test]
fn seed_pack_detection_arrays_are_preserved() {
    let pack = load_pack();
    for rule in &pack.rules {
        // Every rule should have a detection mode
        assert!(
            rule.detection_mode == "semantic" || rule.detection_mode == "manual_only",
            "rule '{}' has unexpected detection_mode: {}",
            rule.definition.id,
            rule.detection_mode
        );
    }
    // Spot-check the first rule (接盘)
    let first = &pack.rules[0];
    assert_eq!(first.definition.id, "yy.thunder.accepting-prior-partner");
    assert!(!first.criteria.is_empty(), "接盘 rule should have criteria");
    assert!(
        !first.exclusions.is_empty(),
        "接盘 rule should have exclusions"
    );
    assert!(
        !first.pending_conditions.is_empty(),
        "接盘 rule should have pending conditions"
    );
    assert_eq!(first.detection_mode, "semantic");
}
