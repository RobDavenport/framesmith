# ZX FSPK Export Format

**Status:** Active
**Last reviewed:** 2026-01-30

## Overview

FSPK (Framesmith Pack) is a compact binary format for storing fighting game character data. It is designed specifically for the Nethercore ZX runtime, which operates in a `no_std` WebAssembly environment with strict memory constraints.

### Why FSPK?

1. **Compact**: Binary format with fixed-size records minimizes storage and bandwidth
2. **Zero-copy**: Data is parsed directly from the buffer without allocations
3. **no_std compatible**: Works in WASM environments without the Rust standard library
4. **Unaligned-safe**: All reads use byte-level access, no alignment requirements

### Components

- **Framesmith export adapter** (`zx-fspack`): Converts character JSON to FSPK binary
- **`framesmith-fspack` crate**: `no_std` Rust library for reading FSPK files at runtime

## Exporting from Framesmith

Use the `zx-fspack` adapter when exporting a character.

- In the app UI: Character Overview -> Export -> "ZX FSPK (Binary)"
- In the CLI: `cd src-tauri && cargo run --bin framesmith -- export --project .. --character test_char --out ../exports/test_char.fspk`
- Programmatically: call the `export_character` command with `adapter = "zx-fspack"`

```rust
// Tauri command signature (Rust side)
// export_character(characters_dir, character_id, adapter, output_path, pretty)
export_character(
    "<project>/characters".to_string(),
    "test_char".to_string(),
    "zx-fspack".to_string(),
    "exports/test_char.fspk".to_string(),
    false,
)?;
```

This produces a `.fspk` binary file containing:
- Character state data (frame counts, damage, hitstun, etc.)
- Hitbox and hurtbox geometry
- Asset key references (mesh and animation keys)

## Runtime Usage (ZX Games)

Games load FSPK files during initialization and resolve asset keys to runtime handles.

### Loading the Pack

```rust
use framesmith_fspack::{PackView, KEY_NONE};

// In ZX game init():

// 1. Get ROM data length and allocate buffer
let len = rom_data_len("char/test_char.fspk");
let mut buffer = alloc_buffer(len);

// 2. Load ROM data into buffer
rom_data("char/test_char.fspk", &mut buffer);

// 3. Parse the pack (zero-copy, borrows from buffer)
let pack = PackView::parse(&buffer).expect("invalid pack");
```

### Resolving Asset Handles

Asset keys are stored as string references into the pack's string table. At init time, resolve these to runtime handles:

```rust
// 4. Load mesh assets by iterating mesh keys
let mut mesh_handles: Vec<MeshHandle> = Vec::new();
if let Some(mesh_keys) = pack.mesh_keys() {
    for i in 0..mesh_keys.len() {
        if let Some((off, len)) = mesh_keys.get(i) {
            if let Some(key) = pack.string(off, len) {
                // mesh key format: "{character_id}.{animation}" (e.g. "test_char.stand_light")
                let handle = rom_mesh(key);
                mesh_handles.push(handle);
            }
        }
    }
}

// 5. Load keyframes (animations) by iterating keyframes keys
let mut keyframes_handles: Vec<KeyframesHandle> = Vec::new();
if let Some(kf_keys) = pack.keyframes_keys() {
    for i in 0..kf_keys.len() {
        if let Some((off, len)) = kf_keys.get(i) {
            if let Some(key) = pack.string(off, len) {
                // key is the animation name like "stand_light"
                let handle = rom_keyframes(key);
                keyframes_handles.push(handle);
            }
        }
    }
}
```

### Accessing State Data at Runtime

Once loaded, access state data using the state ID (index):

```rust
// 6. At runtime, look up state data by ID
if let Some(states) = pack.states() {
    let state_id = 0; // e.g., standing light punch
    if let Some(st) = states.get(state_id) {
        // Frame data
        let startup = st.startup();       // u8: startup frames
        let active = st.active();         // u8: active frames
        let recovery = st.recovery();     // u8: recovery frames
        let total = st.total();           // u16: total duration

        // Combat data
        let damage = st.damage();         // u16: damage value
        let hitstun = st.hitstun();       // u8: hitstun frames
        let blockstun = st.blockstun();   // u8: blockstun frames
        let hitstop = st.hitstop();       // u8: hitstop frames
        let guard = st.guard();           // u8: guard type (high/mid/low)

        // Asset references (indices into handle arrays)
        let mesh_idx = st.mesh_key();
        if mesh_idx != KEY_NONE {
            let mesh = mesh_handles[mesh_idx as usize];
            // Use mesh for rendering
        }

        let kf_idx = st.keyframes_key();
        if kf_idx != KEY_NONE {
            let keyframes = keyframes_handles[kf_idx as usize];
            // Use keyframes for animation playback
        }
    }
}
```

## Format Specification

### Container Header (16 bytes)

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 4 | magic | `"FSPK"` (bytes: 0x46, 0x53, 0x50, 0x4B) |
| 4 | 4 | flags | Reserved (currently `0`) |
| 8 | 4 | total_len | Total size of the pack in bytes |
| 12 | 4 | section_count | Number of sections following the header |

### Section Header (16 bytes each)

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 4 | kind | Section type identifier |
| 4 | 4 | offset | Byte offset of section data from start of file |
| 8 | 4 | len | Length of section data in bytes |
| 12 | 4 | align | Alignment requirement (typically 4) |

### Section Types

| Kind | ID | Description |
|------|-----|-------------|
| STRING_TABLE | 1 | Raw UTF-8 string data |
| MESH_KEYS | 2 | Array of StrRef pointing to mesh asset keys |
| KEYFRAMES_KEYS | 3 | Array of StrRef pointing to animation asset keys |
| STATES | 4 | Array of StateRecord structs |
| HIT_WINDOWS | 5 | Array of HitWindow24 structs (active hitbox frames) |
| HURT_WINDOWS | 6 | Array of HurtWindow12 structs (hurtbox frames) |
| SHAPES | 7 | Array of Shape12 structs (hitbox/hurtbox geometry) |
| CANCELS_U16 | 8 | Array of u16 state IDs for cancel targets |
| RESOURCE_DEFS | 9 | Array of ResourceDef12 structs (character resource pools) |
| STATE_EXTRAS | 10 | Array of StateExtras72 structs (parallel to STATES) |
| EVENT_EMITS | 11 | Array of EventEmit16 structs |
| EVENT_ARGS | 12 | Array of EventArg20 structs |
| STATE_NOTIFIES | 13 | Array of StateNotify12 structs |
| STATE_RESOURCE_COSTS | 14 | Array of StateResourceCost12 structs |
| STATE_RESOURCE_PRECONDITIONS | 15 | Array of StateResourcePrecondition12 structs |
| STATE_RESOURCE_DELTAS | 16 | Array of StateResourceDelta16 structs |
| STATE_TAG_RANGES | 17 | Array of StateTagRange8 structs (parallel to STATES) |
| STATE_TAGS | 18 | Array of StrRef pointing to tag strings |
| CANCEL_TAG_RULES | 19 | Array of CancelTagRule24 structs |
| CANCEL_DENIES | 20 | Array of CancelDeny4 structs |
| CHARACTER_PROPS | 21 | Array of CharacterProp12 structs (dynamic key-value properties) |
| PUSH_WINDOWS | 22 | Array of PushWindow12 structs (body collision boxes) |
| STATE_PROPS | 23 | Per-state properties (index + CharacterProp12 records) |
| SCHEMA | 24 | Property and tag schema definitions |

### Data Structures

#### StrRef (8 bytes)

String references point into the STRING_TABLE section:

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 4 | off | Byte offset into STRING_TABLE |
| 4 | 2 | len | String length in bytes |
| 6 | 2 | _pad | Padding (reserved) |

#### StateRecord (36 bytes)

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 2 | state_id | Index in the STATES array |
| 2 | 2 | mesh_key | Index into MESH_KEYS (0xFFFF = none) |
| 4 | 2 | keyframes_key | Index into KEYFRAMES_KEYS (0xFFFF = none) |
| 6 | 1 | state_type | State type enum |
| 7 | 1 | trigger | Input trigger type |
| 8 | 1 | guard | Guard type (high/mid/low) |
| 9 | 1 | flags | State flags (see below) |
| 10 | 1 | startup | Startup frames |
| 11 | 1 | active | Active frames |
| 12 | 1 | recovery | Recovery frames |
| 13 | 1 | _reserved | Reserved |
| 14 | 2 | total | Total frame duration |
| 16 | 2 | damage | Damage value |
| 18 | 1 | hitstun | Hitstun frames |
| 19 | 1 | blockstun | Blockstun frames |
| 20 | 1 | hitstop | Hitstop frames |
| 21 | 1 | _reserved | Reserved |
| 22 | 4 | hit_windows_off | Byte offset within HIT_WINDOWS section |
| 26 | 2 | hit_windows_len | Number of hit windows |
| 28 | 2 | hurt_windows_off | Byte offset within HURT_WINDOWS section (compressed to u16) |
| 30 | 2 | hurt_windows_len | Number of hurt windows |
| 32 | 2 | push_windows_off | Byte offset within PUSH_WINDOWS section (compressed to u16) |
| 34 | 2 | push_windows_len | Number of push windows |

**State Flags (StateRecord.flags byte):**

| Bit | Flag | Description |
|-----|------|-------------|
| 0x01 | CHAIN | State has chain cancel routes in CANCELS_U16 |
| 0x02 | SPECIAL | State can cancel into special moves |
| 0x04 | SUPER | State can cancel into super moves |
| 0x08 | JUMP | State can cancel into jump |
| 0x10 | SELF_GATLING | State can cancel into itself |

#### ResourceDef12 (12 bytes)

Character resource pool definition.

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 8 | name | StrRef to resource name |
| 8 | 2 | start | Starting amount |
| 10 | 2 | max | Max amount |

#### StateExtras72 (72 bytes)

Per-state offsets/lengths for optional data arrays (parallel to `STATES`). All offsets are byte offsets into their respective backing section.

Each range is 8 bytes: `off(u32) + len(u16) + _pad(u16)`.

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 8 | on_use_emits | Range into `EVENT_EMITS` |
| 8 | 8 | on_hit_emits | Range into `EVENT_EMITS` |
| 16 | 8 | on_block_emits | Range into `EVENT_EMITS` |
| 24 | 8 | notifies | Range into `STATE_NOTIFIES` |
| 32 | 8 | resource_costs | Range into `STATE_RESOURCE_COSTS` |
| 40 | 8 | resource_preconditions | Range into `STATE_RESOURCE_PRECONDITIONS` |
| 48 | 8 | resource_deltas | Range into `STATE_RESOURCE_DELTAS` |
| 56 | 8 | input_notation | StrRef to state input notation (e.g., "5L", "236P") |
| 64 | 8 | cancels | Range into `CANCELS_U16` for chain cancel targets |

#### EventEmit16 (16 bytes)

One event emission: `emit_event(id, args)`.

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 8 | id | StrRef to event id |
| 8 | 8 | args | Range into `EVENT_ARGS` (byte off + count) |

#### EventArg20 (20 bytes)

Flat arg map entry `key -> value`.

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 8 | key | StrRef to arg key |
| 8 | 1 | tag | Value type tag: 0=bool, 1=i64, 2=f32, 3=string/enum |
| 9 | 3 | _reserved | Reserved (0) |
| 12 | 8 | value | Type-dependent payload |

`value` encoding by `tag`:

- `0 (bool)`: `u64` where 0=false, nonzero=true
- `1 (i64)`: `i64` little-endian
- `2 (f32)`: `f32` little-endian in the lower 4 bytes (upper 4 bytes 0)
- `3 (string/enum)`: StrRef packed as `off(u32) + len(u16) + _pad(u16)`

#### StateNotify12 (12 bytes)

Timeline-triggered notify point.

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 2 | frame | Frame number |
| 2 | 2 | _pad | Reserved (0) |
| 4 | 8 | emits | Range into `EVENT_EMITS` |

#### StateResourceCost12 (12 bytes)

Resource-type state costs only (`Cost::Resource`).

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 8 | name | StrRef to resource name |
| 8 | 2 | amount | Cost amount |
| 10 | 2 | _pad | Reserved (0) |

#### StateResourcePrecondition12 (12 bytes)

Resource-type state preconditions only (`Precondition::Resource`).

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 8 | name | StrRef to resource name |
| 8 | 2 | min | Minimum required (0xFFFF = none) |
| 10 | 2 | max | Maximum allowed (0xFFFF = none) |

#### StateResourceDelta16 (16 bytes)

Resource delta applied by a trigger.

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 8 | name | StrRef to resource name |
| 8 | 4 | delta | Signed delta (i32) |
| 12 | 1 | trigger | 0=on_use, 1=on_hit, 2=on_block |
| 13 | 3 | _pad | Reserved (0) |

#### Shape12 (12 bytes)

Hitbox/hurtbox geometry using Q12.4 fixed-point coordinates (1/16 pixel precision):

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 1 | kind | Shape type: 0=aabb, 1=rect, 2=circle, 3=capsule |
| 1 | 1 | flags | Reserved |
| 2 | 2 | a | Q12.4: x for aabb/rect/circle, x1 for capsule |
| 4 | 2 | b | Q12.4: y for aabb/rect/circle, y1 for capsule |
| 6 | 2 | c | Q12.4: width for aabb/rect, radius for circle, x2 for capsule |
| 8 | 2 | d | Q12.4: height for aabb/rect, unused for circle, y2 for capsule |
| 10 | 2 | e | Q8.8: angle for rect, radius for capsule |

#### HitWindow24 (24 bytes)

Active hitbox frame ranges:

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 1 | start_f | Start frame |
| 1 | 1 | end_f | End frame |
| 2 | 1 | guard | Guard type |
| 3 | 1 | _reserved | Reserved |
| 4 | 2 | dmg | Damage |
| 6 | 2 | chip | Chip damage (0 = none) |
| 8 | 1 | hitstun | Hitstun frames |
| 9 | 1 | blockstun | Blockstun frames |
| 10 | 1 | hitstop | Hitstop frames |
| 11 | 1 | _reserved | Reserved |
| 12 | 4 | shapes_off | Offset into SHAPES section |
| 16 | 2 | shapes_len | Number of shapes |
| 18 | 4 | cancels_off | Offset into CANCELS_U16 section |
| 22 | 2 | cancels_len | Number of cancel targets |

#### HurtWindow12 (12 bytes)

Hurtbox frame ranges:

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 1 | start_f | Start frame |
| 1 | 1 | end_f | End frame |
| 2 | 2 | hurt_flags | Hurtbox flags (invincibility, etc.) |
| 4 | 4 | shapes_off | Offset into SHAPES section |
| 8 | 2 | shapes_len | Number of shapes |
| 10 | 2 | _reserved | Reserved |

#### StateTagRange8 (8 bytes)

Per-state tag index range (parallel to STATES section). Points into the STATE_TAGS section.

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 4 | offset | Byte offset into STATE_TAGS section |
| 4 | 2 | count | Number of tags for this state |
| 6 | 2 | _pad | Reserved (0) |

#### CancelTagRule24 (24 bytes)

Tag-based cancel rule. Allows canceling from states with `from_tag` to states with `to_tag` under specified conditions.

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 4 | from_tag_off | StrRef offset for source tag (0xFFFFFFFF = any) |
| 4 | 2 | from_tag_len | StrRef length for source tag |
| 6 | 2 | _pad1 | Reserved (0) |
| 8 | 4 | to_tag_off | StrRef offset for target tag (0xFFFFFFFF = any) |
| 12 | 2 | to_tag_len | StrRef length for target tag |
| 14 | 2 | _pad2 | Reserved (0) |
| 16 | 1 | condition | 0=always, 1=on_hit, 2=on_block, 3=on_whiff |
| 17 | 1 | min_frame | Minimum frame for cancel (0 = no minimum) |
| 18 | 1 | max_frame | Maximum frame for cancel (0 = no maximum) |
| 19 | 1 | flags | Reserved (0) |
| 20 | 4 | _pad3 | Reserved (0) |

**Condition values:**

| Value | Condition | Description |
|-------|-----------|-------------|
| 0 | always | Cancel allowed unconditionally |
| 1 | on_hit | Cancel allowed only on hit |
| 2 | on_block | Cancel allowed only on block |
| 3 | on_whiff | Cancel allowed only on whiff |

#### CancelDeny4 (4 bytes)

Explicit cancel denial between two specific states (overrides tag-based rules).

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 2 | from_idx | Source state index |
| 2 | 2 | to_idx | Target state index |

#### CharacterProp12 (12 bytes)

Dynamic key-value character property. Supports numeric (Q24.8 fixed-point), boolean, and string values.

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 4 | name_off | Byte offset into STRING_TABLE |
| 4 | 2 | name_len | Property name length in bytes |
| 6 | 1 | value_type | Value type (see below) |
| 7 | 1 | _reserved | Reserved (0) |
| 8 | 4 | value | Type-dependent payload |

**Value type encoding:**

| Type | ID | Value encoding |
|------|----|----------------|
| number | 0 | Q24.8 signed fixed-point (i32) |
| bool | 1 | 0=false, nonzero=true |
| string | 2 | Packed StrRef: off(u16) + len(u16) |

Q24.8 provides a range of approximately +/-8 million with 1/256 precision, suitable for values like health (0-99999), speeds (0.0-100.0), and frame counts.

#### STATE_PROPS Section Layout

Per-state properties using the same CharacterProp12 format. Nested properties (Object, Array) are flattened at export time using dot notation:

- `{"movement": {"distance": 80}}` becomes `{"movement.distance": 80}`
- `{"effects": ["spark", 2]}` becomes `{"effects.0": "spark", "effects.1": 2}`

**Section Format:**

1. **Index** (8 bytes per state, parallel to STATES):

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 4 | offset | Byte offset into property data (after index) |
| 4 | 2 | byte_len | Length of property data in bytes |
| 6 | 2 | _pad | Reserved (0) |

2. **Property Data**: Concatenated CharacterProp12 records for all states.

States without properties have `(0, 0)` in their index entry. The `byte_len` field contains the total byte size (number of properties × 12).

#### PushWindow12 (12 bytes)

Body collision box frame ranges. Uses the same format as HurtWindow12.

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 1 | start_f | Start frame |
| 1 | 1 | end_f | End frame |
| 2 | 2 | flags | Reserved (currently unused for push boxes) |
| 4 | 4 | shapes_off | Offset into SHAPES section |
| 8 | 2 | shapes_len | Number of shapes |
| 10 | 2 | _reserved | Reserved |

Push windows define the body collision volume used for character-to-character pushing. When two characters' push boxes overlap, they are separated horizontally to prevent overlap.

### SECTION_SCHEMA (24)

The schema section enables strict schema mode, where property and tag names are defined once and referenced by index. When present, property records shrink from 12 bytes to 8 bytes, reducing file size.

**Purpose:**
- Defines the set of valid property and tag names for the character
- Enables schema validation at export time (typos caught early)
- Reduces property record size from 12 to 8 bytes (33% smaller per property)

**Net space savings calculation:**
- Without schema: Each property = 12 bytes (4-byte name offset + 2-byte name len + 1-byte type + 1-byte reserved + 4-byte value)
- With schema: Each property = 8 bytes (2-byte schema ID + 1-byte type + 1-byte reserved + 4-byte value)
- Schema overhead: 8-byte header + (N properties * 8 bytes for StringRefs) + (M tags * 8 bytes for StringRefs)
- Break-even point: When total property count exceeds schema overhead / 4 bytes saved per property

**Section Layout:**

1. **Header (8 bytes):**

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 2 | char_prop_count | Number of character property names |
| 2 | 2 | state_prop_count | Number of state property names |
| 4 | 2 | tag_count | Number of tag names |
| 6 | 2 | _pad | Reserved (0) |

2. **Character property names:** `[StrRef; char_prop_count]` - 8 bytes each, pointing to STRING_TABLE
3. **State property names:** `[StrRef; state_prop_count]` - 8 bytes each, pointing to STRING_TABLE
4. **Tag names:** `[StrRef; tag_count]` - 8 bytes each, pointing to STRING_TABLE

**Example `framesmith.rules.json` with schema:**

```json
{
  "version": 1,
  "properties": {
    "character": ["health", "walkSpeed", "dashSpeed", "jumpHeight"],
    "state": ["startup", "active", "recovery", "damage", "hitstun"]
  },
  "tags": ["normal", "special", "super", "startup", "active", "recovery"]
}
```

#### SchemaProp8 (8 bytes)

When the SCHEMA section is present, CHARACTER_PROPS and STATE_PROPS use this compact 8-byte format instead of CharacterProp12:

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 2 | schema_id | Index into schema's property list |
| 2 | 1 | value_type | 0=Q24.8, 1=bool, 2=string ref |
| 3 | 1 | _reserved | Reserved (0) |
| 4 | 4 | value | Type-dependent payload |

The `schema_id` indexes into either the character property names or state property names array in the SCHEMA section, depending on which section (CHARACTER_PROPS or STATE_PROPS) contains the record.

**Value type encoding** (same as CharacterProp12):

| Type | ID | Value encoding |
|------|----|----------------|
| number | 0 | Q24.8 signed fixed-point (i32) |
| bool | 1 | 0=false, nonzero=true |
| string | 2 | Packed StrRef: off(u16) + len(u16) |

## Error Handling

The `framesmith-fspack` crate returns specific errors for parse failures:

| Error | Description |
|-------|-------------|
| `TooShort` | Input data too short for valid header |
| `InvalidMagic` | Magic bytes are not "FSPK" |
| `OutOfBounds` | Section offset/length exceeds data bounds |

Example error handling:

```rust
use framesmith_fspack::{PackView, Error};

match PackView::parse(&buffer) {
    Ok(pack) => {
        // Use pack...
    }
    Err(Error::TooShort) => {
        log_error("Pack file truncated or corrupt");
    }
    Err(Error::InvalidMagic) => {
        log_error("Not a valid FSPK file");
    }
    Err(Error::OutOfBounds) => {
        log_error("FSPK file corrupt (invalid section offsets)");
    }
}
```

## Limitations (v1)

The current v1 format has the following limitations:

1. **Basic hitbox shapes only**: Only rectangular (AABB) hitboxes are exported from the current Framesmith schema. Shaped hitboxes (circles, capsules) require the v2 advanced state data schema.

2. **No compression**: Data is stored uncompressed. For bandwidth-sensitive applications, compress the `.fspk` file externally and decompress before parsing.

## Future Enhancements

Planned for future versions:

- **v2**: Support advanced shaped hitboxes (circles, capsules, oriented rects)
- **TBD**: Optional per-section compression

## Changelog

### v1.5 (2026-02-04)

- Added SECTION_SCHEMA (24) for property and tag schema definitions:
  - Defines valid property names for character and state properties
  - Defines valid tag names for state tagging
  - Enables strict schema validation at export time
- Property records now 8 bytes when schema present (was 12):
  - New SchemaProp8 format uses 2-byte schema ID instead of 6-byte string reference
  - 33% size reduction per property record
- **Breaking format change:** Schema-enabled exports use SchemaProp8 format; readers must check for SCHEMA section presence to determine property record format

### v1.4 (2026-02-04)

- Added per-state dynamic properties:
  - Section 23: `STATE_PROPS` - per-state key-value properties using same format as CHARACTER_PROPS
  - Properties support nesting in JSON (Object, Array) which is flattened at export using dot notation
  - Enables engine-agnostic state configuration without fixed schema changes

### v1.3 (2026-02-02)

- Added dynamic character properties:
  - Section 21: `CHARACTER_PROPS` - key-value properties with Q24.8/bool/string values
  - Replaces fixed character fields (health, walk_speed, etc.) with flexible map
- Added push boxes for body collision:
  - Section 22: `PUSH_WINDOWS` - body collision boxes (same format as hurt windows)
  - StateRecord expanded from 32 to 36 bytes to include push_windows_off/len

### v1.2 (2026-02-01)

- Renamed `Move` to `State` throughout (`MoveRecord` -> `StateRecord`, `MOVES` -> `STATES`, etc.)
- Added state tagging system:
  - Section 17: `STATE_TAG_RANGES` - per-state tag index ranges
  - Section 18: `STATE_TAGS` - tag string references
- Added tag-based cancel rules:
  - Section 19: `CANCEL_TAG_RULES` - flexible cancel rules based on source/target tags
  - Section 20: `CANCEL_DENIES` - explicit deny pairs that override tag rules
- Added `SELF_GATLING` flag (0x10) to state flags

### v1.1 (2026-01-30)

- Cancel flags (chain/special/super/jump) now exported in `StateRecord.flags` byte
- Chain cancel routes exported to `CANCELS_U16` section
- `StateExtras` expanded from 56 to 72 bytes to include cancel offset/length at bytes 64-71
