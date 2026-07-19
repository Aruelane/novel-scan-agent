# DeepSeek 全项目执行入口

这套手册用于让 DeepSeek 在较少人工复审的情况下，从当前代码继续完成小说扫评 Agent。它不是一条要求模型“一次写完整项目”的巨型提示词，而是一组按依赖排序、可逐项验证和恢复的任务。

## 用户只需要发送这一段

把下面内容原样发给 DeepSeek/Claude（工作目录必须是本仓库）：

```text
你现在接管“小说扫评 Agent”的后续开发。

先完整读取：
1. DEEPSEEK_START_HERE.md
2. docs/deepseek-runbook/00_EXECUTION_CONTRACT.md
3. docs/deepseek-runbook/01_CURRENT_STATE.md
4. docs/deepseek-runbook/02_TASK_LEDGER.md
5. docs/deepseek-runbook/03_BLOCKERS_AND_DEBT.md
6. 当前任务需要运行命令时再读取 docs/deepseek-runbook/04_COMMANDS_AND_ENVIRONMENT.md

然后按以下顺序选择任务：先恢复唯一的 IN_PROGRESS；没有时选择第一个依赖已满足的 RETRY；再没有时选择第一个依赖已满足的 TODO。若出现多个 IN_PROGRESS，先停止修改并修复账本一致性。每次只读取当前任务所在阶段文件中的那一个任务，不要一次把全部阶段文件塞进上下文。

严格遵守 00_EXECUTION_CONTRACT.md。一个任务完成并验证后，更新 TASK_LEDGER 和必要的阻塞/债务记录，做独立 Git checkpoint，再继续下一个任务。遇到 HUMAN_GATE、真实凭据/签名/设备缺失时，只把对应路径标为 HUMAN_PENDING 或 BLOCKED，并继续执行其他依赖已满足的任务；只有不存在任何可运行任务，或用户明确说暂停时，才停止。

不得删除或弱化测试，不得用演示/Mock 冒充真实产品能力，不得向 main 强制推送，不得提交 API Key、签名密钥、用户小说或本机路径。

当全部可运行工程任务完成、剩余仅为已登记人工门，或全部任务均完成时，按照 docs/deepseek-runbook/90_FINAL_HANDOFF.md 生成最终交接包。不得把未解除人工门写成 PASS；准备好一次性终审材料后停止，等待 Codex。
```

## 文件顺序

| 文件 | 用途 |
| --- | --- |
| `00_EXECUTION_CONTRACT.md` | 永久生效的安全、Git、测试和报告规则 |
| `01_CURRENT_STATE.md` | 代码基线、已完成内容和第一个待修问题 |
| `02_TASK_LEDGER.md` | 唯一进度真相；每个任务完成后必须更新 |
| `03_BLOCKERS_AND_DEBT.md` | 不阻塞主线的小问题、外部阻塞和人工门 |
| `04_COMMANDS_AND_ENVIRONMENT.md` | 仓库内置工具链、测试和 CI 命令 |
| `10_...` 至 `70_...` | S1 至 S7 的具体实现任务 |
| `80_ACCEPTANCE_MATRIX.md` | 用户需求与最终证据的对应关系 |
| `90_FINAL_HANDOFF.md` | 全部完成后交给 Codex 的一次性审查材料 |

## 节省模型额度的原则

- 每个新会话只加载执行合同、状态表和当前一个任务。
- 不反复总结整个仓库；把可靠状态写回 Markdown。
- 小问题若不影响安全、数据正确性、构建或核心闭环，记入债务表后继续。
- 每个阶段只在门禁处跑一次完整 CI；任务内先跑最小相关测试。
- 没有图片多模态能力不构成阻塞：UI 使用 DOM、键盘、ARIA、对比度计算、响应式尺寸和自动化测试验收，不以“看截图感觉不错”作为门槛。

## 产品边界

- 首发 Windows 和 Android；不实现 iOS。
- 成品调用在线模型采用用户自带 API 凭据（BYOK）。ChatGPT Plus/Codex 订阅不等于 OpenAI API 额度。
- 文件支持不能只写 TXT；必须按手册逐步覆盖多格式，并诚实区分可解析、受限、需 OCR/转换和不支持。
- 已确认命中必须能回到书中章节和原文；模型摘要不能充当证据。
- 社区规则必须保留来源和核验状态，不能臆造贴吧内容。
