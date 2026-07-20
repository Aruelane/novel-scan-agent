# 80｜最终需求—证据矩阵

每个阶段 gate 完成后填写“实现证据”和“验证证据”。没有可点击提交、测试、CI、安装或设备证据的条目不能标 `PASS`。

| 用户需求 | 主要阶段 | 最终完成定义 | 实现证据 | 验证证据 | 状态 |
| --- | --- | --- | --- | --- | --- |
| DeepSeek API 接入 | S4 | DeepSeek 兼容端点；provider 契约、提示词、SecretStore、wire schema | `novel-providers` crate: registry, prompt, schema, secret, retry, security (25 tests) | 25 tests pass; HG-001 真实 Key smoke 待用户 | PARTIAL |
| 人性化、非程序员味界面 | S1/S6 | 导入—选规则—扫描—看证据流程清晰；错误可恢复；键盘、读屏、小屏可用 | React 67 tests, WCAG AA 7/7, tabs ARIA, keyboard nav, 390/800/1440 e2e | 67 tests + Playwright e2e | PARTIAL |
| 基于 yy小说吧的雷点/郁闷点 | S5 | 11+21 条逐项来源/判据/排除/待确认/版本；无来源不默认启用 | 32 rules in seed pack, JSON Schema 2020-12, Ajv validation, rulepack crate (16 tests) | 16 tests + 6 negative schema tests | PARTIAL |
| 用户自主选择规则与等级 | S1/S5 | 全局/每书预设、开关、等级覆盖、历史快照和升级差异 | RuleSelector keyboard nav, RulePreset model, three-layer merge (5 tests) | 5 invariant tests pass | PARTIAL |
| 不限于 TXT 的多格式导入 | S2 | TXT/MD/HTML/EPUB/DOCX/文本 PDF 可解析回证；其他格式诚实分级 | TXT/Markdown Ready; GBK/GB18030; ZIP safety layer; EPUB/DOCX/HTML/PDF Pending (28 tests) | 28 tests pass; HTML/EPUB/DOCX/PDF parsers TODO | PARTIAL |
| 自动压缩长上下文 | S3 | 固定预算、滚动摘要、实体关系事件账本、未决候选、长书不线性扩张请求 | Memory ledger (4 types), ContextView budget, multi-window scan (52 tests) | 52 tests pass | PARTIAL |
| 命中标注原书章节和来源 | S2/S3 | confirmed/pending 的证据从原文重建并带格式相关 locator；摘要不能当证据 | EvidenceAnchor, SourceLocator, exact quote reconstruction in scanner | 52 tests pass | PARTIAL |
| 暂停、继续和崩溃恢复 | S3 | 每章事务、checkpoint、取消/重试/进程恢复、源文变化失效 | ScanCheckpoint, StopReason, ScanPersistence, fingerprint validation (52 tests) | 52 tests pass | PARTIAL |
| Windows 客户端 | S6/S7 | 安装、升级、文件选择、扫描闭环和发布资产 | Tauri 2 shell, cargo check passes, CI configured; link.exe resolved | cargo check passes; install/packaging TODO | PARTIAL |
| Android 客户端 | S6/S7 | SAF、持久授权、前台长任务、Keystore、真机闭环、签名 APK | Android debug CI workflow configured | CI config only; HG-003 device needed | BLOCKED |
| 不做 iOS | 全程 | 仓库无 iOS target、证书、workflow 或商店依赖 | `main-local.json` restricted to windows+android | grep confirms no iOS target | PASS |
| BYOK 且密钥安全 | S4/S7 | Key 仅经 Windows/Android 安全存储；前端/SQLite/日志/Git 无明文 | SecretStore, canary tests, credential_ref hardening, 104 migration tests | 104 migration + 25 provider tests; HG-001 real key pending | PARTIAL |
| GitHub 可重复开发与发布 | S1/S7 | CI 全绿、阶段 checkpoint、Windows/Android 构建、Release/校验值/说明 | 72 commits, CI workflows for frontend/rulepack/migration/rust/e2e | 201 tests pass locally; CI verification pending | PARTIAL |

## 状态定义

- `TODO`：未实现或未验证。
- `PARTIAL`：部分实现，但尚未满足完整定义。
- `BLOCKED`：实现完成度受已登记 HUMAN_GATE/EXTERNAL_BLOCKER 限制。
- `PASS`：实现与验证证据齐全，且没有相反的已知事实。
- `ACCEPTED_LIMITATION`：用户在最终发布门明确接受，必须链接到 debt/blocker 记录。

## 发布硬门

以下条目不允许使用 `ACCEPTED_LIMITATION` 绕过：

- 明文密钥或用户小说进入 Git/日志/SQLite。
- confirmed finding 无法从原文重建证据。
- 宣称支持的文件格式不能解析或不能定位来源。
- Android 发布资产无法安装或没有可验证的自建签名；Windows 资产无法安装、校验值不匹配，或没有诚实披露 signed/unsigned 状态。Windows 商业代码签名不是硬门，可由用户明确接受未签名 GitHub 侧载限制。
- CI 核心测试失败。
