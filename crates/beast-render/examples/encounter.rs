//! S9.4 smoke test: open a window, build a 5-creature encounter scene
//! against a forest backdrop, and let the user cycle the selected
//! creature with Tab.
//!
//! Controls
//! --------
//!
//! * Tab       — advance the selected creature
//! * Shift+Tab — go back
//! * Esc       — quit
//!
//! Run with:
//!   cargo run -p beast-render --example encounter

#[cfg(feature = "sdl")]
fn main() -> beast_render::Result<()> {
    use std::collections::BTreeMap;

    use beast_core::Q3232;
    use beast_interpreter::{LifeStage, ResolvedPhenotype};
    use beast_render::encounter::{
        draw_encounter, Backdrop, EncounterEntity, Position2D, Projection,
    };
    use beast_render::{compile_blueprint, directive::ColorSpec, Renderer, WindowConfig};
    use beast_world::BiomeTag;
    use sdl3::{event::Event, keyboard::Keycode};

    // Five hand-tuned phenotypes so each creature in the scene reads
    // visibly different. Channel values map roughly to body-plan
    // archetypes — elastic worm, rigid armor, tall sprinter, etc.
    let phenotype_specs: [&[(&str, f64)]; 5] = [
        // 0: elastic worm
        &[
            ("elastic_deformation", 0.85),
            ("structural_rigidity", 0.05),
            ("mass_density", 0.3),
            ("metabolic_rate", 0.6),
        ],
        // 1: rigid armored
        &[
            ("elastic_deformation", 0.05),
            ("structural_rigidity", 0.85),
            ("mass_density", 0.7),
            ("metabolic_rate", 0.3),
        ],
        // 2: quadruped sprinter
        &[
            ("elastic_deformation", 0.2),
            ("structural_rigidity", 0.3),
            ("mass_density", 0.5),
            ("metabolic_rate", 0.7),
            ("kinetic_force", 0.7),
            ("surface_friction", 0.7),
        ],
        // 3: bioluminescent
        &[
            ("elastic_deformation", 0.3),
            ("structural_rigidity", 0.4),
            ("mass_density", 0.4),
            ("metabolic_rate", 0.5),
            ("light_emission", 0.8),
        ],
        // 4: thermal predator
        &[
            ("elastic_deformation", 0.2),
            ("structural_rigidity", 0.4),
            ("mass_density", 0.6),
            ("metabolic_rate", 0.6),
            ("thermal_output", 0.7),
            ("kinetic_force", 0.6),
        ],
    ];

    let neutral_biome = ColorSpec::rgb(
        Q3232::from_num(120),
        Q3232::from_num(0.4_f64),
        Q3232::from_num(0.5_f64),
    );

    let blueprints: Vec<_> = phenotype_specs
        .iter()
        .enumerate()
        .map(|(i, channels)| {
            let mut p = ResolvedPhenotype::new(Q3232::from_num(50_i32), LifeStage::Adult);
            p.global_channels = channels
                .iter()
                .map(|(k, v)| (k.to_string(), Q3232::from_num(*v)))
                .collect::<BTreeMap<_, _>>();
            compile_blueprint(&p, &[], neutral_biome, format!("creature_{i}"))
        })
        .collect();

    // Position 5 creatures in two rows: 3 in front, 2 in back.
    let positions: [Position2D; 5] = [
        Position2D::new(-1.4, -0.4), // front-left
        Position2D::new(0.0, -0.6),  // front-centre
        Position2D::new(1.4, -0.4),  // front-right
        Position2D::new(-0.7, 0.6),  // back-left
        Position2D::new(0.7, 0.6),   // back-right
    ];

    let mut renderer = Renderer::new(WindowConfig {
        title: "beast-render: S9.4 encounter".to_string(),
        ..Default::default()
    })?;

    let backdrop = Backdrop::new(BiomeTag::Forest);
    let projection = Projection::default();
    let mut selected = 0_usize;

    'mainloop: loop {
        // Snapshot canvas dims + collect events while no canvas borrow
        // is live (same idiom as world_map example).
        let events: Vec<Event> = renderer.event_pump().poll_iter().collect();
        for event in events {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'mainloop,
                Event::KeyDown {
                    keycode: Some(Keycode::Tab),
                    keymod,
                    ..
                } => {
                    let len = blueprints.len();
                    if keymod.contains(sdl3::keyboard::Mod::LSHIFTMOD)
                        || keymod.contains(sdl3::keyboard::Mod::RSHIFTMOD)
                    {
                        selected = (selected + len - 1) % len;
                    } else {
                        selected = (selected + 1) % len;
                    }
                }
                _ => {}
            }
        }

        let entities: Vec<EncounterEntity<'_>> = blueprints
            .iter()
            .zip(positions.iter())
            .enumerate()
            .map(|(i, (bp, pos))| EncounterEntity {
                id: i as u32,
                blueprint: bp,
                position: *pos,
                selected: i == selected,
            })
            .collect();

        let canvas = renderer.canvas();
        canvas.set_draw_color(sdl3::pixels::Color::RGB(0, 0, 0));
        canvas.clear();
        draw_encounter(canvas, &backdrop, &entities, &projection)
            .map_err(beast_render::RenderError::Sdl)?;
        renderer.present();
    }
    Ok(())
}

#[cfg(not(feature = "sdl"))]
fn main() {
    eprintln!("This example requires the `sdl` feature.");
    std::process::exit(2);
}
