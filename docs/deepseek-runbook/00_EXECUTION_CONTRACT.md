# 00｜DeepSeek 永久执行合同

本文件对 S1–S7 全部任务持续生效。阶段文件与本文件冲突时，以更安全、更严格且不扩大任务范围的要求为准；产品需求冲突时停止并记录 `HUMAN_GATE`，不得自行改变产品方向。

## 1. 每个会话的固定启动顺序

1. 运行 `git status -sb`、`git branch --show-current`、`git log -1 --oneline`。
2. 读取本文件、`01_CURRENT_STATE.md`、`02_TASK_LEDGER.md` 和当前任务。
3. 若工作区已有修改，先根据 ledger 判断是否属于上次未完成任务；不能确认来源时不得覆盖，记录阻塞。
4. 任务选择优先级固定为：恢复唯一的 `IN_PROGRESS`；否则选择第一个依赖已满足的 `RETRY`；否则选择第一个依赖已满足的 `TODO`。出现多个 `IN_PROGRESS` 时不得继续改代码，先根据 Git diff/历史修复账本一致性。
5. `AWAITING_CI`、`HUMAN_PENDING`、`BLOCKED` 不会被当作新任务重新执行。它们的依赖语义见第 4、7、10 节；没有任何可运行任务时才停止并汇总未解除条件。
6. 当前任务完成后更新 ledger、债务/阻塞表和必要文档，再提交 checkpoint。不得同时展开两个会修改相同业务文件的任务。

## 2. 上下文与额度控制

- 一次只读取当前任务直接引用的代码和文档，优先使用 `rg`/`rg --files` 定位。
- 不要每次重新读取全部 runbook、全部源码、全部 Git 历史或完整 CI 日志。
- CI 日志先用 job/失败步骤过滤，只保留失败测试名、文件、行号和关键错误。
- 不生成大段重复解释；实现报告使用任务规定的固定字段。
- 阶段门禁之前只运行相关 package/crate 的测试；门禁时才运行 workspace/full build。

## 3. Git 工作流

- 在专用分支工作，推荐 `deepseek/full-build`。不得在不确认远端状态时直接改写 `main`。
- 禁止 `git reset --hard`、`git clean -fd`、强制 checkout 覆盖用户文件、`push --force`。
- 一个完成任务通常对应一个小实现提交，消息统一以任务 ID 开头：`<task-id> <type>(<scope>): <summary>`。
- 只暂存当前任务文件，禁止无判断地 `git add -A`。
- 每次提交前必须确认 `git diff --check`、相关测试和 `git status --short`。
- 可把专用分支推送到 GitHub 作为备份。阶段门禁使用 `workflow_dispatch` 在该分支运行 CI，或在用户同意后维护一个 draft PR；不得为了绿灯删除测试。
- 若 GitHub/`gh` 未认证，记录 `EXTERNAL_BLOCKER`，继续能在本地安全完成的任务；不得索要或打印令牌。

### 控制面文件和 checkpoint 例外

- 阶段任务中的“严格允许文件/范围”描述业务代码范围。以下控制面文件是隐式允许，不算越界：`02_TASK_LEDGER.md` 每任务必须按事实更新；`03_BLOCKERS_AND_DEBT.md` 仅在 blocker/debt/human gate 事实变化时更新；`80_ACCEPTANCE_MATRIX.md` 仅在阶段 gate 或最终证据任务中更新。
- 不得借“控制面”名义修改其他 runbook 任务正文、产品需求或业务源码。阶段任务明确要求生成的证据/报告文件仍按该任务范围处理。
- Git commit 不能在自身内容中写入自己的 SHA。实现提交前可在 ledger 写 `checkpoint=self; subject=<精确提交标题>`；提交后的真实 SHA 写入任务报告，并可在下一次控制面提交或最终交接生成时回填/推导。
- 正常任务保持一个实现 checkpoint。若本机缺少任务硬性要求的 runner/toolchain，允许先提交实现并把状态设为 `AWAITING_CI`，再使用 GitHub runner 验证；CI 结果通过后可做一个只含 ledger/evidence 的 `chore(runbook)` 控制面提交。不要为了追求“一任务一提交”伪造本地通过。

## 4. 允许修复与停止边界

当前任务内允许：

- 修复由本任务直接导致的编译、测试、类型和格式错误。
- 为需求增加聚焦测试、夹具和最小文档。
- 对当前任务明确列出的公共类型传播必要的调用点修改。

必须停止当前任务并记录的情况：

- 需要改变架构方向、数据兼容策略、用户隐私承诺或产品范围。
- 需要真实 API Key、付费账户、发布签名、Android 真机、社区登录或用户选择。
- 发现可能删除/覆盖用户数据、破坏数据库兼容或泄露正文/密钥的风险。
- 实际需要修改的文件显著超过任务列出的范围。

停止“当前任务”不等于停止整个项目：

- 工程实现和可执行的聚焦检查已经完成、仅缺真实凭据、贴吧原文人工核验、设备或发布决定时，将任务记为 `HUMAN_PENDING`，登记对应 `HG-*`，并继续其他依赖已满足的任务。
- 连工程实现也无法安全推进时才记为 `BLOCKED`。后续不依赖该缺口的任务仍可继续。
- `HUMAN_PENDING` 可满足普通“工程已实现”依赖；只有任务明确写出“需要 HG-* 的真实证据”时才不能越过。`BLOCKED` 不满足依赖。

不阻塞主线的小问题：

- 纯文案润色、轻微视觉差异、非关键重复代码、非发布路径的性能微优化。
- 将其写入 `03_BLOCKERS_AND_DEBT.md` 的 `DEBT` 表，注明证据和建议阶段，然后继续。

## 5. 禁止伪完成

- 禁止把 mock provider、固定演示数据或空函数称为真实模型扫描。
- 禁止因扩展名可识别就称格式“已支持”；必须完成解析、损坏/超限测试和来源回证。
- 禁止把模型返回的引用直接保存为精确证据；必须从已导入原文重建。
- 禁止用 `skip`、`ignore`、`continue-on-error`、删除断言或放宽 schema 制造绿灯。
- 禁止在没有 Windows/Android 证据时声称安装包、APK、SAF、Keystore 或长任务已通过。
- 禁止臆造贴吧原文、社区共识、访问日期、来源摘录或规则核验结果。

## 6. 安全与隐私硬规则

- 永不提交：API Key、token、keystore、签名密码、用户小说、真实本机路径、持久 SAF URI、完整模型请求/响应正文。
- 日志和测试失败输出默认不含整章正文；夹具必须短小、原创或明确可再分发。
- 前端不能持有文件系统路径、content URI 或明文密钥；只使用不透明 ID 和安全显示名。
- 小说、HTML/XML、归档、规则文本和模型响应全部按不可信输入处理。
- 不绕过 DRM、登录、验证码、付费墙或平台访问控制。
- 无图片多模态能力时不得猜测视觉结果；使用可计算的布局、颜色对比度、DOM、ARIA 和键盘测试。

## 7. 测试层级

每个任务按以下顺序执行，前一级失败先修复或记录：

1. 格式/静态检查：`git diff --check`、语言 formatter、schema/YAML 校验。
2. 当前 package/crate 的聚焦测试。
3. 当前层的全部测试和生产构建。
4. 阶段门禁：workspace 测试、前端构建、规则包、migration/capability、Windows CI。
5. 涉及 Android/安装/真实 API 时执行任务规定的设备或外部合同测试。

本机若缺少 `link.exe`、Android SDK/NDK 或签名设备，必须保留真实错误并使用 GitHub runner/人工门验证，不能声称通过。

若聚焦检查已通过但唯一缺口是计划内的 CI runner，任务设为 `AWAITING_CI`。该状态可供同阶段后续工程继续，但不能通过阶段 gate；CI 失败时把最小修复项设为 `RETRY`。`PARTIAL` 只用于 `80_ACCEPTANCE_MATRIX.md` 和最终证据，不是 task ledger 状态。

## 8. 数据库与公共合同

- SQLite 迁移一旦进入已发布分支，不回写旧 migration 来改变已发布用户数据库；新增编号 migration 并测试升级。当前尚未发布前若任务允许修改初始 migration，也必须记录原因。
- 公共序列化字段、rule version、provider/model identity、文档指纹和 source locator 变更必须进入恢复/迁移指纹或有明确兼容策略。
- 历史 finding、规则包版本和证据锚点不可被新规则静默改写。

## 9. 每任务固定报告

完成任务后写入 ledger，并输出：

1. 任务 ID 与真实 ledger 状态：`DONE`、`IN_PROGRESS`、`RETRY`、`AWAITING_CI`、`HUMAN_PENDING` 或 `BLOCKED`；不得用 acceptance 的 `PARTIAL` 代替 ledger 状态。
2. 修改文件清单。
3. 行为变化和明确未实现内容。
4. 新增/修改测试名。
5. 每条验证命令的真实退出码与通过数。
6. 环境阻塞及其发生阶段。
7. `git diff --stat`、`git status --short`。
8. checkpoint commit SHA；若未提交，说明原因。
9. 下一任务 ID。

不得只写“全部完成”“应该能工作”或没有命令证据的结论。

## 10. 阶段门禁

- 阶段内任务完成不等于阶段完成。
- 必须执行阶段文件末尾的 gate，保存 CI/run/build/device 证据，并更新 `80_ACCEPTANCE_MATRIX.md`。
- gate 失败时生成最小 `RETRY` 任务置于 ledger 顶部；不要继续堆叠依赖该 gate 的阶段。
- 工程 gate 与人工验收分开记录：工程检查全绿但人工证据未提供时，任务/阶段记为 `HUMAN_PENDING`，acceptance 项记为 `PARTIAL`，不得伪装成最终 `PASS`。
- `HUMAN_GATE` 只暂停需要人的那条路径；能够独立推进的文档、单元测试、其他平台或发布准备任务继续。只有没有任何依赖已满足的 `RETRY`/`TODO` 时才停止等待用户。
