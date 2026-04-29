//! Encounter screen (S10.4).
//!
//! Composition (top → bottom):
//!
//! 1. Status bar: biome label + active-creature name.
//! 2. Content row:
//!    - Left column: a [`crate::RenderViewport`] reserved for the
//!      `beast-render::encounter` 2.5D scene plus a creature `List`.
//!    - Right column: a horizontal action button bar (Attack /
//!      Defend / Flee). Buttons are static today — wiring real
//!      input bindings into the encounter primitive emitter is a
//!      later sprint.
//!
//! Per `documentation/INVARIANTS.md` §6 the screen reads from the
//! [`EncounterSnapshot`] only — no widget in the tree mutates the
//! snapshot or reaches into sim state.

use crate::layout::{Align, Axis};
use crate::paint::Color;
use crate::screens::data::EncounterSnapshot;
use crate::screens::frame::screen_frame;
use crate::widget::{Button, Card, IdAllocator, Label, List, ListItem, RenderViewport, Stack};
use crate::{Size, WidgetTree};

/// Build the encounter screen for a 1280×720 viewport.
pub fn encounter(snapshot: &EncounterSnapshot) -> WidgetTree {
    let mut ids = IdAllocator::new();

    let left = build_left_column(&mut ids, snapshot);
    let right = build_right_column(&mut ids, snapshot);

    let mut content = Stack::new(ids.allocate(), Axis::Horizontal)
        .with_gap(8.0)
        .with_align(Align::Start);
    content.push_child(Box::new(left));
    content.push_child(Box::new(right));

    let biome = snapshot.biome_label.clone();
    let active_for_status = active_creature_name(snapshot, "(none)");
    let frame = screen_frame(
        &mut ids,
        move || format!("encounter | biome: {biome} | active: {active_for_status}"),
        Box::new(content),
    );

    WidgetTree::new(Box::new(frame), Size::new(1280.0, 720.0))
}

/// Render-viewport + creature list, packed vertically.
fn build_left_column(ids: &mut IdAllocator, snapshot: &EncounterSnapshot) -> Stack {
    let mut left = Stack::new(ids.allocate(), Axis::Vertical)
        .with_gap(8.0)
        .with_align(Align::Start);

    let viewport = RenderViewport::new(ids.allocate())
        .with_tint(Color::rgb(0.18, 0.20, 0.10))
        .with_preferred_size(Size::new(900.0, 460.0));
    left.push_child(Box::new(viewport));
    left.push_child(Box::new(build_creature_list(ids, snapshot)));
    left
}

/// `List<u32>` populated with creature labels + applied selection.
fn build_creature_list(ids: &mut IdAllocator, snapshot: &EncounterSnapshot) -> List<u32> {
    let mut creature_list: List<u32> = List::new(ids.allocate());
    let items: Vec<ListItem<u32>> = snapshot
        .creatures
        .iter()
        .map(|c| {
            let label = format!(
                "{} · hp {:>3}%",
                c.name,
                (c.hp_pct.clamp(0.0, 1.0) * 100.0).round() as i32
            );
            ListItem::new(label, c.id)
        })
        .collect();
    creature_list.set_items(items);
    if let Some(idx) = snapshot.selected {
        if idx < snapshot.creatures.len() {
            creature_list.set_selected(Some(idx));
        }
    }
    creature_list
}

/// Action-bar buttons + info card, packed vertically.
fn build_right_column(ids: &mut IdAllocator, snapshot: &EncounterSnapshot) -> Stack {
    let mut right = Stack::new(ids.allocate(), Axis::Vertical)
        .with_gap(8.0)
        .with_align(Align::Start);

    let mut action_bar = Stack::new(ids.allocate(), Axis::Horizontal).with_gap(8.0);
    for label in ["Attack", "Defend", "Flee"] {
        action_bar.push_child(Box::new(Button::new(ids.allocate(), label)));
    }
    right.push_child(Box::new(action_bar));

    let mut info_card = Card::new(ids.allocate(), "encounter");
    let active_name = active_creature_name(snapshot, "(no selection)");
    info_card.push_child(Box::new(Label::new(
        ids.allocate(),
        format!(
            "biome: {}\nactive: {}\ncreatures: {}",
            snapshot.biome_label,
            active_name,
            snapshot.creatures.len(),
        ),
    )));
    right.push_child(Box::new(info_card));
    right
}

/// Active creature's display name, or `fallback` when no creature is
/// selected. Centralised so the status-bar and info-card strings
/// stay in lockstep when the snapshot's selection shape changes.
fn active_creature_name(snapshot: &EncounterSnapshot, fallback: &str) -> String {
    snapshot
        .selected_creature()
        .map(|c| c.name.clone())
        .unwrap_or_else(|| fallback.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dump_layout;
    use crate::screens::data::{EncounterCreatureSnapshot, EncounterSnapshot};

    fn populated_snapshot() -> EncounterSnapshot {
        EncounterSnapshot {
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
        }
    }

    #[test]
    fn encounter_screen_compiles_and_lays_out() {
        let snapshot = populated_snapshot();
        let mut tree = encounter(&snapshot);
        assert!(tree.layout(), "first layout should be a cache miss");
        let dump = dump_layout(&tree);
        assert!(dump.contains("RenderViewport#"), "dump:\n{dump}");
        assert!(dump.contains("List#"), "dump:\n{dump}");
        let buttons = dump.lines().filter(|l| l.starts_with("Button#")).count();
        assert_eq!(buttons, 3, "expected 3 action buttons, dump:\n{dump}");
    }

    #[test]
    fn empty_snapshot_renders_no_selection() {
        let snapshot = EncounterSnapshot::empty("ocean");
        let mut tree = encounter(&snapshot);
        assert!(tree.layout());
        let mut ctx = crate::paint::PaintCtx::new();
        tree.paint(&mut ctx);
        let has_no_selection = ctx.commands().iter().any(|cmd| match cmd {
            crate::paint::DrawCmd::Text { text, .. } => text.contains("(no selection)"),
            _ => false,
        });
        assert!(has_no_selection);
    }
}
