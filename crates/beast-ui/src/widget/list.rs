//! Selectable scrollable list of items.
//!
//! S10.1 implements the data model + selection mutation + a basic paint
//! pass. Real scrolling, virtualization, and filtering land alongside
//! S10.4 screens; this story keeps the surface minimal so the bestiary
//! can later replace items + selection without re-allocating the list.

use crate::event::{EventResult, KeyCode, MouseButton, UiEvent};
use crate::paint::{Color, PaintCtx, Point, Rect, Size};

use super::{LayoutCtx, Widget, WidgetId};

/// One row in a [`List`]. Concrete payload is generic so callers can store
/// bestiary entries, faction summaries, etc., without erasing types.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListItem<T> {
    /// Display label.
    pub label: String,
    /// Caller-attached payload retrieved via [`List::selected`].
    pub payload: T,
}

impl<T> ListItem<T> {
    /// Construct a new list item.
    pub fn new(label: impl Into<String>, payload: T) -> Self {
        Self {
            label: label.into(),
            payload,
        }
    }
}

const ROW_HEIGHT: f32 = 20.0;

/// Selectable list of `ListItem<T>`s.
#[derive(Clone, Debug)]
pub struct List<T> {
    id: WidgetId,
    bounds: Rect,
    items: Vec<ListItem<T>>,
    selected: Option<usize>,
    last_cursor_y: Option<f32>,
}

impl<T> List<T> {
    /// Construct an empty list.
    pub fn new(id: WidgetId) -> Self {
        Self {
            id,
            bounds: Rect::ZERO,
            items: Vec::new(),
            selected: None,
            last_cursor_y: None,
        }
    }

    /// Replace the entire item set. If the previously selected index is
    /// out of range it is cleared.
    pub fn set_items(&mut self, items: Vec<ListItem<T>>) {
        let len = items.len();
        self.items = items;
        if let Some(i) = self.selected {
            if i >= len {
                self.selected = None;
            }
        }
    }

    /// Append a single item.
    pub fn push(&mut self, item: ListItem<T>) {
        self.items.push(item);
    }

    /// Currently selected index, if any.
    pub fn selected_index(&self) -> Option<usize> {
        self.selected
    }

    /// Currently selected item, if any.
    pub fn selected(&self) -> Option<&ListItem<T>> {
        self.selected.and_then(|i| self.items.get(i))
    }

    /// Set the selected index. Out-of-range values clear the selection
    /// rather than panicking.
    pub fn set_selected(&mut self, index: Option<usize>) {
        self.selected = match index {
            Some(i) if i < self.items.len() => Some(i),
            _ => None,
        };
    }

    /// Number of items.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// True if the list has zero items.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Borrow the items for read-only iteration.
    pub fn items(&self) -> &[ListItem<T>] {
        &self.items
    }

    fn row_index_at(&self, y: f32) -> Option<usize> {
        if !self.bounds.contains(Point::new(self.bounds.origin.x, y)) {
            return None;
        }
        let local = y - self.bounds.origin.y;
        let idx = (local / ROW_HEIGHT).floor() as i64;
        if idx < 0 {
            return None;
        }
        let idx = idx as usize;
        if idx < self.items.len() {
            Some(idx)
        } else {
            None
        }
    }
}

impl<T: Clone + std::fmt::Debug> Widget for List<T> {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn bounds(&self) -> Rect {
        self.bounds
    }

    fn set_bounds(&mut self, bounds: Rect) {
        self.bounds = bounds;
    }

    fn measure(&self, _ctx: &LayoutCtx) -> Size {
        Size::new(120.0, ROW_HEIGHT * self.items.len() as f32)
    }

    fn paint(&self, ctx: &mut PaintCtx) {
        ctx.fill_rect(self.bounds, Color::rgb(0.95, 0.95, 0.95));
        for (i, item) in self.items.iter().enumerate() {
            let row_origin = Point::new(
                self.bounds.origin.x,
                self.bounds.origin.y + i as f32 * ROW_HEIGHT,
            );
            let row_rect = Rect::new(row_origin, Size::new(self.bounds.size.width, ROW_HEIGHT));
            if Some(i) == self.selected {
                ctx.fill_rect(row_rect, Color::rgba(0.3, 0.5, 0.9, 0.4));
            }
            ctx.text(
                Point::new(row_origin.x + 4.0, row_origin.y + 2.0),
                item.label.as_str(),
                Color::BLACK,
            );
        }
    }

    fn handle_event(&mut self, event: &UiEvent) -> EventResult {
        match event {
            UiEvent::MouseMove { y, .. } => {
                self.last_cursor_y = Some(*y);
                EventResult::Ignored
            }
            UiEvent::MouseDown {
                button: MouseButton::Primary,
            } => {
                let Some(y) = self.last_cursor_y else {
                    return EventResult::Ignored;
                };
                if let Some(idx) = self.row_index_at(y) {
                    self.selected = Some(idx);
                    EventResult::Consumed
                } else {
                    EventResult::Ignored
                }
            }
            UiEvent::KeyDown(modifiers) => match modifiers.key {
                KeyCode::ArrowDown if !self.items.is_empty() => {
                    let next = match self.selected {
                        Some(i) if i + 1 < self.items.len() => i + 1,
                        Some(i) => i,
                        None => 0,
                    };
                    self.selected = Some(next);
                    EventResult::Consumed
                }
                KeyCode::ArrowUp if !self.items.is_empty() => {
                    let next = match self.selected {
                        Some(0) | None => 0,
                        Some(i) => i - 1,
                    };
                    self.selected = Some(next);
                    EventResult::Consumed
                }
                _ => EventResult::Ignored,
            },
            _ => EventResult::Ignored,
        }
    }

    fn visit_pre_order<'a>(&'a self, visitor: &mut dyn FnMut(&'a dyn Widget)) {
        visitor(self);
    }

    fn kind(&self) -> &'static str {
        "List"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{KeyMods, Modifiers};
    use crate::widget::IdAllocator;

    fn fixture() -> List<u32> {
        let mut ids = IdAllocator::new();
        let mut list = List::new(ids.allocate());
        list.set_bounds(Rect::xywh(0.0, 0.0, 100.0, 100.0));
        list.set_items(vec![
            ListItem::new("alpha", 0),
            ListItem::new("beta", 1),
            ListItem::new("gamma", 2),
        ]);
        list
    }

    #[test]
    fn click_selects_row_under_cursor() {
        let mut list = fixture();
        list.handle_event(&UiEvent::MouseMove { x: 5.0, y: 25.0 });
        let r = list.handle_event(&UiEvent::MouseDown {
            button: MouseButton::Primary,
        });
        assert_eq!(r, EventResult::Consumed);
        assert_eq!(list.selected_index(), Some(1));
        assert_eq!(list.selected().map(|i| i.payload), Some(1));
    }

    #[test]
    fn click_below_last_row_does_not_select() {
        let mut list = fixture();
        list.handle_event(&UiEvent::MouseMove { x: 5.0, y: 80.0 });
        let r = list.handle_event(&UiEvent::MouseDown {
            button: MouseButton::Primary,
        });
        assert_eq!(r, EventResult::Ignored);
        assert_eq!(list.selected_index(), None);
    }

    #[test]
    fn arrow_keys_move_selection() {
        let mut list = fixture();
        list.set_selected(Some(0));
        let r = list.handle_event(&UiEvent::KeyDown(Modifiers {
            key: KeyCode::ArrowDown,
            mods: KeyMods::NONE,
        }));
        assert_eq!(r, EventResult::Consumed);
        assert_eq!(list.selected_index(), Some(1));
        list.handle_event(&UiEvent::KeyDown(Modifiers {
            key: KeyCode::ArrowDown,
            mods: KeyMods::NONE,
        }));
        list.handle_event(&UiEvent::KeyDown(Modifiers {
            key: KeyCode::ArrowDown,
            mods: KeyMods::NONE,
        }));
        // Pinned at the last row, never wraps.
        assert_eq!(list.selected_index(), Some(2));
        list.handle_event(&UiEvent::KeyDown(Modifiers {
            key: KeyCode::ArrowUp,
            mods: KeyMods::NONE,
        }));
        assert_eq!(list.selected_index(), Some(1));
    }

    #[test]
    fn replacing_items_clears_stale_selection() {
        let mut list = fixture();
        list.set_selected(Some(2));
        list.set_items(vec![ListItem::new("only", 99)]);
        assert_eq!(list.selected_index(), None);
    }

    #[test]
    fn paint_highlights_selected_row() {
        let mut list = fixture();
        list.set_selected(Some(1));
        let mut ctx = PaintCtx::new();
        list.paint(&mut ctx);
        // Background fill, selection fill, three text rows = five commands.
        assert_eq!(ctx.commands().len(), 5);
    }
}
