# S6：Windows 与 Android 产品化

本文件给 DeepSeek 使用。遵守 `00_EXECUTION_CONTRACT.md`，一次只执行一个任务并提交。总任务数：**25**。

## S6 总边界

- 产品平台只有 Windows 和 Android；不做 iOS，也不为应用商店上架设计流程。Android 交付方式是 GitHub Release 侧载。
- S2 已负责格式解析和 Windows 文件导入。S6-WIN-01 只审计“安装后的真实 shell”并做必要阻断修复，禁止重写 S2 importer。
- Android 文件来源必须使用 Storage Access Framework（SAF）URI；不得伪造绝对路径，不得假设永久权限一定成功。
- Provider secret 必须由 Android 平台 Keystore 保护，Rust/前端只持有 opaque `secret-ref:`。发布签名用的用户自建 Android signing keystore 是另一个概念：它必须由用户在仓库外创建并保管，正式 Android 资产必须用它签名。
- Windows 商业代码签名是可选人工门 `HG-004W`；缺少证书时允许发布明确标记的未签名 GitHub 侧载资产，并公开 SmartScreen/发行者未知限制。不得伪称已签名。
- 工程验证可在 QA/debug test channel 使用确定性 dev test provider；release build 必须从编译、配置、命令和 UI 中移除该 provider。真实 provider 只要求 `HG-001` 使用用户任选一种 provider 做最小 smoke，不要求耗费额度逐个验证。
- 人工门不连锁阻塞独立工程。工程完成但缺设备、key 或签名材料时，报告可写阶段说明 `ENGINEERING_DONE`，task ledger 写 `HUMAN_PENDING`，并继续其他不依赖该材料的任务。
- DeepSeek 没有图像理解能力：截图/录屏只能由 Playwright/设备脚本自动留档，验收依赖 DOM、几何、ARIA、pixel 工具的数值结果或真实人工复核记录，不能靠 DeepSeek“看图解释”。

## 通用执行协议

每个任务开始：

```powershell
git status --short
git rev-parse --short HEAD
```

只改任务范围；未知改动立即停止。禁止 `git reset --hard`、`git checkout --`、`git clean -fd`、强推、伪造退出码。每个任务结束：

```powershell
git diff --check
git diff --stat
git status --short
```

统一报告：任务 ID、基线、实际文件、设计决定、逐命令退出码、测试通过数、未执行项、人工门/阻塞、状态、提交 SHA。task ledger 状态只用 `DONE`、`RETRY`、`AWAITING_CI`、`HUMAN_PENDING`、`BLOCKED`；`IN_PROGRESS` 只在执行中使用。失败且可继续修复写 `RETRY`，等待已触发 CI 写 `AWAITING_CI`。

## 任务依赖图

| 任务 ID | 依赖 | 原子结果 |
| --- | --- | --- |
| S6-WIN-01 | S2-15、S3-18、S4-20 | 已安装 Windows shell 审计、冒烟与必要阻断修复 |
| S6-WIN-02 | S6-WIN-01 | Windows 安装、升级、卸载验证 |
| S6-AND-01 | S1-15 | 经审查的 Tauri Android scaffold |
| S6-AND-02A | S6-AND-01、S2-12 | SAF 原生合同与 Kotlin 实现 |
| S6-AND-02B | S6-AND-02A | SAF Rust 桥与 URI 生命周期 |
| S6-AND-02C | S6-AND-02B | Android 导入 UI |
| S6-AND-02D | S6-AND-02C | SAF 真机人工门 |
| S6-AND-03A | S6-AND-01、S4-12 | Android Keystore 原生实现 |
| S6-AND-03B | S6-AND-03A | Keystore Rust 桥 |
| S6-AND-03C | S6-AND-03B | Provider secret UI 接线 |
| S6-AND-03D | S6-AND-03C | Keystore 真机人工门 |
| S6-AND-04A | S6-AND-01、S3-09 | 前台服务与通知原生实现 |
| S6-AND-04B | S6-AND-04A | 长任务 Rust 生命周期桥 |
| S6-AND-04C | S6-AND-04B | 暂停/停止/恢复 UI |
| S6-AND-04D | S6-AND-04C | 后台/进程终止真机人工门 |
| S6-UI-01A | S1-15 | 响应式布局 |
| S6-UI-01B | S6-UI-01A | 系统返回与旋转状态保存 |
| S6-UI-01C | S6-UI-01A | 软键盘、大字体与 a11y |
| S6-UI-01D | S6-UI-01B、S6-UI-01C | UI 真机人工门 |
| S6-UX-01A | S6-UI-01A、S2-14、S3-16、S3-17、S5-PRESET-02 | 非程序员主流程与渐进披露 |
| S6-UX-01B | S6-UX-01A | 自然语言状态、错误与技术词隐藏 |
| S6-UX-01C | S6-UX-01B | DOM/几何/ARIA/Playwright UX 门 |
| S6-E2E-01 | S6-AND-02C、S6-AND-03C、S6-AND-04C、S6-UI-01C、S6-UX-01C | 双平台完整流程自动 E2E |
| S6-BUILD-01 | S6-E2E-01 | Windows/Android 构建与签名策略 |
| S6-GATE-01 | S6-WIN-02、S6-AND-02C、S6-AND-03C、S6-AND-04C、S6-UI-01C、S6-UX-01C、S6-E2E-01、S6-BUILD-01 | 产品化工程门和人工门矩阵；D 类人工门可 pending |

---

## S6-WIN-01：已安装 Windows shell 审计与冒烟

**目标**：验证真实安装后应用能调用既有 S2 导入、S3 扫描和 S4 设置，不重新实现 importer。

**精确范围**：优先只新增 `apps/desktop/tests/windows-installed/**`、`apps/desktop/scripts/test-installed-windows.mjs` 和证据说明。仅当冒烟暴露阻断时，允许最小修改 `apps/desktop/src-tauri/tauri.conf.json`、`apps/desktop/src-tauri/capabilities/main-local.json`、`apps/desktop/src-tauri/src/lib.rs`、`apps/client/src/services/nativeBridge.ts`（若文件尚未存在可新建）、`apps/client/src/services/importCapabilities.ts` 和直接测试。**禁止修改 `crates/novel-import/**` 或重写 S2 格式解析。**

**执行**：从安装产物启动，不用 dev server；选择真实本地 fixture，验证命令注册、capability、路径脱敏、导入后章节/指纹、一次 pause/resume 和 provider 设置入口。若失败，先写复现测试，再做唯一最小修复。

**测试/门槛**：测试脚本能区分“未安装”“shell 启动失败”“命令失败”“导入失败”；安装态 smoke 有真实退出码和脱敏日志。无 Windows 运行环境时登记 `EB-002`；已触发 Windows CI 时 ledger 写 `AWAITING_CI`，否则写 `BLOCKED`，不得创建临时人工门。

---

## S6-WIN-02：Windows 安装、升级与卸载

**目标**：确认全新安装、同通道升级和卸载不会静默破坏用户数据。

**精确范围**：`apps/desktop/src-tauri/tauri.conf.json`、Windows bundle 配置、`apps/desktop/tests/windows-installed/**`、`docs/WINDOWS_INSTALL.md` 和必要 workflow；不得改业务模型、importer 或 scanner。

**执行**：用干净 VM/账户验证安装、启动、旧版本数据库升级、卸载行为；数据保留/删除必须显式说明；损坏升级必须停止并给备份指引。

**测试/门槛**：全新安装、升级、卸载三份结构化记录；资产 SHA-256、版本、架构和签名状态明确。无 Windows VM/runner 时登记 `EB-002`；已触发 CI 写 `AWAITING_CI`，否则写 `BLOCKED`。

---

## S6-AND-01：生成并审查 Tauri Android scaffold

**目标**：把可复现 Android 工程纳入版本控制，不把临时生成目录或本机路径提交。

**精确范围**：`apps/desktop/src-tauri/gen/android/**`、Tauri/Gradle 配置、`.gitignore`、`docs/ANDROID_BUILD.md`、`.github/workflows/android*.yml`。不得实现 SAF、Keystore 或前台服务业务。

**执行**：固定 package/application ID、min/target SDK、ABI、WebView/网络安全配置；审查 manifest 权限最小化；确保生成步骤幂等且不写 SDK 绝对路径、证书或 keystore。

**测试/门槛**：locked Gradle 配置检查、manifest 权限快照、debug assemble（环境可用时）和仓库敏感路径扫描通过。

---

## S6-AND-02A：SAF 原生合同与 Kotlin 实现

**目标**：在 Android 原生层完成文档选择、临时读取和持久授权，不涉及 Rust/UI。

**精确范围**：只允许 `apps/desktop/src-tauri/gen/android/app/src/main/java/**/saf/**`、必要 manifest/query 配置和 JVM/instrumentation 测试。不得改 Rust、React 或 importer。

**执行**：定义 request/result/error DTO；使用 `ACTION_OPEN_DOCUMENT`、MIME/扩展过滤和 `takePersistableUriPermission`；返回 opaque URI token、显示名、大小、MIME、授权结果，禁止解析为绝对路径；处理取消、provider 无 size、权限拒绝、URI 失效。

**测试/门槛**：Kotlin 单测覆盖所有结果分支；manifest 不新增广泛存储权限；原生合同文档化。

---

## S6-AND-02B：SAF Rust 桥与 URI 生命周期

**目标**：把 SAF 原生结果安全桥接到既有 S2 importer API。

**精确范围**：`apps/desktop/src-tauri/src/android/saf.rs`、android 模块注册、`apps/desktop/src-tauri/src/lib.rs` 的最小命令注册、`crates/novel-import/src/source.rs`（仅若 S2 已定义 source abstraction 且缺 Android adapter）和直接测试。不得改 React。

**执行**：Rust 只接收 URI/ref 与元数据，通过受控 stream/临时文件喂给 importer；关闭句柄并清理临时文件；重启后验证持久授权，失效时返回稳定错误码；日志禁止完整 URI。

**测试/门槛**：mock bridge 测试覆盖成功、取消、权限拒绝、重启恢复、URI 失效、超限和清理；S2 importer 原测试不退化。

---

## S6-AND-02C：Android 导入 UI

**目标**：在 Android 上用系统选择器完成导入，用户不看到 URI 或路径术语。

**精确范围**：`apps/client/src/components/ImportPanel.*`、`apps/client/src/services/nativeBridge.ts`、`apps/client/src/services/importCapabilities.ts`、相关 hooks/types/tests。不得改 Kotlin、Rust 或格式解析。

**执行**：平台检测后调用 SAF；显示文件名、格式、大小、授权可用性和自然语言错误；取消不显示失败；授权失效提供“重新选择文件”。

**测试/门槛**：DOM 测试覆盖成功、取消、拒绝、失效、过大、unsupported 与重选；不渲染 `content://`、绝对路径或原始错误对象。

---

## S6-AND-02D：SAF 真机门

**目标**：由真实 Android 设备证明本地文件和云文档 provider 的选择、重启和失效行为。

**精确范围**：只允许 `docs/evidence/android/saf/**` 和任务台账；发现代码缺陷只登记并回到 02A/02B/02C 新修复任务，不在 gate 内改业务代码。

**执行/门槛**：人工在至少一台真机测试本地文件、一个可用的云 provider、取消、撤销授权、重启后恢复；记录脱敏 OS/WebView/版本、步骤、结果和自动日志哈希。截图/录屏仅留证，不由 DeepSeek解释。无设备时报告写阶段说明 `ENGINEERING_DONE`、ledger 写 `HUMAN_PENDING`，登记 `HG-003` 的 SAF 子场景。

---

## S6-AND-03A：Android Keystore 原生 secret 实现

**目标**：设备端生成/使用不可导出的 Keystore key，保存 provider secret 密文。

**精确范围**：`apps/desktop/src-tauri/gen/android/app/src/main/java/**/secrets/**`、必要 Android 测试；不得改 Rust、UI 或发布签名配置。

**执行**：定义 put/get/delete/exists 合同；使用 Android Keystore + AEAD，随机 nonce，alias 与 provider profile 绑定；明文只在调用栈短暂存在；处理锁屏、key invalidated、密文篡改和升级。

**测试/门槛**：JVM/instrumentation 能覆盖的 round-trip、不同 alias、篡改、删除、key invalidation 错误映射通过；日志不含 secret。

---

## S6-AND-03B：Keystore Rust 桥

**目标**：实现 S4 `SecretStore` Android adapter，Rust 只暴露 opaque `secret-ref:`。

**精确范围**：`apps/desktop/src-tauri/src/android/secret_store.rs`、android 模块注册、S4 SecretStore 接线和直接 Rust 测试。不得改 React 或 Kotlin 加密算法。

**执行**：ref 后缀只用允许字符；禁止序列化明文；provider 调用时按最短生命周期取 secret 并清零可控缓冲；稳定映射 unavailable/locked/invalidated/not-found。

**测试/门槛**：mock native adapter 覆盖 CRUD、profile 隔离、无明文 persistence、错误映射和日志脱敏；Windows secret adapter 不退化。

---

## S6-AND-03C：Provider secret UI 接线

**目标**：用户在 Android 设置 API 密钥时只看到保存/替换/删除状态，不看到存储技术细节。

**精确范围**：`apps/client/src/components/SettingsPanel.*`、provider service/hook/types 和测试。不得改 native 或 Rust secret 实现。

**执行**：密码框不可回显已存值；保存后清空输入；替换与删除二次确认；锁定/失效显示自然语言恢复步骤；不得展示 `secret-ref:`、alias、Keystore 错误或 token 术语。

**测试/门槛**：DOM 测试覆盖保存、替换、删除、取消、locked/invalidated、卸载后缺失；快照/日志/错误中无 secret。

---

## S6-AND-03D：Keystore 真机门

**目标**：真实设备验证重启、锁屏、升级和卸载后的 secret 行为。

**精确范围**：只允许 `docs/evidence/android/keystore/**` 和任务台账；gate 内不改代码。

**执行/门槛**：人工验证保存、进程重启、设备重启、替换、删除、升级保留、卸载清除；可行时验证 key invalidation。记录自动测试输出哈希，不记录 key。无设备时报告写阶段说明 `ENGINEERING_DONE`、ledger 写 `HUMAN_PENDING`，登记 `HG-003` 的 Keystore 子场景。

---

## S6-AND-04A：前台服务与通知原生实现

**目标**：为长扫描提供合规前台服务、通知通道和停止 action，不涉及 Rust 扫描状态机。

**精确范围**：`apps/desktop/src-tauri/gen/android/app/src/main/java/**/scanservice/**`、manifest/service/notification 资源和原生测试。不得改 Rust 或 React。

**执行**：只在用户启动扫描后启动服务；通知显示真实任务状态，不显示小说标题/正文/provider；Android 版本化处理通知权限；停止 action 发出明确 native event。

**测试/门槛**：原生测试覆盖启动、更新、停止、重复调用、权限拒绝和 service teardown；manifest service/permission 最小化。

---

## S6-AND-04B：长任务 Rust 生命周期桥

**目标**：把 native service action 映射到 S3 pause/cancel/checkpoint/recover，保证幂等。

**精确范围**：`apps/desktop/src-tauri/src/android/scan_service.rs`、`crates/novel-core/src/scanner.rs`/checkpoint 模块的最小接线和测试、命令注册。不得改 React/Kotlin UI。

**执行**：启动扫描成功后才启动 service；pause/stop 先落安全 checkpoint；进程恢复校验 fingerprint；重复 event、乱序 event 和 service 丢失不得重复 finding 或越过确认门。

**测试/门槛**：状态机单测覆盖 start/pause/resume/stop/crash/replay/乱序；checkpoint 与 persisted finding 验证测试不退化。

---

## S6-AND-04C：暂停、停止、恢复与通知权限 UI

**目标**：让用户清楚控制长扫描并理解后台限制。

**精确范围**：`apps/client/src/components/ScanProgress.*`、相关 workspace/hook/native service、测试。不得改 native/Rust 状态机。

**执行**：只显示真实状态和真实已处理章节；暂停、继续、停止二次确认；通知权限拒绝时说明“离开应用后可能中断”，不得伪造后台成功；恢复前显示 checkpoint 时间和待处理数。

**测试/门槛**：DOM 测试覆盖所有状态、重复点击、权限拒绝、恢复失败和用户取消；不使用假进度定时器。

---

## S6-AND-04D：后台与进程终止真机门

**目标**：真机验证锁屏、切后台、系统终止和手动停止后的数据完整性。

**精确范围**：只允许 `docs/evidence/android/long-scan/**` 和台账；gate 内不改代码。

**执行/门槛**：人工运行长 fixture，验证后台、锁屏、强制结束进程、重新打开恢复、通知停止；比较 finding ID/数量/checkpoint，确认无重复/丢失。录屏仅留证；验收用结构化状态日志和哈希。无设备时登记 `HG-003` 的长任务子场景，ledger 写 `HUMAN_PENDING`。

---

## S6-UI-01A：响应式布局

**目标**：主界面在手机、平板和桌面宽度可用，无横向溢出或被遮挡操作。

**精确范围**：`apps/client/src/App.css`、`apps/client/src/index.css`、`apps/client/src/components/*.css` 和只为布局容器所需的 JSX；不得改业务 hook/service。

**执行**：建立 390/800/1440 断点；手机使用单列和底部/紧凑导航，桌面保留侧栏；触控目标至少 44 CSS px；证据文本可换行且不截断定位来源。

**测试/门槛**：Playwright/DOM geometry 在三宽度断言 scrollWidth、可见操作、触控尺寸和 evidence 展开；截图只是产物，不靠模型解释。

---

## S6-UI-01B：系统返回与旋转状态保存

**目标**：Android back、页面层级和横竖屏旋转不丢失进行中输入或扫描上下文。

**精确范围**：`apps/client/src/hooks/useAppState.ts`、导航组件、可新建 `apps/client/src/hooks/useSystemBack.ts`、对应测试。不得改 CSS 体系、native service 或数据库。

**执行**：back 先关闭弹层/详情，再回上层；扫描中退出需确认；旋转后保留当前书、规则草稿、滚动锚点和进行中状态；不得重复发起命令。

**测试/门槛**：组件测试模拟 back 优先级、取消/确认、旋转 remount、重复 event 和扫描中状态。

---

## S6-UI-01C：软键盘、大字体与可访问性

**目标**：键盘不挡输入，200% 字体和键盘/读屏操作仍能完成主流程。

**精确范围**：输入/弹层/导航相关 React/CSS、a11y 测试配置和测试；不得改业务服务。

**执行**：使用动态 viewport/safe-area；焦点可见且顺序合理；弹层有 focus trap/restore；状态不用只靠颜色；label、role、live region、heading 完整。

**测试/门槛**：DOM/ARIA、tab 顺序、Escape/back、200% 字体几何、软键盘 viewport 模拟和 axe（若仓库已有或锁定引入）无 P0/P1。

---

## S6-UI-01D：响应式与系统交互真机门

**目标**：真实 Android 设备复核响应式、返回、旋转、软键盘和大字体。

**精确范围**：只允许 `docs/evidence/android/ui/**` 和台账；不在 gate 内改代码。

**执行/门槛**：人工按固定脚本逐项操作；结构化记录 viewport、字体倍率、焦点/操作结果。自动截图/视频仅留证，DeepSeek 不解释。无设备时 ledger 写 `HUMAN_PENDING`，登记 `HG-003` 的 UI 子场景。

---

## S6-UX-01A：Codex 式非程序员主流程

**目标**：用户无需理解工程术语即可沿“导入小说 → 选关注点 → 开始扫描 → 查看来源”完成任务。

**精确范围**：`apps/client/src/App.tsx`、`apps/client/src/components/Workspace.*`、`ImportPanel.*`、`RuleSelector.*`、`ScanProgress.*`、`EvidencePanel.*`、导航组件和相应 tests。不得改后端逻辑、provider 协议或数据库。

**执行**：主界面一次只强调当前一步；完成后给清晰下一步；保留随时返回和继续；结果卡先写“发现了什么/为什么值得注意/在哪一章”，再提供详情；不要模仿聊天人格或伪装人工判断。

**测试/门槛**：DOM 测试从空库贯通四步，验证主 CTA 唯一、返回不丢状态、来源可到章节；390/800/1440 几何断言通过。

---

## S6-UX-01B：自然语言状态、错误与渐进披露

**目标**：默认界面隐藏程序员概念，只在“高级设置”显式展开后显示必要的 provider 配置。

**精确范围**：`apps/client/src/components/SettingsPanel.*`、错误/状态展示组件、可新建 `apps/client/src/services/errorMessages.ts`、相关 tests。不得改后端错误类型，除非缺稳定错误码导致无法映射；此时停止并登记阻断，不解析任意错误字符串。

**执行**：

1. 默认不显示内部 ID、JSON、endpoint、base URL、URI、路径、credential ref、HTTP 状态、stack 或 token 术语。
2. Provider、模型和 endpoint 等放入“高级设置”；API 密钥用“模型服务密钥”并附费用/数据出站说明。高级区可折叠且默认关闭。
3. 为导入空状态、无规则、扫描未开始、扫描中、暂停、断网、限流、余额不足、服务错误、恢复不匹配和无结果写自然语言解释及下一步。
4. 进度、时间、费用只显示真实测量值；未知写“暂时无法估算”，禁止假百分比、假倒计时、假成功。

**测试/门槛**：表驱动错误映射；DOM 搜索禁止词在默认视图零出现；高级展开后只出现允许字段；所有错误有可执行下一步且不泄漏 raw payload。

---

## S6-UX-01C：非视觉模型可执行的 UX 验收

**目标**：把“人性化”转成自动数值与 DOM 合同，避免要求 DeepSeek 看图。

**精确范围**：`apps/client/src/__tests__/ux-flow.test.tsx`、`apps/client/e2e/ux-flow.spec.ts`、Playwright 配置/固定 fixture、必要 package script 和 `docs/evidence/ux/README.md`。不得修改产品组件。

**执行**：固定测试 provider/fixture；记录 DOM 角色/文本、主 CTA 数、tab 顺序、几何边界、横向溢出、ARIA live 更新、raw 技术词泄漏；如采用 pixel diff，只由工具与已审定基线给数值结论。

**测试/门槛**：390/800/1440 全流程 Playwright 通过；0 个遮挡主要操作、0 个默认 raw ID/JSON/endpoint/URI 泄漏、0 个无 label 关键控件；截图/视频自动上传但 DeepSeek 报告只引用测试数值/人工签字。

---

## S6-E2E-01：Windows 与 Android 自动 E2E

**目标**：自动验证双平台主链，不消耗真实模型额度。

**精确范围**：`apps/client/e2e/**`、`apps/desktop/tests/e2e/**`、确定性 dev test provider 模块、测试配置和 CI。不得把 test provider 注册到 release feature/channel。

**执行**：覆盖导入、规则选择、扫描、证据章节、暂停/恢复、取消、错误恢复；QA/debug channel 可用 deterministic dev test provider。增加 release 构建审计，断言 provider ID、命令、菜单、fixture、字符串和 feature 均不存在。

**测试/门槛**：Windows 自动 E2E 和 Android emulator/instrumentation 可运行部分通过；真机专属门引用 02D/03D/04D/UI01D。真实 provider 不进入自动门，只登记 `HG-001`：用户任选一种 provider 做最小请求/扫描 smoke，缺 key 时 `HUMAN_PENDING` 而非阻塞独立工程。视频/截图只自动留证，不由模型解释。

---

## S6-BUILD-01：构建、Android 必需签名与 Windows 可选签名

**目标**：产出可审计 Windows 安装包和 Android release APK，不泄露签名材料。

**精确范围**：`.github/workflows/*build*.yml`、Android Gradle signing 配置（仅环境变量/secret 引用）、Tauri bundle 配置、构建脚本和 `docs/ANDROID_BUILD.md`、`docs/WINDOWS_INSTALL.md`。不得提交任何 key、证书或密码。

**执行**：

1. Android release 必须由用户在仓库外创建并保管的 signing keystore 签名；登记 `HG-004A`，需要 keystore 的 base64/文件 secret、alias 和密码 secrets，但绝不打印。
2. CI 只在受保护 release job 读取 signing secrets；PR/fork/debug 无权读取。验证 APK 签名、package ID、versionCode/versionName 和 SHA-256。
3. Windows 商业证书为可选 `HG-004W`。有证书则验证 Authenticode；没有则产出明确命名/文档化的未签名 GitHub 侧载资产，说明 SmartScreen 和校验 SHA-256 的步骤。
4. release 构建必须证明 dev test provider、debug 菜单、fixture、devtools、source map（若含敏感路径）和 secrets 不在资产中。

**测试/门槛**：Android 未签名或 debug-signed 资产不得作为 release；自建 signing keystore 缺失时报告可写阶段说明 `ENGINEERING_DONE`、ledger 写 `HUMAN_PENDING`。Windows 缺商业签名不阻止明确的未签名 GitHub 侧载发布，但必须公开限制。

---

## S6-GATE-01：双平台产品化总验收

**目标**：汇总工程结果与人工门，不让单一人工资源缺失掩盖其他完成情况。

**精确范围**：只运行门禁、修复本阶段 P0/P1、更新 evidence/文档/台账；不得新增功能。

**执行/门槛**：

1. 运行 frontend test/build、Rust fmt/check/test、migration/capability、Windows installed smoke、Android debug/release build、E2E、a11y、UX、资产敏感扫描。
2. release 资产中 dev test provider 必须为零；`HG-001` 只需用户任选一种真实 provider 最小 smoke。
3. 分别列出 `EB-002`、`HG-003` 的四个 Android 真机场景、`HG-001`、`HG-004A`、`HG-004W`。每项写全局 gate 状态、负责人、所需资源和下一步；task ledger 只使用统一枚举。
4. Android release 资格要求真机主链和自建 signing keystore 完成；Windows 商业签名可 pending，未签名侧载限制必须进入 README/Release notes。
5. 工程全绿但人工门未齐时，报告可写阶段说明 `ENGINEERING_DONE`，S6-GATE-01 ledger 写 `HUMAN_PENDING`，不得把独立工程连锁标 `BLOCKED`。
6. 视觉/E2E 证据只引用 DOM/ARIA/几何/pixel 自动结论或真实人工复核；截图/视频不由 DeepSeek解释。
