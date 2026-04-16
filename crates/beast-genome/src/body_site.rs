//! Body-site vector — where on the organism a trait gene manifests.
//!
//! Every [`crate::TraitGene`] carries a [`BodyVector`] describing the
//! anatomical location and coverage of the trait's physical expression.
//! The interpreter (System 11) maps these continuous coordinates onto
//! procedural body topology; this module only owns the numeric storage.
//!
//! All fields are in `[0, 1]` and validated at construction.

use beast_core::Q3232;
use serde::{Deserialize, Serialize};

use crate::error::{check_unit, Result};

/// Anatomical location and coverage of a trait gene's physical expression.
///
/// See System 01 §3, Layer 1 ("WHERE on the body it manifests").
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BodyVector {
    /// 0 = deep organ, 1 = surface protrusion.
    pub surface_vs_internal: Q3232,
    /// Continuous anterior/posterior/lateral coordinate in `[0, 1]`;
    /// mapped to procedural body topology by the interpreter.
    pub body_region: Q3232,
    /// Whether the trait is mirrored on the opposite lateral side.
    pub bilateral_symmetry: bool,
    /// Fraction of the body region affected, in `[0, 1]`.
    pub coverage: Q3232,
}

impl BodyVector {
    /// Construct a body vector, validating all `Q3232` fields are in `[0, 1]`.
    pub fn new(
        surface_vs_internal: Q3232,
        body_region: Q3232,
        bilateral_symmetry: bool,
        coverage: Q3232,
    ) -> Result<Self> {
        check_unit("surface_vs_internal", surface_vs_internal)?;
        check_unit("body_region", body_region)?;
        check_unit("coverage", coverage)?;
        Ok(Self {
            surface_vs_internal,
            body_region,
            bilateral_symmetry,
            coverage,
        })
    }

    /// Re-validate all `Q3232` fields against `[0, 1]`. Called by
    /// [`crate::TraitGene::validate_local`] to catch post-mutation drift.
    pub fn validate(&self) -> Result<()> {
        check_unit("surface_vs_internal", self.surface_vs_internal)?;
        check_unit("body_region", self.body_region)?;
        check_unit("coverage", self.coverage)?;
        Ok(())
    }

    /// A default body vector: internal (0), anterior (0), non-mirrored, zero coverage.
    #[inline]
    #[must_use]
    pub const fn default_internal() -> Self {
        Self {
            surface_vs_internal: Q3232::ZERO,
            body_region: Q3232::ZERO,
            bilateral_symmetry: false,
            coverage: Q3232::ZERO,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::GenomeError;

    #[test]
    fn accepts_valid_body_vector() {
        let bv = BodyVector::new(
            Q3232::from_num(0.5_f64),
            Q3232::from_num(0.3_f64),
            true,
            Q3232::from_num(0.8_f64),
        )
        .unwrap();
        assert!(bv.bilateral_symmetry);
    }

    #[test]
    fn accepts_boundary_values() {
        BodyVector::new(Q3232::ZERO, Q3232::ZERO, false, Q3232::ZERO).unwrap();
        BodyVector::new(Q3232::ONE, Q3232::ONE, true, Q3232::ONE).unwrap();
    }

    #[test]
    fn rejects_out_of_range_surface() {
        let err =
            BodyVector::new(Q3232::from_num(1.5_f64), Q3232::ZERO, false, Q3232::ZERO).unwrap_err();
        assert!(matches!(
            err,
            GenomeError::OutOfUnitRange {
                field: "surface_vs_internal",
                ..
            }
        ));
    }

    #[test]
    fn rejects_negative_body_region() {
        let err = BodyVector::new(Q3232::ZERO, -Q3232::from_num(0.1_f64), false, Q3232::ZERO)
            .unwrap_err();
        assert!(matches!(
            err,
            GenomeError::OutOfUnitRange {
                field: "body_region",
                ..
            }
        ));
    }

    #[test]
    fn rejects_out_of_range_coverage() {
        let err =
            BodyVector::new(Q3232::ZERO, Q3232::ZERO, false, Q3232::from_num(2_i32)).unwrap_err();
        assert!(matches!(
            err,
            GenomeError::OutOfUnitRange {
                field: "coverage",
                ..
            }
        ));
    }

    #[test]
    fn serde_roundtrip() {
        let bv = BodyVector::new(
            Q3232::from_num(0.25_f64),
            Q3232::from_num(0.75_f64),
            true,
            Q3232::from_num(0.5_f64),
        )
        .unwrap();
        let json = serde_json::to_string(&bv).unwrap();
        let back: BodyVector = serde_json::from_str(&json).unwrap();
        assert_eq!(bv, back);
    }

    #[test]
    fn default_internal_is_valid() {
        let bv = BodyVector::default_internal();
        assert_eq!(bv.surface_vs_internal, Q3232::ZERO);
        assert_eq!(bv.coverage, Q3232::ZERO);
        assert!(!bv.bilateral_symmetry);
    }
}
