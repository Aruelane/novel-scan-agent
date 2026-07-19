# S5：社区规则核验、用户预设与版本迁移

本文件是给 DeepSeek 的逐任务执行提示词。必须遵守 `00_EXECUTION_CONTRACT.md`：一次只做一个任务；只改该任务列出的路径；真实命令、退出码和阻塞必须写回台账。总任务数：**15**。

## S5 总边界

- 指定首要来源是 `https://tieba.baidu.com/p/10485825386?fr=frs`；补充研究范围是百度贴吧“yy小说吧”里与扫书、雷点、郁闷点直接相关的公开内容。
- 搜索摘要、模型常识、现有 seed 名称和 OCR 初稿都不是“已核验社区规则”。禁止伪造标题、楼层、作者、日期、图片序号、定义或例子，禁止绕过登录、验证码、访问控制和 robots 限制。
- 当前 `packages/rulepack/packs/yy-novel-bar/2026.0.0-seed.1.json` 是 11 个雷点、21 个郁闷点的产品候选 seed；必须保留且默认关闭未核验项，不得原地改写为“社区已核验”。
- 图片来源允许用 OCR 生成待复核转写，但只有人工逐字核对后才能升级为 `verified`。DeepSeek 没有视觉能力时只运行 OCR、结构校验与证据登记，不解释截图内容。
- 阶段说明可以写 `ENGINEERING_DONE` 表示工具、schema、测试和未核验降级路径已完成，但 task ledger 不使用该值；等待人工时 ledger 写 `HUMAN_PENDING`。人工门不阻止不依赖真实社区语义的预设、自定义规则、迁移和 UI 工程继续进行。
- `HG-002A`：指定首帖与必要补充材料的逐字核验；`HG-002B`：另一名人工复核者对 32 行映射、默认开关和定位器做第二复核。DeepSeek 不得自称第二复核者；这两个 ID 必须由总控同步登记到 `03_BLOCKERS_AND_DEBT.md`。
- 只保存核验所需的短摘录或释义；不保存整帖、整套截图或小说正文。测试小说必须原创或明确授权。
- 已发布规则包不可原地修改。历史任务必须保存 `packId@packVersion`、每条 `rule_version` 和规则选择快照。
- 每条 `Finding` 继续只保存一个 `rule_id` 和一个 `rule_version`。跨分类同概念必须产生各自 finding；需要合并阅读体验时只在查询/展示层按 `conceptId` 聚合，禁止把 Finding 改成多规则身份。

## 通用执行协议

每个任务开始先运行：

```powershell
git status --short
git rev-parse --short HEAD
```

若出现不属于上一个已登记任务的改动，立即停止；禁止 `git reset --hard`、`git checkout --`、`git clean -fd`、强推和跳过测试。每个任务结束运行：

```powershell
git diff --check
git diff --stat
git status --short
```

报告必须列出：任务 ID、基线 SHA、实际修改文件、关键设计、每条命令及退出码、测试通过数、未执行项与原因、阻塞、最终状态、提交 SHA。task ledger 状态只能使用 `DONE`、`RETRY`、`AWAITING_CI`、`HUMAN_PENDING`、`BLOCKED`；`IN_PROGRESS` 只在执行中使用。工程失败但可继续修复写 `RETRY`，只有确实等待外部资源且无安全替代时才写 `BLOCKED`。

## 任务依赖图

| 任务 ID | 依赖 | 原子结果 |
| --- | --- | --- |
| S5-RULE-01 | S1-15 | 来源台账 schema、校验器和负例 |
| S5-RULE-02 | S5-RULE-01 | 指定首帖及补充来源的证据登记；可能为 `HUMAN_PENDING` |
| S5-RULE-03 | S5-RULE-02（`DONE`，或工程交付已提交的 `HUMAN_PENDING`） | 只从已核验行生成新规则包；不足时保留 RC/不发布 |
| S5-RULE-04 | S5-RULE-03 | 规则不变量与展示层概念聚合，不改 Finding 单规则身份 |
| S5-PRESET-01A | S5-RULE-04 | 预设领域模型和三层合并纯函数 |
| S5-PRESET-01B | S5-PRESET-01A | 预设与每书覆盖的 SQLite repository |
| S5-PRESET-01C | S5-PRESET-01B | 扫描选择快照与 Tauri 命令 |
| S5-PRESET-02 | S5-PRESET-01C | 普通用户可理解的规则选择 UI |
| S5-CUSTOM-01A | S5-PRESET-01A | 自定义规则导入 schema、解析与安全限制 |
| S5-CUSTOM-01B | S5-CUSTOM-01A、S5-PRESET-01B | 自定义规则持久化与 Tauri commands |
| S5-CUSTOM-01C | S5-CUSTOM-01B | 导入预览、导出与 UI 闭环 |
| S5-UPGRADE-01A | S5-RULE-04、S5-CUSTOM-01A | 规则包差异和迁移计划纯函数 |
| S5-UPGRADE-01B | S5-UPGRADE-01A、S5-PRESET-01B | 事务化迁移应用与回滚 |
| S5-UPGRADE-01C | S5-UPGRADE-01B、S5-PRESET-01C | 历史快照复现和升级确认 UI |
| S5-GATE-01 | S5-RULE-04、S5-PRESET-01C、S5-PRESET-02、S5-CUSTOM-01C、S5-UPGRADE-01C | S5 工程验收包和两个人工门状态；S5-RULE-02/03 可为 human pending |

---

## S5-RULE-01：来源台账 schema 与验证器

**目标**：让“发现线索”“OCR 未复核”“人工核验”成为不可混淆的机器状态。

**精确范围**：只允许新建或修改 `packages/rulepack/research/yy-novel-bar/**`、`packages/rulepack/schemas/source-ledger.schema.json`、`packages/rulepack/scripts/validate-sources.mjs`、对应负例 fixture、`packages/rulepack/package.json` 和确有需要的 `.github/workflows/ci.yml`。不得修改正式 pack、Rust scanner 或 UI。

**执行**：

1. 先读现有 rulepack schema、seed、validator、README 和测试。
2. 定义稳定 `sourceId`、URL、来源类型、访问时间、公开访问状态、可验证元数据、精确定位器、获取方式、OCR 状态、人工逐字复核状态、短摘录/释义哈希和备注。
3. 状态至少区分 `verified_primary`、`verified_supplemental`、`metadata_only`、`ocr_unreviewed`、`blocked_login`、`blocked_captcha`、`not_found`、`unavailable`。
4. 建立规则槽位到来源定位器的多对多映射，分类只能是 `landmine` 或 `frustration`。
5. 加入缺 URL、伪楼层、搜索摘要标 verified、OCR 未复核标 verified、verified 无定位器、重复 source ID 等负例。

**测试/门槛**：正负验证命令均退出 0；每个负例被期望规则拒绝；旧 seed SHA-256 不变。完成状态为 `DONE`。

---

## S5-RULE-02：公开来源采集与人工核验登记

**目标**：形成 11+21 每一槽位可追溯的真实证据状态，不修改产品代码。

**精确范围**：只允许修改 `packages/rulepack/research/yy-novel-bar/**` 中的台账、短说明、OCR 中间文件和研究验证 fixture。不得修改正式 pack、schema 语义、Rust、数据库、UI 或 CI。

**执行**：

1. 公开访问指定 URL 并记录真实访问结果；只使用公开正常访问、用户提供材料或可信公开存档。
2. 搜索摘要只登记为线索。图片只运行 OCR 并标 `ocr_unreviewed`；不得由 DeepSeek解释图片或把 OCR 升级为 verified。
3. 为 32 个槽位逐行登记名称、分类、判据、排除条件、待确认条件、source ID、定位器和状态。跨分类同概念保留两个槽位和两个 rule ID，只共享 `conceptId`。
4. 若人工提供转写/截图复核，将复核者、日期和材料哈希登记到 `HG-002A`；不得记录账号、cookie 或个人信息。
5. 第二人工复核单独登记为 `HG-002B`，必须是另一名真实复核者；未发生就写 `HUMAN_PENDING`，不得伪造。

**测试/门槛**：台账 validator 通过；机器脚本确认 11/11、21/21 槽位唯一且每行有状态。工具和台账结构完成可在报告中写阶段说明 `ENGINEERING_DONE`；任何未逐字核验行使 `HG-002A` 与本任务 ledger 为 `HUMAN_PENDING`，第二复核缺失使 `HG-002B` 为 `HUMAN_PENDING`。这两个门不阻塞后续预设、自定义和升级的独立工程。

---

## S5-RULE-03：从已核验证据生成版本化规则包

**目标**：只把已核验材料写入新版本规则包，旧 seed 保持不变。

**精确范围**：`packages/rulepack/packs/yy-novel-bar/` 的新版本文件、`packages/rulepack/schemas/rulepack.schema.json`、`packages/rulepack/scripts/validate*.mjs`、`packages/rulepack/README.md`、`packages/rulepack/CHANGELOG.md`、`crates/novel-rulepack/**`。禁止修改旧 `2026.0.0-seed.1.json`。

**执行**：

1. 从来源台账生成新文件；每条含稳定 ID、正整数 version、名称、别名、分类、criteria、exclusions、pendingConditions、确认范围、用户边界、来源映射和核验状态。
2. 已核验不足时只生成明确标记的 RC 或继续保留 seed；未核验条目始终 `defaultEnabled: false`。不能为了凑 11+21 发布伪完整版本。
3. 仅当 32 行都通过 `HG-002A` 且 `HG-002B` 已完成，才允许标 `published`；否则报告写阶段说明 `ENGINEERING_DONE`，task ledger 写 `HUMAN_PENDING`。
4. 更新 README/CHANGELOG，说明来源日期、语境、seed 关系和非客观评价免责声明。

**测试/门槛**：schema、业务 validator、负例和 Rust loader 通过；旧 pack 仍可加载且 SHA 不变；新 pack 的 11+21、ID、version、source ref、verified/defaultEnabled 不变量通过。不得把人工门缺失写成技术失败。

---

## S5-RULE-04：规则不变量与只读概念聚合

**目标**：锁定来源、默认开关、选择和跨分类概念行为，同时保持 Finding 的单规则身份。

**精确范围**：`packages/rulepack/schemas/**`、`packages/rulepack/scripts/**`、负例、`crates/novel-rulepack/**`、`crates/novel-core/src/model.rs`、`crates/novel-core/src/scanner.rs` 和对应测试。不得改 UI、导入器或 provider。

**执行**：

1. 验证稳定 rule ID 唯一、source ref 存在、verified 必须有 verified primary evidence、unverified 不能默认启用、选择只能引用锁定 pack。
2. **不得新增 `matchedRuleIds`，不得把 Finding 改成数组规则身份。** 每条 Finding 仍只有一个 `rule_id`、`rule_version`、category 和 evidence。
3. 跨分类同概念分别物化 finding；新增只读查询/展示 DTO 按 `conceptId` 聚合，成员列表只引用已有 finding ID。聚合策略 `keep_separate`、`highest_user_selected_severity`、`ask_user` 只能影响展示/提示，不改成员 finding。
4. 未知 rule、重复 override、非法 severity/sensitivity、恢复时 pack/version/snapshot 不同必须拒绝。

**测试/门槛**：至少一组跨分类同概念产生两条单规则 finding，展示 DTO 可聚合且展开后两条都存在；序列化兼容测试证明 Finding 没有多规则字段；全套正负测试通过。

---

## S5-PRESET-01A：预设领域模型与三层合并

**目标**：用纯领域代码定义“规则包默认值 → 全局预设 → 当前书覆盖”。

**精确范围**：只允许 `crates/novel-core/src/preset.rs`、`crates/novel-core/src/lib.rs` 及同模块单元测试；若类型确需复用，可最小修改 `crates/novel-core/src/model.rs`。不得改数据库、Tauri 或 UI。

**执行**：定义 preset ID/name、锁定 `packId@packVersion`、pending/duplicate policy、逐规则 enabled/severity/sensitivity/boundary；实现确定性合并、恢复默认、校验和稳定排序纯函数。

**测试/门槛**：表驱动测试覆盖三层优先级、空覆盖、恢复默认、未知 rule、重复 override、非法值和 unverified 强制关闭；无需数据库即可运行。

---

## S5-PRESET-01B：预设与每书覆盖持久化

**目标**：通过只新增迁移和 repository 持久化全局预设、每书选择及覆盖。

**精确范围**：新的 `apps/desktop/src-tauri/migrations/0002_*.sql`（编号按仓库实际顺延）、`apps/desktop/src-tauri/src/repository/presets.rs`、repository 模块注册、`apps/desktop/scripts/validate-migration.mjs` 及 Rust/SQL 测试。禁止修改 `0001_initial.sql`、Tauri 命令和 UI。

**执行**：建立外键、唯一约束、事务 CRUD 和 duplicate/apply-to-book；历史 scan job/selection 不得被级联改写；旧数据库前向迁移后可读。

**测试/门槛**：空库、旧 fixture 升级、CRUD、并发/事务回滚、删除被引用预设、未知 pack/rule 和外键完整性测试通过；迁移不可逆破坏必须失败。

---

## S5-PRESET-01C：扫描选择快照与 Tauri 契约

**目标**：扫描启动时只接收已解析选择并冻结不可变 snapshot。

**精确范围**：`crates/novel-core/src/scanner.rs` 和相关模型/测试、`apps/desktop/src-tauri/src/lib.rs`、可新建 `apps/desktop/src-tauri/src/commands/presets.rs`、对应 command 测试与前端类型契约。不得改 UI 样式。

**执行**：提供 create/list/get/update/delete/duplicate/apply/preview-resolved 命令；启动扫描时保存 pack/version、resolved rule overrides 与策略快照；checkpoint 后修改选择必须拒绝或创建新任务。

**测试/门槛**：command 序列化、非法 ID、并发更新、snapshot immutability、resume mismatch、preview 与实际 snapshot 相同均通过。

---

## S5-PRESET-02：规则选择与预设 UI

**目标**：普通用户不接触 JSON/ID 即可按雷点、郁闷点、严重度选择规则并理解扫描范围。

**精确范围**：`apps/client/src/components/RuleSelector.*`、可新增 `PresetPanel.*`、`apps/client/src/hooks/useAppState.ts`、`apps/client/src/domain.ts`、对应测试。不得改 Rust、迁移或来源台账。

**执行**：分类折叠、搜索、批量开关、单规则详情、预设保存/复制/恢复、每书覆盖、未核验锁定及原因、启动前摘要；显示名称与解释，不暴露内部 rule ID。

**测试/门槛**：DOM 测试覆盖键盘、ARIA、批量选择、锁定未核验项、恢复默认和最终摘要；390/800/1440 宽度用 Playwright 获取 DOM 几何数值并断言无横向溢出。截图只作为自动产物：验收必须依赖 DOM/ARIA/几何断言、自动 pixel diff（如建立基线）或真实人工复核记录；DeepSeek 不得凭截图解释视觉质量。

---

## S5-CUSTOM-01A：自定义规则 schema 与安全解析

**目标**：在不污染社区 pack 的前提下安全解析用户自定义规则。

**精确范围**：`packages/rulepack/schemas/custom-rulepack.schema.json`、`packages/rulepack/scripts/validate-custom.mjs`、正负 fixture、`crates/novel-rulepack/src/custom.rs`、模块注册和测试。不得改数据库、Tauri 或 UI。

**执行**：使用独立 namespace/pack ID；限制文件大小、规则数、字符串长度、递归深度和枚举；拒绝脚本、HTML、路径、URL 拉取、模板执行、未知字段、重复 ID 和冒充官方/社区 verified 状态。

**测试/门槛**：合法最小包 round-trip；超限、原型污染键、路径穿越、外部引用、重复 ID、伪 verified、深嵌套和未知字段全部被稳定错误码拒绝。

---

## S5-CUSTOM-01B：自定义规则持久化与 Tauri commands

**目标**：把已通过 01A 解析的自定义规则隔离、事务化保存，并提供不含 UI 的命令合同。

**精确范围**：新的前向迁移、`apps/desktop/src-tauri/src/repository/custom_rules.rs`、`apps/desktop/src-tauri/src/commands/custom_rules.rs`、模块注册和 Rust/SQL/command 测试。不得修改 React、社区 pack 文件或其 verified 状态。

**执行**：commands 只接收 01A 的规范化 DTO；确认后事务写入；冲突默认拒绝，显式复制生成新 ID；删除被历史 snapshot 引用时保留不可变归档；社区和用户 namespace 在表、查询和返回 DTO 中都不可混淆。

**测试/门槛**：空库/旧库迁移、CRUD、确认 token 失效、冲突、复制、删除/归档、重启恢复、社区规则隔离和事务回滚通过。

---

## S5-CUSTOM-01C：导入预览、导出与 UI 闭环

**目标**：用户先看懂变化与风险，再确认导入；可安全导出自定义内容。

**精确范围**：`apps/client/src/components/CustomRules.*`、`apps/client/src/services/customRules.ts`、相关 hooks/types/tests，以及只为 export DTO 所需的 `apps/desktop/src-tauri/src/commands/custom_rules.rs` 最小追加。不得改迁移、repository、社区 pack 或 parser 限制。

**执行**：导入先调用解析/preview，按名称显示新增、冲突、警告和隔离说明；确认/取消明确；导出只含用户自定义内容、schemaVersion 和必要元数据，不含内部路径/凭据/历史结果；下载失败可恢复。

**测试/门槛**：DOM 测试覆盖 preview、确认、取消、冲突、解析错误、导出、导出 round-trip 和自然语言错误；默认视图不暴露内部 JSON/ID；导出敏感字段扫描通过。

---

## S5-UPGRADE-01A：版本差异与迁移计划纯函数

**目标**：确定性计算旧 pack 到新 pack 的 added/removed/changed/unchanged 与用户选择影响。

**精确范围**：`crates/novel-rulepack/src/upgrade.rs`、模块注册和单元测试。不得改数据库、UI 或 pack 内容。

**执行**：比较 rule ID/version/category/default/source/criteria；输出稳定排序计划；removed/ambiguous/custom conflict 不得自动猜映射；同 ID version 回退必须拒绝。

**测试/门槛**：表驱动测试覆盖新增、删除、改名、语义变化、category 变化、版本回退、冲突和无变化；相同输入字节级稳定输出。

---

## S5-UPGRADE-01B：事务化迁移应用与回滚

**目标**：显式确认后原子应用迁移计划，同时保留旧快照。

**精确范围**：`apps/desktop/src-tauri/src/repository/rule_upgrades.rs`、对应命令、必要的新增迁移和 Rust/SQL 测试。不得改 UI、旧迁移和历史 finding。

**执行**：保存计划、用户决定和目标 pack；事务应用到未来默认/预设，绝不重写已有 scan job、checkpoint、finding 或 selection snapshot；中途错误全回滚。

**测试/门槛**：成功、取消、冲突、模拟中途失败、重试幂等、旧历史查询和外键完整性通过。

---

## S5-UPGRADE-01C：历史复现与升级确认 UI

**目标**：用户能看懂升级影响并选择保留旧版、迁移副本或稍后处理；历史结果始终可复现。

**精确范围**：`apps/client/src/components/RuleUpgrade.*`、相关 services/types/tests，以及只为历史加载所需的 `crates/novel-rulepack` 最小 loader 测试。不得重写 pack、迁移或 scanner 语义。

**执行**：显示名称级差异而非内部 JSON；危险变化单独提示；默认不自动迁移；从历史任务打开结果时按 snapshot 加载旧 pack，缺失时明确报错并提供恢复指导。

**测试/门槛**：DOM 测试覆盖三种决定、取消、冲突、旧版缺失、升级后新任务使用新版且旧任务仍使用旧版；不暴露原始 JSON/内部 ID。

---

## S5-GATE-01：S5 工程门与人工门汇总

**目标**：一次生成可审计工程证据，不把未完成人工核验伪装成失败或通过。

**精确范围**：只允许修复本阶段 P0/P1、更新 `packages/rulepack` 文档/验证脚本和阶段证据文件；不得新增功能。

**执行/门槛**：

1. 运行 rulepack 正负验证、Rust rulepack/core 测试、迁移校验、前端测试/build 和受影响 CI。
2. 证明旧 seed SHA 不变、每条 Finding 单一 rule 身份、未核验默认关闭、历史 snapshot 可复现、自定义规则隔离。
3. 输出 `HG-002A` 和 `HG-002B` 各自的 `RESOLVED` 或 `OPEN`、所需材料、负责人和下一步；不得由 DeepSeek补签。对应 task ledger 只写 `DONE` 或 `HUMAN_PENDING`。
4. 若工程门全绿但人工门待办，报告的阶段说明可写 `ENGINEERING_DONE`，S5-GATE-01 ledger 写 `HUMAN_PENDING`；后续独立工程可继续，正式社区 pack 发布资格保持关闭。
5. 视觉证据只接受 DOM/ARIA/几何/pixel 自动结果或真实人工复核记录，不接受无视觉模型的截图解释。
