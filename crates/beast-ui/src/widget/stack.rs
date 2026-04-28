//! Linear container that lays children out along one axis.
//!
//! `Stack` is the workhorse container for action bars, status panels, and
//! anywhere a screen wants a sequence of widgets with a consistent gap.
//! It is intentionally minimal — no flex factors, no stretching — so the
//! layout pass stays a single recursive walk with no fixed-point
//! iterations. Future extensions (proportional `flex` weights, baseline
//! alignment) will land alongside the screens in S10.4 if they prove
//! necessary.

use crate::event::{EventResult, UiEvent};
use crate::layout::{cross_axis_offset, Align, Axis, LayoutConstraints};
use crate::paint::{PaintCtx, Point, Rect, Size};

use super::{LayoutCtx, Widget, WidgetId};

/// Linear container along one axis.
pub struct Stack {
    id: WidgetId,
    bounds: Rect,
    direction: Axis,
    gap: f32,
    align: Align,
    children: Vec<Box<dyn Widget>>,
    last_cursor: Option<Point>,
}

impl Stack {
    /// Construct an empty stack with the given main-axis direction. Gap
    /// defaults to `0.0` and alignment to [`Align::Start`].
    pub fn new(id: WidgetId, direction: Axis) -> Self {
        Self {
            id,
            bounds: Rect::ZERO,
            direction,
            gap: 0.0,
            align: Align::Start,
            children: Vec::new(),
            last_cursor: None,
        }
    }

    /// Override the gap (in pixels) inserted between adjacent children.
    pub fn with_gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Override the cross-axis alignment.
    pub fn with_align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    /// Append a child widget. Children are laid out in declaration order.
    pub fn push_child(&mut self, child: Box<dyn Widget>) {
        self.children.push(child);
    }

    /// Number of children currently held.
    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    /// Read-only access to children — used by tests + future debug
    /// inspectors.
    pub fn children(&self) -> &[Box<dyn Widget>] {
        &self.children
    }

    /// Main-axis direction.
    pub fn direction(&self) -> Axis {
        self.direction
    }

    /// Gap between adjacent children, in pixels.
    pub fn gap(&self) -> f32 {
        self.gap
    }

    /// Cross-axis alignment.
    pub fn align(&self) -> Align {
        self.align
    }
}

impl std::fmt::Debug for Stack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Stack")
            .field("id", &self.id)
            .field("bounds", &self.bounds)
            .field("direction", &self.direction)
            .field("gap", &self.gap)
            .field("align", &self.align)
            .field("children", &self.children.len())
            .finish()
    }
}

impl Widget for Stack {
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
        // Sum of children measure + cumulative gap on the main axis,
        // max child extent on the cross axis. Used by parent containers
        // that want to size the stack tightly without running a full
        // layout pass.
        let n = self.children.len();
        if n == 0 {
            return Size::ZERO;
        }
        let mut main_total: f32 = 0.0;
        let mut cross_max: f32 = 0.0;
        for child in &self.children {
            let s = child.measure(ctx);
            match self.direction {
                Axis::Horizontal => {
                    main_total += s.width;
                    cross_max = cross_max.max(s.height);
                }
                Axis::Vertical => {
                    main_total += s.height;
                    cross_max = cross_max.max(s.width);
                }
            }
        }
        main_total += self.gap * (n as f32 - 1.0);
        match self.direction {
            Axis::Horizontal => Size::new(main_total, cross_max),
            Axis::Vertical => Size::new(cross_max, main_total),
        }
    }

    fn layout(&mut self, ctx: &LayoutCtx, constraints: LayoutConstraints) -> Size {
        // The contract is "parent sets bounds before calling layout".
        // Containers therefore use a two-phase walk:
        //   1. Ask each child for its preferred size via `measure` so
        //      we can compute our own total without polluting child
        //      bounds prematurely.
        //   2. Set each child's bounds against our (correct) origin and
        //      recurse via `layout` so grandchildren are positioned
        //      against the now-final origin — not the stale (0, 0)
        //      bounds the child carried in.
        let n = self.children.len();
        if n == 0 {
            return constraints.constrain(Size::ZERO);
        }
        let child_sizes: Vec<Size> = self.children.iter().map(|c| c.measure(ctx)).collect();

        let total_gap = self.gap * (n as f32 - 1.0).max(0.0);
        let (main_total, cross_max) = match self.direction {
            Axis::Horizontal => (
                child_sizes.iter().map(|s| s.width).sum::<f32>() + total_gap,
                child_sizes.iter().map(|s| s.height).fold(0.0_f32, f32::max),
            ),
            Axis::Vertical => (
                child_sizes.iter().map(|s| s.height).sum::<f32>() + total_gap,
                child_sizes.iter().map(|s| s.width).fold(0.0_f32, f32::max),
            ),
        };
        let intrinsic = match self.direction {
            Axis::Horizontal => Size::new(main_total, cross_max),
            Axis::Vertical => Size::new(cross_max, main_total),
        };
        let final_size = constraints.constrain(intrinsic);

        let mut cursor: f32 = 0.0;
        for (i, child) in self.children.iter_mut().enumerate() {
            let s = child_sizes[i];
            let (offset_x, offset_y) = match self.direction {
                Axis::Horizontal => (
                    cursor,
                    cross_axis_offset(self.align, final_size.height, s.height),
                ),
                Axis::Vertical => (
                    cross_axis_offset(self.align, final_size.width, s.width),
                    cursor,
                ),
            };
            let origin = Point::new(
                self.bounds.origin.x + offset_x,
                self.bounds.origin.y + offset_y,
            );
            child.set_bounds(Rect::new(origin, s));
            // Recurse so any grandchildren see the correct origin.
            let _ = child.layout(ctx, LayoutConstraints::tight(s));

            cursor += match self.direction {
                Axis::Horizontal => s.width,
                Axis::Vertical => s.height,
            };
            if i + 1 < n {
                cursor += self.gap;
            }
        }

        final_size
    }

    fn paint(&self, ctx: &mut PaintCtx) {
        for child in &self.children {
            child.paint(ctx);
        }
    }

    fn handle_event(&mut self, event: &UiEvent) -> EventResult {
        if let UiEvent::MouseMove { x, y } = event {
            self.last_cursor = Some(Point::new(*x, *y));
        }
        // Reverse-iterate so the last-declared (top-most-painted) child
        // gets first crack at the event. Mirrors `Card`'s contract so
        // `Stack`-of-`Stack` composition behaves consistently.
        for child in self.children.iter_mut().rev() {
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
        "Stack"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget::{Button, IdAllocator};

    fn ctx() -> LayoutCtx {
        LayoutCtx::default()
    }

    fn fixed_button(ids: &mut IdAllocator, label: &str, w: f32, h: f32) -> Box<dyn Widget> {
        // Force a known measure independent of the heuristic 8px/glyph
        // formula by pre-setting bounds to (w, h) and overriding measure
        // via the existing Button — Button::measure already returns
        // glyph * 8 + 16 for width and 32 for height, so we instead
        // build a label whose glyph count cleanly hits the target.
        let mut b = Button::new(ids.allocate(), label);
        b.set_bounds(Rect::new(Point::new(0.0, 0.0), Size::new(w, h)));
        // Note: real measure used during layout still comes from Button;
        // tests below assert positions, not sizes.
        Box::new(b)
    }

    #[test]
    fn horizontal_stack_lays_four_buttons_with_gap() {
        let mut ids = IdAllocator::new();
        let mut s = Stack::new(ids.allocate(), Axis::Horizontal).with_gap(8.0);
        s.set_bounds(Rect::xywh(10.0, 20.0, 1000.0, 100.0));
        for label in ["a", "bb", "ccc", "dddd"] {
            s.push_child(fixed_button(&mut ids, label, 0.0, 0.0));
        }

        let _ = s.layout(&ctx(), LayoutConstraints::loose(Size::new(1000.0, 100.0)));

        // Each Button measures `chars * 8 + 16` wide and `32` tall.
        let widths = [
            1.0 * 8.0 + 16.0,
            2.0 * 8.0 + 16.0,
            3.0 * 8.0 + 16.0,
            4.0 * 8.0 + 16.0,
        ];
        let mut expected_x = 10.0_f32;
        for (i, child) in s.children().iter().enumerate() {
            let r = child.bounds();
            assert!(
                (r.origin.x - expected_x).abs() < 1e-3,
                "child {i} x: got {} expected {}",
                r.origin.x,
                expected_x
            );
            assert!((r.origin.y - 20.0).abs() < 1e-3, "child {i} y");
            assert!((r.size.width - widths[i]).abs() < 1e-3, "child {i} width");
            expected_x += widths[i] + 8.0;
        }
    }

    #[test]
    fn vertical_stack_lays_four_buttons_with_gap() {
        let mut ids = IdAllocator::new();
        let mut s = Stack::new(ids.allocate(), Axis::Vertical).with_gap(4.0);
        s.set_bounds(Rect::xywh(0.0, 0.0, 200.0, 1000.0));
        for label in ["a", "b", "c", "d"] {
            s.push_child(fixed_button(&mut ids, label, 0.0, 0.0));
        }
        let _ = s.layout(&ctx(), LayoutConstraints::loose(Size::new(200.0, 1000.0)));

        let mut expected_y = 0.0_f32;
        for (i, child) in s.children().iter().enumerate() {
            let r = child.bounds();
            assert!(
                (r.origin.y - expected_y).abs() < 1e-3,
                "child {i} y: got {} expected {}",
                r.origin.y,
                expected_y
            );
            assert!((r.origin.x - 0.0).abs() < 1e-3, "child {i} x = 0 (Start)");
            // Buttons are 32px tall.
            expected_y += 32.0 + 4.0;
        }
    }

    #[test]
    fn align_center_offsets_cross_axis() {
        let mut ids = IdAllocator::new();
        let mut s = Stack::new(ids.allocate(), Axis::Horizontal).with_align(Align::Center);
        s.set_bounds(Rect::xywh(0.0, 0.0, 400.0, 100.0));
        s.push_child(fixed_button(&mut ids, "ok", 0.0, 0.0));
        let _ = s.layout(&ctx(), LayoutConstraints::tight(Size::new(400.0, 100.0)));
        // Button is 32 tall. (100 - 32) / 2 = 34.
        assert!((s.children()[0].bounds().origin.y - 34.0).abs() < 1e-3);
    }

    #[test]
    fn align_end_anchors_cross_axis() {
        let mut ids = IdAllocator::new();
        let mut s = Stack::new(ids.allocate(), Axis::Horizontal).with_align(Align::End);
        s.set_bounds(Rect::xywh(0.0, 0.0, 400.0, 100.0));
        s.push_child(fixed_button(&mut ids, "ok", 0.0, 0.0));
        let _ = s.layout(&ctx(), LayoutConstraints::tight(Size::new(400.0, 100.0)));
        assert!((s.children()[0].bounds().origin.y - 68.0).abs() < 1e-3);
    }

    #[test]
    fn empty_stack_returns_zero_size() {
        let mut ids = IdAllocator::new();
        let mut s = Stack::new(ids.allocate(), Axis::Horizontal);
        s.set_bounds(Rect::xywh(0.0, 0.0, 100.0, 100.0));
        let size = s.layout(&ctx(), LayoutConstraints::loose(Size::new(100.0, 100.0)));
        assert_eq!(size, Size::ZERO);
    }
}
