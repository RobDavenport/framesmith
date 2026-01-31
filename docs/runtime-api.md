# Framesmith Runtime API Reference

**Status:** Active
**Last reviewed:** 2026-02-01

Complete API documentation for `framesmith-runtime`.

## Types

### CharacterState

Character simulation state. Designed for cheap cloning (rollback netcode) and `no_std` compatibility.

```rust
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct CharacterState {
    /// Current state index (0 = idle by convention).
    pub current_state: u16,

    /// Current frame within the state (0-indexed).
    pub frame: u8,

    /// Instance-specific duration override. 0 = use state's default total().
    pub instance_duration: u8,

    /// State connected with a hit (opens on-hit cancel windows).
    pub hit_confirmed: bool,

    /// State was blocked (opens on-block cancel windows).
    pub block_confirmed: bool,

    /// Resource pool values (meter, heat, ammo, etc.).
    pub resources: [u16; MAX_RESOURCES],
}
```

**Size:** 22 bytes

**Notes:**
- `Copy` trait enables zero-cost state saving/restoration for rollback
- `current_state` is an index into the character's state array
- `frame` saturates at 255 if not transitioned
- When `instance_duration > 0`, it overrides the state's default duration

---

### FrameInput

Input for a single frame of simulation.

```rust
#[derive(Clone, Copy, Debug, Default)]
pub struct FrameInput {
    /// State to transition to, if cancel is valid.
    /// `None` means continue current state.
    pub requested_state: Option<u16>,
}
```

**Notes:**
- Set to `None` to continue the current state
- Set to `Some(state_id)` to request a cancel/transition
- For action cancels, use `state_id = move_count + ACTION_*`

---

### FrameResult

Result of simulating one frame.

```rust
#[derive(Clone, Copy, Debug)]
pub struct FrameResult {
    /// The new character state after this frame.
    pub state: CharacterState,

    /// True if the move reached its final frame.
    /// Game decides whether to loop or transition.
    pub move_ended: bool,
}
```

**Notes:**
- `state` is the updated state after frame advancement and any transitions
- When `move_ended` is true, the game should transition to idle or another state
- The runtime does not auto-loop or auto-transition

---

### HitResult

Result of a hit interaction.

```rust
#[derive(Clone, Copy, Debug)]
pub struct HitResult {
    /// Move ID of the attacking move.
    pub attacker_move: u16,

    /// Index of the hit window that connected.
    pub window_index: u16,

    /// Damage value from the hit window.
    pub damage: u16,

    /// Chip damage (0 if not blocking).
    pub chip_damage: u16,

    /// Hitstun frames.
    pub hitstun: u8,

    /// Blockstun frames.
    pub blockstun: u8,

    /// Hitstop frames (for both attacker and defender).
    pub hitstop: u8,

    /// Guard type (high/mid/low).
    pub guard: u8,

    /// Hit pushback in pixels (applied on hit).
    pub hit_pushback: i32,

    /// Block pushback in pixels (applied on block).
    pub block_pushback: i32,
}
```

**Guard values:**
- `0` - High
- `1` - Mid
- `2` - Low

---

### CheckHitsResult

Fixed-capacity result buffer for hit checks (`no_std` friendly).

```rust
pub struct CheckHitsResult {
    // Internal: [Option<HitResult>; MAX_HIT_RESULTS]
}

impl CheckHitsResult {
    pub fn new() -> Self;
    pub fn push(&mut self, hit: HitResult);
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn get(&self, index: usize) -> Option<&HitResult>;
    pub fn iter(&self) -> impl Iterator<Item = &HitResult>;
}
```

**Capacity:** 8 hits maximum (`MAX_HIT_RESULTS`)

---

### Shape Types

#### Aabb

Axis-aligned bounding box for collision detection.

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Aabb {
    pub x: i32,   // Left edge (pixels)
    pub y: i32,   // Top edge (pixels)
    pub w: u32,   // Width (pixels)
    pub h: u32,   // Height (pixels)
}

impl Aabb {
    /// Create from a ShapeView at a given position offset.
    pub fn from_shape(shape: &ShapeView, offset_x: i32, offset_y: i32) -> Self;
}
```

#### Circle

Circle for collision detection.

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Circle {
    pub x: i32,   // Center X (pixels)
    pub y: i32,   // Center Y (pixels)
    pub r: u32,   // Radius (pixels)
}

impl Circle {
    /// Create from a ShapeView at a given position offset.
    pub fn from_shape(shape: &ShapeView, offset_x: i32, offset_y: i32) -> Self;
}
```

#### Capsule

Capsule (line segment with radius) for collision detection.

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Capsule {
    pub x1: i32,  // Endpoint 1 X (pixels)
    pub y1: i32,  // Endpoint 1 Y (pixels)
    pub x2: i32,  // Endpoint 2 X (pixels)
    pub y2: i32,  // Endpoint 2 Y (pixels)
    pub r: u32,   // Radius (pixels)
}

impl Capsule {
    /// Create from a ShapeView at a given position offset.
    pub fn from_shape(shape: &ShapeView, offset_x: i32, offset_y: i32) -> Self;
}
```

---

## Constants

### MAX_RESOURCES

```rust
pub const MAX_RESOURCES: usize = 8;
```

Maximum number of resource pools per character.

---

### MAX_HIT_RESULTS

```rust
pub const MAX_HIT_RESULTS: usize = 8;
```

Maximum number of hit results that can be stored in `CheckHitsResult`.

---

### Action Cancel Constants

Action IDs are offsets from the move count. Use with `can_cancel_to()`:

```rust
/// Chain cancel action (cancel into normal chain routes)
pub const ACTION_CHAIN: u16 = 0;

/// Special cancel action (cancel into special moves)
pub const ACTION_SPECIAL: u16 = 1;

/// Super cancel action (cancel into super moves)
pub const ACTION_SUPER: u16 = 2;

/// Jump cancel action (cancel into jump)
pub const ACTION_JUMP: u16 = 3;
```

**Usage:**

```rust
let move_count = pack.states().map(|s| s.len()).unwrap_or(0) as u16;
let jump_action = move_count + ACTION_JUMP;

if can_cancel_to(&state, &pack, jump_action) {
    // Jump cancel is allowed
}
```

---

## Functions

### next_frame

Compute the next frame state for a character.

```rust
#[must_use]
pub fn next_frame(
    state: &CharacterState,
    pack: &PackView,
    input: &FrameInput,
) -> FrameResult
```

**Arguments:**
- `state` - Current character state
- `pack` - Character data pack (from FSPK)
- `input` - Frame input (requested state transition)

**Returns:** New state and whether the move ended this frame.

**Behavior:**
1. If `input.requested_state` is `Some(target)` and `can_cancel_to()` returns true:
   - Transition to target state
   - Reset `frame` to 0
   - Clear `hit_confirmed` and `block_confirmed`
   - Apply resource costs via `apply_resource_costs()`
   - Return with `move_ended = false`
2. Otherwise:
   - Increment `frame` (saturating at 255)
   - Check if `frame >= effective_duration`
   - Return with `move_ended` set accordingly

**Notes:**
- This is a **pure function** - it does not mutate the input state
- The game decides whether to apply the returned state

---

### can_cancel_to

Check if a cancel from current state to target is valid.

```rust
#[must_use]
pub fn can_cancel_to(
    state: &CharacterState,
    pack: &PackView,
    target: u16,
) -> bool
```

**Arguments:**
- `state` - Current character state
- `pack` - Character data pack
- `target` - Target state ID (or action ID if `>= move_count`)

**Returns:** `true` if the cancel is valid right now.

**Evaluation order:**
1. If `target >= move_count`: Check action cancel flags
2. Check explicit denies (always blocks if present)
3. Check explicit chain cancels from state extras
4. Check tag-based cancel rules

**Notes:**
- Resource preconditions are checked for both explicit chains and tag rules
- Frame range conditions are checked for tag rules
- Hit/block conditions are checked for tag rules

---

### available_cancels

Get all valid cancel targets from current state.

```rust
#[cfg(feature = "alloc")]
pub fn available_cancels(
    state: &CharacterState,
    pack: &PackView,
) -> alloc::vec::Vec<u16>
```

**Arguments:**
- `state` - Current character state
- `pack` - Character data pack

**Returns:** Vector of valid cancel target state IDs.

**Notes:**
- Requires the `alloc` feature
- Filters by resource preconditions
- Returns explicit chain cancel targets only (not tag-based matches)

---

### available_cancels_buf

Get available cancels into a fixed-size buffer (`no_std` friendly).

```rust
pub fn available_cancels_buf(
    state: &CharacterState,
    pack: &PackView,
    buf: &mut [u16],
) -> usize
```

**Arguments:**
- `state` - Current character state
- `pack` - Character data pack
- `buf` - Buffer to write cancel targets into

**Returns:** Number of cancels written to buffer.

**Notes:**
- Stops writing when buffer is full
- Use for `no_std` environments or when avoiding allocations

---

### check_hits

Check all hitbox vs hurtbox interactions between two characters.

```rust
#[must_use]
pub fn check_hits(
    attacker_state: &CharacterState,
    attacker_pack: &PackView,
    attacker_pos: (i32, i32),
    defender_state: &CharacterState,
    defender_pack: &PackView,
    defender_pos: (i32, i32),
) -> CheckHitsResult
```

**Arguments:**
- `attacker_state` - Attacker's current state
- `attacker_pack` - Attacker's character pack
- `attacker_pos` - Attacker's position `(x, y)` in pixels
- `defender_state` - Defender's current state
- `defender_pack` - Defender's character pack
- `defender_pos` - Defender's position `(x, y)` in pixels

**Returns:** Collection of hit results (up to `MAX_HIT_RESULTS`).

**Behavior:**
1. Iterates attacker's hit windows active this frame
2. Iterates defender's hurt windows active this frame
3. Checks shape overlaps between hitboxes and hurtboxes
4. Returns one hit per hit window maximum

---

### report_hit

Report that the current state connected with a hit.

```rust
#[inline]
pub fn report_hit(state: &mut CharacterState)
```

**Effect:** Sets `state.hit_confirmed = true`

**Purpose:** Opens on-hit cancel windows for tag-based rules with `condition = on_hit`.

---

### report_block

Report that the current state was blocked.

```rust
#[inline]
pub fn report_block(state: &mut CharacterState)
```

**Effect:** Sets `state.block_confirmed = true`

**Purpose:** Opens on-block cancel windows for tag-based rules with `condition = on_block`.

---

### Shape Overlap Functions

#### aabb_overlap

Check if two AABBs overlap.

```rust
#[inline]
#[must_use]
pub fn aabb_overlap(a: &Aabb, b: &Aabb) -> bool
```

**Note:** Edge-touching is NOT considered overlap.

---

#### circle_overlap

Check if two circles overlap.

```rust
#[inline]
#[must_use]
pub fn circle_overlap(a: &Circle, b: &Circle) -> bool
```

**Note:** Edge-touching is NOT considered overlap.

---

#### aabb_circle_overlap

Check if an AABB and circle overlap.

```rust
#[inline]
#[must_use]
pub fn aabb_circle_overlap(aabb: &Aabb, circle: &Circle) -> bool
```

---

#### capsule_overlap

Check if two capsules overlap.

```rust
#[inline]
#[must_use]
pub fn capsule_overlap(a: &Capsule, b: &Capsule) -> bool
```

**Note:** Edge-touching is NOT considered overlap.

---

#### shapes_overlap

Check if two shapes overlap (dispatches to appropriate overlap function).

```rust
#[must_use]
pub fn shapes_overlap(
    a: &ShapeView,
    a_offset: (i32, i32),
    b: &ShapeView,
    b_offset: (i32, i32),
) -> bool
```

**Supported combinations:**
- AABB vs AABB
- Circle vs Circle
- AABB vs Circle
- Circle vs AABB
- Capsule vs Capsule

**Unsupported combinations return `false`:**
- Rotated rectangles
- Mixed capsule with AABB/circle

---

### Resource Functions

#### resource

Get the current value of a resource by index.

```rust
#[inline]
pub fn resource(state: &CharacterState, index: u8) -> u16
```

**Returns:** Resource value, or 0 if index is out of bounds.

---

#### set_resource

Set a resource value by index.

```rust
#[inline]
pub fn set_resource(state: &mut CharacterState, index: u8, value: u16)
```

**Effect:** Does nothing if index is out of bounds.

---

#### init_resources

Initialize resources from pack's resource definitions.

```rust
pub fn init_resources(state: &mut CharacterState, pack: &PackView)
```

**Effect:**
1. Resets all resources to 0
2. Sets each resource to its `start` value from pack definitions

---

#### apply_resource_costs

Apply resource costs for a move transition.

```rust
pub fn apply_resource_costs(
    state: &mut CharacterState,
    pack: &PackView,
    move_index: u16,
) -> bool
```

**Arguments:**
- `state` - Character state to modify
- `pack` - Character pack
- `move_index` - Target state index

**Returns:** `true` if all costs were paid, `false` if any resource was insufficient.

**Effect:** Deducts costs from state using `saturating_sub` (costs are still deducted even if insufficient).

**Note:** Called automatically by `next_frame()` on successful transitions.

---

#### check_resource_preconditions

Check all resource preconditions for a move.

```rust
pub fn check_resource_preconditions(
    state: &CharacterState,
    pack: &PackView,
    move_index: u16,
) -> bool
```

**Arguments:**
- `state` - Current character state
- `pack` - Character pack
- `move_index` - Target state index

**Returns:** `true` if all preconditions are satisfied.

**Precondition types:**
- `min` - Resource must be >= this value
- `max` - Resource must be <= this value

**Note:** Called automatically by `can_cancel_to()`.

---

## Re-exports

The crate re-exports `PackView` from `framesmith-fspack` for convenience:

```rust
pub use framesmith_fspack::PackView;
```

See [zx-fspack.md](zx-fspack.md) for `PackView` documentation.

---

## Feature Flags

### `alloc`

Enables functions that require heap allocation:
- `available_cancels()` - Returns `Vec<u16>`

Without this feature, use buffer-based alternatives (`available_cancels_buf()`).

---

## See Also

- [Runtime Integration Guide](runtime-guide.md) - Practical integration examples
- [ZX FSPK Format](zx-fspack.md) - Binary pack format specification
