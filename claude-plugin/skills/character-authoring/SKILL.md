---
name: character-authoring
description: >-
  Authoring fighting game characters in Framesmith. Covers state definitions,
  hitboxes, hurtboxes, cancel tables, properties, and the one-file-per-state
  JSON format. Use when creating, editing, or understanding character data
  for fighting games.
license: MIT
compatibility: Requires Framesmith (Tauri app) or framesmith CLI.
metadata:
  author: nethercore-systems
  version: "1.0.0"
---

# Framesmith Character Authoring

Framesmith is an engine-agnostic fighting game character authoring tool. Characters
are stored as directories of JSON files -- one file per state, one cancel table,
one character identity file. This format is git-friendly and diffable.

## Project Layout

```
<project>/
  framesmith.rules.json          # Project-wide rules and defaults
  characters/
    <character-id>/
      character.json             # Identity, properties, resources
      cancel_table.json          # All cancel relationships
      states/
        <input>.json             # One file per state (filename = input)
      rules.json                 # Optional character-level rule overrides
      assets.json                # Optional animation preview manifest
      globals.json               # Optional global state opt-ins
```

## State File Format

Each state lives at `states/<input>.json`. The filename must match the `input` field.

### Core Fields

```json
{
  "input": "5L",
  "name": "Standing Light",
  "tags": ["normal", "light"],

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

### Field Reference

| Field | Type | Description |
|-------|------|-------------|
| `input` | string | Notation input (e.g. `5L`, `236P`, `j.H`) |
| `name` | string | Human-readable name |
| `tags` | string[] | Lowercase tags for categorization and cancel rules |
| `startup` | number | Frames before active (>= 1) |
| `active` | number | Active hitbox frames (>= 1) |
| `recovery` | number | Recovery frames after active |
| `damage` | number | Base damage value |
| `hitstun` | number | Frames of stun on hit |
| `blockstun` | number | Frames of stun on block |
| `hitstop` | number | Freeze frames on contact |
| `guard` | string | `"high"`, `"mid"`, `"low"`, or `"unblockable"` |
| `hitboxes` | array | Damage regions with frame ranges |
| `hurtboxes` | array | Vulnerable regions with frame ranges |
| `pushboxes` | array | Body collision regions |
| `pushback` | object | `{ hit, block }` pushback distances |
| `meter_gain` | object | `{ hit, whiff }` meter values |
| `animation` | string | Animation key (references assets.json) |

### Optional Fields

- `type`: `normal | command_normal | special | super | movement | throw`
- `trigger`: `press | release | hold`
- `parent`: string -- for follow-up/rekka chains
- `total`: number -- override computed total duration
- `base`: string -- variant inheritance (single-level, shallow merge)
- `hits[]`: multi-hit definitions with per-hit hitboxes
- `preconditions[]`: conditions to use (meter, charge, airborne, etc.)
- `costs[]`: resource costs (`{ type, name, amount }`)
- `movement`: distance-based or physics-based displacement
- `super_freeze`: cinematic freeze parameters
- `on_use`, `on_hit`, `on_block`: gameplay effects and events
- `notifies[]`: timeline-triggered notification events
- `advanced_hurtboxes[]`: shaped hurtboxes with invulnerability flags

### Variant Inheritance

States can inherit from a base state:

```json
{
  "input": "j.5L",
  "base": "5L",
  "name": "Jumping Light",
  "damage": 25
}
```

Rules: single-level only, fields override via shallow merge, `null` removes inherited fields.

## character.json

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

Properties are flexible key-value pairs (number, boolean, or string). Games can
define custom properties. Resources define per-character gauges.

## Cancel Table Basics

The cancel table (`cancel_table.json`) defines all state transition rules.

### Tag-Based Rules

```json
{
  "tag_rules": [
    { "from": "system", "to": "any", "on": "always" },
    { "from": "normal", "to": "special", "on": ["hit", "block"] },
    { "from": "special", "to": "super", "on": ["hit", "block"] }
  ],
  "deny": {}
}
```

- `from`/`to`: match tags on source/target states (`"any"` = wildcard)
- `on`: condition -- `"always"`, `"hit"`, `"block"`, `"whiff"`, or array of conditions
- `deny`: explicit overrides that block specific transitions

### Explicit Chains (Legacy/Supplement)

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

See `references/cancel-tables.md` for full documentation.

## Naming Conventions

### Numpad Notation

Directions use numpad layout (P1 facing right):

```
7=up-back   8=up     9=up-forward
4=back      5=neutral  6=forward
1=down-back 2=down   3=down-forward
```

### State Input Patterns

| Pattern | Examples | Purpose |
|---------|----------|---------|
| `5L`, `5M`, `5H` | Standing normals | Button strength attacks |
| `2L`, `2M`, `2H` | Crouching normals | Low attacks |
| `j.L`, `j.M`, `j.H` | Air normals | `j.` prefix = airborne |
| `236P`, `214K` | Motion specials | Numpad motion + button |
| `0_idle`, `0_walk_forward` | Movement states | `0_` = ground system |
| `1_crouch` | Crouch states | `1_` = crouch system |
| `66`, `44` | Dashes | Double-tap direction |
| `j.66`, `j.44` | Air dashes | Air double-tap |

### Tags

Lowercase alphanumeric + underscores only. Common tags: `normal`, `special`,
`super`, `system`, `movement`, `light`, `medium`, `heavy`, `reversal`, `starter`.

## When to Load References

- Creating or editing state JSON: read `references/data-formats.md` for full schema
- Building cancel tables: read `references/cancel-tables.md` for patterns and rules
- Understanding rules/validation: see `docs/rules-spec.md` in the Framesmith repo
- Movement mechanics: see `docs/movement-reference.md` in the Framesmith repo
