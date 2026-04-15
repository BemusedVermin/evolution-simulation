//! Criterion benchmarks for `beast-core` hot paths.
//!
//! Run with `cargo bench -p beast-core`. These target the per-operation
//! microbenchmarks called out in Sprint S1's exit criteria: Q3232 multiply,
//! PRNG `next_u64`, and Gaussian sampling. Higher-level throughput
//! benchmarks live in `beast-cli` once the tick loop exists.

use beast_core::{gaussian_q3232, Prng, Q3232, Stream};
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};

fn bench_q3232_mul(c: &mut Criterion) {
    let mut group = c.benchmark_group("q3232");
    group.throughput(Throughput::Elements(1));
    let a = Q3232::from_num(1.337_f64);
    let b = Q3232::from_num(0.42_f64);
    group.bench_function("saturating_mul", |bench| {
        bench.iter(|| black_box(black_box(a).saturating_mul(black_box(b))));
    });
    group.bench_function("saturating_add", |bench| {
        bench.iter(|| black_box(black_box(a).saturating_add(black_box(b))));
    });
    group.bench_function("saturating_div", |bench| {
        bench.iter(|| black_box(black_box(a).saturating_div(black_box(b))));
    });
    group.finish();
}

fn bench_prng(c: &mut Criterion) {
    let mut group = c.benchmark_group("prng");
    group.throughput(Throughput::Elements(1));

    group.bench_function("next_u64", |bench| {
        let mut rng = Prng::from_seed(1);
        bench.iter(|| black_box(rng.next_u64()));
    });

    group.bench_function("next_q3232_unit", |bench| {
        let mut rng = Prng::from_seed(2);
        bench.iter(|| black_box(rng.next_q3232_unit()));
    });

    group.bench_function("split_stream", |bench| {
        let master = Prng::from_seed(3);
        bench.iter(|| black_box(master.split_stream(Stream::Genetics)));
    });

    group.finish();
}

fn bench_gaussian(c: &mut Criterion) {
    let mut group = c.benchmark_group("gaussian");
    group.throughput(Throughput::Elements(1));
    let mean = Q3232::ZERO;
    let sd = Q3232::ONE;
    group.bench_function("box_muller", |bench| {
        let mut rng = Prng::from_seed(4);
        bench.iter(|| black_box(gaussian_q3232(&mut rng, mean, sd)));
    });
    group.finish();
}

criterion_group!(benches, bench_q3232_mul, bench_prng, bench_gaussian);
criterion_main!(benches);
