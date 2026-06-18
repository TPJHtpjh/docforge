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

    fn render_from_ir(&self, _document: &Document) -> Result<RenderedDocument, ConverterError> {
        todo!("Implement DOCX generation")
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
