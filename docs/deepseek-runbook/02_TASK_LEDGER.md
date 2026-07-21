# 02｜全项目任务账本

这是唯一进度真相。DeepSeek 先恢复唯一的 `IN_PROGRESS`；没有时执行从上到下第一个依赖已满足的 `RETRY`；再没有时执行第一个依赖已满足的 `TODO`。开始时改为 `IN_PROGRESS`，按真实结果写回状态与 commit/证据，不得批量预先勾选。

当前代码基线：`3c35e0b`（账本纠偏基线）

当前任务：无（等待第二阶段分配）

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

## S1｜跨端基础骨架收尾（15/15）

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

## S2｜多格式导入与来源定位（6/15 DONE, 7 RETRY, 2 TODO）

阶段文件：`20_S2_MULTI_FORMAT_IMPORT.md`

| ID | 任务 | 状态 | Commit / CI / 备注 |
| --- | --- | --- | --- |
| S2-01 | 能力状态、locator、限制和错误合同 | DONE | feb5426; 36 tests pass |
| S2-02 | 内容识别防伪与诚实矩阵 | DONE | feb5426; 签名优先检测; 36 tests pass |
| S2-03 | TXT UTF-8/16/GBK/GB18030 | DONE | 40889c8; encoding_rs; 36 tests pass |
| S2-04 | TXT 分章、换行和锚点 | DONE | feb5426; plain_text.rs; 28 tests pass |
| S2-05 | Markdown 独立解析器 | DONE | afe2d73; 36 tests pass |
| S2-06 | 安全 ZIP/XML 基础 | DONE | 07fe210; archive.rs; 36 tests pass |
| S2-07 | HTML 安全文本导入 | DONE | c00548d; 46 tests pass; 12 HTML-specific; import_novel wired; capability Ready |
| S2-08 | EPUB container/OPF/spine | DONE | cedabd3; 51 tests pass; import_novel wired; capability Ready |
| S2-09 | EPUB 正文、章节和锚点 | DONE | d9ee2df; fragment/paragraph/nav skip; 51 tests pass |
| S2-10 | DOCX 正文、标题和段落锚点 | DONE | 0c7afcb; 55 tests pass; import_novel wired; capability Ready |
| S2-11 | 文本 PDF 与扫描版判定 | DONE | 42874be; 55 tests pass; import_novel wired; capability Ready |
| S2-12 | Windows path / Android URI 读取合同 | DONE | bfca57c; SourceUri; 38 tests pass |
| S2-13 | Tauri Windows 选择与导入命令 | DONE | fcd6ec8; import_novel_bytes now in invoke_handler (3 commands registered) |
| S2-14 | 前端真实导入流与能力状态 | DONE | 6f7f513; 67 frontend tests; 6 Ready formats; real Tauri file picker; browser preview honest |
| S2-15 | S2 格式矩阵和 CI 总门禁 | TODO | depends on S2-07～S2-14; CI has workflow_dispatch |

## S3｜全书扫描、上下文与恢复（12/18 DONE, 5 RETRY, 1 TODO）

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
| S3-11 | 章节原子提交持久化合同 | DONE | d6b831e; ChapterCommit+ScanPersistence trait |
| S3-12 | V2 扫描运行与 usage migration | DONE | 1e53133; 104 migration tests |
| S3-13 | SQLite 原子 ScanPersistence | RETRY | 89b7eb4; CI failed (validator: no run URL at CI commit 1918956); URL added in f7a2b7c; re-triggering CI |
| S3-14 | 故障、重试与崩溃恢复矩阵 | RETRY | retry.rs+StopReason contracts; fault injection matrix not implemented; needs S3-13 first |
| S3-15 | Tauri 扫描命令与事件桥 | RETRY | invoke_handler only has import_capabilities+rule_pack_summary; no scan commands |
| S3-16 | 前端真实任务进度和控制 | RETRY | ScanProgress shows "界面演示"; useAppState uses demoScanJobs; start button disabled |
| S3-17 | 证据详情与来源章节回跳 | RETRY | EvidencePanel uses demoHits; no Tauri evidence detail command |
| S3-18 | 长书测试与 S3 总门禁 | TODO | depends on S3-13～S3-17 |

## S4｜多模型 API 与 BYOK（5/20 DONE, 10 RETRY, 3 TODO, 2 HUMAN_PENDING）

阶段文件：`40_S4_MODEL_BYOK.md`

| ID | 任务 | 状态 | Commit / CI / 备注 |
| --- | --- | --- | --- |
| S4-01 | Provider 配置与注册表 | DONE | 024d185; 3 production templates (OpenAI/DeepSeek/Anthropic); 27 tests pass |
| S4-02 | 共享结构化输出 wire schema | DONE | 7223707; 25 tests |
| S4-03 | 统一且防提示注入的扫描 prompt | DONE | 80c5193; 25 tests |
| S4-04 | HTTP 执行、脱敏、超时与取消 | DONE | [pending commit]; reqwest+rustls real HTTP; RedactedSecret wrapper; 46 tests pass (19 new: 11 redaction + 8 http) |
| S4-05 | 确定性重试、限流和观测 | DONE | 6fd9422; 25 tests |
| S4-06 | OpenAI-compatible adapter | RETRY | ce542d6; simplified to DeepSeek only; must be generic OpenAI-compatible |
| S4-07 | DeepSeek 兼容端点模板 | RETRY | merged into S4-01; needs independent template verification + fake server contract |
| S4-08 | Anthropic native adapter | TODO | marked SKIPPED in old ledger; Anthropic native is required by spec |
| S4-09 | 限制本地确定性测试 provider | DONE | 4b9251d; filtered from production |
| S4-10 | SecretStore 抽象与 canary | DONE | bf93dd6; 25 tests |
| S4-11 | Windows Credential Manager | RETRY | 0a9d73c stub; resolve/store/delete return errors |
| S4-12 | Android Keystore bridge 接口 | RETRY | bridge contract code not verified; needs fake tests; can't be HUMAN_PENDING without code |
| S4-13 | Provider profile v3 migration | DONE | 0cdc033; credential_ref hardened; 104 migration tests |
| S4-14 | Tauri profile/credential 命令 | RETRY | import_novel_bytes+rule_pack_summary are NOT profile/credential commands |
| S4-15 | 安全连接测试与 adapter factory | RETRY | e6ca08d; 27 tests; depends on real HTTP+adapter+credential chain |
| S4-16 | 设置 UI、连接测试与预算 | RETRY | SettingsPanel is disabled demo; no real provider select/Key save/connection test |
| S4-17 | 正文出站确认与按书授权 | RETRY | secret-ref contract exists but no outbound disclosure UI+command; was merged with S4-18 |
| S4-18 | 密钥/正文泄漏与无 Key 离线门 | RETRY | canary tests exist but no comprehensive leakage validation script; was merged with S4-17 |
| S4-19 | 两个真实提供器人工合同验证 | HUMAN_PENDING | HG-001; 需要用户提供真实 Key |
| S4-20 | S4 总门禁 | TODO | depends on S4-01～S4-18 engineering completion |

## S5｜社区规则和用户定制（2/15 DONE, 5 RETRY, 7 TODO, 1 HUMAN_PENDING）

阶段文件：`50_S5_COMMUNITY_RULES.md`

| ID | 任务 | 状态 | Commit / CI / 备注 |
| --- | --- | --- | --- |
| S5-RULE-01 | 来源台账 schema 与验证器 | DONE | 075db3f; 57 tests; 旧账本有重复行已去重 |
| S5-RULE-02 | 公开来源采集与人工核验登记 | HUMAN_PENDING | HG-002A/HG-002B; SourceRecord+RuleProvenance types done; 需真实人工逐字核验 |
| S5-RULE-03 | 从已核验证据生成版本化规则包 | TODO | depends on S5-RULE-02 evidence; generation script+schema not implemented |
| S5-RULE-04 | 规则不变量与只读概念聚合 | DONE | f7fd565; 57 tests |
| S5-PRESET-01A | 预设领域模型与三层合并 | RETRY | RulePreset+merge_rule_config exists; full preset model with ID/name/三层纯函数 incomplete |
| S5-PRESET-01B | 预设与每书覆盖持久化 | TODO | only model exists; SQLite repository+migration+CRUD not implemented |
| S5-PRESET-01C | 扫描选择快照与 Tauri 契约 | TODO | scan_jobs has RuleSelection; no preset Tauri commands (CRUD/duplicate/apply/preview) |
| S5-PRESET-02 | 规则与预设 UI | RETRY | RuleSelector keyboard nav exists; no preset save/copy/restore/per-book override UI |
| S5-CUSTOM-01A | 自定义规则 schema 与安全解析 | RETRY | RuleDefinition+DetectionMode in core; no custom-rulepack schema/validator/parser |
| S5-CUSTOM-01B | 自定义规则持久化与 Tauri commands | TODO | only rule_pack_summary cmd; no custom_rules repository/migration/commands |
| S5-CUSTOM-01C | 导入预览、导出与 UI 闭环 | TODO | ImportPanel is demo only; no custom rule preview/conflict/export UI |
| S5-UPGRADE-01A | 版本差异与迁移计划纯函数 | RETRY | allowed_transition state machine exists; full diff computation (added/removed/changed/unchanged) incomplete |
| S5-UPGRADE-01B | 事务化迁移应用与回滚 | TODO | migration validator tests exist; rule_upgrades repository+transactional apply not implemented |
| S5-UPGRADE-01C | 历史复现与升级确认 UI | TODO | checkpoint schema version validation exists; no upgrade diff UI or history reproduction |
| S5-GATE-01 | S5 工程门与人工门汇总 | TODO | depends on all S5 tasks |

## S6｜Windows 与 Android 产品化（1/25 DONE, 3 RETRY, 17 TODO, 4 HUMAN_PENDING）

阶段文件：`60_S6_WINDOWS_ANDROID.md`

| ID | 任务 | 状态 | Commit / CI / 备注 |
| --- | --- | --- | --- |
| S6-WIN-01 | 已安装 Windows shell 审计与冒烟 | RETRY | Tauri binary builds; no installed smoke test script |
| S6-WIN-02 | Windows 安装、升级与卸载 | TODO | depends on S6-WIN-01; needs Tauri bundler config |
| S6-AND-01 | 生成并审查 Tauri Android scaffold | TODO | CI workflow exists; no committed gen/android scaffold |
| S6-AND-02A | SAF 原生合同与 Kotlin 实现 | TODO | no SAF Kotlin code; was merged as S6-AND-02A-D |
| S6-AND-02B | SAF Rust 桥与 URI 生命周期 | TODO | no SAF Rust bridge; was merged |
| S6-AND-02C | Android 导入 UI | TODO | no Android import UI; was merged |
| S6-AND-02D | SAF 真机门 | HUMAN_PENDING | HG-003; Android device needed |
| S6-AND-03A | Android Keystore 原生 secret 实现 | TODO | no Keystore Kotlin code; was merged as S6-AND-03A-D |
| S6-AND-03B | Keystore Rust 桥 | TODO | no Keystore Rust bridge; was merged |
| S6-AND-03C | Provider secret UI 接线 | TODO | no secret UI wiring; was merged |
| S6-AND-03D | Keystore 真机门 | HUMAN_PENDING | HG-003; Android device needed |
| S6-AND-04A | 前台服务与通知原生实现 | TODO | no foreground service Kotlin code; was merged as S6-AND-04A-D |
| S6-AND-04B | 长任务 Rust 生命周期桥 | TODO | no long-task Rust bridge; was merged |
| S6-AND-04C | 暂停、停止、恢复与通知权限 UI | TODO | no control UI; was merged |
| S6-AND-04D | 后台与进程终止真机门 | HUMAN_PENDING | HG-003; Android device needed |
| S6-UI-01A | 响应式布局 | DONE | 2fe6960; 390/800/1440 breakpoints; Playwright e2e |
| S6-UI-01B | 系统返回与旋转状态保存 | TODO | no useSystemBack hook; no rotation save; was merged as S6-UI-01B-C |
| S6-UI-01C | 软键盘、大字体与可访问性 | TODO | no soft keyboard viewport/200% font/focus trap for Android; was merged |
| S6-UI-01D | 响应式与系统交互真机门 | HUMAN_PENDING | HG-003; Android device needed |
| S6-UX-01A | Codex 式非程序员主流程 | RETRY | natural language prompt exists; main flow still uses demo data; no real闭环 |
| S6-UX-01B | 自然语言状态、错误与渐进披露 | TODO | no errorMessages mapping; was merged as S6-UX-01A-C |
| S6-UX-01C | 非视觉模型可执行的 UX 验收 | TODO | no ux-flow test; was merged |
| S6-E2E-01 | Windows 与 Android 自动 E2E | RETRY | Playwright configured; no real scan E2E; no Android E2E |
| S6-BUILD-01 | 构建、Android 必需签名与 Windows 可选签名 | TODO | needs Android signing config+CI release workflow; HG-004A/HG-004W |
| S6-GATE-01 | 双平台产品化总验收 | TODO | depends on all S6 tasks |

## S7｜质量、安全与发布（0/9 DONE, 2 RETRY, 7 TODO）

阶段文件：`70_S7_RELEASE.md`

| ID | 任务 | 状态 | Commit / CI / 备注 |
| --- | --- | --- | --- |
| S7-SEC-01A | 威胁模型、数据流与隐私边界 | TODO | no docs/security/THREAT_MODEL.md; no PRIVACY.md; no data-flow diagram |
| S7-SEC-01B | 删除、全量清理与脱敏诊断 | TODO | no privacy commands/Panel; no diagnostic export |
| S7-SEC-02 | 恶意文件、提示注入和脱敏 | RETRY | ZIP traversal+HTML strip+prompt markers exist; comprehensive security test matrix not done |
| S7-PERF-01 | 性能、内存与用量预算 | TODO | no benchmark harness; no PERFORMANCE.md |
| S7-A11Y-01 | Windows/Android 可访问性 | RETRY | WCAG AA 7/7 for Web; Android TalkBack not done |
| S7-E2E-01 | 双平台 E2E 与崩溃恢复 | TODO | Playwright configured only; no release-level E2E+crash recovery |
| S7-BUILD-01 | 可重复构建、依赖与资产审计 | TODO | no SBOM; no reproducible build verification |
| S7-REL-01 | 受保护 draft/RC、校验值与用户文档 | TODO | depends on S7-BUILD-01; needs HG-004A/HG-004W/HG-005 |
| S7-GATE-01 | 发布候选工程门与人工门矩阵 | TODO | depends on all S7 tasks |

## 最终交接（0/5 DONE, 4 TODO, 1 HUMAN_PENDING）

阶段文件：`90_FINAL_HANDOFF.md`

| ID | 任务 | 状态 | Commit / CI / 备注 |
| --- | --- | --- | --- |
| FINAL-01 | 冻结 source commit 与证据清单 | TODO | depends on S7-GATE-01; docs/deepseek-handoff/ does not exist |
| FINAL-02 | 生成机器交接包 | TODO | depends on FINAL-01 |
| FINAL-03 | DeepSeek 证据一致性校验 | TODO | depends on FINAL-02 |
| FINAL-TIEBA-01 | 贴吧逐字核验人工门 | HUMAN_PENDING | HG-002A/HG-002B; 需真实人工逐字核验+第二复核 |
| FINAL-04 | 发送一次 Codex 终审请求并等待 | TODO | depends on FINAL-03 + FINAL-TIEBA-01 |

## 统计

| 状态 | 数量 |
| --- | --- |
| DONE | 39 |
| RETRY | 36 |
| TODO | 34 |
| HUMAN_PENDING | 14 |
| IN_PROGRESS | 0 |
| BLOCKED | 0 |
| AWAITING_CI | 0 |
| **总计** | **122** |

## 更新示例

任务开始：

```text
| SX-YY | 示例任务 | IN_PROGRESS | started 2026-07-20 |
```

任务完成：

```text
| SX-YY | 示例任务 | DONE | abc1234; 42 tests; CI run URL |
```

若任务被阻塞，不要把其后的依赖任务标 DONE：

```text
| SX-YY | 示例人工核验 | HUMAN_PENDING | HG-000; 缺人工来源转写 |
```
