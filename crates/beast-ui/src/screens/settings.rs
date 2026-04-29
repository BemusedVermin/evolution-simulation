//! Settings screen (S10.4).
//!
//! Three side-by-side `Card`s — *Rendering*, *Audio*, *Accessibility*
//! — each holding a static option list. The values are display-only:
//! interactive editing of the settings (toggles, sliders) lands when
//! the application layer wires real input bindings in S13.
//!
//! Per `documentation/INVARIANTS.md` §6 the settings screen never
//! reads or mutates sim state. The rendered text is pure static
//! data baked into the screen builder.

use crate::layout::{Align, Axis};
use crate::screens::frame::screen_frame;
use crate::widget::{Card, IdAllocator, Label, Stack};
use crate::{Size, WidgetTree};

/// Build the settings screen for a 1280×720 viewport.
pub fn settings() -> WidgetTree {
    let mut ids = IdAllocator::new();

    let mut content = Stack::new(ids.allocate(), Axis::Horizontal)
        .with_gap(16.0)
        .with_align(Align::Start);

    content.push_child(Box::new(option_card(
        &mut ids,
        "Rendering",
        &[
            "resolution: 1280x720",
            "vsync: on",
            "fullscreen: off",
            "render scale: 1.0",
        ],
    )));
    content.push_child(Box::new(option_card(
        &mut ids,
        "Audio",
        &[
            "master volume: 80",
            "music volume: 60",
            "sfx volume: 80",
            "mute on focus loss: on",
        ],
    )));
    content.push_child(Box::new(option_card(
        &mut ids,
        "Accessibility",
        &[
            "high contrast: off",
            "reduce motion: off",
            "ui scale: 1.0",
            "colorblind mode: off",
        ],
    )));

    let frame = screen_frame(&mut ids, || "settings".to_owned(), Box::new(content));

    WidgetTree::new(Box::new(frame), Size::new(1280.0, 720.0))
}

fn option_card(ids: &mut IdAllocator, title: &str, options: &[&str]) -> Card {
    let mut card = Card::new(ids.allocate(), title.to_owned());
    for option in options {
        card.push_child(Box::new(Label::new(ids.allocate(), option.to_owned())));
    }
    card
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dump_layout;

    #[test]
    fn settings_screen_has_three_cards() {
        let mut tree = settings();
        assert!(tree.layout());
        let dump = dump_layout(&tree);
        // 1 status-bar card + 3 option cards = 4 Card lines.
        let cards = dump.lines().filter(|l| l.starts_with("Card#")).count();
        assert_eq!(
            cards, 4,
            "expected 4 cards (status + 3 option cards), dump:\n{dump}"
        );
    }

    #[test]
    fn settings_screen_paints_known_option_strings() {
        let mut tree = settings();
        assert!(tree.layout());
        let mut ctx = crate::paint::PaintCtx::new();
        tree.paint(&mut ctx);
        let mut texts = ctx
            .commands()
            .iter()
            .filter_map(|cmd| match cmd {
                crate::paint::DrawCmd::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        texts.sort_unstable();
        assert!(texts.iter().any(|t| t.contains("vsync")));
        assert!(texts.iter().any(|t| t.contains("master volume")));
        assert!(texts.iter().any(|t| t.contains("colorblind mode")));
    }
}
