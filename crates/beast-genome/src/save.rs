//! Genome persistence with registry-fingerprint guard.
//!
//! Saved genomes encode channel contributions positionally (see
//! [`crate::channel_vector::ChannelVector`]). If the channel registry's
//! vocabulary or sort order changes between save and load, the positional
//! index silently remaps — inhibitory contributions can turn sensory, motor
//! outputs can land on metabolic channels, and the save loads "successfully"
//! while producing garbage phenotypes.
//!
//! This module closes that hole with a two-field envelope:
//!
//! ```text
//! GenomeSave {
//!     version: u32 = SAVE_FORMAT_VERSION,
//!     registry_fingerprint: RegistryFingerprint,  // 32-byte BLAKE3
//!     genome: Genome,
//! }
//! ```
//!
//! [`save_genome_to_json`] snapshots the current registry's fingerprint into
//! the envelope, and [`load_genome_from_json`] refuses to hand back a
//! [`Genome`] unless the fingerprint matches the active registry. A version
//! mismatch takes precedence over a fingerprint mismatch, so that future
//! envelope layouts (e.g. a `v2` file that relocates the fingerprint field
//! or omits it entirely) produce a clean "unsupported version" error rather
//! than a spurious "registry mismatch" on a comparison that would have been
//! meaningless.

use beast_channels::{ChannelRegistry, RegistryFingerprint};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::error::GenomeError;
use crate::genome::Genome;

/// On-disk save-envelope version. Bump whenever the envelope layout changes
/// in a way that isn't backward-compatible at the serde level.
///
/// The current layout pins an explicit `version: 1` alongside a 32-byte
/// [`RegistryFingerprint`] and the `genome` payload.
pub const SAVE_FORMAT_VERSION: u32 = 1;

/// The on-disk envelope around a saved [`Genome`].
///
/// `#[serde(deny_unknown_fields)]` ensures a future v2 file that adds new
/// fields will be rejected by v1 readers rather than silently dropping
/// them — a conservative choice appropriate for a deterministic sim.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenomeSave {
    /// Envelope format version. Must equal [`SAVE_FORMAT_VERSION`].
    pub version: u32,
    /// Fingerprint of the registry that indexed the genome at save time.
    pub registry_fingerprint: RegistryFingerprint,
    /// The saved genome.
    pub genome: Genome,
}

/// Errors produced by [`save_genome_to_json`] and [`load_genome_from_json`].
#[derive(Debug, Error)]
pub enum SaveLoadError {
    /// `serde_json` failed to encode or decode the envelope.
    #[error("json (de)serialization failed: {0}")]
    Json(#[from] serde_json::Error),

    /// The envelope's `version` field did not match [`SAVE_FORMAT_VERSION`].
    #[error("unsupported save-format version: expected {expected}, found {found}")]
    UnsupportedVersion {
        /// Version this build understands.
        expected: u32,
        /// Version reported in the envelope.
        found: u32,
    },

    /// The envelope's fingerprint did not match the active registry.
    ///
    /// The hex-formatted fingerprints are included so the mismatch is
    /// diagnosable from a log paste without needing the original file.
    #[error(
        "registry fingerprint mismatch: save was written against registry \
         {saved_fingerprint}, current registry is {current_fingerprint}"
    )]
    RegistryMismatch {
        /// Hex of the registry the save was written against.
        saved_fingerprint: String,
        /// Hex of the registry the simulation is currently using.
        current_fingerprint: String,
    },

    /// The genome payload passed deserialization but failed structural
    /// validation — modifier index out of bounds, duplicate lineage tags,
    /// channel-count mismatch across genes, etc.
    #[error("genome validation failed after load: {0}")]
    InvalidGenome(#[from] GenomeError),
}

/// Serialize a genome and the current registry's fingerprint to JSON.
pub fn save_genome_to_json(
    genome: &Genome,
    registry: &ChannelRegistry,
) -> Result<String, SaveLoadError> {
    let envelope = GenomeSave {
        version: SAVE_FORMAT_VERSION,
        registry_fingerprint: registry.fingerprint(),
        genome: genome.clone(),
    };
    Ok(serde_json::to_string(&envelope)?)
}

/// Parse a JSON save envelope and return the genome if it matches `registry`.
///
/// Checks run in this order; the first mismatch short-circuits:
///
/// 1. JSON parse.
/// 2. `version == SAVE_FORMAT_VERSION`.
/// 3. Envelope fingerprint equals current registry fingerprint.
/// 4. `genome.validate()` passes (defense-in-depth — the fingerprint only
///    guarantees the vocabulary matches, not that intra-genome invariants
///    are intact after third-party edits).
pub fn load_genome_from_json(
    json: &str,
    registry: &ChannelRegistry,
) -> Result<Genome, SaveLoadError> {
    let envelope: GenomeSave = serde_json::from_str(json)?;

    if envelope.version != SAVE_FORMAT_VERSION {
        return Err(SaveLoadError::UnsupportedVersion {
            expected: SAVE_FORMAT_VERSION,
            found: envelope.version,
        });
    }

    let current = registry.fingerprint();
    if current != envelope.registry_fingerprint {
        return Err(SaveLoadError::RegistryMismatch {
            saved_fingerprint: envelope.registry_fingerprint.to_hex(),
            current_fingerprint: current.to_hex(),
        });
    }

    envelope.genome.validate()?;
    Ok(envelope.genome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body_site::BodyVector;
    use crate::gene::{EffectVector, Target, Timing, TraitGene};
    use crate::genome::GenomeParams;
    use crate::lineage::LineageTag;
    use beast_channels::{
        BoundsPolicy, ChannelFamily, ChannelManifest, MutationKernel, Provenance, Range, ScaleBand,
    };
    use beast_core::Q3232;

    fn channel(id: &str) -> ChannelManifest {
        ChannelManifest {
            id: id.into(),
            family: ChannelFamily::Sensory,
            description: "fixture".into(),
            range: Range {
                min: Q3232::ZERO,
                max: Q3232::ONE,
                units: "dimensionless".into(),
            },
            mutation_kernel: MutationKernel {
                sigma: Q3232::from_num(0.1_f64),
                bounds_policy: BoundsPolicy::Clamp,
                genesis_weight: Q3232::ONE,
                correlation_with: Vec::new(),
            },
            composition_hooks: Vec::new(),
            expression_conditions: Vec::new(),
            scale_band: ScaleBand {
                min_kg: Q3232::ZERO,
                max_kg: Q3232::from_num(1000_i32),
            },
            body_site_applicable: false,
            provenance: Provenance::Core,
        }
    }

    fn registry_with(ids: &[&str]) -> ChannelRegistry {
        let mut r = ChannelRegistry::new();
        for id in ids {
            r.register(channel(id)).unwrap();
        }
        r
    }

    fn gene(tag: u64, channel_count: usize) -> TraitGene {
        TraitGene::new(
            "alpha",
            EffectVector::new(
                vec![Q3232::from_num(0.25_f64); channel_count],
                Q3232::from_num(0.5_f64),
                Q3232::from_num(0.25_f64),
                Timing::Passive,
                Target::SelfEntity,
            )
            .unwrap(),
            BodyVector::default_internal(),
            vec![],
            true,
            LineageTag::from_raw(tag),
            Provenance::Core,
        )
        .unwrap()
    }

    fn genome_with(channel_count: usize) -> Genome {
        Genome::new(
            GenomeParams::default(),
            vec![gene(1, channel_count), gene(2, channel_count)],
        )
        .unwrap()
    }

    #[test]
    fn round_trips_through_same_registry() {
        let registry = registry_with(&["alpha", "bravo", "charlie"]);
        let genome = genome_with(registry.len());
        let json = save_genome_to_json(&genome, &registry).unwrap();
        let back = load_genome_from_json(&json, &registry).unwrap();
        assert_eq!(back, genome);
    }

    #[test]
    fn round_trips_through_reordered_insertion_into_same_ids() {
        // Sort order is what matters, not insertion order — BTreeMap makes the
        // registry iteration deterministic by id. A registry with identical
        // ids inserted in a different order must accept the saved genome.
        let saver = registry_with(&["alpha", "bravo", "charlie"]);
        let loader = registry_with(&["charlie", "alpha", "bravo"]);
        let genome = genome_with(saver.len());
        let json = save_genome_to_json(&genome, &saver).unwrap();
        let back = load_genome_from_json(&json, &loader).unwrap();
        assert_eq!(back, genome);
    }

    #[test]
    fn rejects_when_channel_added_at_start_of_registry() {
        // Canonical acceptance test for issue #79: a new channel is added to
        // the registry after a save was written. The new channel sorts ahead
        // of the originals (id "aardvark"), so every positional index in the
        // saved genome is now off by one. The fingerprint mismatch must be
        // caught at load time.
        let saver = registry_with(&["alpha", "bravo", "charlie"]);
        let genome = genome_with(saver.len());
        let json = save_genome_to_json(&genome, &saver).unwrap();

        let mut loader = saver.clone();
        loader.register(channel("aardvark")).unwrap();

        let err = load_genome_from_json(&json, &loader).unwrap_err();
        assert!(
            matches!(err, SaveLoadError::RegistryMismatch { .. }),
            "expected RegistryMismatch, got {err:?}"
        );
    }

    #[test]
    fn rejects_when_channel_renamed() {
        let saver = registry_with(&["alpha", "bravo", "charlie"]);
        let genome = genome_with(saver.len());
        let json = save_genome_to_json(&genome, &saver).unwrap();

        let loader = registry_with(&["alpha", "bravo", "delta"]);
        let err = load_genome_from_json(&json, &loader).unwrap_err();
        assert!(matches!(err, SaveLoadError::RegistryMismatch { .. }));
    }

    #[test]
    fn rejects_unsupported_version_before_fingerprint_check() {
        // A future save format must fail fast with UnsupportedVersion rather
        // than be compared on a fingerprint field whose meaning may have
        // changed. We simulate by hand-crafting a "v2" envelope.
        let registry = registry_with(&["alpha", "bravo"]);
        let zero_fp = serde_json::to_value(RegistryFingerprint([0u8; 32])).unwrap();
        let envelope = serde_json::json!({
            "version": 2,
            "registry_fingerprint": zero_fp,
            "genome": genome_with(registry.len()),
        });
        let json = envelope.to_string();
        let err = load_genome_from_json(&json, &registry).unwrap_err();
        assert!(matches!(
            err,
            SaveLoadError::UnsupportedVersion {
                expected: 1,
                found: 2
            }
        ));
    }

    #[test]
    fn rejects_malformed_json() {
        let registry = registry_with(&["alpha"]);
        let err = load_genome_from_json("{not json", &registry).unwrap_err();
        assert!(matches!(err, SaveLoadError::Json(_)));
    }

    #[test]
    fn envelope_is_denormalized_on_unknown_fields() {
        // A v1 reader must reject a file with unknown top-level fields,
        // rather than silently dropping them and proceeding.
        let registry = registry_with(&["alpha"]);
        let fingerprint = registry.fingerprint();
        let fp_json = serde_json::to_value(fingerprint).unwrap();
        let envelope = serde_json::json!({
            "version": SAVE_FORMAT_VERSION,
            "registry_fingerprint": fp_json,
            "genome": genome_with(registry.len()),
            "unexpected_future_field": true,
        });
        let json = envelope.to_string();
        let err = load_genome_from_json(&json, &registry).unwrap_err();
        assert!(matches!(err, SaveLoadError::Json(_)));
    }
}
