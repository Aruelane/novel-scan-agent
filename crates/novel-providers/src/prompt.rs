//! Unified scan prompt builder.
//!
//! Constructs a provider-agnostic prompt from an `InferenceRequest`. The novel
//! text is always wrapped in explicit boundary markers to prevent prompt injection.
//! Output format requirements are protocol-independent; individual adapters map
//! them to their native tool/function-calling or system/user message conventions.

use novel_core::{InferenceRequest, RuleContext};

/// Wraps novel text with boundary markers to separate it from scan instructions.
pub const CHAPTER_START_MARKER: &str = "─── 章节开始 ───";
pub const CHAPTER_END_MARKER: &str = "─── 章节结束 ───";

/// Builds the system prompt instructing the model how to scan.
pub fn build_system_prompt(rules: &[RuleContext]) -> String {
    let rule_lines: Vec<String> = rules
        .iter()
        .enumerate()
        .map(|(i, rule)| {
            format!(
                "{}. **{}** ({}): {}",
                i + 1,
                rule.name,
                severity_label(rule.alert_level),
                rule.description
            )
        })
        .collect();

    format!(
        "你是一个小说内容分析助手。你的任务是根据以下规则检查提供的章节文本，\
         找出可能的匹配项。\n\n\
         ## 扫描规则\n\n\
         {}\n\n\
         ## 输出格式\n\n\
         对于每个匹配，输出包含以下字段的 JSON 对象：\n\
         - rule_id: 匹配的规则 ID\n\
         - confidence_bps: 置信度 (0-10000)\n\
         - rationale: 匹配理由\n\
         - evidence_ranges: 原文证据的 UTF-8 字节范围数组\n\n\
         ## 重要约束\n\n\
         - 只分析提供的章节文本，不要添加未在文本中出现的内容\n\
         - 每个证据必须能精确对应到原文中的具体位置\n\
         - 不要输出章节文本本身，只输出分析结果\n\
         - 如果没有匹配，返回空的 candidates 数组",
        rule_lines.join("\n")
    )
}

/// Builds the user message containing the chapter text wrapped in markers.
pub fn build_user_message(request: &InferenceRequest) -> String {
    format!(
        "{}「{}」\n\n{}{}",
        CHAPTER_START_MARKER, request.chapter.title, request.chapter.text, CHAPTER_END_MARKER
    )
}

fn severity_label(level: novel_core::AlertLevel) -> &'static str {
    match level {
        novel_core::AlertLevel::Critical => "致命",
        novel_core::AlertLevel::High => "高",
        novel_core::AlertLevel::Medium => "中",
        novel_core::AlertLevel::Low => "低",
        novel_core::AlertLevel::Info => "提示",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use novel_core::{
        AlertLevel, Chapter, ConfirmationScope, ContextSnapshot, DetectionMode, RuleCategory,
        SourceLocator,
    };

    #[test]
    fn system_prompt_contains_all_rule_names() {
        let rules = vec![RuleContext {
            id: "r1".into(),
            version: 1,
            name: "测试规则A".into(),
            description: "描述A".into(),
            category: RuleCategory::Landmine,
            alert_level: AlertLevel::Critical,
            confirmation_scope: ConfirmationScope::Chapter,
            requires_user_boundary: false,
            detection_mode: DetectionMode::Semantic,
            detection_profile_ref: None,
            criteria: vec!["test".into()],
            exclusions: vec!["test".into()],
            pending_conditions: vec!["test".into()],
        }];
        let prompt = build_system_prompt(&rules);
        assert!(prompt.contains("测试规则A"));
        assert!(prompt.contains("致命"));
    }

    #[test]
    fn user_message_wraps_chapter_text() {
        let chapter = Chapter::new(
            "c1",
            0,
            "序章",
            "正文内容",
            SourceLocator::Unknown {
                description: "x".into(),
            },
        );
        let request = InferenceRequest {
            task_id: "t1".into(),
            document_id: "d1".into(),
            chapter,
            rules: vec![],
            context: ContextSnapshot::default(),
        };
        let msg = build_user_message(&request);
        assert!(msg.contains(CHAPTER_START_MARKER));
        assert!(msg.contains(CHAPTER_END_MARKER));
        assert!(msg.contains("序章"));
        assert!(msg.contains("正文内容"));
    }
}
