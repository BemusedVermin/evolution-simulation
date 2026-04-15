//! Math utilities. Populated in Story 1.4.

use crate::fixed_point::Q3232;
use crate::prng::Prng;

/// Clamp a [`Q3232`] value to `[0, 1]`. Full implementation lands in Story 1.4.
#[inline]
pub fn clamp01(v: Q3232) -> Q3232 {
    v.clamp(Q3232::ZERO, Q3232::ONE)
}

/// Linear interpolation between `a` and `b` by `t`. Stub for Story 1.4.
#[inline]
pub fn lerp_q3232(a: Q3232, b: Q3232, t: Q3232) -> Q3232 {
    a + (b - a) * t
}

/// Box–Muller Gaussian sample. Implementation lands in Story 1.4.
pub fn gaussian_q3232(_rng: &mut Prng, _mean: Q3232, _stddev: Q3232) -> Q3232 {
    Q3232::ZERO
}
