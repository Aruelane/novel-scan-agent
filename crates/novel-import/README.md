# novel-import

`novel-import` 是扫评 Agent 的跨端导入边界。它把“能识别这种文件”与“现在真的能安全导入”分开，避免 UI 把路线图上的格式误报为已支持。

当前可运行：

- TXT：UTF-8（含 BOM）与带 BOM 的 UTF-16 LE/BE；
- Markdown：同上，并支持 `#`–`###` 标题；
- 常见网文章节：`第一章`、`第十二回`、`楔子`、`番外`、`Chapter IV` 等；
- 每个章节保留行号及解码后 UTF-8 字节范围，后续命中结果可以生成稳定的章节/行号引用。

已登记但明确标为 `Pending`：EPUB、DOCX、PDF（文本层/OCR 分开）、HTML、MOBI、AZW3、ZIP、7Z。旧版二进制 DOC 明确标为 `Unsupported`。调用 `capability_registry()` 可直接生成产品能力列表；调用 `import_novel()` 遇到未完成格式会返回 `PendingSupport`，不会返回空文档假装成功。

GBK/GB18030 同样不会被静默误解码。当前返回 `EncodingPending`；后续接入跨 Windows/Android 的完整映射解码器后，再把能力状态改为可用。

最小调用示例：

```rust
use novel_import::{import_novel, ImportRequest};

let bytes = "第一章 相遇\n正文".as_bytes();
let document = import_novel(ImportRequest::new("示例.txt", bytes))?;
let source = document.chapters[0].anchor.citation_label();
# Ok::<(), novel_import::ImportError>(())
```

单独验证本 crate：

```text
cargo test --manifest-path crates/novel-import/Cargo.toml
```
