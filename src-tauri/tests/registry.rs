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
