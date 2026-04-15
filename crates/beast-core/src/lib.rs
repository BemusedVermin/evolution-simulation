//! Beast Evolution Game — Layer 0 foundations.
//!
//! This crate establishes the deterministic numerical foundation on which every
//! other `beast-*` crate is built. It is intentionally small and has **no
//! dependencies on any other workspace crate**.
//!
//! The non-negotiable invariants enforced here are documented in
//! `documentation/INVARIANTS.md` at the repository root. In short:
//!
//! * All simulation-state math flows through [`Q3232`], a Q32.32 fixed-point
//!   type with saturating-by-default arithmetic.
//! * Pseudo-randomness flows through [`Prng`], a wrapper around
//!   `Xoshiro256PlusPlus`, with [`Prng::split_stream`] used to derive
//!   independent streams per simulation subsystem.
//! * No wall-clock reads, no OS RNG, no floating point on the sim path.
//!
//! Crate-level lints forbid `unsafe` code and warn on float arithmetic; do not
//! relax either without a written architectural decision in
//! `documentation/PROGRESS_LOG.md`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod entity;
pub mod error;
pub mod fixed_point;
pub mod math;
pub mod prng;
pub mod time;

pub use entity::{EntityId, EntityIdAllocator};
pub use error::{Error, Result};
pub use fixed_point::Q3232;
pub use math::{clamp01, gaussian_q3232, lerp_q3232};
pub use prng::{Prng, Stream};
pub use time::TickCounter;
