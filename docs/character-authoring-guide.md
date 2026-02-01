# Character Authoring Guide

This guide covers creating a complete character with all movement and system states defined in Framesmith (the "all states in Framesmith" approach).

## When to Use This Approach

Use this approach when you want:

- **Full data-driven characters**: All behavior defined in JSON, no engine-side state machines
- **Git-friendly diffs**: One file per state makes merges manageable
- **Portable characters**: Export to any supported runtime format

The alternative is defining only attack moves in Framesmith while handling movement/system states in your engine's native state machine.

## Required States for a Complete Character

A fully playable character needs states in these categories:

### Movement States

| State | Input Convention | Purpose |
|-------|------------------|---------|
| Idle | `0_idle` | Neutral standing, loops indefinitely |
| Walk Forward | `0_walk_forward` or `5_walk` | Moving toward opponent |
| Walk Backward | `0_walk_backward` or `4_walk` | Moving away from opponent |
| Crouch | `1_crouch` | Crouching idle |
| Crouch Walk | `1_crouch_walk` | Optional: moving while crouched |
| Forward Dash | `66` | Double-tap forward dash |
| Back Dash | `44` | Double-tap backward dash |

### Jump States

| State | Input Convention | Purpose |
|-------|------------------|---------|
| Jump Startup | `8_prejump` | Pre-jump squat frames |
| Jump Neutral | `j.8` or `8_airborne` | Rising/falling neutral jump |
| Jump Forward | `j.9` or `9_airborne` | Forward jump arc |
| Jump Backward | `j.7` or `7_airborne` | Backward jump arc |
| Landing | `0_landing` | Landing recovery frames |
| Air Dash | `j.66` | Forward air dash |
| Air Back Dash | `j.44` | Backward air dash |

### Defensive States

| State | Input Convention | Purpose |
|-------|------------------|---------|
| Stand Block | `0_block_high` | Standing block |
| Crouch Block | `1_block_low` | Crouching block |
| Hitstun | `hitstun` | Taking a hit |
| Blockstun | `blockstun` | Blocked attack recovery |
| Knockdown | `knockdown` | Knocked down state |
| Wake Up | `wakeup` | Getting up from knockdown |

### System States (Optional)

| State | Input Convention | Purpose |
|-------|------------------|---------|
| Burst | `burst` | Defensive burst mechanic |
| Throw | `5T` or `throw` | Throw attempt |
| Throw Tech | `throw_tech` | Throw escape |
| Air Tech | `air_tech` | Air recovery |

## State Naming Conventions

### Numpad Notation

Directions use numpad positions (as viewed from P1 side):

```
7 8 9    ↖ ↑ ↗
4 5 6    ← ● →
1 2 3    ↙ ↓ ↘
```

- `5` = neutral (no direction)
- `6` = forward, `4` = backward
- `8` = up, `2` = down
- `j.` prefix = airborne version

### Recommended Naming Scheme

Movement and system states typically use:

- `0_<name>`: Ground system states (idle, walk, landing)
- `1_<name>`: Crouch system states
- `<numpad>`: Directional actions (66 for dash, 44 for backdash)
- `j.<numpad>`: Air actions
- `<name>`: Named system states (hitstun, blockstun)

Attack moves follow standard notation:

- `5L`, `5M`, `5H`: Standing normals (L=Light, M=Medium, H=Heavy)
- `2L`, `2M`, `2H`: Crouching normals
- `j.L`, `j.M`, `j.H`: Air normals
- `236P`: Motion input specials (quarter circle forward + punch)

## Creating Core Movement States

### Idle State

The idle state loops indefinitely. Set `total` to a short value for the loop cycle.

```json
{
  "input": "0_idle",
  "name": "Standing Idle",
  "type": "system",
  "startup": 1,
  "active": 1,
  "recovery": 0,
  "total": 2,
  "damage": 0,
  "hitstun": 0,
  "blockstun": 0,
  "hitstop": 0,
  "guard": "mid",
  "hitboxes": [],
  "hurtboxes": [
    { "frames": [0, 1], "box": { "x": -15, "y": -70, "w": 30, "h": 70 } }
  ],
  "pushback": { "hit": 0, "block": 0 },
  "meter_gain": { "hit": 0, "whiff": 0 },
  "animation": "idle"
}
```

### Forward Dash with Movement

Dashes use the `movement` field for displacement:

```json
{
  "input": "66",
  "name": "Forward Dash",
  "type": "movement",
  "startup": 3,
  "active": 1,
  "recovery": 6,
  "total": 18,
  "damage": 0,
  "hitstun": 0,
  "blockstun": 0,
  "hitstop": 0,
  "guard": "mid",
  "hitboxes": [],
  "hurtboxes": [],
  "pushback": { "hit": 0, "block": 0 },
  "meter_gain": { "hit": 0, "whiff": 0 },
  "animation": "dash_forward",
  "movement": {
    "distance": 80,
    "direction": "forward",
    "curve": "ease-out"
  }
}
```

### Back Dash with Invincibility

Back dashes often have invincibility frames using `advanced_hurtboxes`:

```json
{
  "input": "44",
  "name": "Back Dash",
  "type": "movement",
  "startup": 4,
  "active": 1,
  "recovery": 12,
  "total": 24,
  "movement": {
    "distance": 60,
    "direction": "backward",
    "curve": "ease-out"
  },
  "advanced_hurtboxes": [
    {
      "frames": [0, 8],
      "boxes": [{ "type": "aabb", "x": -12, "y": -65, "w": 24, "h": 65 }],
      "flags": ["full_invuln"]
    },
    {
      "frames": [9, 24],
      "boxes": [{ "type": "aabb", "x": -12, "y": -65, "w": 24, "h": 65 }]
    }
  ]
}
```

### Air Dash with Physics

Air movement uses velocity and acceleration:

```json
{
  "input": "j.66",
  "name": "Air Dash",
  "type": "movement",
  "total": 20,
  "preconditions": [
    { "type": "airborne" }
  ],
  "movement": {
    "airborne": true,
    "velocity": { "x": 12.0, "y": -2.0 },
    "acceleration": { "x": -0.3, "y": 0.5 },
    "frames": [4, 16]
  }
}
```

See `docs/movement-reference.md` for detailed movement field documentation.

## Cancel Tables for Movement

Movement states need cancel rules to transition between each other and into attacks.

### Basic Movement Cancel Table

```json
{
  "chains": {
    "0_idle": ["0_walk_forward", "0_walk_backward", "1_crouch", "66", "44"],
    "0_walk_forward": ["0_idle", "0_walk_backward", "1_crouch", "66"],
    "0_walk_backward": ["0_idle", "0_walk_forward", "1_crouch", "44"],
    "1_crouch": ["0_idle", "0_walk_forward", "0_walk_backward"]
  },
  "tag_rules": [
    {
      "from_tag": "system",
      "to_tag": "normal",
      "condition": "on_complete"
    },
    {
      "from_tag": "normal",
      "to_tag": "special",
      "condition": "on_hit_or_block"
    }
  ]
}
```

### Tag-Based Rules

Instead of explicit pairs, use tags for common patterns:

- Tag movement states as `system` or `movement`
- Tag attack types as `normal`, `special`, `super`
- Define rules like "normals cancel into specials on hit"

## Using Global States

For states shared across multiple characters, use global states:

1. Create the state in `<project>/globals/states/<name>.json`
2. Have characters opt in via `globals.json`:

```json
{
  "includes": [
    { "state": "idle", "as": "0_idle", "override": { "animation": "char_idle" } },
    { "state": "burst", "as": "burst" }
  ]
}
```

See `docs/global-states.md` for details.

## Complete Example: Minimal Playable Character

A minimal character needs at minimum:

```
characters/minimal/
  character.json
  cancel_table.json
  states/
    0_idle.json       # Standing neutral
    1_crouch.json     # Crouching neutral
    66.json           # Forward dash
    44.json           # Back dash
    5L.json           # Standing light attack
    2L.json           # Crouching light attack
```

The `test_char` in this repository demonstrates a more complete character with special moves, supers, and advanced features.

## Validation Tips

- Run `npm run check` to validate TypeScript
- The State Editor shows validation warnings for missing fields
- The MCP server's `validate_character` tool checks for common issues
- Export with `json-blob` adapter to see the fully resolved character data
