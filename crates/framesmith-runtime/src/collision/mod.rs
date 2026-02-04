mod shapes;

// Re-export shape types and functions for backward compatibility
pub use shapes::{
    Aabb, Capsule, Circle,
    aabb_circle_overlap, aabb_overlap, capsule_overlap, circle_overlap, shapes_overlap,
};

use crate::state::CharacterState;
use framesmith_fspack::{PackView, PushWindowView};

/// Maximum number of hit results that can be stored.
pub const MAX_HIT_RESULTS: usize = 8;

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

/// Check all hitbox vs hurtbox interactions between two characters.
///
/// Returns hit results for the game to process.
#[must_use]
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
    let attacker_moves = match attacker_pack.states() {
        Some(m) => m,
        None => return result,
    };
    let attacker_move = match attacker_moves.get(attacker_state.current_state as usize) {
        Some(m) => m,
        None => return result,
    };

    // Get defender's active hurtboxes
    let defender_moves = match defender_pack.states() {
        Some(m) => m,
        None => return result,
    };
    let defender_move = match defender_moves.get(defender_state.current_state as usize) {
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
                &hw,
                &attacker_shapes,
                attacker_pos,
                &hrt,
                &defender_shapes,
                defender_pos,
            ) {
                result.push(HitResult {
                    attacker_move: attacker_state.current_state,
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
    hits: [Option<HitResult>; MAX_HIT_RESULTS],
    count: usize,
}

impl CheckHitsResult {
    pub fn new() -> Self {
        Self {
            hits: [None; MAX_HIT_RESULTS],
            count: 0,
        }
    }

    pub fn push(&mut self, hit: HitResult) {
        if self.count < MAX_HIT_RESULTS {
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

/// Result of pushbox collision check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PushboxResult {
    /// Separation to apply to player 1 (negative = move left, positive = move right).
    pub p1_dx: i32,
    /// Separation to apply to player 2 (negative = move left, positive = move right).
    pub p2_dx: i32,
}

/// Calculate pushbox separation for two overlapping AABBs.
///
/// Returns `None` if the AABBs don't overlap.
/// Returns `Some(PushboxResult)` with separation values if they do overlap.
///
/// The separation is calculated to push characters apart, splitting the overlap
/// equally between them. The direction is determined by center positions:
/// - If P1's center is left of P2's center, P1 is pushed left and P2 right
/// - If P1's center is right of P2's center, P1 is pushed right and P2 left
#[must_use]
pub fn calculate_pushbox_separation(p1_aabb: &Aabb, p2_aabb: &Aabb) -> Option<PushboxResult> {
    // Check if they overlap
    if !aabb_overlap(p1_aabb, p2_aabb) {
        return None;
    }

    // Calculate overlap amount on X axis
    let p1_right = p1_aabb.x.saturating_add(p1_aabb.w as i32);
    let p2_right = p2_aabb.x.saturating_add(p2_aabb.w as i32);

    // Determine which side to push to (based on center positions)
    let p1_center = p1_aabb.x.saturating_add((p1_aabb.w / 2) as i32);
    let p2_center = p2_aabb.x.saturating_add((p2_aabb.w / 2) as i32);

    // Calculate horizontal overlap
    let overlap_x = if p1_center <= p2_center {
        // P1 is to the left of P2
        // Overlap is how far P1's right edge extends past P2's left edge
        p1_right.saturating_sub(p2_aabb.x)
    } else {
        // P1 is to the right of P2
        // Overlap is how far P2's right edge extends past P1's left edge (negative to push P1 right)
        -(p2_right.saturating_sub(p1_aabb.x))
    };

    // Split the overlap between both characters (half each)
    // P1 gets pushed left (negative) if they're overlapping from the left
    // P2 gets pushed right (positive) if they're overlapping from the left
    let half_overlap = overlap_x / 2;
    let remainder = overlap_x % 2;

    Some(PushboxResult {
        p1_dx: -(half_overlap + remainder), // P1 moves opposite to overlap direction
        p2_dx: half_overlap,                 // P2 moves in overlap direction
    })
}

/// Find the active push window for a character at the given frame.
fn find_active_push_window<'a>(
    state: &CharacterState,
    pack: &'a PackView<'a>,
) -> Option<PushWindowView<'a>> {
    let states = pack.states()?;
    let move_state = states.get(state.current_state as usize)?;
    let push_windows = pack.push_windows()?;

    let frame = state.frame;

    // Iterate through push windows for this state and find one active this frame
    for idx in 0..move_state.push_windows_len() as usize {
        let pw = push_windows.get_at(move_state.push_windows_off(), idx)?;
        if frame >= pw.start_frame() && frame <= pw.end_frame() {
            return Some(pw);
        }
    }
    None
}

/// Get the AABB for a push window at a given position.
fn get_pushbox_aabb(
    push_window: &PushWindowView,
    shapes: &framesmith_fspack::ShapesView,
    pos: (i32, i32),
) -> Option<Aabb> {
    // Push windows typically have one shape (the pushbox), but we support multiple
    // by computing the bounding box that encompasses all shapes.
    if push_window.shapes_len() == 0 {
        return None;
    }

    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;

    for i in 0..push_window.shapes_len() as usize {
        let shape = shapes.get_at(push_window.shapes_off(), i)?;
        let aabb = Aabb::from_shape(&shape, pos.0, pos.1);

        min_x = min_x.min(aabb.x);
        min_y = min_y.min(aabb.y);
        max_x = max_x.max(aabb.x.saturating_add(aabb.w as i32));
        max_y = max_y.max(aabb.y.saturating_add(aabb.h as i32));
    }

    if min_x == i32::MAX {
        return None;
    }

    Some(Aabb {
        x: min_x,
        y: min_y,
        w: (max_x - min_x).max(0) as u32,
        h: (max_y - min_y).max(0) as u32,
    })
}

/// Check if two characters' pushboxes overlap and calculate separation.
///
/// Returns `None` if there is no overlap (no separation needed).
/// Returns `Some(PushboxResult)` with the separation values for each player.
///
/// The separation is calculated to push characters apart, splitting the overlap
/// equally between them. Positive values indicate rightward movement.
///
/// # Arguments
/// * `p1_state` - Player 1's character state
/// * `p1_pack` - Player 1's character pack data
/// * `p1_pos` - Player 1's position (x, y)
/// * `p2_state` - Player 2's character state
/// * `p2_pack` - Player 2's character pack data
/// * `p2_pos` - Player 2's position (x, y)
#[must_use]
pub fn check_pushbox(
    p1_state: &CharacterState,
    p1_pack: &PackView,
    p1_pos: (i32, i32),
    p2_state: &CharacterState,
    p2_pack: &PackView,
    p2_pos: (i32, i32),
) -> Option<PushboxResult> {
    // Find active push windows for both characters
    let p1_pw = find_active_push_window(p1_state, p1_pack)?;
    let p2_pw = find_active_push_window(p2_state, p2_pack)?;

    // Get shapes sections
    let p1_shapes = p1_pack.shapes()?;
    let p2_shapes = p2_pack.shapes()?;

    // Get AABBs for both pushboxes
    let p1_aabb = get_pushbox_aabb(&p1_pw, &p1_shapes, p1_pos)?;
    let p2_aabb = get_pushbox_aabb(&p2_pw, &p2_shapes, p2_pos)?;

    // Delegate to the separation calculation helper
    calculate_pushbox_separation(&p1_aabb, &p2_aabb)
}

#[cfg(test)]
mod tests {
    use super::*;

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
                hit_pushback: 0,
                block_pushback: 0,
            });
        }

        // Should cap at 8
        assert_eq!(result.len(), 8);
    }

    // ==========================================================================
    // Pushbox separation tests
    // ==========================================================================

    #[test]
    fn pushbox_separation_returns_none_when_no_overlap() {
        // Two AABBs that don't overlap
        let p1 = Aabb { x: 0, y: 0, w: 20, h: 40 };
        let p2 = Aabb { x: 50, y: 0, w: 20, h: 40 };

        let result = calculate_pushbox_separation(&p1, &p2);
        assert!(result.is_none());
    }

    #[test]
    fn pushbox_separation_returns_none_when_edge_touching() {
        // Two AABBs exactly touching (no overlap)
        let p1 = Aabb { x: 0, y: 0, w: 20, h: 40 };
        let p2 = Aabb { x: 20, y: 0, w: 20, h: 40 };

        let result = calculate_pushbox_separation(&p1, &p2);
        assert!(result.is_none());
    }

    #[test]
    fn pushbox_separation_p1_left_of_p2_even_overlap() {
        // P1 (center=10) is left of P2 (center=25)
        // P1: x=0..20, P2: x=15..35
        // Overlap = P1's right (20) - P2's left (15) = 5
        // Half = 2, remainder = 1
        // P1 moves left by -(2+1) = -3, P2 moves right by 2
        let p1 = Aabb { x: 0, y: 0, w: 20, h: 40 };
        let p2 = Aabb { x: 15, y: 0, w: 20, h: 40 };

        let result = calculate_pushbox_separation(&p1, &p2);
        assert!(result.is_some());
        let sep = result.unwrap();

        // P1 should move left (negative), P2 should move right (positive)
        assert!(sep.p1_dx < 0, "P1 should move left, got {}", sep.p1_dx);
        assert!(sep.p2_dx >= 0, "P2 should move right, got {}", sep.p2_dx);

        // Total separation should equal the overlap (5 pixels)
        assert_eq!(
            sep.p1_dx.abs() + sep.p2_dx.abs(),
            5,
            "Total separation should equal overlap"
        );
    }

    #[test]
    fn pushbox_separation_p1_right_of_p2() {
        // P1 (center=25) is right of P2 (center=10)
        // P1: x=15..35, P2: x=0..20
        // Overlap = P2's right (20) - P1's left (15) = 5, negated = -5
        // Half = -2, remainder = -1
        // P1 moves right by -(-2-1) = 3, P2 moves left by -2
        let p1 = Aabb { x: 15, y: 0, w: 20, h: 40 };
        let p2 = Aabb { x: 0, y: 0, w: 20, h: 40 };

        let result = calculate_pushbox_separation(&p1, &p2);
        assert!(result.is_some());
        let sep = result.unwrap();

        // P1 should move right (positive), P2 should move left (negative)
        assert!(sep.p1_dx > 0, "P1 should move right, got {}", sep.p1_dx);
        assert!(sep.p2_dx <= 0, "P2 should move left, got {}", sep.p2_dx);

        // Total separation should equal the overlap (5 pixels)
        assert_eq!(
            sep.p1_dx.abs() + sep.p2_dx.abs(),
            5,
            "Total separation should equal overlap"
        );
    }

    #[test]
    fn pushbox_separation_perfectly_overlapping() {
        // Two AABBs with identical position and size
        // Centers are equal, so P1 is considered "left" (<=)
        // Overlap = full width = 20
        let p1 = Aabb { x: 0, y: 0, w: 20, h: 40 };
        let p2 = Aabb { x: 0, y: 0, w: 20, h: 40 };

        let result = calculate_pushbox_separation(&p1, &p2);
        assert!(result.is_some());
        let sep = result.unwrap();

        // P1 should move left (negative), P2 should move right (positive)
        assert!(sep.p1_dx < 0, "P1 should move left, got {}", sep.p1_dx);
        assert!(sep.p2_dx >= 0, "P2 should move right, got {}", sep.p2_dx);

        // Total separation should equal the overlap (20 pixels)
        assert_eq!(
            sep.p1_dx.abs() + sep.p2_dx.abs(),
            20,
            "Total separation should equal full overlap"
        );
    }

    #[test]
    fn pushbox_separation_small_overlap() {
        // Minimal overlap of 1 pixel
        // P1: x=0..20, P2: x=19..39
        // P1 center = 10, P2 center = 29
        // Overlap = 20 - 19 = 1
        // Half = 0, remainder = 1
        // P1 moves -1, P2 moves 0
        let p1 = Aabb { x: 0, y: 0, w: 20, h: 40 };
        let p2 = Aabb { x: 19, y: 0, w: 20, h: 40 };

        let result = calculate_pushbox_separation(&p1, &p2);
        assert!(result.is_some());
        let sep = result.unwrap();

        assert_eq!(sep.p1_dx, -1, "P1 should move -1");
        assert_eq!(sep.p2_dx, 0, "P2 should move 0");
    }

    #[test]
    fn pushbox_separation_even_split() {
        // Overlap that divides evenly (6 pixels)
        // P1: x=0..20, P2: x=14..34
        // P1 center = 10, P2 center = 24
        // Overlap = 20 - 14 = 6
        // Half = 3, remainder = 0
        // P1 moves -3, P2 moves 3
        let p1 = Aabb { x: 0, y: 0, w: 20, h: 40 };
        let p2 = Aabb { x: 14, y: 0, w: 20, h: 40 };

        let result = calculate_pushbox_separation(&p1, &p2);
        assert!(result.is_some());
        let sep = result.unwrap();

        assert_eq!(sep.p1_dx, -3, "P1 should move -3");
        assert_eq!(sep.p2_dx, 3, "P2 should move 3");
    }

    #[test]
    fn pushbox_result_struct_equality() {
        let a = PushboxResult { p1_dx: -5, p2_dx: 5 };
        let b = PushboxResult { p1_dx: -5, p2_dx: 5 };
        let c = PushboxResult { p1_dx: -4, p2_dx: 5 };

        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn pushbox_separation_only_y_overlap_returns_none() {
        // AABBs that only overlap on Y axis but not X axis
        let p1 = Aabb { x: 0, y: 0, w: 20, h: 40 };
        let p2 = Aabb { x: 30, y: 10, w: 20, h: 40 };

        let result = calculate_pushbox_separation(&p1, &p2);
        assert!(result.is_none());
    }
}
