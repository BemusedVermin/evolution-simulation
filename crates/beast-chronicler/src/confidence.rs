//! Confidence scoring for chronicler labels (S10.6).
//!
//! The confidence of a label assignment is a single Q32.32 number on
//! `[0, 1]` derived from how often the underlying pattern fires
//! (frequency) and how long the pattern has persisted (stability):
//!
//! ```text
//! freq        = clamp01( count / total_observations )
//! stability   = clamp01( (last_tick - first_tick) / current_tick )
//! confidence  = clamp01( 0.6 * freq + 0.4 * stability )
//! ```
//!
//! Per `documentation/INVARIANTS.md` §1, every step runs through Q32.32
//! saturating arithmetic so the result is bit-identical across platforms
//! — labels are sim-side state per `systems/09_world_history_lore.md` §5.

use beast_core::{TickCounter, Q3232};

use crate::pattern::PatternObservation;

/// Frequency-term weight in the confidence formula.
const FREQUENCY_WEIGHT_BITS: i64 = 0x_0000_0000_9999_999A; // 0.6 in Q32.32

/// Stability-term weight in the confidence formula.
const STABILITY_WEIGHT_BITS: i64 = 0x_0000_0000_6666_6666; // 0.4 in Q32.32

/// Frequency contribution: `clamp01(count / total_observations)`.
///
/// A `total_observations` of zero degenerates to zero rather than a
/// saturating divide; without observations there is nothing to weight.
#[inline]
fn frequency_term(count: u64, total_observations: u64) -> Q3232 {
    if total_observations == 0 {
        return Q3232::ZERO;
    }
    let count_q = Q3232::from_num(count);
    let total_q = Q3232::from_num(total_observations);
    (count_q / total_q).clamp(Q3232::ZERO, Q3232::ONE)
}

/// Stability contribution:
/// `clamp01((last_tick - first_tick) / current_tick)`.
///
/// `current_tick == 0` means the world is at tick zero, so no observation
/// can have elapsed — return zero rather than saturate. The numerator
/// is computed via `TickCounter::saturating_sub`, which already clamps at
/// zero, so out-of-order ingestion (last < first, briefly possible during
/// tests) cannot drive the term negative.
#[inline]
fn stability_term(observation: &PatternObservation, current_tick: TickCounter) -> Q3232 {
    if current_tick.raw() == 0 {
        return Q3232::ZERO;
    }
    let span = observation
        .last_tick
        .saturating_sub(observation.first_tick)
        .raw();
    let span_q = Q3232::from_num(span);
    let current_q = Q3232::from_num(current_tick.raw());
    (span_q / current_q).clamp(Q3232::ZERO, Q3232::ONE)
}

/// Compute the confidence score for an observation.
///
/// `total_observations` is the chronicler's running total ingestion count
/// (`Chronicler::total_ingested`), used to normalise this observation's
/// frequency. `current_tick` is the world's current tick, used to
/// normalise the observation's persistence span.
///
/// The returned value is clamped to `[0, 1]` and saturates rather than
/// panicking on any intermediate overflow.
#[inline]
pub fn compute_confidence(
    observation: &PatternObservation,
    total_observations: u64,
    current_tick: TickCounter,
) -> Q3232 {
    let freq = frequency_term(observation.count, total_observations);
    let stab = stability_term(observation, current_tick);
    let weighted = Q3232::from_bits(FREQUENCY_WEIGHT_BITS) * freq
        + Q3232::from_bits(STABILITY_WEIGHT_BITS) * stab;
    weighted.clamp(Q3232::ZERO, Q3232::ONE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern::PatternSignature;
    use std::collections::BTreeSet;

    fn obs(count: u64, first: u64, last: u64) -> PatternObservation {
        PatternObservation {
            signature: PatternSignature([0u8; 32]),
            count,
            first_tick: TickCounter::new(first),
            last_tick: TickCounter::new(last),
            primitives: BTreeSet::new(),
        }
    }

    #[test]
    fn weight_constants_sum_to_exactly_one() {
        // 0x9999999A + 0x66666666 = 0x100000000 = 2^32, which is the
        // bit pattern of Q3232::ONE.
        let freq = Q3232::from_bits(FREQUENCY_WEIGHT_BITS);
        let stab = Q3232::from_bits(STABILITY_WEIGHT_BITS);
        assert_eq!((freq + stab).to_bits(), Q3232::ONE.to_bits());
    }

    #[test]
    fn zero_total_observations_returns_zero() {
        let o = obs(0, 0, 0);
        assert_eq!(
            compute_confidence(&o, 0, TickCounter::new(100)),
            Q3232::ZERO
        );
    }

    #[test]
    fn zero_current_tick_drops_stability_to_zero() {
        // count == total → freq = 1.0, weighted by 0.6.
        let o = obs(5, 0, 0);
        let c = compute_confidence(&o, 5, TickCounter::ZERO);
        assert_eq!(c, Q3232::from_bits(FREQUENCY_WEIGHT_BITS));
    }

    #[test]
    fn full_frequency_full_stability_returns_one() {
        // count == total (freq = 1) and span == current_tick (stability = 1)
        // → 0.6 + 0.4 = 1.0 exactly.
        let o = obs(10, 5, 105);
        let c = compute_confidence(&o, 10, TickCounter::new(100));
        assert_eq!(c.to_bits(), Q3232::ONE.to_bits());
    }

    #[test]
    fn out_of_order_span_clamps_at_zero() {
        // last < first should not yield a negative stability — saturating
        // sub on TickCounter clamps the span at zero.
        let o = obs(1, 100, 50);
        let c = compute_confidence(&o, 1, TickCounter::new(100));
        // freq = 1.0, stability = 0 → 0.6.
        assert_eq!(c, Q3232::from_bits(FREQUENCY_WEIGHT_BITS));
    }

    #[test]
    fn confidence_is_deterministic_across_runs() {
        // Same inputs must produce bit-identical outputs every call —
        // INVARIANTS §1.
        let o = obs(7, 12, 84);
        let a = compute_confidence(&o, 100, TickCounter::new(120));
        let b = compute_confidence(&o, 100, TickCounter::new(120));
        assert_eq!(a.to_bits(), b.to_bits());
    }

    #[test]
    fn never_exceeds_unit_interval() {
        // Adversarial inputs: count > total, span > current — both clamp.
        let o = obs(u64::MAX, 0, u64::MAX);
        let c = compute_confidence(&o, 1, TickCounter::new(1));
        assert!(c <= Q3232::ONE);
        assert!(c >= Q3232::ZERO);
    }
}
