//! Global states loading and resolution

use crate::schema::{GlobalsManifest, State};
use std::path::Path;

/// Errors that can occur during global state operations
#[derive(Debug, Clone)]
pub enum GlobalsError {
    /// Global state file not found
    NotFound { state: String },
    /// Alias conflicts with a local state
    AliasConflict { alias: String },
    /// Duplicate alias in includes
    DuplicateAlias { alias: String },
    /// IO error reading file
    IoError { path: String, message: String },
    /// JSON parse error
    ParseError { path: String, message: String },
}

impl std::fmt::Display for GlobalsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GlobalsError::NotFound { state } => {
                write!(f, "Global state '{}' not found in globals/states/", state)
            }
            GlobalsError::AliasConflict { alias } => {
                write!(f, "Global alias '{}' conflicts with local state file", alias)
            }
            GlobalsError::DuplicateAlias { alias } => {
                write!(f, "Duplicate global alias '{}' in globals.json", alias)
            }
            GlobalsError::IoError { path, message } => {
                write!(f, "Failed to read '{}': {}", path, message)
            }
            GlobalsError::ParseError { path, message } => {
                write!(f, "Failed to parse '{}': {}", path, message)
            }
        }
    }
}

impl std::error::Error for GlobalsError {}

/// Load the globals manifest for a character
///
/// Returns None if globals.json doesn't exist (globals are optional)
pub fn load_globals_manifest(character_dir: &Path) -> Result<Option<GlobalsManifest>, GlobalsError> {
    let manifest_path = character_dir.join("globals.json");
    if !manifest_path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&manifest_path).map_err(|e| GlobalsError::IoError {
        path: manifest_path.display().to_string(),
        message: e.to_string(),
    })?;

    let manifest: GlobalsManifest =
        serde_json::from_str(&content).map_err(|e| GlobalsError::ParseError {
            path: manifest_path.display().to_string(),
            message: e.to_string(),
        })?;

    Ok(Some(manifest))
}

/// Load a single global state from the project's globals/states/ directory
pub fn load_global_state(project_dir: &Path, state_name: &str) -> Result<State, GlobalsError> {
    let state_path = project_dir.join("globals").join("states").join(format!("{}.json", state_name));

    if !state_path.exists() {
        return Err(GlobalsError::NotFound {
            state: state_name.to_string(),
        });
    }

    let content = std::fs::read_to_string(&state_path).map_err(|e| GlobalsError::IoError {
        path: state_path.display().to_string(),
        message: e.to_string(),
    })?;

    let state: State = serde_json::from_str(&content).map_err(|e| GlobalsError::ParseError {
        path: state_path.display().to_string(),
        message: e.to_string(),
    })?;

    Ok(state)
}

/// List all available global states in a project
pub fn list_global_states(project_dir: &Path) -> Result<Vec<String>, GlobalsError> {
    let globals_dir = project_dir.join("globals").join("states");

    if !globals_dir.exists() {
        return Ok(Vec::new());
    }

    let mut states = Vec::new();
    let entries = std::fs::read_dir(&globals_dir).map_err(|e| GlobalsError::IoError {
        path: globals_dir.display().to_string(),
        message: e.to_string(),
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| GlobalsError::IoError {
            path: globals_dir.display().to_string(),
            message: e.to_string(),
        })?;

        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "json") {
            if let Some(stem) = path.file_stem() {
                states.push(stem.to_string_lossy().to_string());
            }
        }
    }

    states.sort();
    Ok(states)
}

/// Apply overrides to a global state using shallow merge semantics
///
/// - Top-level fields are replaced entirely
/// - Arrays are replaced (not concatenated)
/// - Nested objects are replaced as units (not deep merged)
/// - Null values remove the field
/// - The alias becomes the new input
pub fn apply_overrides(
    base: State,
    overrides: &serde_json::Map<String, serde_json::Value>,
    alias: &str,
) -> Result<State, GlobalsError> {
    // Convert base to JSON, apply overrides, convert back
    let mut base_json = serde_json::to_value(&base).map_err(|e| GlobalsError::ParseError {
        path: "state serialization".to_string(),
        message: e.to_string(),
    })?;

    if let serde_json::Value::Object(ref mut map) = base_json {
        for (key, value) in overrides {
            if value.is_null() {
                map.remove(key);
            } else {
                map.insert(key.clone(), value.clone());
            }
        }
        // Always set input to alias
        map.insert("input".to_string(), serde_json::Value::String(alias.to_string()));
    }

    let result: State = serde_json::from_value(base_json).map_err(|e| GlobalsError::ParseError {
        path: "state deserialization".to_string(),
        message: e.to_string(),
    })?;

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_project() -> TempDir {
        let dir = TempDir::new().unwrap();

        // Create globals/states directory
        let globals_dir = dir.path().join("globals").join("states");
        fs::create_dir_all(&globals_dir).unwrap();

        // Create a test global state
        let burst_state = r#"{
            "id": "burst",
            "input": "burst",
            "name": "Burst",
            "type": "system",
            "startup": 12,
            "active": 4,
            "recovery": 24,
            "total": 40
        }"#;
        fs::write(globals_dir.join("burst.json"), burst_state).unwrap();

        // Create character directory
        let char_dir = dir.path().join("characters").join("test_char");
        fs::create_dir_all(&char_dir).unwrap();

        dir
    }

    #[test]
    fn load_globals_manifest_missing_returns_none() {
        let dir = create_test_project();
        let char_dir = dir.path().join("characters").join("test_char");

        let result = load_globals_manifest(&char_dir).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn load_globals_manifest_exists() {
        let dir = create_test_project();
        let char_dir = dir.path().join("characters").join("test_char");

        let manifest = r#"{ "includes": [{ "state": "burst", "as": "burst" }] }"#;
        fs::write(char_dir.join("globals.json"), manifest).unwrap();

        let result = load_globals_manifest(&char_dir).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().includes.len(), 1);
    }

    #[test]
    fn load_global_state_success() {
        let dir = create_test_project();

        let state = load_global_state(dir.path(), "burst").unwrap();
        assert_eq!(state.input, "burst");
        assert_eq!(state.startup, 12);
    }

    #[test]
    fn load_global_state_not_found() {
        let dir = create_test_project();

        let result = load_global_state(dir.path(), "nonexistent");
        assert!(matches!(result, Err(GlobalsError::NotFound { .. })));
    }

    #[test]
    fn list_global_states_empty() {
        let dir = TempDir::new().unwrap();
        let states = list_global_states(dir.path()).unwrap();
        assert!(states.is_empty());
    }

    #[test]
    fn list_global_states_found() {
        let dir = create_test_project();
        let states = list_global_states(dir.path()).unwrap();
        assert_eq!(states, vec!["burst"]);
    }

    #[test]
    fn apply_overrides_replaces_top_level_field() {
        let base = State {
            input: "idle".to_string(),
            startup: 5,
            ..Default::default()
        };

        let mut overrides = serde_json::Map::new();
        overrides.insert("startup".to_string(), serde_json::json!(10));

        let result = apply_overrides(base, &overrides, "idle").unwrap();
        assert_eq!(result.startup, 10);
        assert_eq!(result.input, "idle"); // unchanged
    }

    #[test]
    fn apply_overrides_replaces_array_entirely() {
        let base = State {
            input: "idle".to_string(),
            tags: vec![
                crate::schema::Tag::new("normal").unwrap(),
                crate::schema::Tag::new("ground").unwrap(),
            ],
            ..Default::default()
        };

        let mut overrides = serde_json::Map::new();
        overrides.insert("tags".to_string(), serde_json::json!(["special"]));

        let result = apply_overrides(base, &overrides, "idle").unwrap();
        assert_eq!(result.tags.len(), 1);
        assert_eq!(result.tags[0].as_str(), "special");
    }

    #[test]
    fn apply_overrides_replaces_nested_object() {
        let base = State {
            input: "idle".to_string(),
            movement: Some(crate::schema::Movement {
                distance: Some(10),
                direction: Some("forward".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };

        let mut overrides = serde_json::Map::new();
        overrides.insert("movement".to_string(), serde_json::json!({ "distance": 20 }));

        let result = apply_overrides(base, &overrides, "idle").unwrap();
        let movement = result.movement.unwrap();
        assert_eq!(movement.distance, Some(20));
        assert!(movement.direction.is_none()); // replaced entirely, not merged
    }

    #[test]
    fn apply_overrides_sets_alias_as_input() {
        let base = State {
            input: "walk_forward".to_string(),
            ..Default::default()
        };

        let overrides = serde_json::Map::new();
        let result = apply_overrides(base, &overrides, "66").unwrap();
        assert_eq!(result.input, "66"); // alias replaces input
    }

    #[test]
    fn apply_overrides_null_removes_field() {
        let base = State {
            input: "idle".to_string(),
            startup: 5,
            total: Some(20),
            ..Default::default()
        };

        let mut overrides = serde_json::Map::new();
        overrides.insert("total".to_string(), serde_json::Value::Null);

        let result = apply_overrides(base, &overrides, "idle").unwrap();
        assert!(result.total.is_none());
    }
}
