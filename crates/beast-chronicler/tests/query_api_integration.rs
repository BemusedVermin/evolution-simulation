//! Integration coverage for the S10.7 [`ChroniclerQuery`] surface against
//! a live [`Chronicler`].
//!
//! The unit tests in `src/query.rs` cover the [`InMemoryChronicler`]
//! fixture; this file pins the contract for the production
//! implementation by ingesting real snapshots and assigning labels.

use std::collections::BTreeSet;

use beast_chronicler::{
    BestiaryEntry, BestiaryFilter, BestiarySortBy, Chronicler, ChroniclerQuery, LabelEngine,
    PatternSignature, PrimitiveSnapshot, SpeciesId,
};
use beast_core::{EntityId, TickCounter};

const ECHO_MANIFEST: &str = r#"{
    "labels": [
        { "id": "echolocation",   "primitives": ["echo", "spatial"], "min_confidence": 0.0 },
        { "id": "bioluminescence", "primitives": ["glow"],            "min_confidence": 0.0 }
    ]
}"#;

fn snap(tick: u64, entity: u32, primitives: &[&str]) -> PrimitiveSnapshot {
    PrimitiveSnapshot::new(
        TickCounter::new(tick),
        EntityId::new(entity),
        primitives.iter().copied(),
    )
}

#[test]
fn label_for_signature_returns_assigned_label_after_assign_pass() {
    let mut c = Chronicler::new();
    for tick in 0..50 {
        c.ingest(&snap(tick, 1, &["echo", "spatial"]));
    }
    let engine = LabelEngine::from_json_str(ECHO_MANIFEST).unwrap();
    c.assign_labels(&engine, c.last_observed_tick());

    let sig = PatternSignature::from_primitives(["spatial", "echo"]);
    let label = c.label_for_signature(&sig).expect("label assigned");
    assert_eq!(label.id, "echolocation");
    assert_eq!(label.signature, sig);

    // Unknown signature — no label.
    assert!(c
        .label_for_signature(&PatternSignature([0xff; 32]))
        .is_none());
}

#[test]
fn labels_for_entity_returns_all_assigned_labels_for_emissions() {
    let mut c = Chronicler::new();
    // Entity 4 emits both echo+spatial AND glow over time.
    for tick in 0..20 {
        c.ingest(&snap(tick, 4, &["echo", "spatial"]));
    }
    for tick in 20..40 {
        c.ingest(&snap(tick, 4, &["glow"]));
    }
    let engine = LabelEngine::from_json_str(ECHO_MANIFEST).unwrap();
    c.assign_labels(&engine, c.last_observed_tick());

    let labels = c.labels_for_entity(EntityId::new(4));
    let ids: BTreeSet<&str> = labels.iter().map(|l| l.id.as_str()).collect();
    assert_eq!(ids, BTreeSet::from(["echolocation", "bioluminescence"]),);
}

#[test]
fn entities_with_label_returns_ascending_entity_ids() {
    let mut c = Chronicler::new();
    // Entities 9, 2, 5 all emit the echolocation pattern; 1 emits glow only.
    for entity in [9u32, 2, 5] {
        for tick in 0..10 {
            c.ingest(&snap(tick, entity, &["echo", "spatial"]));
        }
    }
    for tick in 0..10 {
        c.ingest(&snap(tick, 1, &["glow"]));
    }
    let engine = LabelEngine::from_json_str(ECHO_MANIFEST).unwrap();
    c.assign_labels(&engine, c.last_observed_tick());

    let entities = c.entities_with_label("echolocation");
    assert_eq!(
        entities,
        vec![EntityId::new(2), EntityId::new(5), EntityId::new(9)],
        "result must be ascending by EntityId per INVARIANTS §1",
    );

    // Unknown label.
    assert!(c.entities_with_label("does_not_exist").is_empty());
}

#[test]
fn bestiary_entries_aggregates_across_species() {
    let mut c = Chronicler::new();
    // Species 0: entities 1 + 2, both emit echolocation.
    c.set_entity_species(EntityId::new(1), SpeciesId::new(0));
    c.set_entity_species(EntityId::new(2), SpeciesId::new(0));
    // Species 1: entity 3 emits glow only.
    c.set_entity_species(EntityId::new(3), SpeciesId::new(1));
    // Species 2: entity 4 — registered but ingests nothing (zero observations).
    c.set_entity_species(EntityId::new(4), SpeciesId::new(2));

    for tick in 0..60 {
        c.ingest(&snap(tick, 1, &["echo", "spatial"]));
        c.ingest(&snap(tick, 2, &["echo", "spatial"]));
    }
    for tick in 0..30 {
        c.ingest(&snap(tick, 3, &["glow"]));
    }
    let engine = LabelEngine::from_json_str(ECHO_MANIFEST).unwrap();
    c.assign_labels(&engine, c.last_observed_tick());

    let all = c.bestiary_entries(&BestiaryFilter::default());
    let by_species: Vec<u32> = all.iter().map(|e| e.species.raw()).collect();
    // Default sort is confidence descending; species 0 (echolocation
    // assigned at full saturation) outranks species 1 (smaller share),
    // and species 2 has zero confidence so it lands last.
    assert_eq!(by_species, vec![0, 1, 2]);

    let species0: &BestiaryEntry = all.iter().find(|e| e.species.raw() == 0).unwrap();
    assert_eq!(species0.observation_count, 120);
    assert_eq!(species0.label_ids, vec!["echolocation".to_owned()]);
    assert_eq!(species0.first_tick, TickCounter::new(0));
}

#[test]
fn bestiary_entries_discovered_only_drops_zero_observation_species() {
    let mut c = Chronicler::new();
    c.set_entity_species(EntityId::new(1), SpeciesId::new(0));
    c.set_entity_species(EntityId::new(2), SpeciesId::new(1)); // no ingest
    for tick in 0..10 {
        c.ingest(&snap(tick, 1, &["echo", "spatial"]));
    }

    let filter = BestiaryFilter {
        discovered_only: true,
        ..Default::default()
    };
    let entries = c.bestiary_entries(&filter);
    let species: Vec<u32> = entries.iter().map(|e| e.species.raw()).collect();
    assert_eq!(species, vec![0]);
}

#[test]
fn bestiary_entries_is_deterministic_across_calls() {
    let mut c = Chronicler::new();
    c.set_entity_species(EntityId::new(7), SpeciesId::new(2));
    c.set_entity_species(EntityId::new(3), SpeciesId::new(1));
    c.set_entity_species(EntityId::new(11), SpeciesId::new(0));
    for entity in [7u32, 3, 11] {
        for tick in 0..20 {
            c.ingest(&snap(tick, entity, &["echo", "spatial"]));
        }
    }
    let engine = LabelEngine::from_json_str(ECHO_MANIFEST).unwrap();
    c.assign_labels(&engine, c.last_observed_tick());

    let filter = BestiaryFilter::default();
    let first = c.bestiary_entries(&filter);
    let second = c.bestiary_entries(&filter);
    assert_eq!(first, second);

    // Identical confidences across species → species_id ascending must
    // determine the rendered order.
    let species_order: Vec<u32> = first.iter().map(|e| e.species.raw()).collect();
    assert_eq!(species_order, vec![0, 1, 2]);
}

#[test]
fn bestiary_entry_lookup_returns_none_for_unknown_species() {
    let mut c = Chronicler::new();
    c.set_entity_species(EntityId::new(1), SpeciesId::new(0));
    for tick in 0..5 {
        c.ingest(&snap(tick, 1, &["echo", "spatial"]));
    }
    assert!(c.bestiary_entry(SpeciesId::new(0)).is_some());
    assert!(c.bestiary_entry(SpeciesId::new(42)).is_none());
}

#[test]
fn bestiary_filter_search_is_case_insensitive_substring() {
    let mut c = Chronicler::new();
    c.set_entity_species(EntityId::new(1), SpeciesId::new(0));
    c.set_entity_species(EntityId::new(2), SpeciesId::new(1));
    for tick in 0..30 {
        c.ingest(&snap(tick, 1, &["echo", "spatial"]));
    }
    for tick in 0..30 {
        c.ingest(&snap(tick, 2, &["glow"]));
    }
    let engine = LabelEngine::from_json_str(ECHO_MANIFEST).unwrap();
    c.assign_labels(&engine, c.last_observed_tick());

    let filter = BestiaryFilter {
        search: Some("LUMIN".to_owned()),
        ..Default::default()
    };
    let entries = c.bestiary_entries(&filter);
    let species: Vec<u32> = entries.iter().map(|e| e.species.raw()).collect();
    assert_eq!(species, vec![1]);
}

#[test]
fn bestiary_sort_by_first_tick_orders_ascending() {
    let mut c = Chronicler::new();
    c.set_entity_species(EntityId::new(1), SpeciesId::new(0));
    c.set_entity_species(EntityId::new(2), SpeciesId::new(1));
    // Species 0 first observed at tick 50; species 1 at tick 5.
    c.ingest(&snap(50, 1, &["echo", "spatial"]));
    c.ingest(&snap(5, 2, &["glow"]));

    let filter = BestiaryFilter {
        sort_by: BestiarySortBy::FirstTick,
        ..Default::default()
    };
    let entries = c.bestiary_entries(&filter);
    let species: Vec<u32> = entries.iter().map(|e| e.species.raw()).collect();
    assert_eq!(species, vec![1, 0]);
}

#[test]
fn chronicler_query_trait_object_compiles() {
    // Pinning that ChroniclerQuery is dyn-compatible — UI code holds
    // `&dyn ChroniclerQuery` (per S10.4 screen builders) so a regression
    // here would break the entire screens layer.
    let chronicler = Chronicler::new();
    let _: &dyn ChroniclerQuery = &chronicler;
}
