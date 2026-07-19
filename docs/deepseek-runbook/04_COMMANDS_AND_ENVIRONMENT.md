# 04｜稳定命令与环境说明

任务文件优先指定最小命令；本文件提供仓库通用命令。命令以 Windows PowerShell 为主，必须记录真实退出码。不要因为本机工具缺失就修改项目来绕过 CI。

## Node 24

仓库内置 Node：

```powershell
$taskNode = (Resolve-Path '.toolchain\node-v24.18.0-win-x64\node.exe').Path
& $taskNode --version
```

前端：

```powershell
Push-Location apps/client
& $taskNode ./node_modules/vitest/vitest.mjs run
& $taskNode ./node_modules/typescript/bin/tsc -b
& $taskNode ./node_modules/vite/bin/vite.js build
Pop-Location
```

规则包与原生静态校验：

```powershell
& $taskNode ./packages/rulepack/scripts/validate.mjs
& $taskNode ./packages/rulepack/scripts/validate-negative.mjs
& $taskNode ./apps/desktop/scripts/validate-migration.mjs
& $taskNode ./apps/desktop/scripts/validate-capability.mjs
```

只有 package-lock 与 package.json 一致并且任务需要安装时才运行 `npm ci`；不得用无锁 `npm install` 制造依赖漂移。

## Rust MSVC 工具链

若系统 `cargo` 不在 PATH，使用仓库工具链：

```powershell
$taskRustBin = (Resolve-Path '.toolchain\rustup-msvc\toolchains\1.97.1-x86_64-pc-windows-msvc\bin').Path
$taskCargoHome = (Resolve-Path '.toolchain\cargo').Path
$env:PATH = "$taskRustBin;$env:PATH"
$env:CARGO_HOME = $taskCargoHome
& (Join-Path $taskRustBin 'cargo.exe') --version
```

常用验证：

```powershell
cargo fmt --all -- --check
cargo metadata --locked --offline --format-version 1 --no-deps
cargo check --workspace --all-targets --offline
cargo test --workspace --all-targets --offline
```

本机可能在依赖 build script 阶段报 `link.exe not found`。这只证明本地 MSVC linker 缺失，不证明代码通过或失败；保存错误并使用 GitHub `windows-2022` runner。

## Git 与范围检查

```powershell
git status -sb
git diff --check
git diff --stat
git diff --name-only
git diff --cached --check
git log -3 --oneline --decorate
```

只暂存任务文件：

```powershell
git add -- path/to/file1 path/to/file2
git diff --cached --name-only
```

## GitHub 阶段 CI

专用分支阶段门禁可以显式触发：

```powershell
gh auth status
gh workflow run CI --ref (git branch --show-current)
gh run list --workflow CI --branch (git branch --show-current) --limit 5
```

取得本次 run ID 后：

```powershell
gh run view RUN_ID --json status,conclusion,url,jobs
gh run watch RUN_ID --exit-status --interval 10
```

失败日志只读取失败步骤：

```powershell
gh run view RUN_ID --log-failed
```

不要重跑旧 SHA 后宣称当前提交通过。记录 run URL、head SHA 与各 job 结论。

## Android

Android 命令只能在对应 S6 任务确认 SDK/NDK/JDK 环境后执行。不得提交：

- `local.properties`
- 本机 SDK 绝对路径
- keystore/证书
- 签名口令
- 设备序列号

CI 临时生成 scaffold 只证明基础生成/编译路径，不等于 SAF、Keystore、前台长任务或真机体验已经完成。

## 编码和日志

- Markdown/JSON/TS/Rust/SQL 文件保持 UTF-8。
- PowerShell 显示中文乱码时先确认读取编码，不要据此重写整个文件。
- 测试输出不得包含真实 API Key、完整小说正文、本机路径或持久 URI。
