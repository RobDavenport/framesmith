# FSPK Character Properties & Push Boxes Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add `SECTION_CHARACTER_PROPS` for dynamic key-value character properties and `SECTION_PUSH_WINDOWS` for body collision boxes to the FSPK binary format.

**Architecture:** Extend FSPK with two new sections. Character properties use Q24.8 fixed-point for all numeric values (one format for everything). Push boxes reuse the existing Shape12/Window12 pattern from hurt boxes. The runtime gains a `check_pushbox()` helper function. The JSON schema gains a `properties` map on Character and `pushboxes` array on State.

**Tech Stack:** Rust (no_std for framesmith-fspack, std for export), TypeScript/Svelte for UI

---

## Task 1: Add Q24.8 Fixed-Point Helpers

**Files:**
- Modify: `src-tauri/src/codegen/zx_fspack_format.rs`

**Step 1: Add Q24.8 conversion functions**

Add after the existing Q12.4 helpers (~line 217):

```rust
/// Convert a floating-point value to Q24.8 fixed point.
/// Range: ±8,388,607.996, Precision: 1/256 ≈ 0.0039
#[inline]
pub fn to_q24_8(value: f64) -> i32 {
    (value * 256.0).round() as i32
}

/// Convert Q24.8 fixed point back to floating-point.
#[inline]
pub fn from_q24_8(raw: i32) -> f64 {
    raw as f64 / 256.0
}
```

**Step 2: Add tests for Q24.8**

Add to the `tests` module:

```rust
#[test]
fn test_q24_8_conversion() {
    // Integer values
    assert_eq!(to_q24_8(10000.0), 2_560_000);
    assert_eq!(from_q24_8(2_560_000), 10000.0);

    // Fractional values
    assert_eq!(to_q24_8(4.5), 1152);
    assert_eq!(from_q24_8(1152), 4.5);

    // Negative values
    assert_eq!(to_q24_8(-3.5), -896);
    assert_eq!(from_q24_8(-896), -3.5);

    // Zero
    assert_eq!(to_q24_8(0.0), 0);
    assert_eq!(from_q24_8(0), 0.0);
}
```

**Step 3: Run tests**

```bash
cd src-tauri && cargo test zx_fspack_format::tests::test_q24_8 -- --nocapture
```

Expected: PASS

**Step 4: Commit**

```bash
git add src-tauri/src/codegen/zx_fspack_format.rs
git commit -m "feat(fspk): add Q24.8 fixed-point conversion helpers"
```

---

## Task 2: Define SECTION_CHARACTER_PROPS Constants

**Files:**
- Modify: `src-tauri/src/codegen/zx_fspack_format.rs`
- Modify: `crates/framesmith-fspack/src/view.rs`

**Step 1: Add section constant to format file**

Add after `SECTION_CANCEL_DENIES` (~line 104):

```rust
/// Array of CharacterProp12 structs (dynamic key-value properties)
pub const SECTION_CHARACTER_PROPS: u32 = 21;

// =============================================================================
// Character Property Constants
// =============================================================================

/// Character property record size: name(6) + type(1) + reserved(1) + value(4) = 12 bytes
pub const CHARACTER_PROP12_SIZE: usize = 12;

/// Property type: Q24.8 signed fixed-point number
pub const PROP_TYPE_Q24_8: u8 = 0;
/// Property type: boolean (value != 0)
pub const PROP_TYPE_BOOL: u8 = 1;
/// Property type: string reference (u16 offset + u16 len in value field)
pub const PROP_TYPE_STR: u8 = 2;
```

**Step 2: Add section constant to view.rs**

Add after `SECTION_CANCEL_DENIES` (~line 90):

```rust
/// Array of CharacterProp12 structs
pub const SECTION_CHARACTER_PROPS: u32 = 21;
```

**Step 3: Update MAX_SECTIONS if needed**

In `view.rs`, ensure `MAX_SECTIONS` is at least 24 (already is).

**Step 4: Commit**

```bash
git add src-tauri/src/codegen/zx_fspack_format.rs crates/framesmith-fspack/src/view.rs
git commit -m "feat(fspk): add SECTION_CHARACTER_PROPS constants"
```

---

## Task 3: Define SECTION_PUSH_WINDOWS Constants

**Files:**
- Modify: `src-tauri/src/codegen/zx_fspack_format.rs`
- Modify: `crates/framesmith-fspack/src/view.rs`

**Step 1: Add section constant to format file**

Add after `SECTION_CHARACTER_PROPS`:

```rust
/// Array of PushWindow12 structs (body collision boxes, same format as HurtWindow12)
pub const SECTION_PUSH_WINDOWS: u32 = 22;

/// Push window record size (same as hurt window): start(1) + end(1) + pad(2) + shapes_off(4) + shapes_len(2) + pad(2) = 12 bytes
pub const PUSH_WINDOW12_SIZE: usize = 12;
```

**Step 2: Add section constant to view.rs**

```rust
/// Array of PushWindow12 structs (body collision)
pub const SECTION_PUSH_WINDOWS: u32 = 22;
```

**Step 3: Commit**

```bash
git add src-tauri/src/codegen/zx_fspack_format.rs crates/framesmith-fspack/src/view.rs
git commit -m "feat(fspk): add SECTION_PUSH_WINDOWS constants"
```

---

## Task 4: Update JSON Schema - Character Properties

**Files:**
- Modify: `src-tauri/src/schema/mod.rs`

**Step 1: Add CharacterProperty enum**

Add before the `Character` struct:

```rust
/// A character property value (dynamic key-value).
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum PropertyValue {
    Number(f64),
    Bool(bool),
    String(String),
}
```

**Step 2: Add properties field to Character**

Modify the `Character` struct to add a properties field:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Character {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub properties: std::collections::BTreeMap<String, PropertyValue>,
    #[serde(default)]
    pub resources: Vec<CharacterResource>,
}
```

**Step 3: Remove old fixed fields from Character**

Remove: `archetype`, `health`, `walk_speed`, `back_walk_speed`, `jump_height`, `jump_duration`, `dash_distance`, `dash_duration`

These become entries in `properties` instead.

**Step 4: Run cargo check**

```bash
cd src-tauri && cargo check
```

Fix any compilation errors from removed fields.

**Step 5: Commit**

```bash
git add src-tauri/src/schema/mod.rs
git commit -m "feat(schema): replace fixed character fields with dynamic properties map"
```

---

## Task 5: Update JSON Schema - Push Boxes

**Files:**
- Modify: `src-tauri/src/schema/mod.rs`

**Step 1: Add pushboxes field to State**

Add after `advanced_hurtboxes` in the `State` struct:

```rust
    /// Push boxes for body collision (same format as hurtboxes)
    #[serde(default)]
    pub pushboxes: Vec<FrameHitbox>,
```

**Step 2: Run cargo check**

```bash
cd src-tauri && cargo check
```

**Step 3: Commit**

```bash
git add src-tauri/src/schema/mod.rs
git commit -m "feat(schema): add pushboxes field to State"
```

---

## Task 6: Add CharacterPropsView to framesmith-fspack

**Files:**
- Modify: `crates/framesmith-fspack/src/view.rs`

**Step 1: Add CharacterPropView struct**

Add after existing view structs:

```rust
/// View into a single character property record.
#[derive(Clone, Copy)]
pub struct CharacterPropView<'a> {
    data: &'a [u8],
}

impl<'a> CharacterPropView<'a> {
    /// Name string reference (offset, length).
    pub fn name(&self) -> (u32, u16) {
        let off = read_u32_le(self.data, 0);
        let len = read_u16_le(self.data, 4);
        (off, len)
    }

    /// Property value type.
    pub fn value_type(&self) -> u8 {
        read_u8(self.data, 6)
    }

    /// Raw value bytes (4 bytes).
    pub fn value_raw(&self) -> u32 {
        read_u32_le(self.data, 8)
    }

    /// Value as Q24.8 number (if type is PROP_TYPE_Q24_8).
    pub fn as_q24_8(&self) -> i32 {
        read_i32_le(self.data, 8)
    }

    /// Value as bool (if type is PROP_TYPE_BOOL).
    pub fn as_bool(&self) -> bool {
        read_u8(self.data, 8) != 0
    }

    /// Value as string reference (if type is PROP_TYPE_STR).
    pub fn as_str_ref(&self) -> (u16, u16) {
        let off = read_u16_le(self.data, 8);
        let len = read_u16_le(self.data, 10);
        (off, len)
    }
}
```

**Step 2: Add CharacterPropsView struct**

```rust
/// View into the character properties section.
#[derive(Clone, Copy)]
pub struct CharacterPropsView<'a> {
    data: &'a [u8],
}

const CHARACTER_PROP_SIZE: usize = 12;

impl<'a> CharacterPropsView<'a> {
    /// Number of properties.
    pub fn len(&self) -> usize {
        self.data.len() / CHARACTER_PROP_SIZE
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Get property by index.
    pub fn get(&self, index: usize) -> Option<CharacterPropView<'a>> {
        let start = index * CHARACTER_PROP_SIZE;
        let end = start + CHARACTER_PROP_SIZE;
        if end <= self.data.len() {
            Some(CharacterPropView { data: &self.data[start..end] })
        } else {
            None
        }
    }
}
```

**Step 3: Add character_props() method to PackView**

Add to `impl PackView`:

```rust
    /// Get the character properties section.
    pub fn character_props(&self) -> Option<CharacterPropsView<'_>> {
        self.section_data(SECTION_CHARACTER_PROPS)
            .map(|data| CharacterPropsView { data })
    }
```

**Step 4: Commit**

```bash
git add crates/framesmith-fspack/src/view.rs
git commit -m "feat(fspack): add CharacterPropsView for reading character properties"
```

---

## Task 7: Add PushWindowsView to framesmith-fspack

**Files:**
- Modify: `crates/framesmith-fspack/src/view.rs`

**Step 1: Add PushWindowView struct**

Reuse the same layout as HurtWindowView:

```rust
/// View into a single push window record (same layout as HurtWindow).
pub type PushWindowView<'a> = HurtWindowView<'a>;

/// View into the push windows section.
#[derive(Clone, Copy)]
pub struct PushWindowsView<'a> {
    data: &'a [u8],
}

impl<'a> PushWindowsView<'a> {
    /// Get push window at absolute offset + index.
    pub fn get_at(&self, offset: u32, index: usize) -> Option<PushWindowView<'a>> {
        let start = offset as usize + index * HURT_WINDOW_SIZE;
        let end = start + HURT_WINDOW_SIZE;
        if end <= self.data.len() {
            Some(HurtWindowView { data: &self.data[start..end] })
        } else {
            None
        }
    }
}
```

**Step 2: Add push_windows() method to PackView**

```rust
    /// Get the push windows section.
    pub fn push_windows(&self) -> Option<PushWindowsView<'_>> {
        self.section_data(SECTION_PUSH_WINDOWS)
            .map(|data| PushWindowsView { data })
    }
```

**Step 3: Commit**

```bash
git add crates/framesmith-fspack/src/view.rs
git commit -m "feat(fspack): add PushWindowsView for reading push boxes"
```

---

## Task 8: Extend StateView for Push Windows

**Files:**
- Modify: `crates/framesmith-fspack/src/view.rs`
- Modify: `src-tauri/src/codegen/zx_fspack_format.rs`

**Step 1: Update STATE_RECORD_SIZE**

In `zx_fspack_format.rs`, update the constant:

```rust
/// State record size: existing 32 bytes + push_off(2) + push_len(2) = 36 bytes
pub const STATE_RECORD_SIZE: usize = 36;
```

In `view.rs`, update similarly:

```rust
pub const STATE_RECORD_SIZE: usize = 36;
```

**Step 2: Add push window accessors to StateView**

In `view.rs`, add to `impl StateView`:

```rust
    /// Push windows offset into PUSH_WINDOWS section.
    pub fn push_windows_off(&self) -> u16 {
        read_u16_le(self.data, 32)
    }

    /// Number of push windows.
    pub fn push_windows_len(&self) -> u16 {
        read_u16_le(self.data, 34)
    }
```

**Step 3: Commit**

```bash
git add crates/framesmith-fspack/src/view.rs src-tauri/src/codegen/zx_fspack_format.rs
git commit -m "feat(fspk): extend state record with push window references"
```

---

## Task 9: Export Character Properties

**Files:**
- Modify: `src-tauri/src/codegen/zx_fspack.rs`

**Step 1: Add property packing function**

```rust
use super::zx_fspack_format::{
    to_q24_8, CHARACTER_PROP12_SIZE, PROP_TYPE_Q24_8, PROP_TYPE_BOOL, PROP_TYPE_STR,
    SECTION_CHARACTER_PROPS,
    // ... existing imports
};

fn pack_character_props(
    properties: &std::collections::BTreeMap<String, crate::schema::PropertyValue>,
    strings: &mut StringTable,
) -> Result<Vec<u8>, String> {
    let mut data = Vec::with_capacity(properties.len() * CHARACTER_PROP12_SIZE);

    for (name, value) in properties {
        let name_ref = strings.intern(name)?;

        // Write name strref (6 bytes)
        write_u32_le(&mut data, name_ref.0);
        write_u16_le(&mut data, name_ref.1);

        match value {
            crate::schema::PropertyValue::Number(n) => {
                data.push(PROP_TYPE_Q24_8);
                data.push(0); // reserved
                let q = to_q24_8(*n);
                data.extend_from_slice(&q.to_le_bytes());
            }
            crate::schema::PropertyValue::Bool(b) => {
                data.push(PROP_TYPE_BOOL);
                data.push(0); // reserved
                data.extend_from_slice(&(if *b { 1u32 } else { 0u32 }).to_le_bytes());
            }
            crate::schema::PropertyValue::String(s) => {
                let str_ref = strings.intern(s)?;
                data.push(PROP_TYPE_STR);
                data.push(0); // reserved
                write_u16_le(&mut data, str_ref.0 as u16);
                write_u16_le(&mut data, str_ref.1);
            }
        }
    }

    Ok(data)
}
```

**Step 2: Call pack_character_props in export_zx_fspack**

In the `export_zx_fspack` function, add after resource defs packing:

```rust
    // Pack character properties
    let char_props_data = pack_character_props(&char_data.character.properties, &mut strings)?;
```

**Step 3: Add section to output**

Add the section data struct:

```rust
    if !char_props_data.is_empty() {
        sections.push(SectionData {
            kind: SECTION_CHARACTER_PROPS,
            align: 4,
            bytes: char_props_data,
        });
    }
```

**Step 4: Commit**

```bash
git add src-tauri/src/codegen/zx_fspack.rs
git commit -m "feat(export): pack character properties to FSPK"
```

---

## Task 10: Export Push Windows

**Files:**
- Modify: `src-tauri/src/codegen/zx_fspack.rs`

**Step 1: Add push window packing to pack_moves**

Modify the `pack_moves` function to also pack push windows. Follow the same pattern as hurt windows:

```rust
// In PackedMoveData struct, add:
pub push_windows: Vec<u8>,

// In pack_moves, add push window processing similar to hurt windows:
let mut push_windows_data: Vec<u8> = Vec::new();

for mv in moves {
    let push_off = (push_windows_data.len() / PUSH_WINDOW12_SIZE) as u16;
    let push_len = mv.pushboxes.len() as u16;

    for pb in &mv.pushboxes {
        // Pack same as hurt window
        push_windows_data.push(pb.frames.0);
        push_windows_data.push(pb.frames.1);
        push_windows_data.extend_from_slice(&[0u8; 2]); // padding

        let shape_off = checked_u32(shapes_data.len() / SHAPE12_SIZE, "push shape offset")?;
        let shape = pack_shape(&pb.box_);
        shapes_data.extend_from_slice(&shape);

        write_u32_le(&mut push_windows_data, shape_off);
        write_u16_le(&mut push_windows_data, 1u16); // 1 shape per window
        push_windows_data.extend_from_slice(&[0u8; 2]); // padding
    }

    // Store push_off and push_len for state record
}
```

**Step 2: Update pack_move_record for push windows**

Add `push_windows_off` and `push_windows_len` parameters and write them at bytes 32-35.

**Step 3: Add SECTION_PUSH_WINDOWS to export**

```rust
    if !packed.push_windows.is_empty() {
        sections.push(SectionData {
            kind: SECTION_PUSH_WINDOWS,
            align: 4,
            bytes: packed.push_windows,
        });
    }
```

**Step 4: Commit**

```bash
git add src-tauri/src/codegen/zx_fspack.rs
git commit -m "feat(export): pack push windows to FSPK"
```

---

## Task 11: Add check_pushbox Runtime Helper

**Files:**
- Modify: `crates/framesmith-runtime/src/collision.rs`
- Modify: `crates/framesmith-runtime/src/lib.rs`

**Step 1: Add check_pushbox function**

```rust
/// Check push box collision between two characters.
/// Returns separation vector (dx, dy) if overlapping, None if not.
#[must_use]
pub fn check_pushbox(
    state_a: &CharacterState,
    pack_a: &PackView,
    pos_a: (i32, i32),
    state_b: &CharacterState,
    pack_b: &PackView,
    pos_b: (i32, i32),
) -> Option<(i32, i32)> {
    let frame_a = state_a.frame;
    let frame_b = state_b.frame;

    // Get push windows for both characters
    let moves_a = pack_a.states()?;
    let move_a = moves_a.get(state_a.current_state as usize)?;
    let moves_b = pack_b.states()?;
    let move_b = moves_b.get(state_b.current_state as usize)?;

    let push_windows_a = pack_a.push_windows()?;
    let push_windows_b = pack_b.push_windows()?;
    let shapes = pack_a.shapes()?; // Assume shared shapes section

    // Find active push window for A
    for i in 0..move_a.push_windows_len() as usize {
        let pw_a = push_windows_a.get_at(move_a.push_windows_off() as u32, i)?;
        if frame_a < pw_a.start_frame() || frame_a > pw_a.end_frame() {
            continue;
        }

        // Find active push window for B
        for j in 0..move_b.push_windows_len() as usize {
            let pw_b = push_windows_b.get_at(move_b.push_windows_off() as u32, j)?;
            if frame_b < pw_b.start_frame() || frame_b > pw_b.end_frame() {
                continue;
            }

            // Check overlap and compute separation
            // For simplicity, check first shape of each
            if let (Some(shape_a), Some(shape_b)) = (
                shapes.get_at(pw_a.shapes_off(), 0),
                shapes.get_at(pw_b.shapes_off(), 0),
            ) {
                let aabb_a = Aabb::from_shape(&shape_a, pos_a.0, pos_a.1);
                let aabb_b = Aabb::from_shape(&shape_b, pos_b.0, pos_b.1);

                if aabb_overlap(&aabb_a, &aabb_b) {
                    // Compute separation (push apart horizontally)
                    let overlap_left = (aabb_a.x + aabb_a.w as i32) - aabb_b.x;
                    let overlap_right = (aabb_b.x + aabb_b.w as i32) - aabb_a.x;

                    let dx = if overlap_left < overlap_right {
                        -overlap_left
                    } else {
                        overlap_right
                    };

                    return Some((dx / 2, 0));
                }
            }
        }
    }

    None
}
```

**Step 2: Export from lib.rs**

```rust
pub use collision::check_pushbox;
```

**Step 3: Commit**

```bash
git add crates/framesmith-runtime/src/collision.rs crates/framesmith-runtime/src/lib.rs
git commit -m "feat(runtime): add check_pushbox collision helper"
```

---

## Task 12: Update Test Character Data

**Files:**
- Modify: `characters/test_char/character.json`

**Step 1: Migrate to properties format**

```json
{
  "id": "test_char",
  "name": "TEST_CHAR",
  "properties": {
    "archetype": "all-rounder",
    "health": 1000,
    "walk_speed": 4.5,
    "back_walk_speed": 3.2,
    "jump_height": 120,
    "jump_duration": 45,
    "dash_distance": 80,
    "dash_duration": 18
  },
  "resources": [
    { "name": "heat", "start": 0, "max": 100 },
    { "name": "ammo", "start": 6, "max": 6 },
    { "name": "level", "start": 0, "max": 3 },
    { "name": "install_active", "start": 0, "max": 1 }
  ]
}
```

**Step 2: Add pushboxes to a test state**

Modify `characters/test_char/states/0_idle.json`:

```json
{
  "input": "0_idle",
  "name": "Standing Idle",
  "pushboxes": [
    { "frames": [0, 1], "box": { "x": -12, "y": -70, "w": 24, "h": 70 } }
  ],
  ...existing fields...
}
```

**Step 3: Commit**

```bash
git add characters/test_char/character.json characters/test_char/states/0_idle.json
git commit -m "test: migrate test_char to new properties and pushbox format"
```

---

## Task 13: Update TypeScript Types

**Files:**
- Modify: `src/lib/types.ts`

**Step 1: Update Character type**

```typescript
export interface Character {
  id: string;
  name: string;
  properties: Record<string, number | boolean | string>;
  resources: CharacterResource[];
}
```

**Step 2: Add pushboxes to State type**

```typescript
export interface State {
  // ...existing fields...
  pushboxes?: FrameHitbox[];
}
```

**Step 3: Run type check**

```bash
npm run check
```

**Step 4: Commit**

```bash
git add src/lib/types.ts
git commit -m "feat(types): update TypeScript types for properties and pushboxes"
```

---

## Task 14: Run Full Test Suite

**Step 1: Run Rust tests**

```bash
cd src-tauri && cargo test
```

**Step 2: Run TypeScript tests**

```bash
npm run test:run
```

**Step 3: Run clippy**

```bash
cd src-tauri && cargo clippy --all-targets
```

**Step 4: Fix any failures and commit**

```bash
git add -A
git commit -m "fix: address test failures from schema changes"
```

---

## Task 15: Update Documentation

**Files:**
- Modify: `docs/zx-fspack.md`
- Modify: `docs/data-formats.md`

**Step 1: Add CHARACTER_PROPS section to zx-fspack.md**

Document the new section format, record layout, and value types.

**Step 2: Add PUSH_WINDOWS section to zx-fspack.md**

Document the new section and its relationship to shapes.

**Step 3: Update data-formats.md**

Update the character.json example and add pushboxes to state documentation.

**Step 4: Commit**

```bash
git add docs/zx-fspack.md docs/data-formats.md
git commit -m "docs: document character properties and push boxes"
```

---

## Summary

| Task | Description | Files |
|------|-------------|-------|
| 1 | Q24.8 fixed-point helpers | zx_fspack_format.rs |
| 2 | SECTION_CHARACTER_PROPS constants | format + view |
| 3 | SECTION_PUSH_WINDOWS constants | format + view |
| 4 | Character properties schema | schema/mod.rs |
| 5 | Push boxes schema | schema/mod.rs |
| 6 | CharacterPropsView reader | view.rs |
| 7 | PushWindowsView reader | view.rs |
| 8 | StateView push window accessors | view.rs + format |
| 9 | Export character properties | zx_fspack.rs |
| 10 | Export push windows | zx_fspack.rs |
| 11 | check_pushbox runtime helper | collision.rs |
| 12 | Update test character data | characters/ |
| 13 | Update TypeScript types | types.ts |
| 14 | Run full test suite | - |
| 15 | Update documentation | docs/ |
