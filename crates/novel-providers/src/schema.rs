//! Shared structured output wire schema.
//!
//! Both OpenAI-compatible and Anthropic-native protocols map their responses
//! into this common contract before passing them to novel-core.

use novel_core::ProviderEvidenceRange;
use serde::{Deserialize, Serialize};

/// Maximum size of a provider response body that will be parsed.
pub const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024; // 2 MB

/// Wire-format candidate from any supported protocol. This is what the
/// protocol adapters deserialize and then validate before converting to
/// `ProviderCandidate`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct WireCandidate {
    pub rule_id: String,
    pub confidence_bps: u16,
    pub rationale: String,
    #[serde(default)]
    pub requires_later_confirmation: bool,
    #[serde(default)]
    pub evidence_ranges: Vec<WireEvidenceRange>,
}

/// Wire-format evidence range. Byte offsets are relative to the chapter
/// window text, not the full chapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireEvidenceRange {
    pub utf8_byte_start: usize,
    pub utf8_byte_end: usize,
}

impl From<WireEvidenceRange> for ProviderEvidenceRange {
    fn from(w: WireEvidenceRange) -> Self {
        Self {
            utf8_byte_start: w.utf8_byte_start,
            utf8_byte_end: w.utf8_byte_end,
        }
    }
}

/// Wire-format response content from the model. Protocol adapters deserialize
/// into this, validate, then convert to `ProviderResponse`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct WireResponse {
    #[serde(default)]
    pub candidates: Vec<WireCandidate>,
    #[serde(default)]
    pub usage_input: u64,
    #[serde(default)]
    pub usage_output: u64,
}

/// Validates a wire response. Returns structured errors, never panics on
/// untrusted input.
pub fn validate_wire_response(response: &WireResponse) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    if response.candidates.is_empty() {
        // Empty candidates is valid (no findings in this window)
        return Ok(());
    }

    for (i, candidate) in response.candidates.iter().enumerate() {
        if candidate.rule_id.is_empty() {
            errors.push(format!("candidate[{i}]: empty rule_id"));
        }
        if candidate.confidence_bps > 10_000 {
            errors.push(format!(
                "candidate[{i}]: confidence {} exceeds 10000",
                candidate.confidence_bps
            ));
        }
        if candidate.rationale.len() > 2000 {
            errors.push(format!(
                "candidate[{i}]: rationale too long ({} chars)",
                candidate.rationale.len()
            ));
        }
        for (j, range) in candidate.evidence_ranges.iter().enumerate() {
            if range.utf8_byte_start >= range.utf8_byte_end {
                errors.push(format!("candidate[{i}].evidence_ranges[{j}]: start >= end"));
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_wire_response_passes() {
        let response = WireResponse {
            candidates: vec![WireCandidate {
                rule_id: "r1".into(),
                confidence_bps: 5000,
                rationale: "test".into(),
                requires_later_confirmation: false,
                evidence_ranges: vec![WireEvidenceRange {
                    utf8_byte_start: 0,
                    utf8_byte_end: 10,
                }],
            }],
            usage_input: 100,
            usage_output: 50,
        };
        assert!(validate_wire_response(&response).is_ok());
    }

    #[test]
    fn empty_candidates_is_valid() {
        let response = WireResponse::default();
        assert!(validate_wire_response(&response).is_ok());
    }

    #[test]
    fn confidence_above_10000_rejected() {
        let response = WireResponse {
            candidates: vec![WireCandidate {
                rule_id: "r1".into(),
                confidence_bps: 10_001,
                rationale: "test".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        let errors = validate_wire_response(&response).unwrap_err();
        assert!(errors.iter().any(|e| e.contains("10000")));
    }
}
