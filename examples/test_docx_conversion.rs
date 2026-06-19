use docforge::converters::docx::DocxConverter;
use docforge::converters::html::HtmlConverter;
use docforge::converters::markdown::MarkdownConverter;
use docforge::core::converter::{Converter, RenderedDocument};
use std::fs;

fn main() {
    println!("=== DOCX 转换测试 ===\n");

    // 测试 1: Markdown → IR → DOCX (含数学公式、表格)
    test_markdown_to_docx();

    // 测试 2: HTML → IR → DOCX (含表格)
    test_html_to_docx();

    // 测试 3: DOCX → IR → DOCX (含图片)
    test_docx_to_docx();

    // 测试 4: 复杂 Markdown (混合元素)
    test_complex_markdown_to_docx();

    println!("\n=== 所有测试完成 ===");
}

fn test_markdown_to_docx() {
    println!("--- 测试 1: Markdown → DOCX (数学公式 + 表格) ---");

    let markdown = r#"# 数学公式测试

## 行内公式

这是质能方程 $E = mc^2$ 和欧拉公式 $e^{i\pi} + 1 = 0$。

## 块级公式

$$
\sum_{i=1}^{n} x_i = \frac{n(n+1)}{2}
$$

$$
\int_{0}^{\infty} e^{-x^2} dx = \frac{\sqrt{\pi}}{2}
$$

## 表格测试

| 姓名 | 年龄 | 成绩 |
|------|------|------|
| 张三 | 20   | 95   |
| 李四 | 21   | 88   |
| 王五 | 19   | 92   |

## 代码块

```rust
fn main() {
    println!("Hello, world!");
}
```

结束。
"#;

    let md_converter = MarkdownConverter;
    let docx_converter = DocxConverter;

    let ir = md_converter
        .parse_to_ir(markdown.as_bytes())
        .expect("Markdown 解析失败");

    println!("  IR 节点数: {}", ir.nodes.len());

    match docx_converter.render_from_ir(&ir) {
        Ok(RenderedDocument::Binary(bytes)) => {
            let path = "test_math_table.docx";
            fs::write(path, &bytes).unwrap();
            println!("  ✅ 成功生成 DOCX: {} ({} bytes)", path, bytes.len());
        }
        Ok(RenderedDocument::Text(t)) => {
            println!("  ❌ 意外文本输出: {}", t);
        }
        Err(e) => {
            println!("  ❌ 渲染失败: {}", e);
        }
    }
    println!();
}

fn test_html_to_docx() {
    println!("--- 测试 2: HTML → DOCX (表格 + 混合格式) ---");

    let html = r#"<body>
<h1>HTML 转换测试</h1>
<p>这是一段 <strong>粗体</strong> 和 <em>斜体</em> 文本。</p>
<table>
<tr><th>列 A</th><th>列 B</th><th>列 C</th></tr>
<tr><td>1</td><td>2</td><td>3</td></tr>
<tr><td>4</td><td>5</td><td>6</td></tr>
</table>
<ul>
<li>无序列表项 1</li>
<li>无序列表项 2</li>
</ul>
<ol>
<li>有序列表项 1</li>
<li>有序列表项 2</li>
</ol>
<blockquote><p>这是一个引用块</p></blockquote>
<pre><code class="language-python">def hello():
    print("Hello")</code></pre>
</body>"#;

    let html_converter = HtmlConverter;
    let docx_converter = DocxConverter;

    let ir = html_converter
        .parse_to_ir(html.as_bytes())
        .expect("HTML 解析失败");

    println!("  IR 节点数: {}", ir.nodes.len());

    match docx_converter.render_from_ir(&ir) {
        Ok(RenderedDocument::Binary(bytes)) => {
            let path = "test_html_to_docx.docx";
            fs::write(path, &bytes).unwrap();
            println!("  ✅ 成功生成 DOCX: {} ({} bytes)", path, bytes.len());
        }
        Ok(RenderedDocument::Text(t)) => {
            println!("  ❌ 意外文本输出: {}", t);
        }
        Err(e) => {
            println!("  ❌ 渲染失败: {}", e);
        }
    }
    println!();
}

fn test_docx_to_docx() {
    println!("--- 测试 3: DOCX → DOCX (图片保留) ---");

    let docx_path = "tests/实验报告_stats.docx";
    let docx_converter = DocxConverter;

    let input_bytes = match fs::read(docx_path) {
        Ok(b) => b,
        Err(e) => {
            println!("  ❌ 无法读取 {}: {}", docx_path, e);
            return;
        }
    };

    println!("  输入文件大小: {} bytes", input_bytes.len());

    let ir = docx_converter
        .parse_to_ir(&input_bytes)
        .expect("DOCX 解析失败");

    println!("  IR 节点数: {}", ir.nodes.len());

    // 检查是否有图片
    let mut image_count = 0;
    for node in &ir.nodes {
        if let docforge::core::ir::DocNode::Paragraph { content } = node {
            for inline in content {
                if let docforge::core::ir::InlineNode::InlineImage { id, .. } = inline {
                    image_count += 1;
                    println!("  发现图片: {}", id);
                }
            }
        }
    }
    println!("  图片数量: {}", image_count);

    match docx_converter.render_from_ir(&ir) {
        Ok(RenderedDocument::Binary(bytes)) => {
            let path = "test_docx_roundtrip.docx";
            fs::write(path, &bytes).unwrap();
            println!("  ✅ 成功生成 DOCX: {} ({} bytes)", path, bytes.len());
        }
        Ok(RenderedDocument::Text(t)) => {
            println!("  ❌ 意外文本输出: {}", t);
        }
        Err(e) => {
            println!("  ❌ 渲染失败: {}", e);
        }
    }
    println!();
}

fn test_complex_markdown_to_docx() {
    println!("--- 测试 4: 复杂 Markdown → DOCX (混合所有元素) ---");

    let markdown = r#"# 综合测试文档

## 简介

这是一份**综合测试文档**，用于验证 DOCX 转换的各个方面。

## 数学公式

行内公式：$\\alpha + \\beta = \\gamma$

块级公式：
$$
\\begin{pmatrix}
a & b \\\\
c & d
\\end{pmatrix}
\\begin{pmatrix}
x \\\\
y
\\end{pmatrix}
=
\\begin{pmatrix}
ax + by \\\\
cx + dy
\\end{pmatrix}
$$

## 数据表

| 指标 | 值 | 变化 |
|------|-----|------|
| 准确率 | 95.2% | +2.1% |
| 召回率 | 89.7% | -1.3% |
| F1 分数 | 92.3% | +0.5% |

## 代码示例

```python
import numpy as np

def matrix_multiply(A, B):
    return np.dot(A, B)
```

行内代码：`np.dot()`

## 列表

### 无序列表
- 第一项
- 第二项
  - 嵌套项 A
  - 嵌套项 B
- 第三项

### 有序列表
1. 步骤一
2. 步骤二
3. 步骤三

## 引用

> 这是一段引用文本。
> 可以跨越多行。

## 分隔线

---

## 链接

[访问 GitHub](https://github.com)

~~删除线文本~~

**结束**
"#;

    let md_converter = MarkdownConverter;
    let docx_converter = DocxConverter;

    let ir = md_converter
        .parse_to_ir(markdown.as_bytes())
        .expect("Markdown 解析失败");

    println!("  IR 节点数: {}", ir.nodes.len());

    match docx_converter.render_from_ir(&ir) {
        Ok(RenderedDocument::Binary(bytes)) => {
            let path = "test_complex.docx";
            fs::write(path, &bytes).unwrap();
            println!("  ✅ 成功生成 DOCX: {} ({} bytes)", path, bytes.len());
        }
        Ok(RenderedDocument::Text(t)) => {
            println!("  ❌ 意外文本输出: {}", t);
        }
        Err(e) => {
            println!("  ❌ 渲染失败: {}", e);
        }
    }
    println!();
}
