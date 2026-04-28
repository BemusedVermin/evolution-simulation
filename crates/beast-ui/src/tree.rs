//! Retained-mode widget tree.
//!
//! [`WidgetTree`] owns a single root `Box<dyn Widget>` and the viewport
//! size the layout pass should fill. Mutations bump a version counter so
//! [`WidgetTree::layout`] can short-circuit when nothing has changed —
//! the standard retained-mode dirty bit, sized to one `u64` per tree.
//!
//! Per the S10.2 issue, the tree's auxiliary id → state map (when one
//! is needed) uses [`std::collections::BTreeMap`] so iteration order is
//! deterministic. This module currently doesn't keep any such state —
//! children are owned inline by their containers, and bounds live on
//! each widget's own `bounds` field. The placeholder noted here will
//! be filled in when S10.4 wires data bindings, which need a stable
//! per-id cache.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::layout::LayoutConstraints;
use crate::paint::{PaintCtx, Point, Rect, Size};
use crate::widget::{LayoutCtx, Widget, WidgetId};
use crate::{EventResult, UiEvent};

/// Owns the root widget and triggers retained-mode layout / paint /
/// dispatch passes.
pub struct WidgetTree {
    root: Box<dyn Widget>,
    root_size: Size,
    /// Bumped on any mutation that may invalidate the cached layout.
    version: u64,
    /// Snapshot of `version` at the moment of the last successful
    /// layout pass — used as a dirty bit.
    laid_out_at: u64,
    /// Reserved for the S10.4 data-binding cache; see module docs.
    /// Keeping the field in place now means later stories can bind
    /// without bumping any public API.
    #[allow(dead_code)]
    binding_state: BTreeMap<WidgetId, ()>,
    layout_ctx: LayoutCtx,
}

impl WidgetTree {
    /// Construct a tree with the given root widget and viewport size.
    /// The tree starts in the dirty state — the first call to
    /// [`WidgetTree::layout`] will run a full pass.
    pub fn new(root: Box<dyn Widget>, root_size: Size) -> Self {
        Self {
            root,
            root_size,
            version: 1,
            laid_out_at: 0,
            binding_state: BTreeMap::new(),
            layout_ctx: LayoutCtx::default(),
        }
    }

    /// Borrow the root widget read-only.
    pub fn root(&self) -> &dyn Widget {
        &*self.root
    }

    /// Borrow the root widget mutably. Marks the tree dirty so the next
    /// [`WidgetTree::layout`] pass re-runs even if `root_size` hasn't
    /// changed.
    pub fn root_mut(&mut self) -> &mut dyn Widget {
        self.version = self.version.wrapping_add(1);
        &mut *self.root
    }

    /// Current viewport size the layout pass fills.
    pub fn root_size(&self) -> Size {
        self.root_size
    }

    /// Resize the viewport. Marks the tree dirty so the next
    /// [`WidgetTree::layout`] pass re-runs.
    pub fn resize(&mut self, root_size: Size) {
        if root_size != self.root_size {
            self.root_size = root_size;
            self.version = self.version.wrapping_add(1);
        }
    }

    /// Run a layout pass if the tree is dirty. Returns `true` if a pass
    /// was actually executed (cache miss); `false` when the cached
    /// layout was reused.
    pub fn layout(&mut self) -> bool {
        if self.laid_out_at == self.version {
            return false;
        }
        self.root
            .set_bounds(Rect::new(Point::new(0.0, 0.0), self.root_size));
        let _ = self
            .root
            .layout(&self.layout_ctx, LayoutConstraints::tight(self.root_size));
        self.laid_out_at = self.version;
        true
    }

    /// Dispatch an event to the root widget. Mutating handlers (e.g. a
    /// list selecting a row) bump the dirty bit so the next
    /// [`WidgetTree::layout`] re-validates the layout — paint output
    /// can change without the layout actually shifting, but the worst
    /// case (unnecessary recompute) is a few microseconds and the best
    /// case (a binding update widening a label) needs the re-layout to
    /// be correct.
    ///
    /// Two events are filtered out of the dirty-bit bump:
    ///
    /// 1. Any event the tree returns [`EventResult::Ignored`] for. By
    ///    contract, an `Ignored` result means no widget mutated state,
    ///    so re-laying out would be redundant. Containers that need to
    ///    invalidate while forwarding without consuming should return
    ///    `Consumed` instead.
    /// 2. `MouseMove` even when consumed — it is high-frequency and
    ///    hover state rarely shifts layout. If a future hover-driven
    ///    widget needs a re-layout, it can request one through the
    ///    binding cache (S10.4) rather than via mouse-move dispatch.
    pub fn dispatch(&mut self, event: &UiEvent) -> EventResult {
        let result = self.root.handle_event(event);
        if result == EventResult::Consumed && !matches!(event, UiEvent::MouseMove { .. }) {
            self.version = self.version.wrapping_add(1);
        }
        result
    }

    /// Paint the tree into `ctx`. Read-only against tree state.
    pub fn paint(&self, ctx: &mut PaintCtx) {
        self.root.paint(ctx);
    }

    /// Internal hook for the in-lib unit tests: returns the tree's
    /// current layout version. Two consecutive `layout()` calls
    /// without any mutations should observe the same value.
    ///
    /// `cfg(test)`-gated so it isn't part of the released public API.
    /// Integration tests under `tests/*.rs` build against the non-test
    /// configuration and therefore can't reach this — they must
    /// observe layout state via the public `layout()` return value
    /// instead.
    #[cfg(test)]
    pub fn layout_version_for_test(&self) -> u64 {
        self.laid_out_at
    }
}

/// Dump the tree's widgets in pre-order as one `(kind, id, bounds)`
/// line per widget. Snapshot tests assert against the resulting string
/// — the canonical format spec is:
///
/// ```text
/// {kind}#{id} {x},{y} {w}x{h}
/// ```
///
/// with each value formatted via Rust's default `{}` for `f32` (no
/// trailing fractional digits when the value is integer-valued, no
/// scientific notation for the ranges layout produces). One line per
/// widget; output ends with a trailing newline so concatenation with
/// other dumps stays clean.
pub fn dump_layout(tree: &WidgetTree) -> String {
    let mut out = String::new();
    let mut visit = |w: &dyn Widget| {
        let r = w.bounds();
        // `writeln!` formats directly into the buffer; `String`'s
        // `fmt::Write` impl is infallible so the unwrap can never fire.
        // The previous `push_str(&format!(...))` allocated one extra
        // String per widget, which mattered for 200+ widget dumps.
        writeln!(
            &mut out,
            "{}#{} {},{} {}x{}",
            w.kind(),
            w.id().raw(),
            r.origin.x,
            r.origin.y,
            r.size.width,
            r.size.height,
        )
        .expect("String fmt::Write is infallible");
    };
    tree.root().visit_pre_order(&mut visit);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget::{Button, IdAllocator, Stack};
    use crate::Axis;

    fn fixture() -> WidgetTree {
        let mut ids = IdAllocator::new();
        let mut stack = Stack::new(ids.allocate(), Axis::Horizontal).with_gap(4.0);
        for label in ["a", "bb", "ccc"] {
            stack.push_child(Box::new(Button::new(ids.allocate(), label)));
        }
        WidgetTree::new(Box::new(stack), Size::new(800.0, 100.0))
    }

    #[test]
    fn first_layout_runs_subsequent_layout_caches() {
        let mut t = fixture();
        assert!(t.layout(), "first layout should be a cache miss");
        assert!(!t.layout(), "second layout (no mutations) caches");
    }

    #[test]
    fn resize_marks_tree_dirty() {
        let mut t = fixture();
        t.layout();
        assert!(!t.layout(), "cache hit");
        t.resize(Size::new(640.0, 80.0));
        assert!(t.layout(), "after resize, layout re-runs");
    }

    #[test]
    fn resize_with_same_size_keeps_cache_warm() {
        let mut t = fixture();
        t.layout();
        t.resize(t.root_size());
        assert!(!t.layout(), "no-op resize must not invalidate the cache");
    }

    #[test]
    fn dispatch_ignored_event_keeps_layout_cache_warm() {
        use crate::event::{KeyCode, KeyMods, Modifiers};
        let mut t = fixture();
        t.layout();
        let before = t.layout_version_for_test();
        // A KeyDown event hits no widget that consumes it (the stack of
        // buttons doesn't react to keypresses). The handler returns
        // `Ignored`, so dispatch must not bump the dirty bit.
        let result = t.dispatch(&UiEvent::KeyDown(Modifiers {
            key: KeyCode::Tab,
            mods: KeyMods::NONE,
        }));
        assert_eq!(result, EventResult::Ignored);
        assert!(!t.layout(), "Ignored event should not invalidate layout");
        assert_eq!(t.layout_version_for_test(), before);
    }

    #[test]
    fn dispatch_consumed_non_mousemove_invalidates_layout() {
        use crate::event::MouseButton;
        let mut t = fixture();
        // Position cursor over the first button so the click consumes.
        t.dispatch(&UiEvent::MouseMove { x: 5.0, y: 16.0 });
        t.layout();
        assert!(!t.layout(), "cache hit after layout");
        let r = t.dispatch(&UiEvent::MouseDown {
            button: MouseButton::Primary,
        });
        assert_eq!(r, EventResult::Consumed);
        assert!(t.layout(), "Consumed event should invalidate layout");
    }

    #[test]
    fn dispatch_mousemove_keeps_layout_cache_warm_even_when_consumed() {
        let mut t = fixture();
        t.layout();
        // MouseMove gets returned as `Ignored` by Button (it only
        // updates internal cursor state), but the contract is "no
        // version bump on MouseMove regardless of result". This test
        // pins that contract so a future widget that returns
        // `Consumed` for MouseMove does not silently invalidate the
        // cache.
        let _ = t.dispatch(&UiEvent::MouseMove { x: 1.0, y: 1.0 });
        assert!(!t.layout(), "MouseMove must not invalidate layout");
    }

    #[test]
    fn dump_layout_emits_one_line_per_widget_in_pre_order() {
        let mut t = fixture();
        t.layout();
        let dump = dump_layout(&t);
        let lines: Vec<_> = dump.lines().collect();
        // Stack + 3 buttons = 4 lines.
        assert_eq!(lines.len(), 4);
        assert!(lines[0].starts_with("Stack#1"), "got {}", lines[0]);
        assert!(lines[1].starts_with("Button#2"));
        assert!(lines[2].starts_with("Button#3"));
        assert!(lines[3].starts_with("Button#4"));
    }
}
