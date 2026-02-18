# Framesmith Data Formats Reference

Complete schema reference for Framesmith's JSON data files. The canonical Rust types
live in `src-tauri/src/schema/mod.rs`.

## State File Schema

File: `characters/<id>/states/<input>.json`

### All Fields

```typescript
interface State {
  // -- Identity (required) --
  input: string;              // Notation input, must match filename
  name: string;               // Human-readable name

  // -- Categorization --
  tags?: string[];            // Lowercase tags (e.g. ["normal", "light"])
  type?: StateType;           // "normal" | "command_normal" | "special" | "super" | "movement" | "throw"

  // -- Frame Data (required for attacks) --
  startup: number;            // Frames before active (>= 1)
  active: number;             // Active frames (>= 1)
  recovery: number;           // Recovery frames after active
  total?: number;             // Override total duration (default: startup + active + recovery)

  // -- Damage & Stun --
  damage: number;             // Base damage
  hitstun: number;            // Stun frames on hit
  blockstun: number;          // Stun frames on block
  hitstop: number;            // Freeze frames on contact
  guard: Guard;               // "high" | "mid" | "low" | "unblockable"

  // -- Collision Boxes --
  hitboxes: FrameBox[];       // Damage-dealing regions
  hurtboxes: FrameBox[];      // Vulnerable regions
  pushboxes?: FrameBox[];     // Body collision regions

  // -- Knockback & Meter --
  pushback: { hit: number; block: number };
  meter_gain: { hit: number; whiff: number };

  // -- Animation --
  animation: string;          // Key into assets.json animations

  // -- Optional: Input --
  trigger?: "press" | "release" | "hold";
  parent?: string;            // For follow-ups/rekkas

  // -- Optional: Variant --
  base?: string;              // Inherit from another state (single-level)

  // -- Optional: Multi-Hit --
  hits?: Hit[];               // Per-hit data with shaped hitboxes

  // -- Optional: Conditions & Costs --
  preconditions?: Precondition[];
  costs?: Cost[];

  // -- Optional: Movement --
  movement?: Movement;

  // -- Optional: Super Freeze --
  super_freeze?: SuperFreeze;

  // -- Optional: Effects --
  on_use?: GameplayEffect;
  on_hit?: GameplayEffect;
  on_block?: GameplayEffect;
  notifies?: Notify[];

  // -- Optional: Advanced Hurtboxes --
  advanced_hurtboxes?: AdvancedHurtbox[];
}
```

### FrameBox

Used for hitboxes, hurtboxes, and pushboxes:

```json
{
  "frames": [7, 9],
  "box": { "x": 0, "y": -40, "w": 30, "h": 16 }
}
```

- `frames`: `[start_frame, end_frame]` inclusive range
- `box`: axis-aligned rectangle -- `x,y` = top-left offset from origin, `w,h` = dimensions
- Coordinates are relative to the character's position, Y-up convention

### Hit (Multi-Hit)

```json
{
  "frames": [7, 9],
  "damage": 15,
  "hitstun": 12,
  "blockstun": 8,
  "hitstop": 4,
  "guard": "mid",
  "hitboxes": [
    { "type": "aabb", "x": 0, "y": -40, "w": 30, "h": 16 }
  ]
}
```

Hitbox shapes: `aabb` (rectangle with x/y/w/h) or `circle` (with x/y/r).

### Precondition Types

```json
{ "type": "meter", "min": 100 }
{ "type": "resource", "name": "heat", "min": 1 }
{ "type": "charge", "min_frames": 30 }
{ "type": "airborne" }
{ "type": "grounded" }
{ "type": "health", "max_percent": 30 }
{ "type": "state", "required": "stance_a" }
```

### Cost Types

```json
{ "type": "meter", "amount": 100 }
{ "type": "resource", "name": "ammo", "amount": 1 }
{ "type": "health", "amount": 500 }
```

### Movement

Two modes -- distance-based (simple) and physics-based (complex):

```json
// Distance-based
{
  "distance": 80,
  "direction": "forward",
  "curve": "ease-out",
  "frames": [3, 15]
}

// Physics-based
{
  "airborne": true,
  "velocity": { "x": 12.0, "y": -2.0 },
  "acceleration": { "x": -0.3, "y": 0.5 },
  "frames": [4, 16]
}
```

### SuperFreeze

```json
{
  "frames": 30,
  "zoom": 1.5,
  "darken": 0.6,
  "target": "opponent"
}
```

### GameplayEffect

```json
{
  "resource_deltas": [
    { "name": "heat", "delta": 3 }
  ],
  "events": [
    { "id": "vfx.hit_sparks", "args": { "count": 3, "scale": 1.2 } }
  ]
}
```

Event `args` is a flat map of primitives (boolean, number, string).

### AdvancedHurtbox

```json
{
  "frames": [0, 8],
  "boxes": [
    { "type": "aabb", "x": -12, "y": -65, "w": 24, "h": 65 }
  ],
  "flags": ["full_invuln"]
}
```

Flags: `full_invuln`, `throw_invuln`, `projectile_invuln`, `low_invuln`, `high_invuln`.

## Character.json Schema

File: `characters/<id>/character.json`

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

### PropertyValue Types

Property values support three types:

| Type | JSON | Export (FSPK) |
|------|------|---------------|
| Number | `4.0`, `10000` | Q24.8 fixed-point |
| Boolean | `true`, `false` | 0 or 1 |
| String | `"rushdown"` | String table reference |

Common property keys: `health`, `walk_speed`, `back_walk_speed`, `jump_height`,
`jump_duration`, `dash_distance`, `dash_duration`, `archetype`.

Games can define arbitrary additional properties.

### Resources

```json
{
  "name": "heat",
  "start": 0,
  "max": 10
}
```

Referenced by states via `preconditions`, `costs`, and effect `resource_deltas`.

## Assets.json

File: `characters/<id>/assets.json` (optional, for UI preview)

```json
{
  "version": 1,
  "textures": {
    "atlas_key": "assets/textures/sprite.png"
  },
  "models": {
    "model_key": "assets/models/char.glb"
  },
  "animations": {
    "stand_light": {
      "mode": "sprite",
      "texture": "atlas_key",
      "frame_size": { "w": 256, "h": 256 },
      "frames": 18,
      "pivot": { "x": 128, "y": 192 }
    },
    "stand_light_3d": {
      "mode": "gltf",
      "model": "model_key",
      "clip": "stand_light",
      "fps": 60,
      "pivot": { "x": 0, "y": 0, "z": 0 }
    }
  }
}
```

Animation modes: `sprite` (grid spritesheet) or `gltf` (3D model playback).
Paths are relative to `characters/<id>/`.

## Variant Inheritance Rules

States with a `base` field inherit from another state:

```json
{
  "input": "j.5L",
  "base": "5L",
  "name": "Jumping Light",
  "damage": 25
}
```

Or use filename convention: `5L~aerial.json` creates a variant of `5L`.

**Rules:** Single-level only (no chaining), shallow merge override, `null` removes
inherited fields, `base` is stripped during export.

## Export Formats

- **json-blob**: fully resolved character + states as single JSON (includes all fields)
- **zx-fspack**: compact binary `.fspk` with fixed-size records and Q24.8 fixed-point properties
