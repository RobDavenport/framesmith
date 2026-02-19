//! End-to-end pipeline test: load character -> export FSPK -> parse -> verify.
//!
//! This validates the complete export-to-read pipeline against the real
//! test_char example character, catching integration issues that unit tests miss.

use framesmith_lib::{codegen, commands};

#[test]
fn full_pipeline_load_export_parse_verify() {
    // Step 1: Load real test_char
    let char_data = commands::load_character(
        "../characters".to_string(),
        "test_char".to_string(),
    )
    .expect("load test_char");

    // Verify loaded data is non-trivial
    assert!(!char_data.moves.is_empty(), "test_char should have moves");
    assert!(
        char_data.moves.len() >= 10,
        "test_char should have at least 10 moves, got {}",
        char_data.moves.len()
    );

    // Step 2: Export to FSPK
    let bytes = codegen::export_fspk(&char_data, None).expect("export FSPK");
    assert!(!bytes.is_empty());

    // Step 3: Parse with reader
    let pack = framesmith_fspack::PackView::parse(&bytes).expect("parse FSPK");

    // Step 4: Verify structural consistency
    let states = pack.states().expect("states section");
    assert_eq!(
        states.len(),
        char_data.moves.len(),
        "state count should match"
    );

    // Verify string table exists
    assert!(
        pack.get_section(framesmith_fspack::SECTION_STRING_TABLE)
            .is_some(),
        "string table should exist"
    );

    // Verify mesh/keyframes keys
    let mesh_keys = pack.mesh_keys().expect("mesh keys");
    let kf_keys = pack.keyframes_keys().expect("keyframes keys");
    assert!(!mesh_keys.is_empty());
    assert!(!kf_keys.is_empty());

    // Verify a known move can be found
    let (idx, _) = pack
        .find_state_by_input("5L")
        .expect("should find 5L in exported pack");
    let mv = states.get(idx).expect("get move at index");
    assert_eq!(mv.startup(), 7, "5L startup should be 7");
    assert_eq!(mv.active(), 3, "5L active should be 3");
    assert_eq!(mv.damage(), 30, "5L damage should be 30");

    // Verify cancel tag rules exist (test_char has tag_rules)
    let rules = pack
        .cancel_tag_rules()
        .expect("cancel tag rules should exist");
    assert!(
        rules.len() >= 5,
        "test_char should have at least 5 cancel tag rules, got {}",
        rules.len()
    );

    // Verify resources section
    let resources = pack
        .resource_defs()
        .expect("resource defs should exist");
    assert!(
        resources.len() >= 2,
        "test_char should have at least 2 resources, got {}",
        resources.len()
    );
}

#[test]
fn pipeline_all_moves_have_valid_extras() {
    let char_data = commands::load_character(
        "../characters".to_string(),
        "test_char".to_string(),
    )
    .expect("load test_char");

    let bytes = codegen::export_fspk(&char_data, None).expect("export FSPK");
    let pack = framesmith_fspack::PackView::parse(&bytes).expect("parse FSPK");

    let states = pack.states().expect("states section");
    let extras = pack.state_extras().expect("state extras section");

    assert_eq!(
        states.len(),
        extras.len(),
        "every state should have extras"
    );

    // Verify each move's input can be read from extras
    for i in 0..extras.len() {
        let ex = extras.get(i).unwrap_or_else(|| panic!("extras {}", i));
        let (input_off, input_len) = ex.input();
        let input_str = pack
            .string(input_off, input_len)
            .unwrap_or_else(|| panic!("input string for state {}", i));
        assert!(
            !input_str.is_empty(),
            "state {} should have non-empty input notation",
            i
        );
    }
}

#[test]
fn pipeline_tags_survive_full_chain() {
    let char_data = commands::load_character(
        "../characters".to_string(),
        "test_char".to_string(),
    )
    .expect("load test_char");

    // Count moves with tags
    let moves_with_tags: Vec<_> = char_data
        .moves
        .iter()
        .filter(|m| !m.tags.is_empty())
        .collect();

    if moves_with_tags.is_empty() {
        return; // Skip if no tagged moves
    }

    let bytes = codegen::export_fspk(&char_data, None).expect("export FSPK");
    let pack = framesmith_fspack::PackView::parse(&bytes).expect("parse FSPK");

    // Verify tag ranges section exists
    assert!(
        pack.state_tag_ranges().is_some(),
        "STATE_TAG_RANGES should exist when moves have tags"
    );

    // Verify a specific tagged move
    if let Some((idx, _)) = pack.find_state_by_input("5L") {
        let tags: Vec<&str> = pack.state_tags(idx).expect("5L tags").collect();
        // 5L has tags: ["starter", "poke", "5l"]
        assert!(
            tags.len() >= 2,
            "5L should have at least 2 tags, got {}",
            tags.len()
        );
    }
}
