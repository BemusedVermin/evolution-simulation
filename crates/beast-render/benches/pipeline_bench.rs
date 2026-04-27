//! Criterion benchmarks for the visual pipeline + animation rig.
//!
//! Run with `cargo bench -p beast-render --no-default-features --features headless`.
//!
//! These cover the *headless* surface — the pure-Rust compile +
//! sampling work that runs offline of SDL3. Render-loop benchmarks
//! (world-map / encounter scenes) land alongside the renderers in
//! S9.3 / S9.4 and reuse the harness conventions established here.
//!
//! # Pass criteria (S9.8)
//!
//! * `bench_compile_blueprint` — < 1 ms per creature on the dev box
//!   (uncached — same-genotype caching lands when the renderers do).
//! * `bench_rig_animations` — sub-stage 6 isolated, used to attribute
//!   pipeline cost to rigging vs the rest.
//! * `bench_animator_sample` — per-frame cost; should be << 100 µs at
//!   200 creatures × 60 FPS = 12k samples/sec.

use std::collections::BTreeMap;
use std::hint::black_box;

use beast_core::{BodySite, Prng, Stream, Q3232};
use beast_interpreter::{LifeStage, ResolvedPhenotype};
use beast_render::animation::{rig_animations, Animator};
use beast_render::directive::{
    ColorSpec, Colorize, DirectiveParams, Distribution, Inflate, Protrude, ProtrusionShape,
    SurfaceRegion, TexturePattern, VisualDirective,
};
use beast_render::{compile_blueprint, CreatureBlueprint};
use criterion::{criterion_group, criterion_main, BatchSize, Criterion, Throughput};

const SEED: u64 = 0xBEA5_BEAD_u64;
const CHANNEL_IDS: &[&str] = &[
    "elastic_deformation",
    "structural_rigidity",
    "mass_density",
    "metabolic_rate",
    "surface_friction",
    "kinetic_force",
    "light_emission",
    "chemical_output",
    "thermal_output",
];

fn random_phenotype(rng: &mut Prng) -> ResolvedPhenotype {
    let mut p = ResolvedPhenotype::new(Q3232::from_num(50_i32), LifeStage::Adult);
    let mut channels = BTreeMap::new();
    for id in CHANNEL_IDS {
        channels.insert((*id).to_string(), rng.next_q3232_unit());
    }
    p.global_channels = channels;
    p
}

fn typical_directives() -> Vec<VisualDirective> {
    // Representative-sized directive set: 4 directives covering Inflate,
    // Colorize (with biome-color sentinel), Protrude, and a Crest
    // appendage. Doesn't go through every variant; for variant
    // coverage see `random_creatures_test.rs`.
    vec![
        VisualDirective {
            id: 1,
            body_region: BodySite::Core,
            params: DirectiveParams::Inflate(Inflate {
                scale: Q3232::from_num(1.1_f64),
            }),
            priority: 5,
        },
        VisualDirective {
            id: 2,
            body_region: BodySite::Core,
            params: DirectiveParams::Colorize(Colorize {
                base_color: ColorSpec::biome_color(
                    Q3232::from_num(0.6_f64),
                    Q3232::from_num(0.5_f64),
                ),
                emission: None,
                emission_intensity: Q3232::ZERO,
                pattern: Some(TexturePattern::Mottled),
                pattern_color_secondary: None,
                contrast: Q3232::from_num(0.4_f64),
            }),
            priority: 3,
        },
        VisualDirective {
            id: 3,
            body_region: BodySite::Core,
            params: DirectiveParams::Protrude(Protrude {
                shape: ProtrusionShape::Spike,
                scale: Q3232::from_num(0.4_f64),
                density: 6,
                distribution: Distribution::Regular,
                surface_region: SurfaceRegion::Dorsal,
            }),
            priority: 8,
        },
        VisualDirective {
            id: 4,
            body_region: BodySite::Tail,
            params: DirectiveParams::Inflate(Inflate {
                scale: Q3232::from_num(0.9_f64),
            }),
            priority: 2,
        },
    ]
}

fn neutral_biome() -> ColorSpec {
    ColorSpec::rgb(
        Q3232::from_num(120_i32),
        Q3232::from_num(0.4_f64),
        Q3232::from_num(0.5_f64),
    )
}

// ---------------------------------------------------------------------------
// Benchmarks
// ---------------------------------------------------------------------------

fn bench_compile_blueprint(c: &mut Criterion) {
    let mut group = c.benchmark_group("compile_blueprint");
    group.throughput(Throughput::Elements(1));
    let mut rng = Prng::from_seed(SEED).split_stream(Stream::Testing);
    let directives = typical_directives();

    // Reuse one phenotype: the bench measures pipeline-only cost, so
    // we don't want phenotype generation showing up in the numbers.
    let phenotype = random_phenotype(&mut rng);
    // `black_box` is only useful at the optimisation boundary —
    // wrapping `neutral_biome()` (a pure non-allocating call already
    // inside the iter closure) and the `"bench"` literal would be
    // no-ops, so they're not wrapped.
    group.bench_function("typical_phenotype", |b| {
        b.iter(|| {
            let bp: CreatureBlueprint = compile_blueprint(
                black_box(&phenotype),
                black_box(&directives),
                neutral_biome(),
                "bench",
            );
            black_box(bp)
        });
    });

    // Random phenotype each iter — exercises the channel-driven
    // branching across the pipeline. `BatchSize::SmallInput` keeps
    // setup overhead bounded relative to the measurement window.
    group.bench_function("random_phenotype", |b| {
        let mut rng = Prng::from_seed(SEED).split_stream(Stream::Testing);
        b.iter_batched(
            || random_phenotype(&mut rng),
            |phenotype| {
                let bp =
                    compile_blueprint(&phenotype, black_box(&directives), neutral_biome(), "bench");
                black_box(bp)
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

fn bench_rig_animations(c: &mut Criterion) {
    let mut group = c.benchmark_group("rig_animations");
    group.throughput(Throughput::Elements(1));
    let mut rng = Prng::from_seed(SEED).split_stream(Stream::Testing);
    let phenotype = random_phenotype(&mut rng);
    let blueprint = compile_blueprint(&phenotype, &typical_directives(), neutral_biome(), "bench");

    group.bench_function("typical", |b| {
        b.iter(|| {
            let set = rig_animations(black_box(&blueprint.skeleton), black_box(&phenotype));
            black_box(set)
        });
    });
    group.finish();
}

fn bench_animator_sample(c: &mut Criterion) {
    let mut group = c.benchmark_group("animator_sample");
    group.throughput(Throughput::Elements(1));
    let mut rng = Prng::from_seed(SEED).split_stream(Stream::Testing);
    let phenotype = random_phenotype(&mut rng);
    let blueprint = compile_blueprint(&phenotype, &typical_directives(), neutral_biome(), "bench");
    let walk_clip = blueprint
        .animations
        .locomotion
        .first()
        .expect("walk clip exists");
    let animator = Animator::new(walk_clip);

    // Sample at clip mid-time so the keyframe-pair lookup actually
    // runs the lerp branch, not the early-return endpoint case.
    let mid_t = walk_clip.duration.saturating_mul(Q3232::from_num(0.5_f64));

    group.bench_function("walk_mid_t", |b| {
        b.iter(|| black_box(animator.sample(black_box(mid_t))));
    });
    group.bench_function("walk_t_zero", |b| {
        b.iter(|| black_box(animator.sample(Q3232::ZERO)));
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_compile_blueprint,
    bench_rig_animations,
    bench_animator_sample
);
criterion_main!(benches);
