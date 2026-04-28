//! Snapshot fixture for `dump_layout`.
//!
//! Pins the canonical pre-order dump format and the geometry the
//! Stack + Grid + Card composition produces under the layout pass.
//! Any change here without a corresponding spec update is a layout
//! regression.

use beast_ui::{
    dump_layout, Align, Axis, Button, Card, Grid, IdAllocator, Size, Stack, WidgetTree,
};

#[test]
fn stack_grid_card_snapshot_matches_fixture() {
    let mut ids = IdAllocator::new();
    let mut root = Stack::new(ids.allocate(), Axis::Vertical)
        .with_gap(4.0)
        .with_align(Align::Start);

    let mut action_bar = Stack::new(ids.allocate(), Axis::Horizontal).with_gap(8.0);
    action_bar.push_child(Box::new(Button::new(ids.allocate(), "ok")));
    action_bar.push_child(Box::new(Button::new(ids.allocate(), "no")));
    root.push_child(Box::new(action_bar));

    let mut grid = Grid::new(ids.allocate(), 2).with_gap(2.0);
    grid.push_child(Box::new(Card::new(ids.allocate(), "a")));
    grid.push_child(Box::new(Card::new(ids.allocate(), "b")));
    grid.push_child(Box::new(Card::new(ids.allocate(), "c")));
    root.push_child(Box::new(grid));

    let mut tree = WidgetTree::new(Box::new(root), Size::new(320.0, 240.0));
    assert!(tree.layout(), "first layout pass should be a cache miss");

    // Hand-authored fixture. Ids count up from 1; sizes come from each
    // widget's measure() heuristic:
    //   Button = chars * 8 + 16 wide, 32 tall.
    //   Card (childless) = title_chars * 8 wide, 20 tall (title-bar only).
    // Grid uses uniform cell sizing = max child measure.
    //
    // Stack#1 sits at the root_size (tight constraints from the tree).
    // Action bar Stack#2 = 32 + 8 (gap) + 32 = 72 wide. Grid#5 has 3
    // childless Cards at 8x20 each, packed 2 cols × 2 rows with 2 px
    // gaps -> 18 × 42.
    let expected = "\
Stack#1 0,0 320x240
Stack#2 0,0 72x32
Button#3 0,0 32x32
Button#4 40,0 32x32
Grid#5 0,36 18x42
Card#6 0,36 8x20
Card#7 10,36 8x20
Card#8 0,58 8x20
";
    let actual = dump_layout(&tree);
    assert_eq!(
        actual, expected,
        "layout snapshot drift — actual was:\n{actual}"
    );
}
