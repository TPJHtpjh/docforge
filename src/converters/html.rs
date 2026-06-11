use crate::core::converter::{Converter, ConverterError, RenderedDocument};
use crate::core::ir::{DocNode, Document, InlineNode};
use kuchikiki::traits::*;

#[derive(Debug, Default)]
pub struct HtmlConverter;

impl Converter for HtmlConverter {
    fn parse_to_ir(&self, input: &[u8]) -> Result<Document, ConverterError> {
        let source = std::str::from_utf8(input).map_err(|_| ConverterError::InvalidUtf8)?;
        let document = kuchikiki::parse_html().one(source);

        let nodes = parse_root_block_children(&document);
        Ok(Document::new(nodes))
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
                DocNode::Table { headers, rows } => {
                    output.push_str("<table>\n<thead>\n<tr>");
                    for header in headers {
                        output.push_str(&format!("<th>{}</th>", render_inline_nodes(header)));
                    }
                    output.push_str("</tr>\n</thead>\n<tbody>\n");
                    for row in rows {
                        output.push_str("<tr>");
                        for cell in row {
                            output.push_str(&format!("<td>{}</td>", render_inline_nodes(cell)));
                        }
                        output.push_str("</tr>\n");
                    }
                    output.push_str("</tbody>\n</table>\n");
                }
                DocNode::MathBlock { code } => {
                    output.push_str(&format!(
                        "<div class=\"math\">{}</div>\n",
                        escape_html(code)
                    ));
                }
            }
        }

        Ok(RenderedDocument::Text(output.trim_end().to_owned()))
    }
}

// ── HTML → IR Parser ─────────────────────────────────────────────────────────

/// Walk from the document root to find `<body>`, then parse its block children.
fn parse_root_block_children(document: &kuchikiki::NodeRef) -> Vec<DocNode> {
    for child in document.children() {
        if let Some(el) = child.as_element() {
            if el.name.local.as_ref() == "body" {
                return parse_block_children(&child);
            }
            let children = parse_root_block_children(&child);
            if !children.is_empty() {
                return children;
            }
        }
    }
    Vec::new()
}

/// Parse direct children of a block-level container into `Vec<DocNode>`.
fn parse_block_children(container: &kuchikiki::NodeRef) -> Vec<DocNode> {
    let mut nodes = Vec::new();
    for child in container.children() {
        if let Some(node) = block_node_from_element(&child) {
            nodes.push(node);
        }
    }
    nodes
}

/// Convert a DOM node to `DocNode` if it is a recognized block-level element.
/// Unknown elements fall back to a Paragraph preserving their text content.
fn block_node_from_element(node: &kuchikiki::NodeRef) -> Option<DocNode> {
    let element = node.as_element()?;
    let tag = element.name.local.as_ref();

    match tag {
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
            let level = match tag {
                "h1" => 1,
                "h2" => 2,
                "h3" => 3,
                "h4" => 4,
                "h5" => 5,
                _ => 6,
            };
            let content = collect_inline_children(node);
            Some(DocNode::Heading { level, content })
        }
        "p" => {
            let content = collect_inline_children(node);
            if content.is_empty() {
                None
            } else {
                Some(DocNode::Paragraph { content })
            }
        }
        "pre" => parse_code_block(node),
        "blockquote" => {
            let content = parse_block_children(node);
            Some(DocNode::BlockQuote { content })
        }
        "ul" => Some(parse_list(node, false)),
        "ol" => Some(parse_list(node, true)),
        "table" => Some(parse_table(node)),
        "hr" => Some(DocNode::HorizontalRule),
        "div" => {
            let class = element.attributes.borrow();
            let is_math = class.get("class").map(|v| v.contains("math")).unwrap_or(false);
            drop(class);
            if is_math {
                let text = node.text_contents();
                Some(DocNode::MathBlock {
                    code: text.trim().to_string(),
                })
            } else {
                fallback_block_container(node)
            }
        }
        // Semantic container elements: recurse and fall back to text
        "section" | "article" | "main" | "header" | "footer" | "nav" | "aside" => {
            fallback_block_container(node)
        }
        // Unknown block-level elements: preserve as Paragraph to avoid data loss
        _ => {
            let text = node.text_contents().trim().to_string();
            if text.is_empty() {
                None
            } else {
                Some(DocNode::Paragraph {
                    content: vec![InlineNode::Text(text)],
                })
            }
        }
    }
}

/// For container elements: try to parse block children; if nothing recognized
/// or too many children, fall back to raw text as a Paragraph.
fn fallback_block_container(node: &kuchikiki::NodeRef) -> Option<DocNode> {
    let children = parse_block_children(node);
    if children.len() == 1 {
        return children.into_iter().next();
    }
    if children.len() > 1 {
        // Multiple recognized children — flatten text into a single Paragraph
        let text = node.text_contents().trim().to_string();
        if !text.is_empty() {
            return Some(DocNode::Paragraph {
                content: vec![InlineNode::Text(text)],
            });
        }
    }
    // No recognized block children; use raw text content if present
    let text = node.text_contents().trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(DocNode::Paragraph {
            content: vec![InlineNode::Text(text)],
        })
    }
}

/// Parse `<pre>` into `CodeBlock`, extracting language from `<code class="language-xxx">`.
fn parse_code_block(pre_node: &kuchikiki::NodeRef) -> Option<DocNode> {
    let mut language = None;
    let code_text: String;

    for child in pre_node.children() {
        if let Some(el) = child.as_element() {
            if el.name.local.as_ref() == "code" {
                let attrs = el.attributes.borrow();
                if let Some(class_val) = attrs.get("class") {
                    for part in class_val.split_whitespace() {
                        if let Some(lang) = part.strip_prefix("language-") {
                            language = Some(lang.to_string());
                            break;
                        }
                    }
                }
                code_text = child.text_contents();
                return Some(DocNode::CodeBlock {
                    language,
                    code: code_text.trim().to_string(),
                });
            }
        }
    }

    // No <code> found, use raw text content of <pre>
    code_text = pre_node.text_contents();
    Some(DocNode::CodeBlock {
        language: None,
        code: code_text.trim().to_string(),
    })
}

/// Parse `<ul>` or `<ol>` into `DocNode::List`.
fn parse_list(list_node: &kuchikiki::NodeRef, ordered: bool) -> DocNode {
    let items: Vec<Vec<DocNode>> = list_node
        .children()
        .filter_map(|child| {
            if child
                .as_element()
                .map(|e| e.name.local.as_ref() == "li")
                .unwrap_or(false)
            {
                Some(parse_block_children(&child))
            } else {
                None
            }
        })
        .collect();

    DocNode::List { ordered, items }
}

/// Parse `<table>` into `DocNode::Table`.
fn parse_table(table_node: &kuchikiki::NodeRef) -> DocNode {
    let mut headers: Vec<Vec<InlineNode>> = Vec::new();
    let mut rows: Vec<Vec<Vec<InlineNode>>> = Vec::new();

    for child in table_node.children() {
        if let Some(el) = child.as_element() {
            let tag = el.name.local.as_ref();
            match tag {
                "thead" | "tbody" | "tfoot" => {
                    for row_node in child.children() {
                        if row_node
                            .as_element()
                            .map(|e| e.name.local.as_ref() == "tr")
                            .unwrap_or(false)
                        {
                            let row_data = parse_table_row(&row_node);
                            if tag == "thead" || tag == "tfoot" {
                                if headers.is_empty() {
                                    headers = row_data;
                                }
                            } else {
                                rows.push(row_data);
                            }
                        }
                    }
                }
                "tr" => {
                    let row_data = parse_table_row(&child);
                    rows.push(row_data);
                }
                // Non-content table children (`<colgroup>`, `<caption>`, etc.) are skipped;
                // text content inside them is not meaningful in table context.
                _ => {}
            }
        }
    }

    DocNode::Table { headers, rows }
}

/// Parse a single `<tr>` row into cells: each `<th>` or `<td>` becomes a Vec<InlineNode>.
fn parse_table_row(tr_node: &kuchikiki::NodeRef) -> Vec<Vec<InlineNode>> {
    let mut cells = Vec::new();
    for cell_node in tr_node.children() {
        if let Some(el) = cell_node.as_element() {
            let tag = el.name.local.as_ref();
            if tag == "th" || tag == "td" {
                cells.push(collect_inline_children(&cell_node));
            }
        }
    }
    cells
}

/// Collect inline nodes from all children of this node (text + inline elements).
fn collect_inline_children(parent: &kuchikiki::NodeRef) -> Vec<InlineNode> {
    let mut inlines = Vec::new();
    for child in parent.children() {
        match inline_node_from_child(&child) {
            Some(InlineNode::Text(ref t)) if t.trim().is_empty() => {
                // Skip whitespace-only text nodes between block elements
            }
            Some(node) => inlines.push(node),
            None => {}
        }
    }
    trim_whitespace_edges(&mut inlines);
    inlines
}

/// Trim leading and trailing whitespace-only Text nodes.
fn trim_whitespace_edges(inlines: &mut Vec<InlineNode>) {
    while inlines
        .last()
        .map(|n| matches!(n, InlineNode::Text(t) if t.trim().is_empty()))
        .unwrap_or(false)
    {
        inlines.pop();
    }
    while inlines
        .first()
        .map(|n| matches!(n, InlineNode::Text(t) if t.trim().is_empty()))
        .unwrap_or(false)
    {
        inlines.remove(0);
    }
}

/// Convert a single DOM child node to an InlineNode, or None if not inline content.
fn inline_node_from_child(node: &kuchikiki::NodeRef) -> Option<InlineNode> {
    if let Some(text_ref) = node.as_text() {
        let text = text_ref.borrow().clone();
        return Some(InlineNode::Text(text));
    }

    let element = node.as_element()?;
    let tag = element.name.local.as_ref();

    match tag {
        "em" | "i" => {
            let children = collect_inline_children(node);
            Some(InlineNode::Emphasis(children))
        }
        "strong" | "b" => {
            let children = collect_inline_children(node);
            Some(InlineNode::Strong(children))
        }
        "del" | "s" | "strike" => {
            let children = collect_inline_children(node);
            Some(InlineNode::Strikethrough(children))
        }
        "code" => {
            let text = node.text_contents();
            Some(InlineNode::Code(text))
        }
        "a" => {
            let attrs = element.attributes.borrow();
            let url = attrs.get("href").unwrap_or("").to_string();
            drop(attrs);
            let children = collect_inline_children(node);
            Some(InlineNode::Link {
                text: children,
                url,
            })
        }
        "br" => Some(InlineNode::HardBreak),
        "span" => {
            let attrs = element.attributes.borrow();
            let is_math = attrs.get("class").map(|v| v.contains("math")).unwrap_or(false);
            drop(attrs);
            if is_math {
                let text = node.text_contents();
                Some(InlineNode::MathInline(text.trim().to_string()))
            } else {
                // Unknown span: recurse into inline children
                let mut children = collect_inline_children(node);
                match children.len() {
                    0 => None,
                    1 => Some(children.remove(0)),
                    _ => {
                        let text = node.text_contents();
                        Some(InlineNode::Text(text))
                    }
                }
            }
        }
        // Non-content elements: skip
        "script" | "style" | "meta" | "link" => None,
        // Unknown inline elements: preserve text content to avoid data loss
        _ => {
            let text = node.text_contents();
            if text.trim().is_empty() {
                None
            } else {
                Some(InlineNode::Text(text))
            }
        }
    }
}

// ── IR → HTML Renderer ───────────────────────────────────────────────────────

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
            InlineNode::MathInline(code) => {
                output.push_str("<span class=\"math\">");
                output.push_str(&escape_html(code));
                output.push_str("</span>");
            }
            InlineNode::Strikethrough(children) => {
                output.push_str("<del>");
                output.push_str(&render_inline_nodes(children));
                output.push_str("</del>");
            }
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
