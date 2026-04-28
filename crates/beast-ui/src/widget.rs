//! Widget trait + primitive widgets (Button, Label, List, Card, Dialog).
//!
//! See `documentation/systems/23_ui_overview.md` §3.2 for the design spec.
//! This module is split across submodules per primitive; the trait and
//! shared id machinery live here.

use crate::event::{EventResult, UiEvent};
use crate::layout::LayoutConstraints;
use crate::paint::{PaintCtx, Rect, Size};

mod button;
mod card;
mod dialog;
mod grid;
mod label;
mod list;
mod stack;

pub use button::Button;
pub use card::Card;
pub use dialog::Dialog;
pub use grid::Grid;
pub use label::Label;
pub use list::{List, ListItem};
pub use stack::Stack;

/// Stable, globally unique identifier for a widget.
///
/// Newtype around `u32` to keep widget ids out of lane traffic with other
/// integer ids (entities, sprites, primitives). Allocate via [`IdAllocator`]
/// — never construct a [`WidgetId`] from a hand-picked literal in non-test
/// code, since collisions silently break event routing.
///
/// Carries serde derives so future widget-tree save/restore work doesn't
/// hit a breaking change adding them.
#[derive(
    Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct WidgetId(u32);

impl WidgetId {
    /// Construct a [`WidgetId`] from a raw `u32`. Tests only — production
    /// code allocates ids through [`IdAllocator::next`].
    #[doc(hidden)]
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    /// Raw underlying integer.
    pub const fn raw(self) -> u32 {
        self.0
    }
}

/// Deterministic monotonic id allocator.
///
/// Allocates ids in ascending order starting from `1` (id `0` is reserved
/// for "no id" in tree code). 4 billion widgets per session is well past
/// any realistic usage; if the counter does wrap, we panic on the next
/// allocation rather than silently colliding.
///
/// `Default` is provided as an alias for [`IdAllocator::new`] so callers
/// can use either; both start the counter at `1`.
#[derive(Clone, Debug)]
pub struct IdAllocator {
    next: u32,
}

impl IdAllocator {
    /// Construct an allocator at id `1`.
    pub fn new() -> Self {
        Self { next: 1 }
    }

    /// Reserve and return the next [`WidgetId`].
    pub fn allocate(&mut self) -> WidgetId {
        let id = self.next;
        self.next = self
            .next
            .checked_add(1)
            .expect("WidgetId allocator overflowed u32::MAX widget ids");
        WidgetId(id)
    }
}

impl Default for IdAllocator {
    fn default() -> Self {
        Self::new()
    }
}

/// Layout-time context passed to [`Widget::measure`].
///
/// S10.1 ships a stub — the real `LayoutCtx` (font metrics, viewport
/// scale, available DPI) lands with the layout engine in S10.2. Threading
/// the parameter through the trait now means S10.2 can extend it without
/// a breaking trait change.
#[derive(Copy, Clone, Debug, Default)]
#[non_exhaustive]
pub struct LayoutCtx {}

/// Retained-mode widget contract.
///
/// Widgets are pure data + pure dispatch. The trait is object-safe so a
/// future `WidgetTree` (S10.2) can store children behind `Box<dyn Widget>`
/// without monomorphisation.
///
/// # Determinism
///
/// `paint` and `handle_event` must be deterministic functions of widget
/// state + input. Widget primitives in this crate keep no hidden state
/// (e.g. internal counters, timers) — anything time-varying lives in the
/// caller's data binding.
pub trait Widget {
    /// Stable id assigned to this widget.
    fn id(&self) -> WidgetId;

    /// The widget's screen-space bounds. Set by the layout pass; the
    /// default getter returns the cached bounds and the setter updates
    /// them.
    fn bounds(&self) -> Rect;

    /// Update the widget's screen-space bounds. Called by the layout pass
    /// (S10.2); paint pulls these back out via `bounds()`.
    fn set_bounds(&mut self, bounds: Rect);

    /// Intrinsic minimum + preferred size used by the layout pass.
    ///
    /// S10.1 ignores `ctx` and returns a heuristic [`Size`]; S10.2 will
    /// fill in [`LayoutCtx`] with font metrics + viewport scale and
    /// extend the return shape with flex-style min / max constraints.
    fn measure(&self, ctx: &LayoutCtx) -> Size;

    /// Render the widget by pushing draw commands into `ctx`.
    fn paint(&self, ctx: &mut PaintCtx);

    /// Process an input event. Returns whether the widget consumed the
    /// event ([`EventResult::Consumed`]) or wants it to bubble
    /// ([`EventResult::Ignored`]).
    fn handle_event(&mut self, event: &UiEvent) -> EventResult;

    /// Lay this widget out within the given constraints, recursively
    /// laying out children if any. Returns the final [`Size`] the widget
    /// occupies — the parent uses it to assign the widget's
    /// [`Rect`](crate::paint::Rect).
    ///
    /// The default implementation is the leaf-widget contract: ignore
    /// child structure, take the widget's own [`measure`] result, and
    /// clamp it into the constraint envelope. Container widgets
    /// (`Stack`, `Grid`) override this to walk and position their
    /// children.
    ///
    /// [`measure`]: Widget::measure
    fn layout(&mut self, ctx: &LayoutCtx, constraints: LayoutConstraints) -> Size {
        constraints.constrain(self.measure(ctx))
    }

    /// Pre-order traversal hook. Container widgets visit `self` then
    /// recurse into children; leaf widgets just visit `self`. Used by
    /// [`crate::dump_layout`] and any other tree-walking helper that
    /// wants a deterministic, read-only view over the widget hierarchy.
    ///
    /// This method has no default body because the obvious one
    /// (`visitor(self)`) requires `Self: Sized` to coerce to a trait
    /// object — a constraint that would remove the method from the
    /// `dyn Widget` vtable and break tree traversal. Each `impl Widget`
    /// supplies its own one-line body instead.
    fn visit_pre_order<'a>(&'a self, visitor: &mut dyn FnMut(&'a dyn Widget));

    /// Stable, lowercase-free identifier for the widget's concrete type
    /// (e.g. `"Button"`, `"Stack"`). Used by [`crate::dump_layout`] to
    /// produce stable snapshot strings; tests assert on these names so
    /// they need to outlive any internal renames.
    fn kind(&self) -> &'static str;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_allocator_is_monotonic_and_starts_at_one() {
        let mut a = IdAllocator::new();
        assert_eq!(a.allocate().raw(), 1);
        assert_eq!(a.allocate().raw(), 2);
        assert_eq!(a.allocate().raw(), 3);
    }

    #[test]
    fn widget_ids_are_orderable() {
        let mut a = IdAllocator::new();
        let first = a.allocate();
        let second = a.allocate();
        assert!(first < second);
    }

    #[test]
    fn id_allocator_default_matches_new() {
        let from_new = IdAllocator::new();
        let from_default = IdAllocator::default();
        // `next` field is private; both should behave identically when we
        // pull the first id.
        let mut a = from_new;
        let mut b = from_default;
        assert_eq!(a.allocate(), b.allocate());
    }
}
