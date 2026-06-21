# docforge

一个用 Rust 编写的命令行文档转换工具，支持 **Markdown / HTML / DOCX / PDF** 之间的相互转换。基于统一的中间表示（IR）管道实现，任意两种格式之间只需 "解析源格式 → IR → 渲染目标格式"。

## 功能特性

- 支持 4 种文档格式的互转
- 统一 IR（中间表示）设计，新增格式只需实现 `Converter` trait
- 自动识别文件扩展名（`.md` / `.markdown` / `.html` / `.htm` / `.docx` / `.pdf`）
- 完整保留文档结构：标题、段落、列表、引用、代码块、表格、分割线、数学公式、内联图片
- 内联格式（粗体、斜体、行内代码、删除线、链接、行内公式）正确传递
- DOCX 图片以 base64 内嵌，跨格式转换时不丢失图像数据

## 快速开始

### 环境要求

- Rust 工具链（`stable`，项目通过 `rust-toolchain.toml` 锁定）
- 推荐 Rust 1.85+（`edition = "2024"`）

### 编译

```bash
cargo build --release
```

生成的二进制位于 `target/release/docforge`（Windows 上为 `docforge.exe`）。

### 使用方法

```bash
# Markdown → HTML
docforge --input input.md --output output.html

# HTML → Markdown
docforge --input input.html --output output.md

# DOCX → HTML
docforge --input report.docx --output report.html

# DOCX → Markdown
docforge --input report.docx --output report.md

# Markdown → DOCX
docforge --input notes.md --output notes.docx

# 简写参数
docforge -i input.md -o output.html
```

格式由文件扩展名自动判定，无需额外指定。

## 架构设计

docforge 采用经典的"中间表示（IR）"管道：

```
输入字节 ──parse_to_ir()──▶ Document (IR) ──render_from_ir()──▶ 输出字节
```

每种格式实现 `Converter` trait，包含两个方向：

- `parse_to_ir(&[u8]) -> Result<Document, ConverterError>`
- `render_from_ir(&Document) -> Result<RenderedDocument, ConverterError>`

因此，任意两种格式 `A → B` 的转换流程为：

```
A.parse_to_ir(bytes) → Document → B.render_from_ir(doc) → 输出
```

### 目录结构

```
docforge/
├── src/
│   ├── main.rs              # CLI 入口（clap 参数解析 + 管道调度）
│   ├── lib.rs               # 库根
│   ├── core/
│   │   ├── ir.rs            # 中间表示：Document / DocNode / InlineNode
│   │   ├── converter.rs     # Converter trait、DocumentFormat、错误类型
│   │   └── mod.rs
│   └── converters/
│       ├── markdown.rs      # MarkdownConverter（基于 comrak）
│       ├── html.rs          # HtmlConverter（基于 kuchikiki）
│       ├── docx.rs          # DocxConverter（基于 docx-rs）
│       ├── pdf.rs           # PdfConverter（结构占位）
│       └── mod.rs
├── tests/
│   └── conversion_flow.rs   # 端到端集成测试（15 个）
├── examples/
├── Cargo.toml
└── rust-toolchain.toml
```

### 中间表示（IR）

`core/ir.rs` 定义了与格式无关的文档模型：

- `Document` —— 顶层容器，持有 `Vec<DocNode>`
- `DocNode` —— 块级节点：`Heading` / `Paragraph` / `CodeBlock` / `BlockQuote` / `List` / `Table` / `MathBlock` / `HorizontalRule`
- `InlineNode` —— 内联节点：`Text` / `Emphasis` / `Strong` / `Code` / `Link` / `MathInline` / `Strikethrough` / `SoftBreak` / `HardBreak` / `InlineImage`

IR 同时派生 `Serialize` / `Deserialize`，可作为序列化中间格式持久化。

## 关键依赖

| 依赖 | 版本 | 用途 |
|---|---|---|
| `clap` | 4.6 | 命令行参数解析（derive 风格） |
| `comrak` | 0.52 | Markdown 解析与渲染（CommonMark + GFM 扩展） |
| `kuchikiki` | 0.8 | HTML 解析（CSS 选择器友好） |
| `docx-rs` | 0.4 | DOCX 读写 |
| `printpdf` | 0.9 | PDF 生成 |
| `serde` / `serde_json` | 1.0 | IR 序列化、DOCX JSON 中间结构 |
| `anyhow` | 1.0 | CLI 层错误聚合 |
| `thiserror` | 2.0 | 库层错误类型派生 |

## 测试

```bash
# 运行全部测试
cargo test

# 运行单个测试
cargo test markdown_to_html_example_works
```

测试覆盖位于 `tests/conversion_flow.rs`，共 15 个端到端用例，覆盖 Markdown / HTML 双向解析与渲染、表格、数学公式、代码块、内联元素、列表、引用、分割线等场景。

## 代码规范

提交前请确保通过以下检查：

```bash
cargo fmt        # 代码格式化
cargo clippy --all-targets -- -D warnings   # 静态检查（警告视为错误）
```

项目通过 `rust-toolchain.toml` 锁定 `stable` 通道。

## 实现状态

| 格式 | 解析（→ IR） | 渲染（IR →） |
|---|---|---|
| Markdown | ✅ 完整 | ✅ 完整 |
| HTML | ✅ 完整 | ✅ 完整 |
| DOCX | ✅ 完整（含图片） | ✅ 完整（含图片） |
| PDF | ⏳ 占位 | ⏳ 占位 |

## 许可证

见 [LICENSE](LICENSE)。
