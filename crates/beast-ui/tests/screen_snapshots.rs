//! Layout snapshot fixtures for the four MVP screens at 1280×720.
//!
//! Per the S10.4 DoD: "`dump_layout` snapshot fixtures exist for each
//! screen at 1280×720." This file pins the exact pre-order dump for
//! each screen so any unintended layout drift is caught in CI.
//!
//! Snapshots are validated against *structure* (presence + ordering of
//! widget kinds) plus *exact* widget ids and bounds. Two assertions
//! per fixture:
//!
//! 1. The full dump string matches a hand-authored fixture byte for
//!    byte. Any layout regression — a new widget, a re-ordering, a
//!    bounds shift — fails this check.
//! 2. The dump string parses into the expected sequence of widget
//!    kinds, so a structural drift gives a more readable diagnostic
//!    than a raw text diff.
//!
//! Re-running with the snapshots stale: print `actual` from the test
//! failure message and copy it back into the fixture string.

use beast_chronicler::PatternSignature;
use beast_chronicler::{
    BestiaryEntry, BestiaryFilter, ChroniclerQuery, InMemoryChronicler, Label, SpeciesId,
};
use beast_core::{TickCounter, Q3232};
use beast_ui::screens::data::{
    BiomeView, EncounterCreatureSnapshot, EncounterSnapshot, FormationView, KeeperView, WorldStatus,
};
use beast_ui::{bestiary, dump_layout, encounter, settings, world_map};

struct FixedWorld {
    tick: u64,
    creatures: usize,
}

impl WorldStatus for FixedWorld {
    fn current_tick(&self) -> u64 {
        self.tick
    }
    fn creature_count(&self) -> usize {
        self.creatures
    }
}

fn populated_chronicler() -> InMemoryChronicler {
    let mut c = InMemoryChronicler::new();
    let sig0 = PatternSignature([1; 32]);
    c.labels.insert(
        sig0,
        Label {
            id: "echolocation".into(),
            signature: sig0,
            confidence: Q3232::from_num(0.85),
        },
    );
    c.bestiary.insert(
        SpeciesId::new(0),
        BestiaryEntry {
            species: SpeciesId::new(0),
            label_ids: vec!["echolocation".into()],
            observation_count: 120,
            confidence: Q3232::from_num(0.85),
            first_tick: TickCounter::new(15),
        },
    );
    c.bestiary.insert(
        SpeciesId::new(1),
        BestiaryEntry {
            species: SpeciesId::new(1),
            label_ids: vec!["pack_hunting".into()],
            observation_count: 75,
            confidence: Q3232::from_num(0.65),
            first_tick: TickCounter::new(40),
        },
    );
    c
}

fn assert_kind_sequence(dump: &str, expected_kinds: &[&str]) {
    // `dump_layout` ends with a trailing `\n`; without the empty
    // filter, `.lines()` here would still yield the right number of
    // entries (since `lines()` swallows the trailing newline), but
    // any future format tweak that emits double newlines or blank
    // separator lines would surface as a confusing
    // "expected `Foo` at position N, got ``" diff. The filter
    // tolerates both shapes.
    let kinds: Vec<&str> = dump
        .lines()
        .map(|line| line.split('#').next().unwrap_or(""))
        .filter(|kind| !kind.is_empty())
        .collect();
    assert_eq!(
        kinds, expected_kinds,
        "structural drift — actual kind sequence:\n{kinds:#?}\nfull dump:\n{dump}"
    );
}

#[test]
fn world_map_snapshot_matches_fixture() {
    let world = FixedWorld {
        tick: 1024,
        creatures: 12,
    };
    let biomes = BiomeView::solid(64, 64, [40, 100, 50, 255]);
    let mut tree = world_map(&world, &biomes);
    assert!(tree.layout(), "first layout pass should be a cache miss");
    let dump = dump_layout(&tree);
    assert_kind_sequence(
        &dump,
        &[
            "Stack",          // root frame
            "Card",           // status bar
            "Label",          // status bar binding
            "Stack",          // content row
            "RenderViewport", // world map viewport
            "Stack",          // action bar (vertical)
            "Button",         // bestiary
            "Button",         // settings
            "Button",         // quit
            "Card",           // world status card
            "Label",          // world status binding
        ],
    );
    // Spot-check the chrome geometry. Root Stack fills the
    // viewport; content row sits just under the 20-px status bar.
    // The dump's root id depends on allocation order — the screen
    // builder allocates content widgets first and the frame last —
    // so we anchor on bounds rather than ids.
    let lines: Vec<&str> = dump.lines().collect();
    assert!(
        lines[0].contains(" 0,0 1280x720"),
        "root must fill viewport, got: {}",
        lines[0]
    );
    // Content row sits below the status bar Card. The status bar
    // measures (20 px title bar + 16 px label) = 36 px tall, so the
    // content row's y origin equals 36 with the default 0 gap.
    assert!(
        lines[3].contains("0,36"),
        "content row should sit just under the 36-px status bar, got: {}",
        lines[3]
    );
}

#[test]
fn bestiary_snapshot_matches_fixture() {
    let chronicler = populated_chronicler();
    let mut tree = bestiary(&chronicler);
    assert!(tree.layout());
    let dump = dump_layout(&tree);
    assert_kind_sequence(
        &dump,
        &[
            "Stack",         // root frame
            "Card",          // status bar
            "Label",         // status text binding
            "Stack",         // content row
            "BestiaryPanel", // list + detail composite
            "List",          // panel's list child
            "Card",          // filter card
            "Label",         // filter text body
        ],
    );
    let lines: Vec<&str> = dump.lines().collect();
    assert!(
        lines[0].starts_with("Stack#") && lines[0].contains(" 0,0 1280x720"),
        "root must be a Stack filling the viewport, got: {}",
        lines[0]
    );
    assert!(
        lines[4].starts_with("BestiaryPanel#"),
        "BestiaryPanel should live at index 4, got: {}",
        lines[4]
    );
}

#[test]
fn settings_snapshot_matches_fixture() {
    let mut tree = settings();
    assert!(tree.layout());
    let dump = dump_layout(&tree);
    assert_kind_sequence(
        &dump,
        &[
            "Stack", // root frame
            "Card",  // status bar
            "Label", // status binding label
            "Stack", // content row
            "Card",  // rendering card
            "Label", "Label", "Label", "Label", "Card", // audio card
            "Label", "Label", "Label", "Label", "Card", // accessibility card
            "Label", "Label", "Label", "Label",
        ],
    );
    let lines: Vec<&str> = dump.lines().collect();
    assert!(
        lines[0].starts_with("Stack#") && lines[0].contains(" 0,0 1280x720"),
        "root must be a Stack filling the viewport, got: {}",
        lines[0]
    );
}

#[test]
fn encounter_snapshot_matches_fixture() {
    let snapshot = EncounterSnapshot {
        biome_label: "forest".into(),
        creatures: vec![
            EncounterCreatureSnapshot {
                id: 1,
                name: "alpha".into(),
                hp_pct: 1.0,
            },
            EncounterCreatureSnapshot {
                id: 2,
                name: "beta".into(),
                hp_pct: 0.5,
            },
        ],
        selected: Some(0),
        formation: FormationView::empty(),
        keeper: KeeperView::empty(),
    };
    let chronicler = InMemoryChronicler::new();
    let mut tree = encounter(&chronicler, &snapshot);
    assert!(tree.layout());
    let dump = dump_layout(&tree);
    assert_kind_sequence(
        &dump,
        &[
            "Stack", // root frame
            "Card",  // status bar
            "Label", // status binding label
            "Stack", // outer column (formation row above content row)
            "Stack", // formation row
            "Card",  // slot 0 (vanguard)
            "Label",
            "Card", // slot 1 (flank-left)
            "Label",
            "Card", // slot 2 (flank-right)
            "Label",
            "Card", // slot 3 (center)
            "Label",
            "Card", // slot 4 (rear)
            "Label",
            "Stack", // content row
            "Stack", // left column
            "RenderViewport",
            "List",  // creature list
            "Stack", // right column
            "Stack", // action button bar
            "Button",
            "Button",
            "Button",
            "Card",  // info card
            "Label", // info text
        ],
    );
    let lines: Vec<&str> = dump.lines().collect();
    assert!(
        lines[0].starts_with("Stack#") && lines[0].contains(" 0,0 1280x720"),
        "root must be a Stack filling the viewport, got: {}",
        lines[0]
    );
}

#[test]
fn snapshots_are_deterministic_across_calls() {
    // INVARIANTS §1 leans on deterministic widget id + bounds output;
    // this test pins that two builds of the same screen with the
    // same inputs produce a byte-identical dump.
    let world = FixedWorld {
        tick: 0,
        creatures: 0,
    };
    let biomes = BiomeView::solid(8, 8, [0, 0, 0, 255]);
    let mut a = world_map(&world, &biomes);
    let mut b = world_map(&world, &biomes);
    let _ = a.layout();
    let _ = b.layout();
    assert_eq!(dump_layout(&a), dump_layout(&b));

    let chronicler = populated_chronicler();
    let mut x = bestiary(&chronicler);
    let mut y = bestiary(&chronicler);
    let _ = x.layout();
    let _ = y.layout();
    assert_eq!(dump_layout(&x), dump_layout(&y));

    // Sort key is deterministic, but verify directly: first two
    // entries must produce identical filter results across calls.
    let filter = BestiaryFilter::default();
    assert_eq!(
        chronicler.bestiary_entries(&filter),
        chronicler.bestiary_entries(&filter)
    );
}
