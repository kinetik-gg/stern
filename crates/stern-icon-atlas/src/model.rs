//! Owned, renderer-neutral normalized vector model.

/// A point in the canonical 256 by 256 icon coordinate space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    /// Horizontal coordinate.
    pub x: f64,
    /// Vertical coordinate.
    pub y: f64,
}

/// Arc-free normalized path command.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PathCommand {
    /// Begin a subpath.
    MoveTo(Point),
    /// Add a line segment.
    LineTo(Point),
    /// Add a quadratic Bézier segment.
    QuadTo {
        /// Control point.
        control: Point,
        /// Segment endpoint.
        to: Point,
    },
    /// Add a cubic Bézier segment.
    CubicTo {
        /// First control point.
        control1: Point,
        /// Second control point.
        control2: Point,
        /// Segment endpoint.
        to: Point,
    },
    /// Close the current subpath.
    Close,
}

/// Path filling rule.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum FillRule {
    /// Non-zero winding rule.
    #[default]
    NonZero,
    /// Even-odd rule.
    EvenOdd,
}

/// Stroke line cap.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum StrokeCap {
    /// Flat endpoint.
    #[default]
    Butt,
    /// Rounded endpoint.
    Round,
    /// Square endpoint.
    Square,
}

/// Stroke line join.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum StrokeJoin {
    /// Mitered corner.
    #[default]
    Miter,
    /// Rounded corner.
    Round,
    /// Beveled corner.
    Bevel,
}

/// Optional path stroke.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrokeStyle {
    /// Stroke width.
    pub width: f64,
    /// Endpoint shape.
    pub cap: StrokeCap,
    /// Corner shape.
    pub join: StrokeJoin,
}

/// One ordered path/layer from an SVG icon.
#[derive(Clone, Debug, PartialEq)]
pub struct NormalizedPath {
    /// Ordered path commands, with SVG arcs lowered to cubics.
    pub commands: Vec<PathCommand>,
    /// Whether the path is filled.
    pub filled: bool,
    /// Filling rule.
    pub fill_rule: FillRule,
    /// Layer opacity in the inclusive range zero through one.
    pub opacity: f64,
    /// Optional stroke style.
    pub stroke: Option<StrokeStyle>,
}

/// One normalized icon document.
#[derive(Clone, Debug, PartialEq)]
pub struct NormalizedIcon {
    /// Canonical view-box width.
    pub width: f64,
    /// Canonical view-box height.
    pub height: f64,
    /// Ordered paths, preserving duotone layer order and opacity.
    pub paths: Vec<NormalizedPath>,
}
