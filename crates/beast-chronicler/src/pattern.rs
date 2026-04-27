//! Pattern signature + observation record.
//!
//! [`PatternSignature`] is a 32-byte BLAKE3 hash over a canonical
//! byte stream of the sorted, length-prefixed primitive ids.
//! [`PatternObservation`] holds the per-signature accumulators that
//! later sprints (S10.6 label generation, S10.7 query API) read from.

use std::collections::BTreeSet;

use beast_core::TickCounter;
use serde::{Deserialize, Serialize};

/// 32-byte BLAKE3 fingerprint of a primitive set.
///
/// Two snapshots with identical sets of primitive ids hash to the same
/// signature, regardless of insertion order. Snapshots that differ in
/// even one primitive id hash to a (cryptographically) different
/// signature — collisions are not a concern for the simulation's lifetime.
///
/// Public byte field so save / load can transit signatures without
/// going through a `From<[u8; 32]>` ceremony.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PatternSignature(pub [u8; 32]);

impl PatternSignature {
    /// Compute the signature of an iterable of primitive ids.
    ///
    /// The caller does not need to pre-sort; this routine collects into
    /// a sorted `BTreeSet` first so insertion order does not affect the
    /// result. Duplicate ids in the input collapse to one entry.
    ///
    /// # Determinism contract
    ///
    /// The byte stream fed to BLAKE3 is:
    ///
    /// ```text
    /// for each primitive id in ascending lexicographic order:
    ///     u64-LE(len)  ||  utf8 bytes of id
    /// ```
    ///
    /// Length prefix prevents `["ab", "c"]` from hashing equal to
    /// `["a", "bc"]`. The `u64` width matches the rest of the workspace
    /// (see `beast-sim::determinism::absorb_str`) so future cross-crate
    /// hashes can chain without divergence.
    pub fn from_primitives<I, S>(primitives: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let sorted: BTreeSet<String> = primitives
            .into_iter()
            .map(|s| s.as_ref().to_owned())
            .collect();
        Self::from_sorted_set(&sorted)
    }

    /// Compute the signature of an already-sorted `BTreeSet<String>`.
    ///
    /// Avoids the temporary collection when the caller already holds a
    /// `BTreeSet` (e.g. `PrimitiveSnapshot::primitives`).
    pub fn from_sorted_set(sorted: &BTreeSet<String>) -> Self {
        let mut hasher = blake3::Hasher::new();
        for id in sorted {
            hasher.update(&(id.len() as u64).to_le_bytes());
            hasher.update(id.as_bytes());
        }
        Self(hasher.finalize().into())
    }

    /// Hex-encoded representation, useful for diagnostic output.
    pub fn to_hex(self) -> String {
        let mut out = String::with_capacity(64);
        for byte in &self.0 {
            // `format!("{:02x}", byte)` allocates per-byte; this avoids
            // that. Determinism doesn't care since it's render / debug
            // only, but the hot path benefits.
            const HEX: &[u8; 16] = b"0123456789abcdef";
            out.push(HEX[(byte >> 4) as usize] as char);
            out.push(HEX[(byte & 0x0f) as usize] as char);
        }
        out
    }
}

/// Accumulated record for one [`PatternSignature`].
///
/// The chronicler updates these on each [`Chronicler::ingest`](crate::Chronicler::ingest):
///
/// * `count` — total number of `(tick, entity)` snapshots that hashed to
///   this signature.
/// * `first_tick` — set on the very first observation, never updated.
/// * `last_tick` — overwritten on every subsequent observation.
/// * `primitives` — the actual set of primitive ids the signature stands
///   for, kept around so label-generation (S10.6) and query-side code
///   (S10.7) can reverse-lookup without a separate table.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatternObservation {
    /// Signature this record is keyed by.
    pub signature: PatternSignature,
    /// Number of snapshot ingestions that produced this signature.
    pub count: u64,
    /// Tick of the first observation.
    pub first_tick: TickCounter,
    /// Tick of the most recent observation.
    pub last_tick: TickCounter,
    /// Concrete primitive ids the signature represents.
    pub primitives: BTreeSet<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_is_order_independent() {
        let a = PatternSignature::from_primitives(["echo", "spatial", "sense"]);
        let b = PatternSignature::from_primitives(["sense", "echo", "spatial"]);
        assert_eq!(a, b);
    }

    #[test]
    fn signature_differs_when_primitive_set_differs() {
        let a = PatternSignature::from_primitives(["echo", "spatial"]);
        let b = PatternSignature::from_primitives(["echo", "spatial", "extra"]);
        assert_ne!(a, b);
    }

    #[test]
    fn signature_dedup_collapses_duplicates() {
        let with_dup = PatternSignature::from_primitives(["a", "b", "a"]);
        let without = PatternSignature::from_primitives(["a", "b"]);
        assert_eq!(with_dup, without);
    }

    #[test]
    fn empty_set_hashes_to_blake3_of_empty_input() {
        let empty = PatternSignature::from_primitives::<_, &str>(std::iter::empty());
        let expected: [u8; 32] = blake3::Hasher::new().finalize().into();
        assert_eq!(empty.0, expected);
    }

    #[test]
    fn length_prefix_prevents_collision() {
        // ["ab", "c"] vs ["a", "bc"] — sorted forms ["ab", "c"] vs
        // ["a", "bc"]. Without a length prefix the byte stream would
        // be "abc" in both cases. With length prefix they diverge.
        let one = PatternSignature::from_primitives(["ab", "c"]);
        let two = PatternSignature::from_primitives(["a", "bc"]);
        assert_ne!(one, two);
    }

    #[test]
    fn hex_round_trip_is_64_chars() {
        let sig = PatternSignature::from_primitives(["echo"]);
        let hex = sig.to_hex();
        assert_eq!(hex.len(), 64);
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
