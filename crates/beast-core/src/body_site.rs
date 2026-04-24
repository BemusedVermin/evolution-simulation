//! Canonical body-site taxonomy.
//!
//! A small foundational enum used across layers: `beast_primitives::PrimitiveEffect`
//! tags emissions with the site they apply to, and `beast_interpreter` uses the
//! same variants when building per-region phenotype maps. Living in L0 keeps
//! the type free of both channel and primitive machinery.
//!
//! Ordinal order is the deterministic iteration order used by per-site
//! emission. Do **not** reorder variants without updating fixture tests.

/// Canonical body-site enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BodySite {
    /// The creature as a whole (global emissions).
    Global,
    /// Head.
    Head,
    /// Jaw / mouth.
    Jaw,
    /// Body core / torso.
    Core,
    /// Left limb.
    LimbLeft,
    /// Right limb.
    LimbRight,
    /// Tail.
    Tail,
    /// Generic appendage (antenna, tentacle, etc.).
    Appendage,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pin the declared enum order. Per-site aggregation iterates variants
    /// in this order (see `beast-interpreter::body_map`), so variant
    /// reordering would silently change output-hash byte order and break
    /// the determinism gate. This test guards the invariant at the type's
    /// definition site.
    #[test]
    fn variant_order_is_stable() {
        let ordered = [
            BodySite::Global,
            BodySite::Head,
            BodySite::Jaw,
            BodySite::Core,
            BodySite::LimbLeft,
            BodySite::LimbRight,
            BodySite::Tail,
            BodySite::Appendage,
        ];
        for pair in ordered.windows(2) {
            assert!(
                pair[0] < pair[1],
                "{:?} must sort before {:?}",
                pair[0],
                pair[1]
            );
        }
    }
}
