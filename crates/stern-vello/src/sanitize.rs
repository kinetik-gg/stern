use stern_core::{
    Brush, Color, CornerRadius, LinearGradient, PathData, PathElement, Point, Rect,
    ShadowPrimitive, Size, Stroke, Transform, Vec2,
};
use stern_render::RenderDiagnostic;

pub(crate) fn brush_fallback_color(brush: &Brush) -> Color {
    match brush {
        Brush::Solid(color) => *color,
        Brush::LinearGradient(gradient) => gradient
            .stops()
            .first()
            .map_or(Color::TRANSPARENT, |stop| stop.color),
    }
}

pub(crate) fn finite_non_negative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

pub(crate) fn finite_or_zero(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

pub(crate) fn finite_positive(value: f32) -> Option<f32> {
    value
        .is_finite()
        .then_some(value)
        .filter(|value| *value > 0.0)
}

pub(crate) fn finite_unit(value: f32) -> f32 {
    let value = if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    };
    if value == 0.0 { 0.0 } else { value }
}

pub(crate) fn sanitize_point(
    point: Point,
    diagnostics: &mut Vec<RenderDiagnostic>,
    context: &'static str,
) -> Option<Point> {
    if point.x.is_finite() && point.y.is_finite() {
        Some(point)
    } else {
        diagnostics.push(RenderDiagnostic::InvalidGeometry(context));
        None
    }
}

pub(crate) fn sanitize_vec2(
    offset: Vec2,
    diagnostics: &mut Vec<RenderDiagnostic>,
    context: &'static str,
) -> Vec2 {
    if offset.x.is_finite() && offset.y.is_finite() {
        offset
    } else {
        diagnostics.push(RenderDiagnostic::InvalidGeometry(context));
        Vec2::new(finite_or_zero(offset.x), finite_or_zero(offset.y))
    }
}

pub(crate) fn sanitize_size(size: Size) -> Option<Size> {
    Some(Size::new(
        finite_positive(size.width)?,
        finite_positive(size.height)?,
    ))
}

pub(crate) fn sanitize_rect(
    rect: Rect,
    diagnostics: &mut Vec<RenderDiagnostic>,
    context: &'static str,
) -> Option<Rect> {
    let Some(width) = finite_positive(rect.width) else {
        diagnostics.push(RenderDiagnostic::InvalidGeometry(context));
        return None;
    };
    let Some(height) = finite_positive(rect.height) else {
        diagnostics.push(RenderDiagnostic::InvalidGeometry(context));
        return None;
    };
    let mut invalid = false;
    let x = if rect.x.is_finite() {
        rect.x
    } else {
        invalid = true;
        0.0
    };
    let y = if rect.y.is_finite() {
        rect.y
    } else {
        invalid = true;
        0.0
    };
    if invalid {
        diagnostics.push(RenderDiagnostic::InvalidGeometry(context));
    }
    Some(Rect::new(x, y, width, height))
}

pub(crate) fn sanitize_color(
    color: Color,
    diagnostics: &mut Vec<RenderDiagnostic>,
    context: &'static str,
) -> Color {
    let invalid = [color.r, color.g, color.b, color.a]
        .into_iter()
        .any(|channel| !channel.is_finite() || !(0.0..=1.0).contains(&channel));
    if invalid {
        diagnostics.push(RenderDiagnostic::InvalidGeometry(context));
    }
    Color::rgba(
        finite_unit(color.r),
        finite_unit(color.g),
        finite_unit(color.b),
        finite_unit(color.a),
    )
}

pub(crate) fn sanitize_brush(
    brush: Brush,
    diagnostics: &mut Vec<RenderDiagnostic>,
    context: &'static str,
) -> Brush {
    match brush {
        Brush::Solid(color) => Brush::Solid(sanitize_color(color, diagnostics, context)),
        Brush::LinearGradient(gradient) => sanitize_linear_gradient(gradient, diagnostics, context)
            .map(Brush::LinearGradient)
            .unwrap_or_else(|| {
                Brush::Solid(sanitize_color(
                    brush_fallback_color(&Brush::LinearGradient(gradient)),
                    diagnostics,
                    context,
                ))
            }),
    }
}

pub(crate) fn sanitize_linear_gradient(
    gradient: LinearGradient,
    diagnostics: &mut Vec<RenderDiagnostic>,
    context: &'static str,
) -> Option<LinearGradient> {
    let start = sanitize_point(gradient.start(), diagnostics, context)?;
    let end = sanitize_point(gradient.end(), diagnostics, context)?;
    if start == end {
        diagnostics.push(RenderDiagnostic::InvalidGeometry(context));
        return None;
    }

    let mut stops = gradient.stops().to_vec();
    if stops.len() < 2 {
        diagnostics.push(RenderDiagnostic::InvalidGeometry(context));
        return None;
    }
    for stop in &mut stops {
        if !stop.offset.is_finite() || !(0.0..=1.0).contains(&stop.offset) {
            diagnostics.push(RenderDiagnostic::InvalidGeometry(context));
        }
        stop.offset = finite_unit(stop.offset);
        stop.color = sanitize_color(stop.color, diagnostics, context);
    }
    stops.sort_by(|a, b| {
        a.offset
            .partial_cmp(&b.offset)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    LinearGradient::new(start, end, &stops).ok()
}

pub(crate) fn sanitize_stroke(
    stroke: Stroke,
    diagnostics: &mut Vec<RenderDiagnostic>,
    context: &'static str,
) -> Option<Stroke> {
    let Some(width) = finite_positive(stroke.width) else {
        diagnostics.push(RenderDiagnostic::InvalidGeometry(context));
        return None;
    };
    Some(
        Stroke::new(width, sanitize_brush(stroke.brush, diagnostics, context))
            .with_cap(stroke.cap)
            .with_join(stroke.join),
    )
}

pub(crate) fn sanitize_shadow(
    shadow: ShadowPrimitive,
    diagnostics: &mut Vec<RenderDiagnostic>,
) -> Option<ShadowPrimitive> {
    let rect = sanitize_rect(shadow.rect, diagnostics, "shadow")?;
    let offset = sanitize_vec2(shadow.offset, diagnostics, "shadow_offset");
    let blur_radius = sanitize_non_negative(shadow.blur_radius, diagnostics, "shadow_blur");
    let spread = sanitize_finite(shadow.spread, diagnostics, "shadow_spread");
    let radius = sanitize_non_negative(shadow.radius, diagnostics, "shadow_radius");
    let color = sanitize_color(shadow.color, diagnostics, "shadow_color");
    let shadow_rect = rect.translate(offset).outset(spread).max_zero();
    if shadow_rect.is_empty() {
        diagnostics.push(RenderDiagnostic::InvalidGeometry("shadow_spread"));
        return None;
    }
    Some(ShadowPrimitive::new(
        rect,
        offset,
        blur_radius,
        spread,
        radius,
        color,
    ))
}

pub(crate) fn sanitize_non_negative(
    value: f32,
    diagnostics: &mut Vec<RenderDiagnostic>,
    context: &'static str,
) -> f32 {
    if value.is_finite() && value >= 0.0 {
        value
    } else {
        diagnostics.push(RenderDiagnostic::InvalidGeometry(context));
        finite_non_negative(value)
    }
}

pub(crate) fn sanitize_finite(
    value: f32,
    diagnostics: &mut Vec<RenderDiagnostic>,
    context: &'static str,
) -> f32 {
    if value.is_finite() {
        value
    } else {
        diagnostics.push(RenderDiagnostic::InvalidGeometry(context));
        0.0
    }
}

pub(crate) fn sanitize_radius(
    radius: CornerRadius,
    diagnostics: &mut Vec<RenderDiagnostic>,
    context: &'static str,
) -> CornerRadius {
    let invalid = !radius.top_left.is_finite()
        || !radius.top_right.is_finite()
        || !radius.bottom_right.is_finite()
        || !radius.bottom_left.is_finite()
        || radius.top_left < 0.0
        || radius.top_right < 0.0
        || radius.bottom_right < 0.0
        || radius.bottom_left < 0.0;
    if invalid {
        diagnostics.push(RenderDiagnostic::InvalidGeometry(context));
    }
    CornerRadius {
        top_left: finite_non_negative(radius.top_left),
        top_right: finite_non_negative(radius.top_right),
        bottom_right: finite_non_negative(radius.bottom_right),
        bottom_left: finite_non_negative(radius.bottom_left),
    }
}

pub(crate) fn sanitize_path_elements(
    elements: &PathData,
    diagnostics: &mut Vec<RenderDiagnostic>,
    context: &'static str,
) -> Option<PathData> {
    if elements.is_empty() {
        diagnostics.push(RenderDiagnostic::InvalidGeometry(context));
        return None;
    }

    let mut saw_point = false;
    for element in elements.as_slice() {
        match *element {
            PathElement::MoveTo(point) | PathElement::LineTo(point) => {
                sanitize_point(point, diagnostics, context)?;
                saw_point = true;
            }
            PathElement::QuadTo { ctrl, to } => {
                sanitize_point(ctrl, diagnostics, context)?;
                sanitize_point(to, diagnostics, context)?;
                saw_point = true;
            }
            PathElement::CubicTo { ctrl1, ctrl2, to } => {
                sanitize_point(ctrl1, diagnostics, context)?;
                sanitize_point(ctrl2, diagnostics, context)?;
                sanitize_point(to, diagnostics, context)?;
                saw_point = true;
            }
            PathElement::Close => {}
        }
    }

    if saw_point {
        Some(elements.clone())
    } else {
        diagnostics.push(RenderDiagnostic::InvalidGeometry(context));
        None
    }
}

pub(crate) fn sanitize_opacity(
    opacity: f32,
    diagnostics: &mut Vec<RenderDiagnostic>,
    context: &'static str,
) -> f32 {
    if opacity.is_finite() && (0.0..=1.0).contains(&opacity) {
        opacity
    } else {
        diagnostics.push(RenderDiagnostic::InvalidGeometry(context));
        finite_unit(opacity)
    }
}

pub(crate) fn transform_is_finite(transform: Transform) -> bool {
    transform.m11.is_finite()
        && transform.m12.is_finite()
        && transform.m21.is_finite()
        && transform.m22.is_finite()
        && transform.dx.is_finite()
        && transform.dy.is_finite()
}

pub(crate) fn sanitize_transform(
    transform: Transform,
    diagnostics: &mut Vec<RenderDiagnostic>,
    context: &'static str,
) -> Option<Transform> {
    if transform_is_finite(transform) {
        Some(transform)
    } else {
        diagnostics.push(RenderDiagnostic::InvalidGeometry(context));
        None
    }
}
