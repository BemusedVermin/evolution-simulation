//! Beast Evolution Game — deterministic climate model.
//!
//! Sprint S8.5 (issue #144). Computes per-tick effective temperature
//! and precipitation for a biome cell, layered over the **base**
//! values stored in `BiomeCell` (S8.2) or returned from
//! `beast_world::generate_archipelago` (S8.1). The base values are
//! never mutated — climate is a pure function of `(tick, base,
//! latitude)`, so the simulation can rewind, fast-forward, or
//! sample arbitrary ticks without disturbing world state.
//!
//! # Crate scope
//!
//! Standalone L3 crate. Depends only on `beast-core` (for `Q3232`)
//! plus serde/thiserror — no link with `beast-world` or `beast-ecs`,
//! so this PR rebases independently of S8.1 (#159) and S8.2 (#147).
//! The spawner / tick loop bridges the (base, latitude) inputs to
//! this model from whichever component layer it loads them from.
//!
//! # Closed-cycle invariant
//!
//! Per `documentation/INVARIANTS.md` §1, world dynamics must be
//! tick-deterministic and round-trip exactly. The seasonal cycle is
//! a triangle wave with period `season_period_ticks` (default 1000):
//! at tick `0`, `period/2`, `period`, etc., the seasonal contribution
//! is exactly zero, so after one full cycle the visible value returns
//! to base — no accumulated rounding error from a sinusoidal
//! approximation.
//!
//! Triangle wave (rather than `sin`) was chosen because:
//!
//! * Q3232 has no `sin`/`cos` primitive; importing one would pull in
//!   the `cordic` or `libm` dependency surface.
//! * Triangle wave is exactly representable in fixed-point.
//! * Visual smoothness matters less for climate than for procedural
//!   audio — the gameplay impact is "spring is mid-temperature,
//!   summer is the peak."
//!
//! # Season ordering
//!
//! Phase windows for the default 1000-tick period:
//!
//! | Phase (ticks) | Season | Triangle wave | Notes |
//! |---|---|---|---|
//! | 0 …  249 | Spring | rising 0 → +1 | crosses zero at the start |
//! | 250 … 499 | Summer | falling +1 → 0 | peak warmth at the start |
//! | 500 … 749 | Autumn | falling 0 → −1 | crosses zero at the start |
//! | 750 … 999 | Winter | rising −1 → 0 | trough cold at the start |
//!
//! Demo criterion (epic #20): "season cycles every 1000 ticks" —
//! [`season_at_tick`] returns the four variants in this order.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use beast_core::Q3232;
use serde::{Deserialize, Serialize};

/// Four-season cycle. Variant order matches the canonical season
/// ordering for the default config; see [`season_at_tick`].
///
/// **Ordering:** intentionally does *not* derive `PartialOrd` /
/// `Ord`. Combined with `#[non_exhaustive]`, derived ordering
/// would silently change if a future variant were inserted between
/// the existing four — `BTreeMap<Season, _>` keys and any
/// sort-by-season would flip without a compile error. Use
/// [`Season::ordinal`] for stable canonical ordering instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Season {
    /// Phase 0..0.25 of the seasonal cycle. Triangle wave rises
    /// from 0 to +1.
    #[default]
    Spring,
    /// Phase 0.25..0.5. Triangle wave falls from +1 to 0.
    Summer,
    /// Phase 0.5..0.75. Triangle wave falls from 0 to −1.
    Autumn,
    /// Phase 0.75..1.0. Triangle wave rises from −1 to 0.
    Winter,
}

impl Season {
    /// Stable canonical ordinal: Spring=0, Summer=1, Autumn=2,
    /// Winter=3. Invariant under future `non_exhaustive` variant
    /// insertions — anything that needs to sort by season should
    /// key on this rather than the (intentionally absent) `Ord`
    /// derive.
    #[must_use]
    pub fn ordinal(self) -> u8 {
        match self {
            Season::Spring => 0,
            Season::Summer => 1,
            Season::Autumn => 2,
            Season::Winter => 3,
        }
    }
}

/// Climate model parameters.
///
/// All deltas are amplitudes — the *peak* deviation from base. The
/// seasonal triangle wave multiplies these by a value in `[-1, 1]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClimateConfig {
    /// Length of one full season cycle in simulation ticks. Default
    /// `1000`. Must be ≥ 4 (one tick per season minimum).
    pub season_period_ticks: u32,
    /// Peak temperature deviation from base, in Kelvin (Q3232).
    /// Default `15` — a moderate seasonal swing.
    pub seasonal_temperature_amplitude: Q3232,
    /// Peak precipitation deviation from base, in mm/year (Q3232).
    /// Default `200`.
    pub seasonal_precipitation_amplitude: Q3232,
    /// Per-unit-latitude temperature lapse: cells at `|lat| = 1`
    /// (the poles) are colder than the equator by this amount, in
    /// Kelvin (Q3232). Default `40` — gives a ~40K equator-to-pole
    /// temperature gradient.
    pub latitude_temperature_lapse: Q3232,
}

impl ClimateConfig {
    /// Default config matching the demo criterion in epic #20:
    /// season cycles every 1000 ticks; ±15K seasonal swing; 40K
    /// equator-to-pole lapse. Equivalent to
    /// `<ClimateConfig as Default>::default()`.
    #[must_use]
    pub fn default_mvp() -> Self {
        Self {
            season_period_ticks: 1000,
            seasonal_temperature_amplitude: Q3232::from_num(15_i32),
            seasonal_precipitation_amplitude: Q3232::from_num(200_i32),
            latitude_temperature_lapse: Q3232::from_num(40_i32),
        }
    }
}

impl Default for ClimateConfig {
    fn default() -> Self {
        Self::default_mvp()
    }
}

/// Effective climate at a tick, after applying seasonal and
/// latitudinal modifiers to the base values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClimateReading {
    /// Effective temperature at this tick, in Kelvin (Q3232).
    pub temperature_kelvin: Q3232,
    /// Effective precipitation at this tick, in mm/year (Q3232).
    pub precipitation_mm_per_year: Q3232,
    /// Which seasonal phase this tick falls in.
    pub season: Season,
}

/// Errors returned by the climate model.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ClimateError {
    /// `season_period_ticks` was below the four-season minimum.
    #[error("season_period_ticks must be >= 4; got {0}")]
    SeasonPeriodTooSmall(u32),
    /// `abs_latitude` was outside `[0, 1]` — the contract for the
    /// equator-to-pole normalised coordinate. Out-of-range values
    /// would produce physically nonsensical (saturated) climate
    /// readings, so the model rejects them at the boundary.
    #[error("abs_latitude must be in [0, 1]; got {0:?}")]
    AbsLatitudeOutOfRange(Q3232),
}

/// Compute the season at a given tick under the supplied config.
///
/// `tick % period` is bucketed into four equal quarters. With the
/// default `period = 1000`, ticks 0..249 are Spring, 250..499 are
/// Summer, 500..749 are Autumn, 750..999 are Winter, and tick 1000
/// is Spring again — closing the cycle.
///
/// # Panics
///
/// Panics in both debug and release if `season_period < 4`. This
/// matches the `effective_climate` validation contract — both
/// entry points reject the same range of bad inputs. Use
/// [`try_season_at_tick`] for fallible callers.
#[must_use]
pub fn season_at_tick(tick: u64, season_period: u32) -> Season {
    assert!(
        season_period >= 4,
        "season_period must be >= 4; got {season_period}",
    );
    season_at_tick_unchecked(tick, season_period)
}

/// Fallible variant of [`season_at_tick`] — returns
/// [`ClimateError::SeasonPeriodTooSmall`] instead of panicking.
///
/// # Errors
///
/// * [`ClimateError::SeasonPeriodTooSmall`] when `season_period < 4`.
pub fn try_season_at_tick(tick: u64, season_period: u32) -> Result<Season, ClimateError> {
    if season_period < 4 {
        return Err(ClimateError::SeasonPeriodTooSmall(season_period));
    }
    Ok(season_at_tick_unchecked(tick, season_period))
}

#[inline]
fn season_at_tick_unchecked(tick: u64, season_period: u32) -> Season {
    let period = u64::from(season_period);
    let phase = tick % period;
    // Quarter boundaries: floor((phase * 4) / period). `phase < period
    // <= u32::MAX`, so `phase * 4 < 4 * u32::MAX` which fits easily
    // in u64 — no overflow possible.
    let quarter = (phase * 4) / period;
    match quarter {
        0 => Season::Spring,
        1 => Season::Summer,
        2 => Season::Autumn,
        _ => Season::Winter,
    }
}

/// First tick of the next season after `tick`.
///
/// Useful for UI / AI: "in N ticks the season will change." Returns
/// the absolute tick (i.e., always `> tick`) at which the next
/// quarter boundary fires.
///
/// # Panics
///
/// Panics if `season_period < 4`. Same contract as
/// [`season_at_tick`].
#[must_use]
pub fn next_season_change_tick(tick: u64, season_period: u32) -> u64 {
    assert!(
        season_period >= 4,
        "season_period must be >= 4; got {season_period}",
    );
    let period = u64::from(season_period);
    let phase = tick % period;
    // The current quarter boundary index is floor((phase * 4) / period).
    // The next boundary is at quarter = current + 1.
    let current_quarter = (phase * 4) / period;
    let next_quarter = current_quarter + 1;
    // Tick of the next quarter boundary, in absolute terms.
    let cycle_start = tick - phase;
    // ceil division to land on the boundary.
    let next_phase = (next_quarter * period).div_ceil(4);
    cycle_start + next_phase
}

/// Number of ticks until the next season change, starting from
/// `tick`. Always `>= 1`.
///
/// # Panics
///
/// Panics if `season_period < 4`.
#[must_use]
pub fn ticks_until_next_season(tick: u64, season_period: u32) -> u64 {
    next_season_change_tick(tick, season_period) - tick
}

/// Triangle wave with period `period` ticks, returning a Q3232 in
/// `[-1, 1]`. At tick 0 it is exactly 0; at tick `period/4` it is
/// exactly +1; at tick `period/2` it is 0 again; at tick
/// `3*period/4` it is exactly -1; at tick `period` it is 0,
/// completing the cycle.
///
/// Pure integer arithmetic — no `sin`, no float, no rounding drift
/// across cycles. This is the function that satisfies the
/// closed-cycle invariant.
fn seasonal_triangle(tick: u64, period: u32) -> Q3232 {
    assert!(period >= 4, "period must be >= 4; got {period}");
    let period = u64::from(period);
    let phase = tick % period;
    // half_period = period / 2; quarter = period / 4.
    let half = period / 2;
    let quarter = period / 4;

    // Map phase to a "sawtooth" in [-quarter, +quarter] with the
    // apex at `quarter`, then divide by `quarter` to get [-1, +1].
    //
    // Phase ranges:
    //   [0, quarter):           value rises 0 → +quarter
    //   [quarter, quarter+half):  descending arm: +quarter → -quarter
    //   [quarter+half, period): ascending arm: -quarter → 0
    //
    // For periods not divisible by 4, `period/4` truncates and the
    // ascending and descending arms have unequal lengths — but the
    // closed-cycle property at ticks 0 and `period` still holds
    // exactly (both produce signed = 0). The peak is always at
    // exactly `phase = quarter`, regardless of divisibility.
    let signed: i64 = if phase < quarter {
        // 0 .. quarter
        phase as i64
    } else if phase < quarter + half {
        // quarter .. 3*quarter — descending arm
        let from_apex = (phase - quarter) as i64;
        (quarter as i64) - from_apex
    } else {
        // 3*quarter .. period — ascending arm
        let from_trough = (phase - quarter - half) as i64;
        -(quarter as i64) + from_trough
    };

    // Convert to Q3232 ratio: signed / quarter.
    if quarter == 0 {
        return Q3232::ZERO;
    }
    // SAFETY (overflow): `signed` is bounded by `quarter`
    // (≤ u32::MAX/4 ≈ 1.07e9). `signed << 32` ≤ ~1.07e9 * 2^32
    // ≈ 4.6e18, comfortably under i64::MAX (≈ 9.2e18). No overflow
    // possible for any `period` that fits in u32.
    let ratio_bits = (signed << 32) / (quarter as i64);
    Q3232::from_bits(ratio_bits)
}

/// Compute the effective climate at a tick.
///
/// `abs_latitude` is `|lat|` in `[0, 1]`, where 0 is the equator
/// and 1 is a pole — matches the latitude convention produced by
/// `beast_world::generate_archipelago`.
///
/// Returns the season for downstream UI/labels.
///
/// # Errors
///
/// * [`ClimateError::SeasonPeriodTooSmall`] when
///   `config.season_period_ticks < 4`.
/// * [`ClimateError::AbsLatitudeOutOfRange`] when `abs_latitude`
///   is outside `[0, 1]`. Out-of-range values would produce
///   physically nonsensical (saturated) temperatures, so the model
///   rejects them at the boundary instead of returning a garbage
///   reading.
pub fn effective_climate(
    config: &ClimateConfig,
    base_temp_kelvin: Q3232,
    base_precipitation: Q3232,
    abs_latitude: Q3232,
    tick: u64,
) -> Result<ClimateReading, ClimateError> {
    if config.season_period_ticks < 4 {
        return Err(ClimateError::SeasonPeriodTooSmall(
            config.season_period_ticks,
        ));
    }
    if abs_latitude < Q3232::ZERO || abs_latitude > Q3232::ONE {
        return Err(ClimateError::AbsLatitudeOutOfRange(abs_latitude));
    }

    let season = season_at_tick_unchecked(tick, config.season_period_ticks);
    let triangle = seasonal_triangle(tick, config.season_period_ticks);

    // Temperature modifier:
    //   seasonal: ± amplitude * triangle
    //   latitudinal: - lapse * abs_latitude  (poles colder)
    let seasonal_temp = config
        .seasonal_temperature_amplitude
        .saturating_mul(triangle);
    let lapse_temp = config
        .latitude_temperature_lapse
        .saturating_mul(abs_latitude);
    let temperature_kelvin = base_temp_kelvin
        .saturating_add(seasonal_temp)
        .saturating_sub(lapse_temp);

    // Precipitation modifier: ± amplitude * triangle (no lapse;
    // moisture is set per-cell at world gen).
    let seasonal_precip = config
        .seasonal_precipitation_amplitude
        .saturating_mul(triangle);
    let precipitation_mm_per_year = base_precipitation.saturating_add(seasonal_precip);

    Ok(ClimateReading {
        temperature_kelvin,
        precipitation_mm_per_year,
        season,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn season_cycles_every_period() {
        // Demo criterion (epic #20): the season cycles every 1000
        // ticks. tick 0 and tick 1000 must produce the same season.
        let p = 1000;
        assert_eq!(season_at_tick(0, p), Season::Spring);
        assert_eq!(season_at_tick(1000, p), Season::Spring);
        assert_eq!(season_at_tick(2000, p), Season::Spring);
    }

    #[test]
    fn season_order_is_canonical() {
        let p = 1000;
        // Quarter boundaries.
        assert_eq!(season_at_tick(0, p), Season::Spring);
        assert_eq!(season_at_tick(249, p), Season::Spring);
        assert_eq!(season_at_tick(250, p), Season::Summer);
        assert_eq!(season_at_tick(499, p), Season::Summer);
        assert_eq!(season_at_tick(500, p), Season::Autumn);
        assert_eq!(season_at_tick(749, p), Season::Autumn);
        assert_eq!(season_at_tick(750, p), Season::Winter);
        assert_eq!(season_at_tick(999, p), Season::Winter);
    }

    #[test]
    fn season_default_is_spring() {
        // Avoids "tick 0 looks like a hard winter" when the
        // simulation hasn't been told about climate yet.
        assert_eq!(Season::default(), Season::Spring);
    }

    #[test]
    fn seasonal_triangle_starts_and_ends_at_zero() {
        // Closed-cycle invariant: tick 0 and tick `period` both
        // produce a triangle value of 0, so applying it to any base
        // value returns the original.
        let p = 1000;
        assert_eq!(seasonal_triangle(0, p), Q3232::ZERO);
        assert_eq!(seasonal_triangle(1000, p), Q3232::ZERO);
        assert_eq!(seasonal_triangle(2000, p), Q3232::ZERO);
    }

    #[test]
    fn seasonal_triangle_peak_is_at_quarter() {
        let p = 1000;
        let peak = seasonal_triangle(250, p);
        assert_eq!(peak, Q3232::ONE);
    }

    #[test]
    fn seasonal_triangle_trough_is_at_three_quarters() {
        let p = 1000;
        let trough = seasonal_triangle(750, p);
        assert_eq!(trough, -Q3232::ONE);
    }

    #[test]
    fn seasonal_triangle_zero_crossings() {
        let p = 1000;
        assert_eq!(seasonal_triangle(0, p), Q3232::ZERO);
        assert_eq!(seasonal_triangle(500, p), Q3232::ZERO);
    }

    #[test]
    fn seasonal_triangle_is_pure_function() {
        let p = 1000;
        for tick in [0_u64, 100, 250, 500, 750, 999] {
            let a = seasonal_triangle(tick, p);
            let b = seasonal_triangle(tick, p);
            assert_eq!(a, b);
        }
    }

    #[test]
    fn seasonal_triangle_stays_in_range() {
        let p = 1000;
        for tick in 0..2000 {
            let v = seasonal_triangle(tick, p);
            assert!(
                v >= -Q3232::ONE && v <= Q3232::ONE,
                "out of range at tick {tick}: {v:?}"
            );
        }
    }

    #[test]
    fn effective_climate_returns_base_at_tick_zero_at_equator() {
        // Tick 0 is the spring equinox (triangle = 0); equator is
        // |lat| = 0 (no lapse). So effective values equal base.
        let cfg = ClimateConfig::default_mvp();
        let r = effective_climate(
            &cfg,
            Q3232::from_num(288_i32),
            Q3232::from_num(1000_i32),
            Q3232::ZERO,
            0,
        )
        .unwrap();
        assert_eq!(r.temperature_kelvin, Q3232::from_num(288_i32));
        assert_eq!(r.precipitation_mm_per_year, Q3232::from_num(1000_i32));
        assert_eq!(r.season, Season::Spring);
    }

    #[test]
    fn effective_climate_returns_base_after_one_full_cycle() {
        // Closed-cycle invariant: tick 0 and tick `period` produce
        // identical readings.
        let cfg = ClimateConfig::default_mvp();
        let base_temp = Q3232::from_num(283_i32);
        let base_precip = Q3232::from_num(750_i32);
        let lat = Q3232::from_num(0.3_f64);
        let r0 = effective_climate(&cfg, base_temp, base_precip, lat, 0).unwrap();
        let r1000 = effective_climate(&cfg, base_temp, base_precip, lat, 1000).unwrap();
        assert_eq!(r0.temperature_kelvin, r1000.temperature_kelvin);
        assert_eq!(
            r0.precipitation_mm_per_year,
            r1000.precipitation_mm_per_year
        );
        assert_eq!(r0.season, r1000.season);
    }

    #[test]
    fn effective_climate_summer_peak_is_warmer_than_base() {
        let cfg = ClimateConfig::default_mvp();
        let base_temp = Q3232::from_num(288_i32);
        // Tick 250 = summer solstice (triangle = +1) → +amplitude K.
        let r = effective_climate(&cfg, base_temp, Q3232::ZERO, Q3232::ZERO, 250).unwrap();
        assert_eq!(r.season, Season::Summer);
        assert_eq!(
            r.temperature_kelvin,
            base_temp.saturating_add(cfg.seasonal_temperature_amplitude),
        );
    }

    #[test]
    fn effective_climate_winter_trough_is_colder_than_base() {
        let cfg = ClimateConfig::default_mvp();
        let base_temp = Q3232::from_num(288_i32);
        // Tick 750 = winter solstice (triangle = -1) → -amplitude K.
        let r = effective_climate(&cfg, base_temp, Q3232::ZERO, Q3232::ZERO, 750).unwrap();
        assert_eq!(r.season, Season::Winter);
        assert_eq!(
            r.temperature_kelvin,
            base_temp.saturating_sub(cfg.seasonal_temperature_amplitude),
        );
    }

    #[test]
    fn effective_climate_pole_is_colder_than_equator() {
        // At the same tick, |lat| = 1.0 should be `lapse` colder
        // than |lat| = 0.0.
        let cfg = ClimateConfig::default_mvp();
        let base_temp = Q3232::from_num(288_i32);
        let r_eq = effective_climate(&cfg, base_temp, Q3232::ZERO, Q3232::ZERO, 0).unwrap();
        let r_pole = effective_climate(&cfg, base_temp, Q3232::ZERO, Q3232::ONE, 0).unwrap();
        assert_eq!(
            r_eq.temperature_kelvin
                .saturating_sub(r_pole.temperature_kelvin),
            cfg.latitude_temperature_lapse,
        );
    }

    #[test]
    fn effective_climate_is_deterministic_across_calls() {
        let cfg = ClimateConfig::default_mvp();
        let a = effective_climate(
            &cfg,
            Q3232::from_num(290_i32),
            Q3232::from_num(800_i32),
            Q3232::from_num(0.5_f64),
            123,
        )
        .unwrap();
        let b = effective_climate(
            &cfg,
            Q3232::from_num(290_i32),
            Q3232::from_num(800_i32),
            Q3232::from_num(0.5_f64),
            123,
        )
        .unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn rejects_season_period_below_four() {
        let mut cfg = ClimateConfig::default_mvp();
        cfg.season_period_ticks = 3;
        let err = effective_climate(&cfg, Q3232::from_num(288_i32), Q3232::ZERO, Q3232::ZERO, 0)
            .unwrap_err();
        assert!(matches!(err, ClimateError::SeasonPeriodTooSmall(3)));
    }

    #[test]
    fn rejects_abs_latitude_above_one() {
        let cfg = ClimateConfig::default_mvp();
        let err = effective_climate(
            &cfg,
            Q3232::from_num(288_i32),
            Q3232::ZERO,
            Q3232::from_num(1.5_f64),
            0,
        )
        .unwrap_err();
        assert!(matches!(err, ClimateError::AbsLatitudeOutOfRange(_)));
    }

    #[test]
    fn rejects_abs_latitude_below_zero() {
        let cfg = ClimateConfig::default_mvp();
        let err = effective_climate(
            &cfg,
            Q3232::from_num(288_i32),
            Q3232::ZERO,
            Q3232::from_num(-0.1_f64),
            0,
        )
        .unwrap_err();
        assert!(matches!(err, ClimateError::AbsLatitudeOutOfRange(_)));
    }

    #[test]
    fn season_at_tick_panics_on_period_below_four() {
        // Both season_at_tick and effective_climate must reject the
        // same range — no split validation contract.
        let result = std::panic::catch_unwind(|| season_at_tick(0, 3));
        assert!(result.is_err(), "expected panic, got {result:?}");
    }

    #[test]
    fn try_season_at_tick_returns_error_on_period_below_four() {
        let err = try_season_at_tick(0, 3).unwrap_err();
        assert!(matches!(err, ClimateError::SeasonPeriodTooSmall(3)));
    }

    #[test]
    fn season_ordinal_is_canonical() {
        assert_eq!(Season::Spring.ordinal(), 0);
        assert_eq!(Season::Summer.ordinal(), 1);
        assert_eq!(Season::Autumn.ordinal(), 2);
        assert_eq!(Season::Winter.ordinal(), 3);
    }

    #[test]
    fn climate_config_default_matches_default_mvp() {
        assert_eq!(ClimateConfig::default(), ClimateConfig::default_mvp());
    }

    #[test]
    fn ticks_until_next_season_is_strictly_positive() {
        // Always at least 1, regardless of where in the cycle we are.
        let p = 1000;
        for tick in [0_u64, 1, 100, 249, 250, 500, 749, 999, 1000, 1234] {
            let n = ticks_until_next_season(tick, p);
            assert!(n >= 1, "tick {tick}: expected >= 1, got {n}");
        }
    }

    #[test]
    fn next_season_change_tick_lands_on_quarter_boundary() {
        let p = 1000;
        // From tick 100 (Spring), next change is tick 250 (Summer
        // start).
        assert_eq!(next_season_change_tick(100, p), 250);
        // From tick 249 (still Spring), next change is tick 250.
        assert_eq!(next_season_change_tick(249, p), 250);
        // From tick 250 (Summer just started), next change is 500.
        assert_eq!(next_season_change_tick(250, p), 500);
        // From tick 999 (Winter), next change is 1000 (next Spring).
        assert_eq!(next_season_change_tick(999, p), 1000);
    }

    #[test]
    fn closed_cycle_invariant_holds_across_many_periods() {
        // Same reading at tick 0, period, 5*period, 100*period.
        // Pure integer arithmetic should make drift impossible; this
        // is a regression guard if the formula changes.
        let cfg = ClimateConfig::default_mvp();
        let base_temp = Q3232::from_num(283_i32);
        let base_precip = Q3232::from_num(750_i32);
        let lat = Q3232::from_num(0.3_f64);
        let r0 = effective_climate(&cfg, base_temp, base_precip, lat, 0).unwrap();
        for k in [1_u64, 5, 100] {
            let tick = k * u64::from(cfg.season_period_ticks);
            let r = effective_climate(&cfg, base_temp, base_precip, lat, tick).unwrap();
            assert_eq!(r, r0, "closed-cycle drift at tick {tick} (k = {k} periods)",);
        }
    }
}
