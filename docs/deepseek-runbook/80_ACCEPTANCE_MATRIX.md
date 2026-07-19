# 80｜最终需求—证据矩阵

每个阶段 gate 完成后填写“实现证据”和“验证证据”。没有可点击提交、测试、CI、安装或设备证据的条目不能标 `PASS`。

| 用户需求 | 主要阶段 | 最终完成定义 | 实现证据 | 验证证据 | 状态 |
| --- | --- | --- | --- | --- | --- |
| 可接入多种模型 API | S4 | 统一 provider 契约；OpenAI-compatible + 至少一个不同原生协议；DeepSeek 兼容端点；可选模型/预算/超时；两类协议均有严格 fake 合同，用户任选一种做真实最小 smoke | 待填 | 待填 | TODO |
| 人性化、非程序员味界面 | S1/S6 | 导入—选规则—扫描—看证据流程清晰；错误可恢复；键盘、读屏、小屏可用 | 待填 | 待填 | TODO |
| 基于 yy小说吧的雷点/郁闷点 | S5 | 11+21 条逐项来源/判据/排除/待确认/版本；无来源不默认启用 | 待填 | 待填 | TODO |
| 用户自主选择规则与等级 | S1/S5 | 全局/每书预设、开关、等级覆盖、历史快照和升级差异 | 待填 | 待填 | TODO |
| 不限于 TXT 的多格式导入 | S2 | TXT/MD/HTML/EPUB/DOCX/文本 PDF 可解析回证；其他格式诚实分级 | 待填 | 待填 | TODO |
| 自动压缩长上下文 | S3 | 固定预算、滚动摘要、实体关系事件账本、未决候选、长书不线性扩张请求 | 待填 | 待填 | TODO |
| 命中标注原书章节和来源 | S2/S3 | confirmed/pending 的证据从原文重建并带格式相关 locator；摘要不能当证据 | 待填 | 待填 | TODO |
| 暂停、继续和崩溃恢复 | S3 | 每章事务、checkpoint、取消/重试/进程恢复、源文变化失效 | 待填 | 待填 | TODO |
| Windows 客户端 | S6/S7 | 安装、升级、文件选择、扫描闭环和发布资产；无商业代码签名时明确标注未签名侧载及校验值，不伪装签名状态 | 待填 | 待填 | TODO |
| Android 客户端 | S6/S7 | SAF、持久授权、前台长任务、Keystore、真机闭环、签名 APK | 待填 | 待填 | TODO |
| 不做 iOS | 全程 | 仓库无 iOS target、证书、workflow 或商店依赖 | 待填 | `rg`/构建清单 | TODO |
| BYOK 且密钥安全 | S4/S7 | Key 仅经 Windows/Android 安全存储；前端/SQLite/日志/Git 无明文 | 待填 | secret scan + 平台测试 | TODO |
| GitHub 可重复开发与发布 | S1/S7 | CI 全绿、阶段 checkpoint、Windows/Android 构建、Release/校验值/说明 | 待填 | 待填 | TODO |

## 状态定义

- `TODO`：未实现或未验证。
- `PARTIAL`：部分实现，但尚未满足完整定义。
- `BLOCKED`：实现完成度受已登记 HUMAN_GATE/EXTERNAL_BLOCKER 限制。
- `PASS`：实现与验证证据齐全，且没有相反的已知事实。
- `ACCEPTED_LIMITATION`：用户在最终发布门明确接受，必须链接到 debt/blocker 记录。

## 发布硬门

以下条目不允许使用 `ACCEPTED_LIMITATION` 绕过：

- 明文密钥或用户小说进入 Git/日志/SQLite。
- confirmed finding 无法从原文重建证据。
- 宣称支持的文件格式不能解析或不能定位来源。
- Android 发布资产无法安装或没有可验证的自建签名；Windows 资产无法安装、校验值不匹配，或没有诚实披露 signed/unsigned 状态。Windows 商业代码签名不是硬门，可由用户明确接受未签名 GitHub 侧载限制。
- CI 核心测试失败。
