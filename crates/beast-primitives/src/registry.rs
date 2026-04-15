//! Primitive registry — single deterministic index over primitive manifests.
//!
//! Like [`beast_channels::ChannelRegistry`], the primitive registry is
//! [`BTreeMap`]-backed so iteration order is stable across runs. Primitives
//! are indexed primarily by id and secondarily by [`PrimitiveCategory`].

use std::collections::{BTreeMap, BTreeSet};

use thiserror::Error;

use crate::category::PrimitiveCategory;
use crate::manifest::PrimitiveManifest;

/// Errors produced by registry mutations.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RegistryError {
    /// A primitive with the same id is already registered.
    #[error("duplicate primitive id: {0}")]
    DuplicateId(String),

    /// A `composition_compatibility.channel_id` entry referenced a channel
    /// that is not registered in the supplied channel registry.
    #[error(
        "primitive `{source_id}` references unknown channel `{target}` in composition_compatibility"
    )]
    UnknownChannel {
        /// The owning primitive.
        source_id: String,
        /// The missing channel id.
        target: String,
    },
}

/// Deterministic in-memory index of primitive manifests.
#[derive(Debug, Clone, Default)]
pub struct PrimitiveRegistry {
    by_id: BTreeMap<String, PrimitiveManifest>,
    by_category: BTreeMap<PrimitiveCategory, BTreeSet<String>>,
}

impl PrimitiveRegistry {
    /// Empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a manifest; fail if the id is already present.
    pub fn register(&mut self, manifest: PrimitiveManifest) -> Result<(), RegistryError> {
        if self.by_id.contains_key(&manifest.id) {
            return Err(RegistryError::DuplicateId(manifest.id));
        }
        self.by_category
            .entry(manifest.category)
            .or_default()
            .insert(manifest.id.clone());
        self.by_id.insert(manifest.id.clone(), manifest);
        Ok(())
    }

    /// Number of registered primitives.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    /// Whether no primitives are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    /// Whether the given id is registered.
    #[must_use]
    pub fn contains(&self, id: &str) -> bool {
        self.by_id.contains_key(id)
    }

    /// Look up a manifest by id.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&PrimitiveManifest> {
        self.by_id.get(id)
    }

    /// Iterate `(id, manifest)` pairs in sorted id order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &PrimitiveManifest)> {
        self.by_id.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Iterate primitive ids in sorted order.
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.by_id.keys().map(String::as_str)
    }

    /// All primitive ids in the given category, in sorted order.
    pub fn ids_by_category(&self, category: PrimitiveCategory) -> impl Iterator<Item = &str> {
        self.by_category
            .get(&category)
            .into_iter()
            .flat_map(|set| set.iter().map(String::as_str))
    }

    /// All primitives in the given category, in sorted id order.
    pub fn by_category(
        &self,
        category: PrimitiveCategory,
    ) -> impl Iterator<Item = &PrimitiveManifest> {
        self.ids_by_category(category)
            .filter_map(move |id| self.by_id.get(id))
    }

    /// Verify that every `composition_compatibility.channel_id` refers to a
    /// channel present in `channels`.
    ///
    /// `ChannelFamily` entries always pass (families are a closed enum).
    pub fn validate_channel_references(
        &self,
        channels: &beast_channels::ChannelRegistry,
    ) -> Result<(), RegistryError> {
        for (id, manifest) in &self.by_id {
            for entry in &manifest.composition_compatibility {
                if let crate::manifest::CompatibilityEntry::ChannelId(target) = entry {
                    if !channels.contains(target) {
                        return Err(RegistryError::UnknownChannel {
                            source_id: id.clone(),
                            target: target.clone(),
                        });
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::category::{Modality, PrimitiveCategory};
    use crate::manifest::{
        CompatibilityEntry, CostFunction, ObservableSignature, PrimitiveManifest, Provenance,
    };
    use beast_core::Q3232;
    use std::collections::BTreeMap;

    fn fixture(id: &str, category: PrimitiveCategory) -> PrimitiveManifest {
        PrimitiveManifest {
            id: id.into(),
            category,
            description: "fixture".into(),
            parameter_schema: BTreeMap::new(),
            composition_compatibility: vec![CompatibilityEntry::ChannelFamily(
                beast_channels::ChannelFamily::Motor,
            )],
            cost_function: CostFunction {
                base_metabolic_cost: Q3232::ONE,
                parameter_scaling: Vec::new(),
            },
            observable_signature: ObservableSignature {
                modality: Modality::Behavioral,
                detection_range_m: Q3232::ONE,
                pattern_key: "fixture_v1".into(),
            },
            provenance: Provenance::Core,
        }
    }

    #[test]
    fn registration_is_unique() {
        let mut reg = PrimitiveRegistry::new();
        reg.register(fixture("a", PrimitiveCategory::ForceApplication))
            .unwrap();
        assert!(matches!(
            reg.register(fixture("a", PrimitiveCategory::SignalEmission)),
            Err(RegistryError::DuplicateId(_))
        ));
    }

    #[test]
    fn iteration_sorted_by_id() {
        let mut reg = PrimitiveRegistry::new();
        for id in ["gamma", "alpha", "beta"] {
            reg.register(fixture(id, PrimitiveCategory::SignalEmission))
                .unwrap();
        }
        let ids: Vec<_> = reg.ids().collect();
        assert_eq!(ids, vec!["alpha", "beta", "gamma"]);
    }

    #[test]
    fn by_category_filter_correct() {
        let mut reg = PrimitiveRegistry::new();
        reg.register(fixture("a", PrimitiveCategory::SignalEmission))
            .unwrap();
        reg.register(fixture("b", PrimitiveCategory::ForceApplication))
            .unwrap();
        reg.register(fixture("c", PrimitiveCategory::SignalEmission))
            .unwrap();
        let ids: Vec<_> = reg
            .ids_by_category(PrimitiveCategory::SignalEmission)
            .collect();
        assert_eq!(ids, vec!["a", "c"]);
    }

    #[test]
    fn unknown_channel_reference_rejected() {
        let mut reg = PrimitiveRegistry::new();
        let mut manifest = fixture("a", PrimitiveCategory::SignalEmission);
        manifest
            .composition_compatibility
            .push(CompatibilityEntry::ChannelId("missing_channel".into()));
        reg.register(manifest).unwrap();

        let channels = beast_channels::ChannelRegistry::new();
        assert!(matches!(
            reg.validate_channel_references(&channels),
            Err(RegistryError::UnknownChannel { .. })
        ));
    }
}
