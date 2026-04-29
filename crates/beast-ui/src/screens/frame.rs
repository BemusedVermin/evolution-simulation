//! Shared screen frame: status bar + content area + dialog overlay.
//!
//! Every screen in S10.4 wraps its content in the same vertical
//! [`Stack`]: a status bar that runs the full viewport width along
//! the top, the content widget directly below, and a slot reserved
//! for an optional modal dialog overlay. Centralising the frame here
//! means individual screen builders only need to author the content
//! widget and the status text — the layout snapshot tests then stay
//! tight against the per-screen geometry instead of re-asserting on
//! the chrome.
//!
//! Per `documentation/INVARIANTS.md` §6 the frame is read-only: it
//! never reads or mutates sim state directly. The status text comes
//! through a [`crate::DataBinding`] resolved at paint time.

use crate::layout::{Align, Axis};
use crate::widget::{Card, IdAllocator, Label, Stack, Widget};
use crate::{Bound, FnBinding};

/// Build the shared screen frame around a content widget.
///
/// The returned [`Stack`] has the canonical layout:
///
/// 1. **Status bar** — a single-row [`Stack`] containing one [`Label`]
///    whose text is bound to `status` via [`FnBinding`]. Status text
///    re-evaluates on every paint.
/// 2. **Content area** — `content`, sized by its own measure /
///    layout impl.
///
/// The returned root claims the full viewport via the tree's tight
/// constraint, so the status bar runs full width regardless of the
/// content's intrinsic size.
///
/// Screens that need a modal dialog overlay should push the dialog
/// **after** the content widget; the dialog will render on top and
/// eat outside clicks per the [`crate::Dialog`] modal contract.
pub(crate) fn screen_frame<F>(ids: &mut IdAllocator, status: F, content: Box<dyn Widget>) -> Stack
where
    F: Fn() -> String + 'static,
{
    let mut root = Stack::new(ids.allocate(), Axis::Vertical)
        .with_gap(0.0)
        .with_align(Align::Start);

    // Status bar: one Label bound to the status closure, wrapped in a
    // Card so the snapshot output reads `Card#X Label#Y`. The Card's
    // title double-doubles as a separator strip across the top of the
    // viewport.
    let mut status_bar = Card::new(ids.allocate(), "status");
    let status_label = Label::new(ids.allocate(), "");
    status_bar.push_child(Box::new(Bound::new(status_label, FnBinding::new(status))));
    root.push_child(Box::new(status_bar));

    root.push_child(content);

    root
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::{PaintCtx, Size};
    use crate::widget::Label;
    use crate::WidgetTree;

    #[test]
    fn frame_paints_status_text_from_binding() {
        let mut ids = IdAllocator::new();
        let placeholder = Label::new(ids.allocate(), "content");
        let frame = screen_frame(
            &mut ids,
            || "tick: 42 | creatures: 7".to_owned(),
            Box::new(placeholder),
        );
        let mut tree = WidgetTree::new(Box::new(frame), Size::new(1280.0, 720.0));
        let _ = tree.layout();
        let mut ctx = PaintCtx::new();
        tree.paint(&mut ctx);
        let text_cmd = ctx
            .commands()
            .iter()
            .filter_map(|cmd| match cmd {
                crate::paint::DrawCmd::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .find(|text| text.contains("tick: 42"));
        assert!(
            text_cmd.is_some(),
            "status binding text should appear in the recorded paint commands"
        );
    }
}
