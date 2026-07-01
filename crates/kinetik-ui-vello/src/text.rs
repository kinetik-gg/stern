use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use kinetik_ui_core::{Color, Point};
use kinetik_ui_text::{CosmicTextEngine, ShapedTextLayout, TextLayoutKey, TextStyle};
use vello::{Glyph, Scene, kurbo::Affine, peniko::Fill};

use crate::geometry::vello_color;

pub(crate) const MAX_CACHED_TEXT_LAYOUTS: usize = 4096;
pub(crate) const TEXT_TRANSFORM_EPSILON: f64 = 0.0001;

#[derive(Debug, Default)]
pub(crate) struct ShapedTextCache {
    pub(crate) layouts: HashMap<TextLayoutKey, Arc<ShapedTextLayout>>,
    pub(crate) layout_order: VecDeque<TextLayoutKey>,
}

impl ShapedTextCache {
    pub(crate) fn layout(
        &mut self,
        text_engine: &mut CosmicTextEngine,
        key: TextLayoutKey,
    ) -> Arc<ShapedTextLayout> {
        if let Some(layout) = self.layouts.get(&key) {
            touch_owned_cache_key(&mut self.layout_order, &key);
            return Arc::clone(layout);
        }

        while self.layouts.len() >= MAX_CACHED_TEXT_LAYOUTS {
            let Some(evicted) = self.layout_order.pop_front() else {
                break;
            };
            self.layouts.remove(&evicted);
        }

        let layout = Arc::new(shape_text_with_key(text_engine, &key));
        self.layout_order.push_back(key.clone());
        self.layouts.insert(key, Arc::clone(&layout));
        layout
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.layouts.len()
    }
}

pub(crate) fn touch_owned_cache_key<Id>(order: &mut VecDeque<Id>, id: &Id)
where
    Id: Eq,
{
    if let Some(position) = order.iter().position(|existing| existing == id)
        && let Some(id) = order.remove(position)
    {
        order.push_back(id);
    }
}

pub(crate) fn shape_fallback_text(
    text_engine: &mut CosmicTextEngine,
    text_cache: &mut ShapedTextCache,
    text: &str,
    family: &str,
    size: f32,
    line_height: f32,
) -> Arc<ShapedTextLayout> {
    text_cache.layout(
        text_engine,
        TextLayoutKey::new(text, TextStyle::new(family, size, line_height), 0.0, false),
    )
}

pub(crate) fn shape_text_with_key(
    text_engine: &mut CosmicTextEngine,
    key: &TextLayoutKey,
) -> ShapedTextLayout {
    text_engine.shape_text(key)
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
pub(crate) fn physical_text_layout_for_key(
    text_engine: &mut CosmicTextEngine,
    text_cache: &mut ShapedTextCache,
    transform: Affine,
    key: &TextLayoutKey,
) -> Option<Arc<ShapedTextLayout>> {
    let scale = uniform_axis_aligned_scale(transform)?;
    let physical_size = quantize_physical_text_metric(f64::from(key.style.size()) * scale);
    let physical_line_height =
        quantize_physical_text_metric(f64::from(key.style.line_height()) * scale);
    let physical_width = quantize_physical_text_extent(f64::from(key.width()) * scale);
    (physical_size.is_finite()
        && physical_size > 0.0
        && physical_line_height.is_finite()
        && physical_line_height > 0.0
        && physical_width.is_finite())
    .then(|| {
        text_cache.layout(
            text_engine,
            TextLayoutKey::new(
                key.text.clone(),
                TextStyle::new(
                    key.style.family.clone(),
                    physical_size,
                    physical_line_height,
                ),
                physical_width,
                key.wrap,
            ),
        )
    })
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
pub(crate) fn physical_text_layout(
    text_engine: &mut CosmicTextEngine,
    text_cache: &mut ShapedTextCache,
    transform: Affine,
    text: &str,
    family: &str,
    size: f32,
    line_height: f32,
) -> Option<Arc<ShapedTextLayout>> {
    let scale = uniform_axis_aligned_scale(transform)?;
    let physical_size = quantize_physical_text_metric(f64::from(size) * scale);
    let physical_line_height = quantize_physical_text_metric(f64::from(line_height) * scale);
    (physical_size.is_finite()
        && physical_size > 0.0
        && physical_line_height.is_finite()
        && physical_line_height > 0.0)
        .then(|| {
            text_cache.layout(
                text_engine,
                TextLayoutKey::new(
                    text,
                    TextStyle::new(family, physical_size, physical_line_height),
                    0.0,
                    false,
                ),
            )
        })
}

#[allow(clippy::cast_possible_truncation)]
pub(crate) fn quantize_physical_text_metric(value: f64) -> f32 {
    if value.is_finite() && value > 0.0 {
        value.round().max(1.0) as f32
    } else {
        value as f32
    }
}

#[allow(clippy::cast_possible_truncation)]
pub(crate) fn quantize_physical_text_extent(value: f64) -> f32 {
    if value.is_finite() && value > 0.0 {
        value.round().max(0.0) as f32
    } else {
        value as f32
    }
}

pub(crate) fn encode_text_layout(
    scene: &mut Scene,
    transform: Affine,
    origin: Point,
    layout: &ShapedTextLayout,
    physical_layout: Option<&ShapedTextLayout>,
    color: Color,
    _device_scale: f64,
) {
    if let Some(scale) = uniform_axis_aligned_scale(transform) {
        let origin = transform_point(transform, origin);
        if let Some(physical_layout) = physical_layout {
            encode_shaped_text_device_space(scene, origin, physical_layout, color, 1.0);
        } else {
            encode_shaped_text_device_space(scene, origin, layout, color, scale);
        }
    } else if let Some((scale_x, scale_y)) = axis_aligned_scale(transform) {
        let origin = transform_point(transform, origin);
        encode_shaped_text_axis_aligned_device_space(
            scene, origin, layout, color, scale_x, scale_y,
        );
    } else {
        let transform = snap_text_transform_origin_to_device(transform, origin);
        encode_shaped_text(scene, transform, origin, layout, color);
    }
}

pub(crate) fn encode_shaped_text(
    scene: &mut Scene,
    transform: Affine,
    origin: kinetik_ui_core::Point,
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
pub(crate) fn encode_shaped_text_device_space(
    scene: &mut Scene,
    origin: Point,
    layout: &ShapedTextLayout,
    color: Color,
    scale: f64,
) {
    let origin = snap_text_origin_to_device(origin);
    for run in &layout.runs {
        scene
            .draw_glyphs(&run.font)
            .font_size(run.font_size * scale as f32)
            .hint(true)
            .brush(vello_color(color))
            .draw(
                Fill::NonZero,
                run.glyphs.iter().map(|glyph| Glyph {
                    id: glyph.id,
                    x: snap_text_glyph_position_to_device(origin.x + glyph.x * scale as f32),
                    y: snap_text_glyph_baseline_to_device(origin.y + glyph.y * scale as f32),
                }),
            );
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
pub(crate) fn encode_shaped_text_axis_aligned_device_space(
    scene: &mut Scene,
    origin: Point,
    layout: &ShapedTextLayout,
    color: Color,
    scale_x: f64,
    scale_y: f64,
) {
    let origin = snap_text_origin_to_device(origin);
    for run in &layout.runs {
        let font_size = quantize_physical_text_metric(f64::from(run.font_size) * scale_y);
        let effective_y_scale = if run.font_size > 0.0 {
            f64::from(font_size) / f64::from(run.font_size)
        } else {
            scale_y
        };
        let glyph_transform = non_uniform_axis_aligned_glyph_transform(scale_x, effective_y_scale);
        let mut glyph_run = scene
            .draw_glyphs(&run.font)
            .font_size(font_size)
            .hint(true)
            .brush(vello_color(color));
        if let Some(glyph_transform) = glyph_transform {
            glyph_run = glyph_run.glyph_transform(Some(glyph_transform));
        }
        glyph_run.draw(
            Fill::NonZero,
            run.glyphs.iter().map(|glyph| Glyph {
                id: glyph.id,
                x: snap_text_glyph_position_to_device(
                    origin.x + (f64::from(glyph.x) * scale_x) as f32,
                ),
                y: snap_text_glyph_baseline_to_device(
                    origin.y + (f64::from(glyph.y) * effective_y_scale) as f32,
                ),
            }),
        );
    }
}

pub(crate) fn non_uniform_axis_aligned_glyph_transform(
    scale_x: f64,
    effective_y_scale: f64,
) -> Option<Affine> {
    if !scale_x.is_finite()
        || !effective_y_scale.is_finite()
        || scale_x <= 0.0
        || effective_y_scale <= 0.0
    {
        return None;
    }

    let x_ratio = scale_x / effective_y_scale;
    ((x_ratio - 1.0).abs() > TEXT_TRANSFORM_EPSILON)
        .then(|| Affine::scale_non_uniform(x_ratio, 1.0))
}

pub(crate) fn snap_text_origin_to_device(origin: Point) -> Point {
    Point::new(origin.x.round(), origin.y.round())
}

pub(crate) fn snap_text_glyph_position_to_device(position: f32) -> f32 {
    position.round()
}

pub(crate) fn snap_text_glyph_baseline_to_device(position: f32) -> f32 {
    position.round()
}

pub(crate) fn snap_text_transform_origin_to_device(transform: Affine, origin: Point) -> Affine {
    let device_origin = transform_point(transform, origin);
    let snapped_origin = snap_text_origin_to_device(device_origin);
    let mut coeffs = transform.as_coeffs();
    coeffs[4] += f64::from(snapped_origin.x - device_origin.x);
    coeffs[5] += f64::from(snapped_origin.y - device_origin.y);
    Affine::new(coeffs)
}

pub(crate) fn uniform_axis_aligned_scale(transform: Affine) -> Option<f64> {
    let (scale_x, scale_y) = axis_aligned_scale(transform)?;
    ((scale_x - scale_y).abs() <= TEXT_TRANSFORM_EPSILON).then_some((scale_x + scale_y) * 0.5)
}

pub(crate) fn axis_aligned_scale(transform: Affine) -> Option<(f64, f64)> {
    let coeffs = transform.as_coeffs();
    let scale_x = coeffs[0];
    let skew_y = coeffs[1];
    let skew_x = coeffs[2];
    let scale_y = coeffs[3];
    if skew_y.abs() <= TEXT_TRANSFORM_EPSILON
        && skew_x.abs() <= TEXT_TRANSFORM_EPSILON
        && scale_x.is_finite()
        && scale_y.is_finite()
        && scale_x > 0.0
        && scale_y > 0.0
    {
        Some((scale_x, scale_y))
    } else {
        None
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
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
