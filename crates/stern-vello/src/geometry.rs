use stern_core::{Color, CornerRadius, LinearGradient, Point, Rect, Size, Transform, ViewportInfo};
use stern_render::RenderImageSampling;
use vello::{
    kurbo::{Affine, Rect as KurboRect, RoundedRect, RoundedRectRadii},
    peniko::{
        Gradient as PenikoGradient, InterpolationAlphaSpace,
        color::{AlphaColor, ColorSpaceTag, Srgb},
    },
};

const VIEWPORT_SCALE_EPSILON: f64 = 0.001;

pub(crate) fn transform_is_finite(transform: Transform) -> bool {
    transform.m11.is_finite()
        && transform.m12.is_finite()
        && transform.m21.is_finite()
        && transform.m22.is_finite()
        && transform.dx.is_finite()
        && transform.dy.is_finite()
}

#[allow(clippy::cast_possible_truncation)]
pub(crate) fn quantize_stroke_width_to_device(width: f32, device_scale: f64) -> f32 {
    if width <= 0.0 || !width.is_finite() || !device_scale.is_finite() || device_scale <= 0.0 {
        return width;
    }

    let physical_width = (f64::from(width) * device_scale).round().max(1.0);
    (physical_width / device_scale) as f32
}

pub(crate) fn logical_size_matches(lhs: Size, rhs: Size) -> bool {
    (lhs.width - rhs.width).abs() <= f32::EPSILON && (lhs.height - rhs.height).abs() <= f32::EPSILON
}

pub(crate) fn transform_to_affine(transform: Transform) -> Affine {
    Affine::new([
        f64::from(transform.m11),
        f64::from(transform.m12),
        f64::from(transform.m21),
        f64::from(transform.m22),
        f64::from(transform.dx),
        f64::from(transform.dy),
    ])
}

pub(crate) fn viewport_device_scale(viewport: ViewportInfo) -> f64 {
    if let Some(scale) = viewport_size_device_scale(viewport) {
        return scale;
    }

    let scale = viewport.scale_factor.value();
    if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    }
}

pub(crate) fn viewport_size_device_scale(viewport: ViewportInfo) -> Option<f64> {
    if viewport.physical_size.width == 0
        || viewport.physical_size.height == 0
        || !viewport.logical_size.width.is_finite()
        || !viewport.logical_size.height.is_finite()
        || viewport.logical_size.width <= 0.0
        || viewport.logical_size.height <= 0.0
    {
        return None;
    }

    let scale_x = f64::from(viewport.physical_size.width) / f64::from(viewport.logical_size.width);
    let scale_y =
        f64::from(viewport.physical_size.height) / f64::from(viewport.logical_size.height);
    if !scale_x.is_finite() || !scale_y.is_finite() || scale_x <= 0.0 || scale_y <= 0.0 {
        return None;
    }
    if (scale_x - scale_y).abs() > VIEWPORT_SCALE_EPSILON {
        return None;
    }

    Some((scale_x + scale_y) * 0.5)
}

pub(crate) fn root_transform(device_scale: f64) -> Affine {
    Affine::scale(device_scale.max(f64::EPSILON))
}

pub(crate) fn snap_axis_aligned_translation(transform: Affine) -> Affine {
    let mut coeffs = transform.as_coeffs();
    if coeffs[1].abs() <= f64::EPSILON && coeffs[2].abs() <= f64::EPSILON {
        coeffs[4] = coeffs[4].round();
        coeffs[5] = coeffs[5].round();
    }
    Affine::new(coeffs)
}

pub(crate) fn snap_rect_to_device(rect: Rect, device_scale: f64) -> Rect {
    let min = snap_point_to_device(Point::new(rect.x, rect.y), device_scale);
    let max = snap_point_to_device(Point::new(rect.max_x(), rect.max_y()), device_scale);
    Rect::new(
        min.x,
        min.y,
        (max.x - min.x).max(0.0),
        (max.y - min.y).max(0.0),
    )
}

pub(crate) fn snap_radius_to_device(radius: CornerRadius, device_scale: f64) -> CornerRadius {
    CornerRadius {
        top_left: snap_radius_value_to_device(radius.top_left, device_scale),
        top_right: snap_radius_value_to_device(radius.top_right, device_scale),
        bottom_right: snap_radius_value_to_device(radius.bottom_right, device_scale),
        bottom_left: snap_radius_value_to_device(radius.bottom_left, device_scale),
    }
}

pub(crate) fn snap_radius_value_to_device(value: f32, device_scale: f64) -> f32 {
    if value <= 0.0 || !value.is_finite() || !device_scale.is_finite() || device_scale <= 0.0 {
        return value;
    }
    snap_scalar_to_device(value, device_scale)
}

pub(crate) fn snap_image_rect_to_device(
    rect: Rect,
    _sampling: RenderImageSampling,
    device_scale: f64,
) -> Rect {
    snap_rect_to_device(rect, device_scale)
}

#[allow(clippy::cast_possible_truncation)]
pub(crate) fn snap_stroked_rect_to_device(
    rect: Rect,
    stroke_width: f32,
    device_scale: f64,
) -> Rect {
    if !rect.x.is_finite()
        || !rect.y.is_finite()
        || !rect.width.is_finite()
        || !rect.height.is_finite()
        || stroke_width <= 0.0
        || !stroke_width.is_finite()
        || !device_scale.is_finite()
        || device_scale <= 0.0
    {
        return rect;
    }
    let half_width =
        f64::from(quantize_stroke_width_to_device(stroke_width, device_scale)) * device_scale * 0.5;
    let left = (f64::from(rect.min_x()) * device_scale).round() + half_width;
    let top = (f64::from(rect.min_y()) * device_scale).round() + half_width;
    let right = (f64::from(rect.max_x()) * device_scale).round() - half_width;
    let bottom = (f64::from(rect.max_y()) * device_scale).round() - half_width;

    let min_x = left.min(right);
    let min_y = top.min(bottom);
    let max_x = left.max(right);
    let max_y = top.max(bottom);
    Rect::new(
        (min_x / device_scale) as f32,
        (min_y / device_scale) as f32,
        ((max_x - min_x) / device_scale) as f32,
        ((max_y - min_y) / device_scale) as f32,
    )
}

pub(crate) fn snap_stroked_line_to_device(
    from: Point,
    to: Point,
    stroke_width: f32,
    device_scale: f64,
) -> (Point, Point) {
    if (from.y - to.y).abs() <= f32::EPSILON {
        let y = snap_stroke_center_to_device(from.y, stroke_width, device_scale);
        (
            Point::new(snap_scalar_to_device(from.x, device_scale), y),
            Point::new(snap_scalar_to_device(to.x, device_scale), y),
        )
    } else if (from.x - to.x).abs() <= f32::EPSILON {
        let x = snap_stroke_center_to_device(from.x, stroke_width, device_scale);
        (
            Point::new(x, snap_scalar_to_device(from.y, device_scale)),
            Point::new(x, snap_scalar_to_device(to.y, device_scale)),
        )
    } else {
        (
            snap_point_to_device(from, device_scale),
            snap_point_to_device(to, device_scale),
        )
    }
}

#[allow(clippy::cast_possible_truncation)]
pub(crate) fn snap_stroke_center_to_device(
    value: f32,
    stroke_width: f32,
    device_scale: f64,
) -> f32 {
    if !value.is_finite()
        || stroke_width <= 0.0
        || !stroke_width.is_finite()
        || !device_scale.is_finite()
        || device_scale <= 0.0
    {
        return value;
    }
    let physical_width =
        f64::from(quantize_stroke_width_to_device(stroke_width, device_scale)) * device_scale;
    let physical = f64::from(value) * device_scale;
    let snapped = ((physical - physical_width * 0.5).round() + physical_width * 0.5) / device_scale;
    snapped as f32
}

pub(crate) fn snap_point_to_device(point: Point, device_scale: f64) -> Point {
    Point::new(
        snap_scalar_to_device(point.x, device_scale),
        snap_scalar_to_device(point.y, device_scale),
    )
}

#[allow(clippy::cast_possible_truncation)]
pub(crate) fn snap_scalar_to_device(value: f32, device_scale: f64) -> f32 {
    if !value.is_finite() || !device_scale.is_finite() || device_scale <= 0.0 {
        return value;
    }
    ((f64::from(value) * device_scale).round() / device_scale) as f32
}

pub(crate) fn crisp_rect_border_segments(
    rect: Rect,
    stroke_width: f32,
    device_scale: f64,
) -> Vec<Rect> {
    if stroke_width <= 0.0
        || !stroke_width.is_finite()
        || !device_scale.is_finite()
        || device_scale <= 0.0
    {
        return Vec::new();
    }

    let outer = snap_rect_to_device(rect, device_scale);
    if outer.width <= 0.0 || outer.height <= 0.0 {
        return Vec::new();
    }

    let width = quantize_stroke_width_to_device(stroke_width, device_scale)
        .min(outer.width)
        .min(outer.height);
    if width <= 0.0 || !width.is_finite() {
        return Vec::new();
    }
    if width * 2.0 >= outer.width || width * 2.0 >= outer.height {
        return vec![outer];
    }

    vec![
        Rect::new(outer.x, outer.y, outer.width, width),
        Rect::new(outer.x, outer.max_y() - width, outer.width, width),
        Rect::new(outer.x, outer.y + width, width, outer.height - width * 2.0),
        Rect::new(
            outer.max_x() - width,
            outer.y + width,
            width,
            outer.height - width * 2.0,
        ),
    ]
}

pub(crate) fn radius_is_zero(radius: CornerRadius) -> bool {
    radius.top_left.abs() <= f32::EPSILON
        && radius.top_right.abs() <= f32::EPSILON
        && radius.bottom_right.abs() <= f32::EPSILON
        && radius.bottom_left.abs() <= f32::EPSILON
}

pub(crate) fn compose_transform(parent: Transform, child: Transform) -> Transform {
    Transform {
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

pub(crate) fn rounded_rect(rect: Rect, radius: CornerRadius) -> RoundedRect {
    RoundedRect::from_rect(kurbo_rect(rect), kurbo_radius(radius))
}

pub(crate) fn kurbo_rect(rect: Rect) -> KurboRect {
    KurboRect::new(
        f64::from(rect.min_x()),
        f64::from(rect.min_y()),
        f64::from(rect.max_x()),
        f64::from(rect.max_y()),
    )
}

pub(crate) fn kurbo_radius(radius: CornerRadius) -> RoundedRectRadii {
    RoundedRectRadii::new(
        f64::from(radius.top_left),
        f64::from(radius.top_right),
        f64::from(radius.bottom_right),
        f64::from(radius.bottom_left),
    )
}

pub(crate) fn vello_color(color: Color) -> AlphaColor<Srgb> {
    AlphaColor::<Srgb>::new([color.r, color.g, color.b, color.a])
}

pub(crate) fn vello_gradient(gradient: &LinearGradient) -> PenikoGradient {
    vello_gradient_with_opacity(gradient, 1.0)
}

pub(crate) fn vello_gradient_with_opacity(
    gradient: &LinearGradient,
    opacity: f32,
) -> PenikoGradient {
    let stops: Vec<(f32, vello::peniko::Color)> = gradient
        .stops()
        .iter()
        .map(|stop| {
            (
                stop.offset,
                vello_color(stop.color.with_alpha(stop.color.a * opacity)),
            )
        })
        .collect();
    PenikoGradient::new_linear(
        (f64::from(gradient.start().x), f64::from(gradient.start().y)),
        (f64::from(gradient.end().x), f64::from(gradient.end().y)),
    )
    .with_interpolation_cs(ColorSpaceTag::Srgb)
    .with_interpolation_alpha_space(InterpolationAlphaSpace::Premultiplied)
    .with_stops(stops.as_slice())
}
