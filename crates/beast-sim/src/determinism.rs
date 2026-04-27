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
use beast_manifest::Provenance;
use beast_primitives::PrimitiveEffect;

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
        absorb_opt(&mut hasher, ages.get(entity), absorb_age);
        absorb_opt(&mut hasher, masses.get(entity), absorb_mass);
        absorb_opt(&mut hasher, health.get(entity), absorb_health);
        absorb_opt(&mut hasher, positions.get(entity), absorb_position);
        absorb_opt(&mut hasher, velocities.get(entity), absorb_velocity);
        absorb_opt(&mut hasher, stages.get(entity), absorb_stage);
        absorb_opt(&mut hasher, species.get(entity), absorb_species);
        absorb_opt(&mut hasher, genomes.get(entity), absorb_genome);
        absorb_opt(&mut hasher, phenotypes.get(entity), absorb_phenotype);
    }

    *hasher.finalize().as_bytes()
}

fn absorb_age(h: &mut blake3::Hasher, a: &Age) {
    h.update(&a.ticks.to_le_bytes());
}

fn absorb_mass(h: &mut blake3::Hasher, m: &Mass) {
    h.update(&m.kg.to_bits().to_le_bytes());
}

fn absorb_health(h: &mut blake3::Hasher, s: &HealthState) {
    h.update(&s.health.to_bits().to_le_bytes());
    h.update(&s.energy.to_bits().to_le_bytes());
}

fn absorb_position(h: &mut blake3::Hasher, p: &Position) {
    h.update(&p.x.to_bits().to_le_bytes());
    h.update(&p.y.to_bits().to_le_bytes());
}

fn absorb_velocity(h: &mut blake3::Hasher, v: &Velocity) {
    h.update(&v.vx.to_bits().to_le_bytes());
    h.update(&v.vy.to_bits().to_le_bytes());
}

/// Explicit match rather than `*s as u8` because `DevelopmentalStage` has
/// no `#[repr(u8)]`; inserting a variant in the middle would silently
/// renumber every later variant, invalidating persisted hashes. See PR
/// #122 review note (HIGH).
fn absorb_stage(h: &mut blake3::Hasher, s: &DevelopmentalStage) {
    let stage_byte: u8 = match s {
        DevelopmentalStage::Egg => 0,
        DevelopmentalStage::Larval => 1,
        DevelopmentalStage::Juvenile => 2,
        DevelopmentalStage::Adult => 3,
        DevelopmentalStage::Geriatric => 4,
    };
    h.update(&[stage_byte]);
}

fn absorb_species(h: &mut blake3::Hasher, s: &Species) {
    h.update(&s.id.to_le_bytes());
}

/// Genome: bincode 2.x with the same `config::standard()` that
/// `beast-serde` uses for save files. Stable across rustc / edition
/// upgrades and ~10× cheaper than the previous `format!("{:?}", ...)`
/// pre-S7 code path. Encoding can fail only on a serializer-internal
/// error (size_limit overflow on a >2^32-byte payload, etc.) which a
/// real genome cannot produce — the `expect` carries the same
/// impossibility contract as `Genome::validate`'s `GenomeTooLarge` guard.
fn absorb_genome(h: &mut blake3::Hasher, g: &GenomeComponent) {
    let cfg = bincode::config::standard();
    let bytes = bincode::serde::encode_to_vec(&g.0, cfg)
        .expect("Genome bincode encoding fits within size_limit");
    h.update(&bytes);
}

/// Phenotype: hand-encoded field-by-field instead of bincode'd.
/// `PrimitiveEffect` is L1 and intentionally does **not** derive
/// `Serialize` (that's gated by the wider serialize-on-the-sim-path
/// question — see audit finding #67); a per-effect canonical byte stream
/// gets us the same determinism guarantee without dragging serde into
/// `beast-primitives`.
///
/// Defensive sort: the interpreter contract emits sorted, but the hash
/// gate must not assume — see PR #122 review (CRITICAL). Sort by
/// `(primitive_id, body_site)` (matches INVARIANTS §1's hash-order
/// contract) and sort each `source_channels` Vec since hook firing order
/// isn't pinned.
fn absorb_phenotype(h: &mut blake3::Hasher, p: &PhenotypeComponent) {
    let mut sorted = p.effects.clone();
    sorted.sort_by(|a, b| (&a.primitive_id, &a.body_site).cmp(&(&b.primitive_id, &b.body_site)));
    for effect in &mut sorted {
        effect.source_channels.sort();
    }
    h.update(&(sorted.len() as u64).to_le_bytes());
    for effect in &sorted {
        absorb_primitive_effect(h, effect);
    }
}

fn absorb_preamble(hasher: &mut blake3::Hasher, resources: &Resources) {
    // Domain-separator: a fixed magic so hashes of this crate's
    // compute_state_hash can never collide with hashes of other state
    // that happens to start with the same bytes. The trailing `\0`
    // acts as a length-delimited sentinel — if the separator ever
    // changes length, prefix collisions against the old form are
    // impossible.
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
    // Explicit types pin the byte widths: a future specs release that
    // changes `Entity::id()` → u64 or `Generation::id()` → i64 would
    // still compile via coercion, silently changing hash output. The
    // `let : T` bindings force a compile error on any width change.
    let entity_id: u32 = entity.id();
    let gen_id: i32 = entity.gen().id();
    hasher.update(&entity_id.to_le_bytes());
    hasher.update(&gen_id.to_le_bytes());
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

/// Length-prefixed `&str` absorb. The `u64` length comes first so a
/// trailing string is unambiguously delimited from any following bytes
/// fed into the hasher.
fn absorb_str(hasher: &mut blake3::Hasher, s: &str) {
    hasher.update(&(s.len() as u64).to_le_bytes());
    hasher.update(s.as_bytes());
}

/// Encode a single [`PrimitiveEffect`] into a canonical byte stream
/// for hashing. Stable across `Debug` impl changes and rustc / edition
/// upgrades because every field is laid out in fixed order with
/// explicit length prefixes — the same guarantee bincode would give
/// us, except this avoids forcing a `Serialize` derive onto L1's
/// `PrimitiveEffect` (audit finding #67 tracks the wider question of
/// whether `PrimitiveEffect` should be serializable on the save path).
fn absorb_primitive_effect(hasher: &mut blake3::Hasher, effect: &PrimitiveEffect) {
    absorb_str(hasher, &effect.primitive_id);
    // body_site: 1-byte presence tag + 1-byte ordinal (the
    // ordinal-pin test in `beast-core::body_site` keeps adding a
    // variant from silently shifting hashes).
    match effect.body_site {
        Some(site) => {
            hasher.update(&[1]);
            // BodySite has no #[repr(u8)]; the explicit match below
            // makes adding a variant a compile error, not a silent
            // hash-output change.
            use beast_core::BodySite;
            let ordinal: u8 = match site {
                BodySite::Global => 0,
                BodySite::Head => 1,
                BodySite::Jaw => 2,
                BodySite::Core => 3,
                BodySite::LimbLeft => 4,
                BodySite::LimbRight => 5,
                BodySite::Tail => 6,
                BodySite::Appendage => 7,
            };
            hasher.update(&[ordinal]);
        }
        None => {
            hasher.update(&[0]);
        }
    }
    // source_channels: caller has already sorted them.
    hasher.update(&(effect.source_channels.len() as u64).to_le_bytes());
    for ch in &effect.source_channels {
        absorb_str(hasher, ch);
    }
    // parameters: BTreeMap iterates in key order ⇒ stable.
    hasher.update(&(effect.parameters.len() as u64).to_le_bytes());
    for (k, v) in &effect.parameters {
        absorb_str(hasher, k);
        hasher.update(&v.to_bits().to_le_bytes());
    }
    hasher.update(&effect.activation_cost.to_bits().to_le_bytes());
    hasher.update(&effect.emitter.raw().to_le_bytes());
    absorb_provenance(hasher, &effect.provenance);
}

/// Length-prefixed canonical schema-string form for [`Provenance`].
/// `to_schema_string` is the same form save files use, so the digest
/// is stable across the same dimensions a save round-trip would be.
fn absorb_provenance(hasher: &mut blake3::Hasher, p: &Provenance) {
    absorb_str(hasher, &p.to_schema_string());
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
    fn tick_counter_changes_the_hash() {
        // Empty world → the only thing that varies between `before` and
        // `after` is the tick counter in the preamble. Proves the
        // preamble feeds into the hash. The full-world case is covered
        // by `mutating_component_changes_the_hash` below.
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
