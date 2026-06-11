use comrak::nodes::{AstNode, ListType, NodeValue};
use comrak::{Arena, Options};

use crate::core::converter::{Converter, ConverterError, RenderedDocument};
use crate::core::ir::{DocNode, Document, InlineNode};

#[derive(Debug, Default)]
pub struct MarkdownConverter;

impl MarkdownConverter {
    pub fn markdown_to_html(input: &str) -> String {
        comrak::markdown_to_html(input, &Options::default())
    }
}

impl Converter for MarkdownConverter {
    fn parse_to_ir(&self, input: &[u8]) -> Result<Document, ConverterError> {
        let source = std::str::from_utf8(input).map_err(|_| ConverterError::InvalidUtf8)?;
        let arena = Arena::new();
        let root = comrak::parse_document(&arena, source, &Options::default());

        let nodes: Vec<DocNode> = root.children().filter_map(parse_doc_node).collect();
        Ok(Document::new(nodes))
    }

    fn render_from_ir(&self, document: &Document) -> Result<RenderedDocument, ConverterError> {
        let mut output = String::new();
        for node in &document.nodes {
            output.push_str(&render_node_to_string(node));
        }
        Ok(RenderedDocument::Text(output.trim_end().to_owned()))
    }
}

// ── Parser ────────────────────────────────────────────────────────────────────
fn parse_doc_node<'a>(node: &'a AstNode<'a>) -> Option<DocNode> {
    match &node.data.borrow().value {
        NodeValue::Heading(h) => {
            let content = collect_inlines(node);
            Some(DocNode::Heading {
                level: h.level,
                content,
            })
        }
        NodeValue::Paragraph => {
            let content = collect_inlines(node);
            Some(DocNode::Paragraph { content })
        }
        NodeValue::CodeBlock(cb) => {
            let language = if cb.info.is_empty() {
                None
            } else {
                Some(cb.info.clone())
            };
            Some(DocNode::CodeBlock {
                language,
                code: cb.literal.clone(),
            })
        }
        NodeValue::BlockQuote => {
            let content: Vec<DocNode> = node.children().filter_map(parse_doc_node).collect();
            Some(DocNode::BlockQuote { content })
        }
        NodeValue::List(list) => {
            let ordered = matches!(list.list_type, ListType::Ordered);
            let items: Vec<Vec<DocNode>> = node
                .children()
                .map(|item_node| {
                    item_node
                        .children()
                        .filter_map(parse_doc_node)
                        .collect()
                })
                .collect();
            Some(DocNode::List { ordered, items })
        }
        NodeValue::Table(_) => {
            let mut headers: Vec<Vec<InlineNode>> = Vec::new();
            let mut rows: Vec<Vec<Vec<InlineNode>>> = Vec::new();
            for row_node in node.children() {
                let is_header = match &row_node.data.borrow().value {
                    NodeValue::TableRow(is_h) => *is_h,
                    _ => false,
                };
                let cells: Vec<Vec<InlineNode>> = row_node
                    .children()
                    .map(|cell| collect_inlines(&cell))
                    .collect();
                if is_header {
                    headers = cells;
                } else {
                    rows.push(cells);
                }
            }
            Some(DocNode::Table { headers, rows })
        }
        NodeValue::ThematicBreak => Some(DocNode::HorizontalRule),
        NodeValue::Math(math) => {
            if math.display_math {
                Some(DocNode::MathBlock {
                    code: math.literal.clone(),
                })
            } else {
                // 行内数学公式出现在块级位置，作为段落包裹
                Some(DocNode::Paragraph {
                    content: vec![InlineNode::MathInline(math.literal.clone())],
                })
            }
        }
        // 未知或不重要的块级节点（如 FrontMatter、HtmlBlock 等），
        // 将其子内容作为段落提取，避免信息丢失
        _ => {
            let content = collect_inlines(node);
            if content.is_empty() {
                None
            } else {
                Some(DocNode::Paragraph { content })
            }
        }
    }
}
fn collect_inlines<'a>(node: &'a AstNode<'a>) -> Vec<InlineNode> {
    let mut result = Vec::new();
    // Clone NodeValue to avoid lifetime issues with nested borrows
    let children: Vec<_> = node.children().collect();
    for child in &children {
        match child.data.borrow().value.clone() {
            NodeValue::Text(t) => result.push(InlineNode::Text(t.to_string())),
            NodeValue::Code(c) => result.push(InlineNode::Code(c.literal)),
            NodeValue::Emph => {
                result.push(InlineNode::Emphasis(collect_inlines(child)));
            }
            NodeValue::Strong => {
                result.push(InlineNode::Strong(collect_inlines(child)));
            }
            NodeValue::Strikethrough => {
                result.push(InlineNode::Strikethrough(collect_inlines(child)));
            }
            NodeValue::Link(link) => {
                result.push(InlineNode::Link {
                    text: collect_inlines(child),
                    url: link.url.clone(),
                });
            }
            NodeValue::Math(math) => {
                if math.display_math {
                    // Block math inside inline context — wrap as inline fallback
                    result.push(InlineNode::MathInline(math.literal));
                } else {
                    result.push(InlineNode::MathInline(math.literal));
                }
            }
            NodeValue::SoftBreak => result.push(InlineNode::SoftBreak),
            NodeValue::LineBreak => result.push(InlineNode::HardBreak),
            _ => {}
        }
    }
    result
}

// ── Renderer ──────────────────────────────────────────────────────────────────

fn render_node_to_string(node: &DocNode) -> String {
    let mut output = String::new();
    match node {
        DocNode::Heading { level, content } => {
            output.push_str(&"#".repeat((*level).into()));
            output.push(' ');
            output.push_str(&render_inlines(content));
            output.push_str("\n\n");
        }
        DocNode::Paragraph { content } => {
            output.push_str(&render_inlines(content));
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
        DocNode::BlockQuote { content } => {
            for child in content {
                let child_rendered = render_node_to_string(child);
                for line in child_rendered.trim_end().lines() {
                    if line.is_empty() {
                        output.push_str(">\n");
                    } else {
                        output.push_str("> ");
                        output.push_str(line);
                        output.push('\n');
                    }
                }
            }
            output.push('\n');
        }
        DocNode::List { ordered, items } => {
            for (i, item) in items.iter().enumerate() {
                let marker = if *ordered {
                    format!("{}. ", i + 1)
                } else {
                    "* ".to_string()
                };
                let indent = " ".repeat(marker.len());
                for (j, node) in item.iter().enumerate() {
                    let child_rendered = render_node_to_string(node);
                    let mut lines = child_rendered.trim_end().lines();
                    if let Some(first_line) = lines.next() {
                        if j == 0 {
                            output.push_str(&marker);
                        } else {
                            output.push_str(&indent);
                        }
                        output.push_str(first_line);
                        output.push('\n');
                    }
                    for line in lines {
                        output.push_str(&indent);
                        output.push_str(line);
                        output.push('\n');
                    }
                }
            }
            output.push('\n');
        }
        DocNode::Table { headers, rows } => {
            let header_line = headers
                .iter()
                .map(|cell| render_inlines(cell))
                .collect::<Vec<_>>()
                .join(" | ");
            output.push_str(&header_line);
            output.push('\n');

            let separator_line = headers.iter().map(|_| "---").collect::<Vec<_>>().join(" | ");
            output.push_str(&separator_line);
            output.push('\n');

            for row in rows {
                let row_line = row
                    .iter()
                    .map(|cell| render_inlines(cell))
                    .collect::<Vec<_>>()
                    .join(" | ");
                output.push_str(&row_line);
                output.push('\n');
            }
            output.push('\n');
        }
        DocNode::MathBlock { code } => {
            output.push_str("$$\n");
            output.push_str(code);
            output.push_str("\n$$\n\n");
        }
    }
    output
}

fn render_inlines(inlines: &[InlineNode]) -> String {
    let mut output = String::new();

    for inline in inlines {
        match inline {
            InlineNode::Text(text) => output.push_str(text),
            InlineNode::Code(code) => {
                output.push('`');
                output.push_str(code);
                output.push('`');
            }
            InlineNode::SoftBreak => output.push('\n'),
            InlineNode::HardBreak => output.push_str("  \n"),
            InlineNode::Emphasis(children) => {
                output.push('*');
                output.push_str(&render_inlines(children));
                output.push('*');
            }
            InlineNode::Strong(children) => {
                output.push_str("**");
                output.push_str(&render_inlines(children));
                output.push_str("**");
            }
            InlineNode::Strikethrough(children) => {
                output.push_str("~~");
                output.push_str(&render_inlines(children));
                output.push_str("~~");
            }
            InlineNode::Link { text, url } => {
                output.push('[');
                output.push_str(&render_inlines(text));
                output.push(']');
                output.push('(');
                output.push_str(url);
                output.push(')');
            }
            InlineNode::MathInline(code) => {
                output.push('$');
                output.push_str(code);
                output.push('$');
            }
        }
    }

    output
}
