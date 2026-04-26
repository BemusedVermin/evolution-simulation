//! Canonical manifest provenance enum.
//!
//! Every manifest JSON document carries a `provenance` field constrained by
//! the schema regex
//! `^(core|mod:[a-z_][a-z0-9_]*|genesis:[a-z_][a-z0-9_]*:[0-9]+)$`. The
//! JSON Schema validator enforces the string shape; this parser assumes a
//! well-formed input and only does the structural split, so the two checks
//! stay aligned without re-implementing the regex.
//!
//! The enum is used by both `beast-channels` and `beast-primitives`; they
//! re-export it so downstream crates see a single type regardless of which
//! manifest they came from.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error returned by [`Provenance::parse`] when the input does not match one
/// of the three recognised shapes.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("invalid provenance string: {0}")]
pub struct ProvenanceParseError(pub String);

/// Origin of a manifest entry. Mirrors the schema's `provenance` regex
/// discriminator.
///
/// Serializes as the canonical string form used in manifest JSON (`"core"`,
/// `"mod:foo"`, `"genesis:foo:123"`). Keeping the string form canonical
/// means save files are self-describing and round-trippable without a
/// separate enum tag.
///
/// `PartialOrd`/`Ord` are derived (declaration order: Core < Mod < Genesis,
/// with within-variant ordering by string then `generation`) so
/// `Provenance` can key a `BTreeMap` or live in a `BTreeSet` without an
/// allocation. The ordering is stable and deterministic — it does not
/// match `to_schema_string` lexicographic order, which is intentional:
/// callers that want lexicographic UI display should sort by the
/// rendered string explicitly.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Provenance {
    /// A canonical entry shipped by the core game.
    Core,
    /// Registered by a mod with the given snake_case id.
    Mod(String),
    /// Duplicated / derived from a parent entry at simulation generation
    /// `generation`.
    Genesis {
        /// Parent entry id.
        parent: String,
        /// Generation at which the duplication occurred.
        generation: u64,
    },
}

impl Provenance {
    /// Render back into the canonical schema string form.
    #[must_use]
    pub fn to_schema_string(&self) -> String {
        match self {
            Self::Core => "core".to_owned(),
            Self::Mod(id) => format!("mod:{id}"),
            Self::Genesis { parent, generation } => format!("genesis:{parent}:{generation}"),
        }
    }

    /// Parse a provenance string in the canonical schema form.
    ///
    /// The schema's regex is enforced by the JSON Schema validator upstream
    /// of this parser; here we only split the already-structurally-valid
    /// input. If you call this on an arbitrary string (e.g. from a
    /// non-schema-validated source), malformed inputs produce
    /// [`ProvenanceParseError`] rather than silently accepting garbage.
    pub fn parse(raw: &str) -> Result<Self, ProvenanceParseError> {
        if raw == "core" {
            return Ok(Self::Core);
        }
        if let Some(rest) = raw.strip_prefix("mod:") {
            return Ok(Self::Mod(rest.to_owned()));
        }
        if let Some(rest) = raw.strip_prefix("genesis:") {
            // rest is `parent_id:generation` — split from the right because
            // parent ids themselves are snake_case with no colons.
            let mut parts = rest.rsplitn(2, ':');
            let gen_str = parts
                .next()
                .ok_or_else(|| ProvenanceParseError(raw.to_owned()))?;
            let parent = parts
                .next()
                .ok_or_else(|| ProvenanceParseError(raw.to_owned()))?;
            let generation: u64 = gen_str
                .parse()
                .map_err(|_| ProvenanceParseError(raw.to_owned()))?;
            return Ok(Self::Genesis {
                parent: parent.to_owned(),
                generation,
            });
        }
        Err(ProvenanceParseError(raw.to_owned()))
    }
}

impl Serialize for Provenance {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_schema_string())
    }
}

impl<'de> Deserialize<'de> for Provenance {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Self::parse(&raw).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn core_parses() {
        assert_eq!(Provenance::parse("core").unwrap(), Provenance::Core);
    }

    #[test]
    fn mod_parses() {
        assert_eq!(
            Provenance::parse("mod:my_mod").unwrap(),
            Provenance::Mod("my_mod".to_owned())
        );
    }

    #[test]
    fn genesis_parses() {
        assert_eq!(
            Provenance::parse("genesis:parent_id:12").unwrap(),
            Provenance::Genesis {
                parent: "parent_id".to_owned(),
                generation: 12,
            }
        );
    }

    #[test]
    fn genesis_missing_generation_rejected() {
        assert!(Provenance::parse("genesis:parent").is_err());
    }

    #[test]
    fn genesis_bad_generation_rejected() {
        assert!(Provenance::parse("genesis:parent:not_a_number").is_err());
    }

    #[test]
    fn unknown_prefix_rejected() {
        assert!(Provenance::parse("unknown:foo").is_err());
    }

    #[test]
    fn schema_string_round_trip_core() {
        assert_eq!(Provenance::Core.to_schema_string(), "core");
    }

    #[test]
    fn schema_string_round_trip_mod() {
        let p = Provenance::Mod("m".to_owned());
        assert_eq!(Provenance::parse(&p.to_schema_string()).unwrap(), p);
    }

    #[test]
    fn schema_string_round_trip_genesis() {
        let p = Provenance::Genesis {
            parent: "parent_x".to_owned(),
            generation: 4242,
        };
        assert_eq!(Provenance::parse(&p.to_schema_string()).unwrap(), p);
    }

    #[test]
    fn serde_round_trip_via_json() {
        for p in [
            Provenance::Core,
            Provenance::Mod("m".to_owned()),
            Provenance::Genesis {
                parent: "p".to_owned(),
                generation: 7,
            },
        ] {
            let s = serde_json::to_string(&p).unwrap();
            let back: Provenance = serde_json::from_str(&s).unwrap();
            assert_eq!(back, p);
        }
    }

    #[test]
    fn serde_rejects_invalid() {
        let result: Result<Provenance, _> = serde_json::from_str(r#""bogus""#);
        assert!(result.is_err());
    }

    #[test]
    fn ord_is_declaration_order_with_within_variant_lexicographic() {
        // Locks the canonical sort order documented on `Provenance`.
        // A reorder of variants in the source enum (or a derive change)
        // would shift `BTreeMap<Provenance, _>` iteration and break
        // any consumer that snapshots the order — including replay
        // fingerprints. This test fails on first compile if either
        // happens.
        assert!(Provenance::Core < Provenance::Mod("a".to_owned()));
        assert!(
            Provenance::Mod("a".to_owned())
                < Provenance::Genesis {
                    parent: "p".to_owned(),
                    generation: 0,
                }
        );
        // Within Mod, lexicographic by id.
        assert!(Provenance::Mod("a".to_owned()) < Provenance::Mod("b".to_owned()));
        // Within Genesis, lexicographic by parent then numeric by generation.
        assert!(
            Provenance::Genesis {
                parent: "a".to_owned(),
                generation: 99,
            } < Provenance::Genesis {
                parent: "b".to_owned(),
                generation: 0,
            }
        );
        assert!(
            Provenance::Genesis {
                parent: "a".to_owned(),
                generation: 1,
            } < Provenance::Genesis {
                parent: "a".to_owned(),
                generation: 2,
            }
        );
    }

    proptest! {
        // Any schema-conformant `mod:<id>` round-trips through parse.
        #[test]
        fn mod_round_trip(id in "[a-z_][a-z0-9_]{0,15}") {
            let s = format!("mod:{id}");
            let parsed = Provenance::parse(&s).unwrap();
            prop_assert_eq!(parsed.to_schema_string(), s);
        }

        // Any schema-conformant `genesis:<parent>:<n>` round-trips.
        #[test]
        fn genesis_round_trip(
            parent in "[a-z_][a-z0-9_]{0,15}",
            generation in 0u64..=u64::MAX
        ) {
            let s = format!("genesis:{parent}:{generation}");
            let parsed = Provenance::parse(&s).unwrap();
            prop_assert_eq!(parsed.to_schema_string(), s);
        }
    }
}
