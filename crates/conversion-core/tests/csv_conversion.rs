use conversion_core::{InputFormat, convert_bytes};
use pretty_assertions::assert_eq;

#[test]
fn csv_records_render_as_a_generated_header_table() {
    let output =
        convert_bytes(InputFormat::Csv, b"Alice,30\r\nBob,41").expect("CSV should convert");

    assert_eq!(
        output.markdown,
        "| Column 1 | Column 2 |\n| --- | --- |\n| Alice | 30 |\n| Bob | 41 |\n"
    );
    assert!(output.diagnostics.is_empty());
}

#[test]
fn ragged_csv_rows_are_padded_and_reported() {
    let output = convert_bytes(InputFormat::Csv, b"a,b\nc")
        .expect("ragged CSV should convert deterministically");

    assert_eq!(
        output.markdown,
        "| Column 1 | Column 2 |\n| --- | --- |\n| a | b |\n| c |  |\n"
    );
    assert_eq!(output.diagnostics[0].code, "RAGGED_ROWS");
}

#[test]
fn tsv_uses_tabs_and_escapes_markdown_table_pipes() {
    let output = convert_bytes(InputFormat::Tsv, b"left\tright|value").expect("TSV should convert");

    assert_eq!(
        output.markdown,
        "| Column 1 | Column 2 |\n| --- | --- |\n| left | right\\|value |\n"
    );
}

#[test]
fn records_wider_than_twenty_columns_use_labeled_sections() {
    let source = (1..=21)
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(",");

    let output =
        convert_bytes(InputFormat::Csv, source.as_bytes()).expect("wide CSV should convert");

    assert!(output.markdown.starts_with("## Row 1\n\n"));
    assert!(output.markdown.contains("- **Column 21:** 21\n"));
    assert!(!output.markdown.starts_with('|'));
}
