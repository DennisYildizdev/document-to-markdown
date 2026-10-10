use conversion_core::{InputFormat, convert_bytes};
use pretty_assertions::assert_eq;

#[test]
fn valid_xml_is_preserved_in_an_xml_fence() {
    let source = "<?xml version=\"1.0\"?>\r\n<root><name>T\u{fc}rk\u{e7}e</name></root>";

    let output = convert_bytes(InputFormat::Xml, source.as_bytes()).expect("XML should validate");

    assert_eq!(
        output.markdown,
        "```xml\n<?xml version=\"1.0\"?>\n<root><name>T\u{fc}rk\u{e7}e</name></root>\n```\n"
    );
}

#[test]
fn malformed_xml_is_rejected() {
    let error = convert_bytes(InputFormat::Xml, b"<root><open></root>")
        .expect_err("malformed XML must fail");

    assert_eq!(error.code(), "MALFORMED_DOCUMENT");
}

#[test]
fn xml_doctype_and_entities_are_rejected() {
    let source = br#"<!DOCTYPE root [<!ENTITY x "expanded">]><root>&x;</root>"#;

    let error = convert_bytes(InputFormat::Xml, source).expect_err("DTD must be rejected");

    assert_eq!(error.code(), "MALFORMED_DOCUMENT");
}
