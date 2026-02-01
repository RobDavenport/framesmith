# Global States Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add support for global states - shared state definitions at the project level that characters can opt into with explicit includes and shallow-merge overrides.

**Architecture:** Global states are stored in `globals/states/` at the project root. Characters opt-in via `globals.json` with explicit includes. Resolution happens during character loading, before rules application. Shallow merge semantics keep the mental model simple.

**Tech Stack:** Rust (Tauri backend), TypeScript/Svelte frontend, Vitest for TS tests, inline Rust tests.

---

## Task 1: Add GlobalInclude Schema Types

**Files:**
- Modify: `src-tauri/src/schema/mod.rs`

**Step 1: Write the failing test for GlobalInclude deserialization**

Add to the `#[cfg(test)] mod tests` section at the end of the file:

```rust
#[test]
fn global_include_basic() {
    let json = r#"{ "state": "burst", "as": "burst" }"#;
    let include: GlobalInclude = serde_json::from_str(json).unwrap();
    assert_eq!(include.state, "burst");
    assert_eq!(include.alias, "burst");
    assert!(include.overrides.is_none());
}

#[test]
fn global_include_with_override() {
    let json = r#"{
        "state": "idle",
        "as": "idle",
        "override": { "animation": "ryu_idle" }
    }"#;
    let include: GlobalInclude = serde_json::from_str(json).unwrap();
    assert_eq!(include.state, "idle");
    assert!(include.overrides.is_some());
    let overrides = include.overrides.unwrap();
    assert_eq!(overrides.get("animation").unwrap(), "ryu_idle");
}

#[test]
fn globals_manifest_deserialization() {
    let json = r#"{
        "includes": [
            { "state": "burst", "as": "burst" },
            { "state": "idle", "as": "idle", "override": { "animation": "ryu_idle" } }
        ]
    }"#;
    let manifest: GlobalsManifest = serde_json::from_str(json).unwrap();
    assert_eq!(manifest.includes.len(), 2);
}
```

**Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test global_include`
Expected: FAIL - "cannot find type `GlobalInclude`"

**Step 3: Implement GlobalInclude and GlobalsManifest types**

Add after the `CancelTable` struct (around line 280):

```rust
/// A reference to a global state with optional overrides
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GlobalInclude {
    /// Name of the global state file (without .json)
    pub state: String,
    /// Alias for this character (the input name to use)
    #[serde(rename = "as")]
    pub alias: String,
    /// Optional field overrides (shallow merge)
    #[serde(rename = "override", skip_serializing_if = "Option::is_none")]
    pub overrides: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Character's global state manifest (globals.json)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GlobalsManifest {
    pub includes: Vec<GlobalInclude>,
}
```

**Step 4: Run tests to verify they pass**

Run: `cd src-tauri && cargo test global_include`
Expected: All 3 tests PASS

**Step 5: Commit**

```bash
cd src-tauri && git add src/schema/mod.rs
git commit -m "$(cat <<'EOF'
feat(schema): add GlobalInclude and GlobalsManifest types

Add schema types for global states opt-in system:
- GlobalInclude: references a global state with alias and optional overrides
- GlobalsManifest: character's globals.json structure

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: Add Global State Loading Functions

**Files:**
- Create: `src-tauri/src/globals/mod.rs`
- Modify: `src-tauri/src/lib.rs` (add module declaration)

**Step 1: Create the globals module with failing test**

Create `src-tauri/src/globals/mod.rs`:

```rust
//! Global states loading and resolution

use crate::schema::{GlobalsManifest, State};
use std::collections::HashMap;
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
        if path.extension().map_or(false, |ext| ext == "json") {
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
        assert_eq!(state.startup, Some(12));
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
```

**Step 2: Add module declaration to lib.rs**

In `src-tauri/src/lib.rs`, add after `mod variant;`:

```rust
mod globals;
```

**Step 3: Run tests to verify they pass**

Run: `cd src-tauri && cargo test globals::`
Expected: All 6 tests PASS

**Step 4: Commit**

```bash
cd src-tauri && git add src/globals/mod.rs src/lib.rs
git commit -m "$(cat <<'EOF'
feat(globals): add global state loading functions

Add module for loading global states:
- load_globals_manifest: loads character's globals.json (optional)
- load_global_state: loads a single global state from project
- list_global_states: lists all available global states
- GlobalsError: error types with descriptive messages

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: Implement Shallow Merge for State Overrides

**Files:**
- Modify: `src-tauri/src/globals/mod.rs`

**Step 1: Write failing tests for apply_overrides**

Add to the `#[cfg(test)] mod tests` section:

```rust
#[test]
fn apply_overrides_replaces_top_level_field() {
    let base = State {
        input: "idle".to_string(),
        startup: Some(5),
        ..Default::default()
    };

    let mut overrides = serde_json::Map::new();
    overrides.insert("startup".to_string(), serde_json::json!(10));

    let result = apply_overrides(base, &overrides, "idle").unwrap();
    assert_eq!(result.startup, Some(10));
    assert_eq!(result.input, "idle"); // unchanged
}

#[test]
fn apply_overrides_replaces_array_entirely() {
    let base = State {
        input: "idle".to_string(),
        tags: Some(vec!["normal".to_string(), "ground".to_string()]),
        ..Default::default()
    };

    let mut overrides = serde_json::Map::new();
    overrides.insert("tags".to_string(), serde_json::json!(["special"]));

    let result = apply_overrides(base, &overrides, "idle").unwrap();
    assert_eq!(result.tags, Some(vec!["special".to_string()]));
}

#[test]
fn apply_overrides_replaces_nested_object() {
    let base = State {
        input: "idle".to_string(),
        movement: Some(crate::schema::Movement {
            x: Some(10.0),
            y: Some(5.0),
            ..Default::default()
        }),
        ..Default::default()
    };

    let mut overrides = serde_json::Map::new();
    overrides.insert("movement".to_string(), serde_json::json!({ "x": 20.0 }));

    let result = apply_overrides(base, &overrides, "idle").unwrap();
    let movement = result.movement.unwrap();
    assert_eq!(movement.x, Some(20.0));
    assert!(movement.y.is_none()); // replaced entirely, not merged
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
        startup: Some(5),
        ..Default::default()
    };

    let mut overrides = serde_json::Map::new();
    overrides.insert("startup".to_string(), serde_json::Value::Null);

    let result = apply_overrides(base, &overrides, "idle").unwrap();
    assert!(result.startup.is_none());
}
```

**Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test apply_overrides`
Expected: FAIL - "cannot find function `apply_overrides`"

**Step 3: Implement apply_overrides function**

Add before the `#[cfg(test)]` section in `src-tauri/src/globals/mod.rs`:

```rust
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
```

**Step 4: Run tests to verify they pass**

Run: `cd src-tauri && cargo test apply_overrides`
Expected: All 5 tests PASS

**Step 5: Commit**

```bash
cd src-tauri && git add src/globals/mod.rs
git commit -m "$(cat <<'EOF'
feat(globals): implement shallow merge for state overrides

Add apply_overrides function with shallow merge semantics:
- Top-level fields replaced entirely
- Arrays replaced (not concatenated)
- Nested objects replaced as units
- Null values remove fields
- Alias becomes the new input

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: Implement Full Global Resolution with Validation

**Files:**
- Modify: `src-tauri/src/globals/mod.rs`

**Step 1: Write failing tests for resolve_globals**

Add to the `#[cfg(test)] mod tests` section:

```rust
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
```

**Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test resolve_globals`
Expected: FAIL - "cannot find function `resolve_globals`"

**Step 3: Implement resolve_globals function**

Add before the `#[cfg(test)]` section:

```rust
use std::collections::HashSet;

/// Known State field names for override validation
const KNOWN_STATE_FIELDS: &[&str] = &[
    "id", "input", "name", "type", "tags", "base",
    "startup", "active", "recovery", "total",
    "damage", "chip_damage", "hitstun", "blockstun", "hitstop",
    "guard", "animation", "animation_offset",
    "hitboxes", "hurtboxes", "pushboxes",
    "movement", "on_hit", "on_block", "on_use",
    "hit", "precondition", "cost", "meter_gain", "notifies",
    "can_be_canceled_by", "meter_on_hit", "meter_on_whiff",
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
```

**Step 4: Run tests to verify they pass**

Run: `cd src-tauri && cargo test resolve_globals`
Expected: All 6 tests PASS

**Step 5: Run all globals tests**

Run: `cd src-tauri && cargo test globals::`
Expected: All 17 tests PASS

**Step 6: Commit**

```bash
cd src-tauri && git add src/globals/mod.rs
git commit -m "$(cat <<'EOF'
feat(globals): implement full global resolution with validation

Add resolve_globals function that:
- Loads character's globals.json manifest
- Validates for duplicate aliases
- Validates for conflicts with local states
- Warns on unknown override fields
- Applies overrides and returns resolved states

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: Integrate Globals into Character Loading

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/globals/mod.rs` (make public)

**Step 1: Make globals module public**

In `src-tauri/src/lib.rs`, change:

```rust
mod globals;
```

to:

```rust
pub mod globals;
```

**Step 2: Update load_character_files to return local inputs**

In `src-tauri/src/commands.rs`, modify the `load_character_files` function signature and return type. Add to the return tuple tracking of local inputs:

First, find the current function (around line 43) and note its return type. We need to also return a `HashSet<String>` of local state inputs.

**Step 3: Integrate globals into load_character**

In `src-tauri/src/commands.rs`, in the `load_character` function (around line 214), after loading local states but before flattening variants:

```rust
// After: let (char_path, character, named_states, cancel_table) = load_character_files(...)?;

// Collect local inputs for conflict detection
let local_inputs: std::collections::HashSet<String> = named_states
    .iter()
    .map(|(_, state)| state.input.clone())
    .collect();

// Resolve global states
let project_dir = char_path.parent().and_then(|p| p.parent()).ok_or_else(|| {
    "Could not determine project directory".to_string()
})?;

let (global_states, global_warnings) = crate::globals::resolve_globals(
    project_dir,
    &char_path,
    &local_inputs,
).map_err(|e| e.to_string())?;

// Log warnings (could also return them to frontend)
for warning in &global_warnings {
    eprintln!("Warning: {}", warning);
}

// Combine: local states + global states (as named tuples for variant processing)
let mut all_named_states = named_states;
for state in global_states {
    all_named_states.push((state.input.clone(), state));
}

// Continue with flatten_variants using all_named_states instead of named_states
```

**Step 4: Write integration test**

Create `src-tauri/tests/globals_integration.rs`:

```rust
//! Integration tests for global states

use std::fs;
use tempfile::TempDir;

fn create_test_project_with_globals() -> TempDir {
    let dir = TempDir::new().unwrap();

    // Create project structure
    fs::create_dir_all(dir.path().join("globals/states")).unwrap();
    fs::create_dir_all(dir.path().join("characters/test_char/states")).unwrap();

    // Project rules
    fs::write(
        dir.path().join("framesmith.rules.json"),
        r#"{ "version": 1 }"#,
    ).unwrap();

    // Global state
    fs::write(
        dir.path().join("globals/states/burst.json"),
        r#"{
            "id": "burst",
            "input": "burst",
            "name": "Burst",
            "type": "system",
            "startup": 12,
            "active": 4,
            "recovery": 24,
            "total": 40
        }"#,
    ).unwrap();

    // Character
    fs::write(
        dir.path().join("characters/test_char/character.json"),
        r#"{ "id": "test_char", "name": "Test Character", "health": 10000 }"#,
    ).unwrap();

    // Local state
    fs::write(
        dir.path().join("characters/test_char/states/5L.json"),
        r#"{ "input": "5L", "name": "Light Attack", "startup": 5 }"#,
    ).unwrap();

    // Cancel table
    fs::write(
        dir.path().join("characters/test_char/cancel_table.json"),
        r#"{ "cancels": [] }"#,
    ).unwrap();

    // Globals manifest
    fs::write(
        dir.path().join("characters/test_char/globals.json"),
        r#"{ "includes": [{ "state": "burst", "as": "burst" }] }"#,
    ).unwrap();

    dir
}

#[test]
fn test_character_includes_global_states() {
    // This test verifies that global states are included in character loading
    // Note: This would require importing the Tauri commands which may need
    // additional setup. For now, we test the globals module directly.

    use std::collections::HashSet;

    let dir = create_test_project_with_globals();
    let project_dir = dir.path();
    let char_dir = project_dir.join("characters/test_char");

    let local_inputs: HashSet<String> = ["5L".to_string()].into_iter().collect();

    let (globals, warnings) = framesmith::globals::resolve_globals(
        project_dir,
        &char_dir,
        &local_inputs,
    ).unwrap();

    assert_eq!(globals.len(), 1);
    assert_eq!(globals[0].input, "burst");
    assert!(warnings.is_empty());
}
```

**Step 5: Run the integration test**

Run: `cd src-tauri && cargo test globals_integration`
Expected: PASS

**Step 6: Run clippy**

Run: `cd src-tauri && cargo clippy --all-targets`
Expected: No warnings

**Step 7: Commit**

```bash
cd src-tauri && git add src/lib.rs src/commands.rs tests/globals_integration.rs src/globals/mod.rs
git commit -m "$(cat <<'EOF'
feat(commands): integrate global states into character loading

Global states are now resolved during character loading:
- Loaded after local states
- Conflict detection with local inputs
- Combined before variant flattening
- Warnings logged for unknown override fields

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 6: Add Global States to Export Pipeline

**Files:**
- Modify: `src-tauri/src/codegen/json_blob.rs`
- Modify: `src-tauri/src/codegen/zx_fspack.rs` (if needed)

**Step 1: Verify exports already work**

Since globals are resolved before export and become regular states, the export should already include them. Write a test to verify:

In `src-tauri/src/codegen/json_blob.rs`, add to tests:

```rust
#[test]
fn export_includes_global_derived_states() {
    // Global states become regular states after resolution,
    // so they should be included in exports automatically.
    // This test documents that expectation.

    use crate::schema::{State, Character, CancelTable};

    let states = vec![
        State {
            id: Some("5L".to_string()),
            input: "5L".to_string(),
            name: Some("Light Attack".to_string()),
            ..Default::default()
        },
        State {
            id: Some("burst".to_string()),
            input: "burst".to_string(),
            name: Some("Burst".to_string()),
            r#type: Some("system".to_string()),
            ..Default::default()
        },
    ];

    let character = Character {
        id: "test".to_string(),
        name: "Test".to_string(),
        ..Default::default()
    };

    let cancel_table = CancelTable::default();

    let json = export_json_blob(&states, &character, &cancel_table);

    // Verify both states are in the export
    assert!(json.contains("\"5L\""));
    assert!(json.contains("\"burst\""));
    assert!(json.contains("\"system\"")); // type field from global
}
```

**Step 2: Run the test**

Run: `cd src-tauri && cargo test export_includes_global`
Expected: PASS

**Step 3: Commit**

```bash
cd src-tauri && git add src/codegen/json_blob.rs
git commit -m "$(cat <<'EOF'
test(codegen): verify global states included in exports

Add test documenting that global-derived states are included
in exports since they become regular states after resolution.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 7: Add MCP Tools for Global States

**Files:**
- Modify: `src-tauri/src/bin/mcp.rs`

**Step 1: Add list_global_states tool**

In the MCP server, add a new tool parameter struct and handler:

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListGlobalStatesParam {}

// In the tool handler match:
"list_global_states" => {
    let project_dir = get_project_dir()?;
    let states = framesmith::globals::list_global_states(&project_dir)
        .map_err(|e| mcp_error(format!("Failed to list global states: {}", e)))?;

    let result: Vec<serde_json::Value> = states
        .iter()
        .map(|name| {
            let state = framesmith::globals::load_global_state(&project_dir, name).ok();
            serde_json::json!({
                "id": name,
                "name": state.as_ref().and_then(|s| s.name.clone()),
                "type": state.as_ref().and_then(|s| s.r#type.clone()),
            })
        })
        .collect();

    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string_pretty(&result).unwrap(),
    )]))
}
```

**Step 2: Add get_global_state tool**

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GlobalStateIdParam {
    /// The ID of the global state
    pub id: String,
}

// In the tool handler match:
"get_global_state" => {
    let params: GlobalStateIdParam = serde_json::from_value(arguments.clone())
        .map_err(|e| mcp_error(format!("Invalid parameters: {}", e)))?;

    let project_dir = get_project_dir()?;
    let state = framesmith::globals::load_global_state(&project_dir, &params.id)
        .map_err(|e| mcp_error(format!("Failed to load global state: {}", e)))?;

    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string_pretty(&state).unwrap(),
    )]))
}
```

**Step 3: Add create_global_state tool**

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateGlobalStateParam {
    /// The ID for the new global state
    pub id: String,
    /// The state data
    pub state: serde_json::Value,
}

// In the tool handler match:
"create_global_state" => {
    let params: CreateGlobalStateParam = serde_json::from_value(arguments.clone())
        .map_err(|e| mcp_error(format!("Invalid parameters: {}", e)))?;

    let project_dir = get_project_dir()?;
    let globals_dir = project_dir.join("globals").join("states");

    // Create directory if needed
    std::fs::create_dir_all(&globals_dir)
        .map_err(|e| mcp_error(format!("Failed to create globals directory: {}", e)))?;

    let state_path = globals_dir.join(format!("{}.json", params.id));

    if state_path.exists() {
        return Err(mcp_error(format!("Global state '{}' already exists", params.id)));
    }

    // Validate state structure
    let _state: framesmith::schema::State = serde_json::from_value(params.state.clone())
        .map_err(|e| mcp_error(format!("Invalid state data: {}", e)))?;

    std::fs::write(&state_path, serde_json::to_string_pretty(&params.state).unwrap())
        .map_err(|e| mcp_error(format!("Failed to write global state: {}", e)))?;

    Ok(CallToolResult::success(vec![Content::text(
        format!("Created global state '{}'", params.id),
    )]))
}
```

**Step 4: Add update_global_state tool**

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateGlobalStateParam {
    /// The ID of the global state to update
    pub id: String,
    /// The updated state data
    pub state: serde_json::Value,
}

// In the tool handler match:
"update_global_state" => {
    let params: UpdateGlobalStateParam = serde_json::from_value(arguments.clone())
        .map_err(|e| mcp_error(format!("Invalid parameters: {}", e)))?;

    let project_dir = get_project_dir()?;
    let state_path = project_dir.join("globals").join("states").join(format!("{}.json", params.id));

    if !state_path.exists() {
        return Err(mcp_error(format!("Global state '{}' not found", params.id)));
    }

    // Validate state structure
    let _state: framesmith::schema::State = serde_json::from_value(params.state.clone())
        .map_err(|e| mcp_error(format!("Invalid state data: {}", e)))?;

    std::fs::write(&state_path, serde_json::to_string_pretty(&params.state).unwrap())
        .map_err(|e| mcp_error(format!("Failed to write global state: {}", e)))?;

    Ok(CallToolResult::success(vec![Content::text(
        format!("Updated global state '{}'", params.id),
    )]))
}
```

**Step 5: Add delete_global_state tool**

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeleteGlobalStateParam {
    /// The ID of the global state to delete
    pub id: String,
}

// In the tool handler match:
"delete_global_state" => {
    let params: DeleteGlobalStateParam = serde_json::from_value(arguments.clone())
        .map_err(|e| mcp_error(format!("Invalid parameters: {}", e)))?;

    let project_dir = get_project_dir()?;
    let state_path = project_dir.join("globals").join("states").join(format!("{}.json", params.id));

    if !state_path.exists() {
        return Err(mcp_error(format!("Global state '{}' not found", params.id)));
    }

    // Check if any characters reference this state
    let characters_dir = project_dir.join("characters");
    if characters_dir.exists() {
        for entry in std::fs::read_dir(&characters_dir).map_err(|e| mcp_error(e.to_string()))? {
            let entry = entry.map_err(|e| mcp_error(e.to_string()))?;
            let globals_path = entry.path().join("globals.json");
            if globals_path.exists() {
                let content = std::fs::read_to_string(&globals_path)
                    .map_err(|e| mcp_error(e.to_string()))?;
                if content.contains(&format!("\"state\": \"{}\"", params.id)) {
                    return Err(mcp_error(format!(
                        "Cannot delete: global state '{}' is referenced by character '{}'",
                        params.id,
                        entry.file_name().to_string_lossy()
                    )));
                }
            }
        }
    }

    std::fs::remove_file(&state_path)
        .map_err(|e| mcp_error(format!("Failed to delete global state: {}", e)))?;

    Ok(CallToolResult::success(vec![Content::text(
        format!("Deleted global state '{}'", params.id),
    )]))
}
```

**Step 6: Register tools in list_tools**

Add to the tools vector in the `list_tools` handler:

```rust
ToolInfo {
    name: "list_global_states".into(),
    description: Some("List all global states in the project".into()),
    input_schema: schemars::schema_for!(ListGlobalStatesParam).schema.into(),
},
ToolInfo {
    name: "get_global_state".into(),
    description: Some("Get a specific global state by ID".into()),
    input_schema: schemars::schema_for!(GlobalStateIdParam).schema.into(),
},
ToolInfo {
    name: "create_global_state".into(),
    description: Some("Create a new global state".into()),
    input_schema: schemars::schema_for!(CreateGlobalStateParam).schema.into(),
},
ToolInfo {
    name: "update_global_state".into(),
    description: Some("Update an existing global state".into()),
    input_schema: schemars::schema_for!(UpdateGlobalStateParam).schema.into(),
},
ToolInfo {
    name: "delete_global_state".into(),
    description: Some("Delete a global state (checks for references first)".into()),
    input_schema: schemars::schema_for!(DeleteGlobalStateParam).schema.into(),
},
```

**Step 7: Build and verify**

Run: `cd src-tauri && cargo build --bin mcp`
Expected: Build succeeds

**Step 8: Commit**

```bash
cd src-tauri && git add src/bin/mcp.rs
git commit -m "$(cat <<'EOF'
feat(mcp): add global state management tools

Add MCP tools for managing global states:
- list_global_states: list all globals in project
- get_global_state: get a specific global by ID
- create_global_state: create new global with validation
- update_global_state: update existing global
- delete_global_state: delete with reference checking

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 8: Add Tauri Commands for Frontend

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs` (register commands)

**Step 1: Add list_global_states command**

In `src-tauri/src/commands.rs`, add:

```rust
#[derive(Debug, Clone, Serialize)]
pub struct GlobalStateSummary {
    pub id: String,
    pub name: Option<String>,
    pub r#type: Option<String>,
}

#[tauri::command]
pub fn list_global_states(project_path: String) -> Result<Vec<GlobalStateSummary>, String> {
    let project_dir = std::path::Path::new(&project_path);

    let state_ids = crate::globals::list_global_states(project_dir)
        .map_err(|e| e.to_string())?;

    let mut summaries = Vec::new();
    for id in state_ids {
        let state = crate::globals::load_global_state(project_dir, &id)
            .map_err(|e| e.to_string())?;
        summaries.push(GlobalStateSummary {
            id,
            name: state.name,
            r#type: state.r#type,
        });
    }

    Ok(summaries)
}
```

**Step 2: Add get_global_state command**

```rust
#[tauri::command]
pub fn get_global_state(project_path: String, state_id: String) -> Result<schema::State, String> {
    let project_dir = std::path::Path::new(&project_path);
    crate::globals::load_global_state(project_dir, &state_id)
        .map_err(|e| e.to_string())
}
```

**Step 3: Add save_global_state command**

```rust
#[tauri::command]
pub fn save_global_state(
    project_path: String,
    state_id: String,
    state: schema::State,
) -> Result<(), String> {
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
```

**Step 4: Add delete_global_state command**

```rust
#[tauri::command]
pub fn delete_global_state(project_path: String, state_id: String) -> Result<(), String> {
    let project_dir = std::path::Path::new(&project_path);
    let state_path = project_dir.join("globals").join("states").join(format!("{}.json", state_id));

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
                if content.contains(&format!("\"state\": \"{}\"", state_id)) {
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
```

**Step 5: Register commands in lib.rs**

In `src-tauri/src/lib.rs`, add to the `invoke_handler` list:

```rust
commands::list_global_states,
commands::get_global_state,
commands::save_global_state,
commands::delete_global_state,
```

**Step 6: Build and test**

Run: `cd src-tauri && cargo build`
Expected: Build succeeds

**Step 7: Commit**

```bash
cd src-tauri && git add src/commands.rs src/lib.rs
git commit -m "$(cat <<'EOF'
feat(commands): add Tauri commands for global states

Add frontend-accessible commands:
- list_global_states: list all with summaries
- get_global_state: get full state data
- save_global_state: create or update
- delete_global_state: delete with reference checking

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 9: Add TypeScript Types for Globals

**Files:**
- Modify: `src/lib/types.ts`

**Step 1: Add GlobalInclude type**

In `src/lib/types.ts`, add after the existing types:

```typescript
/** A reference to a global state with optional overrides */
export interface GlobalInclude {
  /** Name of the global state file (without .json) */
  state: string;
  /** Alias for this character (the input name to use) */
  as: string;
  /** Optional field overrides (shallow merge) */
  override?: Record<string, unknown>;
}

/** Character's global state manifest */
export interface GlobalsManifest {
  includes: GlobalInclude[];
}

/** Summary of a global state for listing */
export interface GlobalStateSummary {
  id: string;
  name?: string;
  type?: string;
}
```

**Step 2: Run type checking**

Run: `npm run check`
Expected: No errors

**Step 3: Commit**

```bash
git add src/lib/types.ts
git commit -m "$(cat <<'EOF'
feat(types): add TypeScript types for global states

Add GlobalInclude, GlobalsManifest, and GlobalStateSummary
types matching the Rust schema.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 10: Add Globals Store

**Files:**
- Create: `src/lib/stores/globals.svelte.ts`

**Step 1: Create the globals store**

```typescript
/**
 * Store for managing global states at the project level
 */
import { invoke } from '@tauri-apps/api/core';
import type { State, GlobalStateSummary } from '$lib/types';
import { getProjectPath } from './project.svelte';

// Reactive state
let globalStateList = $state<GlobalStateSummary[]>([]);
let currentGlobalState = $state<State | null>(null);
let selectedGlobalId = $state<string | null>(null);
let loading = $state(false);
let error = $state<string | null>(null);

// Getters
export function getGlobalStateList(): GlobalStateSummary[] {
  return globalStateList;
}

export function getCurrentGlobalState(): State | null {
  return currentGlobalState;
}

export function getSelectedGlobalId(): string | null {
  return selectedGlobalId;
}

export function isLoading(): boolean {
  return loading;
}

export function getError(): string | null {
  return error;
}

// Actions
export async function loadGlobalStateList(): Promise<void> {
  const projectPath = getProjectPath();
  if (!projectPath) {
    globalStateList = [];
    return;
  }

  loading = true;
  error = null;

  try {
    globalStateList = await invoke<GlobalStateSummary[]>('list_global_states', {
      projectPath,
    });
  } catch (e) {
    error = e instanceof Error ? e.message : String(e);
    globalStateList = [];
  } finally {
    loading = false;
  }
}

export async function selectGlobalState(id: string | null): Promise<void> {
  selectedGlobalId = id;

  if (!id) {
    currentGlobalState = null;
    return;
  }

  const projectPath = getProjectPath();
  if (!projectPath) {
    error = 'No project open';
    return;
  }

  loading = true;
  error = null;

  try {
    currentGlobalState = await invoke<State>('get_global_state', {
      projectPath,
      stateId: id,
    });
  } catch (e) {
    error = e instanceof Error ? e.message : String(e);
    currentGlobalState = null;
  } finally {
    loading = false;
  }
}

export async function saveGlobalState(id: string, state: State): Promise<boolean> {
  const projectPath = getProjectPath();
  if (!projectPath) {
    error = 'No project open';
    return false;
  }

  loading = true;
  error = null;

  try {
    await invoke('save_global_state', {
      projectPath,
      stateId: id,
      state,
    });

    // Refresh list
    await loadGlobalStateList();

    // If this is the current state, refresh it
    if (selectedGlobalId === id) {
      await selectGlobalState(id);
    }

    return true;
  } catch (e) {
    error = e instanceof Error ? e.message : String(e);
    return false;
  } finally {
    loading = false;
  }
}

export async function deleteGlobalState(id: string): Promise<boolean> {
  const projectPath = getProjectPath();
  if (!projectPath) {
    error = 'No project open';
    return false;
  }

  loading = true;
  error = null;

  try {
    await invoke('delete_global_state', {
      projectPath,
      stateId: id,
    });

    // Clear selection if deleted
    if (selectedGlobalId === id) {
      selectedGlobalId = null;
      currentGlobalState = null;
    }

    // Refresh list
    await loadGlobalStateList();

    return true;
  } catch (e) {
    error = e instanceof Error ? e.message : String(e);
    return false;
  } finally {
    loading = false;
  }
}

export async function createGlobalState(id: string, state: State): Promise<boolean> {
  return saveGlobalState(id, state);
}

// Reset on project change
export function resetGlobalsStore(): void {
  globalStateList = [];
  currentGlobalState = null;
  selectedGlobalId = null;
  loading = false;
  error = null;
}
```

**Step 2: Run type checking**

Run: `npm run check`
Expected: No errors

**Step 3: Commit**

```bash
git add src/lib/stores/globals.svelte.ts
git commit -m "$(cat <<'EOF'
feat(stores): add globals store for project-level states

Add reactive store for managing global states:
- List, select, save, delete operations
- Integrates with project store
- Error handling and loading states

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 11: Create GlobalStateList Component

**Files:**
- Create: `src/lib/components/GlobalStateList.svelte`

**Step 1: Create the component**

```svelte
<script lang="ts">
  import {
    getGlobalStateList,
    getSelectedGlobalId,
    selectGlobalState,
    deleteGlobalState,
    isLoading,
  } from '$lib/stores/globals.svelte';

  let showDeleteConfirm = $state(false);
  let deleteTargetId = $state<string | null>(null);

  const globalStates = $derived(getGlobalStateList());
  const selectedId = $derived(getSelectedGlobalId());
  const loading = $derived(isLoading());

  function handleSelect(id: string) {
    selectGlobalState(id);
  }

  function handleDeleteClick(id: string, event: MouseEvent) {
    event.stopPropagation();
    deleteTargetId = id;
    showDeleteConfirm = true;
  }

  async function confirmDelete() {
    if (deleteTargetId) {
      await deleteGlobalState(deleteTargetId);
    }
    showDeleteConfirm = false;
    deleteTargetId = null;
  }

  function cancelDelete() {
    showDeleteConfirm = false;
    deleteTargetId = null;
  }
</script>

<div class="global-state-list">
  <div class="header">
    <h3>Global States</h3>
    <span class="indicator" title="Project-wide shared states">🌐</span>
  </div>

  {#if loading}
    <div class="loading">Loading...</div>
  {:else if globalStates.length === 0}
    <div class="empty">
      <p>No global states defined.</p>
      <p class="hint">Create globals in <code>globals/states/</code></p>
    </div>
  {:else}
    <ul class="state-list">
      {#each globalStates as state (state.id)}
        <li
          class="state-item"
          class:selected={selectedId === state.id}
          onclick={() => handleSelect(state.id)}
          onkeydown={(e) => e.key === 'Enter' && handleSelect(state.id)}
          role="button"
          tabindex="0"
        >
          <div class="state-info">
            <span class="state-id">{state.id}</span>
            {#if state.type}
              <span class="state-type">{state.type}</span>
            {/if}
          </div>
          {#if state.name && state.name !== state.id}
            <span class="state-name">{state.name}</span>
          {/if}
          <button
            class="delete-btn"
            onclick={(e) => handleDeleteClick(state.id, e)}
            title="Delete global state"
          >
            ×
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

{#if showDeleteConfirm}
  <div class="modal-overlay" onclick={cancelDelete} role="presentation">
    <div class="modal" onclick={(e) => e.stopPropagation()} role="dialog">
      <h4>Delete Global State</h4>
      <p>Are you sure you want to delete <strong>{deleteTargetId}</strong>?</p>
      <p class="warning">This may break characters that reference it.</p>
      <div class="modal-actions">
        <button onclick={cancelDelete}>Cancel</button>
        <button class="danger" onclick={confirmDelete}>Delete</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .global-state-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem;
    border-bottom: 1px solid var(--border-color, #333);
  }

  .header h3 {
    margin: 0;
    font-size: 0.9rem;
  }

  .indicator {
    font-size: 0.8rem;
  }

  .loading,
  .empty {
    padding: 1rem;
    text-align: center;
    color: var(--text-muted, #888);
  }

  .empty .hint {
    font-size: 0.8rem;
    margin-top: 0.5rem;
  }

  .empty code {
    background: var(--bg-secondary, #222);
    padding: 0.1rem 0.3rem;
    border-radius: 3px;
  }

  .state-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .state-item {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    padding: 0.5rem;
    cursor: pointer;
    border-left: 3px solid transparent;
    position: relative;
  }

  .state-item:hover {
    background: var(--bg-hover, #2a2a2a);
  }

  .state-item.selected {
    background: var(--bg-selected, #333);
    border-left-color: var(--accent-color, #4a9eff);
  }

  .state-info {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .state-id {
    font-family: monospace;
    font-weight: 500;
  }

  .state-type {
    font-size: 0.75rem;
    padding: 0.1rem 0.3rem;
    background: var(--bg-secondary, #222);
    border-radius: 3px;
    color: var(--text-muted, #888);
  }

  .state-name {
    font-size: 0.8rem;
    color: var(--text-muted, #888);
  }

  .delete-btn {
    position: absolute;
    right: 0.5rem;
    top: 50%;
    transform: translateY(-50%);
    background: none;
    border: none;
    color: var(--text-muted, #888);
    cursor: pointer;
    font-size: 1.2rem;
    padding: 0.2rem 0.4rem;
    opacity: 0;
    transition: opacity 0.15s;
  }

  .state-item:hover .delete-btn {
    opacity: 1;
  }

  .delete-btn:hover {
    color: var(--danger-color, #ff4444);
  }

  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal {
    background: var(--bg-primary, #1a1a1a);
    padding: 1.5rem;
    border-radius: 8px;
    max-width: 400px;
    width: 90%;
  }

  .modal h4 {
    margin: 0 0 1rem;
  }

  .modal .warning {
    color: var(--warning-color, #ff9944);
    font-size: 0.9rem;
  }

  .modal-actions {
    display: flex;
    gap: 0.5rem;
    justify-content: flex-end;
    margin-top: 1rem;
  }

  .modal-actions button {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 4px;
    cursor: pointer;
  }

  .modal-actions button.danger {
    background: var(--danger-color, #ff4444);
    color: white;
  }
</style>
```

**Step 2: Run type checking**

Run: `npm run check`
Expected: No errors

**Step 3: Commit**

```bash
git add src/lib/components/GlobalStateList.svelte
git commit -m "$(cat <<'EOF'
feat(ui): add GlobalStateList component

Display project-wide global states with:
- Visual indicator for global states
- Selection highlighting
- Delete with confirmation
- Empty state guidance

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 12: Create GlobalStateEditor Component

**Files:**
- Create: `src/lib/components/GlobalStateEditor.svelte`

**Step 1: Create the component**

```svelte
<script lang="ts">
  import {
    getCurrentGlobalState,
    getSelectedGlobalId,
    saveGlobalState,
    getError,
  } from '$lib/stores/globals.svelte';
  import type { State } from '$lib/types';

  // Local editing state
  let editingState = $state<State | null>(null);
  let hasChanges = $state(false);
  let saving = $state(false);
  let saveError = $state<string | null>(null);

  const currentState = $derived(getCurrentGlobalState());
  const selectedId = $derived(getSelectedGlobalId());
  const storeError = $derived(getError());

  // Sync local state when selection changes
  $effect(() => {
    if (currentState) {
      editingState = structuredClone(currentState);
      hasChanges = false;
      saveError = null;
    } else {
      editingState = null;
    }
  });

  function handleFieldChange(field: keyof State, value: unknown) {
    if (!editingState) return;
    (editingState as Record<string, unknown>)[field] = value;
    hasChanges = true;
  }

  async function handleSave() {
    if (!editingState || !selectedId) return;

    saving = true;
    saveError = null;

    const success = await saveGlobalState(selectedId, editingState);

    if (success) {
      hasChanges = false;
    } else {
      saveError = storeError;
    }

    saving = false;
  }

  function handleRevert() {
    if (currentState) {
      editingState = structuredClone(currentState);
      hasChanges = false;
      saveError = null;
    }
  }
</script>

<div class="global-state-editor">
  {#if !editingState}
    <div class="no-selection">
      <p>Select a global state to edit</p>
    </div>
  {:else}
    <div class="editor-header">
      <h3>
        <span class="indicator">🌐</span>
        {selectedId}
      </h3>
      {#if hasChanges}
        <span class="unsaved">Unsaved changes</span>
      {/if}
    </div>

    {#if saveError}
      <div class="error">{saveError}</div>
    {/if}

    <form class="editor-form" onsubmit={(e) => { e.preventDefault(); handleSave(); }}>
      <div class="field">
        <label for="name">Name</label>
        <input
          id="name"
          type="text"
          value={editingState.name ?? ''}
          oninput={(e) => handleFieldChange('name', e.currentTarget.value || null)}
        />
      </div>

      <div class="field">
        <label for="type">Type</label>
        <input
          id="type"
          type="text"
          value={editingState.type ?? ''}
          oninput={(e) => handleFieldChange('type', e.currentTarget.value || null)}
          placeholder="e.g., system, normal, special"
        />
      </div>

      <div class="field-row">
        <div class="field">
          <label for="startup">Startup</label>
          <input
            id="startup"
            type="number"
            value={editingState.startup ?? ''}
            oninput={(e) => handleFieldChange('startup', e.currentTarget.valueAsNumber || null)}
          />
        </div>

        <div class="field">
          <label for="active">Active</label>
          <input
            id="active"
            type="number"
            value={editingState.active ?? ''}
            oninput={(e) => handleFieldChange('active', e.currentTarget.valueAsNumber || null)}
          />
        </div>

        <div class="field">
          <label for="recovery">Recovery</label>
          <input
            id="recovery"
            type="number"
            value={editingState.recovery ?? ''}
            oninput={(e) => handleFieldChange('recovery', e.currentTarget.valueAsNumber || null)}
          />
        </div>

        <div class="field">
          <label for="total">Total</label>
          <input
            id="total"
            type="number"
            value={editingState.total ?? ''}
            oninput={(e) => handleFieldChange('total', e.currentTarget.valueAsNumber || null)}
          />
        </div>
      </div>

      <div class="field">
        <label for="tags">Tags (comma-separated)</label>
        <input
          id="tags"
          type="text"
          value={editingState.tags?.join(', ') ?? ''}
          oninput={(e) => {
            const tags = e.currentTarget.value
              .split(',')
              .map(t => t.trim())
              .filter(t => t.length > 0);
            handleFieldChange('tags', tags.length > 0 ? tags : null);
          }}
          placeholder="e.g., invulnerable, reversal"
        />
      </div>

      <div class="field">
        <label for="animation">Animation</label>
        <input
          id="animation"
          type="text"
          value={editingState.animation ?? ''}
          oninput={(e) => handleFieldChange('animation', e.currentTarget.value || null)}
        />
      </div>

      <div class="actions">
        <button type="button" onclick={handleRevert} disabled={!hasChanges || saving}>
          Revert
        </button>
        <button type="submit" disabled={!hasChanges || saving}>
          {saving ? 'Saving...' : 'Save'}
        </button>
      </div>
    </form>
  {/if}
</div>

<style>
  .global-state-editor {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .no-selection {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-muted, #888);
  }

  .editor-header {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 1rem;
    border-bottom: 1px solid var(--border-color, #333);
  }

  .editor-header h3 {
    margin: 0;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .indicator {
    font-size: 0.9rem;
  }

  .unsaved {
    font-size: 0.8rem;
    color: var(--warning-color, #ff9944);
  }

  .error {
    padding: 0.75rem 1rem;
    background: rgba(255, 68, 68, 0.1);
    color: var(--danger-color, #ff4444);
    border-left: 3px solid var(--danger-color, #ff4444);
  }

  .editor-form {
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
    overflow-y: auto;
    flex: 1;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .field-row {
    display: flex;
    gap: 1rem;
  }

  .field-row .field {
    flex: 1;
  }

  label {
    font-size: 0.85rem;
    color: var(--text-muted, #888);
  }

  input {
    padding: 0.5rem;
    border: 1px solid var(--border-color, #333);
    border-radius: 4px;
    background: var(--bg-secondary, #222);
    color: inherit;
  }

  input:focus {
    outline: none;
    border-color: var(--accent-color, #4a9eff);
  }

  input[type="number"] {
    width: 100%;
  }

  .actions {
    display: flex;
    gap: 0.5rem;
    justify-content: flex-end;
    padding-top: 1rem;
    border-top: 1px solid var(--border-color, #333);
  }

  .actions button {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 4px;
    cursor: pointer;
  }

  .actions button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .actions button[type="submit"] {
    background: var(--accent-color, #4a9eff);
    color: white;
  }
</style>
```

**Step 2: Run type checking**

Run: `npm run check`
Expected: No errors

**Step 3: Commit**

```bash
git add src/lib/components/GlobalStateEditor.svelte
git commit -m "$(cat <<'EOF'
feat(ui): add GlobalStateEditor component

Form-based editor for global states with:
- Basic frame data fields
- Tag editing
- Unsaved changes tracking
- Save/revert actions

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 13: Create GlobalsManager View

**Files:**
- Create: `src/lib/views/GlobalsManager.svelte`

**Step 1: Create the view**

```svelte
<script lang="ts">
  import GlobalStateList from '$lib/components/GlobalStateList.svelte';
  import GlobalStateEditor from '$lib/components/GlobalStateEditor.svelte';
  import { loadGlobalStateList, resetGlobalsStore } from '$lib/stores/globals.svelte';
  import { onMount, onDestroy } from 'svelte';

  onMount(() => {
    loadGlobalStateList();
  });

  onDestroy(() => {
    resetGlobalsStore();
  });
</script>

<div class="globals-manager">
  <aside class="sidebar">
    <GlobalStateList />
  </aside>
  <main class="editor-panel">
    <GlobalStateEditor />
  </main>
</div>

<style>
  .globals-manager {
    display: flex;
    height: 100%;
    background: var(--bg-primary, #1a1a1a);
  }

  .sidebar {
    width: 280px;
    border-right: 1px solid var(--border-color, #333);
    overflow-y: auto;
  }

  .editor-panel {
    flex: 1;
    overflow: hidden;
  }
</style>
```

**Step 2: Run type checking**

Run: `npm run check`
Expected: No errors

**Step 3: Commit**

```bash
git add src/lib/views/GlobalsManager.svelte
git commit -m "$(cat <<'EOF'
feat(views): add GlobalsManager view

Compose GlobalStateList and GlobalStateEditor into
a full management view for project-level global states.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 14: Add Globals Tab to Main Layout

**Files:**
- Modify: `src/routes/+page.svelte` or main layout file

**Step 1: Find the main layout structure**

Explore the current tab/navigation structure in the app to understand where to add the Globals tab.

**Step 2: Add navigation for Globals Manager**

This step depends on the existing UI structure. The goal is to add a "Globals" tab/link that navigates to the GlobalsManager view. Example integration:

```svelte
<!-- In the navigation component -->
<nav>
  <!-- Existing tabs -->
  <a href="/characters">Characters</a>
  <a href="/globals">Globals</a>
</nav>

<!-- Or if using tabs -->
<button onclick={() => setView('globals')}>
  🌐 Globals
</button>
```

**Step 3: Create route if using file-based routing**

If using SvelteKit routes, create `src/routes/globals/+page.svelte`:

```svelte
<script lang="ts">
  import GlobalsManager from '$lib/views/GlobalsManager.svelte';
</script>

<GlobalsManager />
```

**Step 4: Run and verify**

Run: `npm run dev`
Verify: Navigate to globals view, see empty state message.

**Step 5: Commit**

```bash
git add src/routes/globals/+page.svelte  # or relevant files
git commit -m "$(cat <<'EOF'
feat(ui): add Globals tab to main navigation

Users can now access the GlobalsManager view to browse
and edit project-wide global states.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 15: Update State List to Show Global-Derived States

**Files:**
- Modify: `src/lib/views/FrameDataTable.svelte` (or equivalent state list)

**Step 1: Update to show global indicator**

In the state list component, add visual indication for states that came from globals. This requires the backend to pass through origin information, or we detect by checking if the state's input matches a known global include.

For now, add a simple visual indicator column:

```svelte
<!-- In the table header -->
<th>Origin</th>

<!-- In the table row -->
<td>
  {#if state.type === 'system'}
    <span class="origin-badge global" title="From global state">🌐</span>
  {:else}
    <span class="origin-badge local">Local</span>
  {/if}
</td>
```

**Step 2: Run and verify**

Run: `npm run dev`
Verify: States show origin indicator

**Step 3: Commit**

```bash
git add src/lib/views/FrameDataTable.svelte
git commit -m "$(cat <<'EOF'
feat(ui): show global indicator in state list

Display 🌐 indicator for states originating from globals
(currently detected by type="system" field).

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 16: Add Character Globals Editor

**Files:**
- Create: `src/lib/components/CharacterGlobalsEditor.svelte`

**Step 1: Create the component for editing character's globals.json**

```svelte
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import type { GlobalInclude, GlobalsManifest, GlobalStateSummary } from '$lib/types';
  import { getCurrentCharacter } from '$lib/stores/character.svelte';
  import { getGlobalStateList, loadGlobalStateList } from '$lib/stores/globals.svelte';
  import { getProjectPath } from '$lib/stores/project.svelte';
  import { onMount } from 'svelte';

  let manifest = $state<GlobalsManifest>({ includes: [] });
  let availableGlobals = $state<GlobalStateSummary[]>([]);
  let loading = $state(false);
  let saving = $state(false);
  let error = $state<string | null>(null);

  const character = $derived(getCurrentCharacter());

  onMount(async () => {
    await loadGlobalStateList();
    availableGlobals = getGlobalStateList();
    if (character) {
      await loadManifest();
    }
  });

  async function loadManifest() {
    if (!character) return;

    const projectPath = getProjectPath();
    if (!projectPath) return;

    loading = true;
    error = null;

    try {
      const manifestPath = `${projectPath}/characters/${character.id}/globals.json`;
      const content = await invoke<string>('read_file', { path: manifestPath }).catch(() => null);

      if (content) {
        manifest = JSON.parse(content);
      } else {
        manifest = { includes: [] };
      }
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  async function saveManifest() {
    if (!character) return;

    const projectPath = getProjectPath();
    if (!projectPath) return;

    saving = true;
    error = null;

    try {
      const manifestPath = `${projectPath}/characters/${character.id}/globals.json`;
      await invoke('write_file', {
        path: manifestPath,
        content: JSON.stringify(manifest, null, 2),
      });
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      saving = false;
    }
  }

  function addInclude(globalId: string) {
    if (manifest.includes.some(i => i.state === globalId)) return;

    manifest.includes = [...manifest.includes, {
      state: globalId,
      as: globalId,
    }];
  }

  function removeInclude(index: number) {
    manifest.includes = manifest.includes.filter((_, i) => i !== index);
  }

  function updateAlias(index: number, alias: string) {
    manifest.includes[index].as = alias;
  }
</script>

<div class="character-globals-editor">
  <h4>Global State Includes</h4>

  {#if error}
    <div class="error">{error}</div>
  {/if}

  {#if loading}
    <p>Loading...</p>
  {:else}
    <div class="includes-list">
      {#each manifest.includes as include, index (index)}
        <div class="include-item">
          <span class="global-name">{include.state}</span>
          <label>
            as:
            <input
              type="text"
              value={include.as}
              oninput={(e) => updateAlias(index, e.currentTarget.value)}
            />
          </label>
          <button onclick={() => removeInclude(index)}>Remove</button>
        </div>
      {/each}
    </div>

    <div class="add-global">
      <select onchange={(e) => {
        if (e.currentTarget.value) {
          addInclude(e.currentTarget.value);
          e.currentTarget.value = '';
        }
      }}>
        <option value="">Add global state...</option>
        {#each availableGlobals as global (global.id)}
          {#if !manifest.includes.some(i => i.state === global.id)}
            <option value={global.id}>{global.id} - {global.name ?? global.id}</option>
          {/if}
        {/each}
      </select>
    </div>

    <div class="actions">
      <button onclick={saveManifest} disabled={saving}>
        {saving ? 'Saving...' : 'Save'}
      </button>
    </div>
  {/if}
</div>

<style>
  .character-globals-editor {
    padding: 1rem;
    border: 1px solid var(--border-color, #333);
    border-radius: 4px;
    margin: 1rem 0;
  }

  h4 {
    margin: 0 0 1rem;
  }

  .error {
    padding: 0.5rem;
    background: rgba(255, 68, 68, 0.1);
    color: var(--danger-color, #ff4444);
    margin-bottom: 1rem;
  }

  .includes-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    margin-bottom: 1rem;
  }

  .include-item {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0.5rem;
    background: var(--bg-secondary, #222);
    border-radius: 4px;
  }

  .global-name {
    font-family: monospace;
    font-weight: 500;
  }

  .include-item input {
    width: 150px;
    padding: 0.25rem 0.5rem;
  }

  .add-global select {
    width: 100%;
    padding: 0.5rem;
  }

  .actions {
    margin-top: 1rem;
    display: flex;
    justify-content: flex-end;
  }
</style>
```

**Step 2: Add to character overview or settings**

Integrate this component into the character editing UI.

**Step 3: Run and verify**

Run: `npm run dev`
Verify: Can add/remove global includes for a character

**Step 4: Commit**

```bash
git add src/lib/components/CharacterGlobalsEditor.svelte
git commit -m "$(cat <<'EOF'
feat(ui): add CharacterGlobalsEditor component

Allow characters to configure their global state includes:
- Add/remove global states
- Customize alias (input name)
- Save to globals.json

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 17: Documentation

**Files:**
- Create: `docs/global-states.md`

**Step 1: Write documentation**

```markdown
# Global States

Global states are shared state definitions that live at the project level and can be opted into by any character.

## Use Cases

- **System mechanics**: Burst, Roman Cancel, hard knockdown
- **Common states**: Idle, walk, crouch, jump
- **Character archetypes**: Shared specials for similar characters

## Project Structure

```
project/
  globals/
    states/
      burst.json
      idle.json
      walk_forward.json
  characters/
    ryu/
      globals.json      # Which globals this character uses
      states/
        5L.json         # Local states
```

## Character Opt-In

Characters opt into globals via `globals.json`:

```json
{
  "includes": [
    { "state": "burst", "as": "burst" },
    { "state": "idle", "as": "idle", "override": { "animation": "ryu_idle" } },
    { "state": "walk_forward", "as": "66" }
  ]
}
```

### Fields

- `state`: Name of the global state file (without `.json`)
- `as`: Input name for this character (allows renaming)
- `override`: Optional field overrides

## Override Behavior

Overrides use **shallow merge** semantics:

- Top-level fields are replaced entirely
- Arrays are replaced (not concatenated)
- Nested objects are replaced as units
- `null` removes a field

Example:
```json
// Global: { "damage": 100, "tags": ["normal", "ground"] }
// Override: { "damage": 80 }
// Result: { "damage": 80, "tags": ["normal", "ground"] }
```

## Validation

The system validates:

- Global state exists
- Alias doesn't conflict with local states
- No duplicate aliases
- Unknown override fields generate warnings

## MCP Tools

- `list_global_states`: List all project globals
- `get_global_state`: Get a specific global
- `create_global_state`: Create new global
- `update_global_state`: Update existing global
- `delete_global_state`: Delete (checks references)

## UI

- **Globals Manager**: Browse and edit project-wide globals
- **Character Globals Editor**: Configure includes per character
- **State List**: Shows 🌐 indicator for global-derived states
```

**Step 2: Commit**

```bash
git add docs/global-states.md
git commit -m "$(cat <<'EOF'
docs: add global states documentation

Document global states feature including:
- Project structure
- Character opt-in via globals.json
- Override behavior (shallow merge)
- Validation rules
- MCP tools
- UI components

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 18: Final Integration Testing

**Step 1: Run all backend tests**

Run: `cd src-tauri && cargo test`
Expected: All tests pass

**Step 2: Run clippy**

Run: `cd src-tauri && cargo clippy --all-targets`
Expected: No warnings

**Step 3: Run frontend checks**

Run: `npm run check`
Expected: No errors

**Step 4: Run frontend tests**

Run: `npm run test:run`
Expected: All tests pass

**Step 5: Manual E2E test**

1. Start the app: `npm run tauri dev`
2. Open or create a project
3. Create a global state via Globals Manager
4. Add global include to a character
5. Verify state appears in character's state list
6. Export character and verify global-derived state is included
7. Delete global and verify warning about references

**Step 6: Final commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
feat: global states implementation complete

Implements project-wide global states that characters can opt into:
- Rust: schema types, loading, shallow merge, validation
- Backend: Tauri commands, MCP tools
- Frontend: GlobalsManager, CharacterGlobalsEditor
- Docs: full documentation

Characters explicitly declare globals via globals.json with
optional per-character overrides using shallow merge semantics.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Summary

This plan implements global states in 18 tasks:

1. Schema types (Rust)
2. Loading functions (Rust)
3. Shallow merge (Rust)
4. Full resolution with validation (Rust)
5. Character loading integration (Rust)
6. Export pipeline verification (Rust)
7. MCP tools (Rust)
8. Tauri commands (Rust)
9. TypeScript types
10. Globals store
11. GlobalStateList component
12. GlobalStateEditor component
13. GlobalsManager view
14. Navigation integration
15. State list global indicator
16. CharacterGlobalsEditor component
17. Documentation
18. Final integration testing

Each task follows TDD with explicit file paths, code, and expected outcomes.
