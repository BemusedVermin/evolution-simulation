//! Two independent `Chronicler` instances built from the same snapshot
//! sequence must produce byte-identical pattern indexes.
//!
//! This is the headline determinism gate for S10.5 (issue #210). It runs
//! against a 1000-entity, 100-tick scenario so it exercises the same
//! ingestion volume the design doc cares about.

use std::collections::BTreeSet;

use beast_chronicler::{Chronicler, PatternSignature, PrimitiveSnapshot};
use beast_core::{EntityId, TickCounter};

fn build_snapshots() -> Vec<PrimitiveSnapshot> {
    // A spread of 5 distinct primitive sets across 1000 entities and 100
    // ticks. The per-entity assignment is a stable function of (entity %
    // 5), so the same entity always emits the same primitive set, but
    // different entities see different sets — giving the chronicler a
    // realistic mix of unique signatures and repeated observations.
    let palettes: [&[&str]; 5] = [
        &[
            "emit_acoustic_pulse",
            "receive_acoustic_signal",
            "spatial_integrate",
        ],
        &["apply_bite_force", "inject_substance"],
        &["emit_acoustic_pulse"],
        &[],
        &["form_host_attachment", "spatial_integrate"],
    ];
    let mut out = Vec::with_capacity(1000 * 100);
    for tick in 0..100u64 {
        for entity in 0..1000u32 {
            let palette = palettes[(entity as usize) % palettes.len()];
            out.push(PrimitiveSnapshot {
                tick: TickCounter::new(tick),
                entity: EntityId::new(entity),
                primitives: palette.iter().map(|s| (*s).to_owned()).collect(),
            });
        }
    }
    out
}

fn signatures(c: &Chronicler) -> Vec<(PatternSignature, u64, BTreeSet<String>)> {
    c.observations()
        .values()
        .map(|o| (o.signature, o.count, o.primitives.clone()))
        .collect()
}

#[test]
fn two_chroniclers_agree_under_same_ingestion() {
    let snaps = build_snapshots();
    let mut a = Chronicler::new();
    let mut b = Chronicler::new();
    for s in &snaps {
        a.ingest(s);
    }
    for s in &snaps {
        b.ingest(s);
    }
    assert_eq!(signatures(&a), signatures(&b));
}

#[test]
#[ignore = "perf claim: run with `cargo test -p beast-chronicler --release -- --ignored`"]
fn ingest_1000_entities_100_ticks_under_50ms() {
    // The S10.5 issue (#210) requires `Chronicler::ingest` to handle
    // 1000 entities × 100 ticks in under 50 ms in release. This test
    // is `#[ignore]` because debug builds blow that budget by a wide
    // margin — gate it with `--release --ignored` in CI when we want
    // to enforce the perf floor.
    let snaps = build_snapshots();
    let mut c = Chronicler::new();
    let start = std::time::Instant::now();
    for s in &snaps {
        c.ingest(s);
    }
    let elapsed = start.elapsed();
    println!("ingested {} snapshots in {:?}", snaps.len(), elapsed);
    assert!(
        elapsed < std::time::Duration::from_millis(50),
        "ingest of {} snapshots took {:?}, exceeds 50ms budget",
        snaps.len(),
        elapsed,
    );
}

#[test]
fn ingestion_order_does_not_affect_final_index() {
    // Same snapshot multiset, different ingestion order. The
    // chronicler's signature index must converge to the same state.
    // (Per-observation `last_tick` *can* differ if reorder changes the
    // last tick of a signature, so we compare on (signature, count,
    // primitives) only — that's the load-bearing claim for S10.5.)
    let snaps = build_snapshots();
    let mut forward = Chronicler::new();
    let mut backward = Chronicler::new();
    for s in &snaps {
        forward.ingest(s);
    }
    for s in snaps.iter().rev() {
        backward.ingest(s);
    }
    let f: Vec<_> = signatures(&forward);
    let b: Vec<_> = signatures(&backward);
    assert_eq!(f, b);
}
