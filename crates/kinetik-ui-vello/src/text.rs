use std::sync::Arc;

use kinetik_ui_core::{Color, Point};
use kinetik_ui_text::{ShapedTextLayout, TextLayoutKey, TextLayoutStore, TextStyle};
use vello::{Glyph, Scene, kurbo::Affine, peniko::Fill};

use crate::geometry::vello_color;

pub(crate) fn shape_fallback_text(
    text_layouts: &mut TextLayoutStore,
    text: &str,
    family: &str,
    size: f32,
    line_height: f32,
) -> Arc<ShapedTextLayout> {
    let key = TextLayoutKey::new(text, TextStyle::new(family, size, line_height), 0.0, false);
    resolve_fallback_text_layout(text_layouts, &key, true)
}

#[cfg(test)]
pub(crate) fn encode_forced_transient_text(
    scene: &mut Scene,
    text_layouts: &mut TextLayoutStore,
    key: &TextLayoutKey,
) -> Arc<ShapedTextLayout> {
    let layout = resolve_fallback_text_layout(text_layouts, key, false);
    encode_text_layout(
        scene,
        Affine::IDENTITY,
        Affine::IDENTITY,
        Point::new(0.0, 0.0),
        &layout,
        Color::WHITE,
    );
    layout
}

fn resolve_fallback_text_layout(
    text_layouts: &mut TextLayoutStore,
    key: &TextLayoutKey,
    attempt_retention: bool,
) -> Arc<ShapedTextLayout> {
    if attempt_retention && let Some(id) = text_layouts.try_layout_id(key.clone()) {
        return text_layouts
            .stored_layout(id)
            .expect("accepted fallback text layout must remain resident")
            .layout;
    }

    Arc::new(text_layouts.shape_transient(key))
}

pub(crate) fn encode_text_layout(
    scene: &mut Scene,
    raw_transform: Affine,
    effective_transform: Affine,
    origin: Point,
    layout: &ShapedTextLayout,
    color: Color,
) {
    if let Some((scale_x, scale_y)) = exact_positive_axis_aligned_scale(raw_transform) {
        encode_shaped_text_axis_aligned_device_space(
            scene,
            effective_transform,
            origin,
            layout,
            color,
            scale_x,
            scale_y,
        );
    } else {
        encode_shaped_text(scene, raw_transform, origin, layout, color);
    }
}

pub(crate) fn encode_shaped_text(
    scene: &mut Scene,
    transform: Affine,
    origin: Point,
    layout: &ShapedTextLayout,
    color: Color,
) {
    for run in &layout.runs {
        scene
            .draw_glyphs(&run.font)
            .transform(transform)
            .font_size(run.font_size)
            .brush(vello_color(color))
            .draw(
                Fill::NonZero,
                run.glyphs.iter().map(|glyph| Glyph {
                    id: glyph.id,
                    x: origin.x + glyph.x,
                    y: origin.y + glyph.y,
                }),
            );
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
pub(crate) fn encode_shaped_text_axis_aligned_device_space(
    scene: &mut Scene,
    transform: Affine,
    origin: Point,
    layout: &ShapedTextLayout,
    color: Color,
    scale_x: f64,
    scale_y: f64,
) {
    let glyph_transform = non_uniform_axis_aligned_glyph_transform(scale_x, scale_y);
    for run in &layout.runs {
        let mut glyph_run = scene
            .draw_glyphs(&run.font)
            .font_size(run.font_size * scale_y as f32)
            .hint(true)
            .brush(vello_color(color));
        if let Some(glyph_transform) = glyph_transform {
            glyph_run = glyph_run.glyph_transform(Some(glyph_transform));
        }
        glyph_run.draw(
            Fill::NonZero,
            run.glyphs.iter().map(|glyph| {
                let point = project_text_point_to_device(
                    transform,
                    Point::new(origin.x + glyph.x, origin.y + glyph.y),
                );
                Glyph {
                    id: glyph.id,
                    x: point.x,
                    y: point.y,
                }
            }),
        );
    }
}

#[allow(clippy::float_cmp)]
pub(crate) fn non_uniform_axis_aligned_glyph_transform(
    scale_x: f64,
    scale_y: f64,
) -> Option<Affine> {
    if !scale_x.is_finite() || !scale_y.is_finite() || scale_x <= 0.0 || scale_y <= 0.0 {
        return None;
    }

    let x_ratio = scale_x / scale_y;
    (x_ratio != 1.0).then(|| Affine::scale_non_uniform(x_ratio, 1.0))
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
pub(crate) fn project_text_point_to_device(transform: Affine, point: Point) -> Point {
    let coeffs = transform.as_coeffs();
    let x = coeffs[0].mul_add(
        f64::from(point.x),
        coeffs[2].mul_add(f64::from(point.y), coeffs[4]),
    );
    let y = coeffs[1].mul_add(
        f64::from(point.x),
        coeffs[3].mul_add(f64::from(point.y), coeffs[5]),
    );
    Point::new(x.round() as f32, y.round() as f32)
}

pub(crate) fn exact_positive_axis_aligned_scale(transform: Affine) -> Option<(f64, f64)> {
    let coeffs = transform.as_coeffs();
    let scale_x = coeffs[0];
    let skew_y = coeffs[1];
    let skew_x = coeffs[2];
    let scale_y = coeffs[3];
    (skew_y == 0.0
        && skew_x == 0.0
        && scale_x.is_finite()
        && scale_y.is_finite()
        && scale_x > 0.0
        && scale_y > 0.0)
        .then_some((scale_x, scale_y))
}

#[allow(
    dead_code,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss
)]
pub(crate) fn transform_point(transform: Affine, point: Point) -> Point {
    let coeffs = transform.as_coeffs();
    Point::new(
        (coeffs[0].mul_add(
            f64::from(point.x),
            coeffs[2].mul_add(f64::from(point.y), coeffs[4]),
        )) as f32,
        (coeffs[1].mul_add(
            f64::from(point.x),
            coeffs[3].mul_add(f64::from(point.y), coeffs[5]),
        )) as f32,
    )
}
