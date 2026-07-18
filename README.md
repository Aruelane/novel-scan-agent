# 小说扫评 Agent

一个面向普通网络小说读者的跨端扫书工具：导入自己的小说文件，选择个人在意的雷点与郁闷点，由模型逐章扫描，并把每条结论锚定回原书章节和原文位置。

项目处于第一阶段基础骨架开发中，目标平台是 Windows 与 Android；不规划 iOS。

## 第一阶段能力

- React + TypeScript 的自然化三栏界面，并提供 Android 单栏/底部导航布局。
- 平台无关的 Rust 扫描核心：模型适配接口、逐章 checkpoint、恢复校验、上下文压缩和严格证据回切。
- 多格式导入注册表。TXT 与 Markdown 已有可运行解析器；EPUB、PDF、DOCX、HTML、MOBI/AZW3、ZIP/7Z 已登记但仍明确标为接入中；旧版 DOC 暂不支持。
- 版本化社区规则包：当前种子包保留 11 类雷点与 21 类郁闷点结构，未核验名称全部默认关闭，不冒充贴吧原文。
- Tauri 2 Windows/Android 共享壳与 SQLite 初始迁移已经落盘；主 CI 和手动 Android debug workflow 已配置。Android workflow 仍以首次远端成功运行结果为准，当前不宣称已有可运行或可发布 APK。

## 关于模型与 Key

写代码和运行本地测试不需要模型 API Key。成品应用真正调用在线模型时，用户再自行选择 OpenAI-compatible、Anthropic、Gemini、DeepSeek 或本地模型，并把凭据交给原生安全存储；前端不接触密钥明文。

## 仓库结构

```text
apps/client/            React/Vite 界面
apps/desktop/src-tauri/ Tauri 2 Windows/Android 原生壳
crates/novel-core/      平台无关扫描、压缩、checkpoint 与证据验证
crates/novel-import/    格式识别、导入契约及 TXT/Markdown 解析
packages/rulepack/      规则包 Schema、种子数据和校验脚本
docs/                   架构、项目计划与阶段说明
```

## 本地验证

前端使用 Node.js 24 LTS：

```powershell
cd apps/client
npm install
npm test
npm run build
```

规则包不依赖第三方库：

```powershell
node packages/rulepack/scripts/validate.mjs
```

SQLite 迁移验证使用 Node 24 内建 `node:sqlite`：

```powershell
cd apps/desktop
npm run validate:migration
```

Rust 工具链可用时：

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace --all-targets
```

本机路径包含 `&&` 时，前端脚本已绕开 npm 自动生成的 `.bin` 绝对路径包装，避免 Windows `cmd` 把目录名误作命令分隔符。

## 真实性边界

“识别某种扩展名”不等于“已支持解析”。界面、Rust 能力注册表和文档必须使用同一能力状态；没有可回到导入文件的精确锚点时，结果只能是疑似或待确认，不能标记为已核验。
