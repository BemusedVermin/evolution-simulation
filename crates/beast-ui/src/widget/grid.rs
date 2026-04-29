//! Fixed-column grid container.
//!
//! Children are packed row-major into a `columns`-wide grid: index `i`
//! lives at row `i / columns`, column `i % columns`. Inter-cell
//! *spacing* is uniform — every slot is `cell_w × cell_h` where
//! `cell_w / cell_h` are the maxima across all children's measured
//! sizes — but inside a slot each child is anchored at the slot
//! origin and rendered at its own preferred size. Children smaller
//! than the slot leave dead space below / right; that's intentional,
//! since the slot defines the layout grid, not the rendered footprint.
//! This is sufficient for the bestiary entry list and the other
//! summary-card screens in S10.4. Variable-cell grids (Masonry-style
//! packing) are out of scope.

use crate::event::{EventResult, UiEvent};
use crate::layout::LayoutConstraints;
use crate::paint::{PaintCtx, Point, Rect, Size};

use super::{LayoutCtx, Widget, WidgetId};

/// Row-major fixed-column grid.
pub struct Grid {
    id: WidgetId,
    bounds: Rect,
    columns: u16,
    gap: f32,
    children: Vec<Box<dyn Widget>>,
    last_cursor: Option<Point>,
}

impl Grid {
    /// Construct an empty grid with `columns` (clamped to ≥ 1) columns
    /// and zero gap.
    pub fn new(id: WidgetId, columns: u16) -> Self {
        Self {
            id,
            bounds: Rect::ZERO,
            columns: columns.max(1),
            gap: 0.0,
            children: Vec::new(),
            last_cursor: None,
        }
    }

    /// Override the gap inserted between adjacent rows / columns.
    pub fn with_gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Append a child. Children are placed in row-major declaration
    /// order — child `i` ends up at `(row = i / columns, col = i % columns)`.
    pub fn push_child(&mut self, child: Box<dyn Widget>) {
        self.children.push(child);
    }

    /// Number of children currently held.
    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    /// Read-only access to children — used by tests + future debug
    /// inspectors.
    pub fn children(&self) -> &[Box<dyn Widget>] {
        &self.children
    }

    /// Number of columns.
    pub fn columns(&self) -> u16 {
        self.columns
    }

    /// Gap between adjacent cells, in pixels.
    pub fn gap(&self) -> f32 {
        self.gap
    }

    fn rows(&self) -> usize {
        let cols = self.columns as usize;
        self.children.len().div_ceil(cols)
    }
}

impl std::fmt::Debug for Grid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Grid")
            .field("id", &self.id)
            .field("bounds", &self.bounds)
            .field("columns", &self.columns)
            .field("gap", &self.gap)
            .field("children", &self.children.len())
            .finish()
    }
}

impl Widget for Grid {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn bounds(&self) -> Rect {
        self.bounds
    }

    fn set_bounds(&mut self, bounds: Rect) {
        self.bounds = bounds;
    }

    fn measure(&self, ctx: &LayoutCtx) -> Size {
        let cols = self.columns as usize;
        let rows = self.rows();
        if rows == 0 {
            return Size::ZERO;
        }
        let (cell_w, cell_h) = self.children.iter().fold((0.0_f32, 0.0_f32), |acc, c| {
            let s = c.measure(ctx);
            (acc.0.max(s.width), acc.1.max(s.height))
        });
        Size::new(
            cell_w * cols as f32 + self.gap * (cols.saturating_sub(1)) as f32,
            cell_h * rows as f32 + self.gap * (rows.saturating_sub(1)) as f32,
        )
    }

    fn layout(&mut self, ctx: &LayoutCtx, constraints: LayoutConstraints) -> Size {
        // Same two-phase walk as `Stack`: measure first to pick the
        // uniform cell size, then place + recurse so grandchildren see
        // a correct parent origin.
        let cols = self.columns as usize;
        let rows = self.rows();
        if rows == 0 {
            return constraints.constrain(Size::ZERO);
        }

        let child_sizes: Vec<Size> = self.children.iter().map(|c| c.measure(ctx)).collect();
        let cell_w = child_sizes.iter().map(|s| s.width).fold(0.0_f32, f32::max);
        let cell_h = child_sizes.iter().map(|s| s.height).fold(0.0_f32, f32::max);

        let total_w = cell_w * cols as f32 + self.gap * (cols.saturating_sub(1)) as f32;
        let total_h = cell_h * rows as f32 + self.gap * (rows.saturating_sub(1)) as f32;
        let final_size = constraints.constrain(Size::new(total_w, total_h));

        for (i, child) in self.children.iter_mut().enumerate() {
            let row = i / cols;
            let col = i % cols;
            let x = self.bounds.origin.x + col as f32 * (cell_w + self.gap);
            let y = self.bounds.origin.y + row as f32 * (cell_h + self.gap);
            let s = child_sizes[i];
            child.set_bounds(Rect::new(Point::new(x, y), s));
            // The tight constraint forces the child to return exactly
            // `s` — `let _: Size` is a typed discard that documents the
            // deliberate drop and silences the `#[must_use]` on
            // `Widget::layout`.
            let _: Size = child.layout(ctx, LayoutConstraints::tight(s));
        }

        final_size
    }

    fn paint(&self, ctx: &mut PaintCtx) {
        for child in &self.children {
            child.paint(ctx);
        }
    }

    fn handle_event(&mut self, event: &UiEvent) -> EventResult {
        if let UiEvent::MouseMove { x, y } = event {
            self.last_cursor = Some(Point::new(*x, *y));
        }
        // Hit-test the relevant cursor position against each child's
        // bounds before forwarding — same contract as `Stack`. The
        // earlier `matches!(event, MouseMove)` short-circuit broadcast
        // every cursor frame to every cell at O(n).
        let cursor = match event {
            UiEvent::MouseMove { x, y } => Some(Point::new(*x, *y)),
            _ => self.last_cursor,
        };
        for child in self.children.iter_mut().rev() {
            let inside = cursor.is_some_and(|c| child.bounds().contains(c));
            if inside && child.handle_event(event) == EventResult::Consumed {
                return EventResult::Consumed;
            }
        }
        EventResult::Ignored
    }

    fn visit_pre_order<'a>(&'a self, visitor: &mut dyn FnMut(&'a dyn Widget)) {
        visitor(self);
        for child in &self.children {
            child.visit_pre_order(visitor);
        }
    }

    fn kind(&self) -> &'static str {
        "Grid"
    }

    fn collect_focus_chain(&self, out: &mut Vec<WidgetId>) {
        // Row-major declaration order matches the grid's visual reading
        // order, so Tab steps left-to-right, then top-to-bottom.
        for child in &self.children {
            child.collect_focus_chain(out);
        }
    }

    fn find_widget_mut(&mut self, id: WidgetId) -> Option<&mut dyn Widget> {
        if self.id == id {
            return Some(self);
        }
        for child in &mut self.children {
            if let Some(w) = child.find_widget_mut(id) {
                return Some(w);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget::{Card, IdAllocator};

    fn ctx() -> LayoutCtx {
        LayoutCtx::default()
    }

    #[test]
    fn nine_cards_in_three_columns_pack_row_major() {
        let mut ids = IdAllocator::new();
        let grid_id = ids.allocate();
        let mut grid = Grid::new(grid_id, 3).with_gap(2.0);
        grid.set_bounds(Rect::xywh(0.0, 0.0, 1000.0, 1000.0));

        // Capture each card's id in declaration order so the assertion
        // below matches "row 0 ids = [0,1,2], row 1 = [3,4,5]" precisely.
        let mut ids_in_order: Vec<u32> = Vec::with_capacity(9);
        for _ in 0..9 {
            let id = ids.allocate();
            ids_in_order.push(id.raw());
            grid.push_child(Box::new(Card::new(id, "C")));
        }

        let _ = grid.layout(&ctx(), LayoutConstraints::loose(Size::new(1000.0, 1000.0)));

        // Three rows of three. Verify the row-major contract by reading
        // each child's id back out in the order the grid stores them
        // and confirming row r / col c matches index r*3 + c.
        for (i, child) in grid.children().iter().enumerate() {
            let expected_id = ids_in_order[i];
            assert_eq!(child.id().raw(), expected_id);
            let row = i / 3;
            let col = i % 3;
            // All cards measure at the same default size, so the cell
            // origin is `col * (cell_w + gap)` / `row * (cell_h + gap)`.
            let cell_w = child.bounds().size.width;
            let cell_h = child.bounds().size.height;
            assert!(
                (child.bounds().origin.x - col as f32 * (cell_w + 2.0)).abs() < 1e-3,
                "child {i} x"
            );
            assert!(
                (child.bounds().origin.y - row as f32 * (cell_h + 2.0)).abs() < 1e-3,
                "child {i} y"
            );
        }
    }

    #[test]
    fn empty_grid_returns_zero_size() {
        let mut ids = IdAllocator::new();
        let mut grid = Grid::new(ids.allocate(), 4);
        grid.set_bounds(Rect::xywh(0.0, 0.0, 100.0, 100.0));
        let size = grid.layout(&ctx(), LayoutConstraints::loose(Size::new(100.0, 100.0)));
        assert_eq!(size, Size::ZERO);
    }

    #[test]
    fn columns_clamps_to_at_least_one() {
        let mut ids = IdAllocator::new();
        let grid = Grid::new(ids.allocate(), 0);
        assert_eq!(grid.columns(), 1);
    }

    #[test]
    fn last_partial_row_is_handled() {
        // 5 children in 3 columns -> rows 0,1; row 1 has only 2 cells.
        let mut ids = IdAllocator::new();
        let mut grid = Grid::new(ids.allocate(), 3);
        grid.set_bounds(Rect::xywh(0.0, 0.0, 1000.0, 1000.0));
        for _ in 0..5 {
            grid.push_child(Box::new(Card::new(ids.allocate(), "x")));
        }
        let _ = grid.layout(&ctx(), LayoutConstraints::loose(Size::new(1000.0, 1000.0)));
        // Child 4 sits at (row 1, col 1). With the default zero gap,
        // x = col * (cell_w + gap) = 1 * cell_w = cell_w exactly. Pin
        // the value precisely so a regression in the row-major formula
        // (e.g. swapping rows/cols) is caught instead of silently
        // satisfying a loose `> 0 && < cell_w * 2` range.
        let r = grid.children()[4].bounds();
        let cell_w = grid.children()[0].bounds().size.width;
        let cell_h = grid.children()[0].bounds().size.height;
        assert!((r.origin.x - cell_w).abs() < 1e-3, "child 4 x = cell_w");
        assert!((r.origin.y - cell_h).abs() < 1e-3, "child 4 y = cell_h");
    }
}
