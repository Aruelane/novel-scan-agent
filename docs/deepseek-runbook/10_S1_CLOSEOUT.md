# DeepSeek 执行手册 10：S1 收尾

> 适用基线：`43182c7` 及其后的 runbook 文档提交。已知待修复项从 B07-FIX 开始。本文件按顺序执行；不得跳过失败任务，也不得把本机环境阻塞写成“通过”。

## 统一执行纪律

1. 开始前执行 `git status --short` 与 `git merge-base --is-ancestor 43182c7 HEAD`。工作区不干净则停止，列出文件，不覆盖。
2. 使用 `deepseek/full-build`：若当前已经在该分支则继续；若分支不存在，从包含本 runbook 的当前干净提交创建；若分支已存在，先确认工作区干净再切换。禁止重复创建、直接改写或强推 `main`，禁止 `reset --hard`、`checkout --`、`clean -fd`。
3. 一次只做一个任务。完成门槛全部满足后，以任务 ID 开头提交，例如 `S1-01 fix(rulepack): clarify unverified rule error`。只暂存该任务允许文件。
4. 本机 Rust 若因 `link.exe not found` 失败，记录阶段和退出码 101；格式检查仍须通过。阶段检查点推送到草稿 PR，让 `windows-2022` CI 作编译/测试裁决。
5. 不删除或弱化测试，不调用付费模型，不把演示提供器称为真实 AI，不提交小说原文、Key、本地路径、`content://` URI、`target/`、`node_modules/` 或构建产物。
6. 每个任务结束都执行 `git diff --check`、`git diff --name-only`、`git status --short`。出现范围外文件即停止，不提交。

## S1-01（B07-FIX）：修复未核验规则错误消息

- **目标**：让现有 `unverified_rule_must_not_be_default_enabled` 测试通过，不改变拒绝逻辑。
- **前置依赖**：基线包含 `43182c7`；工作区干净。
- **严格允许文件**：`crates/novel-rulepack/src/lib.rs`。
- **禁止范围**：测试、schema、seed、Cargo/CI、验证条件与顺序均不得修改。
- **逐步实现**：只改 `status != "verified" && default_enabled` 分支的消息；消息必须包含小写 `unverified`、`default_enabled`、规则 ID、实际 status。推荐语义：`unverified rule '<ID>' has status '<STATUS>' but default_enabled is true; only verified rules may be default-enabled`。
- **测试命令**：`cargo fmt --all -- --check`；`cargo test -p novel-rulepack --lib --offline`；`git diff --check`。
- **完成门槛**：仅一处文案变化；CI 中该测试通过。
- **失败/阻塞处理**：`link.exe` 阻塞按统一纪律报告；任何其他测试失败不得提交。
- **报告格式**：任务 ID；修改前/后消息；命令/退出码；diff 文件；阻塞；提交 SHA 或“未提交”。

## S1-02（R14）：rulepack loader 拒绝重复规则 ID

- **目标**：在规则包加载边界拒绝 pack 内重复 ID，不能静默去重或依赖 scanner 晚报错。
- **前置依赖**：S1-01 完成。
- **严格允许文件**：`crates/novel-rulepack/src/lib.rs`、`crates/novel-rulepack/tests/seed_pack_test.rs`。
- **禁止范围**：不得修改 seed JSON、schema、scanner 或跨 pack 规则。
- **逐步实现**：`load_from_json` 转换时用集合记录原始 rule ID；第二次出现立即返回 `RulePackError`，消息含 `duplicate rule id` 与具体 ID；不得保留第一条后继续。
- **测试命令**：`cargo fmt --all -- --check`；`cargo test -p novel-rulepack --offline`；`git diff --check`。
- **完成门槛**：复制真实 seed 第一条并追加后加载失败；原 seed 仍加载 32 条；错误关键词断言准确。
- **失败/阻塞处理**：不得通过修改测试或 seed 规避；链接器阻塞如实报告。
- **报告格式**：任务 ID；检查位置；新增测试场景；命令/退出码；diff；提交 SHA。

## S1-03（R15）：LoadedRule 保留运行时元数据

- **目标**：加载后不丢失 `status`、`defaultEnabled`、`profileRef` 与 provenance。
- **前置依赖**：S1-02 完成。
- **严格允许文件**：`crates/novel-rulepack/src/lib.rs`、`crates/novel-rulepack/tests/seed_pack_test.rs`。
- **禁止范围**：本任务不扩展 core `RuleDefinition/RuleContext`，不改 JSON 数据或 schema。
- **逐步实现**：把 `provenance` 接入 `RuleJson`；为 provenance 类型补齐必要的 `Clone/PartialEq/Eq`；`LoadedRule` 保存上述字段；在 `RuleJson` 被消费前安全析构/克隆，避免默认值伪造。
- **测试命令**：同 S1-02。
- **完成门槛**：真实 seed 第一条精确断言 `status == "draft"`、`default_enabled == false`、`profile_ref == "strict-evidence-v1"`，provenance verification/source refs/note 与 JSON 一致；既有 detection 数组测试不退化。
- **失败/阻塞处理**：不得把必填字段缺失静默转成空字符串；链接器问题照实报告。
- **报告格式**：任务 ID；新增保存字段；真实 seed 断言；命令/退出码；diff；提交 SHA。

## S1-04（R16 + R18）：把完整检测语义纳入核心 RuleDefinition

- **目标**：criteria/exclusions/pending conditions/mode/profileRef 不再只停留在旁路 `LoadedRule`。
- **前置依赖**：S1-03 完成。
- **严格允许文件**：`crates/novel-core/src/model.rs`、`crates/novel-core/src/scanner.rs`（仅更新测试 fixture）、`crates/novel-rulepack/src/lib.rs`、`crates/novel-rulepack/tests/seed_pack_test.rs`。
- **禁止范围**：不得改 provider 传播或 fingerprint（留给 S1-05），不得改 seed/schema。
- **逐步实现**：在 core 定义强类型 `DetectionMode { Semantic, ManualOnly }`；`RuleDefinition` 增加 `detection_profile_ref`、`detection_mode`、`criteria`、`exclusions`、`pending_conditions`；rulepack 映射非法 mode、空 profile/criteria/exclusions/pending 为明确错误；更新所有测试 struct literal，优先用一个合成 helper，禁止 `..unsafe default`；检测语义只能有一个权威副本，若保留 LoadedRule 访问器必须与 definition 同源。
- **测试命令**：`cargo fmt --all -- --check`；`cargo test -p novel-core --offline`；`cargo test -p novel-rulepack --offline`；规则包 Node 正/负校验；`git diff --check`。
- **完成门槛**：seed 首条的完整 detection 可从 `definition` 读取；非法 mode 与空数组负例失败；所有调用点编译。
- **失败/阻塞处理**：编译报缺字段时逐个修 fixture，不得删除新字段；环境阻塞走 CI。
- **报告格式**：任务 ID；领域字段；更新调用点；测试/退出码；diff；提交 SHA。

## S1-05（R17）：完整规则语义到达 provider，并进入恢复指纹

- **目标**：`InferenceRequest.rules` 携带完整检测合同；改变判据后旧 checkpoint 不可恢复。
- **前置依赖**：S1-04 完成。
- **严格允许文件**：`crates/novel-core/src/provider.rs`、`crates/novel-core/src/scanner.rs`。
- **禁止范围**：不得修改 rulepack、数据库或 provider 候选算法。
- **逐步实现**：扩展 `RuleContext` 并在 `from_definition` 复制 profile/mode/三组数组；在 `scan_profile_fingerprint` 中以无歧义的长度前缀或稳定序列化纳入这些字段与数组顺序；新增捕获请求的测试 provider，断言完整语义；新增同 ID/version 仅 criteria 改变时 `ResumeMismatch` 测试。
- **测试命令**：`cargo fmt --all -- --check`；`cargo test -p novel-core --offline`；`git diff --check`。
- **完成门槛**：provider 请求断言与恢复拒绝均通过，完全一致身份仍恢复成功。
- **失败/阻塞处理**：不得用 Debug 拼接含分隔符的字符串制造碰撞；链接器阻塞交 CI。
- **报告格式**：任务 ID；传播路径；指纹编码；测试/退出码；diff；提交 SHA。

## S1-06（R19）：安全转换 ImportedDocument → NovelDocument

- **目标**：明确 importer 1-based `usize` 到 core 0-based `u32` 的组合层合同。
- **前置依赖**：S1-05 完成。
- **严格允许文件**：`apps/desktop/src-tauri/src/lib.rs`、`crates/novel-import/src/lib.rs`（只修相关注释）。
- **禁止范围**：不得使用 `as u32`、`index - 1`、panic/unwrap 处理外部文档；不得实现文件选择。
- **逐步实现**：新增私有 checked adapter；先 `checked_sub(1)`，再 `u32::try_from`；格式和 TextRange 行号映射到 core locator；稳定生成 chapter ID、保留标题/文本；错误不得含完整路径/URI。
- **测试命令**：`cargo fmt --all -- --check`；`cargo test -p novel-scout-desktop --lib --offline`；`git diff --check`。
- **完成门槛**：第一章、多章连续、正文前、index=0、64 位平台 u32 溢出均有测试；无窄化 cast。
- **失败/阻塞处理**：32 位无法构造溢出用 `cfg(target_pointer_width)` 明确跳过，不伪造通过。
- **报告格式**：任务 ID；转换规则；边界测试；命令/退出码；diff；提交 SHA。

## S1-07（R20）：桌面生产入口真实加载 seed rulepack

- **目标**：seed 不再只被测试读取，Tauri 生产路径实际依赖 `novel-rulepack`。
- **前置依赖**：S1-04、S1-05 完成。
- **严格允许文件**：`apps/desktop/src-tauri/Cargo.toml`、`Cargo.lock`、`apps/desktop/src-tauri/src/lib.rs`。
- **禁止范围**：不得联网抓贴吧、改 seed、把测试 provider 暴露成模型。
- **逐步实现**：加入 path dependency；用 `include_str!` 嵌入版本化 seed；建立加载函数和只返回安全规则 DTO/摘要的 Tauri command；注册 command；加载失败返回错误而非 panic；DTO 包含版本、规则 ID/version/category/default/status/detection 但不含路径。
- **测试命令**：`cargo fmt --all -- --check`；`cargo test -p novel-scout-desktop --lib --offline`；`cargo metadata --locked --offline --format-version 1 --no-deps`；`git diff --check`。
- **完成门槛**：生产 command 加载 32 条（11/21），首条 detection 完整；Cargo.lock 仅预期 workspace dependency 变化。
- **失败/阻塞处理**：include 路径失败时从 `lib.rs` 所在目录重新计算，不复制 seed；CI 编译裁决。
- **报告格式**：任务 ID；command/嵌入路径；锁文件变化；测试/退出码；diff；提交 SHA。

## S1-08：移除 WebView 原始 sourceRef

- **目标**：Windows 路径和 Android URI 永不进入前端 Book 状态或演示数据。
- **前置依赖**：S1-07 完成。
- **严格允许文件**：`apps/client/src/domain.ts`、`apps/client/src/demo-data.ts`、`apps/client/src/__tests__/demo-data.test.ts`、`apps/client/src/__tests__/nativeBridge.test.ts`。
- **禁止范围**：不得把路径换名后继续返回；不得把 URI 写日志/错误提示。
- **逐步实现**：删除 `Book.sourceRef`；清除 `/demo/...`；增加递归序列化断言，Book/native 安全 DTO 不含 `sourceRef`、盘符路径、反斜杠路径、`file://`、`content://`。
- **测试命令**：在 `apps/client` 运行 `npm test`、`npm run build`；`git diff --check`。
- **完成门槛**：全仓前端源码无 `sourceRef` 和演示绝对路径。
- **失败/阻塞处理**：只修类型调用点，不新增 opaque path 字段。
- **报告格式**：任务 ID；删除字段；安全断言；命令/退出码/测试数；diff；提交 SHA。

## S1-09：失败状态与预算单位一致

- **目标**：CSS 与领域枚举使用 `failed`；模型 token 窗口与核心字符预算不混称。
- **前置依赖**：S1-08 完成。
- **严格允许文件**：`apps/client/src/domain.ts`、`apps/client/src/demo-data.ts`、`apps/client/src/components/Sidebar.css`、`apps/client/src/components/ScanProgress.tsx`、`apps/client/src/components/ScanProgress.css`、`apps/client/src/components/SettingsPanel.tsx`、聚焦的 `Sidebar.test.tsx`/`ScanProgress.test.tsx`/设置测试。
- **禁止范围**：不得把 `context_budget_chars` 标成 token，不新增可用 API 配置假象。
- **逐步实现**：`.status-dot--error` 改为 `--failed` 并渲染失败 fixture 测试；`ScanProgress` 对全部 `ScanStatus` 使用穷尽映射，`failed` 必须显示“失败”和 danger badge，不能落入“准备中”；为 failed 补 CSS；把设置拆为明确的 `contextWindowTokens`（provider 计划值）与 `contextBudgetChars`（扫描核心值）或只展示实际已有者；界面单位与变量名一致。
- **测试命令**：`npm test`、`npm run build`、`git diff --check`。
- **完成门槛**：Sidebar 与 ScanProgress 的失败状态都有正确中文、class 和可见语义色，五种状态映射有测试；任何 128000 token 值不再冒充字符预算。
- **失败/阻塞处理**：不要为通过测试删掉设置项；无法接后端时继续标“演示”。
- **报告格式**：任务 ID；状态/单位变化；测试数；diff；提交 SHA。

## S1-10：规则开关焦点与严重度键盘合同

- **目标**：键盘用户可看见开关焦点并完整操作 radiogroup。
- **前置依赖**：S1-09 完成。
- **严格允许文件**：`apps/client/src/components/RuleSelector.tsx`、`RuleSelector.css`、新建 `apps/client/src/__tests__/RuleSelector.test.tsx`。
- **禁止范围**：不得靠 mouse-only handler，不改变规则业务语义。
- **逐步实现**：焦点环绑定 `.rule-item__checkbox:focus-visible + .rule-item__switch`；severity 使用 roving tabindex；左右/上下键首尾循环，Home/End，焦点与选中同步；禁用规则时仍允许用户重新启用，但严重度控件不得误触。
- **测试命令**：`npm test -- RuleSelector`、完整 `npm test`、`npm run build`。
- **完成门槛**：Testing Library 用真实键盘事件覆盖循环/Home/End/aria-checked/tabIndex；CSS 选择器正确。
- **失败/阻塞处理**：不得只断言 helper，必须渲染组件。
- **报告格式**：任务 ID；键盘矩阵；命令/测试数；diff；提交 SHA。

## S1-11：Workspace tabs 无障碍合同

- **目标**：tabs/tabpanel 关联完整，键盘移动首尾循环。
- **前置依赖**：S1-10 完成。
- **严格允许文件**：`apps/client/src/components/Workspace.tsx`、`Workspace.css`、新建 `apps/client/src/__tests__/Workspace.test.tsx`。
- **禁止范围**：不改变四个 tab 功能，不删除 hidden panel 的语义关联。
- **逐步实现**：清理未使用变量；ArrowLeft/Right 首尾循环，Home/End；roving tabindex 唯一 0；激活后焦点和 `aria-selected` 同步；`aria-controls`/`aria-labelledby` ID 唯一匹配。
- **测试命令**：`npm test -- Workspace`、完整 test/build。
- **完成门槛**：渲染测试覆盖 click、循环、Home/End、tabpanel hidden；TypeScript 无未使用项。
- **失败/阻塞处理**：不得用 clamp 代替循环，不用 DOM 查询绕过 React 状态。
- **报告格式**：任务 ID；ARIA/键盘合同；测试数；diff；提交 SHA。

## S1-12：颜色对比度自动门禁

- **目标**：普通文字与活跃按钮达到 WCAG AA，不再只在 README 留欠账。
- **前置依赖**：S1-11 完成。
- **严格允许文件**：`apps/client/src/index.css`、相关组件 CSS、`apps/client/src/__tests__/accessibilityColors.test.ts`、`apps/client/README.md`（仅删除已解决欠账）。
- **禁止范围**：不得用更大字号或 opacity 规避；不得只目测。
- **逐步实现**：实现测试内 sRGB 相对亮度/contrast ratio；普通文字背景组合至少 4.5:1，非文字焦点/控件至少 3:1；重点覆盖 muted、warning、info、primary、severity 1–5 活跃态文字。
- **测试命令**：`npm test -- accessibilityColors`、完整 test/build。
- **完成门槛**：每个语义组合显式列入测试并通过；焦点仍清晰。
- **失败/阻塞处理**：调整 foreground/background token，禁止降低门槛。
- **报告格式**：任务 ID；颜色对与最低比值；命令/测试数；diff；提交 SHA。

## S1-13：诚实的导入占位与 favicon

- **目标**：原生导入未实现时不显示可点击/可拖拽假交互；浏览器无 favicon 404。
- **前置依赖**：S1-12 完成。
- **严格允许文件**：`apps/client/src/components/ImportPanel.tsx`、`ImportPanel.css`、`apps/client/src/__tests__/ImportPanel.test.tsx`、`apps/client/index.html`、新建 `apps/client/public/favicon.svg`。
- **禁止范围**：不得在本任务实现文件读取，不添加 drop handler 或伪按钮。
- **逐步实现**：改成明确非交互说明“导入功能将在 S2 接入”；移除“拖拽/从设备选择”动作措辞及事件吞掉逻辑；HTML 引用无脚本 SVG favicon。
- **测试命令**：组件测试、完整 test/build。
- **完成门槛**：占位没有 button/file input/drop handler；提示与能力表仍可读；构建含 favicon。
- **失败/阻塞处理**：不要用 `aria-disabled` 包装一个看似可点控件冒充修复。
- **报告格式**：任务 ID；交互删除项；测试数；diff；提交 SHA。

## S1-14：390/800/1440 响应式自动验证

- **目标**：无需图片模型，真实浏览器证明无水平溢出且布局模式正确。
- **前置依赖**：S1-13 完成。
- **严格允许文件**：`apps/client/package.json`、`apps/client/package-lock.json`、新建 `apps/client/playwright.config.ts`、新建 `apps/client/e2e/responsive.spec.ts`、`apps/client/src/App.css`、必要的组件 CSS、`.github/workflows/ci.yml`。
- **禁止范围**：不得靠截图人工结论，不大改视觉设计；不测试 Android 原生软键盘（留 S6）。
- **逐步实现**：加入 Playwright Chromium 测试；视口 390×900、800×900、1440×900；断言 `scrollWidth <= clientWidth`；390/800 仅一个 `.mobile-visible` 主面板并显示底栏；1440 显示三栏、隐藏底栏、工作区不低于可用最小宽；CI 安装 Chromium 并运行 e2e。
- **测试命令**：`npm ci`、`npx playwright install chromium`、`npm run test:e2e`、`npm test`、`npm run build`。
- **完成门槛**：三视口全过，CI 不使用 `continue-on-error`。
- **失败/阻塞处理**：浏览器下载失败记录网络错误并保留单元测试，不宣称响应式已验收；CI 必须最终验证。
- **报告格式**：任务 ID；视口/断言；命令/测试数；依赖锁变化；diff；提交 SHA。

## S1-15：文档对齐、Tauri 编译与 S1 总门禁

- **目标**：代码、能力表、文档与 CI 同步，S1 所有 job 全绿。
- **前置依赖**：S1-01 至 S1-14 全部完成。
- **严格允许文件**：`README.md`、`apps/client/README.md`、`docs/PROJECT_PLAN.md`、`docs/ROADMAP.md`、`docs/ARCHITECTURE.md`、`.github/workflows/ci.yml`。
- **禁止范围**：原则上不改业务代码；不得宣称真实文件选择、真实模型、Android SAF/Keystore、OCR、旧 DOC 或发布 APK 已完成。
- **逐步实现**：核对 Ready/Pending/Unsupported；修正 rulepack 依赖、演示边界、R14–R20 接线说明；ROADMAP 只勾真正通过项；Windows CI 增加 `tauri build --debug --no-bundle`（若已存在不重复）；保留 fmt/check/test、rulepack 正负、migration/capability、前端 unit/build/e2e。
- **测试命令**：`cargo fmt --all -- --check`；`cargo check --workspace --all-targets --offline`；`cargo test --workspace --all-targets --offline`；四项 Node validator；client `npm test`/build/e2e；`git diff --check`。
- **完成门槛**：推送 `deepseek/full-build`，创建/更新 draft PR；`gh pr checks --watch` 所有 S1 job 成功；工作区干净。Windows 壳启动冒烟仍明确留 S6。
- **失败/阻塞处理**：任一红灯即“S1 未通过”；读取失败 log 修最小回归并重跑，不通过不得进入 S2。
- **报告格式**：`S1 最终报告`；提交范围；本地命令/退出码/测试数；PR 与 CI URL；通过/失败/环境阻塞；明确的未实现项。

S1 全绿后继续执行 `20_S2_MULTI_FORMAT_IMPORT.md`，不要自行进入 S3。
