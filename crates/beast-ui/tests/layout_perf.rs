//! S10.2 DoD: layout pass completes in < 1ms for 200 widgets in a
//! release build.
//!
//! The threshold is generous on purpose — this is a regression guard
//! against accidental quadratic walks, not a tight benchmark. Use the
//! S10.4 / future criterion bench for actual perf tracking.
//!
//! The test is gated on `cfg(not(debug_assertions))`: debug builds run
//! 3-5× slower and would exceed the budget without diagnosing a real
//! regression. CI runs the full workspace under `--release` for the
//! release-build job, where this guard fires.
//!
//! Timing strategy: take the **minimum** of three forced-dirty passes
//! and assert against that. Single-sample wall-time guards on shared
//! CI runners are flaky — a noisy GC / scheduler hiccup pushes one
//! sample past the threshold without diagnosing a real regression.
//! `min` is the right reduction here because we want to ask "what's
//! the best the pass can do?", not "what's the median under load?".

#![cfg(not(debug_assertions))]

use std::time::{Duration, Instant};

use beast_ui::{Axis, Button, Grid, IdAllocator, Size, Stack, WidgetTree};

#[test]
fn layout_pass_completes_under_1ms_for_200_widgets() {
    let mut ids = IdAllocator::new();

    // Build a tree with ~200 widgets:
    //   1 root Stack
    //   + 1 grid (10 cols)
    //   + 200 Buttons inside the grid.
    let mut grid = Grid::new(ids.allocate(), 10).with_gap(2.0);
    for i in 0..200 {
        grid.push_child(Box::new(Button::new(ids.allocate(), format!("b{i}"))));
    }
    let mut root = Stack::new(ids.allocate(), Axis::Vertical);
    root.push_child(Box::new(grid));

    let mut tree = WidgetTree::new(Box::new(root), Size::new(1280.0, 720.0));

    // Warm-up pass — first call allocates the children-size Vecs in
    // Stack/Grid. Subsequent passes reflect steady-state cost.
    assert!(tree.layout(), "warm-up pass");

    let mut samples: [Duration; 3] = [Duration::ZERO; 3];
    for (i, slot) in samples.iter_mut().enumerate() {
        // Resize by a sub-pixel amount on each iteration so
        // `WidgetTree::resize` always invalidates the cache.
        tree.resize(Size::new(1280.0 + i as f32 + 1.0, 720.0));
        let t = Instant::now();
        let did_lay_out = tree.layout();
        *slot = t.elapsed();
        assert!(did_lay_out, "post-resize pass must run (sample {i})");
    }
    let best = samples.iter().min().copied().expect("samples is non-empty");
    let micros = best.as_micros();
    assert!(
        micros < 1_000,
        "layout pass over 1ms (best {micros} µs of 3 samples: {samples:?}) — likely regressed to quadratic walk"
    );
}
