use crate::codegen::{export_json_blob, export_json_blob_pretty, export_zx_fspack};
use crate::schema::{CancelTable, Character, Move};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tauri_plugin_dialog::DialogExt;

fn project_rules_path(characters_dir: &str) -> PathBuf {
    let project_root = Path::new(characters_dir).parent().unwrap_or(Path::new("."));
    project_root.join("framesmith.rules.json")
}

fn load_character_files(
    characters_dir: &str,
    character_id: &str,
) -> Result<(PathBuf, Character, Vec<Move>, CancelTable), String> {
    // Validate character_id to prevent path traversal attacks
    if character_id.contains("..") || character_id.contains('/') || character_id.contains('\\') {
        return Err("Invalid character ID".to_string());
    }

    let char_path = Path::new(characters_dir).join(character_id);
    if !char_path.exists() {
        return Err(format!("Character '{}' not found", character_id));
    }

    // Load character.json
    let char_file = char_path.join("character.json");
    let content = fs::read_to_string(&char_file)
        .map_err(|e| format!("Failed to read character.json: {}", e))?;
    let character: Character = serde_json::from_str(&content)
        .map_err(|e| format!("Invalid character.json format: {}", e))?;

    // Load all moves
    let moves_dir = char_path.join("moves");
    let mut moves = vec![];
    if moves_dir.exists() {
        for entry in fs::read_dir(&moves_dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let move_path = entry.path();
            if move_path.extension().map(|e| e == "json").unwrap_or(false) {
                let content = fs::read_to_string(&move_path).map_err(|e| {
                    format!(
                        "Failed to read move file {:?}: {}",
                        move_path.file_name(),
                        e
                    )
                })?;
                let mv: Move = serde_json::from_str(&content)
                    .map_err(|e| format!("Invalid move file {:?}: {}", move_path.file_name(), e))?;
                moves.push(mv);
            }
        }
    }

    // Load cancel table
    let cancel_file = char_path.join("cancel_table.json");
    let cancel_table: CancelTable = if cancel_file.exists() {
        let content = fs::read_to_string(&cancel_file)
            .map_err(|e| format!("Failed to read cancel_table.json: {}", e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Invalid cancel_table.json format: {}", e))?
    } else {
        CancelTable {
            chains: HashMap::new(),
            special_cancels: vec![],
            super_cancels: vec![],
            jump_cancels: vec![],
        }
    };

    Ok((char_path, character, moves, cancel_table))
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CharacterData {
    pub character: Character,
    pub moves: Vec<Move>,
    pub cancel_table: CancelTable,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CharacterSummary {
    pub id: String,
    pub name: String,
    pub archetype: String,
    pub move_count: usize,
}

#[tauri::command]
pub fn list_characters(characters_dir: String) -> Result<Vec<CharacterSummary>, String> {
    let path = Path::new(&characters_dir);
    if !path.exists() {
        return Ok(vec![]);
    }

    let mut summaries = vec![];
    for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let char_path = entry.path();
        if !char_path.is_dir() {
            continue;
        }

        let char_file = char_path.join("character.json");
        if !char_file.exists() {
            continue;
        }

        let content = fs::read_to_string(&char_file).map_err(|e| e.to_string())?;
        let character: Character = serde_json::from_str(&content).map_err(|e| e.to_string())?;

        let moves_dir = char_path.join("moves");
        let move_count = if moves_dir.exists() {
            fs::read_dir(&moves_dir)
                .map(|dir| dir.filter(|e| e.is_ok()).count())
                .unwrap_or(0)
        } else {
            0
        };

        summaries.push(CharacterSummary {
            id: character.id.clone(),
            name: character.name,
            archetype: character.archetype,
            move_count,
        });
    }

    Ok(summaries)
}

#[tauri::command]
pub fn load_character(
    characters_dir: String,
    character_id: String,
) -> Result<CharacterData, String> {
    let (char_path, character, moves, cancel_table) =
        load_character_files(&characters_dir, &character_id)?;

    let project_rules_path = project_rules_path(&characters_dir);
    let project_rules = crate::rules::load_rules_file(&project_rules_path).map_err(|e| {
        format!(
            "Failed to load project rules file {}: {}",
            project_rules_path.display(),
            e
        )
    })?;

    let character_rules_path = char_path.join("rules.json");
    let character_rules = crate::rules::load_rules_file(&character_rules_path).map_err(|e| {
        format!(
            "Failed to load character rules file {}: {}",
            character_rules_path.display(),
            e
        )
    })?;

    let mut resolved_moves = Vec::with_capacity(moves.len());
    for mv in moves {
        let resolved = crate::rules::apply_rules_to_move(
            project_rules.as_ref(),
            character_rules.as_ref(),
            &mv,
        )
        .map_err(|e| format!("Failed to apply rules to move '{}': {}", mv.input, e))?;
        resolved_moves.push(resolved);
    }

    Ok(CharacterData {
        character,
        moves: resolved_moves,
        cancel_table,
    })
}

#[tauri::command]
pub fn save_move(characters_dir: String, character_id: String, mv: Move) -> Result<(), String> {
    // Validate character_id to prevent path traversal
    if character_id.contains("..") || character_id.contains('/') || character_id.contains('\\') {
        return Err("Invalid character ID".to_string());
    }

    // Validate mv.input to prevent path traversal / invalid filenames
    if mv.input.contains("..") || mv.input.contains('/') || mv.input.contains('\\') {
        return Err("Invalid move input".to_string());
    }

    let char_path = Path::new(&characters_dir).join(&character_id);

    // Load rules for registry-aware validation.
    let project_rules_path = project_rules_path(&characters_dir);
    let project_rules = crate::rules::load_rules_file(&project_rules_path).map_err(|e| {
        format!(
            "Failed to load project rules file {}: {}",
            project_rules_path.display(),
            e
        )
    })?;

    let character_rules_path = char_path.join("rules.json");
    let character_rules = crate::rules::load_rules_file(&character_rules_path).map_err(|e| {
        format!(
            "Failed to load character rules file {}: {}",
            character_rules_path.display(),
            e
        )
    })?;

    // Load character.json for resource validation.
    let char_file = char_path.join("character.json");
    let char_content = fs::read_to_string(&char_file)
        .map_err(|e| format!("Failed to read {}: {}", char_file.display(), e))?;
    let character: Character = serde_json::from_str(&char_content)
        .map_err(|e| format!("Invalid {}: {}", char_file.display(), e))?;

    let registry = crate::rules::merged_registry(project_rules.as_ref(), character_rules.as_ref());
    let mut issues = crate::rules::validate_character_resources_with_registry(&character, &registry);
    for issue in issues.iter_mut() {
        issue.field = format!("character.{}", issue.field);
    }

    let move_issues = crate::rules::validate_move_with_rules(
        project_rules.as_ref(),
        character_rules.as_ref(),
        &mv,
    )
    .map_err(|e| format!("Failed to validate move '{}': {}", mv.input, e))?;
    issues.extend(move_issues);

    let errors: Vec<String> = issues
        .into_iter()
        .filter(|i| i.severity == crate::rules::Severity::Error)
        .map(|i| format!("{}: {}", i.field, i.message))
        .collect();

    if !errors.is_empty() {
        return Err(format!("Validation errors: {}", errors.join("; ")));
    }

    let move_path = char_path
        .join("moves")
        .join(format!("{}.json", mv.input));

    let content = serde_json::to_string_pretty(&mv)
        .map_err(|e| format!("Failed to serialize move: {}", e))?;
    fs::write(&move_path, content).map_err(|e| format!("Failed to write move file: {}", e))?;

    Ok(())
}

#[tauri::command]
pub fn export_character(
    characters_dir: String,
    character_id: String,
    adapter: String,
    output_path: String,
    pretty: bool,
) -> Result<(), String> {
    let (char_path, character, base_moves, cancel_table) =
        load_character_files(&characters_dir, &character_id)?;

    let project_rules_path = project_rules_path(&characters_dir);
    let project_rules = crate::rules::load_rules_file(&project_rules_path).map_err(|e| {
        format!(
            "Failed to load project rules file {}: {}",
            project_rules_path.display(),
            e
        )
    })?;

    let character_rules_path = char_path.join("rules.json");
    let character_rules = crate::rules::load_rules_file(&character_rules_path).map_err(|e| {
        format!(
            "Failed to load character rules file {}: {}",
            character_rules_path.display(),
            e
        )
    })?;

    let mut error_messages = Vec::new();

    let registry = crate::rules::merged_registry(project_rules.as_ref(), character_rules.as_ref());
    let char_issues = crate::rules::validate_character_resources_with_registry(&character, &registry);
    error_messages.extend(
        char_issues
            .into_iter()
            .filter(|i| i.severity == crate::rules::Severity::Error)
            .map(|i| format!("character {}: {}", i.field, i.message)),
    );

    let mut resolved_moves = Vec::with_capacity(base_moves.len());
    for mv in base_moves {
        let issues = crate::rules::validate_move_with_rules(
            project_rules.as_ref(),
            character_rules.as_ref(),
            &mv,
        )
        .map_err(|e| format!("Failed to validate move '{}': {}", mv.input, e))?;

        error_messages.extend(
            issues
                .into_iter()
                .filter(|i| i.severity == crate::rules::Severity::Error)
                .map(|i| format!("{} {}: {}", mv.input, i.field, i.message)),
        );

        let resolved = crate::rules::apply_rules_to_move(
            project_rules.as_ref(),
            character_rules.as_ref(),
            &mv,
        )
        .map_err(|e| format!("Failed to apply rules to move '{}': {}", mv.input, e))?;
        resolved_moves.push(resolved);
    }

    if !error_messages.is_empty() {
        return Err(error_messages.join("; "));
    }

    let char_data = CharacterData {
        character,
        moves: resolved_moves,
        cancel_table,
    };

    let output = match adapter.as_str() {
        "json-blob" => {
            if pretty {
                export_json_blob_pretty(&char_data)?
            } else {
                export_json_blob(&char_data)?
            }
        }
        "zx-fspack" => {
            let bytes = export_zx_fspack(&char_data)?;
            fs::write(&output_path, bytes)
                .map_err(|e| format!("Failed to write export file: {}", e))?;
            return Ok(());
        }
        "breakpoint-rust" => {
            return Err("Breakpoint adapter not yet implemented".to_string());
        }
        _ => return Err(format!("Unknown adapter: {}", adapter)),
    };

    fs::write(&output_path, output).map_err(|e| format!("Failed to write export file: {}", e))?;
    Ok(())
}

// =============================================================================
// Project Management Commands
// =============================================================================

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProjectInfo {
    pub name: String,
    pub path: String,
    pub character_count: usize,
}

#[tauri::command]
pub async fn open_folder_dialog(app: tauri::AppHandle) -> Option<String> {
    let folder = app
        .dialog()
        .file()
        .set_title("Open Project Folder")
        .blocking_pick_folder();

    folder.map(|p| p.to_string())
}

#[tauri::command]
pub fn validate_project(path: String) -> Result<ProjectInfo, String> {
    let project_path = Path::new(&path);

    // Check framesmith.rules.json exists
    let rules_path = project_path.join("framesmith.rules.json");
    if !rules_path.exists() {
        return Err("Not a valid Framesmith project: missing framesmith.rules.json".to_string());
    }

    // Check characters directory exists
    let characters_path = project_path.join("characters");
    if !characters_path.exists() {
        return Err("Not a valid Framesmith project: missing characters/ directory".to_string());
    }

    // Count characters
    let character_count = fs::read_dir(&characters_path)
        .map(|dir| {
            dir.filter(|e| {
                e.as_ref()
                    .map(|entry| entry.path().is_dir())
                    .unwrap_or(false)
            })
            .count()
        })
        .unwrap_or(0);

    // Get project name from folder name
    let name = project_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown")
        .to_string();

    Ok(ProjectInfo {
        name,
        path,
        character_count,
    })
}

#[tauri::command]
pub fn create_project(path: String) -> Result<(), String> {
    let project_path = Path::new(&path);

    // Create framesmith.rules.json
    let rules_path = project_path.join("framesmith.rules.json");
    let rules_content = r#"{"version": 1, "apply": [], "validate": []}"#;
    fs::write(&rules_path, rules_content)
        .map_err(|e| format!("Failed to create framesmith.rules.json: {}", e))?;

    // Create characters directory
    let characters_path = project_path.join("characters");
    fs::create_dir_all(&characters_path)
        .map_err(|e| format!("Failed to create characters directory: {}", e))?;

    Ok(())
}

fn validate_character_id(id: &str) -> Result<(), String> {
    if id.is_empty() {
        return Err("Character ID cannot be empty".to_string());
    }

    // Only allow lowercase alphanumeric, dash, and underscore
    if !id
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
    {
        return Err(
            "Character ID must be lowercase and contain only letters, numbers, dashes, and underscores".to_string(),
        );
    }

    // Prevent path traversal
    if id.contains("..") {
        return Err("Invalid character ID".to_string());
    }

    Ok(())
}

#[tauri::command]
pub fn create_character(
    characters_dir: String,
    id: String,
    name: String,
    archetype: String,
) -> Result<(), String> {
    validate_character_id(&id)?;

    let char_path = Path::new(&characters_dir).join(&id);

    // Check if character already exists
    if char_path.exists() {
        return Err(format!("Character '{}' already exists", id));
    }

    // Create character directory
    fs::create_dir_all(&char_path)
        .map_err(|e| format!("Failed to create character directory: {}", e))?;

    // Create character.json with default stats
    let character = Character {
        id: id.clone(),
        name,
        archetype,
        health: 10000,
        walk_speed: 4.0,
        back_walk_speed: 3.0,
        jump_height: 120,
        jump_duration: 45,
        dash_distance: 80,
        dash_duration: 18,
        resources: vec![],
    };

    let char_json = serde_json::to_string_pretty(&character)
        .map_err(|e| format!("Failed to serialize character: {}", e))?;
    fs::write(char_path.join("character.json"), char_json)
        .map_err(|e| format!("Failed to write character.json: {}", e))?;

    // Create moves directory
    fs::create_dir_all(char_path.join("moves"))
        .map_err(|e| format!("Failed to create moves directory: {}", e))?;

    // Create cancel_table.json with empty chains
    let cancel_table = CancelTable {
        chains: HashMap::new(),
        special_cancels: vec![],
        super_cancels: vec![],
        jump_cancels: vec![],
    };

    let cancel_json = serde_json::to_string_pretty(&cancel_table)
        .map_err(|e| format!("Failed to serialize cancel table: {}", e))?;
    fs::write(char_path.join("cancel_table.json"), cancel_json)
        .map_err(|e| format!("Failed to write cancel_table.json: {}", e))?;

    Ok(())
}

#[tauri::command]
pub fn clone_character(
    characters_dir: String,
    source_id: String,
    new_id: String,
    new_name: String,
) -> Result<(), String> {
    validate_character_id(&source_id)?;
    validate_character_id(&new_id)?;

    let source_path = Path::new(&characters_dir).join(&source_id);
    let dest_path = Path::new(&characters_dir).join(&new_id);

    // Check source exists
    if !source_path.exists() {
        return Err(format!("Source character '{}' not found", source_id));
    }

    // Check destination doesn't exist
    if dest_path.exists() {
        return Err(format!("Character '{}' already exists", new_id));
    }

    // Copy entire directory recursively
    copy_dir_recursive(&source_path, &dest_path)
        .map_err(|e| format!("Failed to copy character: {}", e))?;

    // Update character.json with new id and name
    let char_file = dest_path.join("character.json");
    let content = fs::read_to_string(&char_file)
        .map_err(|e| format!("Failed to read character.json: {}", e))?;
    let mut character: Character =
        serde_json::from_str(&content).map_err(|e| format!("Invalid character.json: {}", e))?;

    character.id = new_id;
    character.name = new_name;

    let updated_json = serde_json::to_string_pretty(&character)
        .map_err(|e| format!("Failed to serialize character: {}", e))?;
    fs::write(&char_file, updated_json)
        .map_err(|e| format!("Failed to write character.json: {}", e))?;

    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

#[tauri::command]
pub fn delete_character(characters_dir: String, character_id: String) -> Result<(), String> {
    validate_character_id(&character_id)?;

    let char_path = Path::new(&characters_dir).join(&character_id);

    // Check character exists
    if !char_path.exists() {
        return Err(format!("Character '{}' not found", character_id));
    }

    // Delete the character directory recursively
    fs::remove_dir_all(&char_path)
        .map_err(|e| format!("Failed to delete character: {}", e))?;

    Ok(())
}

fn validate_move_input(input: &str) -> Result<(), String> {
    if input.is_empty() {
        return Err("Move input cannot be empty".to_string());
    }

    // Prevent path traversal
    if input.contains("..") || input.contains('/') || input.contains('\\') {
        return Err("Invalid move input".to_string());
    }

    // Only allow alphanumeric, plus common fighting game notation characters
    if !input.chars().all(|c| {
        c.is_ascii_alphanumeric()
            || c == '+'
            || c == '['
            || c == ']'
            || c == '.'
            || c == '_'
            || c == '-'
    }) {
        return Err(
            "Move input can only contain letters, numbers, +, [], ., -, and _".to_string(),
        );
    }

    Ok(())
}

#[tauri::command]
pub fn create_move(
    characters_dir: String,
    character_id: String,
    input: String,
    name: String,
) -> Result<Move, String> {
    validate_character_id(&character_id)?;
    validate_move_input(&input)?;

    if name.trim().is_empty() {
        return Err("Move name cannot be empty".to_string());
    }

    let moves_dir = Path::new(&characters_dir)
        .join(&character_id)
        .join("moves");

    // Check moves directory exists
    if !moves_dir.exists() {
        return Err(format!(
            "Character '{}' moves directory not found",
            character_id
        ));
    }

    let move_path = moves_dir.join(format!("{}.json", input));

    // Check move doesn't already exist
    if move_path.exists() {
        return Err(format!("Move '{}' already exists", input));
    }

    // Create move with default values
    let mv = Move {
        input: input.clone(),
        name,
        tags: vec![],
        startup: 5,
        active: 2,
        recovery: 10,
        damage: 500,
        hitstun: 15,
        blockstun: 10,
        hitstop: 10,
        guard: crate::schema::GuardType::Mid,
        hitboxes: vec![],
        hurtboxes: vec![],
        pushback: crate::schema::Pushback { hit: 5, block: 8 },
        meter_gain: crate::schema::MeterGain { hit: 100, whiff: 20 },
        animation: input.clone(),
        move_type: None,
        trigger: None,
        parent: None,
        total: None,
        hits: None,
        preconditions: None,
        costs: None,
        movement: None,
        super_freeze: None,
        on_use: None,
        on_hit: None,
        on_block: None,
        notifies: vec![],
        advanced_hurtboxes: None,
    };

    // Write the move file
    let content = serde_json::to_string_pretty(&mv)
        .map_err(|e| format!("Failed to serialize move: {}", e))?;
    fs::write(&move_path, content).map_err(|e| format!("Failed to write move file: {}", e))?;

    Ok(mv)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_test_character(temp_dir: &TempDir) -> String {
        let characters_dir = temp_dir.path().join("characters");
        fs::create_dir_all(&characters_dir).unwrap();

        let char_dir = characters_dir.join("test-char");
        fs::create_dir_all(&char_dir).unwrap();

        let moves_dir = char_dir.join("moves");
        fs::create_dir_all(&moves_dir).unwrap();

        characters_dir.to_string_lossy().to_string()
    }

    #[test]
    fn test_validate_move_input_empty() {
        let result = validate_move_input("");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Move input cannot be empty");
    }

    #[test]
    fn test_validate_move_input_path_traversal() {
        assert!(validate_move_input("../evil").is_err());
        assert!(validate_move_input("foo/bar").is_err());
        assert!(validate_move_input("foo\\bar").is_err());
    }

    #[test]
    fn test_validate_move_input_valid() {
        assert!(validate_move_input("5L").is_ok());
        assert!(validate_move_input("236P").is_ok());
        assert!(validate_move_input("j.H").is_ok());
        assert!(validate_move_input("5[K]").is_ok());
        assert!(validate_move_input("2+K").is_ok());
        assert!(validate_move_input("test-move").is_ok());
        assert!(validate_move_input("test_move").is_ok());
    }

    #[test]
    fn test_validate_move_input_invalid_chars() {
        assert!(validate_move_input("5L!").is_err());
        assert!(validate_move_input("move@name").is_err());
        assert!(validate_move_input("move name").is_err());
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
        let move_path = Path::new(&characters_dir)
            .join("test-char")
            .join("moves")
            .join("5L.json");
        assert!(move_path.exists());
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
