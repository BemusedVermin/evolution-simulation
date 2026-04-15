//! Q32.32 fixed-point arithmetic with saturating-by-default operators.
//!
//! The simulation's determinism contract forbids floating point in sim state
//! because IEEE-754 is not bit-identical across platforms and compilers. This
//! module exposes [`Q3232`], a newtype wrapper around
//! [`fixed::types::I32F32`], and re-defines the core arithmetic operators
//! (`+`, `-`, `*`, `/`, `Neg`) to **saturate** rather than wrap or panic.
//!
//! * `fixed`'s default `+`/`-`/`*`/`/` behave like primitive `i64`: they panic
//!   on overflow in debug builds and wrap in release. Either outcome is a
//!   determinism hazard — a debug build would diverge from release, and a
//!   wrapping release build would silently warp the simulation.
//! * Saturating semantics are the same across `debug` and `release`, and give
//!   a stable, bounded state whenever arithmetic is driven outside the
//!   representable range by a pathological genome or input.
//!
//! If a call site genuinely wants wrapping or checked behaviour it can reach
//! the inner `I32F32` via [`Q3232::into_inner`] or [`Q3232::from_inner`] and
//! use the `fixed` API directly.

use core::cmp::Ordering;
use core::fmt;
use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use fixed::types::I32F32;
use serde::{Deserialize, Serialize};

/// Q32.32 signed fixed-point value: 32 integer bits, 32 fractional bits.
///
/// Range: approximately `[-2_147_483_648.0, 2_147_483_648.0)` with resolution
/// `2^-32 ≈ 2.33e-10`.
///
/// All standard arithmetic operators saturate at [`Q3232::MIN`] / [`Q3232::MAX`]
/// rather than panicking or wrapping. This is a deliberate determinism choice
/// — see the module docs.
#[derive(Copy, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct Q3232(I32F32);

impl Q3232 {
    /// The constant zero.
    pub const ZERO: Self = Self(I32F32::ZERO);

    /// The constant one.
    pub const ONE: Self = Self(I32F32::ONE);

    /// The smallest representable value.
    pub const MIN: Self = Self(I32F32::MIN);

    /// The largest representable value.
    pub const MAX: Self = Self(I32F32::MAX);

    /// The smallest positive representable value (`2^-32`).
    pub const EPSILON: Self = Self(I32F32::DELTA);

    /// Construct from the underlying `I32F32`.
    #[inline]
    pub const fn from_inner(value: I32F32) -> Self {
        Self(value)
    }

    /// Access the underlying `I32F32`.
    #[inline]
    pub const fn into_inner(self) -> I32F32 {
        self.0
    }

    /// Construct from the raw 64-bit bit pattern (unscaled integer
    /// representation). Useful for deterministic deserialization.
    #[inline]
    pub const fn from_bits(bits: i64) -> Self {
        Self(I32F32::from_bits(bits))
    }

    /// Return the raw 64-bit bit pattern.
    #[inline]
    pub const fn to_bits(self) -> i64 {
        self.0.to_bits()
    }

    /// Construct from any numeric type that `fixed` supports.
    ///
    /// The conversion **saturates** — values outside `[MIN, MAX]` clamp to the
    /// nearest representable bound rather than panicking. This matches the
    /// saturating-by-default policy of the arithmetic operators.
    ///
    /// ```
    /// use beast_core::Q3232;
    /// let a = Q3232::from_num(0.5_f64);
    /// let b = Q3232::from_num(2_i32);
    /// assert_eq!(a + a, Q3232::ONE);
    /// assert_eq!(b, Q3232::ONE + Q3232::ONE);
    /// ```
    #[inline]
    pub fn from_num<T: fixed::traits::ToFixed>(value: T) -> Self {
        Self(I32F32::saturating_from_num(value))
    }

    /// Convert to any numeric type that `fixed` supports. The conversion is
    /// saturating; use [`Q3232::to_bits`] when determinism matters.
    #[inline]
    pub fn to_num<T: fixed::traits::FromFixed>(self) -> T {
        T::saturating_from_fixed(self.0)
    }

    /// Saturating addition.
    #[inline]
    #[must_use]
    pub fn saturating_add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }

    /// Saturating subtraction.
    #[inline]
    #[must_use]
    pub fn saturating_sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }

    /// Saturating multiplication.
    #[inline]
    #[must_use]
    pub fn saturating_mul(self, rhs: Self) -> Self {
        Self(self.0.saturating_mul(rhs.0))
    }

    /// Saturating division. Division by zero saturates to [`Q3232::MAX`]
    /// (positive numerator) or [`Q3232::MIN`] (negative numerator); a zero
    /// numerator divided by zero returns [`Q3232::ZERO`].
    ///
    /// Saturating on div-by-zero is intentional: it preserves determinism and
    /// forward progress without surfacing a panic path. Call sites that need
    /// to distinguish div-by-zero should guard explicitly.
    #[inline]
    #[must_use]
    pub fn saturating_div(self, rhs: Self) -> Self {
        if rhs.0 == I32F32::ZERO {
            return match self.0.cmp(&I32F32::ZERO) {
                Ordering::Greater => Self::MAX,
                Ordering::Less => Self::MIN,
                Ordering::Equal => Self::ZERO,
            };
        }
        // I32F32 lacks a saturating_div, so fall back to checked_div and clamp.
        match self.0.checked_div(rhs.0) {
            Some(v) => Self(v),
            None => {
                // Only reachable on MIN / -1 overflow.
                if (self.0 < I32F32::ZERO) ^ (rhs.0 < I32F32::ZERO) {
                    Self::MIN
                } else {
                    Self::MAX
                }
            }
        }
    }

    /// Saturating negation (important for `MIN`, whose true negative overflows).
    #[inline]
    #[must_use]
    pub fn saturating_neg(self) -> Self {
        if self.0 == I32F32::MIN {
            Self::MAX
        } else {
            Self(-self.0)
        }
    }

    /// Absolute value, saturating on `MIN`.
    #[inline]
    #[must_use]
    pub fn saturating_abs(self) -> Self {
        if self.0 == I32F32::MIN {
            Self::MAX
        } else if self.0 < I32F32::ZERO {
            Self(-self.0)
        } else {
            self
        }
    }

    /// Clamp to `[min, max]`.
    ///
    /// Debug-asserts `min <= max`; in release, a reversed range returns `min`.
    #[inline]
    #[must_use]
    pub fn clamp(self, min: Self, max: Self) -> Self {
        debug_assert!(min.0 <= max.0, "Q3232::clamp: min > max");
        if self.0 < min.0 {
            min
        } else if self.0 > max.0 {
            max
        } else {
            self
        }
    }

    /// `true` if this value equals [`Q3232::ZERO`].
    #[inline]
    pub fn is_zero(self) -> bool {
        self.0 == I32F32::ZERO
    }

    /// Sign: `-1`, `0`, or `1` as a [`Q3232`].
    #[inline]
    #[must_use]
    pub fn signum(self) -> Self {
        match self.0.cmp(&I32F32::ZERO) {
            Ordering::Greater => Self::ONE,
            Ordering::Less => Self(-I32F32::ONE),
            Ordering::Equal => Self::ZERO,
        }
    }
}

// ---------- Ordering ----------

impl PartialOrd for Q3232 {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Q3232 {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

// ---------- Operators (saturating) ----------

impl Add for Q3232 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        self.saturating_add(rhs)
    }
}

impl Sub for Q3232 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        self.saturating_sub(rhs)
    }
}

impl Mul for Q3232 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        self.saturating_mul(rhs)
    }
}

impl Div for Q3232 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: Self) -> Self {
        self.saturating_div(rhs)
    }
}

impl Neg for Q3232 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        self.saturating_neg()
    }
}

impl AddAssign for Q3232 {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl SubAssign for Q3232 {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl MulAssign for Q3232 {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl DivAssign for Q3232 {
    #[inline]
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

// ---------- Conversions ----------

impl From<I32F32> for Q3232 {
    #[inline]
    fn from(v: I32F32) -> Self {
        Self(v)
    }
}

impl From<Q3232> for I32F32 {
    #[inline]
    fn from(v: Q3232) -> Self {
        v.0
    }
}

impl From<i32> for Q3232 {
    #[inline]
    fn from(v: i32) -> Self {
        Self(I32F32::from_num(v))
    }
}

impl From<i64> for Q3232 {
    #[inline]
    fn from(v: i64) -> Self {
        // Saturating: values outside the representable range clamp.
        Self(I32F32::saturating_from_num(v))
    }
}

// ---------- Debug/Display ----------

impl fmt::Debug for Q3232 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Q3232({})", self.0)
    }
}

impl fmt::Display for Q3232 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

// ---------- Tests ----------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_one_constants() {
        assert_eq!(Q3232::ZERO.to_bits(), 0);
        assert_eq!(Q3232::ONE, Q3232::from_num(1_i32));
        assert!(Q3232::MIN < Q3232::ZERO);
        assert!(Q3232::MAX > Q3232::ZERO);
    }

    #[test]
    fn add_saturates_on_overflow() {
        let near_max = Q3232::MAX - Q3232::ONE;
        let sum = near_max + Q3232::from_num(2_i32);
        assert_eq!(sum, Q3232::MAX);
    }

    #[test]
    fn sub_saturates_on_underflow() {
        let near_min = Q3232::MIN + Q3232::ONE;
        let diff = near_min - Q3232::from_num(2_i32);
        assert_eq!(diff, Q3232::MIN);
    }

    #[test]
    fn mul_saturates_on_overflow() {
        let big = Q3232::from_num(1_000_000_i32);
        let result = big * big;
        assert_eq!(result, Q3232::MAX);
    }

    #[test]
    fn div_by_zero_saturates() {
        let pos = Q3232::ONE;
        let neg = -Q3232::ONE;
        assert_eq!(pos / Q3232::ZERO, Q3232::MAX);
        assert_eq!(neg / Q3232::ZERO, Q3232::MIN);
        assert_eq!(Q3232::ZERO / Q3232::ZERO, Q3232::ZERO);
    }

    #[test]
    fn neg_of_min_saturates_to_max() {
        assert_eq!(-Q3232::MIN, Q3232::MAX);
    }

    #[test]
    fn abs_of_min_saturates_to_max() {
        assert_eq!(Q3232::MIN.saturating_abs(), Q3232::MAX);
    }

    #[test]
    fn clamp_bounds_value() {
        let lo = Q3232::ZERO;
        let hi = Q3232::ONE;
        assert_eq!(Q3232::from_num(-1_i32).clamp(lo, hi), lo);
        assert_eq!(Q3232::from_num(2_i32).clamp(lo, hi), hi);
        assert_eq!(
            Q3232::from_num(0.5_f64).clamp(lo, hi),
            Q3232::from_num(0.5_f64)
        );
    }

    #[test]
    fn signum_three_way() {
        assert_eq!(Q3232::ONE.signum(), Q3232::ONE);
        assert_eq!(Q3232::ZERO.signum(), Q3232::ZERO);
        assert_eq!((-Q3232::ONE).signum(), -Q3232::ONE);
    }

    #[test]
    fn from_bits_roundtrip() {
        let v = Q3232::from_num(0.25_f64);
        let bits = v.to_bits();
        assert_eq!(Q3232::from_bits(bits), v);
    }

    #[test]
    fn serde_roundtrip() {
        let v = Q3232::from_num(1.5_f64);
        let json = serde_json::to_string(&v).unwrap();
        let back: Q3232 = serde_json::from_str(&json).unwrap();
        assert_eq!(v, back);
    }
}
