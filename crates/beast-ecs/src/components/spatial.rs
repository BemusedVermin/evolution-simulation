//! Spatial components: [`Position`] and [`Velocity`] on a 2-D plane,
//! both in Q32.32 fixed-point.
//!
//! The world's coordinate system is deliberately deterministic — no
//! `f32`/`f64` anywhere in the hot path. See INVARIANTS §1.

use beast_core::Q3232;
use serde::{Deserialize, Serialize};
use specs::{Component, DenseVecStorage};

/// Position on the 2-D world plane, measured in metres from the
/// world-coordinate origin. Both axes are signed Q32.32.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    /// X coordinate (east-positive).
    pub x: Q3232,
    /// Y coordinate (north-positive).
    pub y: Q3232,
}

impl Position {
    /// Convenience constructor.
    #[must_use]
    pub fn new(x: Q3232, y: Q3232) -> Self {
        Self { x, y }
    }
}

impl Component for Position {
    type Storage = DenseVecStorage<Self>;
}

/// Velocity in metres-per-tick, Q32.32.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Velocity {
    /// Eastward velocity component.
    pub vx: Q3232,
    /// Northward velocity component.
    pub vy: Q3232,
}

impl Velocity {
    /// Convenience constructor.
    #[must_use]
    pub fn new(vx: Q3232, vy: Q3232) -> Self {
        Self { vx, vy }
    }
}

impl Component for Velocity {
    type Storage = DenseVecStorage<Self>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn position_default_is_origin() {
        let p = Position::default();
        assert_eq!(p, Position::new(Q3232::ZERO, Q3232::ZERO));
    }

    #[test]
    fn velocity_default_is_rest() {
        let v = Velocity::default();
        assert_eq!(v, Velocity::new(Q3232::ZERO, Q3232::ZERO));
    }
}
