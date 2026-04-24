//! Beast Evolution Game — Layer 3 ECS foundation.
//!
//! Wraps [`specs::World`] in an [`EcsWorld`] facade so higher-layer crates
//! ([`beast_sim`], [`beast_chronicler`], etc.) never import `specs`
//! directly. Keeping the ECS backend behind this wall means we can swap
//! implementations (e.g., `hecs`, a hand-rolled store) later without
//! touching every system.
//!
//! See `documentation/architecture/CRATE_LAYOUT.md` §Layer 3 for the scope
//! of this crate, and `documentation/architecture/ECS_SCHEDULE.md` for the
//! eight-stage tick loop the scheduler built on top of this wrapper runs.
//!
//! # Non-negotiable invariants
//!
//! * Determinism (INVARIANTS §1) — the wrapper itself does no iteration,
//!   but the types it exposes are chosen so downstream systems can iterate
//!   in sorted entity-key order. See [`entity_id`] (S5.5) for the sorted
//!   index that enforces this.
//! * No floating point in sim state — `beast_core::Q3232` is the unit of
//!   work; this crate re-exports `beast_core` only through its public API,
//!   never through its internal state.
//!
//! # Sprint scope (S5 — epic [#17])
//!
//! | Story | Module                        | Issue |
//! |-------|-------------------------------|-------|
//! | 5.1   | [`world`]                     | #100  |
//! | 5.2   | [`components`]                | TBD   |
//! | 5.3   | [`system`]                    | TBD   |
//! | 5.4   | [`resources`]                 | TBD   |
//! | 5.5   | [`entity_id`]                 | TBD   |
//! | 5.6   | [`storage`]                   | TBD   |
//! | 5.7   | `tests/determinism.rs`        | TBD   |
//!
//! The other modules are stubs that S5.2–S5.7 fill in; this file only
//! wires the surface so each story can ship its own PR without touching
//! the crate-level layout.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod components;
pub mod entity_id;
pub mod error;
pub mod resources;
pub mod storage;
pub mod system;
pub mod world;

pub use entity_id::{MarkerKind, SortedEntityIndex};
pub use error::{EcsError, Result};
pub use storage::{for_each_entity_of, Join, ParJoin};
pub use system::{Resources, System, SystemStage};
pub use world::EcsWorld;

// Re-export the narrow slice of `specs` downstream crates legitimately
// need. Keeping these behind our crate lets us audit every use-site and
// swap the backend later. Anything not listed here is intentionally
// private to this crate.
pub use specs::{
    Builder, Component, DenseVecStorage, Entity, EntityBuilder, NullStorage, ReadStorage,
    VecStorage, WorldExt, WriteStorage,
};
