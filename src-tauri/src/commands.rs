use crate::codegen::{export_json_blob, export_json_blob_pretty, export_zx_fspack};
use crate::schema::{CancelTable, Character, CharacterAssets, State};
use base64::Engine;
use std::fs;
use std::path::{Component, Path, PathBuf};
use tauri_plugin_dialog::DialogExt;

fn validate_asset_relative_path(relative_path: &str) -> Result<PathBuf, String> {
    if relative_path.is_empty() {
        return Err("Invalid asset path".to_string());
    }

    #[cfg(windows)]
    {
        // Reject ':' to avoid Windows drive-relative paths (e.g. "C:foo") and NTFS ADS ("file:stream").
        if relative_path.contains(':') {
            return Err("Invalid asset path".to_string());
        }
    }
    let path = Path::new(relative_path);
    if path.is_absolute() {
        return Err("Invalid asset path".to_string());
    }

    for component in path.components() {
        match component {
            Component::Normal(_) => {}
            _ => return Err("Invalid asset path".to_string()),
        }
    }

    Ok(path.to_path_buf())
}

fn project_rules_path(characters_dir: &str) -> PathBuf {
    let project_root = Path::new(characters_dir).parent().unwrap_or(Path::new("."));
    project_root.join("framesmith.rules.json")
}

/// Named states: Vec<(filename_without_extension, State)>
type NamedStates = Vec<(String, State)>;

/// Resolve global states and merge them with local states
fn resolve_and_merge_globals(
    characters_dir: &str,
    char_path: &Path,
    named_moves: Vec<(String, State)>,
) -> Result<Vec<(String, State)>, String> {
    // Collect local inputs for conflict detection
    let local_inputs: std::collections::HashSet<String> = named_moves
        .iter()
        .map(|(_, state)| state.input.clone())
        .collect();

    // Resolve global states
    let project_dir = Path::new(characters_dir)
        .parent()
        .ok_or_else(|| "Could not determine project directory".to_string())?;

    let (global_states, global_warnings) =
        crate::globals::resolve_globals(project_dir, char_path, &local_inputs)
            .map_err(|e| e.to_string())?;

    // Log warnings
    for warning in &global_warnings {
        eprintln!("Warning: {}", warning);
    }

    // Combine: local states + global states
    let mut all_named_moves = named_moves;
    for state in global_states {
        all_named_moves.push((state.input.clone(), state));
    }

    Ok(all_named_moves)
}

fn load_character_files(
    characters_dir: &str,
    character_id: &str,
) -> Result<(PathBuf, Character, NamedStates, CancelTable), String> {
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

    // Load all states with their filenames (for variant resolution)
    let states_dir = char_path.join("states");
    let mut moves = vec![];
    if states_dir.exists() {
        for entry in fs::read_dir(&states_dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let state_path = entry.path();
            if state_path.extension().map(|e| e == "json").unwrap_or(false) {
                let state_name = state_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .ok_or_else(|| format!("Invalid state filename: {:?}", state_path.file_name()))?
                    .to_string();
                let content = fs::read_to_string(&state_path).map_err(|e| {
                    format!(
                        "Failed to read state file {:?}: {}",
                        state_path.file_name(),
                        e
                    )
                })?;
                let mv: State = serde_json::from_str(&content)
                    .map_err(|e| format!("Invalid state file {:?}: {}", state_path.file_name(), e))?;
                moves.push((state_name, mv));
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
        CancelTable::default()
    };

    Ok((char_path, character, moves, cancel_table))
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CharacterData {
    pub character: Character,
    pub moves: Vec<State>,
    pub cancel_table: CancelTable,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CharacterSummary {
    pub id: String,
    pub name: String,
    pub archetype: String,
    pub move_count: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct GlobalStateSummary {
    pub id: String,
    pub name: String,
    pub r#type: Option<String>,
}

#[tauri::command]
pub fn list_characters(characters_dir: String) -> Result<Vec<CharacterSummary>, String> {
    let path = Path::new(&characters_dir);
    if !path.exists() {
        return Err(format!(
            "Characters directory does not exist: {}",
            characters_dir
        ));
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

        let states_dir = char_path.join("states");
        let move_count = if states_dir.exists() {
            fs::read_dir(&states_dir)
                .map_err(|e| format!("Failed to read states directory: {}", e))?
                .filter(|e| e.is_ok())
                .count()
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

/// Merged rules registry data sent to the frontend.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MergedRegistry {
    pub resources: Vec<String>,
    pub move_types: Option<crate::rules::MoveTypesConfig>,
    pub chain_order: Option<Vec<String>>,
}

#[tauri::command]
pub fn load_rules_registry(
    characters_dir: String,
    character_id: String,
) -> Result<MergedRegistry, String> {
    validate_character_id(&character_id)?;

    let char_path = Path::new(&characters_dir).join(&character_id);
    if !char_path.exists() {
        return Err(format!("Character '{}' not found", character_id));
    }

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

    let registry = crate::rules::merged_registry(project_rules.as_ref(), character_rules.as_ref());

    Ok(MergedRegistry {
        resources: registry.resources,
        move_types: registry.move_types,
        chain_order: registry.chain_order,
    })
}

#[tauri::command]
pub fn load_character(
    characters_dir: String,
    character_id: String,
) -> Result<CharacterData, String> {
    let (char_path, character, named_moves, cancel_table) =
        load_character_files(&characters_dir, &character_id)?;

    let all_named_moves = resolve_and_merge_globals(&characters_dir, &char_path, named_moves)?;
    let moves = crate::variant::flatten_variants(all_named_moves)?;

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

    // Canonicalize move ordering so indices are deterministic and match exporter/runtime.
    resolved_moves.sort_by(|a, b| a.input.cmp(&b.input));

    Ok(CharacterData {
        character,
        moves: resolved_moves,
        cancel_table,
    })
}

#[tauri::command]
pub fn load_character_assets(
    characters_dir: String,
    character_id: String,
) -> Result<CharacterAssets, String> {
    validate_character_id(&character_id)?;

    let char_path = Path::new(&characters_dir).join(&character_id);
    if !char_path.exists() {
        return Err(format!("Character '{}' not found", character_id));
    }

    let assets_file = char_path.join("assets.json");
    if !assets_file.exists() {
        return Ok(CharacterAssets::default());
    }

    let content = fs::read_to_string(&assets_file)
        .map_err(|e| format!("Failed to read assets.json: {}", e))?;
    let assets: CharacterAssets =
        serde_json::from_str(&content).map_err(|e| format!("Invalid assets.json format: {}", e))?;

    Ok(assets)
}

#[tauri::command]
pub fn read_character_asset_base64(
    characters_dir: String,
    character_id: String,
    relative_path: String,
) -> Result<String, String> {
    validate_character_id(&character_id)?;
    let rel_path = validate_asset_relative_path(&relative_path)?;

    let char_path = Path::new(&characters_dir).join(&character_id);
    if !char_path.exists() {
        return Err(format!("Character '{}' not found", character_id));
    }

    let base_canon = char_path
        .canonicalize()
        .map_err(|e| format!("Failed to resolve character directory: {}", e))?;
    let requested = char_path.join(rel_path);
    let requested_canon = requested
        .canonicalize()
        .map_err(|e| format!("Failed to read asset file: {}", e))?;

    if !requested_canon.starts_with(&base_canon) {
        return Err("Invalid asset path".to_string());
    }

    let bytes =
        fs::read(&requested_canon).map_err(|e| format!("Failed to read asset file: {}", e))?;
    Ok(base64::engine::general_purpose::STANDARD.encode(bytes))
}

#[tauri::command]
pub fn save_move(characters_dir: String, character_id: String, mv: State) -> Result<(), String> {
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

    let state_path = char_path
        .join("states")
        .join(format!("{}.json", mv.input));

    let content = serde_json::to_string_pretty(&mv)
        .map_err(|e| format!("Failed to serialize move: {}", e))?;
    fs::write(&state_path, content).map_err(|e| format!("Failed to write state file: {}", e))?;

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
    let (char_path, character, named_moves, cancel_table) =
        load_character_files(&characters_dir, &character_id)?;

    let all_named_moves = resolve_and_merge_globals(&characters_dir, &char_path, named_moves)?;
    let base_moves = crate::variant::flatten_variants(all_named_moves)?;

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
        _ => return Err(format!("Unknown adapter: {}", adapter)),
    };

    fs::write(&output_path, output).map_err(|e| format!("Failed to write export file: {}", e))?;
    Ok(())
}

/// Get FSPK bytes for a character (for training mode WASM runtime).
///
/// Returns the FSPK data as base64-encoded string.
#[tauri::command]
pub fn get_character_fspk(
    characters_dir: String,
    character_id: String,
) -> Result<String, String> {
    let (char_path, character, named_moves, cancel_table) =
        load_character_files(&characters_dir, &character_id)?;

    let all_named_moves = resolve_and_merge_globals(&characters_dir, &char_path, named_moves)?;
    let base_moves = crate::variant::flatten_variants(all_named_moves)?;

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
    let char_issues =
        crate::rules::validate_character_resources_with_registry(&character, &registry);
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

    let bytes = export_zx_fspack(&char_data)?;
    Ok(base64::engine::general_purpose::STANDARD.encode(&bytes))
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

    // Create states directory
    fs::create_dir_all(char_path.join("states"))
        .map_err(|e| format!("Failed to create states directory: {}", e))?;

    // Create cancel_table.json with empty chains
    let cancel_table = CancelTable::default();

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
) -> Result<State, String> {
    validate_character_id(&character_id)?;
    validate_move_input(&input)?;

    if name.trim().is_empty() {
        return Err("Move name cannot be empty".to_string());
    }

    let states_dir = Path::new(&characters_dir)
        .join(&character_id)
        .join("states");

    // Check states directory exists
    if !states_dir.exists() {
        return Err(format!(
            "Character '{}' states directory not found",
            character_id
        ));
    }

    let state_path = states_dir.join(format!("{}.json", input));

    // Check move doesn't already exist
    if state_path.exists() {
        return Err(format!("Move '{}' already exists", input));
    }

    // Create state with default values
    let mv = State {
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
        base: None,
        id: None,
    };

    // Write the state file
    let content = serde_json::to_string_pretty(&mv)
        .map_err(|e| format!("Failed to serialize move: {}", e))?;
    fs::write(&state_path, content).map_err(|e| format!("Failed to write state file: {}", e))?;

    Ok(mv)
}

/// Open a detached training mode window.
///
/// Creates a new browser window for training mode that can run independently
/// of the main editor window. If the window already exists, focuses it instead.
#[tauri::command]
pub async fn open_training_window(
    app: tauri::AppHandle,
    character_id: String,
) -> Result<(), String> {
    use tauri::WebviewUrl;
    use tauri::WebviewWindowBuilder;
    use tauri::Manager;

    const WINDOW_LABEL: &str = "training-detached";

    // Check if window already exists
    if let Some(existing_window) = app.get_webview_window(WINDOW_LABEL) {
        // Window exists, just focus it
        existing_window
            .set_focus()
            .map_err(|e| format!("Failed to focus existing training window: {}", e))?;
        return Ok(());
    }

    // Build the URL with query params for the training route
    let url = format!("/training?character={}&detached=true", character_id);

    let window = WebviewWindowBuilder::new(
        &app,
        WINDOW_LABEL,
        WebviewUrl::App(url.into()),
    )
    .title("Framesmith - Training Mode")
    .inner_size(1024.0, 768.0)
    .min_inner_size(800.0, 600.0)
    .resizable(true)
    .build()
    .map_err(|e| format!("Failed to create training window: {}", e))?;

    // Focus the new window
    window
        .set_focus()
        .map_err(|e| format!("Failed to focus training window: {}", e))?;

    Ok(())
}

// =============================================================================
// Global State Commands
// =============================================================================

#[tauri::command]
pub fn list_global_states(project_path: String) -> Result<Vec<GlobalStateSummary>, String> {
    let project_dir = std::path::Path::new(&project_path);

    let state_ids = crate::globals::list_global_states(project_dir).map_err(|e| e.to_string())?;

    let mut summaries = Vec::new();
    for id in state_ids {
        let state =
            crate::globals::load_global_state(project_dir, &id).map_err(|e| e.to_string())?;
        summaries.push(GlobalStateSummary {
            id,
            name: state.name.clone(),
            r#type: state.move_type.clone(),
        });
    }

    Ok(summaries)
}

#[tauri::command]
pub fn get_global_state(project_path: String, state_id: String) -> Result<State, String> {
    let project_dir = std::path::Path::new(&project_path);
    crate::globals::load_global_state(project_dir, &state_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_global_state(
    project_path: String,
    state_id: String,
    state: State,
) -> Result<(), String> {
    // Validate state_id for path traversal
    if state_id.is_empty()
        || state_id.contains("..")
        || state_id.contains('/')
        || state_id.contains('\\')
    {
        return Err("Invalid global state ID".to_string());
    }

    let project_dir = std::path::Path::new(&project_path);
    let globals_dir = project_dir.join("globals").join("states");

    std::fs::create_dir_all(&globals_dir)
        .map_err(|e| format!("Failed to create globals directory: {}", e))?;

    let state_path = globals_dir.join(format!("{}.json", state_id));
    let json = serde_json::to_string_pretty(&state)
        .map_err(|e| format!("Failed to serialize state: {}", e))?;

    std::fs::write(&state_path, json)
        .map_err(|e| format!("Failed to write global state: {}", e))?;

    Ok(())
}

#[tauri::command]
pub fn delete_global_state(project_path: String, state_id: String) -> Result<(), String> {
    // Validate state_id for path traversal
    if state_id.is_empty()
        || state_id.contains("..")
        || state_id.contains('/')
        || state_id.contains('\\')
    {
        return Err("Invalid global state ID".to_string());
    }

    let project_dir = std::path::Path::new(&project_path);
    let state_path = project_dir
        .join("globals")
        .join("states")
        .join(format!("{}.json", state_id));

    if !state_path.exists() {
        return Err(format!("Global state '{}' not found", state_id));
    }

    // Check for references
    let characters_dir = project_dir.join("characters");
    if characters_dir.exists() {
        for entry in std::fs::read_dir(&characters_dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let globals_path = entry.path().join("globals.json");
            if globals_path.exists() {
                let content = std::fs::read_to_string(&globals_path).map_err(|e| e.to_string())?;
                if content.contains(&format!("\"state\": \"{}\"", state_id))
                    || content.contains(&format!("\"state\":\"{}\"", state_id))
                {
                    return Err(format!(
                        "Cannot delete: global state '{}' is referenced by character '{}'",
                        state_id,
                        entry.file_name().to_string_lossy()
                    ));
                }
            }
        }
    }

    std::fs::remove_file(&state_path)
        .map_err(|e| format!("Failed to delete global state: {}", e))?;

    Ok(())
}

// =============================================================================
// Character Globals Commands
// =============================================================================

#[tauri::command]
pub fn get_character_globals(
    project_path: String,
    character_id: String,
) -> Result<crate::schema::GlobalsManifest, String> {
    validate_character_id(&character_id)?;

    let char_path = std::path::Path::new(&project_path)
        .join("characters")
        .join(&character_id);
    let globals_path = char_path.join("globals.json");

    if !globals_path.exists() {
        return Ok(crate::schema::GlobalsManifest::default());
    }

    let content = std::fs::read_to_string(&globals_path)
        .map_err(|e| format!("Failed to read globals.json: {}", e))?;

    serde_json::from_str(&content).map_err(|e| format!("Failed to parse globals.json: {}", e))
}

#[tauri::command]
pub fn save_character_globals(
    project_path: String,
    character_id: String,
    manifest: crate::schema::GlobalsManifest,
) -> Result<(), String> {
    validate_character_id(&character_id)?;

    let char_path = std::path::Path::new(&project_path)
        .join("characters")
        .join(&character_id);
    let globals_path = char_path.join("globals.json");

    let content = serde_json::to_string_pretty(&manifest)
        .map_err(|e| format!("Failed to serialize globals: {}", e))?;

    std::fs::write(&globals_path, content)
        .map_err(|e| format!("Failed to write globals.json: {}", e))
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
