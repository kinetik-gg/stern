//! Backend-independent vector path data and styles.

use std::ops::Deref;

use crate::{Brush, Point};

/// Rule used to determine which regions of a path are filled.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum FillRule {
    /// Fill regions with a non-zero winding number.
    #[default]
    NonZero,
    /// Fill regions crossed by an odd number of path segments.
    EvenOdd,
}

/// Shape placed at the open ends of stroked paths.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum StrokeCap {
    /// Stop the stroke at the endpoint.
    #[default]
    Butt,
    /// Extend the endpoint with a semicircle.
    Round,
    /// Extend the endpoint with a half-width square.
    Square,
}

/// Shape used where stroked path segments meet.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum StrokeJoin {
    /// Extend segment edges until they meet.
    #[default]
    Miter,
    /// Join segments with a circular arc.
    Round,
    /// Join segments with a bevel.
    Bevel,
}

/// Stroke style.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stroke {
    /// Stroke width in logical units.
    pub width: f32,
    /// Stroke brush.
    pub brush: Brush,
    /// Shape placed at open path ends.
    pub cap: StrokeCap,
    /// Shape used where path segments meet.
    pub join: StrokeJoin,
}

impl Stroke {
    /// Creates a stroke with butt caps and miter joins.
    #[must_use]
    pub const fn new(width: f32, brush: Brush) -> Self {
        Self {
            width,
            brush,
            cap: StrokeCap::Butt,
            join: StrokeJoin::Miter,
        }
    }

    /// Replaces the stroke cap.
    #[must_use]
    pub const fn with_cap(mut self, cap: StrokeCap) -> Self {
        self.cap = cap;
        self
    }

    /// Replaces the stroke join.
    #[must_use]
    pub const fn with_join(mut self, join: StrokeJoin) -> Self {
        self.join = join;
        self
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

/// Owned or immutable static path geometry.
#[derive(Debug, Clone)]
pub enum PathData {
    /// Heap-owned path geometry.
    Owned(Vec<PathElement>),
    /// Borrowed immutable path geometry retained for the program lifetime.
    Static(&'static [PathElement]),
}

impl PathData {
    /// Creates borrowed static path data.
    #[must_use]
    pub const fn from_static(elements: &'static [PathElement]) -> Self {
        Self::Static(elements)
    }

    /// Returns the path elements.
    #[must_use]
    pub fn as_slice(&self) -> &[PathElement] {
        match self {
            Self::Owned(elements) => elements,
            Self::Static(elements) => elements,
        }
    }

    /// Returns true when the geometry is borrowed static data.
    #[must_use]
    pub const fn is_static(&self) -> bool {
        matches!(self, Self::Static(_))
    }
}

impl PartialEq for PathData {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl PartialEq<Vec<PathElement>> for PathData {
    fn eq(&self, other: &Vec<PathElement>) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl PartialEq<PathData> for Vec<PathElement> {
    fn eq(&self, other: &PathData) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl Deref for PathData {
    type Target = [PathElement];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl AsRef<[PathElement]> for PathData {
    fn as_ref(&self) -> &[PathElement] {
        self.as_slice()
    }
}

impl<'a> IntoIterator for &'a PathData {
    type Item = &'a PathElement;
    type IntoIter = core::slice::Iter<'a, PathElement>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}

impl From<Vec<PathElement>> for PathData {
    fn from(elements: Vec<PathElement>) -> Self {
        Self::Owned(elements)
    }
}

impl<const N: usize> From<[PathElement; N]> for PathData {
    fn from(elements: [PathElement; N]) -> Self {
        Self::Owned(elements.into())
    }
}

impl From<&'static [PathElement]> for PathData {
    fn from(elements: &'static [PathElement]) -> Self {
        Self::Static(elements)
    }
}

/// Vector path draw command.
#[derive(Debug, Clone, PartialEq)]
pub struct PathPrimitive {
    /// Path elements in drawing order.
    pub elements: PathData,
    /// Fill brush.
    pub fill: Option<Brush>,
    /// Stroke style.
    pub stroke: Option<Stroke>,
    /// Fill winding rule.
    pub fill_rule: FillRule,
    /// Opacity applied to both fill and stroke.
    pub opacity: f32,
}

impl PathPrimitive {
    /// Creates an owned or static path with non-zero fill and full opacity.
    #[must_use]
    pub fn new(elements: impl Into<PathData>, fill: Option<Brush>, stroke: Option<Stroke>) -> Self {
        Self {
            elements: elements.into(),
            fill,
            stroke,
            fill_rule: FillRule::NonZero,
            opacity: 1.0,
        }
    }

    /// Creates a path backed directly by immutable static geometry.
    #[must_use]
    pub const fn from_static(
        elements: &'static [PathElement],
        fill: Option<Brush>,
        stroke: Option<Stroke>,
    ) -> Self {
        Self {
            elements: PathData::Static(elements),
            fill,
            stroke,
            fill_rule: FillRule::NonZero,
            opacity: 1.0,
        }
    }

    /// Replaces the fill rule.
    #[must_use]
    pub const fn with_fill_rule(mut self, fill_rule: FillRule) -> Self {
        self.fill_rule = fill_rule;
        self
    }

    /// Replaces the path opacity.
    #[must_use]
    pub const fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::{FillRule, PathData, PathElement, PathPrimitive, Stroke, StrokeCap, StrokeJoin};
    use crate::{Brush, Color, Point, Primitive};

    static STATIC_PATH: [PathElement; 2] = [
        PathElement::MoveTo(Point::new(0.0, 0.0)),
        PathElement::LineTo(Point::new(8.0, 8.0)),
    ];

    #[test]
    fn static_path_clone_retains_borrowed_storage() {
        let path = PathPrimitive::from_static(&STATIC_PATH, None, None)
            .with_fill_rule(FillRule::EvenOdd)
            .with_opacity(0.5);
        let cloned = path.clone();

        assert!(cloned.elements.is_static());
        let PathData::Static(elements) = cloned.elements else {
            panic!("expected static path data");
        };
        assert!(core::ptr::eq(elements, STATIC_PATH.as_slice()));
        assert_eq!(cloned.fill_rule, FillRule::EvenOdd);
        assert_eq!(cloned.opacity, 0.5);
    }

    #[test]
    fn primitive_snapshot_clone_retains_static_geometry() {
        let snapshot = vec![Primitive::Path(PathPrimitive::from_static(
            &STATIC_PATH,
            Some(Brush::Solid(Color::WHITE)),
            None,
        ))];
        let cloned = snapshot.clone();

        let Primitive::Path(path) = &cloned[0] else {
            unreachable!()
        };
        let PathData::Static(elements) = &path.elements else {
            panic!("expected static path data");
        };
        assert!(core::ptr::eq(*elements, STATIC_PATH.as_slice()));
    }

    #[test]
    fn owned_and_static_geometry_compare_by_elements() {
        assert_eq!(
            PathData::Owned(STATIC_PATH.to_vec()),
            PathData::Static(&STATIC_PATH)
        );
    }

    #[test]
    fn stroke_preserves_cap_and_join() {
        let stroke = Stroke::new(2.0, Brush::Solid(Color::WHITE))
            .with_cap(StrokeCap::Round)
            .with_join(StrokeJoin::Bevel);

        assert_eq!(stroke.cap, StrokeCap::Round);
        assert_eq!(stroke.join, StrokeJoin::Bevel);
    }
}
