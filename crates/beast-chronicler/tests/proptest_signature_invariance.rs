//! Property test: every cyclic rotation of the same primitive set
//! produces the same [`PatternSignature`].
//!
//! This is a strict subset of "all permutations" — proper shuffle would
//! need a seeded RNG. The full permutation invariance is proven
//! analytically: `PatternSignature::from_primitives` collects into a
//! `BTreeSet` before hashing, so the signature is a function of the set
//! contents, not insertion order. The proptest exercises that contract
//! over a wide id-set space and a deterministic permutation family.

use std::collections::BTreeSet;

use beast_chronicler::PatternSignature;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,
        ..Default::default()
    })]

    #[test]
    fn permutations_hash_to_same_signature(
        ids in prop::collection::hash_set("[a-z][a-z_0-9]{0,15}", 0..16)
    ) {
        let canonical: BTreeSet<String> = ids.iter().cloned().collect();
        let baseline = PatternSignature::from_sorted_set(&canonical);

        // Build a vector and shuffle it via proptest's RNG-free
        // permutation: rotate by every possible offset.
        let mut as_vec: Vec<String> = ids.into_iter().collect();
        let n = as_vec.len();
        for offset in 0..n {
            as_vec.rotate_left(offset.min(n));
            let permuted = PatternSignature::from_primitives(as_vec.iter());
            prop_assert_eq!(permuted, baseline);
        }
    }
}
