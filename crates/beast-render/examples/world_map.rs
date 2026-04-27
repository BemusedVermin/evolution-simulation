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
fn main() -> beast_render::Result<()> {
    use beast_core::Prng;
    use beast_render::world_map::{draw_archipelago, draw_creature_glyphs};
    use beast_render::{Camera, CreatureGlyph, Renderer, WindowConfig};
    use beast_world::{generate_archipelago, WorldConfig};
    use sdl3::{event::Event, keyboard::Keycode, mouse::MouseWheelDirection, pixels::Color};

    // Generate a deterministic archipelago. Same seed → same world,
    // every run.
    let world_config = WorldConfig::default();
    let world_seed: u64 = 0xBEA5_5697_u64;
    let archipelago = generate_archipelago(&world_config, world_seed)
        .map_err(|e| beast_render::RenderError::Sdl(format!("world gen failed: {e}")))?;

    // Scatter 200 deterministic creature glyphs across the world.
    let mut rng = Prng::from_seed(world_seed.wrapping_add(1));
    let mut glyphs = Vec::with_capacity(200);
    for _ in 0..200 {
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

    let mut renderer = Renderer::new(WindowConfig {
        title: "beast-render: S9.3 world map".to_string(),
        ..Default::default()
    })?;

    let mut camera = Camera {
        center_tile_x: archipelago.width as f32 * 0.5,
        center_tile_y: archipelago.height as f32 * 0.5,
        zoom: 16.0,
    };

    // Pan speed in pixels per held-key tick. Held-key polling is done
    // every frame from the keyboard state, separate from the discrete
    // event stream.
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

        // Collect this frame's discrete events (Quit/Keys/Mouse) into
        // an owned Vec so we can release the event-pump borrow before
        // we start drawing.
        let events: Vec<Event> = renderer.event_pump().poll_iter().collect();
        for event in events {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'mainloop,
                Event::MouseMotion { x, y, .. } => {
                    last_mouse = (x, y);
                }
                Event::MouseWheel { y, direction, .. } => {
                    let mut delta = y;
                    if matches!(direction, MouseWheelDirection::Flipped) {
                        delta = -delta;
                    }
                    let factor = if delta > 0.0 { 1.1 } else { 1.0 / 1.1 };
                    camera.zoom_at(factor, last_mouse.0, last_mouse.1, cw, ch);
                }
                _ => {}
            }
        }

        // Held-key pan via keyboard state. `KeyboardState` is a
        // snapshot; reading it once per frame gives smooth panning.
        let (dx, dy) = {
            let kbd = renderer.event_pump().keyboard_state();
            let mut dx = 0.0_f32;
            let mut dy = 0.0_f32;
            if kbd.is_scancode_pressed(sdl3::keyboard::Scancode::Left)
                || kbd.is_scancode_pressed(sdl3::keyboard::Scancode::A)
            {
                dx += pan_step;
            }
            if kbd.is_scancode_pressed(sdl3::keyboard::Scancode::Right)
                || kbd.is_scancode_pressed(sdl3::keyboard::Scancode::D)
            {
                dx -= pan_step;
            }
            if kbd.is_scancode_pressed(sdl3::keyboard::Scancode::Up)
                || kbd.is_scancode_pressed(sdl3::keyboard::Scancode::W)
            {
                dy += pan_step;
            }
            if kbd.is_scancode_pressed(sdl3::keyboard::Scancode::Down)
                || kbd.is_scancode_pressed(sdl3::keyboard::Scancode::S)
            {
                dy -= pan_step;
            }
            (dx, dy)
        };
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

#[cfg(not(feature = "sdl"))]
fn main() {
    eprintln!("This example requires the `sdl` feature.");
    std::process::exit(2);
}
