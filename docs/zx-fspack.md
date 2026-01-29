# ZX FSPK Export Format

**Status:** Draft
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
- Programmatically: call the `export_character` command with `adapter = "zx-fspack"`

```rust
// Tauri command signature (Rust side)
// export_character(characters_dir, character_id, adapter, output_path, pretty)
export_character(
    "<project>/characters".to_string(),
    "glitch".to_string(),
    "zx-fspack".to_string(),
    "exports/glitch.fspk".to_string(),
    false,
)?;
```

This produces a `.fspk` binary file containing:
- Character move data (frame counts, damage, hitstun, etc.)
- Hitbox and hurtbox geometry
- Asset key references (mesh and animation keys)

## Runtime Usage (ZX Games)

Games load FSPK files during initialization and resolve asset keys to runtime handles.

### Loading the Pack

```rust
use framesmith_fspack::{PackView, KEY_NONE};

// In ZX game init():

// 1. Get ROM data length and allocate buffer
let len = rom_data_len("char/glitch.fspk");
let mut buffer = alloc_buffer(len);

// 2. Load ROM data into buffer
rom_data("char/glitch.fspk", &mut buffer);

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
                // mesh key format: "{character_id}.{animation}" (e.g. "glitch.stand_light")
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

### Accessing Move Data at Runtime

Once loaded, access move data using the move ID (index):

```rust
// 6. At runtime, look up move data by ID
if let Some(moves) = pack.moves() {
    let move_id = 0; // e.g., standing light punch
    if let Some(mv) = moves.get(move_id) {
        // Frame data
        let startup = mv.startup();       // u8: startup frames
        let active = mv.active();         // u8: active frames
        let recovery = mv.recovery();     // u8: recovery frames
        let total = mv.total();           // u16: total duration

        // Combat data
        let damage = mv.damage();         // u16: damage value
        let hitstun = mv.hitstun();       // u8: hitstun frames
        let blockstun = mv.blockstun();   // u8: blockstun frames
        let hitstop = mv.hitstop();       // u8: hitstop frames
        let guard = mv.guard();           // u8: guard type (high/mid/low)

        // Asset references (indices into handle arrays)
        let mesh_idx = mv.mesh_key();
        if mesh_idx != KEY_NONE {
            let mesh = mesh_handles[mesh_idx as usize];
            // Use mesh for rendering
        }

        let kf_idx = mv.keyframes_key();
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
| MOVES | 4 | Array of MoveRecord structs |
| HIT_WINDOWS | 5 | Array of HitWindow24 structs (active hitbox frames) |
| HURT_WINDOWS | 6 | Array of HurtWindow12 structs (hurtbox frames) |
| SHAPES | 7 | Array of Shape12 structs (hitbox/hurtbox geometry) |
| CANCELS_U16 | 8 | Array of u16 move IDs for cancel targets (v1: empty) |
| RESOURCE_DEFS | 9 | Array of ResourceDef12 structs (character resource pools) |
| MOVE_EXTRAS | 10 | Array of MoveExtras56 structs (parallel to MOVES) |
| EVENT_EMITS | 11 | Array of EventEmit16 structs |
| EVENT_ARGS | 12 | Array of EventArg20 structs |
| MOVE_NOTIFIES | 13 | Array of MoveNotify12 structs |
| MOVE_RESOURCE_COSTS | 14 | Array of MoveResourceCost12 structs |
| MOVE_RESOURCE_PRECONDITIONS | 15 | Array of MoveResourcePrecondition12 structs |
| MOVE_RESOURCE_DELTAS | 16 | Array of MoveResourceDelta16 structs |

### Data Structures

#### StrRef (8 bytes)

String references point into the STRING_TABLE section:

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 4 | off | Byte offset into STRING_TABLE |
| 4 | 2 | len | String length in bytes |
| 6 | 2 | _pad | Padding (reserved) |

#### MoveRecord (32 bytes)

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 2 | move_id | Index in the MOVES array |
| 2 | 2 | mesh_key | Index into MESH_KEYS (0xFFFF = none) |
| 4 | 2 | keyframes_key | Index into KEYFRAMES_KEYS (0xFFFF = none) |
| 6 | 1 | move_type | Move type enum |
| 7 | 1 | trigger | Input trigger type |
| 8 | 1 | guard | Guard type (high/mid/low) |
| 9 | 1 | flags | Move flags |
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

#### ResourceDef12 (12 bytes)

Character resource pool definition.

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 8 | name | StrRef to resource name |
| 8 | 2 | start | Starting amount |
| 10 | 2 | max | Max amount |

#### MoveExtras56 (56 bytes)

Per-move offsets/lengths for optional data arrays (parallel to `MOVES`). All offsets are byte offsets into their respective backing section.

Each range is 8 bytes: `off(u32) + len(u16) + _pad(u16)`.

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 8 | on_use_emits | Range into `EVENT_EMITS` |
| 8 | 8 | on_hit_emits | Range into `EVENT_EMITS` |
| 16 | 8 | on_block_emits | Range into `EVENT_EMITS` |
| 24 | 8 | notifies | Range into `MOVE_NOTIFIES` |
| 32 | 8 | resource_costs | Range into `MOVE_RESOURCE_COSTS` |
| 40 | 8 | resource_preconditions | Range into `MOVE_RESOURCE_PRECONDITIONS` |
| 48 | 8 | resource_deltas | Range into `MOVE_RESOURCE_DELTAS` |

#### EventEmit16 (16 bytes)

One notification event emission: `emit_event(id, args)`.

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

#### MoveNotify12 (12 bytes)

Timeline-triggered notify point.

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 2 | frame | Frame number |
| 2 | 2 | _pad | Reserved (0) |
| 4 | 8 | emits | Range into `EVENT_EMITS` |

#### MoveResourceCost12 (12 bytes)

Resource-type move costs only (`Cost::Resource`).

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 8 | name | StrRef to resource name |
| 8 | 2 | amount | Cost amount |
| 10 | 2 | _pad | Reserved (0) |

#### MoveResourcePrecondition12 (12 bytes)

Resource-type move preconditions only (`Precondition::Resource`).

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 8 | name | StrRef to resource name |
| 8 | 2 | min | Minimum required (0xFFFF = none) |
| 10 | 2 | max | Maximum allowed (0xFFFF = none) |

#### MoveResourceDelta16 (16 bytes)

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

1. **Cancel routes not included**: The CANCELS_U16 section is present but empty in v1. Cancel relationships are managed separately.

2. **Basic hitbox shapes only**: Only rectangular (AABB) hitboxes are exported from the current Framesmith schema. Shaped hitboxes (circles, capsules) require the v2 advanced move data schema.

3. **No compression**: Data is stored uncompressed. For bandwidth-sensitive applications, compress the `.fspk` file externally and decompress before parsing.

## Future Enhancements

Planned for future versions:

- **v2**: Pack cancel routes from `cancel_table.json` into CANCELS_U16
- **v2**: Support advanced shaped hitboxes (circles, capsules, oriented rects)
- **TBD**: Optional per-section compression
