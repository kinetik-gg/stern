use kinetik_ui_core::{
    Brush, Color, CornerRadius, ImageId, LayerId, PathElement, Rect, Size, Stroke, TextLayoutId,
    TextureId, Transform, Vec2,
};
use kinetik_ui_render::Translation as RenderTranslation;

/// Deterministic command produced before backend drawing.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderCommand {
    /// Layer used by the command.
    pub layer: LayerId,
    /// Clip stack active for the command, outermost to innermost.
    pub clips: Vec<RenderClip>,
    /// Transform used by the command.
    pub transform: Transform,
    /// Command kind.
    pub kind: RenderCommandKind,
}

/// Clip scope captured for a render command.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderClip {
    /// Clip rectangle.
    pub rect: Rect,
    /// Transform active when the clip scope began.
    pub transform: Transform,
}

/// Command kind produced by primitive translation.
#[derive(Debug, Clone, PartialEq)]
pub enum RenderCommandKind {
    /// Filled and/or stroked rectangle.
    Rect {
        /// Rectangle bounds.
        rect: Rect,
        /// Fill brush.
        fill: Option<Brush>,
        /// Stroke style.
        stroke: Option<Stroke>,
        /// Corner radii.
        radius: CornerRadius,
    },
    /// Stroked line.
    Line {
        /// Start x.
        x0: f32,
        /// Start y.
        y0: f32,
        /// End x.
        x1: f32,
        /// End y.
        y1: f32,
        /// Stroke style.
        stroke: Stroke,
    },
    /// Box shadow.
    Shadow {
        /// Source rectangle.
        rect: Rect,
        /// Shadow offset.
        offset: Vec2,
        /// Gaussian blur radius.
        blur_radius: f32,
        /// Spread amount.
        spread: f32,
        /// Uniform corner radius.
        radius: f32,
        /// Shadow color.
        color: Color,
    },
    /// Filled and/or stroked vector path.
    Path {
        /// Path elements in drawing order.
        elements: Vec<PathElement>,
        /// Fill brush.
        fill: Option<Brush>,
        /// Stroke style.
        stroke: Option<Stroke>,
    },
    /// Text command backed by a shaped layout resource or renderer fallback shaping.
    Text {
        /// Optional shaped layout resource.
        layout: Option<TextLayoutId>,
        /// Baseline origin.
        origin: kinetik_ui_core::Point,
        /// Text content.
        text: String,
        /// Font family name or logical family.
        family: String,
        /// Font size in logical units.
        size: f32,
        /// Line height in logical units.
        line_height: f32,
        /// Text color.
        color: Color,
    },
    /// Image resource draw command.
    Image {
        /// Image resource.
        image: ImageId,
        /// Destination rectangle.
        rect: Rect,
        /// Optional color multiplied into the image payload.
        tint: Option<Color>,
    },
    /// Texture resource draw command.
    Texture {
        /// Texture resource.
        texture: TextureId,
        /// Destination rectangle.
        rect: Rect,
        /// Source size in texture pixels.
        source_size: Size,
    },
}

/// Translation result used by tests and renderer internals.
pub type Translation = RenderTranslation<RenderCommand>;
