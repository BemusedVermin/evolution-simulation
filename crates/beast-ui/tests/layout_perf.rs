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

#![cfg(not(debug_assertions))]

use std::time::Instant;

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
    // Stack/Grid. We measure the second pass after a forced
    // invalidation so the timing reflects steady-state cost.
    assert!(tree.layout(), "warm-up pass");
    tree.resize(Size::new(1281.0, 721.0));

    let start = Instant::now();
    let did_lay_out = tree.layout();
    let elapsed = start.elapsed();
    assert!(did_lay_out, "post-resize pass must run");

    let micros = elapsed.as_micros();
    assert!(
        micros < 1_000,
        "layout pass over 1ms ({micros} µs) — likely regressed to quadratic walk"
    );
}
