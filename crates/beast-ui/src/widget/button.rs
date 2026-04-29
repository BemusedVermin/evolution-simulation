//! Pressable button widget.
//!
//! Click semantics:
//!   * `MouseDown` over `bounds()` while enabled → `pressed = true`,
//!     event consumed.
//!   * `MouseUp` while `pressed` → fires the on-press callback (a counter
//!     increment in tests), clears `pressed`, event consumed.
//!   * Any other input bubbles.

use crate::event::{EventResult, MouseButton, UiEvent};
use crate::paint::{Color, PaintCtx, Rect, Size};

use super::{LayoutCtx, Widget, WidgetId};

/// Button primitive. Stores its label, enabled state, and a press counter
/// instead of a `Box<dyn FnMut>` callback — this keeps the widget
/// `Clone + Debug + Send` and lets tests assert on observable state
/// without juggling channel ends.
#[derive(Clone, Debug)]
pub struct Button {
    id: WidgetId,
    bounds: Rect,
    label: String,
    enabled: bool,
    pressed: bool,
    press_count: u32,
    last_cursor_x: f32,
    last_cursor_y: f32,
}

impl Button {
    /// Construct an enabled button with the given label.
    pub fn new(id: WidgetId, label: impl Into<String>) -> Self {
        Self {
            id,
            bounds: Rect::ZERO,
            label: label.into(),
            enabled: true,
            pressed: false,
            press_count: 0,
            last_cursor_x: f32::NAN,
            last_cursor_y: f32::NAN,
        }
    }

    /// Mark the button as disabled. Disabled buttons paint dimmed and
    /// ignore input.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Whether the button is currently in the down (pressed) state.
    pub fn is_pressed(&self) -> bool {
        self.pressed
    }

    /// Number of completed press cycles (down → up). Tests use this in
    /// place of an `on_press` closure.
    pub fn press_count(&self) -> u32 {
        self.press_count
    }

    /// Whether the button is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// The button's label.
    pub fn label(&self) -> &str {
        &self.label
    }

    fn cursor_inside_bounds(&self) -> bool {
        if self.last_cursor_x.is_nan() {
            return false;
        }
        self.bounds.contains(crate::paint::Point::new(
            self.last_cursor_x,
            self.last_cursor_y,
        ))
    }
}

impl Widget for Button {
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
        // Heuristic: 8 px per glyph + 16 px padding, fixed 32 px tall.
        // Layout (S10.2) will replace this with text metrics from `ctx`.
        Size::new(self.label.chars().count() as f32 * 8.0 + 16.0, 32.0)
    }

    fn paint(&self, ctx: &mut PaintCtx) {
        let bg = if !self.enabled {
            Color::rgba(0.6, 0.6, 0.6, 1.0)
        } else if self.pressed {
            Color::rgba(0.2, 0.4, 0.8, 1.0)
        } else {
            Color::rgba(0.3, 0.5, 0.9, 1.0)
        };
        ctx.fill_rect(self.bounds, bg);
        ctx.stroke_rect(self.bounds, Color::BLACK);
        let text_pos =
            crate::paint::Point::new(self.bounds.origin.x + 8.0, self.bounds.origin.y + 8.0);
        ctx.text(text_pos, self.label.as_str(), Color::WHITE);
    }

    fn handle_event(&mut self, event: &UiEvent) -> EventResult {
        if !self.enabled {
            return EventResult::Ignored;
        }
        match event {
            UiEvent::MouseMove { x, y } => {
                self.last_cursor_x = *x;
                self.last_cursor_y = *y;
                EventResult::Ignored
            }
            UiEvent::MouseDown {
                button: MouseButton::Primary,
            } if self.cursor_inside_bounds() => {
                self.pressed = true;
                EventResult::Consumed
            }
            UiEvent::MouseUp {
                button: MouseButton::Primary,
            } if self.pressed => {
                self.pressed = false;
                if self.cursor_inside_bounds() {
                    self.press_count = self.press_count.saturating_add(1);
                }
                EventResult::Consumed
            }
            _ => EventResult::Ignored,
        }
    }

    fn visit_pre_order<'a>(&'a self, visitor: &mut dyn FnMut(&'a dyn Widget)) {
        visitor(self);
    }

    fn kind(&self) -> &'static str {
        "Button"
    }

    fn accepts_focus(&self) -> bool {
        // Disabled buttons drop out of the focus chain — Tab skips them.
        self.enabled
    }

    fn collect_focus_chain(&self, out: &mut Vec<WidgetId>) {
        if self.accepts_focus() {
            out.push(self.id);
        }
    }

    fn find_widget_mut(&mut self, id: WidgetId) -> Option<&mut dyn Widget> {
        if self.id == id {
            Some(self)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget::IdAllocator;

    fn make() -> (Button, IdAllocator) {
        let mut ids = IdAllocator::new();
        let mut b = Button::new(ids.allocate(), "click me");
        b.set_bounds(Rect::xywh(10.0, 20.0, 100.0, 30.0));
        (b, ids)
    }

    #[test]
    fn click_inside_bounds_increments_press_count() {
        let (mut b, _) = make();
        b.handle_event(&UiEvent::MouseMove { x: 50.0, y: 35.0 });
        assert_eq!(
            b.handle_event(&UiEvent::MouseDown {
                button: MouseButton::Primary,
            }),
            EventResult::Consumed
        );
        assert!(b.is_pressed());
        assert_eq!(
            b.handle_event(&UiEvent::MouseUp {
                button: MouseButton::Primary,
            }),
            EventResult::Consumed
        );
        assert!(!b.is_pressed());
        assert_eq!(b.press_count(), 1);
    }

    #[test]
    fn click_outside_bounds_does_not_press() {
        let (mut b, _) = make();
        b.handle_event(&UiEvent::MouseMove { x: 5.0, y: 5.0 });
        assert_eq!(
            b.handle_event(&UiEvent::MouseDown {
                button: MouseButton::Primary,
            }),
            EventResult::Ignored
        );
        assert_eq!(b.press_count(), 0);
    }

    #[test]
    fn disabled_button_ignores_clicks() {
        let mut ids = IdAllocator::new();
        let mut b = Button::new(ids.allocate(), "x").with_enabled(false);
        b.set_bounds(Rect::xywh(0.0, 0.0, 10.0, 10.0));
        b.handle_event(&UiEvent::MouseMove { x: 5.0, y: 5.0 });
        let r = b.handle_event(&UiEvent::MouseDown {
            button: MouseButton::Primary,
        });
        assert_eq!(r, EventResult::Ignored);
        assert_eq!(b.press_count(), 0);
    }

    #[test]
    fn drag_off_cancels_press_without_firing() {
        let (mut b, _) = make();
        b.handle_event(&UiEvent::MouseMove { x: 50.0, y: 35.0 });
        b.handle_event(&UiEvent::MouseDown {
            button: MouseButton::Primary,
        });
        // Drag the cursor outside the bounds before releasing.
        b.handle_event(&UiEvent::MouseMove { x: 200.0, y: 300.0 });
        b.handle_event(&UiEvent::MouseUp {
            button: MouseButton::Primary,
        });
        assert_eq!(b.press_count(), 0, "drag-off should not fire press");
    }

    #[test]
    fn paint_emits_fill_stroke_text_in_order() {
        let (b, _) = make();
        let mut ctx = PaintCtx::new();
        b.paint(&mut ctx);
        let cmds = ctx.commands();
        assert_eq!(cmds.len(), 3);
        assert!(matches!(cmds[0], crate::paint::DrawCmd::FillRect { .. }));
        assert!(matches!(cmds[1], crate::paint::DrawCmd::StrokeRect { .. }));
        assert!(matches!(cmds[2], crate::paint::DrawCmd::Text { .. }));
    }
}
