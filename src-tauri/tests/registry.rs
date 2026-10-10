use document_to_markdown_lib::Registry;
use std::fs;

#[test]
fn registry_registers_and_converts_authorized_text_input() {
    let directory = tempfile::tempdir().expect("temp directory");
    let path = directory.path().join("notes.txt");
    fs::write(&path, "Hello\r\n\r\n* literal").expect("fixture");
    let registry = Registry::default();

    let inputs = registry.register_paths(vec![path]).expect("register input");
    let result = registry
        .convert(&inputs[0].input_id)
        .expect("convert registered input");

    assert_eq!(inputs[0].name, "notes.txt");
    assert_eq!(inputs[0].format, "text");
    assert_eq!(result.markdown, "Hello\n\n\\* literal\n");
}

#[test]
fn registry_rejects_unregistered_or_unsupported_inputs() {
    let directory = tempfile::tempdir().expect("temp directory");
    let path = directory.path().join("archive.zip");
    fs::write(&path, "not supported").expect("fixture");
    let registry = Registry::default();

    let error = registry
        .register_paths(vec![path])
        .expect_err("ZIP must be rejected");

    assert_eq!(error.code, "UNSUPPORTED_FORMAT");
    assert!(registry.convert("input-404").is_err());
}

#[test]
fn registry_detects_and_converts_structured_text_formats() {
    let directory = tempfile::tempdir().expect("temp directory");
    let csv_path = directory.path().join("people.csv");
    let json_path = directory.path().join("data.json");
    let xml_path = directory.path().join("data.xml");
    fs::write(&csv_path, "Ada,37").expect("CSV fixture");
    fs::write(&json_path, "{\"ok\":true}").expect("JSON fixture");
    fs::write(&xml_path, "<ok>true</ok>").expect("XML fixture");
    let registry = Registry::default();

    let inputs = registry
        .register_paths(vec![csv_path, json_path, xml_path])
        .expect("register structured inputs");
    let formats: Vec<&str> = inputs.iter().map(|input| input.format.as_str()).collect();

    assert_eq!(formats, ["csv", "json", "xml"]);
    assert!(registry.convert(&inputs[0].input_id).is_ok());
    assert!(registry.convert(&inputs[1].input_id).is_ok());
    assert!(registry.convert(&inputs[2].input_id).is_ok());
}
