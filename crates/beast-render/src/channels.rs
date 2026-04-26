//! Shared channel-id constants for the visual pipeline.
//!
//! `pipeline.rs` and `animation.rs` both look channels up by string id.
//! Keeping the strings in one place stops one file's rename from
//! silently turning into a `Q3232::ZERO` lookup in the other.

use beast_core::Q3232;
use beast_interpreter::ResolvedPhenotype;

pub(crate) const CH_ELASTIC_DEFORMATION: &str = "elastic_deformation";
pub(crate) const CH_STRUCTURAL_RIGIDITY: &str = "structural_rigidity";
pub(crate) const CH_MASS_DENSITY: &str = "mass_density";
pub(crate) const CH_METABOLIC_RATE: &str = "metabolic_rate";
pub(crate) const CH_SURFACE_FRICTION: &str = "surface_friction";
pub(crate) const CH_KINETIC_FORCE: &str = "kinetic_force";
pub(crate) const CH_LIGHT_EMISSION: &str = "light_emission";
pub(crate) const CH_CHEMICAL_OUTPUT: &str = "chemical_output";
pub(crate) const CH_THERMAL_OUTPUT: &str = "thermal_output";

/// Read a global channel value, defaulting to `Q3232::ZERO` when absent.
pub(crate) fn ch(phenotype: &ResolvedPhenotype, name: &str) -> Q3232 {
    phenotype
        .global_channels
        .get(name)
        .copied()
        .unwrap_or(Q3232::ZERO)
}
