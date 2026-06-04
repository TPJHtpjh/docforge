use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd, html};

use crate::core::converter::{Converter, ConverterError, RenderedDocument};
use crate::core::ir::{DocNode, Document, InlineNode};

#[derive(Debug, Default)]
pub struct MarkdownConverter;

impl MarkdownConverter {
    pub fn markdown_to_html(input: &str) -> String {
        let parser = Parser::new_ext(input, Options::all());
        let mut output = String::new();
        html::push_html(&mut output, parser);
        output
    }
}

impl Converter for MarkdownConverter {
    fn parse_to_ir(&self, input: &[u8]) -> Result<Document, ConverterError> {
        let source = std::str::from_utf8(input).map_err(|_| ConverterError::InvalidUtf8)?;
        let parser = Parser::new_ext(source, Options::all());

        let mut nodes = Vec::new();
        let mut paragraph = Vec::new();
        let mut heading: Option<(u8, Vec<InlineNode>)> = None;

        for event in parser {
            match event {
                Event::Start(Tag::Paragraph) => {
                    paragraph.clear();
                }
                Event::End(TagEnd::Paragraph) if !paragraph.is_empty() => {
                    nodes.push(DocNode::Paragraph {
                        content: paragraph.clone(),
                    });
                    paragraph.clear();
                }
                Event::Start(Tag::Heading { level, .. }) => {
                    heading = Some((heading_level_to_u8(level), Vec::new()));
                }
                Event::End(TagEnd::Heading(_)) => {
                    if let Some((level, content)) = heading.take() {
                        nodes.push(DocNode::Heading { level, content });
                    }
                }
                Event::Text(text) => {
                    push_inline(
                        &mut paragraph,
                        &mut heading,
                        InlineNode::Text(text.into_string()),
                    );
                }
                Event::Code(code) => {
                    push_inline(
                        &mut paragraph,
                        &mut heading,
                        InlineNode::Code(code.into_string()),
                    );
                }
                Event::SoftBreak => {
                    push_inline(&mut paragraph, &mut heading, InlineNode::SoftBreak);
                }
                Event::HardBreak => {
                    push_inline(&mut paragraph, &mut heading, InlineNode::HardBreak);
                }
                _ => {}
            }
        }

        Ok(Document::new(nodes))
    }

    fn render_from_ir(&self, document: &Document) -> Result<RenderedDocument, ConverterError> {
        let mut output = String::new();

        for node in &document.nodes {
            match node {
                DocNode::Heading { level, content } => {
                    output.push_str(&"#".repeat((*level).into()));
                    output.push(' ');
                    output.push_str(&render_inlines_as_text(content));
                    output.push_str("\n\n");
                }
                DocNode::Paragraph { content } => {
                    output.push_str(&render_inlines_as_text(content));
                    output.push_str("\n\n");
                }
                DocNode::CodeBlock { language, code } => {
                    output.push_str("```");
                    if let Some(language) = language {
                        output.push_str(language);
                    }
                    output.push('\n');
                    output.push_str(code);
                    output.push_str("\n```\n\n");
                }
                DocNode::HorizontalRule => {
                    output.push_str("---\n\n");
                }
                DocNode::BlockQuote { .. } | DocNode::List { .. } => {
                    return Err(ConverterError::Render(
                        "rendering for this node type is not implemented in markdown converter"
                            .to_string(),
                    ));
                }
            }
        }

        Ok(RenderedDocument::Text(output.trim_end().to_owned()))
    }
}

fn push_inline(
    paragraph: &mut Vec<InlineNode>,
    heading: &mut Option<(u8, Vec<InlineNode>)>,
    inline: InlineNode,
) {
    if let Some((_, heading_content)) = heading.as_mut() {
        heading_content.push(inline);
    } else {
        paragraph.push(inline);
    }
}

fn heading_level_to_u8(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn render_inlines_as_text(inlines: &[InlineNode]) -> String {
    let mut output = String::new();

    for inline in inlines {
        match inline {
            InlineNode::Text(text) | InlineNode::Code(text) => output.push_str(text),
            InlineNode::SoftBreak => output.push('\n'),
            InlineNode::HardBreak => output.push_str("  \n"),
            InlineNode::Emphasis(children) | InlineNode::Strong(children) => {
                output.push_str(&render_inlines_as_text(children));
            }
            InlineNode::Link { text, .. } => {
                output.push_str(&render_inlines_as_text(text));
            }
        }
    }

    output
}
