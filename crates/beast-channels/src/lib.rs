//! Beast Evolution Game — Layer 1 channel registry and manifest loader.
//!
//! See individual modules for story-specific documentation. Sprint S2
//! populates this crate story-by-story: composition hooks (2.5), then the
//! manifest loader + expression-condition evaluator + schema validator
//! (2.1), then the registry (2.3).

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod composition;
pub mod expression;
pub mod manifest;
pub mod schema;

pub use composition::{evaluate_hook, CompositionHook, CompositionKind, HookOutcome};
pub use expression::{evaluate_expression_conditions, ExpressionCondition, ExpressionContext};
pub use manifest::{
    BoundsPolicy, ChannelFamily, ChannelManifest, CorrelationEntry, MutationKernel, Provenance,
    Range, ScaleBand,
};
pub use schema::{ChannelLoadError, CHANNEL_MANIFEST_SCHEMA};
