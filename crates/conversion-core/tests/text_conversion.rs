use conversion_core::{InputFormat, convert_bytes};
use pretty_assertions::assert_eq;

#[test]
fn text_conversion_preserves_paragraphs_and_escapes_accidental_markdown() {
    let input = b"Title\r\n\r\n* not a list\r\n# not a heading";

    let output = convert_bytes(InputFormat::Text, input).expect("text should convert");

    assert_eq!(
        output.markdown,
        "Title\n\n\\* not a list\n\\# not a heading\n"
    );
    assert!(output.diagnostics.is_empty());
}

#[test]
fn markdown_conversion_normalizes_line_endings_and_final_newline() {
    let input = b"# Heading\r\n\r\nBody";

    let output = convert_bytes(InputFormat::Markdown, input).expect("markdown should convert");

    assert_eq!(output.markdown, "# Heading\n\nBody\n");
}

#[test]
fn invalid_utf8_is_rejected_without_replacement_characters() {
    let error = convert_bytes(InputFormat::Text, &[0xff]).expect_err("invalid UTF-8 must fail");

    assert_eq!(error.code(), "INVALID_ENCODING");
}
