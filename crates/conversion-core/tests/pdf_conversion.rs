use conversion_core::{DiagnosticSeverity, InputFormat, convert_bytes};
use lopdf::{
    Document, EncryptionState, EncryptionVersion, Object, Permissions, Stream,
    content::{Content, Operation},
    dictionary,
};

fn text_pdf(pages: &[Option<&str>]) -> Vec<u8> {
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
        let page_id = document.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
        });
        page_ids.push(page_id);
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

fn encrypted_pdf() -> Vec<u8> {
    let bytes = text_pdf(&[Some("Secret")]);
    let mut document = Document::load_mem(&bytes).expect("reload PDF");
    document.trailer.set(
        "ID",
        vec![Object::string_literal("0123456789abcdef0123456789abcdef")],
    );
    let state = EncryptionState::try_from(EncryptionVersion::V2 {
        document: &document,
        owner_password: "owner",
        user_password: "user",
        key_length: 128,
        permissions: Permissions::empty(),
    })
    .expect("create encryption state");
    document.encrypt(&state).expect("encrypt PDF");
    let mut encrypted = Vec::new();
    document
        .save_to(&mut encrypted)
        .expect("save encrypted PDF");
    encrypted
}

#[test]
fn password_protected_pdf_is_rejected_explicitly() {
    let error = convert_bytes(InputFormat::Pdf, &encrypted_pdf())
        .expect_err("password-protected PDF must fail");

    assert_eq!(error.code(), "PASSWORD_PROTECTED");
}

#[test]
fn textless_pdf_requires_ocr_instead_of_succeeding_empty() {
    let input = text_pdf(&[None, None]);

    let error = convert_bytes(InputFormat::Pdf, &input).expect_err("textless PDF must fail");

    assert_eq!(error.code(), "OCR_REQUIRED");
}

#[test]
fn mixed_pdf_reports_pages_without_usable_text() {
    let input = text_pdf(&[Some("Readable"), None]);

    let output = convert_bytes(InputFormat::Pdf, &input).expect("partially convert PDF");

    assert!(output.markdown.contains("<!-- page 2 -->\n"));
    assert!(
        output
            .markdown
            .contains("[No usable text extracted from this page.]")
    );
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "PAGE_TEXT_MISSING" && diagnostic.severity == DiagnosticSeverity::Warning
    }));
}

#[test]
fn text_pdf_is_extracted_in_page_order_with_markers() {
    let input = text_pdf(&[Some("First page"), Some("Second page")]);

    let output = convert_bytes(InputFormat::Pdf, &input).expect("convert PDF");

    assert!(output.markdown.contains("<!-- page 1 -->\n"));
    assert!(output.markdown.contains("First page"));
    assert!(output.markdown.contains("<!-- page 2 -->\n"));
    assert!(output.markdown.contains("Second page"));
    assert!(
        output.markdown.find("First page").unwrap() < output.markdown.find("Second page").unwrap()
    );
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "PDF_LAYOUT_BEST_EFFORT"
            && diagnostic.severity == DiagnosticSeverity::Warning
    }));
}
