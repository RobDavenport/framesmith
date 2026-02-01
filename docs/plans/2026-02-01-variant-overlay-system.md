# Variant/Overlay System Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add variant inheritance to reduce state duplication while flattening to pure data on export.

**Architecture:** Variants use tilde-separated filenames (`5H~level1.json`) with optional `base` field. Deep merge at load time, flatten before export. Single-level inheritance only.

**Tech Stack:** Rust (serde_json for merging), Tauri commands, existing schema types

---

## Task 1: Add Schema Fields for Variants

**Files:**
- Modify: `src-tauri/src/schema/mod.rs:212-249`

**Step 1: Write failing test for base field deserialization**

Add to the `#[cfg(test)] mod tests` block in `src-tauri/src/schema/mod.rs`:

```rust
#[test]
fn state_with_base_field_deserializes() {
    let json = r#"{
      "base": "5H",
      "damage": 80
    }"#;

    let state: State = serde_json::from_str(json).expect("state should parse");
    assert_eq!(state.base.as_deref(), Some("5H"));
    assert_eq!(state.damage, 80);
}

#[test]
fn state_with_id_field_deserializes() {
    let json = r#"{
      "id": "5H~level1",
      "input": "5H",
      "damage": 80
    }"#;

    let state: State = serde_json::from_str(json).expect("state should parse");
    assert_eq!(state.id.as_deref(), Some("5H~level1"));
}
```

**Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test state_with_base_field_deserializes`
Expected: FAIL with "unknown field `base`"

**Step 3: Add base and id fields to State struct**

In `src-tauri/src/schema/mod.rs`, add to the `State` struct (after line 248, before the closing brace):

```rust
    /// Base state this variant inherits from (authoring only, not exported).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base: Option<String>,
    /// Unique state ID (set during resolution, used in exports).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
```

Also update the `Default` impl to include:
```rust
            base: None,
            id: None,
```

**Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test state_with_base_field`
Expected: PASS

**Step 5: Commit**

```bash
git add src-tauri/src/schema/mod.rs
git commit -m "feat(schema): add base and id fields to State for variant support"
```

---

## Task 2: Create Variant Module with Parsing Functions

**Files:**
- Create: `src-tauri/src/variant/mod.rs`
- Modify: `src-tauri/src/lib.rs`

**Step 1: Write failing tests for variant name parsing**

Create `src-tauri/src/variant/mod.rs`:

```rust
//! Variant resolution for state inheritance.
//!
//! Variants allow states to inherit from base states with targeted overrides.
//! Filename convention: `{base}~{variant}.json` (e.g., `5H~level1.json`)

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_base_state_no_tilde() {
        let (base, variant) = parse_variant_name("5H");
        assert_eq!(base, "5H");
        assert_eq!(variant, None);
    }

    #[test]
    fn parse_simple_variant() {
        let (base, variant) = parse_variant_name("5H~level1");
        assert_eq!(base, "5H");
        assert_eq!(variant, Some("level1"));
    }

    #[test]
    fn parse_hold_notation_as_base() {
        // 5S~ is a base state (hold input), not a variant
        let (base, variant) = parse_variant_name("5S~");
        assert_eq!(base, "5S~");
        assert_eq!(variant, None);
    }

    #[test]
    fn parse_hold_variant() {
        // 5S~~installed is a variant of 5S~
        let (base, variant) = parse_variant_name("5S~~installed");
        assert_eq!(base, "5S~");
        assert_eq!(variant, Some("installed"));
    }

    #[test]
    fn parse_rekka_notation() {
        // 236K~K is NOT a variant - it's a rekka follow-up with input "K" and parent "236K"
        // The tilde here is part of the filename convention for rekkas
        // Our system treats this as base "236K" variant "K" but the file has no "base" field
        // so it won't be treated as inheriting
        let (base, variant) = parse_variant_name("236K~K");
        assert_eq!(base, "236K");
        assert_eq!(variant, Some("K"));
    }

    #[test]
    fn is_variant_checks_correctly() {
        assert!(!is_variant_filename("5H"));
        assert!(is_variant_filename("5H~level1"));
        assert!(!is_variant_filename("5S~")); // empty variant = base
        assert!(is_variant_filename("5S~~installed"));
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test variant`
Expected: FAIL with "cannot find function `parse_variant_name`"

**Step 3: Implement parsing functions**

Add above the tests in `src-tauri/src/variant/mod.rs`:

```rust
/// Parse a state name into (base, variant) components.
///
/// Splits on the **last** tilde. If the portion after the last tilde is empty,
/// treats the whole name as a base state (e.g., `5S~` is a hold input, not a variant).
///
/// # Examples
/// - `"5H"` → `("5H", None)`
/// - `"5H~level1"` → `("5H", Some("level1"))`
/// - `"5S~"` → `("5S~", None)` (empty variant = base state)
/// - `"5S~~installed"` → `("5S~", Some("installed"))`
pub fn parse_variant_name(name: &str) -> (&str, Option<&str>) {
    match name.rfind('~') {
        Some(pos) => {
            let variant_part = &name[pos + 1..];
            if variant_part.is_empty() {
                // Empty variant portion means this is a base state
                (name, None)
            } else {
                (&name[..pos], Some(variant_part))
            }
        }
        None => (name, None),
    }
}

/// Check if a filename represents a variant (has non-empty variant portion).
pub fn is_variant_filename(name: &str) -> bool {
    parse_variant_name(name).1.is_some()
}
```

**Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test variant`
Expected: PASS

**Step 5: Add module to lib.rs**

In `src-tauri/src/lib.rs`, add after line 4:

```rust
pub mod variant;
```

**Step 6: Commit**

```bash
git add src-tauri/src/variant/mod.rs src-tauri/src/lib.rs
git commit -m "feat(variant): add variant name parsing functions"
```

---

## Task 3: Implement Deep Merge for States

**Files:**
- Modify: `src-tauri/src/variant/mod.rs`

**Step 1: Write failing tests for deep merge**

Add to the tests module in `src-tauri/src/variant/mod.rs`:

```rust
use crate::schema::{GuardType, MeterGain, OnHit, Pushback, State};

#[test]
fn merge_scalars_override() {
    let base = State {
        input: "5H".to_string(),
        name: "Standing Heavy".to_string(),
        damage: 50,
        hitstun: 20,
        ..Default::default()
    };
    let overlay = State {
        damage: 80,
        ..Default::default()
    };

    let resolved = resolve_variant(&base, &overlay, "5H~level1");

    assert_eq!(resolved.id.as_deref(), Some("5H~level1"));
    assert_eq!(resolved.input, "5H"); // inherited
    assert_eq!(resolved.name, "Standing Heavy"); // inherited
    assert_eq!(resolved.damage, 80); // overridden
    assert_eq!(resolved.hitstun, 20); // inherited
}

#[test]
fn merge_objects_deep() {
    let base = State {
        on_hit: Some(OnHit {
            gain_meter: Some(10),
            ground_bounce: Some(false),
            ..Default::default()
        }),
        ..Default::default()
    };
    let overlay = State {
        on_hit: Some(OnHit {
            ground_bounce: Some(true),
            wall_bounce: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    };

    let resolved = resolve_variant(&base, &overlay, "5H~level1");

    let on_hit = resolved.on_hit.unwrap();
    assert_eq!(on_hit.gain_meter, Some(10)); // inherited
    assert_eq!(on_hit.ground_bounce, Some(true)); // overridden
    assert_eq!(on_hit.wall_bounce, Some(true)); // added
}

#[test]
fn merge_arrays_replace() {
    use crate::schema::{FrameHitbox, Rect};

    let base = State {
        hitboxes: vec![FrameHitbox {
            frames: (8, 12),
            r#box: Rect { x: 0, y: -50, w: 40, h: 20 },
        }],
        ..Default::default()
    };
    let overlay = State {
        hitboxes: vec![FrameHitbox {
            frames: (8, 14),
            r#box: Rect { x: 0, y: -55, w: 50, h: 25 },
        }],
        ..Default::default()
    };

    let resolved = resolve_variant(&base, &overlay, "5H~level1");

    assert_eq!(resolved.hitboxes.len(), 1);
    assert_eq!(resolved.hitboxes[0].frames, (8, 14)); // replaced, not merged
}

#[test]
fn merge_inherits_input_from_base() {
    let base = State {
        input: "5H".to_string(),
        ..Default::default()
    };
    let overlay = State {
        // no input specified
        damage: 80,
        ..Default::default()
    };

    let resolved = resolve_variant(&base, &overlay, "5H~level1");

    assert_eq!(resolved.input, "5H");
}
```

**Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test merge_scalars`
Expected: FAIL with "cannot find function `resolve_variant`"

**Step 3: Implement resolve_variant using JSON merge**

Add to `src-tauri/src/variant/mod.rs` (above the tests):

```rust
use crate::schema::State;

/// Deep merge two JSON values.
///
/// - Scalars: overlay replaces base
/// - Objects: recursively merge (overlay fields override base fields)
/// - Arrays: overlay replaces base entirely
/// - Explicit null in overlay: clears the field
fn deep_merge(base: serde_json::Value, overlay: serde_json::Value) -> serde_json::Value {
    use serde_json::Value;

    match (base, overlay) {
        // Both objects: recursively merge
        (Value::Object(mut base_map), Value::Object(overlay_map)) => {
            for (key, overlay_val) in overlay_map {
                if overlay_val.is_null() {
                    // Explicit null clears the field
                    base_map.remove(&key);
                } else if let Some(base_val) = base_map.remove(&key) {
                    base_map.insert(key, deep_merge(base_val, overlay_val));
                } else {
                    base_map.insert(key, overlay_val);
                }
            }
            Value::Object(base_map)
        }
        // Overlay is not an object, or base is not an object: overlay wins
        (_, overlay) => overlay,
    }
}

/// Resolve a variant by merging overlay onto base state.
///
/// - Sets `id` to the resolved state ID
/// - Inherits `input` from base if not specified in overlay
/// - Deep merges objects, replaces arrays
/// - Clears the `base` field in the result (not needed after resolution)
pub fn resolve_variant(base: &State, overlay: &State, resolved_id: &str) -> State {
    // Convert both to JSON values
    let base_json = serde_json::to_value(base).expect("base should serialize");
    let overlay_json = serde_json::to_value(overlay).expect("overlay should serialize");

    // Deep merge
    let mut merged = deep_merge(base_json, overlay_json);

    // Set the resolved ID
    if let serde_json::Value::Object(ref mut map) = merged {
        map.insert("id".to_string(), serde_json::Value::String(resolved_id.to_string()));
        // Clear the base field - it's authoring metadata, not runtime data
        map.remove("base");
        // If input is empty/default, inherit from base
        if map.get("input").map(|v| v.as_str() == Some("")).unwrap_or(true) {
            map.insert("input".to_string(), serde_json::Value::String(base.input.clone()));
        }
    }

    // Deserialize back to State
    serde_json::from_value(merged).expect("merged state should deserialize")
}
```

**Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test merge_`
Expected: PASS (all merge tests)

**Step 5: Commit**

```bash
git add src-tauri/src/variant/mod.rs
git commit -m "feat(variant): implement deep merge for state resolution"
```

---

## Task 4: Implement Variant Validation

**Files:**
- Modify: `src-tauri/src/variant/mod.rs`

**Step 1: Write failing tests for validation**

Add to the tests module in `src-tauri/src/variant/mod.rs`:

```rust
#[test]
fn validate_base_exists() {
    let states = vec![
        ("5H~level1".to_string(), State { base: Some("5H".to_string()), ..Default::default() }),
    ];
    let base_names: std::collections::HashSet<_> = std::iter::empty().collect();

    let errors = validate_variants(&states, &base_names);

    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("Base state '5H' not found"));
}

#[test]
fn validate_base_field_matches_filename() {
    let states = vec![
        ("5H~level1".to_string(), State { base: Some("2H".to_string()), ..Default::default() }),
    ];
    let base_names: std::collections::HashSet<_> = ["5H".to_string(), "2H".to_string()].into_iter().collect();

    let errors = validate_variants(&states, &base_names);

    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("doesn't match filename"));
}

#[test]
fn validate_no_chained_inheritance() {
    let states = vec![
        ("5H~level1".to_string(), State { base: Some("5H".to_string()), ..Default::default() }),
        ("5H~level1~enhanced".to_string(), State { base: Some("5H~level1".to_string()), ..Default::default() }),
    ];
    let base_names: std::collections::HashSet<_> = ["5H".to_string()].into_iter().collect();
    let variant_names: std::collections::HashSet<_> = ["5H~level1".to_string()].into_iter().collect();

    let errors = validate_variants_no_chain(&states, &base_names, &variant_names);

    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("cannot inherit from another variant"));
}

#[test]
fn validate_passes_for_valid_variant() {
    let states = vec![
        ("5H~level1".to_string(), State { base: Some("5H".to_string()), ..Default::default() }),
    ];
    let base_names: std::collections::HashSet<_> = ["5H".to_string()].into_iter().collect();

    let errors = validate_variants(&states, &base_names);

    assert!(errors.is_empty());
}
```

**Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test validate_base_exists`
Expected: FAIL with "cannot find function `validate_variants`"

**Step 3: Implement validation functions**

Add to `src-tauri/src/variant/mod.rs`:

```rust
use std::collections::HashSet;

/// Validate variant states have existing bases and matching base fields.
pub fn validate_variants(
    states: &[(String, State)],
    base_names: &HashSet<String>,
) -> Vec<String> {
    let mut errors = Vec::new();

    for (name, state) in states {
        if let Some(ref declared_base) = state.base {
            let (implied_base, variant_part) = parse_variant_name(name);

            // Check base exists
            if !base_names.contains(declared_base) {
                errors.push(format!(
                    "Variant '{}': Base state '{}' not found",
                    name, declared_base
                ));
            }

            // Check declared base matches filename-implied base
            if variant_part.is_some() && declared_base != implied_base {
                errors.push(format!(
                    "Variant '{}': Base field '{}' doesn't match filename implied base '{}'",
                    name, declared_base, implied_base
                ));
            }
        }
    }

    errors
}

/// Validate that variants don't inherit from other variants (single-level only).
pub fn validate_variants_no_chain(
    states: &[(String, State)],
    base_names: &HashSet<String>,
    variant_names: &HashSet<String>,
) -> Vec<String> {
    let mut errors = validate_variants(states, base_names);

    for (name, state) in states {
        if let Some(ref declared_base) = state.base {
            if variant_names.contains(declared_base) {
                errors.push(format!(
                    "Variant '{}': Variants cannot inherit from another variant ('{}')",
                    name, declared_base
                ));
            }
        }
    }

    errors
}
```

**Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test validate_`
Expected: PASS

**Step 5: Commit**

```bash
git add src-tauri/src/variant/mod.rs
git commit -m "feat(variant): add validation for base existence and no chaining"
```

---

## Task 5: Implement Flatten Function

**Files:**
- Modify: `src-tauri/src/variant/mod.rs`

**Step 1: Write failing test for flatten**

Add to the tests module:

```rust
#[test]
fn flatten_resolves_all_variants() {
    let base = State {
        input: "5H".to_string(),
        name: "Standing Heavy".to_string(),
        damage: 50,
        ..Default::default()
    };
    let variant1 = State {
        base: Some("5H".to_string()),
        damage: 60,
        ..Default::default()
    };
    let variant2 = State {
        base: Some("5H".to_string()),
        damage: 75,
        ..Default::default()
    };

    let states = vec![
        ("5H".to_string(), base),
        ("5H~level1".to_string(), variant1),
        ("5H~level2".to_string(), variant2),
    ];

    let flattened = flatten_variants(states).unwrap();

    assert_eq!(flattened.len(), 3);

    // Base state gets id set
    assert_eq!(flattened[0].id.as_deref(), Some("5H"));
    assert_eq!(flattened[0].damage, 50);

    // Variants are resolved
    let v1 = flattened.iter().find(|s| s.id.as_deref() == Some("5H~level1")).unwrap();
    assert_eq!(v1.input, "5H");
    assert_eq!(v1.damage, 60);
    assert!(v1.base.is_none()); // base field cleared

    let v2 = flattened.iter().find(|s| s.id.as_deref() == Some("5H~level2")).unwrap();
    assert_eq!(v2.damage, 75);
}

#[test]
fn flatten_errors_on_missing_base() {
    let variant = State {
        base: Some("5H".to_string()),
        damage: 60,
        ..Default::default()
    };

    let states = vec![
        ("5H~level1".to_string(), variant),
    ];

    let result = flatten_variants(states);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Base state '5H' not found"));
}
```

**Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test flatten_resolves`
Expected: FAIL with "cannot find function `flatten_variants`"

**Step 3: Implement flatten_variants**

Add to `src-tauri/src/variant/mod.rs`:

```rust
use std::collections::HashMap;

/// Flatten all variants into fully resolved states.
///
/// - Base states get their `id` set to their name
/// - Variants are merged with their base and get `id` set to their full name
/// - Returns error if any variant references a non-existent base
pub fn flatten_variants(states: Vec<(String, State)>) -> Result<Vec<State>, String> {
    // Separate base states from variants
    let mut base_map: HashMap<String, State> = HashMap::new();
    let mut variants: Vec<(String, State)> = Vec::new();

    for (name, state) in states {
        if state.base.is_some() {
            variants.push((name, state));
        } else {
            base_map.insert(name, state);
        }
    }

    // Validate variants
    let base_names: HashSet<String> = base_map.keys().cloned().collect();
    let variant_names: HashSet<String> = variants.iter().map(|(n, _)| n.clone()).collect();
    let errors = validate_variants_no_chain(&variants, &base_names, &variant_names);
    if !errors.is_empty() {
        return Err(errors.join("; "));
    }

    // Resolve base states (just set id)
    let mut result: Vec<State> = Vec::new();
    for (name, mut state) in base_map.clone() {
        state.id = Some(name);
        result.push(state);
    }

    // Resolve variants
    for (name, overlay) in variants {
        let base_name = overlay.base.as_ref().unwrap();
        let base = base_map.get(base_name).ok_or_else(|| {
            format!("Base state '{}' not found for variant '{}'", base_name, name)
        })?;
        let resolved = resolve_variant(base, &overlay, &name);
        result.push(resolved);
    }

    Ok(result)
}
```

**Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test flatten_`
Expected: PASS

**Step 5: Commit**

```bash
git add src-tauri/src/variant/mod.rs
git commit -m "feat(variant): implement flatten_variants for export resolution"
```

---

## Task 6: Integrate Variants into Character Loading

**Files:**
- Modify: `src-tauri/src/commands.rs`

**Step 1: Write integration test**

Create a test in `src-tauri/src/variant/mod.rs` that simulates the load flow:

```rust
#[test]
fn integration_load_and_flatten() {
    // Simulate loading files
    let base_json = r#"{
        "input": "5H",
        "name": "Standing Heavy",
        "damage": 50,
        "hitstun": 20,
        "on_hit": { "gain_meter": 10 }
    }"#;

    let variant_json = r#"{
        "base": "5H",
        "damage": 80,
        "on_hit": { "ground_bounce": true }
    }"#;

    let base: State = serde_json::from_str(base_json).unwrap();
    let variant: State = serde_json::from_str(variant_json).unwrap();

    let states = vec![
        ("5H".to_string(), base),
        ("5H~level1".to_string(), variant),
    ];

    let flattened = flatten_variants(states).unwrap();

    assert_eq!(flattened.len(), 2);

    let resolved = flattened.iter().find(|s| s.id.as_deref() == Some("5H~level1")).unwrap();
    assert_eq!(resolved.input, "5H");
    assert_eq!(resolved.damage, 80);
    assert_eq!(resolved.hitstun, 20); // inherited
    let on_hit = resolved.on_hit.as_ref().unwrap();
    assert_eq!(on_hit.gain_meter, Some(10)); // inherited
    assert_eq!(on_hit.ground_bounce, Some(true)); // overridden
}
```

**Step 2: Run test to verify it passes**

Run: `cd src-tauri && cargo test integration_load`
Expected: PASS

**Step 3: Update load_character_files in commands.rs**

Modify `load_character_files` function to return states with their names. Find the function (around line 40-95) and update the state loading section:

```rust
fn load_character_files(
    characters_dir: &str,
    character_id: &str,
) -> Result<(PathBuf, Character, Vec<(String, crate::schema::State)>, CancelTable), String> {
    // ... existing validation code ...

    // Load all states with their names
    let states_dir = char_path.join("states");
    let mut moves = vec![];
    if states_dir.exists() {
        for entry in fs::read_dir(&states_dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let state_path = entry.path();
            if state_path.extension().map(|e| e == "json").unwrap_or(false) {
                let content = fs::read_to_string(&state_path).map_err(|e| {
                    format!(
                        "Failed to read state file {:?}: {}",
                        state_path.file_name(),
                        e
                    )
                })?;
                let mv: State = serde_json::from_str(&content)
                    .map_err(|e| format!("Invalid state file {:?}: {}", state_path.file_name(), e))?;

                // Extract state name from filename (without .json extension)
                let state_name = state_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .ok_or_else(|| format!("Invalid state filename: {:?}", state_path))?
                    .to_string();

                moves.push((state_name, mv));
            }
        }
    }

    // ... rest of function ...
    Ok((char_path, character, moves, cancel_table))
}
```

**Step 4: Update CharacterData and callers**

Update `CharacterData` struct and the `load_character` command to flatten variants:

```rust
#[derive(Debug, Clone, serde::Serialize)]
pub struct CharacterData {
    pub character: Character,
    pub moves: Vec<State>,  // Flattened, resolved states
    pub cancel_table: CancelTable,
}

#[tauri::command]
pub fn load_character(characters_dir: String, character_id: String) -> Result<CharacterData, String> {
    let (_, character, named_moves, cancel_table) = load_character_files(&characters_dir, &character_id)?;

    // Flatten variants
    let moves = crate::variant::flatten_variants(named_moves)?;

    Ok(CharacterData {
        character,
        moves,
        cancel_table,
    })
}
```

**Step 5: Run all tests to verify nothing broke**

Run: `cd src-tauri && cargo test`
Expected: All tests PASS

**Step 6: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat(commands): integrate variant flattening into character loading"
```

---

## Task 7: Add Test Character Variant Examples

**Files:**
- Modify: `characters/test_char/character.json`
- Create: `characters/test_char/states/5H~level1.json`
- Create: `characters/test_char/states/5H~level2.json`
- Create: `characters/test_char/states/5H~level3.json`
- Create: `characters/test_char/states/236P~installed.json`

**Step 1: Update character.json with new resources**

Edit `characters/test_char/character.json`:

```json
{
  "id": "test_char",
  "name": "TEST_CHAR",
  "archetype": "all-rounder",
  "health": 1000,
  "walk_speed": 4.5,
  "back_walk_speed": 3.2,
  "jump_height": 120,
  "jump_duration": 45,
  "dash_distance": 80,
  "dash_duration": 18,
  "resources": [
    { "name": "heat", "start": 0, "max": 100 },
    { "name": "ammo", "start": 6, "max": 6 },
    { "name": "level", "start": 0, "max": 3 },
    { "name": "install_active", "start": 0, "max": 1 }
  ]
}
```

**Step 2: Create level variants for 5H**

Create `characters/test_char/states/5H~level1.json`:

```json
{
  "base": "5H",
  "damage": 100,
  "hitstun": 26,
  "preconditions": [
    { "type": "resource", "name": "level", "min": 1 }
  ]
}
```

Create `characters/test_char/states/5H~level2.json`:

```json
{
  "base": "5H",
  "damage": 115,
  "hitstun": 28,
  "preconditions": [
    { "type": "resource", "name": "level", "min": 2 }
  ],
  "on_hit": {
    "wall_bounce": true,
    "ground_bounce": true
  }
}
```

Create `characters/test_char/states/5H~level3.json`:

```json
{
  "base": "5H",
  "damage": 130,
  "hitstun": 30,
  "startup": 12,
  "preconditions": [
    { "type": "resource", "name": "level", "min": 3 }
  ],
  "on_hit": {
    "wall_bounce": true,
    "ground_bounce": true,
    "gain_meter": 20
  },
  "hitboxes": [
    { "frames": [12, 17], "box": { "x": 10, "y": -60, "w": 60, "h": 30 } }
  ]
}
```

**Step 3: Create install variant for 236P**

Create `characters/test_char/states/236P~installed.json`:

```json
{
  "base": "236P",
  "name": "Enhanced Fireball",
  "startup": 8,
  "damage": 120,
  "preconditions": [
    { "type": "resource", "name": "install_active", "min": 1 }
  ],
  "on_use": {
    "spawn_entity": {
      "type": "projectile",
      "tag": "fireball",
      "data": "enhanced_fireball_projectile"
    }
  }
}
```

**Step 4: Verify by running the app**

Run: `cd framesmith && npm run tauri dev`
Load test_char and verify:
- 5H, 5H~level1, 5H~level2, 5H~level3 all appear
- 236P, 236P~installed both appear
- Variants show correct resolved damage values

**Step 5: Commit**

```bash
git add characters/test_char/
git commit -m "feat(test_char): add level and install variant examples"
```

---

## Task 8: Update Exporters to Use Flattened Data

**Files:**
- Modify: `src-tauri/src/codegen/json_blob.rs`
- Modify: `src-tauri/src/codegen/zx_fspack.rs`

**Step 1: Verify exporters receive flattened data**

The exporters already receive `CharacterData` which now contains flattened moves. No changes needed to the export logic itself.

Add a test to verify export includes variant IDs:

In `src-tauri/src/codegen/json_blob.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{CancelTable, Character, State};

    #[test]
    fn export_includes_state_ids() {
        let character = Character {
            id: "test".to_string(),
            name: "Test".to_string(),
            archetype: "rushdown".to_string(),
            health: 1000,
            walk_speed: 4.0,
            back_walk_speed: 3.0,
            jump_height: 100,
            jump_duration: 40,
            dash_distance: 80,
            dash_duration: 15,
            resources: vec![],
        };

        let moves = vec![
            State {
                id: Some("5H".to_string()),
                input: "5H".to_string(),
                damage: 50,
                ..Default::default()
            },
            State {
                id: Some("5H~level1".to_string()),
                input: "5H".to_string(),
                damage: 80,
                ..Default::default()
            },
        ];

        let data = CharacterData {
            character,
            moves,
            cancel_table: CancelTable::default(),
        };

        let json = export_json_blob(&data).unwrap();

        assert!(json.contains("\"id\":\"5H\""));
        assert!(json.contains("\"id\":\"5H~level1\""));
    }
}
```

**Step 2: Run test to verify**

Run: `cd src-tauri && cargo test export_includes`
Expected: PASS

**Step 3: Commit**

```bash
git add src-tauri/src/codegen/json_blob.rs
git commit -m "test(codegen): verify export includes state IDs"
```

---

## Task 9: Update Rules Registry for New Resources

**Files:**
- Modify: `framesmith.rules.json`

**Step 1: Add level and install_active to project rules**

Edit `framesmith.rules.json` to include the new resources:

```json
{
  "version": 1,
  "registry": {
    "resources": ["heat", "ammo", "level", "install_active"],
    "events": {
      "gain_heat": {
        "contexts": ["on_hit", "notify"],
        "args": {
          "amount": { "type": "i64" }
        }
      }
    }
  }
}
```

**Step 2: Run validation**

Run: `cd framesmith && npm run tauri dev`
Open test_char and verify no validation errors for the new resources.

**Step 3: Commit**

```bash
git add framesmith.rules.json
git commit -m "feat(rules): add level and install_active resources to registry"
```

---

## Task 10: Final Integration Test

**Step 1: Run all Rust tests**

Run: `cd src-tauri && cargo test`
Expected: All tests PASS

**Step 2: Run clippy**

Run: `cd src-tauri && cargo clippy --all-targets`
Expected: No warnings

**Step 3: Manual test in app**

1. Run: `npm run tauri dev`
2. Load test_char
3. Verify Frame Data Table shows all states including variants
4. Verify 5H~level3 shows damage=130, startup=12
5. Export to JSON blob
6. Verify exported JSON contains all variant states with correct IDs

**Step 4: Final commit**

```bash
git add -A
git commit -m "feat: complete variant/overlay system implementation

- Add base and id fields to State schema
- Implement variant name parsing (split on last tilde)
- Implement deep merge for state resolution
- Add validation for base existence and no chaining
- Integrate flattening into character loading
- Add test_char examples (level and install variants)
- Update rules registry for new resources"
```

---

## Verification Checklist

- [ ] `cargo test` passes in src-tauri
- [ ] `cargo clippy --all-targets` has no warnings
- [ ] App loads test_char without errors
- [ ] Variants appear in Frame Data Table
- [ ] Variants show correct resolved values
- [ ] JSON export includes all variants with IDs
- [ ] FSPK export includes all variants
