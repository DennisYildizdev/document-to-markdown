//! Pure-Rust document conversion core.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InputFormat {
    Text,
    Markdown,
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
}

impl ConversionError {
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::InvalidEncoding => "INVALID_ENCODING",
        }
    }
}

pub fn convert_bytes(
    format: InputFormat,
    input: &[u8],
) -> Result<ConversionOutput, ConversionError> {
    let source = std::str::from_utf8(input).map_err(|_| ConversionError::InvalidEncoding)?;
    let normalized = normalize_line_endings(source);
    let markdown = match format {
        InputFormat::Text => escape_plain_text(&normalized),
        InputFormat::Markdown => normalized,
    };

    Ok(ConversionOutput {
        markdown: with_final_newline(markdown),
        diagnostics: Vec::new(),
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

fn with_final_newline(mut markdown: String) -> String {
    while markdown.ends_with('\n') {
        markdown.pop();
    }
    markdown.push('\n');
    markdown
}
