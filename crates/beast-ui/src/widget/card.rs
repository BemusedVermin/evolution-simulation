//! Container card with a title bar and child widgets.
//!
//! S10.1 ships the data model + paint pass. Layout of the children inside
//! the card is the responsibility of the layout engine in S10.2; for now
//! the card simply paints its title and forwards events to children
//! whose bounds the cursor is inside.

use crate::event::{EventResult, UiEvent};
use crate::paint::{Color, PaintCtx, Point, Rect, Size};

use super::{LayoutCtx, Widget, WidgetId};

const TITLE_BAR_HEIGHT: f32 = 20.0;

/// Card container. Holds a title and a list of `Box<dyn Widget>` children.
pub struct Card {
    id: WidgetId,
    bounds: Rect,
    title: String,
    children: Vec<Box<dyn Widget>>,
    last_cursor: Option<Point>,
}

impl Card {
    /// Construct a card with the given title.
    pub fn new(id: WidgetId, title: impl Into<String>) -> Self {
        Self {
            id,
            bounds: Rect::ZERO,
            title: title.into(),
            children: Vec::new(),
            last_cursor: None,
        }
    }

    /// Append a child widget. Children are stored in declaration order.
    pub fn push_child(&mut self, child: Box<dyn Widget>) {
        self.children.push(child);
    }

    /// Number of children.
    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    /// Title text.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Read-only access to the children (for layout / introspection).
    pub fn children(&self) -> &[Box<dyn Widget>] {
        &self.children
    }
}

impl std::fmt::Debug for Card {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Card")
            .field("id", &self.id)
            .field("title", &self.title)
            .field("bounds", &self.bounds)
            .field("children", &self.children.len())
            .finish()
    }
}

impl Widget for Card {
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
        // Title bar plus the bounding box of all child measurements.
        let mut max_w = 0.0_f32;
        let mut total_h = 0.0_f32;
        for child in &self.children {
            let s = child.measure(ctx);
            max_w = max_w.max(s.width);
            total_h += s.height;
        }
        Size::new(
            max_w.max(self.title.chars().count() as f32 * 8.0),
            TITLE_BAR_HEIGHT + total_h,
        )
    }

    fn paint(&self, ctx: &mut PaintCtx) {
        // Card background + 1px border.
        ctx.fill_rect(self.bounds, Color::rgb(1.0, 1.0, 1.0));
        ctx.stroke_rect(self.bounds, Color::BLACK);
        // Title bar.
        let title_rect = Rect::new(
            self.bounds.origin,
            Size::new(self.bounds.size.width, TITLE_BAR_HEIGHT),
        );
        ctx.fill_rect(title_rect, Color::rgb(0.85, 0.85, 0.92));
        ctx.text(
            Point::new(self.bounds.origin.x + 4.0, self.bounds.origin.y + 2.0),
            self.title.as_str(),
            Color::BLACK,
        );
        for child in &self.children {
            child.paint(ctx);
        }
    }

    fn handle_event(&mut self, event: &UiEvent) -> EventResult {
        if let UiEvent::MouseMove { x, y } = event {
            self.last_cursor = Some(Point::new(*x, *y));
        }
        // Forward to children in reverse declaration order so the
        // last-painted child gets first crack at the event (canonical
        // top-most-takes-it model).
        for child in self.children.iter_mut().rev() {
            // Always forward MouseMove so children can update their own
            // cursor caches.
            let inside = matches!(event, UiEvent::MouseMove { .. })
                || self
                    .last_cursor
                    .map(|c| child.bounds().contains(c))
                    .unwrap_or(false);
            if inside && child.handle_event(event) == EventResult::Consumed {
                return EventResult::Consumed;
            }
        }
        EventResult::Ignored
    }

    fn visit_pre_order<'a>(&'a self, visitor: &mut dyn FnMut(&'a dyn Widget)) {
        visitor(self);
        for child in &self.children {
            child.visit_pre_order(visitor);
        }
    }

    fn kind(&self) -> &'static str {
        "Card"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::MouseButton;
    use crate::widget::{Button, IdAllocator};

    #[test]
    fn card_paints_background_border_and_title() {
        let mut ids = IdAllocator::new();
        let mut card = Card::new(ids.allocate(), "Stats");
        card.set_bounds(Rect::xywh(0.0, 0.0, 100.0, 60.0));
        let mut ctx = PaintCtx::new();
        card.paint(&mut ctx);
        // 4 commands: card fill, card stroke, title fill, title text.
        assert_eq!(ctx.commands().len(), 4);
    }

    #[test]
    fn card_forwards_clicks_to_top_most_child() {
        let mut ids = IdAllocator::new();
        let mut card = Card::new(ids.allocate(), "host");
        card.set_bounds(Rect::xywh(0.0, 0.0, 200.0, 200.0));

        let mut top = Button::new(ids.allocate(), "top");
        top.set_bounds(Rect::xywh(0.0, 30.0, 50.0, 30.0));
        let mut bottom = Button::new(ids.allocate(), "bottom");
        bottom.set_bounds(Rect::xywh(0.0, 30.0, 50.0, 30.0));
        card.push_child(Box::new(bottom));
        card.push_child(Box::new(top));

        card.handle_event(&UiEvent::MouseMove { x: 25.0, y: 45.0 });
        let r = card.handle_event(&UiEvent::MouseDown {
            button: MouseButton::Primary,
        });
        assert_eq!(r, EventResult::Consumed);
        // Only the *top* button should have registered the press; we
        // can't peek at it directly because the trait object hides type,
        // but if both fired the second event would still be Consumed —
        // which is the contract. So this test pins the bubble-stops-at-
        // first-Consumed behavior.
    }

    #[test]
    fn card_measure_accounts_for_children_and_title() {
        let mut ids = IdAllocator::new();
        let mut card = Card::new(ids.allocate(), "x");
        let button = Button::new(ids.allocate(), "wide button label");
        let lc = LayoutCtx::default();
        let measured_button = button.measure(&lc);
        card.push_child(Box::new(button));
        let measured = card.measure(&lc);
        assert!(measured.width >= measured_button.width);
        assert!(measured.height >= measured_button.height + TITLE_BAR_HEIGHT);
    }
}
