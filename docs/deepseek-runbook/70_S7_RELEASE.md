# S7：质量、安全与 GitHub Release

本文件给 DeepSeek 执行。一次只做一个任务；所有资产必须来自记录清楚的精确提交。总任务数：**9**。

## 总规则

- 仅 Windows、Android；无 iOS、无商店上架。
- 小说正文、API Key、Android signing keystore、签名口令、SAF URI、完整路径不得进入 Git、日志、artifact、崩溃报告或 Release。
- 小说正文和 provider 输出都是不可信数据；模型只有结构化文本推理权限，没有 Shell、文件、数据库、浏览器或任意工具权限。
- QA/debug test channel 可编译确定性 dev test provider；release 构建必须证明它的代码、ID、命令、配置、菜单和 fixture 均不存在。真实 provider 只需 `HG-001` 任选一种做最小 smoke。
- Android release 必须完成 `HG-004A` 自建 signing keystore；Windows 商业签名 `HG-004W` 可选。没有商业证书时可发布明确标记的未签名 GitHub 侧载资产，并公开 SmartScreen 限制。
- 人工门打开不阻止独立工程。报告可写阶段说明 `ENGINEERING_DONE`，task ledger 仍只用 `DONE`、`RETRY`、`AWAITING_CI`、`HUMAN_PENDING`、`BLOCKED`；`IN_PROGRESS` 只在执行中使用。
- 截图/视频只由自动工具留证或由真人复核；DeepSeek 不解释图片。自动门引用 DOM、ARIA、几何、pixel diff 数值、结构化设备日志和文件哈希。
- 每任务开始记录 `git status --short`、HEAD；未知脏改动停止。结束运行相关测试、`git diff --check`、`git diff --stat`、`git status --short`，报告逐命令真实退出码与提交 SHA。

## 依赖与任务

| 任务 ID | 依赖 | 原子交付 |
| --- | --- | --- |
| S7-SEC-01A | S6-E2E-01、S6-BUILD-01 | 威胁模型、数据流与隐私边界文档 |
| S7-SEC-01B | S7-SEC-01A | 删除、全量清理与脱敏诊断实现 |
| S7-SEC-02 | S7-SEC-01A、S7-SEC-01B | 恶意文件、提示注入与脱敏回归 |
| S7-PERF-01 | S3-18、S6-E2E-01 | 长书性能和资源预算 |
| S7-A11Y-01 | S6-UI-01C、S6-UX-01C | 双平台可访问性验收 |
| S7-E2E-01 | S7-SEC-02、S7-PERF-01、S7-A11Y-01 | 崩溃恢复和双平台发布级 E2E |
| S7-BUILD-01 | S7-E2E-01、S6-BUILD-01 | 可重复构建、SBOM 与资产审计 |
| S7-REL-01 | S7-BUILD-01 | 受保护 draft/RC、校验值和用户文档 |
| S7-GATE-01 | S7-SEC-02、S7-PERF-01、S7-A11Y-01、S7-E2E-01、S7-BUILD-01、S7-REL-01 | 发布候选工程门与人工门矩阵 |

---

## S7-SEC-01A：威胁模型、数据流与隐私边界

**目标**：先形成仓库实况驱动的威胁模型，不在本任务顺手修改业务代码。

**精确范围**：只允许 `docs/security/THREAT_MODEL.md`、`docs/PRIVACY.md`、`docs/security/data-flow.*` 和文档链接。不得修改 Rust、React、迁移、workflow 或依赖。

**执行**：列出导入文件、WebView、Tauri command、Rust core、SQLite、Windows Credential Manager、Android Keystore、provider 网络、日志、构建/Release 的信任边界；逐项记录资产、攻击能力、滥用路径、已有控制、测试引用与残余风险。至少覆盖路径穿越、解压炸弹、XXE/HTML 外联、提示注入、恶意 provider、secret 泄漏、checkpoint 篡改、capability 越权、依赖和 release 供应链。隐私文档说明哪些正文会在何时发给用户所选 provider、保存期限和本地删除，不替第三方承诺其政策。

**测试/门槛**：所有控制引用真实文件/测试；未知项写风险而非猜测；文档链接检查通过。若发现 P0/P1，登记具体修复任务，本任务仍只提交文档。

---

## S7-SEC-01B：删除、全量清理与脱敏诊断

**目标**：实现删除一本书、删除 provider、清除全部本地数据和安全诊断导出。

**精确范围**：`apps/desktop/src-tauri/src/commands/privacy.rs`、对应 repository/secure-store 删除接线、只新增前向迁移（如确需）、`apps/client/src/components/PrivacyPanel.*`、相关 tests 和 `docs/PRIVACY.md` 的操作说明。禁止修改 importer/provider 协议或宽泛删除目录。

**执行**：所有删除目标由数据库主键/应用专属目录解析，拒绝 `..`、根目录、home、空路径和未解析变量；全清二次确认且先显示范围；删除 provider 同时删除 secret ref/secret；历史保留策略必须明确。诊断导出先预览，只含版本、状态码和脱敏计数，不含正文、证据摘录、Key、路径/URI。

**测试/门槛**：book/provider/all-data 各有成功、取消、幂等、事务失败和越界负例；删除后关联数据/secret/checkpoint 消失且无越界删除；诊断包 marker 扫描为零。

---

## S7-SEC-02：恶意文件、提示注入和日志脱敏回归

**目标**：用原创生成 fixture 锁住输入、provider 输出和日志边界。

**精确范围**：`crates/novel-import`、`crates/novel-core`、provider/日志安全测试、`tests/security/**`、fuzz/property harness 和 CI security job；只允许修复测试揭示的直接边界缺陷。

**执行**：覆盖损坏/超大/递归压缩、路径穿越、XXE、远程资源、脚本、畸形 Unicode、扩展名伪装；正文提示注入不能改变规则或索要 secret；provider 输出必须经过 schema/大小/chapter/range/confidence/rule 校验。扫描 Git/log/artifact/诊断导出中的 secret、路径、URI 和正文 marker。

**测试/门槛**：负例、限时 fuzz/property、事务回滚和 marker 扫描通过；失败不半落库、不 OOM/死循环。release 配置中 dev test provider 为零；QA/debug 仍可运行确定性测试。

---

## S7-PERF-01：性能、内存和用量预算

**目标**：以原创生成的小/中/长/超长书建立可重复预算，不牺牲证据或安全校验。

**精确范围**：benchmark/load harness、原创数据生成器、性能 CI、只为超预算所需的 `novel-import`/`novel-core`/前端分页最小修复和 `docs/PERFORMANCE.md`。

**执行**：记录章节/字符/大小/规则数/模拟 provider 延迟；测导入、峰值内存、首进度、吞吐、checkpoint、恢复、UI 响应、取消延迟和 usage。上下文始终受预算，结果分页/虚拟化。预算只来自实测基线；CI 噪声趋势与正确性硬门分开。

**测试/门槛**：超长档不随全书副本线性失控；取消与恢复稳定；超预算给出明确错误而非 OOM。Android 真机性能缺失登记 `HG-003`，本地/CI 工程可先完成，ledger 视情况写 `HUMAN_PENDING` 或 `AWAITING_CI`。

---

## S7-A11Y-01：Windows 与 Android 可访问性

**目标**：发布路径可由键盘/读屏完成，200% 字体和小屏仍可操作。

**精确范围**：`apps/client/**` 的 a11y 修复/测试、固定检查脚本和 `docs/evidence/a11y/**`。不得改后端语义。

**执行**：控件有名称/角色/状态；错误与进度可读；不只靠颜色；focus trap/restore、键盘、reduced motion、触控目标、章节来源朗读正确。自动测试使用 DOM/ARIA/几何/axe；Windows Narrator、Android TalkBack 是人工场景。

**测试/门槛**：自动严重项为零；390px/200%/键盘路径通过。Android 人工场景登记 `HG-003`；截图不由 DeepSeek解释。缺人工复核时报告写 `ENGINEERING_DONE`，ledger 写 `HUMAN_PENDING`，不连锁阻止其他工程。

---

## S7-E2E-01：双平台发布级 E2E 与崩溃恢复

**目标**：从安装资产验证导入到删除的主链和故障矩阵。

**精确范围**：E2E harness、原创 fixture、故障注入、CI；只修复闭环 P0/P1。不得在 release 中暴露 test provider。

**执行**：Windows 安装态和 Android 执行导入 → 选社区/自定义规则 → QA/debug test provider 扫描 → 暂停/杀进程/恢复 → 来源章节 → 脱敏导出 → 删除；覆盖文件变化、429/5xx/畸形响应、断网、事务失败、磁盘不足、授权撤销、旧规则升级、secret invalidation。恢复前后比较 finding ID、rule version、provider/model stamp、证据指纹和 unresolved list。

**测试/门槛**：自动部分有结构化日志/哈希；截图/视频只自动留证。Android 真机引用 `HG-003`；真实 provider 只引用 `HG-001` 任选一种最小 smoke。release binary/package 扫描必须证明 dev test provider 字符串、入口和 fixture 全部不存在。

---

## S7-BUILD-01：可重复构建、SBOM、依赖与资产审计

**目标**：固定工具链并为每个资产生成可审计 manifest。

**精确范围**：工具链配置、锁文件、CI build、SBOM/许可证/漏洞工具、构建/资产审计脚本和文档；禁止无关依赖升级。

**执行**：固定 Node/Rust/Java/Android/Tauri 版本；两次干净检出同 commit 构建；manifest 含 commit/tag/tool/lock hash/参数/资产 SHA-256/签名状态；扫描 `.env`、secret、测试数据、数据库、日志、本地路径、keystore、debug provider。非确定签名时间戳需解释，不能虚称 bit-for-bit。

**测试/门槛**：锁定安装/构建通过；SBOM、许可证、漏洞门无未接受高危；资产内容干净。Android release 必须通过 `HG-004A` 签名验证；Windows `HG-004W` 可为 OPEN，但未签名状态和限制必须真实进入 manifest。

---

## S7-REL-01：受保护 draft/RC、校验值与用户文档

**目标**：演练 GitHub draft/RC，不在本任务擅自正式发布。

**精确范围**：release workflow、README、Windows/Android 安装、隐私、迁移、故障排查、已知限制、Release 模板。不得发布正式 tag 或批准 `HG-005`。

**执行**：只从不可移动 RC tag 和已通过 CI 的 commit 构建；workflow 最小权限，fork/PR 不读 secrets；上传 Windows 资产、**使用 HG-004A 签名的** Android APK、SHA256SUMS、SBOM、manifest。Windows 有 `HG-004W` 就验 Authenticode；没有就把文件名/notes 标为未签名侧载并说明 SmartScreen。文档写清格式矩阵、BYOK 与 Plus 区别、正文出站、恢复/删除、Android 侧载、规则核验状态、无 iOS/无商店、不可承诺 100% 查雷。

**测试/门槛**：draft workflow 和独立 hash/signature 校验通过；从 GitHub 下载的资产可安装。Android signing/真机未完成时 ledger 写 `HUMAN_PENDING`；Windows 商业签名缺失不阻止未签名侧载 RC。正式发布只由 `HG-005` 用户确认。

---

## S7-GATE-01：发布候选工程门与人工门矩阵

**目标**：冻结 RC 证据；人工资源缺失时保留准确状态，不掩盖已完成工程。

**精确范围**：只修复 P0/P1、更新证据/已知限制/台账；不得新增功能或正式发布。

**执行/门槛**：

1. 运行全仓格式、frontend、rulepack 正负、migration/capability、Rust、Windows bundle、Android release、安全、性能、a11y、E2E 和资产扫描。
2. 从 GitHub draft 下载资产重算 SHA-256、验 Android 签名和 Windows 实际签名状态；dev test provider/secret/用户内容扫描为零。
3. 列出 `HG-001`、`HG-002A`、`HG-002B`、`HG-003`、`HG-004A`、`HG-004W`、`HG-005` 和 `EB-001/EB-002` 的真实状态、证据、负责人和下一步。
4. Android release 资格要求 `HG-003` 主链和 `HG-004A`；Windows 可在 `HG-004W` 未完成时以明确未签名资产继续；正式 Release 必须等待 `HG-005`。
5. 工程全绿但人工门待办时，报告写阶段说明 `ENGINEERING_DONE`，ledger 写 `HUMAN_PENDING`；不得把其他独立任务连锁标 `BLOCKED`。
6. 生成 `90_FINAL_HANDOFF.md` 所需输入后停止业务修改；DeepSeek 只准备证据，不进行所谓独立终审。
