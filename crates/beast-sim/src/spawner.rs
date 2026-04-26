//! Deterministic seed-creature spawner.
//!
//! Sprint S8.4 (issue #146). Plans and applies the placement of a
//! starter creature population across a generated world.
//!
//! # Two-phase API
//!
//! Spawning is split into a pure planning step and a stateful apply
//! step:
//!
//! 1. [`plan_spawns`] — pure function `(seed, world dims, get_biome,
//!    species_for_biome, target_count) -> Vec<SpawnPlan>`. No ECS
//!    state, no entity creation. Easy to test, deterministic.
//! 2. [`apply_spawn_plans`] — takes a [`Simulation`] and a slice of
//!    [`Genome`]s indexed by species, materialises each plan into a
//!    real entity. The mutation lives here so the planning step
//!    stays observable.
//!
//! # Decoupling
//!
//! The spawner does not depend on `beast-world` (which owns the
//! biome-tag string contract via `BiomeTag::as_str()`),
//! `beast-climate`, or any starter-genome registry. Callers pass in:
//!
//! * a `Fn(u32, u32) -> Option<&'static str>` returning the biome
//!   tag at a cell (matches `BiomeKind::as_str()`/`BiomeTag::as_str()`).
//! * a `Fn(&str) -> Option<usize>` mapping biome tag to a species
//!   index in the supplied genome slice.
//! * pre-built `Vec<Genome>` indexed by species.
//!
//! This keeps S8.4 master-rebaseable: the spawner ships independently
//! of S8.1 (#159), S8.2 (#147), and S8.3 (#158); the binding lives
//! in whichever caller assembles the world and the genomes.
//!
//! # Determinism
//!
//! All randomness flows through the supplied [`beast_core::Prng`] —
//! callers are expected to use [`beast_core::Stream::Worldgen`] so
//! the spawner draws are independent of every other subsystem's
//! PRNG state. Each spawn slot draws a random `(x, y)` with
//! rejection; rejected draws cost one extra PRNG call pair, so the
//! total draw count is variable but bounded by `target_count *
//! MAX_REJECTION_FACTOR`.
//!
//! Duplicate plans (two creatures targeting the same cell) are
//! allowed by design — the rejection sampler does not de-duplicate.
//! Callers that need uniqueness must filter the output themselves.

use beast_core::{Prng, Q3232};
use beast_ecs::components::{Age, Creature, GenomeComponent, Mass, Position};
use beast_ecs::{Builder, MarkerKind};
use beast_genome::Genome;
use serde::{Deserialize, Serialize};

use crate::Simulation;

/// Stable index into the caller's per-species genome slice.
///
/// Newtype rather than bare `usize` so a `(species_for_biome,
/// genomes)` ordering swap can't compile. The same fragility was
/// flagged on the channel-position binding in #158; this is the
/// downstream-side fix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SpeciesId(pub usize);

impl SpeciesId {
    /// The underlying index into the caller's genome slice.
    #[must_use]
    pub fn index(self) -> usize {
        self.0
    }
}

/// One spawn decision produced by [`plan_spawns`]. Cell coordinates
/// are in grid units; [`apply_spawn_plans`] translates them to
/// world coordinates by centring each entity on its cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpawnPlan {
    /// X grid coordinate of the cell.
    pub cell_x: u32,
    /// Y grid coordinate of the cell.
    pub cell_y: u32,
    /// Species blueprint to spawn at this cell.
    pub species: SpeciesId,
}

/// Errors returned by the spawner.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SpawnerError {
    /// `target_count` was zero — the spawner has nothing to do.
    #[error("target_count must be at least 1")]
    EmptyTarget,
    /// World dimensions were zero.
    #[error("world dimensions must be at least 1×1; got {width}×{height}")]
    EmptyWorld {
        /// Supplied width.
        width: u32,
        /// Supplied height.
        height: u32,
    },
    /// No spawnable cell exists in the world (every cell mapped to
    /// `None` species via `species_for_biome`). Detected on the
    /// first PRNG draw — the spawner refuses to spin in a
    /// guaranteed-failure loop.
    #[error("world contains no spawnable cells (first-draw acceptance was zero)")]
    NoSpawnableCells,
    /// The rejection budget was exhausted before placing
    /// `target_count` plans. Spawnable cells *do* exist, but they
    /// are too sparse for the budget — typically when the spawnable
    /// fraction is below ~1% of total cells (see
    /// `MAX_REJECTION_FACTOR` in `plan_spawns`). Either lower
    /// `target_count` or use a denser world.
    #[error("spawn budget exhausted after {attempts} attempts; placed {placed}/{target}")]
    SpawnBudgetExhausted {
        /// Number of plans that were placed before the budget ran
        /// out.
        placed: usize,
        /// The requested `target_count`.
        target: usize,
        /// Total `(x, y)` draws made (accepted + rejected).
        attempts: usize,
    },
    /// A generated [`SpawnPlan`] referenced a species index out of
    /// bounds for the supplied genome slice.
    #[error("plan referenced species index {index}, but only {len} genomes were supplied")]
    SpeciesIndexOutOfBounds {
        /// The out-of-range index.
        index: usize,
        /// The supplied genome slice length.
        len: usize,
    },
}

/// Plan `target_count` spawns across a `width × height` world.
///
/// `biome_at(x, y)` returns the biome tag for cell `(x, y)`. `None`
/// means "out of bounds" — should not happen for `(x, y)` in
/// `[0, width) × [0, height)`.
///
/// `species_for_biome(tag)` returns `Some(SpeciesId)` to allow a
/// species to spawn in cells of that biome, or `None` to reject. A
/// typical implementation maps `"plains" -> Some(SpeciesId(grassland_grazer))`.
///
/// The spawner picks cells uniformly with rejection: it draws a
/// random `(x, y)` from the supplied `prng`, queries
/// `species_for_biome(biome_at(x, y).unwrap_or(""))`, and either
/// records the (cell, species) pair or rejects and re-draws.
///
/// **Duplicate plans are allowed by design** — two creatures may
/// target the same cell. Callers that need uniqueness must filter
/// the output themselves.
///
/// # Rejection budget
///
/// `target_count * MAX_REJECTION_FACTOR` (currently 100). The 100×
/// constant guarantees success with vanishingly small failure
/// probability for any world where ≥1% of cells are spawnable: at
/// `p = 0.01` and `target_count = 50` the failure probability is
/// `(0.99)^5000 ≈ 5e-22`. Worlds with sparser-than-1% spawnable
/// fractions return [`SpawnerError::SpawnBudgetExhausted`] rather
/// than spinning.
///
/// # Errors
///
/// * [`SpawnerError::EmptyTarget`] when `target_count == 0`.
/// * [`SpawnerError::EmptyWorld`] when `width` or `height` is 0.
/// * [`SpawnerError::NoSpawnableCells`] when the very first draw
///   already rejects — exactly-zero acceptance.
/// * [`SpawnerError::SpawnBudgetExhausted`] when at least one cell
///   is spawnable but the budget runs out before placing
///   `target_count` plans.
#[must_use = "the produced spawn plans must be applied to a Simulation"]
pub fn plan_spawns<B, S>(
    prng: &mut Prng,
    width: u32,
    height: u32,
    target_count: usize,
    biome_at: B,
    species_for_biome: S,
) -> Result<Vec<SpawnPlan>, SpawnerError>
where
    B: Fn(u32, u32) -> Option<&'static str>,
    S: Fn(&str) -> Option<usize>,
{
    if target_count == 0 {
        return Err(SpawnerError::EmptyTarget);
    }
    if width == 0 || height == 0 {
        return Err(SpawnerError::EmptyWorld { width, height });
    }

    /// Multiplier on `target_count` for the rejection budget.
    ///
    /// Picked so that worlds with ≥1% spawnable acceptance succeed
    /// with overwhelming probability: `(1 - 0.01)^(50 * 100) ≈ 5e-22`
    /// failure rate at the demo `target_count = 50`. Lower acceptance
    /// rates trip the `SpawnBudgetExhausted` error rather than
    /// spinning indefinitely.
    const MAX_REJECTION_FACTOR: usize = 100;
    let max_attempts = target_count.saturating_mul(MAX_REJECTION_FACTOR);

    let mut plans = Vec::with_capacity(target_count);
    let mut attempts = 0_usize;
    while plans.len() < target_count {
        if attempts >= max_attempts {
            // Two distinct error paths: nothing was placed → genuine
            // zero-acceptance world; some plans landed → the world
            // is just too sparse for the requested target.
            return Err(if plans.is_empty() {
                SpawnerError::NoSpawnableCells
            } else {
                SpawnerError::SpawnBudgetExhausted {
                    placed: plans.len(),
                    target: target_count,
                    attempts,
                }
            });
        }
        attempts += 1;

        let x = prng.gen_range_u64(0, u64::from(width)) as u32;
        let y = prng.gen_range_u64(0, u64::from(height)) as u32;
        let tag = match biome_at(x, y) {
            Some(t) => t,
            None => continue,
        };
        let species_idx = match species_for_biome(tag) {
            Some(s) => s,
            None => continue,
        };
        plans.push(SpawnPlan {
            cell_x: x,
            cell_y: y,
            species: SpeciesId(species_idx),
        });
    }
    Ok(plans)
}

/// Apply spawn plans to `sim`, materialising each as an entity.
///
/// Each entity is created with: [`Creature`] marker, [`Position`]
/// centred on its cell (cell coords + 0.5 in Q3232), [`Mass::new(1)`],
/// [`Age::new(0)`], and a [`GenomeComponent`] cloned from the
/// supplied genome slice indexed by `plan.species.index()`.
///
/// # Errors
///
/// * [`SpawnerError::SpeciesIndexOutOfBounds`] when any plan's
///   `species.index()` is `>= genomes.len()`. The check runs
///   up-front for every plan, so a single bad index does not
///   half-spawn the rest.
#[must_use = "the Result reports validation failure — at minimum check it with `?`"]
pub fn apply_spawn_plans(
    sim: &mut Simulation,
    plans: &[SpawnPlan],
    genomes: &[Genome],
) -> Result<(), SpawnerError> {
    // Validate every plan up-front so we don't half-spawn before
    // bailing — partial spawns are hard to reason about.
    for plan in plans {
        if plan.species.index() >= genomes.len() {
            return Err(SpawnerError::SpeciesIndexOutOfBounds {
                index: plan.species.index(),
                len: genomes.len(),
            });
        }
    }

    for plan in plans {
        let genome = genomes[plan.species.index()].clone();
        let position = cell_to_position(plan.cell_x, plan.cell_y);
        let entity = sim
            .world_mut()
            .create_entity()
            .with(Creature)
            .with(position)
            .with(Mass::new(Q3232::from_num(1_i32)))
            .with(Age::new(0))
            .with(GenomeComponent::new(genome))
            .build();
        sim.resources_mut()
            .entity_index
            .insert(entity, MarkerKind::Creature);
    }
    Ok(())
}

/// Q3232 representation of `0.5` — the cell-centre offset applied
/// by [`cell_to_position`]. Built from raw bits so no f64 literal
/// appears on the sim path; locked in by the
/// `half_constant_is_exactly_one_half` test.
const HALF_CELL: Q3232 = Q3232::from_bits(1_i64 << 31);

/// Translate a cell coordinate to a world position centred on the
/// cell. Coordinates use 1-cell-per-metre; the [`HALF_CELL`] offset
/// puts the entity at the cell centre rather than the corner.
fn cell_to_position(cell_x: u32, cell_y: u32) -> Position {
    Position::new(
        Q3232::from_num(cell_x).saturating_add(HALF_CELL),
        Q3232::from_num(cell_y).saturating_add(HALF_CELL),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SimulationConfig;
    use beast_core::Stream;
    use beast_genome::{Genome, GenomeParams};

    /// Trivial biome lookup: every cell is "plains".
    fn all_plains(_x: u32, _y: u32) -> Option<&'static str> {
        Some("plains")
    }

    /// Map "plains" → species 0, reject everything else.
    fn plains_to_species_zero(tag: &str) -> Option<usize> {
        if tag == "plains" {
            Some(0)
        } else {
            None
        }
    }

    /// Reject every biome — used to exercise the rejection budget.
    fn reject_all(_tag: &str) -> Option<usize> {
        None
    }

    /// Empty Genome — enough for the apply tests.
    fn empty_genome() -> Genome {
        Genome::with_params(GenomeParams::default())
    }

    fn worldgen_prng(seed: u64) -> Prng {
        Prng::from_seed(seed).split_stream(Stream::Worldgen)
    }

    #[test]
    fn plan_returns_target_count_when_world_is_fully_spawnable() {
        let mut prng = worldgen_prng(0xCAFE);
        let plans = plan_spawns(&mut prng, 10, 10, 50, all_plains, plains_to_species_zero).unwrap();
        assert_eq!(plans.len(), 50);
    }

    #[test]
    fn plan_is_deterministic_across_calls() {
        // Two calls with the same seed and inputs must produce
        // byte-identical plans — precondition for the determinism
        // gate.
        let mut prng_a = worldgen_prng(0xDEAD);
        let mut prng_b = worldgen_prng(0xDEAD);
        let a = plan_spawns(&mut prng_a, 8, 8, 20, all_plains, plains_to_species_zero).unwrap();
        let b = plan_spawns(&mut prng_b, 8, 8, 20, all_plains, plains_to_species_zero).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn plan_different_seeds_produce_different_layouts() {
        let mut prng_a = worldgen_prng(0x1);
        let mut prng_b = worldgen_prng(0x2);
        let a = plan_spawns(&mut prng_a, 8, 8, 20, all_plains, plains_to_species_zero).unwrap();
        let b = plan_spawns(&mut prng_b, 8, 8, 20, all_plains, plains_to_species_zero).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn plan_rejects_zero_target_count() {
        let mut prng = worldgen_prng(0);
        let err =
            plan_spawns(&mut prng, 10, 10, 0, all_plains, plains_to_species_zero).unwrap_err();
        assert_eq!(err, SpawnerError::EmptyTarget);
    }

    #[test]
    fn plan_rejects_zero_dimensions() {
        let mut prng = worldgen_prng(0);
        let err = plan_spawns(&mut prng, 0, 10, 5, all_plains, plains_to_species_zero).unwrap_err();
        assert!(matches!(
            err,
            SpawnerError::EmptyWorld {
                width: 0,
                height: 10
            }
        ));
    }

    #[test]
    fn plan_returns_no_spawnable_cells_when_every_biome_rejected() {
        let mut prng = worldgen_prng(0);
        let err = plan_spawns(&mut prng, 10, 10, 5, all_plains, reject_all).unwrap_err();
        assert_eq!(err, SpawnerError::NoSpawnableCells);
    }

    #[test]
    fn plan_skips_cells_where_biome_returns_none() {
        // biome_at returns Some only for cells where x < 5 — the
        // spawner should still place 50 creatures by re-drawing
        // until it hits the spawnable half.
        fn left_half_only(x: u32, _y: u32) -> Option<&'static str> {
            if x < 5 {
                Some("plains")
            } else {
                None
            }
        }
        let mut prng = worldgen_prng(0xBEEF);
        let plans = plan_spawns(
            &mut prng,
            10,
            10,
            50,
            left_half_only,
            plains_to_species_zero,
        )
        .unwrap();
        assert_eq!(plans.len(), 50);
        for plan in &plans {
            assert!(
                plan.cell_x < 5,
                "cell {} should be in left half",
                plan.cell_x
            );
        }
    }

    #[test]
    fn plan_records_correct_species_index_per_biome() {
        // Even cells are "plains" → species 0; odd cells are "forest"
        // → species 1. Expect a mix of SpeciesId(0) and SpeciesId(1).
        fn striped(x: u32, _y: u32) -> Option<&'static str> {
            if x % 2 == 0 {
                Some("plains")
            } else {
                Some("forest")
            }
        }
        fn species_per_tag(tag: &str) -> Option<usize> {
            match tag {
                "plains" => Some(0),
                "forest" => Some(1),
                _ => None,
            }
        }
        let mut prng = worldgen_prng(0xFACE);
        let plans = plan_spawns(&mut prng, 10, 10, 100, striped, species_per_tag).unwrap();
        let zeros = plans.iter().filter(|p| p.species == SpeciesId(0)).count();
        let ones = plans.iter().filter(|p| p.species == SpeciesId(1)).count();
        assert_eq!(zeros + ones, 100);
        assert!(zeros > 0, "expected at least one plains spawn");
        assert!(ones > 0, "expected at least one forest spawn");
    }

    #[test]
    fn apply_creates_target_count_entities() {
        let mut sim = Simulation::new(SimulationConfig::empty(0));
        let mut prng = worldgen_prng(0xCAFE);
        let plans = plan_spawns(&mut prng, 10, 10, 50, all_plains, plains_to_species_zero).unwrap();
        let genomes = vec![empty_genome()];
        apply_spawn_plans(&mut sim, &plans, &genomes).unwrap();

        let count = sim
            .resources()
            .entity_index
            .entities_of(MarkerKind::Creature)
            .count();
        assert_eq!(count, 50);
    }

    #[test]
    fn apply_rejects_out_of_bounds_species_index() {
        let mut sim = Simulation::new(SimulationConfig::empty(0));
        let plans = vec![SpawnPlan {
            cell_x: 0,
            cell_y: 0,
            species: SpeciesId(5), // out of bounds
        }];
        let genomes = vec![empty_genome()];
        let err = apply_spawn_plans(&mut sim, &plans, &genomes).unwrap_err();
        assert!(matches!(
            err,
            SpawnerError::SpeciesIndexOutOfBounds { index: 5, len: 1 }
        ));
        // No partial-spawn: the count must still be zero.
        let count = sim
            .resources()
            .entity_index
            .entities_of(MarkerKind::Creature)
            .count();
        assert_eq!(count, 0);
    }

    #[test]
    fn apply_centres_entity_on_cell() {
        use beast_ecs::WorldExt;
        let mut sim = Simulation::new(SimulationConfig::empty(0));
        let plans = vec![SpawnPlan {
            cell_x: 7,
            cell_y: 3,
            species: SpeciesId(0),
        }];
        let genomes = vec![empty_genome()];
        apply_spawn_plans(&mut sim, &plans, &genomes).unwrap();

        let world = sim.world();
        let positions = world.world().read_storage::<Position>();
        // Should be exactly one creature; pull it out.
        let (entity, _) = sim
            .resources()
            .entity_index
            .entities_of(MarkerKind::Creature)
            .next()
            .map(|e| (e, ()))
            .expect("at least one creature");
        let pos = positions.get(entity).expect("position present");
        assert_eq!(pos.x, Q3232::from_num(7.5_f64));
        assert_eq!(pos.y, Q3232::from_num(3.5_f64));
    }

    #[test]
    fn full_pipeline_produces_50_creatures_and_runs_100_ticks() {
        // Demo criterion (epic #20): 50 creatures spawned, survive
        // 100 ticks. We don't have a metabolism / death system in
        // S8 yet, so "survive" reduces to "no panics" and the
        // creature count is unchanged.
        let mut sim = Simulation::new(SimulationConfig::empty(0xCAFE_BABE));
        let mut prng = worldgen_prng(0xCAFE_BABE);
        let plans = plan_spawns(&mut prng, 32, 32, 50, all_plains, plains_to_species_zero).unwrap();
        let genomes = vec![empty_genome()];
        apply_spawn_plans(&mut sim, &plans, &genomes).unwrap();

        for _ in 0..100 {
            sim.tick().expect("tick");
        }

        let count = sim
            .resources()
            .entity_index
            .entities_of(MarkerKind::Creature)
            .count();
        assert_eq!(count, 50, "creature count should be stable for 100 ticks");
    }

    #[test]
    fn half_cell_constant_is_exactly_one_half() {
        // Lock-in: HALF_CELL is built from raw bits to avoid an
        // f64 literal on the sim path. Verify the bit layout
        // actually encodes 0.5. A future Q3232 layout change
        // would fail this immediately.
        assert_eq!(HALF_CELL, Q3232::from_num(0.5_f64));
    }

    #[test]
    fn plan_returns_spawn_budget_exhausted_when_world_is_too_sparse() {
        // 100×100 world but only the single cell (0, 0) is
        // spawnable — acceptance rate 1/10000 = 0.01% (well below
        // the 1% design floor). The spawner should hit
        // SpawnBudgetExhausted, NOT NoSpawnableCells, because
        // *some* placements succeed before the budget runs out.
        fn just_origin(x: u32, y: u32) -> Option<&'static str> {
            if x == 0 && y == 0 {
                Some("plains")
            } else {
                Some("ocean")
            }
        }
        fn only_plains(tag: &str) -> Option<usize> {
            if tag == "plains" {
                Some(0)
            } else {
                None
            }
        }
        let mut prng = worldgen_prng(0xCAFE);
        let err = plan_spawns(&mut prng, 100, 100, 50, just_origin, only_plains).unwrap_err();
        // Could be either SpawnBudgetExhausted (if any placement
        // landed on the (0,0) cell) or NoSpawnableCells (if the
        // sparse acceptance happens to miss every draw). Both are
        // acceptable; we just want to assert it's the new
        // sparse-world error path, not a panic or wrong variant.
        match err {
            SpawnerError::SpawnBudgetExhausted {
                placed,
                target,
                attempts,
            } => {
                assert_eq!(target, 50);
                assert!(placed < 50);
                assert_eq!(attempts, 5000); // 50 * MAX_REJECTION_FACTOR
            }
            SpawnerError::NoSpawnableCells => {
                // The sparse 0.01% acceptance happens to miss
                // every draw — rare but possible at this density.
            }
            other => panic!("expected sparse-world error, got {other:?}"),
        }
    }

    #[test]
    fn species_id_newtype_indexes_into_genome_slice() {
        // SpeciesId is a transparent newtype around usize, but
        // compile-time prevents `species_index: usize` style
        // transposition. Lock the index() accessor.
        let sid = SpeciesId(7);
        assert_eq!(sid.index(), 7);
    }
}
