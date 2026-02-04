//! Q12.4 fixed-point type (4 fractional bits).

use core::ops::{Add, Neg, Sub};

/// Q12.4 fixed-point (4 fractional bits, 1/16 precision).
///
/// Used for: shape coordinates, dimensions, pushback values.
///
/// Range: -2048.0 to +2047.9375 (i16 range / 16)
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct Q12_4(pub i16);

impl Q12_4 {
    /// Number of fractional bits.
    pub const FRAC_BITS: u32 = 4;

    /// Scale factor (1 << FRAC_BITS).
    pub const SCALE: i32 = 16;

    /// Zero value.
    pub const ZERO: Self = Self(0);

    /// One (1.0 in fixed-point).
    pub const ONE: Self = Self(16);

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

impl Add for Q12_4 {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self {
        self.saturating_add(rhs)
    }
}

impl Sub for Q12_4 {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self {
        self.saturating_sub(rhs)
    }
}

impl Neg for Q12_4 {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        self.saturating_neg()
    }
}

#[cfg(any(feature = "std", feature = "float"))]
impl Q12_4 {
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
    fn q12_4_from_int_roundtrip() {
        assert_eq!(Q12_4::from_int(0).to_int(), 0);
        assert_eq!(Q12_4::from_int(1).to_int(), 1);
        assert_eq!(Q12_4::from_int(-1).to_int(), -1);
        assert_eq!(Q12_4::from_int(100).to_int(), 100);
        assert_eq!(Q12_4::from_int(-100).to_int(), -100);
    }

    #[test]
    fn q12_4_raw_access() {
        let v = Q12_4::from_int(5);
        assert_eq!(v.raw(), 80); // 5 * 16

        let v = Q12_4::from_raw(24);
        assert_eq!(v.to_int(), 1); // 24 / 16 = 1 (truncated)
        assert_eq!(v.raw(), 24);
    }

    #[test]
    fn q12_4_saturating_add() {
        let a = Q12_4::from_int(10);
        let b = Q12_4::from_int(5);
        assert_eq!((a + b).to_int(), 15);

        // Test saturation
        let max = Q12_4::from_raw(i16::MAX);
        let one = Q12_4::ONE;
        assert_eq!((max + one).raw(), i16::MAX);
    }

    #[test]
    fn q12_4_saturating_sub() {
        let a = Q12_4::from_int(10);
        let b = Q12_4::from_int(3);
        assert_eq!((a - b).to_int(), 7);

        // Test saturation
        let min = Q12_4::from_raw(i16::MIN);
        let one = Q12_4::ONE;
        assert_eq!((min - one).raw(), i16::MIN);
    }

    #[test]
    fn q12_4_negation() {
        let v = Q12_4::from_int(5);
        assert_eq!((-v).to_int(), -5);

        let v = Q12_4::from_int(-5);
        assert_eq!((-v).to_int(), 5);

        // Test saturation at MIN
        let min = Q12_4::from_raw(i16::MIN);
        assert_eq!((-min).raw(), i16::MAX);
    }

    #[test]
    fn q12_4_abs() {
        assert_eq!(Q12_4::from_int(5).abs().to_int(), 5);
        assert_eq!(Q12_4::from_int(-5).abs().to_int(), 5);
        assert_eq!(Q12_4::from_int(0).abs().to_int(), 0);
    }

    #[test]
    fn q12_4_min_max() {
        let a = Q12_4::from_int(5);
        let b = Q12_4::from_int(10);

        assert_eq!(a.min(b), a);
        assert_eq!(a.max(b), b);
        assert_eq!(b.min(a), a);
        assert_eq!(b.max(a), b);
    }

    #[test]
    fn q12_4_constants() {
        assert_eq!(Q12_4::ZERO.raw(), 0);
        assert_eq!(Q12_4::ONE.raw(), 16);
        assert_eq!(Q12_4::ONE.to_int(), 1);
    }

    #[test]
    fn q12_4_wrapping_ops() {
        let max = Q12_4::from_raw(i16::MAX);
        let one = Q12_4::ONE;

        // Wrapping should wrap around
        let wrapped = max.wrapping_add(one);
        assert_eq!(wrapped.raw(), i16::MAX.wrapping_add(16));
    }

    #[test]
    fn q12_4_truncation_positive() {
        // 1.5 in Q12.4 = raw 24
        let v = Q12_4::from_raw(24);
        assert_eq!(v.to_int(), 1); // floors to 1

        // 1.9375 in Q12.4 = raw 31 (15/16)
        let v = Q12_4::from_raw(31);
        assert_eq!(v.to_int(), 1); // floors to 1
    }

    #[test]
    fn q12_4_truncation_negative() {
        // -0.5 in Q12.4 = raw -8
        let v = Q12_4::from_raw(-8);
        assert_eq!(v.to_int(), -1); // floors to -1 (toward negative infinity)

        // -1.5 in Q12.4 = raw -24
        let v = Q12_4::from_raw(-24);
        assert_eq!(v.to_int(), -2); // floors to -2

        // -0.0625 in Q12.4 = raw -1
        let v = Q12_4::from_raw(-1);
        assert_eq!(v.to_int(), -1); // floors to -1
    }

    #[test]
    fn q12_4_ordering() {
        let a = Q12_4::from_int(-10);
        let b = Q12_4::from_int(0);
        let c = Q12_4::from_int(10);

        assert!(a < b);
        assert!(b < c);
        assert!(a < c);

        // Fractional ordering
        let half = Q12_4::from_raw(8); // 0.5
        let one = Q12_4::ONE;
        assert!(half < one);
        assert!(Q12_4::ZERO < half);
    }

    #[test]
    fn q12_4_equality() {
        let a = Q12_4::from_int(5);
        let b = Q12_4::from_raw(80); // 5 * 16
        assert_eq!(a, b);

        let c = Q12_4::from_raw(81); // 5.0625
        assert_ne!(a, c);
    }

    #[test]
    fn q12_4_add_fractional() {
        // 0.5 + 0.5 = 1.0
        let half = Q12_4::from_raw(8);
        let result = half + half;
        assert_eq!(result.raw(), 16);
        assert_eq!(result.to_int(), 1);
    }

    #[test]
    fn q12_4_sub_to_negative() {
        let a = Q12_4::from_int(5);
        let b = Q12_4::from_int(10);
        let result = a - b;
        assert_eq!(result.to_int(), -5);
    }

    #[test]
    fn q12_4_zero_operations() {
        let zero = Q12_4::ZERO;
        let five = Q12_4::from_int(5);

        assert_eq!(zero + five, five);
        assert_eq!(five + zero, five);
        assert_eq!(five - zero, five);
        assert_eq!(zero - five, Q12_4::from_int(-5));
    }

    #[test]
    fn scale_constants_correct() {
        assert_eq!(Q12_4::SCALE, 16);
        assert_eq!(Q12_4::FRAC_BITS, 4);
    }

    #[test]
    fn one_equals_scale() {
        assert_eq!(Q12_4::ONE.raw() as i32, Q12_4::SCALE);
    }

    #[cfg(any(feature = "std", feature = "float"))]
    mod float_tests {
        use super::*;

        #[test]
        fn q12_4_f32_roundtrip() {
            let v = Q12_4::from_f32(10.5);
            assert!((v.to_f32() - 10.5).abs() < 0.1);

            let v = Q12_4::from_f32(-3.25);
            assert!((v.to_f32() - (-3.25)).abs() < 0.1);
        }

        #[test]
        fn q12_4_f32_precision() {
            // Q12.4 has 1/16 = 0.0625 precision
            assert_eq!(Q12_4::from_f32(0.0625).raw(), 1);
            assert_eq!(Q12_4::from_f32(0.125).raw(), 2);
            assert_eq!(Q12_4::from_f32(0.5).raw(), 8);
            assert_eq!(Q12_4::from_f32(1.0).raw(), 16);

            // Verify exact representation
            assert!((Q12_4::from_raw(1).to_f32() - 0.0625).abs() < 0.0001);
            assert!((Q12_4::from_raw(8).to_f32() - 0.5).abs() < 0.0001);
        }
    }
}
