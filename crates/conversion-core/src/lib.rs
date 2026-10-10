//! Pure-Rust document conversion core.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InputFormat {
    Text,
    Markdown,
    Csv,
    Tsv,
    Json,
    Xml,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversionOutput {
    pub markdown: String,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: String,
    pub severity: DiagnosticSeverity,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConversionError {
    #[error("input is not valid UTF-8")]
    InvalidEncoding,
    #[error("document syntax is malformed: {0}")]
    MalformedDocument(String),
}

impl ConversionError {
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::InvalidEncoding => "INVALID_ENCODING",
            Self::MalformedDocument(_) => "MALFORMED_DOCUMENT",
        }
    }
}

pub fn convert_bytes(
    format: InputFormat,
    input: &[u8],
) -> Result<ConversionOutput, ConversionError> {
    let source = std::str::from_utf8(input).map_err(|_| ConversionError::InvalidEncoding)?;
    let normalized = normalize_line_endings(source);
    let (markdown, diagnostics) = match format {
        InputFormat::Text => (escape_plain_text(&normalized), Vec::new()),
        InputFormat::Markdown => (normalized, Vec::new()),
        InputFormat::Csv => render_delimited(&normalized, b',')?,
        InputFormat::Tsv => render_delimited(&normalized, b'\t')?,
        InputFormat::Json => render_json(&normalized)?,
        InputFormat::Xml => render_xml(&normalized)?,
    };

    Ok(ConversionOutput {
        markdown: with_final_newline(markdown),
        diagnostics,
    })
}

fn normalize_line_endings(source: &str) -> String {
    source.replace("\r\n", "\n").replace('\r', "\n")
}

fn escape_plain_text(source: &str) -> String {
    source
        .lines()
        .map(|line| match line.chars().next() {
            Some('#' | '*' | '-' | '+' | '>' | '`') => format!("\\{line}"),
            _ => line.to_owned(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_delimited(
    source: &str,
    delimiter: u8,
) -> Result<(String, Vec<Diagnostic>), ConversionError> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .delimiter(delimiter)
        .from_reader(source.as_bytes());
    let rows = reader
        .records()
        .map(|record| {
            record
                .map(|record| record.iter().map(str::to_owned).collect::<Vec<_>>())
                .map_err(|error| ConversionError::MalformedDocument(error.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;

    if rows.is_empty() {
        return Ok(("_Empty document._".to_owned(), Vec::new()));
    }

    let width = rows.iter().map(Vec::len).max().unwrap_or_default();
    let diagnostics = if rows.iter().any(|row| row.len() != width) {
        vec![Diagnostic {
            code: "RAGGED_ROWS".to_owned(),
            severity: DiagnosticSeverity::Warning,
            message: "Rows contain different field counts; missing cells were padded.".to_owned(),
        }]
    } else {
        Vec::new()
    };
    let mut markdown = String::new();
    if width > 20 {
        for (row_index, row) in rows.iter().enumerate() {
            markdown.push_str(&format!("## Row {}\n\n", row_index + 1));
            for column in 0..width {
                let value = row.get(column).map(String::as_str).unwrap_or_default();
                markdown.push_str(&format!(
                    "- **Column {}:** {}\n",
                    column + 1,
                    escape_plain_inline(value)
                ));
            }
            markdown.push('\n');
        }
        return Ok((markdown, diagnostics));
    }
    write_table_row(
        &mut markdown,
        (1..=width).map(|column| format!("Column {column}")),
    );
    write_table_row(&mut markdown, (0..width).map(|_| "---".to_owned()));
    for row in &rows {
        write_table_row(
            &mut markdown,
            (0..width).map(|column| row.get(column).cloned().unwrap_or_default()),
        );
    }

    Ok((markdown, diagnostics))
}

fn write_table_row(markdown: &mut String, cells: impl IntoIterator<Item = String>) {
    markdown.push('|');
    for cell in cells {
        markdown.push(' ');
        markdown.push_str(&escape_table_cell(&cell));
        markdown.push_str(" |");
    }
    markdown.push('\n');
}

fn escape_table_cell(cell: &str) -> String {
    cell.replace('|', "\\|").replace('\n', "<br>")
}

fn escape_plain_inline(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('*', "\\*")
        .replace('_', "\\_")
        .replace('[', "\\[")
        .replace(']', "\\]")
        .replace('\n', "  \n")
}

fn render_json(source: &str) -> Result<(String, Vec<Diagnostic>), ConversionError> {
    serde_json::from_str::<serde_json::Value>(source)
        .map_err(|error| ConversionError::MalformedDocument(error.to_string()))?;
    Ok((fenced_source("json", source), Vec::new()))
}

fn render_xml(source: &str) -> Result<(String, Vec<Diagnostic>), ConversionError> {
    let mut reader = quick_xml::Reader::from_str(source);
    loop {
        match reader.read_event() {
            Ok(quick_xml::events::Event::Eof) => break,
            Ok(quick_xml::events::Event::DocType(_)) => {
                return Err(ConversionError::MalformedDocument(
                    "XML document types and entity declarations are not permitted".to_owned(),
                ));
            }
            Ok(_) => {}
            Err(error) => {
                return Err(ConversionError::MalformedDocument(error.to_string()));
            }
        }
    }
    Ok((fenced_source("xml", source), Vec::new()))
}

fn fenced_source(language: &str, source: &str) -> String {
    let fence_length = longest_backtick_run(source).saturating_add(1).max(3);
    let fence = "`".repeat(fence_length);
    format!("{fence}{language}\n{source}\n{fence}")
}

fn longest_backtick_run(source: &str) -> usize {
    source
        .split(|character| character != '`')
        .map(str::len)
        .max()
        .unwrap_or_default()
}

fn with_final_newline(mut markdown: String) -> String {
    while markdown.ends_with('\n') {
        markdown.pop();
    }
    markdown.push('\n');
    markdown
}
