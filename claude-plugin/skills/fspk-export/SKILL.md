---
name: fspk-export
description: >-
  Exporting fighting game characters to FSPK binary format and integrating with
  the Nethercore ZX runtime. Covers the FSPK container format, section types,
  zero-copy parsing, asset resolution, and runtime APIs (next_frame, cancel
  system, hit detection, resources). Use when exporting characters, reading FSPK
  files, or integrating character data into a game engine.
license: MIT
compatibility: Requires Framesmith (Tauri app or CLI) for export. Runtime requires framesmith-fspack and framesmith-runtime crates.
metadata:
  author: nethercore-systems
  version: "1.0.0"
---

# FSPK Export & Runtime Integration

FSPK (Framesmith Pack) is a compact binary format for fighting game character data, designed for the Nethercore ZX `no_std` WebAssembly runtime.

## Why FSPK?

- **Compact**: Fixed-size records minimize storage and bandwidth
- **Zero-copy**: Parsed directly from buffer without allocations
- **no_std compatible**: Works in WASM without the Rust standard library
- **Unaligned-safe**: All reads use byte-level access

## Exporting

### From the App

Character Overview > Export > "ZX FSPK (Binary)"

### From the CLI

```bash
cd src-tauri
cargo run --bin framesmith -- export \
  --project .. \
  --character test_char \
  --out ../exports/test_char.fspk
```

### Programmatically (Tauri command)

```rust
export_character(
    "<project>/characters".to_string(),
    "test_char".to_string(),
    "zx-fspack".to_string(),
    "exports/test_char.fspk".to_string(),
    false,
)?;
```

## Format Overview

An FSPK file consists of:

1. **Container header** (16 bytes): magic `"FSPK"`, flags, total length, section count
2. **Section headers** (16 bytes each): kind, offset, length, alignment
3. **Section data**: variable-length payloads referenced by headers

### Section Types (24 total)

| Kind | ID | Purpose |
|------|-----|---------|
| STRING_TABLE | 1 | UTF-8 string data |
| MESH_KEYS | 2 | Mesh asset key references |
| KEYFRAMES_KEYS | 3 | Animation asset key references |
| STATES | 4 | StateRecord structs (36 bytes each) |
| HIT_WINDOWS | 5 | Active hitbox frame ranges (24 bytes) |
| HURT_WINDOWS | 6 | Hurtbox frame ranges (12 bytes) |
| SHAPES | 7 | Hitbox/hurtbox geometry (12 bytes) |
| CANCELS_U16 | 8 | Cancel target state IDs |
| RESOURCE_DEFS | 9 | Character resource pools |
| STATE_EXTRAS | 10 | Per-state optional data ranges (72 bytes) |
| EVENT_EMITS | 11 | Event emissions |
| EVENT_ARGS | 12 | Event arguments |
| STATE_NOTIFIES | 13 | Timeline notify points |
| STATE_RESOURCE_COSTS | 14 | Resource costs per state |
| STATE_RESOURCE_PRECONDITIONS | 15 | Resource preconditions |
| STATE_RESOURCE_DELTAS | 16 | Resource deltas (on_use/on_hit/on_block) |
| STATE_TAG_RANGES | 17 | Per-state tag index ranges |
| STATE_TAGS | 18 | Tag string references |
| CANCEL_TAG_RULES | 19 | Tag-based cancel rules |
| CANCEL_DENIES | 20 | Explicit cancel denials |
| CHARACTER_PROPS | 21 | Dynamic key-value properties |
| PUSH_WINDOWS | 22 | Body collision boxes |
| STATE_PROPS | 23 | Per-state properties |
| SCHEMA | 24 | Property/tag schema definitions |

See `references/fspk-format.md` for complete byte-level specifications.

## Runtime Integration

### Loading a Pack

```rust
use framesmith_fspack::{PackView, KEY_NONE};

let len = rom_data_len("char/test_char.fspk");
let mut buffer = alloc_buffer(len);
rom_data("char/test_char.fspk", &mut buffer);
let pack = PackView::parse(&buffer).expect("invalid pack");
```

### Resolving Asset Handles

Asset keys are string references into the pack's string table. Resolve at init time:

```rust
// Mesh handles
let mut mesh_handles: Vec<MeshHandle> = Vec::new();
if let Some(mesh_keys) = pack.mesh_keys() {
    for i in 0..mesh_keys.len() {
        if let Some((off, len)) = mesh_keys.get(i) {
            if let Some(key) = pack.string(off, len) {
                mesh_handles.push(rom_mesh(key));
            }
        }
    }
}
```

### Frame-by-Frame Simulation

```rust
use framesmith_runtime::{CharacterState, FrameInput, next_frame, init_resources};

let mut state = CharacterState::default();
init_resources(&mut state, &pack);

loop {
    let input = FrameInput {
        requested_state: player_wants_to_attack().then_some(1),
    };
    let result = next_frame(&state, &pack, &input);
    state = result.state;
    if result.move_ended {
        state.current_state = 0;
        state.frame = 0;
    }
}
```

### Cancel System

Cancels are validated in this order:
1. **Explicit denies** - hard blocks between specific states
2. **Explicit chain cancels** - state-specific routes (rekkas, target combos)
3. **Tag-based rules** - pattern rules like "normals cancel into specials on hit"

```rust
use framesmith_runtime::can_cancel_to;

if can_cancel_to(&state, &pack, target_state) {
    // Cancel is allowed
}
```

Cancel conditions: `always`, `on_hit`, `on_block`, `on_whiff`

### Hit Detection

```rust
use framesmith_runtime::{check_hits, report_hit, report_block};

let hits = check_hits(
    &attacker_state, &attacker_pack, attacker_pos,
    &defender_state, &defender_pack, defender_pos,
);

for hit in hits.iter() {
    // hit.damage, hit.hitstun, hit.blockstun, hit.hitstop, hit.guard
}
```

### Resources

Characters have up to 8 resource pools. Costs and preconditions are handled automatically by the runtime. Deltas must be applied manually by the engine (rollback safety).

```rust
use framesmith_runtime::{init_resources, resource, set_resource};

init_resources(&mut state, &pack);
let meter = resource(&state, 0);
set_resource(&mut state, 0, meter + 10);
```

### Rollback Netcode

`CharacterState` is 22 bytes, `Copy`, and deterministic - designed for rollback:

```rust
let saved = game.p1_state;  // Save (zero-cost copy)
// ... rollback needed ...
game.p1_state = saved;       // Restore (instant)
```

## Error Handling

```rust
use framesmith_fspack::{PackView, Error};

match PackView::parse(&buffer) {
    Ok(pack) => { /* use pack */ }
    Err(Error::TooShort) => { /* truncated or corrupt */ }
    Err(Error::InvalidMagic) => { /* not a valid FSPK file */ }
    Err(Error::OutOfBounds) => { /* corrupt section offsets */ }
}
```

## Key Constraints

- **FSPK = fixed-size records, zero-copy.** Never use variable-length encoding in sections.
- **Writer and reader must stay in sync.** `codegen/fspk/` (writer) and `crates/framesmith-fspack/` (reader) must agree on byte offsets.
- **Nested properties are flattened.** `Object`/`Array` PropertyValues become dot-path keys at export (e.g., `movement.distance`).
- **Resource deltas are engine responsibility.** The runtime handles costs/preconditions automatically but deltas must be applied manually for rollback safety.

## References

- `references/fspk-format.md` - Complete byte-level format specification
- `references/runtime-integration.md` - Runtime API reference and integration patterns
