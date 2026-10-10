# Document to Markdown

Windows-first, local document-to-Markdown desktop application built with Tauri 2, Rust, React, TypeScript, Tailwind CSS, and shadcn-style source components.

The complete product and engineering specification is in [`docs/implement.md`](docs/implement.md).

## Current implementation slice

Working end to end:

- Tauri 2 desktop shell with a narrow command capability.
- Backend-owned native file selection and opaque input/result tickets.
- Pure-Rust TXT, Markdown, CSV, TSV, JSON, and XML conversion.
- Strict UTF-8 decoding, LF normalization, deterministic final newline, and plain-text Markdown escaping.
- CSV/TSV table generation, wide-row sections, ragged-row warnings, JSON/XML validation, dynamic fences, and XML DTD rejection.
- Queue, conversion action, safe rendered preview, and user-selected export directory.
- No-overwrite export naming (`name.md`, `name (2).md`, ...).
- React workflow test, Rust conversion tests, registry tests, and capability regression test.

Not implemented yet: HTML, DOCX, XLSX, PDF, batch scheduling/cancellation, complete format options, installer signing, SBOM, and optional OCR.

## Prerequisites

- Windows 11 x64
- Node.js 24 and npm 11 (pinned dependency versions are recorded in `package-lock.json`)
- Rust 1.96.1 with the `x86_64-pc-windows-msvc` target
- MSVC C++ build tools and Windows SDK

## Development

```bash
npm install
npm test
npm run build
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
npm run tauri dev
```

Run commands from the repository root. The shipped application does not use Python or a conversion sidecar.

## Repository layout

```text
crates/conversion-core/   Pure-Rust conversion behavior
src/                      React/TypeScript application
src-tauri/                Tauri commands, registries, native dialogs, export
src-tauri/capabilities/   Narrow main-window permissions
tests/fixtures/           Local test documents
docs/implement.md         Full implementation specification
```
