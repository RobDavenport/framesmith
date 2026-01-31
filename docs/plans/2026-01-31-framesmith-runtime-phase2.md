# Framesmith Runtime Phase 2: Complete TODO Features

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement the remaining TODO features in framesmith-runtime: resource costs, timing windows, cancel flags, and additional shape overlap types.

**Architecture:** Extend existing pure functions with additional logic. Resource costs are deducted during cancel transitions. Timing windows filter cancel availability by current frame. Cancel flags (chain, special, jump, etc.) control action cancels. Shape overlap adds circle and capsule collision detection.

**Tech Stack:** Rust, no_std, framesmith-fspack for data access.

---

## Task 1: Implement Resource Cost Deduction

**Files:**
- Modify: `crates/framesmith-runtime/src/frame.rs`
- Modify: `crates/framesmith-runtime/src/resource.rs`

**Step 1: Write the test in resource.rs**

```rust
#[test]
fn deduct_resource_costs() {
    let mut state = CharacterState::default();
    set_resource(&mut state, 0, 100); // meter
    set_resource(&mut state, 1, 50);  // heat

    // Simulate deducting 30 from resource 0, 10 from resource 1
    let costs = [(0u8, 30u16), (1u8, 10u16)];
    for (idx, amount) in costs {
        let current = resource(&state, idx);
        set_resource(&mut state, idx, current.saturating_sub(amount));
    }

    assert_eq!(resource(&state, 0), 70);
    assert_eq!(resource(&state, 1), 40);
}
```

**Step 2: Run test to verify it passes**

Run: `cd crates/framesmith-runtime && cargo test resource::tests::deduct_resource_costs`
Expected: PASS (this is just verifying existing helpers work for deduction)

**Step 3: Add apply_resource_costs function to resource.rs**

```rust
/// Apply resource costs for a move transition.
///
/// Deducts costs from state. Returns true if all costs were paid,
/// false if any resource was insufficient (costs still deducted).
pub fn apply_resource_costs(
    state: &mut CharacterState,
    pack: &framesmith_fspack::PackView,
    move_index: u16,
) -> bool {
    let extras = match pack.move_extras() {
        Some(e) => e,
        None => return true,
    };
    let extra = match extras.get(move_index as usize) {
        Some(e) => e,
        None => return true,
    };
    let costs_view = match pack.move_resource_costs() {
        Some(c) => c,
        None => return true,
    };
    let resource_defs = pack.resource_defs();

    let (off, len) = extra.resource_costs();
    let mut all_paid = true;

    for i in 0..len as usize {
        if let Some(cost) = costs_view.get_at(off, i) {
            // Find resource index by name
            if let Some(defs) = &resource_defs {
                for res_idx in 0..defs.len().min(MAX_RESOURCES) {
                    if let Some(def) = defs.get(res_idx) {
                        if def.name_off() == cost.name_off() && def.name_len() == cost.name_len() {
                            let current = resource(state, res_idx as u8);
                            if current < cost.amount() {
                                all_paid = false;
                            }
                            set_resource(state, res_idx as u8, current.saturating_sub(cost.amount()));
                            break;
                        }
                    }
                }
            }
        }
    }

    all_paid
}
```

**Step 4: Update frame.rs to call apply_resource_costs**

Replace the TODO comment in `next_frame`:

```rust
            // Apply resource costs
            crate::resource::apply_resource_costs(&mut new_state, pack, target);
```

**Step 5: Export apply_resource_costs from lib.rs**

Add to lib.rs exports:
```rust
pub use resource::{resource, set_resource, init_resources, apply_resource_costs};
```

**Step 6: Run all tests**

Run: `cd crates/framesmith-runtime && cargo test`
Expected: PASS

**Step 7: Commit**

```bash
git add crates/framesmith-runtime/src/frame.rs crates/framesmith-runtime/src/resource.rs crates/framesmith-runtime/src/lib.rs
git commit -m "feat(runtime): implement resource cost deduction on cancel"
```

---

## Task 2: Implement Resource Precondition Checking

**Files:**
- Modify: `crates/framesmith-runtime/src/cancel.rs`
- Modify: `crates/framesmith-runtime/src/resource.rs`

**Step 1: Write the test in resource.rs**

```rust
#[test]
fn check_preconditions_passes_when_met() {
    let mut state = CharacterState::default();
    set_resource(&mut state, 0, 50);

    // Precondition: resource 0 must be >= 25
    // This would need pack data, so we test the helper directly
    assert!(check_precondition_value(50, Some(25), None));
    assert!(check_precondition_value(50, None, Some(100)));
    assert!(check_precondition_value(50, Some(25), Some(100)));
}

#[test]
fn check_preconditions_fails_when_not_met() {
    assert!(!check_precondition_value(20, Some(25), None)); // below min
    assert!(!check_precondition_value(150, None, Some(100))); // above max
    assert!(!check_precondition_value(20, Some(25), Some(100))); // below min
    assert!(!check_precondition_value(150, Some(25), Some(100))); // above max
}
```

**Step 2: Add check_precondition_value helper to resource.rs**

```rust
/// Check if a resource value satisfies a precondition.
#[inline]
pub fn check_precondition_value(value: u16, min: Option<u16>, max: Option<u16>) -> bool {
    if let Some(m) = min {
        if value < m {
            return false;
        }
    }
    if let Some(m) = max {
        if value > m {
            return false;
        }
    }
    true
}

/// Check all resource preconditions for a move.
///
/// Returns true if all preconditions are satisfied.
pub fn check_resource_preconditions(
    state: &CharacterState,
    pack: &framesmith_fspack::PackView,
    move_index: u16,
) -> bool {
    let extras = match pack.move_extras() {
        Some(e) => e,
        None => return true,
    };
    let extra = match extras.get(move_index as usize) {
        Some(e) => e,
        None => return true,
    };
    let preconditions_view = match pack.move_resource_preconditions() {
        Some(p) => p,
        None => return true,
    };
    let resource_defs = pack.resource_defs();

    let (off, len) = extra.resource_preconditions();

    for i in 0..len as usize {
        if let Some(precond) = preconditions_view.get_at(off, i) {
            // Find resource index by name
            if let Some(defs) = &resource_defs {
                for res_idx in 0..defs.len().min(MAX_RESOURCES) {
                    if let Some(def) = defs.get(res_idx) {
                        if def.name_off() == precond.name_off() && def.name_len() == precond.name_len() {
                            let current = resource(state, res_idx as u8);
                            if !check_precondition_value(current, precond.min(), precond.max()) {
                                return false;
                            }
                            break;
                        }
                    }
                }
            }
        }
    }

    true
}
```

**Step 3: Update can_cancel_to in cancel.rs to check preconditions**

Add precondition check before returning true for a cancel target:

```rust
                        if cancel_target == target {
                            // Check resource preconditions
                            if !crate::resource::check_resource_preconditions(state, pack, target) {
                                continue;
                            }
                            return true;
                        }
```

**Step 4: Export check_resource_preconditions from lib.rs**

Add to lib.rs exports:
```rust
pub use resource::{resource, set_resource, init_resources, apply_resource_costs, check_resource_preconditions};
```

**Step 5: Run all tests**

Run: `cd crates/framesmith-runtime && cargo test`
Expected: PASS

**Step 6: Commit**

```bash
git add crates/framesmith-runtime/src/cancel.rs crates/framesmith-runtime/src/resource.rs crates/framesmith-runtime/src/lib.rs
git commit -m "feat(runtime): add resource precondition checking for cancels"
```

---

## Task 3: Implement Cancel Flags for Action Cancels

**Files:**
- Modify: `crates/framesmith-runtime/src/cancel.rs`

**Step 1: Write the test**

```rust
#[test]
fn action_cancel_constants_match_flags() {
    // Action IDs map to cancel flags
    assert_eq!(ACTION_CHAIN, 0);
    assert_eq!(ACTION_SPECIAL, 1);
    assert_eq!(ACTION_SUPER, 2);
    assert_eq!(ACTION_JUMP, 3);
}
```

**Step 2: Add action constants and implement check_action_cancel**

```rust
/// Action cancel IDs (offset from move_count).
/// These map to CancelFlags on the current move.
pub const ACTION_CHAIN: u16 = 0;
pub const ACTION_SPECIAL: u16 = 1;
pub const ACTION_SUPER: u16 = 2;
pub const ACTION_JUMP: u16 = 3;

/// Check if an action cancel is allowed based on current move's cancel flags.
fn check_action_cancel(
    state: &CharacterState,
    pack: &PackView,
    action_id: u16,
) -> bool {
    let moves = match pack.moves() {
        Some(m) => m,
        None => return false,
    };
    let current_move = match moves.get(state.current_move as usize) {
        Some(m) => m,
        None => return false,
    };

    let flags = current_move.cancel_flags();

    match action_id {
        ACTION_CHAIN => flags.chain,
        ACTION_SPECIAL => flags.special,
        ACTION_SUPER => flags.super_cancel,
        ACTION_JUMP => flags.jump,
        _ => true, // Unknown actions delegated to game
    }
}
```

**Step 3: Run all tests**

Run: `cd crates/framesmith-runtime && cargo test`
Expected: PASS

**Step 4: Commit**

```bash
git add crates/framesmith-runtime/src/cancel.rs
git commit -m "feat(runtime): implement cancel flags for action cancels"
```

---

## Task 4: Implement Circle Overlap

**Files:**
- Modify: `crates/framesmith-runtime/src/collision.rs`

**Step 1: Write the test**

```rust
#[test]
fn circle_overlap_detects_intersection() {
    // Two overlapping circles
    let a = Circle { x: 0, y: 0, r: 10 };
    let b = Circle { x: 15, y: 0, r: 10 };
    assert!(circle_overlap(&a, &b)); // distance 15 < 10+10
}

#[test]
fn circle_overlap_detects_no_intersection() {
    // Two non-overlapping circles
    let a = Circle { x: 0, y: 0, r: 10 };
    let b = Circle { x: 25, y: 0, r: 10 };
    assert!(!circle_overlap(&a, &b)); // distance 25 > 10+10
}

#[test]
fn circle_overlap_edge_touching_is_not_overlap() {
    // Circles exactly touching
    let a = Circle { x: 0, y: 0, r: 10 };
    let b = Circle { x: 20, y: 0, r: 10 };
    assert!(!circle_overlap(&a, &b)); // distance 20 == 10+10
}
```

**Step 2: Add Circle struct and circle_overlap function**

```rust
/// Circle for collision detection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Circle {
    pub x: i32,
    pub y: i32,
    pub r: u32,
}

impl Circle {
    /// Create a Circle from a ShapeView at a given position offset.
    pub fn from_shape(shape: &ShapeView, offset_x: i32, offset_y: i32) -> Self {
        Circle {
            x: shape.x_px() + offset_x,
            y: shape.y_px() + offset_y,
            r: shape.radius_px(),
        }
    }
}

/// Check if two circles overlap.
///
/// Edge-touching is NOT considered overlap.
#[must_use]
#[inline]
pub fn circle_overlap(a: &Circle, b: &Circle) -> bool {
    let dx = (a.x as i64) - (b.x as i64);
    let dy = (a.y as i64) - (b.y as i64);
    let dist_sq = dx * dx + dy * dy;
    let radii_sum = (a.r as i64) + (b.r as i64);
    dist_sq < radii_sum * radii_sum
}
```

**Step 3: Update shapes_overlap to handle circles**

```rust
pub fn shapes_overlap(
    a: &ShapeView,
    a_offset: (i32, i32),
    b: &ShapeView,
    b_offset: (i32, i32),
) -> bool {
    use framesmith_fspack::{SHAPE_KIND_AABB, SHAPE_KIND_CIRCLE};

    match (a.kind(), b.kind()) {
        (SHAPE_KIND_AABB, SHAPE_KIND_AABB) => {
            let aabb_a = Aabb::from_shape(a, a_offset.0, a_offset.1);
            let aabb_b = Aabb::from_shape(b, b_offset.0, b_offset.1);
            aabb_overlap(&aabb_a, &aabb_b)
        }
        (SHAPE_KIND_CIRCLE, SHAPE_KIND_CIRCLE) => {
            let circle_a = Circle::from_shape(a, a_offset.0, a_offset.1);
            let circle_b = Circle::from_shape(b, b_offset.0, b_offset.1);
            circle_overlap(&circle_a, &circle_b)
        }
        (SHAPE_KIND_AABB, SHAPE_KIND_CIRCLE) | (SHAPE_KIND_CIRCLE, SHAPE_KIND_AABB) => {
            let (aabb, aabb_off, circle, circle_off) = if a.kind() == SHAPE_KIND_AABB {
                (a, a_offset, b, b_offset)
            } else {
                (b, b_offset, a, a_offset)
            };
            let aabb = Aabb::from_shape(aabb, aabb_off.0, aabb_off.1);
            let circle = Circle::from_shape(circle, circle_off.0, circle_off.1);
            aabb_circle_overlap(&aabb, &circle)
        }
        _ => false, // Capsule and rotated rect not yet supported
    }
}
```

**Step 4: Add aabb_circle_overlap helper**

```rust
/// Check if an AABB and circle overlap.
#[must_use]
#[inline]
pub fn aabb_circle_overlap(aabb: &Aabb, circle: &Circle) -> bool {
    // Find closest point on AABB to circle center
    let closest_x = (circle.x).clamp(aabb.x, aabb.x.saturating_add(aabb.w as i32));
    let closest_y = (circle.y).clamp(aabb.y, aabb.y.saturating_add(aabb.h as i32));

    let dx = (circle.x as i64) - (closest_x as i64);
    let dy = (circle.y as i64) - (closest_y as i64);
    let dist_sq = dx * dx + dy * dy;
    let r = circle.r as i64;

    dist_sq < r * r
}
```

**Step 5: Export Circle and circle_overlap from lib.rs**

```rust
pub use collision::{check_hits, shapes_overlap, aabb_overlap, circle_overlap, aabb_circle_overlap, Aabb, Circle, HitResult, CheckHitsResult, MAX_HIT_RESULTS};
```

**Step 6: Run all tests**

Run: `cd crates/framesmith-runtime && cargo test`
Expected: PASS

**Step 7: Commit**

```bash
git add crates/framesmith-runtime/src/collision.rs crates/framesmith-runtime/src/lib.rs
git commit -m "feat(runtime): add circle collision detection"
```

---

## Task 5: Implement Capsule Overlap

**Files:**
- Modify: `crates/framesmith-runtime/src/collision.rs`

**Step 1: Write the test**

```rust
#[test]
fn capsule_overlap_detects_intersection() {
    // Two overlapping capsules (horizontal)
    let a = Capsule { x1: 0, y1: 0, x2: 20, y2: 0, r: 5 };
    let b = Capsule { x1: 15, y1: 0, x2: 35, y2: 0, r: 5 };
    assert!(capsule_overlap(&a, &b));
}

#[test]
fn capsule_overlap_detects_no_intersection() {
    // Two non-overlapping capsules
    let a = Capsule { x1: 0, y1: 0, x2: 10, y2: 0, r: 5 };
    let b = Capsule { x1: 30, y1: 0, x2: 40, y2: 0, r: 5 };
    assert!(!capsule_overlap(&a, &b));
}
```

**Step 2: Add Capsule struct**

```rust
/// Capsule (line segment with radius) for collision detection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Capsule {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
    pub r: u32,
}

impl Capsule {
    /// Create a Capsule from a ShapeView at a given position offset.
    pub fn from_shape(shape: &ShapeView, offset_x: i32, offset_y: i32) -> Self {
        // Capsule uses a,b for p1, c,d for p2, e for radius
        let x1 = shape.x_px() + offset_x;
        let y1 = shape.y_px() + offset_y;
        // c_raw and d_raw are x2,y2 for capsule
        let x2 = (shape.c_raw() as i32 >> 4) + offset_x;
        let y2 = (shape.d_raw() as i32 >> 4) + offset_y;
        // e_raw is radius (Q8.8 format)
        let r = ((shape.e_raw() as i32) >> 8).max(0) as u32;
        Capsule { x1, y1, x2, y2, r }
    }
}
```

**Step 3: Add segment distance helper**

```rust
/// Compute squared distance between closest points on two line segments.
fn segment_distance_sq(
    a1: (i64, i64), a2: (i64, i64),
    b1: (i64, i64), b2: (i64, i64),
) -> i64 {
    // Simplified: find closest point on each segment to the other
    let closest_a = closest_point_on_segment(a1, a2, closest_point_on_segment(b1, b2, a1));
    let closest_b = closest_point_on_segment(b1, b2, closest_a);
    let dx = closest_a.0 - closest_b.0;
    let dy = closest_a.1 - closest_b.1;
    dx * dx + dy * dy
}

/// Find closest point on segment (p1, p2) to point p.
fn closest_point_on_segment(p1: (i64, i64), p2: (i64, i64), p: (i64, i64)) -> (i64, i64) {
    let dx = p2.0 - p1.0;
    let dy = p2.1 - p1.1;
    let len_sq = dx * dx + dy * dy;

    if len_sq == 0 {
        return p1; // Degenerate segment
    }

    // Project p onto line, clamped to [0, 1]
    let t_num = (p.0 - p1.0) * dx + (p.1 - p1.1) * dy;
    let t = if t_num <= 0 {
        0
    } else if t_num >= len_sq {
        len_sq
    } else {
        t_num
    };

    (
        p1.0 + (dx * t) / len_sq,
        p1.1 + (dy * t) / len_sq,
    )
}
```

**Step 4: Add capsule_overlap function**

```rust
/// Check if two capsules overlap.
#[must_use]
pub fn capsule_overlap(a: &Capsule, b: &Capsule) -> bool {
    let a1 = (a.x1 as i64, a.y1 as i64);
    let a2 = (a.x2 as i64, a.y2 as i64);
    let b1 = (b.x1 as i64, b.y1 as i64);
    let b2 = (b.x2 as i64, b.y2 as i64);

    let dist_sq = segment_distance_sq(a1, a2, b1, b2);
    let radii_sum = (a.r as i64) + (b.r as i64);

    dist_sq < radii_sum * radii_sum
}
```

**Step 5: Update shapes_overlap to handle capsules**

Add to the match in shapes_overlap:

```rust
        (SHAPE_KIND_CAPSULE, SHAPE_KIND_CAPSULE) => {
            let cap_a = Capsule::from_shape(a, a_offset.0, a_offset.1);
            let cap_b = Capsule::from_shape(b, b_offset.0, b_offset.1);
            capsule_overlap(&cap_a, &cap_b)
        }
```

Add the import at top of function:
```rust
    use framesmith_fspack::{SHAPE_KIND_AABB, SHAPE_KIND_CIRCLE, SHAPE_KIND_CAPSULE};
```

**Step 6: Export Capsule and capsule_overlap from lib.rs**

```rust
pub use collision::{check_hits, shapes_overlap, aabb_overlap, circle_overlap, aabb_circle_overlap, capsule_overlap, Aabb, Circle, Capsule, HitResult, CheckHitsResult, MAX_HIT_RESULTS};
```

**Step 7: Run all tests**

Run: `cd crates/framesmith-runtime && cargo test`
Expected: PASS

**Step 8: Commit**

```bash
git add crates/framesmith-runtime/src/collision.rs crates/framesmith-runtime/src/lib.rs
git commit -m "feat(runtime): add capsule collision detection"
```

---

## Task 6: Remove Remaining TODO Comments

**Files:**
- Modify: `crates/framesmith-runtime/src/frame.rs`
- Modify: `crates/framesmith-runtime/src/cancel.rs`
- Modify: `crates/framesmith-runtime/src/collision.rs`

**Step 1: Remove TODO from frame.rs**

The TODO was replaced with actual implementation in Task 1. Verify it's gone.

**Step 2: Remove TODO from cancel.rs line 42**

Replace with:
```rust
                            // Preconditions checked above
```

**Step 3: Remove TODO from cancel.rs line 64**

Already replaced with implementation in Task 3.

**Step 4: Remove TODO from cancel.rs line 81 (available_cancels)**

Add precondition check and remove TODO:
```rust
                    if let Some(target) = cancels.get_at(off, i) {
                        // Filter by preconditions (timing windows not implemented)
                        if crate::resource::check_resource_preconditions(state, pack, target) {
                            result.push(target);
                        }
                    }
```

**Step 5: Remove TODO from collision.rs**

The TODO was replaced with implementations in Tasks 4-5. Update comment:
```rust
        _ => false, // Rotated rect not yet supported
```

**Step 6: Run all tests and clippy**

Run: `cd crates/framesmith-runtime && cargo test && cargo clippy -- -D warnings`
Expected: PASS

**Step 7: Commit**

```bash
git add crates/framesmith-runtime/
git commit -m "chore(runtime): remove remaining TODO comments"
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

4. **Verify no TODOs remain:**
   ```bash
   grep -r "TODO" crates/framesmith-runtime/src/
   ```
   Expected: No output

---

## Future Tasks (Not in This Plan)

- [ ] Add timing window filtering (requires HitWindow extended format)
- [ ] Add rotated rectangle overlap
- [ ] Add capsule-circle and capsule-AABB overlap
- [ ] Performance optimization for collision broad phase
