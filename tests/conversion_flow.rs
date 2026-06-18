use docforge::converters::html::HtmlConverter;
use docforge::converters::markdown::MarkdownConverter;
use docforge::core::converter::{Converter, RenderedDocument};
use docforge::core::ir::DocNode;

#[test]
fn markdown_to_html_example_works() {
    let html = MarkdownConverter::markdown_to_html("# Title\n\nHello **world**");

    assert!(html.contains("<h1>Title</h1>"));
    assert!(html.contains("<p>Hello <strong>world</strong></p>"));
}

#[test]
fn markdown_ir_can_be_rendered_as_html() {
    let markdown = "# Heading\n\nParagraph text.";
    let markdown_converter = MarkdownConverter;
    let html_converter = HtmlConverter;

    let document = markdown_converter
        .parse_to_ir(markdown.as_bytes())
        .expect("markdown should parse to IR");

    let rendered = html_converter
        .render_from_ir(&document)
        .expect("IR should render to HTML");

    let RenderedDocument::Text(html) = rendered else {
        panic!("expected HTML text output");
    };

    assert!(html.contains("<h1>Heading</h1>"));
    assert!(html.contains("<p>Paragraph text.</p>"));
}

#[test]
fn markdown_ir_round_trips() {
    let markdown_converter = MarkdownConverter;

    let inputs = vec![
        "# Title\n\nHello **world**",
        "- Item 1\n- Item 2\n- Item 3",
        "1. One\n2. Two\n3. Three",
        "> Blockquote\n>\n> Second para",
        "```rust\nfn main() {}\n```",
        "Inline `code` and **bold** and *italic*",
        "[Link](https://example.com)",
        "~~strikethrough~~",
    ];

    for input in inputs {
        let document = markdown_converter
            .parse_to_ir(input.as_bytes())
            .expect("should parse markdown");

        let RenderedDocument::Text(output) = markdown_converter
            .render_from_ir(&document)
            .expect("should render to markdown")
        else {
            panic!("expected text output");
        };

        assert!(
            !output.is_empty(),
            "output should not be empty for: {input}"
        );
    }
}

#[test]
fn markdown_math_formula_parses() {
    let markdown = "Inline $E=mc^2$ and block $$\\sum_{i=1}^n x_i$$";
    let converter = MarkdownConverter;

    let document = converter
        .parse_to_ir(markdown.as_bytes())
        .expect("should parse markdown with math");

    // Render back and check math syntax is preserved
    let RenderedDocument::Text(output) = converter
        .render_from_ir(&document)
        .expect("should render to markdown")
    else {
        panic!("expected text output");
    };

    assert!(output.contains("$E=mc^2$"));
    assert!(output.contains("$$"));
    assert!(output.contains(r"\sum_{i=1}^n x_i"));
}

#[test]
fn markdown_table_parses() {
    let markdown = "| H1 | H2 |\n| --- | --- |\n| A | B |";
    let converter = MarkdownConverter;

    let document = converter
        .parse_to_ir(markdown.as_bytes())
        .expect("should parse table");

    let RenderedDocument::Text(output) = converter
        .render_from_ir(&document)
        .expect("should render table")
    else {
        panic!("expected text output");
    };

    assert!(output.contains("H1"));
    assert!(output.contains("H2"));
    assert!(output.contains("A"));
    assert!(output.contains("B"));
}

#[test]
fn markdown_code_block_parses() {
    let markdown = "```rust\nlet x = 42;\n```";
    let converter = MarkdownConverter;

    let document = converter
        .parse_to_ir(markdown.as_bytes())
        .expect("should parse code block");

    let RenderedDocument::Text(output) = converter
        .render_from_ir(&document)
        .expect("should render code block")
    else {
        panic!("expected text output");
    };

    assert!(output.contains("```rust"));
    assert!(output.contains("let x = 42;"));
}

// ── HTML → IR ────────────────────────────────────────────────────────────────

#[test]
fn html_parses_headings() {
    let html = "<body><h1>Title</h1><h2>Sub</h2></body>";
    let converter = HtmlConverter;
    let doc = converter
        .parse_to_ir(html.as_bytes())
        .expect("should parse HTML headings");

    assert_eq!(doc.nodes.len(), 2);
    assert!(matches!(&doc.nodes[0], DocNode::Heading { level: 1, .. }));
    assert!(matches!(&doc.nodes[1], DocNode::Heading { level: 2, .. }));
}

#[test]
fn html_parses_paragraph() {
    let html = "<body><p>Hello <strong>world</strong></p></body>";
    let converter = HtmlConverter;
    let doc = converter
        .parse_to_ir(html.as_bytes())
        .expect("should parse paragraph");

    assert_eq!(doc.nodes.len(), 1);
    assert!(matches!(&doc.nodes[0], DocNode::Paragraph { .. }));
}

#[test]
fn html_parses_code_block() {
    let html = "<body><pre><code class=\"language-rust\">let x = 42;</code></pre></body>";
    let converter = HtmlConverter;
    let doc = converter
        .parse_to_ir(html.as_bytes())
        .expect("should parse code block");

    assert_eq!(doc.nodes.len(), 1);
    assert!(matches!(&doc.nodes[0], DocNode::CodeBlock { .. }));
}

#[test]
fn html_parses_list() {
    let html = "<body><ul><li>Item 1</li><li>Item 2</li></ul></body>";
    let converter = HtmlConverter;
    let doc = converter
        .parse_to_ir(html.as_bytes())
        .expect("should parse list");

    assert_eq!(doc.nodes.len(), 1);
    assert!(matches!(
        &doc.nodes[0],
        DocNode::List { ordered: false, .. }
    ));
}

#[test]
fn html_parses_blockquote() {
    let html = "<body><blockquote><p>Quote</p></blockquote></body>";
    let converter = HtmlConverter;
    let doc = converter
        .parse_to_ir(html.as_bytes())
        .expect("should parse blockquote");

    assert_eq!(doc.nodes.len(), 1);
    assert!(matches!(&doc.nodes[0], DocNode::BlockQuote { .. }));
}

#[test]
fn html_parses_table() {
    let html = "<body><table><thead><tr><th>H1</th></tr></thead><tbody><tr><td>V1</td></tr></tbody></table></body>";
    let converter = HtmlConverter;
    let doc = converter
        .parse_to_ir(html.as_bytes())
        .expect("should parse table");

    assert_eq!(doc.nodes.len(), 1);
    assert!(matches!(&doc.nodes[0], DocNode::Table { .. }));
}

#[test]
fn html_parses_horizontal_rule() {
    let html = "<body><hr></body>";
    let converter = HtmlConverter;
    let doc = converter
        .parse_to_ir(html.as_bytes())
        .expect("should parse hr");

    assert_eq!(doc.nodes.len(), 1);
    assert!(matches!(&doc.nodes[0], DocNode::HorizontalRule));
}

#[test]
fn html_parses_math_block() {
    let html = "<body><div class=\"math\">E=mc^2</div></body>";
    let converter = HtmlConverter;
    let doc = converter
        .parse_to_ir(html.as_bytes())
        .expect("should parse math block");

    assert_eq!(doc.nodes.len(), 1);
    assert!(matches!(&doc.nodes[0], DocNode::MathBlock { .. }));
}

#[test]
fn html_inline_elements_preserved() {
    let html = "<body><p><em>italic</em>, <strong>bold</strong>, <code>code</code>, <del>del</del>, <a href=\"http://x.com\">link</a>, <span class=\"math\">x+1</span></p></body>";
    let converter = HtmlConverter;
    let doc = converter
        .parse_to_ir(html.as_bytes())
        .expect("should parse inline elements");

    assert_eq!(doc.nodes.len(), 1);
    // Round-trip through markdown to verify inline elements survived
    let md = MarkdownConverter;
    let RenderedDocument::Text(output) = md.render_from_ir(&doc).unwrap() else {
        panic!("expected text");
    };
    assert!(output.contains("*italic*"));
    assert!(output.contains("**bold**"));
    assert!(output.contains("`code`"));
    assert!(output.contains("~~del~~"));
    assert!(output.contains("[link](http://x.com)"));
    assert!(output.contains("$x+1$"));
}
