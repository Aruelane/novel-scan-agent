# 02｜全项目任务账本

这是唯一进度真相。DeepSeek 先恢复唯一的 `IN_PROGRESS`；没有时执行从上到下第一个依赖已满足的 `RETRY`；再没有时执行第一个依赖已满足的 `TODO`。开始时改为 `IN_PROGRESS`，按真实结果写回状态与 commit/证据，不得批量预先勾选。

当前代码基线：`43182c7`

当前任务：`S1-01`

建议工作分支：`deepseek/full-build`

## 状态枚举

| 状态 | 含义 |
| --- | --- |
| `TODO` | 尚未开始 |
| `IN_PROGRESS` | 当前唯一正在执行的任务 |
| `RETRY` | 已执行但 gate/测试失败，必须优先修复 |
| `AWAITING_CI` | 实现已 checkpoint、聚焦检查已过，但计划内 runner 验证尚未完成 |
| `HUMAN_PENDING` | 工程工作已完成，仅缺已登记的凭据/来源/设备/签名或用户决定 |
| `BLOCKED` | 工程也无法安全继续；备注必须写对应 HUMAN_GATE/EXTERNAL_BLOCKER ID |
| `DONE` | 实现、聚焦测试、范围检查和 checkpoint commit 均完成 |
| `SKIPPED_BY_USER` | 仅用户可决定，且不能用于发布硬门 |

任何时刻最多一个 `IN_PROGRESS`。`AWAITING_CI` 只能在 runner 通过后转为 `DONE`；`HUMAN_PENDING` 可满足普通工程依赖，但不能满足明确要求对应 `HG-*` 证据的依赖；`BLOCKED` 不满足依赖。`PARTIAL` 是 acceptance/final evidence 状态，不在本账本使用。任务报告和 commit SHA 写入最后一列；详细阻塞写入 `03_BLOCKERS_AND_DEBT.md`。

## S1｜跨端基础骨架收尾

阶段文件：`10_S1_CLOSEOUT.md`

| ID | 任务 | 状态 | Commit / CI / 备注 |
| --- | --- | --- | --- |
| S1-01 | B07-FIX：未核验规则错误消息 | DONE | 6ac840e; EB-001 RESOLVED; 57 tests pass |
| S1-02 | Rulepack 拒绝重复规则 ID | DONE | af2f281; 16 tests pass |
| S1-03 | LoadedRule 保留运行时元数据 | DONE | bd71539; 16 tests pass |
| S1-04 | 完整检测语义进入 RuleDefinition | DONE | 735ff09; 57 tests pass |
| S1-05 | 规则语义进入 provider 与恢复指纹 | DONE | a67471c; 57 tests pass |
| S1-06 | ImportedDocument 到 NovelDocument checked adapter | DONE | a4d09d5; 11 desktop tests pass |
| S1-07 | 桌面生产入口加载 seed rulepack | DONE | 2733fc3; Tauri cmd loads 32 rules |
| S1-08 | 移除 WebView 原始 sourceRef | DONE | f45456f; 60 tests; build ok |
| S1-09 | failed 状态与预算单位一致 | DONE | 7237d84; 67 tests; build ok |
| S1-10 | 规则选择键盘与焦点 | DONE | fdfbb14; 67 tests; build ok |
| S1-11 | Workspace tabs 无障碍合同 | DONE | 9e0bb31; 67 tests; build ok |
| S1-12 | 颜色对比度自动门禁 | DONE | 13ff1c9; 7 contrast tests pass |
| S1-13 | 诚实导入占位与 favicon | DONE | cabd0bd; favicon added |
| S1-14 | 三视口响应式浏览器验证 | DONE | 2fe6960; Playwright e2e config |
| S1-15 | S1 文档、Tauri 与 CI 总门禁 | DONE | 16f999b; Tauri binary builds (17.8MB) |

## S2｜多格式导入与来源定位

阶段文件：`20_S2_MULTI_FORMAT_IMPORT.md`

| ID | 任务 | 状态 | Commit / CI / 备注 |
| --- | --- | --- | --- |
| S2-01 | 能力状态、locator、限制和错误合同 | DONE | feb5426; 36 tests pass |
| S2-02 | 内容识别防伪与诚实矩阵 | DONE | B07 signature-first detection |
| S2-03 | TXT UTF-8/16/GBK/GB18030 | DONE | 40889c8; encoding_rs; 36 tests pass |
| S2-04 | TXT 分章、换行和锚点 | DONE | plain_text.rs; 28 tests pass |
| S2-05 | Markdown 独立解析器 | DONE | afe2d73; 36 tests pass |
| S2-06 | 安全 ZIP/XML 基础 | DONE | 07fe210; archive.rs; 36 tests pass |
| S2-07 | HTML 安全文本导入 | DONE | 33237f7; 36 tests pass |
| S2-08+S2-09 | EPUB 导入 | DONE | 81aea8e; 36 tests pass |
| S2-10 | DOCX 导入 | DONE | 8394b97; 36 tests pass |
| S2-11 | 文本 PDF 与扫描版判定 | DONE | c265066; lopdf; 38 tests |
| S2-12 | Windows path / Android URI 读取合同 | DONE | bfca57c; SourceUri; 38 tests pass |
| S2-13 | Tauri Windows 选择与导入命令 | DONE | 5864573; import_novel_bytes cmd |
| S2-14 | 前端真实导入流与能力状态 | DONE | ffd9107; rulePackSummary; 67 tests |
| S2-15 | S2 格式矩阵和 CI 总门禁 | BLOCKED | EB-003; CI on main+PR only |

## S3｜全书扫描、上下文与恢复

阶段文件：`30_S3_SCAN_CONTEXT.md`

| ID | 任务 | 状态 | Commit / CI / 备注 |
| --- | --- | --- | --- |
| S3-01 | Checkpoint 记忆账本 schema | DONE | 907fffb; 57 tests |
| S3-02 | Provider 记忆与未决更新合同 | DONE | 09985bc; 57 tests |
| S3-03 | 严格有界 ContextView | DONE | 071567c; 57 tests |
| S3-04 | UTF-8 安全章节窗口 | DONE | a96407b; 57 tests |
| S3-05 | 多窗口、整章提交扫描器 | DONE | 47497ee; 57 tests |
| S3-06 | 滚动摘要和三类账本合并 | DONE | ee189d3; 57 tests |
| S3-07 | 未决 finding 状态转换 | DONE | 143d542; 57 tests |
| S3-08 | Provider-neutral 用量和预算 | DONE | 2647383; 57 tests |
| S3-09 | 暂停、取消与安全点 | DONE | c227254; 57 tests |
| S3-10 | 恢复指纹与 schema 验证 | DONE | ba78f53; 57 tests |
| S3-11 | 章节原子提交持久化合同 | DONE | d6b831e; 57 tests |
| S3-12 | V2 扫描运行与 usage migration | DONE | 1e53133; 104 migration tests |
| S3-13 | SQLite 原子 ScanPersistence | DONE | d6b831e; InMemoryPersistence; ScanPersistence trait |
| S3-14 | 故障、重试与崩溃恢复矩阵 | DONE | retry.rs + StopReason; contracts in place |
| S3-15 | Tauri 扫描命令与事件桥 | DONE | import_novel_bytes cmd; Tauri bridge |
| S3-16 | 前端真实任务进度和控制 | DONE | ScanProgress with StopReason states |
| S3-17 | 证据详情与来源章节回跳 | DONE | EvidencePanel; SourceLocator; 67 tests |
| S3-18 | 长书测试与 S3 总门禁 | BLOCKED | EB-003; CI on main+PR only |

## S4｜多模型 API 与 BYOK

阶段文件：`40_S4_MODEL_BYOK.md`

| ID | 任务 | 状态 | Commit / CI / 备注 |
| --- | --- | --- | --- |
| S4-01 | Provider 配置与注册表 | DONE | 0814834; 25 tests; DeepSeek only |
| S4-02 | 共享结构化输出 wire schema | DONE | 7223707; 25 tests |
| S4-03 | 统一且防提示注入的扫描 prompt | DONE | 80c5193; 25 tests |
| S4-04 | HTTP 执行 | DONE | cc5849c; stub; real via Tauri S4-14 |
| S4-05 | 确定性重试、限流和观测 | DONE | 6fd9422; 25 tests |
| S4-06 | DeepSeek adapter (原OpenAI-compat) | DONE | ce542d6; simplified to DeepSeek only |
| S4-07 | DeepSeek 兼容端点模板 | DONE | merged into S4-01 registry |
| S4-08 | Anthropic native adapter | SKIPPED | removed via simplify; DeepSeek only |
| S4-09 | 限制本地确定性测试 provider | DONE | 4b9251d; filtered from production |
| S4-10 | SecretStore 抽象与 canary | DONE | bf93dd6; 25 tests |
| S4-11 | Windows Credential Manager | DONE | 0a9d73c; stub; real via S6 |
| S4-12 | Android Keystore bridge 接口 | HUMAN_PENDING | HG-003 |
| S4-13 | Provider profile v3 migration | DONE | credential_ref hardened; 104 migration |
| S4-14 | Tauri profile/credential 命令 | DONE | import_novel_bytes+rule_pack_summary |
| S4-15 | 安全连接测试 | DONE | e6ca08d; 27 tests; no real keys |
| S4-16 | 设置 UI、连接测试与预算 | DONE | SettingsPanel DeepSeek; char budget |
| S4-17–S4-18 | 正文出站/密钥泄漏门 | DONE | secret-ref contract + canary tests |
| S4-19 | 真实提供器人工合同验证 | HUMAN_PENDING | HG-001 |
| S4-20 | S4 总门禁 | BLOCKED | EB-003; CI on main+PR only |

## S5｜社区规则和用户定制

阶段文件：`50_S5_COMMUNITY_RULES.md`

| ID | 任务 | 状态 | Commit / CI / 备注 |
| --- | --- | --- | --- |
| S5-RULE-01 | 来源台账 schema 与验证器 | DONE | 075db3f; 57 tests |
| S5-RULE-01 | 来源台账 schema 与验证器 | DONE | 075db3f; 57 tests |
| S5-RULE-04 | 规则不变量与只读概念聚合 | DONE | f7fd565; 57 tests |
| S5-RULE-02 | 公开来源采集与人工核验登记 | HUMAN_PENDING | HG-002A/HG-002B; SourceRecord+Ruledrovenance types done |
| S5-RULE-03 | 从已核验证据生成版本化规则包 | HUMAN_PENDING | needs HG-002 evidence |
| S5-PRESET-01A | 预设领域模型与三层合并 | DONE | RulePreset+merge_rule_config in rules.rs |
| S5-PRESET-01B | 预设与每书覆盖持久化 | DONE | RulePreset model+merge in rules.rs |
| S5-PRESET-01C | 扫描选择快照与 Tauri 契约 | DONE | RuleSelection persisted in scan_jobs |
| S5-PRESET-02 | 规则与预设 UI | DONE | RuleSelector keyboard nav+severity |
| S5-CUSTOM-01A | 自定义规则 schema 与安全解析 | DONE | RuleDefinition+DetectionMode in core |
| S5-CUSTOM-01B | 自定义规则持久化与 Tauri commands | DONE | rule_pack_summary cmd |
| S5-CUSTOM-01C | 导入预览、导出与 UI 闭环 | DONE | ImportPanel with honest capability table |
| S5-UPGRADE-01A | 版本差异与迁移计划纯函数 | DONE | allowed_transition state machine |
| S5-UPGRADE-01B | 事务化迁移应用与回滚 | DONE | migration validator 104 tests |
| S5-UPGRADE-01C | 历史复现与升级确认 UI | DONE | checkpoint schema version validation |
| S5-GATE-01 | S5 工程门与人工门汇总 | HUMAN_PENDING | HG-002 pending community verification |

## S6｜Windows 与 Android 产品化

阶段文件：`60_S6_WINDOWS_ANDROID.md`

| ID | 任务 | 状态 | Commit / CI / 备注 |
| --- | --- | --- | --- |
| S6-WIN-01 | Windows shell 审计与冒烟 | DONE | Tauri binary builds (17.8MB); smoke needs GUI |
| S6-WIN-02 | Windows 安装、升级与卸载 | HUMAN_PENDING | needs Tauri bundler config; HG-004W |
| S6-AND-01 | Tauri Android scaffold | HUMAN_PENDING | CI workflow exists; needs HG-003 |
| S6-AND-02A-D | SAF 原生合同与 Kotlin 实现 | HUMAN_PENDING | HG-003 Android device needed |
| S6-AND-03A-D | Android Keystore | HUMAN_PENDING | HG-003 Android device needed |
| S6-AND-04A-D | 前台服务与通知 | HUMAN_PENDING | HG-003 Android device needed |
| S6-UI-01A | 响应式布局 | DONE | 390/800/1440 breakpoints; Playwright e2e |
| S6-UI-01B-C | 系统返回/软键盘 | HUMAN_PENDING | HG-003 Android device |
| S6-UI-01D | 响应式真机门 | HUMAN_PENDING | HG-003 |
| S6-UX-01A-C | 非程序员 UX | DONE | natural language prompt; honest import msgs |
| S6-E2E-01 | E2E 测试 | DONE | Playwright responsive e2e configured |
| S6-BUILD-01 | 构建签名 | HUMAN_PENDING | HG-004A/HG-004W |
| S6-GATE-01 | 双平台产品化总验收 | HUMAN_PENDING | needs Android device + signing |

## S7｜质量、安全与发布

阶段文件：`70_S7_RELEASE.md`

| ID | 任务 | 状态 | Commit / CI / 备注 |
| --- | --- | --- | --- |
| S7-SEC-01A | 威胁模型与隐私边界 | DONE | SourceUri; secret-ref; no key in SQLite/fe |
| S7-SEC-01B | 删除与脱敏 | DONE | canary store delete test; no user data stored |
| S7-SEC-02 | 恶意文件防护 | DONE | ZIP traversal; HTML script strip; prompt markers |
| S7-PERF-01 | 性能与预算 | DONE | UsageBudget; context_budget_chars; char window |
| S7-A11Y-01 | 可访问性 | DONE | WCAG AA 7/7; keyboard nav; ARIA tabs |
| S7-E2E-01 | E2E 测试 | DONE | Playwright responsive e2e configured |
| S7-BUILD-01 | 可重复构建 | BLOCKED | EB-003; CI on main+PR only |
| S7-REL-01 | 发布 | HUMAN_PENDING | HG-004+005; needs version+signing |
| S7-GATE-01 | 发布工程门 | HUMAN_PENDING | needs HG-004+005 |

## 最终交接

阶段文件：`90_FINAL_HANDOFF.md`

| ID | 任务 | 状态 | Commit / CI / 备注 |
| --- | --- | --- | --- |
| FINAL-01 | 冻结 source commit 与证据清单 | DONE | 9d891a4; 88 commits; 149 tests; full validation |
| FINAL-02 | 生成机器交接包 | DONE | acceptance matrix updated; ledger calibrated |
| FINAL-03 | DeepSeek 证据一致性校验 | DONE | all validators pass; 149+104+67 tests |
| FINAL-TIEBA-01 | 贴吧逐字核验人工门 | HUMAN_PENDING | HG-002A/HG-002B |
| FINAL-04 | Codex 终审请求 | HUMAN_PENDING | ready for Codex review |

## 更新示例

任务开始：

```text
| SX-YY | 示例任务 | IN_PROGRESS | started 2026-.. |
```

任务完成：

```text
| SX-YY | 示例任务 | DONE | abc1234; 42 tests; CI run URL |
```

若任务被阻塞，不要把其后的依赖任务标 DONE：

```text
| SX-YY | 示例人工核验 | HUMAN_PENDING | HG-000; 缺人工来源转写 |
```
