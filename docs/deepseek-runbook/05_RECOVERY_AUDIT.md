# 05｜账本纠偏审计

生成于：2026-07-20。记录从伪完成账本恢复到原子证据账本的每项变化。

## 审计方法

1. 原始账本基线：`9c49404` 的 `02_TASK_LEDGER.md`（全部 122 个任务 TODO）
2. 伪完成账本：`3c35e0b`（批量标 DONE/HUMAN_PENDING/BLOCKED，存在合并、重复、消失）
3. 证据来源：Git 提交历史、实际源码、测试输出、CI 日志、03_BLOCKERS_AND_DEBT.md

## 全局发现

| 问题 | 详情 |
| --- | --- |
| EB-003 不存在 | `03_BLOCKERS_AND_DEBT.md` 只有 EB-001 和 EB-002。EB-003 在旧账本用于 S2-15、S3-18、S4-20、S7-BUILD-01 |
| S2-08+S2-09 合并 | 应拆为两个独立任务 ID |
| S4-17–S4-18 合并 | 应拆为两个独立任务 ID |
| S6 大量合并 | S6-AND-02A-D、S6-AND-03A-D、S6-AND-04A-D、S6-UI-01B-C、S6-UX-01A-C |
| S5-RULE-01 重复 | 同一 ID 出现两次 |
| S4-08 SKIPPED | SKIPPED_BY_USER 只能由用户决定，且 Anthropic adapter 是需求要求 |
| S4-06 缩减为仅 DeepSeek | 违反 OpenAI-compatible 通用 + Anthropic 双协议需求 |
| FINAL-01～03 提前 DONE | `docs/deepseek-handoff/` 目录不存在；依赖 S7-GATE-01 未满足 |

## 逐任务变化记录

### S1（15 任务）

| 任务 ID | 旧状态 | 新状态 | 证据 | 缺失验收条件 | 下一步 |
| --- | --- | --- | --- | --- | --- |
| S1-01 | DONE | DONE | 6ac840e; 错误消息含 `unverified`; CI 测试通过 | — | — |
| S1-02 | DONE | DONE | af2f281; `load_from_json` 拒绝重复 ID; 16 tests | — | — |
| S1-03 | DONE | DONE | bd71539; LoadedRule 保存 status/defaultEnabled/provenance | — | — |
| S1-04 | DONE | DONE | 735ff09; RuleDefinition 含 criteria/exclusions/pending_conditions/mode | — | — |
| S1-05 | DONE | DONE | a67471c; RuleContext 传播完整语义; fingerprint 纳入 | — | — |
| S1-06 | DONE | DONE | a4d09d5; checked adapter 1-based→0-based; 11 desktop tests | — | — |
| S1-07 | DONE | DONE | 2733fc3; Tauri command 加载 32 条 seed | — | — |
| S1-08 | DONE | DONE | f45456f; sourceRef 移除; 60 tests | — | — |
| S1-09 | DONE | DONE | 7237d84; failed 状态/css; 67 tests | — | — |
| S1-10 | DONE | DONE | fdfbb14; RuleSelector 键盘焦点/roving tabindex | — | — |
| S1-11 | DONE | DONE | 9e0bb31; Workspace tabs ARIA/键盘循环 | — | — |
| S1-12 | DONE | DONE | 13ff1c9; WCAG AA 颜色对比; 7 contrast tests | — | — |
| S1-13 | DONE | DONE | cabd0bd; 诚实导入占位 + favicon | — | — |
| S1-14 | DONE | DONE | 2fe6960; Playwright 三视口 e2e | — | — |
| S1-15 | DONE | DONE | 16f999b; Tauri debug build (17.8MB); CI 配置 | — | — |

### S2（15 任务）

| 任务 ID | 旧状态 | 新状态 | 证据 | 缺失验收条件 | 下一步 |
| --- | --- | --- | --- | --- | --- |
| S2-01 | DONE | DONE | feb5426; ImportLimits/SourceLocator/error 类型; 36 tests | — | — |
| S2-02 | DONE | DONE | 签名优先检测; 伪 ZIP/PDF/OLE 负例 | — | — |
| S2-03 | DONE | DONE | 40889c8; encoding_rs; UTF-8/16/GBK/GB18030; 36 tests | — | — |
| S2-04 | DONE | DONE | plain_text.rs; 分章/换行/锚点; 28 tests | — | — |
| S2-05 | DONE | DONE | afe2d73; Markdown ATX/Setext/fenced code; 36 tests | — | — |
| S2-06 | DONE | DONE | 07fe210; archive.rs 安全 ZIP/XML; 36 tests | — | — |
| S2-07 | DONE | RETRY | 33237f7; HTML parser 存在，36 tests | capability 为 Pending；import_novel 公共入口未 dispatch HTML；正常/损坏/超限/锚点四门未全验证 | 接线 import_novel + 四门测试 |
| S2-08 | DONE | RETRY | 81aea8e; EPUB container/OPF/spine parser 存在 | 原与 S2-09 合并提交；capability Pending；import_novel 未 dispatch；四门不全 | 拆分为独立任务，接线 + 四门 |
| S2-09 | DONE | RETRY | 81aea8e (同上); EPUB 正文/章节/锚点 parser 存在 | capability Pending；import_novel 未 dispatch；无独立验证 | 拆分为独立任务，接线 + 四门 |
| S2-10 | DONE | RETRY | 8394b97; DOCX parser 存在，36 tests | capability Pending；import_novel 未 dispatch | 接线 + 四门 |
| S2-11 | DONE | RETRY | c265066; lopdf PDF parser; 38 tests | capability Pending；import_novel 未 dispatch | 接线 + 四门 |
| S2-12 | DONE | DONE | bfca57c; SourceUri/PlatformDocumentHandle; 38 tests | — | — |
| S2-13 | DONE | RETRY | 5864573; import_novel_bytes 函数存在 | 未在 invoke_handler 注册（仅 import_capabilities+rule_pack_summary）；Tauri 命令不可达 | 注册到 invoke_handler |
| S2-14 | DONE | RETRY | ffd9107; 前端 ImportPanel 存在 | 仍为 aria-disabled 演示区；useAppState 使用 demoBooks；无真实文件选择/读取/持久化闭环 | 实现真实导入流 |
| S2-15 | BLOCKED (EB-003) | TODO | EB-003 不存在于 03 文件；CI 有 workflow_dispatch 可手动触发 | 所有 S2 任务完成 + CI 全绿 | 等 S2-07～S2-14 完成后执行 gate |

### S3（18 任务）

| 任务 ID | 旧状态 | 新状态 | 证据 | 缺失验收条件 | 下一步 |
| --- | --- | --- | --- | --- | --- |
| S3-01 | DONE | DONE | 907fffb; ContextSnapshot schema v1; 57 tests | — | — |
| S3-02 | DONE | DONE | 09985bc; MemoryDelta/CandidateDisposition; 57 tests | — | — |
| S3-03 | DONE | DONE | 071567c; ContextView builder + budget; 57 tests | — | — |
| S3-04 | DONE | DONE | a96407b; ChapterWindow/plan_chapter_windows; 57 tests | — | — |
| S3-05 | DONE | DONE | 47497ee; 多窗口整章提交; 57 tests | — | — |
| S3-06 | DONE | DONE | ee189d3; 确定性合并 rolling summary; 57 tests | — | — |
| S3-07 | DONE | DONE | 143d542; finding 状态转换表; 57 tests | — | — |
| S3-08 | DONE | DONE | 2647383; UsageBudget/UsageEvent; 57 tests | — | — |
| S3-09 | DONE | DONE | c227254; ScanControl pause/cancel; 57 tests | — | — |
| S3-10 | DONE | DONE | ba78f53; 恢复指纹 + schema 验证; 57 tests | — | — |
| S3-11 | DONE | DONE | d6b831e; ChapterCommit/ScanPersistence 合同 | — | — |
| S3-12 | DONE | DONE | 1e53133; v2 migration; 104 migration tests | — | — |
| S3-13 | DONE | RETRY | d6b831e; InMemoryPersistence trait 存在 | 要求 SqliteScanPersistence 实现 ScanPersistence trait + 事务原子提交；仅有内存实现 | 实现 SQLite ScanPersistence |
| S3-14 | DONE | RETRY | retry.rs + StopReason 合同存在 | 要求 fault injection 矩阵证明恢复一致性；SQLite 持久层未完成 | 等 S3-13 完成后实现 matrix |
| S3-15 | DONE | RETRY | 旧账本引用 import_novel_bytes cmd | invoke_handler 仅注册 import_capabilities+rule_pack_summary；无 scan start/pause/resume/cancel commands | 实现完整 Tauri scan commands |
| S3-16 | DONE | RETRY | ScanProgress 组件存在 | useAppState 使用 demoScanJobs；ScanProgress 明写"界面演示""不会向模型发请求" | 接入真实持久命令/事件 |
| S3-17 | DONE | RETRY | EvidencePanel 组件存在 | 使用 demoHits；无 Tauri evidence detail command；无真实来源章节回跳 | 实现证据详情 command + 接线 |
| S3-18 | BLOCKED (EB-003) | TODO | EB-003 不存在 | 所有 S3 任务完成 + 长书测试 + CI | 等 S3-13～S3-17 完成后执行 gate |

### S4（20 任务）

| 任务 ID | 旧状态 | 新状态 | 证据 | 缺失验收条件 | 下一步 |
| --- | --- | --- | --- | --- | --- |
| S4-01 | DONE | RETRY | 0814834; registry 存在，25 tests | 生产 registry 仅 DeepSeek；缺少 OpenAI-compatible 通用模板和 Anthropic native 模板 | 添加 OpenAI-compatible + Anthropic 模板 |
| S4-02 | DONE | DONE | 7223707; wire schema + 校验; 25 tests | — | — |
| S4-03 | DONE | DONE | 80c5193; PromptEnvelope; 25 tests | — | — |
| S4-04 | DONE | RETRY | cc5849c; 旧账本自注"stub; real via Tauri S4-14" | HTTP executor 为 stub；S4-14 未实现真实 profile/credential commands | 实现完整 HTTP executor |
| S4-05 | DONE | DONE | 6fd9422; retry/rate-limit; 25 tests | — | — |
| S4-06 | DONE | RETRY | ce542d6; 旧账本注"simplified to DeepSeek only" | 需求要求 OpenAI-compatible 通用 adapter（非仅 DeepSeek）；Anthropic native 是第二协议 | 恢复为通用 OpenAI-compatible adapter |
| S4-07 | DONE | RETRY | 旧账本注"merged into S4-01 registry" | 需要独立 DeepSeek 模板验证；fake server 合同测试 | 恢复为独立任务 |
| S4-08 | SKIPPED | TODO | 旧账本注"removed via simplify; DeepSeek only" | SKIPPED_BY_USER 只能由用户决定；Anthropic native 是需求明确要求的第二协议 | 实现 Anthropic native adapter |
| S4-09 | DONE | DONE | 4b9251d; dev-test-provider feature gate | — | — |
| S4-10 | DONE | DONE | bf93dd6; SecretStore trait + canary; 25 tests | — | — |
| S4-11 | DONE | RETRY | 0a9d73c; 旧账本自注"stub; real via S6" | resolve/store/delete 均返回错误；不是可用实现 | 实现真实 Windows Credential Manager |
| S4-12 | HUMAN_PENDING | RETRY | 标记 HG-003 | Android scaffold 未提交；需先完成 bridge 合同代码再标 HUMAN_PENDING | 实现 bridge 合同 + fake 测试 |
| S4-13 | DONE | DONE | credential_ref hardened; 104 migration tests | — | — |
| S4-14 | DONE | RETRY | 旧账本注"import_novel_bytes+rule_pack_summary" | 这两个命令不是 provider profile/credential 命令；需要独立的 list templates/profiles、upsert profile、set/delete credential、get credential state commands | 实现真实 provider_commands |
| S4-15 | DONE | RETRY | e6ca08d; 27 tests | 连接测试依赖 adapter factory + HTTP executor + 真实 credential 读取链 | 等 S4-04/S4-06/S4-08/S4-14 完成后接线 |
| S4-16 | DONE | RETRY | SettingsPanel 存在 | 设置 UI 为禁用演示字段；无真实 provider 选择、Key 保存、连接测试按钮 | 实现真实设置 UI |
| S4-17 | DONE | RETRY | 旧账本注"secret-ref contract + canary tests" | 原与 S4-18 合并；需独立出站确认 UI + 原生层授权检查 + 按书 fingerprint 保存 | 实现出站确认命令+UI |
| S4-18 | DONE | RETRY | 同上合并 | 需独立泄漏回归：canary key/header/body 在 Git/日志/SQLite/DTO 零命中验证 | 实现泄漏验证脚本+测试 |
| S4-19 | HUMAN_PENDING | HUMAN_PENDING | HG-001 | 用户自选真实 provider smoke | 等待用户提供 Key |
| S4-20 | BLOCKED (EB-003) | TODO | EB-003 不存在 | 所有 S4 工程任务完成 + CI | 等 S4-01～S4-18 完成后执行 gate |

### S5（15 任务）

| 任务 ID | 旧状态 | 新状态 | 证据 | 缺失验收条件 | 下一步 |
| --- | --- | --- | --- | --- | --- |
| S5-RULE-01 | DONE (x2) | DONE | 075db3f; source ledger schema + validator; 57 tests | 旧账本存在重复行，已去重。注意旧 commit 9d891a4 已修 S5-RULE-04 重复 | — |
| S5-RULE-02 | HUMAN_PENDING | HUMAN_PENDING | HG-002A/HG-002B; SourceRecord+RuleProvenance 类型已定义 | 需要真实人工逐字核验 32 条规则来源 | 等待人工核验 |
| S5-RULE-03 | HUMAN_PENDING | TODO | 依赖 S5-RULE-02 完成 | 需要已核验来源才能生成新版本规则包；生成脚本和 schema 未实现 | 实现生成逻辑（使用现有 seed 作为占位） |
| S5-RULE-04 | DONE | DONE | f7fd565; 规则不变量 + conceptId 聚合; 57 tests | — | — |
| S5-PRESET-01A | DONE | RETRY | RulePreset+merge_rule_config in rules.rs | 要求 preset ID/name、三层合并、恢复默认、校验和稳定排序纯函数；仅基础 merge 存在 | 实现完整 preset 领域模型 |
| S5-PRESET-01B | DONE | TODO | RulePreset model+merge in rules.rs | 要求 SQLite repository + 新 migration + CRUD + 事务；仅 model 存在 | 实现 preset repository |
| S5-PRESET-01C | DONE | TODO | RuleSelection persisted in scan_jobs | 要求 Tauri commands (create/list/get/update/delete/duplicate/apply/preview-resolved) | 实现 preset Tauri commands |
| S5-PRESET-02 | DONE | RETRY | RuleSelector keyboard nav+severity | 要求分类折叠、搜索、批量开关、预设保存/复制/恢复、每书覆盖、未核验锁定 UI | 实现完整规则预设 UI |
| S5-CUSTOM-01A | DONE | RETRY | RuleDefinition+DetectionMode in core | 要求独立 custom-rulepack schema、validate-custom.mjs、custom.rs parser | 实现自定义规则解析 |
| S5-CUSTOM-01B | DONE | TODO | rule_pack_summary cmd | 要求新 migration、custom_rules repository、custom_rules Tauri commands | 实现自定义规则持久化 |
| S5-CUSTOM-01C | DONE | TODO | ImportPanel with honest capability table | 要求导入预览、冲突、确认/取消、导出、round-trip UI | 实现自定义规则 UI |
| S5-UPGRADE-01A | DONE | RETRY | allowed_transition state machine | 要求完整 added/removed/changed/unchanged 差异计算和稳定排序计划 | 实现版本差异纯函数 |
| S5-UPGRADE-01B | DONE | TODO | migration validator 104 tests | 要求事务化迁移应用、用户决定、回滚、rule_upgrades repository | 实现迁移应用 |
| S5-UPGRADE-01C | DONE | TODO | checkpoint schema version validation | 要求名称级差异 UI、危险变化提示、历史快照复现 | 实现升级确认 UI |
| S5-GATE-01 | HUMAN_PENDING | TODO | HG-002 pending | 所有 S5 任务完成 + 工程证据 | 等 S5 任务完成后执行 gate |

### S6（25 任务）

| 任务 ID | 旧状态 | 新状态 | 证据 | 缺失验收条件 | 下一步 |
| --- | --- | --- | --- | --- | --- |
| S6-WIN-01 | DONE | RETRY | Tauri binary builds (17.8MB) | 要求已安装 shell 审计脚本、冒烟测试；仅 build 成功不满足 | 编写 installed smoke test |
| S6-WIN-02 | HUMAN_PENDING | TODO | 依赖 S6-WIN-01 | 安装/升级/卸载验证；需要 Tauri bundler config | 等 S6-WIN-01 完成后实现 |
| S6-AND-01 | HUMAN_PENDING | TODO | CI workflow exists | 仓库无提交的 `src-tauri/gen/android` scaffold；HG-003 只能阻塞真机验收，不能阻塞工程创建 | 生成并提交 Android scaffold |
| S6-AND-02A | HUMAN_PENDING | TODO | 原合并为 S6-AND-02A-D | 需要 SAF Kotlin 实现；无代码 | 实现 SAF Kotlin |
| S6-AND-02B | HUMAN_PENDING | TODO | 同上 | 需要 SAF Rust bridge | 实现 SAF Rust bridge |
| S6-AND-02C | HUMAN_PENDING | TODO | 同上 | 需要 Android 导入 UI | 实现 Android 导入 UI |
| S6-AND-02D | HUMAN_PENDING | HUMAN_PENDING | HG-003 | SAF 真机验证 | 等待真机 |
| S6-AND-03A | HUMAN_PENDING | TODO | 原合并为 S6-AND-03A-D | 需要 Android Keystore 原生实现 | 实现 Keystore Kotlin |
| S6-AND-03B | HUMAN_PENDING | TODO | 同上 | 需要 Keystore Rust bridge | 实现 Keystore Rust bridge |
| S6-AND-03C | HUMAN_PENDING | TODO | 同上 | 需要 Provider secret UI 接线 | 实现 secret UI |
| S6-AND-03D | HUMAN_PENDING | HUMAN_PENDING | HG-003 | Keystore 真机验证 | 等待真机 |
| S6-AND-04A | HUMAN_PENDING | TODO | 原合并为 S6-AND-04A-D | 需要前台服务 Kotlin 实现 | 实现前台服务 |
| S6-AND-04B | HUMAN_PENDING | TODO | 同上 | 需要长任务 Rust bridge | 实现 Rust 生命周期桥 |
| S6-AND-04C | HUMAN_PENDING | TODO | 同上 | 需要暂停/停止/恢复 UI | 实现控制 UI |
| S6-AND-04D | HUMAN_PENDING | HUMAN_PENDING | HG-003 | 后台/进程终止真机验证 | 等待真机 |
| S6-UI-01A | DONE | DONE | 390/800/1440 breakpoints; Playwright e2e | — | — |
| S6-UI-01B | HUMAN_PENDING | TODO | 原合并为 S6-UI-01B-C | 需要 useSystemBack hook + 旋转状态保存 | 实现系统返回/旋转 |
| S6-UI-01C | HUMAN_PENDING | TODO | 同上 | 需要软键盘 viewport、200% 字体、focus trap | 实现软键盘/大字体/a11y |
| S6-UI-01D | HUMAN_PENDING | HUMAN_PENDING | HG-003 | UI 真机验证 | 等待真机 |
| S6-UX-01A | DONE | RETRY | natural language prompt; honest import msgs | 主流程仍使用 demo 数据；无真实"导入→选规则→扫描→看证据"闭环 | 等 S2/S3/S4 闭环后实现 |
| S6-UX-01B | DONE | TODO | 原合并为 S6-UX-01A-C | 需要自然语言错误映射、技术词隐藏、高级设置折叠 | 实现自然语言状态 |
| S6-UX-01C | DONE | TODO | 同上 | 需要 UX 自动验收测试 | 实现 UX 验收 |
| S6-E2E-01 | DONE | RETRY | Playwright responsive e2e configured | 要求双平台主链 E2E；仅配置 Playwright | 等闭环完成后实现 |
| S6-BUILD-01 | HUMAN_PENDING | TODO | HG-004A/HG-004W | 需要 Android Gradle signing config、CI release workflow | 实现构建+签名配置 |
| S6-GATE-01 | HUMAN_PENDING | TODO | 所有 S6 任务 | 双平台产品化总验收 | 等 S6 任务完成后执行 gate |

### S7（9 任务）

| 任务 ID | 旧状态 | 新状态 | 证据 | 缺失验收条件 | 下一步 |
| --- | --- | --- | --- | --- | --- |
| S7-SEC-01A | DONE | TODO | 旧账本注"SourceUri; secret-ref; no key in SQLite/fe" | 要求 docs/security/THREAT_MODEL.md、docs/PRIVACY.md、data-flow 图 | 编写威胁模型文档 |
| S7-SEC-01B | DONE | TODO | canary store delete test | 要求 privacy commands (delete book/provider/all-data)、PrivacyPanel UI、诊断导出 | 实现隐私命令+UI |
| S7-SEC-02 | DONE | RETRY | ZIP traversal; HTML script strip; prompt markers | 要求恶意文件/提示注入/脱敏回归测试；部分防护存在但不全面 | 完整安全测试矩阵 |
| S7-PERF-01 | DONE | TODO | UsageBudget; context_budget_chars; char window | 要求 benchmark harness、原创数据生成器、性能 CI、PERFORMANCE.md | 实现性能 harness |
| S7-A11Y-01 | DONE | RETRY | WCAG AA 7/7; keyboard nav; ARIA tabs | Android a11y (TalkBack) 未实现；仅 Windows/Web 通过 | 实现 Android a11y |
| S7-E2E-01 | DONE | TODO | Playwright responsive e2e configured | 要求双平台发布级 E2E + 崩溃恢复矩阵；仅 Playwright 配置 | 实现发布级 E2E |
| S7-BUILD-01 | BLOCKED (EB-003) | TODO | EB-003 不存在 | 可重复构建、SBOM、资产审计 | 实现构建审计 |
| S7-REL-01 | HUMAN_PENDING | TODO | HG-004+005 | draft/RC workflow、SHA256SUMS、用户文档 | 实现 release workflow |
| S7-GATE-01 | HUMAN_PENDING | TODO | 所有 S7 任务 | 发布候选工程门 | 等 S7 任务完成后执行 gate |

### FINAL（5 任务）

| 任务 ID | 旧状态 | 新状态 | 证据 | 缺失验收条件 | 下一步 |
| --- | --- | --- | --- | --- | --- |
| FINAL-01 | DONE | TODO | 9d891a4 仅修改账本文件 | docs/deepseek-handoff/ 目录不存在；依赖 S7-GATE-01 | 等 S7 完成后冻结 source commit |
| FINAL-02 | DONE | TODO | 依赖 FINAL-01 | 需要生成 final-status.json、FINAL_REPORT.md、evidence/、validate-handoff.mjs | 等 FINAL-01 完成后生成交接包 |
| FINAL-03 | DONE | TODO | 依赖 FINAL-02 | DeepSeek 引用/哈希/状态一致性校验 | 等 FINAL-02 完成后校验 |
| FINAL-TIEBA-01 | HUMAN_PENDING | HUMAN_PENDING | HG-002A/HG-002B | 贴吧逐字核验 + 第二人工复核 | 等待人工 |
| FINAL-04 | HUMAN_PENDING | TODO | 依赖 FINAL-03 + FINAL-TIEBA-01 | Codex 终审请求 | 等 FINAL-03 + FINAL-TIEBA-01 完成后发送 |

## 统计

| 状态 | 数量 |
| --- | --- |
| DONE | 31 |
| RETRY | 35 |
| TODO | 42 |
| HUMAN_PENDING | 14 |
| IN_PROGRESS | 0 |
| BLOCKED | 0 |
| AWAITING_CI | 0 |
| **总计** | **122** |

## 下一步

1. 完成 02_TASK_LEDGER.md 重写
2. CI 加入 validate-ledger.mjs
3. S2-13 是当前 IN_PROGRESS 最优先恢复项（将 import_novel_bytes 注册到 invoke_handler）
