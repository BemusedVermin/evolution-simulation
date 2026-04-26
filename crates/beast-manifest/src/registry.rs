//! Deterministic, [`BTreeMap`]-backed registry helpers.
//!
//! Channel and primitive registries each expose a primary lookup by id and
//! a secondary index by a domain-specific grouping key (channel family /
//! primitive category). The shared [`SortedRegistry`] captures both indices
//! with iteration that is deterministic by construction — a load-bearing
//! invariant for replay (`documentation/INVARIANTS.md` §1).
//!
//! Domain crates wrap [`SortedRegistry`] rather than re-export it: they keep
//! their own public type name and add semantic-validation methods that
//! consult other registries (e.g. primitives validating channel references).

use std::collections::{BTreeMap, BTreeSet};

use thiserror::Error;

/// Error returned when [`SortedRegistry::insert`] sees an id already present.
///
/// Domain crates wrap this in their own `RegistryError` enum so the public
/// API can carry additional variants (unknown-reference checks, etc.).
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("duplicate id: {0}")]
pub struct DuplicateId(pub String);

/// Trait marking a value that can be stored in [`SortedRegistry`].
///
/// Implementors expose a unique id and a grouping key used for the
/// secondary index. [`Self::Group`] must be `Ord + Copy` so the registry
/// can BTreeMap-index it.
///
/// `Clone` is a supertrait because [`SortedRegistry`] derives `Clone`
/// and stores `M` by value in its internal `BTreeMap`. Without
/// `Clone` here, `SortedRegistry::clone()` would fail at the call
/// site rather than at the trait impl, which is a confusing
/// diagnostic.
///
/// Current implementors: `beast_channels::ChannelManifest` and
/// `beast_primitives::PrimitiveManifest` (both already derive
/// `Clone`).
pub trait Manifest: Clone {
    /// Type of the grouping key (e.g. `ChannelFamily`, `PrimitiveCategory`).
    type Group: Ord + Copy;

    /// Unique snake_case id used for the primary index.
    fn id(&self) -> &str;

    /// Grouping key used for the secondary index.
    fn group(&self) -> Self::Group;
}

/// A [`BTreeMap`]-backed registry with a primary `id → manifest` index and
/// a secondary `group → ids` index.
///
/// * Iteration is deterministic because `BTreeMap` iterates in sort order
///   of keys — iteration never depends on hash randomisation.
/// * [`Self::insert`] is strongly exception-safe: on error the registry is
///   unchanged.
/// * Clone is cheap-ish (shallow; scales linearly with count).
#[derive(Debug, Clone)]
pub struct SortedRegistry<M: Manifest> {
    by_id: BTreeMap<String, M>,
    by_group: BTreeMap<M::Group, BTreeSet<String>>,
}

impl<M: Manifest> Default for SortedRegistry<M> {
    fn default() -> Self {
        Self {
            by_id: BTreeMap::new(),
            by_group: BTreeMap::new(),
        }
    }
}

impl<M: Manifest> SortedRegistry<M> {
    /// An empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a manifest, failing if its id is already registered.
    ///
    /// On [`DuplicateId`], the registry is unchanged.
    pub fn insert(&mut self, manifest: M) -> Result<(), DuplicateId> {
        if self.by_id.contains_key(manifest.id()) {
            return Err(DuplicateId(manifest.id().to_owned()));
        }
        let id = manifest.id().to_owned();
        let group = manifest.group();
        self.by_group.entry(group).or_default().insert(id.clone());
        self.by_id.insert(id, manifest);
        Ok(())
    }

    /// Number of registered manifests.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    /// Whether no manifests are registered.
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
    pub fn get(&self, id: &str) -> Option<&M> {
        self.by_id.get(id)
    }

    /// Iterate `(id, manifest)` pairs in sorted id order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &M)> {
        self.by_id.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Iterate manifest ids in sorted order.
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.by_id.keys().map(String::as_str)
    }

    /// Iterate ids in the given group, in sorted order. Returns an empty
    /// iterator if the group has no members.
    pub fn ids_by_group(&self, group: M::Group) -> impl Iterator<Item = &str> {
        self.by_group
            .get(&group)
            .into_iter()
            .flat_map(|set| set.iter().map(String::as_str))
    }

    /// Iterate manifests in the given group, in sorted id order.
    pub fn by_group(&self, group: M::Group) -> impl Iterator<Item = &M> {
        self.ids_by_group(group)
            .filter_map(move |id| self.by_id.get(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    enum Color {
        Red,
        Blue,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct Item {
        id: String,
        color: Color,
    }

    impl Manifest for Item {
        type Group = Color;
        fn id(&self) -> &str {
            &self.id
        }
        fn group(&self) -> Color {
            self.color
        }
    }

    fn item(id: &str, color: Color) -> Item {
        Item {
            id: id.to_owned(),
            color,
        }
    }

    #[test]
    fn insert_is_unique() {
        let mut reg: SortedRegistry<Item> = SortedRegistry::new();
        reg.insert(item("a", Color::Red)).unwrap();
        let err = reg.insert(item("a", Color::Blue)).unwrap_err();
        assert_eq!(err.0, "a");
        // Registry is unchanged after error.
        assert_eq!(reg.len(), 1);
        assert_eq!(reg.get("a").unwrap().color, Color::Red);
    }

    #[test]
    fn iteration_sorted_by_id() {
        let mut reg: SortedRegistry<Item> = SortedRegistry::new();
        for id in ["charlie", "alpha", "bravo"] {
            reg.insert(item(id, Color::Red)).unwrap();
        }
        let ids: Vec<_> = reg.ids().collect();
        assert_eq!(ids, vec!["alpha", "bravo", "charlie"]);
    }

    #[test]
    fn by_group_returns_sorted_members() {
        let mut reg: SortedRegistry<Item> = SortedRegistry::new();
        reg.insert(item("b2", Color::Blue)).unwrap();
        reg.insert(item("r1", Color::Red)).unwrap();
        reg.insert(item("b1", Color::Blue)).unwrap();
        let blue: Vec<_> = reg.ids_by_group(Color::Blue).collect();
        assert_eq!(blue, vec!["b1", "b2"]);
        let red: Vec<_> = reg.ids_by_group(Color::Red).collect();
        assert_eq!(red, vec!["r1"]);
    }

    #[test]
    fn by_group_for_empty_group_is_empty() {
        let reg: SortedRegistry<Item> = SortedRegistry::new();
        let empty: Vec<_> = reg.ids_by_group(Color::Red).collect();
        assert!(empty.is_empty());
    }

    #[test]
    fn contains_and_get() {
        let mut reg: SortedRegistry<Item> = SortedRegistry::new();
        reg.insert(item("x", Color::Red)).unwrap();
        assert!(reg.contains("x"));
        assert!(!reg.contains("y"));
        assert_eq!(reg.get("x").unwrap().id, "x");
        assert!(reg.get("y").is_none());
    }

    #[test]
    fn iter_yields_pairs() {
        let mut reg: SortedRegistry<Item> = SortedRegistry::new();
        reg.insert(item("a", Color::Red)).unwrap();
        reg.insert(item("b", Color::Blue)).unwrap();
        let pairs: Vec<_> = reg.iter().map(|(id, m)| (id, m.color)).collect();
        assert_eq!(pairs, vec![("a", Color::Red), ("b", Color::Blue)]);
    }

    #[test]
    fn default_is_empty() {
        let reg: SortedRegistry<Item> = SortedRegistry::default();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
    }
}
