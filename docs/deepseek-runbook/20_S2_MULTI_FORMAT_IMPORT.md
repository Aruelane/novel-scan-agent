# DeepSeek 执行手册 20：S2 多格式导入与来源定位

> 前置门槛：`10_S1_CLOSEOUT.md` 的 S1-15 全绿。S2 完成时 Ready 仅包括真正有解析器与回证测试的 TXT、Markdown、HTML、EPUB、DOCX、文本型 PDF。旧 DOC 必须提示转换；扫描版 PDF 必须返回 OCR 提示；MOBI/AZW3/ZIP/7Z 保持待实现。

## S2 共同规则

- 继续使用 `deepseek/full-build`，一次一个任务、一次一个提交；每个任务前后检查工作区与允许文件。
- 测试材料只能是代码生成的微型原创 fixture；禁止提交用户小说或网上书籍。
- 先限流再分配/解压/解析；HTML/XML 不访问网络、不执行脚本、不解析外部实体；不绕过 DRM/密码。
- 错误、DTO、日志只使用安全 display name，不含完整 Windows path、`file://` 或 Android `content://`。
- 每个 Ready 格式最终必须具备：正常、损坏、超限、来源锚点回归四类测试。缺一类就保持 Pending。

## S2-01：统一能力状态、来源 locator、限制与错误

- **目标**：先建立所有解析器共用的强类型合同。
- **前置依赖**：S1-15。
- **严格允许文件**：`crates/novel-import/src/model.rs`、`capability.rs`、`error.rs`、`lib.rs`。
- **禁止范围**：本任务不把任何 Pending 改 Ready，不引入具体解析依赖。
- **逐步实现**：新增 `ImportLimits`（原始字节、解码字节、章节数、归档条目/单项/总展开、XML、PDF 页数等）并进入 `ImportOptions`；扩展错误为 corrupt/limit/OCR required/conversion required/protected；`SourceLocator` 增加 TextRange、Html DOM/block、Epub spine/href/fragment/paragraph、Docx paragraph、Pdf page/block 变体并修 citation；取消不再成立的 `Copy`；必要 DTO 派生 serde。
- **测试命令**：fmt；`cargo test -p novel-import --offline`；diff check。
- **完成门槛**：默认限制非零、可用小限制测试；错误 Display 不泄露原始路径；现有 TXT 测试不退化。
- **失败/阻塞处理**：不得用 `usize::MAX` 当默认无限制；编译错误逐调用点修正但不得改范围外文件，若确需则停止报告依赖。
- **报告格式**：任务 ID；新合同/默认限额；测试/退出码；diff；提交 SHA。

## S2-02：格式识别防伪与能力诚实矩阵

- **目标**：强签名优先，扩展名不能把二进制伪装成文本或把普通 ZIP 当 EPUB/DOCX 成功导入。
- **前置依赖**：S2-01。
- **严格允许文件**：`crates/novel-import/src/capability.rs`、`lib.rs`。
- **禁止范围**：不实现容器解析；不把 MIME/扩展名当最终可信证据。
- **逐步实现**：PDF/7Z/MOBI/OLE/ZIP 强签名；TXT/Markdown/HTML 只有通过文本可解码性才按扩展/MIME；ZIP 的 epub/docx 仅为候选，后续结构校验失败必须 corrupt；识别旧 OLE DOC 为 conversion required；未知 NUL-heavy 数据拒绝。
- **测试命令**：fmt；novel-import tests；diff check。
- **完成门槛**：覆盖错误扩展、错误 MIME、大小写、查询串、空文件、伪 ZIP/伪 PDF、OLE DOC；能力表仍不夸大。
- **失败/阻塞处理**：不要为方便把未知字节 lossy 转 UTF-8。
- **报告格式**：任务 ID；检测优先级；负例；测试数；diff；SHA。

## S2-03：TXT 的 UTF-8/UTF-16/GBK/GB18030

- **目标**：完成多编码无乱码导入与明确失败。
- **前置依赖**：S2-02。
- **严格允许文件**：`crates/novel-import/Cargo.toml`、`Cargo.lock`、`crates/novel-import/src/encoding.rs`、`model.rs`、`plain_text.rs`。
- **禁止范围**：不得 lossy decode，不把任意二进制猜作 GBK，不调用平台专属 API。
- **逐步实现**：使用维护中的纯 Rust 编码库；BOM 优先、有效 UTF-8 次之、显式 hint 必须服从；`.txt` 的无 hint 旧编码只能在严格 round-trip/控制字符检查通过时接受；记录实际 `TextEncoding::Gbk/Gb18030`；解码前检查原始限制、解码后检查 UTF-8 限制。
- **测试命令**：fmt；novel-import tests；locked metadata；diff check。
- **完成门槛**：UTF-8/BOM、UTF-16 LE/BE、GBK 中文、GB18030 四字节字符、错误 hint、截断序列、二进制、超限全覆盖。
- **失败/阻塞处理**：库不支持真正 GB18030 四字节时不得谎称，换合适纯 Rust 库或保持 Pending 并报告。
- **报告格式**：任务 ID；判码顺序；依赖/锁变化；测试矩阵；退出码；SHA。

## S2-04：TXT 分章、换行与锚点回归

- **目标**：多编码、CRLF/LF/无末尾换行下行号和 decoded UTF-8 范围稳定。
- **前置依赖**：S2-03。
- **严格允许文件**：`crates/novel-import/src/plain_text.rs`、`model.rs`。
- **禁止范围**：不扩展 Markdown 语法，不伪造章节标题。
- **逐步实现**：章节数检查在 push 前；空正文明确失败；front matter 保留；标题误判保护；所有 range 必须能从 decoded text 精确切片回 chapter text。
- **测试命令**：fmt；novel-import tests；diff check。
- **完成门槛**：正常/空损坏/章节超限/逐章切片回归四类齐全；index 始终 1-based。
- **失败/阻塞处理**：不得 normalize 换行后仍声称原始行号。
- **报告格式**：任务 ID；覆盖换行/标题；测试数；diff；SHA。

## S2-05：Markdown 独立解析器

- **目标**：标题分章不受 fenced code 等伪标题干扰，并保留原始行号。
- **前置依赖**：S2-04。
- **严格允许文件**：`crates/novel-import/src/lib.rs`、新建 `src/markdown.rs`、必要时 `Cargo.toml`/`Cargo.lock`。
- **禁止范围**：不执行 HTML，不下载图片/链接，不把渲染后文本坐标冒充文件字节坐标。
- **逐步实现**：支持 ATX 与 Setext 标题；忽略 fenced/indented code 内标题；章节 text 保持可从 decoded 原文切片；无标题全文回退并警告；limits 生效。
- **测试命令**：fmt；novel-import tests；diff check。
- **完成门槛**：正常、未闭合 fence/坏文本、超限、heading/line/byte anchor 回切均测试；然后 Markdown 才保持 Ready。
- **失败/阻塞处理**：若依赖只给渲染 offset，必须验证 offset 对原字符串，不可猜。
- **报告格式**：任务 ID；语法边界；测试矩阵；依赖变化；SHA。

## S2-06：共享安全 ZIP/XML 基础

- **目标**：给 EPUB/DOCX 提供一次实现、两处复用的容器安全层。
- **前置依赖**：S2-05。
- **严格允许文件**：`crates/novel-import/Cargo.toml`、`Cargo.lock`、新建 `src/archive.rs`、新建 `src/xml.rs`、`src/lib.rs`。
- **禁止范围**：不启用 ZIP/7Z 用户导入，不写磁盘解压，不解析外部实体。
- **逐步实现**：内存只读枚举；拒绝绝对路径、盘符、`..`、反斜杠逃逸、重复规范化名、加密项；在读取前累计 entry count/declared size，读取中再硬限单项/总量；XML 禁 DTD/entity/network，节点/文本限额。
- **测试命令**：fmt；novel-import tests；diff check。
- **完成门槛**：路径穿越、重复、压缩炸弹声明/实际、过多条目、坏 CRC/XML 均失败；无临时落盘。
- **失败/阻塞处理**：压缩库不能报告加密时必须在解析错误中诚实拒绝，不尝试破解。
- **报告格式**：任务 ID；安全门；恶意 fixture；依赖/退出码；SHA。

## S2-07：HTML 安全文本导入

- **目标**：本地 HTML 正文/标题导入，脚本和外部资源永不执行/请求。
- **前置依赖**：S2-03、S2-01。
- **严格允许文件**：`crates/novel-import/Cargo.toml`、`Cargo.lock`、`src/lib.rs`、新建 `src/html.rs`。
- **禁止范围**：无 WebView 渲染、无 HTTP、无 JS、无 CSS selector 执行用户代码。
- **逐步实现**：用容错 HTML parser；处理受支持 charset；剔除 script/style/noscript/template；h1–h3 分章，无标题 body 回退；生成稳定的资源名 + DOM/block ordinal locator；limits 生效。
- **测试命令**：fmt；novel-import tests；diff check。
- **完成门槛**：正常、多 charset、恶意 script/external URL、损坏 HTML、超限、DOM locator 回归全过；才将 HTML Ready。
- **失败/阻塞处理**：解析器容错不等于成功，正文为空必须错误。
- **报告格式**：任务 ID；抽取/过滤规则；测试矩阵；依赖；SHA。

## S2-08：EPUB 容器、OPF 与 spine

- **目标**：严格验证 EPUB 结构并按 spine 排序资源。
- **前置依赖**：S2-06、S2-07。
- **严格允许文件**：`src/lib.rs`、新建 `src/epub.rs`、必要 `Cargo.toml`/`Cargo.lock`。
- **禁止范围**：不支持 DRM/远程资源/脚本，不以 ZIP 扩展名代替结构验证。
- **逐步实现**：校验 mimetype、`META-INF/container.xml`、rootfile、OPF manifest/spine；按 OPF 基准规范化相对路径并经 archive 安全层；缺失/重复/逃逸/encryption.xml 明确失败。
- **测试命令**：fmt；novel-import tests；diff check。
- **完成门槛**：最小原创 EPUB 得到正确 spine；坏 mimetype/container/OPF/missing item/traversal/DRM/超限均拒绝。
- **失败/阻塞处理**：不要“跳过坏章节继续成功”；结构损坏整体失败。
- **报告格式**：任务 ID；结构验证；负例；测试数；SHA。

## S2-09：EPUB 正文、章节与来源锚点

- **目标**：spine XHTML 形成章节，锚点可回到 href/fragment/paragraph。
- **前置依赖**：S2-08。
- **严格允许文件**：`src/epub.rs`、`src/model.rs`。
- **禁止范围**：不声称完整 EPUB CFI，除非确实实现并测试；不拼接远程资源。
- **逐步实现**：复用 HTML 抽取但保留 spine index/href；标题不足时用文档标题或安全资源名，不伪造“第X章”；记录 fragment（存在时）和 paragraph range；跳过纯导航资源必须有明确规则。
- **测试命令**：fmt；novel-import tests；diff check。
- **完成门槛**：多 spine 顺序、fragment、重复标题、空 spine 项、章节/锚点回归和超限测试齐全；EPUB 改 Ready。
- **失败/阻塞处理**：部分章节为空时整体 corrupt 或显式 warning，不能静默报告完整导入。
- **报告格式**：任务 ID；spine→chapter 映射；locator 精度；测试数；SHA。

## S2-10：DOCX 正文、标题与段落锚点

- **目标**：只解析 Word 主文档，明确页眉页脚/脚注规则。
- **前置依赖**：S2-06。
- **严格允许文件**：`src/lib.rs`、新建 `src/docx.rs`、`src/model.rs`、必要 Cargo 文件。
- **禁止范围**：不执行宏/OLE，不读取外部关系，不声称支持旧 DOC。
- **逐步实现**：验证 `[Content_Types].xml` 与 `word/document.xml`；解析 paragraph/run/text/tab/break；用内置/本地化 Heading 样式与 outline level 分章；locator 保存 1-based paragraph range；首版只取主文档，页眉页脚、脚注/尾注忽略并产生具体 warning。
- **测试命令**：fmt；novel-import tests；diff check。
- **完成门槛**：正常、多 run 中文、标题、无标题、损坏 ZIP/XML、外部关系、宏提示、超限、paragraph anchor 回归全过；DOCX 改 Ready；Legacy DOC 仍 conversion required。
- **失败/阻塞处理**：缺 styles 可全文回退，缺 document.xml 必须失败。
- **报告格式**：任务 ID；包含/忽略规则；测试矩阵；依赖；SHA。

## S2-11：文本型 PDF 与扫描版判定

- **目标**：纯 Rust、逐页提取文本并保留页码；无可靠文本层时明确要求 OCR。
- **前置依赖**：S2-01。
- **严格允许文件**：`Cargo.toml`、`Cargo.lock`、`src/lib.rs`、新建 `src/pdf.rs`、`src/model.rs`。
- **禁止范围**：不做 OCR、不调用外部程序、不绕过密码、不伪造章节名/坐标。
- **逐步实现**：使用纯 Rust PDF 库逐页解析；加密/密码文件拒绝；每个有文本页作为可引用 section（标题“第 N 页”，不是章节推断）；记录 page + text block；空白页 warning；全书无足够文本返回 `OcrRequired`；页数/文本量 limits。
- **测试命令**：fmt；novel-import tests；diff check。
- **完成门槛**：代码生成的多页文本 PDF、截断/损坏、加密、页数/文本超限、扫描式无文字、page anchor 回归全过；能力说明为“文本型 PDF Ready，扫描版需 OCR”。
- **失败/阻塞处理**：库不能稳定逐页定位就保持 Pending，不允许用全文 form-feed 猜页码。
- **报告格式**：任务 ID；页模型/OCR 判定；测试矩阵；依赖；SHA。

## S2-12：Windows path / Android URI 的同一原生读取合同

- **目标**：两种平台来源都在 native 层转为 bounded bytes + 安全 display name，core 不认识平台 URI。
- **前置依赖**：S2-03 至 S2-11。
- **严格允许文件**：新建 `apps/desktop/src-tauri/src/import_source.rs`、`apps/desktop/src-tauri/src/lib.rs`。
- **禁止范围**：不得把 PathBuf/URI Serialize 给 WebView；本任务不实现 Android SAF 权限持久化（S6）。
- **逐步实现**：定义私有 `PlatformDocumentHandle::{WindowsPath, AndroidContentUri}` 与 reader trait；公开给 importer 的只有 display_name/media_type/size/bytes；读前 size limit、读中硬限；用 fake Windows/Android reader 证明二者进入同一个 `ImportRequest`；返回安全错误码。
- **测试命令**：fmt；desktop lib tests；workspace check；diff check。
- **完成门槛**：序列化 DTO/错误中无盘符、反斜杠、`file://`、`content://`；fake URI 测试通过；文档注明真实 SAF 留 S6。
- **失败/阻塞处理**：不要在 WebView 侧用 fetch 读 content URI；平台 API 不可用就只交合同与 fake 测试。
- **报告格式**：任务 ID；边界图；泄露负例；测试/退出码；SHA。

## S2-13：Tauri Windows 选择与导入命令

- **目标**：Windows 用户可实际选择支持文件；浏览器/Android 未实现路径诚实降级。
- **前置依赖**：S2-12。
- **严格允许文件**：`apps/desktop/src-tauri/src/lib.rs`、`import_source.rs`、`apps/desktop/src-tauri/Cargo.toml`、`Cargo.lock`、`apps/desktop/src-tauri/capabilities/main-local.json`。
- **禁止范围**：不得让 JS dialog API先收到路径；不得扩大 capability 到 shell/network/任意 fs。
- **逐步实现**：从 Rust 端调用 dialog，选择后在 native 层读取并 `import_novel`；返回 `ImportedBookSummary`（ID、display name、format、章节数、统计、warnings），不返回 handle/path/全文；取消选择为独立非错误结果；只开放最小 command。
- **测试命令**：fmt；desktop tests/check；capability validator；Tauri debug no-bundle（Windows CI）。
- **完成门槛**：command 单元层覆盖 cancel/ready/corrupt/limit/pending/OCR/conversion；capability allowlist 仍最小。
- **失败/阻塞处理**：dialog 难以单测时抽 trait 注入 fake，不以真实 GUI 作为唯一测试。
- **报告格式**：任务 ID；command DTO；权限变化；测试/CI；SHA。

## S2-14：前端真实导入流与诚实状态

- **目标**：Tauri 中启用按钮并展示结果/错误；浏览器与 Android 未接 SAF 时不伪装可用。
- **前置依赖**：S2-13。
- **严格允许文件**：`apps/client/src/domain.ts`、`services/importCapabilities.ts`、新建 `services/importBooks.ts`、`hooks/useAppState.ts`、`components/ImportPanel.tsx/css`、`App.tsx`、相关 `src/__tests__/*`。
- **禁止范围**：不得创建 `<input type=file>` 把本地文件读进 WebView；不得把 Pending 显示“可导入”。
- **逐步实现**：Tauri 环境按钮调用 native command；浏览器显示“仅预览”；Android 若 command 报 platform unavailable，显示 SAF 将在 S6 接入；按 Ready/Pending/需转换/不支持和 per-file OCR 错误分别文案；成功只把安全 summary 加入书架。
- **测试命令**：client unit/build/e2e；Rust capability validator；diff check。
- **完成门槛**：mock native 覆盖 cancel/success/每类错误；Ready 集合正好 TXT/Markdown/HTML/EPUB/DOCX/PDF；MOBI/AZW3/ZIP/7Z pending，DOC conversion，扫描 PDF OCR。
- **失败/阻塞处理**：native bridge 不可用必须固定、无路径 fallback；不得回退静默成功。
- **报告格式**：任务 ID；状态/文案矩阵；测试数；diff；SHA。

## S2-15：格式矩阵、文档与 CI 退出门禁

- **目标**：独立证明每个 Ready 格式的正常/损坏/超限/锚点四门，并同步所有声明。
- **前置依赖**：S2-01 至 S2-14。
- **严格允许文件**：`crates/novel-import/tests/import_matrix.rs`、`.github/workflows/ci.yml`、`README.md`、`docs/PROJECT_PLAN.md`、`docs/ROADMAP.md`、`docs/ARCHITECTURE.md`、`apps/client/README.md`。
- **禁止范围**：原则上不改解析实现；不把旧 DOC/扫描 PDF/MOBI/AZW3/ZIP/7Z 写成支持。
- **逐步实现**：代码生成六种格式 fixture；统一表驱动四门；每个成功文档逐章验证 locator、chapter text、citation 非空且可回到对应资源/页/段落；CI 执行 workspace tests、client test/build/e2e、所有 validator 与 Tauri check；文档列出精度和限制。
- **测试命令**：`cargo fmt --all -- --check`；workspace check/test；client test/build/e2e；rulepack/migration/capability；`git diff --check`。
- **完成门槛**：draft PR 全绿；能力矩阵与 Rust registry、前端 fallback、README 完全一致；工作区干净。实际 Android SAF 仍列 S6，不阻塞 S2 合同验收。
- **失败/阻塞处理**：任一 Ready 格式缺一门，立即降回 Pending并在报告列原因；任一 CI 红灯则“S2 未通过”，不得进入 S3。
- **报告格式**：`S2 最终报告`；Ready/Pending/转换/OCR矩阵；每格式四门测试数；本地命令/退出码；PR/CI URL；未实现项；提交 SHA 范围。

S2 全绿后按总索引继续下一阶段；不要自行把 Android SAF、OCR、旧 DOC 或压缩包批量导入标为完成。
