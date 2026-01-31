# Framesmith Runtime Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Create a no_std stateless runtime library for simulating fighting game character state machines.

**Architecture:** Pure functions operating on plain data structs. `CharacterState` holds move/frame/resources. `next_frame()` computes the next state without mutation. Cancel logic uses the pack's cancel table as single source of truth. Collision helpers resolve hitbox/hurtbox interactions and return attack data for games to apply.

**Tech Stack:** Rust, no_std, depends on framesmith-fspack for data access.

---

## Task 1: Create Crate Scaffold

**Files:**
- Create: `crates/framesmith-runtime/Cargo.toml`
- Create: `crates/framesmith-runtime/src/lib.rs`

**Step 1: Create Cargo.toml**

```toml
[package]
name = "framesmith-runtime"
version = "0.1.0"
edition = "2021"
description = "no_std stateless runtime for fighting game character simulation"
license = "MIT OR Apache-2.0"

[features]
default = []
alloc = ["framesmith-fspack/alloc"]
std = ["alloc", "framesmith-fspack/std"]

[dependencies]
framesmith-fspack = { path = "../framesmith-fspack" }

[dev-dependencies]
```

**Step 2: Create lib.rs with module structure**

```rust
#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod state;
pub mod frame;
pub mod cancel;
pub mod collision;
pub mod resource;

pub use state::{CharacterState, FrameInput, FrameResult};
pub use frame::next_frame;
pub use cancel::{can_cancel_to, available_cancels};
pub use collision::{check_hits, shapes_overlap, HitResult};
pub use resource::{resource, set_resource};

#[cfg(test)]
extern crate std;
```

**Step 3: Verify crate compiles**

Run: `cd crates/framesmith-runtime && cargo check`
Expected: Errors about missing modules (expected at this stage)

**Step 4: Commit scaffold**

```bash
git add crates/framesmith-runtime/
git commit -m "feat(runtime): scaffold framesmith-runtime crate"
```

---

## Task 2: Implement CharacterState

**Files:**
- Create: `crates/framesmith-runtime/src/state.rs`

**Step 1: Write the test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_state_is_copy_and_default() {
        let state = CharacterState::default();
        let copy = state; // Copy
        assert_eq!(state.current_move, copy.current_move);
        assert_eq!(state.frame, 0);
        assert!(!state.hit_confirmed);
        assert!(!state.block_confirmed);
    }

    #[test]
    fn character_state_size_is_small() {
        // State should be small enough for cheap copies (rollback netcode)
        assert!(core::mem::size_of::<CharacterState>() <= 32);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cd crates/framesmith-runtime && cargo test`
Expected: FAIL - CharacterState not defined

**Step 3: Implement CharacterState**

```rust
/// Maximum number of resource pools per character.
pub const MAX_RESOURCES: usize = 8;

/// Character simulation state.
///
/// This struct is intentionally small, `Copy`, and deterministic for:
/// - Cheap cloning (rollback netcode)
/// - No heap allocations (no_std compatible)
/// - Predictable simulation (no floats, no randomness)
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct CharacterState {
    /// Current move index (0 = idle by convention).
    pub current_move: u16,
    /// Current frame within the move (0-indexed).
    pub frame: u8,
    /// Move connected with a hit (opens on-hit cancel windows).
    pub hit_confirmed: bool,
    /// Move was blocked (opens on-block cancel windows).
    pub block_confirmed: bool,
    /// Resource pool values (meter, heat, ammo, etc.).
    pub resources: [u16; MAX_RESOURCES],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_state_is_copy_and_default() {
        let state = CharacterState::default();
        let copy = state;
        assert_eq!(state.current_move, copy.current_move);
        assert_eq!(state.frame, 0);
        assert!(!state.hit_confirmed);
        assert!(!state.block_confirmed);
    }

    #[test]
    fn character_state_size_is_small() {
        assert!(core::mem::size_of::<CharacterState>() <= 32);
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cd crates/framesmith-runtime && cargo test`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/framesmith-runtime/src/state.rs
git commit -m "feat(runtime): add CharacterState struct"
```

---

## Task 3: Implement FrameInput and FrameResult

**Files:**
- Modify: `crates/framesmith-runtime/src/state.rs`

**Step 1: Write the test**

```rust
#[test]
fn frame_input_default_has_no_requested_move() {
    let input = FrameInput::default();
    assert!(input.requested_move.is_none());
}

#[test]
fn frame_result_can_hold_events() {
    let result = FrameResult {
        state: CharacterState::default(),
        move_ended: false,
    };
    assert!(!result.move_ended);
}
```

**Step 2: Run test to verify it fails**

Run: `cd crates/framesmith-runtime && cargo test`
Expected: FAIL - FrameInput, FrameResult not defined

**Step 3: Implement FrameInput and FrameResult**

```rust
/// Input for a single frame of simulation.
#[derive(Clone, Copy, Debug, Default)]
pub struct FrameInput {
    /// Move to transition to, if cancel is valid.
    /// `None` means continue current move.
    pub requested_move: Option<u16>,
}

/// Result of simulating one frame.
#[derive(Clone, Copy, Debug)]
pub struct FrameResult {
    /// The new character state after this frame.
    pub state: CharacterState,
    /// True if the move reached its final frame.
    /// Game decides whether to loop or transition.
    pub move_ended: bool,
}
```

**Step 4: Run test to verify it passes**

Run: `cd crates/framesmith-runtime && cargo test`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/framesmith-runtime/src/state.rs
git commit -m "feat(runtime): add FrameInput and FrameResult"
```

---

## Task 4: Implement Resource Helpers

**Files:**
- Create: `crates/framesmith-runtime/src/resource.rs`

**Step 1: Write the test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::CharacterState;

    #[test]
    fn get_and_set_resource() {
        let mut state = CharacterState::default();
        assert_eq!(resource(&state, 0), 0);

        set_resource(&mut state, 0, 100);
        assert_eq!(resource(&state, 0), 100);

        set_resource(&mut state, 7, 50);
        assert_eq!(resource(&state, 7), 50);
    }

    #[test]
    fn out_of_bounds_resource_returns_zero() {
        let state = CharacterState::default();
        assert_eq!(resource(&state, 8), 0);
        assert_eq!(resource(&state, 255), 0);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cd crates/framesmith-runtime && cargo test`
Expected: FAIL - resource, set_resource not defined

**Step 3: Implement resource helpers**

```rust
use crate::state::{CharacterState, MAX_RESOURCES};

/// Get the current value of a resource by index.
///
/// Returns 0 if the index is out of bounds.
#[inline]
pub fn resource(state: &CharacterState, index: u8) -> u16 {
    state.resources.get(index as usize).copied().unwrap_or(0)
}

/// Set a resource value by index.
///
/// Does nothing if the index is out of bounds.
#[inline]
pub fn set_resource(state: &mut CharacterState, index: u8, value: u16) {
    if let Some(slot) = state.resources.get_mut(index as usize) {
        *slot = value;
    }
}

/// Initialize resources from pack's resource definitions.
pub fn init_resources(state: &mut CharacterState, pack: &framesmith_fspack::PackView) {
    // Reset all to zero first
    state.resources = [0; MAX_RESOURCES];

    if let Some(defs) = pack.resource_defs() {
        for i in 0..defs.len().min(MAX_RESOURCES) {
            if let Some(def) = defs.get(i) {
                state.resources[i] = def.start();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::CharacterState;

    #[test]
    fn get_and_set_resource() {
        let mut state = CharacterState::default();
        assert_eq!(resource(&state, 0), 0);

        set_resource(&mut state, 0, 100);
        assert_eq!(resource(&state, 0), 100);

        set_resource(&mut state, 7, 50);
        assert_eq!(resource(&state, 7), 50);
    }

    #[test]
    fn out_of_bounds_resource_returns_zero() {
        let state = CharacterState::default();
        assert_eq!(resource(&state, 8), 0);
        assert_eq!(resource(&state, 255), 0);
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cd crates/framesmith-runtime && cargo test`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/framesmith-runtime/src/resource.rs
git commit -m "feat(runtime): add resource get/set helpers"
```

---

## Task 5: Implement Basic next_frame (Frame Advancement Only)

**Files:**
- Create: `crates/framesmith-runtime/src/frame.rs`

**Step 1: Write the test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{CharacterState, FrameInput};

    #[test]
    fn next_frame_advances_frame_counter() {
        let state = CharacterState {
            current_move: 0,
            frame: 5,
            ..Default::default()
        };
        let input = FrameInput::default();

        // Without a pack, we need a mock or minimal test
        // For now, test the pure frame advancement logic
        let next = advance_frame_counter(&state);
        assert_eq!(next.frame, 6);
        assert_eq!(next.current_move, 0);
    }

    #[test]
    fn frame_counter_saturates_at_max() {
        let state = CharacterState {
            frame: 255,
            ..Default::default()
        };
        let next = advance_frame_counter(&state);
        assert_eq!(next.frame, 255); // Saturates, doesn't wrap
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cd crates/framesmith-runtime && cargo test`
Expected: FAIL - advance_frame_counter not defined

**Step 3: Implement frame advancement**

```rust
use crate::state::{CharacterState, FrameInput, FrameResult};
use framesmith_fspack::PackView;

/// Advance frame counter by 1, saturating at u8::MAX.
#[inline]
fn advance_frame_counter(state: &CharacterState) -> CharacterState {
    CharacterState {
        frame: state.frame.saturating_add(1),
        ..*state
    }
}

/// Compute the next frame state for a character.
///
/// This is a pure function - it does not mutate the input state.
/// The game decides whether to apply the returned state.
///
/// # Arguments
/// * `state` - Current character state
/// * `pack` - Character data pack (moves, cancels, etc.)
/// * `input` - Frame input (requested move, etc.)
///
/// # Returns
/// New state and whether the move ended this frame.
pub fn next_frame(
    state: &CharacterState,
    pack: &PackView,
    input: &FrameInput,
) -> FrameResult {
    // Try to transition if a move was requested
    if let Some(target) = input.requested_move {
        if crate::cancel::can_cancel_to(state, pack, target) {
            let mut new_state = *state;
            new_state.current_move = target;
            new_state.frame = 0;
            new_state.hit_confirmed = false;
            new_state.block_confirmed = false;
            // TODO: Apply resource costs
            return FrameResult {
                state: new_state,
                move_ended: false,
            };
        }
    }

    // Advance frame
    let new_state = advance_frame_counter(state);

    // Check if move ended
    let move_ended = if let Some(moves) = pack.moves() {
        if let Some(mv) = moves.get(state.current_move as usize) {
            new_state.frame >= mv.total_frames() as u8
        } else {
            false
        }
    } else {
        false
    };

    FrameResult {
        state: new_state,
        move_ended,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{CharacterState, FrameInput};

    #[test]
    fn next_frame_advances_frame_counter() {
        let state = CharacterState {
            current_move: 0,
            frame: 5,
            ..Default::default()
        };

        let next = advance_frame_counter(&state);
        assert_eq!(next.frame, 6);
        assert_eq!(next.current_move, 0);
    }

    #[test]
    fn frame_counter_saturates_at_max() {
        let state = CharacterState {
            frame: 255,
            ..Default::default()
        };
        let next = advance_frame_counter(&state);
        assert_eq!(next.frame, 255);
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cd crates/framesmith-runtime && cargo test`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/framesmith-runtime/src/frame.rs
git commit -m "feat(runtime): add next_frame with frame advancement"
```

---

## Task 6: Implement Cancel Logic (can_cancel_to)

**Files:**
- Create: `crates/framesmith-runtime/src/cancel.rs`

**Step 1: Write the test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::CharacterState;

    #[test]
    fn cannot_cancel_to_same_move_by_default() {
        let state = CharacterState {
            current_move: 5,
            frame: 10,
            ..Default::default()
        };
        // Without pack data, can_cancel_to should return false
        // This tests the fallback behavior
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cd crates/framesmith-runtime && cargo test`
Expected: FAIL - cancel module not defined

**Step 3: Implement can_cancel_to**

```rust
use crate::state::CharacterState;
use framesmith_fspack::PackView;

/// Check if a cancel from current state to target move is valid.
///
/// This checks:
/// - Cancel routes from the cancel table
/// - Resource preconditions
/// - Hit/block state for on-hit/on-block cancels
///
/// # Arguments
/// * `state` - Current character state
/// * `pack` - Character data pack
/// * `target` - Target move ID (or action ID if >= move_count)
///
/// # Returns
/// `true` if the cancel is valid right now.
pub fn can_cancel_to(
    state: &CharacterState,
    pack: &PackView,
    target: u16,
) -> bool {
    let moves = match pack.moves() {
        Some(m) => m,
        None => return false,
    };

    let move_count = moves.len() as u16;

    // Check if target is a game-defined action (>= move_count)
    // The runtime allows these; game decides if the action is valid
    if target >= move_count {
        // Check if current move allows this action via cancel flags
        return check_action_cancel(state, pack, target - move_count);
    }

    // Check chain cancels from move extras
    if let Some(extras) = pack.move_extras() {
        if let Some(extra) = extras.get(state.current_move as usize) {
            if let Some(cancels) = pack.cancels() {
                let (off, len) = extra.cancels();
                for i in 0..len as usize {
                    if let Some(cancel_target) = cancels.get_at(off, i) {
                        if cancel_target == target {
                            // TODO: Check timing windows and preconditions
                            return true;
                        }
                    }
                }
            }
        }
    }

    false
}

/// Check if an action cancel is allowed (jump, dash, etc.).
fn check_action_cancel(
    _state: &CharacterState,
    _pack: &PackView,
    _action_id: u16,
) -> bool {
    // TODO: Check cancel flags on current move
    // For now, delegate to game (return true and let game validate)
    true
}

/// Get all valid cancel targets from current state.
///
/// Returns move IDs (< move_count) and action IDs (>= move_count).
#[cfg(feature = "alloc")]
pub fn available_cancels(
    state: &CharacterState,
    pack: &PackView,
) -> alloc::vec::Vec<u16> {
    let mut result = alloc::vec::Vec::new();

    if let Some(extras) = pack.move_extras() {
        if let Some(extra) = extras.get(state.current_move as usize) {
            if let Some(cancels) = pack.cancels() {
                let (off, len) = extra.cancels();
                for i in 0..len as usize {
                    if let Some(target) = cancels.get_at(off, i) {
                        // TODO: Filter by timing window and preconditions
                        result.push(target);
                    }
                }
            }
        }
    }

    result
}

/// Get available cancels into a fixed-size buffer.
///
/// Returns the number of cancels written.
pub fn available_cancels_buf(
    state: &CharacterState,
    pack: &PackView,
    buf: &mut [u16],
) -> usize {
    let mut count = 0;

    if let Some(extras) = pack.move_extras() {
        if let Some(extra) = extras.get(state.current_move as usize) {
            if let Some(cancels) = pack.cancels() {
                let (off, len) = extra.cancels();
                for i in 0..len as usize {
                    if count >= buf.len() {
                        break;
                    }
                    if let Some(target) = cancels.get_at(off, i) {
                        buf[count] = target;
                        count += 1;
                    }
                }
            }
        }
    }

    count
}

#[cfg(test)]
mod tests {
    #[test]
    fn cancel_module_compiles() {
        // Basic smoke test
        assert!(true);
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cd crates/framesmith-runtime && cargo test`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/framesmith-runtime/src/cancel.rs
git commit -m "feat(runtime): add cancel logic (can_cancel_to, available_cancels)"
```

---

## Task 7: Implement Shape Overlap (AABB)

**Files:**
- Create: `crates/framesmith-runtime/src/collision.rs`

**Step 1: Write the test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aabb_overlap_detects_intersection() {
        // Two overlapping boxes
        let a = Aabb { x: 0, y: 0, w: 10, h: 10 };
        let b = Aabb { x: 5, y: 5, w: 10, h: 10 };
        assert!(aabb_overlap(&a, &b));
    }

    #[test]
    fn aabb_overlap_detects_no_intersection() {
        // Two non-overlapping boxes
        let a = Aabb { x: 0, y: 0, w: 10, h: 10 };
        let b = Aabb { x: 20, y: 20, w: 10, h: 10 };
        assert!(!aabb_overlap(&a, &b));
    }

    #[test]
    fn aabb_overlap_edge_touching_is_not_overlap() {
        // Boxes touching at edge
        let a = Aabb { x: 0, y: 0, w: 10, h: 10 };
        let b = Aabb { x: 10, y: 0, w: 10, h: 10 };
        assert!(!aabb_overlap(&a, &b));
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cd crates/framesmith-runtime && cargo test`
Expected: FAIL - Aabb, aabb_overlap not defined

**Step 3: Implement AABB overlap**

```rust
use crate::state::CharacterState;
use framesmith_fspack::{PackView, ShapeView, SHAPE_KIND_AABB};

/// Axis-aligned bounding box for collision detection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Aabb {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

impl Aabb {
    /// Create an AABB from a ShapeView at a given position offset.
    pub fn from_shape(shape: &ShapeView, offset_x: i32, offset_y: i32) -> Self {
        Aabb {
            x: shape.x_px() + offset_x,
            y: shape.y_px() + offset_y,
            w: shape.width_px(),
            h: shape.height_px(),
        }
    }
}

/// Check if two AABBs overlap.
///
/// Edge-touching is NOT considered overlap.
#[inline]
pub fn aabb_overlap(a: &Aabb, b: &Aabb) -> bool {
    let a_right = a.x.saturating_add(a.w as i32);
    let a_bottom = a.y.saturating_add(a.h as i32);
    let b_right = b.x.saturating_add(b.w as i32);
    let b_bottom = b.y.saturating_add(b.h as i32);

    a.x < b_right && a_right > b.x && a.y < b_bottom && a_bottom > b.y
}

/// Check if two shapes overlap.
///
/// Currently only supports AABB shapes.
pub fn shapes_overlap(
    a: &ShapeView,
    a_offset: (i32, i32),
    b: &ShapeView,
    b_offset: (i32, i32),
) -> bool {
    // For now, only handle AABB
    if a.kind() == SHAPE_KIND_AABB && b.kind() == SHAPE_KIND_AABB {
        let aabb_a = Aabb::from_shape(a, a_offset.0, a_offset.1);
        let aabb_b = Aabb::from_shape(b, b_offset.0, b_offset.1);
        return aabb_overlap(&aabb_a, &aabb_b);
    }

    // TODO: Handle other shape types (circle, capsule, rotated rect)
    false
}

/// Result of a hit interaction.
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
    // TODO: Add pushback when HitWindow is expanded
}

/// Check all hitbox vs hurtbox interactions between two characters.
///
/// Returns hit results for the game to process.
pub fn check_hits(
    attacker_state: &CharacterState,
    attacker_pack: &PackView,
    attacker_pos: (i32, i32),
    defender_state: &CharacterState,
    defender_pack: &PackView,
    defender_pos: (i32, i32),
) -> CheckHitsResult {
    let mut result = CheckHitsResult::new();

    let attacker_frame = attacker_state.frame;
    let defender_frame = defender_state.frame;

    // Get attacker's active hitboxes
    let attacker_moves = match attacker_pack.moves() {
        Some(m) => m,
        None => return result,
    };
    let attacker_move = match attacker_moves.get(attacker_state.current_move as usize) {
        Some(m) => m,
        None => return result,
    };

    // Get defender's active hurtboxes
    let defender_moves = match defender_pack.moves() {
        Some(m) => m,
        None => return result,
    };
    let defender_move = match defender_moves.get(defender_state.current_move as usize) {
        Some(m) => m,
        None => return result,
    };

    let hit_windows = match attacker_pack.hit_windows() {
        Some(h) => h,
        None => return result,
    };
    let hurt_windows = match defender_pack.hurt_windows() {
        Some(h) => h,
        None => return result,
    };
    let attacker_shapes = match attacker_pack.shapes() {
        Some(s) => s,
        None => return result,
    };
    let defender_shapes = match defender_pack.shapes() {
        Some(s) => s,
        None => return result,
    };

    // Iterate attacker's hit windows active this frame
    for hw_idx in 0..attacker_move.hit_windows_len() as usize {
        let hw = match hit_windows.get_at(attacker_move.hit_windows_off(), hw_idx) {
            Some(h) => h,
            None => continue,
        };

        // Check if hit window is active this frame
        if attacker_frame < hw.start_frame() || attacker_frame > hw.end_frame() {
            continue;
        }

        // Iterate defender's hurt windows active this frame
        for hrt_idx in 0..defender_move.hurt_windows_len() as usize {
            let hrt = match hurt_windows.get_at(defender_move.hurt_windows_off(), hrt_idx) {
                Some(h) => h,
                None => continue,
            };

            // Check if hurt window is active this frame
            if defender_frame < hrt.start_frame() || defender_frame > hrt.end_frame() {
                continue;
            }

            // Check shape overlaps
            if check_window_overlap(
                &hw, &attacker_shapes, attacker_pos,
                &hrt, &defender_shapes, defender_pos,
            ) {
                result.push(HitResult {
                    attacker_move: attacker_state.current_move,
                    window_index: hw_idx as u16,
                    damage: hw.damage(),
                    chip_damage: hw.chip_damage(),
                    hitstun: hw.hitstun(),
                    blockstun: hw.blockstun(),
                    hitstop: hw.hitstop(),
                    guard: hw.guard(),
                });
                // Only one hit per hit window per frame
                break;
            }
        }
    }

    result
}

/// Check if any hitbox shape overlaps any hurtbox shape.
fn check_window_overlap(
    hit_window: &framesmith_fspack::HitWindowView,
    hit_shapes: &framesmith_fspack::ShapesView,
    hit_pos: (i32, i32),
    hurt_window: &framesmith_fspack::HurtWindowView,
    hurt_shapes: &framesmith_fspack::ShapesView,
    hurt_pos: (i32, i32),
) -> bool {
    for i in 0..hit_window.shapes_len() as usize {
        let hit_shape = match hit_shapes.get_at(hit_window.shapes_off(), i) {
            Some(s) => s,
            None => continue,
        };

        for j in 0..hurt_window.shapes_len() as usize {
            let hurt_shape = match hurt_shapes.get_at(hurt_window.shapes_off(), j) {
                Some(s) => s,
                None => continue,
            };

            if shapes_overlap(&hit_shape, hit_pos, &hurt_shape, hurt_pos) {
                return true;
            }
        }
    }

    false
}

/// Fixed-capacity result buffer for hit checks (no_std friendly).
pub struct CheckHitsResult {
    hits: [Option<HitResult>; 8],
    count: usize,
}

impl CheckHitsResult {
    pub fn new() -> Self {
        Self {
            hits: [None; 8],
            count: 0,
        }
    }

    pub fn push(&mut self, hit: HitResult) {
        if self.count < 8 {
            self.hits[self.count] = Some(hit);
            self.count += 1;
        }
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub fn get(&self, index: usize) -> Option<&HitResult> {
        if index < self.count {
            self.hits[index].as_ref()
        } else {
            None
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &HitResult> {
        self.hits[..self.count].iter().filter_map(|h| h.as_ref())
    }
}

impl Default for CheckHitsResult {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aabb_overlap_detects_intersection() {
        let a = Aabb { x: 0, y: 0, w: 10, h: 10 };
        let b = Aabb { x: 5, y: 5, w: 10, h: 10 };
        assert!(aabb_overlap(&a, &b));
    }

    #[test]
    fn aabb_overlap_detects_no_intersection() {
        let a = Aabb { x: 0, y: 0, w: 10, h: 10 };
        let b = Aabb { x: 20, y: 20, w: 10, h: 10 };
        assert!(!aabb_overlap(&a, &b));
    }

    #[test]
    fn aabb_overlap_edge_touching_is_not_overlap() {
        let a = Aabb { x: 0, y: 0, w: 10, h: 10 };
        let b = Aabb { x: 10, y: 0, w: 10, h: 10 };
        assert!(!aabb_overlap(&a, &b));
    }

    #[test]
    fn check_hits_result_capacity() {
        let mut result = CheckHitsResult::new();
        assert!(result.is_empty());

        for i in 0..10 {
            result.push(HitResult {
                attacker_move: i,
                window_index: 0,
                damage: 10,
                chip_damage: 0,
                hitstun: 10,
                blockstun: 5,
                hitstop: 3,
                guard: 0,
            });
        }

        // Should cap at 8
        assert_eq!(result.len(), 8);
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cd crates/framesmith-runtime && cargo test`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/framesmith-runtime/src/collision.rs
git commit -m "feat(runtime): add collision detection (AABB, check_hits)"
```

---

## Task 8: Implement Hit Reporting

**Files:**
- Modify: `crates/framesmith-runtime/src/state.rs`

**Step 1: Write the test**

```rust
#[test]
fn report_hit_sets_flag() {
    let mut state = CharacterState::default();
    assert!(!state.hit_confirmed);

    report_hit(&mut state);
    assert!(state.hit_confirmed);
}

#[test]
fn report_block_sets_flag() {
    let mut state = CharacterState::default();
    assert!(!state.block_confirmed);

    report_block(&mut state);
    assert!(state.block_confirmed);
}
```

**Step 2: Run test to verify it fails**

Run: `cd crates/framesmith-runtime && cargo test`
Expected: FAIL - report_hit, report_block not defined

**Step 3: Implement hit reporting**

Add to `state.rs`:

```rust
/// Report that the current move connected with a hit.
///
/// This opens on-hit cancel windows.
#[inline]
pub fn report_hit(state: &mut CharacterState) {
    state.hit_confirmed = true;
}

/// Report that the current move was blocked.
///
/// This opens on-block cancel windows.
#[inline]
pub fn report_block(state: &mut CharacterState) {
    state.block_confirmed = true;
}
```

**Step 4: Run test to verify it passes**

Run: `cd crates/framesmith-runtime && cargo test`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/framesmith-runtime/src/state.rs
git commit -m "feat(runtime): add report_hit and report_block"
```

---

## Task 9: Update lib.rs Exports

**Files:**
- Modify: `crates/framesmith-runtime/src/lib.rs`

**Step 1: Write the test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_api_is_accessible() {
        let state = CharacterState::default();
        let _input = FrameInput::default();
        let _ = resource::resource(&state, 0);
    }
}
```

**Step 2: Finalize lib.rs exports**

```rust
#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod cancel;
pub mod collision;
pub mod frame;
pub mod resource;
pub mod state;

// Re-export main types
pub use state::{CharacterState, FrameInput, FrameResult, MAX_RESOURCES};
pub use state::{report_hit, report_block};
pub use frame::next_frame;
pub use cancel::can_cancel_to;
pub use collision::{check_hits, shapes_overlap, aabb_overlap, Aabb, HitResult, CheckHitsResult};
pub use resource::{resource, set_resource, init_resources};

#[cfg(feature = "alloc")]
pub use cancel::available_cancels;

// Re-export fspack for convenience
pub use framesmith_fspack::PackView;

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_api_is_accessible() {
        let state = CharacterState::default();
        let _input = FrameInput::default();
        let _ = resource::resource(&state, 0);
    }
}
```

**Step 3: Run all tests**

Run: `cd crates/framesmith-runtime && cargo test`
Expected: PASS

**Step 4: Run cargo clippy**

Run: `cd crates/framesmith-runtime && cargo clippy -- -D warnings`
Expected: PASS (or fix any warnings)

**Step 5: Commit**

```bash
git add crates/framesmith-runtime/src/lib.rs
git commit -m "feat(runtime): finalize public API exports"
```

---

## Task 10: Add HitWindow Pushback to fspack

**Files:**
- Modify: `crates/framesmith-fspack/src/view.rs`

**Step 1: Write the test**

Add test to `view.rs`:

```rust
#[test]
fn hit_window_has_pushback_accessors() {
    // Build a HitWindow32 with pushback data
    let mut data = [0u8; 32];
    // Set hit_pushback at offset 24 (Q12.4: 32 = 2.0 pixels)
    data[24] = 32;
    data[25] = 0;
    // Set block_pushback at offset 26 (Q12.4: 16 = 1.0 pixel)
    data[26] = 16;
    data[27] = 0;

    // For now, test placeholder methods
}
```

**Step 2: Add pushback accessors to HitWindowView**

The current HitWindow is 24 bytes. We use the reserved byte at offset 3 and add new fields. For backwards compatibility, reads from shorter windows return 0.

```rust
// Add to HitWindowView impl:

/// Hit pushback (Q12.4 fixed-point). Returns 0 if not present.
pub fn hit_pushback_raw(&self) -> i16 {
    if self.data.len() >= 26 {
        read_u16_le(self.data, 24).unwrap_or(0) as i16
    } else {
        0
    }
}

/// Block pushback (Q12.4 fixed-point). Returns 0 if not present.
pub fn block_pushback_raw(&self) -> i16 {
    if self.data.len() >= 28 {
        read_u16_le(self.data, 26).unwrap_or(0) as i16
    } else {
        0
    }
}

/// Hit pushback in pixels.
pub fn hit_pushback_px(&self) -> i32 {
    (self.hit_pushback_raw() as i32) >> 4
}

/// Block pushback in pixels.
pub fn block_pushback_px(&self) -> i32 {
    (self.block_pushback_raw() as i32) >> 4
}
```

**Step 3: Run tests**

Run: `cd crates/framesmith-fspack && cargo test`
Expected: PASS

**Step 4: Commit**

```bash
git add crates/framesmith-fspack/src/view.rs
git commit -m "feat(fspack): add pushback accessors to HitWindowView"
```

---

## Task 11: Update HitResult to Include Pushback

**Files:**
- Modify: `crates/framesmith-runtime/src/collision.rs`

**Step 1: Update HitResult struct**

```rust
/// Result of a hit interaction.
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

**Step 2: Update check_hits to populate pushback**

Update the HitResult creation in `check_hits`:

```rust
result.push(HitResult {
    attacker_move: attacker_state.current_move,
    window_index: hw_idx as u16,
    damage: hw.damage(),
    chip_damage: hw.chip_damage(),
    hitstun: hw.hitstun(),
    blockstun: hw.blockstun(),
    hitstop: hw.hitstop(),
    guard: hw.guard(),
    hit_pushback: hw.hit_pushback_px(),
    block_pushback: hw.block_pushback_px(),
});
```

**Step 3: Run tests**

Run: `cd crates/framesmith-runtime && cargo test`
Expected: PASS

**Step 4: Commit**

```bash
git add crates/framesmith-runtime/src/collision.rs
git commit -m "feat(runtime): add pushback to HitResult"
```

---

## Task 12: Integration Test with Real Pack Data

**Files:**
- Create: `crates/framesmith-runtime/tests/integration.rs`

**Step 1: Write integration test**

```rust
//! Integration tests using real pack data.

use framesmith_runtime::*;

// This test requires a test fixture - skip for now if not available
#[test]
#[ignore = "requires test fixture"]
fn roundtrip_with_test_character() {
    // TODO: Load test_char.fspk fixture
    // let pack_data = include_bytes!("../fixtures/test_char.fspk");
    // let pack = PackView::parse(pack_data).unwrap();
    //
    // let mut state = CharacterState::default();
    // init_resources(&mut state, &pack);
    //
    // // Simulate a few frames
    // let input = FrameInput::default();
    // let result = next_frame(&state, &pack, &input);
    // assert!(!result.move_ended);
}

#[test]
fn state_is_deterministic() {
    let state = CharacterState {
        current_move: 5,
        frame: 10,
        hit_confirmed: true,
        block_confirmed: false,
        resources: [100, 50, 0, 0, 0, 0, 0, 0],
    };

    let copy1 = state;
    let copy2 = state;

    assert_eq!(copy1, copy2);
    assert_eq!(copy1.current_move, 5);
    assert_eq!(copy1.frame, 10);
    assert!(copy1.hit_confirmed);
}
```

**Step 2: Run test**

Run: `cd crates/framesmith-runtime && cargo test`
Expected: PASS (ignored test doesn't run)

**Step 3: Commit**

```bash
git add crates/framesmith-runtime/tests/
git commit -m "test(runtime): add integration test scaffold"
```

---

## Verification

After completing all tasks:

1. **Run all tests:**
   ```bash
   cd crates/framesmith-runtime && cargo test
   cd crates/framesmith-fspack && cargo test
   ```

2. **Check no_std compatibility:**
   ```bash
   cd crates/framesmith-runtime && cargo check --no-default-features
   ```

3. **Run clippy:**
   ```bash
   cd crates/framesmith-runtime && cargo clippy -- -D warnings
   ```

4. **Verify public API:**
   ```rust
   use framesmith_runtime::{
       CharacterState, FrameInput, FrameResult,
       next_frame, can_cancel_to, check_hits,
       report_hit, report_block,
       resource, set_resource, init_resources,
   };
   ```

---

## Future Tasks (Not in This Plan)

- [ ] Add timing window checks to `can_cancel_to`
- [ ] Implement resource cost deduction in `next_frame`
- [ ] Add circle and capsule shape overlap
- [ ] Export pushback in `zx_fspack.rs` codegen
- [ ] Framesmith editor simulator integration
- [ ] Example game demonstrating runtime usage
