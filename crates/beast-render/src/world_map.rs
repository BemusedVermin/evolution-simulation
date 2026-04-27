//! World-map renderer (S9.3).
//!
//! Draws an [`beast_world::Archipelago`] grid + a slice of
//! [`CreatureGlyph`]s through a 2D [`Camera`] (pan + zoom). Camera state
//! lives entirely in render space — pure floats, never written to sim.
//!
//! The module is split into a *pure* surface (Camera math, biome
//! palette, glyph types, culling) that compiles and tests without SDL,
//! and an SDL-only backend that performs the actual drawing. CI exercises
//! the pure surface in headless mode; the SDL backend is the smoke test
//! at `examples/world_map.rs`.
//!
//! See `documentation/systems/10_procgen_visual_pipeline.md` and the
//! issue body of #183 for the design contract.

use beast_world::BiomeTag;

// ---------------------------------------------------------------------------
// Pure types (always compiled — no SDL coupling)
// ---------------------------------------------------------------------------

/// A creature glyph that the renderer should draw at a given tile
/// coordinate, tinted by the supplied RGB color.
///
/// Tile coords are integer cell indices into the archipelago grid;
/// fractional positions belong on the encounter view, not here. The
/// `species_tint` is applied as a colour-modulation pass over the
/// generic creature glyph sprite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CreatureGlyph {
    pub tile_x: i32,
    pub tile_y: i32,
    pub species_tint: [u8; 3],
}

/// Camera state. Floats are fine — render-only, never round-tripped
/// through sim state per INVARIANTS §1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    /// Tile coordinate at the centre of the viewport.
    pub center_tile_x: f32,
    pub center_tile_y: f32,
    /// Pixels per tile; min 4, max 128. Higher = zoomed in.
    pub zoom: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            center_tile_x: 0.0,
            center_tile_y: 0.0,
            zoom: 16.0,
        }
    }
}

impl Camera {
    pub const MIN_ZOOM: f32 = 4.0;
    pub const MAX_ZOOM: f32 = 128.0;

    /// Pan by a screen-space delta (pixels). Internally divided by
    /// zoom so panning is consistent regardless of zoom level.
    pub fn pan_pixels(&mut self, dx: f32, dy: f32) {
        if self.zoom > 0.0 {
            self.center_tile_x -= dx / self.zoom;
            self.center_tile_y -= dy / self.zoom;
        }
    }

    /// Zoom by a multiplicative factor (e.g. 1.1 for one wheel notch).
    /// Result is clamped to `[MIN_ZOOM, MAX_ZOOM]`.
    pub fn zoom_by(&mut self, factor: f32) {
        if factor.is_finite() && factor > 0.0 {
            self.zoom = (self.zoom * factor).clamp(Self::MIN_ZOOM, Self::MAX_ZOOM);
        }
    }

    /// Zoom toward / away from a screen-space anchor (e.g. the cursor).
    /// The world position under the anchor stays under the anchor — the
    /// classic "zoom to cursor" UX. Canvas dimensions are required to
    /// resolve "centre of screen" to a tile coord.
    pub fn zoom_at(
        &mut self,
        factor: f32,
        anchor_x: f32,
        anchor_y: f32,
        canvas_w: u32,
        canvas_h: u32,
    ) {
        let world_anchor = self.screen_to_tile(anchor_x, anchor_y, canvas_w, canvas_h);
        self.zoom_by(factor);
        // Re-pan so `world_anchor` lands back under `anchor`.
        let new_anchor_screen =
            self.tile_to_screen(world_anchor.0, world_anchor.1, canvas_w, canvas_h);
        self.pan_pixels(
            anchor_x - new_anchor_screen.0,
            anchor_y - new_anchor_screen.1,
        );
    }

    /// Convert tile coords to screen-space pixels (origin top-left).
    pub fn tile_to_screen(
        &self,
        tile_x: f32,
        tile_y: f32,
        canvas_w: u32,
        canvas_h: u32,
    ) -> (f32, f32) {
        debug_assert!(self.zoom > 0.0, "Camera::zoom must be positive");
        let cw = canvas_w as f32;
        let ch = canvas_h as f32;
        (
            (tile_x - self.center_tile_x) * self.zoom + cw * 0.5,
            (tile_y - self.center_tile_y) * self.zoom + ch * 0.5,
        )
    }

    /// Inverse of [`Self::tile_to_screen`].
    pub fn screen_to_tile(&self, sx: f32, sy: f32, canvas_w: u32, canvas_h: u32) -> (f32, f32) {
        debug_assert!(self.zoom > 0.0, "Camera::zoom must be positive");
        let cw = canvas_w as f32;
        let ch = canvas_h as f32;
        (
            (sx - cw * 0.5) / self.zoom + self.center_tile_x,
            (sy - ch * 0.5) / self.zoom + self.center_tile_y,
        )
    }

    /// Tile range visible in the viewport, inclusive on the start and
    /// exclusive on the end. Result is clamped to `[0, max_w/h]`. Used
    /// to skip drawing tiles outside the visible window.
    ///
    /// Clamping is done in `f32` *before* the `as u32` cast: a very
    /// large positive `tx0` would otherwise saturate to `u32::MAX`
    /// (silently producing an empty `x0..x1` range with no draws), and
    /// a very negative `tx0` would saturate to 0 (silently drawing the
    /// world from the origin). Both are silent failure modes, so we
    /// fold both bounds into the float math first.
    pub fn visible_tile_range(
        &self,
        canvas_w: u32,
        canvas_h: u32,
        grid_w: u32,
        grid_h: u32,
    ) -> (u32, u32, u32, u32) {
        let (tx0, ty0) = self.screen_to_tile(0.0, 0.0, canvas_w, canvas_h);
        let (tx1, ty1) = self.screen_to_tile(canvas_w as f32, canvas_h as f32, canvas_w, canvas_h);
        let gw = grid_w as f32;
        let gh = grid_h as f32;
        let x0 = tx0.floor().clamp(0.0, gw) as u32;
        let y0 = ty0.floor().clamp(0.0, gh) as u32;
        let x1 = tx1.ceil().clamp(0.0, gw) as u32;
        let y1 = ty1.ceil().clamp(0.0, gh) as u32;
        (x0, y0, x1, y1)
    }
}

/// RGB tint per [`BiomeTag`]. Hand-tuned palette; will become
/// genome-derived in a future story.
///
/// `BiomeTag` is `#[non_exhaustive]`, so a wildcard arm covers any
/// future variant. New variants should be palette-mapped explicitly
/// before merging — the wildcard is a safety net, not a styling
/// shortcut.
#[must_use]
pub fn biome_tint(tag: BiomeTag) -> [u8; 3] {
    match tag {
        BiomeTag::Ocean => [30, 70, 140],
        BiomeTag::Forest => [40, 100, 50],
        BiomeTag::Plains => [140, 170, 80],
        BiomeTag::Desert => [220, 200, 120],
        BiomeTag::Mountain => [120, 110, 100],
        BiomeTag::Tundra => [220, 230, 240],
        // Future-variant fallback — magenta so it's loud in screenshots.
        _ => [255, 0, 255],
    }
}

// ---------------------------------------------------------------------------
// SDL backend
// ---------------------------------------------------------------------------

#[cfg(feature = "sdl")]
mod sdl_backend {
    use super::{biome_tint, Camera, CreatureGlyph};
    use beast_world::Archipelago;
    use sdl3::pixels::Color;
    use sdl3::rect::Rect;
    use sdl3::render::WindowCanvas;

    /// Draw the archipelago tiles visible through `camera` onto
    /// `canvas`. Off-screen tiles are skipped via
    /// [`Camera::visible_tile_range`].
    ///
    /// The renderer is functional / stateless — pass it the data, get
    /// drawing back. Future caching layers (texture pre-bake, dirty-rect
    /// tracking) layer on top without changing this signature.
    pub fn draw_archipelago(
        canvas: &mut WindowCanvas,
        camera: &Camera,
        archipelago: &Archipelago,
    ) -> Result<(), String> {
        let (cw, ch) = canvas.output_size().map_err(|e| e.to_string())?;
        let (x0, y0, x1, y1) =
            camera.visible_tile_range(cw, ch, archipelago.width, archipelago.height);
        // Inflate by 1 px to cover sub-pixel tile edges at fractional
        // zoom values (e.g. zoom = 16.7 leaves 0.7-px gaps between
        // adjacent tiles otherwise).
        let zoom_i = camera.zoom.ceil() as i32 + 1;

        // TODO(perf, follow-up of #201): drawing 64x64 tiles each frame
        // is up to 4096 `set_draw_color` + `fill_rect` calls. For real
        // 60 FPS targets, batch by biome (one `fill_rects(&[Rect])` per
        // palette slot) or pre-bake the tile grid into a single texture
        // refreshed only when the world changes. Profile-driven before
        // optimising — current usage is the smoke test.
        for ty in y0..y1 {
            for tx in x0..x1 {
                let Some(tag) = archipelago.get(tx, ty) else {
                    continue;
                };
                let [r, g, b] = biome_tint(tag);
                canvas.set_draw_color(Color::RGB(r, g, b));
                let (sx, sy) = camera.tile_to_screen(tx as f32, ty as f32, cw, ch);
                let rect = Rect::new(
                    sx.floor() as i32,
                    sy.floor() as i32,
                    zoom_i as u32,
                    zoom_i as u32,
                );
                canvas.fill_rect(rect).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    /// Draw a slice of creature glyphs as small filled circles
    /// (approximated by a 3-px rect for simplicity). The SDL backend
    /// will graduate to atlas-based glyphs once a placeholder PNG ships.
    pub fn draw_creature_glyphs(
        canvas: &mut WindowCanvas,
        camera: &Camera,
        glyphs: &[CreatureGlyph],
    ) -> Result<(), String> {
        let (cw, ch) = canvas.output_size().map_err(|e| e.to_string())?;
        // 0.4 of a tile, capped to 6 px so glyphs stay visible at low
        // zoom without dominating high-zoom views.
        let glyph_px = (camera.zoom * 0.4).clamp(3.0, 6.0).round() as u32;
        let half = (glyph_px as i32) / 2;

        // Cull to the visible tile range — same hygiene as
        // `draw_archipelago` so off-screen creatures don't
        // unconditionally cost a `set_draw_color` + `fill_rect` per
        // frame. Half a tile of slop on each side covers glyphs whose
        // centre is just past the viewport edge but whose body
        // extends in.
        let (tx0, ty0) = camera.screen_to_tile(0.0, 0.0, cw, ch);
        let (tx1, ty1) = camera.screen_to_tile(cw as f32, ch as f32, cw, ch);
        let (cull_x0, cull_y0) = (tx0 - 0.5, ty0 - 0.5);
        let (cull_x1, cull_y1) = (tx1 + 0.5, ty1 + 0.5);

        for g in glyphs {
            let gx = g.tile_x as f32;
            let gy = g.tile_y as f32;
            if gx < cull_x0 || gx > cull_x1 || gy < cull_y0 || gy > cull_y1 {
                continue;
            }
            let (sx, sy) = camera.tile_to_screen(gx + 0.5, gy + 0.5, cw, ch);
            let [r, gc, b] = g.species_tint;
            canvas.set_draw_color(Color::RGB(r, gc, b));
            let rect = Rect::new(
                sx.floor() as i32 - half,
                sy.floor() as i32 - half,
                glyph_px,
                glyph_px,
            );
            canvas.fill_rect(rect).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

#[cfg(feature = "sdl")]
pub use sdl_backend::{draw_archipelago, draw_creature_glyphs};

// ---------------------------------------------------------------------------
// Pure tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }

    #[test]
    fn default_camera_centres_origin() {
        let cam = Camera::default();
        assert_eq!(cam.center_tile_x, 0.0);
        assert_eq!(cam.center_tile_y, 0.0);
    }

    #[test]
    fn pan_pixels_inverts_zoom() {
        let mut cam = Camera {
            zoom: 16.0,
            ..Camera::default()
        };
        cam.pan_pixels(32.0, 0.0);
        // 32 px / 16 px/tile = 2 tiles; pan moves the *world*, so
        // center moves opposite the screen drag.
        assert!(approx(cam.center_tile_x, -2.0));
    }

    #[test]
    fn zoom_by_clamps_to_bounds() {
        let mut cam = Camera::default();
        cam.zoom_by(1000.0);
        assert!(approx(cam.zoom, Camera::MAX_ZOOM));
        cam.zoom_by(0.001);
        assert!(approx(cam.zoom, Camera::MIN_ZOOM));
    }

    #[test]
    fn zoom_by_rejects_non_finite_factor() {
        let mut cam = Camera::default();
        let before = cam.zoom;
        cam.zoom_by(f32::NAN);
        cam.zoom_by(f32::INFINITY);
        cam.zoom_by(-1.0);
        cam.zoom_by(0.0);
        assert!(approx(cam.zoom, before));
    }

    #[test]
    fn screen_to_tile_round_trip() {
        let cam = Camera {
            center_tile_x: 4.0,
            center_tile_y: 7.0,
            zoom: 16.0,
        };
        let (tx, ty) = cam.screen_to_tile(640.0, 360.0, 1280, 720);
        assert!(approx(tx, 4.0));
        assert!(approx(ty, 7.0));
        let (sx, sy) = cam.tile_to_screen(tx, ty, 1280, 720);
        assert!(approx(sx, 640.0));
        assert!(approx(sy, 360.0));
    }

    #[test]
    fn zoom_at_keeps_world_under_cursor() {
        let mut cam = Camera::default();
        let cw = 1280;
        let ch = 720;
        let cursor = (1000.0, 200.0);
        let world_before = cam.screen_to_tile(cursor.0, cursor.1, cw, ch);
        cam.zoom_at(2.0, cursor.0, cursor.1, cw, ch);
        let world_after = cam.screen_to_tile(cursor.0, cursor.1, cw, ch);
        assert!(approx(world_before.0, world_after.0));
        assert!(approx(world_before.1, world_after.1));
    }

    #[test]
    fn visible_tile_range_clips_to_grid() {
        let cam = Camera {
            center_tile_x: 32.0,
            center_tile_y: 32.0,
            zoom: 32.0,
        };
        // 1280x720 / 32 px/tile = 40x22.5 tile viewport, centred at (32, 32).
        let (x0, y0, x1, y1) = cam.visible_tile_range(1280, 720, 64, 64);
        assert!(x1 <= 64);
        assert!(y1 <= 64);
        assert!(x1 > x0 && y1 > y0);
        assert!(x0 < 64 && y0 < 64);
    }

    #[test]
    fn visible_tile_range_clips_at_world_edge() {
        // Camera off the world: range should clamp to [0,0] x grid.
        let cam = Camera {
            center_tile_x: -1000.0,
            center_tile_y: -1000.0,
            zoom: 16.0,
        };
        let (x0, y0, x1, y1) = cam.visible_tile_range(1280, 720, 64, 64);
        assert_eq!(x0, 0);
        assert_eq!(y0, 0);
        // Off to the negative side, x1 clamps to 0 too because tx1 < 0.
        assert_eq!(x1, 0);
        assert_eq!(y1, 0);
    }

    #[test]
    fn visible_tile_range_clips_at_positive_edge() {
        // Camera centred far past the world's positive edge. Without
        // the f32→u32 saturation guard, `tx0 as u32` would saturate to
        // u32::MAX and the loop `x0..x1` would silently produce nothing.
        // After the fix, both bounds clamp to `grid_w` / `grid_h`,
        // giving an empty (`x0 == x1`) range — distinguishable from
        // saturation by the equality.
        let cam = Camera {
            center_tile_x: 10_000.0,
            center_tile_y: 10_000.0,
            zoom: 16.0,
        };
        let (x0, y0, x1, y1) = cam.visible_tile_range(1280, 720, 64, 64);
        assert_eq!(x0, 64);
        assert_eq!(y0, 64);
        assert_eq!(x1, 64);
        assert_eq!(y1, 64);
    }

    #[test]
    fn pan_pixels_is_noop_when_zoom_is_zero() {
        // `Camera::zoom = 0.0` is constructible directly because all
        // fields are `pub`. `pan_pixels` guards the divide-by-zero;
        // this test pins that guard so a future refactor doesn't
        // remove it silently.
        let mut cam = Camera {
            center_tile_x: 4.0,
            center_tile_y: 9.0,
            zoom: 0.0,
        };
        cam.pan_pixels(100.0, -50.0);
        assert_eq!(cam.center_tile_x, 4.0);
        assert_eq!(cam.center_tile_y, 9.0);
    }

    #[test]
    fn biome_tint_is_deterministic_per_variant() {
        // Pin one expected colour per biome so a palette tweak shows
        // up in this PR's diff rather than as a silent visual change.
        assert_eq!(biome_tint(BiomeTag::Ocean), [30, 70, 140]);
        assert_eq!(biome_tint(BiomeTag::Forest), [40, 100, 50]);
        assert_eq!(biome_tint(BiomeTag::Plains), [140, 170, 80]);
        assert_eq!(biome_tint(BiomeTag::Desert), [220, 200, 120]);
        assert_eq!(biome_tint(BiomeTag::Mountain), [120, 110, 100]);
        assert_eq!(biome_tint(BiomeTag::Tundra), [220, 230, 240]);
    }

    #[test]
    fn creature_glyph_is_value_type() {
        let g = CreatureGlyph {
            tile_x: 1,
            tile_y: 2,
            species_tint: [255, 0, 0],
        };
        let h = g;
        assert_eq!(g, h);
    }
}
