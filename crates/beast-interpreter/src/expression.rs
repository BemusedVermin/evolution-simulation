//! Environmental affordance filter (S4.2 — issue #56).
//!
//! Thin interpreter-side wrapper over
//! [`beast_channels::evaluate_expression_conditions`]. Given an
//! [`crate::Environment`] and a slice of [`crate::InterpreterHook`]s, returns
//! the sorted subset of hook ids whose `expression_conditions` pass.
//!
//! See `documentation/systems/11_phenotype_interpreter.md` §5.0c (Operator C:
//! Environmental Affordances) and §6.2 Stage 2A (Filter hooks by environmental
//! affordances).
//!
//! # Determinism
//!
//! * The output `Vec<HookId>` is sorted ascending by [`HookId`], which is
//!   lexicographic on the underlying `u32`. No `HashMap` / `HashSet` iteration
//!   leaks into sim state.
//! * Condition evaluation delegates to [`beast_channels`], which operates on
//!   [`beast_core::Q3232`] and string equality only — no floats, no RNG, no
//!   wall-clock.

use beast_channels::{evaluate_expression_conditions, ExpressionContext};
use beast_core::Q3232;

use crate::composition::{HookId, InterpreterHook};
use crate::phenotype::{Environment, LifeStage};

/// Return the sorted ids of hooks whose environmental affordances all hold.
///
/// A hook with an empty `expression_conditions` list is always kept (schema
/// semantics: "no conditions" means "always express"; see §5.0c).
///
/// The output is sorted ascending by [`HookId`] so downstream stages can rely
/// on stable iteration order without re-sorting.
///
/// # Arguments
///
/// * `env` — environmental snapshot (biome flags, season, population density).
///   `life_stage` and `body_mass_kg` are passed separately because they live
///   on the resolved phenotype, not on `Environment`.
/// * `life_stage` — creature's current life stage; mapped onto
///   `ExpressionContext::developmental_stage` via [`LifeStage::as_str`].
/// * `body_mass_kg` — creature's body mass, used by
///   [`beast_channels::ExpressionCondition::ScaleBand`] gates that may
///   appear on composition hooks.
/// * `hooks` — interpreter-level hooks to filter. Iterated once, in input
///   order; the result is sorted at the end.
///
/// # Example
///
/// ```
/// use beast_core::Q3232;
/// use beast_channels::ExpressionCondition;
/// use beast_interpreter::{
///     filter_hooks_by_affordances, Environment, HookId, InterpreterHook, LifeStage,
/// };
/// use beast_interpreter::composition::CompositionKind;
///
/// let hook = InterpreterHook {
///     id: HookId(1),
///     kind: CompositionKind::Additive,
///     channel_ids: Vec::new(),
///     thresholds: Vec::new(),
///     coefficient: Q3232::from_num(1_i32),
///     expression_conditions: vec![ExpressionCondition::Season {
///         season: "spring".into(),
///     }],
///     emits: Vec::new(),
/// };
///
/// let env = Environment {
///     season: Some("spring".into()),
///     ..Environment::default()
/// };
/// let active = filter_hooks_by_affordances(
///     &env,
///     LifeStage::Adult,
///     Q3232::from_num(10_i32),
///     std::slice::from_ref(&hook),
/// );
/// assert_eq!(active, vec![HookId(1)]);
/// ```
pub fn filter_hooks_by_affordances(
    env: &Environment,
    life_stage: LifeStage,
    body_mass_kg: Q3232,
    hooks: &[InterpreterHook],
) -> Vec<HookId> {
    // Borrow the Vec<String> biome flags as &[&str] for ExpressionContext.
    // Single allocation up front so the borrow is reused across every hook.
    let biome_flags_borrowed: Vec<&str> = env.biome_flags.iter().map(String::as_str).collect();

    let ctx = ExpressionContext {
        biome_flags: &biome_flags_borrowed[..],
        body_mass_kg: Some(body_mass_kg),
        season: env.season.as_deref(),
        developmental_stage: Some(life_stage.as_str()),
        population_density_per_km2: env.population_density_per_km2,
    };

    let mut active: Vec<HookId> = hooks
        .iter()
        .filter(|h| evaluate_expression_conditions(&h.expression_conditions, &ctx))
        .map(|h| h.id)
        .collect();
    active.sort();
    active
}

#[cfg(test)]
mod tests {
    use super::*;

    use beast_channels::ExpressionCondition;
    use beast_core::Q3232;

    use crate::composition::{CompositionKind, EmitSpec, HookId, InterpreterHook};

    fn q(v: f64) -> Q3232 {
        Q3232::from_num(v)
    }

    /// Build a bare hook with the supplied id and expression_conditions; other
    /// fields are minimal placeholders not exercised by the filter.
    fn hook(id: u32, conditions: Vec<ExpressionCondition>) -> InterpreterHook {
        InterpreterHook {
            id: HookId(id),
            kind: CompositionKind::Additive,
            channel_ids: Vec::new(),
            thresholds: Vec::new(),
            coefficient: q(1.0),
            expression_conditions: conditions,
            emits: Vec::<EmitSpec>::new(),
        }
    }

    fn env_default() -> Environment {
        Environment::default()
    }

    #[test]
    fn empty_conditions_list_always_passes() {
        let h = hook(7, Vec::new());
        let out = filter_hooks_by_affordances(
            &env_default(),
            LifeStage::Adult,
            q(10.0),
            std::slice::from_ref(&h),
        );
        assert_eq!(out, vec![HookId(7)]);
    }

    #[test]
    fn biome_flag_gate_passes_when_flag_active() {
        let h = hook(
            1,
            vec![ExpressionCondition::BiomeFlag {
                flag: "aquatic".into(),
            }],
        );
        let env = Environment {
            biome_flags: vec!["aquatic".into(), "cold".into()],
            ..Environment::default()
        };
        let out =
            filter_hooks_by_affordances(&env, LifeStage::Adult, q(10.0), std::slice::from_ref(&h));
        assert_eq!(out, vec![HookId(1)]);
    }

    #[test]
    fn biome_flag_gate_rejects_when_flag_absent() {
        let h = hook(
            1,
            vec![ExpressionCondition::BiomeFlag {
                flag: "aquatic".into(),
            }],
        );
        let env = Environment {
            biome_flags: vec!["desert".into()],
            ..Environment::default()
        };
        let out =
            filter_hooks_by_affordances(&env, LifeStage::Adult, q(10.0), std::slice::from_ref(&h));
        assert!(out.is_empty());
    }

    #[test]
    fn scale_band_gate_uses_body_mass_argument() {
        let h = hook(
            2,
            vec![ExpressionCondition::ScaleBand {
                min_kg: q(1.0),
                max_kg: q(100.0),
            }],
        );
        let in_band = filter_hooks_by_affordances(
            &env_default(),
            LifeStage::Adult,
            q(50.0),
            std::slice::from_ref(&h),
        );
        assert_eq!(in_band, vec![HookId(2)]);

        let out_of_band = filter_hooks_by_affordances(
            &env_default(),
            LifeStage::Adult,
            q(500.0),
            std::slice::from_ref(&h),
        );
        assert!(out_of_band.is_empty());
    }

    #[test]
    fn season_gate_matches_env_season() {
        let h = hook(
            3,
            vec![ExpressionCondition::Season {
                season: "spring".into(),
            }],
        );
        let env_ok = Environment {
            season: Some("spring".into()),
            ..Environment::default()
        };
        let env_bad = Environment {
            season: Some("winter".into()),
            ..Environment::default()
        };
        assert_eq!(
            filter_hooks_by_affordances(
                &env_ok,
                LifeStage::Adult,
                q(10.0),
                std::slice::from_ref(&h)
            ),
            vec![HookId(3)],
        );
        assert!(filter_hooks_by_affordances(
            &env_bad,
            LifeStage::Adult,
            q(10.0),
            std::slice::from_ref(&h)
        )
        .is_empty());
    }

    #[test]
    fn developmental_stage_gate_uses_life_stage_argument() {
        let h = hook(
            4,
            vec![ExpressionCondition::DevelopmentalStage {
                stage: "juvenile".into(),
            }],
        );
        let juvenile = filter_hooks_by_affordances(
            &env_default(),
            LifeStage::Juvenile,
            q(10.0),
            std::slice::from_ref(&h),
        );
        assert_eq!(juvenile, vec![HookId(4)]);

        let adult = filter_hooks_by_affordances(
            &env_default(),
            LifeStage::Adult,
            q(10.0),
            std::slice::from_ref(&h),
        );
        assert!(adult.is_empty());
    }

    #[test]
    fn social_density_gate_uses_env_density() {
        let h = hook(
            5,
            vec![ExpressionCondition::SocialDensity {
                min_per_km2: q(10.0),
                max_per_km2: q(100.0),
            }],
        );
        let env_ok = Environment {
            population_density_per_km2: Some(q(50.0)),
            ..Environment::default()
        };
        let env_bad = Environment {
            population_density_per_km2: Some(q(5.0)),
            ..Environment::default()
        };
        assert_eq!(
            filter_hooks_by_affordances(
                &env_ok,
                LifeStage::Adult,
                q(10.0),
                std::slice::from_ref(&h)
            ),
            vec![HookId(5)],
        );
        assert!(filter_hooks_by_affordances(
            &env_bad,
            LifeStage::Adult,
            q(10.0),
            std::slice::from_ref(&h)
        )
        .is_empty());

        // And if the env did not report density at all, a density condition
        // evaluates to false (conservative default, per beast-channels).
        let env_missing = Environment::default();
        assert!(filter_hooks_by_affordances(
            &env_missing,
            LifeStage::Adult,
            q(10.0),
            std::slice::from_ref(&h),
        )
        .is_empty());
    }

    #[test]
    fn multiple_conditions_require_all_to_hold() {
        let h = hook(
            6,
            vec![
                ExpressionCondition::Season {
                    season: "spring".into(),
                },
                ExpressionCondition::DevelopmentalStage {
                    stage: "adult".into(),
                },
                ExpressionCondition::ScaleBand {
                    min_kg: q(1.0),
                    max_kg: q(100.0),
                },
            ],
        );
        let env_spring = Environment {
            season: Some("spring".into()),
            ..Environment::default()
        };
        // All three conditions hold.
        assert_eq!(
            filter_hooks_by_affordances(
                &env_spring,
                LifeStage::Adult,
                q(10.0),
                std::slice::from_ref(&h)
            ),
            vec![HookId(6)],
        );
        // Stage mismatch — hook drops.
        assert!(filter_hooks_by_affordances(
            &env_spring,
            LifeStage::Juvenile,
            q(10.0),
            std::slice::from_ref(&h),
        )
        .is_empty());
        // Mass outside band — hook drops.
        assert!(filter_hooks_by_affordances(
            &env_spring,
            LifeStage::Adult,
            q(1000.0),
            std::slice::from_ref(&h),
        )
        .is_empty());
    }

    #[test]
    fn mixed_hooks_only_passing_ids_returned_sorted() {
        // Three hooks with varied gating; feed in non-sorted id order so the
        // sort() step is observable.
        let passing_a = hook(
            30,
            vec![ExpressionCondition::BiomeFlag {
                flag: "forest".into(),
            }],
        );
        let failing = hook(
            20,
            vec![ExpressionCondition::Season {
                season: "winter".into(),
            }],
        );
        let passing_b = hook(10, Vec::new()); // unconditional
        let hooks = [passing_a, failing, passing_b];
        let env = Environment {
            biome_flags: vec!["forest".into()],
            season: Some("spring".into()),
            ..Environment::default()
        };

        let out = filter_hooks_by_affordances(&env, LifeStage::Adult, q(10.0), &hooks);
        assert_eq!(out, vec![HookId(10), HookId(30)]);
    }

    #[test]
    fn output_is_deterministic_across_calls() {
        // Determinism check: same inputs → byte-identical Vec<HookId>.
        let hooks = [
            hook(
                3,
                vec![ExpressionCondition::Season {
                    season: "spring".into(),
                }],
            ),
            hook(1, Vec::new()),
            hook(
                2,
                vec![ExpressionCondition::BiomeFlag {
                    flag: "aquatic".into(),
                }],
            ),
            hook(
                4,
                vec![ExpressionCondition::ScaleBand {
                    min_kg: q(1.0),
                    max_kg: q(100.0),
                }],
            ),
        ];
        let env = Environment {
            biome_flags: vec!["aquatic".into()],
            season: Some("spring".into()),
            ..Environment::default()
        };

        let first = filter_hooks_by_affordances(&env, LifeStage::Adult, q(10.0), &hooks);
        let second = filter_hooks_by_affordances(&env, LifeStage::Adult, q(10.0), &hooks);
        assert_eq!(first, second);
        assert_eq!(first, vec![HookId(1), HookId(2), HookId(3), HookId(4)]);
    }
}
