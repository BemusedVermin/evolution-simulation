//! Manifest-driven label assignment (S10.6).
//!
//! Per `documentation/INVARIANTS.md` §2 (Mechanics-Label Separation), no
//! human-readable ability name appears in simulation control flow.
//! Labels live entirely in this module: the [`LabelEngine`] reads a
//! [`LabelManifest`] (loaded from JSON), exact-matches the manifest
//! entries against ingested [`PatternObservation`]s, and emits a
//! [`Label`] when the run-time confidence (see [`crate::confidence`])
//! crosses the entry's `min_confidence` floor.
//!
//! Per `documentation/systems/09_world_history_lore.md` §3.8, label
//! catalogs are *manifest-only* — no fallback heuristic is allowed in
//! Rust code. Adding a new label = shipping a new manifest entry.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use beast_core::{TickCounter, Q3232};
use beast_manifest::{format_schema_errors, CompiledSchema, SchemaLoadError, SchemaViolation};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::confidence::compute_confidence;
use crate::pattern::{PatternObservation, PatternSignature};

/// Raw JSON Schema source for the label manifest. Embedded at compile
/// time so callers need no runtime filesystem access.
pub const LABEL_MANIFEST_SCHEMA: &str =
    include_str!("../../../documentation/schemas/label_manifest.schema.json");

/// Errors produced while loading a label manifest.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum LabelLoadError {
    /// Input was not parsable as JSON.
    #[error("label manifest is not valid JSON: {0}")]
    InvalidJson(String),

    /// JSON Schema validation produced one or more errors.
    #[error("label manifest failed schema validation:\n{}", format_schema_errors(.0))]
    SchemaViolation(Vec<SchemaViolation>),

    /// Schema accepted the document but typed deserialization failed —
    /// indicates a drift between schema and Rust types (treat as a bug).
    #[error("label manifest deserialized unexpectedly: {0}")]
    BadShape(String),

    /// Two manifest entries shared the same `id`.
    #[error("duplicate label id `{0}` in manifest")]
    DuplicateId(String),
}

impl From<SchemaLoadError> for LabelLoadError {
    fn from(e: SchemaLoadError) -> Self {
        match e {
            SchemaLoadError::InvalidJson(s) => Self::InvalidJson(s),
            SchemaLoadError::SchemaViolation(v) => Self::SchemaViolation(v),
        }
    }
}

fn compiled_schema() -> &'static CompiledSchema {
    static SCHEMA: OnceLock<CompiledSchema> = OnceLock::new();
    SCHEMA.get_or_init(|| CompiledSchema::compile(LABEL_MANIFEST_SCHEMA))
}

/// One entry in a label manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelManifestEntry {
    /// Unique snake_case label id.
    pub id: String,
    /// Set of primitive ids the manifest entry pattern-matches against.
    /// Sorted by `BTreeSet` so equality with an observation's
    /// `primitives` is a single set comparison.
    pub primitives: BTreeSet<String>,
    /// Confidence floor on `[0, 1]` below which the label is suppressed.
    pub min_confidence: Q3232,
}

/// Validated, in-memory label manifest.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LabelManifest {
    entries: Vec<LabelManifestEntry>,
}

impl LabelManifest {
    /// Load and validate a label manifest from a JSON source string.
    ///
    /// Runs JSON Schema validation followed by typed deserialization
    /// and a duplicate-id check. The returned manifest preserves the
    /// declaration order of entries.
    pub fn from_json_str(source: &str) -> Result<Self, LabelLoadError> {
        let value = compiled_schema().load(source)?;
        let raw: RawLabelManifest =
            serde_json::from_value(value).map_err(|e| LabelLoadError::BadShape(e.to_string()))?;

        let mut seen = BTreeSet::new();
        let mut entries = Vec::with_capacity(raw.labels.len());
        for raw_entry in raw.labels {
            if !seen.insert(raw_entry.id.clone()) {
                return Err(LabelLoadError::DuplicateId(raw_entry.id));
            }
            entries.push(LabelManifestEntry {
                id: raw_entry.id,
                primitives: raw_entry.primitives.into_iter().collect(),
                min_confidence: Q3232::from_num(raw_entry.min_confidence),
            });
        }
        Ok(Self { entries })
    }

    /// Read-only access to the manifest entries, in declaration order.
    pub fn entries(&self) -> &[LabelManifestEntry] {
        &self.entries
    }

    /// Number of entries in the manifest.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// `true` if the manifest contains no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Raw serde-facing mirror of the manifest schema. Private — callers
/// consume the typed [`LabelManifest`].
#[derive(Debug, Deserialize)]
struct RawLabelManifest {
    labels: Vec<RawLabelEntry>,
}

#[derive(Debug, Deserialize)]
struct RawLabelEntry {
    id: String,
    primitives: Vec<String>,
    min_confidence: f64,
}

/// One assigned label.
///
/// Stored in [`crate::Chronicler::labels`]; surfaced to the UI via the
/// query API in S10.7. The `signature` field lets the UI cross-reference
/// the underlying [`PatternObservation`] without going through the
/// observation index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Label {
    /// Manifest-defined label id (snake_case).
    pub id: String,
    /// Pattern signature this label was assigned to.
    pub signature: PatternSignature,
    /// Run-time confidence on `[0, 1]`, computed by
    /// [`crate::confidence::compute_confidence`].
    pub confidence: Q3232,
}

/// Manifest-driven label assignment.
///
/// Holds a [`LabelManifest`] keyed by primitive set so a single
/// observation lookup is `O(log n)`.
#[derive(Debug, Clone, Default)]
pub struct LabelEngine {
    /// Map from a sorted primitive set to its manifest entry. Two
    /// entries with the same primitive set would shadow each other —
    /// rejected at construction.
    by_primitives: BTreeMap<BTreeSet<String>, LabelManifestEntry>,
}

/// Errors produced while constructing a [`LabelEngine`].
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum LabelEngineError {
    /// Two manifest entries declared the same primitive set, which would
    /// make label assignment ambiguous.
    #[error(
        "label entries `{first}` and `{second}` share the same primitive set; \
         exact-match assignment would be ambiguous"
    )]
    AmbiguousPrimitiveSet {
        /// First entry id encountered.
        first: String,
        /// Conflicting entry id.
        second: String,
    },
}

impl LabelEngine {
    /// Construct an engine from a validated manifest.
    ///
    /// Returns an error if two entries declare the same set of
    /// primitives — the exact-match assignment surface (S10.6) cannot
    /// disambiguate them. The richer rank-based pipeline lands later
    /// (System 09 §17.3).
    pub fn from_manifest(manifest: LabelManifest) -> Result<Self, LabelEngineError> {
        let mut by_primitives: BTreeMap<BTreeSet<String>, LabelManifestEntry> = BTreeMap::new();
        for entry in manifest.entries {
            if let Some(existing) = by_primitives.get(&entry.primitives) {
                return Err(LabelEngineError::AmbiguousPrimitiveSet {
                    first: existing.id.clone(),
                    second: entry.id,
                });
            }
            by_primitives.insert(entry.primitives.clone(), entry);
        }
        Ok(Self { by_primitives })
    }

    /// Convenience: parse a manifest from JSON and build the engine in
    /// one call.
    pub fn from_json_str(source: &str) -> Result<Self, LabelLoadError> {
        let manifest = LabelManifest::from_json_str(source)?;
        Self::from_manifest(manifest).map_err(|e| LabelLoadError::BadShape(e.to_string()))
    }

    /// Number of label entries the engine can match.
    pub fn len(&self) -> usize {
        self.by_primitives.len()
    }

    /// `true` if the engine has no entries.
    pub fn is_empty(&self) -> bool {
        self.by_primitives.is_empty()
    }

    /// Look up a manifest entry by primitive set (used by tests; the
    /// production hot path goes through [`Self::assign`]).
    pub fn entry_for_primitives(
        &self,
        primitives: &BTreeSet<String>,
    ) -> Option<&LabelManifestEntry> {
        self.by_primitives.get(primitives)
    }

    /// Try to assign a label to one observation.
    ///
    /// Returns `Some(Label)` iff:
    ///
    /// 1. The observation's primitive set exactly matches a manifest
    ///    entry's `primitives`.
    /// 2. The run-time confidence (see [`crate::confidence`]) is at
    ///    least the entry's `min_confidence`.
    ///
    /// `total_observations` and `current_tick` parameterise the
    /// confidence formula; both should come from the chronicler that
    /// owns the observation (`Chronicler::total_ingested` and
    /// `Chronicler::last_observed_tick` are the standard sources).
    pub fn assign(
        &self,
        observation: &PatternObservation,
        total_observations: u64,
        current_tick: TickCounter,
    ) -> Option<Label> {
        let entry = self.by_primitives.get(&observation.primitives)?;
        let confidence = compute_confidence(observation, total_observations, current_tick);
        if confidence < entry.min_confidence {
            return None;
        }
        Some(Label {
            id: entry.id.clone(),
            signature: observation.signature,
            confidence,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obs_with(primitives: &[&str], count: u64, first: u64, last: u64) -> PatternObservation {
        let primitives: BTreeSet<String> = primitives.iter().map(|s| (*s).to_owned()).collect();
        let signature = PatternSignature::from_sorted_set(&primitives);
        PatternObservation {
            signature,
            count,
            first_tick: TickCounter::new(first),
            last_tick: TickCounter::new(last),
            primitives,
        }
    }

    #[test]
    fn rejects_invalid_json() {
        let err = LabelManifest::from_json_str("not json").unwrap_err();
        assert!(matches!(err, LabelLoadError::InvalidJson(_)));
    }

    #[test]
    fn rejects_missing_required_field() {
        // `min_confidence` omitted.
        let src = r#"{
            "labels": [
                { "id": "foo", "primitives": ["a", "b"] }
            ]
        }"#;
        let err = LabelManifest::from_json_str(src).unwrap_err();
        assert!(matches!(err, LabelLoadError::SchemaViolation(_)));
    }

    #[test]
    fn rejects_uppercase_id() {
        let src = r#"{
            "labels": [
                { "id": "Foo", "primitives": ["a"], "min_confidence": 0.5 }
            ]
        }"#;
        let err = LabelManifest::from_json_str(src).unwrap_err();
        assert!(matches!(err, LabelLoadError::SchemaViolation(_)));
    }

    #[test]
    fn rejects_min_confidence_above_one() {
        let src = r#"{
            "labels": [
                { "id": "foo", "primitives": ["a"], "min_confidence": 1.5 }
            ]
        }"#;
        let err = LabelManifest::from_json_str(src).unwrap_err();
        assert!(matches!(err, LabelLoadError::SchemaViolation(_)));
    }

    #[test]
    fn rejects_empty_primitives() {
        let src = r#"{
            "labels": [
                { "id": "foo", "primitives": [], "min_confidence": 0.5 }
            ]
        }"#;
        let err = LabelManifest::from_json_str(src).unwrap_err();
        assert!(matches!(err, LabelLoadError::SchemaViolation(_)));
    }

    #[test]
    fn rejects_duplicate_label_id() {
        let src = r#"{
            "labels": [
                { "id": "foo", "primitives": ["a"], "min_confidence": 0.1 },
                { "id": "foo", "primitives": ["b"], "min_confidence": 0.1 }
            ]
        }"#;
        let err = LabelManifest::from_json_str(src).unwrap_err();
        assert!(matches!(err, LabelLoadError::DuplicateId(_)));
    }

    #[test]
    fn loads_minimal_valid_manifest() {
        let src = r#"{
            "labels": [
                { "id": "foo", "primitives": ["a", "b"], "min_confidence": 0.5 }
            ]
        }"#;
        let manifest = LabelManifest::from_json_str(src).unwrap();
        assert_eq!(manifest.len(), 1);
        assert_eq!(manifest.entries()[0].id, "foo");
        assert_eq!(manifest.entries()[0].primitives.len(), 2);
    }

    #[test]
    fn engine_rejects_ambiguous_primitive_sets() {
        let src = r#"{
            "labels": [
                { "id": "foo", "primitives": ["a", "b"], "min_confidence": 0.5 },
                { "id": "bar", "primitives": ["b", "a"], "min_confidence": 0.7 }
            ]
        }"#;
        let err = LabelEngine::from_json_str(src).unwrap_err();
        assert!(matches!(err, LabelLoadError::BadShape(_)));
    }

    #[test]
    fn assign_returns_none_when_below_min_confidence() {
        let src = r#"{
            "labels": [
                { "id": "foo", "primitives": ["a", "b"], "min_confidence": 0.9 }
            ]
        }"#;
        let engine = LabelEngine::from_json_str(src).unwrap();
        // freq = 1/100 = 0.01, stab = 0/100 = 0.0 → conf = 0.006 << 0.9.
        let observation = obs_with(&["a", "b"], 1, 0, 0);
        assert!(engine
            .assign(&observation, 100, TickCounter::new(100))
            .is_none());
    }

    #[test]
    fn assign_returns_label_when_threshold_met() {
        let src = r#"{
            "labels": [
                { "id": "foo", "primitives": ["a", "b"], "min_confidence": 0.5 }
            ]
        }"#;
        let engine = LabelEngine::from_json_str(src).unwrap();
        // freq = 100/100 = 1.0, stab = 100/100 = 1.0 → conf = 1.0 ≥ 0.5.
        let observation = obs_with(&["a", "b"], 100, 0, 100);
        let label = engine
            .assign(&observation, 100, TickCounter::new(100))
            .expect("label should be assigned");
        assert_eq!(label.id, "foo");
        assert_eq!(label.signature, observation.signature);
        assert_eq!(label.confidence, Q3232::ONE);
    }

    #[test]
    fn assign_returns_none_when_primitives_do_not_match() {
        let src = r#"{
            "labels": [
                { "id": "foo", "primitives": ["a", "b"], "min_confidence": 0.0 }
            ]
        }"#;
        let engine = LabelEngine::from_json_str(src).unwrap();
        // Primitive set `["a", "c"]` has no manifest counterpart.
        let observation = obs_with(&["a", "c"], 100, 0, 100);
        assert!(engine
            .assign(&observation, 100, TickCounter::new(100))
            .is_none());
    }

    #[test]
    fn assign_is_deterministic_across_calls() {
        let src = r#"{
            "labels": [
                { "id": "foo", "primitives": ["a", "b"], "min_confidence": 0.0 }
            ]
        }"#;
        let engine = LabelEngine::from_json_str(src).unwrap();
        let observation = obs_with(&["a", "b"], 17, 3, 91);
        let a = engine
            .assign(&observation, 200, TickCounter::new(150))
            .unwrap();
        let b = engine
            .assign(&observation, 200, TickCounter::new(150))
            .unwrap();
        assert_eq!(a.confidence.to_bits(), b.confidence.to_bits());
        assert_eq!(a.id, b.id);
        assert_eq!(a.signature, b.signature);
    }

    #[test]
    fn primitive_order_in_manifest_does_not_matter() {
        let a_src = r#"{
            "labels": [
                { "id": "foo", "primitives": ["a", "b", "c"], "min_confidence": 0.0 }
            ]
        }"#;
        let b_src = r#"{
            "labels": [
                { "id": "foo", "primitives": ["c", "b", "a"], "min_confidence": 0.0 }
            ]
        }"#;
        let observation = obs_with(&["b", "a", "c"], 5, 0, 5);
        let from_a = LabelEngine::from_json_str(a_src)
            .unwrap()
            .assign(&observation, 5, TickCounter::new(5))
            .unwrap();
        let from_b = LabelEngine::from_json_str(b_src)
            .unwrap()
            .assign(&observation, 5, TickCounter::new(5))
            .unwrap();
        assert_eq!(from_a.id, from_b.id);
        assert_eq!(from_a.confidence.to_bits(), from_b.confidence.to_bits());
    }
}
