//! Provider profile and credential management — core logic.
//! Tauri command wrappers register in lib.rs invoke_handler.
//! All DTOs camelCase, no secrets ever returned.

use novel_providers::secret::{ResolvedSecret, SecretStore};
use serde::Serialize;
use std::collections::HashMap;

use crate::secrets::windows::WindowsFileStore;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateDto {
    pub id: String,
    pub display_name: String,
    pub protocol: String,
    pub default_base_url: String,
    pub description: String,
    pub supports_streaming: bool,
    pub supports_tool_calls: bool,
    pub max_context_chars: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderProfile {
    pub id: String,
    pub display_name: String,
    pub protocol: String,
    pub base_url: String,
    pub model_id: String,
    pub timeout_seconds: u32,
    pub retry_max_attempts: u32,
    pub credential_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileDto {
    pub id: String,
    pub display_name: String,
    pub protocol: String,
    pub base_url: String,
    pub model_id: String,
    pub timeout_seconds: u32,
    pub retry_max_attempts: u32,
    pub credential_state: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialStateDto {
    pub state: String,
}

fn protocol_str(proto: &str) -> String {
    match proto {
        "openai_compatible" | "openai-compatible" => "openai_compatible".to_string(),
        "deepseek" => "deepseek".to_string(),
        "anthropic_native" | "anthropic-native" => "anthropic_native".to_string(),
        _ => proto.to_string(),
    }
}
fn credential_state(p: &ProviderProfile, store: &dyn SecretStore) -> String {
    (match &p.credential_ref {
        Some(r) => match store.resolve(r) {
            Ok(ResolvedSecret::Key(_)) => "configured",
            Ok(ResolvedSecret::Missing) => "missing",
            _ => "unavailable",
        },
        None => "missing",
    })
    .to_string()
}
fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
}
fn valid_url(url: &str) -> Result<(), String> {
    if url.is_empty() {
        return Err("URL empty".into());
    }
    let s = url.find("://").unwrap_or(0);
    if &url[..s] != "https" && &url[..s] != "http" {
        return Err("scheme must be https".into());
    }
    let rest = &url[s + 3..];
    if rest.contains('@') {
        return Err("URL must not contain userinfo".into());
    }
    if rest.split('/').next().unwrap_or("").is_empty() {
        return Err("URL must have host".into());
    }
    Ok(())
}

// ── Public logic functions ─────────────────────────────────────

pub fn list_templates() -> Vec<TemplateDto> {
    use novel_providers::config::ProviderProtocol;
    use novel_providers::registry::{builtin_templates, default_capabilities};
    builtin_templates()
        .into_iter()
        .filter(|t| t.protocol != ProviderProtocol::DeterministicTest)
        .map(|t| {
            let caps = default_capabilities(t.protocol);
            TemplateDto {
                id: t.id,
                display_name: t.display_name,
                protocol: protocol_str(&format!("{:?}", t.protocol).to_lowercase()),
                default_base_url: t.default_base_url,
                description: t.description,
                supports_streaming: caps.supports_streaming,
                supports_tool_calls: caps.supports_tool_calls,
                max_context_chars: caps.max_context_chars,
            }
        })
        .collect()
}

pub fn list_profiles(profiles: &HashMap<String, ProviderProfile>) -> Vec<ProfileDto> {
    let store = WindowsFileStore::new();
    profiles
        .values()
        .map(|p| ProfileDto {
            id: p.id.clone(),
            display_name: p.display_name.clone(),
            protocol: p.protocol.clone(),
            base_url: p.base_url.clone(),
            model_id: p.model_id.clone(),
            timeout_seconds: p.timeout_seconds,
            retry_max_attempts: p.retry_max_attempts,
            credential_state: credential_state(p, &store),
        })
        .collect()
}

pub fn upsert_profile(
    profiles: &mut HashMap<String, ProviderProfile>,
    id: String,
    display_name: String,
    protocol: String,
    base_url: String,
    model_id: String,
    timeout_seconds: u32,
    retry_max_attempts: u32,
) -> Result<ProfileDto, String> {
    if !valid_id(&id) {
        return Err(format!("invalid profile ID: {id}"));
    }
    valid_url(&base_url)?;
    let proto = protocol_str(&protocol);
    let existing_ref = profiles.get(&id).and_then(|p| p.credential_ref.clone());
    let p = ProviderProfile {
        id: id.clone(),
        display_name,
        protocol: proto.into(),
        base_url,
        model_id,
        timeout_seconds,
        retry_max_attempts,
        credential_ref: existing_ref,
    };
    let store = WindowsFileStore::new();
    let cs = credential_state(&p, &store);
    profiles.insert(id.clone(), p.clone());
    Ok(ProfileDto {
        id: p.id,
        display_name: p.display_name,
        protocol: p.protocol,
        base_url: p.base_url,
        model_id: p.model_id,
        timeout_seconds: p.timeout_seconds,
        retry_max_attempts: p.retry_max_attempts,
        credential_state: cs,
    })
}

pub fn set_credential(
    profiles: &mut HashMap<String, ProviderProfile>,
    profile_id: &str,
    api_key: &str,
) -> Result<String, String> {
    if api_key.is_empty() {
        return Err("API key must not be empty".into());
    }
    let store = WindowsFileStore::new();
    let p = profiles
        .get_mut(profile_id)
        .ok_or_else(|| format!("profile not found: {profile_id}"))?;
    let h = store
        .store(api_key, &format!("novel-scout:{profile_id}"))
        .map_err(|e| format!("secret store error: {e}"))?;
    if let Some(old) = p.credential_ref.replace(h) {
        let _ = store.delete(&old);
    }
    Ok("configured".into())
}

pub fn delete_credential(
    profiles: &mut HashMap<String, ProviderProfile>,
    profile_id: &str,
) -> Result<String, String> {
    let store = WindowsFileStore::new();
    let p = profiles
        .get_mut(profile_id)
        .ok_or_else(|| format!("profile not found: {profile_id}"))?;
    if let Some(old) = p.credential_ref.take() {
        store
            .delete(&old)
            .map_err(|e| format!("secret store error: {e}"))?;
    }
    Ok("missing".into())
}

pub fn get_credential_state(
    profiles: &HashMap<String, ProviderProfile>,
    profile_id: &str,
) -> Result<String, String> {
    let store = WindowsFileStore::new();
    let p = profiles
        .get(profile_id)
        .ok_or_else(|| format!("profile not found: {profile_id}"))?;
    Ok(credential_state(p, &store))
}

// ── App state ──────────────────────────────────────────────────

pub struct ProfileState {
    pub profiles: std::sync::Mutex<HashMap<String, ProviderProfile>>,
}
impl ProfileState {
    pub fn new() -> Self {
        Self {
            profiles: std::sync::Mutex::new(HashMap::new()),
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn map() -> HashMap<String, ProviderProfile> {
        HashMap::new()
    }

    #[test]
    fn templates_3_production() {
        let t = list_templates();
        assert_eq!(t.len(), 3);
        assert!(!t.iter().any(|x| x.id == "deterministic-test"));
    }

    #[test]
    fn template_dto_no_secrets() {
        assert!(!serde_json::to_string(&list_templates())
            .unwrap()
            .contains("api_key"));
    }

    #[test]
    fn upsert_and_list() {
        let mut m = map();
        upsert_profile(
            &mut m,
            "t1".into(),
            "T".into(),
            "deepseek".into(),
            "https://x.com/v1".into(),
            "m".into(),
            30,
            3,
        )
        .unwrap();
        assert_eq!(list_profiles(&m).len(), 1);
    }

    #[test]
    fn credential_round_trip() {
        let mut m = map();
        upsert_profile(
            &mut m,
            "c1".into(),
            "C".into(),
            "deepseek".into(),
            "https://x.com/v1".into(),
            "m".into(),
            30,
            3,
        )
        .unwrap();
        assert_eq!(
            set_credential(&mut m, "c1", "sk-test").unwrap(),
            "configured"
        );
        assert_eq!(get_credential_state(&m, "c1").unwrap(), "configured");
        assert_eq!(delete_credential(&mut m, "c1").unwrap(), "missing");
        assert_eq!(get_credential_state(&m, "c1").unwrap(), "missing");
    }

    #[test]
    fn dto_never_leaks_key() {
        let mut m = map();
        upsert_profile(
            &mut m,
            "s".into(),
            "S".into(),
            "deepseek".into(),
            "https://x.com/v1".into(),
            "m".into(),
            30,
            3,
        )
        .unwrap();
        set_credential(&mut m, "s", "sk-secret-12345").unwrap();
        let json = serde_json::to_string(&list_profiles(&m)).unwrap();
        assert!(!json.contains("sk-secret"));
        assert!(json.contains("configured"));
    }

    #[test]
    fn invalid_id_rejected() {
        assert!(upsert_profile(
            &mut map(),
            "".into(),
            "x".into(),
            "deepseek".into(),
            "https://x.com".into(),
            "m".into(),
            30,
            3
        )
        .is_err());
    }

    #[test]
    fn bad_url_rejected() {
        assert!(upsert_profile(
            &mut map(),
            "p".into(),
            "x".into(),
            "deepseek".into(),
            "https://u:p@h.com".into(),
            "m".into(),
            30,
            3
        )
        .is_err());
    }

    #[test]
    fn empty_key_rejected() {
        let mut m = map();
        upsert_profile(
            &mut m,
            "k".into(),
            "K".into(),
            "deepseek".into(),
            "https://x.com".into(),
            "m".into(),
            30,
            3,
        )
        .unwrap();
        assert!(set_credential(&mut m, "k", "").is_err());
    }

    #[test]
    fn delete_idempotent() {
        let mut m = map();
        upsert_profile(
            &mut m,
            "d".into(),
            "D".into(),
            "deepseek".into(),
            "https://x.com".into(),
            "m".into(),
            30,
            3,
        )
        .unwrap();
        assert!(delete_credential(&mut m, "d").is_ok());
        assert!(delete_credential(&mut m, "d").is_ok());
    }

    #[test]
    fn cross_profile_isolation() {
        let mut m = map();
        upsert_profile(
            &mut m,
            "a".into(),
            "A".into(),
            "deepseek".into(),
            "https://a.com".into(),
            "ma".into(),
            30,
            3,
        )
        .unwrap();
        upsert_profile(
            &mut m,
            "b".into(),
            "B".into(),
            "deepseek".into(),
            "https://b.com".into(),
            "mb".into(),
            30,
            3,
        )
        .unwrap();
        set_credential(&mut m, "a", "ka").unwrap();
        set_credential(&mut m, "b", "kb").unwrap();
        delete_credential(&mut m, "a").unwrap();
        assert_eq!(get_credential_state(&m, "a").unwrap(), "missing");
        assert_eq!(get_credential_state(&m, "b").unwrap(), "configured");
    }
}
