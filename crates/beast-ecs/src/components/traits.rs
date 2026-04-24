//! Genetic/phenotypic components — [`GenomeComponent`] and
//! [`PhenotypeComponent`].
//!
//! These are newtype wrappers over types defined in `beast-genome`
//! (L1) and `beast-primitives` (L1). Keeping the `specs::Component`
//! impls here avoids adding a `specs` dependency to the L1 crates — the
//! ECS is an L3 concern.

use beast_genome::Genome;
use beast_primitives::PrimitiveEffect;
use serde::{Deserialize, Serialize};
use specs::{Component, DenseVecStorage};

// NOTE: `PrimitiveEffect` does not currently derive `Serialize` /
// `Deserialize` (see `beast-primitives`). When S7 adds save/load, the
// derive there will land in the same PR that re-enables it on
// [`PhenotypeComponent`]. Until then, `PhenotypeComponent` is save-path
// opaque — the interpreter re-derives it each tick from the genome, so
// a save can simply drop the cached phenotype and rebuild.

/// A creature's evolvable genotype. Newtype over [`beast_genome::Genome`]
/// so the L1 genome crate does not have to know about `specs`.
///
/// Mutable per tick (the mutation system in Stage 1 writes it back).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenomeComponent(pub Genome);

impl GenomeComponent {
    /// Wrap an existing genome as a component.
    #[must_use]
    pub fn new(genome: Genome) -> Self {
        Self(genome)
    }

    /// Immutable view of the underlying genome.
    #[must_use]
    pub fn genome(&self) -> &Genome {
        &self.0
    }

    /// Mutable view — the mutation system in Stage 1 calls this to
    /// apply point mutations.
    pub fn genome_mut(&mut self) -> &mut Genome {
        &mut self.0
    }
}

impl Component for GenomeComponent {
    type Storage = DenseVecStorage<Self>;
}

/// The creature's current phenotype — the `Vec<PrimitiveEffect>`
/// produced by the interpreter in Stage 2. Read by every downstream
/// physics/combat/physiology system; rewritten each tick by Stage 2.
///
/// Stored as a sorted `Vec` rather than a `Set` because ordering
/// already comes out of the interpreter sorted by `primitive_id`
/// (emission merges by `(primitive_id, site_id)` and returns sorted).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PhenotypeComponent {
    /// Primitive effects emitted by Stage 2 for this creature, sorted by
    /// `(primitive_id, site_id)` per the interpreter contract.
    pub effects: Vec<PrimitiveEffect>,
}

impl PhenotypeComponent {
    /// Build from an already-sorted effect list (the interpreter emits
    /// them sorted — no re-sorting required here).
    #[must_use]
    pub fn new(effects: Vec<PrimitiveEffect>) -> Self {
        Self { effects }
    }
}

impl Component for PhenotypeComponent {
    type Storage = DenseVecStorage<Self>;
}
