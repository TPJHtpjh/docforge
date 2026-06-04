use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use clap::Parser;
use docforge::converters::docx::DocxConverter;
use docforge::converters::html::HtmlConverter;
use docforge::converters::markdown::MarkdownConverter;
use docforge::converters::pdf::PdfConverter;
use docforge::core::converter::{Converter, DocumentFormat, RenderedDocument};

#[derive(Debug, Parser)]
#[command(
    author,
    version,
    about = "Convert documents across markdown/html/docx/pdf"
)]
struct Cli {
    /// Input document path
    #[arg(short, long)]
    input: PathBuf,

    /// Output document path
    #[arg(short, long)]
    output: PathBuf,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let input_format = detect_format(&cli.input)?;
    let output_format = detect_format(&cli.output)?;

    let parser = converter_for_format(input_format);
    let renderer = converter_for_format(output_format);

    let input_bytes = fs::read(&cli.input)
        .with_context(|| format!("failed to read input file: {}", cli.input.display()))?;

    let document = parser
        .parse_to_ir(&input_bytes)
        .map_err(|error| anyhow!(error))?;

    let rendered = renderer
        .render_from_ir(&document)
        .map_err(|error| anyhow!(error))?;

    match rendered {
        RenderedDocument::Text(content) => {
            fs::write(&cli.output, content).with_context(|| {
                format!("failed to write output file: {}", cli.output.display())
            })?;
        }
        RenderedDocument::Binary(content) => {
            fs::write(&cli.output, content).with_context(|| {
                format!("failed to write output file: {}", cli.output.display())
            })?;
        }
    }

    Ok(())
}

fn converter_for_format(format: DocumentFormat) -> Box<dyn Converter> {
    match format {
        DocumentFormat::Markdown => Box::<MarkdownConverter>::default(),
        DocumentFormat::Html => Box::<HtmlConverter>::default(),
        DocumentFormat::Docx => Box::<DocxConverter>::default(),
        DocumentFormat::Pdf => Box::<PdfConverter>::default(),
    }
}

fn detect_format(path: &Path) -> Result<DocumentFormat> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .ok_or_else(|| anyhow!("missing file extension: {}", path.display()))?
        .to_ascii_lowercase();

    match extension.as_str() {
        "md" | "markdown" => Ok(DocumentFormat::Markdown),
        "html" | "htm" => Ok(DocumentFormat::Html),
        "docx" => Ok(DocumentFormat::Docx),
        "pdf" => Ok(DocumentFormat::Pdf),
        _ => Err(anyhow!("unsupported extension: .{extension}")),
    }
}
