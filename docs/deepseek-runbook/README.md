# DeepSeek 项目完成 Runbook

入口位于仓库根目录 [`DEEPSEEK_START_HERE.md`](../../DEEPSEEK_START_HERE.md)。

本目录的目标是让能力和上下文较弱的模型也能稳定推进：任务按依赖排序、一次只改有限文件、每步有可执行验收、状态落盘，并把必须由用户处理的凭据/设备/签名问题与普通开发分离。

## 阅读规则

每个会话只读取：

1. `00_EXECUTION_CONTRACT.md`
2. `01_CURRENT_STATE.md`
3. `02_TASK_LEDGER.md`
4. 当前任务所在阶段文件中的当前任务
5. 当前任务明确引用的少量源码/文档

不要一次读取全部阶段文件。

## 阶段文件

| 顺序 | 文件 | 主题 |
| --- | --- | --- |
| 00 | `00_EXECUTION_CONTRACT.md` | 永久安全/Git/测试规则 |
| 01 | `01_CURRENT_STATE.md` | 当前接续事实 |
| 02 | `02_TASK_LEDGER.md` | 进度真相 |
| 03 | `03_BLOCKERS_AND_DEBT.md` | 人工门、外部阻塞、非阻塞债务 |
| 04 | `04_COMMANDS_AND_ENVIRONMENT.md` | 仓库工具链、测试与 CI 命令 |
| 10 | `10_S1_CLOSEOUT.md` | 关闭跨端基础骨架阶段 |
| 20 | `20_S2_MULTI_FORMAT_IMPORT.md` | 多格式导入和来源锚点 |
| 30 | `30_S3_SCAN_CONTEXT.md` | 全书扫描、压缩、证据和恢复 |
| 40 | `40_S4_MODEL_BYOK.md` | 多模型 API、BYOK 和秘密存储 |
| 50 | `50_S5_COMMUNITY_RULES.md` | 社区规则核验和用户定制 |
| 60 | `60_S6_WINDOWS_ANDROID.md` | Windows/Android 产品化 |
| 70 | `70_S7_RELEASE.md` | 安全、质量和 GitHub Release |
| 80 | `80_ACCEPTANCE_MATRIX.md` | 用户需求的最终证据矩阵 |
| 90 | `90_FINAL_HANDOFF.md` | 一次性 Codex 终审交接包 |

## 执行循环

```text
先恢复唯一 IN_PROGRESS
        ↓（没有）
选择第一个依赖已满足的 RETRY
        ↓（没有）
选择第一个依赖已满足的 TODO
        ↓
只读取该任务
        ↓
检查允许文件与前置依赖
        ↓
先写失败测试/夹具（适用时）
        ↓
最小实现
        ↓
聚焦测试 → 层级测试
        ↓
更新 ledger / blocker / acceptance
        ↓
checkpoint commit
        ↓
继续下一个任务
```

阶段末运行 stage gate。失败时在 ledger 顶部插入最小 `RETRY`，不得越过失败 gate 继续依赖它的阶段。

`HUMAN_PENDING` 只代表工程已经完成、仍缺用户凭据/设备/来源或发布决定。它不阻止无须该人工证据的后续工程；只有明确把该人工证据列为前置条件的任务才等待。没有任何可运行任务时才停止并汇总人工门。

## “可发布完成”的含义

不是所有 checkbox 被手工勾选，而是：

- 阶段 gate 有真实命令/CI/设备证据；
- `80_ACCEPTANCE_MATRIX.md` 的硬门全部为 `PASS`；
- 人工门有用户结论或被明确标为未完成，未被伪装成成功；
- 最终交接包可以让 Codex 不重跑整个开发过程，只审查高风险 diff、失败记录和发布证据。

若工程任务已穷尽但仍有 `HUMAN_PENDING`，可以生成诚实的交接包并停下等待人工输入；这叫“工程交接就绪”，不叫“可发布完成”。
