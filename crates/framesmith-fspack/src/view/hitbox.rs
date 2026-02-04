//! Hitbox window and shape views.

use crate::bytes::{read_u16_le, read_u32_le, read_u8};
use crate::fixed::{Q12_4, Q8_8};

/// HitWindow record size (24 bytes)
pub const HIT_WINDOW_SIZE: usize = 24;

/// Shape record size (12 bytes)
pub const SHAPE_SIZE: usize = 12;

// Shape Type Constants
/// Shape type: axis-aligned bounding box
pub const SHAPE_KIND_AABB: u8 = 0;

/// Shape type: rotated rectangle
pub const SHAPE_KIND_RECT: u8 = 1;

/// Shape type: circle
pub const SHAPE_KIND_CIRCLE: u8 = 2;

/// Shape type: capsule (two endpoints + radius)
pub const SHAPE_KIND_CAPSULE: u8 = 3;

/// Zero-copy view over hit windows section.
///
/// Each entry is a HitWindow24 (24 bytes).
#[derive(Clone, Copy)]
pub struct HitWindowsView<'a> {
    data: &'a [u8],
}

impl<'a> HitWindowsView<'a> {
    /// Create a new view from raw bytes.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    /// Returns the total number of hit windows.
    pub fn len(&self) -> usize {
        self.data.len() / HIT_WINDOW_SIZE
    }

    /// Returns true if there are no hit windows.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get a hit window by global index.
    pub fn get(&self, index: usize) -> Option<HitWindowView<'a>> {
        let off = index.checked_mul(HIT_WINDOW_SIZE)?;
        let end = off.checked_add(HIT_WINDOW_SIZE)?;
        if end > self.data.len() {
            return None;
        }
        Some(HitWindowView {
            data: &self.data[off..end],
        })
    }

    /// Get a hit window at a byte offset + index.
    ///
    /// This is used to access a move's hit windows when you have the
    /// byte offset and want to iterate by index within that range.
    pub fn get_at(&self, offset_bytes: u32, index: usize) -> Option<HitWindowView<'a>> {
        let base = (offset_bytes as usize).checked_add(index.checked_mul(HIT_WINDOW_SIZE)?)?;
        let end = base.checked_add(HIT_WINDOW_SIZE)?;
        if end > self.data.len() {
            return None;
        }
        Some(HitWindowView {
            data: &self.data[base..end],
        })
    }
}

/// Zero-copy view over a single HitWindow24 record (24 bytes minimum).
///
/// Layout:
/// - 0: start_f (u8)
/// - 1: end_f (u8)
/// - 2: guard (u8)
/// - 3: _reserved (u8)
/// - 4-5: dmg (u16)
/// - 6-7: chip (u16)
/// - 8: hitstun (u8)
/// - 9: blockstun (u8)
/// - 10: hitstop (u8)
/// - 11: _reserved (u8)
/// - 12-15: shapes_off (u32)
/// - 16-17: shapes_len (u16)
/// - 18-21: cancels_off (u32)
/// - 22-23: cancels_len (u16)
/// Optional extended fields (backwards-compatible):
/// - 24-25: hit_pushback (i16, Q12.4 fixed-point)
/// - 26-27: block_pushback (i16, Q12.4 fixed-point)
#[derive(Clone, Copy)]
pub struct HitWindowView<'a> {
    data: &'a [u8],
}

impl<'a> HitWindowView<'a> {
    /// Start frame of this hit window.
    pub fn start_frame(&self) -> u8 {
        read_u8(self.data, 0).unwrap_or(0)
    }

    /// End frame of this hit window.
    pub fn end_frame(&self) -> u8 {
        read_u8(self.data, 1).unwrap_or(0)
    }

    /// Guard type for this hit window.
    pub fn guard(&self) -> u8 {
        read_u8(self.data, 2).unwrap_or(0)
    }

    /// Damage value for this hit window.
    pub fn damage(&self) -> u16 {
        read_u16_le(self.data, 4).unwrap_or(0)
    }

    /// Chip damage for this hit window (0 = none).
    pub fn chip_damage(&self) -> u16 {
        read_u16_le(self.data, 6).unwrap_or(0)
    }

    /// Hitstun frames for this hit window.
    pub fn hitstun(&self) -> u8 {
        read_u8(self.data, 8).unwrap_or(0)
    }

    /// Blockstun frames for this hit window.
    pub fn blockstun(&self) -> u8 {
        read_u8(self.data, 9).unwrap_or(0)
    }

    /// Hitstop frames for this hit window.
    pub fn hitstop(&self) -> u8 {
        read_u8(self.data, 10).unwrap_or(0)
    }

    /// Byte offset into SHAPES section.
    pub fn shapes_off(&self) -> u32 {
        read_u32_le(self.data, 12).unwrap_or(0)
    }

    /// Number of shapes in this hit window.
    pub fn shapes_len(&self) -> u16 {
        read_u16_le(self.data, 16).unwrap_or(0)
    }

    /// Byte offset into CANCELS_U16 section.
    pub fn cancels_off(&self) -> u32 {
        read_u32_le(self.data, 18).unwrap_or(0)
    }

    /// Number of cancel targets for this hit window.
    pub fn cancels_len(&self) -> u16 {
        read_u16_le(self.data, 22).unwrap_or(0)
    }

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

    /// Hit pushback as Q12.4 fixed-point.
    #[inline]
    pub fn hit_pushback_fixed(&self) -> Q12_4 {
        Q12_4::from_raw(self.hit_pushback_raw())
    }

    /// Block pushback as Q12.4 fixed-point.
    #[inline]
    pub fn block_pushback_fixed(&self) -> Q12_4 {
        Q12_4::from_raw(self.block_pushback_raw())
    }
}

/// Zero-copy view over shapes section.
///
/// Each entry is a Shape12 (12 bytes).
#[derive(Clone, Copy)]
pub struct ShapesView<'a> {
    data: &'a [u8],
}

impl<'a> ShapesView<'a> {
    /// Create a new view from raw bytes.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    /// Returns the total number of shapes.
    pub fn len(&self) -> usize {
        self.data.len() / SHAPE_SIZE
    }

    /// Returns true if there are no shapes.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get a shape by global index.
    pub fn get(&self, index: usize) -> Option<ShapeView<'a>> {
        let off = index.checked_mul(SHAPE_SIZE)?;
        let end = off.checked_add(SHAPE_SIZE)?;
        if end > self.data.len() {
            return None;
        }
        Some(ShapeView {
            data: &self.data[off..end],
        })
    }

    /// Get a shape at a byte offset + index.
    ///
    /// This is used to access shapes referenced by a hit/hurt window.
    pub fn get_at(&self, offset_bytes: u32, index: usize) -> Option<ShapeView<'a>> {
        let base = (offset_bytes as usize).checked_add(index.checked_mul(SHAPE_SIZE)?)?;
        let end = base.checked_add(SHAPE_SIZE)?;
        if end > self.data.len() {
            return None;
        }
        Some(ShapeView {
            data: &self.data[base..end],
        })
    }
}

/// Zero-copy view over a single Shape12 record (12 bytes).
///
/// Uses Q12.4 fixed-point coordinates (1/16 pixel precision).
///
/// Layout:
/// - 0: kind (u8) - 0=aabb, 1=rect, 2=circle, 3=capsule
/// - 1: flags (u8) - reserved
/// - 2-3: a (i16 Q12.4) - x for aabb/rect/circle, x1 for capsule
/// - 4-5: b (i16 Q12.4) - y for aabb/rect/circle, y1 for capsule
/// - 6-7: c (i16 Q12.4) - width for aabb/rect, radius for circle, x2 for capsule
/// - 8-9: d (i16 Q12.4) - height for aabb/rect, unused for circle, y2 for capsule
/// - 10-11: e (i16 Q8.8) - angle for rect, radius for capsule
#[derive(Clone, Copy)]
pub struct ShapeView<'a> {
    data: &'a [u8],
}

impl<'a> ShapeView<'a> {
    /// Shape type: 0=aabb, 1=rect, 2=circle, 3=capsule.
    pub fn kind(&self) -> u8 {
        read_u8(self.data, 0).unwrap_or(0)
    }

    /// Shape flags (reserved).
    pub fn flags(&self) -> u8 {
        read_u8(self.data, 1).unwrap_or(0)
    }

    /// Raw field a (Q12.4 fixed-point).
    /// For AABB/rect/circle: x coordinate.
    /// For capsule: x1 coordinate.
    pub fn a_raw(&self) -> i16 {
        read_u16_le(self.data, 2).unwrap_or(0) as i16
    }

    /// Raw field b (Q12.4 fixed-point).
    /// For AABB/rect/circle: y coordinate.
    /// For capsule: y1 coordinate.
    pub fn b_raw(&self) -> i16 {
        read_u16_le(self.data, 4).unwrap_or(0) as i16
    }

    /// Raw field c (Q12.4 fixed-point).
    /// For AABB/rect: width.
    /// For circle: radius.
    /// For capsule: x2 coordinate.
    pub fn c_raw(&self) -> i16 {
        read_u16_le(self.data, 6).unwrap_or(0) as i16
    }

    /// Raw field d (Q12.4 fixed-point).
    /// For AABB/rect: height.
    /// For circle: unused.
    /// For capsule: y2 coordinate.
    pub fn d_raw(&self) -> i16 {
        read_u16_le(self.data, 8).unwrap_or(0) as i16
    }

    /// Raw field e (Q8.8 fixed-point).
    /// For rect: rotation angle.
    /// For capsule: radius.
    pub fn e_raw(&self) -> i16 {
        read_u16_le(self.data, 10).unwrap_or(0) as i16
    }

    /// Convert Q12.4 fixed-point to integer pixels (rounding down).
    #[inline]
    fn q12_4_to_px(v: i16) -> i32 {
        (v as i32) >> 4
    }

    /// Get AABB x coordinate in pixels (valid for kind=0,1,2).
    pub fn x_px(&self) -> i32 {
        Self::q12_4_to_px(self.a_raw())
    }

    /// Get AABB y coordinate in pixels (valid for kind=0,1,2).
    pub fn y_px(&self) -> i32 {
        Self::q12_4_to_px(self.b_raw())
    }

    /// Get width in pixels (valid for kind=0,1).
    pub fn width_px(&self) -> u32 {
        Self::q12_4_to_px(self.c_raw()).max(0) as u32
    }

    /// Get height in pixels (valid for kind=0,1).
    pub fn height_px(&self) -> u32 {
        Self::q12_4_to_px(self.d_raw()).max(0) as u32
    }

    /// Get radius in pixels (valid for kind=2).
    pub fn radius_px(&self) -> u32 {
        Self::q12_4_to_px(self.c_raw()).max(0) as u32
    }

    /// Check if this is an AABB shape.
    pub fn is_aabb(&self) -> bool {
        self.kind() == SHAPE_KIND_AABB
    }

    /// X coordinate as Q12.4 fixed-point (valid for kind=0,1,2).
    /// For capsule: x1 coordinate.
    #[inline]
    pub fn x_fixed(&self) -> Q12_4 {
        Q12_4::from_raw(self.a_raw())
    }

    /// Y coordinate as Q12.4 fixed-point (valid for kind=0,1,2).
    /// For capsule: y1 coordinate.
    #[inline]
    pub fn y_fixed(&self) -> Q12_4 {
        Q12_4::from_raw(self.b_raw())
    }

    /// Width as Q12.4 fixed-point (valid for kind=0,1).
    /// For circle: radius.
    /// For capsule: x2 coordinate.
    #[inline]
    pub fn width_fixed(&self) -> Q12_4 {
        Q12_4::from_raw(self.c_raw())
    }

    /// Height as Q12.4 fixed-point (valid for kind=0,1).
    /// For capsule: y2 coordinate.
    #[inline]
    pub fn height_fixed(&self) -> Q12_4 {
        Q12_4::from_raw(self.d_raw())
    }

    /// Capsule x2 coordinate as Q12.4 fixed-point.
    #[inline]
    pub fn x2_fixed(&self) -> Q12_4 {
        Q12_4::from_raw(self.c_raw())
    }

    /// Capsule y2 coordinate as Q12.4 fixed-point.
    #[inline]
    pub fn y2_fixed(&self) -> Q12_4 {
        Q12_4::from_raw(self.d_raw())
    }

    /// Capsule radius as Q8.8 fixed-point.
    /// Also used for rect rotation angle.
    #[inline]
    pub fn radius_fixed(&self) -> Q8_8 {
        Q8_8::from_raw(self.e_raw())
    }
}
