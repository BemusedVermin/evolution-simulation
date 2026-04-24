//! Deterministic state hashing (S6.5 — issue #121).
//!
//! [`compute_state_hash`] produces a BLAKE3 digest of the full
//! simulation state, in the stable order defined by
//! [`beast_ecs::SortedEntityIndex`] and a fixed component-visitation
//! order. The output is used by the S6.6 replay gate: two runs of the
//! same `SimulationConfig` plus the same tick sequence must produce
//! bit-identical hashes every tick (INVARIANTS §1).
//!
//! # Why BLAKE3 and not `DefaultHasher`
//!
//! `std::collections::hash_map::DefaultHasher` uses `RandomState`
//! internally — its output is salted per-process, which is exactly what
//! a determinism gate does **not** want. BLAKE3 is deterministic,
//! cross-platform, has a stable spec, and is already a workspace
//! dependency.

use beast_ecs::components::{
    Age, DevelopmentalStage, GenomeComponent, HealthState, Mass, PhenotypeComponent, Position,
    Species, Velocity,
};
use beast_ecs::{Entity, MarkerKind, Resources, WorldExt};

use crate::simulation::Simulation;

/// Hash the entire simulation state deterministically.
///
/// # Algorithm
///
/// 1. Seed a BLAKE3 hasher with the tick counter + world seed so two
///    worlds that happen to have identical entity state but different
///    seeds or tick counts produce distinct hashes.
/// 2. Iterate [`beast_ecs::SortedEntityIndex::iter_all`], which yields
///    `(MarkerKind, Entity)` in `(MarkerKind asc, Entity asc)` order.
/// 3. For each entity, absorb:
///    * marker discriminant (u8)
///    * `entity.id()` (u32) + `entity.gen().id()` (i32) — both little-endian
///    * data components in a **fixed registration order** (Age, Mass,
///      HealthState, Position, Velocity, DevelopmentalStage, Species,
///      Genome, Phenotype). Each component emits a 1-byte tag (`1` if
///      present, `0` if absent) followed by its canonical bytes.
///
/// The `present/absent` tag matters: two entities where only one has a
/// `Mass` must hash differently, even if everything else matches.
///
/// Two sims with the same `SimulationConfig` and the same tick sequence
/// must produce bit-identical outputs on every tick (INVARIANTS §1).
#[must_use]
pub fn compute_state_hash(sim: &Simulation) -> [u8; 32] {
    let world = sim.world();
    let resources = sim.resources();
    let mut hasher = blake3::Hasher::new();
    absorb_preamble(&mut hasher, resources);

    // Pull every storage up front; a join-per-entity would re-fetch in
    // the inner loop and slow hashing ~10x.
    let ages = world.world().read_storage::<Age>();
    let masses = world.world().read_storage::<Mass>();
    let health = world.world().read_storage::<HealthState>();
    let positions = world.world().read_storage::<Position>();
    let velocities = world.world().read_storage::<Velocity>();
    let stages = world.world().read_storage::<DevelopmentalStage>();
    let species = world.world().read_storage::<Species>();
    let genomes = world.world().read_storage::<GenomeComponent>();
    let phenotypes = world.world().read_storage::<PhenotypeComponent>();

    for (marker, entity) in resources.entity_index.iter_all() {
        absorb_entity_header(&mut hasher, marker, entity);

        absorb_opt(&mut hasher, ages.get(entity), |h, a| {
            h.update(&a.ticks.to_le_bytes());
        });
        absorb_opt(&mut hasher, masses.get(entity), |h, m| {
            h.update(&m.kg.to_bits().to_le_bytes());
        });
        absorb_opt(&mut hasher, health.get(entity), |h, s| {
            h.update(&s.health.to_bits().to_le_bytes());
            h.update(&s.energy.to_bits().to_le_bytes());
        });
        absorb_opt(&mut hasher, positions.get(entity), |h, p| {
            h.update(&p.x.to_bits().to_le_bytes());
            h.update(&p.y.to_bits().to_le_bytes());
        });
        absorb_opt(&mut hasher, velocities.get(entity), |h, v| {
            h.update(&v.vx.to_bits().to_le_bytes());
            h.update(&v.vy.to_bits().to_le_bytes());
        });
        absorb_opt(&mut hasher, stages.get(entity), |h, s| {
            // Explicit match rather than `*s as u8` because
            // DevelopmentalStage has no #[repr(u8)]; inserting a
            // variant in the middle would silently renumber every
            // later variant, invalidating persisted hashes. See PR
            // #122 review note (HIGH).
            let stage_byte: u8 = match s {
                DevelopmentalStage::Egg => 0,
                DevelopmentalStage::Larval => 1,
                DevelopmentalStage::Juvenile => 2,
                DevelopmentalStage::Adult => 3,
                DevelopmentalStage::Geriatric => 4,
            };
            h.update(&[stage_byte]);
        });
        absorb_opt(&mut hasher, species.get(entity), |h, s| {
            h.update(&s.id.to_le_bytes());
        });
        // Genome and Phenotype are serialisable but their bit layout
        // can change between crate versions. Feed a canonical byte
        // stream: the Debug repr. This is slower than bincode but has
        // zero dependency cost and is only invoked once per tick.
        // When S7 lands, swap to the canonical bincode form.
        absorb_opt(&mut hasher, genomes.get(entity), |h, g| {
            h.update(format!("{:?}", g.0).as_bytes());
        });
        // Phenotype requires defensive sorting before feeding into the
        // hasher. `PrimitiveEffect.source_channels` is a `Vec<String>`
        // whose element order comes from whichever hook fired first —
        // the interpreter is expected to produce this sorted, but the
        // hash gate must not silently assume it. See PR #122 review
        // (CRITICAL). Sort is by primitive_id then by the
        // pre-sorted source_channels slice, which matches the
        // interpreter's contract. Allocations are per-tick, not
        // per-frame.
        absorb_opt(&mut hasher, phenotypes.get(entity), |h, p| {
            let mut sorted = p.effects.clone();
            sorted.sort_by(|a, b| {
                (&a.primitive_id, &a.body_site).cmp(&(&b.primitive_id, &b.body_site))
            });
            for effect in &mut sorted {
                effect.source_channels.sort();
            }
            h.update(format!("{:?}", sorted).as_bytes());
        });
    }

    *hasher.finalize().as_bytes()
}

fn absorb_preamble(hasher: &mut blake3::Hasher, resources: &Resources) {
    // Domain-separator: a fixed magic so hashes of this crate's
    // compute_state_hash can never collide with hashes of other state
    // that happens to start with the same bytes.
    hasher.update(b"beast-sim::state-hash::v1\0");
    hasher.update(&resources.tick_counter.raw().to_le_bytes());
    hasher.update(&resources.world_seed.to_le_bytes());
}

fn absorb_entity_header(hasher: &mut blake3::Hasher, marker: MarkerKind, entity: Entity) {
    // MarkerKind is repr(default) — map variants to a stable u8 here so
    // adding a variant later cannot silently reshuffle existing hashes.
    let marker_byte: u8 = match marker {
        MarkerKind::Creature => 0,
        MarkerKind::Pathogen => 1,
        MarkerKind::Agent => 2,
        MarkerKind::Faction => 3,
        MarkerKind::Settlement => 4,
        MarkerKind::Biome => 5,
    };
    hasher.update(&[marker_byte]);
    hasher.update(&entity.id().to_le_bytes());
    hasher.update(&entity.gen().id().to_le_bytes());
}

fn absorb_opt<T, F>(hasher: &mut blake3::Hasher, opt: Option<&T>, absorb: F)
where
    F: FnOnce(&mut blake3::Hasher, &T),
{
    match opt {
        Some(value) => {
            hasher.update(&[1]);
            absorb(hasher, value);
        }
        None => {
            hasher.update(&[0]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use beast_core::Q3232;
    use beast_ecs::components::{Age, Creature, Mass};
    use beast_ecs::{Builder, MarkerKind, WorldExt};

    use crate::{Simulation, SimulationConfig};

    #[test]
    fn empty_sim_hashes_reproducibly() {
        let a = Simulation::new(SimulationConfig::empty(1));
        let b = Simulation::new(SimulationConfig::empty(1));
        assert_eq!(compute_state_hash(&a), compute_state_hash(&b));
    }

    #[test]
    fn different_seeds_produce_different_hashes() {
        // Different world seeds ⇒ different preamble ⇒ different hashes
        // even with zero entities. Guards against accidentally forgetting
        // the seed in the preamble.
        let a = Simulation::new(SimulationConfig::empty(1));
        let b = Simulation::new(SimulationConfig::empty(2));
        assert_ne!(compute_state_hash(&a), compute_state_hash(&b));
    }

    #[test]
    fn ticking_changes_the_hash() {
        let mut sim = Simulation::new(SimulationConfig::empty(3));
        let before = compute_state_hash(&sim);
        sim.tick().expect("tick");
        let after = compute_state_hash(&sim);
        assert_ne!(before, after);
    }

    #[test]
    fn inserting_entity_changes_the_hash() {
        let mut sim = Simulation::new(SimulationConfig::empty(5));
        let before = compute_state_hash(&sim);
        // Add one creature with mass + age.
        let entity = sim
            .world_mut()
            .create_entity()
            .with(Creature)
            .with(Mass::new(Q3232::from_num(10)))
            .with(Age::new(0))
            .build();
        sim.resources_mut()
            .entity_index
            .insert(entity, MarkerKind::Creature);
        let after = compute_state_hash(&sim);
        assert_ne!(before, after);
    }

    #[test]
    fn mutating_component_changes_the_hash() {
        let mut sim = Simulation::new(SimulationConfig::empty(7));
        let entity = sim
            .world_mut()
            .create_entity()
            .with(Creature)
            .with(Age::new(0))
            .build();
        sim.resources_mut()
            .entity_index
            .insert(entity, MarkerKind::Creature);
        let before = compute_state_hash(&sim);

        // Bump age; the hash should strictly differ afterwards.
        {
            let mut ages = sim.world().world().write_storage::<Age>();
            ages.get_mut(entity).unwrap().ticks = 42;
        }

        let after = compute_state_hash(&sim);
        assert_ne!(before, after);
    }

    #[test]
    fn hash_is_byte_identical_across_repeated_calls() {
        // Same sim, no mutations between calls — the hash must not
        // depend on any hidden RandomState seed.
        let mut sim = Simulation::new(SimulationConfig::empty(11));
        let entity = sim
            .world_mut()
            .create_entity()
            .with(Creature)
            .with(Age::new(5))
            .build();
        sim.resources_mut()
            .entity_index
            .insert(entity, MarkerKind::Creature);

        let h1 = compute_state_hash(&sim);
        let h2 = compute_state_hash(&sim);
        let h3 = compute_state_hash(&sim);
        assert_eq!(h1, h2);
        assert_eq!(h2, h3);
    }

    #[test]
    fn component_presence_vs_absence_changes_the_hash() {
        // Two sims, identical except that one entity in sim_b has a Mass
        // component. Hashes must differ because absorb_opt emits a
        // different tag byte.
        let mut a = Simulation::new(SimulationConfig::empty(13));
        let mut b = Simulation::new(SimulationConfig::empty(13));

        let entity_a = a.world_mut().create_entity().with(Creature).build();
        a.resources_mut()
            .entity_index
            .insert(entity_a, MarkerKind::Creature);

        let entity_b = b
            .world_mut()
            .create_entity()
            .with(Creature)
            .with(Mass::new(Q3232::from_num(50)))
            .build();
        b.resources_mut()
            .entity_index
            .insert(entity_b, MarkerKind::Creature);

        assert_ne!(compute_state_hash(&a), compute_state_hash(&b));
    }
}
