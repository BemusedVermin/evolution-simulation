//! Read-only data bindings between widgets and their data sources.
//!
//! Per `documentation/INVARIANTS.md` §6 and `documentation/systems/23_ui_overview.md` §7,
//! the UI layer is read-only against simulation state — widgets must
//! never mutate sim data through a binding. The trait surface here
//! enforces that at the type level: [`DataBinding::read`] takes
//! `&self` and returns a value, never an `&mut` borrow into the source.
//!
//! S10.3 ships:
//!
//! * [`DataBinding`] trait with a single `read(&self) -> T` method.
//! * [`StaticBinding`] — wraps an owned `T: Clone`; useful for tests
//!   and screens that don't need a live data source yet.
//! * [`FnBinding`] — wraps a `Fn() -> T` closure; the canonical way
//!   to bind to a sim-side getter (`|| world.tick()`).
//! * [`Bound<W, B>`] — generic binding wrapper. The `Widget` impl is
//!   currently provided for `Bound<Label, B: DataBinding<String>>`,
//!   so screen code can compose `Bound::new(Label::new(...), ...)` to
//!   get a label whose text refreshes from the binding on every paint.
//!
//! Future widget types (e.g. a `Bound<Card, B>` that updates the card
//! title) get their own `Widget` impl in this module without changing
//! the trait — the wrapper is always read-only by construction.

use crate::event::{EventResult, UiEvent};
use crate::layout::LayoutConstraints;
use crate::paint::{PaintCtx, Point, Rect, Size};
use crate::widget::{LayoutCtx, Widget, WidgetId};
use crate::Label;

/// Read-only access to a value of type `T`.
///
/// Implementations must be deterministic relative to their inputs —
/// repeated `read` calls without external mutation return the same
/// value. The `&self` receiver pins the read-only contract: a binding
/// cannot mutate its source through the trait.
pub trait DataBinding<T> {
    /// Resolve the current value.
    fn read(&self) -> T;
}

/// Binding that always returns the same `Clone`d value.
///
/// Useful for unit tests and screen scaffolding before a real getter
/// is wired in. Implementations clone on every `read`; callers
/// concerned about clone cost should reach for [`FnBinding`] with a
/// borrowing closure instead.
#[derive(Clone, Debug)]
pub struct StaticBinding<T>(pub T);

impl<T: Clone> DataBinding<T> for StaticBinding<T> {
    fn read(&self) -> T {
        self.0.clone()
    }
}

/// Binding that resolves through a closure on every `read`.
///
/// The closure is `Fn() -> T`, so it must be re-entrant and side-effect
/// free as far as observable state goes. Capture sim handles by
/// shared reference — never `&mut` — to keep the read-only contract.
pub struct FnBinding<T, F: Fn() -> T> {
    f: F,
    _t: core::marker::PhantomData<T>,
}

impl<T, F: Fn() -> T> FnBinding<T, F> {
    /// Wrap a closure as a [`DataBinding`].
    pub fn new(f: F) -> Self {
        Self {
            f,
            _t: core::marker::PhantomData,
        }
    }
}

impl<T, F: Fn() -> T> DataBinding<T> for FnBinding<T, F> {
    fn read(&self) -> T {
        (self.f)()
    }
}

impl<T, F: Fn() -> T> std::fmt::Debug for FnBinding<T, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FnBinding").finish_non_exhaustive()
    }
}

/// Wrap a widget `W` together with a [`DataBinding`] `B`.
///
/// The pair is generic so the same struct can host bindings for
/// arbitrary widget types — only widgets with a `Widget for Bound<W, B>`
/// impl are usable in a [`crate::WidgetTree`]. S10.3 ships the impl
/// for `Bound<Label, B: DataBinding<String>>`; further specialisations
/// land alongside the widgets that need them.
pub struct Bound<W, B> {
    widget: W,
    binding: B,
}

impl<W, B> Bound<W, B> {
    /// Construct a new bound widget.
    pub fn new(widget: W, binding: B) -> Self {
        Self { widget, binding }
    }

    /// Borrow the wrapped widget read-only.
    pub fn widget(&self) -> &W {
        &self.widget
    }

    /// Borrow the wrapped widget mutably — for layout-time bookkeeping
    /// (e.g. setting bounds) that the wrapper cannot do without
    /// reaching into `W`.
    pub fn widget_mut(&mut self) -> &mut W {
        &mut self.widget
    }

    /// Borrow the binding read-only.
    pub fn binding(&self) -> &B {
        &self.binding
    }
}

impl<W: std::fmt::Debug, B: std::fmt::Debug> std::fmt::Debug for Bound<W, B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Bound")
            .field("widget", &self.widget)
            .field("binding", &self.binding)
            .finish()
    }
}

impl<B: DataBinding<String>> Widget for Bound<Label, B> {
    fn id(&self) -> WidgetId {
        self.widget.id()
    }

    fn bounds(&self) -> Rect {
        self.widget.bounds()
    }

    fn set_bounds(&mut self, bounds: Rect) {
        self.widget.set_bounds(bounds);
    }

    fn measure(&self, ctx: &LayoutCtx) -> Size {
        // The wrapped Label's measure is a heuristic over its cached
        // text. The binding may evaluate to a longer / shorter string
        // at paint time; keeping the measure stable means layout
        // doesn't reshuffle every frame the binding's string drifts
        // (typical for tick counters etc.). Screens that need
        // binding-aware sizing can call `binding.read()` themselves
        // and hand the result to a sized container.
        self.widget.measure(ctx)
    }

    fn layout(&mut self, ctx: &LayoutCtx, constraints: LayoutConstraints) -> Size {
        self.widget.layout(ctx, constraints)
    }

    fn paint(&self, ctx: &mut PaintCtx) {
        // Re-render with the *live* binding value rather than the
        // wrapped Label's cached text. This is the contract called out
        // in the S10.3 DoD: "Bound<Label, B> re-renders the label text
        // from the binding on each paint() call."
        let live = self.binding.read();
        let bounds = self.widget.bounds();
        ctx.text(
            Point::new(bounds.origin.x, bounds.origin.y),
            live.as_str(),
            self.widget.color(),
        );
    }

    fn handle_event(&mut self, event: &UiEvent) -> EventResult {
        // Delegate so any future mouse-aware Label logic still reaches
        // the wrapped widget. Today Label::handle_event is a constant
        // `Ignored`.
        self.widget.handle_event(event)
    }

    fn visit_pre_order<'a>(&'a self, visitor: &mut dyn FnMut(&'a dyn Widget)) {
        // Treat Bound as transparent: the visitor sees `self`, not the
        // inner Label. `kind()` reports "Label" for snapshot stability
        // so existing dump_layout snapshots don't have to be rewritten
        // when callers swap a Label for a Bound<Label, _>.
        visitor(self);
    }

    fn kind(&self) -> &'static str {
        // Pose as the wrapped Label so screen-builder snapshot tests
        // see the same layout dump regardless of whether the label is
        // statically declared or bound to a getter.
        "Label"
    }

    fn collect_focus_chain(&self, _out: &mut Vec<WidgetId>) {
        // Labels — bound or not — are never focusable.
    }

    fn find_widget_mut(&mut self, id: WidgetId) -> Option<&mut dyn Widget> {
        if self.id() == id {
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
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[test]
    fn static_binding_returns_value_on_each_read() {
        let b = StaticBinding("hello".to_string());
        assert_eq!(b.read(), "hello");
        assert_eq!(b.read(), "hello");
    }

    #[test]
    fn fn_binding_reads_through_closure() {
        // Drive the binding off a counter to verify `read()` re-runs
        // the closure rather than caching.
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = Arc::clone(&counter);
        let binding =
            FnBinding::new(move || counter_clone.fetch_add(1, Ordering::SeqCst).to_string());
        assert_eq!(binding.read(), "0");
        assert_eq!(binding.read(), "1");
        assert_eq!(binding.read(), "2");
    }

    #[test]
    fn bound_label_paints_live_binding_value() {
        let mut ids = IdAllocator::new();
        let mut label = Label::new(ids.allocate(), "stale");
        label.set_bounds(Rect::xywh(2.0, 3.0, 100.0, 16.0));
        let bound = Bound::new(label, StaticBinding("fresh".to_string()));
        let mut ctx = PaintCtx::new();
        bound.paint(&mut ctx);
        match &ctx.commands()[0] {
            DrawCmd::Text { text, pos, .. } => {
                assert_eq!(text, "fresh");
                assert_eq!(*pos, Point::new(2.0, 3.0));
            }
            other => panic!("expected Text command, got {other:?}"),
        }
    }

    #[test]
    fn bound_label_text_changes_between_paints_when_binding_changes() {
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = Arc::clone(&counter);
        let mut ids = IdAllocator::new();
        let mut label = Label::new(ids.allocate(), "");
        label.set_bounds(Rect::xywh(0.0, 0.0, 100.0, 16.0));
        let bound = Bound::new(
            label,
            FnBinding::new(move || {
                format!("tick {}", counter_clone.fetch_add(1, Ordering::SeqCst))
            }),
        );

        let mut ctx_a = PaintCtx::new();
        bound.paint(&mut ctx_a);
        let mut ctx_b = PaintCtx::new();
        bound.paint(&mut ctx_b);

        let extract = |ctx: &PaintCtx| match &ctx.commands()[0] {
            DrawCmd::Text { text, .. } => text.clone(),
            other => panic!("expected Text command, got {other:?}"),
        };
        assert_eq!(extract(&ctx_a), "tick 0");
        assert_eq!(extract(&ctx_b), "tick 1");
    }

    #[test]
    fn bound_label_is_not_focusable() {
        let mut ids = IdAllocator::new();
        let label = Label::new(ids.allocate(), "x");
        let bound = Bound::new(label, StaticBinding(String::new()));
        let mut chain = Vec::new();
        bound.collect_focus_chain(&mut chain);
        assert!(chain.is_empty());
    }

    #[test]
    fn bound_label_kind_matches_wrapped_label() {
        // Snapshot tests that walk dump_layout should see "Label" both
        // for Label and Bound<Label, _>.
        let mut ids = IdAllocator::new();
        let label = Label::new(ids.allocate(), "x");
        let bound = Bound::new(label, StaticBinding(String::new()));
        assert_eq!(bound.kind(), "Label");
    }
}
