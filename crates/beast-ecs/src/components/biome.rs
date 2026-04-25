//! Biome geography components — [`BiomeCell`] and [`BiomeKind`].
//!
//! Pairs with the existing [`crate::components::Biome`] marker
//! (`NullStorage`, presence-only). When an entity carries the `Biome`
//! marker, it should also carry a `BiomeCell` describing the patch's
//! terrain, base climate, and ecological budget.
//!
//! `BiomeCell.temperature_kelvin` and `BiomeCell.precipitation_mm_per_year`
//! are the **base** values for the cell. The climate system (S8.5)
//! reads them and applies a season-aware seasonal delta when reading,
//! never mutating the underlying field — this keeps the closed-cycle
//! invariant exact (after 1000 ticks the visible value matches the
//! base, with no accumulated rounding error).
//!
//! # Determinism
//!
//! All numeric fields are Q32.32 fixed-point integers. `BiomeKind` is
//! `Ord` and serialises as snake_case so two equal cells round-trip
//! to byte-identical JSON.

use beast_core::Q3232;
use serde::{Deserialize, Serialize};
use specs::{Component, DenseVecStorage};

/// Coarse terrain classification for a biome cell. The MVP keeps the
/// taxonomy small; future expansion (rainforest, taiga, savanna…)
/// happens behind the `#[non_exhaustive]` attribute so existing match
/// sites won't break.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum BiomeKind {
    /// Open salt-water sea — the default for un-generated cells.
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

impl BiomeKind {
    /// String form expected by future channel manifests that filter
    /// by terrain (e.g., "expression_conditions: { terrain: forest }").
    /// Stable across versions — a rename would break every shipping
    /// manifest that references it.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            BiomeKind::Ocean => "ocean",
            BiomeKind::Forest => "forest",
            BiomeKind::Plains => "plains",
            BiomeKind::Desert => "desert",
            BiomeKind::Mountain => "mountain",
            BiomeKind::Tundra => "tundra",
        }
    }
}

/// Per-cell biome state. Stored on every entity carrying the
/// [`crate::components::Biome`] marker; the world-gen pass (S8.1)
/// allocates one entity per grid cell.
///
/// `Default` is intentionally not derived — every cell needs an
/// explicit `BiomeKind` choice; the all-zeros default would be silent
/// "Ocean with 0K, 0mm rainfall, 0 carrying capacity" which is rarely
/// the intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BiomeCell {
    /// Terrain classification.
    pub kind: BiomeKind,
    /// Base annual mean temperature in Kelvin, Q32.32. The climate
    /// system overlays a seasonal delta; this field is the closed
    /// reference point.
    pub temperature_kelvin: Q3232,
    /// Base annual precipitation in millimetres per year, Q32.32.
    pub precipitation_mm_per_year: Q3232,
    /// Maximum number of creatures the cell sustains. Population
    /// dynamics (S15+) consult this to throttle reproduction.
    pub carrying_capacity: u32,
}

impl BiomeCell {
    /// Convenience constructor.
    #[must_use]
    pub fn new(
        kind: BiomeKind,
        temperature_kelvin: Q3232,
        precipitation_mm_per_year: Q3232,
        carrying_capacity: u32,
    ) -> Self {
        Self {
            kind,
            temperature_kelvin,
            precipitation_mm_per_year,
            carrying_capacity,
        }
    }

    /// Conventional ocean cell with the spec's default 288.15K (15°C)
    /// surface temperature and 1000mm precipitation. Used by the
    /// world-gen default when a coordinate hasn't been classified yet.
    #[must_use]
    pub fn ocean() -> Self {
        Self::new(
            BiomeKind::Ocean,
            Q3232::from_num(288),
            Q3232::from_num(1000),
            0,
        )
    }
}

impl Component for BiomeCell {
    type Storage = DenseVecStorage<Self>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn biome_kind_default_is_ocean() {
        assert_eq!(BiomeKind::default(), BiomeKind::Ocean);
    }

    #[test]
    fn biome_kind_as_str_is_stable() {
        // Locked in: a rename of these strings would break every
        // shipped channel manifest that expression-filters on terrain.
        assert_eq!(BiomeKind::Ocean.as_str(), "ocean");
        assert_eq!(BiomeKind::Forest.as_str(), "forest");
        assert_eq!(BiomeKind::Plains.as_str(), "plains");
        assert_eq!(BiomeKind::Desert.as_str(), "desert");
        assert_eq!(BiomeKind::Mountain.as_str(), "mountain");
        assert_eq!(BiomeKind::Tundra.as_str(), "tundra");
    }

    #[test]
    fn biome_kind_serialises_as_snake_case_in_json() {
        let s = serde_json::to_string(&BiomeKind::Forest).unwrap();
        assert_eq!(s, "\"forest\"");
    }

    #[test]
    fn biome_cell_ocean_preset_round_trips_through_serde() {
        let original = BiomeCell::ocean();
        let s = serde_json::to_string(&original).unwrap();
        let parsed: BiomeCell = serde_json::from_str(&s).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn biome_cell_constructor_carries_inputs() {
        let cell = BiomeCell::new(
            BiomeKind::Forest,
            Q3232::from_num(283),
            Q3232::from_num(1500),
            64,
        );
        assert_eq!(cell.kind, BiomeKind::Forest);
        assert_eq!(cell.temperature_kelvin, Q3232::from_num(283));
        assert_eq!(cell.precipitation_mm_per_year, Q3232::from_num(1500));
        assert_eq!(cell.carrying_capacity, 64);
    }

    #[test]
    fn biome_cell_storage_is_densevec() {
        // Locks in the storage choice — switching to VecStorage would
        // waste memory for a sparse biome map.
        fn is_dense<C>()
        where
            C: specs::Component<Storage = specs::DenseVecStorage<C>>,
        {
        }
        is_dense::<BiomeCell>();
    }

    #[test]
    fn biome_kind_ord_is_declaration_order() {
        // BiomeKind backs deterministic iteration in places that key by
        // it (e.g., a future BTreeMap<BiomeKind, _>). Lock the order.
        use BiomeKind::*;
        assert!(Ocean < Forest);
        assert!(Forest < Plains);
        assert!(Plains < Desert);
        assert!(Desert < Mountain);
        assert!(Mountain < Tundra);
    }
}
