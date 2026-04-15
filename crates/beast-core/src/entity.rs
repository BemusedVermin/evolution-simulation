//! Entity identifiers. Populated in Story 1.3.

use serde::{Deserialize, Serialize};

/// Opaque entity identifier used across the ECS. Implementation lands in
/// Story 1.3 — this stub exists so the module tree compiles during S1.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct EntityId(pub u32);
