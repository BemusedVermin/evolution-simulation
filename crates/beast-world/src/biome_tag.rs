//! [`BiomeTag`] — terrain classification produced by world generation.
//!
//! Mirrors `beast_ecs::components::BiomeKind` (S8.2). Defined locally
//! so this crate (L3) doesn't depend up on `beast-ecs` (also L3 but
//! conceptually one step further along the pipeline). The string
//! returned by [`BiomeTag::as_str`] is the contract between the two
//! types — the spawner (S8.4) bridges them via that string.
//!
//! A future refactor (tracked in the S8 epic) can collapse the two
//! types once the layer DAG allows it without churning every story
//! PR.

use serde::{Deserialize, Serialize};

/// Six-way terrain classification matching `BiomeKind`.
///
/// Variants and their string forms are locked to `BiomeKind::as_str()`
/// values; a regression test in `tests/biome_tag_string_lock.rs`
/// reads the BiomeKind source and asserts equivalence whenever
/// beast-ecs is in scope (the test is skipped when beast-ecs is not
/// a dev-dep, so this crate stays standalone-buildable).
///
/// # Ordering contract
///
/// `PartialOrd + Ord` are derived from declaration order. New
/// variants **must be appended at the end** to preserve the sort
/// order of any `BTreeMap<BiomeTag, _>` consumers. Combined with
/// `#[non_exhaustive]`, this gives consumers append-only growth
/// without breaking previously-built `BTreeMap` ordering. The
/// `ord_is_declaration_order` test locks the *current* order; the
/// append-only rule is what protects it from being shuffled.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum BiomeTag {
    /// Open salt-water sea — the default for un-generated cells and
    /// for cells with elevation below `sea_level`.
    #[default]
    Ocean,
    /// Wooded continent interior, mid-temperature, mid-precipitation.
    Forest,
    /// Open grassland, mid-temperature, low precipitation.
    Plains,
    /// Hot, dry, low carrying capacity.
    Desert,
    /// High elevation, cold, sparse cover.
    Mountain,
    /// Polar / sub-polar, cold, mid precipitation as snow.
    Tundra,
}

impl BiomeTag {
    /// Lower-snake-case string id. Stable across versions — a rename
    /// would break every shipping channel manifest that filters by
    /// terrain.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            BiomeTag::Ocean => "ocean",
            BiomeTag::Forest => "forest",
            BiomeTag::Plains => "plains",
            BiomeTag::Desert => "desert",
            BiomeTag::Mountain => "mountain",
            BiomeTag::Tundra => "tundra",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_ocean() {
        assert_eq!(BiomeTag::default(), BiomeTag::Ocean);
    }

    #[test]
    fn as_str_matches_serde_for_every_variant() {
        // A new variant gets its serde string for free via
        // `rename_all`. Lock per-variant equivalence so a missing
        // `as_str` arm fails this test on first add.
        for tag in [
            BiomeTag::Ocean,
            BiomeTag::Forest,
            BiomeTag::Plains,
            BiomeTag::Desert,
            BiomeTag::Mountain,
            BiomeTag::Tundra,
        ] {
            let s = serde_json::to_string(&tag).unwrap();
            let trimmed = s.trim_matches('"');
            assert_eq!(
                tag.as_str(),
                trimmed,
                "as_str / serde divergence for {tag:?}"
            );
        }
    }

    #[test]
    fn ord_is_declaration_order() {
        // Locks the iteration order in case any consumer keys a
        // BTreeMap by BiomeTag.
        use BiomeTag::*;
        assert!(Ocean < Forest);
        assert!(Forest < Plains);
        assert!(Plains < Desert);
        assert!(Desert < Mountain);
        assert!(Mountain < Tundra);
    }
}
