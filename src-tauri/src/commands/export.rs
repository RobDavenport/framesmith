use crate::codegen::{export_fspk, export_json_blob, export_json_blob_pretty};
use crate::schema::State;
use std::fs;

// Import CharacterData and internal helpers from the character module
use super::character::{
    load_character_files, project_rules_path, resolve_and_merge_globals, CharacterData,
};

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
        "fspk" => {
            let merged_rules =
                crate::rules::MergedRules::merge(project_rules.as_ref(), character_rules.as_ref());
            let bytes = export_fspk(&char_data, Some(&merged_rules))?;
            fs::write(&output_path, bytes)
                .map_err(|e| format!("Failed to write export file: {}", e))?;
            return Ok(());
        }
        _ => return Err(format!("Unknown adapter: {}", adapter)),
    };

    fs::write(&output_path, output).map_err(|e| format!("Failed to write export file: {}", e))?;
    Ok(())
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct GlobalStateSummary {
    pub id: String,
    pub name: String,
    pub r#type: Option<String>,
}

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
