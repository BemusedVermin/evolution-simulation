//! Placeholder for an embedded `beast-render` viewport (S10.4).
//!
//! `RenderViewport` reserves a rectangle inside a screen's widget tree
//! that the SDL renderer (S13) will paint into directly. The widget
//! itself owns no renderer state — it just records bounds and a tint
//! so layout snapshots and headless tests can assert on the geometry.
//!
//! The chosen contract:
//!
//! * `paint()` emits a single [`crate::paint::DrawCmd::FillRect`] with
//!   the viewport's tint. The eventual SDL backend (S13) will replace
//!   this with the real renderer call; until then the widget paints a
//!   visible rectangle so headless screenshots reflect that something
//!   should appear here.
//! * `handle_event()` always returns [`EventResult::Ignored`]. Mouse /
//!   keyboard interactions on the rendered view (camera pan, encounter
//!   creature selection) live in dedicated screen-side widgets layered
//!   on top — the viewport is presentation-only.
//! * The viewport is never focusable: keyboard input flows to the
//!   action bar / list widgets, not into the SDL surface.

use crate::event::{EventResult, UiEvent};
use crate::paint::{Color, PaintCtx, Rect, Size};

use super::{LayoutCtx, Widget, WidgetId};

/// Placeholder for a `beast-render` surface embedded in a screen.
#[derive(Clone, Debug)]
pub struct RenderViewport {
    id: WidgetId,
    bounds: Rect,
    tint: Color,
    preferred: Size,
}

impl RenderViewport {
    /// Construct a viewport with a default mid-grey tint and a 320×240
    /// preferred size. Callers wire it into a [`crate::Stack`] /
    /// [`crate::Card`] which assigns the real bounds during layout.
    pub fn new(id: WidgetId) -> Self {
        Self {
            id,
            bounds: Rect::ZERO,
            tint: Color::rgb(0.10, 0.12, 0.18),
            preferred: Size::new(320.0, 240.0),
        }
    }

    /// Override the placeholder fill colour. Useful for tests that
    /// want to discriminate the world-map viewport from the
    /// encounter viewport in the recorded paint commands.
    pub fn with_tint(mut self, tint: Color) -> Self {
        self.tint = tint;
        self
    }

    /// Override the preferred size returned from
    /// [`Widget::measure`]. The screen builders set this so the
    /// viewport claims most of the content area instead of the
    /// default 320×240.
    pub fn with_preferred_size(mut self, size: Size) -> Self {
        self.preferred = size;
        self
    }

    /// Tint colour the placeholder fills.
    pub fn tint(&self) -> Color {
        self.tint
    }
}

impl Widget for RenderViewport {
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
        self.preferred
    }

    fn paint(&self, ctx: &mut PaintCtx) {
        ctx.fill_rect(self.bounds, self.tint);
    }

    fn handle_event(&mut self, _event: &UiEvent) -> EventResult {
        EventResult::Ignored
    }

    fn visit_pre_order<'a>(&'a self, visitor: &mut dyn FnMut(&'a dyn Widget)) {
        visitor(self);
    }

    fn kind(&self) -> &'static str {
        "RenderViewport"
    }

    // The renderer surface is not focusable — its keyboard
    // interactions are owned by the surrounding screen widgets.
    fn collect_focus_chain(&self, _out: &mut Vec<WidgetId>) {}

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
    use crate::paint::DrawCmd;
    use crate::widget::IdAllocator;

    #[test]
    fn paint_emits_single_fill_rect_with_tint() {
        let mut ids = IdAllocator::new();
        let mut v = RenderViewport::new(ids.allocate()).with_tint(Color::rgb(0.5, 0.6, 0.7));
        v.set_bounds(Rect::xywh(10.0, 20.0, 100.0, 80.0));
        let mut ctx = PaintCtx::new();
        v.paint(&mut ctx);
        assert_eq!(ctx.commands().len(), 1);
        match &ctx.commands()[0] {
            DrawCmd::FillRect { rect, color } => {
                assert_eq!(*rect, Rect::xywh(10.0, 20.0, 100.0, 80.0));
                assert!((color.r - 0.5).abs() < 1e-6);
                assert!((color.g - 0.6).abs() < 1e-6);
                assert!((color.b - 0.7).abs() < 1e-6);
            }
            other => panic!("expected FillRect, got {other:?}"),
        }
    }

    #[test]
    fn measure_uses_preferred_size() {
        let mut ids = IdAllocator::new();
        let v = RenderViewport::new(ids.allocate()).with_preferred_size(Size::new(800.0, 600.0));
        let lc = LayoutCtx::default();
        assert_eq!(v.measure(&lc), Size::new(800.0, 600.0));
    }

    #[test]
    fn ignores_all_events_and_is_not_focusable() {
        let mut ids = IdAllocator::new();
        let mut v = RenderViewport::new(ids.allocate());
        v.set_bounds(Rect::xywh(0.0, 0.0, 100.0, 100.0));
        assert_eq!(
            v.handle_event(&UiEvent::MouseMove { x: 50.0, y: 50.0 }),
            EventResult::Ignored,
        );
        let mut chain = Vec::new();
        v.collect_focus_chain(&mut chain);
        assert!(chain.is_empty());
    }
}
