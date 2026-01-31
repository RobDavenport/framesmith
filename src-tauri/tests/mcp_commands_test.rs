//! TDD tests for Framesmith MCP commands.
//!
//! These tests validate all MCP-exposed functions to ensure they behave correctly
//! for both happy paths and error cases.

use framesmith_lib::commands::{
    create_character, create_move, delete_character, list_characters, load_character,
};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

// =============================================================================
// Test Helpers
// =============================================================================

/// Sets up a test project with framesmith.rules.json and characters directory.
fn setup_test_project(temp_dir: &TempDir) -> String {
    let project_root = temp_dir.path();

    // Create framesmith.rules.json
    let rules_content = r#"{"version": 1, "apply": [], "validate": []}"#;
    fs::write(project_root.join("framesmith.rules.json"), rules_content).unwrap();

    // Create characters directory
    let characters_dir = project_root.join("characters");
    fs::create_dir_all(&characters_dir).unwrap();

    characters_dir.to_string_lossy().to_string()
}

/// Creates a test character with minimal valid structure.
fn create_test_character(characters_dir: &str, id: &str) {
    let char_path = Path::new(characters_dir).join(id);
    fs::create_dir_all(&char_path).unwrap();

    // Create character.json
    let character_json = format!(
        r#"{{
            "id": "{}",
            "name": "Test Character",
            "archetype": "balanced",
            "health": 10000,
            "walk_speed": 4.0,
            "back_walk_speed": 3.0,
            "jump_height": 120,
            "jump_duration": 45,
            "dash_distance": 80,
            "dash_duration": 18,
            "resources": []
        }}"#,
        id
    );
    fs::write(char_path.join("character.json"), character_json).unwrap();

    // Create states directory
    fs::create_dir_all(char_path.join("states")).unwrap();

    // Create cancel_table.json
    let cancel_json = r#"{"chains": {}, "special_cancels": [], "super_cancels": [], "jump_cancels": []}"#;
    fs::write(char_path.join("cancel_table.json"), cancel_json).unwrap();

    // Create empty rules.json for character
    let rules_json = r#"{"version": 1, "apply": [], "validate": []}"#;
    fs::write(char_path.join("rules.json"), rules_json).unwrap();
}

/// Creates a test move for a character.
fn create_test_move(characters_dir: &str, char_id: &str, input: &str, name: &str) {
    let move_path = Path::new(characters_dir)
        .join(char_id)
        .join("states")
        .join(format!("{}.json", input));

    let move_json = format!(
        r#"{{
            "input": "{}",
            "name": "{}",
            "tags": [],
            "startup": 5,
            "active": 2,
            "recovery": 10,
            "damage": 500,
            "hitstun": 15,
            "blockstun": 10,
            "hitstop": 10,
            "guard": "mid",
            "hitboxes": [],
            "hurtboxes": [],
            "pushback": {{ "hit": 5, "block": 8 }},
            "meter_gain": {{ "hit": 100, "whiff": 20 }},
            "animation": "{}",
            "notifies": []
        }}"#,
        input, name, input
    );
    fs::write(move_path, move_json).unwrap();
}

// =============================================================================
// Priority 1: The Bug - list_characters nonexistent directory
// =============================================================================

#[test]
fn list_characters_nonexistent_dir_returns_error() {
    // This is THE bug: currently returns Ok([]) instead of Err
    let result = list_characters("/nonexistent/path/that/does/not/exist".to_string());
    assert!(
        result.is_err(),
        "list_characters should return Err for nonexistent directory, got Ok({:?})",
        result.unwrap()
    );
}

// =============================================================================
// Priority 2: Core Functions - Happy Paths
// =============================================================================

#[test]
fn list_characters_with_valid_directory() {
    let temp_dir = TempDir::new().unwrap();
    let characters_dir = setup_test_project(&temp_dir);

    // Create two test characters
    create_test_character(&characters_dir, "ryu");
    create_test_character(&characters_dir, "ken");

    let result = list_characters(characters_dir).unwrap();

    assert_eq!(result.len(), 2);
    let ids: Vec<&str> = result.iter().map(|s| s.id.as_str()).collect();
    assert!(ids.contains(&"ryu"));
    assert!(ids.contains(&"ken"));
}

#[test]
fn list_characters_empty_directory_returns_empty_vec() {
    let temp_dir = TempDir::new().unwrap();
    let characters_dir = setup_test_project(&temp_dir);

    // No characters created
    let result = list_characters(characters_dir).unwrap();
    assert!(result.is_empty());
}

#[test]
fn get_character_returns_full_data() {
    let temp_dir = TempDir::new().unwrap();
    let characters_dir = setup_test_project(&temp_dir);

    create_test_character(&characters_dir, "test_char");
    create_test_move(&characters_dir, "test_char", "5L", "Light Punch");
    create_test_move(&characters_dir, "test_char", "2M", "Crouching Medium");

    let result = load_character(characters_dir, "test_char".to_string()).unwrap();

    assert_eq!(result.character.id, "test_char");
    assert_eq!(result.character.name, "Test Character");
    assert_eq!(result.moves.len(), 2);

    let inputs: Vec<&str> = result.moves.iter().map(|m| m.input.as_str()).collect();
    assert!(inputs.contains(&"5L"));
    assert!(inputs.contains(&"2M"));
}

#[test]
fn get_character_nonexistent_returns_error() {
    let temp_dir = TempDir::new().unwrap();
    let characters_dir = setup_test_project(&temp_dir);

    let result = load_character(characters_dir, "nonexistent".to_string());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

#[test]
fn list_moves_returns_summaries() {
    let temp_dir = TempDir::new().unwrap();
    let characters_dir = setup_test_project(&temp_dir);

    create_test_character(&characters_dir, "test-char");
    create_test_move(&characters_dir, "test-char", "5L", "Light Punch");
    create_test_move(&characters_dir, "test-char", "236P", "Fireball");

    let result = load_character(characters_dir, "test-char".to_string()).unwrap();

    assert_eq!(result.moves.len(), 2);
    let move_names: Vec<&str> = result.moves.iter().map(|m| m.name.as_str()).collect();
    assert!(move_names.contains(&"Light Punch"));
    assert!(move_names.contains(&"Fireball"));
}

#[test]
fn get_move_returns_single_move() {
    let temp_dir = TempDir::new().unwrap();
    let characters_dir = setup_test_project(&temp_dir);

    create_test_character(&characters_dir, "test-char");
    create_test_move(&characters_dir, "test-char", "5L", "Light Punch");

    let result = load_character(characters_dir, "test-char".to_string()).unwrap();
    let mv = result.moves.iter().find(|m| m.input == "5L").unwrap();

    assert_eq!(mv.input, "5L");
    assert_eq!(mv.name, "Light Punch");
    assert_eq!(mv.startup, 5);
}

// =============================================================================
// Priority 3: CRUD Operations
// =============================================================================

#[test]
fn test_create_move_success() {
    let temp_dir = TempDir::new().unwrap();
    let characters_dir = setup_test_project(&temp_dir);

    create_test_character(&characters_dir, "test-char");

    let result = create_move(
        characters_dir.clone(),
        "test-char".to_string(),
        "5H".to_string(),
        "Heavy Punch".to_string(),
    );

    assert!(result.is_ok());
    let mv = result.unwrap();
    assert_eq!(mv.input, "5H");
    assert_eq!(mv.name, "Heavy Punch");

    // Verify file was created
    let move_path = Path::new(&characters_dir)
        .join("test-char")
        .join("states")
        .join("5H.json");
    assert!(move_path.exists());
}

#[test]
fn test_create_move_duplicate_returns_error() {
    let temp_dir = TempDir::new().unwrap();
    let characters_dir = setup_test_project(&temp_dir);

    create_test_character(&characters_dir, "test-char");

    // Create first move
    create_move(
        characters_dir.clone(),
        "test-char".to_string(),
        "5L".to_string(),
        "Light Punch".to_string(),
    )
    .unwrap();

    // Try to create duplicate
    let result = create_move(
        characters_dir,
        "test-char".to_string(),
        "5L".to_string(),
        "Another Light".to_string(),
    );

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("already exists"));
}

#[test]
fn test_create_move_validates_input() {
    let temp_dir = TempDir::new().unwrap();
    let characters_dir = setup_test_project(&temp_dir);

    create_test_character(&characters_dir, "test-char");

    // Empty input
    let result = create_move(
        characters_dir.clone(),
        "test-char".to_string(),
        "".to_string(),
        "Empty Input".to_string(),
    );
    assert!(result.is_err());

    // Path traversal
    let result = create_move(
        characters_dir.clone(),
        "test-char".to_string(),
        "../evil".to_string(),
        "Evil Move".to_string(),
    );
    assert!(result.is_err());
}

#[test]
fn test_delete_character_removes_directory() {
    let temp_dir = TempDir::new().unwrap();
    let characters_dir = setup_test_project(&temp_dir);

    create_test_character(&characters_dir, "test-char");

    let char_path = Path::new(&characters_dir).join("test-char");
    assert!(char_path.exists());

    let result = delete_character(characters_dir.clone(), "test-char".to_string());
    assert!(result.is_ok());
    assert!(!char_path.exists());
}

#[test]
fn test_delete_nonexistent_character_returns_error() {
    let temp_dir = TempDir::new().unwrap();
    let characters_dir = setup_test_project(&temp_dir);

    let result = delete_character(characters_dir, "nonexistent".to_string());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

// =============================================================================
// Priority 4: Security - Path Traversal
// =============================================================================

#[test]
fn get_character_path_traversal_rejected() {
    let temp_dir = TempDir::new().unwrap();
    let characters_dir = setup_test_project(&temp_dir);

    // Test various path traversal attempts
    let traversal_attempts = vec![
        "../etc/passwd",
        "..\\windows\\system32",
        "foo/../../../etc/passwd",
        "valid/../../escape",
    ];

    for attempt in traversal_attempts {
        let result = load_character(characters_dir.clone(), attempt.to_string());
        assert!(
            result.is_err(),
            "Path traversal '{}' should be rejected",
            attempt
        );
    }
}

#[test]
fn create_character_path_traversal_rejected() {
    let temp_dir = TempDir::new().unwrap();
    let characters_dir = setup_test_project(&temp_dir);

    let result = create_character(
        characters_dir,
        "../evil".to_string(),
        "Evil".to_string(),
        "evil".to_string(),
    );

    assert!(result.is_err());
}

#[test]
fn create_move_character_id_path_traversal_rejected() {
    let temp_dir = TempDir::new().unwrap();
    let characters_dir = setup_test_project(&temp_dir);

    let result = create_move(
        characters_dir,
        "../evil".to_string(),
        "5L".to_string(),
        "Light Punch".to_string(),
    );

    assert!(result.is_err());
}

#[test]
fn create_move_input_path_traversal_rejected() {
    let temp_dir = TempDir::new().unwrap();
    let characters_dir = setup_test_project(&temp_dir);

    create_test_character(&characters_dir, "test-char");

    let result = create_move(
        characters_dir,
        "test-char".to_string(),
        "../evil".to_string(),
        "Evil Move".to_string(),
    );

    assert!(result.is_err());
}

// =============================================================================
// Additional Edge Cases
// =============================================================================

#[test]
fn list_characters_ignores_non_directory_entries() {
    let temp_dir = TempDir::new().unwrap();
    let characters_dir = setup_test_project(&temp_dir);

    create_test_character(&characters_dir, "valid-char");

    // Create a random file in characters dir (should be ignored)
    fs::write(
        Path::new(&characters_dir).join("random.txt"),
        "not a character",
    )
    .unwrap();

    let result = list_characters(characters_dir).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].id, "valid-char");
}

#[test]
fn list_characters_ignores_directories_without_character_json() {
    let temp_dir = TempDir::new().unwrap();
    let characters_dir = setup_test_project(&temp_dir);

    create_test_character(&characters_dir, "valid-char");

    // Create a directory without character.json (should be ignored)
    fs::create_dir_all(Path::new(&characters_dir).join("incomplete-char")).unwrap();

    let result = list_characters(characters_dir).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].id, "valid-char");
}

#[test]
fn character_summary_includes_move_count() {
    let temp_dir = TempDir::new().unwrap();
    let characters_dir = setup_test_project(&temp_dir);

    create_test_character(&characters_dir, "test-char");
    create_test_move(&characters_dir, "test-char", "5L", "Light");
    create_test_move(&characters_dir, "test-char", "5M", "Medium");
    create_test_move(&characters_dir, "test-char", "5H", "Heavy");

    let result = list_characters(characters_dir).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].move_count, 3);
}

#[test]
fn load_character_with_no_moves_returns_empty_moves_vec() {
    let temp_dir = TempDir::new().unwrap();
    let characters_dir = setup_test_project(&temp_dir);

    create_test_character(&characters_dir, "empty-moveset");

    let result = load_character(characters_dir, "empty-moveset".to_string()).unwrap();
    assert!(result.moves.is_empty());
}
