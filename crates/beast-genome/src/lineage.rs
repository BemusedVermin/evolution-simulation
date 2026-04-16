//! Lineage tags for phylogenetic tracking.
//!
//! Every [`crate::TraitGene`] carries a [`LineageTag`] — an opaque 64-bit
//! identifier that survives mutation and inheritance. When a paralog is
//! created by gene duplication (System 01 §2B) the parent keeps its tag and
//! the new paralog receives a fresh one, giving speciation metrics a stable
//! ancestor marker without walking a full phylogenetic tree.
//!
//! Tags are drawn from the caller's PRNG so allocation is deterministic. The
//! 64-bit space means collision within a single world is astronomically
//! unlikely, but callers that care about hard uniqueness can validate with
//! [`crate::Genome::validate`] — the genome refuses to hold two genes with
//! the same tag.

use beast_core::Prng;
use serde::{Deserialize, Serialize};

/// A phylogenetic tag identifying a gene's lineage.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LineageTag(u64);

impl LineageTag {
    /// Construct a tag from an explicit `u64`. Use in tests or when loading
    /// from a save file; live code should prefer [`LineageTag::fresh`].
    #[inline]
    #[must_use]
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }

    /// Draw a fresh tag from the `Genetics` PRNG stream.
    ///
    /// Takes `&mut Prng` rather than an owned PRNG so callers can continue
    /// to use the stream for the rest of their mutation batch.
    #[inline]
    #[must_use]
    pub fn fresh(rng: &mut Prng) -> Self {
        Self(rng.next_u64())
    }

    /// Return the raw `u64` value.
    #[inline]
    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

impl core::fmt::Display for LineageTag {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_is_deterministic() {
        let mut a = Prng::from_seed(42);
        let mut b = Prng::from_seed(42);
        for _ in 0..128 {
            assert_eq!(LineageTag::fresh(&mut a), LineageTag::fresh(&mut b));
        }
    }

    #[test]
    fn fresh_does_not_collide_trivially() {
        // 1024 draws from a good 64-bit PRNG should be collision-free.
        let mut rng = Prng::from_seed(7);
        let mut seen = std::collections::BTreeSet::new();
        for _ in 0..1024 {
            let tag = LineageTag::fresh(&mut rng);
            assert!(seen.insert(tag), "unexpected collision");
        }
    }

    #[test]
    fn serde_roundtrip() {
        let tag = LineageTag::from_raw(0xABCD_1234_5678_9ABCu64);
        let json = serde_json::to_string(&tag).unwrap();
        let back: LineageTag = serde_json::from_str(&json).unwrap();
        assert_eq!(tag, back);
    }
}
