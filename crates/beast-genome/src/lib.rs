//! Beast Evolution Game — Layer 2 genotype & mutation crate.
//!
//! See `crates/beast-genome/README.md` and
//! `documentation/systems/01_evolutionary_model.md` for the canonical
//! specification. Non-negotiable invariants enforced here:
//!
//! * All numeric fields on sim-state types use [`beast_core::Q3232`].
//! * All randomness comes from the caller-supplied [`beast_core::Prng`]
//!   handle (derived from [`beast_core::Stream::Genetics`]).
//! * Genomes iterate by index; no hashing in any sim-state container.
//!
//! Sprint S3 lands each story as its own module; the public surface is
//! re-exported below.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod body_site;
pub mod error;
pub mod gene;
pub mod genome;
pub mod lineage;
pub mod modifier;
pub mod mutation;

pub use body_site::BodyVector;
pub use error::{GenomeError, Result};
pub use gene::{EffectVector, Target, Timing, TraitGene};
pub use genome::{Genome, GenomeParams};
pub use lineage::LineageTag;
pub use modifier::{Modifier, ModifierEffect};
pub use mutation::{mutate_point, mutate_regulatory};
