//! Beast Evolution Game — Layer 1 primitive effect registry.
//!
//! Sprint S2 populates this crate story-by-story. This commit lands the
//! manifest loader (2.2); the registry and cost evaluator (2.4) follow.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod category;
pub mod manifest;
pub mod schema;

pub use category::{Modality, PrimitiveCategory};
pub use manifest::{
    CompatibilityEntry, CostFunction, ObservableSignature, ParameterScaling, ParameterSpec,
    ParameterType, PrimitiveManifest, Provenance,
};
pub use schema::{PrimitiveLoadError, PRIMITIVE_MANIFEST_SCHEMA};
