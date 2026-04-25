//! `SaveValidator` — reject saves containing UI-derived state
//! (S7.4 — issue #132).
//!
//! Per INVARIANTS §6 and `documentation/systems/22_master_serialization.md`
//! §2.6, the save file is sim state only. Anything derivable at the UI
//! layer must not round-trip through the save:
//!
//! * `bestiary_discovered` — derived as `observation_count >= 1` at
//!   load time. Persisting it would let UI state retroactively
//!   contaminate replays.
//! * Anything matching `^ui_[a-z_]+$` — camera positions, scroll
//!   offsets, expanded-card flags, filter selections.
//!
//! The validator scans the parsed JSON tree for forbidden keys (or
//! caller-registered extras) at any nesting depth. It runs in O(n)
//! over the value tree. The save layer pipes a SaveFile through the
//! validator before writing to disk and before deserializing into a
//! `Simulation` — both directions are guarded.
//!
//! # Why a JSON-level check
//!
//! The struct shape in `crate::save` already excludes UI-flag fields,
//! so a well-behaved producer never trips this check. The validator
//! exists to defend against:
//!
//! * Hand-edited save files smuggling UI state into the schema's
//!   `extras` map.
//! * A future breaking change that adds a serde-flatten on a UI struct
//!   without updating the save layer.
//! * Mods that ship custom `extras` entries with UI-coupled keys.
//!
//! It is a belt + suspenders against the UI-state-in-save invariant,
//! not a substitute for the structural schema.

use serde_json::Value;
use thiserror::Error;

/// Default forbidden keys: literal matches that are never allowed in
/// any save file. Extended at runtime via [`SaveValidator::with_forbidden`]
/// for mod-specific UI state.
const DEFAULT_FORBIDDEN_LITERALS: &[&str] = &["bestiary_discovered"];

/// Default forbidden prefixes: any key starting with one of these is
/// rejected. Catches the `ui_*` family wholesale (camera, scroll,
/// filter selections, etc.) per System 22 §2.6. Extended at runtime
/// via [`SaveValidator::with_forbidden_prefix`] for mod-specific UI
/// state families (e.g., a mod shipping a `mod_ui_*` set).
const DEFAULT_FORBIDDEN_PREFIXES: &[&str] = &["ui_"];

/// Validates parsed save JSON against the UI-state-in-save invariant.
///
/// `SaveValidator` is constructed with the default forbidden set and
/// extended via the builder for caller-specific extras. Reuse one
/// validator across many `validate` calls — it is `Clone` and stateless
/// per validation.
///
/// # Determinism note
///
/// The validator does not hash, randomise, or otherwise affect sim
/// state — it only inspects an externally-supplied `serde_json::Value`.
/// The same input always produces the same `Result`.
#[derive(Debug, Clone)]
pub struct SaveValidator {
    forbidden_literals: Vec<String>,
    forbidden_prefixes: Vec<String>,
}

impl Default for SaveValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl SaveValidator {
    /// Build a validator preloaded with the default `bestiary_discovered`
    /// + `ui_*` rejection rules.
    #[must_use]
    pub fn new() -> Self {
        Self {
            forbidden_literals: DEFAULT_FORBIDDEN_LITERALS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            forbidden_prefixes: DEFAULT_FORBIDDEN_PREFIXES
                .iter()
                .map(|s| s.to_string())
                .collect(),
        }
    }

    /// Add a literal forbidden key — present in any save value at any
    /// depth, the validator rejects. Useful for mod-specific UI
    /// flags that the default rules don't anticipate.
    #[must_use]
    pub fn with_forbidden(mut self, key: impl Into<String>) -> Self {
        self.forbidden_literals.push(key.into());
        self
    }

    /// Add a forbidden prefix — any key beginning with this string is
    /// rejected. Useful for mod-specific UI families (e.g., a mod
    /// shipping a `mod_ui_*` set). Per PR #138 review.
    #[must_use]
    pub fn with_forbidden_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.forbidden_prefixes.push(prefix.into());
        self
    }

    /// Validate a parsed JSON value. Walks the tree and flags the first
    /// forbidden key encountered. The reported `path` is a
    /// dotted/indexed JSON Pointer-ish form (e.g.,
    /// `entities[3].extras.ui_camera_x`) so the offending location is
    /// easy to grep for in a save file.
    ///
    /// # Errors
    ///
    /// Returns [`ValidationError::ForbiddenKey`] on the first hit.
    /// Walks the entire tree on a clean pass; ordering matches
    /// `serde_json::Map` iteration (insertion order for the standard
    /// build, alphabetical with the `preserve_order` feature off in
    /// practice — both deterministic).
    pub fn validate(&self, value: &Value) -> Result<(), ValidationError> {
        self.walk(value, &mut PathBuf::default())
    }

    fn is_forbidden(&self, key: &str) -> bool {
        self.forbidden_literals.iter().any(|f| f == key)
            || self
                .forbidden_prefixes
                .iter()
                .any(|p| !p.is_empty() && key.starts_with(p.as_str()))
    }

    fn walk(&self, value: &Value, path: &mut PathBuf) -> Result<(), ValidationError> {
        match value {
            Value::Object(map) => {
                for (k, v) in map {
                    if self.is_forbidden(k) {
                        let mut full = path.clone();
                        full.push_field(k);
                        return Err(ValidationError::ForbiddenKey {
                            key: k.clone(),
                            path: full.render(),
                        });
                    }
                    path.push_field(k);
                    self.walk(v, path)?;
                    path.pop();
                }
            }
            Value::Array(items) => {
                for (i, v) in items.iter().enumerate() {
                    path.push_index(i);
                    self.walk(v, path)?;
                    path.pop();
                }
            }
            _ => {}
        }
        Ok(())
    }
}

/// Errors produced by [`SaveValidator::validate`]. `non_exhaustive` so
/// future schema-violation variants slot in without breaking match
/// sites.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ValidationError {
    /// The validator hit a forbidden key while walking the value tree.
    #[error("forbidden key `{key}` at `{path}` — UI-derived state must not appear in save")]
    ForbiddenKey {
        /// The offending key.
        key: String,
        /// JSON-pointer-ish path to the offending key.
        path: String,
    },
}

/// Internal helper: builds the dotted/indexed path string used in
/// [`ValidationError::ForbiddenKey::path`]. Owned here rather than
/// using a `Vec<Cow<'_, str>>` so the API can stay simple — the path
/// is reset on every `validate` call so allocation cost is bounded.
#[derive(Debug, Default, Clone)]
struct PathBuf {
    segments: Vec<Segment>,
}

#[derive(Debug, Clone)]
enum Segment {
    Field(String),
    Index(usize),
}

impl PathBuf {
    fn push_field(&mut self, name: &str) {
        self.segments.push(Segment::Field(name.to_string()));
    }
    fn push_index(&mut self, i: usize) {
        self.segments.push(Segment::Index(i));
    }
    fn pop(&mut self) {
        self.segments.pop();
    }
    fn render(&self) -> String {
        if self.segments.is_empty() {
            return "<root>".to_string();
        }
        let mut out = String::new();
        for (i, seg) in self.segments.iter().enumerate() {
            match seg {
                Segment::Field(name) => {
                    if i > 0 {
                        out.push('.');
                    }
                    // Per PR #138 review (MEDIUM): if the field name
                    // itself contains characters used by the path
                    // grammar (`.` for separator, `[` / `]` for
                    // indices), wrap it in backticks so the diagnostic
                    // is unambiguous. `entities[1].extras.bar` and a
                    // single field named `entities[1].extras.bar`
                    // would otherwise render identically.
                    if name.contains(['.', '[', ']']) {
                        out.push('`');
                        out.push_str(name);
                        out.push('`');
                    } else {
                        out.push_str(name);
                    }
                }
                Segment::Index(idx) => {
                    use std::fmt::Write as _;
                    let _ = write!(out, "[{idx}]");
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn empty_object_passes() {
        SaveValidator::new().validate(&json!({})).unwrap();
    }

    #[test]
    fn deeply_nested_clean_value_passes() {
        let v = json!({
            "format_version": "0.1.0",
            "entities": [
                {"id": 0, "extras": {"foo": 1}},
                {"id": 1, "extras": {"bar": 2}},
            ],
            "rng_streams": {"genetics": {"inner": {"state": [1, 2, 3, 4]}}},
        });
        SaveValidator::new().validate(&v).unwrap();
    }

    #[test]
    fn rejects_top_level_bestiary_discovered() {
        let v = json!({"bestiary_discovered": true});
        let err = SaveValidator::new().validate(&v).unwrap_err();
        match err {
            ValidationError::ForbiddenKey { key, path } => {
                assert_eq!(key, "bestiary_discovered");
                assert_eq!(path, "bestiary_discovered");
            }
        }
    }

    #[test]
    fn rejects_nested_bestiary_discovered_with_path() {
        let v = json!({
            "entities": [
                {"id": 0, "extras": {}},
                {"id": 1, "extras": {"bestiary_discovered": false}},
            ],
        });
        let err = SaveValidator::new().validate(&v).unwrap_err();
        match err {
            ValidationError::ForbiddenKey { key, path } => {
                assert_eq!(key, "bestiary_discovered");
                assert_eq!(path, "entities[1].extras.bestiary_discovered");
            }
        }
    }

    #[test]
    fn rejects_ui_prefix_keys() {
        for ui_key in ["ui_camera_x", "ui_filter_text", "ui_expanded_cards"] {
            let v = json!({ ui_key: 1 });
            let err = SaveValidator::new().validate(&v).unwrap_err();
            assert!(matches!(err, ValidationError::ForbiddenKey { .. }));
        }
    }

    #[test]
    fn allows_keys_that_merely_contain_ui_substring() {
        // Only the *prefix* `ui_` is forbidden; substring matches must
        // pass. `behaviour_uid` and `gui_state` look UI-ish but are not
        // covered by the rule.
        let v = json!({"behaviour_uid": 99, "gui_state": "open"});
        SaveValidator::new().validate(&v).unwrap();
    }

    #[test]
    fn caller_can_register_extra_forbidden_keys() {
        let v = json!({"mod_window_state": "open"});
        let validator = SaveValidator::new().with_forbidden("mod_window_state");
        let err = validator.validate(&v).unwrap_err();
        match err {
            ValidationError::ForbiddenKey { key, .. } => assert_eq!(key, "mod_window_state"),
        }
        // Without the registration, the same value passes.
        SaveValidator::new().validate(&v).unwrap();
    }

    #[test]
    fn savefile_from_save_module_passes_validation() {
        // The shipping `SaveFile::to_json` output must always be clean.
        // If the save shape ever sprouts a UI-derived field, this test
        // catches it.
        //
        // Per PR #138 review (LOW): exercise a populated SaveFile with
        // a non-empty `extras` map on at least one entity — that's the
        // attack surface the validator guards. The previous version of
        // this test only validated an empty world.
        use crate::save::SaveFile;
        use beast_channels::ChannelRegistry;
        use beast_ecs::components::{Age, Creature, Mass};
        use beast_ecs::{Builder, MarkerKind};
        use beast_primitives::PrimitiveRegistry;
        use beast_sim::{Simulation, SimulationConfig};

        let mut sim = Simulation::new(SimulationConfig {
            world_seed: 1,
            channels: ChannelRegistry::new(),
            primitives: PrimitiveRegistry::new(),
        });
        // Spawn a creature so SaveFile.entities is non-empty.
        let entity = sim
            .world_mut()
            .create_entity()
            .with(Creature)
            .with(Age::new(7))
            .with(Mass::new(beast_core::Q3232::from_num(3)))
            .build();
        sim.resources_mut()
            .entity_index
            .insert(entity, MarkerKind::Creature);

        let mut save = crate::manager::save_game(&sim).unwrap();
        // Manually populate `extras` on the entity so the validator
        // walks a non-trivial subtree. The keys here are deliberately
        // *allowed* (no UI prefix, no forbidden literal) — the assertion
        // is that valid extras pass validation.
        if let Some(rec) = save.entities.first_mut() {
            rec.extras
                .insert("origin_seed".into(), serde_json::json!(42));
            rec.extras
                .insert("biome_hint".into(), serde_json::json!("forest"));
        } else {
            unreachable!("test fixture must have an entity")
        }

        let s = save.to_json().unwrap();
        let value: serde_json::Value = serde_json::from_str(&s).unwrap();
        SaveValidator::new().validate(&value).unwrap();
        let _ = SaveFile::from_json(&s).unwrap(); // sanity: round-trips
    }

    #[test]
    fn caller_can_register_extra_forbidden_prefix() {
        // Per PR #138 review (MEDIUM): mod-specific UI families need
        // prefix extension. Confirms `with_forbidden_prefix` rejects
        // every key under the prefix.
        let v = json!({"mod_ui_camera_x": 0, "mod_ui_filter": "all"});
        let validator = SaveValidator::new().with_forbidden_prefix("mod_ui_");
        let err = validator.validate(&v).unwrap_err();
        assert!(matches!(err, ValidationError::ForbiddenKey { .. }));
        // Without the prefix registration, both keys are allowed.
        SaveValidator::new().validate(&v).unwrap();
    }

    #[test]
    fn path_renderer_escapes_field_names_with_path_chars() {
        // Per PR #138 review (MEDIUM): if a field name contains `.` /
        // `[` / `]`, the diagnostic must distinguish it from a real
        // path segment.
        let v = json!({
            "weird.key.with.dots": {"ui_inner": 1}
        });
        let err = SaveValidator::new().validate(&v).unwrap_err();
        match err {
            ValidationError::ForbiddenKey { path, .. } => {
                assert_eq!(path, "`weird.key.with.dots`.ui_inner");
            }
        }
    }

    #[test]
    fn path_uses_dot_for_fields_and_brackets_for_indices() {
        // The path renderer is the diagnostic of last resort; lock it
        // in so a refactor doesn't accidentally start emitting `/`-
        // separated JSON Pointers (which would silently confuse anyone
        // already grepping log output for `entities[N]`).
        let v = json!({
            "a": [
                {"b": {"c": [{"ui_x": 1}]}}
            ]
        });
        let err = SaveValidator::new().validate(&v).unwrap_err();
        match err {
            ValidationError::ForbiddenKey { path, .. } => {
                assert_eq!(path, "a[0].b.c[0].ui_x");
            }
        }
    }
}
