# CODEBUDDY.md This file provides guidance to CodeBuddy when working with code in this repository.

## Build & Test Commands

```bash
# Build
cargo build

# Run (example: markdown → html)
cargo run -- --input input.md --output output.html

# Run all tests
cargo test

# Run a single test
cargo test markdown_to_html_example_works

# Check code (no binary output)
cargo check
```

## Architecture

`docforge` is a CLI document converter (Markdown / HTML / DOCX / PDF) built on an **Intermediate Representation (IR)** pipeline:

```
Input bytes → parse_to_ir() → Document (IR) → render_from_ir() → Output bytes
```

This means conversion between any two formats is simply: parse source → render target.

### Core IR (`src/core/`)

`ir.rs` defines the shared data model:
- `Document` — top-level container with a `Vec<DocNode>`
- `DocNode` — block-level nodes: `Heading`, `Paragraph`, `CodeBlock`, `BlockQuote`, `List`, `HorizontalRule`
- `InlineNode` — inline content: `Text`, `Emphasis`, `Strong`, `Code`, `Link`, `SoftBreak`, `HardBreak`

`converter.rs` defines the `Converter` trait (`parse_to_ir()` / `render_from_ir()`) and supporting types: `DocumentFormat`, `RenderedDocument` (Text or Binary), `ConverterError`.

### Converters (`src/converters/`)

Each format has its own module implementing the `Converter` trait (Markdown → IR, IR → Markdown; HTML → IR, IR → HTML; etc.).

**Implementation status:**
- `MarkdownConverter` — `parse_to_ir` fully implemented (via `pulldown-cmark`); `render_from_ir` partially implemented (BlockQuote & List rendering returns error)
- `HtmlConverter` — `render_from_ir` fully implemented; `parse_to_ir` is a stub (`todo!`)
- `DocxConverter`, `PdfConverter` — both parse and render are stubs (`todo!`)

### CLI (`src/main.rs`)

Uses `clap` derive with `--input` and `--output` flags. Format is auto-detected from file extension in `detect_format()`. The pipeline reads input bytes, selects parser/renderer via `converter_for_format()`, runs parse → render, writes output.

### Key dependencies

| Dependency | Purpose |
|---|---|
| `clap` | CLI argument parsing |
| `pulldown-cmark` | Markdown → HTML parsing |
| `htmd` | HTML → Markdown (for HTML parser impl) |
| `docx-rs` | DOCX read/write |
| `printpdf` | PDF generation |
| `serde` / `serde_json` | IR serialization |
| `anyhow` / `thiserror` | Error handling |

### Git Hooks & Formatting

- Uses `rust-toolchain.toml` (channel = `stable`).
- Run `cargo fmt` before committing.
