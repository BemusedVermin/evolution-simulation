//! Channel-registry identity fingerprint.
//!
//! The fingerprint is a BLAKE3 digest over the *semantic identity* of the
//! registered channels: id, family tag, and provenance. It is intentionally
//! **not** keyed by tuning parameters (sigma, composition hooks, scale bands,
//! numeric bounds) because those are expected to drift between mod versions
//! without invalidating existing saves. What the fingerprint catches is the
//! dangerous case: a new channel is inserted into the vocabulary, or an
//! existing channel is renamed/removed, which would silently reshuffle the
//! positional `EffectVector.channel` indexing that saved genomes depend on
//! (see `documentation/INVARIANTS.md` §1 "Determinism" and §3 "Channel
//! Registry Monolithicism").
//!
//! ## Hash input format (CRF1)
//!
//! ```text
//! "CRF1"                              (4 bytes, ASCII magic)
//! u32 LE — count                       (4 bytes)
//! for each channel (iterated in sorted id order via BTreeMap):
//!     u32 LE — len(id)                 (4 bytes)
//!     id bytes                         (len bytes, UTF-8)
//!     u32 LE — len(family_tag)         (4 bytes)
//!     family_tag bytes                 (len bytes, ASCII)
//!     u32 LE — len(provenance_str)     (4 bytes)
//!     provenance_str bytes             (len bytes, canonical schema form)
//! ```
//!
//! The length prefix before each string prevents ambiguity attacks like
//! `"ab" + "c"` hashing to the same bytes as `"a" + "bc"`. If the mapping
//! between [`ChannelFamily`] and its ASCII tag ever changes intentionally,
//! bump the magic (`CRF1` → `CRF2`) so older saves are rejected cleanly
//! rather than silently misvalidating.

use crate::manifest::{ChannelFamily, ChannelManifest};
use crate::registry::ChannelRegistry;

/// Magic bytes identifying version 1 of the fingerprint format.
const FINGERPRINT_MAGIC: &[u8; 4] = b"CRF1";

/// A 256-bit BLAKE3 digest identifying a channel-registry vocabulary.
///
/// Two registries that register the same channel ids (each with the same
/// family and provenance) in **any** insertion order will produce equal
/// fingerprints — iteration is sorted by id via the backing `BTreeMap`.
/// Tuning changes (sigma, composition hooks, ranges) deliberately do **not**
/// affect the fingerprint.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct RegistryFingerprint(pub [u8; 32]);

impl RegistryFingerprint {
    /// Render the fingerprint as a lowercase hex string (64 characters).
    ///
    /// Used in error messages so the fingerprint survives copy/paste through
    /// logs and bug reports.
    #[must_use]
    pub fn to_hex(&self) -> String {
        let mut out = String::with_capacity(64);
        for byte in &self.0 {
            use core::fmt::Write as _;
            let _ = write!(out, "{byte:02x}");
        }
        out
    }
}

/// Map a [`ChannelFamily`] to its stable ASCII tag.
///
/// This is intentionally decoupled from serde's `rename_all = "snake_case"`
/// so refactoring the serde rename pattern cannot silently shift save-file
/// compatibility. If this mapping is ever changed deliberately, bump the
/// fingerprint version magic.
const fn family_tag(family: ChannelFamily) -> &'static str {
    match family {
        ChannelFamily::Sensory => "sensory",
        ChannelFamily::Motor => "motor",
        ChannelFamily::Metabolic => "metabolic",
        ChannelFamily::Structural => "structural",
        ChannelFamily::Regulatory => "regulatory",
        ChannelFamily::Social => "social",
        ChannelFamily::Cognitive => "cognitive",
        ChannelFamily::Reproductive => "reproductive",
        ChannelFamily::Developmental => "developmental",
    }
}

fn write_len_prefixed(hasher: &mut blake3::Hasher, bytes: &[u8]) {
    let len = u32::try_from(bytes.len()).expect("channel metadata length exceeds u32::MAX");
    hasher.update(&len.to_le_bytes());
    hasher.update(bytes);
}

fn hash_manifest(hasher: &mut blake3::Hasher, manifest: &ChannelManifest) {
    write_len_prefixed(hasher, manifest.id.as_bytes());
    write_len_prefixed(hasher, family_tag(manifest.family).as_bytes());
    let provenance = manifest.provenance.to_schema_string();
    write_len_prefixed(hasher, provenance.as_bytes());
}

impl ChannelRegistry {
    /// Compute the identity fingerprint for this registry.
    ///
    /// The result is stable across:
    ///
    /// * insertion order (the backing `BTreeMap` sorts by id),
    /// * tuning changes (sigma, composition hooks, range bounds, scale bands).
    ///
    /// The result **does** change when a channel is added, removed, renamed,
    /// re-familied, or gets a different provenance string.
    ///
    /// See the module-level documentation for the exact byte layout.
    #[must_use]
    pub fn fingerprint(&self) -> RegistryFingerprint {
        let mut hasher = blake3::Hasher::new();
        hasher.update(FINGERPRINT_MAGIC);
        let count = u32::try_from(self.len()).expect("registry size exceeds u32::MAX");
        hasher.update(&count.to_le_bytes());
        for (_id, manifest) in self.iter() {
            hash_manifest(&mut hasher, manifest);
        }
        RegistryFingerprint(*hasher.finalize().as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composition::{CompositionHook, CompositionKind};
    use crate::expression::ExpressionCondition;
    use crate::manifest::{
        BoundsPolicy, ChannelFamily, MutationKernel, Provenance, Range, ScaleBand,
    };
    use beast_core::Q3232;

    fn fixture(id: &str, family: ChannelFamily, provenance: Provenance) -> ChannelManifest {
        ChannelManifest {
            id: id.into(),
            family,
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
            expression_conditions: Vec::<ExpressionCondition>::new(),
            scale_band: ScaleBand {
                min_kg: Q3232::ZERO,
                max_kg: Q3232::from_num(1000_i32),
            },
            body_site_applicable: false,
            provenance,
        }
    }

    fn build(channels: &[(&str, ChannelFamily)]) -> ChannelRegistry {
        let mut reg = ChannelRegistry::new();
        for (id, family) in channels {
            reg.register(fixture(id, *family, Provenance::Core)).unwrap();
        }
        reg
    }

    #[test]
    fn hex_is_lowercase_hex_of_64_chars() {
        let reg = build(&[("alpha", ChannelFamily::Sensory)]);
        let hex = reg.fingerprint().to_hex();
        assert_eq!(hex.len(), 64);
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()));
    }

    #[test]
    fn hooks_do_not_affect_fingerprint() {
        // Two registries with identical (id, family, provenance) but different
        // composition hooks and mutation kernels must produce the same hash.
        let plain = build(&[("alpha", ChannelFamily::Sensory)]);
        let mut fancy = ChannelRegistry::new();
        let mut adorned = fixture("alpha", ChannelFamily::Sensory, Provenance::Core);
        adorned.composition_hooks.push(CompositionHook {
            with: "self".into(),
            kind: CompositionKind::Multiplicative,
            coefficient: Q3232::from_num(0.5_f64),
            threshold: None,
        });
        adorned.mutation_kernel.sigma = Q3232::from_num(0.42_f64);
        fancy.register(adorned).unwrap();

        assert_eq!(plain.fingerprint(), fancy.fingerprint());
    }

    #[test]
    fn insertion_order_does_not_affect_fingerprint() {
        let a = build(&[
            ("alpha", ChannelFamily::Sensory),
            ("bravo", ChannelFamily::Motor),
            ("charlie", ChannelFamily::Sensory),
        ]);
        let b = build(&[
            ("charlie", ChannelFamily::Sensory),
            ("alpha", ChannelFamily::Sensory),
            ("bravo", ChannelFamily::Motor),
        ]);
        assert_eq!(a.fingerprint(), b.fingerprint());
    }

    #[test]
    fn different_id_changes_fingerprint() {
        let a = build(&[("alpha", ChannelFamily::Sensory)]);
        let b = build(&[("beta", ChannelFamily::Sensory)]);
        assert_ne!(a.fingerprint(), b.fingerprint());
    }

    #[test]
    fn different_family_changes_fingerprint() {
        let a = build(&[("alpha", ChannelFamily::Sensory)]);
        let b = build(&[("alpha", ChannelFamily::Motor)]);
        assert_ne!(a.fingerprint(), b.fingerprint());
    }

    #[test]
    fn different_provenance_changes_fingerprint() {
        let mut a = ChannelRegistry::new();
        a.register(fixture("alpha", ChannelFamily::Sensory, Provenance::Core))
            .unwrap();
        let mut b = ChannelRegistry::new();
        b.register(fixture(
            "alpha",
            ChannelFamily::Sensory,
            Provenance::Mod("acme".into()),
        ))
        .unwrap();
        assert_ne!(a.fingerprint(), b.fingerprint());
    }

    #[test]
    fn empty_registry_has_stable_fingerprint() {
        let a = ChannelRegistry::new();
        let b = ChannelRegistry::new();
        assert_eq!(a.fingerprint(), b.fingerprint());
    }

    #[test]
    fn length_prefixing_avoids_concatenation_collisions() {
        // "ab"+"c" vs "a"+"bc" — length prefixes keep these separate.
        let a = build(&[("ab", ChannelFamily::Sensory), ("c", ChannelFamily::Sensory)]);
        let b = build(&[("a", ChannelFamily::Sensory), ("bc", ChannelFamily::Sensory)]);
        assert_ne!(a.fingerprint(), b.fingerprint());
    }
}
