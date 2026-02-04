//! Q8.8 fixed-point type (8 fractional bits).

use core::ops::{Add, Neg, Sub};

/// Q8.8 fixed-point (8 fractional bits, 1/256 precision).
///
/// Used for: capsule radius, rotation angles.
///
/// Range: -128.0 to +127.99609375 (i16 range / 256)
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct Q8_8(pub i16);

impl Q8_8 {
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
    pub const fn from_raw(raw: i16) -> Self {
        Self(raw)
    }

    /// Get raw fixed-point value for integer math.
    #[inline]
    pub const fn raw(self) -> i16 {
        self.0
    }

    /// Convert to integer (floors toward negative infinity).
    #[inline]
    pub const fn to_int(self) -> i32 {
        (self.0 as i32) >> Self::FRAC_BITS
    }

    /// Create from integer value.
    #[inline]
    pub const fn from_int(val: i32) -> Self {
        Self((val << Self::FRAC_BITS) as i16)
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

    /// Returns the absolute value (saturating at i16::MIN).
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

impl Add for Q8_8 {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self {
        self.saturating_add(rhs)
    }
}

impl Sub for Q8_8 {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self {
        self.saturating_sub(rhs)
    }
}

impl Neg for Q8_8 {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        self.saturating_neg()
    }
}

#[cfg(any(feature = "std", feature = "float"))]
impl Q8_8 {
    /// Convert to f32.
    #[inline]
    pub fn to_f32(self) -> f32 {
        (self.0 as f32) / (Self::SCALE as f32)
    }

    /// Create from f32 (rounds toward zero).
    #[inline]
    pub fn from_f32(val: f32) -> Self {
        Self((val * Self::SCALE as f32) as i16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn q8_8_from_int_roundtrip() {
        assert_eq!(Q8_8::from_int(0).to_int(), 0);
        assert_eq!(Q8_8::from_int(1).to_int(), 1);
        assert_eq!(Q8_8::from_int(-1).to_int(), -1);
        assert_eq!(Q8_8::from_int(50).to_int(), 50);
    }

    #[test]
    fn q8_8_raw_access() {
        let v = Q8_8::from_int(2);
        assert_eq!(v.raw(), 512); // 2 * 256

        let v = Q8_8::from_raw(384);
        assert_eq!(v.to_int(), 1); // 384 / 256 = 1 (truncated)
    }

    #[test]
    fn q8_8_constants() {
        assert_eq!(Q8_8::ZERO.raw(), 0);
        assert_eq!(Q8_8::ONE.raw(), 256);
        assert_eq!(Q8_8::ONE.to_int(), 1);
    }

    #[test]
    fn q8_8_saturating_ops() {
        let max = Q8_8::from_raw(i16::MAX);
        let one = Q8_8::ONE;
        assert_eq!((max + one).raw(), i16::MAX);

        let min = Q8_8::from_raw(i16::MIN);
        assert_eq!((min - one).raw(), i16::MIN);
    }

    #[test]
    fn q8_8_truncation_negative() {
        // -0.5 in Q8.8 = raw -128
        let v = Q8_8::from_raw(-128);
        assert_eq!(v.to_int(), -1); // floors to -1
    }

    #[test]
    fn scale_constants_correct() {
        assert_eq!(Q8_8::SCALE, 256);
        assert_eq!(Q8_8::FRAC_BITS, 8);
    }

    #[test]
    fn one_equals_scale() {
        assert_eq!(Q8_8::ONE.raw() as i32, Q8_8::SCALE);
    }

    #[cfg(any(feature = "std", feature = "float"))]
    mod float_tests {
        use super::*;

        #[test]
        fn q8_8_f32_roundtrip() {
            let v = Q8_8::from_f32(5.5);
            assert!((v.to_f32() - 5.5).abs() < 0.01);
        }

        #[test]
        fn q8_8_f32_precision() {
            // Q8.8 has 1/256 = 0.00390625 precision
            assert_eq!(Q8_8::from_f32(0.00390625).raw(), 1);
            assert_eq!(Q8_8::from_f32(0.5).raw(), 128);
            assert_eq!(Q8_8::from_f32(1.0).raw(), 256);
        }
    }
}
