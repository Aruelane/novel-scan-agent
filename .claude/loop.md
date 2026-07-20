# .claude/loop.md — 持续开发循环指令

## 每轮启动

1. 运行 `node docs/deepseek-runbook/validate-ledger.mjs`
2. 若 validator 失败，修复账本（仅限允许的控制面文件）→ commit → 推送 → 重新运行 validator
3. 若 validator 通过，读取 `02_TASK_LEDGER.md` 确定当前任务

## 任务选择（固定优先级）

1. 查找唯一的 `IN_PROGRESS` → 恢复执行
2. 查找第一个依赖已满足的 `RETRY` → 设为 `IN_PROGRESS` → 执行
3. 查找第一个依赖已满足的 `TODO` → 设为 `IN_PROGRESS` → 执行

## 执行循环

- 有 `IN_PROGRESS`/`RETRY`/`TODO`/`AWAITING_CI` → 立即开始实际开发
- 一次只实现一个原子任务
- 开始时把该任务设为 `IN_PROGRESS`
- 阅读对应阶段文件中的完整目标、范围、执行、测试和门槛
- 实现真实代码和端到端接线，不接受只有 model/type/stub/demo
- 运行聚焦测试和必要全量测试
- 只有全部门槛满足才设 `DONE`
- 更新 ledger、audit/blocker 证据
- commit 以任务 ID 开头
- 推送 `deepseek/full-build`
- 立即进入下一任务

## 停止条件

以下情况允许停止（其他情况必须继续）：

A. validator 证明全部 122 个任务状态真实一致，所有可自动完成任务均已完成，CI/构建/交接条件满足；或
B. 不存在任何可运行的 IN_PROGRESS/RETRY/TODO/AWAITING_CI，剩余任务均为已登记 HUMAN_PENDING/BLOCKED

## 禁止行为

- 不得只输出总结而不实际开发
- 不得询问"是否继续"
- 不得把单任务完成当成会话终点
- 不得输出 `PROJECT_COMPLETE` 除非满足停止条件 A 或 B
- 不得自行清空或篡改账本来满足停止条件

## 强制中断时

若平台/额度/网络/API 强制中断，最后一行必须严格输出：

```
CONTINUE_REQUIRED:<下一个任务ID>
```
