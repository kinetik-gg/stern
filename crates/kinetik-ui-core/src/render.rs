//! Backend-independent render primitives.

use crate::{Point, Rect, Size, Vec2};

/// RGBA color in linear toolkit space.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
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
    /// Transparent black.
    pub const TRANSPARENT: Self = Self::rgba(0.0, 0.0, 0.0, 0.0);
    /// Opaque black.
    pub const BLACK: Self = Self::rgba(0.0, 0.0, 0.0, 1.0);
    /// Opaque white.
    pub const WHITE: Self = Self::rgba(1.0, 1.0, 1.0, 1.0);

    /// Creates a color from RGBA channels.
    #[must_use]
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Creates an opaque color from RGB channels.
    #[must_use]
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self::rgba(r, g, b, 1.0)
    }

    /// Returns this color with a replaced alpha channel.
    #[must_use]
    pub const fn with_alpha(self, a: f32) -> Self {
        Self { a, ..self }
    }
}

/// Fill/stroke brush.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Brush {
    /// Solid color brush.
    Solid(Color),
    /// Linear gradient brush.
    LinearGradient(LinearGradient),
}

/// Maximum color stops stored inline by a gradient brush.
pub const MAX_GRADIENT_STOPS: usize = 8;

/// Error returned when a gradient cannot be constructed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GradientBuildError {
    /// A gradient needs at least two color stops.
    TooFewStops,
    /// The gradient exceeded [`MAX_GRADIENT_STOPS`].
    TooManyStops,
}

/// One color stop in a gradient ramp.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct GradientStop {
    /// Normalized stop offset, where 0 is the start and 1 is the end.
    pub offset: f32,
    /// Stop color.
    pub color: Color,
}

impl GradientStop {
    /// Creates a gradient stop.
    #[must_use]
    pub const fn new(offset: f32, color: Color) -> Self {
        Self { offset, color }
    }
}

/// Inline linear gradient brush.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinearGradient {
    start: Point,
    end: Point,
    stops: [GradientStop; MAX_GRADIENT_STOPS],
    stop_count: usize,
}

impl LinearGradient {
    /// Creates a two-color linear gradient.
    #[must_use]
    pub const fn between(start: Point, end: Point, start_color: Color, end_color: Color) -> Self {
        let mut stops = [GradientStop::new(0.0, Color::TRANSPARENT); MAX_GRADIENT_STOPS];
        stops[0] = GradientStop::new(0.0, start_color);
        stops[1] = GradientStop::new(1.0, end_color);
        Self {
            start,
            end,
            stops,
            stop_count: 2,
        }
    }

    /// Creates a linear gradient from explicit stops.
    ///
    /// # Errors
    ///
    /// Returns [`GradientBuildError::TooFewStops`] when fewer than two stops are provided and
    /// [`GradientBuildError::TooManyStops`] when the stop count exceeds [`MAX_GRADIENT_STOPS`].
    pub fn new(
        start: Point,
        end: Point,
        stops: &[GradientStop],
    ) -> Result<Self, GradientBuildError> {
        if stops.len() < 2 {
            return Err(GradientBuildError::TooFewStops);
        }
        if stops.len() > MAX_GRADIENT_STOPS {
            return Err(GradientBuildError::TooManyStops);
        }
        let mut storage = [GradientStop::new(0.0, Color::TRANSPARENT); MAX_GRADIENT_STOPS];
        storage[..stops.len()].copy_from_slice(stops);
        Ok(Self {
            start,
            end,
            stops: storage,
            stop_count: stops.len(),
        })
    }

    /// Creates a linear gradient with evenly spaced colors.
    ///
    /// # Errors
    ///
    /// Returns [`GradientBuildError::TooFewStops`] when fewer than two colors are provided and
    /// [`GradientBuildError::TooManyStops`] when the color count exceeds [`MAX_GRADIENT_STOPS`].
    pub fn from_colors(
        start: Point,
        end: Point,
        colors: &[Color],
    ) -> Result<Self, GradientBuildError> {
        if colors.len() < 2 {
            return Err(GradientBuildError::TooFewStops);
        }
        if colors.len() > MAX_GRADIENT_STOPS {
            return Err(GradientBuildError::TooManyStops);
        }
        let denom = f32::from(u16::try_from(colors.len() - 1).unwrap_or(1));
        let mut stops = [GradientStop::new(0.0, Color::TRANSPARENT); MAX_GRADIENT_STOPS];
        for (index, color) in colors.iter().copied().enumerate() {
            let offset = f32::from(u16::try_from(index).unwrap_or(u16::MAX)) / denom;
            stops[index] = GradientStop::new(offset, color);
        }
        Ok(Self {
            start,
            end,
            stops,
            stop_count: colors.len(),
        })
    }

    /// Returns the gradient start point.
    #[must_use]
    pub const fn start(self) -> Point {
        self.start
    }

    /// Returns the gradient end point.
    #[must_use]
    pub const fn end(self) -> Point {
        self.end
    }

    /// Returns the active color stops.
    #[must_use]
    pub fn stops(&self) -> &[GradientStop] {
        &self.stops[..self.stop_count]
    }

    /// Returns the number of active stops.
    #[must_use]
    pub const fn stop_count(self) -> usize {
        self.stop_count
    }
}

/// Stroke style.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stroke {
    /// Stroke width in logical units.
    pub width: f32,
    /// Stroke brush.
    pub brush: Brush,
}

impl Stroke {
    /// Creates a stroke.
    #[must_use]
    pub const fn new(width: f32, brush: Brush) -> Self {
        Self { width, brush }
    }
}

/// Corner radii for rounded rectangles.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CornerRadius {
    /// Top-left radius.
    pub top_left: f32,
    /// Top-right radius.
    pub top_right: f32,
    /// Bottom-right radius.
    pub bottom_right: f32,
    /// Bottom-left radius.
    pub bottom_left: f32,
}

impl CornerRadius {
    /// Creates equal corner radii.
    #[must_use]
    pub const fn all(value: f32) -> Self {
        Self {
            top_left: value,
            top_right: value,
            bottom_right: value,
            bottom_left: value,
        }
    }
}

/// Static image resource handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ImageId(u64);

/// Symbolic icon resource handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct IconId(u64);

impl IconId {
    /// Creates an icon ID from raw bits.
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns raw ID bits.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

impl ImageId {
    /// Creates an image ID from raw bits.
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns raw ID bits.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// GPU-resident texture surface handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TextureId(u64);

impl TextureId {
    /// Creates a texture ID from raw bits.
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns raw ID bits.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// Shaped text layout resource handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TextLayoutId(u64);

impl TextLayoutId {
    /// Creates a text layout ID from raw bits.
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns raw ID bits.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// Clip command identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ClipId(u64);

impl ClipId {
    /// Creates a clip ID from raw bits.
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns raw ID bits.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// Layer command identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LayerId(u64);

impl LayerId {
    /// Creates a layer ID from raw bits.
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns raw ID bits.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// 2D affine transform matrix.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    /// Scale/skew x component.
    pub m11: f32,
    /// Skew y component.
    pub m12: f32,
    /// Skew x component.
    pub m21: f32,
    /// Scale/skew y component.
    pub m22: f32,
    /// Translation x.
    pub dx: f32,
    /// Translation y.
    pub dy: f32,
}

impl Transform {
    /// Identity transform.
    pub const IDENTITY: Self = Self {
        m11: 1.0,
        m12: 0.0,
        m21: 0.0,
        m22: 1.0,
        dx: 0.0,
        dy: 0.0,
    };

    /// Creates a translation transform.
    #[must_use]
    pub const fn translation(offset: Vec2) -> Self {
        Self {
            dx: offset.x,
            dy: offset.y,
            ..Self::IDENTITY
        }
    }

    /// Creates a scale transform.
    #[must_use]
    pub const fn scale(scale: Vec2) -> Self {
        Self {
            m11: scale.x,
            m22: scale.y,
            ..Self::IDENTITY
        }
    }

    /// Returns true when every transform component is finite.
    #[must_use]
    pub fn is_finite(self) -> bool {
        self.m11.is_finite()
            && self.m12.is_finite()
            && self.m21.is_finite()
            && self.m22.is_finite()
            && self.dx.is_finite()
            && self.dy.is_finite()
    }

    /// Applies this transform to a point.
    #[must_use]
    pub fn transform_point(self, point: Point) -> Point {
        Point::new(
            self.m11
                .mul_add(point.x, self.m21.mul_add(point.y, self.dx)),
            self.m12
                .mul_add(point.x, self.m22.mul_add(point.y, self.dy)),
        )
    }

    /// Composes a parent transform with a child transform.
    #[must_use]
    pub fn compose(parent: Self, child: Self) -> Self {
        Self {
            m11: parent.m11.mul_add(child.m11, parent.m21 * child.m12),
            m12: parent.m12.mul_add(child.m11, parent.m22 * child.m12),
            m21: parent.m11.mul_add(child.m21, parent.m21 * child.m22),
            m22: parent.m12.mul_add(child.m21, parent.m22 * child.m22),
            dx: parent
                .m11
                .mul_add(child.dx, parent.m21.mul_add(child.dy, parent.dx)),
            dy: parent
                .m12
                .mul_add(child.dx, parent.m22.mul_add(child.dy, parent.dy)),
        }
    }

    /// Returns this transform followed by a child transform.
    #[must_use]
    pub fn then(self, child: Self) -> Self {
        Self::compose(self, child)
    }

    /// Returns the inverse transform when this transform is finite and invertible.
    #[must_use]
    pub fn try_inverse(self) -> Option<Self> {
        if !self.is_finite() {
            return None;
        }

        let determinant = self.m11.mul_add(self.m22, -(self.m21 * self.m12));
        if !determinant.is_finite() || determinant == 0.0 {
            return None;
        }

        let inverse_determinant = determinant.recip();
        let inverse = Self {
            m11: self.m22 * inverse_determinant,
            m12: -self.m12 * inverse_determinant,
            m21: -self.m21 * inverse_determinant,
            m22: self.m11 * inverse_determinant,
            dx: 0.0,
            dy: 0.0,
        };
        let inverse = Self {
            dx: -(inverse.m11.mul_add(self.dx, inverse.m21 * self.dy)),
            dy: -(inverse.m12.mul_add(self.dx, inverse.m22 * self.dy)),
            ..inverse
        };

        inverse.is_finite().then_some(inverse)
    }

    /// Returns the inverse transform when this transform is finite and invertible.
    #[must_use]
    pub fn inverse(self) -> Option<Self> {
        self.try_inverse()
    }
}

/// Rectangle draw command.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RectPrimitive {
    /// Rectangle bounds.
    pub rect: Rect,
    /// Fill brush.
    pub fill: Option<Brush>,
    /// Stroke style.
    pub stroke: Option<Stroke>,
    /// Corner radii.
    pub radius: CornerRadius,
}

/// Line draw command.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinePrimitive {
    /// Start point.
    pub from: Point,
    /// End point.
    pub to: Point,
    /// Stroke style.
    pub stroke: Stroke,
}

/// Box shadow draw command for elevated surfaces.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadowPrimitive {
    /// Source rectangle that casts the shadow.
    pub rect: Rect,
    /// Shadow offset in logical units.
    pub offset: Vec2,
    /// Gaussian blur radius in logical units.
    pub blur_radius: f32,
    /// Amount to expand or shrink the source rectangle before blurring.
    pub spread: f32,
    /// Uniform corner radius for the shadow shape.
    pub radius: f32,
    /// Shadow color.
    pub color: Color,
}

impl ShadowPrimitive {
    /// Creates a box shadow primitive.
    #[must_use]
    pub const fn new(
        rect: Rect,
        offset: Vec2,
        blur_radius: f32,
        spread: f32,
        radius: f32,
        color: Color,
    ) -> Self {
        Self {
            rect,
            offset,
            blur_radius,
            spread,
            radius,
            color,
        }
    }
}

/// One element in a vector path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathElement {
    /// Move the current point without drawing.
    MoveTo(Point),
    /// Draw a straight segment to a point.
    LineTo(Point),
    /// Draw a quadratic Bezier segment.
    QuadTo {
        /// Control point.
        ctrl: Point,
        /// Segment end point.
        to: Point,
    },
    /// Draw a cubic Bezier segment.
    CubicTo {
        /// First control point.
        ctrl1: Point,
        /// Second control point.
        ctrl2: Point,
        /// Segment end point.
        to: Point,
    },
    /// Close the current subpath.
    Close,
}

/// Vector path draw command.
#[derive(Debug, Clone, PartialEq)]
pub struct PathPrimitive {
    /// Path elements in drawing order.
    pub elements: Vec<PathElement>,
    /// Fill brush.
    pub fill: Option<Brush>,
    /// Stroke style.
    pub stroke: Option<Stroke>,
}

impl PathPrimitive {
    /// Creates a path primitive.
    #[must_use]
    pub fn new(
        elements: impl Into<Vec<PathElement>>,
        fill: Option<Brush>,
        stroke: Option<Stroke>,
    ) -> Self {
        Self {
            elements: elements.into(),
            fill,
            stroke,
        }
    }
}

/// Text draw command.
#[derive(Debug, Clone, PartialEq)]
pub struct TextPrimitive {
    /// Optional shaped text layout resource.
    pub layout: Option<TextLayoutId>,
    /// Text baseline origin.
    pub origin: Point,
    /// Text content.
    pub text: String,
    /// Font family name or logical family.
    pub family: String,
    /// Font size in logical units.
    pub size: f32,
    /// Line height in logical units.
    pub line_height: f32,
    /// Fill brush.
    pub brush: Brush,
}

/// Static image draw command.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImagePrimitive {
    /// Image handle.
    pub image: ImageId,
    /// Destination rectangle.
    pub rect: Rect,
    /// Optional color multiplied into the image payload at render time.
    pub tint: Option<Color>,
}

/// Texture draw command.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TexturePrimitive {
    /// Texture handle.
    pub texture: TextureId,
    /// Destination rectangle.
    pub rect: Rect,
    /// Source size in texture pixels.
    pub source_size: Size,
}

/// Backend-independent draw command.
#[derive(Debug, Clone, PartialEq)]
pub enum Primitive {
    /// Rectangle or rounded rectangle.
    Rect(RectPrimitive),
    /// Straight line.
    Line(LinePrimitive),
    /// Box shadow.
    Shadow(ShadowPrimitive),
    /// Vector path.
    Path(PathPrimitive),
    /// Text.
    Text(TextPrimitive),
    /// Static image.
    Image(ImagePrimitive),
    /// GPU texture surface.
    Texture(TexturePrimitive),
    /// Begin rectangular clipping.
    ClipBegin {
        /// Clip command identity.
        id: ClipId,
        /// Clip rectangle.
        rect: Rect,
    },
    /// End clipping.
    ClipEnd {
        /// Clip command identity.
        id: ClipId,
    },
    /// Begin layer.
    LayerBegin {
        /// Layer command identity.
        id: LayerId,
    },
    /// End layer.
    LayerEnd {
        /// Layer command identity.
        id: LayerId,
    },
    /// Begin transform.
    TransformBegin(Transform),
    /// End transform.
    TransformEnd,
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::{
        Brush, ClipId, Color, CornerRadius, GradientBuildError, GradientStop, IconId, ImageId,
        ImagePrimitive, LayerId, LinePrimitive, LinearGradient, MAX_GRADIENT_STOPS, PathElement,
        PathPrimitive, Primitive, RectPrimitive, ShadowPrimitive, Stroke, TextLayoutId,
        TextPrimitive, TextureId, TexturePrimitive, Transform,
    };
    use crate::{Point, Rect, Size, Vec2};

    #[test]
    fn constructs_color_and_brush_values() {
        let color = Color::rgb(0.1, 0.2, 0.3).with_alpha(0.4);

        assert_eq!(color, Color::rgba(0.1, 0.2, 0.3, 0.4));
        assert_eq!(Brush::Solid(color), Brush::Solid(color));
    }

    #[test]
    fn constructs_linear_gradient_brushes() {
        let gradient = LinearGradient::between(
            Point::new(0.0, 1.0),
            Point::new(10.0, 11.0),
            Color::BLACK,
            Color::WHITE,
        );

        assert_eq!(gradient.start(), Point::new(0.0, 1.0));
        assert_eq!(gradient.end(), Point::new(10.0, 11.0));
        assert_eq!(gradient.stop_count(), 2);
        assert_eq!(gradient.stops()[0], GradientStop::new(0.0, Color::BLACK));
        assert_eq!(gradient.stops()[1], GradientStop::new(1.0, Color::WHITE));
        assert_eq!(
            Brush::LinearGradient(gradient),
            Brush::LinearGradient(gradient)
        );
    }

    #[test]
    fn builds_linear_gradient_from_evenly_spaced_colors() {
        let gradient = LinearGradient::from_colors(
            Point::new(0.0, 0.0),
            Point::new(20.0, 0.0),
            &[Color::BLACK, Color::rgb(0.5, 0.5, 0.5), Color::WHITE],
        )
        .expect("valid gradient");

        assert_eq!(gradient.stops()[0].offset, 0.0);
        assert_eq!(gradient.stops()[1].offset, 0.5);
        assert_eq!(gradient.stops()[2].offset, 1.0);
    }

    #[test]
    fn rejects_invalid_linear_gradient_stop_counts() {
        assert_eq!(
            LinearGradient::new(Point::new(0.0, 0.0), Point::new(1.0, 0.0), &[]),
            Err(GradientBuildError::TooFewStops)
        );

        let stops = [GradientStop::new(0.0, Color::BLACK); MAX_GRADIENT_STOPS + 1];
        assert_eq!(
            LinearGradient::new(Point::new(0.0, 0.0), Point::new(1.0, 0.0), &stops),
            Err(GradientBuildError::TooManyStops)
        );
    }

    #[test]
    fn constructs_stroke_and_radius_values() {
        let stroke = Stroke::new(1.5, Brush::Solid(Color::WHITE));

        assert_eq!(stroke.width, 1.5);
        assert_eq!(CornerRadius::all(4.0).top_left, 4.0);
    }

    #[test]
    fn constructs_shadow_primitives() {
        let shadow = ShadowPrimitive::new(
            Rect::new(1.0, 2.0, 30.0, 40.0),
            Vec2::new(3.0, 4.0),
            12.0,
            2.0,
            6.0,
            Color::rgba(0.0, 0.0, 0.0, 0.35),
        );

        assert_eq!(shadow.rect, Rect::new(1.0, 2.0, 30.0, 40.0));
        assert_eq!(shadow.offset, Vec2::new(3.0, 4.0));
        assert_eq!(shadow.blur_radius, 12.0);
        assert_eq!(shadow.spread, 2.0);
        assert_eq!(shadow.radius, 6.0);
    }

    #[test]
    fn resource_handles_are_stable() {
        assert_eq!(IconId::from_raw(5).raw(), 5);
        assert_eq!(ImageId::from_raw(7).raw(), 7);
        assert_eq!(TextureId::from_raw(9).raw(), 9);
        assert_eq!(TextLayoutId::from_raw(11).raw(), 11);
        assert_ne!(ImageId::from_raw(7).raw(), TextureId::from_raw(9).raw());
    }

    #[test]
    fn creates_translation_transform() {
        let transform = Transform::translation(Vec2::new(3.0, 4.0));

        assert_eq!(transform.dx, 3.0);
        assert_eq!(transform.dy, 4.0);
        assert_eq!(transform.m11, 1.0);
    }

    #[test]
    fn primitive_sequence_preserves_order() {
        let primitives = [
            Primitive::LayerBegin {
                id: LayerId::from_raw(1),
            },
            Primitive::ClipBegin {
                id: ClipId::from_raw(2),
                rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            },
            Primitive::Rect(RectPrimitive {
                rect: Rect::new(1.0, 1.0, 8.0, 8.0),
                fill: Some(Brush::Solid(Color::BLACK)),
                stroke: None,
                radius: CornerRadius::all(2.0),
            }),
            Primitive::ClipEnd {
                id: ClipId::from_raw(2),
            },
            Primitive::LayerEnd {
                id: LayerId::from_raw(1),
            },
        ];

        assert!(matches!(primitives[0], Primitive::LayerBegin { .. }));
        assert!(matches!(primitives[1], Primitive::ClipBegin { .. }));
        assert!(matches!(primitives[2], Primitive::Rect(_)));
        assert!(matches!(primitives[3], Primitive::ClipEnd { .. }));
        assert!(matches!(primitives[4], Primitive::LayerEnd { .. }));
    }

    #[test]
    fn creates_text_image_texture_line_path_and_shadow_primitives() {
        let stroke = Stroke::new(1.0, Brush::Solid(Color::WHITE));

        let line = Primitive::Line(LinePrimitive {
            from: Point::new(0.0, 0.0),
            to: Point::new(1.0, 1.0),
            stroke,
        });
        let shadow = Primitive::Shadow(ShadowPrimitive::new(
            Rect::new(0.0, 0.0, 10.0, 10.0),
            Vec2::new(0.0, 2.0),
            6.0,
            1.0,
            4.0,
            Color::rgba(0.0, 0.0, 0.0, 0.3),
        ));
        let text = Primitive::Text(TextPrimitive {
            layout: Some(TextLayoutId::from_raw(3)),
            origin: Point::new(1.0, 2.0),
            text: "Label".to_owned(),
            family: "sans-serif".to_owned(),
            size: 12.0,
            line_height: 16.0,
            brush: Brush::Solid(Color::WHITE),
        });
        let path = Primitive::Path(PathPrimitive::new(
            vec![
                PathElement::MoveTo(Point::new(0.0, 0.0)),
                PathElement::LineTo(Point::new(10.0, 0.0)),
                PathElement::QuadTo {
                    ctrl: Point::new(12.0, 4.0),
                    to: Point::new(10.0, 8.0),
                },
                PathElement::CubicTo {
                    ctrl1: Point::new(8.0, 10.0),
                    ctrl2: Point::new(2.0, 10.0),
                    to: Point::new(0.0, 8.0),
                },
                PathElement::Close,
            ],
            Some(Brush::Solid(Color::BLACK)),
            Some(stroke),
        ));
        let image = Primitive::Image(ImagePrimitive {
            image: ImageId::from_raw(1),
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            tint: None,
        });
        let texture = Primitive::Texture(TexturePrimitive {
            texture: TextureId::from_raw(2),
            rect: Rect::new(0.0, 0.0, 20.0, 10.0),
            source_size: Size::new(1920.0, 1080.0),
        });

        assert!(matches!(line, Primitive::Line(_)));
        assert!(matches!(shadow, Primitive::Shadow(_)));
        assert!(matches!(text, Primitive::Text(_)));
        assert!(matches!(path, Primitive::Path(_)));
        assert!(matches!(image, Primitive::Image(_)));
        assert!(matches!(texture, Primitive::Texture(_)));
    }
}
