# Framesmith Data Formats

**Status:** Active
**Last reviewed:** 2026-02-02

Framesmith stores character data as a directory of JSON files. The Rust types in `src-tauri/src/schema/mod.rs` are the canonical definitions.

## Project Layout

A Framesmith project is a folder containing:

```text
<project>/
  framesmith.rules.json
  characters/
    <character-id>/
      character.json
      cancel_table.json
      states/
        <state-input>.json
      rules.json                (optional character-level rules)
```

Notes:

- `framesmith.rules.json` is required at the project root.
- `characters/` is required.
- `assets.json` is used by the UI animation preview (and can be loaded via backend commands).
- Files like `hurtboxes.json` may exist in some projects, but are currently ignored by the backend.

## `characters/<id>/character.json`

Character identity + dynamic properties.

```json
{
  "id": "test_char",
  "name": "GLITCH",
  "properties": {
    "archetype": "rushdown",
    "health": 10000,
    "walk_speed": 4.0,
    "back_walk_speed": 3.0,
    "jump_height": 120,
    "jump_duration": 45,
    "dash_distance": 80,
    "dash_duration": 18
  },
  "resources": [
    { "name": "heat", "start": 0, "max": 10 }
  ]
}
```

### Properties Map

The `properties` field is a flexible key-value map that replaces the old fixed fields. Property values can be:

- **Number**: floating-point values (exported as Q24.8 fixed-point)
- **Boolean**: true/false
- **String**: text values (e.g., archetype names)

Common property keys:

| Key | Type | Description |
|-----|------|-------------|
| `health` | number | Maximum health points |
| `walk_speed` | number | Forward walk speed (units/frame) |
| `back_walk_speed` | number | Backward walk speed (units/frame) |
| `jump_height` | number | Jump apex height in pixels |
| `jump_duration` | number | Jump duration in frames |
| `dash_distance` | number | Dash travel distance in pixels |
| `dash_duration` | number | Dash duration in frames |
| `archetype` | string | Character archetype (informational) |

Games can define additional custom properties as needed. Property names are validated against the rules registry if configured.

`resources[]` is optional; if omitted it defaults to `[]`.

## `characters/<id>/assets.json`

Per-character asset manifest for the Move Editor preview.

`move.animation` typically references a key in `assets.json` -> `animations`.

```json
{
  "version": 1,
  "textures": {
    "test_char_atlas": "assets/textures/test_char.png"
  },
  "models": {
    "test_char": "assets/models/test_char.glb"
  },
  "animations": {
    "stand_light": {
      "mode": "sprite",
      "texture": "test_char_atlas",
      "frame_size": { "w": 256, "h": 256 },
      "frames": 18,
      "pivot": { "x": 128, "y": 192 }
    },
    "stand_light_3d": {
      "mode": "gltf",
      "model": "test_char",
      "clip": "stand_light",
      "fps": 60,
      "pivot": { "x": 0, "y": 0, "z": 0 }
    }
  }
}
```

Notes:

- `textures` values are paths relative to `characters/<id>/`.
- `models` values are paths relative to `characters/<id>/`.
- `animations[*].mode`:
  - `sprite`: grid spritesheet playback.
  - `gltf`: glTF/GLB model playback (GLB recommended).

## `characters/<id>/states/<input>.json`

One file per state.

Practical guidance:

- The filename is typically `${input}.json`.
- Avoid filesystem-hostile characters in `input` (especially on Windows: `<>:"/\\|?*`).
- The UI "Create State" flow enforces a conservative subset; MCP/manual creation can still use additional characters as long as the OS supports the filename.

### Minimal (Core) State

These “core” fields are what the current UI surfaces and what the current exporters primarily use.

```json
{
  "input": "5L",
  "name": "Standing Light",
  "tags": [],

  "startup": 7,
  "active": 3,
  "recovery": 8,

  "damage": 30,
  "hitstun": 17,
  "blockstun": 11,
  "hitstop": 6,
  "guard": "mid",

  "hitboxes": [
    { "frames": [7, 9], "box": { "x": 0, "y": -40, "w": 30, "h": 16 } }
  ],
  "hurtboxes": [
    { "frames": [0, 6], "box": { "x": -10, "y": -60, "w": 30, "h": 60 } }
  ],
  "pushboxes": [
    { "frames": [0, 17], "box": { "x": -12, "y": -70, "w": 24, "h": 70 } }
  ],

  "pushback": { "hit": 5, "block": 8 },
  "meter_gain": { "hit": 5, "whiff": 2 },
  "animation": "stand_light"
}
```

### Optional / Advanced Fields

Moves also support additional optional fields (all are optional unless stated otherwise):

- `type`: `normal | command_normal | special | super | movement | throw`
- `trigger`: `press | release | hold` (default behavior is `press` when omitted)
- `parent`: string (for follow-ups / strings)
- `total`: number (override total duration)
- `hits[]`: multi-hit model with shaped hitboxes (currently not exported by `zx-fspack` v1)
- `preconditions[]`: requirements to use the move (meter/charge/state/etc.)
- `costs[]`: meter/health/resource costs
- `movement`: distance/velocity-based movement data
- `super_freeze`: cinematic freeze parameters
- `on_use`, `on_hit`, `on_block`: gameplay effects + notification events
- `notifies[]`: timeline-triggered notification events
- `advanced_hurtboxes[]`: shaped hurtboxes with flags (currently not exported by `zx-fspack` v1)
- `pushboxes[]`: body collision boxes for character-to-character push separation (same format as hurtboxes)

## Events (Notification)

Moves can emit notification events in these places:

- `on_use.events[]`
- `on_hit.events[]`
- `on_block.events[]`
- `notifies[].events[]`

Event structure:

```json
{
  "id": "vfx.hit_sparks",
  "args": {
    "enabled": true,
    "count": 3,
    "scale": 1.2,
    "strength": "light"
  }
}
```

`args` is a flat map of primitives: boolean, integer, float, string.

Event IDs and their allowed contexts/args are validated against the rules registry (see `docs/rules-spec.md`).

## Resources (Gameplay)

Character resources live in `character.json` as `resources[]`.

Moves can reference resources via:

- `preconditions`: `{ "type": "resource", "name": "heat", "min": 1 }`
- `costs`: `{ "type": "resource", "name": "ammo", "amount": 1 }`
- `on_use.resource_deltas[]`, `on_hit.resource_deltas[]`, `on_block.resource_deltas[]`: `{ "name": "heat", "delta": 1 }`

Resource names are validated against the rules registry.

## `characters/<id>/cancel_table.json`

Central cancel relationship table:

```json
{
  "chains": {
    "5L": ["5L", "5M"],
    "5M": ["5H"]
  },
  "special_cancels": ["5L", "5M", "5H"],
  "super_cancels": ["5H"],
  "jump_cancels": ["5H"]
}
```

## Rules Files

- Project rules: `<project>/framesmith.rules.json`
- Character rules (optional): `<project>/characters/<id>/rules.json`

Rules semantics, matching, and built-in validations are specified in `docs/rules-spec.md`.

## Export Outputs

- `json-blob`: a single JSON blob containing the resolved character + moves (after rule defaults are applied). It includes optional/advanced fields when present.
- `zx-fspack`: a compact binary pack documented in `docs/zx-fspack.md`
