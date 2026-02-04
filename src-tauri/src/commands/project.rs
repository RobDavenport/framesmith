use crate::schema::{Character, PropertyValue};
use std::fs;
use std::path::Path;
use tauri_plugin_dialog::DialogExt;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProjectInfo {
    pub name: String,
    pub path: String,
    pub character_count: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CharacterSummary {
    pub id: String,
    pub name: String,
    pub archetype: Option<String>,
    pub move_count: usize,
}

/// Merged rules registry data sent to the frontend.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MergedRegistry {
    pub resources: Vec<String>,
    pub move_types: Option<crate::rules::MoveTypesConfig>,
    pub chain_order: Option<Vec<String>>,
}

fn project_rules_path(characters_dir: &str) -> std::path::PathBuf {
    let project_root = Path::new(characters_dir).parent().unwrap_or(Path::new("."));
    project_root.join("framesmith.rules.json")
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

        let archetype = character.properties.get("archetype").and_then(|v| {
            if let PropertyValue::String(s) = v {
                Some(s.clone())
            } else {
                None
            }
        });

        summaries.push(CharacterSummary {
            id: character.id.clone(),
            name: character.name,
            archetype,
            move_count,
        });
    }

    Ok(summaries)
}

#[tauri::command]
pub fn load_rules_registry(
    characters_dir: String,
    character_id: String,
) -> Result<MergedRegistry, String> {
    // Import validate_character_id from character module
    super::character::validate_character_id(&character_id)?;

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
