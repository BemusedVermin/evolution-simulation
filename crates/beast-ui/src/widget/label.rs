//! Static text label.
//!
//! [`Label`] is the simplest primitive: it holds a string and renders it.
//! It never consumes events.

use crate::event::{EventResult, UiEvent};
use crate::paint::{Color, PaintCtx, Point, Rect, Size};

use super::{Widget, WidgetId};

/// Static text widget. Doesn't react to input.
#[derive(Clone, Debug)]
pub struct Label {
    id: WidgetId,
    bounds: Rect,
    text: String,
    color: Color,
}

impl Label {
    /// Construct a label with the given text. Bounds default to zero —
    /// the layout pass (S10.2) is responsible for assigning real bounds.
    pub fn new(id: WidgetId, text: impl Into<String>) -> Self {
        Self {
            id,
            bounds: Rect::ZERO,
            text: text.into(),
            color: Color::BLACK,
        }
    }

    /// Override the default text color. Returns `self` for chaining.
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Replace the label's text.
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
    }

    /// Read the current text.
    pub fn text(&self) -> &str {
        &self.text
    }
}

impl Widget for Label {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn bounds(&self) -> Rect {
        self.bounds
    }

    fn set_bounds(&mut self, bounds: Rect) {
        self.bounds = bounds;
    }

    fn measure(&self) -> Size {
        // S10.2 will replace this with real text metrics. For S10.1 we
        // just return a constant-height row sized to a heuristic
        // 8px-per-char estimate so layout tests have *some* signal.
        Size::new(self.text.chars().count() as f32 * 8.0, 16.0)
    }

    fn paint(&self, ctx: &mut PaintCtx) {
        ctx.text(
            Point::new(self.bounds.origin.x, self.bounds.origin.y),
            self.text.clone(),
            self.color,
        );
    }

    fn handle_event(&mut self, _event: &UiEvent) -> EventResult {
        EventResult::Ignored
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::DrawCmd;
    use crate::widget::IdAllocator;

    #[test]
    fn label_paints_its_text() {
        let mut ids = IdAllocator::new();
        let mut label = Label::new(ids.allocate(), "hello");
        label.set_bounds(Rect::xywh(2.0, 3.0, 100.0, 16.0));
        let mut ctx = PaintCtx::new();
        label.paint(&mut ctx);
        match &ctx.commands()[0] {
            DrawCmd::Text { text, pos, .. } => {
                assert_eq!(text, "hello");
                assert_eq!(*pos, Point::new(2.0, 3.0));
            }
            other => panic!("expected text draw, got {other:?}"),
        }
    }

    #[test]
    fn label_ignores_all_events() {
        let mut ids = IdAllocator::new();
        let mut label = Label::new(ids.allocate(), "x");
        let result = label.handle_event(&UiEvent::Tick { dt_ms: 16 });
        assert_eq!(result, EventResult::Ignored);
    }

    #[test]
    fn measure_scales_with_text_length() {
        let mut ids = IdAllocator::new();
        let short = Label::new(ids.allocate(), "ab");
        let long = Label::new(ids.allocate(), "abcdefgh");
        assert!(long.measure().width > short.measure().width);
    }
}
