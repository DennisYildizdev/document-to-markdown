use serde_json::Value;
use std::{collections::HashSet, fs};

#[test]
fn main_window_capability_grants_every_registered_command() {
    let capability_path = format!("{}/capabilities/default.json", env!("CARGO_MANIFEST_DIR"));
    let content = fs::read_to_string(capability_path).expect("capability file");
    let value: Value = serde_json::from_str(&content).expect("valid capability JSON");
    let permissions: HashSet<&str> = value["permissions"]
        .as_array()
        .expect("permissions array")
        .iter()
        .filter_map(Value::as_str)
        .collect();

    for command in [
        "allow-get-capabilities",
        "allow-pick-inputs",
        "allow-convert-input",
        "allow-export-result",
    ] {
        assert!(permissions.contains(command), "missing {command}");
    }
}
