use document_to_markdown::Registry;
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

fn text_pdf(pages: &[Option<&str>]) -> Vec<u8> {
    use lopdf::{
        Document, Object, Stream,
        content::{Content, Operation},
        dictionary,
    };

    let mut document = Document::with_version("1.5");
    let pages_id = document.new_object_id();
    let font_id = document.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Courier",
    });
    let resources_id = document.add_object(dictionary! {
        "Font" => dictionary! { "F1" => font_id },
    });

    let mut page_ids = Vec::new();
    for text in pages {
        let operations = text.map_or_else(Vec::new, |text| {
            vec![
                Operation::new("BT", vec![]),
                Operation::new("Tf", vec!["F1".into(), 12.into()]),
                Operation::new("Td", vec![72.into(), 720.into()]),
                Operation::new("Tj", vec![Object::string_literal(text)]),
                Operation::new("ET", vec![]),
            ]
        });
        let content = Content { operations }.encode().expect("encode page");
        let content_id = document.add_object(Stream::new(dictionary! {}, content));
        page_ids.push(document.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
        }));
    }

    document.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => page_ids.iter().copied().map(Object::Reference).collect::<Vec<_>>(),
            "Count" => pages.len() as i64,
            "Resources" => resources_id,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
        }),
    );
    let catalog_id = document.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    document.trailer.set("Root", catalog_id);

    let mut bytes = Vec::new();
    document.save_to(&mut bytes).expect("save PDF");
    bytes
}

#[test]
fn registry_registers_and_converts_text_bearing_pdf() {
    let directory = tempfile::tempdir().expect("temp directory");
    let path = directory.path().join("report.pdf");
    fs::write(&path, text_pdf(&[Some("Quarterly report")])).expect("PDF fixture");
    let registry = Registry::default();

    let inputs = registry.register_paths(vec![path]).expect("register PDF");
    let result = registry
        .convert(&inputs[0].input_id)
        .expect("convert registered PDF");

    assert_eq!(inputs[0].format, "pdf");
    assert!(result.markdown.contains("<!-- page 1 -->"));
    assert!(result.markdown.contains("Quarterly report"));
}

#[test]
fn registry_reports_scanned_pdf_as_ocr_required_instead_of_empty_export() {
    let directory = tempfile::tempdir().expect("temp directory");
    let path = directory.path().join("scanned.pdf");
    fs::write(&path, text_pdf(&[None])).expect("textless PDF fixture");
    let registry = Registry::default();

    let inputs = registry.register_paths(vec![path]).expect("register PDF");
    let error = registry
        .convert(&inputs[0].input_id)
        .expect_err("textless PDF must not succeed");

    assert_eq!(error.code, "OCR_REQUIRED");
}
