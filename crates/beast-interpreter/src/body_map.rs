//! Body-region tiling and per-site aggregation (S4.5 — issue #59).
//!
//! When a composition hook references channels marked
//! `body_site_applicable = true`, emission fans out to one
//! [`beast_primitives::PrimitiveEffect`] per body region on the creature.
//! Global channels broadcast their value to every site invocation. This
//! module also provides `aggregate_to_global(channel, strategy, per_site)`
//! for UI / combat code that needs a single scalar summary per body-site
//! channel.
//!
//! See `documentation/systems/11_phenotype_interpreter.md` §6.0B.

// Implementation — see story S4.5 (#59).
