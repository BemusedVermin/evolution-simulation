//! Expression conditions — gating predicates loaded from a channel manifest.
//!
//! Each condition is a discriminated-union variant in the JSON Schema. All
//! conditions on a channel must hold simultaneously for the channel to
//! express; this module provides a straightforward evaluator that takes an
//! [`ExpressionContext`] describing the current environment/creature and
//! returns `true` iff every condition is satisfied.
//!
//! Evaluation is purely a comparison over [`beast_core::Q3232`] and string
//! equality, so it inherits determinism from those primitives.

use beast_core::Q3232;

/// A single expression-condition predicate parsed from a channel manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpressionCondition {
    /// Named biome / environmental flag that must be active.
    BiomeFlag {
        /// Flag name (e.g. `"aquatic"`, `"nocturnal"`).
        flag: String,
    },
    /// Body-mass gate (creature's mass must be in `[min_kg, max_kg]`).
    ScaleBand {
        /// Lower bound (kg, inclusive).
        min_kg: Q3232,
        /// Upper bound (kg, inclusive).
        max_kg: Q3232,
    },
    /// Current season must match.
    Season {
        /// Season name (e.g. `"spring"`, `"winter"`).
        season: String,
    },
    /// Creature's life stage must match.
    DevelopmentalStage {
        /// Stage name (e.g. `"juvenile"`, `"breeding_adult"`).
        stage: String,
    },
    /// Local population density must be in `[min_per_km2, max_per_km2]`.
    SocialDensity {
        /// Lower bound (individuals / km², inclusive).
        min_per_km2: Q3232,
        /// Upper bound (individuals / km², inclusive).
        max_per_km2: Q3232,
    },
}

/// Snapshot of environment/creature state used by the expression evaluator.
///
/// Callers pass any subset they know about. Missing fields cause the
/// corresponding condition kind to evaluate to `false`, which is the
/// conservative default — the channel will not express if we cannot confirm
/// the condition holds.
#[derive(Debug, Clone, Default)]
pub struct ExpressionContext<'a> {
    /// Biome flags currently active at the creature's location.
    pub biome_flags: &'a [&'a str],
    /// Creature body mass in kg.
    pub body_mass_kg: Option<Q3232>,
    /// Current season.
    pub season: Option<&'a str>,
    /// Creature life stage.
    pub developmental_stage: Option<&'a str>,
    /// Local population density, individuals per km².
    pub population_density_per_km2: Option<Q3232>,
}

/// Evaluate a slice of conditions, returning `true` iff every condition holds.
///
/// An empty slice always evaluates to `true` (schema semantics: "no
/// conditions" means "always express").
///
/// ```
/// use beast_channels::{evaluate_expression_conditions, ExpressionCondition, ExpressionContext};
/// use beast_core::Q3232;
///
/// let conditions = [ExpressionCondition::ScaleBand {
///     min_kg: Q3232::from_num(0.01_f64),
///     max_kg: Q3232::from_num(1000_i32),
/// }];
///
/// let ctx = ExpressionContext {
///     body_mass_kg: Some(Q3232::from_num(100_i32)),
///     ..ExpressionContext::default()
/// };
/// assert!(evaluate_expression_conditions(&conditions, &ctx));
///
/// let ctx_out = ExpressionContext {
///     body_mass_kg: Some(Q3232::from_num(5000_i32)),
///     ..ExpressionContext::default()
/// };
/// assert!(!evaluate_expression_conditions(&conditions, &ctx_out));
/// ```
pub fn evaluate_expression_conditions(
    conditions: &[ExpressionCondition],
    ctx: &ExpressionContext<'_>,
) -> bool {
    conditions.iter().all(|c| evaluate_one(c, ctx))
}

fn evaluate_one(cond: &ExpressionCondition, ctx: &ExpressionContext<'_>) -> bool {
    match cond {
        ExpressionCondition::BiomeFlag { flag } => ctx.biome_flags.contains(&flag.as_str()),
        ExpressionCondition::ScaleBand { min_kg, max_kg } => match ctx.body_mass_kg {
            Some(mass) => mass >= *min_kg && mass <= *max_kg,
            None => false,
        },
        ExpressionCondition::Season { season } => ctx.season == Some(season.as_str()),
        ExpressionCondition::DevelopmentalStage { stage } => {
            ctx.developmental_stage == Some(stage.as_str())
        }
        ExpressionCondition::SocialDensity {
            min_per_km2,
            max_per_km2,
        } => match ctx.population_density_per_km2 {
            Some(d) => d >= *min_per_km2 && d <= *max_per_km2,
            None => false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(v: f64) -> Q3232 {
        Q3232::from_num(v)
    }

    #[test]
    fn empty_conditions_always_pass() {
        assert!(evaluate_expression_conditions(
            &[],
            &ExpressionContext::default()
        ));
    }

    #[test]
    fn biome_flag_matches() {
        let cond = [ExpressionCondition::BiomeFlag {
            flag: "aquatic".into(),
        }];
        let flags = ["aquatic", "cold"];
        let ctx = ExpressionContext {
            biome_flags: &flags,
            ..ExpressionContext::default()
        };
        assert!(evaluate_expression_conditions(&cond, &ctx));
    }

    #[test]
    fn biome_flag_misses_when_absent() {
        let cond = [ExpressionCondition::BiomeFlag {
            flag: "aquatic".into(),
        }];
        let ctx = ExpressionContext::default();
        assert!(!evaluate_expression_conditions(&cond, &ctx));
    }

    #[test]
    fn scale_band_inclusive() {
        let cond = [ExpressionCondition::ScaleBand {
            min_kg: q(1.0),
            max_kg: q(100.0),
        }];
        for mass in [1.0, 50.0, 100.0] {
            let ctx = ExpressionContext {
                body_mass_kg: Some(q(mass)),
                ..ExpressionContext::default()
            };
            assert!(evaluate_expression_conditions(&cond, &ctx), "{mass}");
        }
        for mass in [0.5, 100.5] {
            let ctx = ExpressionContext {
                body_mass_kg: Some(q(mass)),
                ..ExpressionContext::default()
            };
            assert!(!evaluate_expression_conditions(&cond, &ctx), "{mass}");
        }
    }

    #[test]
    fn multiple_conditions_all_must_hold() {
        let cond = [
            ExpressionCondition::Season {
                season: "spring".into(),
            },
            ExpressionCondition::DevelopmentalStage {
                stage: "breeding_adult".into(),
            },
        ];
        let ctx_ok = ExpressionContext {
            season: Some("spring"),
            developmental_stage: Some("breeding_adult"),
            ..ExpressionContext::default()
        };
        assert!(evaluate_expression_conditions(&cond, &ctx_ok));

        let ctx_bad = ExpressionContext {
            season: Some("spring"),
            developmental_stage: Some("juvenile"),
            ..ExpressionContext::default()
        };
        assert!(!evaluate_expression_conditions(&cond, &ctx_bad));
    }
}
