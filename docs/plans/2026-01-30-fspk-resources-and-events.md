# FSPK Resources + Notification Events Plan

**Status:** Draft
**Date:** 2026-01-30

## Goal

Add two engine-agnostic systems to Framesmith and export them through FSPK:

1) **Resources** (gameplay): named resource pools with per-character `start/max`, move `costs`, `preconditions`, and `on_use/on_hit/on_block` deltas.

2) **Notification events** (non-gameplay): `emit_event(id, args)` hooks for SFX/VFX/camera/script. Events can fire:

- **On hit/block** (contact result)
- **On timeline frames** (move notifies)

Games may ignore either system, but Framesmith must validate them to avoid silent errors (unknown IDs, wrong arg types, etc.).

## Decisions

- Events are **notification-only** (do not change gameplay state).
- Resources are **typed gameplay** (not encoded as events).
- Event args are a **flat map** of `string -> primitive` (no nested objects/arrays).
- Project-level registries live in `framesmith.rules.json` (typo-proofing + validation).
- FSPK header: **remove the `version` field** (lockstep bundler+game assumption).

## Data Model Changes

### Rules Registry (project + character rules)

Extend `framesmith.rules.json` schema with an optional `registry` block:

```json
{
  "version": 1,
  "registry": {
    "resources": ["heat", "ammo"],
    "events": {
      "vfx.hit_sparks": {
        "contexts": ["on_hit", "on_block"],
        "args": {
          "strength": { "type": "enum", "values": ["light", "med", "heavy"] },
          "scale": { "type": "f32", "min": 0.0, "max": 10.0 }
        }
      },
      "vfx.swing_trail": {
        "contexts": ["notify"],
        "args": {
          "bone": { "type": "string" },
          "color": { "type": "string" }
        }
      }
    }
  },
  "apply": [],
  "validate": []
}
```

Merging rules:

- `resources`: union + dedup (project then character).
- `events`: merge by key; character overrides event definitions with the same id.

### Character Resources

Add `resources` to `characters/<id>/character.json`:

```json
"resources": [
  { "name": "heat", "start": 0, "max": 10 }
]
```

### Move Notification Events

Add events in two places:

- `on_use.events[]`, `on_hit.events[]`, `on_block.events[]`
- `notifies[]`: timeline-triggered events

Example:

```json
"on_hit": {
  "events": [
    { "id": "vfx.hit_sparks", "args": { "strength": "light", "scale": 1.2 } }
  ]
},
"notifies": [
  { "frame": 7, "events": [ { "id": "vfx.swing_trail", "args": { "bone": "hand_r" } } ] }
]
```

Arg values allowed: `bool`, `i64`, `f32`, `string`, and `enum` (encoded as string).

## Validation (No Silent Errors)

Add built-in validations (hard errors on save/export):

Resources:

- Character resource names must exist in registry.
- Character resource names must be unique.
- `0 <= start <= max`.
- Any move reference to a resource name (preconditions/costs/deltas) must exist in registry.

Events:

- Event id must exist in registry.
- Event must be allowed in the context where it is used (`on_use`, `on_hit`, `on_block`, `notify`).
- Args:
  - unknown arg key is an error
  - missing required args (if we add `required`) is an error
  - value type mismatch is an error
  - `enum` values must be one of the allowed strings
  - numeric min/max enforced when present

Notifies:

- `frame` must be within `[0, total_frames]` where `total_frames = startup + active + recovery`.

Integration points:

- Extend the rules validation pipeline (`src-tauri/src/rules/mod.rs`) to validate moves against the merged registry.
- Update the MCP `update_move`/`create_move` path to use registry-aware validation (so MCP edits can’t bypass registry checks).

## FSPK Export Changes

### Header

Remove `version: u16` from the container header.

- Keep header length 16 bytes.
- Recommended replacement layout: `magic[4] + flags(u32) + total_len(u32) + section_count(u32)`.

Update both:

- Exporter (`src-tauri/src/codegen/zx_fspack.rs` + `src-tauri/src/codegen/zx_fspack_format.rs`)
- Reader crate (`crates/framesmith-fspack`)
- Docs (`docs/zx-fspack.md`)

### New Optional Sections (v-next)

Add new section kinds (stable IDs) for resources + events. Keep existing sections intact.

Recommended structure to avoid modifying `MoveRecord`:

- `MOVE_EXTRAS`: fixed-size record parallel to `MOVES` that stores offsets/lengths for optional per-move data.
- Backing arrays:
  - `EVENT_EMITS`
  - `EVENT_ARGS`
  - `MOVE_NOTIFIES`
  - `MOVE_RESOURCE_DELTAS` (separate by trigger or include trigger enum in record)
- Store event ids and arg keys (and string/enum values) in `STRING_TABLE` via `StrRef`.

## Implementation Tasks

1) Update rules schema + docs
- Add `registry` to `src-tauri/src/rules/mod.rs` types and JSON schema generation.
- Update `docs/rules-spec.md` with `registry` format and merge semantics.
- Add minimal examples to `framesmith.rules.json`.

2) Add resources + events to Rust schema
- Extend `src-tauri/src/schema/mod.rs`:
  - `Character.resources`
  - `OnUse/OnHit/OnBlock.events` and `resource_deltas`
  - `Move.notifies`
  - event arg value enum (flat primitives)

3) Registry-aware validation
- Implement registry merge helper in `src-tauri/src/rules/mod.rs`.
- Validate:
  - character resources vs registry
  - move resource references vs registry
  - event emits vs registry (contexts + arg schemas)
  - notify frame ranges
- Update MCP tool validation path to use the same checks.

4) FSPK format updates
- Remove `version` field from header in exporter + reader.
- Add section constants + docs.
- Implement exporter packing for resources + notification events.
- Implement reader views for the new sections.

5) Tests
- Unit tests for:
  - registry merging
  - event arg validation (unknown id/key/type/enum)
  - notify frame bounds
- Roundtrip exporter->reader tests that assert:
  - events are present and decodable
  - resource defs + deltas are present and decodable

## Verification

- `cd src-tauri && cargo test`
- `cargo test -p framesmith-fspack`
- Export `zx-fspack` for a sample character and confirm:
  - reader parses pack
  - events + resources sections readable
