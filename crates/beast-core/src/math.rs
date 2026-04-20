//! Math utilities used across the simulation.
//!
//! * [`clamp01`] / [`clamp_q3232`] — bounded clamps in `Q3232`.
//! * [`lerp_q3232`] / [`inv_lerp_q3232`] — linear interpolation primitives.
//! * [`min_q3232`] / [`max_q3232`] — total-order min/max that are safe with
//!   `Q3232`'s `Ord` implementation.
//! * [`gaussian_q3232`] — Box–Muller Gaussian sampler.
//!
//! ### Box–Muller rationale
//!
//! Box–Muller is the simplest standard Gaussian sampler that round-trips
//! cleanly through a deterministic PRNG: two uniform draws produce two
//! independent normal draws via
//! `z0 = sqrt(-2 ln u1) cos(2π u2)`. Ziggurat is faster but depends on
//! precomputed float lookup tables and branch-prediction assumptions that
//! complicate cross-platform determinism.
//!
//! The transform itself runs through `f64` (not `Q3232`) because `ln`, `sqrt`,
//! and `cos` have no exact fixed-point equivalents. `f64`'s IEEE-754 output
//! is not strictly bit-identical across every platform compiler flag, but
//! **for these three functions evaluated on `f64` operands** the results are
//! identical across all major x86/x64/ARM64 targets we care about, provided
//! the CPU is in its default rounding mode. The result is converted back to
//! `Q3232` via saturating conversion, which is deterministic. If a future
//! determinism audit flags this we can swap in a fixed-point approximation
//! (Padé / Chebyshev) without changing the public API.
//!
//! The crate-level `clippy::float_arithmetic = "warn"` lint is suppressed
//! locally here because this function is the *one* sanctioned float use in
//! `beast-core`.

use crate::fixed_point::Q3232;
use crate::prng::Prng;

/// Clamp a [`Q3232`] value to `[0, 1]`.
#[inline]
#[must_use]
pub fn clamp01(v: Q3232) -> Q3232 {
    v.clamp(Q3232::ZERO, Q3232::ONE)
}

/// Reflect a [`Q3232`] value back into `[0, 1]` off the nearest boundary,
/// then hard-clamp any residual overshoot.
///
/// Reflection preserves the Gaussian distribution shape near boundaries
/// better than truncation. For a value `v`:
/// - If `v < 0`: reflect off 0 → `-v`
/// - If `v > 1`: reflect off 1 → `2 - v`
///
/// A final clamp handles pathological outliers (e.g. `v > 2` or `v < -1`)
/// that overshoot even after one reflection.
#[inline]
#[must_use]
pub fn reflect_clamp01(v: Q3232) -> Q3232 {
    if v >= Q3232::ZERO && v <= Q3232::ONE {
        return v;
    }
    let two = Q3232::from_num(2_i32);
    let reflected = if v < Q3232::ZERO { -v } else { two - v };
    reflected.clamp(Q3232::ZERO, Q3232::ONE)
}

/// Clamp a [`Q3232`] value to `[lo, hi]`. Debug-asserts `lo <= hi`.
#[inline]
#[must_use]
pub fn clamp_q3232(v: Q3232, lo: Q3232, hi: Q3232) -> Q3232 {
    v.clamp(lo, hi)
}

/// Minimum of two [`Q3232`] values (saturating semantics inherited from `Ord`).
#[inline]
#[must_use]
pub fn min_q3232(a: Q3232, b: Q3232) -> Q3232 {
    if a <= b {
        a
    } else {
        b
    }
}

/// Maximum of two [`Q3232`] values.
#[inline]
#[must_use]
pub fn max_q3232(a: Q3232, b: Q3232) -> Q3232 {
    if a >= b {
        a
    } else {
        b
    }
}

/// Linear interpolation: `a * (1 - t) + b * t`, evaluated via
/// `a + (b - a) * t`. Saturating on each sub-operation.
#[inline]
#[must_use]
pub fn lerp_q3232(a: Q3232, b: Q3232, t: Q3232) -> Q3232 {
    a + (b - a) * t
}

/// Inverse linear interpolation: returns `t` such that `lerp(a, b, t) = v`,
/// clamped to `[0, 1]`. Returns `0` when `a == b` (saturating div-by-zero
/// already handled inside `/`).
#[inline]
#[must_use]
pub fn inv_lerp_q3232(a: Q3232, b: Q3232, v: Q3232) -> Q3232 {
    if a == b {
        Q3232::ZERO
    } else {
        clamp01((v - a) / (b - a))
    }
}

/// Sample from a Gaussian (normal) distribution with the given mean and
/// standard deviation, using the Box–Muller transform.
///
/// See the module docs for the determinism rationale. The call consumes two
/// `u64`s from `rng`, and returns one sample per call (the second Box–Muller
/// output is discarded — callers that need both should add a paired variant).
///
/// ```
/// use beast_core::{gaussian_q3232, Prng, Q3232};
/// let mut rng = Prng::from_seed(1);
/// let mean = Q3232::from_num(5_i32);
/// let stddev = Q3232::from_num(2_i32);
/// let sample = gaussian_q3232(&mut rng, mean, stddev);
/// // Sanity: samples within ±8 stddev are overwhelmingly likely.
/// let delta = (sample - mean).saturating_abs();
/// assert!(delta <= Q3232::from_num(16_i32));
/// ```
#[allow(clippy::float_arithmetic)]
pub fn gaussian_q3232(rng: &mut Prng, mean: Q3232, stddev: Q3232) -> Q3232 {
    // Draw two uniforms in (0, 1]. Box–Muller requires u1 > 0 because of ln.
    // We shift the standard [0, 1) draw by one ULP worth (2^-53) so that
    // u1 is strictly positive. This biases the distribution by a negligible
    // amount (< 2^-53) compared to the sampling noise itself.
    let u1 = {
        let raw = rng.next_f64_unit();
        // Map [0, 1) to (0, 1] by adding 2^-53 (smallest f64 representable
        // increment in that range). This is the standard technique used by
        // libm-style Gaussian samplers.
        raw + f64::from_bits(0x3CA0_0000_0000_0000) // 2^-53
    };
    let u2 = rng.next_f64_unit();

    // Box–Muller.
    let r = (-2.0_f64 * u1.ln()).sqrt();
    let theta = core::f64::consts::TAU * u2;
    let z0 = r * theta.cos();

    // Convert back into Q3232. `z0` is unbounded in principle; 8σ covers
    // > 99.99999999999% of draws. Saturating conversion handles the rest.
    let mean_f = mean.to_num::<f64>();
    let stddev_f = stddev.to_num::<f64>();
    let sample_f = z0.mul_add(stddev_f, mean_f);
    Q3232::from_num(sample_f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp01_bounds() {
        assert_eq!(clamp01(Q3232::from_num(-0.1_f64)), Q3232::ZERO);
        assert_eq!(clamp01(Q3232::from_num(1.1_f64)), Q3232::ONE);
        assert_eq!(clamp01(Q3232::from_num(0.5_f64)), Q3232::from_num(0.5_f64));
    }

    #[test]
    fn reflect_clamp01_in_range_passthrough() {
        assert_eq!(reflect_clamp01(Q3232::ZERO), Q3232::ZERO);
        assert_eq!(reflect_clamp01(Q3232::ONE), Q3232::ONE);
        assert_eq!(
            reflect_clamp01(Q3232::from_num(0.5_f64)),
            Q3232::from_num(0.5_f64)
        );
    }

    #[test]
    fn reflect_clamp01_reflects_above_one() {
        assert_eq!(
            reflect_clamp01(Q3232::from_num(1.3_f64)),
            Q3232::from_num(0.7_f64)
        );
        assert_eq!(
            reflect_clamp01(Q3232::from_num(1.8_f64)),
            Q3232::from_num(0.2_f64)
        );
    }

    #[test]
    fn reflect_clamp01_reflects_below_zero() {
        assert_eq!(
            reflect_clamp01(Q3232::from_num(-0.3_f64)),
            Q3232::from_num(0.3_f64)
        );
        assert_eq!(
            reflect_clamp01(Q3232::from_num(-0.9_f64)),
            Q3232::from_num(0.9_f64)
        );
    }

    #[test]
    fn reflect_clamp01_clamps_pathological() {
        assert_eq!(reflect_clamp01(Q3232::from_num(2.5_f64)), Q3232::ZERO);
        assert_eq!(reflect_clamp01(Q3232::from_num(-1.5_f64)), Q3232::ONE);
    }

    #[test]
    fn lerp_endpoints_and_midpoint() {
        let a = Q3232::ZERO;
        let b = Q3232::from_num(10_i32);
        assert_eq!(lerp_q3232(a, b, Q3232::ZERO), a);
        assert_eq!(lerp_q3232(a, b, Q3232::ONE), b);
        assert_eq!(
            lerp_q3232(a, b, Q3232::from_num(0.5_f64)),
            Q3232::from_num(5_i32)
        );
    }

    #[test]
    fn inv_lerp_roundtrip() {
        let a = Q3232::from_num(2_i32);
        let b = Q3232::from_num(8_i32);
        let v = Q3232::from_num(5_i32);
        // 5 is halfway between 2 and 8 → t = 0.5
        assert_eq!(inv_lerp_q3232(a, b, v), Q3232::from_num(0.5_f64));
    }

    #[test]
    fn inv_lerp_equal_endpoints_returns_zero() {
        let a = Q3232::from_num(3_i32);
        assert_eq!(inv_lerp_q3232(a, a, a), Q3232::ZERO);
    }

    #[test]
    fn min_max_basics() {
        let a = Q3232::from_num(1_i32);
        let b = Q3232::from_num(2_i32);
        assert_eq!(min_q3232(a, b), a);
        assert_eq!(max_q3232(a, b), b);
        assert_eq!(min_q3232(a, a), a);
    }

    #[test]
    fn gaussian_is_deterministic() {
        let mut a = Prng::from_seed(1234);
        let mut b = Prng::from_seed(1234);
        let mean = Q3232::ZERO;
        let sd = Q3232::ONE;
        for _ in 0..256 {
            assert_eq!(
                gaussian_q3232(&mut a, mean, sd),
                gaussian_q3232(&mut b, mean, sd)
            );
        }
    }

    #[test]
    fn gaussian_mean_is_close_to_requested() {
        // Empirical: 10k draws from N(0, 1) should have |mean| < 0.05.
        let mut rng = Prng::from_seed(5);
        let mean = Q3232::ZERO;
        let sd = Q3232::ONE;
        let n = 10_000;
        let mut sum = Q3232::ZERO;
        for _ in 0..n {
            sum += gaussian_q3232(&mut rng, mean, sd);
        }
        let empirical_mean = sum / Q3232::from_num(n);
        assert!(
            empirical_mean.saturating_abs() < Q3232::from_num(0.05_f64),
            "empirical mean {:?} too far from 0",
            empirical_mean
        );
    }

    #[test]
    fn gaussian_stddev_is_close_to_requested() {
        // Empirical: 10k draws from N(0, 1) have stddev within 0.05 of 1.
        let mut rng = Prng::from_seed(6);
        let mean = Q3232::ZERO;
        let sd = Q3232::ONE;
        let n = 10_000_i32;
        let mut samples: Vec<Q3232> = Vec::with_capacity(n as usize);
        let mut sum = Q3232::ZERO;
        for _ in 0..n {
            let s = gaussian_q3232(&mut rng, mean, sd);
            sum += s;
            samples.push(s);
        }
        let m = sum / Q3232::from_num(n);
        let mut var = Q3232::ZERO;
        for s in &samples {
            let d = *s - m;
            var += d * d;
        }
        var /= Q3232::from_num(n - 1);
        // sqrt via f64 (acceptable in a test).
        let var_f: f64 = var.to_num();
        let std_emp = var_f.sqrt();
        assert!(
            (std_emp - 1.0).abs() < 0.05,
            "empirical stddev {} too far from 1",
            std_emp
        );
    }

    #[test]
    fn gaussian_respects_mean_and_scale() {
        // Draws from N(100, 0.01) should have mean ~100, very tight spread.
        let mut rng = Prng::from_seed(7);
        let mean = Q3232::from_num(100_i32);
        let sd = Q3232::from_num(0.01_f64);
        for _ in 0..200 {
            let s = gaussian_q3232(&mut rng, mean, sd);
            let delta = (s - mean).saturating_abs();
            // 8σ bound.
            assert!(
                delta <= Q3232::from_num(0.08_f64),
                "sample {:?} outside 8σ",
                s
            );
        }
    }
}
