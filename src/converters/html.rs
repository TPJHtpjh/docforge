use crate::core::converter::{Converter, ConverterError, RenderedDocument};
use crate::core::ir::{DocNode, Document, InlineNode};

#[derive(Debug, Default)]
pub struct HtmlConverter;

impl Converter for HtmlConverter {
    fn parse_to_ir(&self, _input: &[u8]) -> Result<Document, ConverterError> {
        // Intentionally a structural stub; HTML traversal rules can be layered in later.
        todo!("Implement specific logic")
    }

    fn render_from_ir(&self, document: &Document) -> Result<RenderedDocument, ConverterError> {
        let mut output = String::new();

        for node in &document.nodes {
            match node {
                DocNode::Heading { level, content } => {
                    output.push_str(&format!(
                        "<h{level}>{}</h{level}>\n",
                        render_inline_nodes(content)
                    ));
                }
                DocNode::Paragraph { content } => {
                    output.push_str(&format!("<p>{}</p>\n", render_inline_nodes(content)));
                }
                DocNode::CodeBlock { language, code } => {
                    if let Some(language) = language {
                        output.push_str(&format!(
                            "<pre><code class=\"language-{language}\">{}</code></pre>\n",
                            escape_html(code)
                        ));
                    } else {
                        output
                            .push_str(&format!("<pre><code>{}</code></pre>\n", escape_html(code)));
                    }
                }
                DocNode::HorizontalRule => {
                    output.push_str("<hr />\n");
                }
                DocNode::BlockQuote { content } => {
                    let nested = render_block_nodes(content)?;
                    output.push_str(&format!("<blockquote>\n{nested}</blockquote>\n"));
                }
                DocNode::List { ordered, items } => {
                    let tag = if *ordered { "ol" } else { "ul" };
                    output.push_str(&format!("<{tag}>\n"));
                    for item in items {
                        output.push_str("<li>");
                        output.push_str(&render_block_nodes(item)?);
                        output.push_str("</li>\n");
                    }
                    output.push_str(&format!("</{tag}>\n"));
                }
            }
        }

        Ok(RenderedDocument::Text(output.trim_end().to_owned()))
    }
}

fn render_block_nodes(nodes: &[DocNode]) -> Result<String, ConverterError> {
    let nested_doc = Document::new(nodes.to_vec());
    let rendered = HtmlConverter.render_from_ir(&nested_doc)?;

    if let RenderedDocument::Text(text) = rendered {
        Ok(text)
    } else {
        Err(ConverterError::Render(
            "unexpected binary output when rendering HTML".to_string(),
        ))
    }
}

fn render_inline_nodes(inlines: &[InlineNode]) -> String {
    let mut output = String::new();

    for inline in inlines {
        match inline {
            InlineNode::Text(text) => output.push_str(&escape_html(text)),
            InlineNode::Code(code) => {
                output.push_str("<code>");
                output.push_str(&escape_html(code));
                output.push_str("</code>");
            }
            InlineNode::Emphasis(children) => {
                output.push_str("<em>");
                output.push_str(&render_inline_nodes(children));
                output.push_str("</em>");
            }
            InlineNode::Strong(children) => {
                output.push_str("<strong>");
                output.push_str(&render_inline_nodes(children));
                output.push_str("</strong>");
            }
            InlineNode::Link { text, url } => {
                output.push_str("<a href=\"");
                output.push_str(&escape_html(url));
                output.push_str("\">");
                output.push_str(&render_inline_nodes(text));
                output.push_str("</a>");
            }
            InlineNode::SoftBreak => output.push('\n'),
            InlineNode::HardBreak => output.push_str("<br />"),
        }
    }

    output
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}
