# 扫文助手 · 前端演示

网络小说扫评/扫雷 Agent 的第一阶段前端竖切（Vite + React + TypeScript）。界面中的书名、作者与摘录均为本项目原创演示内容。

## 快速启动

```bash
cd apps/client
npm install
npm run dev      # 开发服务器 http://localhost:1420
npm run build    # 生产构建
npm run test     # 运行 Vitest 单元测试
```

## 范围说明

此目录为小说扫评 Agent 的前端界面（Vite + React + TypeScript）。已通过 Tauri native bridge 连接 Rust 核心；在浏览器中也可使用安全静态回退独立运行。界面中的书名、作者与摘录均为本项目原创演示内容。

- 桌面三栏 + 移动端自适应布局
- 书架任务管理（左栏）
- 自然语言主工作区（中栏）：导入、规则选择、扫描进度、对话式要求、设置
- 命中证据面板（右栏）：章节 + 位置 + 原文摘录 + 置信状态
- 演示数据：3 本原创示例书、12 条规则、1 个模拟扫描作业、2 个原创命中示例
- LLM 提供商配置界面（5 种，不强制启动时配置 Key）
- 移动端底部三标签导航（书架 / 工作区 / 命中）
- 格式能力如实展示：TXT、Markdown 为 Ready；EPUB、PDF、DOCX、HTML、MOBI、AZW3、ZIP、7Z 为 Pending；旧 DOC 为 Unsupported

### 后续阶段

- 对接更多 Tauri 2 原生能力（文件选择器、SQLite 持久化、安全存储）
- 继续接入 Pending 状态的 EPUB、PDF、DOCX 等解析器
- 真实 API 调用与流式进度
- 上下文压缩策略的 UI 反馈
- 结果导出 / 分享

## 目录结构

```
apps/client/
├── package.json              # 依赖与脚本
├── tsconfig.json             # TypeScript 配置
├── vite.config.ts            # Vite + Vitest 配置
├── index.html                # 入口 HTML
├── README.md                 # 本文件
└── src/
    ├── main.tsx              # React 入口
    ├── App.tsx               # 应用根组件（三栏布局 + 移动端导航）
    ├── App.css               # 布局样式
    ├── index.css             # 全局样式与 CSS 变量
    ├── vite-env.d.ts         # Vite 类型声明
    ├── domain.ts             # 类型定义 + 工具函数（无依赖）
    ├── demo-data.ts          # 演示用静态数据
    ├── hooks/
    │   └── useAppState.ts    # 应用状态管理 hook
    ├── components/
    │   ├── Sidebar.tsx/css        # 书架侧栏
    │   ├── Workspace.tsx/css      # 主工作区（标签容器）
    │   ├── ImportPanel.tsx/css    # 导入面板（11 种格式及真实能力状态）
    │   ├── RuleSelector.tsx/css   # 规则选择器（雷点/郁闷点分组）
    │   ├── ScanProgress.tsx/css   # 扫描进度（含记忆整理、暂停/继续、对话演示）
    │   ├── SettingsPanel.tsx/css  # 设置面板（5 种 LLM 提供商）
    │   ├── EvidencePanel.tsx/css  # 命中证据面板（列表容器）
    │   ├── HitCard.tsx/css        # 单条命中卡片（摘录、理由、审核）
    │   └── BottomNav.tsx/css      # 移动端底部导航
    └── __tests__/
        ├── domain.test.ts              # 纯函数单元测试
        ├── demo-data.test.ts         # 演示数据完整性测试
        └── importCapabilities.test.ts # native bridge 安全校验测试
```

## 设计原则

- **零外部 UI 依赖**：不引入 MUI、Ant Design 等组件库，全部手写 CSS
- **CSS 变量驱动**：`index.css` 中定义完整暖色系变量，一处修改全局生效
- **语义 HTML**：`<aside>`、`<nav>`、`<main>`、`<article>`、`<blockquote>` 等
- **无障碍**：所有交互元素有 `aria-label`、`role`，支持键盘导航
- **类型安全**：`src/domain.ts` 定义全部接口，组件 Props 显式声明
- **数据层分离**：`domain.ts`（类型）+ `demo-data.ts`（数据），替换为 Tauri Rust 数据时不改动类型

## TypeScript 一致性检查

确保以下检查通过：

```bash
npx tsc --noEmit
```

## 重点复核风险

1. **移动端适配**：目前仅通过 CSS media query 切换，实际 Android 真机上 `overflow: hidden` + 软键盘弹出时可能遮挡底部导航。需在 WebView 包装层处理 safe-area-inset。
2. **虚拟列表**：当书架书籍 > 50 或命中列表 > 200 时，当前无虚拟滚动，需接入 `react-window` 或等 Tauri 后评估。
3. **Context 跨越**：当前状态全部通过 props 传递。组件数增长后建议引入轻量状态管理（zustand）。
4. **Key 安全**：前端只接收 `credentialState`，不持有或渲染密钥明文；实际凭据存取仍待原生安全存储接入。
5. **颜色可访问性**：当前严重度色彩对比度在浅色背景下大部分通过 WCAG AA，但 `--severity-1` (浅灰) + 白字在 1-2 级按钮上可能不足，建议后续用 Lighthouse 审计。
