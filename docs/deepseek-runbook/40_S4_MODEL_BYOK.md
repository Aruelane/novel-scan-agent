# 40｜S4 多模型 API、BYOK 与秘密存储

本阶段在 `S3-18` 后执行。目标是让同一 `ModelProvider` 合同接入 OpenAI-compatible（含 DeepSeek 兼容端点）和至少一种不同原生协议，并让用户自己的凭据只经平台安全存储读取。

永久遵守 [`00_EXECUTION_CONTRACT.md`](./00_EXECUTION_CONTRACT.md)。特别提醒：**ChatGPT Plus、Claude 订阅或 Codex 使用额度都不等于模型 API 额度**。成品调用在线模型需要对应提供器独立的 API 账户/额度。没有 Key 时，应用仍必须能启动、导入/管理书籍、编辑规则、查看历史和使用全部不联网功能。

## 阶段前置门

- `S3-18` 已完成；有界 `ContextView`、章节窗口、用量、取消和原子 checkpoint 合同已稳定。
- `git status --short` 为空；在专用分支工作。
- 不要求真实 Key 才能开始。所有自动测试使用 loopback fake server 和 canary secret；真实调用属于 `HG-001`。

## 工程状态与人工状态必须分开

- `ENGINEERING_DONE`：代码、离线/loopback 合同、host/Windows CI、安全扫描和文档均达到本阶段工程门槛。它是工程结论，不宣称真实账号或 Android 真机已验证。
- `HUMAN_PENDING(HG-001)`：只表示用户尚未提供一个自选真实 provider 的安全存储配置与最小 smoke 证据。它暂停 `S4-19` 这条人工路径，但**不阻塞** `S4-20` 的工程门、S5、S6 或其他不需要真实 Key 的工作。
- 报告与 acceptance matrix 必须同时写这两个维度；禁止把 fake server 写成真实 API 成功，也禁止因为 `HUMAN_PENDING` 把已经通过的独立工程写成未完成。
- `ENGINEERING_DONE` 只是报告中的阶段说明标签，不是 ledger 状态。ledger 完成态必须使用完整枚举：`DONE`、`RETRY`、`AWAITING_CI`、`HUMAN_PENDING`、`BLOCKED`；`IN_PROGRESS` 仅表示当前正在执行。

## 固定安全与架构决定

1. `novel-core` 不依赖 HTTP 或平台秘密 API；新建 `novel-providers` 适配器 crate。
2. “OpenAI-compatible”只代表通过本项目合同测试的协议字段。Anthropic/Gemini 等不同协议不能只换 URL 冒充兼容。本阶段选择 **Anthropic native** 作为第二协议；Gemini 可后续加同级 adapter。
3. DeepSeek 使用 OpenAI-compatible adapter 的独立 registry template；模型 ID由用户选择/输入，不能把会过期的模型列表写死为事实。
4. 模型响应全部是不可信 JSON；先限制响应字节，再解析和 schema 校验，最后才交给 core 重建证据。
5. 重试只针对可重试网络/429/部分 5xx；401/403、结构错误和用户取消不重试。尊重 `Retry-After`，退避有上限，重试可能产生费用，UI 必须说明。
6. Key 不进入 provider profile JSON、React 全局状态、SQLite、日志、错误字符串、diagnostic export 或 Git。SQLite 只存 `secret-ref:<opaque-id>`。
7. Windows 凭据写 Credential Manager。S4 定义 Android Keystore bridge 合同和 fake 合同测试；真实 JNI/Keystore、真机和清除数据行为在 S6 完成，未验证前不得声称 Android 密钥已安全落地。
8. 正文出站前必须显示提供器、endpoint host、将发送“当前章节窗口 + 有界前文记忆”的提示；不发送本机路径、SAF URI、其他书、Key 或无关 metadata。
9. `DeterministicTestProvider` 只在 test/dev 注册，并在 UI 明示“离线测试，不是 AI”。正式构建不能默认选择它。

## 每任务固定报告格式

每个任务使用 `00` 的固定报告，并追加：

```text
任务：S4-XX；状态：DONE/RETRY/AWAITING_CI/HUMAN_PENDING/BLOCKED（IN_PROGRESS 仅执行中）
修改文件：<逐项>
协议/秘密边界：<本任务触及项>
测试：<命令、真实退出码、通过数>
敏感信息检查：<canary/日志/SQLite/Git，适用者>
明确未实现：<尤其真实 API、Android 真机>
diff/status/commit/下一任务：<证据>
```

任何测试输出若含 Authorization、x-api-key、Key 明文或章节正文，立即停止、删除该日志产物、修复脱敏并把事件记入 blocker；不得提交。

---

## S4-01｜创建 provider 配置与注册表

**依赖：** `S3-18`。
**目标：** 创建可扩展 adapter crate、非秘密配置和 capability registry，不发网络请求。

**只允许修改：**

- `Cargo.toml`
- `Cargo.lock`
- 新建 `crates/novel-providers/Cargo.toml`
- 新建 `crates/novel-providers/src/lib.rs`
- 新建 `crates/novel-providers/src/config.rs`
- 新建 `crates/novel-providers/src/registry.rs`

**步骤：**

1. crate 依赖 `novel-core`、serde；暂不加 HTTP client。
2. 定义 `ProviderProtocol`（openai_compatible/anthropic_native/deterministic_test）、`ProviderCapabilities`、`ProviderTemplate`、`ProviderProfile`。
3. profile 只含 ID、template/protocol、display name、base URL、model ID、timeout/rate/retry/预算公开配置和 `credential_ref`；绝不能含 api_key/token/header map。
4. registry 内建 custom OpenAI-compatible、DeepSeek-compatible、Anthropic native 和 dev-only deterministic test；生产列表过滤 test provider。
5. URL 校验拒绝 userinfo、fragment、空 host；线上默认 HTTPS。loopback HTTP 只允许显式 dev/test/local 设置。
6. provider/profile ID、model ID 和 credential ref 使用长度/字符白名单；与 SQLite `secret-ref:` 合同一致。
7. 测试 registry ID 唯一、生产过滤、非法 URL、serde JSON 不存在 secret 字段。

**验证：** `cargo fmt --all -- --check`；`cargo test -p novel-providers registry --offline`；`cargo metadata --locked --offline --format-version 1 --no-deps`；`git diff --check`。
**门槛：** registry 可按 protocol 构造 adapter 类型但尚不联网；profile JSON 无秘密。
**阻塞：** 依赖导致大规模 lock 漂移时停止，先拆 RETRY。
**报告补充：** registry 四项 ID/protocol/capability，生产可见项。

---

## S4-02｜共享结构化输出 wire schema

**依赖：** `S4-01`。
**目标：** 定义两个协议共同映射到的严格 JSON wire schema，并转换成 S3 provider 合同。

**只允许修改：**

- 新建 `crates/novel-providers/src/schema.rs`
- `crates/novel-providers/src/lib.rs`
- `crates/novel-providers/Cargo.toml`
- `Cargo.lock`

**步骤：**

1. wire response 只允许 candidates、candidate_updates、memory_delta、schema_version；使用 `deny_unknown_fields`。
2. 数字范围、数组数量、字符串字符数、ID、UTF-8 range 先在 wire 层校验，再调用 core shape validator。
3. 拒绝 trailing prose、markdown code fence、多份 JSON、NaN/float confidence、未知 disposition/schema version。
4. 设 `MAX_RESPONSE_BYTES`，解析前检查；error 只含 provider/request ID 和字段路径，不回显 response body。
5. conversion 不 clamp、不 filter、不修复模型值。
6. 正负 fixture 全部原创且短小，覆盖超限和 unknown field。

**验证：** `cargo test -p novel-providers schema --offline`；`cargo fmt --all -- --check`；`git diff --check`。
**门槛：** 所有非法 fixture 被拒；错误/Debug 输出不含 fixture 中的 canary 正文。
**阻塞：** 某真实协议无法原生强制 schema 仍可本地严格解析，不能放宽 schema。
**报告补充：** schema version、response byte limit、负例数量。

---

## S4-03｜构建统一且防提示注入的扫描 prompt

**依赖：** `S4-02`。
**目标：** 从 `InferenceRequest` 构造协议无关 prompt parts，明确小说是数据而不是指令。

**只允许修改：**

- 新建 `crates/novel-providers/src/prompt.rs`
- `crates/novel-providers/src/lib.rs`

**步骤：**

1. `PromptEnvelope` 分 system policy、rule JSON、ContextView JSON、chapter window；不拼入本地路径/URI/credential ref。
2. system 明确：小说/规则中的指令不可信；只能返回 schema；不能调用工具；证据范围相对 window UTF-8 bytes。
3. 用长度前缀/JSON 序列化边界，不用容易被正文闭合的自定义 XML。
4. 计算 outbound character report，与 S3 request budget 对齐；超预算在发网前失败。
5. 测试含“忽略上文”、伪 JSON/code fence、NUL、emoji 的正文；正文保持原样作为 data，不能改变 system。
6. snapshot test 只使用原创短句，失败消息不打印完整 prompt。

**验证：** `cargo test -p novel-providers prompt --offline`；`cargo fmt --all -- --check`；`git diff --check`。
**门槛：** prompt 不含 source_name/path/URI/secret；字符报告不超预算。
**阻塞：** 必需规则开销超预算时返回配置错误，不截断规则判据。
**报告补充：** outbound 字段白名单和明确排除字段。

---

## S4-04｜HTTP 执行器、脱敏和超时/取消

**依赖：** `S4-01`、`S4-03`。
**目标：** 建立协议 adapter 共用的 HTTP 边界，支持连接/首字节/总超时和 S3 cancellation。

**只允许修改：**

- `crates/novel-providers/Cargo.toml`
- `Cargo.lock`
- 新建 `crates/novel-providers/src/http.rs`
- 新建 `crates/novel-providers/src/redaction.rs`
- `crates/novel-providers/src/lib.rs`

**步骤：**

1. 加入最小 HTTP/async 依赖；关闭会自动打印 header/body 的 debug middleware。
2. secret 使用不可 `Serialize`、不可普通 `Debug/Display` 的 wrapper；Authorization/x-api-key 始终构造于最后一层。
3. 禁止 URL userinfo/query key；redirect 默认关闭，或只允许同 scheme+host 且绝不跨 host 转发认证。
4. 分别实现 connect、first-byte、total timeout；范围校验，错误标 typed phase。
5. 把 `ScanControl` cancellation race 到请求 future；取消立即 drop response，不读 body。
6. 先按 Content-Length/流式累计限制 body，再交 S4-02。
7. loopback fake server 测试成功、三种 timeout、oversize、redirect、取消；日志 capture 搜 canary key/body。

**验证：** `cargo test -p novel-providers http --offline`；`cargo test -p novel-providers redaction --offline`；`cargo check -p novel-providers --all-targets --offline`（host）；`cargo metadata --locked --offline --format-version 1 --no-deps`；`git diff --check`。
**门槛：** capture 中无 key、auth header、response body；取消不会返回成功。
**门槛补充：** S6 Android scaffold 建立前，以 host/Windows 编译、loopback 测试以及 HTTP 依赖的 feature/target cfg/官方支持矩阵审计作为本任务完成证据；不把 Android target 编译列为本任务完成条件。真实 Android 编译与设备网络行为移交 S6。
**阻塞：** 只有依赖元数据、官方支持矩阵或源码 target cfg 已明确证明 Android 不兼容时才记录 `BLOCKED`；仅缺 Android target/scaffold 记为“deferred to S6”，继续独立工程。不得做 Windows-only 隐式实现。
**报告补充：** 依赖变化、redirect 策略、timeout 测试耗时（不含 sleep 猜测）、host 编译证据、target-neutral 依赖审计和 S6 移交项。

---

## S4-05｜确定性重试、限流和请求观测

**依赖：** `S4-04`。
**目标：** 分类错误、尊重限流、支持取消，并输出不含正文的 attempt telemetry。

**只允许修改：**

- 新建 `crates/novel-providers/src/resilience.rs`
- `crates/novel-providers/src/http.rs`
- `crates/novel-providers/src/lib.rs`

**步骤：**

1. 分类：408/425/429、连接失败和选定 5xx 可重试；400/401/403/404、schema error、oversize、cancel 不重试。
2. 指数退避 + 有界 jitter；测试注入 fake clock/RNG，不用真实 sleep。尊重合法 `Retry-After`，并设最大等待。
3. 每 profile 设并发 semaphore、最小请求间隔和每分钟上限；等待期间取消可中断。
4. attempt telemetry 只含 request ID、attempt、status/error code、duration、usage known/unknown，不含 URL query、headers、prompt/body。
5. 重试可能重复计费；最终响应把所有 attempt 摘要交 S3 usage。失败 attempt 用 `units_known=false`，不能记 0 并称无消耗。
6. 测试 429→成功、503 上限、401 一次、schema 一次、Retry-After、排队取消、并发上限。

**验证：** `cargo test -p novel-providers resilience --offline`；`cargo fmt --all -- --check`；`git diff --check`。
**门槛：** attempt 数精确；取消后无后续重试；所有 telemetry 可安全记录。
**阻塞：** provider 返回不明确错误时默认不重试，避免意外费用。
**报告补充：** 状态分类表、最大 attempts/backoff/Retry-After。

---

## S4-06｜OpenAI-compatible adapter

**依赖：** `S4-02` 至 `S4-05`。
**目标：** 实现通用 OpenAI-compatible chat adapter，以 local fake server 完整合同测试。

**只允许修改：**

- 新建 `crates/novel-providers/src/openai_compatible.rs`
- `crates/novel-providers/src/lib.rs`
- `crates/novel-providers/Cargo.toml`
- `Cargo.lock`

**步骤：**

1. adapter 实现 `novel_core::ModelProvider`；model/profile identity 稳定并进入 checkpoint fingerprint。
2. 请求映射 system/user messages、temperature 0/default、明确 max output；支持 capability 宣称后的 JSON schema response format，否则用 JSON-only + 本地严格校验。
3. endpoint path 用安全 URL join，不能双 `/v1`、不能将 model/path 注入 URL。
4. 解析 choices、finish reason 和 usage；空 choices、多 choices（未支持时）、truncated、refusal、非 JSON均 typed error。
5. usage 缺失标 unknown；不猜 token。
6. fake server 检查 auth header 存在但测试输出只记布尔；测试正常、429 retry、401、schema bad、oversize、cancel、usage。

**验证：** `cargo test -p novel-providers openai_compatible --offline`；`cargo fmt --all -- --check`；`git diff --check`。
**门槛：** 无公网/Key即可通过全部合同；core 只收到已校验结构。
**阻塞：** 某 endpoint 不支持 schema 时只能按 capability fallback，不把所有“类 OpenAI”服务宣称兼容。
**报告补充：** 请求/响应映射、finish reason 行为、usage unknown 行为。

---

## S4-07｜DeepSeek 兼容端点模板

**依赖：** `S4-06`。
**目标：** 使用同一 OpenAI-compatible adapter 提供 DeepSeek 独立模板和回归测试，不复制 adapter。

**只允许修改：**

- `crates/novel-providers/src/registry.rs`
- `crates/novel-providers/src/openai_compatible.rs`（仅兼容差异）
- 新建 `crates/novel-providers/tests/deepseek_contract.rs`

**步骤：**

1. 根据实现时可访问的 DeepSeek 官方文档确认 base URL/path/header/usage 字段；把访问 URL 与日期写测试注释，不复制大段文档。
2. 模型 ID保持用户可编辑，不把“最新模型”写死；模板只给当时核验过的建议值或留空。
3. fake server 按该映射验证请求；不得调用公网。
4. 若官方兼容接口不支持某 OpenAI feature，registry capability 必须关闭并走本地 schema fallback。
5. 错误/日志中 provider 标识可见，Key/正文不可见。

**验证：** `cargo test -p novel-providers --test deepseek_contract --offline`；`cargo test -p novel-providers openai_compatible --offline`；`git diff --check`。
**门槛：** DeepSeek template 能由 registry 构造并通过 fake 合同；没有复制请求栈。
**阻塞：** 无法访问官方文档时标 `EXTERNAL_BLOCKER`，保留 generic adapter，不臆造“已核验 DeepSeek”。
**报告补充：** 官方文档 URL/访问日、capability 差异、是否仅 mock 验证。

---

## S4-08｜Anthropic native adapter

**依赖：** `S4-02` 至 `S4-05`。
**目标：** 实现与 OpenAI 不同协议的 Anthropic native adapter，证明 registry 不是只换 URL。

**只允许修改：**

- 新建 `crates/novel-providers/src/anthropic.rs`
- `crates/novel-providers/src/lib.rs`
- `crates/novel-providers/src/registry.rs`
- `crates/novel-providers/Cargo.toml`
- `Cargo.lock`

**步骤：**

1. 根据 Anthropic 官方文档核验 messages endpoint、version header、x-api-key 和 usage；记录 URL/日期。
2. 映射 system/message content，使用协议支持的结构化工具/schema方式；无强制能力时仍经 S4-02 严格本地解析。
3. 解析 content blocks、stop reason、refusal/error envelope、input/output usage；不能复用 OpenAI choices parser。
4. 使用共用 HTTP/resilience/redaction/cancellation。
5. fake server 覆盖成功、tool/schema result、文本 JSON fallback、401、429、5xx、truncated、usage unknown、cancel。

**验证：** `cargo test -p novel-providers anthropic --offline`；`cargo test -p novel-providers --offline`；`git diff --check`。
**门槛：** 两个协议走不同 wire mapper、同一 core schema；自动测试不需真实 Key。
**阻塞：** 官方协议变化无法确认时标 blocker，不以 OpenAI body 猜测。
**报告补充：** 与 OpenAI 的 header/body/response 三项差异、官方文档证据。

---

## S4-09｜限制本地确定性测试 provider

**依赖：** `S4-01`。
**目标：** 保留无 Key 的可重复测试能力，但阻止其在正式 UI 冒充 AI。

**只允许修改：**

- `crates/novel-core/src/provider.rs`
- `crates/novel-core/Cargo.toml`（仅当 core 构造器也需 feature gate）
- `crates/novel-providers/src/registry.rs`
- `crates/novel-providers/Cargo.toml`（仅定义非默认 `dev-test-provider` feature 及必要的 feature 转发）
- `apps/desktop/src-tauri/src/scan_commands.rs`
- `apps/desktop/src-tauri/Cargo.toml`（仅定义非默认 QA/debug test-channel feature 及必要的 feature 转发）
- `Cargo.lock`（仅上述 Cargo feature/dependency 解析产生的最小变化）

**步骤：**

1. 添加 provider classification/test-only metadata；正式 registry 列表不返回 deterministic test。
2. 建立显式 feature 链：需要时由 desktop 的 `dev-test-provider` 转发到 `novel-providers`，再转发到 `novel-core`。所有相关 Cargo.toml 的 default features 都不得包含它；不要用 `debug_assertions`、环境变量或仅凭 debug build 自动启用。
3. 只有单元测试 `cfg(test)` 或明确带 `--features dev-test-provider` 的 QA/debug test-channel 能构造/显示它。普通开发启动与任何 release channel 均不得暴露；release build 无该 feature 时 command 必须拒绝其 ID。
4. 测试 provider 不需要 secret，usage 标“test units”，响应保持确定性。
5. 测试两条通道：显式 QA feature 下显示名称含“离线测试/不是 AI”且可构造；无 feature 的 release 配置中 registry 不含它且 command 拒绝。若为测试写辅助检查，不能改变生产可见列表。

**验证：** `cargo test -p novel-core provider --offline`；`cargo test -p novel-providers registry --no-default-features --offline`；`cargo test -p novel-providers registry --features dev-test-provider --offline`；`cargo test -p novel-scout-desktop test_provider --features dev-test-provider --offline`；`cargo check -p novel-scout-desktop --release --no-default-features --offline`；`cargo tree -p novel-scout-desktop -e features --no-default-features --offline`（输出不得出现 `dev-test-provider`）；`cargo metadata --locked --offline --format-version 1 --no-deps`；`git diff --check`。
**门槛：** 正式用户不能误选测试 provider；无 Key 的 QA/debug test-channel 仍可跑自动测试；default/release feature graph 中不存在 `dev-test-provider`。
**阻塞：** 不得用环境变量偷偷在 release 启用。
**报告补充：** build feature 和生产过滤证据。

---

## S4-10｜原生 SecretStore 抽象与 canary 测试

**依赖：** `S4-01`。
**目标：** 定义 secret write/read/delete 接口、opaque ref 和内存 fake；不实现平台 API。

**只允许修改：**

- 新建 `apps/desktop/src-tauri/src/secrets/mod.rs`
- 新建 `apps/desktop/src-tauri/src/secrets/memory.rs`
- `apps/desktop/src-tauri/src/lib.rs`（仅 module 声明）
- `apps/desktop/src-tauri/Cargo.toml`
- `Cargo.lock`

**步骤：**

1. `SecretStore` 接口 save/read/delete/availability；返回/接收 `SecretRef`，公开层只返回 state 和脱敏末尾标识。
2. secret wrapper 禁止 Serialize/Clone/Debug 明文，drop 时尽力 zeroize；错误不含输入。
3. opaque ID使用密码学随机，不由 provider/model/key hash 可猜；最终 ref 符合 SQL 长度/白名单。
4. memory fake 仅测试编译；测试 CRUD、覆盖、缺失、不可用、并发和 Debug/serde 防泄漏。
5. canary secret 不得出现在 panic、error、日志 capture 或序列化 DTO。

**验证：** `cargo test -p novel-scout-desktop secrets --offline`；`cargo fmt --all -- --check`；`cargo metadata --locked --offline --format-version 1 --no-deps`；`git diff --check`。
**门槛：** API 调用者拿不到可序列化明文；ref 符合 DB constraint。
**阻塞：** Rust 中无法保证拷贝完全清零时如实记录限制，不声称“内存绝对无痕”。
**报告补充：** secret wrapper traits、ref 示例（非真实）、canary scan。

---

## S4-11｜Windows Credential Manager adapter

**依赖：** `S4-10`。
**目标：** Windows 使用系统 Credential Manager 保存 API Key，SQLite 只保存 ref。

**只允许修改：**

- 新建 `apps/desktop/src-tauri/src/secrets/windows.rs`
- `apps/desktop/src-tauri/src/secrets/mod.rs`
- `apps/desktop/src-tauri/Cargo.toml`
- `Cargo.lock`

**步骤：**

1. 使用审计过的 Windows API/crate 实现 generic credential write/read/delete；target name 仅含 app namespace + opaque ID。
2. 系统 API buffer 必须按文档释放；禁止 unsafe 泄漏到业务模块，若必须 unsafe 则不能违反 workspace `forbid(unsafe_code)`，优先使用安全 wrapper。
3. 读取只在发请求前的原生 adapter factory 中发生；不返回 Tauri DTO。
4. 用 injectable Windows API fake 做 CI 单测；另写 Windows-only smoke test，生成随机 target 并在 finally/delete 清理。
5. 测试 Unicode key、覆盖、delete idempotent、系统 unavailable、错误脱敏。

**验证：** `cargo test -p novel-scout-desktop windows_secret --offline`；`cargo check --workspace --all-targets --offline`；Windows CI；`git diff --check`。
**门槛：** smoke 写读删成功且清理；日志/SQLite无 canary。
**阻塞：** runner policy 不允许真实 credential smoke 时 fake 合同可过但任务标 `PARTIAL/BLOCKED`，不能声称系统已验证。
**报告补充：** API/crate、target 命名、smoke cleanup 证据。

---

## S4-12｜Android Keystore bridge 接口

**依赖：** `S4-10`。
**目标：** 固定 Rust ↔ Android 原生密钥封装合同和 fake 测试，为 S6 真机实现留清晰边界。

**只允许修改：**

- 新建 `apps/desktop/src-tauri/src/secrets/android.rs`
- `apps/desktop/src-tauri/src/secrets/mod.rs`
- 新建 `docs/ANDROID_KEYSTORE_CONTRACT.md`

**步骤：**

1. 定义 Android bridge save/read/delete/availability，Rust 只传 opaque alias 和 secret bytes；不传 SQLite 路径或 profile JSON。
2. 文档规定：Keystore 中的 non-exportable key 加密 app-private blob；清除应用数据后不可恢复；不得使用硬编码 key/IV；认证失败 typed error。
3. fake bridge 测 CRUD、alias 隔离、删除、key invalidated、清除数据模拟、错误脱敏。
4. `#[cfg(target_os="android")]` 边界必须可编译；若 native scaffold 尚未提交，只提供 trait/bridge，不写空函数返回成功。
5. 明确本任务不是 Android 真机 Keystore 完成证据，S6 task 接续 JNI/Kotlin 和设备测试。

**验证：** `cargo test -p novel-scout-desktop android_secret_contract --offline`；能运行时 `cargo check --target aarch64-linux-android`；`git diff --check`。
**门槛：** fake 合同通过、无“成功但没存”的 stub、文档列出 S6 验证点。
**阻塞：** 无 Android target 记录 EB/HG，继续其他 S4；不得标真机 PASS。
**报告补充：** bridge 方法、真实实现状态、S6 handoff。

---

## S4-13｜provider profile v3 迁移

**依赖：** `S4-01`、`S4-10`。
**目标：** 持久化公开 provider 配置、预算和 secret ref，数据库没有 Key/header/body 列。

**只允许修改：**

- 新建 `apps/desktop/src-tauri/migrations/0003_provider_runtime.sql`
- `apps/desktop/src-tauri/src/lib.rs`（只注册 v3）
- `apps/desktop/scripts/validate-migration.mjs`

**步骤：**

1. 不改 v1/v2。新增 protocol、timeout、retry/rate、预算、outbound acknowledgement/version 等公开列或规范 JSON。
2. `credential_ref` 继续原约束；配置为 configured 必须有 ref，删除 secret 后更新 missing。
3. base URL 不允许 userinfo/fragment 的复杂验证放 Rust；DB 至少做非空/长度。
4. validator 覆盖 fresh v1→v2→v3、旧 profile 保留、合法配置、非法范围、foreign key/integrity。
5. 用 canary key 搜整个数据库 dump/列名/值，必须不存在；`credential_ref` 可存在。

**验证：** 内置 Node 运行 `validate-migration.mjs`；`cargo metadata --locked --offline --format-version 1 --no-deps`；`git diff --check`。
**门槛：** 升级数据保留、canary 无命中、旧 migration 无 diff。
**阻塞：** 发现旧 profile 曾存明文时立即 SECURITY BLOCKER，不自动复制。
**报告补充：** v3 列/约束、新增 validator 数、canary 结果。

---

## S4-14｜Tauri profile 与凭据命令

**依赖：** `S4-11`、`S4-12`、`S4-13`。
**目标：** 前端可管理公开配置和一次性提交 Key，但任何返回 DTO 不含 secret/ref 内部细节。

**只允许修改：**

- 新建 `apps/desktop/src-tauri/src/provider_commands.rs`
- `apps/desktop/src-tauri/src/lib.rs`
- `apps/desktop/src-tauri/src/secrets/mod.rs`
- `apps/desktop/src-tauri/src/persistence.rs`（仅 profile CRUD）

**步骤：**

1. commands：list templates/profiles、upsert profile、set credential、delete credential、get credential state；不提供 get plaintext。
2. set credential 的明文只存在命令参数和 secret wrapper，写入成功后保存 ref；若 DB 写失败，删除新 secret；若 secret 写失败，不改 DB。
3. 更新 credential 时先写新 secret→事务换 ref→删除旧 secret；清理失败记录脱敏 warning 和 cleanup debt，不能丢新配置。
4. profile DTO 只返回 `missing/configured/unavailable` 和可选 masked hint；不得返回 `credential_ref`。
5. command/tauri tracing 不记录 args；错误不回显 endpoint query、key 或系统 target。
6. 测试每个失败时序、无 key 启动、delete idempotent、跨 profile 隔离。

**验证：** `cargo test -p novel-scout-desktop provider_commands --offline`；`cargo fmt --all -- --check`；`git diff --check`。
**门槛：** 失败时 secret/DB 不产生悬空不一致；DTO snapshot 无秘密。
**阻塞：** 无平台 secret store 时 profile 可保存为 missing，本地功能继续；绝不能回退 SQLite 明文。
**报告补充：** command DTO 字段、四个失败时序结果。

---

## S4-15｜安全的连接测试与 adapter factory

**依赖：** `S4-06`、`S4-08`、`S4-14`。
**目标：** 从 registry/profile/SecretStore 构造 adapter，并用不含小说正文的最小请求测试连接。

**只允许修改：**

- 新建 `crates/novel-providers/src/factory.rs`
- `crates/novel-providers/src/lib.rs`
- `apps/desktop/src-tauri/Cargo.toml`（加入 `novel-providers` 本地 path dependency；不得用 crates.io 同名包替代）
- `Cargo.lock`（仅记录 workspace/path dependency 接线产生的最小变化）
- `apps/desktop/src-tauri/src/provider_commands.rs`
- `apps/desktop/src-tauri/src/scan_commands.rs`

**步骤：**

1. 在 `apps/desktop/src-tauri/Cargo.toml` 显式加入 `novel-providers = { path = "../../../crates/novel-providers" }`（若实际相对路径不同，先用 `Resolve-Path` 验证后使用真实路径），更新 lockfile，并用 metadata 确认 desktop 解析到当前 workspace crate；不得复制 adapter 代码到 Tauri crate。
2. factory 校验 profile protocol 与 template；原生层按 ref 读取 secret 并立即构造 adapter，前端不可接触。
3. connection test 使用固定原创最小 schema/ping，不读取任何 book/chapter，UI/DTO 标明可能产生少量 API 消耗。
4. 返回 latency、provider/model、capability、typed result；不返回 raw response。
5. scan job 创建时冻结 provider/model/profile公开配置 hash；切换 profile 不改变历史 job。
6. Key missing/unavailable、auth、model not found、rate limit、timeout、schema unsupported 分开。
7. fake server 为两个协议测试；断言请求 body 不含任意 fixture book marker，并断言 desktop command 实际经 `novel-providers` factory 构造，未出现第二份 wire mapper。

**验证：** `cargo metadata --locked --offline --format-version 1`（输出中 desktop dependency 必须指向 workspace `novel-providers`）；`cargo tree -p novel-scout-desktop --edges normal --offline`（必须含一个 workspace `novel-providers`）；`cargo test -p novel-providers factory --offline`；`cargo test -p novel-scout-desktop connection_test --offline`；`git diff --check`。
**门槛：** desktop 的 normal dependency graph 正确接入且只接入一份 workspace `novel-providers`；两协议均由 factory 构造；连接测试不出站小说内容；missing key 是可恢复配置错误。
**阻塞：** 真实 Key 不在本任务需要。
**报告补充：** ping payload 白名单、错误分类。

---

## S4-16｜设置 UI、连接测试与预算

**依赖：** `S4-14`、`S4-15`。
**目标：** 非程序员风格完成 provider 添加、Key 保存、模型/endpoint/timeout/预算设置和连接测试。

**只允许修改：**

- `apps/client/src/domain.ts`
- 新建 `apps/client/src/services/providerService.ts`
- `apps/client/src/components/SettingsPanel.tsx`
- `apps/client/src/components/SettingsPanel.css`
- `apps/client/src/hooks/useAppState.ts`
- 新建或修改 `apps/client/src/__tests__/SettingsPanel.test.tsx`
- 新建或修改 `apps/client/src/__tests__/providerService.test.ts`

**步骤：**

1. 文案明确“ChatGPT Plus/Codex/Claude 订阅不包含 API 调用额度”；给官方获取 Key 帮助链接占位由文档核验。
2. Key 使用 uncontrolled password input，不进入全局 React state/localStorage/sessionStorage/URL；提交后立即清空，不能提供显示明文按钮。
3. profile 表单按 template 显示 endpoint/model；custom endpoint 展示隐私/信任警告，非法 URL前端和原生双校验。
4. 连接测试按钮说明可能有少量费用；显示 typed 结果，不显示 raw JSON。
5. 预算设置映射 S3 request/input/output/total units；没有可靠价格表时不显示虚构人民币估价。
6. 无 Key/profile 时仍能进入所有本地页面；开始在线扫描 disabled，并给自然语言说明。
7. 浏览器预览明确“未连接原生安全存储”；不能把输入 Key 保存在 demo state。
8. 测试键盘、label/description、清空 password、service DTO 无 key 返回、missing/unavailable、连接分类、小屏布局 DOM。

**验证：** 在 `apps/client` 运行聚焦 Vitest、`npm test`、`npm run build`；`git diff --check`。
**门槛：** 前端持久状态/DTO snapshot 无 key；无 Key 本地功能可操作；订阅与 API 文案准确。
**阻塞：** UI 自动填充行为无法完全禁止时使用标准 `autocomplete="new-password"` 并记录限制，不存储。
**报告补充：** Key 生命周期、Plus 文案、无 Key 可用功能清单。

---

## S4-17｜正文出站确认与按书授权

**依赖：** `S4-15`、`S4-16`。
**目标：** 第一次用某 profile 扫某本书前清楚告知出站数据，并保存版本化确认；切换 host 需重新确认。

**只允许修改：**

- `apps/desktop/src-tauri/src/provider_commands.rs`
- `apps/desktop/src-tauri/src/scan_commands.rs`
- `apps/client/src/domain.ts`
- `apps/client/src/components/ScanProgress.tsx`
- `apps/client/src/components/ScanProgress.css`
- `apps/client/src/__tests__/ScanProgress.test.tsx`

**步骤：**

1. 展示 provider 名、endpoint host、model、发送内容（当前窗口/规则/有界记忆）、不发送内容（路径/URI/Key/其他书）。
2. 授权按 book fingerprint + profile config hash + disclosure version 保存；正文变化、host/protocol/披露版本变化重新确认。
3. 用户拒绝时不发请求且本地功能不受影响；不能用预勾 checkbox。
4. scan command 在原生层也检查授权，不能只靠前端。
5. 连接测试不需要正文授权，因为 S4-15 保证没有书内容；文案仍说明有网络请求。
6. 测试接受/拒绝、切 host、改 model（按策略）、改正文、浏览器 fallback、键盘焦点。

**验证：** desktop 聚焦测试；前端 ScanProgress 测试、全 test/build；`git diff --check`。
**门槛：** 未确认时 fake server 请求数为 0；确认后 outbound 字段符合 S4-03 白名单。
**阻塞：** 不得以“用户配置过 Key”推定同意上传正文。
**报告补充：** 授权 fingerprint 字段和重新确认矩阵。

---

## S4-18｜密钥/正文泄漏回归与无 Key 离线门

**依赖：** `S4-01` 至 `S4-17`。
**目标：** 自动证明 Key 不在 Git/日志/SQLite/DTO，完整正文不在日志，请求只含白名单；无 Key 可正常启动和使用本地功能。

**只允许修改：**

- 新建 `apps/desktop/scripts/validate-secrets.mjs`
- `apps/desktop/package.json`
- `.github/workflows/ci.yml`（只增加 validation step）
- 新建 `crates/novel-providers/tests/leakage.rs`
- 新建或修改 `apps/client/src/__tests__/offlineWithoutKey.test.tsx`

**步骤：**

1. 生成唯一 canary key、auth header、chapter marker；运行 fake provider 成功/401/429/500/schema/timeout/cancel 路径，capture 日志/错误/DTO/SQLite dump。
2. 断言 key/header 在上述产物零命中；章节完整 marker 在日志/SQLite零命中。HTTP fake 收到允许的当前 window 是预期，但测试输出不能打印。
3. validator 静态检查 migration/TS demo/env/example/log 配置中无 key 字段或疑似真实 key；降低误报要用 allowlist 加理由，不能删除规则。
4. 测试无 provider profile、missing key、secret store unavailable 时 app 启动、导入/书架/规则/历史/删除均可用，在线扫描明确 disabled。
5. CI 不读取 GitHub secrets、不访问公网。

**验证：** `npm run validate:secrets`（desktop）；`cargo test -p novel-providers --test leakage --offline`；frontend 聚焦测试和 build；`git diff --check`。
**门槛：** 所有 canary 零泄漏；CI 离线可过；无 Key UI 测试通过。
**阻塞：** 任一明文命中是 SECURITY RETRY，S4 不得继续 live test。
**报告补充：** 扫描产物清单、canary 命中数、allowlist（应尽量为空）。

---

## S4-19｜用户自选一个真实 provider 的最小 smoke

**依赖：** 工程准备依赖 `S4-18`；执行真实 smoke 时才触发 `HG-001`。
**目标：** 用户从已经实现的 provider 中任选 **一个**真实 provider（OpenAI-compatible/DeepSeek 模板或 Anthropic native），在原生安全存储配置后做一次最低成本 smoke；第二种协议以严格 loopback fake 合同作为必需证据，第二次真实实测完全可选。仓库和报告不接触 Key。

**只允许修改：**

- `docs/deepseek-runbook/02_TASK_LEDGER.md`
- `docs/deepseek-runbook/03_BLOCKERS_AND_DEBT.md`
- 新建 `docs/provider-live-test-template.md`

**步骤：**

1. DeepSeek 只生成操作说明：用户在 app 设置页输入 Key；禁止让用户粘贴到聊天、shell history、env 文件或 GitHub issue。操作说明默认只要求一个 provider，不能诱导用户购买第二个账号或更多额度。
2. 先附上 OpenAI-compatible 与 Anthropic native 两条严格 loopback fake 合同的最新 CI 证据；两条都必须通过 schema、timeout/cancel、oversize、脱敏和 evidence 重建合同。
3. 用户任选一个真实 provider，使用一章极短原创夹具和一条测试规则；先 connection test，再执行一个窗口扫描。请求次数固定为最小值，失败后不得自动反复烧额度。
4. 只记录所选 provider 的 template/protocol、model ID、时间、结果状态、usage 是否返回、finding 是否由原文重建、日志/DB canary 检查；不记录 raw prompt/response/key。
5. auth error、cancel 或 rate-limit 的真实验证均为可选；已有 fake 合同即可，不得为了覆盖率故意制造付费错误。第二协议的真实 smoke 也仅在用户主动愿意且已有账号时执行。
6. 用户没有任何 Key/额度时：本任务 ledger 状态写 `HUMAN_PENDING`，备注写 `HG-001`；同时明确 S4 工程可为 `ENGINEERING_DONE`，继续 `S4-20`、S5、S6 等不依赖 live API 的工作。不得把该人工等待传播成全项目阻塞，也不得误写为 `BLOCKED`。

**验证：** 人工记录 + 对应本地数据库/日志脱敏扫描；不得用 curl 直接把 Key 写命令。
**门槛：** 两种协议的严格 fake 合同均通过，且用户自选的一个真实 provider 完成 connection test + 单窗口扫描，才把 `S4-19` 标 `DONE`。第二协议真实成功不是门槛。
**人工等待：** 额度/账号完全由用户决定；无 Key 时使用 `HUMAN_PENDING(HG-001)`，不得建议购买 Pro 作为 API 解决方案。
**报告补充：** 两协议 fake 合同证据、一条可选 real smoke 脱敏记录、第二 real smoke 是否跳过、`ENGINEERING_DONE` 与 `HUMAN_PENDING(HG-001)` 两个独立状态。

---

## S4-20｜S4 阶段门禁

**依赖：** 工程门依赖 `S4-01` 至 `S4-18`。`S4-19` 是并行人工验收：若为 `HUMAN_PENDING(HG-001)`，不得阻止执行或完成本工程门。
**目标：** 全量验证 provider、安全存储边界、设置/授权 UI、恢复与 CI，并把工程完成与真实账号人工验收分开记录。

**只允许修改：**

- `docs/deepseek-runbook/02_TASK_LEDGER.md`
- `docs/deepseek-runbook/03_BLOCKERS_AND_DEBT.md`
- `docs/deepseek-runbook/80_ACCEPTANCE_MATRIX.md`
- `.github/workflows/ci.yml`（仅缺少既有 S4 校验步骤时）

**步骤与门槛：**

1. `cargo fmt --all -- --check`、`cargo check --workspace --all-targets`、`cargo test --workspace --all-targets` 在 Windows CI 全绿。
2. `novel-providers` 两协议 fake 合同、重试/限流/timeout/cancel/schema/oversize/leakage 全过。
3. migration v1→v2→v3、capability、secret validator 全过；canary key 在 Git diff、日志、SQLite、DTO 零命中。
4. frontend 全 test/build；无 Key 本地功能、credential UI、connection test、outbound disclosure、预算/错误恢复均有测试。
5. Windows Credential Manager 有真实写读删清理证据；Android 只能标“bridge contract complete”，真实 Keystore 留 S6。
6. provider/network 失败后 S3 checkpoint position 不丢、不重复 findings/usage。
7. 两协议严格 fake 合同与全部工程门通过时记录 `ENGINEERING_DONE`。若 `S4-19` 有一个真实 provider 成功证据，真实 smoke 行标 `PASS`；否则单独标 `HUMAN_PENDING(HG-001)`，不能把 fake 写成真实成功，但也不能把 S4 工程或后续 S5/S6 标为阻塞。
8. 保存 CI run URL、提交 SHA、依赖审计摘要；确认 deterministic test provider 不在 release registry。

**完整验证命令：**

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --offline
cargo test --workspace --all-targets --offline
Push-Location apps/client
npm test
npm run build
Pop-Location
Push-Location apps/desktop
npm run validate:migration
npm run validate:capability
npm run validate:secrets
Pop-Location
git diff --check
git status --short
```

**完成状态：** 所有工程项通过后，`S4-20` 可标 `DONE` 并报告 `ENGINEERING_DONE`；真实 Key 缺失只在 S4-19/acceptance 对应行保留 `HUMAN_PENDING(HG-001)`。二者必须同时诚实呈现。
**阻塞策略：** 本地 `link.exe` 用 Windows CI补证；真实 Key、Android真机只登记 HUMAN_GATE，不阻塞独立工程。任何密钥泄漏、授权绕过、正文日志、schema 放宽、checkpoint 丢失均为硬 RETRY。
**报告补充：** CI URL、mock/real provider矩阵、Windows/Android secret状态、canary零泄漏、acceptance matrix状态、`ENGINEERING_DONE` 与 `HUMAN_PENDING` 分栏。

## S4 最终产物清单

完成 `S4-20` 的工程门后应具备：

- 可扩展 provider registry；OpenAI-compatible、DeepSeek template、Anthropic native 和仅测试可见的 deterministic provider；
- 严格结构化输出、响应大小限制、timeout、取消、重试、限流和安全 telemetry；
- Windows Credential Manager 实现、Android Keystore bridge 合同、SQLite opaque secret ref；
- 设置、连接测试、预算和正文出站确认 UI；
- 明确“ChatGPT Plus/Codex 订阅不等于 API 额度”；无 Key 本地功能照常可用；
- 日志、错误、DTO、SQLite 和 Git 无明文 Key，日志不含整章正文；
- 无真实账号时保留 `HUMAN_PENDING(HG-001)`，不把 fake server 结果冒充真实 API 成功，也不阻断 S5/S6；有账号时只要求用户任选一个 provider 做最低成本真实 smoke。
