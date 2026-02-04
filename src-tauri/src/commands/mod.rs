// Module declarations
pub mod character;
pub mod export;
pub mod project;

// Re-export all command functions for backward compatibility
pub use character::{
    clone_character, create_character, create_move, delete_character, get_character_fspk,
    get_character_globals, load_character, load_character_assets, read_character_asset_base64,
    save_character_globals, save_move, CharacterData, CharacterSummary,
};

pub use export::{
    delete_global_state, export_character, get_global_state, list_global_states,
    save_global_state, GlobalStateSummary,
};

pub use project::{
    create_project, list_characters, load_rules_registry, open_folder_dialog, open_training_window,
    validate_project, MergedRegistry, ProjectInfo,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    fn setup_test_character(temp_dir: &TempDir) -> String {
        let characters_dir = temp_dir.path().join("characters");
        fs::create_dir_all(&characters_dir).unwrap();

        let char_dir = characters_dir.join("test-char");
        fs::create_dir_all(&char_dir).unwrap();

        let states_dir = char_dir.join("states");
        fs::create_dir_all(&states_dir).unwrap();

        characters_dir.to_string_lossy().to_string()
    }

    #[test]
    fn test_load_character_assets_missing_returns_default() {
        let temp_dir = TempDir::new().unwrap();
        let characters_dir = setup_test_character(&temp_dir);

        let assets = load_character_assets(characters_dir, "test-char".to_string()).unwrap();
        assert_eq!(assets.version, 1);
        assert!(assets.textures.is_empty());
        assert!(assets.models.is_empty());
        assert!(assets.animations.is_empty());
    }

    #[test]
    fn test_read_character_asset_base64_rejects_traversal() {
        let temp_dir = TempDir::new().unwrap();
        let characters_dir = setup_test_character(&temp_dir);

        let err = read_character_asset_base64(
            characters_dir.clone(),
            "test-char".to_string(),
            "../x".to_string(),
        )
        .unwrap_err();
        assert_eq!(err, "Invalid asset path");

        #[cfg(windows)]
        {
            let err = read_character_asset_base64(
                characters_dir,
                "test-char".to_string(),
                "..\\x".to_string(),
            )
            .unwrap_err();
            assert_eq!(err, "Invalid asset path");
        }
    }

    #[test]
    fn test_read_character_asset_base64_reads_fixture() {
        let temp_dir = TempDir::new().unwrap();
        let characters_dir = setup_test_character(&temp_dir);

        let file_path = Path::new(&characters_dir).join("test-char").join("hello.bin");
        fs::write(&file_path, b"hello").unwrap();

        let b64 = read_character_asset_base64(
            characters_dir,
            "test-char".to_string(),
            "hello.bin".to_string(),
        )
        .unwrap();

        assert_eq!(b64, "aGVsbG8=");
    }

    #[test]
    fn test_load_character_sorts_moves_by_input() {
        let temp_dir = TempDir::new().unwrap();
        let characters_dir = setup_test_character(&temp_dir);

        let char_dir = Path::new(&characters_dir).join("test-char");
        fs::write(
            char_dir.join("character.json"),
            r#"{
              "id": "test-char",
              "name": "Test",
              "archetype": "test",
              "health": 10000,
              "walk_speed": 4.0,
              "back_walk_speed": 3.0,
              "jump_height": 120,
              "jump_duration": 45,
              "dash_distance": 80,
              "dash_duration": 18,
              "resources": []
            }"#,
        )
        .unwrap();

        let states_dir = char_dir.join("states");
        // Write in an order that is likely to differ from lexicographic sort.
        fs::write(
            states_dir.join("5M.json"),
            r#"{ "input": "5M", "startup": 1, "active": 1 }"#,
        )
        .unwrap();
        fs::write(
            states_dir.join("5L.json"),
            r#"{ "input": "5L", "startup": 1, "active": 1 }"#,
        )
        .unwrap();

        let data = load_character(characters_dir, "test-char".to_string()).unwrap();
        let inputs: Vec<&str> = data.moves.iter().map(|m| m.input.as_str()).collect();
        assert_eq!(inputs, vec!["5L", "5M"]);
    }

    #[test]
    fn test_validate_move_input_empty() {
        let result = character::validate_move_input("");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Move input cannot be empty");
    }

    #[test]
    fn test_validate_move_input_path_traversal() {
        assert!(character::validate_move_input("../evil").is_err());
        assert!(character::validate_move_input("foo/bar").is_err());
        assert!(character::validate_move_input("foo\\bar").is_err());
    }

    #[test]
    fn test_validate_move_input_valid() {
        assert!(character::validate_move_input("5L").is_ok());
        assert!(character::validate_move_input("236P").is_ok());
        assert!(character::validate_move_input("j.H").is_ok());
        assert!(character::validate_move_input("5[K]").is_ok());
        assert!(character::validate_move_input("2+K").is_ok());
        assert!(character::validate_move_input("test-move").is_ok());
        assert!(character::validate_move_input("test_move").is_ok());
    }

    #[test]
    fn test_validate_move_input_invalid_chars() {
        assert!(character::validate_move_input("5L!").is_err());
        assert!(character::validate_move_input("move@name").is_err());
        assert!(character::validate_move_input("move name").is_err());
    }

    #[test]
    fn test_create_move_success() {
        let temp_dir = TempDir::new().unwrap();
        let characters_dir = setup_test_character(&temp_dir);

        let result = create_move(
            characters_dir.clone(),
            "test-char".to_string(),
            "5L".to_string(),
            "Light Punch".to_string(),
        );

        assert!(result.is_ok());
        let mv = result.unwrap();
        assert_eq!(mv.input, "5L");
        assert_eq!(mv.name, "Light Punch");

        // Verify file was created
        let state_path = Path::new(&characters_dir)
            .join("test-char")
            .join("states")
            .join("5L.json");
        assert!(state_path.exists());
    }

    #[test]
    fn test_create_move_duplicate() {
        let temp_dir = TempDir::new().unwrap();
        let characters_dir = setup_test_character(&temp_dir);

        // Create first move
        create_move(
            characters_dir.clone(),
            "test-char".to_string(),
            "5L".to_string(),
            "Light Punch".to_string(),
        ).unwrap();

        // Try to create duplicate
        let result = create_move(
            characters_dir,
            "test-char".to_string(),
            "5L".to_string(),
            "Another Light Punch".to_string(),
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already exists"));
    }

    #[test]
    fn test_create_move_empty_name() {
        let temp_dir = TempDir::new().unwrap();
        let characters_dir = setup_test_character(&temp_dir);

        let result = create_move(
            characters_dir,
            "test-char".to_string(),
            "5L".to_string(),
            "   ".to_string(),
        );

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Move name cannot be empty");
    }

    #[test]
    fn test_create_move_invalid_character_id() {
        let temp_dir = TempDir::new().unwrap();
        let characters_dir = setup_test_character(&temp_dir);

        let result = create_move(
            characters_dir,
            "../evil".to_string(),
            "5L".to_string(),
            "Light Punch".to_string(),
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_create_move_nonexistent_character() {
        let temp_dir = TempDir::new().unwrap();
        let characters_dir = setup_test_character(&temp_dir);

        let result = create_move(
            characters_dir,
            "nonexistent".to_string(),
            "5L".to_string(),
            "Light Punch".to_string(),
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }
}
