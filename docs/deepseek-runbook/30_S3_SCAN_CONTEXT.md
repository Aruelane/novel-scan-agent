# 30｜S3 全书扫描、长上下文与恢复

本阶段在 S2 多格式导入通过之后执行。目标不是“让模型一次读完整本书”，而是让任意长度的书被拆成有界请求，逐章原子落盘，暂停或崩溃后从最后一个完整章节恢复，并且所有可确认命中都能重新从导入原文构造章节、位置和精确摘录。

永久遵守 [`00_EXECUTION_CONTRACT.md`](./00_EXECUTION_CONTRACT.md)。一次只执行 ledger 中一个任务；不得把本文件一次性当成“大改扫描器”的提示词。

## 阶段前置门

开始 `S3-01` 前必须同时满足：

- `S1-15` 与 `S2-15` 已在 ledger 标为 `DONE`，CI 证据可访问。
- TXT、Markdown、HTML、EPUB、DOCX 和文本型 PDF 中被标为 `ready` 的格式，均能生成 `NovelDocument`、章节指纹和可回证 `SourceLocator`。
- `git status --short` 为空，当前分支是专用开发分支，不直接改写远端 `main`。
- 当前 `novel-core` 的 finding、rule version、provider/model stamp、证据重建和 checkpoint 完整性测试保持通过。

不满足时记录 `BLOCKED`，不得用演示数据替代 S2，也不得继续 S3。

## 工程状态与人工状态必须分开

- `ENGINEERING_DONE`：本阶段代码、host/Windows CI、确定性测试、持久化与恢复门均通过；它不宣称 S6 尚未建立的 Android scaffold/真机验证已经完成。
- `HUMAN_PENDING`：只用于确实需要用户设备、账号或材料的独立证据路径，只暂停那条路径，不阻塞可独立执行的后续工程。缺少 Android target/scaffold 本身既不是 S3 的 `HUMAN_PENDING`，也不是失败；应明确移交 S6。
- 已有明确证据证明依赖或设计不兼容目标平台时，属于工程 `RETRY/BLOCKED`，不能误标为人工等待。
- `ENGINEERING_DONE` 只是报告中的阶段说明标签，不是 ledger 状态。ledger 完成态必须使用完整枚举：`DONE`、`RETRY`、`AWAITING_CI`、`HUMAN_PENDING`、`BLOCKED`；`IN_PROGRESS` 仅表示当前正在执行。

## 本阶段固定架构决定

DeepSeek 不得自行改变以下决定：

1. `novel-core` 继续平台无关：不能依赖 Tauri、React、文件系统、HTTP、Windows 或 Android API。
2. 模型输入使用“章节窗口”，但证据偏移最终必须换算为完整章节的绝对 UTF-8 字节偏移。
3. 一个章节可能产生多个模型请求；只有该章所有窗口、上下文合并、finding 和用量都成功后，才提交该章 checkpoint。中途失败就整章重做。
4. 持久 checkpoint 可保存完整账本与未决 ID；发送给模型的是从 checkpoint 派生的严格有界 `ContextView`。不能把不断增长的 `processed_chapter_ids` 全量发给模型。
5. 摘要、实体、关系和事件都是“推理记忆”，永远不是证据。`confirmed`/`pending_confirmation` 的每个 anchor 都从 `NovelDocument.chapters[*].text` 重新切片。
6. 固定预算按 Unicode 标量字符计数，所有算法必须确定性排序；不得用平台相关 token 估算冒充精确 token 数。真实提供器可在 S4 再施加更严 token 限制。
7. 暂停或取消只在安全点生效。取消当前网络请求后，未完整提交的章节保持未处理；已有 checkpoint 可继续。首版可把“用户取消本次运行”持久化为 `paused + stop_reason=cancelled`，不必破坏现有 task status 枚举。
8. 已进入远端历史的 `0001_initial.sql` 不回写；数据库变化新增编号 migration，并验证从 v1 升级。
9. 长书测试只使用程序生成的原创短句，不提交用户小说、贴吧正文或大体积夹具。

## 每任务固定报告格式

每个 S3 任务完成后，除 `00` 的固定报告外，必须按以下顺序输出并写入 ledger：

```text
任务：S3-XX
状态：DONE / RETRY / AWAITING_CI / HUMAN_PENDING / BLOCKED（IN_PROGRESS 仅执行中）
前置提交：<sha>
修改文件：<逐项列出>
行为变化：<最多 5 条>
明确未实现：<不得省略>
新增/修改测试：<测试名>
验证：<命令 -> 真实退出码/通过数>
边界证据：<预算/事务/恢复/证据，本任务适用者>
diff：<git diff --stat>
工作区：<git status --short>
提交：<checkpoint sha 或未提交原因>
下一任务：<ID>
```

若本机缺 `link.exe`，保留退出码 `101` 和首个真实错误；完成 formatter、metadata、Node 校验后，在 checkpoint 分支用 Windows CI 验证。不得写“Rust 测试应该通过”。

---

## S3-01｜定义 checkpoint 记忆账本 schema

**依赖：** `S2-15`。
**目标：** 在不改变扫描行为的前提下，为滚动摘要、实体、关系、事件和未决候选建立明确、可序列化、可版本校验的持久模型。

**只允许修改：**

- `crates/novel-core/src/compression.rs`
- `crates/novel-core/src/lib.rs`

**只读参考：** `model.rs`、`scanner.rs`、`ARCHITECTURE.md`。

**逐步实现：**

1. 在 `compression.rs` 增加 `CONTEXT_SNAPSHOT_SCHEMA_VERSION`，并给 `ContextSnapshot` 增加 `schema_version`；默认值必须是当前版本，旧 JSON 缺字段反序列化为 `0`，以便恢复时明确拒绝而不是静默接纳。
2. 保留现有 `EntityMemory` 字段，增加确定性 ID、`last_seen_chapter_id` 和短状态；不要存整段正文。
3. 新增 `RelationshipMemory`：稳定 ID、两端 entity ID、关系类型/短状态、最后出现章节。
4. 新增 `EventMemory`：稳定 ID、短描述、参与 entity ID、状态（open/resolved）、最后出现章节。
5. 新增 `UnresolvedMemory`：finding ID、rule ID、短线索、来源章节 ID、最近触达 revision；finding ID 是唯一身份。
6. `ContextSnapshot` 保存上述列表；为旧的 `unresolved_candidates: Vec<String>` 制定兼容策略：本任务只允许提供显式迁移函数或兼容字段，不得静默丢 ID。
7. 所有类型 `Serialize/Deserialize/Clone/PartialEq/Eq`；字符串和列表必须在注释中说明“不属于证据”。
8. 增加 JSON round-trip、旧 schema 被标记为 0、重复 ID 检测辅助函数的单元测试。不要在本任务重写 compressor。

**验证：**

```powershell
cargo fmt --all -- --check
cargo test -p novel-core compression::tests --offline
cargo test -p novel-core --lib --offline
git diff --check
```

**完成门槛：** 账本类型可往返序列化；旧 snapshot 不会被误当当前版本；现有 compressor 测试不退化。
**阻塞策略：** 若 serde 兼容会导致已保存 checkpoint 静默丢字段，停止并记录 `HUMAN_GATE` 讨论迁移，不得用 `#[serde(default)]` 掩盖。
**报告附加字段：** 当前 schema version、旧 JSON 处理结果、兼容字段何时删除。

---

## S3-02｜扩展 provider 的结构化记忆与未决更新合同

**依赖：** `S3-01`。
**目标：** 让 provider 返回候选之外的“记忆增量”和“已存在未决项更新”，但本任务不改变扫描器状态转换。

**只允许修改：**

- `crates/novel-core/src/provider.rs`
- `crates/novel-core/src/lib.rs`
- `crates/novel-core/src/scanner.rs`（仅给现有 `ProviderResponse` struct literal 补齐新字段，并更新同文件内相关测试夹具；不得改变扫描流程）

**逐步实现：**

1. 新增 `MemoryDelta`，包含实体、关系、事件的 upsert 列表；列表默认空。
2. 新增 `CandidateDisposition`（`keep_pending`、`confirm`、`reject`）和 `ProviderCandidateUpdate`：只能引用既有 finding ID，可携带当前窗口证据范围与短理由。
3. `ProviderResponse` 增加 `memory_delta`、`candidate_updates`，均用 serde default 保持旧 JSON 兼容。注意 serde default **不会**让 Rust struct literal 自动补字段：先用 `rg -n "ProviderResponse\\s*\\{" crates/novel-core` 列出全部构造点，再在 `provider.rs` 与 `scanner.rs` 的既有 provider/测试夹具中显式写空默认值。
4. 给 provider 输出设置纯领域上限常量：候选数、更新数、每候选 evidence range 数、rationale 字符数、记忆项数和字段字符数。这里只定义和校验结构，不做 HTTP。
5. 增加 `validate_provider_response_shape`；拒绝 confidence > 10000、空/过长 ID、重复 memory ID、重复 update finding ID、反向或空 evidence range。不能再依赖后续 `.min(10_000)` 修正不可信输出。
6. `DeterministicTestProvider` 明确返回空 delta/update；更新其测试。
7. 只做上述编译传播：不得在 `scanner.rs` 消费、合并或持久化 delta/update；这些行为分别留给 `S3-05`、`S3-06`、`S3-07`。增加/更新测试，证明旧 provider 返回空增量时 finding 与 checkpoint 行为不变。

**验证：**

```powershell
cargo fmt --all -- --check
cargo test -p novel-core provider::tests --offline
cargo test -p novel-core scanner::tests --offline
cargo test -p novel-core --lib --offline
git diff --check
```

**完成门槛：** 合法结构可往返；每种超限/重复/非法范围有负例；全部 `ProviderResponse` struct literal 编译通过；现有测试 provider 仍只被标为测试用途；扫描结果没有因空默认增量改变。
**阻塞策略：** 若必须改变 `Finding` 身份语义，停止并拆成 RETRY；`scanner.rs` 只允许最小 struct-literal 传播，不得借机实现后续状态逻辑。
**报告附加字段：** 所有上限常量和对应负例测试名。

---

## S3-03｜建立严格有界的 ContextView

**依赖：** `S3-01`。
**目标：** 区分“完整持久 snapshot”和“发给模型的有界视图”，证明视图永不超过配置字符预算。

**只允许修改：**

- 新建 `crates/novel-core/src/context_view.rs`
- `crates/novel-core/src/lib.rs`
- `crates/novel-core/src/provider.rs`
- `crates/novel-core/src/scanner.rs`（仅传播 `InferenceRequest.context` 类型、在现有 request 构造点调用 builder，并更新同文件内相关测试；不得改章节循环或提交语义）

**逐步实现：**

1. 定义 `ContextView`，只含 revision、截断后的滚动摘要、选中的实体/关系/事件和未决项；不能包含 processed chapter 全表、原文或 locator URI。
2. 定义唯一的 `context_view_char_count`。计数涵盖所有将序列化给 provider 的字符串和固定分隔开销，使用 `.chars().count()`；不得只数 summary。
3. 定义 `ContextViewBuilder`：输入 `ContextSnapshot`、全部 finding、当前章节相关 entity ID（可为空）和 budget。
4. 选择优先级固定：当前相关未决项 → 最近未决项 → 当前相关实体/关系/事件 → 最近其他记忆 → rolling summary。相同优先级按稳定 ID 排序。
5. 每个字段从末尾截断时不得破坏 Unicode；任何项如果连最小表示都放不下就跳过，并返回 `omitted_counts`。不得省略 finding ID 后仍把短线索放入。
6. budget 为 0 返回明确 `InvalidInput`；构建后再次断言/校验 `char_count <= budget`。
7. `InferenceRequest.context` 改为 `ContextView`。先用 `rg -n "InferenceRequest\\s*\\{" crates/novel-core` 找齐构造点；在 `scanner.rs` 现有 request 构造点从 checkpoint snapshot 和 `task.config.context_budget_chars` 构建视图，只传播编译所需类型，不改变章节循环、provider 调用次数、finding 物化或 checkpoint 提交时机。
8. 测试预算 1、中文、emoji、超长列表、输入顺序打乱仍输出相同、processed IDs 不出现在序列化视图；scanner 聚焦测试还要捕获 request，断言收到的确是有界 `ContextView` 且原有单章扫描结果不变。

**验证：**

```powershell
cargo fmt --all -- --check
cargo test -p novel-core context_view --offline
cargo test -p novel-core scanner::tests --offline
cargo test -p novel-core --lib --offline
git diff --check
```

**完成门槛：** 属性化或循环测试覆盖多组预算，全部 `actual <= budget`；同一集合不同输入顺序得到完全相同 JSON；全部 `InferenceRequest` 构造点编译通过且 scanner 的原有单章行为测试不退化。
**阻塞策略：** 若 `rg` 找到允许文件外的生产构造点，记录精确路径并标 RETRY，请求把该文件加入最小传播范围；不要留下双份不一致上下文，也不要用 `Default` 绕过真实 builder。
**报告附加字段：** 精确计数公式、优先级、最小/最大预算测试结果。

---

## S3-04｜规划 UTF-8 安全章节窗口

**依赖：** `S3-03`。
**目标：** 超长章节不突破单次请求预算，窗口范围能无损映射回完整章节。

**只允许修改：**

- 新建 `crates/novel-core/src/chunking.rs`
- `crates/novel-core/src/lib.rs`
- `crates/novel-core/src/provider.rs`
- `crates/novel-core/src/scanner.rs`（仅把既有 request 构造点传播为单个 whole-chapter `ChapterWindow`，并更新同文件内相关测试；多窗口循环留给 `S3-05`）

**逐步实现：**

1. 定义 `ChapterWindow`：chapter ID/ordinal/title、window index/count、完整章节 content hash、绝对 UTF-8 byte start/end、窗口 text。
2. 定义 `RequestBudget`：`max_request_chars`、`reserved_output_chars`、`window_overlap_chars`；所有字段必须有非零/上界验证。
3. `plan_chapter_windows` 的可用正文字符 = 总预算 - 规则开销 - ContextView 开销 - 固定 prompt 开销 - 输出保留。开销大于预算时返回明确错误，不得生成空请求。
4. 窗口在 Unicode 字符边界切分，byte start/end 必须能在原章 `get(start..end)` 成功；相邻窗口采用固定 overlap，最后一窗覆盖结尾。
5. 单个超长标题/规则开销不能靠截断规则语义绕过；返回“预算太小及最低需求”。
6. `InferenceRequest` 用 `ChapterWindow` 替换完整 `Chapter`；provider 返回范围仍相对 `window.text`，后续 scanner 负责加 absolute start。先用 `rg -n "InferenceRequest\\s*\\{" crates/novel-core` 找齐所有构造点。
7. 为编译传播提供经过边界校验的“整章单窗口”构造方式，并在 `scanner.rs` 的现有 request 构造点使用它：`window_index=0`、`window_count=1`、absolute range 覆盖整章。不得在本任务提前增加多窗口 provider 循环或改变 checkpoint 次数，真正窗口规划接入由 `S3-05` 完成。
8. 更新 deterministic provider 只扫描 window text；更新 scanner 测试夹具，断言短章仍每章只调用一次、range 为 `0..chapter.text.len()`、finding/evidence 与改造前一致。
9. 测试空章、短章、中文/emoji、恰好边界、超长章、overlap、预算不足、所有原文字符被至少一个窗口覆盖。

**验证：**

```powershell
cargo fmt --all -- --check
cargo test -p novel-core chunking --offline
cargo test -p novel-core provider::tests --offline
cargo test -p novel-core scanner::tests --offline
git diff --check
```

**完成门槛：** 任一窗口请求估算不超预算；所有 byte range 合法；窗口规划确定性；所有 `InferenceRequest` 构造点已编译传播，且 scanner 仍保持一章一个兼容窗口，未提前改变提交语义。
**阻塞策略：** 若规则本身超过预算，返回用户可读错误并停该任务扫描，绝不能删规则描述或静默换规则。
**报告附加字段：** 开销公式、overlap 默认值、最大章测试的窗口数和最大请求字符数。

---

## S3-05｜把扫描器改为“多窗口、整章提交”

**依赖：** `S3-02`、`S3-03`、`S3-04`。
**目标：** 一个章节按窗口调用 provider，聚合结果并换算绝对范围，但仍只在整章成功后更新 checkpoint。

**只允许修改：**

- `crates/novel-core/src/model.rs`
- `crates/novel-core/src/scanner.rs`
- `crates/novel-core/src/provider.rs`
- `crates/novel-core/src/lib.rs`

**逐步实现：**

1. 给 `ScanConfig` 增加带 serde default 的 `max_request_chars`、`reserved_output_chars`、`window_overlap_chars`；保留 `context_budget_chars` 表示 ContextView 上限。
2. 每章开始时从旧 checkpoint 构建一次有界 ContextView，再按剩余预算规划 windows。
3. 顺序处理窗口并先放入临时 `ChapterWork`；任何 window 失败时丢弃本章临时 findings/context，checkpoint position 不变。
4. 将 provider range 用 checked addition 转为完整章节绝对 byte range，再从 `chapter.text` 重建 evidence。禁止从 `window.text` 复制 exact quote 后直接落盘。
5. overlap 产生的相同绝对候选按稳定 finding ID 去重；ID 材料使用绝对 evidence ranges，不得依赖窗口 index。
6. 调用 S3-02 shape validator；未知 rule ID、非法范围或超限输出作为 provider contract error，不能静默 filter/clamp。
7. 所有窗口成功后才调用 compressor 一次、推进 chapter position 一次、保存一次。
8. 更新现有 inline scanner tests，并增加“第二窗失败不推进”“overlap 去重”“中文绝对 offset”“短章只一窗”。

**验证：**

```powershell
cargo fmt --all -- --check
cargo test -p novel-core scanner::tests --offline
cargo test -p novel-core --lib --offline
git diff --check
```

**完成门槛：** 窗口失败时 store 无本章提交；成功时只提交一次；证据绝对偏移能从完整章节切回同一摘录。
**阻塞策略：** 若为了过测试需要保存半章 checkpoint，停止并记录架构冲突；禁止这样实现。
**报告附加字段：** 每章提交次数、失败窗位置、绝对 offset 示例。

---

## S3-06｜确定性合并滚动摘要和三类账本

**依赖：** `S3-01`、`S3-02`、`S3-05`。
**目标：** compressor 合并 memory delta，更新滚动摘要和未决项，结果与 provider 返回顺序无关。

**只允许修改：**

- `crates/novel-core/src/compression.rs`
- `crates/novel-core/src/scanner.rs`（只传播 `MemoryDelta`）

**逐步实现：**

1. `CompressionRequest` 增加本章 `MemoryDelta`，scanner 只在所有窗口成功后合并各窗 delta。
2. 合并 entity/relationship/event 时按稳定 ID upsert；同 revision 重复 ID 且内容冲突必须报错，完全相同可去重。
3. 列表最终按稳定 ID 排序；参与 entity ID 去重排序；不保留整章正文。
4. rolling summary 继续受独立字符上限约束，Unicode 安全；不要把 evidence exact quote 大段复制进 summary。
5. unresolved 由 finding 的真实状态派生：suspected/pending 存在，confirmed/rejected 移除；不得相信 provider 自报列表。
6. 校验 relationship 的两端 entity 必须存在，event participant 必须存在；未知引用作为 compression error，不伪造 entity。
7. 加入顺序置换、重复相同、重复冲突、状态更新、Unicode budget 测试。

**验证：**

```powershell
cargo fmt --all -- --check
cargo test -p novel-core compression::tests --offline
cargo test -p novel-core scanner::tests --offline
git diff --check
```

**完成门槛：** 相同语义、不同输入顺序产生 byte-for-byte 相同 snapshot JSON；未决列表与 finding 状态一致。
**阻塞策略：** provider 给出未知 entity 引用时保留旧 checkpoint并返回可诊断错误，不自动创建“unknown”。
**报告附加字段：** upsert 冲突规则、排序键、summary 最大字符实测。

---

## S3-07｜实现未决 finding 的合法状态转换

**依赖：** `S3-02`、`S3-05`、`S3-06`。
**目标：** 后续章节可保留、确认或排除既有候选，且任何确认仍有原文证据和 scope gate。

**只允许修改：**

- `crates/novel-core/src/scanner.rs`
- `crates/novel-core/src/model.rs`

**逐步实现：**

1. 为 finding 状态转换写一个独立私有函数和状态表；禁止在 scan loop 散落赋值。
2. update 只能引用 checkpoint 中 `Suspected`/`PendingConfirmation` 的 finding；未知、重复、已 resolved ID 拒绝整个章节。
3. `keep_pending` 可添加从当前章重建的 anchor，但不能降低已有证据完整性。
4. `confirm` 必须至少有一个有效原文 anchor；`CrossChapter` 需要来源跨至少两个已处理章节，`WholeBook` 只允许最后一章完成时确认，`requires_user_boundary=true` 仍不能自动 confirmed。
5. `reject` 保留 finding 和历史 evidence，写短 verification note，并从 unresolved 移除。
6. stable finding ID、rule version、category、alert 和 provider/model snapshot 不变。
7. 状态更新和本章新 findings 一起进入该章临时工作，失败不改旧 checkpoint。
8. 测试所有允许/禁止转换、未知 ID、重复 update、全书过早确认、user boundary gate、后文证据定位。

**验证：**

```powershell
cargo fmt --all -- --check
cargo test -p novel-core scanner::tests --offline
cargo test -p novel-core --lib --offline
git diff --check
```

**完成门槛：** 没有 exact source anchor 的 update 永远不能 confirmed；恢复校验会重新应用同一状态表。
**阻塞策略：** 若规则语义无法自动确认，保持 `pending_confirmation` 并写 verification note，不猜测用户边界。
**报告附加字段：** 状态转换表、每个 gate 的测试名。

---

## S3-08｜加入 provider-neutral 用量事件和扫描预算

**依赖：** `S3-05`。
**目标：** 按请求和章节累计输入/输出单位，达到用户预算后在下一安全点暂停，不伪造价格。

**只允许修改：**

- `crates/novel-core/src/model.rs`
- `crates/novel-core/src/provider.rs`
- `crates/novel-core/src/scanner.rs`
- `crates/novel-core/src/lib.rs`

**逐步实现：**

1. 定义 `UsageBudget`：可选 max requests/input/output/total units；`None` 表示未设，0 必须有明确语义并测试。
2. 定义 `UsageEvent`：稳定 event ID、task/chapter/window、provider/model、input/output units、attempt、outcome、timestamp 由调用方传入或使用确定性 sequence；不得存正文或 prompt。
3. `ScanCheckpoint` 保存累计 totals 和已提交 usage events，恢复时验证 event ID 唯一、stamp 一致、sum 与 totals 一致。
4. 每个成功窗口的 `ProviderUsage` 转为事件；整章提交前计算本章合计。达到/超过限制时允许当前已经完成的章原子落盘，然后返回 `paused + BudgetReached`；下一请求不得发出。
5. 在请求前若剩余 request 数为 0，直接暂停；未知 provider 计费单位只能显示“provider units”，不得换算金额。
6. 所有加法使用 checked/saturating 策略并明确溢出错误，不 wrap。
7. 测试多窗口累计、恢复后继续累计、恰好到限、当前章轻微越限、0 请求、篡改 totals/event。

**验证：**

```powershell
cargo fmt --all -- --check
cargo test -p novel-core usage --offline
cargo test -p novel-core scanner::tests --offline
git diff --check
```

**完成门槛：** 任意恢复路径的 totals 等于事件求和；预算耗尽后 provider 调用次数不增加。
**阻塞策略：** provider 未返回 usage 时记录 `units_known=false` 和请求次数，UI 明示未知；不得填 0 后声称无消耗。
**报告附加字段：** 预算到限语义、未知 usage 表示、溢出测试。

---

## S3-09｜暂停、取消与安全点控制

**依赖：** `S3-05`、`S3-08`。
**目标：** UI/平台能请求暂停或取消，扫描器在请求前后和事务前检查，不留下半章状态。

**只允许修改：**

- 新建 `crates/novel-core/src/control.rs`
- `crates/novel-core/src/lib.rs`
- `crates/novel-core/src/provider.rs`
- `crates/novel-core/src/scanner.rs`

**逐步实现：**

1. 用标准库原子类型实现 cloneable `ScanControl`，状态至少 `running/pause_requested/cancel_requested`；不要在 core 引入 Tokio。
2. `ModelProvider::analyze` 接收只读 cancellation handle，使 S4 HTTP adapter 可主动取消；更新 deterministic provider。
3. scanner 在章前、每窗前、provider 返回后、compression 返回后、commit 前检查控制状态。
4. pause：不启动新窗，保留最后完整章 checkpoint；cancel：请求 provider 取消，丢弃当前章临时结果，返回可恢复的 stop reason。
5. 若 cancel 恰好发生在持久化成功后，只能把该章视为已提交；不得回退已提交事务。
6. provider/compressor error 不自动标完成；重跑从同一 next chapter 开始，stable ID 防重复。
7. 使用可控 future 和 fake store 测试每个时序点，禁止 sleep-based flaky 测试。

**验证：**

```powershell
cargo fmt --all -- --check
cargo test -p novel-core control --offline
cargo test -p novel-core scanner::tests --offline
git diff --check
```

**完成门槛：** 所有时序测试中 checkpoint 只处于“上一完整章”或“本章完整提交”两种状态。
**阻塞策略：** 某 provider 不能取消时允许等到其超时，但必须标明 `cancellation_pending`，不能假装已停止；S4 再实现网络取消。
**报告附加字段：** 安全点清单和每个时序测试最终 position。

---

## S3-10｜强化恢复指纹与 schema 验证

**依赖：** `S3-01` 至 `S3-09`。
**目标：** 任何会改变窗口、上下文、prompt 或状态转换的配置变化都让旧 checkpoint 明确失效。

**只允许修改：**

- `crates/novel-core/src/compression.rs`
- `crates/novel-core/src/provider.rs`
- `crates/novel-core/src/scanner.rs`

**逐步实现：**

1. `ContextCompressor` 暴露稳定 `compressor_id` 与 `compressor_version`；deterministic 实现固定值。
2. scan profile fingerprint 加入 context snapshot schema、provider request schema、compressor identity/version、所有窗口/预算参数和用量预算。
3. 长度前缀或规范编码每个可变字符串，避免拼接歧义；排序规则保持确定性。
4. 恢复时先验证 checkpoint/context schema，再验证 document/chapter hash、profile、position、finding、usage、unresolved。
5. fingerprint 错误消息列出“哪类身份不匹配”，但不包含正文、路径或秘密。
6. 测试逐一改变每个字段都 ResumeMismatch；输入顺序变化但语义相同不改变 fingerprint；provider/model 旧测试保留。

**验证：**

```powershell
cargo fmt --all -- --check
cargo test -p novel-core resume --offline
cargo test -p novel-core --lib --offline
git diff --check
```

**完成门槛：** 新增的每个行为配置都有“一项变化即失效”测试；旧 schema 不 panic。
**阻塞策略：** 不提供危险的“忽略指纹继续”参数；UI 只能新建扫描任务或明确重扫。
**报告附加字段：** fingerprint 字段清单、旧 checkpoint 用户可见处理。

---

## S3-11｜定义章节原子提交持久化合同

**依赖：** `S3-08`、`S3-09`。
**目标：** 用一个领域合同表达“本章 findings/evidence/状态更新/usage/context/checkpoint 一起成功或一起失败”。

**只允许修改：**

- 新建 `crates/novel-core/src/persistence.rs`
- `crates/novel-core/src/lib.rs`
- `crates/novel-core/src/scanner.rs`

**逐步实现：**

1. 新增 `ChapterCommit`，包含预期旧 position、完整新 checkpoint、本章新增/更新 finding ID、本章 usage event ID。
2. 新增 `ScanPersistence`：load、`commit_chapter`、`save_run_state`；commit 必须做 compare-and-swap position，防止两个运行实例同时提交。
3. 将旧 `CheckpointStore` 保留为 deprecated 兼容层或一次性迁移；不得同时保存两套互相可能不一致的状态。
4. 改造 in-memory store，在单一 mutex 临界区验证旧 position 后原子替换；提供注入 `fail_before_commit`，失败不变。
5. zero-size batch、pause 和 failure 只允许更新 run state，不制造“已处理但无 findings/context”的章节。
6. scanner 完成一章只调用一次 `commit_chapter`。
7. 测试并发陈旧 position、commit 前失败、重复提交、已经完成任务恢复。

**验证：**

```powershell
cargo fmt --all -- --check
cargo test -p novel-core persistence --offline
cargo test -p novel-core scanner::tests --offline
git diff --check
```

**完成门槛：** fake store 日志证明成功章一次 commit；失败时 checkpoint/findings/usage/context 全不变。
**阻塞策略：** 若兼容旧 trait 会造成双写，删除兼容调用并一次传播；不得用“先 save findings 再 save checkpoint”。
**报告附加字段：** commit payload、CAS 条件、失败注入结果。

---

## S3-12｜新增 v2 扫描运行与 usage 数据迁移

**依赖：** `S3-08`、`S3-11`。
**目标：** 在 SQLite 中保存用量和运行停止原因，并验证从现有 v1 schema 升级；不回写 `0001_initial.sql`。

**只允许修改：**

- 新建 `apps/desktop/src-tauri/migrations/0002_scan_runtime.sql`
- `apps/desktop/src-tauri/src/lib.rs`（只注册 migration 2）
- `apps/desktop/scripts/validate-migration.mjs`

**逐步实现：**

1. 新 migration 增加 `usage_events`，字段映射 S3-08；正文、prompt、response、API key 不得有列。
2. 给 `scan_jobs` 增加 stop reason、已知/未知 usage totals 和预算列；用 SQLite 支持的 ALTER/新表方式，约束非负。
3. 用 trigger/约束保证 usage provider/model 与 scan job snapshot 一致、chapter 属于同一本书、event ID 唯一。
4. checkpoint JSON 仍保存上下文账本；不要在两个表保存可漂移的第二份完整 snapshot。
5. validator 先应用 v1 fixture，再应用 v2；测试 fresh install、upgrade、合法插入、负数/跨任务/错误 stamp/明文 secret-like 列缺失。
6. 迁移重复执行应按迁移框架只执行一次；不要用 `IF NOT EXISTS` 掩盖漏记版本。

**验证：**

```powershell
& '.\.toolchain\node-v24.18.0-win-x64\node.exe' '.\apps\desktop\scripts\validate-migration.mjs'
cargo fmt --all -- --check
cargo metadata --locked --offline --format-version 1 --no-deps
git diff --check
```

**完成门槛：** validator 全通过，包含从 v1 升级；`PRAGMA foreign_key_check` 空、`integrity_check` 为 ok；v1 文件无 diff。
**阻塞策略：** SQLite 无法安全 ALTER 时用新表复制并在单事务换名；必须测试旧数据保留，不能 DROP 后算完成。
**报告附加字段：** migration version、升级前后行数、validator 新增测试数。

---

## S3-13｜实现 SQLite 原子 ScanPersistence

**依赖：** `S3-11`、`S3-12`。
**目标：** 在原生 Rust 层用一笔 SQLite transaction 保存一章全部状态，并能恢复领域 checkpoint。

**只允许修改：**

- `apps/desktop/src-tauri/Cargo.toml`
- `Cargo.lock`
- 新建 `apps/desktop/src-tauri/src/persistence.rs`
- `apps/desktop/src-tauri/src/lib.rs`（只声明模块/构造 store）

**逐步实现：**

1. 选择平台中立、没有已知 Windows/Android 排斥项的 SQLite Rust 访问方式；优先复用现有依赖。若必须新增 `rusqlite`/`sqlx`，先用 `cargo metadata` 和依赖 feature/target 条件审计确认没有大范围升级、没有显式 desktop-only API，并记录理由。S6 scaffold 尚未建立时不要求安装或编译 Android target。
2. `SqliteScanPersistence` 实现 S3-11；事务开始后验证 scan job 和旧 position，upsert finding、重建 evidence rows、insert usage、update checkpoint/job position，最后 commit。
3. finding 更新不得改变 rule/version/category/alert/provider/model identity；使用 SQL 约束和 Rust 校验双层保护。
4. JSON 序列化失败、任一 SQL 失败、CAS 失败都 rollback；错误不包含正文、完整路径或 SQL 参数值。
5. load 后必须调用 core `validate_checkpoint` 等价公开校验，不能只反序列化就恢复。
6. 用 in-memory SQLite 应用 v1+v2 migration，测试成功提交、findings/evidence/usage/checkpoint 行数、故障 rollback、陈旧 position、重新打开恢复。
7. 不在本任务创建 Tauri UI command。

**验证：**

```powershell
cargo fmt --all -- --check
cargo metadata --locked --offline --format-version 1 --no-deps
cargo test -p novel-scout-desktop persistence --offline
cargo check --workspace --all-targets --offline
git diff --check
```

**完成门槛：** host/Windows CI 的 rollback 测试中所有相关表保持提交前状态；成功后 load 得到 byte-for-byte 等价 checkpoint；依赖与接口审计未发现目标平台绑定。此处达到 `ENGINEERING_DONE` 不代表 Android 真机构建完成。
**阻塞策略：** 只有依赖元数据、官方支持矩阵或源码中的 target cfg 已证明存在 Android 不兼容，才把本任务标 `BLOCKED`；仅仅“本地没有 Android target/scaffold”不阻塞，记录“Android compile deferred to S6”并继续后续独立工程。不得写 Windows-only store 冒充共享实现。
**报告附加字段：** 依赖变化、事务 SQL 顺序、rollback 行数、host/Windows CI 证据、target-neutral 依赖审计结论、移交 S6 的 Android 编译项。

---

## S3-14｜故障、重试和崩溃恢复矩阵

**依赖：** `S3-09`、`S3-10`、`S3-13`。
**目标：** 用确定性 fault injection 证明 provider/compressor/SQLite/进程中断均不损坏最后完整章节。

**只允许修改：**

- 新建 `crates/novel-core/tests/recovery_matrix.rs`
- `apps/desktop/src-tauri/src/persistence.rs`（仅测试注入接口）
- 新建 `apps/desktop/src-tauri/tests/sqlite_recovery.rs`（若 crate 布局支持；否则同模块 `#[cfg(test)]`）

**逐步实现：**

1. 建立失败点：第 N 窗 provider error、provider cancellation、compression error、序列化 error、SQL 每个写入步骤前失败、commit 后客户端未收到响应。
2. 每种失败后销毁 engine/store handle，重新打开并继续，而不是在同对象里假恢复。
3. 比较“不失败一次扫完”和“失败后恢复”的最终 finding（含 ID/状态/证据）、context、usage totals；除明确的失败尝试 telemetry 外必须一致。
4. commit 已成功但响应丢失时，重试必须由 CAS/稳定 ID识别已提交章，不重复 findings/usage。
5. 源文/章节 hash、规则、provider、compressor、预算任一变化均拒绝恢复。
6. 测试无 sleep、无网络、无用户文件。

**验证：**

```powershell
cargo fmt --all -- --check
cargo test -p novel-core --test recovery_matrix --offline
cargo test -p novel-scout-desktop sqlite_recovery --offline
git diff --check
```

**完成门槛：** 恢复矩阵每一行都有断言；无 duplicate ID；失败点后数据库 integrity/foreign key clean。
**阻塞策略：** 本地 link.exe 阻塞时推专用分支跑 Windows CI；CI 未绿不得标 DONE。
**报告附加字段：** 矩阵行数、每行恢复 position、最终等价比较字段。

---

## S3-15｜Tauri 扫描命令与事件桥

**依赖：** `S3-13`、`S3-14`。
**目标：** 原生层提供创建任务、执行下一批、暂停、继续、取消、查询进度/结果的 typed command；不引入真实在线 provider。

**只允许修改：**

- 新建 `apps/desktop/src-tauri/src/scan_commands.rs`
- `apps/desktop/src-tauri/src/lib.rs`
- `apps/desktop/src-tauri/Cargo.toml`
- `Cargo.lock`
- `apps/desktop/src-tauri/capabilities/main-local.json`（仅确需新增权限时）

**逐步实现：**

1. 定义 camelCase DTO，不能把 `Chapter.text`、原生路径、SAF URI 或完整 checkpoint 发送前端。
2. command 至少：create job、run next batch、request pause、request cancel、resume、get job、list findings、get evidence detail。
3. 每个 job 只允许一个 active runner；第二次 start 返回 typed conflict，不并发扫描同一 checkpoint。
4. 使用 app-managed state 保存 runner handle/ScanControl；持久真相来自 SQLite，不依赖窗口内存。
5. 进度按已提交章节数和已提交原文字符数计算；不得伪造“模型百分比”或剩余时间。
6. 本阶段只允许显式标注的 deterministic test provider 用于开发/测试。没有 S4 provider 时 create/run 返回“尚未配置真实模型”，不能把测试 provider 展示为 AI。
7. 事件只含 job ID、status、position、totals 和 finding count，不含正文/密钥。
8. command 单元测试覆盖非法 ID、重复 runner、pause/cancel、重启后查询。

**验证：**

```powershell
cargo fmt --all -- --check
cargo test -p novel-scout-desktop scan_commands --offline
cargo check --workspace --all-targets --offline
git diff --check
```

**完成门槛：** command DTO 无敏感字段；两个 runner 不能同时提交；重建 app state 后仍能查询/恢复。
**阻塞策略：** 若后台长任务需要 Android 前台服务，登记为 S6 依赖；本任务只保证领域/持久化和短批 command，不声称后台存活。
**报告附加字段：** command/event 清单、前端可见字段、并发冲突测试。

---

## S3-16｜前端接入真实任务进度和控制

**依赖：** `S3-15`。
**目标：** 对已真实导入的书显示持久进度、用量和暂停/继续/取消结果，移除该路径的 demo 状态。

**只允许修改：**

- `apps/client/src/domain.ts`
- 新建 `apps/client/src/services/scanService.ts`
- `apps/client/src/hooks/useAppState.ts`
- `apps/client/src/components/ScanProgress.tsx`
- `apps/client/src/components/ScanProgress.css`
- 新建或修改 `apps/client/src/__tests__/ScanProgress.test.tsx`
- 新建或修改 `apps/client/src/__tests__/scanService.test.ts`

**逐步实现：**

1. TypeScript DTO 与 Tauri camelCase command 一一对应；`progress` 从 committed/total 计算，不能由后端传任意 float。
2. 浏览器预览没有 Tauri 时显示“本地预览，未连接扫描服务”，不能回退为看似真实的运行中 demo。
3. start 前若无真实 provider，按钮 disabled 并说明 S4 配置；测试 provider 仅在显式开发标志下显示“测试扫描”。
4. pause/cancel 请求显示“正在到达安全停止点”，收到持久状态后才显示已暂停。
5. 显示已提交章节/字符、请求数、输入/输出 provider units、usage 是否未知；不要显示虚构剩余时间。
6. 错误按 retryable/配置变化/源文变化/预算到限分类，给出“重试本章/新建扫描/调整预算”可恢复动作。
7. 使用订阅事件加定期 query 兜底，卸载组件时清理 listener/timer。
8. 测试浏览器 fallback、真实 DTO、暂停 pending、取消、预算到限、源文变化、键盘操作和 aria-live。

**验证：**

```powershell
Push-Location apps/client
& '..\..\.toolchain\node-v24.18.0-win-x64\node.exe' '.\node_modules\vitest\vitest.mjs' run src/__tests__/ScanProgress.test.tsx src/__tests__/scanService.test.ts
& '..\..\.toolchain\node-v24.18.0-win-x64\node.exe' '.\node_modules\typescript\bin\tsc' -b
& '..\..\.toolchain\node-v24.18.0-win-x64\node.exe' '.\node_modules\vite\bin\vite.js' build
Pop-Location
git diff --check
```

**完成门槛：** 正式路径不再由 `demoScanJobs` 驱动；UI 状态来自持久 command/query；无 provider 时本地功能仍可用。
**阻塞策略：** 不得为了浏览器预览继续伪造扫描完成；使用明确 offline shell。
**报告附加字段：** demo 路径剩余位置、真实/预览状态文案、前端测试数。

---

## S3-17｜证据详情与来源章节回跳闭环

**依赖：** `S3-15`、`S3-16`，以及 `S2` 的格式 locator。
**目标：** 用户从 finding 打开来源章节，看见由原文重建的精确摘录和格式相关位置；摘要内容不能冒充证据。

**只允许修改：**

- `crates/novel-core/src/scanner.rs`（仅公开/复用 evidence revalidation）
- `apps/desktop/src-tauri/src/scan_commands.rs`
- `apps/client/src/domain.ts`
- `apps/client/src/components/EvidencePanel.tsx`
- `apps/client/src/components/EvidencePanel.css`
- `apps/client/src/__tests__/EvidencePanel.test.tsx`

**逐步实现：**

1. 抽出/公开只读 `rebuild_evidence_anchor(document, finding/anchor identity)`；每次返回详情前复核 chapter hash、byte boundaries、quote hash 和 locator。
2. Tauri 详情 command 只按 finding ID 查询，原生层从数据库章节正文重新切片；不信任 SQLite 的 `exact_quote` 单独值。
3. DTO 返回 display-safe locator：TXT/MD 行、EPUB resource/fragment、PDF 页、DOCX 段、HTML resource/selector；不返回完整本机路径或 SAF URI。
4. `suspected` 无有效 anchor 时只显示“无法从原文核验”，不得显示模型 rationale 为引文。
5. `pending_confirmation` 清楚区分“原文线索已核验”和“规则结论仍待后文/用户确认”。
6. 源文变化/锚点失效时详情 command 返回 stale evidence，UI 阻止显示旧摘录并建议重新导入/重扫。
7. 测试所有 SourceLocator DTO、中文 byte range、tamper、suspected/pending/confirmed、键盘关闭和读屏标签。

**验证：**

```powershell
cargo fmt --all -- --check
cargo test -p novel-core evidence --offline
cargo test -p novel-scout-desktop evidence --offline
Push-Location apps/client
npm test -- --run src/__tests__/EvidencePanel.test.tsx
npm run build
Pop-Location
git diff --check
```

**完成门槛：** confirmed/pending 摘录均由当前原文重建；章节和 locator 可见；tamper/stale 不显示旧引文。
**阻塞策略：** 某 S2 ready 格式不能提供 display locator 时回退 S2 RETRY，不能在 UI 假造段落/页码。
**报告附加字段：** 各格式 locator 示例（只用原创夹具）、tamper 行为。

---

## S3-18｜长书与阶段门禁

**依赖：** `S3-01` 至 `S3-17`。
**目标：** 用三档程序生成长书和真实 SQLite 恢复测试证明固定请求预算、稳定结果和无内容泄露。

**只允许修改：**

- 新建 `crates/novel-core/tests/long_book.rs`
- 新建 `apps/desktop/src-tauri/tests/scan_end_to_end.rs`（或现有测试模块中的同名 gate）
- `.github/workflows/ci.yml`（仅增加 S3 gate 命令/超时，不删除旧步骤）
- `docs/deepseek-runbook/02_TASK_LEDGER.md`
- `docs/deepseek-runbook/80_ACCEPTANCE_MATRIX.md`
- `docs/deepseek-runbook/03_BLOCKERS_AND_DEBT.md`（仅发现事实时）

**逐步实现：**

1. 程序生成 small/medium/long 三档原创书；建议 20/200/2000 章，包含中文、emoji、超长单章、跨窗短语和可确定匹配。不要把生成正文打印到日志。
2. 记录每次 provider request 的估算字符数，断言全部 `<= max_request_chars`，并断言书长增加不会让单次请求线性增长。
3. 对每档比较 uninterrupted、每章暂停恢复、每 17 章重建 engine、指定故障点恢复的最终 finding/context/usage。
4. 修改一个已处理章节字符，断言旧 checkpoint 失效且旧 evidence 不显示。
5. SQLite E2E 断言每章原子提交、外键/完整性 clean、无重复 finding/usage。
6. 扫描测试日志和临时数据库 schema，确认没有 API key 字段、完整 prompt/response 或非夹具正文日志。
7. 在 Windows CI 运行 workspace fmt/check/test，再运行 Node migration、前端 test/build；保存 run URL。
8. 只有全部通过才在 acceptance matrix 更新 S3 项为 `PASS`；Android 进程回收/前台任务仍留 S6，不能提前 PASS。

**验证：**

```powershell
cargo fmt --all -- --check
cargo test -p novel-core --test long_book --offline
cargo test -p novel-scout-desktop scan_end_to_end --offline
cargo test --workspace --all-targets --offline
Push-Location apps/client
npm test
npm run build
Pop-Location
& '.\.toolchain\node-v24.18.0-win-x64\node.exe' '.\apps\desktop\scripts\validate-migration.mjs'
git diff --check
```

**完成门槛：**

- 三档全部扫完；最大单请求不超预算。
- 恢复前后 finding ID/状态/证据、context revision/ledger、已知 usage totals 一致。
- 源文变化明确拒绝旧 checkpoint。
- Windows CI 全绿，有 run URL；无敏感内容进入 Git/diff/log。

**阻塞策略：** 性能慢但正确时记录数据和 DEBT；预算突破、恢复不一致、证据失效、事务半提交属于 `RETRY`，不得进入 S4。
**报告附加字段：** 三档章数/总字符/窗口数/最大请求字符、四种运行模式 hash、CI URL、S3 acceptance 行。

## S3 最终产物清单

完成 `S3-18` 时应能用代码和测试证明：

- 长章窗口化、全书逐章扫描且单次请求固定有界；
- 滚动摘要、实体、关系、事件和未决项有 schema、确定性合并与恢复校验；
- finding 后文状态更新受 scope/user boundary/原文证据约束；
- 用量、结果、上下文和 checkpoint 每章原子提交；
- 暂停、取消、重试、崩溃和响应丢失均从最后完整章恢复；
- 文档/章节/规则/provider/compressor/预算变化使旧 checkpoint 失效；
- UI 显示真实已提交进度并能回到章节与格式 locator；
- 测试 provider 始终明确标为测试，真实模型接入留给 S4。
