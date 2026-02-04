//! Fixed-point number types for deterministic game math.
//!
//! This module provides type-safe fixed-point types that games can use with either:
//! - **Raw i32 logic** - direct integer operations for deterministic netcode
//! - **f32 helpers** - convenient float conversions (behind `float` feature)
//!
//! # Design Principles
//!
//! 1. **Zero-cost abstractions** - newtype wrappers compile away
//! 2. **no_std by default** - f32 helpers behind feature flag
//! 3. **Dual interface** - both raw access and converted values
//! 4. **Type safety** - can't accidentally mix Q12.4 with Q8.8
//!
//! # Types
//!
//! - [`Q12_4`] - 4 fractional bits (1/16 precision), used for coordinates/dimensions
//! - [`Q8_8`] - 8 fractional bits (1/256 precision), used for angles/radii
//! - [`Q24_8`] - 8 fractional bits, wider range, used for character properties
//!
//! # Example: Raw i32 Logic (deterministic netcode)
//!
//! ```
//! use framesmith_fspack::fixed::Q12_4;
//!
//! let x = Q12_4::from_int(10);  // 10.0 in Q12.4
//! let velocity = Q12_4::from_raw(8);  // 0.5 in Q12.4 (8/16)
//! let new_x = x.saturating_add(velocity);
//!
//! assert_eq!(new_x.to_int(), 10);  // Still 10 (0.5 truncated)
//! assert_eq!(new_x.raw(), 168);    // 160 + 8 = 168
//! ```
//!
//! # Example: f32 Helpers (convenience)
//!
//! ```ignore
//! use framesmith_fspack::fixed::Q12_4;
//!
//! let x = Q12_4::from_f32(10.5);
//! assert_eq!(x.to_f32(), 10.5);
//! ```

use core::ops::{Add, Neg, Sub};

// Re-export types from submodules
mod fixed_q12_4;
mod fixed_q8_8;

pub use fixed_q12_4::Q12_4;
pub use fixed_q8_8::Q8_8;

// =============================================================================
// Q24.8 Fixed-Point (8 fractional bits, wider range)
// =============================================================================

/// Q24.8 fixed-point (8 fractional bits, 1/256 precision, wider range).
///
/// Used for: character properties (health, speed, damage).
///
/// Range: approximately -8388608.0 to +8388607.99609375 (i32 range / 256)
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct Q24_8(pub i32);

impl Q24_8 {
    /// Number of fractional bits.
    pub const FRAC_BITS: u32 = 8;

    /// Scale factor (1 << FRAC_BITS).
    pub const SCALE: i32 = 256;

    /// Zero value.
    pub const ZERO: Self = Self(0);

    /// One (1.0 in fixed-point).
    pub const ONE: Self = Self(256);

    /// Create from raw fixed-point value.
    #[inline]
    pub const fn from_raw(raw: i32) -> Self {
        Self(raw)
    }

    /// Get raw fixed-point value for integer math.
    #[inline]
    pub const fn raw(self) -> i32 {
        self.0
    }

    /// Convert to integer (floors toward negative infinity).
    #[inline]
    pub const fn to_int(self) -> i32 {
        self.0 >> Self::FRAC_BITS
    }

    /// Create from integer value.
    #[inline]
    pub const fn from_int(val: i32) -> Self {
        Self(val << Self::FRAC_BITS)
    }

    /// Saturating addition.
    #[inline]
    pub const fn saturating_add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }

    /// Saturating subtraction.
    #[inline]
    pub const fn saturating_sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }

    /// Saturating negation.
    #[inline]
    pub const fn saturating_neg(self) -> Self {
        Self(self.0.saturating_neg())
    }

    /// Wrapping addition (for when overflow is intentional).
    #[inline]
    pub const fn wrapping_add(self, rhs: Self) -> Self {
        Self(self.0.wrapping_add(rhs.0))
    }

    /// Wrapping subtraction (for when overflow is intentional).
    #[inline]
    pub const fn wrapping_sub(self, rhs: Self) -> Self {
        Self(self.0.wrapping_sub(rhs.0))
    }

    /// Returns the absolute value (saturating at i32::MIN).
    #[inline]
    pub const fn abs(self) -> Self {
        Self(self.0.saturating_abs())
    }

    /// Returns the minimum of two values.
    #[inline]
    pub const fn min(self, other: Self) -> Self {
        if self.0 < other.0 {
            self
        } else {
            other
        }
    }

    /// Returns the maximum of two values.
    #[inline]
    pub const fn max(self, other: Self) -> Self {
        if self.0 > other.0 {
            self
        } else {
            other
        }
    }
}

impl Add for Q24_8 {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self {
        self.saturating_add(rhs)
    }
}

impl Sub for Q24_8 {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self {
        self.saturating_sub(rhs)
    }
}

impl Neg for Q24_8 {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        self.saturating_neg()
    }
}

#[cfg(any(feature = "std", feature = "float"))]
impl Q24_8 {
    /// Convert to f32.
    #[inline]
    pub fn to_f32(self) -> f32 {
        (self.0 as f32) / (Self::SCALE as f32)
    }

    /// Create from f32 (rounds toward zero).
    #[inline]
    pub fn from_f32(val: f32) -> Self {
        Self((val * Self::SCALE as f32) as i32)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // Q24_8 tests
    // -------------------------------------------------------------------------

    #[test]
    fn q24_8_from_int_roundtrip() {
        assert_eq!(Q24_8::from_int(0).to_int(), 0);
        assert_eq!(Q24_8::from_int(1).to_int(), 1);
        assert_eq!(Q24_8::from_int(-1).to_int(), -1);
        assert_eq!(Q24_8::from_int(1000000).to_int(), 1000000);
    }

    #[test]
    fn q24_8_raw_access() {
        let v = Q24_8::from_int(100);
        assert_eq!(v.raw(), 25600); // 100 * 256

        let v = Q24_8::from_raw(512);
        assert_eq!(v.to_int(), 2);
    }

    #[test]
    fn q24_8_constants() {
        assert_eq!(Q24_8::ZERO.raw(), 0);
        assert_eq!(Q24_8::ONE.raw(), 256);
        assert_eq!(Q24_8::ONE.to_int(), 1);
    }

    #[test]
    fn q24_8_saturating_ops() {
        let max = Q24_8::from_raw(i32::MAX);
        let one = Q24_8::ONE;
        assert_eq!((max + one).raw(), i32::MAX);

        let min = Q24_8::from_raw(i32::MIN);
        assert_eq!((min - one).raw(), i32::MIN);
    }

    #[test]
    fn q24_8_large_values() {
        let v = Q24_8::from_int(1_000_000);
        assert_eq!(v.to_int(), 1_000_000);
        assert_eq!(v.raw(), 256_000_000);
    }

    #[test]
    fn q24_8_truncation_negative() {
        // -0.5 in Q24.8 = raw -128
        let v = Q24_8::from_raw(-128);
        assert_eq!(v.to_int(), -1); // floors to -1
    }

    #[test]
    fn q24_8_scale_constants() {
        assert_eq!(Q24_8::SCALE, 256);
        assert_eq!(Q24_8::FRAC_BITS, 8);
    }

    #[test]
    fn q24_8_one_equals_scale() {
        assert_eq!(Q24_8::ONE.raw(), Q24_8::SCALE);
    }

    // -------------------------------------------------------------------------
    // Default trait tests (shared across all types)
    // -------------------------------------------------------------------------

    #[test]
    fn default_is_zero() {
        assert_eq!(Q12_4::default(), Q12_4::ZERO);
        assert_eq!(Q8_8::default(), Q8_8::ZERO);
        assert_eq!(Q24_8::default(), Q24_8::ZERO);
    }

    // -------------------------------------------------------------------------
    // f32 conversion tests (only when float feature is enabled)
    // -------------------------------------------------------------------------

    #[cfg(any(feature = "std", feature = "float"))]
    mod float_tests {
        use super::*;

        #[test]
        fn q24_8_f32_roundtrip() {
            let v = Q24_8::from_f32(1000.5);
            assert!((v.to_f32() - 1000.5).abs() < 0.01);
        }
    }
}
