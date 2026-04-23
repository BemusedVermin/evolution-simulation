//! Beast Evolution Game — shared manifest-loading infrastructure (L1).
//!
//! Both [`beast-channels`] and [`beast-primitives`] load JSON manifests with
//! the same two-stage pipeline (JSON Schema validation, then semantic parse
//! into strong types) and expose the same three supporting primitives:
//!
//! * a flattened [`schema::SchemaViolation`] error shape so downstream callers
//!   don't depend on the `jsonschema` types directly,
//! * a canonical [`provenance::Provenance`] enum matching the schemas'
//!   `core | mod:<id> | genesis:<parent>:<n>` discriminator,
//! * a [`BTreeMap`]-backed [`registry::SortedRegistry`] with a secondary
//!   index by a domain-specific grouping key.
//!
//! Before this crate existed each loader was duplicated verbatim; adding a
//! new manifest kind meant editing two parallel pipelines. Keeping the shared
//! infrastructure here removes the change-amplification tax without folding
//! the domain-specific semantic validation (range-ordering, compatibility
//! checks, etc.) into a common place — those still live in the consumer
//! crates.
//!
//! [`beast-channels`]: https://docs.rs/beast-channels
//! [`beast-primitives`]: https://docs.rs/beast-primitives
//! [`BTreeMap`]: std::collections::BTreeMap

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod provenance;
pub mod registry;
pub mod schema;

pub use provenance::{Provenance, ProvenanceParseError};
pub use registry::{DuplicateId, Manifest, SortedRegistry};
pub use schema::{format_schema_errors, CompiledSchema, SchemaLoadError, SchemaViolation};
