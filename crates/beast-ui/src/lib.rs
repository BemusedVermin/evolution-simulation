//! Retained-mode widget framework for the Beast Evolution Game UI.
//!
//! `beast-ui` defines the [`Widget`] trait and a small set of primitive
//! widgets ([`Button`], [`Label`], [`List`], [`Card`], [`Dialog`]) that
//! higher layers compose into screens (S10.4). Per
//! `documentation/INVARIANTS.md` §6, the UI layer is **read-only** against
//! simulation state — none of the widget primitives in this crate take a
//! mutable handle to anything sim-side.
//!
//! # Feature flags
//!
//! * `sdl` (default) — re-exports `beast-render` with its real SDL3 backend.
//! * `headless` — re-exports `beast-render` in headless mode so the crate
//!   builds on display-less CI runners.
//!
//! Both features delegate the SDL link to `beast-render`. This story only
//! defines the widget data model + paint recording surface; SDL drawing is
//! wired in later sprints when screens (S10.4) embed renderer viewports.
//!
//! # Determinism
//!
//! Widget primitives are pure data structures. They never read sim state and
//! never feed values back into the simulation. Paint output is recorded into
//! a deterministic [`paint::PaintCtx`] command list — useful for snapshot
//! tests and for future flushing into a real renderer surface.

#![warn(missing_docs)]

pub mod event;
pub mod paint;
pub mod widget;

pub use event::{EventResult, KeyCode, KeyMods, Modifiers, MouseButton, UiEvent};
pub use paint::{Color, DrawCmd, PaintCtx, Point, Rect, Size};
pub use widget::{Button, Card, Dialog, IdAllocator, Label, List, ListItem, Widget, WidgetId};
