//! S10.4 DoD coverage:
//!
//! * "Selecting a bestiary list item updates the detail card text
//!   (verified by event dispatch test)."
//! * "Status bar binding reads `current_tick` and `creature_count`
//!   from a fake `World` snapshot."
//! * "No screen builder takes `&mut World` or `&mut Chronicler` —
//!   read-only refs only." (Compile-time signature check.)
//!
//! These tests exercise the public `beast_ui::*` surface only — no
//! crate-internals access — so a future refactor that keeps the
//! contract intact stays free to reshape the implementation.

use beast_chronicler::{BestiaryEntry, ChroniclerQuery, InMemoryChronicler, SpeciesId};
use beast_core::{TickCounter, Q3232};
use beast_ui::screens::data::{BiomeView, EncounterSnapshot, WorldStatus};
use beast_ui::{
    bestiary, encounter, paint::DrawCmd, settings, world_map, BestiaryPanel, EventResult,
    IdAllocator, MouseButton, PaintCtx, Rect, Size, UiEvent, WidgetTree,
};

struct FakeWorld {
    tick: u64,
    creatures: usize,
}

impl WorldStatus for FakeWorld {
    fn current_tick(&self) -> u64 {
        self.tick
    }
    fn creature_count(&self) -> usize {
        self.creatures
    }
}

fn collect_text(tree: &WidgetTree) -> Vec<String> {
    let mut ctx = PaintCtx::new();
    tree.paint(&mut ctx);
    ctx.commands()
        .iter()
        .filter_map(|cmd| match cmd {
            DrawCmd::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn world_map_status_bar_reads_world_status_through_binding() {
    // The S10.4 DoD says the status bar binding "reads `current_tick`
    // and `creature_count` from a fake `World` snapshot." Today the
    // status values are *snapped* from `WorldStatus` at construction
    // time and captured by value into the `FnBinding` closure (see
    // `world_map.rs` doc comment) — the binding still re-fires on
    // every paint, but always returns the same snapped values. A
    // genuinely live binding lands in S13.
    //
    // We pin two contracts here:
    //
    //  1. `WorldStatus::current_tick()` and `creature_count()` are
    //     called during construction and the rendered status reflects
    //     them on the first paint.
    //  2. Re-paints with the same screen produce the same status —
    //     no per-paint mutation source is hidden in the closure.
    //
    // To verify (1) we use distinctive integers that can't appear in
    // any other rendered text. To verify (2) we paint twice and
    // compare the recovered status strings.
    let world = FakeWorld {
        tick: 1234,
        creatures: 17,
    };
    let biomes = BiomeView::solid(8, 8, [40, 100, 50, 255]);
    let mut tree = world_map(&world, &biomes);
    assert!(tree.layout(), "first layout should be a cache miss");

    let first = collect_text(&tree);
    assert!(
        first.iter().any(|t| t.contains("tick: 1234")),
        "first paint must surface current_tick from WorldStatus, got: {first:?}"
    );
    assert!(
        first.iter().any(|t| t.contains("creatures: 17")),
        "first paint must surface creature_count from WorldStatus, got: {first:?}"
    );

    // Repaint without touching the source world. The closure
    // captured snapped values, so the tick text must reproduce
    // exactly across paints. If a future refactor wires a live
    // binding (e.g. `Rc<Cell<u64>>`), this assertion becomes an
    // explicit regression marker — the world_map.rs doc + this
    // comment need to be updated together at that point.
    let later = collect_text(&tree);
    assert_eq!(
        first.iter().filter(|t| t.contains("tick: 1234")).count(),
        later.iter().filter(|t| t.contains("tick: 1234")).count(),
        "snapshot binding must keep tick text stable across paints"
    );
    // Borrow check: `world` was only used during construction, so
    // it doesn't need to outlive the second paint — but holding the
    // reference here documents the contract that `WorldStatus` is
    // queried at build time and never again.
    let _ = world.creature_count();
}

#[test]
fn bestiary_selection_updates_detail_card_text() {
    // DoD: "Selecting a bestiary list item updates the detail card
    // text (verified by event dispatch test)."
    //
    // We construct a `BestiaryPanel` directly so we can drive the
    // mouse hit-test against bounds we've assigned ourselves —
    // simulating the dispatch the screen would route through, but
    // without standing up the full widget tree.
    let mut ids = IdAllocator::new();
    let entries = vec![
        BestiaryEntry {
            species: SpeciesId::new(0),
            label_ids: vec!["echolocation".into()],
            observation_count: 100,
            confidence: Q3232::from_num(0.9),
            first_tick: TickCounter::new(10),
        },
        BestiaryEntry {
            species: SpeciesId::new(1),
            label_ids: vec!["pack_hunting".into()],
            observation_count: 50,
            confidence: Q3232::from_num(0.6),
            first_tick: TickCounter::new(25),
        },
        BestiaryEntry {
            species: SpeciesId::new(2),
            label_ids: vec!["bioluminescence".into()],
            observation_count: 5,
            confidence: Q3232::from_num(0.2),
            first_tick: TickCounter::new(60),
        },
    ];
    let mut panel = BestiaryPanel::new(&mut ids, entries);
    panel
        .list_mut()
        .set_bounds(Rect::xywh(0.0, 0.0, 280.0, 600.0));

    // Initially: no selection, default detail.
    assert_eq!(panel.detail_text(), "select an entry");

    // Move cursor over the second row (each row is 20px tall) and
    // primary-click to select it. Dispatched through the panel so
    // its `handle_event` re-syncs the detail text.
    use beast_ui::Widget;
    let _ = panel.handle_event(&UiEvent::MouseMove { x: 5.0, y: 25.0 });
    let result = panel.handle_event(&UiEvent::MouseDown {
        button: MouseButton::Primary,
    });
    assert_eq!(result, EventResult::Consumed);
    assert!(
        panel.detail_text().contains("pack_hunting"),
        "detail must reflect row 1, got: {}",
        panel.detail_text()
    );

    // Click the third row.
    let _ = panel.handle_event(&UiEvent::MouseMove { x: 5.0, y: 45.0 });
    let _ = panel.handle_event(&UiEvent::MouseDown {
        button: MouseButton::Primary,
    });
    assert!(
        panel.detail_text().contains("bioluminescence"),
        "detail must reflect row 2, got: {}",
        panel.detail_text()
    );
}

#[test]
fn bestiary_screen_routes_selection_through_widget_tree() {
    // End-to-end variant: build the full bestiary screen via the
    // public builder, dispatch a click against the embedded list,
    // and verify the rendered detail text changes between paints.
    let mut chronicler = InMemoryChronicler::new();
    chronicler.bestiary.insert(
        SpeciesId::new(0),
        BestiaryEntry {
            species: SpeciesId::new(0),
            label_ids: vec!["echolocation".into()],
            observation_count: 100,
            confidence: Q3232::from_num(0.9),
            first_tick: TickCounter::new(10),
        },
    );
    chronicler.bestiary.insert(
        SpeciesId::new(1),
        BestiaryEntry {
            species: SpeciesId::new(1),
            label_ids: vec!["pack_hunting".into()],
            observation_count: 50,
            confidence: Q3232::from_num(0.6),
            first_tick: TickCounter::new(25),
        },
    );

    let mut tree = bestiary(&chronicler);
    let _ = tree.layout();

    // Initial paint — detail shows the default placeholder.
    let pre_texts = collect_text(&tree);
    assert!(
        pre_texts.iter().any(|t| t == "select an entry"),
        "default detail text should be rendered before selection"
    );

    // Drive a click on the first list row. The List rows live at
    // (x = panel.bounds.x, y = panel.bounds.y + 0..20). We don't
    // know the exact origin without inspecting the dump, so use
    // the dump to find the BestiaryPanel + List bounds and click
    // inside row 0.
    let dump = beast_ui::dump_layout(&tree);
    let list_line = dump
        .lines()
        .find(|line| line.starts_with("List#"))
        .expect("bestiary tree must contain a List");
    let (origin_x, origin_y) = parse_bounds_origin(list_line);

    let _ = tree.dispatch(&UiEvent::MouseMove {
        x: origin_x + 5.0,
        y: origin_y + 5.0,
    });
    let result = tree.dispatch(&UiEvent::MouseDown {
        button: MouseButton::Primary,
    });
    assert_eq!(result, EventResult::Consumed, "list click must consume");

    let post_texts = collect_text(&tree);
    let any_label_in_detail = post_texts
        .iter()
        .any(|t| t.contains("echolocation") || t.contains("pack_hunting"));
    assert!(
        any_label_in_detail,
        "after dispatching a click the detail text should contain a label name, got: {post_texts:?}"
    );
}

fn parse_bounds_origin(line: &str) -> (f32, f32) {
    // Format: `Kind#id x,y wxh`. Split on whitespace; bounds start
    // at index 1. A regression in `dump_layout`'s output format
    // would otherwise panic with a non-descriptive index/parse
    // error pointing at this helper rather than the layout bug.
    let parts: Vec<&str> = line.split_whitespace().collect();
    let xy = parts
        .get(1)
        .unwrap_or_else(|| panic!("malformed dump line, expected `Kind#id x,y wxh`: {line:?}"));
    let mut iter = xy.split(',');
    let x_str = iter
        .next()
        .unwrap_or_else(|| panic!("missing x coord in dump line: {line:?}"));
    let y_str = iter
        .next()
        .unwrap_or_else(|| panic!("missing y coord in dump line: {line:?}"));
    let x: f32 = x_str
        .parse()
        .unwrap_or_else(|e| panic!("cannot parse x coord {x_str:?} in {line:?}: {e}"));
    let y: f32 = y_str
        .parse()
        .unwrap_or_else(|e| panic!("cannot parse y coord {y_str:?} in {line:?}: {e}"));
    (x, y)
}

#[test]
fn settings_screen_takes_no_inputs() {
    // No-arg builder — implicit proof that no `&mut` reference can
    // be passed in.
    let mut tree = settings();
    assert!(tree.layout());
}

#[test]
fn screens_accept_only_read_only_references() {
    // Compile-time check: every screen builder accepts shared
    // references / trait objects, never `&mut`. If a refactor
    // accidentally widens the surface to take a mutable handle,
    // this function fails to compile.
    let world = FakeWorld {
        tick: 0,
        creatures: 0,
    };
    let biomes = BiomeView::solid(2, 2, [0, 0, 0, 255]);
    let chronicler = InMemoryChronicler::new();
    let snapshot = EncounterSnapshot::empty("ocean");

    fn takes_dyn_world_status(w: &dyn WorldStatus, b: &BiomeView) -> WidgetTree {
        world_map(w, b)
    }
    fn takes_dyn_chronicler_query(q: &dyn ChroniclerQuery) -> WidgetTree {
        bestiary(q)
    }
    fn takes_encounter_ref(s: &EncounterSnapshot) -> WidgetTree {
        encounter(s)
    }

    let _ = takes_dyn_world_status(&world, &biomes);
    let _ = takes_dyn_chronicler_query(&chronicler);
    let _ = takes_encounter_ref(&snapshot);
}

#[test]
fn world_map_screen_root_is_1280x720() {
    let world = FakeWorld {
        tick: 0,
        creatures: 0,
    };
    let biomes = BiomeView::empty();
    let tree = world_map(&world, &biomes);
    assert_eq!(tree.root_size(), Size::new(1280.0, 720.0));
}

#[test]
fn all_screens_have_1280x720_viewport() {
    let world = FakeWorld {
        tick: 0,
        creatures: 0,
    };
    let biomes = BiomeView::empty();
    let chronicler = InMemoryChronicler::new();
    let snapshot = EncounterSnapshot::empty("ocean");

    let target = Size::new(1280.0, 720.0);
    assert_eq!(world_map(&world, &biomes).root_size(), target);
    assert_eq!(bestiary(&chronicler).root_size(), target);
    assert_eq!(settings().root_size(), target);
    assert_eq!(encounter(&snapshot).root_size(), target);
}
