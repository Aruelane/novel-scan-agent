# 最终交接：DeepSeek 准备证据，Codex 只终审一次

本文件只用于全部独立工程结束后的证据整理。DeepSeek 不在这里自称审查者，也不替 Codex 给出终审结论。总任务数：**5**。

## 不可违反的规则

- task ledger 状态只用 `DONE`、`RETRY`、`AWAITING_CI`、`HUMAN_PENDING`、`BLOCKED`；`IN_PROGRESS` 只在执行中使用。报告可另写阶段说明 `ENGINEERING_DONE`，但不能作为 ledger 状态。
- `PASS` 只用于 `final-status.json` 内的证据结论，必须有真实命令退出码、GitHub run、设备记录或 Release 资产支撑；未运行是 `NOT_RUN`，等待外部条件是 `BLOCKED`。
- DeepSeek 的职责是冻结事实、生成 schema/JSON/报告、验证引用一致性。禁止把同一个 DeepSeek 的复述称为“独立复核”。
- 贴吧逐字核验是独立人工任务 `FINAL-TIEBA-01`，引用全局 `HG-002A`（逐字核验）与 `HG-002B`（另一名人工第二复核）。OCR、截图文件名、搜索摘要或无视觉模型解释均不能替代人工核验。
- 不把 debug APK、unsigned Android APK、mock UI、release 中可见的 dev test provider、临时 scaffold 或未核验规则称为正式完成。Windows 商业签名可选；未签名 GitHub 侧载资产必须真实标注。Android 自建 release keystore 签名必需。
- 交接包不得含小说正文、证据长摘录、API Key、cookie、token、keystore、签名口令、绝对本地路径、SAF URI、设备 serial 或个人信息。
- FINAL-04 只提交/展示一次 Codex 终审请求，然后 ledger 写 `HUMAN_PENDING` 并停止。Codex 返回前不移动 tag、不替换资产、不继续“自审”。

## 必须生成的文件

仅在 `docs/deepseek-handoff/` 创建：

1. `final-status.schema.json`：JSON Schema 2020-12，`additionalProperties: false`。
2. `final-status.json`：机器可审计的唯一事实源。
3. `FINAL_REPORT.md`：只引用 JSON 已有事实的中文摘要。
4. `evidence/commands.jsonl`：命令、时间、退出码、HEAD、脱敏摘要和日志哈希。
5. `evidence/artifacts.json`：资产、字节数、SHA-256、平台、签名状态和验证引用。
6. `validate-handoff.mjs` 与负例 fixture：验证 schema、交叉引用、HEAD/tag/资产一致性和敏感字段禁令。
7. `CODEX_REVIEW_REQUEST.md`：FINAL-04 可直接发送给 Codex 的唯一终审消息。

除上述文件、validator 和必要 package script 外，不修改业务代码。缺证据写 `NOT_RUN/BLOCKED` 和 blocker ref，不伪造 URL、SHA、时间或退出码。

## `final-status.json` 最低合同

顶层至少包含：

- `schemaVersion`、`generatedAt`、`repository`、`branch`、`sourceCommit`、`sourceTreeClean`、`handoffCommit`；
- `product`：名称、版本、`platforms: [windows, android]`、`iosSupported: false`；
- `stageSummary[]` 与 `taskResults[]`：每个 runbook task ID 恰好一次，含 ledger 状态、commit、changedFiles、evidence refs、blocker refs；
- `testEvidence[]`：命令/workflow、时间、exitCode/conclusion、headSha、工具版本、artifact/URL、SHA-256、摘要；
- `ciEvidence[]`、`deviceEvidence[]`、`releaseEvidence`、`securityEvidence`；
- `ruleEvidence`：pack/version、11/21、verified/unverified/default-enabled 数、`HG-002A/HG-002B` 状态和证据 refs；
- `humanGates[]`：`HG-001`、`HG-002A`、`HG-002B`、`HG-003`、`HG-004A`、`HG-004W`、`HG-005`；
- `knownLimitations[]`、`blockers[]`、`reviewScope`。

证据结论枚举为 `PASS`、`PARTIAL`、`BLOCKED`、`FAIL`、`NOT_RUN`、`NOT_APPLICABLE`。只有明确不在范围内的 iOS 可用 `NOT_APPLICABLE`；不能把未做的 Windows/Android 工作标 N/A。

## 任务依赖

| 任务 ID | 依赖 | 原子结果 |
| --- | --- | --- |
| FINAL-01 | S7-GATE-01 的独立工程已停止改动 | 冻结 source commit 与真实证据清单 |
| FINAL-02 | FINAL-01 | 机器交接包与 validator |
| FINAL-03 | FINAL-02 | DeepSeek 仅做引用/哈希/状态一致性校验 |
| FINAL-TIEBA-01 | S5-RULE-02、S5-GATE-01 | 登记 HG-002A/HG-002B 人工逐字核验结果 |
| FINAL-04 | FINAL-03、FINAL-TIEBA-01 | 发出一次 Codex 终审请求并等待 |

人工 gate pending 只暂停依赖它的 FINAL-04；不得阻止 FINAL-01～03 的证据工程。

---

## FINAL-01：冻结 source commit 与证据清单

**精确范围**：只读审计和 `docs/deepseek-handoff/evidence/` 的临时清单；不得修改业务代码。

**执行**：记录 branch/HEAD/remote/tag 与 clean 状态；从 ledger 枚举每个任务的 commit、测试和 blocker；只接受同一 source HEAD 的 GitHub run；从 draft 下载资产到仓库外临时目录，重算 SHA-256、验证 Android 签名和 Windows 实际签名状态。失败、未运行和外部缺口逐条登记，不先修小问题。

**门槛**：唯一 `sourceCommit`、完整 task ID 集合和每项证据/blocker 已形成；若发现 P0/P1，ledger 写 `RETRY` 并返回对应原任务，修复后重新冻结。P2/P3 只入 backlog。

---

## FINAL-02：生成机器交接包

**精确范围**：仅 `docs/deepseek-handoff/**` 和调用 validator 所需的 package script。

**执行**：按最低合同创建 schema、commands、artifacts、final-status 和 FINAL_REPORT；稳定排序 ID；所有 ref 必须存在；PASS 必须有 evidence；exitCode 非 0 不得 PASS；CI headSha 等于 sourceCommit；release tag/commit/manifest 一致；禁止敏感键名、值模式和绝对路径。

**测试/门槛**：合法 fixture 通过；假 PASS、错 SHA、悬空 ref、CI SHA 不同、重复/缺 task、敏感字段和 Android 未签名冒充 release 的负例都失败。所有 JSON 为 UTF-8、稳定排序。

---

## FINAL-03：DeepSeek 证据一致性校验

**精确范围**：只更新 `docs/deepseek-handoff/**` 中的证据事实和 validator；不得改产品、重新解释截图或声称独立终审。

**执行**：逐 URL 核对 repo/run/head/conclusion；逐下载资产重算 hash/signature；核对 README/隐私/能力矩阵/Release notes；检查 release 中 dev test provider、secret 和用户内容 marker 为零；重跑 handoff validator。截图/视频仅确认文件、哈希和人工签字是否存在，不由 DeepSeek判断画面内容。

**门槛**：引用、hash、commit、签名状态和文档一致；差异写入 blocker。该任务完成只表示“证据包自洽”，不表示产品已通过独立审查。

---

## FINAL-TIEBA-01：贴吧逐字核验人工门

**精确范围**：只更新来源台账中的人工复核元数据和 `docs/deepseek-handoff/**` 的 gate/evidence 引用；不得修改规则语义、默认开关或产品代码。

**执行**：

1. `HG-002A` 由真人逐字对照指定首帖/必要补充材料与 32 行台账，登记复核者代号、日期、材料哈希、定位器和差异；不保存整帖。
2. `HG-002B` 由另一名真人独立核对 32 行映射、默认开关、分类和定位器；同一个 DeepSeek 不能充当任一人工复核者。
3. 图片可用 OCR 帮助定位，但人工必须对原图逐字核验；无可视材料或无人复核时保持 `OPEN`，task ledger 写 `HUMAN_PENDING`。
4. 若有差异，回到 S5 对应任务修复并重新生成 FINAL-01～03；不得在本任务顺手改规则。

**门槛**：两个人工门都 `RESOLVED` 才可将本任务 ledger 写 `DONE`；否则写 `HUMAN_PENDING`。这不会回溯阻塞已经完成的独立工程，但 FINAL-04 等待。

---

## FINAL-04：发送一次 Codex 终审请求并等待

**精确范围**：`docs/deepseek-handoff/CODEX_REVIEW_REQUEST.md`、交接提交和终审消息；不得修改业务代码、tag、draft 资产或正式发布。

**执行**：

1. 生成只包含仓库 URL、branch/sourceCommit/tag、FINAL_REPORT、final-status SHA-256、draft URL、P0/P1 blocker 和明确 review scope 的短消息。
2. 请求 Codex 只审：安全/隐私、数据完整性/恢复、社区规则证据真实性、Windows/Android 安装主链、release 供应链和明显错误声明；P2/P3 小 UI/文案先列 backlog。
3. 提交并推送交接包，确认远端存在精确 SHA；向用户展示 `CODEX_REVIEW_REQUEST.md`，让用户交给 Codex。
4. 此刻 ledger 写 `HUMAN_PENDING` 并停止。DeepSeek 不自行模拟 Codex 回复，不移动 tag、不替换资产、不批准 `HG-005`。

**Codex 返回后的唯一规则**：有 P0/P1 时由用户重新启动对应实现任务并重跑 FINAL；只有 P2/P3 时登记 backlog。无 P0/P1 后，是否正式发布仍由用户通过 `HG-005` 决定，禁止无限轮审。
