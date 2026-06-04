use thiserror::Error;

use crate::core::ir::Document;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentFormat {
    Markdown,
    Html,
    Docx,
    Pdf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderedDocument {
    Text(String),
    Binary(Vec<u8>),
}

#[derive(Debug, Error)]
pub enum ConverterError {
    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),
    #[error("invalid UTF-8 input")]
    InvalidUtf8,
    #[error("parse error: {0}")]
    Parse(String),
    #[error("render error: {0}")]
    Render(String),
}

pub trait Converter {
    fn parse_to_ir(&self, input: &[u8]) -> Result<Document, ConverterError>;
    fn render_from_ir(&self, document: &Document) -> Result<RenderedDocument, ConverterError>;
}
