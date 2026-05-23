use framesmith_lib::schema::{CancelTable, Character, State};
use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

const CONTRACT_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../docs/export-fidelity-contract.json"
));
const CONTRACT_DOC: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../docs/export-fidelity-contract.md"
));
const HANDOFF_DOC: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../docs/production-handoff-decision.md"
));
const FSPK_ROUNDTRIP_TEST_SOURCE: &str = include_str!("fspk_roundtrip.rs");

#[derive(Debug, Deserialize)]
struct ExportContract {
    version: u8,
    status_values: BTreeSet<String>,
    adapters: BTreeMap<String, AdapterContract>,
}

#[derive(Debug, Deserialize)]
struct AdapterContract {
    character: BTreeMap<String, FieldContract>,
    state: BTreeMap<String, FieldContract>,
    cancel_table: BTreeMap<String, FieldContract>,
}

#[derive(Debug, Deserialize)]
struct FieldContract {
    status: String,
    notes: String,
}

fn load_contract() -> ExportContract {
    serde_json::from_str(CONTRACT_JSON).expect("export fidelity contract should parse")
}

fn schema_fields<T: JsonSchema>() -> BTreeSet<String> {
    let schema = schemars::schema_for!(T);
    let value = serde_json::to_value(schema).expect("schema should serialize");
    let props = value
        .get("properties")
        .and_then(|v| v.as_object())
        .unwrap_or_else(|| panic!("schema has no object properties: {value}"));

    props.keys().cloned().collect()
}

fn contract_fields(fields: &BTreeMap<String, FieldContract>) -> BTreeSet<String> {
    fields.keys().cloned().collect()
}

fn fspk_roundtrip_coverage() -> BTreeMap<&'static str, Vec<&'static str>> {
    BTreeMap::from([
        (
            "character.id",
            vec!["fspk_mesh_keys_include_character_id_and_animation"],
        ),
        (
            "character.properties",
            vec!["character_properties_scalar_survive_roundtrip"],
        ),
        (
            "character.resources",
            vec!["fspk_exports_resources_and_events_sections"],
        ),
        ("state.input", vec!["fspk_exports_move_input_notation"]),
        ("state.tags", vec!["tags_survive_roundtrip"]),
        (
            "state.startup",
            vec!["fspk_move_record_fields_match_reader_layout"],
        ),
        (
            "state.active",
            vec!["fspk_move_record_fields_match_reader_layout"],
        ),
        (
            "state.recovery",
            vec!["fspk_move_record_fields_match_reader_layout"],
        ),
        (
            "state.damage",
            vec!["fspk_move_record_fields_match_reader_layout"],
        ),
        (
            "state.hitstun",
            vec!["fspk_move_record_fields_match_reader_layout"],
        ),
        (
            "state.blockstun",
            vec!["fspk_move_record_fields_match_reader_layout"],
        ),
        (
            "state.hitstop",
            vec!["fspk_move_record_fields_match_reader_layout"],
        ),
        (
            "state.guard",
            vec!["fspk_move_record_fields_match_reader_layout"],
        ),
        (
            "state.hitboxes",
            vec!["fspk_move_record_fields_match_reader_layout"],
        ),
        (
            "state.hurtboxes",
            vec!["fspk_move_record_fields_match_reader_layout"],
        ),
        (
            "state.pushback",
            vec!["fspk_exports_pushback_and_meter_gain_to_runtime_sections"],
        ),
        (
            "state.meter_gain",
            vec!["fspk_exports_pushback_and_meter_gain_to_runtime_sections"],
        ),
        (
            "state.animation",
            vec!["fspk_mesh_keys_include_character_id_and_animation"],
        ),
        (
            "state.type",
            vec!["fspk_move_record_fields_match_reader_layout"],
        ),
        (
            "state.trigger",
            vec!["fspk_move_record_fields_match_reader_layout"],
        ),
        (
            "state.total",
            vec!["fspk_move_record_fields_match_reader_layout"],
        ),
        (
            "state.preconditions",
            vec!["fspk_exports_resources_and_events_sections"],
        ),
        (
            "state.costs",
            vec!["fspk_exports_resources_and_events_sections"],
        ),
        (
            "state.on_use",
            vec!["fspk_exports_resources_and_events_sections"],
        ),
        (
            "state.on_hit",
            vec!["fspk_exports_resources_and_events_sections"],
        ),
        (
            "state.on_block",
            vec!["fspk_exports_resources_and_events_sections"],
        ),
        (
            "state.notifies",
            vec!["fspk_exports_resources_and_events_sections"],
        ),
        ("state.pushboxes", vec!["fspk_pushbox_chain_roundtrip"]),
        (
            "state.properties",
            vec![
                "state_properties_scalar_survive_roundtrip",
                "state_properties_nested_flattened_on_export",
            ],
        ),
        ("cancel_table.tag_rules", vec!["cancel_tag_rules_roundtrip"]),
        ("cancel_table.deny", vec!["cancel_denies_roundtrip"]),
    ])
}

#[test]
fn export_fidelity_contract_covers_current_schema_direct_fields() {
    let contract = load_contract();
    assert_eq!(contract.version, 1);

    let character_fields = schema_fields::<Character>();
    let state_fields = schema_fields::<State>();
    let cancel_table_fields = schema_fields::<CancelTable>();

    for (adapter_name, adapter) in &contract.adapters {
        assert_eq!(
            contract_fields(&adapter.character),
            character_fields,
            "{adapter_name} character field classifications must match schema"
        );
        assert_eq!(
            contract_fields(&adapter.state),
            state_fields,
            "{adapter_name} state field classifications must match schema"
        );
        assert_eq!(
            contract_fields(&adapter.cancel_table),
            cancel_table_fields,
            "{adapter_name} cancel table field classifications must match schema"
        );
    }
}

#[test]
fn fspk_preserved_and_derived_fields_have_named_roundtrip_coverage() {
    let contract = load_contract();
    let adapter = contract
        .adapters
        .get("fspk")
        .expect("fspk adapter contract");
    let coverage = fspk_roundtrip_coverage();

    for (section_name, fields) in [
        ("character", &adapter.character),
        ("state", &adapter.state),
        ("cancel_table", &adapter.cancel_table),
    ] {
        for (field_name, field) in fields {
            if field.status == "preserved" || field.status == "derived" {
                let key = format!("{section_name}.{field_name}");
                let test_names = coverage
                    .get(key.as_str())
                    .unwrap_or_else(|| panic!("{key} needs named FSPK roundtrip coverage"));
                assert!(
                    !test_names.is_empty(),
                    "{key} coverage must list at least one test"
                );

                for test_name in test_names {
                    assert!(
                        FSPK_ROUNDTRIP_TEST_SOURCE.contains(&format!("fn {test_name}(")),
                        "{key} references missing fspk_roundtrip test '{test_name}'"
                    );
                }
            }
        }
    }

    for key in coverage.keys() {
        let (section_name, field_name) = key
            .split_once('.')
            .unwrap_or_else(|| panic!("invalid coverage key {key}"));
        let fields = match section_name {
            "character" => &adapter.character,
            "state" => &adapter.state,
            "cancel_table" => &adapter.cancel_table,
            _ => panic!("unknown coverage section {section_name}"),
        };
        let field = fields
            .get(field_name)
            .unwrap_or_else(|| panic!("coverage key {key} is not in the contract"));
        assert!(
            field.status == "preserved" || field.status == "derived",
            "coverage key {key} should only target preserved or derived fields"
        );
    }
}

#[test]
fn export_fidelity_contract_documents_known_lossy_examples() {
    for required in [
        "## FSPK V1 Lossy Examples",
        "### Resolved Variant Identity",
        "\"id\": \"5H~level2\"",
        "FSPK v1 result: `input` is preserved as `5H`; `id` and `name` are omitted.",
        "### Advanced Multi-Hit Data",
        "\"hits\"",
        "FSPK v1 result: `hits[]` is omitted.",
        "### Movement Ownership",
        "\"movement\"",
        "FSPK v1 result: `movement` is not serialized.",
        "### Advanced Hurtbox Flags",
        "\"advanced_hurtboxes\"",
        "FSPK v1 result: `advanced_hurtboxes[]` is omitted.",
        "### Super Freeze",
        "\"super_freeze\"",
        "FSPK v1 result: `super_freeze` is omitted.",
    ] {
        assert!(
            CONTRACT_DOC.contains(required),
            "export fidelity docs should include lossy example text: {required}"
        );
    }
}

#[test]
fn production_handoff_decision_documents_json_blob_and_movement_policy() {
    for required in [
        "# Production Handoff Decision",
        "For the first production target, `json-blob` is the canonical source-of-truth",
        "`fspk` v1 is a compact validated runtime pack",
        "Movement is `json-blob` only for FSPK v1.",
        "cargo run --bin framesmith-cli -- export --project .. --character test_char --adapter json-blob --pretty --out ../exports/test_char.json",
        "cargo run --bin framesmith-cli -- export --project .. --character test_char --adapter fspk --out ../exports/test_char.fspk",
        "Use FSPK as the canonical handoff only after FSPK v2 or later",
    ] {
        assert!(
            HANDOFF_DOC.contains(required),
            "handoff decision should document: {required}"
        );
    }

    assert!(CONTRACT_DOC.contains("production-handoff-decision.md"));
}

#[test]
fn export_fidelity_contract_statuses_are_known_and_explained() {
    let contract = load_contract();
    let expected_statuses = BTreeSet::from([
        "derived".to_string(),
        "engine-owned".to_string(),
        "omitted".to_string(),
        "preserved".to_string(),
    ]);
    assert_eq!(contract.status_values, expected_statuses);

    for (adapter_name, adapter) in &contract.adapters {
        for (section_name, fields) in [
            ("character", &adapter.character),
            ("state", &adapter.state),
            ("cancel_table", &adapter.cancel_table),
        ] {
            for (field_name, field) in fields {
                assert!(
                    contract.status_values.contains(&field.status),
                    "{adapter_name}.{section_name}.{field_name} has unknown status '{}'",
                    field.status
                );
                assert!(
                    !field.notes.trim().is_empty(),
                    "{adapter_name}.{section_name}.{field_name} needs explanatory notes"
                );
            }
        }
    }
}
