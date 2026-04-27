//! Input events delivered to widgets and the result widgets return.
//!
//! S10.1 only needs enough of the event surface to verify that primitives
//! like [`Button`](crate::Button) and modal [`Dialog`](crate::Dialog)
//! consume / forward events correctly. Full keyboard / focus dispatch
//! through the tree lands in S10.3.

use serde::{Deserialize, Serialize};

/// Mouse buttons handled by the widget framework.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MouseButton {
    /// Primary (usually left) button.
    Primary,
    /// Secondary (usually right) button.
    Secondary,
    /// Middle button.
    Middle,
}

/// Subset of physical key codes the widget framework cares about.
///
/// The vocabulary is intentionally narrow; keys that don't matter for
/// widget interaction (e.g. function keys, media keys) bubble up as
/// [`UiEvent::TextInput`] when applicable or are simply not represented.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyCode {
    /// Enter / return key.
    Enter,
    /// Escape key.
    Escape,
    /// Tab key — used for focus cycling in S10.3.
    Tab,
    /// Spacebar.
    Space,
    /// Backspace.
    Backspace,
    /// Up arrow.
    ArrowUp,
    /// Down arrow.
    ArrowDown,
    /// Left arrow.
    ArrowLeft,
    /// Right arrow.
    ArrowRight,
    /// Letter or digit key — payload is the lowercase ASCII glyph.
    Char(char),
}

/// Modifier-key bitflags layered over [`KeyCode`].
///
/// Implemented as a small newtype rather than pulling in the `bitflags`
/// crate — this is the only bitset in `beast-ui` and the surface fits in
/// a few `const`s + `BitOr`. If we add a second bitflags type later, swap
/// to the real crate.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyMods(u8);

impl KeyMods {
    /// No modifiers held.
    pub const NONE: Self = Self(0);
    /// Shift held.
    pub const SHIFT: Self = Self(0b0001);
    /// Control held.
    pub const CTRL: Self = Self(0b0010);
    /// Alt held.
    pub const ALT: Self = Self(0b0100);
    /// Super / Cmd / Windows key held.
    pub const SUPER: Self = Self(0b1000);

    /// Returns true if every bit in `other` is set in `self`.
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Returns true if any bit in `other` is set in `self`.
    pub const fn intersects(self, other: Self) -> bool {
        (self.0 & other.0) != 0
    }
}

impl core::ops::BitOr for KeyMods {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitAnd for KeyMods {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

impl core::ops::BitOrAssign for KeyMods {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

/// Pair of [`KeyCode`] + [`KeyMods`] passed to widgets.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Modifiers {
    /// Physical key.
    pub key: KeyCode,
    /// Modifier mask.
    pub mods: KeyMods,
}

/// Input event delivered to a [`Widget`](crate::Widget).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum UiEvent {
    /// Mouse moved to an absolute widget-space coordinate.
    MouseMove {
        /// X coordinate.
        x: f32,
        /// Y coordinate.
        y: f32,
    },
    /// Mouse button pressed at the cursor's last position.
    MouseDown {
        /// Which button was pressed.
        button: MouseButton,
    },
    /// Mouse button released at the cursor's last position.
    MouseUp {
        /// Which button was released.
        button: MouseButton,
    },
    /// Key was pressed.
    KeyDown(Modifiers),
    /// Key was released.
    KeyUp(Modifiers),
    /// Text input from the IME / keyboard layer (post-translation).
    TextInput(String),
    /// Time delta for tick-based animations / timers.
    Tick {
        /// Milliseconds since the previous tick.
        dt_ms: u32,
    },
}

/// Outcome of a widget's [`Widget::handle_event`](crate::Widget::handle_event)
/// call.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventResult {
    /// The widget consumed the event; siblings / ancestors should not see
    /// it again.
    Consumed,
    /// The widget did not act on the event but agrees to let it bubble up.
    Ignored,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_mods_combine_via_bitor() {
        let m = KeyMods::SHIFT | KeyMods::CTRL;
        assert!(m.contains(KeyMods::SHIFT));
        assert!(m.contains(KeyMods::CTRL));
        assert!(!m.contains(KeyMods::ALT));
        assert!(m.intersects(KeyMods::SHIFT));
    }

    #[test]
    fn modifiers_carry_key_and_mask() {
        let m = Modifiers {
            key: KeyCode::Char('a'),
            mods: KeyMods::SHIFT,
        };
        assert_eq!(m.key, KeyCode::Char('a'));
        assert!(m.mods.contains(KeyMods::SHIFT));
    }
}
