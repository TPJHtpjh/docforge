use docforge::converters::html::HtmlConverter;
use docforge::converters::markdown::MarkdownConverter;
use docforge::core::converter::{Converter, RenderedDocument};

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
