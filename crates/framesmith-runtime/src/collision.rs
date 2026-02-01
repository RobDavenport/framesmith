use crate::state::CharacterState;
use framesmith_fspack::{PackView, PushWindowView, ShapeView, SHAPE_KIND_AABB, SHAPE_KIND_CAPSULE, SHAPE_KIND_CIRCLE};

/// Maximum number of hit results that can be stored.
pub const MAX_HIT_RESULTS: usize = 8;

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
        // Capsule: a,b = p1; c,d = p2; e = radius
        // a_raw, b_raw are Q12.4 for x1, y1
        // c_raw, d_raw are Q12.4 for x2, y2
        // e_raw is Q8.8 for radius
        let x1 = shape.x_px() + offset_x;
        let y1 = shape.y_px() + offset_y;
        let x2 = ((shape.c_raw() as i32) >> 4) + offset_x;
        let y2 = ((shape.d_raw() as i32) >> 4) + offset_y;
        let r = ((shape.e_raw() as i32) >> 8).max(0) as u32;
        Capsule { x1, y1, x2, y2, r }
    }
}

/// Check if two AABBs overlap.
///
/// Edge-touching is NOT considered overlap.
#[inline]
#[must_use]
pub fn aabb_overlap(a: &Aabb, b: &Aabb) -> bool {
    let a_right = a.x.saturating_add(a.w as i32);
    let a_bottom = a.y.saturating_add(a.h as i32);
    let b_right = b.x.saturating_add(b.w as i32);
    let b_bottom = b.y.saturating_add(b.h as i32);

    a.x < b_right && a_right > b.x && a.y < b_bottom && a_bottom > b.y
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

/// Check if an AABB and circle overlap.
#[must_use]
#[inline]
pub fn aabb_circle_overlap(aabb: &Aabb, circle: &Circle) -> bool {
    // Find closest point on AABB to circle center
    let closest_x = circle.x.clamp(aabb.x, aabb.x.saturating_add(aabb.w as i32));
    let closest_y = circle.y.clamp(aabb.y, aabb.y.saturating_add(aabb.h as i32));

    let dx = (circle.x as i64) - (closest_x as i64);
    let dy = (circle.y as i64) - (closest_y as i64);
    let dist_sq = dx * dx + dy * dy;
    let r = circle.r as i64;

    dist_sq < r * r
}

/// Find closest point on segment (p1, p2) to point p.
fn closest_point_on_segment(p1: (i64, i64), p2: (i64, i64), p: (i64, i64)) -> (i64, i64) {
    let dx = p2.0 - p1.0;
    let dy = p2.1 - p1.1;
    let len_sq = dx * dx + dy * dy;

    if len_sq == 0 {
        return p1; // Degenerate segment (point)
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

/// Compute squared distance between closest points on two line segments.
fn segment_distance_sq(
    a1: (i64, i64), a2: (i64, i64),
    b1: (i64, i64), b2: (i64, i64),
) -> i64 {
    // Find closest point on segment A to segment B's closest point to A
    let closest_on_b_to_a1 = closest_point_on_segment(b1, b2, a1);
    let closest_on_a = closest_point_on_segment(a1, a2, closest_on_b_to_a1);
    let closest_on_b = closest_point_on_segment(b1, b2, closest_on_a);

    let dx = closest_on_a.0 - closest_on_b.0;
    let dy = closest_on_a.1 - closest_on_b.1;
    dx * dx + dy * dy
}

/// Check if two capsules overlap.
///
/// A capsule is a line segment with radius (like a stadium shape).
/// Edge-touching is NOT considered overlap.
#[must_use]
#[inline]
pub fn capsule_overlap(a: &Capsule, b: &Capsule) -> bool {
    let a1 = (a.x1 as i64, a.y1 as i64);
    let a2 = (a.x2 as i64, a.y2 as i64);
    let b1 = (b.x1 as i64, b.y1 as i64);
    let b2 = (b.x2 as i64, b.y2 as i64);

    let dist_sq = segment_distance_sq(a1, a2, b1, b2);
    let radii_sum = (a.r as i64) + (b.r as i64);

    dist_sq < radii_sum * radii_sum
}

/// Check if two shapes overlap.
#[must_use]
pub fn shapes_overlap(
    a: &ShapeView,
    a_offset: (i32, i32),
    b: &ShapeView,
    b_offset: (i32, i32),
) -> bool {
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
        (SHAPE_KIND_AABB, SHAPE_KIND_CIRCLE) => {
            let aabb = Aabb::from_shape(a, a_offset.0, a_offset.1);
            let circle = Circle::from_shape(b, b_offset.0, b_offset.1);
            aabb_circle_overlap(&aabb, &circle)
        }
        (SHAPE_KIND_CIRCLE, SHAPE_KIND_AABB) => {
            let circle = Circle::from_shape(a, a_offset.0, a_offset.1);
            let aabb = Aabb::from_shape(b, b_offset.0, b_offset.1);
            aabb_circle_overlap(&aabb, &circle)
        }
        (SHAPE_KIND_CAPSULE, SHAPE_KIND_CAPSULE) => {
            let cap_a = Capsule::from_shape(a, a_offset.0, a_offset.1);
            let cap_b = Capsule::from_shape(b, b_offset.0, b_offset.1);
            capsule_overlap(&cap_a, &cap_b)
        }
        _ => false, // Rotated rect and mixed capsule types not yet supported
    }
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

    // Check if they overlap
    if !aabb_overlap(&p1_aabb, &p2_aabb) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aabb_overlap_detects_intersection() {
        let a = Aabb {
            x: 0,
            y: 0,
            w: 10,
            h: 10,
        };
        let b = Aabb {
            x: 5,
            y: 5,
            w: 10,
            h: 10,
        };
        assert!(aabb_overlap(&a, &b));
    }

    #[test]
    fn aabb_overlap_detects_no_intersection() {
        let a = Aabb {
            x: 0,
            y: 0,
            w: 10,
            h: 10,
        };
        let b = Aabb {
            x: 20,
            y: 20,
            w: 10,
            h: 10,
        };
        assert!(!aabb_overlap(&a, &b));
    }

    #[test]
    fn aabb_overlap_edge_touching_is_not_overlap() {
        let a = Aabb {
            x: 0,
            y: 0,
            w: 10,
            h: 10,
        };
        let b = Aabb {
            x: 10,
            y: 0,
            w: 10,
            h: 10,
        };
        assert!(!aabb_overlap(&a, &b));
    }

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

    #[test]
    fn aabb_circle_overlap_detects_intersection() {
        let aabb = Aabb { x: 0, y: 0, w: 20, h: 20 };
        let circle = Circle { x: 25, y: 10, r: 10 };
        assert!(aabb_circle_overlap(&aabb, &circle)); // circle touches right edge
    }

    #[test]
    fn aabb_circle_overlap_detects_no_intersection() {
        let aabb = Aabb { x: 0, y: 0, w: 20, h: 20 };
        let circle = Circle { x: 35, y: 10, r: 5 };
        assert!(!aabb_circle_overlap(&aabb, &circle)); // too far right
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
                hit_pushback: 0,
                block_pushback: 0,
            });
        }

        // Should cap at 8
        assert_eq!(result.len(), 8);
    }

    #[test]
    fn capsule_overlap_detects_intersection() {
        // Two overlapping horizontal capsules
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

    #[test]
    fn capsule_overlap_edge_touching_is_not_overlap() {
        // Two capsules exactly touching (distance == sum of radii)
        let a = Capsule { x1: 0, y1: 0, x2: 10, y2: 0, r: 5 };
        let b = Capsule { x1: 20, y1: 0, x2: 30, y2: 0, r: 5 };
        assert!(!capsule_overlap(&a, &b)); // distance 10 == 5+5
    }
}
