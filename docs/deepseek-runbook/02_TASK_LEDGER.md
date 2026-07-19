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
| S1-01 | B07-FIX：未核验规则错误消息 | AWAITING_CI | 6ac840e; fmt ok; link.exe EB-001 |
| S1-02 | Rulepack 拒绝重复规则 ID | AWAITING_CI | af2f281; fmt ok; link.exe EB-001 |
| S1-03 | LoadedRule 保留运行时元数据 | AWAITING_CI | bd71539; fmt ok; link.exe EB-001 |
| S1-04 | 完整检测语义进入 RuleDefinition | IN_PROGRESS | started |
| S1-05 | 规则语义进入 provider 与恢复指纹 | TODO | |
| S1-06 | ImportedDocument 到 NovelDocument checked adapter | TODO | |
| S1-07 | 桌面生产入口加载 seed rulepack | TODO | |
| S1-08 | 移除 WebView 原始 sourceRef | TODO | |
| S1-09 | failed 状态与预算单位一致 | TODO | |
| S1-10 | 规则选择键盘与焦点 | TODO | |
| S1-11 | Workspace tabs 无障碍合同 | TODO | |
| S1-12 | 颜色对比度自动门禁 | TODO | |
| S1-13 | 诚实导入占位与 favicon | TODO | |
| S1-14 | 三视口响应式浏览器验证 | TODO | |
| S1-15 | S1 文档、Tauri 与 CI 总门禁 | TODO | |

## S2｜多格式导入与来源定位

阶段文件：`20_S2_MULTI_FORMAT_IMPORT.md`

| ID | 任务 | 状态 | Commit / CI / 备注 |
| --- | --- | --- | --- |
| S2-01 | 能力状态、locator、限制和错误合同 | TODO | |
| S2-02 | 内容识别防伪与诚实矩阵 | TODO | |
| S2-03 | TXT UTF-8/16/GBK/GB18030 | TODO | |
| S2-04 | TXT 分章、换行和锚点 | TODO | |
| S2-05 | Markdown 独立解析器 | TODO | |
| S2-06 | 安全 ZIP/XML 基础 | TODO | |
| S2-07 | HTML 安全文本导入 | TODO | |
| S2-08 | EPUB container/OPF/spine | TODO | |
| S2-09 | EPUB 正文、章节和锚点 | TODO | |
| S2-10 | DOCX 正文、标题和段落锚点 | TODO | |
| S2-11 | 文本 PDF 与扫描版判定 | TODO | |
| S2-12 | Windows path / Android URI 读取合同 | TODO | |
| S2-13 | Tauri Windows 选择与导入命令 | TODO | |
| S2-14 | 前端真实导入流与能力状态 | TODO | |
| S2-15 | S2 格式矩阵和 CI 总门禁 | TODO | |

## S3｜全书扫描、上下文与恢复

阶段文件：`30_S3_SCAN_CONTEXT.md`

| ID | 任务 | 状态 | Commit / CI / 备注 |
| --- | --- | --- | --- |
| S3-01 | Checkpoint 记忆账本 schema | TODO | |
| S3-02 | Provider 记忆与未决更新合同 | TODO | |
| S3-03 | 严格有界 ContextView | TODO | |
| S3-04 | UTF-8 安全章节窗口 | TODO | |
| S3-05 | 多窗口、整章提交扫描器 | TODO | |
| S3-06 | 滚动摘要和三类账本合并 | TODO | |
| S3-07 | 未决 finding 状态转换 | TODO | |
| S3-08 | Provider-neutral 用量和预算 | TODO | |
| S3-09 | 暂停、取消与安全点 | TODO | |
| S3-10 | 恢复指纹与 schema 验证 | TODO | |
| S3-11 | 章节原子提交持久化合同 | TODO | |
| S3-12 | V2 扫描运行与 usage migration | TODO | |
| S3-13 | SQLite 原子 ScanPersistence | TODO | |
| S3-14 | 故障、重试与崩溃恢复矩阵 | TODO | |
| S3-15 | Tauri 扫描命令与事件桥 | TODO | |
| S3-16 | 前端真实任务进度和控制 | TODO | |
| S3-17 | 证据详情与来源章节回跳 | TODO | |
| S3-18 | 长书测试与 S3 总门禁 | TODO | |

## S4｜多模型 API 与 BYOK

阶段文件：`40_S4_MODEL_BYOK.md`

| ID | 任务 | 状态 | Commit / CI / 备注 |
| --- | --- | --- | --- |
| S4-01 | Provider 配置与注册表 | TODO | |
| S4-02 | 共享结构化输出 wire schema | TODO | |
| S4-03 | 统一且防提示注入的扫描 prompt | TODO | |
| S4-04 | HTTP 执行、脱敏、超时与取消 | TODO | |
| S4-05 | 确定性重试、限流和观测 | TODO | |
| S4-06 | OpenAI-compatible adapter | TODO | |
| S4-07 | DeepSeek 兼容端点模板 | TODO | |
| S4-08 | Anthropic native adapter | TODO | |
| S4-09 | 限制本地确定性测试 provider | TODO | |
| S4-10 | SecretStore 抽象与 canary | TODO | |
| S4-11 | Windows Credential Manager | TODO | |
| S4-12 | Android Keystore bridge 接口 | TODO | |
| S4-13 | Provider profile v3 migration | TODO | |
| S4-14 | Tauri profile/credential 命令 | TODO | |
| S4-15 | 安全连接测试与 adapter factory | TODO | |
| S4-16 | 设置 UI、连接测试与预算 | TODO | |
| S4-17 | 正文出站确认与按书授权 | TODO | |
| S4-18 | 密钥/正文泄漏与无 Key 离线门 | TODO | |
| S4-19 | 两个真实提供器人工合同验证 | TODO | HG-001 |
| S4-20 | S4 总门禁 | TODO | |

## S5｜社区规则和用户定制

阶段文件：`50_S5_COMMUNITY_RULES.md`

| ID | 任务 | 状态 | Commit / CI / 备注 |
| --- | --- | --- | --- |
| S5-RULE-01 | 来源台账 schema 与验证器 | TODO | |
| S5-RULE-02 | 公开来源采集与人工核验登记 | TODO | HG-002A/HG-002B |
| S5-RULE-03 | 从已核验证据生成版本化规则包 | TODO | |
| S5-RULE-04 | 规则不变量与只读概念聚合 | TODO | |
| S5-PRESET-01A | 预设领域模型与三层合并 | TODO | |
| S5-PRESET-01B | 预设与每书覆盖持久化 | TODO | |
| S5-PRESET-01C | 扫描选择快照与 Tauri 契约 | TODO | |
| S5-PRESET-02 | 规则与预设 UI | TODO | |
| S5-CUSTOM-01A | 自定义规则 schema 与安全解析 | TODO | |
| S5-CUSTOM-01B | 自定义规则持久化与 Tauri commands | TODO | |
| S5-CUSTOM-01C | 导入预览、导出与 UI 闭环 | TODO | |
| S5-UPGRADE-01A | 版本差异与迁移计划纯函数 | TODO | |
| S5-UPGRADE-01B | 事务化迁移应用与回滚 | TODO | |
| S5-UPGRADE-01C | 历史复现与升级确认 UI | TODO | |
| S5-GATE-01 | S5 工程门与人工门汇总 | TODO | |

## S6｜Windows 与 Android 产品化

阶段文件：`60_S6_WINDOWS_ANDROID.md`

| ID | 任务 | 状态 | Commit / CI / 备注 |
| --- | --- | --- | --- |
| S6-WIN-01 | 已安装 Windows shell 审计与冒烟 | TODO | EB-002 if unavailable |
| S6-WIN-02 | Windows 安装、升级与卸载 | TODO | |
| S6-AND-01 | 生成并审查 Tauri Android scaffold | TODO | |
| S6-AND-02A | SAF 原生合同与 Kotlin 实现 | TODO | |
| S6-AND-02B | SAF Rust 桥与 URI 生命周期 | TODO | |
| S6-AND-02C | Android 导入 UI | TODO | |
| S6-AND-02D | SAF 真机门 | TODO | HG-003 |
| S6-AND-03A | Android Keystore 原生 secret 实现 | TODO | |
| S6-AND-03B | Keystore Rust 桥 | TODO | |
| S6-AND-03C | Provider secret UI 接线 | TODO | |
| S6-AND-03D | Keystore 真机门 | TODO | HG-003 |
| S6-AND-04A | 前台服务与通知原生实现 | TODO | |
| S6-AND-04B | 长任务 Rust 生命周期桥 | TODO | |
| S6-AND-04C | 暂停、停止、恢复与通知权限 UI | TODO | |
| S6-AND-04D | 后台与进程终止真机门 | TODO | HG-003 |
| S6-UI-01A | 响应式布局 | TODO | |
| S6-UI-01B | 系统返回与旋转状态保存 | TODO | |
| S6-UI-01C | 软键盘、大字体与可访问性 | TODO | |
| S6-UI-01D | 响应式与系统交互真机门 | TODO | HG-003 |
| S6-UX-01A | Codex 式非程序员主流程 | TODO | |
| S6-UX-01B | 自然语言状态、错误与渐进披露 | TODO | |
| S6-UX-01C | 非视觉模型可执行的 UX 验收 | TODO | |
| S6-E2E-01 | Windows 与 Android 自动 E2E | TODO | |
| S6-BUILD-01 | 构建、Android 必需签名与 Windows 可选签名 | TODO | HG-004A/HG-004W |
| S6-GATE-01 | 双平台产品化总验收 | TODO | |

## S7｜质量、安全与发布

阶段文件：`70_S7_RELEASE.md`

| ID | 任务 | 状态 | Commit / CI / 备注 |
| --- | --- | --- | --- |
| S7-SEC-01A | 威胁模型、数据流与隐私边界 | TODO | |
| S7-SEC-01B | 删除、全量清理与脱敏诊断 | TODO | |
| S7-SEC-02 | 恶意文件、提示注入和脱敏 | TODO | |
| S7-PERF-01 | 性能、内存与用量预算 | TODO | |
| S7-A11Y-01 | Windows/Android 可访问性 | TODO | |
| S7-E2E-01 | 双平台 E2E 与崩溃恢复 | TODO | |
| S7-BUILD-01 | 可重复构建、依赖与资产审计 | TODO | |
| S7-REL-01 | 受保护 draft/RC、校验值与用户文档 | TODO | HG-004A/HG-004W/HG-005 |
| S7-GATE-01 | 发布候选工程门与人工门矩阵 | TODO | |

## 最终交接

阶段文件：`90_FINAL_HANDOFF.md`

| ID | 任务 | 状态 | Commit / CI / 备注 |
| --- | --- | --- | --- |
| FINAL-01 | 冻结 source commit 与证据清单 | TODO | |
| FINAL-02 | 生成机器交接包 | TODO | |
| FINAL-03 | DeepSeek 证据一致性校验 | TODO | |
| FINAL-TIEBA-01 | 贴吧逐字核验人工门 | TODO | HG-002A/HG-002B |
| FINAL-04 | 发送一次 Codex 终审请求并等待 | TODO | Codex review |

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
