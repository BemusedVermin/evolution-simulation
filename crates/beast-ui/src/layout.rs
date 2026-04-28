//! Layout primitives shared by container widgets.
//!
//! S10.2 introduces a tree-driven layout pass. Containers (`Stack`, `Grid`)
//! receive [`LayoutConstraints`] from their parent, recursively lay out
//! children, and return their own size — the standard "constraints in,
//! size out" contract from Flutter / Druid. The parent uses the returned
//! size to assign each child's [`Rect`](crate::paint::Rect).
//!
//! The original S10.2 issue sketched a separate `Layout: Widget` supertrait
//! with `layout(&mut self, c, children: &mut [LayoutChild]) -> Size`. That
//! shape forces `Box<dyn Widget>` storage to dynamic-cast to `Layout` for
//! containers (or carry two trait objects), and a `&mut self + &mut [LayoutChild]`
//! pair conflicts when the children live inside `self.children`. Folding
//! `layout()` into [`Widget`](crate::widget::Widget) as a default method
//! keeps the storage uniform: leaf widgets get the default measure-based
//! impl, containers override.

use crate::paint::Size;

/// Min / max [`Size`] envelope a parent passes to a child during layout.
///
/// Semantics match Flutter: the child must return a [`Size`] inside this
/// envelope. The parent then positions the child at a [`Rect`](crate::paint::Rect)
/// of exactly that size.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct LayoutConstraints {
    /// Smallest size the child is permitted to return.
    pub min: Size,
    /// Largest size the child is permitted to return.
    pub max: Size,
}

impl LayoutConstraints {
    /// Constraints that pin the child to exactly `size`.
    pub const fn tight(size: Size) -> Self {
        Self {
            min: size,
            max: size,
        }
    }

    /// Constraints that allow any size up to `max`.
    pub const fn loose(max: Size) -> Self {
        Self {
            min: Size::ZERO,
            max,
        }
    }

    /// Clamp `size` into the `[min, max]` envelope on each axis.
    pub fn constrain(self, size: Size) -> Size {
        Size::new(
            clamp(size.width, self.min.width, self.max.width),
            clamp(size.height, self.min.height, self.max.height),
        )
    }
}

/// Direction along which a [`Stack`](crate::widget::Stack) lays out its
/// children.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Axis {
    /// Children flow left-to-right; main axis = X, cross axis = Y.
    Horizontal,
    /// Children flow top-to-bottom; main axis = Y, cross axis = X.
    Vertical,
}

/// Cross-axis alignment for [`Stack`](crate::widget::Stack) children.
///
/// `Stretch` is intentionally omitted from S10.2: stretching requires a
/// second pass that re-lays out children with tightened cross-axis
/// constraints, which would muddy the determinism story for the snapshot
/// tests. Containers that need stretch can compose a `Stack` inside an
/// outer fixed-size cell for now.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Align {
    /// Anchor children at the cross-axis start (left for horizontal,
    /// top for vertical).
    Start,
    /// Center children on the cross axis.
    Center,
    /// Anchor children at the cross-axis end.
    End,
}

/// Compute the cross-axis offset for a child of `child_size` inside a
/// container of `parent_size`, given the requested [`Align`].
///
/// Public so [`Stack`](crate::widget::Stack) can share the helper with
/// downstream containers (e.g. a future `Wrap`) without duplicating the
/// arithmetic.
pub fn cross_axis_offset(align: Align, parent_extent: f32, child_extent: f32) -> f32 {
    let slack = (parent_extent - child_extent).max(0.0);
    match align {
        Align::Start => 0.0,
        Align::Center => slack * 0.5,
        Align::End => slack,
    }
}

/// Clamp `value` into `[min, max]`.
///
/// Avoids `f32::clamp`'s `debug_assert!(min <= max)` panic — layout
/// constraints are constructed at runtime from widget measure output
/// and the compiler cannot prove the relationship statically. NaN
/// behaviour matches `f32::clamp`: `NaN < min` and `NaN > max` are
/// both false, so a NaN input falls through unchanged. Widget
/// implementations should not produce NaN sizes; if one ever does,
/// the snapshot test `dump_layout` output will surface it as the
/// literal string "NaN" and the regression will be loud.
fn clamp(value: f32, min: f32, max: f32) -> f32 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tight_constraints_pin_size() {
        let c = LayoutConstraints::tight(Size::new(100.0, 50.0));
        assert_eq!(c.constrain(Size::new(10.0, 10.0)), Size::new(100.0, 50.0));
        assert_eq!(
            c.constrain(Size::new(1000.0, 1000.0)),
            Size::new(100.0, 50.0)
        );
    }

    #[test]
    fn loose_constraints_clamp_to_max() {
        let c = LayoutConstraints::loose(Size::new(100.0, 50.0));
        assert_eq!(c.constrain(Size::new(40.0, 30.0)), Size::new(40.0, 30.0));
        assert_eq!(
            c.constrain(Size::new(1000.0, 1000.0)),
            Size::new(100.0, 50.0)
        );
    }

    #[test]
    fn cross_axis_offset_handles_each_align() {
        assert_eq!(cross_axis_offset(Align::Start, 100.0, 30.0), 0.0);
        assert_eq!(cross_axis_offset(Align::Center, 100.0, 30.0), 35.0);
        assert_eq!(cross_axis_offset(Align::End, 100.0, 30.0), 70.0);
    }

    #[test]
    fn cross_axis_offset_with_overflowing_child_clamps_to_zero() {
        // Child wider than parent — Start / Center / End all anchor at 0
        // rather than producing a negative offset.
        assert_eq!(cross_axis_offset(Align::Center, 10.0, 100.0), 0.0);
        assert_eq!(cross_axis_offset(Align::End, 10.0, 100.0), 0.0);
    }
}
