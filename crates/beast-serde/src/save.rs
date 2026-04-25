//! `SaveFile` — the canonical, versioned envelope for one tick of
//! simulation state (S7.1 — issue #129).
//!
//! Capturing a [`beast_sim::Simulation`] into a [`SaveFile`] and
//! restoring one is the orchestrator's job (S7.2). This story only
//! defines the envelope shape and proves it round-trips byte-stable
//! through both the binary ([`bincode`]) and JSON ([`serde_json`])
//! representations.
//!
//! # Determinism contract
//!
//! Every collection field is a sorted container (`Vec`, `BTreeMap`,
//! `BTreeSet`); no `HashMap`/`HashSet` ever crosses the serialization
//! boundary. The two encoders chosen here both honour declared field
//! order:
//!
//! * `bincode` 2.x with `config::standard()` writes a deterministic,
//!   little-endian, length-prefixed wire format; same struct value →
//!   same byte sequence on every machine and every run.
//! * `serde_json` writes fields in declaration order for structs and
//!   key order for `BTreeMap`s. Two `SaveFile`s with equal contents
//!   serialize to byte-identical JSON.
//!
//! # Forward-compatibility
//!
//! [`SAVE_FORMAT_VERSION`] is the schema version stamped into every
//! envelope. The migration registry (S7.5) consumes this field to
//! upgrade older saves. The envelope uses
//! `#[serde(deny_unknown_fields)]` so an older binary that encounters
//! a new field fails loudly instead of silently dropping data.

use std::collections::BTreeMap;

use beast_channels::RegistryFingerprint;
use beast_core::prng::Prng;
use beast_ecs::components::{
    Age, DevelopmentalStage, GenomeComponent, HealthState, Mass, Position, Species, Velocity,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// On-disk envelope schema version. Bump whenever the layout changes in
/// a way the migration registry (S7.5) cannot handle automatically via a
/// `from -> to` step.
///
/// Convention: semver string. The MVP starts at `0.1.0`.
pub const SAVE_FORMAT_VERSION: &str = "0.1.0";

/// The persisted form of a single [`beast_sim::Simulation`] at a tick
/// boundary.
///
/// Captures everything required to byte-reproduce the run from this
/// point forward, given the same channel + primitive registries
/// (identified by [`Self::channel_fingerprint`] /
/// [`Self::primitive_fingerprint`]).
///
/// The registries themselves are **not** stored — they live in
/// manifest files on disk, loaded at process start. The fingerprints
/// guard against loading a save against a divergent vocabulary, which
/// would silently break replay determinism.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveFile {
    /// Schema version stamp. Must equal [`SAVE_FORMAT_VERSION`] for the
    /// running binary, modulo migrations applied by the migration
    /// registry.
    pub format_version: String,
    /// Tick the snapshot was captured at. Round-trips back into the
    /// [`beast_core::TickCounter`] of the restored simulation.
    pub current_tick: u64,
    /// Master world seed. Combined with the eight per-stream PRNG
    /// states this uniquely determines all future stochastic decisions.
    pub world_seed: u64,
    /// Snapshot of the eight per-subsystem PRNG streams, in the
    /// declared `Stream` enum order. Restored verbatim — never
    /// re-derived from `world_seed`, because subsystems may have
    /// already drawn from their streams before the save.
    pub rng_streams: PrngStreams,
    /// Fingerprint of the channel registry the run was using at save
    /// time. Loader compares against the current `ChannelRegistry::fingerprint`
    /// and refuses to hydrate on mismatch (S7.4).
    pub channel_fingerprint: RegistryFingerprint,
    /// Fingerprint of the primitive registry. Same role as
    /// `channel_fingerprint` but for the primitive vocabulary.
    pub primitive_fingerprint: RegistryFingerprint,
    /// Per-entity records, sorted by [`SerializedEntity::id`].
    pub entities: Vec<SerializedEntity>,
}

impl SaveFile {
    /// Render a `SaveFile` to a deterministic binary blob via
    /// [`bincode`] with `config::standard()`. Two equal `SaveFile`s
    /// produce byte-identical output.
    ///
    /// # Errors
    ///
    /// Returns [`SaveError::Bincode`] if the bincode encoder reports a
    /// failure (in practice impossible for the in-memory shape used
    /// here, since every field is statically sized or a length-prefixed
    /// `Vec`/`BTreeMap`).
    pub fn to_bincode(&self) -> Result<Vec<u8>, SaveError> {
        self.assert_entities_sorted();
        let cfg = bincode::config::standard();
        bincode::serde::encode_to_vec(self, cfg).map_err(|e| SaveError::Bincode(e.to_string()))
    }

    /// Restore a `SaveFile` from a bincode blob produced by
    /// [`Self::to_bincode`].
    ///
    /// # Errors
    ///
    /// Returns [`SaveError::Bincode`] on any decoder failure (truncated
    /// input, version mismatch, **or trailing bytes after the first
    /// record**). The trailing-bytes check exists so a caller passing a
    /// concatenated stream (journal blob, fuzz fixture with checksum
    /// trailer) receives a loud failure instead of silently getting only
    /// the first record. Per PR #135 review.
    pub fn from_bincode(bytes: &[u8]) -> Result<Self, SaveError> {
        let cfg = bincode::config::standard();
        let (decoded, consumed) = bincode::serde::decode_from_slice::<Self, _>(bytes, cfg)
            .map_err(|e| SaveError::Bincode(e.to_string()))?;
        if consumed != bytes.len() {
            return Err(SaveError::Bincode(format!(
                "trailing bytes after save record: consumed {consumed}, slice was {} bytes",
                bytes.len()
            )));
        }
        Ok(decoded)
    }

    /// Render a `SaveFile` to canonical JSON via [`serde_json`]. Field
    /// order follows struct declaration order; `BTreeMap` keys are
    /// sorted, so two equal `SaveFile`s produce byte-identical JSON
    /// (modulo whitespace from `to_string` vs `to_string_pretty`).
    ///
    /// # Errors
    ///
    /// Returns [`SaveError::Json`] on encoder failure.
    pub fn to_json(&self) -> Result<String, SaveError> {
        self.assert_entities_sorted();
        serde_json::to_string(self).map_err(SaveError::Json)
    }

    /// Debug-only sanity: `entities` must be sorted by `id` for the
    /// save to be deterministic. Producers (today only
    /// `crate::manager::save_game`) build the vec from a `BTreeMap`
    /// so the invariant holds by construction; this guard catches
    /// future hand-built fixtures or out-of-band manipulation. Per
    /// PR #135 review.
    fn assert_entities_sorted(&self) {
        debug_assert!(
            self.entities.windows(2).all(|w| w[0].id <= w[1].id),
            "SaveFile::entities must be sorted ascending by id; found out-of-order entry"
        );
    }

    /// Restore a `SaveFile` from JSON produced by [`Self::to_json`].
    ///
    /// # Errors
    ///
    /// Returns [`SaveError::Json`] on parse failure or unknown fields
    /// (the envelope is `deny_unknown_fields`).
    pub fn from_json(s: &str) -> Result<Self, SaveError> {
        serde_json::from_str(s).map_err(SaveError::Json)
    }
}

/// The eight per-subsystem PRNG streams in declared `Stream` enum order
/// (`beast_core::prng::Stream`). Stored as a struct rather than an
/// 8-element `Vec` so a future stream addition (e.g., `rng_climate`)
/// requires a deliberate field bump + migration step rather than a
/// silent index shift.
///
/// **No `#[serde(deny_unknown_fields)]` here** — when a future `Stream`
/// variant adds a field, an old save with the old field set is meant
/// to flow through the migration registry (S7.5), not die at decode
/// with a misleading "missing field" error. The top-level [`SaveFile`]
/// keeps `deny_unknown_fields` because envelope-level surprises (a
/// rogue UI flag, a typo) should fail loud; per-subtype strictness
/// would defeat the migration story. Per PR #135 review.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrngStreams {
    /// Genetic mutation operators.
    pub genetics: Prng,
    /// Phenotype interpreter stochastic tie-breaks.
    pub phenotype: Prng,
    /// Physics & movement jitter.
    pub physics: Prng,
    /// Combat & interaction resolution.
    pub combat: Prng,
    /// Metabolism, ageing, healing.
    pub physiology: Prng,
    /// Ecology: spawning, biome dynamics, random events.
    pub ecology: Prng,
    /// World generation.
    pub worldgen: Prng,
    /// Chronicler pattern clustering.
    pub chronicler: Prng,
}

/// One entity's worth of state, captured as `Option<Component>` per
/// component type. `None` means the entity does not have that
/// component; `Some(_)` means it does and the value is the verbatim
/// component data.
///
/// `id` is the linearised specs entity id (index + generation packed
/// into a `u64`). The save layer does not reuse the original specs
/// `Entity` value on load — it allocates fresh entities and rebuilds
/// the same `(MarkerKind, Entity)` mapping in the
/// [`beast_ecs::SortedEntityIndex`]. Two saves of the same world
/// produce equal `id`s when the entity allocator is deterministic
/// (which `specs::World` is, given the same insert sequence).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SerializedEntity {
    /// Linearised entity id used for stable round-trip ordering.
    pub id: u64,
    /// Marker components present on this entity. Sorted so two equal
    /// entity-states produce byte-identical save output.
    pub markers: Vec<SerializedMarker>,
    /// Position, when present.
    pub position: Option<Position>,
    /// Velocity, when present.
    pub velocity: Option<Velocity>,
    /// Age, when present.
    pub age: Option<Age>,
    /// Mass, when present.
    pub mass: Option<Mass>,
    /// Health/energy, when present.
    pub health: Option<HealthState>,
    /// Developmental stage, when present.
    pub stage: Option<DevelopmentalStage>,
    /// Species id, when present.
    pub species: Option<Species>,
    /// Genome, when present.
    pub genome: Option<GenomeComponent>,
    /// Per-entity opaque scratch data the save layer doesn't yet model.
    /// Reserved for forward-compatible additions in S8+; today this is
    /// always an empty map.
    ///
    /// Typed as `serde_json::Value` so a future caller can stash a
    /// `u64` counter, a boolean flag, or a nested struct without
    /// forcing a breaking format change — the whole point of an
    /// "extras" escape hatch. JSON `Value` round-trips through both
    /// bincode and serde_json without losing structure. Per PR #135
    /// review (was `BTreeMap<String, Q3232>`).
    pub extras: BTreeMap<String, serde_json::Value>,
}

/// Marker-component presence flag. Mirrors `beast_ecs::MarkerKind` but
/// is owned by the save layer so the wire format is decoupled from the
/// in-memory enum's variant order — adding a new marker variant in
/// `beast-ecs` does not silently shift the byte representation here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SerializedMarker {
    /// Macro-scale beast.
    Creature,
    /// Micro-scale organism (disease, parasite, symbiont).
    Pathogen,
    /// NPC / diplomat / caravan leader.
    Agent,
    /// Social grouping of agents/settlements.
    Faction,
    /// Persistent inhabited location.
    Settlement,
    /// Biome cell on the world map.
    Biome,
}

/// Errors produced by [`SaveFile`] (de)serialization. The enum is
/// `non_exhaustive` because S7.4 + S7.5 will add validator and
/// migration variants without breaking downstream `match` arms.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SaveError {
    /// The bincode encoder/decoder reported a failure.
    #[error("bincode (de)serialization failed: {0}")]
    Bincode(String),
    /// The JSON encoder/decoder reported a failure.
    #[error("json (de)serialization failed: {0}")]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use beast_core::prng::{Prng, Stream};
    use beast_core::Q3232;

    fn sample_save() -> SaveFile {
        let master = Prng::from_seed(0xDEAD_BEEF_CAFE_F00D);
        SaveFile {
            format_version: SAVE_FORMAT_VERSION.to_string(),
            current_tick: 42,
            world_seed: 0xDEAD_BEEF_CAFE_F00D,
            rng_streams: PrngStreams {
                genetics: master.split_stream(Stream::Genetics),
                phenotype: master.split_stream(Stream::Phenotype),
                physics: master.split_stream(Stream::Physics),
                combat: master.split_stream(Stream::Combat),
                physiology: master.split_stream(Stream::Physiology),
                ecology: master.split_stream(Stream::Ecology),
                worldgen: master.split_stream(Stream::Worldgen),
                chronicler: master.split_stream(Stream::Chronicler),
            },
            channel_fingerprint: RegistryFingerprint([0u8; 32]),
            primitive_fingerprint: RegistryFingerprint([0u8; 32]),
            entities: vec![
                SerializedEntity {
                    id: 0,
                    markers: vec![SerializedMarker::Creature],
                    position: Some(Position::new(Q3232::from_num(1), Q3232::from_num(2))),
                    velocity: None,
                    age: Some(Age::new(7)),
                    mass: Some(Mass::new(Q3232::from_num(10))),
                    health: Some(HealthState::full()),
                    stage: Some(DevelopmentalStage::Adult),
                    species: Some(Species::new(3)),
                    genome: None,
                    extras: BTreeMap::new(),
                },
                SerializedEntity {
                    id: 1,
                    markers: vec![SerializedMarker::Pathogen],
                    position: None,
                    velocity: None,
                    age: None,
                    mass: Some(Mass::new(Q3232::from_num(0))),
                    health: None,
                    stage: None,
                    species: None,
                    genome: None,
                    extras: BTreeMap::new(),
                },
            ],
        }
    }

    #[test]
    fn bincode_round_trip_is_lossless() {
        let original = sample_save();
        let bytes = original.to_bincode().expect("encode");
        let decoded = SaveFile::from_bincode(&bytes).expect("decode");
        assert_eq!(original, decoded);
    }

    #[test]
    fn json_round_trip_is_lossless() {
        let original = sample_save();
        let s = original.to_json().expect("encode");
        let decoded = SaveFile::from_json(&s).expect("decode");
        assert_eq!(original, decoded);
    }

    #[test]
    fn equal_savefiles_serialize_to_equal_bincode() {
        // Determinism contract: same value -> same byte sequence.
        let a = sample_save();
        let b = sample_save();
        assert_eq!(a.to_bincode().unwrap(), b.to_bincode().unwrap());
    }

    #[test]
    fn equal_savefiles_serialize_to_equal_json() {
        let a = sample_save();
        let b = sample_save();
        assert_eq!(a.to_json().unwrap(), b.to_json().unwrap());
    }

    #[test]
    fn json_rejects_unknown_envelope_field() {
        // `deny_unknown_fields` guards against a v2 file silently
        // round-tripping through a v1 reader (which would drop data).
        // Build a valid envelope, then splice a `future_field` into the
        // top-level object — this avoids hand-writing the Prng serde
        // shape, which is owned by `rand_xoshiro` and not part of the
        // beast-serde contract.
        let mut tampered: serde_json::Value =
            serde_json::from_str(&sample_save().to_json().unwrap()).unwrap();
        tampered["future_field"] = serde_json::json!(1);
        let bad = serde_json::to_string(&tampered).unwrap();

        let err = SaveFile::from_json(&bad).expect_err("expected unknown-field rejection");
        assert!(
            err.to_string().contains("unknown field"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn save_format_version_is_semver_shaped() {
        // Cheap sanity: three dot-separated integers. Migration logic
        // (S7.5) leans on this shape.
        let parts: Vec<&str> = SAVE_FORMAT_VERSION.split('.').collect();
        assert_eq!(parts.len(), 3, "version not semver: {SAVE_FORMAT_VERSION}");
        for p in parts {
            assert!(p.parse::<u32>().is_ok(), "non-numeric version part: {p}");
        }
    }

    #[test]
    fn serialized_marker_is_snake_case_in_json() {
        // Locked in: a marker rename in beast-ecs cannot silently
        // change wire format because we own SerializedMarker here. The
        // snake_case rename keeps JSON keys human-readable (e.g. for
        // hand-edited test fixtures) without needing a custom Visitor.
        let m = SerializedMarker::Creature;
        let s = serde_json::to_string(&m).unwrap();
        assert_eq!(s, "\"creature\"");
    }
}
