//! World-map screen (S10.4).
//!
//! Composition (top → bottom):
//!
//! 1. Status bar (shared frame) bound to `world.current_tick()` /
//!    `world.creature_count()` via [`crate::FnBinding`].
//! 2. Content row: a [`crate::RenderViewport`] on the left where the
//!    world-map renderer (S9.3) will draw the archipelago + creature
//!    glyphs, and a vertical action bar on the right with three
//!    buttons (Bestiary, Settings, Quit).
//! 3. A small status [`crate::Card`] under the action bar surfacing
//!    the world dimensions and creature count again — the status
//!    bar is full-width chrome; this card is on-screen documentation
//!    so the per-button context stays visible while the action bar
//!    receives focus.
//!
//! The screen is read-only: every binding takes `Fn() -> ...` and is
//! evaluated at paint time. No widget in the tree mutates `world` or
//! `biomes`.

use crate::layout::{Align, Axis};
use crate::screens::data::{BiomeView, WorldStatus};
use crate::screens::frame::screen_frame;
use crate::widget::{Button, Card, IdAllocator, Label, RenderViewport, Stack};
use crate::{Bound, FnBinding, Size, WidgetTree};

/// Width of the right-hand action bar column.
const ACTION_BAR_WIDTH: f32 = 200.0;

/// Build the world-map screen for a 1280×720 viewport.
///
/// Status values are *snapped* from `world` and `biomes` at call time
/// and captured by value into the status-bar / world-card closures.
/// Subsequent mutations of the source `world` are not reflected by
/// later paints — to refresh, rebuild the screen. A truly live
/// binding (e.g. `Rc<Cell<u64>>`) is planned for S13 once the
/// application layer wires real input handling; until then each
/// frame's status reflects the world handed to `world_map(..)`
/// at construction.
pub fn world_map(world: &dyn WorldStatus, biomes: &BiomeView) -> WidgetTree {
    let mut ids = IdAllocator::new();

    // Snapshot the values the status-bar binding needs. Capturing
    // owned `u64` / `usize` lets the closure live for `'static`
    // (which `FnBinding` requires) without holding a borrow into
    // `world`. Future stories that want a live binding swap this
    // closure for one that calls into the application's world
    // handle.
    let world_tick = world.current_tick();
    let world_creatures = world.creature_count();
    let biome_label = if biomes.is_empty() {
        "world: loading…".to_owned()
    } else {
        format!("world: {}×{}", biomes.width, biomes.height)
    };

    // Content row: viewport on the left, action bar on the right.
    let mut content_row = Stack::new(ids.allocate(), Axis::Horizontal).with_gap(8.0);

    let viewport = RenderViewport::new(ids.allocate())
        .with_preferred_size(Size::new(1280.0 - ACTION_BAR_WIDTH - 24.0, 660.0));
    content_row.push_child(Box::new(viewport));

    // Action bar — vertical Stack of three buttons + a status card.
    let mut action_bar = Stack::new(ids.allocate(), Axis::Vertical)
        .with_gap(8.0)
        .with_align(Align::Start);
    action_bar.push_child(Box::new(Button::new(ids.allocate(), "Bestiary")));
    action_bar.push_child(Box::new(Button::new(ids.allocate(), "Settings")));
    action_bar.push_child(Box::new(Button::new(ids.allocate(), "Quit")));

    // Status card under the action bar with read-only world summary.
    let mut status_card = Card::new(ids.allocate(), "world");
    let world_label = Label::new(ids.allocate(), "");
    status_card.push_child(Box::new(Bound::new(
        world_label,
        FnBinding::new(move || {
            format!("{biome_label}\ntick: {world_tick}\ncreatures: {world_creatures}")
        }),
    )));
    action_bar.push_child(Box::new(status_card));
    content_row.push_child(Box::new(action_bar));

    let frame = screen_frame(
        &mut ids,
        move || format!("tick: {world_tick} | creatures: {world_creatures}"),
        Box::new(content_row),
    );

    WidgetTree::new(Box::new(frame), Size::new(1280.0, 720.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dump_layout;

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

    #[test]
    fn world_map_screen_compiles_and_lays_out() {
        let world = FixedWorld {
            tick: 1024,
            creatures: 12,
        };
        let biomes = BiomeView::solid(64, 64, [40, 100, 50, 255]);
        let mut tree = world_map(&world, &biomes);
        assert!(tree.layout(), "first layout should be a cache miss");
        let dump = dump_layout(&tree);
        // Sanity checks: there's a RenderViewport, an action bar Stack
        // with three Buttons, and a status Card. We don't pin the
        // exact pre-order line set here — the dedicated snapshot test
        // (tests/screen_snapshots.rs) does that.
        assert!(
            dump.contains("RenderViewport#"),
            "world-map tree should contain a RenderViewport, got:\n{dump}"
        );
        let button_lines = dump
            .lines()
            .filter(|line| line.starts_with("Button#"))
            .count();
        assert_eq!(
            button_lines, 3,
            "expected 3 action-bar buttons, dump:\n{dump}"
        );
    }

    #[test]
    fn empty_biome_view_renders_loading_status() {
        let world = FixedWorld {
            tick: 0,
            creatures: 0,
        };
        let biomes = BiomeView::empty();
        let mut tree = world_map(&world, &biomes);
        assert!(tree.layout());
        let mut ctx = crate::paint::PaintCtx::new();
        tree.paint(&mut ctx);
        let has_loading = ctx.commands().iter().any(|cmd| match cmd {
            crate::paint::DrawCmd::Text { text, .. } => text.contains("loading"),
            _ => false,
        });
        assert!(
            has_loading,
            "empty biome view should surface a loading status"
        );
    }
}
