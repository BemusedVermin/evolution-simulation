//! S9.6 smoke test: render one creature looping its walk cycle as a
//! stick-figure overlay.
//!
//! Bones are drawn as lines — for each bone we project its rest pose
//! into 2D screen space, then add the per-tick rotation delta from
//! [`beast_render::animation::Animator::sample`]. This is the
//! minimum-viable visualiser: enough to confirm the rig is alive,
//! deferring "real" mesh rendering to S9.4.
//!
//! Run with:
//!   cargo run -p beast-render --example animation

#[cfg(feature = "sdl")]
use beast_core::Q3232;
#[cfg(feature = "sdl")]
use beast_interpreter::{LifeStage, ResolvedPhenotype};
#[cfg(feature = "sdl")]
use beast_render::blueprint::CreatureBlueprint;
#[cfg(feature = "sdl")]
use beast_render::{compile_blueprint, directive::ColorSpec, Animator, Renderer, WindowConfig};
#[cfg(feature = "sdl")]
use sdl3::{event::Event, keyboard::Keycode, pixels::Color, render::WindowCanvas};

#[cfg(feature = "sdl")]
fn main() -> beast_render::Result<()> {
    let blueprint = build_walker_blueprint();
    let walk_clip = blueprint
        .animations
        .locomotion
        .first()
        .ok_or_else(|| beast_render::RenderError::Sdl("no walk clip".into()))?;
    let animator = Animator::new(walk_clip);

    let mut renderer = Renderer::new(WindowConfig {
        title: "beast-render: S9.6 animation demo".to_string(),
        ..Default::default()
    })?;

    let start = std::time::Instant::now();
    'mainloop: loop {
        for event in renderer.event_pump().poll_iter() {
            if matches!(
                event,
                Event::Quit { .. }
                    | Event::KeyDown {
                        keycode: Some(Keycode::Escape),
                        ..
                    }
            ) {
                break 'mainloop;
            }
        }

        let elapsed = start.elapsed().as_secs_f32();
        let pose = animator.sample(Q3232::from_num(elapsed));

        let canvas = renderer.canvas();
        canvas.set_draw_color(Color::RGB(20, 24, 32));
        canvas.clear();
        draw_skeleton(canvas, &blueprint, &pose)?;
        renderer.present();
    }

    Ok(())
}

/// Build a "quadruped walker" phenotype so we get a 4-limb skeleton and
/// the QuadrupedWalk locomotion style.
#[cfg(feature = "sdl")]
fn build_walker_blueprint() -> CreatureBlueprint {
    use std::collections::BTreeMap;

    let mut phenotype = ResolvedPhenotype::new(Q3232::from_num(50_i32), LifeStage::Adult);
    let channels: BTreeMap<String, Q3232> = [
        ("elastic_deformation", 0.2_f64),
        ("structural_rigidity", 0.2),
        ("mass_density", 0.5),
        ("metabolic_rate", 0.6),
        ("surface_friction", 0.9),
        ("kinetic_force", 0.7),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), Q3232::from_num(v)))
    .collect();
    phenotype.global_channels = channels;

    compile_blueprint(
        &phenotype,
        &[],
        ColorSpec::rgb(
            Q3232::from_num(120),
            Q3232::from_num(0.4_f64),
            Q3232::from_num(0.5_f64),
        ),
        "demo",
    )
}

/// Draw each bone as a line. Origin is the screen centre; each bone-id
/// maps to a horizontal slot so motion is visible.
#[cfg(feature = "sdl")]
fn draw_skeleton(
    canvas: &mut WindowCanvas,
    blueprint: &CreatureBlueprint,
    pose: &beast_render::animation::PoseFrame,
) -> beast_render::Result<()> {
    canvas.set_draw_color(Color::RGB(220, 220, 240));
    let cx: i32 = 640;
    let cy: i32 = 360;
    let bone_count = blueprint.skeleton.bones.len() as i32;
    for (i, bone) in blueprint.skeleton.bones.iter().enumerate() {
        let rotation_deg = pose
            .bone_rotations
            .iter()
            .find(|r| r.bone_id == bone.id)
            .map(|r| r.rotation.to_num::<f32>())
            .unwrap_or(0.0);
        let length: f32 = bone.length.to_num::<f32>() * 60.0;
        let theta = rotation_deg.to_radians();
        let (sin, cos) = theta.sin_cos();
        let x0 = cx + (i as i32 * 40) - (bone_count * 20);
        let y0 = cy;
        let x1 = x0 + (length * cos) as i32;
        let y1 = y0 + (length * sin) as i32;
        canvas
            .draw_line((x0, y0), (x1, y1))
            .map_err(|e| beast_render::RenderError::Sdl(e.to_string()))?;
    }
    Ok(())
}

#[cfg(not(feature = "sdl"))]
fn main() {
    eprintln!("This example requires the `sdl` feature.");
    std::process::exit(2);
}
