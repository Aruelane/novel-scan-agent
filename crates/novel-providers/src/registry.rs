use crate::config::{ProviderCapabilities, ProviderProtocol, ProviderTemplate};

pub fn builtin_templates() -> Vec<ProviderTemplate> {
    vec![
        ProviderTemplate {
            id: "openai-compatible".into(),
            display_name: "OpenAI 兼容".into(),
            protocol: ProviderProtocol::OpenAICompatible,
            default_base_url: "https://api.openai.com/v1".into(),
            description: "通用 OpenAI Chat Completions 兼容端点，可用于 OpenAI、Ollama、vLLM、LocalAI 等服务".into(),
        },
        ProviderTemplate {
            id: "deepseek".into(),
            display_name: "DeepSeek".into(),
            protocol: ProviderProtocol::DeepSeek,
            default_base_url: "https://api.deepseek.com/v1".into(),
            description: "DeepSeek 在线模型，兼容 OpenAI Chat Completions 协议".into(),
        },
        ProviderTemplate {
            id: "anthropic".into(),
            display_name: "Anthropic".into(),
            protocol: ProviderProtocol::AnthropicNative,
            default_base_url: "https://api.anthropic.com/v1".into(),
            description: "Anthropic Claude 模型原生 Messages API".into(),
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
        ProviderProtocol::OpenAICompatible => ProviderCapabilities {
            supports_streaming: true,
            supports_tool_calls: true,
            max_context_chars: Some(128_000),
        },
        ProviderProtocol::DeepSeek => ProviderCapabilities {
            supports_streaming: true,
            supports_tool_calls: false,
            max_context_chars: Some(128_000),
        },
        ProviderProtocol::AnthropicNative => ProviderCapabilities {
            supports_streaming: true,
            supports_tool_calls: true,
            max_context_chars: Some(200_000),
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
        // OpenAI-compatible, DeepSeek, Anthropic = 3 production templates
        assert_eq!(prod.len(), 3);
    }

    #[test]
    fn all_production_protocols_in_builtin() {
        let all = builtin_templates();
        assert!(all.iter().any(|t| t.id == "openai-compatible"));
        assert!(all.iter().any(|t| t.id == "deepseek"));
        assert!(all.iter().any(|t| t.id == "anthropic"));
    }
}
