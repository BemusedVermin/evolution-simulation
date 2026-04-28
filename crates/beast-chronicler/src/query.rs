//! Read-only query surface for the UI layer (S10.7).
//!
//! Per `documentation/INVARIANTS.md` §6 the UI never mutates sim state and
//! never reads primitive-level data. [`ChroniclerQuery`] is the only
//! sanctioned channel: every method takes `&self` and returns owned data
//! or borrows tied to `&self`. There is no mutating method on the trait.
//!
//! Per `documentation/systems/09_world_history_lore.md` §16, the UI
//! consumes high-level catalogs (bestiary entries, labels) — it must not
//! reach into [`PatternObservation`] directly. The trait shape here is the
//! S10.7 MVP slice of the broader design in §16; the richer surface
//! (factions, lineages, event feed) lands in S12+.
//!
//! Per `documentation/INVARIANTS.md` §6, `bestiary_discovered` is a *UI*
//! flag, not sim state. [`BestiaryEntry`] therefore deliberately omits a
//! `discovered` field: the UI computes it from `observation_count >= 1`.
//! [`BestiaryFilter::discovered_only`] applies the same rule on the query
//! side.

use std::collections::{BTreeMap, BTreeSet};

use beast_core::{EntityId, TickCounter, Q3232};
use serde::{Deserialize, Serialize};

use crate::label::Label;
use crate::pattern::PatternSignature;

/// Manifest-defined label id. Same string space as [`Label::id`]; a type
/// alias keeps API call sites self-documenting without forcing a newtype
/// conversion through every existing label site.
pub type LabelId = String;

/// Opaque species identifier used by the bestiary catalog.
///
/// Defined here (rather than imported from `beast-sim`) because
/// `beast-chronicler` sits at L4 and cannot depend on simulation-layer
/// crates. Higher-layer code is expected to bridge its own species id
/// type into [`SpeciesId`] when it registers entities — see
/// [`Chronicler::set_entity_species`](crate::Chronicler::set_entity_species).
#[derive(
    Copy, Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(transparent)]
#[repr(transparent)]
pub struct SpeciesId(pub u32);

impl SpeciesId {
    /// Construct from a raw `u32`.
    #[inline]
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// The underlying `u32`.
    #[inline]
    pub const fn raw(self) -> u32 {
        self.0
    }
}

/// Sort orderings for [`BestiaryFilter`].
///
/// All variants produce a *total* order on [`BestiaryEntry`]: when the
/// primary key ties, the species id breaks the tie ascending. Without the
/// secondary key, equal primary values would let `BTreeMap` insertion
/// order leak into the rendered list — which violates determinism on the
/// query surface (INVARIANTS §1).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum BestiarySortBy {
    /// Confidence descending (most-confident first); ties broken by
    /// `species_id` ascending. Default for the bestiary screen.
    #[default]
    Confidence,
    /// Earliest first-tick first; ties broken by `species_id` ascending.
    FirstTick,
    /// Highest observation count first; ties broken by `species_id`
    /// ascending.
    Observations,
}

/// Filter parameters for [`ChroniclerQuery::bestiary_entries`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BestiaryFilter {
    /// If `true`, drop entries with `observation_count == 0`. Per
    /// INVARIANTS §6 the UI is the canonical place that rephrases this
    /// as `bestiary_discovered`; the query side simply enforces the
    /// underlying numeric rule.
    pub discovered_only: bool,
    /// Sort order applied after filtering.
    pub sort_by: BestiarySortBy,
    /// Optional case-insensitive substring filter applied against
    /// [`BestiaryEntry::label_ids`]. `None` means no text filter.
    pub search: Option<String>,
}

impl BestiaryFilter {
    /// All-defaults filter: include everything, sort by confidence desc.
    pub fn new() -> Self {
        Self::default()
    }
}

/// Per-species summary surfaced on the bestiary screen.
///
/// Contains only *aggregate* sim-side state plus derivable fields. Notably
/// **no `discovered` field** — that flag is derived at the UI layer per
/// INVARIANTS §6.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BestiaryEntry {
    /// Species this entry summarises.
    pub species: SpeciesId,
    /// All distinct label ids assigned to any signature this species has
    /// emitted, sorted ascending. Sorted-ness is required for
    /// determinism of the rendered bestiary list (INVARIANTS §1).
    pub label_ids: Vec<LabelId>,
    /// Total snapshot ingestions for entities of this species. Always
    /// non-negative. Driven by [`Chronicler::ingest`](crate::Chronicler::ingest)
    /// + the `entity → species` registration.
    pub observation_count: u64,
    /// Highest confidence across all assigned labels for this species.
    /// `Q3232::ZERO` when no label has been assigned yet.
    pub confidence: Q3232,
    /// Earliest tick on which any signature emitted by this species was
    /// observed. `TickCounter::ZERO` when there are no observations.
    pub first_tick: TickCounter,
}

/// Read-only query surface consumed by the UI layer.
///
/// Per INVARIANTS §6 every method is `&self`. There is intentionally no
/// `&mut self` method on this trait; mutation flows through
/// [`Chronicler::ingest`](crate::Chronicler::ingest) /
/// [`Chronicler::assign_labels`](crate::Chronicler::assign_labels) /
/// [`Chronicler::set_entity_species`](crate::Chronicler::set_entity_species),
/// none of which the UI is allowed to call.
pub trait ChroniclerQuery {
    /// Look up the assigned label for a single pattern signature. Returns
    /// `None` if the chronicler has not assigned a label to that
    /// signature (manifest miss, below `min_confidence`, or labels have
    /// not been computed yet).
    fn label_for_signature(&self, sig: &PatternSignature) -> Option<&Label>;

    /// Every label assigned to any signature the given entity has been
    /// observed emitting. Iteration order is ascending
    /// [`PatternSignature`], which is the natural total order over the
    /// underlying `BTreeMap` and stays stable across calls.
    fn labels_for_entity(&self, entity: EntityId) -> Vec<&Label>;

    /// Every entity that has been observed emitting any signature
    /// labelled `label_id`, in ascending [`EntityId`] order. Per
    /// INVARIANTS §1 the sort is required so iteration over the result
    /// does not leak insertion order into downstream sim state.
    fn entities_with_label(&self, label_id: &str) -> Vec<EntityId>;

    /// Every species the chronicler has been told about, projected
    /// through `filter`. Order is determined by [`BestiaryFilter::sort_by`]
    /// with `species_id` as the secondary key, so two calls with the
    /// same filter produce a byte-identical `Vec`.
    fn bestiary_entries(&self, filter: &BestiaryFilter) -> Vec<BestiaryEntry>;

    /// One bestiary entry by species id. Returns `None` when no entity
    /// has been registered for that species.
    fn bestiary_entry(&self, species: SpeciesId) -> Option<BestiaryEntry>;
}

/// Lightweight, hand-rolled fixture implementing [`ChroniclerQuery`].
///
/// Useful for downstream UI tests (S10.4 / S10.8) that want to exercise
/// the bestiary / label rendering paths without standing up a full
/// [`Chronicler`](crate::Chronicler) and ingesting snapshots. Push values
/// into the public fields directly — none of them are checked for
/// consistency, so test authors are responsible for keeping the maps
/// coherent (e.g. don't reference an [`EntityId`] in
/// `entities_by_label_id` that isn't in `entity_signatures`).
#[derive(Clone, Debug, Default)]
pub struct InMemoryChronicler {
    /// `signature → label` map mirroring [`Chronicler::labels`](crate::Chronicler::labels).
    pub labels: BTreeMap<PatternSignature, Label>,
    /// `entity → set of signatures the entity has emitted`.
    pub entity_signatures: BTreeMap<EntityId, BTreeSet<PatternSignature>>,
    /// Pre-computed bestiary catalog keyed by species id. Returned
    /// (filtered + sorted) by [`Self::bestiary_entries`]; looked up by
    /// [`Self::bestiary_entry`].
    pub bestiary: BTreeMap<SpeciesId, BestiaryEntry>,
}

impl InMemoryChronicler {
    /// Construct an empty fixture.
    pub fn new() -> Self {
        Self::default()
    }
}

impl ChroniclerQuery for InMemoryChronicler {
    fn label_for_signature(&self, sig: &PatternSignature) -> Option<&Label> {
        self.labels.get(sig)
    }

    fn labels_for_entity(&self, entity: EntityId) -> Vec<&Label> {
        let Some(sigs) = self.entity_signatures.get(&entity) else {
            return Vec::new();
        };
        sigs.iter().filter_map(|sig| self.labels.get(sig)).collect()
    }

    fn entities_with_label(&self, label_id: &str) -> Vec<EntityId> {
        let signatures: BTreeSet<PatternSignature> = self
            .labels
            .iter()
            .filter(|(_, label)| label.id == label_id)
            .map(|(sig, _)| *sig)
            .collect();
        if signatures.is_empty() {
            return Vec::new();
        }
        // BTreeMap iteration is ascending by EntityId — sorted output
        // falls out of the storage choice (INVARIANTS §1).
        self.entity_signatures
            .iter()
            .filter(|(_, sigs)| !sigs.is_disjoint(&signatures))
            .map(|(entity, _)| *entity)
            .collect()
    }

    fn bestiary_entries(&self, filter: &BestiaryFilter) -> Vec<BestiaryEntry> {
        let mut entries: Vec<BestiaryEntry> = self
            .bestiary
            .values()
            .filter(|entry| !filter.discovered_only || entry.observation_count >= 1)
            .filter(|entry| match &filter.search {
                None => true,
                Some(needle) => label_ids_match_search(&entry.label_ids, needle),
            })
            .cloned()
            .collect();
        sort_bestiary(&mut entries, filter.sort_by);
        entries
    }

    fn bestiary_entry(&self, species: SpeciesId) -> Option<BestiaryEntry> {
        self.bestiary.get(&species).cloned()
    }
}

pub(crate) fn label_ids_match_search(label_ids: &[LabelId], needle: &str) -> bool {
    let lc = needle.to_lowercase();
    label_ids.iter().any(|id| id.to_lowercase().contains(&lc))
}

pub(crate) fn sort_bestiary(entries: &mut [BestiaryEntry], sort_by: BestiarySortBy) {
    match sort_by {
        BestiarySortBy::Confidence => entries.sort_by(|a, b| {
            // Higher confidence first; tie-break by species id ascending.
            b.confidence
                .cmp(&a.confidence)
                .then_with(|| a.species.cmp(&b.species))
        }),
        BestiarySortBy::FirstTick => entries.sort_by(|a, b| {
            a.first_tick
                .cmp(&b.first_tick)
                .then_with(|| a.species.cmp(&b.species))
        }),
        BestiarySortBy::Observations => entries.sort_by(|a, b| {
            b.observation_count
                .cmp(&a.observation_count)
                .then_with(|| a.species.cmp(&b.species))
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_label(id: &str, sig_byte: u8, conf: f64) -> Label {
        Label {
            id: id.to_owned(),
            signature: PatternSignature([sig_byte; 32]),
            confidence: Q3232::from_num(conf),
        }
    }

    // Synthetic label ids used purely for unit-test fixtures. These
    // intentionally do *not* match any shipped manifest id — the
    // `tests/no_hardcoded_label_strings.rs` invariant test scans `src/`
    // for shipped ids, and any leak there indicates an INVARIANTS §2
    // violation rather than a fixture string.
    const FIXTURE_ALPHA: &str = "fixture_alpha";
    const FIXTURE_BETA: &str = "fixture_beta";
    const FIXTURE_GAMMA: &str = "fixture_gamma";

    fn populated_fixture() -> InMemoryChronicler {
        let mut chronicler = InMemoryChronicler::new();
        let alpha_sig = PatternSignature([1; 32]);
        let beta_sig = PatternSignature([2; 32]);
        let gamma_sig = PatternSignature([3; 32]);
        chronicler
            .labels
            .insert(alpha_sig, make_label(FIXTURE_ALPHA, 1, 0.9));
        chronicler
            .labels
            .insert(beta_sig, make_label(FIXTURE_BETA, 2, 0.7));
        chronicler
            .labels
            .insert(gamma_sig, make_label(FIXTURE_GAMMA, 3, 0.5));
        chronicler
            .entity_signatures
            .insert(EntityId::new(2), BTreeSet::from([alpha_sig, beta_sig]));
        chronicler
            .entity_signatures
            .insert(EntityId::new(5), BTreeSet::from([gamma_sig]));
        chronicler
            .entity_signatures
            .insert(EntityId::new(7), BTreeSet::from([alpha_sig]));
        chronicler.bestiary.insert(
            SpeciesId::new(0),
            BestiaryEntry {
                species: SpeciesId::new(0),
                label_ids: vec![FIXTURE_ALPHA.to_owned(), FIXTURE_BETA.to_owned()],
                observation_count: 200,
                confidence: Q3232::from_num(0.9),
                first_tick: TickCounter::new(10),
            },
        );
        chronicler.bestiary.insert(
            SpeciesId::new(1),
            BestiaryEntry {
                species: SpeciesId::new(1),
                label_ids: vec![FIXTURE_GAMMA.to_owned()],
                observation_count: 50,
                confidence: Q3232::from_num(0.5),
                first_tick: TickCounter::new(5),
            },
        );
        chronicler.bestiary.insert(
            SpeciesId::new(2),
            BestiaryEntry {
                species: SpeciesId::new(2),
                label_ids: vec![],
                observation_count: 0,
                confidence: Q3232::ZERO,
                first_tick: TickCounter::ZERO,
            },
        );
        chronicler
    }

    #[test]
    fn label_for_signature_returns_assigned_label() {
        let c = populated_fixture();
        let label = c.label_for_signature(&PatternSignature([1; 32])).unwrap();
        assert_eq!(label.id, FIXTURE_ALPHA);
        assert!(c.label_for_signature(&PatternSignature([99; 32])).is_none());
    }

    #[test]
    fn labels_for_entity_collects_all_assigned_labels() {
        let c = populated_fixture();
        let labels = c.labels_for_entity(EntityId::new(2));
        let ids: Vec<&str> = labels.iter().map(|l| l.id.as_str()).collect();
        // Iteration order follows BTreeSet<PatternSignature> ascending →
        // [1; 32] before [2; 32] → alpha before beta.
        assert_eq!(ids, vec![FIXTURE_ALPHA, FIXTURE_BETA]);
    }

    #[test]
    fn labels_for_entity_returns_empty_for_unknown_entity() {
        let c = populated_fixture();
        assert!(c.labels_for_entity(EntityId::new(999)).is_empty());
    }

    #[test]
    fn entities_with_label_returns_ascending_entity_ids() {
        let c = populated_fixture();
        let entities = c.entities_with_label(FIXTURE_ALPHA);
        // EntityId(2) and EntityId(7) both emit the alpha signature;
        // result must be ascending (INVARIANTS §1).
        assert_eq!(entities, vec![EntityId::new(2), EntityId::new(7)]);
    }

    #[test]
    fn entities_with_label_handles_unknown_label() {
        let c = populated_fixture();
        assert!(c.entities_with_label("does_not_exist").is_empty());
    }

    #[test]
    fn bestiary_entries_default_filter_returns_all_sorted_by_confidence_desc() {
        let c = populated_fixture();
        let entries = c.bestiary_entries(&BestiaryFilter::default());
        let ids: Vec<u32> = entries.iter().map(|e| e.species.raw()).collect();
        // confidence ordering: 0.9 (sp0), 0.5 (sp1), 0.0 (sp2)
        assert_eq!(ids, vec![0, 1, 2]);
    }

    #[test]
    fn bestiary_entries_discovered_only_drops_zero_observations() {
        let c = populated_fixture();
        let filter = BestiaryFilter {
            discovered_only: true,
            ..Default::default()
        };
        let entries = c.bestiary_entries(&filter);
        let ids: Vec<u32> = entries.iter().map(|e| e.species.raw()).collect();
        assert_eq!(ids, vec![0, 1]);
    }

    #[test]
    fn bestiary_entries_sort_by_first_tick() {
        let c = populated_fixture();
        let filter = BestiaryFilter {
            sort_by: BestiarySortBy::FirstTick,
            ..Default::default()
        };
        let entries = c.bestiary_entries(&filter);
        let ticks: Vec<u64> = entries.iter().map(|e| e.first_tick.raw()).collect();
        // ZERO, 5, 10 — ascending.
        assert_eq!(ticks, vec![0, 5, 10]);
    }

    #[test]
    fn bestiary_entries_sort_by_observations() {
        let c = populated_fixture();
        let filter = BestiaryFilter {
            sort_by: BestiarySortBy::Observations,
            ..Default::default()
        };
        let entries = c.bestiary_entries(&filter);
        let counts: Vec<u64> = entries.iter().map(|e| e.observation_count).collect();
        // 200, 50, 0 — descending.
        assert_eq!(counts, vec![200, 50, 0]);
    }

    #[test]
    fn bestiary_entries_search_is_case_insensitive_substring() {
        let c = populated_fixture();
        let filter = BestiaryFilter {
            search: Some("GAMMA".to_owned()),
            ..Default::default()
        };
        let entries = c.bestiary_entries(&filter);
        let ids: Vec<u32> = entries.iter().map(|e| e.species.raw()).collect();
        assert_eq!(ids, vec![1]);
    }

    #[test]
    fn bestiary_entries_is_deterministic_across_calls() {
        let c = populated_fixture();
        let filter = BestiaryFilter::default();
        let first = c.bestiary_entries(&filter);
        let second = c.bestiary_entries(&filter);
        assert_eq!(first, second);
    }

    #[test]
    fn bestiary_entry_lookup() {
        let c = populated_fixture();
        let entry = c.bestiary_entry(SpeciesId::new(1)).unwrap();
        assert_eq!(entry.observation_count, 50);
        assert!(c.bestiary_entry(SpeciesId::new(99)).is_none());
    }

    #[test]
    fn confidence_tie_break_uses_species_id_ascending() {
        // Two entries with identical confidence — species id must break
        // the tie so the rendered list is stable.
        let mut c = InMemoryChronicler::new();
        c.bestiary.insert(
            SpeciesId::new(7),
            BestiaryEntry {
                species: SpeciesId::new(7),
                label_ids: vec![],
                observation_count: 1,
                confidence: Q3232::from_num(0.5),
                first_tick: TickCounter::new(0),
            },
        );
        c.bestiary.insert(
            SpeciesId::new(3),
            BestiaryEntry {
                species: SpeciesId::new(3),
                label_ids: vec![],
                observation_count: 1,
                confidence: Q3232::from_num(0.5),
                first_tick: TickCounter::new(0),
            },
        );
        let entries = c.bestiary_entries(&BestiaryFilter::default());
        let ids: Vec<u32> = entries.iter().map(|e| e.species.raw()).collect();
        assert_eq!(ids, vec![3, 7]);
    }

    #[test]
    fn bestiary_entry_has_no_discovered_field() {
        // INVARIANTS §6: `discovered` is UI-derived state and must not
        // leak into sim-side records. Compile-time check via field
        // pattern: if a `discovered` field were ever added, this test
        // would still pass — so we additionally enumerate the fields in
        // a destructure that would fail to compile if a new field
        // sneaked in.
        let entry = BestiaryEntry {
            species: SpeciesId::new(0),
            label_ids: vec![],
            observation_count: 0,
            confidence: Q3232::ZERO,
            first_tick: TickCounter::ZERO,
        };
        let BestiaryEntry {
            species: _,
            label_ids: _,
            observation_count: _,
            confidence: _,
            first_tick: _,
        } = entry;
    }
}
