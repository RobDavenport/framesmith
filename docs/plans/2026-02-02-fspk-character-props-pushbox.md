# FSPK Character Properties & Push Boxes - Complete Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add `SECTION_CHARACTER_PROPS` for dynamic key-value character properties and `SECTION_PUSH_WINDOWS` for body collision boxes to the FSPK binary format, with full integration into training mode and UI editors.

**Scope:** This plan covers the complete vertical slice:
- Data layer (schema, FSPK format, export)
- Runtime layer (collision helpers, WASM bindings)
- Integration layer (training mode, UI editors)

**Tech Stack:** Rust (no_std for framesmith-fspack, std for export), TypeScript/Svelte for UI

---

## Phase 1: Data Layer (Tasks 1-10)

### Task 1: Add Q24.8 Fixed-Point Helpers

**Files:** `src-tauri/src/codegen/zx_fspack_format.rs`

**Step 1:** Add Q24.8 conversion functions after existing Q12.4 helpers (~line 217):

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

**Step 2:** Add tests for Q24.8 in the `tests` module:

```rust
#[test]
fn test_q24_8_conversion() {
    assert_eq!(to_q24_8(10000.0), 2_560_000);
    assert_eq!(from_q24_8(2_560_000), 10000.0);
    assert_eq!(to_q24_8(4.5), 1152);
    assert_eq!(from_q24_8(1152), 4.5);
    assert_eq!(to_q24_8(-3.5), -896);
    assert_eq!(from_q24_8(-896), -3.5);
    assert_eq!(to_q24_8(0.0), 0);
    assert_eq!(from_q24_8(0), 0.0);
}
```

**Verify:** `cd src-tauri && cargo test zx_fspack_format::tests::test_q24_8`

---

### Task 2: Define SECTION_CHARACTER_PROPS Constants

**Files:** `src-tauri/src/codegen/zx_fspack_format.rs`, `crates/framesmith-fspack/src/view.rs`

**Step 1:** Add to format file after `SECTION_CANCEL_DENIES` (~line 104):

```rust
/// Array of CharacterProp12 structs (dynamic key-value properties)
pub const SECTION_CHARACTER_PROPS: u32 = 21;

/// Character property record size: name_off(4) + name_len(2) + type(1) + reserved(1) + value(4) = 12 bytes
pub const CHARACTER_PROP12_SIZE: usize = 12;

/// Property type: Q24.8 signed fixed-point number
pub const PROP_TYPE_Q24_8: u8 = 0;
/// Property type: boolean (value != 0)
pub const PROP_TYPE_BOOL: u8 = 1;
/// Property type: string reference (u16 offset + u16 len in value field)
pub const PROP_TYPE_STR: u8 = 2;
```

**Step 2:** Add to view.rs after `SECTION_CANCEL_DENIES` (~line 90):

```rust
/// Array of CharacterProp12 structs
pub const SECTION_CHARACTER_PROPS: u32 = 21;
```

---

### Task 3: Define SECTION_PUSH_WINDOWS Constants

**Files:** `src-tauri/src/codegen/zx_fspack_format.rs`, `crates/framesmith-fspack/src/view.rs`

**Step 1:** Add to format file after `SECTION_CHARACTER_PROPS`:

```rust
/// Array of PushWindow12 structs (body collision boxes, same format as HurtWindow12)
pub const SECTION_PUSH_WINDOWS: u32 = 22;

/// Push window record size (same as hurt window): start(1) + end(1) + pad(2) + shapes_off(4) + shapes_len(2) + pad(2) = 12 bytes
pub const PUSH_WINDOW12_SIZE: usize = 12;
```

**Step 2:** Add to view.rs:

```rust
/// Array of PushWindow12 structs (body collision)
pub const SECTION_PUSH_WINDOWS: u32 = 22;
```

---

### Task 4: Update JSON Schema - Character Properties

**Files:** `src-tauri/src/schema/mod.rs`

**Step 1:** Add PropertyValue enum before Character struct:

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

**Step 2:** Replace fixed fields in Character struct with properties map:

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

**Step 3:** Remove old fixed fields: `archetype`, `health`, `walk_speed`, `back_walk_speed`, `jump_height`, `jump_duration`, `dash_distance`, `dash_duration`

**Verify:** `cd src-tauri && cargo check`

---

### Task 5: Update JSON Schema - Push Boxes

**Files:** `src-tauri/src/schema/mod.rs`

Add pushboxes field to State struct after `advanced_hurtboxes`:

```rust
/// Push boxes for body collision (same format as hurtboxes)
#[serde(default)]
pub pushboxes: Vec<FrameHitbox>,
```

**Verify:** `cd src-tauri && cargo check`

---

### Task 6: Add CharacterPropsView to framesmith-fspack

**Files:** `crates/framesmith-fspack/src/view.rs`

**Step 1:** Add CharacterPropView struct:

```rust
/// View into a single character property record.
#[derive(Clone, Copy)]
pub struct CharacterPropView<'a> {
    data: &'a [u8],
}

impl<'a> CharacterPropView<'a> {
    pub fn name(&self) -> (u32, u16) {
        let off = read_u32_le(self.data, 0);
        let len = read_u16_le(self.data, 4);
        (off, len)
    }

    pub fn value_type(&self) -> u8 {
        read_u8(self.data, 6)
    }

    pub fn value_raw(&self) -> u32 {
        read_u32_le(self.data, 8)
    }

    pub fn as_q24_8(&self) -> i32 {
        read_i32_le(self.data, 8)
    }

    pub fn as_bool(&self) -> bool {
        read_u8(self.data, 8) != 0
    }

    pub fn as_str_ref(&self) -> (u16, u16) {
        let off = read_u16_le(self.data, 8);
        let len = read_u16_le(self.data, 10);
        (off, len)
    }
}
```

**Step 2:** Add CharacterPropsView struct and PackView method.

---

### Task 7: Add PushWindowsView to framesmith-fspack

**Files:** `crates/framesmith-fspack/src/view.rs`

Reuse HurtWindowView layout:

```rust
pub type PushWindowView<'a> = HurtWindowView<'a>;

#[derive(Clone, Copy)]
pub struct PushWindowsView<'a> {
    data: &'a [u8],
}

impl<'a> PushWindowsView<'a> {
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

Add `push_windows()` method to PackView.

---

### Task 8: Extend StateView for Push Windows

**Files:** `crates/framesmith-fspack/src/view.rs`, `src-tauri/src/codegen/zx_fspack_format.rs`

**Step 1:** Update STATE_RECORD_SIZE from 32 to 36 bytes in both files.

**Step 2:** Add push window accessors to StateView:

```rust
pub fn push_windows_off(&self) -> u16 {
    read_u16_le(self.data, 32)
}

pub fn push_windows_len(&self) -> u16 {
    read_u16_le(self.data, 34)
}
```

---

### Task 9: Export Character Properties

**Files:** `src-tauri/src/codegen/zx_fspack.rs`

Add `pack_character_props()` function and call it in `export_zx_fspack()`. Add SECTION_CHARACTER_PROPS to output sections.

---

### Task 10: Export Push Windows

**Files:** `src-tauri/src/codegen/zx_fspack.rs`

Update `pack_moves()` to also pack push windows following the hurt window pattern. Update pack_move_record to write push_off and push_len at bytes 32-35. Add SECTION_PUSH_WINDOWS to output sections.

---

## Phase 2: Runtime Layer (Tasks 11-13)

### Task 11: Add check_pushbox Runtime Helper

**Files:** `crates/framesmith-runtime/src/collision.rs`, `crates/framesmith-runtime/src/lib.rs`

Add `check_pushbox()` function that:
1. Gets push windows for both characters
2. Finds active push window for current frame
3. Checks AABB overlap
4. Returns separation vector (dx, dy) if overlapping

Export from lib.rs: `pub use collision::check_pushbox;`

---

### Task 12: Expose Pushbox Collision in WASM

**Files:** `crates/framesmith-runtime-wasm/src/lib.rs`

**Step 1:** Add to FrameResult struct:

```rust
pub push_separation: Option<(i32, i32)>,  // (dx, dy) if characters are overlapping
```

**Step 2:** Call check_pushbox in tick() after hit detection:

```rust
let push_sep = check_pushbox(
    &self.player_state, &player_pack, (player_pos_x, player_pos_y),
    &self.dummy_state, &dummy_pack, (dummy_pos_x, dummy_pos_y),
);
result.push_separation = push_sep;
```

**Step 3:** Ensure FrameResult is properly exposed via wasm-bindgen.

---

### Task 13: Add Property Accessors to WASM

**Files:** `crates/framesmith-runtime-wasm/src/lib.rs`

Add method to read character property by name:

```rust
#[wasm_bindgen]
impl TrainingSession {
    pub fn get_property(&self, name: &str) -> Option<f64> {
        let pack = PackView::parse(&self.player_fspk).ok()?;
        let props = pack.character_props()?;
        let strings = pack.strings()?;

        for i in 0..props.len() {
            let prop = props.get(i)?;
            let (off, len) = prop.name();
            let prop_name = strings.get(off, len)?;
            if prop_name == name {
                return Some(from_q24_8(prop.as_q24_8()));
            }
        }
        None
    }
}
```

---

## Phase 3: Training Mode Integration (Tasks 14-17)

### Task 14: Update TrainingMode Property Reading

**Files:** `src/lib/views/TrainingMode.svelte`

**Step 1:** Update health initialization (line ~257):

```typescript
// Before: maxHealth = currentCharacter.character.health;
maxHealth = currentCharacter.character.properties?.health ?? 1000;
```

**Step 2:** Update walk speed reading (lines ~542-544):

```typescript
// Before: const walkSpeed = char.walk_speed;
const walkSpeed = char.properties?.walk_speed ?? 4.5;
const backWalkSpeed = char.properties?.back_walk_speed ?? 3.2;
```

**Step 3:** Add type helper:

```typescript
function getCharProp(char: Character, key: string, fallback: number): number {
    const val = char.properties?.[key];
    return typeof val === 'number' ? val : fallback;
}
```

---

### Task 15: Add Pushbox Rendering to HitboxOverlay

**Files:** `src/lib/components/training/HitboxOverlay.svelte`

**Step 1:** Add pushbox colors:

```typescript
const PUSHBOX_COLOR = 'rgba(255, 255, 0, 0.4)';  // Yellow
const PUSHBOX_STROKE = '#FFFF00';
```

**Step 2:** Add pushbox drawing after hurtbox drawing:

```typescript
if (move.pushboxes) {
    for (const pb of move.pushboxes) {
        const [startFrame, endFrame] = pb.frames;
        if (currentFrame >= startFrame && currentFrame <= endFrame) {
            const box = pb.box;
            ctx.fillStyle = PUSHBOX_COLOR;
            ctx.strokeStyle = PUSHBOX_STROKE;
            ctx.fillRect(offsetX + box.x * scale, offsetY - (box.y + box.h) * scale, box.w * scale, box.h * scale);
            ctx.strokeRect(offsetX + box.x * scale, offsetY - (box.y + box.h) * scale, box.w * scale, box.h * scale);
        }
    }
}
```

---

### Task 16: Handle Push Separation in TrainingMode

**Files:** `src/lib/views/TrainingMode.svelte`

In the tick handler, after processing hits:

```typescript
if (result.push_separation) {
    const [dx, _dy] = result.push_separation;
    playerPosX += dx;
    dummyPosX -= dx;
}
```

---

### Task 17: Add Pushbox Toggle to Training Debug View

**Files:** `src/lib/views/TrainingMode.svelte`

Add pushbox toggle to debug overlay controls alongside existing hitbox/hurtbox toggles.

---

## Phase 4: UI Editor Integration (Tasks 18-21)

### Task 18: Update CharacterOverview for Dynamic Properties

**Files:** `src/lib/views/CharacterOverview.svelte`

Replace hardcoded property display (lines 113-144) with dynamic iteration:

```svelte
{#each Object.entries(character.properties ?? {}) as [key, value]}
    <div class="stat-row">
        <span class="stat-label">{formatPropertyName(key)}</span>
        <span class="stat-value">{formatPropertyValue(value)}</span>
    </div>
{/each}
```

Add helper functions for formatting property names and values.

---

### Task 19: Add Character Property Editor Component

**Files:** `src/lib/components/CharacterPropertyEditor.svelte` (NEW)

Create a component for editing the dynamic properties map with:
- Key-value editing for each property
- Type-aware inputs (number/boolean/string)
- Add/remove property buttons

---

### Task 20: Add Pushbox Section to StateEditor

**Files:** `src/lib/views/StateEditor.svelte`

Add pushboxes section after hurtboxes (~line 555):

```svelte
<CollapsibleSection title="Pushboxes" bind:expanded={pushboxesExpanded}>
    {#if state.pushboxes?.length}
        {#each state.pushboxes as pb, i}
            <!-- Frame range and box dimension inputs -->
        {/each}
    {:else}
        <p class="empty-hint">No pushboxes defined</p>
    {/if}
    <button on:click={addPushbox}>+ Add Pushbox</button>
</CollapsibleSection>
```

Add `addPushbox()` and `removePushbox()` helper functions.

---

### Task 21: Add Pushbox Layer to MoveAnimationPreview

**Files:** `src/lib/components/MoveAnimationPreview.svelte`

**Step 1:** Extend Layer type:

```typescript
type Layer = "hitboxes" | "hurtboxes" | "pushboxes";
```

**Step 2:** Add layer colors:

```typescript
const LAYER_COLORS = {
    hitboxes: { fill: 'rgba(255, 0, 0, 0.3)', stroke: '#FF0000' },
    hurtboxes: { fill: 'rgba(0, 255, 0, 0.3)', stroke: '#00FF00' },
    pushboxes: { fill: 'rgba(255, 255, 0, 0.3)', stroke: '#FFFF00' },
};
```

**Step 3:** Update layer selector UI and `getLayerArray()` to handle pushboxes.

---

## Phase 5: Test Data & Validation (Tasks 22-24)

### Task 22: Update Test Character Data

**Files:** `characters/test_char/character.json`, `characters/test_char/states/0_idle.json`

**character.json:**
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
  "resources": [...]
}
```

**0_idle.json:** Add pushboxes array:
```json
{
  "pushboxes": [
    { "frames": [0, 1], "box": { "x": -12, "y": -70, "w": 24, "h": 70 } }
  ]
}
```

---

### Task 23: Run Full Test Suite

```bash
cd src-tauri && cargo test
cd src-tauri && cargo clippy --all-targets
npm run check
npm run test:run
```

---

### Task 24: Manual Integration Test

1. Open Framesmith with test_char
2. Verify character properties display correctly in CharacterOverview
3. Edit a property value, save, verify it persists
4. Open StateEditor for 0_idle, verify pushbox section appears
5. Add/edit a pushbox, save, verify it persists
6. Open Training Mode, verify:
   - Health initializes from properties.health
   - Walk speed uses properties.walk_speed
   - Pushboxes render in overlay (yellow)
   - Push separation works when characters overlap

---

## Phase 6: Documentation (Task 25)

### Task 25: Update Documentation

**Files:** `docs/zx-fspack.md`, `docs/data-formats.md`

- Document SECTION_CHARACTER_PROPS (section 21) format
- Document SECTION_PUSH_WINDOWS (section 22) format
- Update character.json example with properties map
- Add pushboxes to state documentation

---

## Summary

| Phase | Tasks | Description |
|-------|-------|-------------|
| 1 | 1-10 | Data layer (schema, FSPK, export) |
| 2 | 11-13 | Runtime layer (collision, WASM) |
| 3 | 14-17 | Training mode integration |
| 4 | 18-21 | UI editor integration |
| 5 | 22-24 | Test data & validation |
| 6 | 25 | Documentation |

**Total Tasks:** 25

**Critical Dependencies:**
- Tasks 1-10 must complete before 11-13 (runtime needs format)
- Tasks 4-5 must complete before 14-21 (TypeScript needs schema)
- Tasks 11-13 must complete before 14-17 (training mode needs WASM)

---

## Verification Checklist

- [ ] `cargo test` passes
- [ ] `cargo clippy --all-targets` has no warnings
- [ ] `npm run check` passes
- [ ] `npm run test:run` passes
- [ ] Test character loads with new properties format
- [ ] Character properties editable in UI
- [ ] Pushboxes editable in StateEditor
- [ ] Pushboxes visible in MoveAnimationPreview
- [ ] Training mode reads properties correctly
- [ ] Training mode renders pushboxes
- [ ] Push collision separation works
- [ ] FSPK export includes both new sections
