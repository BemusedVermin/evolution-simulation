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

use crate::error::EcsError;

// `PrimitiveEffect` derives `Serialize` / `Deserialize` (added in
// audit fix #67); the per-tick interpreter output is therefore
// save-path-routable. The `BodySite` field on each effect rides along
// via `beast_core::BodySite`'s own derive. The save layer
// (`beast-serde::SerializedEntity::phenotype`) round-trips this
// component as `Option<PhenotypeComponent>` so loaded sims hash
// identically to pre-save state.

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
    ///
    /// # Caller contract
    ///
    /// This handle bypasses [`Genome::validate`]: the mutation
    /// operators in `beast-genome` are written so that, **applied as a
    /// set in their documented order**, they preserve every genome
    /// invariant (modifier indices in range, no self-loops, no
    /// duplicate lineage tags, channel-count parity). A caller that
    /// performs partial / hand-rolled edits through this handle owns
    /// the responsibility to call [`Genome::validate`] before the
    /// genome leaves the tick (e.g., before persistence in Stage 7 or
    /// before being read back into the interpreter on the next tick).
    ///
    /// Save-game serialisation (`beast-serde`) re-validates loaded
    /// genomes as a defense-in-depth step — but a corrupt genome
    /// produced by a buggy custom mutator and consumed by the same
    /// process before save will not be caught. New code that wants a
    /// validated mutation point should add a `try_mutate` helper that
    /// runs `validate` before returning, rather than handing out the
    /// raw `&mut Genome`.
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
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhenotypeComponent {
    /// Primitive effects emitted by Stage 2 for this creature, sorted by
    /// `(primitive_id, site_id)` per the interpreter contract.
    pub effects: Vec<PrimitiveEffect>,
}

impl PhenotypeComponent {
    /// Build from an already-sorted effect list, **trusting the caller**.
    ///
    /// In debug builds this `debug_assert!`s that `effects` is sorted
    /// by `(primitive_id, body_site)`. Release builds skip the check.
    /// Use this from the per-tick interpreter hot path where the sort
    /// is guaranteed by `beast_interpreter::emit_primitives` and the
    /// caller has already paid the sort cost.
    ///
    /// For untrusted input — save-loaders, fixture builders, fuzz
    /// harnesses — call [`Self::try_new`] instead, which validates in
    /// release builds and returns
    /// [`EcsError::PhenotypeNotSorted`] on violation.
    #[must_use]
    pub fn new(effects: Vec<PrimitiveEffect>) -> Self {
        debug_assert!(
            Self::find_unsorted_index(&effects).is_none(),
            "PhenotypeComponent::new: effects must be sorted by \
             (primitive_id, body_site); caller bypassed the interpreter \
             emission path. Use try_new for untrusted input."
        );
        Self { effects }
    }

    /// Build from an effect list, validating sort order in **all build
    /// profiles**. Use from save-loaders, test fixtures, and any path
    /// where the caller cannot statically guarantee sort order — a
    /// release build with an unsorted input would otherwise silently
    /// break INVARIANTS §1 (downstream systems hash phenotypes in
    /// visit order).
    ///
    /// # Errors
    ///
    /// Returns [`EcsError::PhenotypeNotSorted`] with the zero-based
    /// index of the first out-of-order pair when `effects` is not
    /// sorted by `(primitive_id, body_site)`.
    pub fn try_new(effects: Vec<PrimitiveEffect>) -> Result<Self, EcsError> {
        match Self::find_unsorted_index(&effects) {
            Some(index) => Err(EcsError::PhenotypeNotSorted { index }),
            None => Ok(Self { effects }),
        }
    }

    /// Return the index `i` of the first pair where
    /// `effects[i] > effects[i + 1]` by `(primitive_id, body_site)`,
    /// or `None` if the slice is sorted. Shared by [`Self::new`]'s
    /// debug assertion and [`Self::try_new`]'s release validation so
    /// the two paths can never disagree.
    fn find_unsorted_index(effects: &[PrimitiveEffect]) -> Option<usize> {
        effects.windows(2).enumerate().find_map(|(i, pair)| {
            let lhs = (&pair[0].primitive_id, &pair[0].body_site);
            let rhs = (&pair[1].primitive_id, &pair[1].body_site);
            (lhs > rhs).then_some(i)
        })
    }
}

impl Component for PhenotypeComponent {
    type Storage = DenseVecStorage<Self>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use beast_core::{BodySite, EntityId, Q3232};
    use beast_primitives::Provenance;
    use std::collections::BTreeMap;

    fn effect(primitive_id: &str, body_site: Option<BodySite>) -> PrimitiveEffect {
        PrimitiveEffect {
            primitive_id: primitive_id.into(),
            body_site,
            source_channels: Vec::new(),
            parameters: BTreeMap::new(),
            activation_cost: Q3232::ZERO,
            emitter: EntityId::new(0),
            provenance: Provenance::Core,
        }
    }

    #[test]
    fn try_new_accepts_sorted_input() {
        let effects = vec![
            effect("alpha", None),
            effect("alpha", Some(BodySite::Head)),
            effect("beta", None),
        ];
        let component = PhenotypeComponent::try_new(effects.clone()).unwrap();
        assert_eq!(component.effects, effects);
    }

    #[test]
    fn try_new_accepts_empty_and_single_element_input() {
        assert!(PhenotypeComponent::try_new(Vec::new()).is_ok());
        assert!(PhenotypeComponent::try_new(vec![effect("zeta", None)]).is_ok());
    }

    #[test]
    fn try_new_rejects_unsorted_input_with_first_violating_index() {
        // alpha < beta < gamma is correct, but the second pair (beta, alpha)
        // is unsorted — index 1 must be reported, not 0.
        let effects = vec![
            effect("alpha", None),
            effect("beta", None),
            effect("alpha", None),
            effect("gamma", None),
        ];
        let err = PhenotypeComponent::try_new(effects).unwrap_err();
        match err {
            EcsError::PhenotypeNotSorted { index } => assert_eq!(index, 1),
            other => panic!("expected PhenotypeNotSorted, got {other:?}"),
        }
    }

    #[test]
    fn try_new_uses_body_site_as_secondary_key() {
        // Same primitive_id; body_site None < Some(0) < Some(1) per Option's
        // derived Ord. Out-of-order body_site within a primitive_id must
        // still trigger.
        let effects = vec![
            effect("alpha", Some(BodySite::Jaw)),
            effect("alpha", Some(BodySite::Head)),
        ];
        let err = PhenotypeComponent::try_new(effects).unwrap_err();
        assert!(matches!(err, EcsError::PhenotypeNotSorted { index: 0 }));
    }

    #[test]
    fn new_accepts_sorted_input_in_release_or_debug() {
        // Mirror try_new's happy path so both constructors stay aligned;
        // the debug_assert! inside `new` would catch unsorted input under
        // `cargo test` (debug profile) but we want explicit coverage that
        // the sorted-input fast path does not panic either way.
        let effects = vec![effect("alpha", None), effect("beta", None)];
        let component = PhenotypeComponent::new(effects.clone());
        assert_eq!(component.effects, effects);
    }
}
