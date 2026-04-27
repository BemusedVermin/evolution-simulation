//! Per-tick primitive emission snapshot.
//!
//! [`PrimitiveSnapshot`] is the unit the chronicler ingests. The
//! interpreter / sim layer is expected to feed one snapshot per
//! `(tick, entity)` pair where the entity emitted at least one primitive
//! that tick.

use std::collections::BTreeSet;

use beast_core::{EntityId, TickCounter};
use serde::{Deserialize, Serialize};

/// Snapshot of one entity's primitive emissions on a single tick.
///
/// `primitives` is a [`BTreeSet`] so iteration order is fixed — that's
/// the property that makes [`PatternSignature`](crate::PatternSignature)
/// stable. A `Vec` would let callers accidentally vary the signature by
/// reordering pushes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrimitiveSnapshot {
    /// Tick the snapshot was taken on.
    pub tick: TickCounter,
    /// Entity that emitted these primitives.
    pub entity: EntityId,
    /// Set of primitive ids active for this entity on this tick.
    ///
    /// The phenotype interpreter is responsible for converting its raw
    /// `Vec<PrimitiveEffect>` into this set: collapse duplicates by
    /// `primitive_id`, drop body-site information (the chronicler
    /// signature is a whole-creature pattern), and collect into the
    /// `BTreeSet`. Empty sets are valid; they hash to the all-empty
    /// signature.
    pub primitives: BTreeSet<String>,
}

impl PrimitiveSnapshot {
    /// Construct a snapshot from `(tick, entity, primitives)`. The
    /// primitives iterator is consumed into a `BTreeSet` so callers
    /// don't need to pre-sort.
    pub fn new<I, S>(tick: TickCounter, entity: EntityId, primitives: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            tick,
            entity,
            primitives: primitives.into_iter().map(Into::into).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_collects_primitives_into_btreeset() {
        let s = PrimitiveSnapshot::new(
            TickCounter::new(7),
            EntityId::new(42),
            ["b", "a", "c", "a"].iter().copied(),
        );
        let mut iter = s.primitives.iter();
        assert_eq!(iter.next().map(String::as_str), Some("a"));
        assert_eq!(iter.next().map(String::as_str), Some("b"));
        assert_eq!(iter.next().map(String::as_str), Some("c"));
        assert_eq!(iter.next(), None);
    }
}
