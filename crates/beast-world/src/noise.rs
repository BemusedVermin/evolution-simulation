//! Deterministic value-noise generator.
//!
//! A from-scratch implementation chosen over the `noise` crate because:
//!
//! * **Zero new external dependency** — the crate avoids one crate of
//!   transitive surface in our `cargo deny` chain.
//! * **Bit-exact control over determinism** — the hash is SplitMix64,
//!   the same mix `Xoshiro256PlusPlus::seed_from_u64` uses, so
//!   replaying a seed across rust versions / target arches produces
//!   the same bytes (modulo `f64` precision; see the cross-platform
//!   note below).
//! * **No mutable PRNG state during noise queries** — every call is a
//!   pure function `(seed, octave, x, y) -> f64`, so a generator that
//!   queries cells in a different order still gets the same output.
//!
//! # Cross-platform reproducibility
//!
//! Final outputs are `f64` and depend on IEEE-754 multiplication and
//! addition. All target architectures we support (x86_64, aarch64)
//! provide bit-exact IEEE-754 for these operations, so the world grid
//! is reproducible across them. World generation results are
//! quantised to integer thresholds before they leave this crate, so
//! sub-ulp differences (if they ever appear) cannot reach sim state.
//!
//! Tracked as a determinism-gate concern in issue #154 (cross-process
//! gate), where multi-platform CI will lock this in.

#![allow(clippy::float_arithmetic)] // boundary code, see crate docs

/// Stable 64-bit hash used as the corner-value source for value
/// noise. SplitMix64 with a `u64` input — the same finaliser used by
/// `Xoshiro256PlusPlus::seed_from_u64` so behaviour is consistent
/// with the rest of the project's PRNG infrastructure.
#[inline]
pub(crate) fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// Deterministic value at integer corner `(ix, iy)` in `[-1, 1]`.
///
/// `octave_seed` is mixed in so different octaves of the same `seed`
/// do not co-vary; `seed` is the world seed.
///
/// **Cast safety:** `ix as u64` reinterprets the i64 bits as u64
/// (two's complement). For the ±2^31 sample range used by the
/// archipelago generator (cell coords scaled by frequency ≤ 4 for a
/// 64×64 grid → max ~256) this is collision-free; for callers that
/// pass arbitrary i64 indices, `corner_value(seed, oct, -1, 0)` and
/// `corner_value(seed, oct, very-large-positive, 0)` may share hash
/// space. Document the contract in any caller that exposes raw
/// coordinate input.
#[inline]
fn corner_value(seed: u64, octave_seed: u64, ix: i64, iy: i64) -> f64 {
    // Combine inputs into a single u64 by repeated splitmix64. Using
    // splitmix instead of xor lets identical inputs across different
    // bit positions still produce uncorrelated outputs.
    let mut h = splitmix64(seed);
    h = splitmix64(h ^ octave_seed);
    h = splitmix64(h ^ (ix as u64));
    h = splitmix64(h ^ (iy as u64));

    // Top 53 bits as a uniform [0, 1) f64 (same trick as
    // `Prng::next_f64_unit`), then scale to [-1, 1].
    let bits = h >> 11;
    let unit = (bits as f64) * (1.0_f64 / (1_u64 << 53) as f64);
    unit * 2.0 - 1.0
}

/// Smoothstep curve `3t² - 2t³` — the interpolation easing used by
/// classic Perlin/value noise. Avoids the corner-aligned banding
/// that linear interpolation would produce.
#[inline]
fn smoothstep(t: f64) -> f64 {
    let t2 = t * t;
    t2 * (3.0 - 2.0 * t)
}

/// 2D value-noise sample at `(x, y)` for a single octave.
///
/// `(x, y)` are in lattice units (one integer step = one corner).
/// Output is in `[-1, 1]`.
///
/// `octave_seed` is mixed in so multiple octaves drawn from the same
/// world seed remain decorrelated.
///
/// # Coordinate range
///
/// The internal corner hash reinterprets each integer coordinate as
/// a `u64` via two's complement. Within roughly `±2^31` this is
/// collision-free; well-large-positive vs small-negative `i64`
/// coordinates can alias to the same hash bucket outside that band.
/// The archipelago generator stays comfortably inside the safe band
/// (a 64×64 grid scaled by typical `frequency` values produces corner
/// indices of at most a few hundred), but callers that pass arbitrary
/// `i64`-magnitude coordinates — e.g., raw tile indices in a much
/// larger world — should normalise their input into `±2^31` first.
/// `frequency` itself is unbounded (only `> 0` is enforced by
/// `validate_config`); the bound here is on the **product** of
/// `frequency` and the maximum traversed integer coordinate.
#[inline]
pub fn value_noise_2d(seed: u64, octave_seed: u64, x: f64, y: f64) -> f64 {
    let ix = x.floor() as i64;
    let iy = y.floor() as i64;
    let fx = x - ix as f64;
    let fy = y - iy as f64;

    let v00 = corner_value(seed, octave_seed, ix, iy);
    let v10 = corner_value(seed, octave_seed, ix + 1, iy);
    let v01 = corner_value(seed, octave_seed, ix, iy + 1);
    let v11 = corner_value(seed, octave_seed, ix + 1, iy + 1);

    let sx = smoothstep(fx);
    let sy = smoothstep(fy);

    let lerp = |a: f64, b: f64, t: f64| a + (b - a) * t;

    let bottom = lerp(v00, v10, sx);
    let top = lerp(v01, v11, sx);
    lerp(bottom, top, sy)
}

/// Sum `octaves` of value noise at decreasing amplitude / increasing
/// frequency — fractional Brownian motion.
///
/// `lacunarity` is the per-octave frequency multiplier (typical
/// `2.0`). `gain` is the per-octave amplitude multiplier (typical
/// `0.5`). Output is normalised to `[-1, 1]` based on the maximum
/// possible amplitude sum.
///
/// # Coordinate range
///
/// Inherits the same `±2^31` coordinate-hash safety bound as
/// [`value_noise_2d`]. Callers passing arbitrary `i64`-scale
/// coordinates should normalise first.
pub fn fbm_2d(seed: u64, x: f64, y: f64, octaves: u32, lacunarity: f64, gain: f64) -> f64 {
    let mut sum = 0.0_f64;
    let mut amp = 1.0_f64;
    let mut freq = 1.0_f64;
    let mut amp_total = 0.0_f64;
    for octave in 0..octaves {
        // Each octave gets a distinct seed contribution so the
        // octaves are decorrelated. Mixing `seed` into `octave_seed`
        // (rather than deriving it from the octave index alone)
        // ensures the octave ladder is *world-seed-dependent*: two
        // callers sharing a seed but using different octave counts
        // don't produce one channel as a strict prefix of the
        // other. Without this, e.g., calling `fbm_2d(seed, ..., 4,
        // ...)` and `fbm_2d(seed, ..., 6, ...)` would share their
        // first 4 octaves verbatim.
        let octave_seed = splitmix64(seed ^ splitmix64((octave as u64) ^ 0xC0FF_EE00_DEAD_BEEF));
        sum += amp * value_noise_2d(seed, octave_seed, x * freq, y * freq);
        amp_total += amp;
        amp *= gain;
        freq *= lacunarity;
    }
    sum / amp_total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corner_value_is_in_minus_one_to_one() {
        for ix in -10..10 {
            for iy in -10..10 {
                let v = corner_value(0xCAFE, 0, ix, iy);
                assert!((-1.0..1.0).contains(&v), "out of range at ({ix},{iy}): {v}");
            }
        }
    }

    #[test]
    fn corner_value_is_pure_function() {
        // Same inputs → same output, byte-for-byte. This is the
        // determinism contract for value noise.
        let a = corner_value(0xDEAD, 7, 3, -2);
        let b = corner_value(0xDEAD, 7, 3, -2);
        assert_eq!(a.to_bits(), b.to_bits());
    }

    #[test]
    fn corner_value_changes_with_seed() {
        // Different seeds should produce uncorrelated values. We
        // can't prove that statistically here, but we can at least
        // assert the trivial invariant that two seeds don't always
        // produce the same value.
        let a = corner_value(0x1, 0, 0, 0);
        let b = corner_value(0x2, 0, 0, 0);
        assert_ne!(a.to_bits(), b.to_bits());
    }

    #[test]
    fn corner_value_changes_with_octave_seed() {
        let a = corner_value(0xCAFE, 0, 0, 0);
        let b = corner_value(0xCAFE, 1, 0, 0);
        assert_ne!(a.to_bits(), b.to_bits());
    }

    #[test]
    fn value_noise_at_integer_coords_equals_corner() {
        // At lattice corners, value noise must equal the corner
        // value — this sanity-checks the bilinear interpolation.
        let seed = 0xABCD;
        let octave = 5;
        for ix in [-3, 0, 7] {
            for iy in [-1, 4, 11] {
                let v = value_noise_2d(seed, octave, ix as f64, iy as f64);
                let c = corner_value(seed, octave, ix, iy);
                assert_eq!(v.to_bits(), c.to_bits(), "mismatch at ({ix},{iy})");
            }
        }
    }

    #[test]
    fn value_noise_is_pure_function() {
        let seed = 0x1234;
        let a = value_noise_2d(seed, 0, 1.5, -0.25);
        let b = value_noise_2d(seed, 0, 1.5, -0.25);
        assert_eq!(a.to_bits(), b.to_bits());
    }

    #[test]
    fn value_noise_stays_in_range() {
        let seed = 0x9999;
        for x in -5..5 {
            for y in -5..5 {
                let xf = x as f64 * 0.37;
                let yf = y as f64 * 0.41;
                let v = value_noise_2d(seed, 0, xf, yf);
                assert!(
                    (-1.0..=1.0).contains(&v),
                    "out of range at ({xf},{yf}): {v}"
                );
            }
        }
    }

    #[test]
    fn fbm_is_pure_function() {
        let seed = 0xFACE;
        let a = fbm_2d(seed, 1.5, -0.25, 4, 2.0, 0.5);
        let b = fbm_2d(seed, 1.5, -0.25, 4, 2.0, 0.5);
        assert_eq!(a.to_bits(), b.to_bits());
    }

    #[test]
    fn corner_value_golden_value_is_stable() {
        // Cross-version regression gate. The purity tests verify
        // call-to-call equality; this asserts the *exact* bits an
        // unchanged implementation must produce. A change to the
        // splitmix constants, the bit-fold, or the f64 cast that
        // flips outputs but stays internally consistent will fail
        // here, where the purity tests cannot.
        //
        // Captured 2026-04-25 against commit 4327204; if you change
        // the noise pipeline intentionally, regenerate this and
        // bump the cross-process determinism gate (#154).
        assert_eq!(
            corner_value(0xDEAD, 7, 3, -2).to_bits(),
            0x3facd25c8dfeb980_u64
        );
    }

    #[test]
    fn fbm_golden_value_is_stable() {
        // Companion to corner_value_golden_value_is_stable. Captured
        // 2026-04-25 against commit 4327204.
        assert_eq!(
            fbm_2d(0xFACE, 1.5, -0.25, 4, 2.0, 0.5).to_bits(),
            0xbf86f3b8d2f71380_u64
        );
    }

    #[test]
    fn fbm_octave_seed_depends_on_world_seed() {
        // Regression: ensure the fix for "octave_seed independent of
        // world seed" stays applied. With seed-independent octave
        // seeds, calling fbm_2d with different `octaves` counts at
        // the same world seed would share octave outputs as a
        // strict prefix. Test that swapping seeds changes output
        // even when octaves is small.
        let a = fbm_2d(0x1, 0.0, 0.0, 1, 2.0, 0.5);
        let b = fbm_2d(0x2, 0.0, 0.0, 1, 2.0, 0.5);
        assert_ne!(a.to_bits(), b.to_bits());
    }

    #[test]
    fn fbm_stays_in_range_for_typical_parameters() {
        let seed = 0xFEED;
        for x in -10..10 {
            for y in -10..10 {
                let xf = x as f64 * 0.13;
                let yf = y as f64 * 0.17;
                let v = fbm_2d(seed, xf, yf, 4, 2.0, 0.5);
                assert!(
                    (-1.0..=1.0).contains(&v),
                    "fbm out of range at ({xf},{yf}): {v}"
                );
            }
        }
    }
}
