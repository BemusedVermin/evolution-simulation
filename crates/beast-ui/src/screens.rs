//! Screen builders that compose [`crate::Widget`] primitives into
//! [`crate::WidgetTree`]s ready for the renderer + dispatch loop.
//!
//! Per `documentation/INVARIANTS.md` §6 every screen builder takes
//! read-only references to its inputs (`&dyn WorldStatus`,
//! `&dyn ChroniclerQuery`, `&BiomeView`, `&EncounterSnapshot`) — none
//! of them accept `&mut World` / `&mut Chronicler`. The trees they
//! produce dispatch input through the standard widget pipeline; the
//! actual *application* of selection / press events to sim state
//! happens at the application layer (S13) by inspecting widget state
//! after dispatch.
//!
//! Screen surface:
//!
//! * [`world_map`] — archipelago renderer + action bar.
//! * [`bestiary`] — list-plus-detail panel against
//!   `&dyn ChroniclerQuery`.
//! * [`settings`] — three static option `Card`s.
//! * [`encounter`] — encounter renderer + creature list + action bar.

pub mod bestiary;
pub mod data;
pub mod encounter;
pub(crate) mod frame;
pub mod settings;
pub mod world_map;

pub use bestiary::{bestiary, BestiaryPanel};
pub use data::{BiomeView, EncounterCreatureSnapshot, EncounterSnapshot, WorldStatus};
pub use encounter::encounter;
pub use settings::settings;
pub use world_map::world_map;
