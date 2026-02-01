//! Global states loading and resolution

use crate::schema::{GlobalsManifest, State};
use std::collections::HashSet;
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

/// Known State field names for override validation
const KNOWN_STATE_FIELDS: &[&str] = &[
    "id", "input", "name", "type", "tags", "base",
    "startup", "active", "recovery", "total",
    "damage", "hitstun", "blockstun", "hitstop",
    "guard", "animation",
    "hitboxes", "hurtboxes", "pushback",
    "movement", "on_hit", "on_block", "on_use",
    "hits", "preconditions", "costs", "meter_gain", "notifies",
    "trigger", "parent", "super_freeze", "advanced_hurtboxes",
];

/// Resolve all global states for a character
///
/// Returns (resolved_states, warnings)
/// - resolved_states: Global states with overrides applied
/// - warnings: Non-fatal issues (unknown override fields)
pub fn resolve_globals(
    project_dir: &Path,
    character_dir: &Path,
    local_inputs: &HashSet<String>,
) -> Result<(Vec<State>, Vec<String>), GlobalsError> {
    let manifest = match load_globals_manifest(character_dir)? {
        Some(m) => m,
        None => return Ok((Vec::new(), Vec::new())),
    };

    let mut resolved = Vec::new();
    let mut warnings = Vec::new();
    let mut seen_aliases = HashSet::new();

    for include in &manifest.includes {
        // Check for duplicate aliases
        if seen_aliases.contains(&include.alias) {
            return Err(GlobalsError::DuplicateAlias {
                alias: include.alias.clone(),
            });
        }
        seen_aliases.insert(include.alias.clone());

        // Check for conflict with local states
        if local_inputs.contains(&include.alias) {
            return Err(GlobalsError::AliasConflict {
                alias: include.alias.clone(),
            });
        }

        // Load the global state
        let base_state = load_global_state(project_dir, &include.state)?;

        // Validate override fields and collect warnings
        if let Some(ref overrides) = include.overrides {
            for key in overrides.keys() {
                if !KNOWN_STATE_FIELDS.contains(&key.as_str()) {
                    warnings.push(format!(
                        "Override field '{}' not present in global state '{}'",
                        key, include.state
                    ));
                }
            }
        }

        // Apply overrides
        let empty_overrides = serde_json::Map::new();
        let overrides = include.overrides.as_ref().unwrap_or(&empty_overrides);
        let resolved_state = apply_overrides(base_state, overrides, &include.alias)?;

        resolved.push(resolved_state);
    }

    Ok((resolved, warnings))
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

    #[test]
    fn resolve_globals_basic() {
        let dir = create_test_project();
        let char_dir = dir.path().join("characters").join("test_char");

        let manifest = r#"{ "includes": [{ "state": "burst", "as": "burst" }] }"#;
        fs::write(char_dir.join("globals.json"), manifest).unwrap();

        let local_inputs: std::collections::HashSet<String> = std::collections::HashSet::new();

        let (states, warnings) = resolve_globals(dir.path(), &char_dir, &local_inputs).unwrap();
        assert_eq!(states.len(), 1);
        assert_eq!(states[0].input, "burst");
        assert!(warnings.is_empty());
    }

    #[test]
    fn resolve_globals_with_alias() {
        let dir = create_test_project();
        let char_dir = dir.path().join("characters").join("test_char");

        let manifest = r#"{ "includes": [{ "state": "burst", "as": "reversal" }] }"#;
        fs::write(char_dir.join("globals.json"), manifest).unwrap();

        let local_inputs: std::collections::HashSet<String> = std::collections::HashSet::new();

        let (states, _) = resolve_globals(dir.path(), &char_dir, &local_inputs).unwrap();
        assert_eq!(states[0].input, "reversal");
    }

    #[test]
    fn resolve_globals_conflict_with_local() {
        let dir = create_test_project();
        let char_dir = dir.path().join("characters").join("test_char");

        let manifest = r#"{ "includes": [{ "state": "burst", "as": "5L" }] }"#;
        fs::write(char_dir.join("globals.json"), manifest).unwrap();

        let mut local_inputs: std::collections::HashSet<String> = std::collections::HashSet::new();
        local_inputs.insert("5L".to_string());

        let result = resolve_globals(dir.path(), &char_dir, &local_inputs);
        assert!(matches!(result, Err(GlobalsError::AliasConflict { .. })));
    }

    #[test]
    fn resolve_globals_duplicate_alias() {
        let dir = create_test_project();
        let char_dir = dir.path().join("characters").join("test_char");

        // Add another global state
        let idle_state = r#"{ "id": "idle", "input": "idle", "name": "Idle" }"#;
        fs::write(dir.path().join("globals/states/idle.json"), idle_state).unwrap();

        let manifest = r#"{ "includes": [
            { "state": "burst", "as": "same_alias" },
            { "state": "idle", "as": "same_alias" }
        ] }"#;
        fs::write(char_dir.join("globals.json"), manifest).unwrap();

        let local_inputs: std::collections::HashSet<String> = std::collections::HashSet::new();

        let result = resolve_globals(dir.path(), &char_dir, &local_inputs);
        assert!(matches!(result, Err(GlobalsError::DuplicateAlias { .. })));
    }

    #[test]
    fn resolve_globals_warns_on_unknown_override_field() {
        let dir = create_test_project();
        let char_dir = dir.path().join("characters").join("test_char");

        let manifest = r#"{ "includes": [{
            "state": "burst",
            "as": "burst",
            "override": { "nonexistent_field": 123 }
        }] }"#;
        fs::write(char_dir.join("globals.json"), manifest).unwrap();

        let local_inputs: std::collections::HashSet<String> = std::collections::HashSet::new();

        let (_, warnings) = resolve_globals(dir.path(), &char_dir, &local_inputs).unwrap();
        assert!(!warnings.is_empty());
        assert!(warnings[0].contains("nonexistent_field"));
    }

    #[test]
    fn resolve_globals_no_manifest_returns_empty() {
        let dir = create_test_project();
        let char_dir = dir.path().join("characters").join("test_char");
        // No globals.json created

        let local_inputs: std::collections::HashSet<String> = std::collections::HashSet::new();

        let (states, warnings) = resolve_globals(dir.path(), &char_dir, &local_inputs).unwrap();
        assert!(states.is_empty());
        assert!(warnings.is_empty());
    }
}
