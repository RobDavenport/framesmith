//! Integration tests for global states

use framesmith_lib::globals;
use std::collections::HashSet;
use std::fs;
use tempfile::TempDir;

fn create_test_project_with_globals() -> TempDir {
    let dir = TempDir::new().unwrap();

    // Create project structure
    fs::create_dir_all(dir.path().join("globals/states")).unwrap();
    fs::create_dir_all(dir.path().join("characters/test_char/states")).unwrap();

    // Project rules
    fs::write(
        dir.path().join("framesmith.rules.json"),
        r#"{ "version": 1 }"#,
    )
    .unwrap();

    // Global state
    fs::write(
        dir.path().join("globals/states/burst.json"),
        r#"{
            "input": "burst",
            "name": "Burst",
            "type": "system",
            "startup": 12,
            "active": 4,
            "recovery": 24
        }"#,
    )
    .unwrap();

    // Character
    fs::write(
        dir.path().join("characters/test_char/character.json"),
        r#"{ "id": "test_char", "name": "Test Character", "health": 10000, "archetype": "test", "walk_speed": 4.0, "back_walk_speed": 3.0, "jump_height": 120, "jump_duration": 45, "dash_distance": 80, "dash_duration": 18 }"#,
    )
    .unwrap();

    // Local state
    fs::write(
        dir.path().join("characters/test_char/states/5L.json"),
        r#"{ "input": "5L", "name": "Light Attack", "startup": 5 }"#,
    )
    .unwrap();

    // Cancel table
    fs::write(
        dir.path().join("characters/test_char/cancel_table.json"),
        r#"{ "tag_rules": [], "chains": {}, "deny": {} }"#,
    )
    .unwrap();

    // Globals manifest
    fs::write(
        dir.path().join("characters/test_char/globals.json"),
        r#"{ "includes": [{ "state": "burst", "as": "burst" }] }"#,
    )
    .unwrap();

    dir
}

#[test]
fn test_resolve_globals_with_local_inputs() {
    let dir = create_test_project_with_globals();
    let project_dir = dir.path();
    let char_dir = project_dir.join("characters/test_char");

    let local_inputs: HashSet<String> = ["5L".to_string()].into_iter().collect();

    let (globals, warnings) =
        globals::resolve_globals(project_dir, &char_dir, &local_inputs).unwrap();

    assert_eq!(globals.len(), 1);
    assert_eq!(globals[0].input, "burst");
    assert!(warnings.is_empty());
}

#[test]
fn test_resolve_globals_alias_conflict_with_local() {
    let dir = create_test_project_with_globals();
    let project_dir = dir.path();
    let char_dir = project_dir.join("characters/test_char");

    // Update globals.json to use "5L" as alias (conflicts with local state)
    fs::write(
        char_dir.join("globals.json"),
        r#"{ "includes": [{ "state": "burst", "as": "5L" }] }"#,
    )
    .unwrap();

    let local_inputs: HashSet<String> = ["5L".to_string()].into_iter().collect();

    let result = globals::resolve_globals(project_dir, &char_dir, &local_inputs);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("conflicts with local state"));
}

#[test]
fn test_resolve_globals_with_override() {
    let dir = create_test_project_with_globals();
    let project_dir = dir.path();
    let char_dir = project_dir.join("characters/test_char");

    // Update globals.json with an override
    fs::write(
        char_dir.join("globals.json"),
        r#"{ "includes": [{ "state": "burst", "as": "reversal", "override": { "startup": 8 } }] }"#,
    )
    .unwrap();

    let local_inputs: HashSet<String> = ["5L".to_string()].into_iter().collect();

    let (globals, warnings) =
        globals::resolve_globals(project_dir, &char_dir, &local_inputs).unwrap();

    assert_eq!(globals.len(), 1);
    assert_eq!(globals[0].input, "reversal"); // alias applied
    assert_eq!(globals[0].startup, 8); // override applied
    assert!(warnings.is_empty());
}

#[test]
fn test_resolve_globals_no_manifest() {
    let dir = create_test_project_with_globals();
    let project_dir = dir.path();
    let char_dir = project_dir.join("characters/test_char");

    // Remove globals.json
    fs::remove_file(char_dir.join("globals.json")).unwrap();

    let local_inputs: HashSet<String> = ["5L".to_string()].into_iter().collect();

    let (globals, warnings) =
        globals::resolve_globals(project_dir, &char_dir, &local_inputs).unwrap();

    assert!(globals.is_empty());
    assert!(warnings.is_empty());
}
