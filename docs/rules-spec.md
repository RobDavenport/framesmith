# Framesmith Rules Specification

This document is the **single source of truth** for Framesmith's validation rules system. All other documentation (MCP resources, in-app help, JSON schemas) derives from or complements this specification.

## Overview

Framesmith supports a flexible rules system that lets you:
- **Apply** default values to moves based on matching criteria
- **Validate** moves to enforce constraints with configurable severity

Rules are defined in JSON files named `framesmith.rules.json`. They can exist at two levels:
1. **Project-level**: At the project root (applies to all characters)
2. **Character-level**: Inside a character directory (overrides project rules)

## Rules File Format

```json
{
  "version": 1,
  "apply": [...],
  "validate": [...]
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `version` | `number` | Yes | Schema version. Currently must be `1`. |
| `apply` | `ApplyRule[]` | No | Rules that set default values on moves. |
| `validate` | `ValidateRule[]` | No | Rules that enforce constraints on moves. |

## Apply Rules

Apply rules set default values on moves that match certain criteria. They only fill in values that are unset (null, empty, or zero).

```json
{
  "match": { "type": "normal" },
  "set": {
    "hitstop": 8,
    "pushback": { "hit": 2, "block": 2 }
  }
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `match` | `MatchSpec` | Yes | Criteria for which moves this rule applies to. |
| `set` | `object` | Yes | Key-value pairs to set on matching moves. Nested paths supported. |

### How Apply Rules Work

1. Rules are evaluated in order (project rules first, then character rules)
2. Character rules with the same `match` spec **replace** project rules
3. For each matching rule, only **unset** fields are filled in
4. Later rules can override earlier defaults (if the field is still unset)

## Validate Rules

Validate rules enforce constraints on moves, producing errors or warnings.

```json
{
  "match": { "type": "special" },
  "require": {
    "startup": { "min": 1 },
    "animation": { "exists": true }
  },
  "severity": "error",
  "message": "Special moves must have startup and animation defined"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `match` | `MatchSpec` | Yes | Criteria for which moves this rule applies to. |
| `require` | `object` | Yes | Constraint definitions (see Constraint Types). |
| `severity` | `"error"` \| `"warning"` | Yes | How to report violations. |
| `message` | `string` | No | Custom message for violations. |

### Validation Order

1. Apply rules are processed first (to fill defaults)
2. Built-in validation runs (always enforced)
3. Custom validate rules run against the resolved move data
4. Character rules with the same `match` spec **replace** project rules

---

## Match Criteria

The `match` object determines which moves a rule applies to. All specified fields must match (AND logic). Within a single field, multiple values use OR logic.

```json
{
  "type": "normal",
  "button": ["L", "M", "H"],
  "guard": "mid",
  "tags": ["starter"],
  "input": "5*"
}
```

### Match Fields

| Field | Type | Description |
|-------|------|-------------|
| `type` | `string` \| `string[]` | Move type: `normal`, `command_normal`, `special`, `super`, `movement`, `throw` |
| `button` | `string` \| `string[]` | Button extracted from input (e.g., `"236P"` -> `"P"`) |
| `guard` | `string` \| `string[]` | Guard type: `high`, `mid`, `low`, `unblockable` |
| `tags` | `string[]` | Tags that must ALL be present on the move (AND logic) |
| `input` | `string` \| `string[]` | Input notation with glob pattern support |

### Glob Patterns for Input

The `input` field supports glob patterns:
- `*` matches any sequence of characters (including empty)
- `?` matches exactly one character

| Pattern | Matches | Doesn't Match |
|---------|---------|---------------|
| `5*` | `5L`, `5M`, `5H` | `2L`, `j.H` |
| `236*` | `236P`, `236K`, `236236P` | `214P` |
| `*P` | `5P`, `236P`, `623P` | `5K`, `236K` |
| `5?` | `5L`, `5M`, `5H` | `5LL`, `j.H` |
| `[*]*` | `[4]6P`, `[2]8K` | `236P` |

### Button Extraction

The `button` field matches the trailing alphabetic characters of the input:
- `5L` -> button is `L`
- `236P` -> button is `P`
- `632146PP` -> button is `PP`
- `j.H` -> button is `H`

### Match Examples

```json
// All normals
{ "type": "normal" }

// Light attacks (any type)
{ "button": "L" }

// Standing normals only
{ "type": "normal", "input": "5*" }

// Specials or supers
{ "type": ["special", "super"] }

// Quarter-circle moves
{ "input": ["236*", "214*"] }

// Moves tagged as both "starter" AND "reversal"
{ "tags": ["starter", "reversal"] }

// Empty match = all moves
{ }
```

---

## Constraint Types

Constraints are used in validate rules to check move properties.

### `exists`

Checks whether a field is set (non-null, non-empty, non-zero).

```json
{ "animation": { "exists": true } }
{ "meter_gain": { "exists": false } }
```

### `min` / `max`

Numeric bounds (inclusive).

```json
{ "startup": { "min": 1, "max": 30 } }
{ "damage": { "min": 1 } }
{ "recovery": { "max": 60 } }
```

### `equals`

Exact value match.

```json
{ "guard": { "equals": "mid" } }
{ "hitstop": { "equals": 8 } }
```

### `in`

Value must be one of the specified options.

```json
{ "guard": { "in": ["mid", "low"] } }
{ "type": { "in": ["special", "super"] } }
```

### Nested Constraints

Constraints can target nested fields using object nesting:

```json
{
  "require": {
    "pushback": {
      "hit": { "min": 1 },
      "block": { "min": 1 }
    },
    "meter_gain": {
      "whiff": { "exists": true }
    }
  }
}
```

---

## Settable Fields

Apply rules can set any field on a move. Common fields:

### Core Frame Data
- `startup` - Startup frames
- `active` - Active frames
- `recovery` - Recovery frames
- `total` - Total frames (optional, computed if not set)

### Damage & Stun
- `damage` - Base damage
- `hitstun` - Hitstun frames
- `blockstun` - Blockstun frames
- `hitstop` - Hitstop frames

### Properties
- `guard` - Guard type: `"high"`, `"mid"`, `"low"`, `"unblockable"`
- `type` - Move type: `"normal"`, `"command_normal"`, `"special"`, `"super"`, `"movement"`, `"throw"`
- `tags` - Array of string tags

### Nested Objects
- `pushback.hit` - Pushback on hit
- `pushback.block` - Pushback on block
- `meter_gain.hit` - Meter gained on hit
- `meter_gain.whiff` - Meter gained on whiff

---

## Built-in Validation

These validations always run and cannot be disabled. They ensure move data is structurally valid.

### Frame Data
| Field | Constraint | Error Message |
|-------|------------|---------------|
| `startup` | must be >= 1 | "startup must be at least 1 frame" |
| `active` | must be >= 1 | "active must be at least 1 frame" |
| `input` | must be non-empty | "input cannot be empty" |

### Hitboxes (Legacy)
| Field | Constraint | Error Message |
|-------|------------|---------------|
| `hitboxes[i].frames` | start <= end | "start frame cannot be after end frame" |
| `hitboxes[i].frames` | end <= total | "end frame {n} exceeds total frames {total}" |

### Hits (v2 Schema)
| Field | Constraint | Error Message |
|-------|------------|---------------|
| `hits[i].frames` | start <= end | "start frame cannot be after end frame" |
| `hits[i].hitboxes[j].w` | must be > 0 | "width must be greater than 0" |
| `hits[i].hitboxes[j].h` | must be > 0 | "height must be greater than 0" |
| `hits[i].hitboxes[j].r` | must be > 0 | "radius must be greater than 0" |

### Preconditions
| Field | Constraint | Error Message |
|-------|------------|---------------|
| `preconditions[i]` (Meter) | min <= max | "meter min cannot be greater than max" |
| `preconditions[i]` (Charge) | min_frames > 0 | "charge min_frames must be greater than 0" |
| `preconditions[i]` (Health) | percent <= 100 | "health min/max_percent cannot exceed 100" |
| `preconditions[i]` (Health) | min <= max | "health min_percent cannot be greater than max_percent" |
| Various range types | min <= max | "{type} min cannot be greater than max" |

### Costs
| Field | Constraint | Error Message |
|-------|------------|---------------|
| `costs[i].amount` | must be > 0 | "cost amount must be greater than 0" |

### Movement
| Field | Constraint | Error Message |
|-------|------------|---------------|
| `movement` | distance or velocity required | "movement must have either distance or velocity defined" |
| `movement.distance` | must be > 0 | "movement distance must be greater than 0" |

### Super Freeze
| Field | Constraint | Error Message |
|-------|------------|---------------|
| `super_freeze.frames` | must be > 0 | "super_freeze frames must be greater than 0" |
| `super_freeze.zoom` | must be > 0 | "super_freeze zoom must be greater than 0" |
| `super_freeze.darken` | 0.0 to 1.0 | "super_freeze darken must be between 0.0 and 1.0" |

### Status Effects
| Field | Constraint | Error Message |
|-------|------------|---------------|
| `*.duration` | must be > 0 | "duration must be greater than 0" |
| `*.damage_per_frame` | must be > 0 | "damage_per_frame must be greater than 0" |
| `slow.multiplier` | 0.0 to 1.0 | "slow multiplier must be between 0.0 and 1.0" |

### Advanced Hurtboxes
| Field | Constraint | Error Message |
|-------|------------|---------------|
| `advanced_hurtboxes[i].frames` | start <= end | "start frame cannot be after end frame" |
| `advanced_hurtboxes[i].boxes[j].*` | dimensions > 0 | Same as hitbox shape validation |

---

## Rule Hierarchy

Rules are merged from project and character levels:

```
Project: framesmith.rules.json
    │
    ├── apply rules (global defaults)
    └── validate rules (global constraints)
         │
         ▼
Character: characters/{id}/framesmith.rules.json
    │
    ├── apply rules (character-specific defaults)
    └── validate rules (character-specific constraints)
```

### Merge Behavior

1. Project rules are loaded first
2. Character rules are loaded second
3. If a character rule has the **same match spec** as a project rule, the character rule **replaces** it entirely
4. Different match specs coexist (both apply)

### Example

```json
// Project rules
{
  "apply": [
    { "match": { "type": "normal" }, "set": { "hitstop": 8 } }
  ]
}

// Character rules
{
  "apply": [
    { "match": { "type": "normal" }, "set": { "hitstop": 10 } }
  ]
}

// Result: normals get hitstop=10 (character replaces project)
```

---

## Complete Examples

### Project-Level Rules (Sensible Defaults)

```json
{
  "version": 1,
  "apply": [
    {
      "match": { "type": "normal" },
      "set": {
        "hitstop": 8,
        "pushback": { "hit": 2, "block": 2 },
        "meter_gain": { "hit": 5, "whiff": 2 }
      }
    },
    {
      "match": { "type": "special" },
      "set": {
        "hitstop": 10,
        "pushback": { "hit": 4, "block": 3 },
        "meter_gain": { "hit": 10, "whiff": 5 }
      }
    },
    {
      "match": { "button": "L" },
      "set": { "damage": 30 }
    },
    {
      "match": { "button": "M" },
      "set": { "damage": 50 }
    },
    {
      "match": { "button": "H" },
      "set": { "damage": 80 }
    }
  ],
  "validate": [
    {
      "match": {},
      "require": {
        "startup": { "min": 1 },
        "active": { "min": 1 },
        "animation": { "exists": true }
      },
      "severity": "warning",
      "message": "Move should have startup, active frames, and animation defined"
    },
    {
      "match": { "type": "super" },
      "require": {
        "super_freeze": { "exists": true }
      },
      "severity": "warning",
      "message": "Super moves should have super_freeze defined"
    }
  ]
}
```

### Character-Level Rules (Grappler with Slower Normals)

```json
{
  "version": 1,
  "apply": [
    {
      "match": { "type": "normal" },
      "set": {
        "hitstop": 10,
        "pushback": { "hit": 3, "block": 3 }
      }
    }
  ],
  "validate": [
    {
      "match": { "type": "throw" },
      "require": {
        "damage": { "min": 100 }
      },
      "severity": "warning",
      "message": "Grappler throws should deal at least 100 damage"
    }
  ]
}
```

---

## Using with JSON Schema

For IDE autocomplete, reference the JSON Schema in your rules file:

```json
{
  "$schema": "./schemas/rules.schema.json",
  "version": 1,
  "apply": [...],
  "validate": [...]
}
```

The schema is generated from the Rust type definitions and provides:
- Field name autocomplete
- Type validation
- Documentation on hover (in supported editors)
