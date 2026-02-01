# Movement Reference

Technical reference for defining and applying movement in Framesmith states.

## Movement Field Schema

The `movement` field on a state defines how the character moves during that state:

```typescript
interface Movement {
  // Simple mode (distance-based)
  distance?: number;           // Units to travel
  direction?: "forward" | "backward";
  curve?: "linear" | "ease-in" | "ease-out" | "ease-in-out";

  // Complex mode (physics-based)
  velocity?: { x: number; y: number };
  acceleration?: { x: number; y: number };

  // Shared
  frames?: [number, number];   // When movement applies
  airborne?: boolean;          // Is this air movement?
}
```

Use **simple mode** for ground movement with predictable distance.
Use **complex mode** for air movement or physics-driven motion.

## Simple Movement (Distance-Based)

Distance-based movement travels a fixed distance over the state's duration.

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `distance` | number | Total units to travel |
| `direction` | string | `"forward"` or `"backward"` (relative to facing) |
| `curve` | string | Easing function for velocity |

### How It Works

The total distance is distributed across the active frames:

1. If `frames` is set, movement only applies during `[start, end]`
2. Otherwise, movement applies for the full state duration
3. Per-frame delta = `distance / frame_count`, modified by curve

### Curve Options

| Curve | Effect |
|-------|--------|
| `linear` (default) | Constant velocity throughout |
| `ease-in` | Starts slow, accelerates |
| `ease-out` | Starts fast, decelerates |
| `ease-in-out` | Slow at start and end |

### Example: Forward Dash

```json
{
  "input": "66",
  "name": "Forward Dash",
  "type": "movement",
  "total": 18,
  "movement": {
    "distance": 80,
    "direction": "forward",
    "curve": "ease-out"
  }
}
```

This moves 80 units forward over 18 frames, decelerating.

### Example: Lunge Attack

```json
{
  "input": "6M",
  "name": "Shoulder Tackle",
  "type": "command_normal",
  "startup": 12,
  "active": 4,
  "recovery": 18,
  "movement": {
    "distance": 40,
    "direction": "forward",
    "frames": [8, 16]
  }
}
```

This moves 40 units forward only during frames 8-16 (during the active portion).

## Complex Movement (Physics-Based)

Physics-based movement uses velocity and acceleration for dynamic motion.

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `velocity` | `{x, y}` | Initial velocity (units/frame) |
| `acceleration` | `{x, y}` | Velocity change per frame |
| `frames` | `[start, end]` | Frame range for movement |
| `airborne` | boolean | Marks this as air movement |

### How It Works

Each frame during the active range:

```
position.x += velocity.x
position.y += velocity.y
velocity.x += acceleration.x
velocity.y += acceleration.y
```

Positive X = forward (relative to facing)
Negative Y = upward (screen coordinates)

### Example: Air Dash

```json
{
  "input": "j.66",
  "name": "Air Dash",
  "type": "movement",
  "total": 20,
  "preconditions": [{ "type": "airborne" }],
  "movement": {
    "airborne": true,
    "velocity": { "x": 12.0, "y": -2.0 },
    "acceleration": { "x": -0.3, "y": 0.5 },
    "frames": [4, 16]
  }
}
```

This:
- Starts moving forward at 12 units/frame, slightly upward
- Decelerates horizontally by 0.3/frame
- Falls due to 0.5/frame downward acceleration
- Only applies during frames 4-16

### Example: Jump Arc

```json
{
  "input": "j.8",
  "name": "Neutral Jump",
  "type": "movement",
  "total": 45,
  "movement": {
    "airborne": true,
    "velocity": { "x": 0, "y": -20.0 },
    "acceleration": { "x": 0, "y": 0.9 }
  }
}
```

This creates a parabolic arc: fast upward initially, slowing, then falling.

## Frame Ranges

The `frames` field limits when movement applies:

```json
"frames": [8, 16]
```

- Movement starts at frame 8
- Movement ends at frame 16
- Frames outside this range have no movement

This is useful for:
- Lunging attacks that only move during active frames
- Dashes with startup and recovery that don't move
- Air dashes with brief momentum windows

## Applying Movement in Your Engine

### Per-Frame Processing

```typescript
function applyMovement(state: State, currentFrame: number, position: Vec2, facing: number) {
  const movement = state.movement;
  if (!movement) return;

  const [startFrame, endFrame] = movement.frames ?? [0, state.total ?? getTotalFrames(state)];
  if (currentFrame < startFrame || currentFrame > endFrame) return;

  if (movement.distance !== undefined) {
    // Simple mode
    const frameCount = endFrame - startFrame + 1;
    const baseSpeed = movement.distance / frameCount;
    const t = (currentFrame - startFrame) / frameCount;
    const easedSpeed = applyEasing(baseSpeed, t, movement.curve ?? "linear");
    const direction = movement.direction === "backward" ? -1 : 1;
    position.x += easedSpeed * direction * facing;
  } else if (movement.velocity) {
    // Complex mode - track velocity state externally
    position.x += velocity.x * facing;
    position.y += velocity.y;
    velocity.x += (movement.acceleration?.x ?? 0);
    velocity.y += (movement.acceleration?.y ?? 0);
  }
}
```

### Coordinate System

- `x`: Horizontal, positive = forward relative to facing
- `y`: Vertical, negative = up (standard screen coordinates)
- `facing`: 1 for right, -1 for left

### Handling Collisions

Your engine must handle:
- Stage boundaries (walls, floor)
- Character collision (push boxes)
- Corner interactions

Movement data defines intent; collision resolution is engine-specific.

### Rollback Considerations

For rollback netcode:
- Store velocity as part of character state
- Movement fields are deterministic (same input = same output)
- Re-simulate from stored state on rollback

## Worked Examples

| Move Type | Schema | Per-Frame Effect |
|-----------|--------|------------------|
| Walk | `distance: 4, direction: forward` | +4 units/frame forward |
| Dash | `distance: 80, curve: ease-out` | Variable, ~8 -> 2 units/frame |
| Jump | `velocity: {y: -20}, accel: {y: 0.9}` | Arc trajectory |
| Air Dash | `velocity: {x: 12, y: -2}, accel: {x: -0.3, y: 0.5}` | Diagonal with decel |
| Lunge | `distance: 40, frames: [8, 16]` | +5 units/frame during active |

## Current Limitations

1. **FSPK Export**: Movement is tagged (state type 4) but values are not serialized in the v1 binary format. Use `json-blob` for full movement data.

2. **UI Preview**: The State Editor shows movement fields but does not visualize the motion path.

3. **Runtime Simulation**: Framesmith exports movement data; your engine is responsible for applying it.

4. **Easing Curves**: Only standard easing functions are supported. Custom curves require engine-side implementation.

## See Also

- `docs/data-formats.md` - Full state schema
- `docs/character-authoring-guide.md` - Creating movement states
- `docs/runtime-guide.md` - Integrating with game engines
