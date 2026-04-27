//! Paint context + geometric primitives shared across widgets.
//!
//! The widget framework records paint operations into a [`PaintCtx`] rather
//! than driving a renderer directly. This decouples widget tests from the
//! SDL backend (the `headless` feature builds them with no SDL link at all)
//! and gives layout / snapshot tests a deterministic command list to assert
//! on.
//!
//! Future work: a `Renderer` adapter will consume the recorded commands and
//! issue real draw calls. That work belongs to the screen-wiring story
//! (S10.4) — it is intentionally not part of this crate today.

use serde::{Deserialize, Serialize};

/// 2D point in widget-space coordinates (top-left origin, pixels).
#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Point {
    /// Horizontal coordinate, increasing rightwards.
    pub x: f32,
    /// Vertical coordinate, increasing downwards.
    pub y: f32,
}

impl Point {
    /// Construct a [`Point`] from explicit `x` / `y`.
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// 2D size in widget-space pixels. Both axes are non-negative by convention;
/// callers should not produce negative sizes.
#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Size {
    /// Width in pixels.
    pub width: f32,
    /// Height in pixels.
    pub height: f32,
}

impl Size {
    /// Construct a [`Size`] from explicit width / height.
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    /// Zero-sized rectangle; useful as a default for empty widgets.
    pub const ZERO: Self = Self {
        width: 0.0,
        height: 0.0,
    };
}

/// Axis-aligned rectangle expressed by its top-left origin and [`Size`].
#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    /// Top-left corner.
    pub origin: Point,
    /// Width / height.
    pub size: Size,
}

impl Rect {
    /// Construct a [`Rect`] from origin + size.
    pub const fn new(origin: Point, size: Size) -> Self {
        Self { origin, size }
    }

    /// Construct a [`Rect`] from explicit `x`, `y`, `width`, `height`.
    pub const fn xywh(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            origin: Point::new(x, y),
            size: Size::new(width, height),
        }
    }

    /// Rectangle with zero origin and zero size.
    pub const ZERO: Self = Self::xywh(0.0, 0.0, 0.0, 0.0);

    /// Right edge x-coordinate.
    pub fn right(&self) -> f32 {
        self.origin.x + self.size.width
    }

    /// Bottom edge y-coordinate.
    pub fn bottom(&self) -> f32 {
        self.origin.y + self.size.height
    }

    /// Returns true if `point` falls inside this rectangle. Edges are
    /// inclusive on the top / left and exclusive on the bottom / right —
    /// matches the half-open convention used by most 2D graphics APIs.
    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.origin.x
            && point.x < self.right()
            && point.y >= self.origin.y
            && point.y < self.bottom()
    }
}

/// RGBA color, channels in `[0.0, 1.0]`.
#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Color {
    /// Red channel.
    pub r: f32,
    /// Green channel.
    pub g: f32,
    /// Blue channel.
    pub b: f32,
    /// Alpha channel.
    pub a: f32,
}

impl Color {
    /// Construct an RGBA color.
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Opaque RGB color (alpha = 1.0).
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self::rgba(r, g, b, 1.0)
    }

    /// Fully transparent black.
    pub const TRANSPARENT: Self = Self::rgba(0.0, 0.0, 0.0, 0.0);
    /// Opaque white.
    pub const WHITE: Self = Self::rgb(1.0, 1.0, 1.0);
    /// Opaque black.
    pub const BLACK: Self = Self::rgb(0.0, 0.0, 0.0);
}

/// One recorded draw operation.
///
/// The widget framework accumulates these into a [`PaintCtx`]; downstream
/// code either snapshots them for tests or flushes them through a renderer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum DrawCmd {
    /// Filled rectangle with a single color.
    FillRect {
        /// Rectangle to fill.
        rect: Rect,
        /// Fill color.
        color: Color,
    },
    /// Stroked rectangle outline (1px wide for now).
    StrokeRect {
        /// Rectangle outline.
        rect: Rect,
        /// Stroke color.
        color: Color,
    },
    /// Plain text at a baseline anchor. Font + size are deferred to S10.4.
    Text {
        /// Position the text starts drawing from (top-left).
        pos: Point,
        /// String content.
        text: String,
        /// Text color.
        color: Color,
    },
}

/// Paint command recorder. A widget receives `&mut PaintCtx` from the tree
/// during a paint pass and pushes [`DrawCmd`]s for downstream consumption.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PaintCtx {
    commands: Vec<DrawCmd>,
}

impl PaintCtx {
    /// Construct an empty paint context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a [`DrawCmd::FillRect`].
    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.commands.push(DrawCmd::FillRect { rect, color });
    }

    /// Push a [`DrawCmd::StrokeRect`].
    pub fn stroke_rect(&mut self, rect: Rect, color: Color) {
        self.commands.push(DrawCmd::StrokeRect { rect, color });
    }

    /// Push a [`DrawCmd::Text`].
    pub fn text(&mut self, pos: Point, text: impl Into<String>, color: Color) {
        self.commands.push(DrawCmd::Text {
            pos,
            text: text.into(),
            color,
        });
    }

    /// Recorded commands, in push order.
    pub fn commands(&self) -> &[DrawCmd] {
        &self.commands
    }

    /// Move recorded commands out of the context, leaving it empty.
    pub fn take_commands(&mut self) -> Vec<DrawCmd> {
        std::mem::take(&mut self.commands)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_contains_uses_half_open_edges() {
        let r = Rect::xywh(10.0, 20.0, 30.0, 40.0);
        assert!(r.contains(Point::new(10.0, 20.0)), "top-left inclusive");
        assert!(r.contains(Point::new(39.999, 59.999)), "interior");
        assert!(
            !r.contains(Point::new(40.0, 60.0)),
            "bottom-right exclusive"
        );
        assert!(!r.contains(Point::new(9.0, 20.0)), "outside left");
    }

    #[test]
    fn paint_ctx_records_commands_in_order() {
        let mut ctx = PaintCtx::new();
        ctx.fill_rect(Rect::xywh(0.0, 0.0, 10.0, 10.0), Color::WHITE);
        ctx.text(Point::new(2.0, 2.0), "hi", Color::BLACK);
        assert_eq!(ctx.commands().len(), 2);
        assert!(matches!(ctx.commands()[0], DrawCmd::FillRect { .. }));
        assert!(matches!(ctx.commands()[1], DrawCmd::Text { .. }));
    }

    #[test]
    fn paint_ctx_take_commands_empties_context() {
        let mut ctx = PaintCtx::new();
        ctx.fill_rect(Rect::ZERO, Color::WHITE);
        let cmds = ctx.take_commands();
        assert_eq!(cmds.len(), 1);
        assert!(ctx.commands().is_empty());
    }
}
