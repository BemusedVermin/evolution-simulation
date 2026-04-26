//! Q3232 fixed-point exponent / logarithm helpers used by the cost evaluator.
//!
//! These are kept private to `beast-primitives` for now. The deterministic
//! interpreter (Sprint 4) will almost certainly want the same helpers; when
//! that happens we should promote them to `beast-core::math`. Until then we
//! keep the surface small and well-tested in isolation.
//!
//! The algorithms:
//!
//! * [`q_ln`] — transforms `x > 0` into `2^k * m` with `m ∈ [1, 2)`, then uses
//!   the `artanh` series:
//!   `ln(m) = 2·(z + z³/3 + z⁵/5 + …)` where `z = (m - 1) / (m + 1)`.
//!   `|z| < 1/3` on `[1, 2)`, so 16 odd-terms are more than enough for Q32.32.
//! * [`q_exp`] — splits `x` into `k + f` with `k = trunc(x)` and `f ∈ (-1, 1)`,
//!   computes `e^f` via the Taylor series (20 terms), and then multiplies by
//!   `e^k` using repeated multiplication by a pre-computed constant `e`.
//! * [`q_pow`] — `base^exp = e^(exp · ln(base))` for positive `base`; integer
//!   exponents on non-positive bases are handled directly.
//!
//! Everything operates on saturating Q32.32 arithmetic, so out-of-range
//! results clamp instead of panicking. This is consistent with the crate's
//! determinism invariant: given the same inputs, the output is identical
//! across platforms.
//!
//! ## Cut-offs in [`q_exp`]
//!
//! Q32.32 ranges over roughly `[-2.15e9, 2.15e9)` with ULP `2^-32 ≈ 2.33e-10`.
//! That sets two fast-path cut-offs:
//!
//! * `x > 22` — `e^21 ≈ 1.32e9` still fits; `e^22 ≈ 3.58e9` already
//!   overflows. The cut-off short-circuits to `Q3232::MAX` so the Taylor
//!   path is never asked to produce an out-of-range result.
//! * `x < -23` — `e^-23 ≈ 1.02e-10 < ULP`, so the true value is below
//!   Q32.32 precision and snapping to `Q3232::ZERO` is exact.
//!
//! The lower cut-off in the previous revision of this module was `x < -22`;
//! the reviewer pointed out that `e^-22 ≈ 2.74e-10` is one ULP above ULP and
//! therefore *representable*. It turns out to be representable but **not
//! computable** by the current algorithm: `e^f · e^k` is realised as `e^f /
//! e · e / e · …` (22 saturating divisions for `k = -22`), and the
//! accumulated rounding eats the final bits, so the computed result snaps
//! to zero around `x ≈ -22` in practice. Tightening the cut-off to `-23`
//! costs nothing (those values would have computed to zero anyway) and
//! removes a one-ULP regime where the cut-off was visibly more aggressive
//! than the maths demanded. Actually preserving values near `x = -22`
//! would require a faster algorithm (precomputed `e^-1, e^-2, e^-4, …` and
//! binary decomposition of `k`) — a sensible upgrade when this helper is
//! promoted to `beast-core` but out of scope for Sprint 2.

use beast_core::Q3232;
use fixed::types::I32F32;

/// `ln(2)` to Q32.32 precision.
fn ln2() -> Q3232 {
    Q3232::from_num(core::f64::consts::LN_2)
}

/// Euler's number `e` to Q32.32 precision.
fn e() -> Q3232 {
    Q3232::from_num(core::f64::consts::E)
}

/// Natural logarithm on Q32.32.
///
/// Returns `None` for `x <= 0` (mathematically undefined).
///
/// # Precision
///
/// Implementation: range reduction to `m ∈ [1, 2)` followed by 16
/// terms of the artanh-form series for `ln(m)`. For the reduced
/// argument, `|z| < 1/3` and the truncation error of the series is
/// bounded by `|z|^33 / (33 * (1 - z²))` which is well under
/// `2^-50`. Saturating fixed-point division accumulates a per-step
/// rounding error of `≤ 2^-32`; over 16 terms this contributes
/// `≤ 16 * 2^-32 ≈ 4e-9`. The dominant contributor is the
/// fixed-point quantisation, not the truncation. Empirically
/// `ln_of_e_is_one` (in tests below) holds to `1e-5`, so callers
/// can treat results as accurate to roughly **4–5 decimal places**.
/// Adequate for cost evaluation and balance tuning; unsuitable for
/// numerical analyses that need ULP-level fidelity.
pub(crate) fn q_ln(x: Q3232) -> Option<Q3232> {
    if x <= Q3232::ZERO {
        return None;
    }
    // Decompose x = 2^k * m, m in [1, 2).
    let inner: I32F32 = x.into_inner();
    // `int_log2` panics on non-positive input; the guard above guarantees
    // `inner > 0`, so this call is infallible. Tying the two together here
    // so a future refactor can't silently drop the guard and reinstate a
    // panic path.
    let k = inner.int_log2();

    // Compute m_bits = x / 2^k. For I32F32 the raw bit pattern is an i64 of
    // the value scaled by 2^32, so dividing the value by 2^k is equivalent to
    // dividing the raw bits by 2^k (sign-extended arithmetic shift).
    let m_bits = if k >= 0 {
        inner.to_bits() >> k
    } else {
        // k is negative (x < 1) — multiply by 2^(-k). Arithmetic left shift
        // is safe because we just proved the original value fits, and the
        // shift amount is bounded by int_log2's range.
        inner.to_bits() << (-k)
    };
    let m = Q3232::from_inner(I32F32::from_bits(m_bits));

    // z = (m - 1) / (m + 1)
    let z = (m - Q3232::ONE) / (m + Q3232::ONE);
    let z2 = z * z;

    // Sum of the odd-terms: z + z³/3 + z⁵/5 + ...
    let mut sum = z;
    let mut term = z;
    let mut n: i32 = 3;
    for _ in 0..16 {
        term *= z2;
        sum += term / Q3232::from(n);
        n += 2;
    }
    let ln_m = Q3232::from(2_i32) * sum;

    // ln(x) = k * ln(2) + ln(m)
    Some(Q3232::from(k) * ln2() + ln_m)
}

/// Natural exponential on Q32.32 with saturating behaviour outside the
/// representable range.
///
/// Cut-offs (see module docs for derivation):
/// * `x > 22` → [`Q3232::MAX`] (would overflow)
/// * `x < -23` → [`Q3232::ZERO`] (below Q3232 ULP, zero is exact)
pub(crate) fn q_exp(x: Q3232) -> Q3232 {
    if x > Q3232::from(22_i32) {
        return Q3232::MAX;
    }
    if x < Q3232::from(-23_i32) {
        return Q3232::ZERO;
    }

    // Split into integer part k and fractional part f in (-1, 1). The cut-off
    // above bounds `|x| <= 23`, which fits in i8 let alone i32 — no silent
    // truncation from the Q3232 → i32 conversion.
    let k: i32 = x.to_num::<i32>();
    debug_assert!(
        (-23..=22).contains(&k),
        "q_exp cut-off guard violated: k={k}"
    );
    let f = x - Q3232::from(k);

    // e^f via Taylor: 1 + f + f²/2! + f³/3! + ... (converges fast, |f| < 1).
    let mut sum = Q3232::ONE;
    let mut term = Q3232::ONE;
    for n in 1_i32..=20 {
        term = term * f / Q3232::from(n);
        sum += term;
    }

    // Multiply by e^k via repeated mul/div. k is small (|k| <= 23 from the
    // cut-off), so the loop is bounded and cheap.
    let base = e();
    let mut result = sum;
    if k > 0 {
        for _ in 0..k {
            result *= base;
        }
    } else if k < 0 {
        for _ in 0..(-k) {
            result /= base;
        }
    }
    result
}

/// `base^exp` on Q32.32.
///
/// Returns `None` when the result is mathematically undefined (e.g. negative
/// base with a non-integer exponent, or `0^0` which is left to the caller to
/// define) *or* when the magnitude underflows the Q32.32 grid during the
/// negative-base integer-exponent loop below, which would otherwise silently
/// reappear as [`Q3232::MAX`] through saturating reciprocation. Zero-base
/// with positive exponent returns `Some(0)`.
pub(crate) fn q_pow(base: Q3232, exp: Q3232) -> Option<Q3232> {
    if base > Q3232::ZERO {
        return Some(q_exp(exp * q_ln(base)?));
    }
    if base == Q3232::ZERO {
        if exp > Q3232::ZERO {
            return Some(Q3232::ZERO);
        }
        if exp.is_zero() {
            // 0^0 is contested; cost evaluation treats this as 1 (no-op term).
            return Some(Q3232::ONE);
        }
        return None;
    }
    // base < 0: only integer exponents are real-valued. For cost evaluation
    // we don't expect negative parameter values so this branch is largely
    // defensive.
    let rounded: i64 = exp.to_num::<i64>();
    // `Q3232::from(i32)` round-trips integers in `[i32::MIN, i32::MAX]`
    // exactly. For exponents outside that range, `try_from` fails →
    // we reject as undefined rather than relying on the silent
    // narrowing that the previous `rounded as i32` did. (Even very
    // large integer exponents would produce values far outside the
    // Q32.32 representable range, so refusing them here is the right
    // policy regardless.)
    let Ok(rounded_i32) = i32::try_from(rounded) else {
        return None;
    };
    if Q3232::from(rounded_i32) != exp {
        return None;
    }
    let mut result = Q3232::ONE;
    let base_abs = -base;
    let iters = rounded.unsigned_abs();
    for _ in 0..iters {
        result *= base_abs;
    }
    if rounded % 2 != 0 {
        result = -result;
    }
    if rounded < 0 {
        // If the repeated-multiply loop saturated `result` to zero (e.g. a
        // base_abs well below 1 with a large `|rounded|`), the subsequent
        // 1/result would saturate to ±Q3232::MAX via saturating division and
        // silently masquerade as a real value. Surface the underflow as
        // `None` so callers — like the cost evaluator — can report it
        // instead of propagating a bogus finite number.
        if result == Q3232::ZERO {
            return None;
        }
        result = Q3232::ONE / result;
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Q3232, b: Q3232, tol: f64) -> bool {
        let diff: f64 = (a - b).saturating_abs().to_num::<f64>();
        diff <= tol
    }

    #[test]
    fn ln_of_one_is_zero() {
        let result = q_ln(Q3232::ONE).unwrap();
        assert!(close(result, Q3232::ZERO, 1e-6));
    }

    #[test]
    fn ln_of_e_is_one() {
        let result = q_ln(e()).unwrap();
        assert!(close(result, Q3232::ONE, 1e-5));
    }

    #[test]
    fn ln_rejects_non_positive() {
        assert!(q_ln(Q3232::ZERO).is_none());
        assert!(q_ln(-Q3232::ONE).is_none());
    }

    #[test]
    fn exp_of_zero_is_one() {
        let result = q_exp(Q3232::ZERO);
        assert!(close(result, Q3232::ONE, 1e-6));
    }

    #[test]
    fn exp_of_one_is_e() {
        let result = q_exp(Q3232::ONE);
        assert!(close(result, e(), 1e-5));
    }

    #[test]
    fn pow_integer_exponents() {
        let two = Q3232::from(2_i32);
        let four = Q3232::from(4_i32);
        // 2^2 = 4
        assert!(close(q_pow(two, two).unwrap(), four, 1e-5));
        // 2^0 = 1
        assert!(close(q_pow(two, Q3232::ZERO).unwrap(), Q3232::ONE, 1e-5));
    }

    #[test]
    fn pow_half_integer_exponents() {
        let four = Q3232::from(4_i32);
        let half = Q3232::from_num(0.5_f64);
        // 4^0.5 = 2
        assert!(close(q_pow(four, half).unwrap(), Q3232::from(2_i32), 1e-4));
    }

    #[test]
    fn pow_fractional_exponents() {
        let two = Q3232::from(2_i32);
        let exp = Q3232::from_num(1.5_f64);
        // 2^1.5 ≈ 2.828427
        let expected = Q3232::from_num(2.828_427_f64);
        assert!(close(q_pow(two, exp).unwrap(), expected, 1e-3));
    }

    #[test]
    fn pow_negative_exponent() {
        let two = Q3232::from(2_i32);
        let neg_two = Q3232::from(-2_i32);
        // 2^-2 = 0.25
        assert!(close(
            q_pow(two, neg_two).unwrap(),
            Q3232::from_num(0.25_f64),
            1e-5
        ));
    }

    #[test]
    fn pow_is_deterministic() {
        let base = Q3232::from_num(7.5_f64);
        let exp = Q3232::from_num(0.7_f64);
        let a = q_pow(base, exp).unwrap();
        for _ in 0..10 {
            assert_eq!(q_pow(base, exp).unwrap(), a);
        }
    }

    #[test]
    fn exp_preserves_values_well_above_ulp() {
        // e^-18 ≈ 1.52e-8 ≈ 65 ULP — comfortably inside what the iterative
        // algorithm can resolve after accumulated rounding. Locks in that
        // the lower cut-off hasn't crept into the regime the algorithm
        // actually supports.
        let result = q_exp(Q3232::from(-18_i32));
        assert!(
            result > Q3232::ZERO,
            "e^-18 should be representable and computable, got {result:?}"
        );
    }

    #[test]
    fn exp_snaps_well_below_ulp_to_zero() {
        // e^-24 ≈ 3.78e-11 is well below Q3232 ULP; returning zero is exact.
        assert_eq!(q_exp(Q3232::from(-24_i32)), Q3232::ZERO);
    }

    #[test]
    fn pow_negative_base_underflow_reported_not_faked() {
        // base_abs = 0.1, rounded = -100: the repeated-multiply loop
        // saturates result to ZERO, then 1/ZERO would saturate to MAX.
        // Instead we must surface None.
        let base = Q3232::from_num(-0.1_f64);
        let exp = Q3232::from(-100_i32);
        assert_eq!(q_pow(base, exp), None);
    }
}
