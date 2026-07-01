#[allow(clippy::wildcard_imports)]
use super::*;

pub(crate) fn fit_scale(source: Size, bounds: Size) -> f32 {
    let Some(source_width) = finite_positive(source.width) else {
        return 0.0;
    };
    let Some(source_height) = finite_positive(source.height) else {
        return 0.0;
    };
    let Some(bounds_width) = finite_positive(bounds.width) else {
        return 0.0;
    };
    let Some(bounds_height) = finite_positive(bounds.height) else {
        return 0.0;
    };
    (bounds_width / source_width).min(bounds_height / source_height)
}

pub(crate) fn fill_scale(source: Size, bounds: Size) -> f32 {
    let Some(source_width) = finite_positive(source.width) else {
        return 0.0;
    };
    let Some(source_height) = finite_positive(source.height) else {
        return 0.0;
    };
    let Some(bounds_width) = finite_positive(bounds.width) else {
        return 0.0;
    };
    let Some(bounds_height) = finite_positive(bounds.height) else {
        return 0.0;
    };
    (bounds_width / source_width).max(bounds_height / source_height)
}

pub(crate) fn finite_or_zero(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

pub(crate) fn finite_or_none(value: f32) -> Option<f32> {
    value.is_finite().then_some(value)
}

pub(crate) fn finite_non_negative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

pub(crate) fn finite_positive(value: f32) -> Option<f32> {
    value
        .is_finite()
        .then_some(value)
        .filter(|value| *value > 0.0)
}

pub(crate) fn finite_point(point: Point) -> Option<Point> {
    (point.x.is_finite() && point.y.is_finite()).then_some(point)
}

pub(crate) fn finite_positive_rect(rect: Rect) -> Option<Rect> {
    (rect.x.is_finite()
        && rect.y.is_finite()
        && rect.width.is_finite()
        && rect.height.is_finite()
        && rect.width > 0.0
        && rect.height > 0.0)
        .then_some(rect)
}

pub(crate) fn sanitize_rect(rect: Rect) -> Rect {
    Rect::new(
        finite_or_zero(rect.x),
        finite_or_zero(rect.y),
        finite_non_negative(rect.width),
        finite_non_negative(rect.height),
    )
}

pub(crate) fn finite_content_guide_position(
    surface: ViewportSurface,
    orientation: ViewportGuideOrientation,
    position: f32,
) -> Option<f32> {
    let position = finite_or_none(position)?;
    let source = surface.effective_source_size()?;
    let max = match orientation {
        ViewportGuideOrientation::Horizontal => source.height,
        ViewportGuideOrientation::Vertical => source.width,
    };
    (position >= 0.0 && position <= max).then_some(position)
}

pub(crate) fn guide_position_inside_bounds(
    bounds: Rect,
    orientation: ViewportGuideOrientation,
    position: f32,
) -> bool {
    match orientation {
        ViewportGuideOrientation::Horizontal => position >= bounds.y && position <= bounds.max_y(),
        ViewportGuideOrientation::Vertical => position >= bounds.x && position <= bounds.max_x(),
    }
}

pub(crate) fn guide_screen_rect(
    bounds: Rect,
    orientation: ViewportGuideOrientation,
    position: f32,
) -> Option<Rect> {
    if !position.is_finite() || !guide_position_inside_bounds(bounds, orientation, position) {
        return None;
    }
    let rect = match orientation {
        ViewportGuideOrientation::Horizontal => {
            Rect::new(bounds.x, position - 0.5, bounds.width, 1.0)
        }
        ViewportGuideOrientation::Vertical => {
            Rect::new(position - 0.5, bounds.y, 1.0, bounds.height)
        }
    };
    finite_positive_rect(rect)
}

pub(crate) fn guide_sort_key(guide: &ViewportResolvedGuide) -> f32 {
    match guide.placement {
        ViewportGuidePlacement::Content(position) | ViewportGuidePlacement::Screen(position) => {
            position
        }
    }
}

pub(crate) fn visible_ruler_content_range(
    surface: ViewportSurface,
    edge: ViewportRulerEdge,
    scale_factor: ScaleFactor,
) -> Option<(f32, f32)> {
    let bounds = surface.effective_bounds();
    let source = surface.effective_source_size()?;
    let (screen_min, screen_max, content_max) = match edge {
        ViewportRulerEdge::Top => (
            Point::new(bounds.x, bounds.y),
            Point::new(bounds.max_x(), bounds.y),
            source.width,
        ),
        ViewportRulerEdge::Left => (
            Point::new(bounds.x, bounds.y),
            Point::new(bounds.x, bounds.max_y()),
            source.height,
        ),
    };
    let content_min = surface.screen_to_content_at(screen_min, scale_factor)?;
    let content_max_point = surface.screen_to_content_at(screen_max, scale_factor)?;
    let (start, end) = match edge {
        ViewportRulerEdge::Top => (content_min.x, content_max_point.x),
        ViewportRulerEdge::Left => (content_min.y, content_max_point.y),
    };
    let min = start.min(end).max(0.0);
    let max = start.max(end).min(content_max);
    (min.is_finite() && max.is_finite() && max > min).then_some((min, max))
}

pub(crate) fn ruler_axis_screen_position(
    surface: ViewportSurface,
    edge: ViewportRulerEdge,
    value: f32,
    scale_factor: ScaleFactor,
) -> Option<f32> {
    let value = finite_or_none(value)?;
    let point = match edge {
        ViewportRulerEdge::Top => {
            surface.content_to_screen_at(Point::new(value, 0.0), scale_factor)?
        }
        ViewportRulerEdge::Left => {
            surface.content_to_screen_at(Point::new(0.0, value), scale_factor)?
        }
    };
    let position = match edge {
        ViewportRulerEdge::Top => point.x,
        ViewportRulerEdge::Left => point.y,
    };
    finite_or_none(position)
}

pub(crate) fn viewport_ruler_ticks(
    surface: ViewportSurface,
    edge: ViewportRulerEdge,
    visible_content_range: (f32, f32),
    origin_content: f32,
    max_ticks: usize,
    scale_factor: ScaleFactor,
) -> Vec<ViewportRulerTick> {
    let scale = finite_positive(surface.content_scale_at(scale_factor)).unwrap_or(1.0);
    let mut ticks = ruler_ticks(visible_content_range.0, visible_content_range.1, scale)
        .into_iter()
        .filter(|value| {
            value.is_finite()
                && *value >= visible_content_range.0
                && *value <= visible_content_range.1
        })
        .take(max_ticks)
        .filter_map(|value| {
            let screen_position = ruler_axis_screen_position(surface, edge, value, scale_factor)?;
            let major = is_major_ruler_tick(value, origin_content);
            Some(ViewportRulerTick {
                value,
                screen_position,
                major,
                label: major.then(|| ruler_tick_label(value - origin_content)),
            })
        })
        .collect::<Vec<_>>();
    ticks.sort_by(|left, right| {
        left.value
            .total_cmp(&right.value)
            .then_with(|| left.screen_position.total_cmp(&right.screen_position))
    });
    ticks
}

pub(crate) fn is_major_ruler_tick(value: f32, origin_content: f32) -> bool {
    let relative = value - origin_content;
    if !relative.is_finite() {
        return false;
    }
    let rounded = (relative / 50.0).round();
    (relative / 50.0 - rounded).abs() <= 0.001
}

pub(crate) fn ruler_tick_label(value: f32) -> String {
    if (value - value.round()).abs() <= 0.001 {
        format!("{value:.0}")
    } else {
        format!("{value:.2}")
    }
}

pub(crate) const fn default_viewport_overlay_priority(kind: ViewportOverlayKind) -> i32 {
    match kind {
        ViewportOverlayKind::TextureSurface => 0,
        ViewportOverlayKind::ContentBounds => 10,
        ViewportOverlayKind::Guide => 20,
        ViewportOverlayKind::ToolRegion => 30,
    }
}

/// Viewport guide line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Guide {
    /// Horizontal guide at y.
    Horizontal(f32),
    /// Vertical guide at x.
    Vertical(f32),
}

/// Computes ruler tick positions.
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
pub fn ruler_ticks(start: f32, end: f32, zoom: f32) -> Vec<f32> {
    let Some(zoom) = finite_positive(zoom) else {
        return Vec::new();
    };
    if !start.is_finite() || !end.is_finite() {
        return Vec::new();
    }
    let min = start.min(end);
    let max = start.max(end);
    let span = max - min;
    if span <= 0.0 {
        return Vec::new();
    }
    let mut step = if zoom >= 2.0 {
        10.0
    } else if zoom >= 1.0 {
        25.0
    } else {
        50.0
    };

    let mut first = (min / step).floor() as i32;
    let mut last = (max / step).ceil() as i32;
    while last.saturating_sub(first) > 4096 {
        step *= 2.0;
        first = (min / step).floor() as i32;
        last = (max / step).ceil() as i32;
    }
    (first..=last).map(|index| index as f32 * step).collect()
}

/// Emits guide line primitives.
#[must_use]
pub fn guide_primitives(bounds: Rect, guides: &[Guide], color: Color) -> Vec<Primitive> {
    guides
        .iter()
        .map(|guide| match *guide {
            Guide::Horizontal(y) => Primitive::Line(LinePrimitive {
                from: Point::new(bounds.x, y),
                to: Point::new(bounds.max_x(), y),
                stroke: Stroke::new(1.0, Brush::Solid(color)),
            }),
            Guide::Vertical(x) => Primitive::Line(LinePrimitive {
                from: Point::new(x, bounds.y),
                to: Point::new(x, bounds.max_y()),
                stroke: Stroke::new(1.0, Brush::Solid(color)),
            }),
        })
        .collect()
}

/// Crosshair overlay state.
#[derive(Debug, Clone, PartialEq)]
pub struct Crosshair {
    /// Whether the crosshair is visible.
    pub visible: bool,
    /// Cursor position.
    pub position: Point,
    /// Optional label.
    pub label: Option<String>,
    /// Crosshair color.
    pub color: Color,
}

impl Crosshair {
    pub(crate) fn with_position(&self, position: Point) -> Self {
        Self {
            visible: self.visible,
            position,
            label: self.label.clone(),
            color: self.color,
        }
    }

    /// Emits crosshair primitives.
    #[must_use]
    pub fn primitives(&self, bounds: Rect) -> Vec<Primitive> {
        let bounds = Rect::new(
            finite_or_zero(bounds.x),
            finite_or_zero(bounds.y),
            finite_non_negative(bounds.width),
            finite_non_negative(bounds.height),
        );
        if !self.visible
            || finite_point(self.position).is_none()
            || !bounds.contains_point(self.position)
        {
            return Vec::new();
        }
        let mut primitives = vec![
            Primitive::Line(LinePrimitive {
                from: Point::new(bounds.x, self.position.y),
                to: Point::new(bounds.max_x(), self.position.y),
                stroke: Stroke::new(1.0, Brush::Solid(self.color)),
            }),
            Primitive::Line(LinePrimitive {
                from: Point::new(self.position.x, bounds.y),
                to: Point::new(self.position.x, bounds.max_y()),
                stroke: Stroke::new(1.0, Brush::Solid(self.color)),
            }),
        ];
        if let Some(label) = &self.label {
            primitives.push(Primitive::Text(TextPrimitive {
                layout: None,
                origin: Point::new(self.position.x + 6.0, self.position.y - 6.0),
                text: label.clone(),
                family: "sans-serif".to_owned(),
                size: 11.0,
                line_height: 15.0,
                brush: Brush::Solid(self.color),
            }));
        }
        primitives
    }
}

/// Viewport overlay composition request.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportComposition {
    /// Surface.
    pub surface: ViewportSurface,
    /// Guides.
    pub guides: Vec<Guide>,
    /// Crosshair.
    pub crosshair: Option<Crosshair>,
    /// Clip identity.
    pub clip: ClipId,
}

impl ViewportComposition {
    /// Emits primitives in deterministic viewport order.
    #[must_use]
    pub fn primitives(&self) -> Vec<Primitive> {
        self.primitives_at(ScaleFactor::ONE)
    }

    /// Emits primitives in deterministic viewport order for a viewport scale factor.
    #[must_use]
    pub fn primitives_at(&self, scale_factor: ScaleFactor) -> Vec<Primitive> {
        let mut primitives = vec![
            Primitive::ClipBegin {
                id: self.clip,
                rect: self.surface.bounds,
            },
            self.surface.texture_primitive_at(scale_factor),
        ];
        primitives.extend(self.surface.content_guide_primitives_at(
            &self.guides,
            Color::rgba(1.0, 1.0, 1.0, 0.35),
            scale_factor,
        ));
        if let Some(crosshair) = &self.crosshair {
            primitives.extend(
                self.surface
                    .content_crosshair_primitives_at(crosshair, scale_factor),
            );
        }
        primitives.push(Primitive::ClipEnd { id: self.clip });
        primitives
    }
}

#[allow(clippy::cast_possible_truncation)]
pub(crate) fn native_logical_pixel_scale(scale_factor: ScaleFactor) -> f32 {
    if scale_factor.is_valid() {
        (1.0 / scale_factor.value()) as f32
    } else {
        1.0
    }
}

pub(crate) fn snap_rect_to_scale(rect: Rect, scale_factor: ScaleFactor) -> Rect {
    if !rect.x.is_finite()
        || !rect.y.is_finite()
        || !rect.width.is_finite()
        || !rect.height.is_finite()
        || !scale_factor.is_valid()
        || rect.width < 0.0
        || rect.height < 0.0
    {
        return rect;
    }

    scale_factor.snap_rect_to_physical_grid(rect)
}
