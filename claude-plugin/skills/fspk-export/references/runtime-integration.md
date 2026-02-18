# Runtime API Reference

Complete API documentation for `framesmith-runtime` and `framesmith-fspack`.

## Core Types

### CharacterState (22 bytes, Copy)

```rust
pub struct CharacterState {
    pub current_state: u16,       // Active state index (0 = idle)
    pub frame: u8,                // Current frame within state
    pub instance_duration: u8,    // Override duration (0 = use default)
    pub hit_confirmed: bool,      // Hit connected (opens on-hit cancels)
    pub block_confirmed: bool,    // Attack blocked (opens on-block cancels)
    pub resources: [u16; 8],      // Resource pools (meter, heat, etc.)
}
```

### FrameInput

```rust
pub struct FrameInput {
    pub requested_state: Option<u16>,  // None = continue, Some = request cancel
}
```

### FrameResult

```rust
pub struct FrameResult {
    pub state: CharacterState,  // Updated state after this frame
    pub move_ended: bool,       // True if state reached final frame
}
```

### HitResult

```rust
pub struct HitResult {
    pub attacker_move: u16,
    pub window_index: u16,
    pub damage: u16,
    pub chip_damage: u16,
    pub hitstun: u8,
    pub blockstun: u8,
    pub hitstop: u8,
    pub guard: u8,           // 0=high, 1=mid, 2=low
    pub hit_pushback: i32,
    pub block_pushback: i32,
}
```

### CheckHitsResult

Fixed-capacity buffer (max 8 hits). Methods: `len()`, `is_empty()`, `get(index)`, `iter()`.

## Constants

```rust
pub const MAX_RESOURCES: usize = 8;
pub const MAX_HIT_RESULTS: usize = 8;

// Action cancel offsets (add to move_count)
pub const ACTION_CHAIN: u16 = 0;
pub const ACTION_SPECIAL: u16 = 1;
pub const ACTION_SUPER: u16 = 2;
pub const ACTION_JUMP: u16 = 3;
```

## Functions

### next_frame

```rust
pub fn next_frame(
    state: &CharacterState,
    pack: &PackView,
    input: &FrameInput,
) -> FrameResult
```

Pure function. If `input.requested_state` is valid, transitions immediately (resets frame, clears hit/block confirmed, applies resource costs). Otherwise increments frame and checks for move end.

### can_cancel_to

```rust
pub fn can_cancel_to(
    state: &CharacterState,
    pack: &PackView,
    target: u16,
) -> bool
```

Evaluation order:
1. If `target >= move_count`: check action cancel flags
2. Check explicit denies (always blocks)
3. Check explicit chain cancels from state extras
4. Check tag-based cancel rules (with frame range and condition checks)

Resource preconditions are checked for both chains and tag rules.

### available_cancels / available_cancels_buf

```rust
// Requires "alloc" feature
pub fn available_cancels(state: &CharacterState, pack: &PackView) -> Vec<u16>

// no_std friendly
pub fn available_cancels_buf(state: &CharacterState, pack: &PackView, buf: &mut [u16]) -> usize
```

### check_hits

```rust
pub fn check_hits(
    attacker_state: &CharacterState, attacker_pack: &PackView, attacker_pos: (i32, i32),
    defender_state: &CharacterState, defender_pack: &PackView, defender_pos: (i32, i32),
) -> CheckHitsResult
```

Checks all hitbox vs hurtbox overlaps. Returns up to MAX_HIT_RESULTS hits.

### report_hit / report_block

```rust
pub fn report_hit(state: &mut CharacterState)   // Sets hit_confirmed = true
pub fn report_block(state: &mut CharacterState)  // Sets block_confirmed = true
```

Opens on-hit / on-block cancel windows for tag-based rules.

### Resource Functions

```rust
pub fn resource(state: &CharacterState, index: u8) -> u16
pub fn set_resource(state: &mut CharacterState, index: u8, value: u16)
pub fn init_resources(state: &mut CharacterState, pack: &PackView)
pub fn apply_resource_costs(state: &mut CharacterState, pack: &PackView, move_index: u16) -> bool
pub fn check_resource_preconditions(state: &CharacterState, pack: &PackView, move_index: u16) -> bool
```

`init_resources`: resets all to 0, then sets starting values from pack.
`apply_resource_costs`: called automatically by `next_frame()` on transition.
`check_resource_preconditions`: called automatically by `can_cancel_to()`.

### Shape Overlap Functions

```rust
pub fn aabb_overlap(a: &Aabb, b: &Aabb) -> bool
pub fn circle_overlap(a: &Circle, b: &Circle) -> bool
pub fn aabb_circle_overlap(aabb: &Aabb, circle: &Circle) -> bool
pub fn capsule_overlap(a: &Capsule, b: &Capsule) -> bool
pub fn shapes_overlap(a: &ShapeView, a_offset: (i32, i32), b: &ShapeView, b_offset: (i32, i32)) -> bool
```

Edge-touching is NOT considered overlap. Unsupported shape combinations return false.

## Game Loop Pattern

```rust
fn game_tick(game: &mut GameState) {
    // 1. Map player inputs to requested states
    let p1_input = FrameInput { requested_state: game.p1_buffered_input.take() };
    let p2_input = FrameInput { requested_state: game.p2_buffered_input.take() };

    // 2. Advance character states
    let p1_result = next_frame(&game.p1_state, &game.p1_pack, &p1_input);
    let p2_result = next_frame(&game.p2_state, &game.p2_pack, &p2_input);
    game.p1_state = p1_result.state;
    game.p2_state = p2_result.state;

    // 3. Check hits (both directions)
    let p1_hits = check_hits(&game.p1_state, &game.p1_pack, game.p1_pos,
                             &game.p2_state, &game.p2_pack, game.p2_pos);

    // 4. Process hits, apply damage, report hit/block
    // 5. Handle state completion (return to idle)
}
```

## Rollback Pattern

```rust
let saved = game.p1_state;  // 22 bytes, Copy, instant

// ... prediction was wrong ...

game.p1_state = saved;  // Restore
for frame in rollback_start..current_frame {
    let input = get_corrected_input(frame);
    let result = next_frame(&game.p1_state, &game.p1_pack, &input);
    game.p1_state = result.state;
}
```

## WASM/Browser Usage

```typescript
import { TrainingSession, DummyState } from 'framesmith-runtime-wasm';

const session = new TrainingSession(
    new Uint8Array(playerFspk),
    new Uint8Array(dummyFspk)
);

function tick() {
    const result = session.tick(playerInput, DummyState.Stand);
    render(result);
    requestAnimationFrame(tick);
}
```

## Resource Handling Split

| Aspect | Handled By | When |
|--------|------------|------|
| Costs | Runtime (automatic) | Deducted on state transition |
| Preconditions | Runtime (automatic) | Checked before allowing cancel |
| Deltas | Engine (manual) | Applied by game based on events |

Deltas are manual because the runtime is stateless - rollback might revert them. The engine reads deltas from FSPK and applies when hits are confirmed by game logic.

## Feature Flags

- **`alloc`**: Enables `available_cancels()` (returns `Vec<u16>`). Without it, use `available_cancels_buf()`.

## Re-exports

`framesmith-runtime` re-exports `PackView` from `framesmith-fspack` for convenience.
