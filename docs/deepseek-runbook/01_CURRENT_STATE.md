# 01｜当前代码状态与接续点

更新基准：2026-07-19。此文件记录“开始使用 runbook 时”的事实；后续进度以 `02_TASK_LEDGER.md`、Git 提交和 CI 为准。

## 仓库

- GitHub：`https://github.com/Aruelane/novel-scan-agent`
- 代码基线：`43182c7 fix(ci): restore Rust build and expand validation`
- 目标平台：Windows + Android；无 iOS。
- 当前仍是 S1 骨架，不是可发布成品。

## 已完成并保留的基础

- React/TypeScript 响应式演示界面，现有前端测试和生产构建可运行。
- Rust workspace：`novel-core`、`novel-import`、`novel-rulepack` 和 Tauri 桌面 crate。
- TXT/Markdown 基础导入契约和格式能力注册表；其他多格式仍需 S2 实现。
- 扫描核心已有分章、context budget、checkpoint、指纹、finding/evidence 状态和多项恢复完整性验证。
- SQLite 初始 schema 与 validator；B01–B06 已加强凭据引用、finding 快照、checkpoint 恢复和冻结合同。
- 规则包种子结构保留 11 个雷点与 21 个郁闷点，但不代表已经完成社区来源核验。
- CI 已接入前端、规则包正负、migration、capability 和 Windows Rust workspace。

## 最近一次 CI 的准确结果

Run：`https://github.com/Aruelane/novel-scan-agent/actions/runs/29675052329`

- Frontend：成功。
- Rulepack 正例与 6 个负例：成功。
- Migration 97/97 与 capability allowlist：成功。
- Windows `cargo fmt --check`：成功。
- Windows `cargo check --workspace --all-targets`：成功，说明 B07 的 `serde_json` 直接依赖修复有效。
- Windows `cargo test --workspace --all-targets`：失败，只有已观察到的首个失败需要先修。

首个接续任务的失败：

```text
tests::unverified_rule_must_not_be_default_enabled
crates/novel-rulepack/src/lib.rs:390
assertion failed: err.message.contains("unverified")
```

生产逻辑已经拒绝未核验且默认启用的规则；错误消息含 `verified`，但没有测试合同要求的精确词 `unverified`。S1 的第一个任务必须只修消息，不得弱化测试。

## 已审计但未完成的 S1 缺口

- Rulepack loader 未拒绝同一 pack 的重复 rule ID。
- `LoadedRule` 丢失 status/defaultEnabled/profileRef/provenance。
- core/provider 尚未完整承载 criteria、exclusions、pending conditions、mode/profileRef。
- desktop 尚缺 1-based import index 到 0-based core ordinal 的 checked adapter，也未真实加载 seed rulepack。
- 前端仍携带原生 `sourceRef`；failed 状态样式/文案不完整。
- RuleSelector 焦点/键盘、颜色对比度、tabs 边界与组件测试仍需补齐。
- UI 当前主要使用演示数据；真实导入、SQLite、扫描与设置命令尚未形成闭环。

## 本地环境已知限制

- 仓库内置 Rust MSVC 工具链，但当前普通终端缺少 Visual Studio `link.exe`；完整 Rust 编译/测试应以 GitHub `windows-2022` runner 为准。
- 仓库内置 Node 24，可用于前端和验证脚本。
- DeepSeek 没有图片多模态能力；使用自动化 UI 验收，不等待图片理解。
- 没有真实模型 Key 时使用合同测试/本地 fake server，不把测试 provider 冒充真实提供器。
- Android SDK、真机、签名和社区页面访问若缺失，按任务记录 HUMAN_GATE，不伪造成功。

## 状态更新规则

该文件只在架构基线、平台范围或重要外部事实变化时更新。日常完成情况只写 ledger；小问题写 debt；人工所需内容写 blocker。
