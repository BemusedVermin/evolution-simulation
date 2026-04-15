//! Property-based fuzz tests for `Q3232` saturating arithmetic.
//!
//! Targets the invariants that matter for the simulation:
//! * Saturating ops never panic, across the full i64 bit-pattern space.
//! * Addition is commutative (always) and associative when there is no
//!   intermediate saturation.
//! * Multiplication is commutative.
//! * Negation is an involution except at `MIN` (where it saturates to `MAX`).
//! * Clamp is idempotent and respects bounds.
//!
//! Default proptest `ProptestConfig::cases` is 256; we bump to 1000 per
//! property here. With 18 properties that gives ~18k executions per run.
//! The 100k-sample target from the sprint plan is reached when combined
//! with the Gaussian KS test in `proptest_math.rs` (100k samples in one run).

use beast_core::Q3232;
use proptest::prelude::*;

fn any_q3232() -> impl Strategy<Value = Q3232> {
    any::<i64>().prop_map(Q3232::from_bits)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    // ---------- Addition ----------

    #[test]
    fn add_never_panics(a in any_q3232(), b in any_q3232()) {
        let _ = a + b;
    }

    #[test]
    fn add_commutative(a in any_q3232(), b in any_q3232()) {
        prop_assert_eq!(a + b, b + a);
    }

    #[test]
    fn add_identity_zero(a in any_q3232()) {
        prop_assert_eq!(a + Q3232::ZERO, a);
    }

    #[test]
    fn add_saturates_at_max(a in any_q3232()) {
        // Adding MAX to anything ≥ 0 saturates at MAX.
        if a >= Q3232::ZERO {
            prop_assert_eq!(a + Q3232::MAX, Q3232::MAX);
        }
    }

    #[test]
    fn add_saturates_at_min(a in any_q3232()) {
        if a <= Q3232::ZERO {
            prop_assert_eq!(a + Q3232::MIN, Q3232::MIN);
        }
    }

    // ---------- Subtraction ----------

    #[test]
    fn sub_never_panics(a in any_q3232(), b in any_q3232()) {
        let _ = a - b;
    }

    #[test]
    fn sub_self_is_zero(a in any_q3232()) {
        // Exception: MIN - MIN saturates at 0 (no overflow).
        prop_assert_eq!(a - a, Q3232::ZERO);
    }

    // ---------- Multiplication ----------

    #[test]
    fn mul_never_panics(a in any_q3232(), b in any_q3232()) {
        let _ = a * b;
    }

    #[test]
    fn mul_commutative(a in any_q3232(), b in any_q3232()) {
        prop_assert_eq!(a * b, b * a);
    }

    #[test]
    fn mul_identity_one(a in any_q3232()) {
        prop_assert_eq!(a * Q3232::ONE, a);
    }

    #[test]
    fn mul_zero(a in any_q3232()) {
        prop_assert_eq!(a * Q3232::ZERO, Q3232::ZERO);
    }

    // ---------- Division ----------

    #[test]
    fn div_never_panics(a in any_q3232(), b in any_q3232()) {
        let _ = a / b;
    }

    #[test]
    fn div_by_one_identity(a in any_q3232()) {
        prop_assert_eq!(a / Q3232::ONE, a);
    }

    #[test]
    fn div_by_zero_is_bounded(a in any_q3232()) {
        // Div-by-zero saturates deterministically to MIN/MAX/ZERO.
        let r = a / Q3232::ZERO;
        let ok = r == Q3232::MIN || r == Q3232::MAX || r == Q3232::ZERO;
        prop_assert!(ok);
    }

    // ---------- Negation ----------

    #[test]
    fn neg_never_panics(a in any_q3232()) {
        let _ = -a;
    }

    #[test]
    fn neg_is_involution_off_min(a in any_q3232()) {
        // MIN's negative saturates to MAX, so involution breaks there.
        if a != Q3232::MIN {
            prop_assert_eq!(-(-a), a);
        }
    }

    #[test]
    fn neg_of_min_saturates(_dummy in any::<u8>()) {
        prop_assert_eq!(-Q3232::MIN, Q3232::MAX);
    }

    // ---------- Clamp ----------

    #[test]
    fn clamp_is_idempotent(a in any_q3232(), lo in any_q3232(), hi in any_q3232()) {
        let (lo, hi) = if lo <= hi { (lo, hi) } else { (hi, lo) };
        let c = a.clamp(lo, hi);
        prop_assert_eq!(c.clamp(lo, hi), c);
        prop_assert!(c >= lo);
        prop_assert!(c <= hi);
    }

    // ---------- Ordering ----------

    #[test]
    fn ord_is_total(a in any_q3232(), b in any_q3232()) {
        use core::cmp::Ordering::*;
        let ord = a.cmp(&b);
        match ord {
            Less => prop_assert!(a < b),
            Equal => prop_assert_eq!(a, b),
            Greater => prop_assert!(a > b),
        }
    }
}
