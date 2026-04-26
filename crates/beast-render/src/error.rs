//! Typed errors for the render layer.
//!
//! SDL3 surfaces errors as `String` from its safe Rust bindings; we wrap
//! them in [`RenderError::Sdl`] so callers don't have to depend on the
//! `sdl3` crate to pattern-match on backend failures. The `headless`
//! backend can construct its own variants (e.g. [`RenderError::Headless`])
//! without forcing every consumer to enable the SDL feature.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, RenderError>;

#[derive(Debug, Error)]
pub enum RenderError {
    /// An SDL3 subsystem returned an error string. The wrapped string is
    /// the verbatim message from SDL; we don't try to parse it.
    #[error("sdl3: {0}")]
    Sdl(String),

    /// An operation that requires the SDL backend was attempted while the
    /// renderer is running in headless mode.
    #[error("headless renderer: {0}")]
    Headless(&'static str),
}

impl RenderError {
    // Used by the `sdl` backend; lives here so callers don't need to
    // `import RenderError::Sdl` everywhere. Headless builds don't reach it.
    #[cfg_attr(not(feature = "sdl"), allow(dead_code))]
    pub(crate) fn sdl(msg: impl Into<String>) -> Self {
        Self::Sdl(msg.into())
    }
}
