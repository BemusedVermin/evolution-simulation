//! Modal / non-modal dialog overlay.
//!
//! When `modal == true`, the dialog consumes any input event whose target
//! falls *outside* its own bounds — this is what stops widgets behind a
//! modal dialog from receiving clicks. Non-modal dialogs let everything
//! bubble.

use crate::event::{EventResult, UiEvent};
use crate::paint::{Color, PaintCtx, Point, Rect, Size};

use super::{Card, Widget, WidgetId};

/// Dialog overlay wrapping a [`Card`].
pub struct Dialog {
    inner: Card,
    modal: bool,
    last_cursor: Option<Point>,
}

impl Dialog {
    /// Construct a dialog with the given title and modality.
    pub fn new(id: WidgetId, title: impl Into<String>, modal: bool) -> Self {
        Self {
            inner: Card::new(id, title),
            modal,
            last_cursor: None,
        }
    }

    /// Whether the dialog is modal (eats outside events).
    pub fn is_modal(&self) -> bool {
        self.modal
    }

    /// Append a child widget — delegates to the wrapped [`Card`].
    pub fn push_child(&mut self, child: Box<dyn Widget>) {
        self.inner.push_child(child);
    }
}

impl std::fmt::Debug for Dialog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Dialog")
            .field("inner", &self.inner)
            .field("modal", &self.modal)
            .finish()
    }
}

impl Widget for Dialog {
    fn id(&self) -> WidgetId {
        self.inner.id()
    }

    fn bounds(&self) -> Rect {
        self.inner.bounds()
    }

    fn set_bounds(&mut self, bounds: Rect) {
        self.inner.set_bounds(bounds);
    }

    fn measure(&self) -> Size {
        self.inner.measure()
    }

    fn paint(&self, ctx: &mut PaintCtx) {
        if self.modal {
            // Dim the rest of the screen behind the dialog. The screen
            // bounds aren't known here yet (S10.2 will add a layout-time
            // viewport hand-off), so for now we paint the scrim using
            // the dialog's own bounds expanded by a scrim rect at
            // (-100k, -100k, 200k, 200k). It's overkill in pixel terms
            // but keeps the recorded command set deterministic. A proper
            // viewport-relative scrim lands with the WidgetTree.
            ctx.fill_rect(
                Rect::xywh(-100_000.0, -100_000.0, 200_000.0, 200_000.0),
                Color::rgba(0.0, 0.0, 0.0, 0.4),
            );
        }
        self.inner.paint(ctx);
    }

    fn handle_event(&mut self, event: &UiEvent) -> EventResult {
        if let UiEvent::MouseMove { x, y } = event {
            self.last_cursor = Some(Point::new(*x, *y));
        }
        let inner_result = self.inner.handle_event(event);
        if inner_result == EventResult::Consumed {
            return EventResult::Consumed;
        }
        // If we're modal and the event targets outside the dialog, eat it
        // anyway so back-of-stack widgets never see it. MouseMove still
        // bubbles because it never carries a target point check here —
        // tests assert on the click-through-modal contract.
        if self.modal {
            match event {
                UiEvent::MouseDown { .. } | UiEvent::MouseUp { .. } => {
                    let cursor_outside = self
                        .last_cursor
                        .map(|c| !self.bounds().contains(c))
                        .unwrap_or(false);
                    if cursor_outside {
                        return EventResult::Consumed;
                    }
                }
                UiEvent::KeyDown(_) | UiEvent::KeyUp(_) | UiEvent::TextInput(_) => {
                    return EventResult::Consumed;
                }
                _ => {}
            }
        }
        EventResult::Ignored
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::MouseButton;
    use crate::widget::{Button, IdAllocator};

    fn fixture(modal: bool) -> Dialog {
        let mut ids = IdAllocator::new();
        let mut dlg = Dialog::new(ids.allocate(), "Confirm", modal);
        dlg.set_bounds(Rect::xywh(40.0, 40.0, 100.0, 100.0));
        let mut ok = Button::new(ids.allocate(), "ok");
        ok.set_bounds(Rect::xywh(50.0, 100.0, 40.0, 20.0));
        dlg.push_child(Box::new(ok));
        dlg
    }

    #[test]
    fn modal_dialog_eats_outside_clicks() {
        let mut dlg = fixture(true);
        dlg.handle_event(&UiEvent::MouseMove { x: 10.0, y: 10.0 });
        let r = dlg.handle_event(&UiEvent::MouseDown {
            button: MouseButton::Primary,
        });
        assert_eq!(r, EventResult::Consumed);
    }

    #[test]
    fn modal_dialog_lets_inside_clicks_through_to_children() {
        let mut dlg = fixture(true);
        // Cursor on the OK button inside the dialog.
        dlg.handle_event(&UiEvent::MouseMove { x: 60.0, y: 110.0 });
        let r = dlg.handle_event(&UiEvent::MouseDown {
            button: MouseButton::Primary,
        });
        assert_eq!(r, EventResult::Consumed);
    }

    #[test]
    fn nonmodal_dialog_lets_outside_clicks_bubble() {
        let mut dlg = fixture(false);
        dlg.handle_event(&UiEvent::MouseMove { x: 10.0, y: 10.0 });
        let r = dlg.handle_event(&UiEvent::MouseDown {
            button: MouseButton::Primary,
        });
        assert_eq!(r, EventResult::Ignored);
    }
}
