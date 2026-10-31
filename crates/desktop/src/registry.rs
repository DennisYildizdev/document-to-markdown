use conversion_core::{Diagnostic, InputFormat, convert_bytes};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

const MAX_TEXT_INPUT_BYTES: u64 = 20 * 1024 * 1024;
const MAX_PDF_INPUT_BYTES: u64 = 100 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct InputSummary {
    pub input_id: String,
    pub name: String,
    pub format: String,
    pub size: u64,
}

/// Extensions accepted by the file dialog and by drag and drop.
pub const SUPPORTED_EXTENSIONS: &[&str] =
    &["txt", "md", "markdown", "csv", "tsv", "json", "xml", "pdf"];

#[derive(Debug, Clone)]
pub struct ConversionResult {
    pub result_id: String,
    pub markdown: String,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone)]
pub struct AppError {
    pub code: &'static str,
    pub message: String,
    pub retryable: bool,
}

impl AppError {
    fn new(code: &'static str, message: impl Into<String>, retryable: bool) -> Self {
        Self {
            code,
            message: message.into(),
            retryable,
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[derive(Debug, Clone)]
struct InputRecord {
    path: PathBuf,
    format: InputFormat,
    summary: InputSummary,
}

#[derive(Debug, Clone)]
struct ResultRecord {
    markdown: String,
    source_name: String,
}

#[derive(Default)]
pub struct Registry {
    next_input_id: AtomicU64,
    next_result_id: AtomicU64,
    inputs: Mutex<HashMap<String, InputRecord>>,
    results: Mutex<HashMap<String, ResultRecord>>,
}

impl Registry {
    pub fn register_paths(&self, paths: Vec<PathBuf>) -> Result<Vec<InputSummary>, AppError> {
        let mut registered = Vec::with_capacity(paths.len());
        let mut inputs = self
            .inputs
            .lock()
            .map_err(|_| internal_error("input registry is unavailable"))?;

        for path in paths {
            let format = detect_format(&path)?;
            let metadata = fs::metadata(&path)
                .map_err(|error| access_error("cannot inspect selected file", error))?;
            if !metadata.is_file() {
                return Err(AppError::new(
                    "ACCESS_DENIED",
                    "The selected path is not a regular file.",
                    false,
                ));
            }
            if metadata.len() > max_input_bytes(format) {
                return Err(AppError::new(
                    "RESOURCE_LIMIT",
                    format!(
                        "The selected file exceeds the {} MiB limit for {} input.",
                        max_input_bytes(format) / (1024 * 1024),
                        format_name(format)
                    ),
                    false,
                ));
            }

            if let Some(existing) = inputs.values().find(|record| record.path == path) {
                registered.push(existing.summary.clone());
                continue;
            }

            let input_id = format!(
                "input-{}",
                self.next_input_id.fetch_add(1, Ordering::Relaxed) + 1
            );
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| AppError::new("ACCESS_DENIED", "Invalid filename.", false))?
                .to_owned();
            let summary = InputSummary {
                input_id: input_id.clone(),
                name,
                format: format_name(format).to_owned(),
                size: metadata.len(),
            };
            inputs.insert(
                input_id,
                InputRecord {
                    path,
                    format,
                    summary: summary.clone(),
                },
            );
            registered.push(summary);
        }

        Ok(registered)
    }

    pub fn convert(&self, input_id: &str) -> Result<ConversionResult, AppError> {
        let input = self
            .inputs
            .lock()
            .map_err(|_| internal_error("input registry is unavailable"))?
            .get(input_id)
            .cloned()
            .ok_or_else(|| AppError::new("ACCESS_DENIED", "Unknown input ticket.", false))?;
        let bytes = fs::read(&input.path)
            .map_err(|error| access_error("cannot read selected file", error))?;
        let output = convert_bytes(input.format, &bytes)
            .map_err(|error| AppError::new(error.code(), error.to_string(), false))?;
        let result_id = format!(
            "result-{}",
            self.next_result_id.fetch_add(1, Ordering::Relaxed) + 1
        );
        self.results
            .lock()
            .map_err(|_| internal_error("result registry is unavailable"))?
            .insert(
                result_id.clone(),
                ResultRecord {
                    markdown: output.markdown.clone(),
                    source_name: input.summary.name,
                },
            );

        Ok(ConversionResult {
            result_id,
            markdown: output.markdown,
            diagnostics: output.diagnostics,
        })
    }

    /// `report.pdf` → `report.md`.
    pub fn suggested_file_name(&self, result_id: &str) -> Result<String, AppError> {
        let result = self.result(result_id)?;
        Ok(format!("{}.md", source_stem(&result.source_name)))
    }

    /// Exports into `directory` without overwriting: `name.md`, `name (2).md`, ...
    pub fn export_to_directory(
        &self,
        result_id: &str,
        directory: &Path,
    ) -> Result<PathBuf, AppError> {
        let result = self.result(result_id)?;
        let stem = source_stem(&result.source_name);
        let mut candidate = directory.join(format!("{stem}.md"));
        let mut suffix = 2;
        while candidate.exists() {
            candidate = directory.join(format!("{stem} ({suffix}).md"));
            suffix += 1;
        }
        write_verified(&candidate, &result.markdown)?;
        Ok(candidate)
    }

    /// Exports to a path the user picked in a native Save dialog. The dialog
    /// has already asked before replacing an existing file. A `.md` extension
    /// is added when missing.
    pub fn export_to_file(&self, result_id: &str, path: &Path) -> Result<PathBuf, AppError> {
        let result = self.result(result_id)?;
        let mut path = path.to_path_buf();
        let has_md = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("md"));
        if !has_md {
            let mut name = path.file_name().unwrap_or_default().to_os_string();
            name.push(".md");
            path.set_file_name(name);
        }

        // Write next to the target and rename, so a failed write never
        // leaves a truncated Markdown file behind.
        let mut staging_name = path.file_name().unwrap_or_default().to_os_string();
        staging_name.push(".partial");
        let staging = path.with_file_name(staging_name);
        if let Err(error) = write_verified(&staging, &result.markdown) {
            let _ = fs::remove_file(&staging);
            return Err(error);
        }
        if let Err(error) = fs::rename(&staging, &path) {
            let _ = fs::remove_file(&staging);
            return Err(access_error("cannot export Markdown", error));
        }
        Ok(path)
    }

    fn result(&self, result_id: &str) -> Result<ResultRecord, AppError> {
        self.results
            .lock()
            .map_err(|_| internal_error("result registry is unavailable"))?
            .get(result_id)
            .cloned()
            .ok_or_else(|| AppError::new("ACCESS_DENIED", "Unknown result ticket.", false))
    }
}

fn source_stem(source_name: &str) -> &str {
    Path::new(source_name)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("document")
}

fn write_verified(path: &Path, markdown: &str) -> Result<(), AppError> {
    fs::write(path, markdown.as_bytes())
        .map_err(|error| access_error("cannot export Markdown", error))?;
    let written = fs::metadata(path)
        .map_err(|error| access_error("cannot verify exported Markdown", error))?
        .len();
    if written != markdown.len() as u64 {
        return Err(AppError::new(
            "EXPORT_FAILED",
            "Export verification failed.",
            true,
        ));
    }
    Ok(())
}

fn detect_format(path: &Path) -> Result<InputFormat, AppError> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase);
    match extension.as_deref() {
        Some("txt") => Ok(InputFormat::Text),
        Some("md" | "markdown") => Ok(InputFormat::Markdown),
        Some("csv") => Ok(InputFormat::Csv),
        Some("tsv") => Ok(InputFormat::Tsv),
        Some("json") => Ok(InputFormat::Json),
        Some("xml") => Ok(InputFormat::Xml),
        Some("pdf") => Ok(InputFormat::Pdf),
        _ => Err(AppError::new(
            "UNSUPPORTED_FORMAT",
            "This file type is not supported yet.",
            false,
        )),
    }
}

const fn max_input_bytes(format: InputFormat) -> u64 {
    match format {
        InputFormat::Pdf => MAX_PDF_INPUT_BYTES,
        InputFormat::Text
        | InputFormat::Markdown
        | InputFormat::Csv
        | InputFormat::Tsv
        | InputFormat::Json
        | InputFormat::Xml => MAX_TEXT_INPUT_BYTES,
    }
}

const fn format_name(format: InputFormat) -> &'static str {
    match format {
        InputFormat::Text => "text",
        InputFormat::Markdown => "markdown",
        InputFormat::Csv => "csv",
        InputFormat::Tsv => "tsv",
        InputFormat::Json => "json",
        InputFormat::Xml => "xml",
        InputFormat::Pdf => "pdf",
    }
}

fn access_error(context: &str, error: std::io::Error) -> AppError {
    AppError::new("ACCESS_DENIED", format!("{context}: {error}"), true)
}

fn internal_error(message: &str) -> AppError {
    AppError::new("INTERNAL_ERROR", message, true)
}
