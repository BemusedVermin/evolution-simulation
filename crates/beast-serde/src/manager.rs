//! `SaveManager` — capture a [`Simulation`] into a [`SaveFile`] and
//! restore one (S7.2 — issue #130).
//!
//! The capture/restore pair is the user-facing entry point of the save
//! layer; everything else (validator in S7.4, migration registry in
//! S7.5) wraps these two functions.
//!
//! # Determinism contract
//!
//! Round-tripping any simulation through `save_game` → `load_game` must
//! produce a sim whose [`compute_state_hash`](beast_sim::compute_state_hash)
//! equals the original. This is the property the M2 Determinism gate
//! (S7.6) checks across an extended tick window.
//!
//! Two subtleties keep the round-trip clean:
//!
//! 1. **Entity allocator parity.** A fresh `specs::World` allocates
//!    entity ids deterministically — `1, 2, 3, …` with `gen=1` — given
//!    insertions in the same order. The save file lists entities in
//!    `id`-ascending order, and `load_game` re-inserts them in that
//!    order, so the restored `Entity` values (`id` + `gen`) match the
//!    originals byte-for-byte. This breaks if the original sim deleted
//!    entities (specs reuses slots and bumps generation), so MVP-era
//!    saves that contain deletions need the slot-reuse story to land
//!    first; not a concern today because no system deletes entities.
//!
//! 2. **PRNG state restoration.** `Resources::new` re-derives the eight
//!    streams from `world_seed`. We overwrite them after construction
//!    with the persisted `PrngStreams` so any draws made before the
//!    snapshot are preserved.
//!
//! # Atomic file I/O
//!
//! [`save_to_path`] writes through a [`tempfile::NamedTempFile`] in the
//! destination directory and then `persist`s atomically over the target.
//! A crash mid-write leaves the temp file behind but never corrupts the
//! existing save — the rename is atomic on every supported platform.

use std::fs;
use std::io;
use std::path::Path;

use beast_channels::{ChannelRegistry, RegistryFingerprint};
use beast_core::TickCounter;
use beast_ecs::components::{
    Age, Creature, DevelopmentalStage, GenomeComponent, HealthState, Mass, Pathogen, Position,
    Species, Velocity,
};
use beast_ecs::components::{Agent, Biome, Faction, Settlement};
use beast_ecs::{Builder, MarkerKind, WorldExt};
use beast_primitives::PrimitiveRegistry;
use beast_sim::{Simulation, SimulationConfig};
use thiserror::Error;

use crate::save::{PrngStreams, SaveFile, SerializedEntity, SerializedMarker, SAVE_FORMAT_VERSION};

/// Errors produced by [`save_game`], [`load_game`], and the path
/// variants. Distinct from [`crate::save::SaveError`] because the
/// orchestrator surfaces failure modes the envelope-level encoder
/// can't see (fingerprint mismatch, version mismatch, I/O failure).
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ManagerError {
    /// The envelope-level (de)serialization failed.
    #[error("save (de)serialization failed: {0}")]
    Save(#[from] crate::save::SaveError),

    /// The save file's `format_version` does not match the binary's
    /// expected version. Migrations (S7.5) will close this gap; for now
    /// the loader requires an exact match.
    #[error("unsupported save format version: expected {expected}, found {found}")]
    UnsupportedVersion {
        /// Version this build understands ([`SAVE_FORMAT_VERSION`]).
        expected: &'static str,
        /// Version the save file declared.
        found: String,
    },

    /// The channel registry the caller passed in does not match the
    /// fingerprint stamped into the save. Loading would silently break
    /// determinism — the genome's positional indices reference a
    /// different vocabulary.
    #[error(
        "channel registry mismatch: save was written against {actual}, current registry is {expected}"
    )]
    ChannelRegistryMismatch {
        /// Hex of the active registry's fingerprint.
        expected: String,
        /// Hex of the fingerprint stored in the save.
        actual: String,
    },

    /// Same as [`Self::ChannelRegistryMismatch`] but for the primitive
    /// vocabulary.
    #[error(
        "primitive registry mismatch: save was written against {actual}, current registry is {expected}"
    )]
    PrimitiveRegistryMismatch {
        /// Hex of the active registry's fingerprint.
        expected: String,
        /// Hex of the fingerprint stored in the save.
        actual: String,
    },

    /// The on-disk path operation failed.
    #[error("save i/o failed: {0}")]
    Io(#[from] io::Error),
}

/// Magic for the primitive-registry fingerprint. Mirrors `CRF1` for
/// channels but keyed `PRF1` so a channel-vs-primitive id swap is
/// caught even if both vocabularies happen to be empty.
const PRIMITIVE_FINGERPRINT_MAGIC: &[u8; 4] = b"PRF1";

/// Compute a `RegistryFingerprint` for a [`PrimitiveRegistry`].
///
/// Hash layout intentionally mirrors `ChannelRegistry::fingerprint`'s
/// `CRF1` form, with two changes:
///
/// * Magic bytes are `PRF1` instead of `CRF1`, so identical-shaped
///   channel and primitive registries never produce equal fingerprints.
/// * Only `id` and `provenance` participate — the primitive's
///   `category` is treated like the channel's `family` (a stable
///   semantic tag).
///
/// Today the only callers are the save layer; if other crates need this
/// helper, hoist it into `beast-primitives` to keep the contract on the
/// owning crate.
pub fn primitive_fingerprint(registry: &PrimitiveRegistry) -> RegistryFingerprint {
    let mut hasher = blake3::Hasher::new();
    hasher.update(PRIMITIVE_FINGERPRINT_MAGIC);
    let count = u32::try_from(registry.len()).expect("primitive registry exceeds u32::MAX");
    hasher.update(&count.to_le_bytes());
    for (id, manifest) in registry.iter() {
        write_len_prefixed(&mut hasher, id.as_bytes());
        write_len_prefixed(&mut hasher, category_tag(manifest).as_bytes());
        let provenance = manifest.provenance.to_schema_string();
        write_len_prefixed(&mut hasher, provenance.as_bytes());
    }
    RegistryFingerprint(*hasher.finalize().as_bytes())
}

fn write_len_prefixed(hasher: &mut blake3::Hasher, bytes: &[u8]) {
    let len = u32::try_from(bytes.len()).expect("primitive metadata length exceeds u32::MAX");
    hasher.update(&len.to_le_bytes());
    hasher.update(bytes);
}

fn category_tag(manifest: &beast_primitives::PrimitiveManifest) -> &'static str {
    use beast_primitives::PrimitiveCategory;
    // Stable ASCII tags decoupled from `serde(rename_all = "snake_case")`
    // so a future serde rename pattern cannot silently shift save-file
    // fingerprints — same discipline as `family_tag` in
    // `beast-channels::fingerprint`.
    match manifest.category {
        PrimitiveCategory::SignalEmission => "signal_emission",
        PrimitiveCategory::SignalReception => "signal_reception",
        PrimitiveCategory::ForceApplication => "force_application",
        PrimitiveCategory::StateInduction => "state_induction",
        PrimitiveCategory::SpatialIntegration => "spatial_integration",
        PrimitiveCategory::MassTransfer => "mass_transfer",
        PrimitiveCategory::EnergyModulation => "energy_modulation",
        PrimitiveCategory::BondFormation => "bond_formation",
    }
}

/// Capture a [`Simulation`] into a deterministic [`SaveFile`].
///
/// # Errors
///
/// Returns [`ManagerError::Save`] if a component fails to serialize.
/// (In practice this is unreachable for the components shipped today,
/// but the result type leaves room for future storage backends.)
pub fn save_game(sim: &Simulation) -> Result<SaveFile, ManagerError> {
    let resources = sim.resources();
    let world = sim.world();

    let ages = world.world().read_storage::<Age>();
    let masses = world.world().read_storage::<Mass>();
    let health = world.world().read_storage::<HealthState>();
    let positions = world.world().read_storage::<Position>();
    let velocities = world.world().read_storage::<Velocity>();
    let stages = world.world().read_storage::<DevelopmentalStage>();
    let species = world.world().read_storage::<Species>();
    let genomes = world.world().read_storage::<GenomeComponent>();

    // Group `(MarkerKind, Entity)` pairs by entity so the same entity
    // can carry multiple markers (e.g., a Faction that's also a
    // Settlement once those systems exist). Today every entity carries
    // at most one marker, but the wire format accommodates more.
    use std::collections::BTreeMap;
    let mut grouped: BTreeMap<u64, (beast_ecs::Entity, Vec<SerializedMarker>)> = BTreeMap::new();
    for (marker, entity) in resources.entity_index.iter_all() {
        let key = pack_entity_key(entity);
        grouped
            .entry(key)
            .or_insert_with(|| (entity, Vec::new()))
            .1
            .push(marker_to_wire(marker));
    }

    let mut entities = Vec::with_capacity(grouped.len());
    for (id, (entity, markers)) in grouped {
        entities.push(SerializedEntity {
            id,
            markers,
            position: positions.get(entity).copied(),
            velocity: velocities.get(entity).copied(),
            age: ages.get(entity).copied(),
            mass: masses.get(entity).copied(),
            health: health.get(entity).copied(),
            stage: stages.get(entity).copied(),
            species: species.get(entity).copied(),
            genome: genomes.get(entity).cloned(),
            extras: BTreeMap::new(),
        });
    }

    Ok(SaveFile {
        format_version: SAVE_FORMAT_VERSION.to_string(),
        current_tick: resources.tick_counter.raw(),
        world_seed: resources.world_seed,
        rng_streams: PrngStreams {
            genetics: resources.rng_genetics.clone(),
            phenotype: resources.rng_phenotype.clone(),
            physics: resources.rng_physics.clone(),
            combat: resources.rng_combat.clone(),
            physiology: resources.rng_physiology.clone(),
            ecology: resources.rng_ecology.clone(),
            worldgen: resources.rng_worldgen.clone(),
            chronicler: resources.rng_chronicler.clone(),
        },
        channel_fingerprint: resources.channels.fingerprint(),
        primitive_fingerprint: primitive_fingerprint(&resources.primitives),
        entities,
    })
}

/// Restore a [`Simulation`] from a [`SaveFile`].
///
/// `channels` and `primitives` are loaded by the application from
/// manifest files; the loader compares their fingerprints against the
/// values stamped into the save and refuses to hydrate on mismatch.
///
/// # Errors
///
/// * [`ManagerError::UnsupportedVersion`] if the save's
///   `format_version` differs from [`SAVE_FORMAT_VERSION`]. The
///   migration registry in S7.5 will narrow this to "no migration path
///   from `from` to `to`".
/// * [`ManagerError::ChannelRegistryMismatch`] /
///   [`ManagerError::PrimitiveRegistryMismatch`] if the supplied
///   registries do not match the save's stored fingerprints.
pub fn load_game(
    file: SaveFile,
    channels: ChannelRegistry,
    primitives: PrimitiveRegistry,
) -> Result<Simulation, ManagerError> {
    if file.format_version != SAVE_FORMAT_VERSION {
        return Err(ManagerError::UnsupportedVersion {
            expected: SAVE_FORMAT_VERSION,
            found: file.format_version,
        });
    }

    let actual_channel_fp = channels.fingerprint();
    if actual_channel_fp != file.channel_fingerprint {
        return Err(ManagerError::ChannelRegistryMismatch {
            expected: actual_channel_fp.to_hex(),
            actual: file.channel_fingerprint.to_hex(),
        });
    }
    let actual_primitive_fp = primitive_fingerprint(&primitives);
    if actual_primitive_fp != file.primitive_fingerprint {
        return Err(ManagerError::PrimitiveRegistryMismatch {
            expected: actual_primitive_fp.to_hex(),
            actual: file.primitive_fingerprint.to_hex(),
        });
    }

    let mut sim = Simulation::new(SimulationConfig {
        world_seed: file.world_seed,
        channels,
        primitives,
    });

    // Overwrite the freshly-derived PRNG streams with the persisted
    // ones — the simulation may have already drawn from them before the
    // snapshot was taken.
    {
        let resources = sim.resources_mut();
        resources.rng_genetics = file.rng_streams.genetics;
        resources.rng_phenotype = file.rng_streams.phenotype;
        resources.rng_physics = file.rng_streams.physics;
        resources.rng_combat = file.rng_streams.combat;
        resources.rng_physiology = file.rng_streams.physiology;
        resources.rng_ecology = file.rng_streams.ecology;
        resources.rng_worldgen = file.rng_streams.worldgen;
        resources.rng_chronicler = file.rng_streams.chronicler;
        resources.tick_counter = TickCounter::new(file.current_tick);
    }

    // Re-create entities in `id`-ascending order. With a fresh
    // `specs::World`, this regenerates the same `(id, gen)` pairs the
    // original simulation produced — see module docs for the
    // assumption (no entity deletion in MVP).
    let mut sorted = file.entities;
    sorted.sort_by_key(|e| e.id);
    for entity_record in sorted {
        let entity = build_entity(&mut sim, &entity_record);
        for marker in &entity_record.markers {
            sim.resources_mut()
                .entity_index
                .insert(entity, wire_to_marker(*marker));
        }
    }

    Ok(sim)
}

/// Pack the specs `(id, gen)` pair into a single `u64`. Layout:
/// `(gen.id() as u32) << 32 | id`. This is byte-stable for any specs
/// version that keeps the inherent `id() -> u32` and `gen().id() ->
/// i32` shapes — the existing determinism hash relies on the same
/// guarantee (see `beast-sim::determinism::absorb_entity_header`).
fn pack_entity_key(entity: beast_ecs::Entity) -> u64 {
    let id = u64::from(entity.id());
    // `gen().id()` is `i32` (positive after first allocation). Cast to
    // u32 first to avoid sign-extension into the high bits.
    let gen = u64::from(entity.gen().id() as u32);
    (gen << 32) | id
}

fn marker_to_wire(marker: MarkerKind) -> SerializedMarker {
    match marker {
        MarkerKind::Creature => SerializedMarker::Creature,
        MarkerKind::Pathogen => SerializedMarker::Pathogen,
        MarkerKind::Agent => SerializedMarker::Agent,
        MarkerKind::Faction => SerializedMarker::Faction,
        MarkerKind::Settlement => SerializedMarker::Settlement,
        MarkerKind::Biome => SerializedMarker::Biome,
    }
}

fn wire_to_marker(marker: SerializedMarker) -> MarkerKind {
    match marker {
        SerializedMarker::Creature => MarkerKind::Creature,
        SerializedMarker::Pathogen => MarkerKind::Pathogen,
        SerializedMarker::Agent => MarkerKind::Agent,
        SerializedMarker::Faction => MarkerKind::Faction,
        SerializedMarker::Settlement => MarkerKind::Settlement,
        SerializedMarker::Biome => MarkerKind::Biome,
    }
}

/// Re-create one entity from its serialized form.
///
/// We attach the relevant marker components first, then the data
/// components. Markers are zero-sized (`NullStorage`) so the order is
/// observationally indistinguishable, but doing markers first keeps the
/// builder chain short and matches the order used by spawner code in
/// `beast-sim`'s tests.
fn build_entity(sim: &mut Simulation, record: &SerializedEntity) -> beast_ecs::Entity {
    let mut builder = sim.world_mut().create_entity();
    for marker in &record.markers {
        builder = match marker {
            SerializedMarker::Creature => builder.with(Creature),
            SerializedMarker::Pathogen => builder.with(Pathogen),
            SerializedMarker::Agent => builder.with(Agent),
            SerializedMarker::Faction => builder.with(Faction),
            SerializedMarker::Settlement => builder.with(Settlement),
            SerializedMarker::Biome => builder.with(Biome),
        };
    }
    if let Some(p) = record.position {
        builder = builder.with(p);
    }
    if let Some(v) = record.velocity {
        builder = builder.with(v);
    }
    if let Some(a) = record.age {
        builder = builder.with(a);
    }
    if let Some(m) = record.mass {
        builder = builder.with(m);
    }
    if let Some(h) = record.health {
        builder = builder.with(h);
    }
    if let Some(s) = record.stage {
        builder = builder.with(s);
    }
    if let Some(s) = record.species {
        builder = builder.with(s);
    }
    if let Some(g) = record.genome.clone() {
        builder = builder.with(g);
    }
    builder.build()
}

/// Capture and write a [`Simulation`] to `path` atomically.
///
/// Writes the bincode form into a sibling temp file in the same
/// directory and renames it over the target. A mid-write crash leaves
/// the existing save untouched; the temp file is removed if `persist`
/// fails.
///
/// # Errors
///
/// Returns [`ManagerError::Io`] if the temp-file creation, write, or
/// rename fails. Returns [`ManagerError::Save`] if encoding fails.
pub fn save_to_path(sim: &Simulation, path: &Path) -> Result<(), ManagerError> {
    let bytes = save_game(sim)?.to_bincode()?;
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty());
    let parent = match parent {
        Some(p) => p,
        // No parent (e.g., a bare filename in CWD) — write into ".".
        None => Path::new("."),
    };
    fs::create_dir_all(parent)?;
    let mut temp = tempfile_in(parent)?;
    use std::io::Write as _;
    temp.write_all(&bytes)?;
    temp.flush()?;
    temp.persist(path).map_err(|e| ManagerError::Io(e.error))?;
    Ok(())
}

/// Read a save file from `path` and restore it.
///
/// # Errors
///
/// Same set as [`load_game`], plus [`ManagerError::Io`] for filesystem
/// and decode failures.
pub fn load_from_path(
    path: &Path,
    channels: ChannelRegistry,
    primitives: PrimitiveRegistry,
) -> Result<Simulation, ManagerError> {
    let bytes = fs::read(path)?;
    let file = SaveFile::from_bincode(&bytes)?;
    load_game(file, channels, primitives)
}

/// Centralise the tempfile creation so a future swap (e.g., to a
/// hand-rolled atomic-rename implementation) is one line.
fn tempfile_in(dir: &Path) -> io::Result<tempfile::NamedTempFile> {
    tempfile::NamedTempFile::new_in(dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use beast_core::Q3232;
    use beast_ecs::components::{Age, Creature, Mass};
    use beast_ecs::{Builder, MarkerKind};
    use beast_sim::{compute_state_hash, Simulation, SimulationConfig};

    fn fixture(seed: u64, n_creatures: usize) -> Simulation {
        let mut sim = Simulation::new(SimulationConfig::empty(seed));
        for i in 0..n_creatures {
            let entity = sim
                .world_mut()
                .create_entity()
                .with(Creature)
                .with(Age::new(i as u64))
                .with(Mass::new(Q3232::from_num((i + 1) as i32)))
                .build();
            sim.resources_mut()
                .entity_index
                .insert(entity, MarkerKind::Creature);
        }
        sim
    }

    #[test]
    fn round_trip_preserves_state_hash_empty_world() {
        let sim = Simulation::new(SimulationConfig::empty(123));
        let save = save_game(&sim).unwrap();
        let loaded = load_game(save, ChannelRegistry::new(), PrimitiveRegistry::new()).unwrap();
        assert_eq!(compute_state_hash(&sim), compute_state_hash(&loaded));
    }

    #[test]
    fn round_trip_preserves_state_hash_with_entities() {
        let sim = fixture(42, 16);
        let save = save_game(&sim).unwrap();
        let loaded = load_game(save, ChannelRegistry::new(), PrimitiveRegistry::new()).unwrap();
        assert_eq!(compute_state_hash(&sim), compute_state_hash(&loaded));
    }

    #[test]
    fn round_trip_preserves_tick_counter() {
        let mut sim = fixture(42, 4);
        for _ in 0..7 {
            sim.tick().unwrap();
        }
        let save = save_game(&sim).unwrap();
        assert_eq!(save.current_tick, 7);
        let loaded = load_game(save, ChannelRegistry::new(), PrimitiveRegistry::new()).unwrap();
        assert_eq!(loaded.current_tick().raw(), 7);
    }

    #[test]
    fn round_trip_preserves_prng_state() {
        // Original sim draws from rng_genetics 5 times; the next draw
        // after a save/load round-trip must match what the original
        // would have produced.
        let mut sim = fixture(7, 0);
        for _ in 0..5 {
            sim.resources_mut().rng_genetics.next_u64();
        }
        let next_original = sim.resources_mut().rng_genetics.next_u64();

        let mut sim2 = fixture(7, 0);
        for _ in 0..5 {
            sim2.resources_mut().rng_genetics.next_u64();
        }
        let save = save_game(&sim2).unwrap();
        let mut loaded = load_game(save, ChannelRegistry::new(), PrimitiveRegistry::new()).unwrap();
        let next_loaded = loaded.resources_mut().rng_genetics.next_u64();

        assert_eq!(next_original, next_loaded);
    }

    // Helper: load_game returns Result<Simulation, _> and `Simulation`
    // does not derive `Debug`, so `expect_err` is unavailable. Match
    // on the result directly instead.
    fn expect_load_err(result: Result<Simulation, ManagerError>, msg: &str) -> ManagerError {
        match result {
            Err(e) => e,
            Ok(_) => panic!("{msg}"),
        }
    }

    #[test]
    fn loader_rejects_format_version_mismatch() {
        let sim = Simulation::new(SimulationConfig::empty(1));
        let mut save = save_game(&sim).unwrap();
        save.format_version = "9.9.9".to_string();
        let err = expect_load_err(
            load_game(save, ChannelRegistry::new(), PrimitiveRegistry::new()),
            "expected version mismatch",
        );
        match err {
            ManagerError::UnsupportedVersion { expected, found } => {
                assert_eq!(expected, SAVE_FORMAT_VERSION);
                assert_eq!(found, "9.9.9");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn loader_rejects_channel_fingerprint_mismatch() {
        let sim = Simulation::new(SimulationConfig::empty(1));
        let mut save = save_game(&sim).unwrap();
        // Tamper the stored fingerprint.
        save.channel_fingerprint = RegistryFingerprint([0xAB; 32]);
        let err = expect_load_err(
            load_game(save, ChannelRegistry::new(), PrimitiveRegistry::new()),
            "expected channel fingerprint mismatch",
        );
        assert!(matches!(err, ManagerError::ChannelRegistryMismatch { .. }));
    }

    #[test]
    fn loader_rejects_primitive_fingerprint_mismatch() {
        let sim = Simulation::new(SimulationConfig::empty(1));
        let mut save = save_game(&sim).unwrap();
        save.primitive_fingerprint = RegistryFingerprint([0xCD; 32]);
        let err = expect_load_err(
            load_game(save, ChannelRegistry::new(), PrimitiveRegistry::new()),
            "expected primitive fingerprint mismatch",
        );
        assert!(matches!(
            err,
            ManagerError::PrimitiveRegistryMismatch { .. }
        ));
    }

    #[test]
    fn save_to_path_and_load_back_round_trips() {
        let sim = fixture(99, 8);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("world.bsv");

        save_to_path(&sim, &path).unwrap();
        assert!(path.exists());

        let loaded =
            load_from_path(&path, ChannelRegistry::new(), PrimitiveRegistry::new()).unwrap();
        assert_eq!(compute_state_hash(&sim), compute_state_hash(&loaded));
    }

    #[test]
    fn save_to_path_overwrites_atomically() {
        // Write twice to the same path. The second write succeeds and
        // produces a hash matching the second sim.
        let sim_a = fixture(1, 4);
        let sim_b = fixture(2, 4);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("save.bsv");

        save_to_path(&sim_a, &path).unwrap();
        save_to_path(&sim_b, &path).unwrap();
        let loaded =
            load_from_path(&path, ChannelRegistry::new(), PrimitiveRegistry::new()).unwrap();
        assert_eq!(compute_state_hash(&sim_b), compute_state_hash(&loaded));
        // Hash A and hash B differ — sanity check that the test really
        // exercised the second write.
        assert_ne!(compute_state_hash(&sim_a), compute_state_hash(&sim_b));
    }

    #[test]
    fn primitive_fingerprint_is_stable_for_empty_registry() {
        let a = primitive_fingerprint(&PrimitiveRegistry::new());
        let b = primitive_fingerprint(&PrimitiveRegistry::new());
        assert_eq!(a, b);
        // Different magic from channel registry — empty channel
        // registry must produce a different fingerprint than empty
        // primitive registry.
        let chan = ChannelRegistry::new().fingerprint();
        assert_ne!(a, chan, "empty primitive vs channel collision");
    }
}
