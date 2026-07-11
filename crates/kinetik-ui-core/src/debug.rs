//! Debug inspection models for renderer-neutral UI diagnostics.

use crate::runtime::spatial::SpatialStack;
use crate::{ClipId, FrameMetrics, LayerId, Point, Primitive, Rect, WidgetId};

/// Severity for structured frame diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticSeverity {
    /// Recoverable runtime issue reported as a frame warning.
    Warning,
}

/// Stable diagnostic grouping for downstream filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticCategory {
    /// Canonical input stream and compatibility projection issues.
    Input,
    /// Widget identity and stable ID issues.
    Identity,
    /// Accessibility semantic tree validation issues.
    SemanticTree,
    /// Unbalanced render primitive stack issues.
    PrimitiveStack,
}

/// Structured diagnostic source location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticLocation {
    /// The frame's canonical input stream.
    InputStream,
    /// A widget identity.
    Widget(WidgetId),
    /// A clip primitive stack scope.
    Clip(ClipId),
    /// A layer primitive stack scope.
    Layer(LayerId),
    /// The transform primitive stack.
    TransformStack,
    /// The frame semantic tree.
    SemanticTree,
}

/// Stable diagnostic metadata derived from frame warnings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FrameDiagnostic {
    /// Stable Kinetik-owned diagnostic code.
    pub code: &'static str,
    /// Diagnostic severity.
    pub severity: DiagnosticSeverity,
    /// Diagnostic category.
    pub category: DiagnosticCategory,
    /// Structured diagnostic location.
    pub location: DiagnosticLocation,
}

/// Primitive category used by debug tooling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrimitiveKind {
    /// Rectangle primitive.
    Rect,
    /// Line primitive.
    Line,
    /// Shadow primitive.
    Shadow,
    /// Path primitive.
    Path,
    /// Text primitive.
    Text,
    /// Image primitive.
    Image,
    /// Texture primitive.
    Texture,
    /// Clip begin command.
    ClipBegin,
    /// Clip end command.
    ClipEnd,
    /// Layer begin command.
    LayerBegin,
    /// Layer end command.
    LayerEnd,
    /// Transform begin command.
    TransformBegin,
    /// Transform end command.
    TransformEnd,
}

/// Debug inspection row for one primitive.
#[derive(Debug, Clone, PartialEq)]
pub struct PrimitiveInspection {
    /// Primitive index in the submitted stream.
    pub index: usize,
    /// Primitive category.
    pub kind: PrimitiveKind,
    /// Screen-logical bounds after resolving active transforms and clips.
    pub bounds: Option<Rect>,
    /// Human-readable summary.
    pub summary: String,
}

/// Returns the primitive kind.
#[must_use]
pub const fn primitive_kind(primitive: &Primitive) -> PrimitiveKind {
    match primitive {
        Primitive::Rect(_) => PrimitiveKind::Rect,
        Primitive::Line(_) => PrimitiveKind::Line,
        Primitive::Shadow(_) => PrimitiveKind::Shadow,
        Primitive::Path(_) => PrimitiveKind::Path,
        Primitive::Text(_) => PrimitiveKind::Text,
        Primitive::Image(_) => PrimitiveKind::Image,
        Primitive::Texture(_) => PrimitiveKind::Texture,
        Primitive::ClipBegin { .. } => PrimitiveKind::ClipBegin,
        Primitive::ClipEnd { .. } => PrimitiveKind::ClipEnd,
        Primitive::LayerBegin { .. } => PrimitiveKind::LayerBegin,
        Primitive::LayerEnd { .. } => PrimitiveKind::LayerEnd,
        Primitive::TransformBegin(_) => PrimitiveKind::TransformBegin,
        Primitive::TransformEnd => PrimitiveKind::TransformEnd,
    }
}

/// Returns bounds for primitives that carry explicit geometry.
#[must_use]
pub fn primitive_bounds(primitive: &Primitive) -> Option<Rect> {
    match primitive {
        Primitive::Rect(primitive) => Some(primitive.rect),
        Primitive::Image(primitive) => Some(primitive.rect),
        Primitive::Texture(primitive) => Some(primitive.rect),
        Primitive::ClipBegin { rect, .. } => Some(*rect),
        Primitive::Line(primitive) => Some(line_bounds(primitive.from, primitive.to)),
        Primitive::Shadow(primitive) => Some(shadow_bounds(primitive)),
        Primitive::Path(primitive) => path_bounds(&primitive.elements),
        Primitive::Text(_)
        | Primitive::ClipEnd { .. }
        | Primitive::LayerBegin { .. }
        | Primitive::LayerEnd { .. }
        | Primitive::TransformBegin(_)
        | Primitive::TransformEnd => None,
    }
}

fn shadow_bounds(primitive: &crate::ShadowPrimitive) -> Rect {
    primitive
        .rect
        .translate(primitive.offset)
        .outset(primitive.spread + primitive.blur_radius.max(0.0) * 2.5)
        .max_zero()
}

fn line_bounds(from: Point, to: Point) -> Rect {
    let x = from.x.min(to.x);
    let y = from.y.min(to.y);
    Rect::new(x, y, from.x.max(to.x) - x, from.y.max(to.y) - y)
}

fn path_bounds(elements: &[crate::PathElement]) -> Option<Rect> {
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    let mut saw_point = false;
    for point in elements.iter().flat_map(path_element_points) {
        saw_point = true;
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }
    saw_point.then_some(Rect::new(min_x, min_y, max_x - min_x, max_y - min_y))
}

fn path_element_points(element: &crate::PathElement) -> Vec<Point> {
    match *element {
        crate::PathElement::MoveTo(point) | crate::PathElement::LineTo(point) => vec![point],
        crate::PathElement::QuadTo { ctrl, to } => vec![ctrl, to],
        crate::PathElement::CubicTo { ctrl1, ctrl2, to } => vec![ctrl1, ctrl2, to],
        crate::PathElement::Close => Vec::new(),
    }
}

/// Builds primitive inspection rows.
#[must_use]
pub fn inspect_primitives(primitives: &[Primitive]) -> Vec<PrimitiveInspection> {
    let mut spatial = SpatialStack::default();
    let mut inspections = Vec::with_capacity(primitives.len());
    for (index, primitive) in primitives.iter().enumerate() {
        let bounds = match primitive {
            Primitive::ClipBegin { .. } => {
                spatial.observe_primitive(primitive);
                spatial.effective_clip_bounds()
            }
            Primitive::TransformBegin(_)
            | Primitive::LayerBegin { .. }
            | Primitive::ClipEnd { .. }
            | Primitive::TransformEnd
            | Primitive::LayerEnd { .. } => {
                spatial.observe_primitive(primitive);
                None
            }
            Primitive::Rect(_)
            | Primitive::Line(_)
            | Primitive::Shadow(_)
            | Primitive::Path(_)
            | Primitive::Text(_)
            | Primitive::Image(_)
            | Primitive::Texture(_) => {
                primitive_bounds(primitive).and_then(|bounds| spatial.project_rect(bounds))
            }
        };
        inspections.push(PrimitiveInspection {
            index,
            kind: primitive_kind(primitive),
            bounds,
            summary: primitive_summary(primitive),
        });
    }
    inspections
}

fn primitive_summary(primitive: &Primitive) -> String {
    match primitive {
        Primitive::Rect(primitive) => format!("rect {:?}", primitive.rect),
        Primitive::Line(primitive) => format!("line {:?} -> {:?}", primitive.from, primitive.to),
        Primitive::Shadow(primitive) => format!("shadow {:?}", primitive.rect),
        Primitive::Path(primitive) => format!("path {} elements", primitive.elements.len()),
        Primitive::Text(primitive) => format!("text {:?}", primitive.text),
        Primitive::Image(primitive) => format!("image {}", primitive.image.raw()),
        Primitive::Texture(primitive) => format!("texture {}", primitive.texture.raw()),
        Primitive::ClipBegin { id, .. } => format!("clip begin {id:?}"),
        Primitive::ClipEnd { id } => format!("clip end {id:?}"),
        Primitive::LayerBegin { id } => format!("layer begin {id:?}"),
        Primitive::LayerEnd { id } => format!("layer end {id:?}"),
        Primitive::TransformBegin(transform) => format!("transform {transform:?}"),
        Primitive::TransformEnd => "transform end".to_owned(),
    }
}

/// Data-only debug overlay model.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DebugOverlay {
    /// Whether the overlay should be shown.
    pub visible: bool,
    /// Latest frame metrics.
    pub metrics: Option<FrameMetrics>,
    /// Primitive inspection rows.
    pub primitives: Vec<PrimitiveInspection>,
}

impl DebugOverlay {
    /// Creates a hidden debug overlay.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an overlay snapshot from metrics and primitives.
    #[must_use]
    pub fn from_frame(metrics: FrameMetrics, primitives: &[Primitive]) -> Self {
        Self {
            visible: true,
            metrics: Some(metrics),
            primitives: inspect_primitives(primitives),
        }
    }

    /// Returns compact overlay rows for display by any renderer or widget layer.
    #[must_use]
    pub fn rows(&self) -> Vec<String> {
        let mut rows = Vec::new();
        if let Some(metrics) = self.metrics {
            rows.push(format!(
                "frame {}: {:?}",
                metrics.frame_index,
                metrics.timings.total()
            ));
            rows.push(format!("primitives: {}", metrics.counters.primitives));
        }
        rows.extend(
            self.primitives
                .iter()
                .map(|primitive| format!("#{} {}", primitive.index, primitive.summary)),
        );
        rows
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{DebugOverlay, PrimitiveKind, inspect_primitives, primitive_bounds};
    use crate::{
        Brush, ClipId, Color, CornerRadius, FrameCounters, FrameMetrics, FrameTimings,
        LinePrimitive, PathElement, PathPrimitive, Point, Primitive, Rect, RectPrimitive,
        ShadowPrimitive, Stroke, Transform, Vec2,
    };

    #[test]
    fn primitive_inspection_reports_kind_bounds_and_summary() {
        let primitives = vec![
            Primitive::Rect(RectPrimitive {
                rect: Rect::new(0.0, 0.0, 10.0, 10.0),
                fill: Some(Brush::Solid(Color::BLACK)),
                stroke: None,
                radius: CornerRadius::all(0.0),
            }),
            Primitive::Line(LinePrimitive {
                from: Point::new(2.0, 4.0),
                to: Point::new(8.0, 12.0),
                stroke: Stroke::new(1.0, Brush::Solid(Color::WHITE)),
            }),
            Primitive::Shadow(ShadowPrimitive::new(
                Rect::new(10.0, 10.0, 20.0, 20.0),
                Vec2::new(1.0, 2.0),
                4.0,
                2.0,
                5.0,
                Color::rgba(0.0, 0.0, 0.0, 0.25),
            )),
            Primitive::Path(PathPrimitive::new(
                vec![
                    PathElement::MoveTo(Point::new(1.0, 2.0)),
                    PathElement::LineTo(Point::new(5.0, 2.0)),
                    PathElement::QuadTo {
                        ctrl: Point::new(7.0, 4.0),
                        to: Point::new(5.0, 6.0),
                    },
                ],
                None,
                Some(Stroke::new(1.0, Brush::Solid(Color::WHITE))),
            )),
        ];

        let rows = inspect_primitives(&primitives);

        assert_eq!(rows[0].kind, PrimitiveKind::Rect);
        assert_eq!(rows[0].bounds, Some(Rect::new(0.0, 0.0, 10.0, 10.0)));
        assert_eq!(
            primitive_bounds(&primitives[1]),
            Some(Rect::new(2.0, 4.0, 6.0, 8.0))
        );
        assert_eq!(rows[2].kind, PrimitiveKind::Shadow);
        assert_eq!(
            primitive_bounds(&primitives[2]),
            Some(Rect::new(-1.0, 0.0, 44.0, 44.0))
        );
        assert_eq!(rows[3].kind, PrimitiveKind::Path);
        assert_eq!(
            primitive_bounds(&primitives[3]),
            Some(Rect::new(1.0, 2.0, 6.0, 4.0))
        );
        assert!(rows[1].summary.contains("line"));
        assert!(rows[2].summary.contains("shadow"));
        assert!(rows[3].summary.contains("path"));
    }

    #[test]
    fn debug_overlay_builds_display_rows() {
        let metrics = FrameMetrics::new(
            3,
            FrameTimings {
                render: Duration::from_millis(2),
                ..FrameTimings::default()
            },
            FrameCounters {
                primitives: 1,
                ..FrameCounters::default()
            },
        );
        let primitive = Primitive::Rect(RectPrimitive {
            rect: Rect::new(0.0, 0.0, 1.0, 1.0),
            fill: None,
            stroke: None,
            radius: CornerRadius::all(0.0),
        });
        let overlay = DebugOverlay::from_frame(metrics, &[primitive]);

        assert!(overlay.visible);
        assert!(overlay.rows().iter().any(|row| row.contains("frame 3")));
        assert!(
            overlay
                .rows()
                .iter()
                .any(|row| row.contains("primitives: 1"))
        );
    }

    #[test]
    fn primitive_inspection_resolves_transform_clip_and_singular_scope_bounds() {
        let clip = ClipId::from_raw(9);
        let rect = |bounds| {
            Primitive::Rect(RectPrimitive {
                rect: bounds,
                fill: Some(Brush::Solid(Color::WHITE)),
                stroke: None,
                radius: CornerRadius::all(0.0),
            })
        };
        let primitives = vec![
            Primitive::TransformBegin(Transform::translation(Vec2::new(10.0, 20.0))),
            Primitive::ClipBegin {
                id: clip,
                rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            },
            rect(Rect::new(5.0, 5.0, 10.0, 10.0)),
            Primitive::ClipEnd { id: clip },
            Primitive::TransformEnd,
            Primitive::TransformBegin(Transform::scale(Vec2::new(0.0, 1.0))),
            rect(Rect::new(0.0, 0.0, 5.0, 5.0)),
            Primitive::TransformEnd,
            rect(Rect::new(1.0, 2.0, 3.0, 4.0)),
        ];

        let rows = inspect_primitives(&primitives);

        assert_eq!(rows[1].bounds, Some(Rect::new(10.0, 20.0, 10.0, 10.0)));
        assert_eq!(rows[2].bounds, Some(Rect::new(15.0, 25.0, 5.0, 5.0)));
        assert_eq!(rows[6].bounds, None);
        assert_eq!(rows[8].bounds, Some(Rect::new(1.0, 2.0, 3.0, 4.0)));
    }
}
