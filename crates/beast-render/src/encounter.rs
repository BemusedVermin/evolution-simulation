//! Encounter-view renderer (S9.4).
//!
//! Draws a small 2.5D scene — typically the 5-creature encounter view —
//! by mapping the encounter's world-space y axis to a vertical screen
//! offset, scaling the x axis to 0.6 of the natural pixel ratio, and
//! drawing entities back-to-front so creatures further back are
//! occluded by closer ones.
//!
//! The module is split into a *pure* surface (projection math, depth
//! ordering, [`Position2D`], [`EncounterEntity`], [`Backdrop`]) that
//! compiles and tests without SDL, and an SDL-only backend that draws
//! silhouettes + a selection ring + a backdrop strip. CI exercises the
//! pure surface in headless mode; the SDL backend ships a smoke test
//! at `examples/encounter.rs`.
//!
//! See `documentation/systems/10_procgen_visual_pipeline.md` and the
//! issue body of #184 for the design contract.

use beast_world::BiomeTag;

use crate::blueprint::CreatureBlueprint;

// ---------------------------------------------------------------------------
// Pure types
// ---------------------------------------------------------------------------

/// Position in encounter-local world space.
///
/// `x` ∈ roughly `[-2, 2]` (typical encounter slot range).
/// `y` is the depth axis: `y > 0` is further from the camera (drawn
/// behind), `y < 0` is closer (drawn in front). Floats are fine — the
/// encounter view is purely render-side, never feeds back into sim
/// state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Position2D {
    pub x: f32,
    pub y: f32,
}

impl Position2D {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// One entity in the encounter view. Each carries a borrowed
/// blueprint so the renderer can read the bounding box for silhouette
/// sizing without owning the data.
#[derive(Debug, Clone, Copy)]
pub struct EncounterEntity<'a> {
    pub id: u32,
    pub blueprint: &'a CreatureBlueprint,
    pub position: Position2D,
    /// True for the active / selected entity; gets a ring underneath.
    pub selected: bool,
}

/// Backdrop description: biome tile colour + horizon line position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Backdrop {
    pub biome: BiomeTag,
    /// Horizon as a fraction of the canvas height, `[0, 1]`. 0.5 puts
    /// the horizon at the vertical centre. Defaults to 0.45 — a touch
    /// above centre so the ground takes more space than the sky.
    pub horizon_fraction: f32,
}

impl Backdrop {
    pub const fn new(biome: BiomeTag) -> Self {
        Self {
            biome,
            horizon_fraction: 0.45,
        }
    }
}

/// 2.5D projection parameters. Hand-tuned to match the design doc's
/// "2.5D perspective" idiom: y depth → vertical offset + slight x
/// compression so the scene reads as receding rather than orthographic.
#[derive(Debug, Clone, Copy)]
pub struct Projection {
    /// Pixels per world-space unit on x.
    pub px_per_unit_x: f32,
    /// Pixels per world-space unit on y (depth). Smaller than x so
    /// depth compresses.
    pub px_per_unit_y: f32,
    /// Horizontal compression at the back of the scene. 1.0 = no
    /// compression, 0.6 = back rows are 60 % the width of front rows.
    pub depth_x_scale: f32,
}

impl Default for Projection {
    fn default() -> Self {
        Self {
            px_per_unit_x: 120.0,
            px_per_unit_y: 80.0,
            depth_x_scale: 0.6,
        }
    }
}

impl Projection {
    /// Project an encounter world position to screen-space pixels
    /// (origin top-left). Canvas dims define the centre point.
    ///
    /// Math:
    /// * `screen_x = canvas_w/2 + x * px_per_unit_x * lerp(1, depth_x_scale, y_norm)`
    /// * `screen_y = ground_y - y * px_per_unit_y`  (where ground_y is
    ///   slightly below the horizon)
    ///
    /// `y_norm` = `(y - y_min) / (y_max - y_min)` clamped to `[0, 1]`,
    /// where `(y_min, y_max)` come from the entity batch's depth range
    /// — passed in as `depth_extents` so all entities scale
    /// consistently.
    pub fn project(
        &self,
        pos: Position2D,
        canvas_w: u32,
        canvas_h: u32,
        depth_extents: (f32, f32),
    ) -> (f32, f32) {
        let cw = canvas_w as f32;
        let ch = canvas_h as f32;
        let (y_min, y_max) = depth_extents;
        let y_norm = if y_max > y_min {
            ((pos.y - y_min) / (y_max - y_min)).clamp(0.0, 1.0)
        } else {
            0.5
        };
        // Lerp: front (y_norm = 0) keeps full x; back (y_norm = 1)
        // compresses to depth_x_scale.
        let x_scale = 1.0 - (1.0 - self.depth_x_scale) * y_norm;

        let ground_y = ch * 0.65;
        let screen_x = cw * 0.5 + pos.x * self.px_per_unit_x * x_scale;
        let screen_y = ground_y - pos.y * self.px_per_unit_y;
        (screen_x, screen_y)
    }
}

/// Sort an entity batch back-to-front (largest `y` first). Returns a
/// `Vec<usize>` of indices into the original slice; the renderer
/// iterates this to issue draw calls in the right order. Stable sort
/// — ties between identical `y` values keep their insertion order.
pub fn depth_order(entities: &[EncounterEntity<'_>]) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..entities.len()).collect();
    indices.sort_by(|&a, &b| {
        // partial_cmp is fine: encounter positions are render-only and
        // we control inputs. NaN lands at the front.
        entities[b]
            .position
            .y
            .partial_cmp(&entities[a].position.y)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    indices
}

/// Compute the depth range `(y_min, y_max)` of an entity batch.
/// Empty batch → `(0.0, 0.0)`.
pub fn depth_extents(entities: &[EncounterEntity<'_>]) -> (f32, f32) {
    let mut iter = entities.iter().map(|e| e.position.y);
    let Some(first) = iter.next() else {
        return (0.0, 0.0);
    };
    iter.fold((first, first), |(lo, hi), y| {
        (if y < lo { y } else { lo }, if y > hi { y } else { hi })
    })
}

/// Silhouette pixel size derived from a blueprint's bounding box.
///
/// Treats the blueprint's bbox as world units (1 unit ≈ 1 body length),
/// scales by the projection's x-pixel-per-unit (with depth
/// compression) for width, and by y-pixel-per-unit for height. Returns
/// (w_px, h_px) — the silhouette is drawn as a filled rect of this
/// size, centred on the projected position.
pub fn silhouette_size(
    blueprint: &CreatureBlueprint,
    projection: &Projection,
    depth_norm: f32,
) -> (u32, u32) {
    let bb = &blueprint.metadata.bounding_box;
    let world_w = (bb.max.x.to_num::<f32>() - bb.min.x.to_num::<f32>()).max(0.5);
    let world_h = (bb.max.y.to_num::<f32>() - bb.min.y.to_num::<f32>()).max(0.5);
    let x_scale = 1.0 - (1.0 - projection.depth_x_scale) * depth_norm;
    let w_px = (world_w * projection.px_per_unit_x * x_scale).clamp(8.0, 256.0);
    // Depth doesn't compress vertical extent — it shifts vertical
    // position. Height stays consistent so creatures at different
    // depths read as the same scale, just offset.
    let h_px = (world_h * projection.px_per_unit_y).clamp(8.0, 256.0);
    (w_px.round() as u32, h_px.round() as u32)
}

// ---------------------------------------------------------------------------
// SDL backend
// ---------------------------------------------------------------------------

#[cfg(feature = "sdl")]
mod sdl_backend {
    use super::{
        depth_extents, depth_order, silhouette_size, Backdrop, EncounterEntity, Projection,
    };
    use crate::world_map::biome_tint;
    use sdl3::pixels::Color;
    use sdl3::rect::Rect;
    use sdl3::render::WindowCanvas;

    /// Draw the backdrop: sky strip (slightly tinted neutral) + ground
    /// fill in biome colour, separated by the horizon line.
    pub fn draw_backdrop(canvas: &mut WindowCanvas, backdrop: &Backdrop) -> Result<(), String> {
        let (cw, ch) = canvas.output_size().map_err(|e| e.to_string())?;
        let horizon = (ch as f32 * backdrop.horizon_fraction).round() as i32;

        // Sky: cool, slightly desaturated.
        canvas.set_draw_color(Color::RGB(140, 160, 180));
        canvas
            .fill_rect(Rect::new(0, 0, cw, horizon.max(0) as u32))
            .map_err(|e| e.to_string())?;

        // Ground: biome tint, darkened a touch so creatures contrast.
        let [r, g, b] = biome_tint(backdrop.biome);
        canvas.set_draw_color(Color::RGB(
            (r as u16 * 7 / 10) as u8,
            (g as u16 * 7 / 10) as u8,
            (b as u16 * 7 / 10) as u8,
        ));
        let ground_h = (ch as i32 - horizon).max(0) as u32;
        canvas
            .fill_rect(Rect::new(0, horizon, cw, ground_h))
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Draw an entity batch back-to-front, with optional selection
    /// ring under the active creature(s).
    ///
    /// Silhouettes are filled rectangles sized by
    /// [`super::silhouette_size`]; the eventual procedural mesh path
    /// from S9.5 will replace the rect with a sprite or mesh draw
    /// without changing this signature.
    pub fn draw_encounter(
        canvas: &mut WindowCanvas,
        backdrop: &Backdrop,
        entities: &[EncounterEntity<'_>],
        projection: &Projection,
    ) -> Result<(), String> {
        draw_backdrop(canvas, backdrop)?;
        let (cw, ch) = canvas.output_size().map_err(|e| e.to_string())?;
        let extents = depth_extents(entities);
        let (y_min, y_max) = extents;

        for idx in depth_order(entities) {
            let entity = &entities[idx];
            let depth_norm = if y_max > y_min {
                ((entity.position.y - y_min) / (y_max - y_min)).clamp(0.0, 1.0)
            } else {
                0.5
            };
            let (sx, sy) = projection.project(entity.position, cw, ch, extents);
            let (w_px, h_px) = silhouette_size(entity.blueprint, projection, depth_norm);

            // Drop shadow: faint dark ellipse-ish rect under the feet.
            canvas.set_draw_color(Color::RGBA(0, 0, 0, 80));
            canvas
                .fill_rect(Rect::new(
                    sx as i32 - (w_px as i32) / 2,
                    sy as i32,
                    w_px,
                    (h_px / 6).max(2),
                ))
                .map_err(|e| e.to_string())?;

            // Selection ring (drawn before the silhouette so it sits
            // *behind* the creature's feet, reading as a marker on the
            // ground).
            if entity.selected {
                draw_selection_ring(canvas, sx as i32, sy as i32, w_px)?;
            }

            // Silhouette: simple filled rect for now. Tinted from the
            // blueprint's first global material colour if present;
            // otherwise mid-grey.
            let tint = silhouette_tint(entity.blueprint);
            canvas.set_draw_color(Color::RGB(tint[0], tint[1], tint[2]));
            canvas
                .fill_rect(Rect::new(
                    sx as i32 - (w_px as i32) / 2,
                    sy as i32 - h_px as i32,
                    w_px,
                    h_px,
                ))
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    fn draw_selection_ring(
        canvas: &mut WindowCanvas,
        cx: i32,
        feet_y: i32,
        width: u32,
    ) -> Result<(), String> {
        let ring_w = (width as f32 * 1.3) as u32;
        let ring_h = (width as f32 * 0.4).max(8.0) as u32;
        canvas.set_draw_color(Color::RGB(255, 220, 80));
        canvas
            .draw_rect(Rect::new(
                cx - ring_w as i32 / 2,
                feet_y - ring_h as i32 / 2,
                ring_w,
                ring_h,
            ))
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn silhouette_tint(blueprint: &crate::blueprint::CreatureBlueprint) -> [u8; 3] {
        // Pick the first global material's base colour; convert HSV
        // (Q3232) to RGB via a lossy approximation. For the smoke-test
        // silhouette this only needs to be visually distinguishable —
        // exact colour science waits for the procedural-mesh path.
        let global = blueprint.materials.iter().find(|m| {
            matches!(
                m.target,
                crate::blueprint::MaterialTarget::Global
                    | crate::blueprint::MaterialTarget::Volume { .. }
            )
        });
        let Some(mat) = global else {
            return [120, 120, 120];
        };
        // Map hue (0-360) to a coarse RGB by region. Saturation +
        // value live in [0,1].
        let hue: f32 = mat
            .props
            .base_color
            .hue
            .map(|h| h.to_num::<f32>())
            .unwrap_or(120.0);
        let s: f32 = mat
            .props
            .base_color
            .saturation
            .to_num::<f32>()
            .clamp(0.0, 1.0);
        let v: f32 = mat.props.base_color.value.to_num::<f32>().clamp(0.0, 1.0);
        hsv_to_rgb(hue, s, v)
    }

    fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [u8; 3] {
        // Standard HSV→RGB; lifted here so the SDL backend doesn't pull
        // in a colour crate. h in degrees, s/v in [0, 1].
        let h = h.rem_euclid(360.0);
        let c = v * s;
        let h_section = h / 60.0;
        let x = c * (1.0 - (h_section.rem_euclid(2.0) - 1.0).abs());
        let (r1, g1, b1) = match h_section as i32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        let m = v - c;
        [
            ((r1 + m) * 255.0).clamp(0.0, 255.0) as u8,
            ((g1 + m) * 255.0).clamp(0.0, 255.0) as u8,
            ((b1 + m) * 255.0).clamp(0.0, 255.0) as u8,
        ]
    }
}

#[cfg(feature = "sdl")]
pub use sdl_backend::{draw_backdrop, draw_encounter};

// ---------------------------------------------------------------------------
// Pure tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::{AnimationClip, AnimationSet};
    use crate::blueprint::{Aabb, BlueprintMetadata, BoneTree, Vec3};
    use beast_core::Q3232;

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }

    fn dummy_blueprint() -> CreatureBlueprint {
        CreatureBlueprint {
            skeleton: BoneTree { bones: Vec::new() },
            volumes: Vec::new(),
            surfaces: Vec::new(),
            materials: Vec::new(),
            effects: Vec::new(),
            animations: AnimationSet {
                locomotion: Vec::new(),
                idle: Vec::new(),
                damage: AnimationClip {
                    name: "damage".to_string(),
                    duration: Q3232::ONE,
                    looping: false,
                    bone_tracks: Vec::new(),
                },
                death: AnimationClip {
                    name: "death".to_string(),
                    duration: Q3232::ONE,
                    looping: false,
                    bone_tracks: Vec::new(),
                },
            },
            metadata: BlueprintMetadata {
                bounding_box: Aabb {
                    min: Vec3::ZERO,
                    max: Vec3::new(Q3232::from_num(1), Q3232::from_num(1), Q3232::ZERO),
                },
                display_name: "test".to_string(),
            },
        }
    }

    #[test]
    fn projection_centres_origin() {
        let proj = Projection::default();
        let (sx, sy) = proj.project(Position2D::new(0.0, 0.0), 1280, 720, (-1.0, 1.0));
        assert!(approx(sx, 640.0));
        // ground_y = 720 * 0.65 = 468; y=0 puts us right on it.
        assert!(approx(sy, 468.0));
    }

    #[test]
    fn projection_depth_offsets_vertically() {
        let proj = Projection::default();
        let front = proj.project(Position2D::new(0.0, -0.5), 1280, 720, (-0.5, 0.5));
        let back = proj.project(Position2D::new(0.0, 0.5), 1280, 720, (-0.5, 0.5));
        // Larger y → drawn higher up the screen (smaller screen y).
        assert!(back.1 < front.1);
    }

    #[test]
    fn projection_depth_compresses_x() {
        let proj = Projection::default();
        let front_right = proj.project(Position2D::new(1.0, -0.5), 1280, 720, (-0.5, 0.5));
        let back_right = proj.project(Position2D::new(1.0, 0.5), 1280, 720, (-0.5, 0.5));
        // Same x, but back row should be closer to centre by depth_x_scale.
        let centre = 640.0;
        let front_offset = front_right.0 - centre;
        let back_offset = back_right.0 - centre;
        assert!(back_offset.abs() < front_offset.abs());
        // 0.6 compression at the back: ratio should be ~0.6.
        let ratio = back_offset / front_offset;
        assert!(ratio > 0.55 && ratio < 0.65, "depth-x ratio {ratio}");
    }

    #[test]
    fn projection_handles_zero_depth_extent() {
        // All entities at the same y → y_norm defaults to 0.5,
        // projection still produces a finite position.
        let proj = Projection::default();
        let (sx, sy) = proj.project(Position2D::new(0.5, 0.0), 1280, 720, (0.0, 0.0));
        assert!(sx.is_finite());
        assert!(sy.is_finite());
    }

    fn entity(id: u32, x: f32, y: f32, bp: &CreatureBlueprint) -> EncounterEntity<'_> {
        EncounterEntity {
            id,
            blueprint: bp,
            position: Position2D::new(x, y),
            selected: false,
        }
    }

    #[test]
    fn depth_order_sorts_back_to_front() {
        let bp = dummy_blueprint();
        let entities = [
            entity(0, 0.0, -0.5, &bp), // closest
            entity(1, 0.0, 0.5, &bp),  // furthest
            entity(2, 0.0, 0.0, &bp),  // middle
        ];
        let order = depth_order(&entities);
        // Index 1 (y=0.5) first, then 2 (y=0.0), then 0 (y=-0.5).
        assert_eq!(order, vec![1, 2, 0]);
    }

    #[test]
    fn depth_order_is_stable_for_ties() {
        let bp = dummy_blueprint();
        let entities = [
            entity(0, 0.0, 0.0, &bp),
            entity(1, 0.0, 0.0, &bp),
            entity(2, 0.0, 0.0, &bp),
        ];
        let order = depth_order(&entities);
        // Ties → insertion order preserved.
        assert_eq!(order, vec![0, 1, 2]);
    }

    #[test]
    fn depth_extents_handles_empty_batch() {
        let extents: (f32, f32) = depth_extents(&[]);
        assert_eq!(extents, (0.0, 0.0));
    }

    #[test]
    fn depth_extents_finds_min_max() {
        let bp = dummy_blueprint();
        let entities = [
            entity(0, 0.0, -1.5, &bp),
            entity(1, 0.0, 0.7, &bp),
            entity(2, 0.0, -0.2, &bp),
        ];
        let (lo, hi) = depth_extents(&entities);
        assert!(approx(lo, -1.5));
        assert!(approx(hi, 0.7));
    }

    #[test]
    fn silhouette_size_clamps_to_min_when_blueprint_is_tiny() {
        // A blueprint with a degenerate bbox should still produce a
        // visible silhouette; test pins the floor at 8 px.
        let bp = dummy_blueprint();
        let proj = Projection::default();
        let (w, h) = silhouette_size(&bp, &proj, 0.0);
        assert!(w >= 8 && h >= 8);
    }

    #[test]
    fn silhouette_size_compresses_with_depth() {
        let bp = dummy_blueprint();
        let proj = Projection::default();
        let (w_front, _) = silhouette_size(&bp, &proj, 0.0);
        let (w_back, _) = silhouette_size(&bp, &proj, 1.0);
        // Back row narrower than front row by depth_x_scale.
        assert!(w_back < w_front, "{w_back} >= {w_front}");
    }

    #[test]
    fn backdrop_default_horizon_is_above_centre() {
        let bd = Backdrop::new(BiomeTag::Forest);
        assert_eq!(bd.biome, BiomeTag::Forest);
        assert!(bd.horizon_fraction > 0.0 && bd.horizon_fraction < 0.5);
    }
}
