use crate::config::{ProviderCapabilities, ProviderProtocol, ProviderTemplate};

pub fn builtin_templates() -> Vec<ProviderTemplate> {
    vec![
        ProviderTemplate {
            id: "deepseek".into(),
            display_name: "DeepSeek".into(),
            protocol: ProviderProtocol::DeepSeek,
            default_base_url: "https://api.deepseek.com/v1".into(),
            description: "DeepSeek 在线模型，兼容 OpenAI Chat Completions 协议".into(),
        },
        ProviderTemplate {
            id: "deterministic-test".into(),
            display_name: "确定性测试 (非 AI)".into(),
            protocol: ProviderProtocol::DeterministicTest,
            default_base_url: "offline://deterministic-test".into(),
            description: "离线测试提供器，不调用任何模型".into(),
        },
    ]
}

pub fn production_templates_owned() -> Vec<ProviderTemplate> {
    builtin_templates()
        .into_iter()
        .filter(|t| t.protocol != ProviderProtocol::DeterministicTest)
        .collect()
}

pub fn default_capabilities(protocol: ProviderProtocol) -> ProviderCapabilities {
    match protocol {
        ProviderProtocol::DeepSeek => ProviderCapabilities {
            supports_streaming: true,
            supports_tool_calls: false,
            max_context_chars: Some(128_000),
        },
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
        assert_eq!(prod.len(), 1);
    }

    #[test]
    fn deepseek_is_in_builtin() {
        let all = builtin_templates();
        assert!(all.iter().any(|t| t.id == "deepseek"));
    }
}
