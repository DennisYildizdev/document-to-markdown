# Document to Markdown

Windows-first, local document-to-Markdown desktop application written entirely in Rust, with an [egui](https://github.com/emilk/egui) interface.

The complete product and engineering specification is in [`docs/implement.md`](docs/implement.md).

## Download

Windows x64 builds are attached to [GitHub Releases](https://github.com/DennisYildizdev/document-to-markdown/releases). The release is a single `document-to-markdown.exe`; it needs no installer, WebView, Node.js, or Python.

## Current implementation slice

Working end to end:

- Native egui window (eframe, OpenGL) with system light/dark theme and Segoe UI.
- Native file dialog, drag and drop onto the window, and file paths passed on the command line (Explorer "Open with").
- Pure-Rust TXT, Markdown, CSV, TSV, JSON, XML, and PDF conversion.
- Strict UTF-8 decoding, LF normalization, deterministic final newline, and plain-text Markdown escaping.
- CSV/TSV table generation, wide-row sections, ragged-row warnings, JSON/XML validation, dynamic fences, and XML DTD rejection.
- PDF page-ordered text extraction with `<!-- page N -->` markers, `PDF_LAYOUT_BEST_EFFORT` and `PAGE_TEXT_MISSING` warnings, `OCR_REQUIRED` for textless input, and `PASSWORD_PROTECTED` for encrypted input.
- Queue with per-file status, background conversion on a worker thread, rendered and source preview (first 256 KiB, labeled when cut), copy to clipboard, and export to a user-selected folder.
- Preview never loads images, never interprets raw HTML, and never opens links.
- No-overwrite export naming (`name.md`, `name (2).md`, ...).
- Rust conversion tests and registry tests.

Not implemented yet: HTML, DOCX, XLSX, batch cancellation, complete format options, installer signing, SBOM, and optional OCR.

## Keyboard

| Key | Action |
|---|---|
| `Ctrl+O` | Add files |
| `Ctrl+Enter` | Convert every document not converted yet |
| `Ctrl+S` | Export the selected result |
| `Up` / `Down` | Move through the queue |
| `Delete` | Remove the selected document from the queue |
| `Esc` | Dismiss a message |

## Prerequisites

- Windows 11 x64
- Rust 1.96.1 with the `x86_64-pc-windows-msvc` target
- MSVC C++ build tools and Windows SDK

## Development

```bash
cargo run -p document-to-markdown
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --release -p document-to-markdown
```

Run commands from the repository root. The release build is `target/release/document-to-markdown.exe`. The shipped application does not use Python, a WebView, or a conversion sidecar.

## Repository layout

```text
crates/conversion-core/   Pure-Rust conversion behavior
crates/desktop/           egui application
  src/app.rs              Window layout, queue, preview, shortcuts
  src/theme.rs            Colors, type scale, fonts
  src/registry.rs         Input tickets, conversion, no-overwrite export
tests/fixtures/           Local test documents
docs/implement.md         Full implementation specification
```

## License

This project is licensed under the [MIT License](LICENSE).
