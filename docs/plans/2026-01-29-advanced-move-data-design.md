# Advanced Move Data Schema Design

**Status:** Draft
**Date:** 2026-01-29
**Goal:** Extend Framesmith's data model to support the full range of fighting game mechanics

## Overview

This document defines schema extensions to support advanced fighting game features including multi-hit attacks, rekka chains, stances, charge moves, resource systems, status effects, and conditional logic.

The core design principle: **explicit over implicit**. Each move is a separate file. Complex mechanics are expressed through composition of simple, well-defined fields rather than variant systems or inheritance.

---

## File Structure

```
characters/
  glitch/
    character.json
    hurtboxes.json
    cancel_table.json
    assets.json
    moves/
      # Normal moves
      5L.json
      5M.json
      5H.json

      # Movement (moves without hitboxes)
      66.json                    # Forward dash
      44.json                    # Back dash
      j.66.json                  # Air dash

      # Specials
      236P.json                  # Fireball
      236PP.json                 # EX Fireball (meter cost)

      # Rekka chains (flat with parent field)
      236K.json                  # Rekka starter
      236K~K.json                # Rekka follow-up 1
      236K~K~K.json              # Rekka follow-up 2
      236K~P.json                # Rekka branch (overhead)

      # Tekken-style strings
      1.json
      1,1.json
      1,1,2.json
      1,2.json
      1,2,4.json

      # Stance moves (prefixed for clarity)
      214S.json                  # Stance activation
      puppet.5L.json             # Stance-specific move
      puppet.5M.json
      puppet.5H.json
      puppet.236P.json

      # Install-enhanced moves
      install.236P.json          # Install-only fireball

      # Supers
      236236P.json               # Level 1 super
      214214K.json               # Level 3 super
```

---

## Move Schema

### Complete Move Definition

```json
{
  "input": "236P",
  "name": "Fireball",
  "type": "special",

  "startup": 12,
  "recovery": 18,
  "total": 45,

  "trigger": "press",
  "parent": null,

  "preconditions": [],
  "costs": [],

  "hits": [
    {
      "frames": [12, 14],
      "damage": 80,
      "chip_damage": 8,
      "hitstun": 20,
      "blockstun": 14,
      "hitstop": 8,
      "guard": "mid",
      "hitboxes": [],
      "cancels": []
    }
  ],

  "hurtboxes": [],

  "movement": null,
  "super_freeze": null,

  "on_use": {},
  "on_hit": {},
  "on_block": {},

  "meter_gain": { "hit": 10, "whiff": 3 },
  "pushback": { "hit": 20, "block": 30 },

  "animation": "fireball"
}
```

---

## Field Reference

### Identity

| Field | Type | Description |
|-------|------|-------------|
| `input` | string | Numpad notation + button (e.g., "236P", "j.H", "66") |
| `name` | string | Human-readable name |
| `type` | enum | `normal`, `command_normal`, `special`, `super`, `movement`, `throw` |

### Timing

| Field | Type | Description |
|-------|------|-------------|
| `startup` | u8 | Frames before first active frame |
| `recovery` | u8 | Frames after last active frame |
| `total` | u8 | Total move duration (computed: last hit end frame + recovery) |

### Input Modifiers

| Field | Type | Description |
|-------|------|-------------|
| `trigger` | enum | `press` (default), `release` (negative edge), `hold` |
| `parent` | string? | Move ID this follows from (for rekkas/strings) |

---

## Preconditions

Preconditions are requirements that must be met before a move can be performed.

```json
"preconditions": [
  { "type": "meter", "min": 50 },
  { "type": "charge", "direction": "4", "min_frames": 45 },
  { "type": "state", "in": "puppet" },
  { "type": "grounded" },
  { "type": "airborne" },
  { "type": "health", "max_percent": 30 },
  { "type": "entity_count", "tag": "fireball", "max": 0 },
  { "type": "resource", "name": "install_timer", "min": 1 },
  { "type": "combo_count", "min": 5 },
  { "type": "opponent_state", "in": ["hitstun", "knockdown"] },
  { "type": "distance", "max": 50 }
]
```

### Precondition Types

| Type | Fields | Example Use |
|------|--------|-------------|
| `meter` | `min`, `max` | EX moves, supers |
| `charge` | `direction`, `min_frames` | Sonic Boom, Flash Kick |
| `state` | `in` | Stance-only moves |
| `grounded` | - | Ground-only moves |
| `airborne` | - | Air-only moves |
| `health` | `min_percent`, `max_percent` | Desperation moves |
| `entity_count` | `tag`, `min`, `max` | Limit projectiles on screen |
| `resource` | `name`, `min`, `max` | Custom resources (install timer, stacks) |
| `combo_count` | `min`, `max` | Combo-only moves |
| `opponent_state` | `in` (array) | Pursuit attacks, OTG moves |
| `distance` | `min`, `max` | Close/far proximity moves |

---

## Costs

Resources consumed when the move is performed.

```json
"costs": [
  { "type": "meter", "amount": 50 },
  { "type": "health", "amount": 10 },
  { "type": "resource", "name": "burst", "amount": 100 },
  { "type": "resource", "name": "install_timer", "amount": 30 }
]
```

---

## Hits Array

Every move uses a `hits` array, even single-hit moves. Each hit defines its own timing, damage, and cancel options.

```json
"hits": [
  {
    "frames": [8, 10],
    "damage": 30,
    "chip_damage": 3,
    "hitstun": 15,
    "blockstun": 10,
    "hitstop": 6,
    "guard": "mid",
    "hitboxes": [
      { "type": "aabb", "x": 10, "y": -40, "w": 30, "h": 20 }
    ],
    "cancels": ["5M", "2M", "236P"]
  },
  {
    "frames": [18, 22],
    "damage": 50,
    "chip_damage": 5,
    "hitstun": 20,
    "blockstun": 14,
    "hitstop": 8,
    "guard": "mid",
    "hitboxes": [
      { "type": "circle", "x": 50, "y": -35, "r": 18 }
    ],
    "cancels": ["236PP"]
  }
]
```

### Hit Fields

| Field | Type | Description |
|-------|------|-------------|
| `frames` | [u8, u8] | Start and end frame (inclusive) |
| `damage` | u16 | Damage on hit |
| `chip_damage` | u16 | Damage on block (optional, default 0) |
| `hitstun` | u8 | Frames opponent is in hitstun |
| `blockstun` | u8 | Frames opponent is in blockstun |
| `hitstop` | u8 | Frames both players freeze |
| `guard` | enum | `high`, `mid`, `low`, `unblockable` |
| `hitboxes` | array | Hitbox definitions for this hit |
| `cancels` | array | Move IDs this hit can cancel into |

---

## Hitbox Shapes

Four hitbox shapes are supported:

### AABB (Axis-Aligned Bounding Box)
```json
{ "type": "aabb", "x": 10, "y": -40, "w": 30, "h": 20 }
```
- Fastest collision detection
- Cannot rotate
- Use for most attacks

### Rect (Rotatable Rectangle)
```json
{ "type": "rect", "x": 10, "y": -40, "w": 50, "h": 15, "angle": 30 }
```
- `angle` in degrees, clockwise from horizontal
- Use for diagonal sword swings, angled attacks

### Circle
```json
{ "type": "circle", "x": 50, "y": -30, "r": 20 }
```
- Use for projectiles, spherical effects, explosions

### Capsule
```json
{ "type": "capsule", "x1": 0, "y1": -40, "x2": 60, "y2": -30, "r": 10 }
```
- Line segment with radius
- Use for extended limbs, sword slashes, sweeping motions

---

## Hurtboxes

Per-frame vulnerability boxes. Same shape types as hitboxes.

```json
"hurtboxes": [
  {
    "frames": [0, 7],
    "boxes": [
      { "type": "aabb", "x": -15, "y": -70, "w": 30, "h": 70 }
    ]
  },
  {
    "frames": [8, 20],
    "boxes": [
      { "type": "aabb", "x": -10, "y": -60, "w": 25, "h": 60 },
      { "type": "capsule", "x1": 10, "y1": -40, "x2": 50, "y2": -35, "r": 8 }
    ],
    "flags": ["strike_invuln"]
  }
]
```

### Hurtbox Flags

| Flag | Description |
|------|-------------|
| `strike_invuln` | Cannot be hit by strikes |
| `throw_invuln` | Cannot be thrown |
| `projectile_invuln` | Cannot be hit by projectiles |
| `full_invuln` | Complete invincibility |
| `armor` | Takes hit but doesn't flinch (may need armor_hits count) |

---

## Movement

For dashes, jumps, and moves with displacement.

```json
"movement": {
  "distance": 80,
  "direction": "forward",
  "curve": "ease-out",
  "airborne": false
}
```

Or for more complex movement:

```json
"movement": {
  "velocity": { "x": 15, "y": 0 },
  "acceleration": { "x": -0.5, "y": 0 },
  "frames": [3, 15]
}
```

---

## Super Freeze

Cinematic pause effect for supers.

```json
"super_freeze": {
  "frames": 45,
  "zoom": 1.5,
  "darken": 0.7,
  "flash": true
}
```

---

## Effects

### On Use (when move starts)

```json
"on_use": {
  "enters_state": {
    "name": "install",
    "duration": 600,
    "persistent": true
  },
  "spawn_entity": {
    "type": "projectile",
    "tag": "fireball",
    "data": "fireball_projectile"
  },
  "gain_meter": 5
}
```

### On Hit (when move connects)

```json
"on_hit": {
  "gain_meter": 25,
  "heal": 10,
  "status": [
    { "type": "poison", "damage_per_frame": 1, "duration": 120 },
    { "type": "stun", "duration": 90 },
    { "type": "slow", "multiplier": 0.5, "duration": 60 }
  ],
  "knockback": {
    "type": "launch",
    "x": 30,
    "y": 80
  },
  "wall_bounce": true,
  "ground_bounce": true
}
```

### On Block

```json
"on_block": {
  "gain_meter": 5,
  "pushback": 40
}
```

### Status Effect Types

| Type | Fields | Description |
|------|--------|-------------|
| `poison` | `damage_per_frame`, `duration` | Damage over time |
| `burn` | `damage_per_frame`, `duration` | Damage over time (fire variant) |
| `stun` | `duration` | Cannot act |
| `slow` | `multiplier`, `duration` | Reduced speed |
| `weaken` | `damage_multiplier`, `duration` | Take more damage |
| `seal` | `move_types`, `duration` | Cannot use certain move types |

---

## Rekka / Follow-up Chains

Use `parent` field to define chains. Flat file structure.

### Example: 3-Hit Rekka

```json
// 236K.json - Starter
{
  "input": "236K",
  "name": "Rekka 1",
  "startup": 14,
  "recovery": 12,
  "hits": [
    {
      "frames": [14, 17],
      "damage": 60,
      "hitstun": 18,
      "blockstun": 12,
      "guard": "mid",
      "hitboxes": [{ "type": "aabb", "x": 10, "y": -50, "w": 40, "h": 30 }],
      "cancels": []
    }
  ],
  "on_use": {
    "enters_state": {
      "name": "rekka",
      "duration": 25,
      "persistent": false
    }
  }
}
```

```json
// 236K~K.json - Follow-up 1
{
  "input": "K",
  "name": "Rekka 2",
  "parent": "236K",
  "preconditions": [
    { "type": "state", "in": "rekka" }
  ],
  "startup": 10,
  "recovery": 14,
  "hits": [
    {
      "frames": [10, 14],
      "damage": 70,
      "hitstun": 20,
      "blockstun": 14,
      "guard": "mid",
      "hitboxes": [{ "type": "aabb", "x": 15, "y": -55, "w": 45, "h": 35 }],
      "cancels": []
    }
  ],
  "on_use": {
    "enters_state": {
      "name": "rekka2",
      "duration": 25,
      "persistent": false
    }
  }
}
```

```json
// 236K~K~K.json - Follow-up 2 (ender)
{
  "input": "K",
  "name": "Rekka 3",
  "parent": "236K~K",
  "preconditions": [
    { "type": "state", "in": "rekka2" }
  ],
  "startup": 12,
  "recovery": 22,
  "hits": [
    {
      "frames": [12, 18],
      "damage": 100,
      "hitstun": 30,
      "blockstun": 18,
      "guard": "mid",
      "hitboxes": [{ "type": "capsule", "x1": 0, "y1": -50, "x2": 60, "y2": -40, "r": 15 }],
      "cancels": ["236236P"]
    }
  ]
}
```

---

## Stances

Stances are persistent states with their own move lists.

### Stance Activation

```json
// 214S.json
{
  "input": "214S",
  "name": "Enter Puppet Stance",
  "type": "special",
  "startup": 8,
  "recovery": 0,
  "total": 8,
  "hits": [],
  "on_use": {
    "enters_state": {
      "name": "puppet",
      "duration": null,
      "persistent": true,
      "exit_input": "214S"
    }
  }
}
```

### Stance-Specific Moves

```json
// puppet.5L.json
{
  "input": "5L",
  "name": "Puppet Jab",
  "preconditions": [
    { "type": "state", "in": "puppet" }
  ],
  "startup": 5,
  "recovery": 8,
  "hits": [
    {
      "frames": [5, 7],
      "damage": 25,
      "hitstun": 12,
      "blockstun": 8,
      "guard": "mid",
      "hitboxes": [{ "type": "aabb", "x": 10, "y": -45, "w": 25, "h": 15 }],
      "cancels": ["puppet.5M", "puppet.236P"]
    }
  ]
}
```

---

## Charge Moves

Use charge precondition.

```json
// 6P.json (Sonic Boom style)
{
  "input": "6P",
  "name": "Sonic Boom",
  "preconditions": [
    { "type": "charge", "direction": "4", "min_frames": 45 }
  ],
  "startup": 13,
  "recovery": 30,
  "hits": [
    {
      "frames": [13, 13],
      "damage": 70,
      "hitstun": 18,
      "blockstun": 12,
      "guard": "mid",
      "hitboxes": []
    }
  ],
  "on_use": {
    "spawn_entity": {
      "type": "projectile",
      "tag": "sonic_boom",
      "data": "sonic_boom_projectile",
      "position": { "x": 30, "y": -40 }
    }
  }
}
```

---

## EX / Enhanced Moves

Separate files with meter requirements.

```json
// 236PP.json
{
  "input": "236PP",
  "name": "EX Fireball",
  "preconditions": [
    { "type": "meter", "min": 25 }
  ],
  "costs": [
    { "type": "meter", "amount": 25 }
  ],
  "startup": 10,
  "recovery": 15,
  "hits": [
    { "frames": [10, 12], "damage": 40, "hitstun": 15, "blockstun": 10, "guard": "mid", "hitboxes": [], "cancels": [] },
    { "frames": [16, 18], "damage": 40, "hitstun": 15, "blockstun": 10, "guard": "mid", "hitboxes": [], "cancels": [] },
    { "frames": [22, 24], "damage": 50, "hitstun": 25, "blockstun": 15, "guard": "mid", "hitboxes": [], "cancels": [] }
  ],
  "on_use": {
    "spawn_entity": {
      "type": "projectile",
      "tag": "fireball",
      "data": "ex_fireball_projectile"
    }
  }
}
```

---

## Install / Powerup States

```json
// 236236K.json - Install activation
{
  "input": "236236K",
  "name": "Power Install",
  "type": "super",
  "preconditions": [
    { "type": "meter", "min": 100 }
  ],
  "costs": [
    { "type": "meter", "amount": 100 }
  ],
  "startup": 5,
  "recovery": 20,
  "hits": [],
  "super_freeze": {
    "frames": 60,
    "zoom": 1.3,
    "darken": 0.6
  },
  "on_use": {
    "enters_state": {
      "name": "install",
      "duration": 600,
      "persistent": true
    }
  }
}
```

```json
// install.236P.json - Install-only move
{
  "input": "236P",
  "name": "Empowered Fireball",
  "preconditions": [
    { "type": "state", "in": "install" }
  ],
  "startup": 8,
  "recovery": 12,
  "hits": [
    {
      "frames": [8, 10],
      "damage": 120,
      "hitstun": 25,
      "blockstun": 18,
      "guard": "mid",
      "hitboxes": [],
      "cancels": []
    }
  ],
  "on_hit": {
    "status": [
      { "type": "burn", "damage_per_frame": 2, "duration": 60 }
    ]
  }
}
```

---

## Movement Moves

Dashes, jumps, and other movement options.

```json
// 66.json - Forward dash
{
  "input": "66",
  "name": "Forward Dash",
  "type": "movement",
  "startup": 3,
  "recovery": 6,
  "total": 18,
  "hits": [],
  "movement": {
    "distance": 80,
    "direction": "forward",
    "curve": "ease-out"
  },
  "hurtboxes": [
    {
      "frames": [0, 18],
      "boxes": [{ "type": "aabb", "x": -12, "y": -65, "w": 24, "h": 65 }]
    }
  ]
}
```

```json
// 44.json - Back dash (with invuln)
{
  "input": "44",
  "name": "Back Dash",
  "type": "movement",
  "startup": 4,
  "recovery": 12,
  "total": 24,
  "hits": [],
  "movement": {
    "distance": 60,
    "direction": "backward",
    "curve": "ease-out"
  },
  "hurtboxes": [
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

---

## Negative Edge

Use `trigger: "release"` for moves that activate on button release.

```json
{
  "input": "P",
  "name": "Release Slash",
  "trigger": "release",
  "preconditions": [
    { "type": "state", "in": "charge_stance" }
  ],
  "startup": 3,
  "recovery": 20,
  "hits": [...]
}
```

---

## Summary of Changes from v1 Schema

| Feature | v1 | v2 |
|---------|----|----|
| Hits | Flat fields (`damage`, `startup`, etc.) | `hits` array |
| Hitbox shapes | Rectangle only (`x, y, w, h`) | `aabb`, `rect`, `circle`, `capsule` |
| Cancels | In `cancel_table.json` only | Per-hit `cancels` array + cancel table |
| Move chains | Not supported | `parent` field |
| Stances | Not supported | `enters_state` + `preconditions` |
| Charge moves | Not supported | `preconditions: [{ type: "charge" }]` |
| Meter cost | Not supported | `costs` array |
| Status effects | Not supported | `on_hit.status` array |
| Super freeze | Not supported | `super_freeze` object |
| Invincibility | Not supported | `hurtbox.flags` array |
| Movement | Not supported | `movement` object |
| Negative edge | Not supported | `trigger: "release"` |

---

## Migration Path

1. Existing moves keep working - new fields are optional
2. Single-hit moves wrap existing data in `hits[0]`
3. Hitboxes default to `type: "aabb"` if unspecified
4. `trigger` defaults to `"press"`
5. `parent`, `preconditions`, `costs` default to null/empty

---

## Open Questions

1. **Projectile data format** - Should projectiles be defined inline or as separate entity files?
2. **Animation blending** - How to handle transitions between moves?
3. **Sound/VFX references** - Add to hits array or separate?
4. **Combo scaling** - Define in character.json or per-move?
5. **Input priority** - When multiple moves match, which wins?

---

## Next Steps

1. Update TypeScript types to match new schema
2. Migrate existing test characters
3. Update editor UI for new fields
4. Add validation for new precondition types
5. Document entity/projectile format
