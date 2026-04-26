//! S9.1 smoke test: open a window, clear it to a constant color, exit on
//! Quit / Escape. Demonstrates that the SDL3 backend links and that the
//! event loop is reachable from `Renderer`.
//!
//! Run with:
//!   cargo run -p beast-render --example window
//!
//! Skipped on headless builds (binary is unbuildable without the `sdl`
//! feature; we explicitly require it via `required-features` so cargo
//! refuses rather than silently producing a no-op binary).

#[cfg(feature = "sdl")]
fn main() -> beast_render::Result<()> {
    use beast_render::{Renderer, WindowConfig};
    use sdl3::{event::Event, keyboard::Keycode, pixels::Color};

    let mut renderer = Renderer::new(WindowConfig {
        title: "beast-render: S9.1 smoke test".to_string(),
        ..Default::default()
    })?;

    'mainloop: loop {
        for event in renderer.event_pump().poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'mainloop,
                _ => {}
            }
        }

        let canvas = renderer.canvas();
        canvas.set_draw_color(Color::RGB(20, 24, 32));
        canvas.clear();
        renderer.present();
    }

    Ok(())
}

#[cfg(not(feature = "sdl"))]
fn main() {
    eprintln!("This example requires the `sdl` feature.");
    std::process::exit(2);
}
