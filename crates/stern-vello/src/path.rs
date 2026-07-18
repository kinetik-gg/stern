use stern_core::{Brush, FillRule, PathElement, Stroke, StrokeCap, StrokeJoin};
use vello::{
    Scene,
    kurbo::{Affine, BezPath, Shape},
    peniko::{BlendMode, Fill},
};

use crate::geometry::{
    quantize_stroke_width_to_device, snap_point_to_device, snap_stroked_line_to_device,
    vello_color, vello_gradient_with_opacity,
};

pub(crate) fn stroke_shape(
    scene: &mut Scene,
    transform: Affine,
    stroke: &Stroke,
    shape: &impl Shape,
    device_scale: f64,
) {
    stroke_shape_with_opacity(scene, transform, stroke, shape, 1.0, device_scale);
}

fn stroke_shape_with_opacity(
    scene: &mut Scene,
    transform: Affine,
    stroke: &Stroke,
    shape: &impl Shape,
    opacity: f32,
    device_scale: f64,
) {
    let style = vello_stroke(*stroke, device_scale);
    match stroke.brush {
        Brush::Solid(color) => {
            scene.stroke(
                &style,
                transform,
                vello_color(color.with_alpha(color.a * opacity)),
                None,
                shape,
            );
        }
        Brush::LinearGradient(gradient) => {
            let gradient = vello_gradient_with_opacity(&gradient, opacity);
            scene.stroke(&style, transform, &gradient, None, shape);
        }
    }
}

pub(crate) fn vello_stroke(stroke: Stroke, device_scale: f64) -> vello::kurbo::Stroke {
    vello::kurbo::Stroke::new(f64::from(quantize_stroke_width_to_device(
        stroke.width,
        device_scale,
    )))
    .with_caps(match stroke.cap {
        StrokeCap::Butt => vello::kurbo::Cap::Butt,
        StrokeCap::Round => vello::kurbo::Cap::Round,
        StrokeCap::Square => vello::kurbo::Cap::Square,
    })
    .with_join(match stroke.join {
        StrokeJoin::Miter => vello::kurbo::Join::Miter,
        StrokeJoin::Round => vello::kurbo::Join::Round,
        StrokeJoin::Bevel => vello::kurbo::Join::Bevel,
    })
}

pub(crate) const fn vello_fill(fill_rule: FillRule) -> Fill {
    match fill_rule {
        FillRule::NonZero => Fill::NonZero,
        FillRule::EvenOdd => Fill::EvenOdd,
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn encode_path(
    scene: &mut Scene,
    transform: Affine,
    elements: &[PathElement],
    fill: Option<Brush>,
    stroke: Option<Stroke>,
    fill_rule: FillRule,
    opacity: f32,
    device_scale: f64,
) {
    let grouped = opacity < 1.0 && fill.is_some() && stroke.is_some();
    if grouped {
        let outset = stroke.map_or(0.0, |stroke| f64::from(stroke.width) * 4.0);
        let bounds = bez_path(elements).bounding_box().inflate(outset, outset);
        scene.push_layer(
            Fill::NonZero,
            BlendMode::default(),
            opacity,
            transform,
            &bounds,
        );
    }
    let paint_opacity = if grouped { 1.0 } else { opacity };
    if let Some(fill) = fill {
        let path = filled_bez_path(elements, device_scale);
        match fill {
            Brush::Solid(color) => scene.fill(
                vello_fill(fill_rule),
                transform,
                vello_color(color.with_alpha(color.a * paint_opacity)),
                None,
                &path,
            ),
            Brush::LinearGradient(gradient) => scene.fill(
                vello_fill(fill_rule),
                transform,
                &vello_gradient_with_opacity(&gradient, paint_opacity),
                None,
                &path,
            ),
        }
    }
    if let Some(stroke) = stroke {
        let path = stroked_bez_path(elements, stroke.width, device_scale);
        stroke_shape_with_opacity(
            scene,
            transform,
            &stroke,
            &path,
            paint_opacity,
            device_scale,
        );
    }
    if grouped {
        scene.pop_layer();
    }
}

fn has_curves(elements: &[PathElement]) -> bool {
    elements.iter().any(|element| {
        matches!(
            element,
            PathElement::QuadTo { .. } | PathElement::CubicTo { .. }
        )
    })
}

fn filled_bez_path(elements: &[PathElement], device_scale: f64) -> BezPath {
    if has_curves(elements) {
        return bez_path(elements);
    }
    let mut path = BezPath::new();
    for element in elements {
        match *element {
            PathElement::MoveTo(point) => {
                let point = snap_point_to_device(point, device_scale);
                path.move_to((f64::from(point.x), f64::from(point.y)));
            }
            PathElement::LineTo(point) => {
                let point = snap_point_to_device(point, device_scale);
                path.line_to((f64::from(point.x), f64::from(point.y)));
            }
            PathElement::Close => path.close_path(),
            PathElement::QuadTo { .. } | PathElement::CubicTo { .. } => unreachable!(),
        }
    }
    path
}

fn stroked_bez_path(elements: &[PathElement], stroke_width: f32, device_scale: f64) -> BezPath {
    if has_curves(elements) {
        return bez_path(elements);
    }

    let mut path = BezPath::new();
    let mut pending_move = None;
    let mut current = None;
    for element in elements {
        match *element {
            PathElement::MoveTo(point) => {
                if let Some(point) = pending_move.replace(point) {
                    let point = snap_point_to_device(point, device_scale);
                    path.move_to((f64::from(point.x), f64::from(point.y)));
                }
                current = Some(point);
            }
            PathElement::LineTo(point) => {
                if let Some(from) = current {
                    let (from, to) =
                        snap_stroked_line_to_device(from, point, stroke_width, device_scale);
                    if pending_move.take().is_some() {
                        path.move_to((f64::from(from.x), f64::from(from.y)));
                    }
                    path.line_to((f64::from(to.x), f64::from(to.y)));
                } else {
                    let point = snap_point_to_device(point, device_scale);
                    path.line_to((f64::from(point.x), f64::from(point.y)));
                }
                current = Some(point);
            }
            PathElement::Close => {
                if let Some(point) = pending_move.take() {
                    let point = snap_point_to_device(point, device_scale);
                    path.move_to((f64::from(point.x), f64::from(point.y)));
                }
                path.close_path();
                current = None;
            }
            PathElement::QuadTo { .. } | PathElement::CubicTo { .. } => unreachable!(),
        }
    }
    if let Some(point) = pending_move {
        let point = snap_point_to_device(point, device_scale);
        path.move_to((f64::from(point.x), f64::from(point.y)));
    }
    path
}

fn bez_path(elements: &[PathElement]) -> BezPath {
    let mut path = BezPath::new();
    for element in elements {
        match *element {
            PathElement::MoveTo(point) => {
                path.move_to((f64::from(point.x), f64::from(point.y)));
            }
            PathElement::LineTo(point) => {
                path.line_to((f64::from(point.x), f64::from(point.y)));
            }
            PathElement::QuadTo { ctrl, to } => {
                path.quad_to(
                    (f64::from(ctrl.x), f64::from(ctrl.y)),
                    (f64::from(to.x), f64::from(to.y)),
                );
            }
            PathElement::CubicTo { ctrl1, ctrl2, to } => {
                path.curve_to(
                    (f64::from(ctrl1.x), f64::from(ctrl1.y)),
                    (f64::from(ctrl2.x), f64::from(ctrl2.y)),
                    (f64::from(to.x), f64::from(to.y)),
                );
            }
            PathElement::Close => path.close_path(),
        }
    }
    path
}

#[cfg(test)]
pub(crate) fn snap_filled_path_elements_to_device(
    elements: &[PathElement],
    device_scale: f64,
) -> Vec<PathElement> {
    if has_curves(elements) {
        return elements.to_vec();
    }
    elements
        .iter()
        .map(|element| match *element {
            PathElement::MoveTo(point) => {
                PathElement::MoveTo(snap_point_to_device(point, device_scale))
            }
            PathElement::LineTo(point) => {
                PathElement::LineTo(snap_point_to_device(point, device_scale))
            }
            PathElement::Close => PathElement::Close,
            PathElement::QuadTo { .. } | PathElement::CubicTo { .. } => unreachable!(),
        })
        .collect()
}

#[cfg(test)]
pub(crate) fn snap_stroked_path_elements_to_device(
    elements: &[PathElement],
    stroke_width: f32,
    device_scale: f64,
) -> Vec<PathElement> {
    if has_curves(elements) {
        return elements.to_vec();
    }

    let mut snapped = Vec::with_capacity(elements.len());
    let mut pending_move = None;
    let mut current = None;
    for element in elements {
        match *element {
            PathElement::MoveTo(point) => {
                if let Some(point) = pending_move.replace(point) {
                    snapped.push(PathElement::MoveTo(snap_point_to_device(
                        point,
                        device_scale,
                    )));
                }
                current = Some(point);
            }
            PathElement::LineTo(point) => {
                if let Some(from) = current {
                    let (from, to) =
                        snap_stroked_line_to_device(from, point, stroke_width, device_scale);
                    if pending_move.take().is_some() {
                        snapped.push(PathElement::MoveTo(from));
                    }
                    snapped.push(PathElement::LineTo(to));
                } else {
                    snapped.push(PathElement::LineTo(snap_point_to_device(
                        point,
                        device_scale,
                    )));
                }
                current = Some(point);
            }
            PathElement::Close => {
                if let Some(point) = pending_move.take() {
                    snapped.push(PathElement::MoveTo(snap_point_to_device(
                        point,
                        device_scale,
                    )));
                }
                snapped.push(PathElement::Close);
                current = None;
            }
            PathElement::QuadTo { .. } | PathElement::CubicTo { .. } => unreachable!(),
        }
    }
    if let Some(point) = pending_move {
        snapped.push(PathElement::MoveTo(snap_point_to_device(
            point,
            device_scale,
        )));
    }
    snapped
}
