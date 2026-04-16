//! Deterministic pseudo-random number generation.
//!
//! The simulation uses **one** master seed chosen at world creation, and
//! derives an independent [`Prng`] stream per subsystem via
//! [`Prng::split_stream`]. Splitting is performed by calling
//! `Xoshiro256PlusPlus::long_jump` a stream-dependent number of times; each
//! long-jump is equivalent to `2^192` `next_u64` calls, so two streams derived
//! from the same master will not overlap within any plausible simulation
//! lifetime.
//!
//! Crate-wide invariants:
//!
//! * The OS RNG is never used. `Prng::new` takes an explicit seed.
//! * `Prng` is `Serialize`/`Deserialize`, so the full PRNG state round-trips
//!   through save files — essential for replay determinism.
//! * `Stream` is an enum rather than a `u64` so that adding a new subsystem
//!   forces explicit allocation of a stream slot rather than accidentally
//!   colliding with an existing one.

use rand_core::{RngCore, SeedableRng};
use rand_xoshiro::Xoshiro256PlusPlus;
use serde::{Deserialize, Serialize};

/// Enumeration of per-subsystem PRNG streams. Each variant corresponds to
/// exactly one independent `Prng` instance in the simulation.
///
/// **Adding a new variant**: append only — never reorder, never remove. The
/// discriminant drives the `long_jump` count used to derive the stream, so
/// reordering would shuffle every subsystem's stream and break replay
/// compatibility with all existing saves.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u16)]
pub enum Stream {
    /// Genetic mutation operators (point mutation, duplication, etc.).
    Genetics = 0,
    /// Phenotype interpreter stochastic tie-breaks.
    Phenotype = 1,
    /// Physics and movement jitter.
    Physics = 2,
    /// Combat and interaction resolution.
    Combat = 3,
    /// Physiology (metabolism, ageing) randomness.
    Physiology = 4,
    /// Ecology (spawning, biome events).
    Ecology = 5,
    /// World generation (terrain, biome placement).
    Worldgen = 6,
    /// Chronicler sampling and clustering randomness.
    Chronicler = 7,
    /// Explicit testing stream — never used by the live simulation.
    Testing = 0xFFFF,
}

impl Stream {
    /// How many `long_jump()` invocations separate this stream from the root.
    #[inline]
    pub const fn jumps(self) -> u32 {
        // `self as u16 as u32` is a const-friendly discriminant read.
        self as u16 as u32
    }
}

/// Deterministic 64-bit PRNG used by every simulation subsystem.
///
/// Construct a master [`Prng`] with [`Prng::from_seed`]; derive per-subsystem
/// children with [`Prng::split_stream`]. Never instantiate `Xoshiro256PlusPlus`
/// directly elsewhere in the codebase.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Prng {
    inner: Xoshiro256PlusPlus,
}

impl Prng {
    /// Construct a master PRNG from a 64-bit seed.
    ///
    /// `seed_from_u64` internally runs SplitMix64 to expand the seed into the
    /// 256-bit Xoshiro state; this is documented and stable across
    /// `rand_xoshiro` versions.
    #[inline]
    pub fn from_seed(seed: u64) -> Self {
        Self {
            inner: Xoshiro256PlusPlus::seed_from_u64(seed),
        }
    }

    /// Construct from an explicit 32-byte Xoshiro seed. Use this only when
    /// loading an existing PRNG state from a save file; new worlds should go
    /// through [`Prng::from_seed`].
    #[inline]
    pub fn from_state(state: [u8; 32]) -> Self {
        Self {
            inner: Xoshiro256PlusPlus::from_seed(state),
        }
    }

    /// Derive an independent child stream for a named subsystem.
    ///
    /// Implementation: clone the master state, then apply
    /// `stream.jumps() + 1` long-jumps. The `+ 1` guarantees the child is
    /// non-aliased with the master even when the stream discriminant is `0`
    /// (otherwise `Stream::Genetics`, disc = 0, would share the master's
    /// sequence). The result is a PRNG whose sequence does not overlap with
    /// any other stream or with the master for `2^192` steps per distinct
    /// `Stream` variant.
    ///
    /// The master is unaffected — it remains positioned at its pre-split
    /// state so subsequent calls to `split_stream` are reproducible.
    #[inline]
    pub fn split_stream(&self, stream: Stream) -> Self {
        let mut child = self.inner.clone();
        // +1 so discriminant 0 still jumps once (master-disjoint).
        for _ in 0..=stream.jumps() {
            child.long_jump();
        }
        Self { inner: child }
    }

    /// Generate the next `u64`.
    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        self.inner.next_u64()
    }

    /// Generate the next `u32`.
    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        self.inner.next_u32()
    }

    /// Generate a uniform `f64` in `[0, 1)` — **render/UI use only**. Never
    /// call this from a sim-state system; use [`Prng::next_q3232_unit`]
    /// instead.
    ///
    /// This helper exists so render code has a convenient entry point
    /// without hand-rolling the bit tricks.
    #[inline]
    #[allow(clippy::float_arithmetic)]
    pub fn next_f64_unit(&mut self) -> f64 {
        // 53-bit mantissa trick: take top 53 bits of next_u64, divide by 2^53.
        let bits = self.next_u64() >> 11;
        (bits as f64) * (1.0_f64 / (1_u64 << 53) as f64)
    }

    /// Generate a uniform [`crate::Q3232`] in `[0, 1)`.
    ///
    /// Uses the top 32 bits of `next_u64` as the fractional part, which
    /// exactly fills Q32.32's fractional range. This is the canonical way
    /// for sim systems to draw a random unit-interval value.
    #[inline]
    pub fn next_q3232_unit(&mut self) -> crate::Q3232 {
        let bits = (self.next_u64() >> 32) as i64; // top 32 bits → [0, 2^32)
        crate::Q3232::from_bits(bits)
    }

    /// Draw a `u64` in `[low, high)`. Panics in debug if `low >= high`. Uses
    /// the widening-multiply unbiased technique (no rejection loop) — this
    /// has a tiny statistical bias for ranges that are not a power of two,
    /// acceptable for sim use where we want bounded-time draws.
    #[inline]
    pub fn gen_range_u64(&mut self, low: u64, high: u64) -> u64 {
        debug_assert!(low < high, "gen_range_u64: empty range");
        let span = high - low;
        // widening: (next_u64 * span) >> 64
        let product = (self.next_u64() as u128) * (span as u128);
        low + (product >> 64) as u64
    }

    /// Draw an `i64` in `[low, high)`.
    ///
    /// Handles ranges wider than `i64::MAX` correctly (e.g.
    /// `low = i64::MIN, high = i64::MAX`). The final addition is done in
    /// `i128` and then narrowed — the narrow is always in-range because the
    /// widening-multiply result is guaranteed to lie in `[0, span)`.
    #[inline]
    pub fn gen_range_i64(&mut self, low: i64, high: i64) -> i64 {
        debug_assert!(low < high, "gen_range_i64: empty range");
        let span = (high as i128 - low as i128) as u64;
        let product = (self.next_u64() as u128) * (span as u128);
        let offset = (product >> 64) as u64; // in [0, span)
                                             // Add in i128 to avoid overflow when span > i64::MAX; the sum is
                                             // always in [low, high) which fits in i64 by construction.
        ((low as i128) + (offset as i128)) as i64
    }

    /// Flip an unbiased coin.
    #[inline]
    pub fn next_bool(&mut self) -> bool {
        (self.next_u64() >> 63) == 1
    }
}

impl RngCore for Prng {
    #[inline]
    fn next_u32(&mut self) -> u32 {
        self.inner.next_u32()
    }
    #[inline]
    fn next_u64(&mut self) -> u64 {
        self.inner.next_u64()
    }
    #[inline]
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.inner.fill_bytes(dest)
    }
}

// ---------- Tests ----------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_produces_identical_sequence() {
        let mut a = Prng::from_seed(0xDEAD_BEEF);
        let mut b = Prng::from_seed(0xDEAD_BEEF);
        for _ in 0..1024 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn different_seeds_diverge() {
        let mut a = Prng::from_seed(1);
        let mut b = Prng::from_seed(2);
        // Overwhelmingly likely: find divergence in the first 16 draws.
        let mut differed = false;
        for _ in 0..16 {
            if a.next_u64() != b.next_u64() {
                differed = true;
                break;
            }
        }
        assert!(differed);
    }

    #[test]
    fn split_streams_are_independent() {
        let master = Prng::from_seed(42);
        let mut genetics = master.split_stream(Stream::Genetics);
        let mut physics = master.split_stream(Stream::Physics);
        // Different streams should produce different first draws.
        assert_ne!(genetics.next_u64(), physics.next_u64());
    }

    #[test]
    fn split_stream_does_not_alias_master() {
        // Regression: Stream::Genetics has discriminant 0; the split logic
        // must still produce a PRNG disjoint from the master (otherwise any
        // sim code that accidentally drew from the master would collide
        // with the Genetics stream).
        let master = Prng::from_seed(99);
        let mut m = master.clone();
        let mut g = master.split_stream(Stream::Genetics);
        assert_ne!(m.next_u64(), g.next_u64());
    }

    #[test]
    fn split_stream_is_deterministic() {
        let m1 = Prng::from_seed(42);
        let m2 = Prng::from_seed(42);
        let mut a = m1.split_stream(Stream::Combat);
        let mut b = m2.split_stream(Stream::Combat);
        for _ in 0..256 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn split_does_not_advance_master() {
        let master = Prng::from_seed(7);
        let mut m1 = master.clone();
        let _child = master.split_stream(Stream::Physiology);
        let mut m2 = master.clone();
        // Master is unchanged by split.
        for _ in 0..32 {
            assert_eq!(m1.next_u64(), m2.next_u64());
        }
    }

    #[test]
    fn serde_roundtrip_preserves_sequence() {
        let mut a = Prng::from_seed(123);
        // Advance the state a bit.
        for _ in 0..17 {
            a.next_u64();
        }
        let serialized = serde_json::to_string(&a).unwrap();
        let mut b: Prng = serde_json::from_str(&serialized).unwrap();
        for _ in 0..128 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn next_q3232_unit_in_range() {
        use crate::Q3232;
        let mut rng = Prng::from_seed(9);
        for _ in 0..10_000 {
            let v = rng.next_q3232_unit();
            assert!(v >= Q3232::ZERO);
            assert!(v < Q3232::ONE);
        }
    }

    #[test]
    fn gen_range_u64_respects_bounds() {
        let mut rng = Prng::from_seed(10);
        for _ in 0..1_000 {
            let v = rng.gen_range_u64(10, 20);
            assert!((10..20).contains(&v));
        }
    }

    #[test]
    fn gen_range_i64_respects_bounds() {
        let mut rng = Prng::from_seed(11);
        for _ in 0..1_000 {
            let v = rng.gen_range_i64(-5, 5);
            assert!((-5..5).contains(&v));
        }
    }

    #[test]
    fn gen_range_i64_handles_span_larger_than_i64_max() {
        // Regression for PR review: span = u64::MAX was wrapping negative
        // through `(product >> 64) as i64`, causing `low + negative` to
        // overflow on extreme endpoints.
        let mut rng = Prng::from_seed(0xF1F1_F1F1);
        for _ in 0..10_000 {
            let v = rng.gen_range_i64(i64::MIN, i64::MAX);
            assert!(v < i64::MAX);
        }
    }

    #[test]
    fn gen_range_i64_asymmetric_wide_spans() {
        // Two wide spans that previously overflowed: (MIN, 0) and (0, MAX).
        let mut rng = Prng::from_seed(0x5A5A_5A5A);
        for _ in 0..2_000 {
            let v = rng.gen_range_i64(i64::MIN, 0);
            assert!(v < 0);
        }
        for _ in 0..2_000 {
            let v = rng.gen_range_i64(0, i64::MAX);
            assert!((0..i64::MAX).contains(&v));
        }
    }

    #[test]
    fn stream_variants_have_distinct_jumps() {
        let streams = [
            Stream::Genetics,
            Stream::Phenotype,
            Stream::Physics,
            Stream::Combat,
            Stream::Physiology,
            Stream::Ecology,
            Stream::Worldgen,
            Stream::Chronicler,
            Stream::Testing,
        ];
        let mut jumps: Vec<u32> = streams.iter().map(|s| s.jumps()).collect();
        jumps.sort_unstable();
        jumps.dedup();
        assert_eq!(jumps.len(), streams.len(), "stream discriminants collided");
    }
}
