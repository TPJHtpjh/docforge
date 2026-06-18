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
    // 标题：包含标题级别 (h1~h6, 用 1~6 表达) 和标题里的内联内容（文本、链接、加粗等）
    Heading {
        level: u8,
        content: Vec<InlineNode>,
    },

    // 段落：普通的文本块，包含内联节点
    Paragraph {
        content: Vec<InlineNode>,
    },

    // 代码块：包含代码对应的语言名（可选）以及原生的代码字符串
    CodeBlock {
        language: Option<String>,
        code: String,
    },

    // 引用块：引用里可以嵌套其他的块级节点（如段落、列表甚至其他引用）
    BlockQuote {
        content: Vec<DocNode>,
    },

    // 列表：记录列表本身是有序的还是无序的，以及其包含的数个列表项，每个列表项又是一组块级节点的集合
    List {
        ordered: bool,
        items: Vec<Vec<DocNode>>,
    },
    Table {
        headers: Vec<Vec<InlineNode>>,
        rows: Vec<Vec<Vec<InlineNode>>>,
    },
    // 数学公式块：$$ ... $$
    MathBlock {
        code: String,
    },
    // 分割线：如 ---
    HorizontalRule,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InlineNode {
    Text(String),
    Emphasis(Vec<InlineNode>),
    Strong(Vec<InlineNode>),
    Code(String),
    Link {
        text: Vec<InlineNode>,
        url: String,
    },
    // 行内代码：`code`
    // 数学公式：$ ... $
    MathInline(String),
    // 删除线：~~text~~
    Strikethrough(Vec<InlineNode>),
    SoftBreak,
    HardBreak,
    // 内联图片：DOCX 中的图片以 base64 编码存储在 IR 中
    InlineImage {
        id: String,
        data: String, // base64 encoded image data
        width: Option<u32>,
        height: Option<u32>,
    },
}
