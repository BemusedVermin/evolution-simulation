//! Bestiary screen (S10.4).
//!
//! Composition (top → bottom):
//!
//! 1. Status bar with the entry count.
//! 2. Filter `Card` documenting which view is active (discovered_only,
//!    sort_by, search). The filter values are baked into the screen at
//!    build time — interactive filter editing is out of scope for
//!    S10.4 (lands when the input bindings reach the application
//!    layer in S13).
//! 3. [`BestiaryPanel`] — the list-plus-detail composite widget that
//!    keeps the right-hand detail [`Card`] in sync with the
//!    left-hand list selection.
//!
//! Per `documentation/INVARIANTS.md` §6, the screen never persists a
//! `discovered` flag; the filter applies the underlying
//! `observation_count >= 1` rule via [`BestiaryFilter::discovered_only`].

use beast_chronicler::{BestiaryEntry, BestiaryFilter, BestiarySortBy, ChroniclerQuery, SpeciesId};

use crate::event::{EventResult, UiEvent};
use crate::layout::{Axis, LayoutConstraints};
use crate::paint::{Color, PaintCtx, Point, Rect, Size};
use crate::screens::frame::screen_frame;
use crate::widget::{Card, IdAllocator, Label, LayoutCtx, List, ListItem, Stack, Widget, WidgetId};
use crate::WidgetTree;

/// Width reserved for the bestiary list column.
const LIST_WIDTH: f32 = 280.0;

/// Build the bestiary screen for a 1280×720 viewport.
///
/// The `query` reference need only outlive the construction call —
/// the snapshot of bestiary entries is collected eagerly and the
/// screen is fully self-contained afterwards. This matches the
/// `&dyn ChroniclerQuery` contract from
/// `documentation/systems/23_ui_overview.md` §4.1.
pub fn bestiary(query: &dyn ChroniclerQuery) -> WidgetTree {
    let mut ids = IdAllocator::new();

    let filter = BestiaryFilter {
        discovered_only: false,
        sort_by: BestiarySortBy::Confidence,
        search: None,
    };
    let entries = query.bestiary_entries(&filter);
    let entry_count = entries.len();

    let mut filter_card = Card::new(ids.allocate(), "filter");
    let filter_text = format!(
        "discovered_only: {}\nsort_by: {:?}\nsearch: {}",
        filter.discovered_only,
        filter.sort_by,
        filter.search.as_deref().unwrap_or("(none)"),
    );
    filter_card.push_child(Box::new(Label::new(ids.allocate(), filter_text)));

    let panel = BestiaryPanel::new(&mut ids, entries);

    let mut content_row = Stack::new(ids.allocate(), Axis::Horizontal).with_gap(8.0);
    content_row.push_child(Box::new(panel));
    content_row.push_child(Box::new(filter_card));

    let frame = screen_frame(
        &mut ids,
        move || format!("bestiary | entries: {entry_count}"),
        Box::new(content_row),
    );

    WidgetTree::new(Box::new(frame), Size::new(1280.0, 720.0))
}

/// Composite list-plus-detail widget used by the bestiary screen.
///
/// Public so the integration test fixture in
/// `tests/screen_bindings.rs` can reach in and assert on the detail
/// label after dispatching a list-selection event. Constructed via
/// `BestiaryPanel::new`; consumers shouldn't construct one by hand
/// outside of tests because the internal id allocation must come
/// from the same [`IdAllocator`] the surrounding tree uses.
pub struct BestiaryPanel {
    id: WidgetId,
    bounds: Rect,
    list: List<SpeciesId>,
    detail_id: WidgetId,
    detail_bounds: Rect,
    detail_texts: Vec<String>,
    last_selected: Option<usize>,
}

impl std::fmt::Debug for BestiaryPanel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BestiaryPanel")
            .field("id", &self.id)
            .field("bounds", &self.bounds)
            .field("entries", &self.detail_texts.len())
            .field("last_selected", &self.last_selected)
            .finish()
    }
}

impl BestiaryPanel {
    /// Construct a panel from a snapshot of bestiary entries.
    ///
    /// The detail label starts on `"select an entry"`. As soon as the
    /// list selection lands on a row, [`Widget::handle_event`]
    /// re-syncs the detail text with that row's pre-formatted
    /// summary.
    pub fn new(ids: &mut IdAllocator, entries: Vec<BestiaryEntry>) -> Self {
        let panel_id = ids.allocate();
        let list_id = ids.allocate();
        let detail_id = ids.allocate();

        let mut list: List<SpeciesId> = List::new(list_id);
        let detail_texts: Vec<String> = entries.iter().map(format_detail).collect();
        let list_items: Vec<ListItem<SpeciesId>> = entries
            .iter()
            .map(|entry| {
                let primary = entry
                    .label_ids
                    .first()
                    .cloned()
                    .unwrap_or_else(|| format!("species #{}", entry.species.raw()));
                let label = format!("{primary} · obs {}", entry.observation_count);
                ListItem::new(label, entry.species)
            })
            .collect();
        list.set_items(list_items);

        Self {
            id: panel_id,
            bounds: Rect::ZERO,
            list,
            detail_id,
            detail_bounds: Rect::ZERO,
            detail_texts,
            last_selected: None,
        }
    }

    /// Read the detail panel's current text. Tests use this to
    /// verify selection-driven updates without poking at internal
    /// widget state.
    pub fn detail_text(&self) -> &str {
        match self.last_selected {
            Some(i) => self
                .detail_texts
                .get(i)
                .map(String::as_str)
                .unwrap_or("select an entry"),
            None => "select an entry",
        }
    }

    /// Read-only access to the embedded list — exposed for the
    /// integration test that programmatically sets the selection.
    pub fn list(&self) -> &List<SpeciesId> {
        &self.list
    }

    /// Mutable access to the embedded list. Tests use it to drive
    /// selection without simulating the full mouse-down + cursor
    /// hit-test pipeline.
    pub fn list_mut(&mut self) -> &mut List<SpeciesId> {
        &mut self.list
    }
}

impl Widget for BestiaryPanel {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn bounds(&self) -> Rect {
        self.bounds
    }

    fn set_bounds(&mut self, bounds: Rect) {
        self.bounds = bounds;
    }

    fn measure(&self, ctx: &LayoutCtx) -> Size {
        let list = self.list.measure(ctx);
        // Reserve a default detail-panel size — actual width is
        // assigned by `layout` from the constraint envelope.
        Size::new(LIST_WIDTH + 8.0 + 400.0, list.height.max(120.0))
    }

    fn layout(&mut self, ctx: &LayoutCtx, constraints: LayoutConstraints) -> Size {
        let final_size = constraints.constrain(Size::new(1080.0, 600.0));
        let height = final_size.height;
        let detail_width = (final_size.width - LIST_WIDTH - 8.0).max(0.0);

        let list_rect = Rect::xywh(
            self.bounds.origin.x,
            self.bounds.origin.y,
            LIST_WIDTH,
            height,
        );
        self.list.set_bounds(list_rect);
        let _: Size = self
            .list
            .layout(ctx, LayoutConstraints::tight(list_rect.size));

        self.detail_bounds = Rect::xywh(
            self.bounds.origin.x + LIST_WIDTH + 8.0,
            self.bounds.origin.y,
            detail_width,
            height,
        );

        final_size
    }

    fn paint(&self, ctx: &mut PaintCtx) {
        // List paints itself.
        self.list.paint(ctx);

        // Detail card chrome + live text. We render the detail
        // ourselves rather than going through a `Bound<Label, _>`
        // wrapper so the text is *always* the panel's current
        // selection, even if the surrounding screen ignores the
        // selection-update event the panel emits.
        ctx.fill_rect(self.detail_bounds, Color::WHITE);
        ctx.stroke_rect(self.detail_bounds, Color::BLACK);
        let title_rect = Rect::new(
            self.detail_bounds.origin,
            Size::new(self.detail_bounds.size.width, 20.0),
        );
        ctx.fill_rect(title_rect, Color::rgb(0.85, 0.85, 0.92));
        ctx.text(
            Point::new(
                self.detail_bounds.origin.x + 4.0,
                self.detail_bounds.origin.y + 2.0,
            ),
            "detail",
            Color::BLACK,
        );
        ctx.text(
            Point::new(
                self.detail_bounds.origin.x + 4.0,
                self.detail_bounds.origin.y + 24.0,
            ),
            self.detail_text(),
            Color::BLACK,
        );
    }

    fn handle_event(&mut self, event: &UiEvent) -> EventResult {
        let result = self.list.handle_event(event);
        // Re-derive selection after every event. Cheap (one index
        // copy) and avoids duplicating selection state between the
        // List and the panel.
        self.last_selected = self.list.selected_index();
        result
    }

    fn visit_pre_order<'a>(&'a self, visitor: &mut dyn FnMut(&'a dyn Widget)) {
        // Pre-order: panel itself, then list. The detail surface is
        // not a separate widget object — its bounds live on the
        // panel — so we don't recurse into a synthetic child here.
        // The dump_layout snapshot still records the panel itself
        // plus the list line; the detail bounds aren't separately
        // visible, which is fine because the snapshot only pins
        // the things the layout pass *positions* as widgets.
        visitor(self);
        self.list.visit_pre_order(visitor);
    }

    fn kind(&self) -> &'static str {
        "BestiaryPanel"
    }

    fn collect_focus_chain(&self, out: &mut Vec<WidgetId>) {
        self.list.collect_focus_chain(out);
    }

    fn find_widget_mut(&mut self, id: WidgetId) -> Option<&mut dyn Widget> {
        if self.id == id {
            return Some(self);
        }
        if id == self.detail_id {
            // The detail panel is rendered by `BestiaryPanel::paint`
            // directly; there is no separate widget object whose
            // mutable handle we could return. Returning `None`
            // matches "no separate widget" — tests that want to
            // assert the detail text use `Self::detail_text`
            // instead.
            return None;
        }
        self.list.find_widget_mut(id)
    }
}

/// Format a [`BestiaryEntry`] for the detail card.
///
/// Multi-line summary: a header with the species id and the highest
/// confidence label, followed by the observation count and first
/// tick. Confidence is rendered via `Q3232::Display` so the value
/// stays readable for tests without depending on platform-specific
/// floating-point formatting.
fn format_detail(entry: &BestiaryEntry) -> String {
    let primary = entry
        .label_ids
        .first()
        .cloned()
        .unwrap_or_else(|| format!("species #{}", entry.species.raw()));
    let labels = if entry.label_ids.is_empty() {
        "(no labels)".to_owned()
    } else {
        entry.label_ids.join(", ")
    };
    format!(
        "{primary}\nlabels: {labels}\nobservations: {}\nfirst tick: {}\nconfidence: {}",
        entry.observation_count,
        entry.first_tick.raw(),
        entry.confidence,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dump_layout;
    use beast_chronicler::{InMemoryChronicler, SpeciesId};
    use beast_core::{TickCounter, Q3232};

    fn populated_query() -> InMemoryChronicler {
        let mut c = InMemoryChronicler::new();
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

    #[test]
    fn bestiary_screen_compiles_and_lays_out() {
        let chronicler = populated_query();
        let mut tree = bestiary(&chronicler);
        assert!(tree.layout(), "first layout pass should be a cache miss");
        let dump = dump_layout(&tree);
        assert!(dump.contains("BestiaryPanel#"), "dump:\n{dump}");
        assert!(dump.contains("List#"), "dump:\n{dump}");
        let cards = dump.lines().filter(|l| l.starts_with("Card#")).count();
        assert!(
            cards >= 2,
            "expected 2+ Cards (status + filter), dump:\n{dump}"
        );
    }

    #[test]
    fn detail_text_updates_after_selection_change() {
        let mut ids = IdAllocator::new();
        let entries = vec![
            BestiaryEntry {
                species: SpeciesId::new(7),
                label_ids: vec!["echolocation".into()],
                observation_count: 50,
                confidence: Q3232::from_num(0.9),
                first_tick: TickCounter::new(3),
            },
            BestiaryEntry {
                species: SpeciesId::new(11),
                label_ids: vec!["pack_hunting".into()],
                observation_count: 12,
                confidence: Q3232::from_num(0.4),
                first_tick: TickCounter::new(20),
            },
        ];
        let mut panel = BestiaryPanel::new(&mut ids, entries);
        assert_eq!(panel.detail_text(), "select an entry");

        panel
            .list_mut()
            .set_bounds(Rect::xywh(0.0, 0.0, 280.0, 600.0));
        panel.list_mut().set_selected(Some(0));
        let _ = panel.handle_event(&UiEvent::Tick { dt_ms: 16 });
        assert!(
            panel.detail_text().contains("echolocation"),
            "detail should reflect first selection, got: {}",
            panel.detail_text()
        );

        panel.list_mut().set_selected(Some(1));
        let _ = panel.handle_event(&UiEvent::Tick { dt_ms: 16 });
        assert!(
            panel.detail_text().contains("pack_hunting"),
            "detail should reflect second selection, got: {}",
            panel.detail_text()
        );
    }

    #[test]
    fn empty_bestiary_renders_zero_count() {
        let chronicler = InMemoryChronicler::new();
        let mut tree = bestiary(&chronicler);
        assert!(tree.layout());
        let mut ctx = PaintCtx::new();
        tree.paint(&mut ctx);
        let zero_count = ctx.commands().iter().any(|cmd| match cmd {
            crate::paint::DrawCmd::Text { text, .. } => text.contains("entries: 0"),
            _ => false,
        });
        assert!(zero_count);
    }
}
