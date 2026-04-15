//! Beast Evolution Game — Layer 1 channel registry and manifest loader.
//!
//! See individual modules for story-specific documentation. Sprint S2
//! populates this crate story-by-story: composition hooks (2.5) first so
//! the manifest types in 2.1 can reference them, then the manifest loader
//! and schema validator, then the registry.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod composition;

pub use composition::{evaluate_hook, CompositionHook, CompositionKind, HookOutcome};
