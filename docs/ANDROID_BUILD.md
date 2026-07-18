# Android scaffold 与 CI 验证

## 当前边界

Android 是首发目标之一，但不是 iOS 或应用商店项目的附属项。Tauri 2 壳、前端构建入口和手动 Android workflow 已经落盘；S1 不把未经原生复核的生成目录当作产品资产，也不声称已经产出可发布 APK。

`.github/workflows/android-scaffold.yml` 只能由 `workflow_dispatch` 手动触发。它会：

1. 校验前端、Tauri 配置和两个 npm 锁文件；
2. 在临时 Ubuntu runner 安装 Java 17、Android API 36、Build Tools 36.0.0、NDK r29 `29.0.14206865` 和 `aarch64-linux-android` Rust target；
3. 如果当前提交已有 `apps/desktop/src-tauri/gen/android/gradlew`，就使用已提交 scaffold；否则调用 `npm run tauri -- android init --ci --skip-targets-install` 临时生成；
4. 只构建 aarch64 debug APK，并把无生产签名的调试产物短期保存为 workflow artifact。

Tauri CLI 官方说明 `android init` 支持 `--ci`（不提问）和 `--skip-targets-install`（不重复安装 Rust targets）。Android 官方资料把 Android 16 的 API level 定为 36，Build Tools 示例为 36.0.0；当前稳定 NDK r29 的版本号为 `29.0.14206865`。

- [Tauri CLI：`android init`](https://v2.tauri.app/reference/cli/#android-init)
- [Tauri Android 前置条件](https://v2.tauri.app/start/prerequisites/#android)
- [Android 16 SDK 设置](https://developer.android.com/about/versions/16/setup-sdk)
- [Android SDK Build Tools](https://developer.android.com/tools/releases/build-tools)
- [Android NDK 下载](https://developer.android.com/ndk/downloads)

## 临时 scaffold 不等于产品化

runner 临时生成的 `gen/android` 会随 job 销毁，不会自动回写仓库。一次成功的 debug 构建最多证明“该提交可由当前 Tauri CLI 和固定工具链生成基础工程并完成编译”，不证明以下事项已经完成：

- application ID、最低/目标 SDK、Gradle/Android 插件版本和升级策略已经定稿；
- 文件权限、Storage Access Framework、通知、前台长任务与系统返回行为已经实现；
- 小屏、横屏、软键盘、字体缩放、低内存恢复和真机兼容性已经验证；
- Android Keystore、发布签名、升级连续性或商店分发已经配置。

这些是 S6“Windows 与 Android 产品化”的工作。届时应在开发分支生成 scaffold，逐项审查差异，只提交需要版本管理的原生文件和定制；本机 `local.properties`、SDK 绝对路径、keystore、口令与 API Key 永远不得提交。手动 workflow 尚未在 GitHub 成功运行前，路线图中的远端构建门禁保持未勾选。

## 本地生成与复核

在满足 Tauri 官方 Android 前置条件后，从 `apps/desktop` 执行：

```powershell
npm ci
npm run tauri -- android init
npm run tauri -- android build --ci --debug --target aarch64
```

生成后至少复核：

- `beforeDevCommand` / `beforeBuildCommand` 是否使用 `apps/client` 的锁文件和脚本；
- application ID 是否稳定，并与后续发布签名身份一致；
- Gradle 中的 compile/target/min SDK 与已验证依赖是否兼容；
- 只申请文件选择、网络、通知和前台任务实际需要的权限；
- 小说导入使用 Storage Access Framework，不申请宽泛的全盘存储权限；
- debug 与 release 的签名、更新渠道和日志策略严格分开。

开发机应分别验证工具是否真的可用，而不是从某个环境变量存在就推断可构建：

```powershell
node --version
npm --version
rustc --version
java -version
adb version
sdkmanager --list
```

## Android 运行时边界

- 文件：通过 SAF 获取 `content://` URI，记录显示名和必要的持久授权；不要把 URI 当普通文件路径拼接。
- 密钥：API Key 的加密密钥由 Android Keystore 保护，SQLite 只存 secret reference 和非秘密配置。
- 长任务：全书扫描持续写入章节检查点，并使用 Android 允许的前台/持久工作机制显示通知；不能依赖 WebView 永不被回收。
- 网络：支持取消、重试和离线恢复；切换网络后不得重复确认同一章节结果。
- UI：处理安全区、软键盘、横竖屏、系统返回键、字体缩放和低内存恢复。
- 分发：首发可以侧载签名 APK；不把 Play Store 或 iOS App Store 上架作为完成条件。

## 生产签名不进入普通 CI

当前 debug workflow 只做工程可构建性验证。生产发布需要单独的受保护 workflow/environment，并从 GitHub encrypted secrets 或等价密钥服务临时注入 keystore 与口令。日志不得打印秘密，构建结束后不得把 keystore 作为 artifact 上传。
