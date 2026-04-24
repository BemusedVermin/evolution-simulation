//! Beast Evolution Game — Layer 4 simulation orchestration.
//!
//! Owns the top-level [`Simulation`] type: the [`beast_ecs::EcsWorld`] +
//! [`beast_ecs::Resources`] pair plus the scheduler that drives the
//! eight-stage tick loop from
//! `documentation/architecture/ECS_SCHEDULE.md`.
//!
//! See `documentation/architecture/CRATE_LAYOUT.md` §Layer 4 for this
//! crate's scope, and `documentation/INVARIANTS.md` §1 for the
//! determinism contract the tick loop enforces.
//!
//! # Non-negotiable invariants
//!
//! * Determinism: bit-identical replay is a CI gate once S6.6 lands.
//!   Every mutation this crate performs goes through
//!   [`beast_core::Q3232`] arithmetic, sorted iteration, and
//!   subsystem-specific PRNG streams taken from
//!   [`beast_ecs::Resources`].
//! * No wall-clock reads on the sim path — S6.4's budget tracker reads
//!   `std::time::Instant` for **observation only**; its output never
//!   influences sim state.
//!
//! # Sprint scope (S6 — epic [#18])
//!
//! | Story | Module                      | Issue |
//! |-------|-----------------------------|-------|
//! | 6.1   | [`simulation`]              | #114  |
//! | 6.2   | [`schedule`]                | TBD   |
//! | 6.3   | [`schedule`] parallelism    | TBD   |
//! | 6.4   | [`budget`]                  | TBD   |
//! | 6.5   | [`determinism`]             | TBD   |
//! | 6.6   | `tests/determinism_test.rs` | TBD   |
//! | 6.7   | [`budget`] profiling        | TBD   |

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

// `budget` is pub(crate): its public surface (`TickResult`) is
// re-exported at the crate root below. Keeping the module path
// crate-internal prevents future items added to `budget.rs` from
// accidentally becoming part of the public API without an explicit
// re-export decision.
pub(crate) mod budget;
pub mod determinism;
pub mod error;
pub mod schedule;
pub mod simulation;
pub mod tick;

pub use budget::TickResult;
pub use determinism::compute_state_hash;
pub use error::{Result, SimError};
pub use simulation::{Simulation, SimulationConfig};
