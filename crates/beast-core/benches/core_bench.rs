//! Criterion benchmarks for `beast-core`. Populated in Story 1.6.

use criterion::{criterion_group, criterion_main, Criterion};

fn placeholder(c: &mut Criterion) {
    c.bench_function("placeholder", |b| b.iter(|| 1_u64.wrapping_add(1)));
}

criterion_group!(benches, placeholder);
criterion_main!(benches);
