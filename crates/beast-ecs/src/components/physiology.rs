//! Physiological components: [`Age`], [`Mass`], [`HealthState`],
//! [`DevelopmentalStage`], [`Species`].

use beast_core::Q3232;
use serde::{Deserialize, Serialize};
use specs::{Component, DenseVecStorage};

/// Creature age, measured in simulation ticks since birth. A `u64`
/// because a long-lived creature plus a large tick budget could exceed
/// `u32::MAX` within a single replay.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Age {
    /// Ticks since birth.
    pub ticks: u64,
}

impl Age {
    /// Convenience constructor.
    #[must_use]
    pub fn new(ticks: u64) -> Self {
        Self { ticks }
    }
}

impl Component for Age {
    type Storage = DenseVecStorage<Self>;
}

/// Body mass in kilograms. The scale-band invariant (INVARIANTS §5)
/// uses this value to decide channel expressibility; keep the field
/// strictly positive and in Q32.32.
///
/// `Default` is intentionally **not** derived — a zero-mass creature
/// would break scale-band filtering (every band starts at `≥ 0`, so
/// zero is always "in band" vacuously) and would also divide-by-zero
/// in any system that normalises by mass. Callers must supply an
/// explicit value via `Mass::new`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mass {
    /// Kilograms, Q32.32.
    pub kg: Q3232,
}

impl Mass {
    /// Convenience constructor. Does not validate positivity — mass
    /// values can be arbitrarily small in the micro/pathogen band
    /// (`1e-15 kg`). A `debug_assert!(kg > Q3232::ZERO)` would be
    /// tempting but would forbid the intentionally-zero test fixtures
    /// the interpreter uses for "dormant channel" checks; leave the
    /// contract in prose.
    #[must_use]
    pub fn new(kg: Q3232) -> Self {
        Self { kg }
    }
}

impl Component for Mass {
    type Storage = DenseVecStorage<Self>;
}

/// Discrete health state: two unit-interval Q32.32 values.
///
/// * `health` — structural integrity; zero triggers the death check.
/// * `energy` — metabolic reserve; low values feed starvation logic.
///
/// Both values are clamped to `[0, 1]` by convention; systems that
/// update them use saturating Q3232 arithmetic.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthState {
    /// Structural integrity, `[0, 1]`.
    pub health: Q3232,
    /// Metabolic reserve, `[0, 1]`.
    pub energy: Q3232,
}

impl HealthState {
    /// Fully healthy, fully fed.
    #[must_use]
    pub fn full() -> Self {
        Self {
            health: Q3232::ONE,
            energy: Q3232::ONE,
        }
    }

    /// Convenience constructor with explicit values.
    #[must_use]
    pub fn new(health: Q3232, energy: Q3232) -> Self {
        Self { health, energy }
    }
}

impl Component for HealthState {
    type Storage = DenseVecStorage<Self>;
}

/// Developmental stage a creature currently occupies.
///
/// The enum order reflects normal progression; ordering comparisons are
/// meaningful (`Egg < Larval < Juvenile < Adult < Geriatric`) so
/// systems can `if stage < DevelopmentalStage::Juvenile { ... }`.
///
/// Maps onto `beast_channels::ExpressionCondition::DevelopmentalStage`
/// strings via `as_str` — the interpreter consumes that form.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum DevelopmentalStage {
    /// Pre-hatch / pre-birth.
    Egg,
    /// Post-hatch juvenile; some channels still dormant.
    Larval,
    /// Grown juvenile, not yet reproductively active.
    Juvenile,
    /// Reproductively active adult. Default.
    #[default]
    Adult,
    /// Declining: senescence modifiers start to dominate.
    Geriatric,
}

impl DevelopmentalStage {
    /// String form expected by
    /// `beast_channels::ExpressionCondition::DevelopmentalStage`. Stable
    /// across versions — a rename would break every channel manifest.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            DevelopmentalStage::Egg => "egg",
            DevelopmentalStage::Larval => "larval",
            DevelopmentalStage::Juvenile => "juvenile",
            DevelopmentalStage::Adult => "adult",
            DevelopmentalStage::Geriatric => "geriatric",
        }
    }
}

impl Component for DevelopmentalStage {
    type Storage = DenseVecStorage<Self>;
}

/// Species membership. The `id` is assigned by the speciation system
/// (S12); two creatures with the same `id` are reproductively
/// compatible.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct Species {
    /// Species id. `0` is the default "unassigned" bucket.
    pub id: u32,
}

impl Species {
    /// Convenience constructor.
    #[must_use]
    pub fn new(id: u32) -> Self {
        Self { id }
    }
}

impl Component for Species {
    type Storage = DenseVecStorage<Self>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn developmental_stage_ordering_is_birth_to_death() {
        use DevelopmentalStage::*;
        assert!(Egg < Larval);
        assert!(Larval < Juvenile);
        assert!(Juvenile < Adult);
        assert!(Adult < Geriatric);
    }

    #[test]
    fn developmental_stage_str_is_stable() {
        // Locked in as the canonical form for channel manifest matching.
        assert_eq!(DevelopmentalStage::Egg.as_str(), "egg");
        assert_eq!(DevelopmentalStage::Larval.as_str(), "larval");
        assert_eq!(DevelopmentalStage::Juvenile.as_str(), "juvenile");
        assert_eq!(DevelopmentalStage::Adult.as_str(), "adult");
        assert_eq!(DevelopmentalStage::Geriatric.as_str(), "geriatric");
    }

    #[test]
    fn health_full_is_one_by_one() {
        let h = HealthState::full();
        assert_eq!(h.health, Q3232::ONE);
        assert_eq!(h.energy, Q3232::ONE);
    }

    #[test]
    fn species_default_is_zero() {
        assert_eq!(Species::default(), Species::new(0));
    }
}
