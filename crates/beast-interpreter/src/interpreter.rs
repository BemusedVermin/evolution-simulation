//! Top-level entry point [`interpret_phenotype`].
//!
//! Sequencing through the six stages in §6 of the design doc:
//!
//! 1. Scale-band filter ([`crate::scale_band`])
//! 2. Affordance filter ([`crate::expression`])
//! 3. Composition hook resolve ([`crate::composition`])
//! 4. Primitive emission + dedup/merge ([`crate::emission`])
//! 5. Body-region tiling ([`crate::body_map`])
//!
//! Behaviour compilation (stage 3 in the doc) lives downstream and is not part
//! of S4.
//!
//! Wired up incrementally as each story in epic #16 lands; the current stub
//! exists so downstream crates can reference the entry-point signature.

// Implementation — wired up as stories 4.1–4.6 complete.
