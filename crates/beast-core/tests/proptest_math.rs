//! Property tests for math utilities.

use beast_core::{
    clamp01, gaussian_q3232, inv_lerp_q3232, lerp_q3232, max_q3232, min_q3232, Prng, Q3232,
};
use proptest::prelude::*;

fn any_q3232() -> impl Strategy<Value = Q3232> {
    any::<i64>().prop_map(Q3232::from_bits)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    fn clamp01_result_in_unit(v in any_q3232()) {
        let c = clamp01(v);
        prop_assert!(c >= Q3232::ZERO);
        prop_assert!(c <= Q3232::ONE);
    }

    #[test]
    fn min_max_total_order(a in any_q3232(), b in any_q3232()) {
        let lo = min_q3232(a, b);
        let hi = max_q3232(a, b);
        prop_assert!(lo <= hi);
        prop_assert!(lo == a || lo == b);
        prop_assert!(hi == a || hi == b);
    }

    #[test]
    fn lerp_t_zero_returns_a(a in any_q3232(), b in any_q3232()) {
        prop_assert_eq!(lerp_q3232(a, b, Q3232::ZERO), a);
    }

    #[test]
    fn lerp_t_one_returns_b(a in any_q3232(), b in any_q3232()) {
        // Exception: extreme magnitude inputs can saturate inside `(b - a) * t`
        // even when t == 1. Skip pathological cases where saturation occurs.
        let diff = b.saturating_sub(a);
        if diff != Q3232::MIN && diff != Q3232::MAX {
            prop_assert_eq!(lerp_q3232(a, b, Q3232::ONE), b);
        }
    }

    #[test]
    fn inv_lerp_always_in_unit(a in any_q3232(), b in any_q3232(), v in any_q3232()) {
        let t = inv_lerp_q3232(a, b, v);
        prop_assert!(t >= Q3232::ZERO);
        prop_assert!(t <= Q3232::ONE);
    }
}

/// 100k-sample Gaussian sanity test: empirical mean/stddev of N(0,1) are
/// close to their theoretical values. This is *not* a rigorous normality
/// test; it's a guard against obvious regressions in the Box-Muller
/// implementation.
#[test]
fn gaussian_100k_samples_match_distribution() {
    let mut rng = Prng::from_seed(0xC0DE_CAFE_D00D_B055);
    let mean = Q3232::ZERO;
    let sd = Q3232::ONE;
    let n = 100_000_i32;
    let mut sum = 0.0_f64;
    let mut sum_sq = 0.0_f64;
    for _ in 0..n {
        let s: f64 = gaussian_q3232(&mut rng, mean, sd).to_num();
        sum += s;
        sum_sq += s * s;
    }
    let m = sum / n as f64;
    let variance = sum_sq / (n - 1) as f64 - m * m * n as f64 / (n - 1) as f64;
    let std_emp = variance.sqrt();
    // Over 100k N(0,1) draws, |mean| < 0.02 and |stddev - 1| < 0.02 with
    // overwhelming probability.
    assert!(m.abs() < 0.02, "empirical mean {m} too far from 0");
    assert!(
        (std_emp - 1.0).abs() < 0.02,
        "empirical stddev {std_emp} too far from 1"
    );
}
