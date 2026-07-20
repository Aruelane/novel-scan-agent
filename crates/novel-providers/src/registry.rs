use crate::config::{ProviderCapabilities, ProviderProtocol, ProviderTemplate};

/// Built-in provider templates known to the application. Users can add custom
/// templates via the UI; this is the factory list.
pub fn builtin_templates() -> Vec<ProviderTemplate> {
    vec![
        ProviderTemplate {
            id: "openai-custom".into(),
            display_name: "OpenAI Compatible".into(),
            protocol: ProviderProtocol::OpenAiCompatible,
            default_base_url: "https://api.openai.com/v1".into(),
            description: "兼容 OpenAI Chat Completions 协议的在线模型".into(),
        },
        ProviderTemplate {
            id: "deepseek-custom".into(),
            display_name: "DeepSeek Compatible".into(),
            protocol: ProviderProtocol::OpenAiCompatible,
            default_base_url: "https://api.deepseek.com/v1".into(),
            description: "DeepSeek 在线模型，兼容 OpenAI Chat Completions 协议".into(),
        },
        ProviderTemplate {
            id: "anthropic-native".into(),
            display_name: "Anthropic Claude".into(),
            protocol: ProviderProtocol::AnthropicNative,
            default_base_url: "https://api.anthropic.com".into(),
            description: "通过 Anthropic Messages API 直接调用 Claude 模型".into(),
        },
        ProviderTemplate {
            id: "deterministic-test".into(),
            display_name: "确定性测试 (非 AI)".into(),
            protocol: ProviderProtocol::DeterministicTest,
            default_base_url: "offline://deterministic-test".into(),
            description: "离线测试提供器，不调用任何模型。仅用于测试和开发".into(),
        },
    ]
}

/// Production-facing templates (excludes test-only providers).
pub fn production_templates() -> Vec<ProviderTemplate> {
    builtin_templates()
        .into_iter()
        .filter(|t| t.protocol != ProviderProtocol::DeterministicTest)
        .collect()
}

/// Returns the production-ready templates as owned values.
pub fn production_templates_owned() -> Vec<ProviderTemplate> {
    builtin_templates()
        .into_iter()
        .filter(|t| t.protocol != ProviderProtocol::DeterministicTest)
        .collect()
}

/// Default capabilities for a protocol. These are conservative estimates;
/// actual capabilities are negotiated at connection time.
pub fn default_capabilities(protocol: ProviderProtocol) -> ProviderCapabilities {
    match protocol {
        ProviderProtocol::OpenAiCompatible | ProviderProtocol::AnthropicNative => {
            ProviderCapabilities {
                supports_streaming: true,
                supports_tool_calls: false,
                max_context_chars: Some(128_000),
            }
        }
        ProviderProtocol::DeterministicTest => ProviderCapabilities {
            supports_streaming: false,
            supports_tool_calls: false,
            max_context_chars: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_template_ids_are_unique() {
        let mut ids = std::collections::HashSet::new();
        for t in builtin_templates() {
            assert!(ids.insert(t.id.clone()), "duplicate template id: {}", t.id);
        }
    }

    #[test]
    fn production_excludes_test_provider() {
        let prod = production_templates_owned();
        assert!(!prod
            .iter()
            .any(|t| t.protocol == ProviderProtocol::DeterministicTest));
        assert!(prod.len() >= 3);
    }

    #[test]
    fn deterministic_test_is_in_builtin() {
        let all = builtin_templates();
        assert!(all
            .iter()
            .any(|t| t.protocol == ProviderProtocol::DeterministicTest));
    }
}
