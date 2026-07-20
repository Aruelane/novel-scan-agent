# 小说扫评 Agent — 项目持久记忆

## 项目概述

跨平台（Windows + Android）小说扫评工具。用户导入小说文件，选择雷点/郁闷点规则，由 AI 模型逐章扫描标注，结果定位到章节和原文来源。非程序员友好的人性化界面。不做 iOS。

## 关键文件索引

- **执行合同**：`docs/deepseek-runbook/00_EXECUTION_CONTRACT.md`（永久规则）
- **任务账本**：`docs/deepseek-runbook/02_TASK_LEDGER.md`（唯一进度真相，122 原子任务）
- **阻塞/债务**：`docs/deepseek-runbook/03_BLOCKERS_AND_DEBT.md`
- **恢复审计**：`docs/deepseek-runbook/05_RECOVERY_AUDIT.md`
- **账本验证器**：`docs/deepseek-runbook/validate-ledger.mjs`（CI 门禁）
- **循环指令**：`.claude/loop.md`

## 核心技术栈

- **Rust** workspace：`novel-core`、`novel-import`、`novel-rulepack`、`novel-providers`、Tauri desktop
- **前端**：React/TypeScript (`apps/client`)
- **桌面**：Tauri 2 (`apps/desktop`)
- **规则包**：JSON Schema 2020-12 (`packages/rulepack`)
- **数据库**：SQLite
- **多格式**：TXT、Markdown、HTML、EPUB、DOCX、文本 PDF

## 122 个原子任务不可合并

每个任务 ID 是独立的。禁止任何形式的合并：`S2-08+S2-09`、`S4-17–S4-18`、`S6-AND-02A-D` 等均不合法。任务 ID 的字母后缀（如 `S6-AND-02A`、`S6-AND-02B`）各自独立。

## 状态规则

| 状态 | 何时使用 |
| --- | --- |
| `TODO` | 尚未开始 |
| `IN_PROGRESS` | 当前唯一正在执行（最多 1 个） |
| `RETRY` | 有代码但未通过门槛 |
| `AWAITING_CI` | 已提交，等 CI runner 验证（必须含 run URL） |
| `HUMAN_PENDING` | 工程完成，仅缺人工凭据/设备/签名（必须引用 HG-*） |
| `BLOCKED` | 工程也无法推进（必须引用 EB-*） |
| `DONE` | 全部门槛满足（必须含 commit SHA） |

## 证据要求

- `DONE` 必须含可解析 commit SHA 和关键验收证据
- `AWAITING_CI` 必须含真实 `github.com/.../actions/runs/...` URL
- `BLOCKED` 必须引用 `03_BLOCKERS_AND_DEBT.md` 中的 EB-*
- `HUMAN_PENDING` 必须引用 `03_BLOCKERS_AND_DEBT.md` 中的 HG-*
- 禁止使用 `SKIPPED_BY_USER` 绕过需求（只能由用户主动决定）

## 任务选择顺序

1. 恢复唯一的 `IN_PROGRESS`
2. 否则：第一个依赖已满足的 `RETRY`
3. 否则：第一个依赖已满足的 `TODO`
4. `AWAITING_CI`/`HUMAN_PENDING`/`BLOCKED` 不被选为执行目标

## 禁止伪完成

- 禁止将 stub/mock/demo data/disabled UI 标为 DONE
- 禁止存在类型/接口/组件就声称实现完成
- 禁止单元测试通过就声称 Tauri/前端/Android 闭环完成
- 禁止把测试 provider 冒充真实模型扫描
- 禁止以"需要设备"为由跳过可先完成的工程代码
- 禁止使用"应该可用""基本完成""核心已就绪"代替证据

## Git 规则

- 当前分支：`deepseek/full-build`
- 禁止 `git reset --hard`、`git checkout --`、`git clean -fd`、`push --force`、rebase
- 禁止合并 main
- 禁止创建正式 tag 或 Release
- commit 消息以任务 ID 开头
- 每任务一个 commit；完成一个任务后推送

## 需求底线（不可缩减）

- 支持多模型 API/BYOK：至少 OpenAI-compatible 通用、DeepSeek 模板、Anthropic native（三种协议）
- 不能擅自缩减为仅 DeepSeek
- 多格式导入：TXT/Markdown/HTML/EPUB/DOCX/文本 PDF
- Windows + Android 双平台

## 完成一项后自动继续下一项

每次完成任务并提交后，按任务选择顺序立即进入下一个任务，不询问是否继续。完成单一任务不是停止条件。只有 validator 证明全部 122 个任务状态真实一致且无可执行任务时才停止。
