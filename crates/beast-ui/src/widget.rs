//! Widget trait + primitive widgets (Button, Label, List, Card, Dialog).
//!
//! See `documentation/systems/23_ui_overview.md` §3.2 for the design spec.
//! This module is split across submodules per primitive; the trait and
//! shared id machinery live here.

use crate::event::{EventResult, UiEvent};
use crate::paint::{PaintCtx, Rect, Size};

mod button;
mod card;
mod dialog;
mod label;
mod list;

pub use button::Button;
pub use card::Card;
pub use dialog::Dialog;
pub use label::Label;
pub use list::{List, ListItem};

/// Stable, globally unique identifier for a widget.
///
/// Newtype around `u32` to keep widget ids out of lane traffic with other
/// integer ids (entities, sprites, primitives). Allocate via [`IdAllocator`]
/// — never construct a [`WidgetId`] from a hand-picked literal in non-test
/// code, since collisions silently break event routing.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
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
    /// S10.1 returns a single [`Size`]; S10.2 will extend this to handle
    /// flex-style min / max constraints.
    fn measure(&self) -> Size;

    /// Render the widget by pushing draw commands into `ctx`.
    fn paint(&self, ctx: &mut PaintCtx);

    /// Process an input event. Returns whether the widget consumed the
    /// event ([`EventResult::Consumed`]) or wants it to bubble
    /// ([`EventResult::Ignored`]).
    fn handle_event(&mut self, event: &UiEvent) -> EventResult;
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
