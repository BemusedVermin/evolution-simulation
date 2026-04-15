//! Channel registry — the single authoritative lookup table.
//!
//! Per `documentation/INVARIANTS.md` Invariant 3 ("Channel Registry
//! Monolithicism"), the simulation holds exactly one [`ChannelRegistry`] at
//! runtime. Core, mod, and genesis-derived channels all register through this
//! surface; nothing downstream hardcodes channel ids. The registry is backed
//! by [`BTreeMap`] so iteration order is deterministic and independent of
//! hash randomization — a requirement for deterministic replay.

use std::collections::{BTreeMap, BTreeSet};

use thiserror::Error;

use crate::manifest::{ChannelFamily, ChannelManifest};

/// Errors produced by registry mutations.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RegistryError {
    /// A channel with the same id is already registered.
    #[error("duplicate channel id: {0}")]
    DuplicateId(String),

    /// A composition hook or correlation entry referenced a channel that is
    /// not (yet) registered. Reported by [`ChannelRegistry::validate_cross_references`].
    #[error("channel `{source_id}` references unknown channel `{target}`")]
    UnknownReference {
        /// The owning channel.
        source_id: String,
        /// The missing reference target.
        target: String,
    },
}

/// An authoritative in-memory index of channel manifests.
///
/// Clone is cheap-ish (shallow: `BTreeMap` clones are linear in key+value
/// pointers). Treat a `ChannelRegistry` as an append-only structure built
/// during world initialization; mutation afterwards (genesis events) is
/// allowed but must go through [`Self::register`] so the secondary indices
/// stay consistent.
#[derive(Debug, Clone, Default)]
pub struct ChannelRegistry {
    by_id: BTreeMap<String, ChannelManifest>,
    by_family: BTreeMap<ChannelFamily, BTreeSet<String>>,
}

impl ChannelRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a manifest, failing if its id is already registered.
    ///
    /// Returns an error *before* inserting, so on error the registry is
    /// unchanged (strong exception safety).
    pub fn register(&mut self, manifest: ChannelManifest) -> Result<(), RegistryError> {
        if self.by_id.contains_key(&manifest.id) {
            return Err(RegistryError::DuplicateId(manifest.id));
        }
        self.by_family
            .entry(manifest.family)
            .or_default()
            .insert(manifest.id.clone());
        self.by_id.insert(manifest.id.clone(), manifest);
        Ok(())
    }

    /// Number of registered channels.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    /// Whether no channels are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    /// Check whether the given id is registered.
    #[must_use]
    pub fn contains(&self, id: &str) -> bool {
        self.by_id.contains_key(id)
    }

    /// Look up a manifest by id.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&ChannelManifest> {
        self.by_id.get(id)
    }

    /// Iterate manifests in `(id, manifest)` order. Iteration is deterministic
    /// because the backing storage is a [`BTreeMap`].
    pub fn iter(&self) -> impl Iterator<Item = (&str, &ChannelManifest)> {
        self.by_id.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Iterate channel ids in sorted order.
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.by_id.keys().map(String::as_str)
    }

    /// All channel ids belonging to `family`, in sorted order.
    pub fn ids_by_family(&self, family: ChannelFamily) -> impl Iterator<Item = &str> {
        self.by_family
            .get(&family)
            .into_iter()
            .flat_map(|set| set.iter().map(String::as_str))
    }

    /// All manifests belonging to `family`, in sorted id order.
    pub fn by_family(&self, family: ChannelFamily) -> impl Iterator<Item = &ChannelManifest> {
        self.ids_by_family(family)
            .filter_map(move |id| self.by_id.get(id))
    }

    /// Verify that every composition hook `with` reference and every
    /// mutation-kernel correlation target exists in this registry.
    ///
    /// The literal `"self"` is always accepted as a hook target
    /// (auto-interaction). Genesis registration ordering can put the parent
    /// channel ahead of its children — call this after all core/mod/genesis
    /// manifests are loaded.
    pub fn validate_cross_references(&self) -> Result<(), RegistryError> {
        for (id, manifest) in &self.by_id {
            for hook in &manifest.composition_hooks {
                if hook.with == "self" {
                    continue;
                }
                if !self.by_id.contains_key(&hook.with) {
                    return Err(RegistryError::UnknownReference {
                        source_id: id.clone(),
                        target: hook.with.clone(),
                    });
                }
            }
            for corr in &manifest.mutation_kernel.correlation_with {
                if !self.by_id.contains_key(&corr.channel) {
                    return Err(RegistryError::UnknownReference {
                        source_id: id.clone(),
                        target: corr.channel.clone(),
                    });
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composition::{CompositionHook, CompositionKind};
    use crate::expression::ExpressionCondition;
    use crate::manifest::{
        BoundsPolicy, ChannelFamily, CorrelationEntry, MutationKernel, Provenance, Range, ScaleBand,
    };
    use beast_core::Q3232;

    fn fixture(id: &str, family: ChannelFamily) -> ChannelManifest {
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
            provenance: Provenance::Core,
        }
    }

    #[test]
    fn registration_is_unique() {
        let mut reg = ChannelRegistry::new();
        reg.register(fixture("a", ChannelFamily::Sensory)).unwrap();
        let err = reg
            .register(fixture("a", ChannelFamily::Motor))
            .unwrap_err();
        assert!(matches!(err, RegistryError::DuplicateId(_)));
    }

    #[test]
    fn iteration_is_sorted() {
        let mut reg = ChannelRegistry::new();
        for id in ["charlie", "alpha", "bravo"] {
            reg.register(fixture(id, ChannelFamily::Sensory)).unwrap();
        }
        let ids: Vec<_> = reg.ids().collect();
        assert_eq!(ids, vec!["alpha", "bravo", "charlie"]);
    }

    #[test]
    fn by_family_filters_correctly() {
        let mut reg = ChannelRegistry::new();
        reg.register(fixture("s1", ChannelFamily::Sensory)).unwrap();
        reg.register(fixture("s2", ChannelFamily::Sensory)).unwrap();
        reg.register(fixture("m1", ChannelFamily::Motor)).unwrap();
        let sensory: Vec<_> = reg.ids_by_family(ChannelFamily::Sensory).collect();
        assert_eq!(sensory, vec!["s1", "s2"]);
        let motor: Vec<_> = reg.ids_by_family(ChannelFamily::Motor).collect();
        assert_eq!(motor, vec!["m1"]);
    }

    #[test]
    fn cross_reference_validation_catches_unknown_hooks() {
        let mut reg = ChannelRegistry::new();
        let mut owner = fixture("owner", ChannelFamily::Sensory);
        owner.composition_hooks.push(CompositionHook {
            with: "ghost".into(),
            kind: CompositionKind::Additive,
            coefficient: Q3232::ONE,
            threshold: None,
        });
        reg.register(owner).unwrap();
        assert!(matches!(
            reg.validate_cross_references(),
            Err(RegistryError::UnknownReference { .. })
        ));
    }

    #[test]
    fn cross_reference_validation_accepts_self() {
        let mut reg = ChannelRegistry::new();
        let mut owner = fixture("owner", ChannelFamily::Sensory);
        owner.composition_hooks.push(CompositionHook {
            with: "self".into(),
            kind: CompositionKind::Multiplicative,
            coefficient: Q3232::ONE,
            threshold: None,
        });
        reg.register(owner).unwrap();
        assert!(reg.validate_cross_references().is_ok());
    }

    #[test]
    fn cross_reference_validation_catches_unknown_correlations() {
        let mut reg = ChannelRegistry::new();
        let mut owner = fixture("owner", ChannelFamily::Sensory);
        owner
            .mutation_kernel
            .correlation_with
            .push(CorrelationEntry {
                channel: "missing".into(),
                coefficient: Q3232::from_num(0.5_f64),
            });
        reg.register(owner).unwrap();
        assert!(matches!(
            reg.validate_cross_references(),
            Err(RegistryError::UnknownReference { .. })
        ));
    }
}
