# YY小说吧规则包（种子版）

这个目录只负责“规则是什么、从哪里来、用户怎样选择”，不负责模型调用或小说解析。

当前包是 `2026.0.0-seed.1`：数据结构可用，但不是已经核验完成的社区吧规抄本。用户指定的贴吧页面目前未能稳定取得完整正文/图片，因此除了已观察到的“接盘”跨雷点档与郁闷点档现象，其他名称均明确写成“产品候选”；最后一个槽位保留为纯占位。所有未核验条目默认关闭。

## 文件

- `schemas/rulepack.schema.json`：版本化规则包的 JSON Schema。
- `schemas/rule-selection.schema.json`：用户逐条开关、严重度覆写和个人边界的 JSON Schema。
- `packs/yy-novel-bar/2026.0.0-seed.1.json`：11 类雷点、21 类郁闷点的首个种子包。
- `examples/rule-selection.example.json`：用户选择示例。
- `scripts/validate.mjs`：使用 Ajv 2020-12 进行 JSON Schema 校验，并执行跨字段业务不变量检查。
- `scripts/validate-negative.mjs`：针对非法 category、缺少 version、非法 scope 等的 Schema 负例测试。

运行校验：

```powershell
cd packages/rulepack
npm ci
npm run validate
npm run validate:negative
```

Windows 环境可运行 PowerShell 封装（内部调用 Node 验证器）：

```powershell
powershell -ExecutionPolicy Bypass -File packages/rulepack/scripts/validate.ps1
```

## 关键语义

`category` 与 `severity` 是两回事：

- `landmine` / `frustration` 表示社区分档。
- `critical` 到 `info` 表示当前用户的提示强度；用户可覆写。

同一概念可跨档。当前”接盘”有两个不同 `rule.id`，共享 `conceptId = relationship.accepting-prior-partner`。跨分类去重策略（`duplicateConceptPolicy`）当前为预留字段，S1 尚未在核心扫描器中实现 conceptId 去重或 `matchedRuleIds` 聚合。

发现状态必须区分：

- `suspected`：只有局部线索，不能下结论。
- `pending_confirmation`：需要后文、角色身份或关系事实确认。
- `confirmed`：已回查原文，且有章节及段落、页码或行号定位。
- `rejected`：线索被后文反转或排除条件否定。

`confirmed` 不是模型的“高置信度”同义词。没有可回到导入文件的原文定位，就不能确认。

## 用户覆盖合并顺序

1. 读取指定 `packId@packVersion` 的不可变规则包。
2. 未核验规则保持默认关闭。
3. 按 `ruleId` 合并用户的 `enabled`、`severity`、`sensitivity` 和 `customBoundary`。
4. 扫描时按 `conceptId` 聚合候选；按用户的 `duplicateConceptPolicy` 去重或分别显示。
5. 报告保存使用过的规则包版本和选择快照，确保未来规则升级后仍可复现旧结果。

## 来源核验流程

把候选升级为正式规则时，应同时完成：

1. 核对原帖中的准确名称、定义、例外和分档，不靠搜索摘要或转述补全。
2. 记录帖子 URL、楼层/图片序号、发布日期与访问日期。
3. 使用短摘录或释义保存判据，避免复制大段社区内容。
4. 为容易误判的传闻、梦境、假死、伪装、强迫、角色误认等情况补齐排除条件。
5. 将 `nameStatus`、`status` 与 `provenance.verification` 一起升级，并发布新版本；不要原地改写已经发布的包。

版本遵循 SemVer 风格：字段不兼容变更升级主版本，新增兼容字段或规则升级次版本，仅修正文案/来源升级补丁版本；未核验阶段使用 `-seed.N` 预发布标记。
