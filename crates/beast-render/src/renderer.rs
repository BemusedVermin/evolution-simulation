//! [`Renderer`] facade: the entry point higher-level crates use.
//!
//! Two backends compile-switched by feature flag:
//!
//! * `sdl` — owns an [`sdl3::Sdl`] context, the [`sdl3::VideoSubsystem`],
//!   a [`sdl3::video::Window`], and a `Canvas`. Construction is fallible
//!   because SDL initialisation can fail (missing libs, unsupported
//!   platform, etc).
//! * `headless` — stores only the [`WindowConfig`] and exposes the same
//!   public surface so test code doesn't have to branch on cfg.

use crate::error::{RenderError, Result};

/// Caller-supplied window settings. Fields use `u32` (no negatives) and
/// `String` so consumers don't need to know SDL3's types.
#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub resizable: bool,
    /// When true, the SDL backend asks for a vsync-enabled present so the
    /// app loop is locked to the display refresh rate. The headless
    /// backend ignores this.
    pub vsync: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "Beast Evolution Game".to_string(),
            width: 1280,
            height: 720,
            resizable: true,
            vsync: true,
        }
    }
}

// ---------------------------------------------------------------------------
// SDL backend
// ---------------------------------------------------------------------------

#[cfg(feature = "sdl")]
mod sdl_backend {
    use super::{RenderError, Result, WindowConfig};
    use sdl3::{render::WindowCanvas, video::Window, EventPump, Sdl, VideoSubsystem};

    pub struct Renderer {
        // Field order matters for drop: canvas must drop before window,
        // window before video, video before sdl. Rust drops in declaration
        // order, so we keep the most-derived field first.
        canvas: WindowCanvas,
        event_pump: EventPump,
        config: WindowConfig,
        // Held so the Window stays alive — the canvas borrows from it
        // internally via SDL refcounting.
        _video: VideoSubsystem,
        _sdl: Sdl,
    }

    impl Renderer {
        pub fn new(config: WindowConfig) -> Result<Self> {
            let sdl = sdl3::init().map_err(|e| RenderError::sdl(e.to_string()))?;
            let video = sdl.video().map_err(|e| RenderError::sdl(e.to_string()))?;
            let event_pump = sdl
                .event_pump()
                .map_err(|e| RenderError::sdl(e.to_string()))?;

            let mut builder = video.window(&config.title, config.width, config.height);
            builder.position_centered();
            if config.resizable {
                builder.resizable();
            }
            let window: Window = builder
                .build()
                .map_err(|e| RenderError::sdl(e.to_string()))?;

            let canvas = window.into_canvas();
            // sdl3 0.18 doesn't surface SDL_SetRenderVSync on Canvas /
            // WindowBuilder yet (see follow-up). The native SDL3 default
            // present mode is FIFO (= vsync on), so this only matters
            // when callers want to disable vsync — currently we can't.
            // `config.vsync` is wired through so the caller's intent is
            // visible; we drop a runtime warning for explicit-off so the
            // mismatch is loud rather than silent.
            if !config.vsync {
                eprintln!(
                    "beast-render: WindowConfig.vsync=false requested, but sdl3 0.18 \
                     does not expose SDL_SetRenderVSync; vsync stays at the SDL3 default."
                );
            }

            Ok(Self {
                canvas,
                event_pump,
                config,
                _video: video,
                _sdl: sdl,
            })
        }

        pub fn config(&self) -> &WindowConfig {
            &self.config
        }

        pub fn canvas(&mut self) -> &mut WindowCanvas {
            &mut self.canvas
        }

        pub fn event_pump(&mut self) -> &mut EventPump {
            &mut self.event_pump
        }

        pub fn present(&mut self) {
            self.canvas.present();
        }
    }
}

#[cfg(feature = "sdl")]
pub use sdl_backend::Renderer;

// ---------------------------------------------------------------------------
// Headless backend
// ---------------------------------------------------------------------------

#[cfg(all(not(feature = "sdl"), feature = "headless"))]
mod headless_backend {
    use super::{RenderError, Result, WindowConfig};

    /// No-op renderer. `present` and event-pump access return
    /// [`RenderError::Headless`] so tests can assert SDL-only paths
    /// short-circuit cleanly.
    pub struct Renderer {
        config: WindowConfig,
    }

    impl Renderer {
        pub fn new(config: WindowConfig) -> Result<Self> {
            Ok(Self { config })
        }

        pub fn config(&self) -> &WindowConfig {
            &self.config
        }

        pub fn present(&mut self) -> Result<()> {
            Err(RenderError::Headless(
                "present is unavailable in headless mode",
            ))
        }
    }
}

#[cfg(all(not(feature = "sdl"), feature = "headless"))]
pub use headless_backend::Renderer;

// ---------------------------------------------------------------------------
// Compile-time assertion: at least one backend must be selected.
// ---------------------------------------------------------------------------

#[cfg(not(any(feature = "sdl", feature = "headless")))]
compile_error!("beast-render requires at least one of the `sdl` or `headless` features");

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_config_defaults_match_design_doc() {
        // Design doc / S9.1: 1280x720 default, resizable, vsync on.
        let cfg = WindowConfig::default();
        assert_eq!(cfg.width, 1280);
        assert_eq!(cfg.height, 720);
        assert!(cfg.resizable);
        assert!(cfg.vsync);
    }

    #[test]
    fn window_config_is_clone() {
        let cfg = WindowConfig::default();
        let cfg2 = cfg.clone();
        assert_eq!(cfg.width, cfg2.width);
    }

    #[cfg(all(not(feature = "sdl"), feature = "headless"))]
    #[test]
    fn headless_renderer_constructs_and_rejects_present() {
        let mut r = Renderer::new(WindowConfig::default())
            .expect("headless renderer should always construct");
        assert_eq!(r.config().width, 1280);
        let err = r.present().expect_err("headless present must error");
        assert!(matches!(err, RenderError::Headless(_)));
    }
}
