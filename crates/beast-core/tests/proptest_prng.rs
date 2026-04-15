//! Property tests for `Prng`: determinism, stream independence, and
//! basic statistical sanity.

use beast_core::{Prng, Q3232, Stream};
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    fn same_seed_gives_identical_sequence(seed: u64, draws in 1u16..256) {
        let mut a = Prng::from_seed(seed);
        let mut b = Prng::from_seed(seed);
        for _ in 0..draws {
            prop_assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn split_stream_is_deterministic(seed: u64, draws in 1u16..128) {
        let m1 = Prng::from_seed(seed);
        let m2 = Prng::from_seed(seed);
        let mut a = m1.split_stream(Stream::Combat);
        let mut b = m2.split_stream(Stream::Combat);
        for _ in 0..draws {
            prop_assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn next_q3232_unit_always_in_bounds(seed: u64, draws in 1u16..1024) {
        let mut rng = Prng::from_seed(seed);
        for _ in 0..draws {
            let v = rng.next_q3232_unit();
            prop_assert!(v >= Q3232::ZERO);
            prop_assert!(v < Q3232::ONE);
        }
    }

    #[test]
    fn gen_range_u64_in_bounds(seed: u64, lo in 0u64..1_000_000, span in 1u64..1_000_000) {
        let mut rng = Prng::from_seed(seed);
        let hi = lo + span;
        for _ in 0..64 {
            let v = rng.gen_range_u64(lo, hi);
            prop_assert!(v >= lo);
            prop_assert!(v < hi);
        }
    }

    #[test]
    fn gen_range_i64_in_bounds(seed: u64, lo in -1_000_000i64..0, hi in 0i64..1_000_000) {
        let mut rng = Prng::from_seed(seed);
        for _ in 0..64 {
            let v = rng.gen_range_i64(lo, hi);
            prop_assert!(v >= lo);
            prop_assert!(v < hi);
        }
    }
}

/// Deterministic 100k-sample fuzz confirming `next_u64` output doesn't
/// obviously clump. This is a sanity check, not a statistical proof —
/// Xoshiro256++ has been extensively audited upstream.
#[test]
fn prng_100k_draw_sanity() {
    let mut rng = Prng::from_seed(0xAAAA_BBBB_CCCC_DDDD);
    let mut buckets = [0u32; 16];
    for _ in 0..100_000 {
        let top4 = (rng.next_u64() >> 60) as usize;
        buckets[top4] += 1;
    }
    // Each bucket should be ~6250. Allow ±20%.
    for (i, &count) in buckets.iter().enumerate() {
        assert!(
            (5000..7500).contains(&count),
            "bucket {} count {} outside expected range",
            i,
            count
        );
    }
}

/// Same-seed reproducibility over 100k draws — this is the core determinism
/// guarantee the sim depends on.
#[test]
fn prng_100k_same_seed_identical() {
    let mut a = Prng::from_seed(0xDEAD_BEEF_FEED_FACE);
    let mut b = Prng::from_seed(0xDEAD_BEEF_FEED_FACE);
    for i in 0..100_000 {
        let va = a.next_u64();
        let vb = b.next_u64();
        assert_eq!(va, vb, "divergence at draw {i}");
    }
}
