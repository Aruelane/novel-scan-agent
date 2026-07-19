# 03｜阻塞、人工门与非阻塞债务

DeepSeek 必须维护本文件，但不能用它掩盖核心功能未完成。任何 `BLOCKER` 都要包含解除条件；任何 `DEBT` 都要说明为何不影响当前阶段门禁。

## HUMAN_GATE

| ID | 阶段 | 需要用户提供/确认 | 安全要求 | 状态 |
| --- | --- | --- | --- | --- |
| HG-001 | S4 | 用户任选一个真实模型提供器做最小连接/结构化响应 smoke；第二协议真实账号可选，严格 fake 合同仍必须通过 | Key 只进入系统安全存储，不写聊天、文件、日志或 SQLite | OPEN |
| HG-002A | S5 | 贴吧来源可访问内容或用户导出的原文/截图文字，并由真实人工逐字核验 OCR/转写 | 不绕过登录/验证码；不能仅凭二手转述、搜索摘要或未复核 OCR 标 verified | OPEN |
| HG-002B | S5 | 另一名真实复核者交叉检查 32 条规则映射、默认开关和来源定位器 | DeepSeek/Codex 不能冒充第二复核者；只登记结论和材料哈希，不保存账号或整帖 | OPEN |
| HG-003 | S6 | Android 真机或可信模拟器环境、USB/调试授权 | 不提交本机 SDK 路径和设备标识 | OPEN |
| HG-004A | S6/S7 | Android 自建 release keystore 和签名口令 | 仅放 GitHub Encrypted Secrets/本机安全存储；不得提交仓库 | OPEN |
| HG-004W | S6/S7 | 是否提供 Windows 商业代码签名证书，或接受明确标注的未签名 GitHub 侧载资产 | 证书仅放安全存储；无证书时必须公开 SmartScreen/未知发行者限制，不得伪称已签名 | OPEN |
| HG-005 | S7 | 首发版本号、公开隐私说明和发布确认 | 发布前由用户确认支持范围与已知限制 | OPEN |

## EXTERNAL_BLOCKER

| ID | 任务 | 证据/错误 | 可继续的工作 | 解除条件 | 状态 |
| --- | --- | --- | --- | --- | --- |
| EB-001 | 本地 Rust 全量测试 | 普通终端缺少 MSVC `link.exe` | formatter、metadata、Node 测试、GitHub Windows CI | Windows runner 全绿或安装 VS Build Tools | OPEN |
| EB-002 | Windows 已安装 shell/打包冒烟 | 执行环境若不是可安装 Tauri 包的 Windows 主机，则无法生成本机安装证据 | 编写脚本、静态合同、CI 构建和 Android 独立工作 | 在 Windows 主机运行安装/升级/卸载脚本并保存脱敏证据 | OPEN |

## DEBT

| ID | 发现阶段 | 问题 | 为什么暂不阻塞 | 建议处理任务/阶段 | 状态 |
| --- | --- | --- | --- | --- | --- |
| DEBT-001 | S1 | GitHub Actions 对旧 action runtime 给出 Node 20 deprecation warning | 当前 action 被 runner 强制使用 Node 24 且 job 可运行，不影响代码正确性 | S7 发布工作流更新时评估新版 action | OPEN |

## 新增记录模板

```text
ID:
类型：HUMAN_GATE / EXTERNAL_BLOCKER / DEBT
首次发现任务：
证据（命令、日志、文件与行号）：
影响范围：
已尝试的安全方案：
为什么不能在当前任务解决：
解除条件：
负责人：DeepSeek / 用户 / Codex 终审
状态：OPEN / RESOLVED / ACCEPTED_FOR_RELEASE
```

## 处理规则

- 安全、数据丢失、密钥泄露、证据错误、数据库不兼容、核心闭环不可用不得降级为 DEBT。
- `RESOLVED` 时追加解决提交和验证证据，不删除历史记录。
- `ACCEPTED_FOR_RELEASE` 只能由用户在 S7 明确接受，DeepSeek 不能自行决定。
- 人工门打开时，相关 task ledger 使用 `HUMAN_PENDING`；只有它使工程本身也无法继续时才用 `BLOCKED`。打开的人工门不会自动阻塞其他独立任务。
