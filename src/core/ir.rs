use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Document {
    pub nodes: Vec<DocNode>,
}

impl Document {
    pub fn new(nodes: Vec<DocNode>) -> Self {
        Self { nodes }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocNode {
    Heading {
        level: u8,
        content: Vec<InlineNode>,
    },
    Paragraph {
        content: Vec<InlineNode>,
    },
    CodeBlock {
        language: Option<String>,
        code: String,
    },
    BlockQuote {
        content: Vec<DocNode>,
    },
    List {
        ordered: bool,
        items: Vec<Vec<DocNode>>,
    },
    HorizontalRule,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InlineNode {
    Text(String),
    Emphasis(Vec<InlineNode>),
    Strong(Vec<InlineNode>),
    Code(String),
    Link { text: Vec<InlineNode>, url: String },
    SoftBreak,
    HardBreak,
}
