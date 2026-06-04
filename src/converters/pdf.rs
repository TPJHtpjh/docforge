use crate::core::converter::{Converter, ConverterError, RenderedDocument};
use crate::core::ir::Document;

#[derive(Debug, Default)]
pub struct PdfConverter;

impl Converter for PdfConverter {
    fn parse_to_ir(&self, _input: &[u8]) -> Result<Document, ConverterError> {
        // Intentionally a structural stub; PDF parsing details can be layered in later.
        todo!("Implement specific logic")
    }

    fn render_from_ir(&self, _document: &Document) -> Result<RenderedDocument, ConverterError> {
        // Intentionally a structural stub; PDF layout/coordinates can be layered in later.
        todo!("Implement specific logic")
    }
}
