use docforge::converters::docx::DocxConverter;
use docforge::converters::html::HtmlConverter;
use docforge::converters::markdown::MarkdownConverter;
use docforge::core::converter::{Converter, RenderedDocument};
use std::fs;

fn main() {
    println!("=== 全转换测试 ===\n");

    // 1. Markdown → DOCX (数学公式 + 表格 + 代码块)
    test_markdown_to_docx();

    // 2. DOCX → Markdown (含图片)
    test_docx_to_markdown();

    // 3. DOCX → HTML (含图片)
    test_docx_to_html();

    // 4. Markdown → HTML (通过 IR)
    test_markdown_to_html();

    // 5. HTML → Markdown (通过 IR)
    test_html_to_markdown();

    println!("\n=== 所有测试完成 ===");
}

fn test_markdown_to_docx() {
    println!("--- 测试 1: Markdown → DOCX ---");

    let markdown = r#"# 数学公式测试

行内公式：$E = mc^2$

块级公式：
$$
\sum_{i=1}^{n} x_i = \frac{n(n+1)}{2}
$$

## 代码块

```rust
fn main() {
    println!("Hello, world!");
}
```

## 表格

| 姓名 | 年龄 |
|------|------|
| 张三 | 20   |
| 李四 | 21   |
"#;

    let md_converter = MarkdownConverter;
    let docx_converter = DocxConverter;

    let ir = md_converter
        .parse_to_ir(markdown.as_bytes())
        .expect("MD parse failed");
    println!("  IR 节点数: {}", ir.nodes.len());

    match docx_converter.render_from_ir(&ir) {
        Ok(RenderedDocument::Binary(bytes)) => {
            let path = "test_md_to_docx.docx";
            fs::write(path, &bytes).unwrap();
            println!("  ✅ {} ({} bytes)", path, bytes.len());
        }
        Ok(RenderedDocument::Text(t)) => println!("  ❌ 文本: {}", t),
        Err(e) => println!("  ❌ 错误: {}", e),
    }
    println!();
}

fn test_docx_to_markdown() {
    println!("--- 测试 2: DOCX → Markdown ---");

    let docx_path = "tests/实验报告_stats.docx";
    let docx_converter = DocxConverter;
    let md_converter = MarkdownConverter;

    let input_bytes = match fs::read(docx_path) {
        Ok(b) => b,
        Err(e) => {
            println!("  ❌ 无法读取: {}", e);
            return;
        }
    };

    let ir = docx_converter
        .parse_to_ir(&input_bytes)
        .expect("DOCX parse failed");
    println!("  IR 节点数: {}", ir.nodes.len());

    match md_converter.render_from_ir(&ir) {
        Ok(RenderedDocument::Text(text)) => {
            let path = "test_docx_to_md.md";
            fs::write(path, &text).unwrap();
            println!("  ✅ {} ({} chars)", path, text.len());
            // 打印前 500 字符预览
            let preview: String = text.chars().take(500).collect();
            println!("  预览:\n{}", preview);
        }
        Ok(RenderedDocument::Binary(b)) => println!("  ❌ 二进制: {} bytes", b.len()),
        Err(e) => println!("  ❌ 错误: {}", e),
    }
    println!();
}

fn test_docx_to_html() {
    println!("--- 测试 3: DOCX → HTML ---");

    let docx_path = "tests/实验报告_stats.docx";
    let docx_converter = DocxConverter;
    let html_converter = HtmlConverter;

    let input_bytes = match fs::read(docx_path) {
        Ok(b) => b,
        Err(e) => {
            println!("  ❌ 无法读取: {}", e);
            return;
        }
    };

    let ir = docx_converter
        .parse_to_ir(&input_bytes)
        .expect("DOCX parse failed");
    println!("  IR 节点数: {}", ir.nodes.len());

    match html_converter.render_from_ir(&ir) {
        Ok(RenderedDocument::Text(text)) => {
            let path = "test_docx_to_html.html";
            fs::write(path, &text).unwrap();
            println!("  ✅ {} ({} chars)", path, text.len());
            let preview: String = text.chars().take(500).collect();
            println!("  预览:\n{}", preview);
        }
        Ok(RenderedDocument::Binary(b)) => println!("  ❌ 二进制: {} bytes", b.len()),
        Err(e) => println!("  ❌ 错误: {}", e),
    }
    println!();
}

fn test_markdown_to_html() {
    println!("--- 测试 4: Markdown → HTML (via IR) ---");

    let markdown = r#"# 标题

**粗体** 和 *斜体* 和 `代码`。

$$
E = mc^2
$$

| A | B |
|---|---|
| 1 | 2 |
"#;

    let md_converter = MarkdownConverter;
    let html_converter = HtmlConverter;

    let ir = md_converter
        .parse_to_ir(markdown.as_bytes())
        .expect("MD parse failed");
    println!("  IR 节点数: {}", ir.nodes.len());

    match html_converter.render_from_ir(&ir) {
        Ok(RenderedDocument::Text(text)) => {
            let path = "test_md_to_html.html";
            fs::write(path, &text).unwrap();
            println!("  ✅ {} ({} chars)", path, text.len());
            println!("  输出:\n{}", text);
        }
        Ok(RenderedDocument::Binary(b)) => println!("  ❌ 二进制: {} bytes", b.len()),
        Err(e) => println!("  ❌ 错误: {}", e),
    }
    println!();
}

fn test_html_to_markdown() {
    println!("--- 测试 5: HTML → Markdown (via IR) ---");

    let html = r#"<body>
<h1>标题</h1>
<p><strong>粗体</strong> 和 <em>斜体</em> 和 <code>代码</code>。</p>
<pre><code class="language-python">def hello():
    print("Hello")
</code></pre>
<table>
<tr><th>A</th><th>B</th></tr>
<tr><td>1</td><td>2</td></tr>
</table>
</body>"#;

    let html_converter = HtmlConverter;
    let md_converter = MarkdownConverter;

    let ir = html_converter
        .parse_to_ir(html.as_bytes())
        .expect("HTML parse failed");
    println!("  IR 节点数: {}", ir.nodes.len());

    match md_converter.render_from_ir(&ir) {
        Ok(RenderedDocument::Text(text)) => {
            let path = "test_html_to_md.md";
            fs::write(path, &text).unwrap();
            println!("  ✅ {} ({} chars)", path, text.len());
            println!("  输出:\n{}", text);
        }
        Ok(RenderedDocument::Binary(b)) => println!("  ❌ 二进制: {} bytes", b.len()),
        Err(e) => println!("  ❌ 错误: {}", e),
    }
    println!();
}
