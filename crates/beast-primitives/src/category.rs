//! Functional category and observable modality enums.
//!
//! Mirror the schema's `category` and `observable_signature.modality` enums.
//! Both derive `Ord` so registries can use them as [`BTreeMap`] keys for
//! deterministic iteration.
//!
//! [`BTreeMap`]: std::collections::BTreeMap

use serde::{Deserialize, Serialize};

/// Functional category of a primitive effect.
///
/// Maps 1:1 to the `category` enum in the schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrimitiveCategory {
    /// Broadcasting info to environment (acoustic pulses, markers, etc.).
    SignalEmission,
    /// Passive sensing from environment.
    SignalReception,
    /// Mechanical action on environment.
    ForceApplication,
    /// Physiological state change in self or target.
    StateInduction,
    /// Fusing multi-sensory signals into maps.
    SpatialIntegration,
    /// Moving substances between spaces.
    MassTransfer,
    /// Controlling metabolic rate and energy budget.
    EnergyModulation,
    /// Establishing behavioral/physiological attachments.
    BondFormation,
}

/// Sense modality via which a primitive's emission is observable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Modality {
    /// Sound.
    Acoustic,
    /// Olfaction / chemical signals.
    Chemical,
    /// Light.
    Visual,
    /// Electroreception.
    Electric,
    /// Touch / vibration / pressure.
    Mechanical,
    /// Heat.
    Thermal,
    /// Spatial / morphological arrangement.
    Topological,
    /// Behavioral / action patterns.
    Behavioral,
}
