use framesmith_fspack::{ShapeView, SHAPE_KIND_AABB, SHAPE_KIND_CAPSULE, SHAPE_KIND_CIRCLE};

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
            x: shape.x_px().saturating_add(offset_x),
            y: shape.y_px().saturating_add(offset_y),
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
            x: shape.x_px().saturating_add(offset_x),
            y: shape.y_px().saturating_add(offset_y),
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
        // Use typed fixed-point accessors for clarity
        let x1 = shape.x_fixed().to_int().saturating_add(offset_x);
        let y1 = shape.y_fixed().to_int().saturating_add(offset_y);
        let x2 = shape.x2_fixed().to_int().saturating_add(offset_x);
        let y2 = shape.y2_fixed().to_int().saturating_add(offset_y);
        let r = shape.radius_fixed().to_int().max(0) as u32;
        Capsule { x1, y1, x2, y2, r }
    }
}

/// Check if two AABBs overlap.
///
/// Edge-touching is NOT considered overlap.
#[inline]
#[must_use]
pub fn aabb_overlap(a: &Aabb, b: &Aabb) -> bool {
    if a.w == 0 || a.h == 0 || b.w == 0 || b.h == 0 {
        return false;
    }
    let a_right = i128::from(a.x) + i128::from(a.w);
    let a_bottom = i128::from(a.y) + i128::from(a.h);
    let b_right = i128::from(b.x) + i128::from(b.w);
    let b_bottom = i128::from(b.y) + i128::from(b.h);
    i128::from(a.x) < b_right
        && a_right > i128::from(b.x)
        && i128::from(a.y) < b_bottom
        && a_bottom > i128::from(b.y)
}

/// Check if two circles overlap.
///
/// Edge-touching is NOT considered overlap.
#[must_use]
#[inline]
pub fn circle_overlap(a: &Circle, b: &Circle) -> bool {
    let dx = (a.x as i128) - (b.x as i128);
    let dy = (a.y as i128) - (b.y as i128);
    let dist_sq = dx * dx + dy * dy;
    let radii_sum = (a.r as i128) + (b.r as i128);
    dist_sq < radii_sum * radii_sum
}

/// Check if an AABB and circle overlap.
#[must_use]
#[inline]
pub fn aabb_circle_overlap(aabb: &Aabb, circle: &Circle) -> bool {
    // Find closest point on AABB to circle center
    if aabb.w == 0 || aabb.h == 0 {
        return false;
    }
    let closest_x =
        i128::from(circle.x).clamp(i128::from(aabb.x), i128::from(aabb.x) + i128::from(aabb.w));
    let closest_y =
        i128::from(circle.y).clamp(i128::from(aabb.y), i128::from(aabb.y) + i128::from(aabb.h));

    let dx = (circle.x as i128) - closest_x;
    let dy = (circle.y as i128) - closest_y;
    let dist_sq = dx * dx + dy * dy;
    let r = circle.r as i128;

    dist_sq < r * r
}

/// Find closest point on segment (p1, p2) to point p.
fn closest_point_on_segment(p1: (i128, i128), p2: (i128, i128), p: (i128, i128)) -> (i128, i128) {
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

    (p1.0 + (dx * t) / len_sq, p1.1 + (dy * t) / len_sq)
}

/// Compute squared distance between closest points on two line segments.
fn segment_distance_sq(
    a1: (i128, i128),
    a2: (i128, i128),
    b1: (i128, i128),
    b2: (i128, i128),
) -> i128 {
    let cross = |p: (i128, i128), q: (i128, i128), r: (i128, i128)| {
        (q.0 - p.0) * (r.1 - p.1) - (q.1 - p.1) * (r.0 - p.0)
    };
    let straddles = |x: i128, y: i128| (x <= 0 && y >= 0) || (x >= 0 && y <= 0);
    if a1.0.min(a2.0) <= b1.0.max(b2.0)
        && b1.0.min(b2.0) <= a1.0.max(a2.0)
        && a1.1.min(a2.1) <= b1.1.max(b2.1)
        && b1.1.min(b2.1) <= a1.1.max(a2.1)
        && straddles(cross(a1, a2, b1), cross(a1, a2, b2))
        && straddles(cross(b1, b2, a1), cross(b1, b2, a2))
    {
        return 0;
    }
    // ponytail: non-crossing projections use integer pixels; use engine geometry
    // from the typed payload when subpixel contact policy matters.
    [(a1, b1, b2), (a2, b1, b2), (b1, a1, a2), (b2, a1, a2)]
        .into_iter()
        .map(|(point, start, end)| {
            let closest = closest_point_on_segment(start, end, point);
            let dx = point.0 - closest.0;
            let dy = point.1 - closest.1;
            dx * dx + dy * dy
        })
        .min()
        .unwrap_or(i128::MAX)
}

/// Check if two capsules overlap.
///
/// A capsule is a line segment with radius (like a stadium shape).
/// Edge-touching is NOT considered overlap.
#[must_use]
#[inline]
pub fn capsule_overlap(a: &Capsule, b: &Capsule) -> bool {
    let a1 = (a.x1 as i128, a.y1 as i128);
    let a2 = (a.x2 as i128, a.y2 as i128);
    let b1 = (b.x1 as i128, b.y1 as i128);
    let b2 = (b.x2 as i128, b.y2 as i128);

    let dist_sq = segment_distance_sq(a1, a2, b1, b2);
    let radii_sum = (a.r as i128) + (b.r as i128);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extreme_coordinates_and_crossing_capsules_are_safe() {
        let wide = Aabb {
            x: i32::MIN,
            y: 0,
            w: u32::MAX,
            h: 10,
        };
        let inside = Aabb {
            x: 0,
            y: 1,
            w: 2,
            h: 2,
        };
        assert!(aabb_overlap(&wide, &inside));
        assert!(aabb_circle_overlap(&wide, &Circle { x: 0, y: 1, r: 1 }));
        assert!(!aabb_overlap(&Aabb { w: 0, ..inside }, &wide));
        assert!(!circle_overlap(
            &Circle {
                x: i32::MIN,
                y: i32::MIN,
                r: 1
            },
            &Circle {
                x: i32::MAX,
                y: i32::MAX,
                r: 1
            }
        ));
        let a = Capsule {
            x1: 0,
            y1: 0,
            x2: 100,
            y2: 0,
            r: 1,
        };
        let b = Capsule {
            x1: 0,
            y1: 10,
            x2: 100,
            y2: -20,
            r: 1,
        };
        assert!(capsule_overlap(&a, &b), "center lines cross");
        assert_eq!(capsule_overlap(&a, &b), capsule_overlap(&b, &a));
    }

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
        let aabb = Aabb {
            x: 0,
            y: 0,
            w: 20,
            h: 20,
        };
        let circle = Circle {
            x: 25,
            y: 10,
            r: 10,
        };
        assert!(aabb_circle_overlap(&aabb, &circle)); // circle touches right edge
    }

    #[test]
    fn aabb_circle_overlap_detects_no_intersection() {
        let aabb = Aabb {
            x: 0,
            y: 0,
            w: 20,
            h: 20,
        };
        let circle = Circle { x: 35, y: 10, r: 5 };
        assert!(!aabb_circle_overlap(&aabb, &circle)); // too far right
    }

    #[test]
    fn capsule_overlap_detects_intersection() {
        // Two overlapping horizontal capsules
        let a = Capsule {
            x1: 0,
            y1: 0,
            x2: 20,
            y2: 0,
            r: 5,
        };
        let b = Capsule {
            x1: 15,
            y1: 0,
            x2: 35,
            y2: 0,
            r: 5,
        };
        assert!(capsule_overlap(&a, &b));
    }

    #[test]
    fn capsule_overlap_detects_no_intersection() {
        // Two non-overlapping capsules
        let a = Capsule {
            x1: 0,
            y1: 0,
            x2: 10,
            y2: 0,
            r: 5,
        };
        let b = Capsule {
            x1: 30,
            y1: 0,
            x2: 40,
            y2: 0,
            r: 5,
        };
        assert!(!capsule_overlap(&a, &b));
    }

    #[test]
    fn capsule_overlap_edge_touching_is_not_overlap() {
        // Two capsules exactly touching (distance == sum of radii)
        let a = Capsule {
            x1: 0,
            y1: 0,
            x2: 10,
            y2: 0,
            r: 5,
        };
        let b = Capsule {
            x1: 20,
            y1: 0,
            x2: 30,
            y2: 0,
            r: 5,
        };
        assert!(!capsule_overlap(&a, &b)); // distance 10 == 5+5
    }
}
