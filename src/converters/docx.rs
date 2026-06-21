use crate::core::converter::{Converter, ConverterError, RenderedDocument};
use crate::core::ir::{DocNode, Document, InlineNode};

#[derive(Debug, Default)]
pub struct DocxConverter;

impl Converter for DocxConverter {
    fn parse_to_ir(&self, input: &[u8]) -> Result<Document, ConverterError> {
        let docx_json = docx_rs::read_docx(input)
            .map_err(|e| ConverterError::Parse(format!("docx-rs error: {e}")))?;

        let json_str = docx_json.json();
        let json: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| ConverterError::Parse(format!("JSON parse error: {e}")))?;

        // Extract top-level images array: [[id, path, base64_data, base64_png], ...]
        let images: Vec<&serde_json::Value> = json
            .get("images")
            .and_then(|i| i.as_array())
            .map(|arr| arr.iter().collect())
            .unwrap_or_default();

        let children = json
            .get("document")
            .and_then(|d| d.get("children"))
            .and_then(|c| c.as_array())
            .ok_or_else(|| ConverterError::Parse("missing document.children".into()))?;

        let nodes = children
            .iter()
            .filter_map(|node| parse_top_level_node(node, &images))
            .collect::<Vec<_>>();

        Ok(Document::new(nodes))
    }

    fn render_from_ir(&self, document: &Document) -> Result<RenderedDocument, ConverterError> {
        let mut docx = docx_rs::Docx::new();

        // Set A4 page size with reasonable margins
        docx = docx.page_size(11906, 16838).page_margin(
            docx_rs::PageMargin::new()
                .top(1440)
                .bottom(1440)
                .left(1800)
                .right(1800)
                .header(720)
                .footer(720)
                .gutter(0),
        );

        // Add heading styles
        for level in 1..=6 {
            let style_name = format!("Heading{level}");
            docx = docx.add_style(
                docx_rs::Style::new(&style_name, docx_rs::StyleType::Paragraph)
                    .name(&format!("Heading {level}"))
                    .bold()
                    .size((32 - level as u32 * 4) as usize),
            );
        }

        for node in &document.nodes {
            match node {
                DocNode::Heading { level, content } => {
                    let style_name = format!("Heading{level}");
                    let runs = inlines_to_runs(content);
                    let mut para = docx_rs::Paragraph::new().style(&style_name);
                    for run in runs {
                        para = para.add_run(run);
                    }
                    docx = docx.add_paragraph(para);
                }
                DocNode::Paragraph { content } => {
                    if content.is_empty() {
                        continue;
                    }
                    let runs = inlines_to_runs(content);
                    let mut para = docx_rs::Paragraph::new();
                    for run in runs {
                        para = para.add_run(run);
                    }
                    docx = docx.add_paragraph(para);
                }
                DocNode::CodeBlock { code, .. } => {
                    // Render code block with background shading for visual distinction
                    let mut para = docx_rs::Paragraph::new();
                    let lines: Vec<&str> = code.lines().collect();
                    for (i, &line) in lines.iter().enumerate() {
                        let shading = docx_rs::Shading::new().color("auto").fill("F0F0F0");
                        let run = docx_rs::Run::new()
                            .add_text(line)
                            .fonts(docx_rs::RunFonts::new().ascii("Consolas"))
                            .size(20)
                            .color("333333")
                            .shading(shading);
                        para = para.add_run(run);
                        // Add newline after each line except the last
                        if i < lines.len() - 1 {
                            para = para.add_run(
                                docx_rs::Run::new().add_break(docx_rs::BreakType::TextWrapping),
                            );
                        }
                    }
                    docx = docx.add_paragraph(para);
                }
                DocNode::BlockQuote { content } => {
                    docx = render_blockquote(docx, content, 0);
                }
                DocNode::List { ordered, items } => {
                    docx = render_list(docx, items, *ordered, 0);
                }
                DocNode::Table { headers, rows } => {
                    docx = render_table(docx, headers, rows);
                }
                DocNode::HorizontalRule => {
                    // Render as a thin bottom-border paragraph (visual horizontal rule)
                    let mut para = docx_rs::Paragraph::new();
                    let run = docx_rs::Run::new()
                        .add_text("─".repeat(50))
                        .color("999999")
                        .size(18);
                    para = para.add_run(run);
                    docx = docx.add_paragraph(para);
                }
                DocNode::MathBlock { code } => {
                    // Render math block with Cambria Math font for proper math display
                    let run = docx_rs::Run::new()
                        .add_text(code)
                        .fonts(docx_rs::RunFonts::new().ascii("Cambria Math"))
                        .size(24)
                        .color("000000");
                    let para = docx_rs::Paragraph::new()
                        .align(docx_rs::AlignmentType::Center)
                        .add_run(run);
                    docx = docx.add_paragraph(para);
                }
            }
        }

        // Build to a Vec<u8> buffer using Cursor (provides Seek)
        let mut buf = std::io::Cursor::new(Vec::new());
        docx.build()
            .pack(&mut buf)
            .map_err(|e| ConverterError::Render(format!("DOCX pack error: {e}")))?;

        Ok(RenderedDocument::Binary(buf.into_inner()))
    }
}

fn parse_top_level_node(
    node: &serde_json::Value,
    images: &[&serde_json::Value],
) -> Option<DocNode> {
    let node_type = node.get("type")?.as_str()?;
    let data = node.get("data")?;

    match node_type {
        "paragraph" => parse_paragraph(data, images),
        "table" => parse_table(data, images),
        _ => None,
    }
}

fn parse_paragraph(data: &serde_json::Value, images: &[&serde_json::Value]) -> Option<DocNode> {
    let children = data.get("children").and_then(|c| c.as_array())?;

    let property = data.get("property");
    let heading_level = property
        .and_then(|p| p.get("style"))
        .and_then(|s| s.as_str())
        .and_then(|style_name| match style_name {
            "1" => Some(1u8),
            "2" => Some(2),
            "3" => Some(3),
            "4" => Some(4),
            _ => {
                if style_name.starts_with("Heading") {
                    style_name
                        .strip_prefix("Heading")
                        .and_then(|n| n.parse::<u8>().ok())
                } else if style_name.starts_with("heading") {
                    style_name
                        .strip_prefix("heading")
                        .and_then(|n| n.parse::<u8>().ok())
                } else {
                    None
                }
            }
        });

    let content: Vec<InlineNode> = children
        .iter()
        .filter_map(|node| parse_inline_node(node, images))
        .collect();

    if content.is_empty() {
        return None;
    }

    if let Some(level) = heading_level {
        Some(DocNode::Heading { level, content })
    } else {
        Some(DocNode::Paragraph { content })
    }
}

fn parse_inline_node(
    node: &serde_json::Value,
    images: &[&serde_json::Value],
) -> Option<InlineNode> {
    let node_type = node.get("type")?.as_str()?;
    let data = node.get("data")?;

    match node_type {
        "run" => parse_run(data, images),
        "hyperlink" => parse_hyperlink(data, images),
        "drawing" => parse_drawing(data, images),
        "bookmarkStart" | "bookmarkEnd" => None,
        _ => None,
    }
}

fn parse_run(data: &serde_json::Value, images: &[&serde_json::Value]) -> Option<InlineNode> {
    let run_property = data.get("runProperty");
    let is_bold = run_property
        .and_then(|p| p.get("bold"))
        .and_then(|b| b.as_bool())
        .unwrap_or(false);
    let is_italic = run_property
        .and_then(|p| p.get("italic"))
        .and_then(|b| b.as_bool())
        .unwrap_or(false);
    let is_strikethrough = run_property
        .and_then(|p| p.get("strike"))
        .and_then(|b| b.as_bool())
        .unwrap_or(false);

    let children = data.get("children").and_then(|c| c.as_array())?;

    // Check if this run contains a drawing (image) — if so, return the image directly
    for child in children {
        if child.get("type").and_then(|t| t.as_str()) == Some("drawing") {
            if let Some(img) = parse_drawing(child.get("data").unwrap_or(child), images) {
                return Some(img);
            }
        }
    }

    let mut text = String::new();
    for child in children {
        if child.get("type").and_then(|t| t.as_str()) == Some("text") {
            if let Some(t) = child
                .get("data")
                .and_then(|d| d.get("text"))
                .and_then(|t| t.as_str())
            {
                text.push_str(t);
            }
        }
    }

    if text.is_empty() {
        return None;
    }

    if is_bold {
        Some(InlineNode::Strong(vec![InlineNode::Text(text)]))
    } else if is_italic {
        Some(InlineNode::Emphasis(vec![InlineNode::Text(text)]))
    } else if is_strikethrough {
        Some(InlineNode::Strikethrough(vec![InlineNode::Text(text)]))
    } else {
        Some(InlineNode::Text(text))
    }
}

fn parse_hyperlink(data: &serde_json::Value, images: &[&serde_json::Value]) -> Option<InlineNode> {
    let url = data
        .get("url")
        .or_else(|| data.get("anchor"))
        .and_then(|u| u.as_str())
        .unwrap_or("")
        .to_string();

    let children: Vec<InlineNode> = data
        .get("children")
        .and_then(|c| c.as_array())
        .into_iter()
        .flatten()
        .filter_map(|node| parse_inline_node(node, images))
        .collect();

    if children.is_empty() {
        None
    } else {
        Some(InlineNode::Link {
            text: children,
            url,
        })
    }
}

fn parse_drawing(data: &serde_json::Value, images: &[&serde_json::Value]) -> Option<InlineNode> {
    // docx-rs drawing JSON structure (v0.4.x):
    //
    // The drawing node in a run's children looks like:
    //   { "type": "drawing", "data": { "data": { "id": "rId4", "size": [w, h], "type": "pic", ... }, ... } }
    //
    // The actual image data is NOT in the drawing node itself.
    // Instead, the drawing only contains an `id` (relationship ID like "rId4"),
    // and the base64 image data lives in the top-level `images` array:
    //   "images": [["rId4", "word/media/image1.png", "base64_orig", "base64_png"], ...]
    //
    // So we: extract `id` from drawing → look up in images array → get base64 data.

    // Extract the relationship ID from the drawing data
    let id = data
        .get("data") // inner data object
        .and_then(|d| d.get("id"))
        .and_then(|i| i.as_str())
        .or_else(|| {
            // Fallback: id might be at top level of data
            data.get("id").and_then(|i| i.as_str())
        })
        .unwrap_or("")
        .to_string();

    if id.is_empty() {
        return None;
    }

    // Look up the image in the top-level images array
    // Format: [[id, path, base64_original, base64_png], ...]
    let image_entry = images.iter().find(|entry| {
        entry
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|e| e.as_str())
            == Some(&id)
    })?;

    let arr = image_entry.as_array()?;
    
    // DEBUG: log image entry structure
    eprintln!("[DEBUG parse_drawing] id={}, arr.len={}, arr[1]={:?}, arr[2].len={}, arr[3].len={}",
        id,
        arr.len(),
        arr.get(1).and_then(|v| v.as_str()).unwrap_or(""),
        arr.get(2).and_then(|v| v.as_str()).map(|s| s.len()).unwrap_or(0),
        arr.get(3).and_then(|v| v.as_str()).map(|s| s.len()).unwrap_or(0),
    );
    
    let image_b64 = arr
        .get(3)
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .or_else(|| arr.get(2).and_then(|v| v.as_str()))
        .unwrap_or("")
        .to_string();

    if image_b64.is_empty() {
        return None;
    }

    // Extract size (in EMU — English Metric Units)
    let (width, height) = data
        .get("data")
        .and_then(|d| d.get("size"))
        .or_else(|| data.get("size"))
        .and_then(|s| s.as_array())
        .map(|arr| {
            let w = arr.first().and_then(|v| v.as_u64()).map(|v| v as u32);
            let h = arr.get(1).and_then(|v| v.as_u64()).map(|v| v as u32);
            (w, h)
        })
        .unwrap_or((None, None));

    Some(InlineNode::InlineImage {
        id,
        data: image_b64,
        width,
        height,
    })
}

fn parse_table(data: &serde_json::Value, images: &[&serde_json::Value]) -> Option<DocNode> {
    let rows = data.get("rows").and_then(|r| r.as_array())?;

    let mut headers: Vec<Vec<InlineNode>> = Vec::new();
    let mut body_rows: Vec<Vec<Vec<InlineNode>>> = Vec::new();

    for (i, row) in rows.iter().enumerate() {
        let cells = row.get("cells").and_then(|c| c.as_array())?;

        let row_data: Vec<Vec<InlineNode>> = cells
            .iter()
            .map(|cell| {
                let cell_children = cell
                    .get("children")
                    .and_then(|c| c.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|node| parse_top_level_node(node, images))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                cell_children
                    .into_iter()
                    .flat_map(block_to_inlines)
                    .collect()
            })
            .collect();

        if i == 0 {
            headers = row_data;
        } else {
            body_rows.push(row_data);
        }
    }

    Some(DocNode::Table {
        headers,
        rows: body_rows,
    })
}

fn block_to_inlines(node: DocNode) -> Vec<InlineNode> {
    match node {
        DocNode::Paragraph { content } => content,
        DocNode::Heading { content, .. } => content,
        _ => vec![],
    }
}

/// Render a list of inline nodes into multiple docx-rs Runs.
/// Each inline element becomes its own Run so formatting is preserved.
fn inlines_to_runs(inlines: &[InlineNode]) -> Vec<docx_rs::Run> {
    let mut runs: Vec<docx_rs::Run> = Vec::new();

    for inline in inlines {
        match inline {
            InlineNode::Text(text) => {
                runs.push(docx_rs::Run::new().add_text(text));
            }
            InlineNode::Emphasis(content) => {
                let text: String = extract_text_from_runs(content);
                runs.push(docx_rs::Run::new().add_text(&text).italic());
            }
            InlineNode::Strong(content) => {
                let text: String = extract_text_from_runs(content);
                runs.push(docx_rs::Run::new().add_text(&text).bold());
            }
            InlineNode::Code(code) => {
                runs.push(
                    docx_rs::Run::new()
                        .add_text(code)
                        .fonts(docx_rs::RunFonts::new().ascii("Consolas"))
                        .size(20)
                        .color("333333"),
                );
            }
            InlineNode::Link { text, url: _ } => {
                let link_run = docx_rs::Run::new()
                    .add_text(&extract_text_from_runs(text))
                    .color("0563C1")
                    .underline("single");
                runs.push(link_run);
                // Note: proper hyperlinks require Hyperlink::new added to Paragraph
                // For simplicity we style the run to look like a link
            }
            InlineNode::MathInline(code) => {
                runs.push(
                    docx_rs::Run::new()
                        .add_text(code)
                        .fonts(docx_rs::RunFonts::new().ascii("Cambria Math"))
                        .size(22)
                        .italic()
                        .color("1F4E79"),
                );
            }
            InlineNode::Strikethrough(content) => {
                let text: String = extract_text_from_runs(content);
                runs.push(docx_rs::Run::new().add_text(&text).strike());
            }
            InlineNode::SoftBreak => {
                runs.push(docx_rs::Run::new().add_text(" "));
            }
            InlineNode::HardBreak => {
                runs.push(docx_rs::Run::new().add_break(docx_rs::BreakType::TextWrapping));
            }
            InlineNode::InlineImage {
                data,
                width,
                height,
                ..
            } => {
                if let Ok(img_bytes) = base64_decode(data) {
                    // Width/height from IR are already in EMU (English Metric Units).
                    // Use directly with a reasonable max to prevent overflow.
                    let w_emu = width.unwrap_or(5_486_400).min(10_000_000);
                    let h_emu = height.unwrap_or(3_600_000).min(10_000_000);
                    let pic = docx_rs::Pic::new(&img_bytes).size(w_emu, h_emu);
                    runs.push(docx_rs::Run::new().add_image(pic));
                } else {
                    runs.push(docx_rs::Run::new().add_text("[图片]"));
                }
            }
        }
    }

    runs
}

/// Extract plain text from a list of inline nodes (for flattening nested inlines)
fn extract_text_from_runs(inlines: &[InlineNode]) -> String {
    let mut text = String::new();
    for inline in inlines {
        match inline {
            InlineNode::Text(t) => text.push_str(t),
            InlineNode::Emphasis(content)
            | InlineNode::Strong(content)
            | InlineNode::Strikethrough(content) => {
                text.push_str(&extract_text_from_runs(content));
            }
            InlineNode::Code(c) => text.push_str(c),
            InlineNode::Link {
                text: link_text, ..
            } => {
                text.push_str(&extract_text_from_runs(link_text));
            }
            InlineNode::MathInline(code) => text.push_str(code),
            InlineNode::SoftBreak | InlineNode::HardBreak => {
                text.push(' ');
            }
            InlineNode::InlineImage { .. } => {
                text.push_str("[图片]");
            }
        }
    }
    text
}

/// Render a table to the document
fn render_table(
    docx: docx_rs::Docx,
    headers: &[Vec<InlineNode>],
    rows: &[Vec<Vec<InlineNode>>],
) -> docx_rs::Docx {
    let mut table_rows: Vec<docx_rs::TableRow> = Vec::new();

    // Header row
    if !headers.is_empty() {
        let header_cells: Vec<docx_rs::TableCell> = headers
            .iter()
            .map(|cell_content| {
                let runs = inlines_to_runs(cell_content);
                let mut para = docx_rs::Paragraph::new();
                for run in runs {
                    para = para.add_run(run.bold());
                }
                docx_rs::TableCell::new().add_paragraph(para)
            })
            .collect();
        table_rows.push(docx_rs::TableRow::new(header_cells));
    }

    // Body rows
    for row in rows {
        let cells: Vec<docx_rs::TableCell> = row
            .iter()
            .map(|cell_content| {
                let runs = inlines_to_runs(cell_content);
                let mut para = docx_rs::Paragraph::new();
                for run in runs {
                    para = para.add_run(run);
                }
                docx_rs::TableCell::new().add_paragraph(para)
            })
            .collect();
        table_rows.push(docx_rs::TableRow::new(cells));
    }

    if table_rows.is_empty() {
        return docx;
    }

    let num_cols = table_rows[0].cells.len();
    let col_widths: Vec<usize> = vec![2000; num_cols.max(1)];

    docx.add_table(
        docx_rs::Table::new(table_rows)
            .set_grid(col_widths)
            .style("TableGrid"),
    )
}

/// Render a blockquote with proper indentation and support for nested content.
fn render_blockquote(docx: docx_rs::Docx, content: &[DocNode], depth: i32) -> docx_rs::Docx {
    let indent = 720 + depth * 360; // increasing indent per depth level
    let mut docx = docx;

    for node in content {
        match node {
            DocNode::Paragraph { content } => {
                if content.is_empty() {
                    // Empty paragraph in blockquote — render as empty indented line
                    let para = docx_rs::Paragraph::new().indent(None, None, Some(indent), None);
                    docx = docx.add_paragraph(para);
                } else {
                    let runs = inlines_to_runs(content);
                    let mut para = docx_rs::Paragraph::new().indent(None, None, Some(indent), None);
                    for run in runs {
                        para = para.add_run(run);
                    }
                    docx = docx.add_paragraph(para);
                }
            }
            DocNode::List { ordered, items } => {
                // List inside blockquote — render with additional indent
                docx = render_list(docx, items, *ordered, depth + 1);
            }
            DocNode::BlockQuote { content: nested } => {
                // Nested blockquote — recurse
                docx = render_blockquote(docx, nested, depth + 1);
            }
            DocNode::CodeBlock { code, .. } => {
                let mut para = docx_rs::Paragraph::new().indent(None, None, Some(indent), None);
                let shading = docx_rs::Shading::new().color("auto").fill("F0F0F0");
                let run = docx_rs::Run::new()
                    .add_text(code)
                    .fonts(docx_rs::RunFonts::new().ascii("Consolas"))
                    .size(20)
                    .shading(shading);
                para = para.add_run(run);
                docx = docx.add_paragraph(para);
            }
            DocNode::HorizontalRule => {
                let run = docx_rs::Run::new().add_text("─".repeat(40)).color("999999");
                let para = docx_rs::Paragraph::new()
                    .indent(None, None, Some(indent), None)
                    .add_run(run);
                docx = docx.add_paragraph(para);
            }
            // For any other node type, try to render as paragraph
            DocNode::Heading { content, .. } => {
                let runs = inlines_to_runs(content);
                let mut para = docx_rs::Paragraph::new().indent(None, None, Some(indent), None);
                for run in runs {
                    para = para.add_run(run.bold());
                }
                docx = docx.add_paragraph(para);
            }
            DocNode::Table { headers, rows } => {
                docx = render_table(docx, headers, rows);
            }
            DocNode::MathBlock { code } => {
                let run = docx_rs::Run::new()
                    .add_text(code)
                    .fonts(docx_rs::RunFonts::new().ascii("Cambria Math"))
                    .size(24);
                let para = docx_rs::Paragraph::new()
                    .indent(None, None, Some(indent), None)
                    .add_run(run);
                docx = docx.add_paragraph(para);
            }
        }
    }
    docx
}

/// Render a list with support for nested lists and proper indentation.
fn render_list(
    docx: docx_rs::Docx,
    items: &[Vec<DocNode>],
    ordered: bool,
    depth: i32,
) -> docx_rs::Docx {
    let mut docx = docx;
    let base_indent = 720 + depth * 360;
    let hanging_indent = 360;

    for (idx, item) in items.iter().enumerate() {
        for node in item {
            match node {
                DocNode::Paragraph { content } => {
                    let prefix = if ordered {
                        format!("{}.", idx + 1)
                    } else {
                        "•".to_string()
                    };
                    let mut para = docx_rs::Paragraph::new().indent(
                        None,
                        None,
                        Some(base_indent),
                        Some(hanging_indent),
                    );
                    para = para.add_run(docx_rs::Run::new().add_text(&prefix));
                    if !content.is_empty() {
                        para = para.add_run(docx_rs::Run::new().add_text(" "));
                        for run in inlines_to_runs(content) {
                            para = para.add_run(run);
                        }
                    }
                    docx = docx.add_paragraph(para);
                }
                DocNode::List {
                    ordered: nested_ordered,
                    items: nested_items,
                } => {
                    // Nested list — recurse with increased depth
                    docx = render_list(docx, nested_items, *nested_ordered, depth + 1);
                }
                // For any other node in a list item, render as a regular paragraph with indent
                DocNode::Heading { content, .. } => {
                    let runs = inlines_to_runs(content);
                    let mut para = docx_rs::Paragraph::new().indent(
                        None,
                        None,
                        Some(base_indent + hanging_indent),
                        None,
                    );
                    for run in runs {
                        para = para.add_run(run.bold());
                    }
                    docx = docx.add_paragraph(para);
                }
                DocNode::BlockQuote { content: nested } => {
                    docx = render_blockquote(docx, nested, depth + 1);
                }
                DocNode::Table { headers, rows } => {
                    docx = render_table(docx, headers, rows);
                }
                DocNode::MathBlock { code } => {
                    let run = docx_rs::Run::new()
                        .add_text(code)
                        .fonts(docx_rs::RunFonts::new().ascii("Cambria Math"))
                        .size(24);
                    let para = docx_rs::Paragraph::new()
                        .indent(None, None, Some(base_indent + hanging_indent), None)
                        .add_run(run);
                    docx = docx.add_paragraph(para);
                }
                DocNode::CodeBlock { code, .. } => {
                    let mut para = docx_rs::Paragraph::new().indent(
                        None,
                        None,
                        Some(base_indent + hanging_indent),
                        None,
                    );
                    let shading = docx_rs::Shading::new().color("auto").fill("F0F0F0");
                    let run = docx_rs::Run::new()
                        .add_text(code)
                        .fonts(docx_rs::RunFonts::new().ascii("Consolas"))
                        .size(20)
                        .shading(shading);
                    para = para.add_run(run);
                    docx = docx.add_paragraph(para);
                }
                DocNode::HorizontalRule => {
                    let run = docx_rs::Run::new().add_text("─".repeat(40)).color("999999");
                    let para = docx_rs::Paragraph::new()
                        .indent(None, None, Some(base_indent + hanging_indent), None)
                        .add_run(run);
                    docx = docx.add_paragraph(para);
                }
            }
        }
    }
    docx
}

/// Simple base64 decoder (minimal implementation to avoid adding a dependency)
fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    let mut result = Vec::with_capacity(input.len() * 3 / 4);
    let mut buf: u32 = 0;
    let mut bits = 0u32;

    for byte in input.bytes() {
        let val = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => break,
            _ => continue, // skip whitespace/newlines
        };
        buf = (buf << 6) | val as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            result.push((buf >> bits) as u8);
            buf &= (1u32 << bits) - 1;
        }
    }

    if result.is_empty() {
        Err("empty decoded data".to_string())
    } else {
        Ok(result)
    }
}
