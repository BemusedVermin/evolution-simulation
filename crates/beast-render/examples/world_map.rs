//! S9.3 smoke test: open a window, generate a 64×64 archipelago, scatter
//! some creature glyphs over it, and let the user pan/zoom around.
//!
//! Controls
//! --------
//!
//! * Arrow keys (or WASD) — pan
//! * Mouse wheel — zoom (centred on the cursor)
//! * Esc — quit
//!
//! Run with:
//!   cargo run -p beast-render --example world_map

#[cfg(feature = "sdl")]
use beast_core::Prng;
#[cfg(feature = "sdl")]
use beast_render::world_map::{draw_archipelago, draw_creature_glyphs};
#[cfg(feature = "sdl")]
use beast_render::{Camera, CreatureGlyph, Renderer, WindowConfig};
#[cfg(feature = "sdl")]
use beast_world::{generate_archipelago, Archipelago, WorldConfig};
#[cfg(feature = "sdl")]
use sdl3::{event::Event, keyboard::Keycode, mouse::MouseWheelDirection};

#[cfg(feature = "sdl")]
fn main() -> beast_render::Result<()> {
    use sdl3::pixels::Color;

    let world_seed: u64 = 0xBEA5_5697_u64;
    let archipelago = generate_archipelago(&WorldConfig::default(), world_seed)
        .map_err(|e| beast_render::RenderError::Sdl(format!("world gen failed: {e}")))?;
    let glyphs = scatter_glyphs(&archipelago, world_seed.wrapping_add(1), 200);

    let mut renderer = Renderer::new(WindowConfig {
        title: "beast-render: S9.3 world map".to_string(),
        ..Default::default()
    })?;

    let mut camera = Camera {
        center_tile_x: archipelago.width as f32 * 0.5,
        center_tile_y: archipelago.height as f32 * 0.5,
        zoom: 16.0,
    };

    let pan_step: f32 = 12.0;
    let mut last_mouse = (0_f32, 0_f32);

    'mainloop: loop {
        // Snapshot the canvas size before borrowing the event pump —
        // SDL's pump and canvas both borrow the renderer mutably, so
        // they can't be live at the same time.
        let (cw, ch) = renderer
            .canvas()
            .output_size()
            .map_err(|e| beast_render::RenderError::Sdl(e.to_string()))?;

        let events: Vec<Event> = renderer.event_pump().poll_iter().collect();
        for event in events {
            if !handle_event(event, &mut camera, &mut last_mouse, cw, ch) {
                break 'mainloop;
            }
        }

        let (dx, dy) = read_pan_input(&mut renderer, pan_step);
        if dx != 0.0 || dy != 0.0 {
            camera.pan_pixels(dx, dy);
        }

        let canvas = renderer.canvas();
        canvas.set_draw_color(Color::RGB(10, 10, 14));
        canvas.clear();
        draw_archipelago(canvas, &camera, &archipelago).map_err(beast_render::RenderError::Sdl)?;
        draw_creature_glyphs(canvas, &camera, &glyphs).map_err(beast_render::RenderError::Sdl)?;
        renderer.present();
    }
    Ok(())
}

/// Generate `count` deterministic glyphs scattered over the archipelago.
#[cfg(feature = "sdl")]
fn scatter_glyphs(archipelago: &Archipelago, seed: u64, count: usize) -> Vec<CreatureGlyph> {
    let mut rng = Prng::from_seed(seed);
    let mut glyphs = Vec::with_capacity(count);
    for _ in 0..count {
        let tile_x = (rng.next_u32() % archipelago.width) as i32;
        let tile_y = (rng.next_u32() % archipelago.height) as i32;
        let tint = [
            (rng.next_u32() & 0xFF) as u8,
            (rng.next_u32() & 0xFF) as u8,
            (rng.next_u32() & 0xFF) as u8,
        ];
        glyphs.push(CreatureGlyph {
            tile_x,
            tile_y,
            species_tint: tint,
        });
    }
    glyphs
}

/// Returns `false` to request shutdown of the main loop.
#[cfg(feature = "sdl")]
fn handle_event(
    event: Event,
    camera: &mut Camera,
    last_mouse: &mut (f32, f32),
    canvas_w: u32,
    canvas_h: u32,
) -> bool {
    match event {
        Event::Quit { .. }
        | Event::KeyDown {
            keycode: Some(Keycode::Escape),
            ..
        } => return false,
        Event::MouseMotion { x, y, .. } => *last_mouse = (x, y),
        Event::MouseWheel { y, direction, .. } => {
            let delta = if matches!(direction, MouseWheelDirection::Flipped) {
                -y
            } else {
                y
            };
            let factor = if delta > 0.0 { 1.1 } else { 1.0 / 1.1 };
            camera.zoom_at(factor, last_mouse.0, last_mouse.1, canvas_w, canvas_h);
        }
        _ => {}
    }
    true
}

/// Held-key pan via keyboard state. `KeyboardState` is a snapshot;
/// reading it once per frame gives smooth panning.
#[cfg(feature = "sdl")]
fn read_pan_input(renderer: &mut Renderer, pan_step: f32) -> (f32, f32) {
    use sdl3::keyboard::Scancode;
    let kbd = renderer.event_pump().keyboard_state();
    let mut dx = 0.0_f32;
    let mut dy = 0.0_f32;
    if kbd.is_scancode_pressed(Scancode::Left) || kbd.is_scancode_pressed(Scancode::A) {
        dx += pan_step;
    }
    if kbd.is_scancode_pressed(Scancode::Right) || kbd.is_scancode_pressed(Scancode::D) {
        dx -= pan_step;
    }
    if kbd.is_scancode_pressed(Scancode::Up) || kbd.is_scancode_pressed(Scancode::W) {
        dy += pan_step;
    }
    if kbd.is_scancode_pressed(Scancode::Down) || kbd.is_scancode_pressed(Scancode::S) {
        dy -= pan_step;
    }
    (dx, dy)
}

#[cfg(not(feature = "sdl"))]
fn main() {
    eprintln!("This example requires the `sdl` feature.");
    std::process::exit(2);
}
