//! Generates the JSON Schema for rules files.
//!
//! Run with: cargo run --bin generate_schema

use d_developmentnethercore_projectframesmith_lib::rules::generate_rules_schema;
use std::fs;
use std::path::Path;

fn main() {
    let schema = generate_rules_schema();
    let json = serde_json::to_string_pretty(&schema).expect("Failed to serialize schema");

    // Output to schemas directory (relative to project root)
    let output_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("schemas")
        .join("rules.schema.json");

    fs::create_dir_all(output_path.parent().unwrap()).expect("Failed to create schemas directory");
    fs::write(&output_path, json).expect("Failed to write schema file");

    println!("Schema written to: {}", output_path.display());
}
