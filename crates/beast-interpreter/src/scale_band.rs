//! Stage 1A: scale-band channel filtering (S4.1 — issue #55).
//!
//! Filters the channel values on a [`crate::ResolvedPhenotype`] so that any
//! channel whose manifest `scale_band` excludes the creature's body mass is
//! reduced to [`beast_core::Q3232::ZERO`]. This makes the dormant-channel
//! propagation rule in §6.2 (zero operand ⇒ threshold fails, zero parameter
//! ⇒ zero intensity) fall out of the arithmetic automatically.
//!
//! See `documentation/systems/11_phenotype_interpreter.md` §6.0 and
//! INVARIANTS §5 (scale-band unification).

// Implementation — see story S4.1 (#55).
