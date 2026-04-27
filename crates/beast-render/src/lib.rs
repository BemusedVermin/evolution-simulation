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

// Visual-pipeline modules. `blueprint` and `directive` are public so
// downstream crates (and the eventual interpreter Stage 4 emitter) can
// see the IR + output types. `pipeline` itself is crate-private — its
// only public surface is [`compile_blueprint`] re-exported below — to
// keep substage helpers from leaking into anyone's API.
pub mod animation;
pub mod blueprint;
pub(crate) mod channels;
pub mod directive;
pub(crate) mod pipeline;

// Sprite atlas: id → Rect lookup loaded from a JSON manifest. Pure-Rust;
// the GPU-upload step lands alongside the SDL renderers in S9.3 / S9.4.
pub mod sprite;

// World-map renderer: pure Camera math + biome tint + (under `sdl`) the
// SDL drawing entry points (`draw_archipelago`, `draw_creature_glyphs`).
pub mod world_map;

// Encounter-view renderer: 2.5D projection + depth ordering + (under
// `sdl`) the SDL drawing entry points (`draw_backdrop`, `draw_encounter`).
pub mod encounter;

pub use animation::{
    rig_animations, AnimationClip, AnimationSet, Animator, BoneRotation, BoneTrack, Easing,
    Keyframe, LocomotionStyle, PoseFrame,
};
pub use blueprint::CreatureBlueprint;
pub use directive::{ColorSpec, DirectiveParams, VisualDirective};
pub use encounter::{
    depth_extents, depth_order, normalize_depth, silhouette_size, Backdrop, EncounterEntity,
    Position2D, Projection,
};
pub use error::{RenderError, Result};
pub use pipeline::compile_blueprint;
pub use renderer::{Renderer, WindowConfig};
pub use sprite::{AtlasError, Rect, SpriteAtlas, SpriteId};
pub use world_map::{biome_tint, Camera, CreatureGlyph};
