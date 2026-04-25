//! Schema-version migrations for save files (S7.5 — issue #133).
//!
//! Per System 22 §2 every save records its `format_version`. On load,
//! the migration registry walks a series of `from -> to` steps until
//! the value matches the binary's current version. Unknown versions
//! and forward-compat saves (newer than the binary) are surfaced as
//! distinct errors so user-facing tooling can tell them apart.
//!
//! The MVP only ships one schema (`0.1.0`); the registry returned by
//! [`MigrationRegistry::default`] is empty. The framework is in place
//! so the next breaking-format PR drops in one [`Migration`] impl plus
//! a registration line — no reshuffling of the load path.
//!
//! # How a migration step looks
//!
//! ```ignore
//! struct V0_1_0_to_V0_2_0;
//! impl Migration for V0_1_0_to_V0_2_0 {
//!     fn from_version(&self) -> &'static str { "0.1.0" }
//!     fn to_version(&self)   -> &'static str { "0.2.0" }
//!     fn migrate(&self, mut value: serde_json::Value)
//!         -> Result<serde_json::Value, MigrationError>
//!     {
//!         // Mutate `value` in place — add fields, rename keys, etc.
//!         value["format_version"] = serde_json::json!("0.2.0");
//!         Ok(value)
//!     }
//! }
//! ```
//!
//! # Determinism
//!
//! Migrations are pure functions of the input value. They never read
//! the clock, draw randomness, or hash anything stable. This makes the
//! upgraded save bit-identical across machines for any given old save.

use std::collections::BTreeMap;

use serde_json::Value;
use thiserror::Error;

use crate::save::SAVE_FORMAT_VERSION;

/// One `from -> to` schema step. Implementations are pure functions
/// over the parsed JSON tree.
///
/// The `from_*` / `to_*` accessor names match the domain (schema
/// migrations are universally described as "from version X to version
/// Y"); they are not constructor-style. The clippy lint is suppressed
/// for that reason — a rename to e.g. `source_version` would obscure
/// the semantics.
#[allow(clippy::wrong_self_convention)]
pub trait Migration: Send + Sync {
    /// The version this migration accepts as input.
    fn from_version(&self) -> &'static str;
    /// The version produced by [`Self::migrate`].
    fn to_version(&self) -> &'static str;
    /// Apply the schema change. Implementations must update the
    /// envelope's `format_version` field to [`Self::to_version`] —
    /// the registry uses that to drive subsequent steps.
    fn migrate(&self, value: Value) -> Result<Value, MigrationError>;
}

/// Indexed registry of [`Migration`]s. `upgrade` walks them deterministically
/// to bring an old save up to the binary's current `format_version`.
///
/// The registry is built once at process start and read-only thereafter.
/// `Default` returns the registry shipped with the running binary —
/// today this is empty (no schema bumps have happened yet); future
/// schema changes register their step in the same place.
pub struct MigrationRegistry {
    /// Indexed by `from_version`. Storing one step per key forces
    /// migrations to form a linear chain — branching upgrades
    /// (`0.1.0 -> 0.2.0` *and* `0.1.0 -> 0.3.0`) are not representable
    /// without an explicit decision, which is the right default for a
    /// deterministic save format.
    by_from: BTreeMap<String, Box<dyn Migration>>,
    current_version: &'static str,
}

impl Default for MigrationRegistry {
    fn default() -> Self {
        // The MVP binary speaks only SAVE_FORMAT_VERSION; no migrations
        // registered. When a 0.2.0 lands, push a `V0_1_0_to_V0_2_0` step
        // here.
        Self {
            by_from: BTreeMap::new(),
            current_version: SAVE_FORMAT_VERSION,
        }
    }
}

impl MigrationRegistry {
    /// Build an empty registry whose `current_version` is whatever the
    /// caller declares. Production code should use [`Self::default`];
    /// this constructor exists for tests that synthesize older / newer
    /// versions.
    #[must_use]
    pub fn new(current_version: &'static str) -> Self {
        Self {
            by_from: BTreeMap::new(),
            current_version,
        }
    }

    /// Register a migration step. The `from_version` is the key.
    ///
    /// # Panics
    ///
    /// Panics if a step with the same `from_version` is already
    /// registered. Two steps from the same source are ambiguous (which
    /// path do we take?) and indicate a programming error in the
    /// shipping registry — better to fail loudly at startup than to
    /// silently pick one.
    pub fn register(&mut self, step: Box<dyn Migration>) {
        let key = step.from_version().to_string();
        if self.by_from.contains_key(&key) {
            panic!(
                "duplicate migration registered for from_version `{}` — only one step per source allowed",
                key
            );
        }
        self.by_from.insert(key, step);
    }

    /// Version this registry knows how to produce as the final upgrade
    /// target.
    #[must_use]
    pub fn current_version(&self) -> &'static str {
        self.current_version
    }

    /// Apply the migration chain starting at `value["format_version"]`
    /// until the version matches [`Self::current_version`]. A no-op
    /// when the value is already current.
    ///
    /// # Errors
    ///
    /// * [`MigrationError::MissingFormatVersion`] if the input value
    ///   has no `format_version` field or it is not a string.
    /// * [`MigrationError::UnknownVersion`] if no registered step
    ///   accepts the current `format_version` and it is not the
    ///   target. The version is reported verbatim.
    /// * [`MigrationError::ForwardCompat`] if the input version is
    ///   *newer* than `current_version` (lexicographic comparison; for
    ///   the semver ordering we use, this is sufficient as long as
    ///   versions stay zero-padded). Reported as a distinct variant so
    ///   tooling can prompt for a binary upgrade rather than mark the
    ///   save corrupt.
    /// * [`MigrationError::Step`] if the step itself returned an
    ///   error.
    /// * [`MigrationError::WrongOutputVersion`] if a step claimed to
    ///   produce version X but the resulting value's `format_version`
    ///   is not X.
    pub fn upgrade(&self, mut value: Value) -> Result<Value, MigrationError> {
        loop {
            let current = value
                .get("format_version")
                .and_then(Value::as_str)
                .ok_or(MigrationError::MissingFormatVersion)?
                .to_string();

            if current == self.current_version {
                return Ok(value);
            }

            // Forward-compat detection: only declare it when the input
            // version is strictly greater than the binary's current
            // version *and* no migration step accepts that input. The
            // second condition prevents false positives in a future
            // where the binary supports a downgrade chain.
            if current.as_str() > self.current_version && !self.by_from.contains_key(&current) {
                return Err(MigrationError::ForwardCompat {
                    binary: self.current_version,
                    save: current,
                });
            }

            let Some(step) = self.by_from.get(&current) else {
                return Err(MigrationError::UnknownVersion(current));
            };
            let expected_to = step.to_version();
            value = step.migrate(value)?;

            // Confirm the step actually wrote the version it promised.
            // This catches a malformed step before it tail-recurses
            // forever in the loop above.
            let after = value
                .get("format_version")
                .and_then(Value::as_str)
                .ok_or(MigrationError::MissingFormatVersion)?;
            if after != expected_to {
                return Err(MigrationError::WrongOutputVersion {
                    declared: expected_to,
                    actual: after.to_string(),
                });
            }
        }
    }
}

/// Failures emitted by [`MigrationRegistry::upgrade`] and individual
/// [`Migration::migrate`] implementations.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum MigrationError {
    /// The input value lacked a `format_version` string field.
    #[error("save value is missing required `format_version` string field")]
    MissingFormatVersion,

    /// No registered step accepts the current `format_version`.
    #[error("no migration step registered for save version `{0}`")]
    UnknownVersion(String),

    /// The save's version is newer than the running binary supports.
    #[error(
        "save format `{save}` is newer than this binary supports (`{binary}`); upgrade the binary"
    )]
    ForwardCompat {
        /// Version the binary speaks.
        binary: &'static str,
        /// Version found in the save.
        save: String,
    },

    /// A migration step itself failed (key missing, value malformed,
    /// etc.). Use this from inside [`Migration::migrate`] to surface
    /// arbitrary failures with context.
    #[error("migration step failed: {0}")]
    Step(String),

    /// A migration step claimed to produce one version but wrote a
    /// different one. Indicates a buggy step.
    #[error(
        "migration step declared output version `{declared}` but produced `{actual}` — buggy step"
    )]
    WrongOutputVersion {
        /// Version the step's `to_version` returns.
        declared: &'static str,
        /// Version actually written into the migrated value.
        actual: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn default_registry_has_current_save_format_version() {
        let r = MigrationRegistry::default();
        assert_eq!(r.current_version(), SAVE_FORMAT_VERSION);
    }

    #[test]
    fn upgrade_is_noop_when_already_current() {
        let r = MigrationRegistry::default();
        let value = json!({ "format_version": SAVE_FORMAT_VERSION, "payload": 42 });
        let upgraded = r.upgrade(value.clone()).unwrap();
        assert_eq!(upgraded, value);
    }

    #[test]
    fn unknown_version_returns_clear_error() {
        let r = MigrationRegistry::default();
        let value = json!({ "format_version": "0.0.7" });
        let err = r.upgrade(value).unwrap_err();
        match err {
            MigrationError::UnknownVersion(v) => assert_eq!(v, "0.0.7"),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn missing_format_version_returns_dedicated_error() {
        let r = MigrationRegistry::default();
        let value = json!({ "payload": 42 });
        assert!(matches!(
            r.upgrade(value).unwrap_err(),
            MigrationError::MissingFormatVersion
        ));
    }

    #[test]
    fn forward_compat_detection() {
        // Save written by a future binary against schema 9.9.9. The
        // current registry has no step for 9.9.9 and the version is
        // > current — that's forward-compat, not unknown-version.
        let r = MigrationRegistry::default();
        let value = json!({ "format_version": "9.9.9" });
        match r.upgrade(value).unwrap_err() {
            MigrationError::ForwardCompat { binary, save } => {
                assert_eq!(binary, SAVE_FORMAT_VERSION);
                assert_eq!(save, "9.9.9");
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    /// Synthetic step used by the multi-step upgrade test below.
    /// Bumps the version and adds a key — exercises the chain logic
    /// without forcing the real codebase to ship a fake schema bump.
    struct AddPayloadKey {
        from: &'static str,
        to: &'static str,
        key: &'static str,
    }
    impl Migration for AddPayloadKey {
        fn from_version(&self) -> &'static str {
            self.from
        }
        fn to_version(&self) -> &'static str {
            self.to
        }
        fn migrate(&self, mut value: Value) -> Result<Value, MigrationError> {
            value[self.key] = json!(true);
            value["format_version"] = json!(self.to);
            Ok(value)
        }
    }

    #[test]
    fn multi_step_upgrade_chain() {
        // 0.0.1 -> 0.0.2 -> 0.0.3, each step adds a key.
        let mut r = MigrationRegistry::new("0.0.3");
        r.register(Box::new(AddPayloadKey {
            from: "0.0.1",
            to: "0.0.2",
            key: "added_in_2",
        }));
        r.register(Box::new(AddPayloadKey {
            from: "0.0.2",
            to: "0.0.3",
            key: "added_in_3",
        }));

        let input = json!({ "format_version": "0.0.1" });
        let out = r.upgrade(input).unwrap();
        assert_eq!(out["format_version"], "0.0.3");
        assert_eq!(out["added_in_2"], true);
        assert_eq!(out["added_in_3"], true);
    }

    #[test]
    fn buggy_step_that_does_not_write_declared_version_is_caught() {
        // Step claims 0.0.2 but writes 0.0.7. The registry must catch
        // this rather than spinning waiting for 0.0.7 to arrive.
        struct LiarStep;
        impl Migration for LiarStep {
            fn from_version(&self) -> &'static str {
                "0.0.1"
            }
            fn to_version(&self) -> &'static str {
                "0.0.2"
            }
            fn migrate(&self, mut value: Value) -> Result<Value, MigrationError> {
                value["format_version"] = json!("0.0.7"); // wrong!
                Ok(value)
            }
        }
        let mut r = MigrationRegistry::new("0.0.2");
        r.register(Box::new(LiarStep));
        let err = r.upgrade(json!({ "format_version": "0.0.1" })).unwrap_err();
        match err {
            MigrationError::WrongOutputVersion { declared, actual } => {
                assert_eq!(declared, "0.0.2");
                assert_eq!(actual, "0.0.7");
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    #[should_panic(expected = "duplicate migration")]
    fn duplicate_registration_panics() {
        let mut r = MigrationRegistry::new("0.0.2");
        r.register(Box::new(AddPayloadKey {
            from: "0.0.1",
            to: "0.0.2",
            key: "k",
        }));
        // Second registration with same `from` — should panic.
        r.register(Box::new(AddPayloadKey {
            from: "0.0.1",
            to: "0.0.2",
            key: "other",
        }));
    }

    #[test]
    fn step_can_propagate_its_own_errors() {
        struct Failing;
        impl Migration for Failing {
            fn from_version(&self) -> &'static str {
                "0.0.1"
            }
            fn to_version(&self) -> &'static str {
                "0.0.2"
            }
            fn migrate(&self, _value: Value) -> Result<Value, MigrationError> {
                Err(MigrationError::Step("synthetic failure".into()))
            }
        }
        let mut r = MigrationRegistry::new("0.0.2");
        r.register(Box::new(Failing));
        let err = r.upgrade(json!({ "format_version": "0.0.1" })).unwrap_err();
        assert!(matches!(err, MigrationError::Step(s) if s.contains("synthetic")));
    }
}
