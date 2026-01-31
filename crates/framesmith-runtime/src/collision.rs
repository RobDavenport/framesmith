use crate::state::CharacterState;
use framesmith_fspack::{PackView, ShapeView, SHAPE_KIND_AABB, SHAPE_KIND_CIRCLE};

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
        _ => false, // Capsule and rotated rect not yet supported
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
                &hw,
                &attacker_shapes,
                attacker_pos,
                &hrt,
                &defender_shapes,
                defender_pos,
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
}
