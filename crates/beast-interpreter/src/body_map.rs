//! Body-site aggregation helpers for the phenotype interpreter.
//!
//! See `documentation/systems/11_phenotype_interpreter.md` §6.0B
//! ("Body-Site Aggregation") for the authoritative specification.
//!
//! ## Purpose
//!
//! A [`ResolvedPhenotype`] carries both a global channel map and a
//! per-[`BodyRegion`] amplitude map. Many channels accumulate per-site
//! values (e.g. `kinetic_force` per limb, `claw_sharpness` per appendage)
//! that must be collapsed into a single global figure for downstream
//! systems and primitive emission.
//!
//! This module ships the deterministic primitives that do that
//! collapsing:
//!
//! * [`AggregationStrategy`] — `Max` / `Mean` / `Sum`; selected by the
//!   channel manifest.
//! * [`aggregate_to_global`] — collapse a `BTreeMap<BodySite, Q3232>` to
//!   a single [`Q3232`].
//! * [`per_site_channel_values`] — extract per-site amplitudes for a
//!   given channel from a phenotype, merging duplicate sites with a
//!   caller-supplied strategy.
//! * [`aggregate_channel_globally`] — convenience combinator.
//!
//! ## Determinism
//!
//! All iteration is over `BTreeMap<BodySite, Q3232>`, which orders by
//! the `BodySite` enum's `Ord` derivation. All arithmetic uses
//! saturating [`Q3232`] operations; no floats. Identical inputs
//! therefore produce bit-identical outputs across runs and platforms,
//! as required by the determinism invariant (INVARIANTS §1).
//!
//! ## Out of scope
//!
//! Per-site [`PrimitiveEffect`] fan-out is **not** implemented here;
//! that requires extending `beast_primitives::PrimitiveEffect` with a
//! `body_site` field and is tracked in issue #67. This story (S4.5 /
//! #59) ships only the deterministic aggregation primitives the
//! interpreter will need when that wiring lands.
//!
//! [`BodyRegion`]: crate::phenotype::BodyRegion
//! [`PrimitiveEffect`]: beast_primitives::PrimitiveEffect

use std::collections::BTreeMap;

use beast_core::Q3232;

use crate::phenotype::{BodySite, ResolvedPhenotype};

/// How to collapse per-body-site channel values into a single global
/// value.
///
/// The strategy for each channel is defined in the channel manifest
/// (§6.0B). The interpreter selects a strategy based on the channel's
/// semantics — for example `kinetic_force` uses [`Self::Max`] (strongest
/// limb wins), `claw_sharpness` uses [`Self::Mean`], and additive
/// quantities like `surface_area` use [`Self::Sum`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AggregationStrategy {
    /// Maximum value across sites.
    Max,
    /// Arithmetic mean across sites.
    Mean,
    /// Sum across sites.
    Sum,
}

/// Collapse a per-body-site value map to a single global [`Q3232`].
///
/// Empty input returns [`Q3232::ZERO`]: it is the neutral element for
/// [`AggregationStrategy::Sum`], a safe floor for
/// [`AggregationStrategy::Max`] in the absence of any samples, and the
/// mean over zero samples is defined as zero here to keep downstream
/// arithmetic total.
///
/// All arithmetic is saturating: overflow clamps to
/// [`Q3232::MAX`] / [`Q3232::MIN`] rather than panicking. Iteration is
/// deterministic because the input is a `BTreeMap<BodySite, _>` and
/// `BodySite` derives `Ord`.
#[must_use]
pub fn aggregate_to_global(
    strategy: AggregationStrategy,
    per_site_values: &BTreeMap<BodySite, Q3232>,
) -> Q3232 {
    if per_site_values.is_empty() {
        return Q3232::ZERO;
    }

    match strategy {
        AggregationStrategy::Max => {
            let mut values = per_site_values.values().copied();
            // Safe to unwrap-via-expect — we returned early on empty above.
            let mut acc = values
                .next()
                .expect("non-empty values guaranteed by early-return above");
            for value in values {
                if value > acc {
                    acc = value;
                }
            }
            acc
        }
        AggregationStrategy::Sum => {
            let mut acc = Q3232::ZERO;
            for value in per_site_values.values().copied() {
                acc = acc.saturating_add(value);
            }
            acc
        }
        AggregationStrategy::Mean => {
            let mut sum = Q3232::ZERO;
            for value in per_site_values.values().copied() {
                sum = sum.saturating_add(value);
            }
            // `count` is non-zero because we returned early on empty.
            let count = Q3232::from_num(per_site_values.len() as i64);
            sum.saturating_div(count)
        }
    }
}

/// Collect per-body-site amplitudes of a single channel from a
/// phenotype.
///
/// Reads `phenotype.body_map[*].channel_amplitudes[channel_id]` and
/// assembles a `BTreeMap<BodySite, Q3232>`. Regions that do not carry
/// an entry for `channel_id` are omitted, so [`aggregate_to_global`]
/// only ever sees real site values.
///
/// If multiple regions share the same [`BodySite`] variant (future body
/// plans may, for example, split a limb into several regions all tagged
/// [`BodySite::LimbLeft`]), their amplitudes are merged using
/// `multi_region_strategy`:
///
/// * [`AggregationStrategy::Max`] — keep the larger of the two values.
/// * [`AggregationStrategy::Sum`] — add the values (saturating).
/// * [`AggregationStrategy::Mean`] — arithmetic mean of the pair. Note
///   that this pair-wise fold is not fully associative for three or
///   more regions, but it stays deterministic because
///   [`ResolvedPhenotype::body_map`] has a fixed iteration order.
///
/// If the channel is absent from every region's amplitudes, the
/// returned map is empty.
#[must_use]
pub fn per_site_channel_values(
    phenotype: &ResolvedPhenotype,
    channel_id: &str,
    multi_region_strategy: AggregationStrategy,
) -> BTreeMap<BodySite, Q3232> {
    let mut out: BTreeMap<BodySite, Q3232> = BTreeMap::new();
    for region in &phenotype.body_map {
        let Some(&amplitude) = region.channel_amplitudes.get(channel_id) else {
            continue;
        };
        out.entry(region.body_site)
            .and_modify(|existing| {
                *existing = merge_two(*existing, amplitude, multi_region_strategy);
            })
            .or_insert(amplitude);
    }
    out
}

/// Merge two per-site amplitudes under the given strategy.
///
/// Factored out to keep [`per_site_channel_values`] readable and to
/// avoid allocating a temporary collection for each body-site slot.
fn merge_two(a: Q3232, b: Q3232, strategy: AggregationStrategy) -> Q3232 {
    match strategy {
        AggregationStrategy::Max => {
            if a >= b {
                a
            } else {
                b
            }
        }
        AggregationStrategy::Sum => a.saturating_add(b),
        AggregationStrategy::Mean => {
            let sum = a.saturating_add(b);
            let two = Q3232::from_num(2_i64);
            sum.saturating_div(two)
        }
    }
}

/// Convenience: per-site collection followed by global aggregation.
///
/// Equivalent to
/// [`aggregate_to_global`]`(strategy,
/// &`[`per_site_channel_values`]`(phenotype, channel_id, strategy))`
/// — the same strategy is used both to merge regions that share a
/// [`BodySite`] variant and to collapse sites into the global scalar.
///
/// # Mean semantics caveat
///
/// For [`AggregationStrategy::Mean`], the two stages behave differently:
/// the inner [`per_site_channel_values`] pair-wise folds regions that
/// share a [`BodySite`] (non-associative for three or more co-located
/// regions — see its docs), while the outer [`aggregate_to_global`]
/// computes a true arithmetic mean across sites. The composition is
/// still deterministic (body-map iteration order is fixed), but the
/// final value for a body plan with three or more same-site regions is
/// not the sum-of-values divided by total-region-count. If that
/// behaviour is required, collect per-region values yourself and pass
/// a flat slice to a mean helper.
///
/// A `BodyRegion` tagged [`BodySite::Global`] is currently passed
/// through unchanged — the spec (§6.0B) only anticipates per-anatomical
/// sites in `body_map`, so callers should treat a `Global`-tagged
/// region as a body-plan construction bug rather than a meaningful
/// input.
#[must_use]
pub fn aggregate_channel_globally(
    phenotype: &ResolvedPhenotype,
    channel_id: &str,
    strategy: AggregationStrategy,
) -> Q3232 {
    let per_site = per_site_channel_values(phenotype, channel_id, strategy);
    aggregate_to_global(strategy, &per_site)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phenotype::{BodyRegion, LifeStage, ResolvedPhenotype};
    use proptest::prelude::*;
    use std::collections::BTreeMap;

    fn q(n: i64) -> Q3232 {
        Q3232::from_num(n)
    }

    fn site_map(pairs: &[(BodySite, i64)]) -> BTreeMap<BodySite, Q3232> {
        pairs.iter().map(|&(s, v)| (s, q(v))).collect()
    }

    fn region(id: u32, site: BodySite, channels: &[(&str, i64)]) -> BodyRegion {
        let channel_amplitudes: BTreeMap<String, Q3232> = channels
            .iter()
            .map(|&(k, v)| (k.to_string(), q(v)))
            .collect();
        BodyRegion {
            id,
            body_site: site,
            surface_vs_internal: Q3232::ZERO,
            channel_amplitudes,
        }
    }

    fn phenotype_with_regions(regions: Vec<BodyRegion>) -> ResolvedPhenotype {
        let mut p = ResolvedPhenotype::new(q(1), LifeStage::Adult);
        p.body_map = regions;
        p
    }

    // ---- aggregate_to_global ---------------------------------------------------

    #[test]
    fn aggregate_to_global_empty_returns_zero() {
        let empty: BTreeMap<BodySite, Q3232> = BTreeMap::new();
        assert_eq!(
            aggregate_to_global(AggregationStrategy::Max, &empty),
            Q3232::ZERO
        );
        assert_eq!(
            aggregate_to_global(AggregationStrategy::Mean, &empty),
            Q3232::ZERO
        );
        assert_eq!(
            aggregate_to_global(AggregationStrategy::Sum, &empty),
            Q3232::ZERO
        );
    }

    #[test]
    fn aggregate_to_global_single_entry_returns_that_value() {
        let m = site_map(&[(BodySite::Head, 7)]);
        assert_eq!(aggregate_to_global(AggregationStrategy::Max, &m), q(7));
        assert_eq!(aggregate_to_global(AggregationStrategy::Mean, &m), q(7));
        assert_eq!(aggregate_to_global(AggregationStrategy::Sum, &m), q(7));
    }

    #[test]
    fn aggregate_to_global_max_picks_largest() {
        let m = site_map(&[
            (BodySite::Head, 3),
            (BodySite::Jaw, 9),
            (BodySite::Core, 1),
            (BodySite::LimbLeft, 5),
        ]);
        assert_eq!(aggregate_to_global(AggregationStrategy::Max, &m), q(9));
    }

    #[test]
    fn aggregate_to_global_sum_adds_all() {
        let m = site_map(&[
            (BodySite::Head, 2),
            (BodySite::Jaw, 3),
            (BodySite::LimbLeft, 4),
            (BodySite::LimbRight, 5),
        ]);
        assert_eq!(aggregate_to_global(AggregationStrategy::Sum, &m), q(14));
    }

    #[test]
    fn aggregate_to_global_mean_is_sum_over_count() {
        let m = site_map(&[
            (BodySite::Head, 2),
            (BodySite::Jaw, 4),
            (BodySite::LimbLeft, 6),
            (BodySite::LimbRight, 8),
        ]);
        // (2+4+6+8)/4 = 5
        assert_eq!(aggregate_to_global(AggregationStrategy::Mean, &m), q(5));
    }

    // ---- per_site_channel_values ----------------------------------------------

    #[test]
    fn per_site_channel_values_four_region_phenotype() {
        let p = phenotype_with_regions(vec![
            region(0, BodySite::Head, &[("force", 2)]),
            region(1, BodySite::Jaw, &[("force", 3)]),
            region(2, BodySite::LimbLeft, &[("force", 4)]),
            region(3, BodySite::LimbRight, &[("force", 5)]),
        ]);
        let got = per_site_channel_values(&p, "force", AggregationStrategy::Max);
        assert_eq!(got.len(), 4);
        assert_eq!(got.get(&BodySite::Head), Some(&q(2)));
        assert_eq!(got.get(&BodySite::Jaw), Some(&q(3)));
        assert_eq!(got.get(&BodySite::LimbLeft), Some(&q(4)));
        assert_eq!(got.get(&BodySite::LimbRight), Some(&q(5)));
    }

    #[test]
    fn per_site_channel_values_missing_channel_returns_empty() {
        let p = phenotype_with_regions(vec![
            region(0, BodySite::Head, &[("force", 2)]),
            region(1, BodySite::Jaw, &[("sharpness", 3)]),
        ]);
        let got = per_site_channel_values(&p, "missing", AggregationStrategy::Max);
        assert!(got.is_empty());
    }

    #[test]
    fn per_site_channel_values_partial_coverage_skips_empty_sites() {
        let p = phenotype_with_regions(vec![
            region(0, BodySite::Head, &[("force", 2)]),
            region(1, BodySite::Jaw, &[("sharpness", 3)]),
            region(2, BodySite::LimbLeft, &[("force", 5)]),
        ]);
        let got = per_site_channel_values(&p, "force", AggregationStrategy::Max);
        assert_eq!(got.len(), 2);
        assert_eq!(got.get(&BodySite::Head), Some(&q(2)));
        assert_eq!(got.get(&BodySite::LimbLeft), Some(&q(5)));
        assert!(!got.contains_key(&BodySite::Jaw));
    }

    #[test]
    fn per_site_channel_values_duplicate_site_merges_max() {
        let p = phenotype_with_regions(vec![
            region(0, BodySite::LimbLeft, &[("force", 3)]),
            region(1, BodySite::LimbLeft, &[("force", 7)]),
        ]);
        let got = per_site_channel_values(&p, "force", AggregationStrategy::Max);
        assert_eq!(got.len(), 1);
        assert_eq!(got.get(&BodySite::LimbLeft), Some(&q(7)));
    }

    #[test]
    fn per_site_channel_values_duplicate_site_merges_sum() {
        let p = phenotype_with_regions(vec![
            region(0, BodySite::LimbLeft, &[("force", 3)]),
            region(1, BodySite::LimbLeft, &[("force", 7)]),
        ]);
        let got = per_site_channel_values(&p, "force", AggregationStrategy::Sum);
        assert_eq!(got.len(), 1);
        assert_eq!(got.get(&BodySite::LimbLeft), Some(&q(10)));
    }

    #[test]
    fn per_site_channel_values_duplicate_site_merges_mean() {
        let p = phenotype_with_regions(vec![
            region(0, BodySite::LimbLeft, &[("force", 4)]),
            region(1, BodySite::LimbLeft, &[("force", 8)]),
        ]);
        let got = per_site_channel_values(&p, "force", AggregationStrategy::Mean);
        assert_eq!(got.len(), 1);
        // (4 + 8) / 2 = 6
        assert_eq!(got.get(&BodySite::LimbLeft), Some(&q(6)));
    }

    // ---- aggregate_channel_globally -------------------------------------------

    #[test]
    fn aggregate_channel_globally_combines_collection_and_aggregation() {
        let p = phenotype_with_regions(vec![
            region(0, BodySite::LimbLeft, &[("force", 3)]),
            region(1, BodySite::LimbRight, &[("force", 9)]),
            region(2, BodySite::Jaw, &[("force", 5)]),
        ]);
        assert_eq!(
            aggregate_channel_globally(&p, "force", AggregationStrategy::Max),
            q(9)
        );
        assert_eq!(
            aggregate_channel_globally(&p, "force", AggregationStrategy::Sum),
            q(17)
        );
        // (3 + 9 + 5) / 3 = 17/3 in Q3232.
        let expected_mean = q(17).saturating_div(q(3));
        assert_eq!(
            aggregate_channel_globally(&p, "force", AggregationStrategy::Mean),
            expected_mean
        );
    }

    #[test]
    fn aggregate_channel_globally_missing_channel_yields_zero() {
        let p = phenotype_with_regions(vec![region(0, BodySite::Head, &[("other", 7)])]);
        assert_eq!(
            aggregate_channel_globally(&p, "absent", AggregationStrategy::Max),
            Q3232::ZERO
        );
    }

    // ---- proptest: purity and deterministic repeatability ---------------------

    fn strategy_from_tag(tag: u8) -> AggregationStrategy {
        match tag % 3 {
            0 => AggregationStrategy::Max,
            1 => AggregationStrategy::Sum,
            _ => AggregationStrategy::Mean,
        }
    }

    fn site_from_tag(tag: u8) -> BodySite {
        match tag % 8 {
            0 => BodySite::Global,
            1 => BodySite::Head,
            2 => BodySite::Jaw,
            3 => BodySite::Core,
            4 => BodySite::LimbLeft,
            5 => BodySite::LimbRight,
            6 => BodySite::Tail,
            _ => BodySite::Appendage,
        }
    }

    proptest! {
        #[test]
        fn aggregate_to_global_is_pure(
            strategy_tag in any::<u8>(),
            entries in prop::collection::vec((any::<u8>(), any::<i32>()), 0..12),
        ) {
            let strategy = strategy_from_tag(strategy_tag);
            let map: BTreeMap<BodySite, Q3232> = entries
                .into_iter()
                .map(|(tag, raw)| (site_from_tag(tag), Q3232::from_num(raw as i64)))
                .collect();
            let a = aggregate_to_global(strategy, &map);
            let b = aggregate_to_global(strategy, &map);
            prop_assert_eq!(a.to_bits(), b.to_bits());
        }

        #[test]
        fn per_site_channel_values_is_pure(
            strategy_tag in any::<u8>(),
            channels in prop::collection::vec((any::<u8>(), any::<i32>()), 0..10),
        ) {
            let strategy = strategy_from_tag(strategy_tag);
            let regions: Vec<BodyRegion> = channels
                .iter()
                .enumerate()
                .map(|(i, &(tag, raw))| {
                    let mut amplitudes: BTreeMap<String, Q3232> = BTreeMap::new();
                    amplitudes.insert("force".to_string(), Q3232::from_num(raw as i64));
                    BodyRegion {
                        id: i as u32,
                        body_site: site_from_tag(tag),
                        surface_vs_internal: Q3232::ZERO,
                        channel_amplitudes: amplitudes,
                    }
                })
                .collect();
            let p = phenotype_with_regions(regions);
            let a = per_site_channel_values(&p, "force", strategy);
            let b = per_site_channel_values(&p, "force", strategy);
            prop_assert_eq!(a, b);
        }
    }
}
