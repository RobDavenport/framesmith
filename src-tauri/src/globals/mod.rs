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
}
