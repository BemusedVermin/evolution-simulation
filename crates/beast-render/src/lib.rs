//! Rendering layer for the Beast Evolution Game.
//!
//! `beast-render` owns the SDL3 lifecycle (video subsystem, window, canvas)
//! and exposes a [`Renderer`] facade that the higher-level UI / app crates
//! call into. Sim state is *never* mutated from this crate — see
//! `documentation/INVARIANTS.md` §1 ("never feed render-derived values back
//! into sim state") and §6 (UI vs sim state).
//!
//! # Feature flags
//!
//! * `sdl` (default) — links a vendored, statically built SDL3 via the
//!   `sdl3` crate's `build-from-source-static` feature. Requires `cmake`
//!   and a C toolchain on PATH at build time.
//! * `headless` — compiles the public API without linking SDL3. The
//!   [`Renderer`] in this mode is a no-op stub used by CI and integration
//!   tests on machines that don't have SDL3 native libs available.
//!
//! Both features may be enabled at once (cargo-deny runs with
//! `all-features = true`); the `sdl` backend wins when both are set.

#![cfg_attr(docsrs, feature(doc_cfg))]

// Modules are crate-private; consumers should depend on the re-exports
// below so they don't couple to internal module paths.
pub(crate) mod error;
pub(crate) mod renderer;

pub use error::{RenderError, Result};
pub use renderer::{Renderer, WindowConfig};
