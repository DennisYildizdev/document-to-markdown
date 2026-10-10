fn main() {
    let attributes =
        tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(&[
            "get_capabilities",
            "pick_inputs",
            "convert_input",
            "export_result",
        ]));
    tauri_build::try_build(attributes).expect("failed to run tauri-build");
}
