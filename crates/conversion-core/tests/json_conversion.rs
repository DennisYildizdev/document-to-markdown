use conversion_core::{InputFormat, convert_bytes};
use pretty_assertions::assert_eq;

#[test]
fn valid_json_is_preserved_in_a_json_fence() {
    let source = b"{\r\n  \"large\": 123456789012345678901234567890,\r\n  \"name\": \"T\xc3\xbcrk\xc3\xa7e\"\r\n}";

    let output = convert_bytes(InputFormat::Json, source).expect("JSON should validate");

    assert_eq!(
        output.markdown,
        "```json\n{\n  \"large\": 123456789012345678901234567890,\n  \"name\": \"T\u{fc}rk\u{e7}e\"\n}\n```\n"
    );
}

#[test]
fn malformed_json_is_rejected() {
    let error = convert_bytes(InputFormat::Json, br#"{"missing": }"#)
        .expect_err("malformed JSON must fail");

    assert_eq!(error.code(), "MALFORMED_DOCUMENT");
}

#[test]
fn fenced_source_uses_more_backticks_than_the_document() {
    let output = convert_bytes(InputFormat::Json, br#"{"code":"```"}"#)
        .expect("JSON containing backticks should convert");

    assert_eq!(output.markdown, "````json\n{\"code\":\"```\"}\n````\n");
}
