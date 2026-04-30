//! Encounter screen — combat readout (S10.4 + S11.6).
//!
//! Composition (top → bottom):
//!
//! 1. Status bar (in the screen frame): biome label + active-creature
//!    name + leadership-budget bar.
//! 2. Formation row: five slot cards (vanguard, flank-left,
//!    flank-right, center, rear) showing per-occupant HP / stamina
//!    plus a label list pulled from
//!    [`ChroniclerQuery::labels_for_entity`] — the only surface
//!    where named-ability strings ("echolocation", "pack_hunting")
//!    appear (INVARIANTS §2).
//! 3. Content row:
//!    - Left column: a [`crate::RenderViewport`] reserved for the
//!      `beast-render::encounter` 2.5D scene plus a creature `List`.
//!    - Right column: a horizontal action button bar (Attack /
//!      Defend / Flee). Buttons are static today — wiring real
//!      input bindings into the encounter primitive emitter is a
//!      later sprint.
//!
//! Per `documentation/INVARIANTS.md` §6 the screen reads from the
//! [`EncounterSnapshot`] and the `ChroniclerQuery` only — no widget
//! in the tree mutates the snapshot or reaches into sim state.

use beast_chronicler::ChroniclerQuery;
use beast_core::EntityId;

use crate::layout::{Align, Axis};
use crate::paint::Color;
use crate::screens::data::{EncounterSnapshot, FormationSlotView};
use crate::screens::frame::screen_frame;
use crate::widget::{Button, Card, IdAllocator, Label, List, ListItem, RenderViewport, Stack};
use crate::{Size, WidgetTree};

/// Build the encounter screen for a 1280×720 viewport.
///
/// Takes both a chronicler query (for ability label lookup) and a
/// read-only `EncounterSnapshot`. Per the S11.6 DoD: never `&mut`,
/// never reaches into sim state.
pub fn encounter(query: &dyn ChroniclerQuery, snapshot: &EncounterSnapshot) -> WidgetTree {
    let mut ids = IdAllocator::new();

    let formation_row = build_formation_row(&mut ids, query, snapshot);
    let left = build_left_column(&mut ids, snapshot);
    let right = build_right_column(&mut ids, snapshot);

    let mut content = Stack::new(ids.allocate(), Axis::Horizontal)
        .with_gap(8.0)
        .with_align(Align::Start);
    content.push_child(Box::new(left));
    content.push_child(Box::new(right));

    // Outer column: formation row above the content row.
    let mut outer = Stack::new(ids.allocate(), Axis::Vertical)
        .with_gap(8.0)
        .with_align(Align::Start);
    outer.push_child(Box::new(formation_row));
    outer.push_child(Box::new(content));

    let biome = snapshot.biome_label.clone();
    let active_for_status = active_creature_name(snapshot, "(none)");
    let leadership_pct = snapshot.keeper.leadership_pct.clamp(0.0, 1.0);
    let stress_pct = snapshot.keeper.stress_pct.clamp(0.0, 1.0);
    let keeper_name = snapshot.keeper.name.clone();
    let frame = screen_frame(
        &mut ids,
        move || {
            // Leadership / stress live in the status string until the
            // frame gains a dedicated horizontal bar widget. Format is
            // pinned by `encounter_screen_paints_leadership_in_status`.
            let keeper_label = if keeper_name.is_empty() {
                "(none)".to_string()
            } else {
                keeper_name.clone()
            };
            format!(
                "encounter | biome: {biome} | active: {active_for_status} | \
                 keeper: {keeper_label} | leadership: {:>3}% | stress: {:>3}%",
                (leadership_pct * 100.0).round() as i32,
                (stress_pct * 100.0).round() as i32,
            )
        },
        Box::new(outer),
    );

    WidgetTree::new(Box::new(frame), Size::new(1280.0, 720.0))
}

/// Five formation slot cards, one per `SLOT_COUNT` position.
///
/// Per slot:
/// - title from `slot_label` ("vanguard" etc.)
/// - occupant name + HP / stamina percentages
/// - engagement / exposure percentages
/// - chronicler-resolved ability labels (the only labelled-ability
///   surface — INVARIANTS §2 keeps `primitive_id` strings off the UI
///   path; we read `Label::id` from the chronicler instead)
fn build_formation_row(
    ids: &mut IdAllocator,
    query: &dyn ChroniclerQuery,
    snapshot: &EncounterSnapshot,
) -> Stack {
    let mut row = Stack::new(ids.allocate(), Axis::Horizontal)
        .with_gap(6.0)
        .with_align(Align::Start);
    for slot in &snapshot.formation.slots {
        row.push_child(Box::new(build_formation_slot_card(ids, query, slot)));
    }
    row
}

fn build_formation_slot_card(
    ids: &mut IdAllocator,
    query: &dyn ChroniclerQuery,
    slot: &FormationSlotView,
) -> Card {
    let mut card = Card::new(ids.allocate(), slot.slot_label.clone());
    let occupant_label = if let Some(occ) = slot.occupant {
        // Render occupant id in the body text alongside the name —
        // useful for debugging without forcing the test fixture to
        // generate display names.
        let labels = query.labels_for_entity(EntityId::new(occ));
        let label_text = if labels.is_empty() {
            "labels: (none)".to_string()
        } else {
            // Sort by label id so the rendered string is order-stable
            // (chronicler returns labels in PatternSignature order;
            // re-sort by id here to lock in deterministic UI output
            // independent of any future signature-iteration tweak).
            // Named `label_strs` (not `ids`) to avoid shadowing the
            // outer `IdAllocator` parameter — same allocator is still
            // used at the `ids.allocate()` call below.
            let mut label_strs: Vec<&str> = labels.iter().map(|l| l.id.as_str()).collect();
            label_strs.sort_unstable();
            format!("labels: {}", label_strs.join(", "))
        };
        format!(
            "{}\nhp {:>3}% · stm {:>3}%\neng {:>3}% · exp {:>3}%\n{}",
            if slot.occupant_name.is_empty() {
                format!("(id {occ})")
            } else {
                slot.occupant_name.clone()
            },
            (slot.hp_pct.clamp(0.0, 1.0) * 100.0).round() as i32,
            (slot.stamina_pct.clamp(0.0, 1.0) * 100.0).round() as i32,
            (slot.engagement_pct.clamp(0.0, 1.0) * 100.0).round() as i32,
            (slot.exposure_pct.clamp(0.0, 1.0) * 100.0).round() as i32,
            label_text,
        )
    } else {
        "(empty)".to_string()
    };
    card.push_child(Box::new(Label::new(ids.allocate(), occupant_label)));
    card
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
    use crate::screens::data::{
        EncounterCreatureSnapshot, EncounterSnapshot, FormationSlotView, FormationView, KeeperView,
    };
    use beast_chronicler::query::InMemoryChronicler;
    use beast_chronicler::{Label, PatternSignature};
    use beast_core::{EntityId, Q3232};

    fn empty_chronicler() -> InMemoryChronicler {
        InMemoryChronicler::new()
    }

    fn populated_snapshot() -> EncounterSnapshot {
        let mut formation = FormationView::empty();
        // Put alpha in vanguard, beta in rear.
        formation.slots[0] = FormationSlotView {
            occupant: Some(1),
            occupant_name: "alpha".into(),
            slot_label: "vanguard".into(),
            hp_pct: 1.0,
            stamina_pct: 0.8,
            engagement_pct: 0.9,
            exposure_pct: 0.6,
        };
        formation.slots[4] = FormationSlotView {
            occupant: Some(2),
            occupant_name: "beta".into(),
            slot_label: "rear".into(),
            hp_pct: 0.5,
            stamina_pct: 0.6,
            engagement_pct: 0.1,
            exposure_pct: 0.3,
        };
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
            formation,
            keeper: KeeperView {
                name: "Captain".into(),
                leadership_pct: 0.75,
                stress_pct: 0.20,
            },
        }
    }

    #[test]
    fn encounter_screen_compiles_and_lays_out() {
        let chron = empty_chronicler();
        let snapshot = populated_snapshot();
        let mut tree = encounter(&chron, &snapshot);
        assert!(tree.layout(), "first layout should be a cache miss");
        let dump = dump_layout(&tree);
        assert!(dump.contains("RenderViewport#"), "dump:\n{dump}");
        assert!(dump.contains("List#"), "dump:\n{dump}");
        let buttons = dump.lines().filter(|l| l.starts_with("Button#")).count();
        assert_eq!(buttons, 3, "expected 3 action buttons, dump:\n{dump}");
        // Status-bar Card (from screen_frame) + 5 formation slot cards
        // + the "encounter" info card = 7 total.
        let cards = dump.lines().filter(|l| l.starts_with("Card#")).count();
        assert_eq!(
            cards, 7,
            "expected 7 cards (1 status + 5 slot + 1 info), dump:\n{dump}",
        );
    }

    #[test]
    fn empty_snapshot_renders_no_selection() {
        let chron = empty_chronicler();
        let snapshot = EncounterSnapshot::empty("ocean");
        let mut tree = encounter(&chron, &snapshot);
        assert!(tree.layout());
        let mut ctx = crate::paint::PaintCtx::new();
        tree.paint(&mut ctx);
        let has_no_selection = ctx.commands().iter().any(|cmd| match cmd {
            crate::paint::DrawCmd::Text { text, .. } => text.contains("(no selection)"),
            _ => false,
        });
        assert!(has_no_selection);
    }

    #[test]
    fn empty_snapshot_renders_five_empty_slot_cards() {
        // EncounterSnapshot::empty must produce a panic-safe screen
        // with the canonical five formation slots present (per the
        // S11.6 DoD: empty constructor stays panic-safe).
        let chron = empty_chronicler();
        let snapshot = EncounterSnapshot::empty("ocean");
        let mut tree = encounter(&chron, &snapshot);
        assert!(tree.layout());
        let dump = dump_layout(&tree);
        let cards = dump.lines().filter(|l| l.starts_with("Card#")).count();
        // 1 status bar + 5 slot cards + 1 info card = 7.
        assert_eq!(cards, 7, "dump:\n{dump}");
    }

    #[test]
    fn encounter_screen_paints_leadership_in_status() {
        // Pin the screen-frame status format so a refactor that drops
        // the leadership bar fails loudly.
        let chron = empty_chronicler();
        let snapshot = populated_snapshot();
        let mut tree = encounter(&chron, &snapshot);
        assert!(tree.layout());
        let mut ctx = crate::paint::PaintCtx::new();
        tree.paint(&mut ctx);
        let has_leadership = ctx.commands().iter().any(|cmd| match cmd {
            crate::paint::DrawCmd::Text { text, .. } => {
                text.contains("leadership: ") && text.contains("stress: ")
            }
            _ => false,
        });
        assert!(has_leadership, "leadership/stress missing from status");
    }

    #[test]
    fn formation_slot_renders_chronicler_labels() {
        // Set up an in-memory chronicler with one assigned label for
        // entity id 1 (alpha in vanguard). The slot card must surface
        // the label id in its rendered text — this is the chronicler
        // → UI hand-off the issue specifies.
        let mut chron = InMemoryChronicler::new();
        let sig = PatternSignature([7u8; 32]);
        chron.labels.insert(
            sig,
            Label {
                id: "echolocation".into(),
                signature: sig,
                confidence: Q3232::ONE,
            },
        );
        let mut entity_sigs = std::collections::BTreeSet::new();
        entity_sigs.insert(sig);
        chron
            .entity_signatures
            .insert(EntityId::new(1), entity_sigs);

        let snapshot = populated_snapshot();
        let mut tree = encounter(&chron, &snapshot);
        assert!(tree.layout());
        let mut ctx = crate::paint::PaintCtx::new();
        tree.paint(&mut ctx);
        let has_label = ctx.commands().iter().any(|cmd| match cmd {
            crate::paint::DrawCmd::Text { text, .. } => text.contains("labels: echolocation"),
            _ => false,
        });
        assert!(has_label, "echolocation label missing from slot card");
    }

    #[test]
    fn formation_slot_renders_no_labels_marker_when_chronicler_empty() {
        let chron = empty_chronicler();
        let snapshot = populated_snapshot();
        let mut tree = encounter(&chron, &snapshot);
        assert!(tree.layout());
        let mut ctx = crate::paint::PaintCtx::new();
        tree.paint(&mut ctx);
        let has_none = ctx.commands().iter().any(|cmd| match cmd {
            crate::paint::DrawCmd::Text { text, .. } => text.contains("labels: (none)"),
            _ => false,
        });
        assert!(has_none, "(none) label marker missing from slot card");
    }
}
