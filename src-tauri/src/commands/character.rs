use crate::codegen::export_fspk;
use crate::schema::{CancelTable, Character, CharacterAssets, PropertyValue, State};
use base64::Engine;
use std::fs;
use std::path::{Component, Path, PathBuf};

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

pub(super) fn project_rules_path(characters_dir: &str) -> PathBuf {
    let project_root = Path::new(characters_dir).parent().unwrap_or(Path::new("."));
    project_root.join("framesmith.rules.json")
}

/// Named states: Vec<(filename_without_extension, State)>
pub(super) type NamedStates = Vec<(String, State)>;

/// Resolve global states and merge them with local states
pub(super) fn resolve_and_merge_globals(
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

pub(super) fn load_character_files(
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
    pub archetype: Option<String>,
    pub move_count: usize,
}

pub fn validate_character_id(id: &str) -> Result<(), String> {
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
    let mut properties = std::collections::BTreeMap::new();
    properties.insert("archetype".to_string(), PropertyValue::String(archetype));
    properties.insert("health".to_string(), PropertyValue::Number(10000.0));
    properties.insert("walk_speed".to_string(), PropertyValue::Number(4.0));
    properties.insert("back_walk_speed".to_string(), PropertyValue::Number(3.0));
    properties.insert("jump_height".to_string(), PropertyValue::Number(120.0));
    properties.insert("jump_duration".to_string(), PropertyValue::Number(45.0));
    properties.insert("dash_distance".to_string(), PropertyValue::Number(80.0));
    properties.insert("dash_duration".to_string(), PropertyValue::Number(18.0));

    let character = Character {
        id: id.clone(),
        name,
        properties,
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

pub fn validate_move_input(input: &str) -> Result<(), String> {
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
        pushboxes: vec![],
        properties: std::collections::BTreeMap::new(),
        base: None,
        id: None,
    };

    // Write the state file
    let content = serde_json::to_string_pretty(&mv)
        .map_err(|e| format!("Failed to serialize move: {}", e))?;
    fs::write(&state_path, content).map_err(|e| format!("Failed to write state file: {}", e))?;

    Ok(mv)
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

    let merged_rules =
        crate::rules::MergedRules::merge(project_rules.as_ref(), character_rules.as_ref());
    let bytes = export_fspk(&char_data, Some(&merged_rules))?;
    Ok(base64::engine::general_purpose::STANDARD.encode(&bytes))
}

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
