//! Debug inspection models for renderer-neutral UI diagnostics.

use crate::{FrameMetrics, Point, Primitive, Rect};

/// Primitive category used by debug tooling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrimitiveKind {
    /// Rectangle primitive.
    Rect,
    /// Line primitive.
    Line,
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
    /// Bounds when the primitive has explicit geometry.
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
        Primitive::Text(_)
        | Primitive::ClipEnd { .. }
        | Primitive::LayerBegin { .. }
        | Primitive::LayerEnd { .. }
        | Primitive::TransformBegin(_)
        | Primitive::TransformEnd => None,
    }
}

fn line_bounds(from: Point, to: Point) -> Rect {
    let x = from.x.min(to.x);
    let y = from.y.min(to.y);
    Rect::new(x, y, from.x.max(to.x) - x, from.y.max(to.y) - y)
}

/// Builds primitive inspection rows.
#[must_use]
pub fn inspect_primitives(primitives: &[Primitive]) -> Vec<PrimitiveInspection> {
    primitives
        .iter()
        .enumerate()
        .map(|(index, primitive)| PrimitiveInspection {
            index,
            kind: primitive_kind(primitive),
            bounds: primitive_bounds(primitive),
            summary: primitive_summary(primitive),
        })
        .collect()
}

fn primitive_summary(primitive: &Primitive) -> String {
    match primitive {
        Primitive::Rect(primitive) => format!("rect {:?}", primitive.rect),
        Primitive::Line(primitive) => format!("line {:?} -> {:?}", primitive.from, primitive.to),
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
        Brush, Color, CornerRadius, FrameCounters, FrameMetrics, FrameTimings, LinePrimitive,
        Point, Primitive, Rect, RectPrimitive, Stroke,
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
        ];

        let rows = inspect_primitives(&primitives);

        assert_eq!(rows[0].kind, PrimitiveKind::Rect);
        assert_eq!(rows[0].bounds, Some(Rect::new(0.0, 0.0, 10.0, 10.0)));
        assert_eq!(
            primitive_bounds(&primitives[1]),
            Some(Rect::new(2.0, 4.0, 6.0, 8.0))
        );
        assert!(rows[1].summary.contains("line"));
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
}
