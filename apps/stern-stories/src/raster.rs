//! Deterministic CPU rasterization of story frames.
//!
//! Reuses the existing deterministic pipeline stages: widget primitives are
//! translated and sanitized by `stern-vello`'s public [`translate_primitives`]
//! (the same command stream the GPU backend encodes), text comes pre-shaped
//! from `stern-text`'s bundled fonts, and pixel coverage is produced by
//! `tiny-skia` with glyph outlines scaled by `swash`. No GPU, no window, no
//! system fonts.
//!
//! This CPU path is its own golden baseline: it deliberately does not claim
//! pixel parity with the Vello GPU output. It exists so composed scenes can
//! be reviewed and regression-diffed deterministically.

use stern::core::{
    Brush, Color, CornerRadius, FillRule, PathElement, Point, Size, Stroke, StrokeCap, StrokeJoin,
    Transform,
};
use stern::render::RenderResources;
use stern::render_vello::{RenderClip, RenderCommand, RenderCommandKind, translate_primitives};
use stern::text::{ShapedTextLayout, TextLayoutKey, TextLayoutStore, TextStyle};
use swash::zeno::{Command as ZenoCommand, PathData as _};
use tiny_skia::{
    FillRule as SkiaFillRule, LineCap, LineJoin, Mask, Paint, PathBuilder, Pixmap, PixmapPaint,
    Shader, SpreadMode, Transform as SkiaTransform,
};

use crate::diff::RgbaImage;

/// Result of rasterizing one composed frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RasterFrame {
    /// Straight-alpha RGBA8 pixels at device resolution.
    pub image: RgbaImage,
    /// Human-readable diagnostics from translation and rasterization.
    pub diagnostics: Vec<String>,
}

/// Rasterizes sanitized widget primitives at `logical` size and `scale`.
///
/// Returns `None` only when the device size is degenerate.
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn rasterize(
    primitives: &[stern::core::Primitive],
    resources: &RenderResources,
    logical: Size,
    scale: f32,
    background: Color,
) -> Option<RasterFrame> {
    let width = (logical.width * scale).round().max(1.0) as u32;
    let height = (logical.height * scale).round().max(1.0) as u32;
    let translation = translate_primitives(primitives, resources);
    let mut diagnostics: Vec<String> = translation
        .diagnostics
        .iter()
        .map(|diagnostic| format!("{diagnostic:?}"))
        .collect();

    let mut target = Pixmap::new(width, height)?;
    target.fill(skia_color(background, 1.0));

    let mut executor = Executor {
        resources,
        fallback_text: TextLayoutStore::new(),
        scale_context: swash::scale::ScaleContext::new(),
        root: SkiaTransform::from_scale(scale, scale),
        width,
        height,
        clip_stack: Vec::new(),
        clip_mask: None,
        group_stack: Vec::new(),
        diagnostics: &mut diagnostics,
    };
    for command in &translation.commands {
        executor.execute(&mut target, command);
    }
    // Unbalanced opacity groups are composited so nothing is silently lost.
    while let Some((parent, opacity)) = executor.group_stack.pop() {
        let child = std::mem::replace(&mut target, parent);
        target.draw_pixmap(
            0,
            0,
            child.as_ref(),
            &PixmapPaint {
                opacity,
                ..PixmapPaint::default()
            },
            SkiaTransform::identity(),
            None,
        );
    }

    let pixels = target
        .pixels()
        .iter()
        .flat_map(|pixel| {
            let demultiplied = pixel.demultiply();
            [
                demultiplied.red(),
                demultiplied.green(),
                demultiplied.blue(),
                demultiplied.alpha(),
            ]
        })
        .collect();
    Some(RasterFrame {
        image: RgbaImage::new(width, height, pixels)?,
        diagnostics,
    })
}

struct Executor<'a> {
    resources: &'a RenderResources,
    fallback_text: TextLayoutStore,
    scale_context: swash::scale::ScaleContext,
    root: SkiaTransform,
    width: u32,
    height: u32,
    clip_stack: Vec<RenderClip>,
    clip_mask: Option<Mask>,
    group_stack: Vec<(Pixmap, f32)>,
    diagnostics: &'a mut Vec<String>,
}

impl Executor<'_> {
    #[allow(clippy::too_many_lines)]
    fn execute(&mut self, target: &mut Pixmap, command: &RenderCommand) {
        self.refresh_clip_mask(&command.clips);
        let transform = to_skia(command.transform).post_concat(self.root);
        match &command.kind {
            RenderCommandKind::OpacityGroupBegin { opacity, .. } => {
                if let Some(blank) = Pixmap::new(self.width, self.height) {
                    let parent = std::mem::replace(target, blank);
                    self.group_stack.push((parent, *opacity));
                }
            }
            RenderCommandKind::OpacityGroupEnd => {
                if let Some((parent, opacity)) = self.group_stack.pop() {
                    let child = std::mem::replace(target, parent);
                    target.draw_pixmap(
                        0,
                        0,
                        child.as_ref(),
                        &PixmapPaint {
                            opacity,
                            ..PixmapPaint::default()
                        },
                        SkiaTransform::identity(),
                        None,
                    );
                }
            }
            RenderCommandKind::Rect {
                rect,
                fill,
                stroke,
                radius,
            } => {
                if let Some(path) = rounded_rect_path(*rect, *radius) {
                    if let Some(brush) = fill {
                        self.fill(target, &path, brush, FillRule::NonZero, 1.0, transform);
                    }
                    if let Some(stroke) = stroke {
                        self.stroke(target, &path, stroke, 1.0, transform);
                    }
                }
            }
            RenderCommandKind::Line {
                x0,
                y0,
                x1,
                y1,
                stroke,
            } => {
                let mut builder = PathBuilder::new();
                builder.move_to(*x0, *y0);
                builder.line_to(*x1, *y1);
                if let Some(path) = builder.finish() {
                    self.stroke(target, &path, stroke, 1.0, transform);
                }
            }
            RenderCommandKind::Shadow {
                rect,
                offset,
                blur_radius,
                spread,
                radius,
                color,
            } => {
                let shadow_rect = stern::core::Rect::new(
                    rect.x + offset.x - spread,
                    rect.y + offset.y - spread,
                    (rect.width + 2.0 * spread).max(0.0),
                    (rect.height + 2.0 * spread).max(0.0),
                );
                let corner = CornerRadius::all((radius + spread).max(0.0));
                if let Some(path) = rounded_rect_path(shadow_rect, corner) {
                    self.paint_shadow(target, &path, *blur_radius, *color, transform);
                }
            }
            RenderCommandKind::Path {
                elements,
                fill,
                stroke,
                fill_rule,
                opacity,
            } => {
                if let Some(path) = elements_path(elements.as_slice()) {
                    if let Some(brush) = fill {
                        self.fill(target, &path, brush, *fill_rule, *opacity, transform);
                    }
                    if let Some(stroke) = stroke {
                        self.stroke(target, &path, stroke, *opacity, transform);
                    }
                }
            }
            RenderCommandKind::Text {
                layout,
                origin,
                text,
                family,
                size,
                line_height,
                color,
            } => {
                let resolved = layout
                    .and_then(|id| self.resources.text_layout(id).cloned())
                    .unwrap_or_else(|| {
                        let key = TextLayoutKey::new(
                            text.clone(),
                            TextStyle::new(family.clone(), *size, *line_height),
                            0.0,
                            false,
                        );
                        self.fallback_text.shape_transient(&key)
                    });
                self.paint_text(target, &resolved, *origin, *color, transform);
            }
            RenderCommandKind::Image { image, rect, tint } => {
                let pixels = self.resources.image(*image).and_then(|resource| {
                    resource.pixels.clone().or_else(|| {
                        resource.atlas_region.as_ref().and_then(|region| {
                            self.resources
                                .image(region.atlas)
                                .and_then(|atlas| atlas.pixels.clone())
                        })
                    })
                });
                if let Some(pixels) = pixels {
                    self.paint_image(target, &pixels, *rect, *tint, transform);
                } else {
                    self.diagnostics
                        .push(format!("image {image:?} has no CPU pixels; skipped"));
                }
            }
            RenderCommandKind::Texture { texture, rect, .. } => {
                let snapshot = self
                    .resources
                    .texture(*texture)
                    .and_then(|resource| resource.snapshot.clone());
                if let Some(snapshot) = snapshot {
                    self.paint_image(target, &snapshot, *rect, None, transform);
                } else {
                    self.diagnostics
                        .push(format!("texture {texture:?} has no CPU snapshot; skipped"));
                }
            }
        }
    }

    fn refresh_clip_mask(&mut self, clips: &[RenderClip]) {
        if clips == self.clip_stack.as_slice() {
            return;
        }
        self.clip_stack = clips.to_vec();
        self.clip_mask = None;
        if clips.is_empty() {
            return;
        }
        let Some(mut mask) = Mask::new(self.width, self.height) else {
            return;
        };
        let mut first = true;
        for clip in clips {
            let rect = tiny_skia::Rect::from_xywh(
                clip.rect.x,
                clip.rect.y,
                clip.rect.width.max(0.001),
                clip.rect.height.max(0.001),
            )
            .unwrap_or_else(|| {
                tiny_skia::Rect::from_xywh(0.0, 0.0, 0.001, 0.001)
                    .expect("static rect must construct")
            });
            let path = PathBuilder::from_rect(rect);
            let transform = to_skia(clip.transform).post_concat(self.root);
            if first {
                mask.fill_path(&path, SkiaFillRule::Winding, true, transform);
                first = false;
            } else {
                mask.intersect_path(&path, SkiaFillRule::Winding, true, transform);
            }
        }
        self.clip_mask = Some(mask);
    }

    fn fill(
        &mut self,
        target: &mut Pixmap,
        path: &tiny_skia::Path,
        brush: &Brush,
        rule: FillRule,
        opacity: f32,
        transform: SkiaTransform,
    ) {
        let Some(paint) = brush_paint(brush, opacity) else {
            return;
        };
        target.fill_path(
            path,
            &paint,
            match rule {
                FillRule::NonZero => SkiaFillRule::Winding,
                FillRule::EvenOdd => SkiaFillRule::EvenOdd,
            },
            transform,
            self.clip_mask.as_ref(),
        );
    }

    fn stroke(
        &mut self,
        target: &mut Pixmap,
        path: &tiny_skia::Path,
        stroke: &Stroke,
        opacity: f32,
        transform: SkiaTransform,
    ) {
        let Some(paint) = brush_paint(&stroke.brush, opacity) else {
            return;
        };
        let skia_stroke = tiny_skia::Stroke {
            width: stroke.width.max(0.0),
            line_cap: match stroke.cap {
                StrokeCap::Butt => LineCap::Butt,
                StrokeCap::Round => LineCap::Round,
                StrokeCap::Square => LineCap::Square,
            },
            line_join: match stroke.join {
                StrokeJoin::Miter => LineJoin::Miter,
                StrokeJoin::Round => LineJoin::Round,
                StrokeJoin::Bevel => LineJoin::Bevel,
            },
            ..tiny_skia::Stroke::default()
        };
        target.stroke_path(
            path,
            &paint,
            &skia_stroke,
            transform,
            self.clip_mask.as_ref(),
        );
    }

    fn paint_text(
        &mut self,
        target: &mut Pixmap,
        layout: &ShapedTextLayout,
        origin: Point,
        color: Color,
        transform: SkiaTransform,
    ) {
        let paint = solid_paint(color, 1.0);
        for run in &layout.runs {
            let font_data = run.font.data.data();
            let Some(font_ref) = swash::FontRef::from_index(font_data, run.font.index as usize)
            else {
                self.diagnostics
                    .push("text run font could not be parsed; run skipped".to_owned());
                continue;
            };
            let mut scaler = self
                .scale_context
                .builder(font_ref)
                .size(run.font_size)
                .hint(false)
                .normalized_coords(run.normalized_coords.iter().copied())
                .build();
            for glyph in &run.glyphs {
                let Ok(glyph_id) = u16::try_from(glyph.id) else {
                    continue;
                };
                let Some(outline) = scaler.scale_outline(glyph_id) else {
                    continue;
                };
                let Some(path) = zeno_path(&outline) else {
                    continue;
                };
                let glyph_transform =
                    SkiaTransform::from_translate(origin.x + glyph.x, origin.y + glyph.y)
                        .post_concat(transform);
                target.fill_path(
                    &path,
                    &paint,
                    SkiaFillRule::Winding,
                    glyph_transform,
                    self.clip_mask.as_ref(),
                );
            }
        }
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    fn paint_image(
        &mut self,
        target: &mut Pixmap,
        image: &stern::render::RenderImage,
        rect: stern::core::Rect,
        tint: Option<Color>,
        transform: SkiaTransform,
    ) {
        let Some(pixmap) = render_image_pixmap(image, tint) else {
            self.diagnostics
                .push("image payload could not be converted; skipped".to_owned());
            return;
        };
        if rect.width <= 0.0 || rect.height <= 0.0 {
            return;
        }
        let source_width = image.width.max(1) as f32;
        let source_height = image.height.max(1) as f32;
        let placement = SkiaTransform::from_row(
            rect.width / source_width,
            0.0,
            0.0,
            rect.height / source_height,
            rect.x,
            rect.y,
        )
        .post_concat(transform);
        target.draw_pixmap(
            0,
            0,
            pixmap.as_ref(),
            &PixmapPaint::default(),
            placement,
            self.clip_mask.as_ref(),
        );
    }

    fn paint_shadow(
        &mut self,
        target: &mut Pixmap,
        path: &tiny_skia::Path,
        blur_radius: f32,
        color: Color,
        transform: SkiaTransform,
    ) {
        let Some(mut layer) = Pixmap::new(self.width, self.height) else {
            return;
        };
        let paint = solid_paint(color, 1.0);
        layer.fill_path(path, &paint, SkiaFillRule::Winding, transform, None);
        let device_blur = blur_radius * self.root.sx;
        if device_blur > 0.5 {
            box_blur(&mut layer, device_blur * 0.5);
        }
        target.draw_pixmap(
            0,
            0,
            layer.as_ref(),
            &PixmapPaint::default(),
            SkiaTransform::identity(),
            self.clip_mask.as_ref(),
        );
    }
}

fn to_skia(transform: Transform) -> SkiaTransform {
    SkiaTransform::from_row(
        transform.m11,
        transform.m12,
        transform.m21,
        transform.m22,
        transform.dx,
        transform.dy,
    )
}

fn skia_color(color: Color, opacity: f32) -> tiny_skia::Color {
    tiny_skia::Color::from_rgba(
        color.r.clamp(0.0, 1.0),
        color.g.clamp(0.0, 1.0),
        color.b.clamp(0.0, 1.0),
        (color.a * opacity).clamp(0.0, 1.0),
    )
    .unwrap_or_else(|| tiny_skia::Color::from_rgba8(255, 0, 200, 255))
}

fn solid_paint<'a>(color: Color, opacity: f32) -> Paint<'a> {
    let mut paint = Paint {
        shader: Shader::SolidColor(skia_color(color, opacity)),
        ..Paint::default()
    };
    paint.anti_alias = true;
    paint
}

fn brush_paint<'a>(brush: &Brush, opacity: f32) -> Option<Paint<'a>> {
    match brush {
        Brush::Solid(color) => Some(solid_paint(*color, opacity)),
        Brush::LinearGradient(gradient) => {
            let stops: Vec<tiny_skia::GradientStop> = gradient
                .stops()
                .iter()
                .map(|stop| {
                    tiny_skia::GradientStop::new(
                        stop.offset.clamp(0.0, 1.0),
                        skia_color(stop.color, opacity),
                    )
                })
                .collect();
            let shader = tiny_skia::LinearGradient::new(
                tiny_skia::Point::from_xy(gradient.start().x, gradient.start().y),
                tiny_skia::Point::from_xy(gradient.end().x, gradient.end().y),
                stops,
                SpreadMode::Pad,
                SkiaTransform::identity(),
            )?;
            let mut paint = Paint {
                shader,
                ..Paint::default()
            };
            paint.anti_alias = true;
            Some(paint)
        }
    }
}

fn rounded_rect_path(rect: stern::core::Rect, radius: CornerRadius) -> Option<tiny_skia::Path> {
    const KAPPA: f32 = 0.552_284_8;
    if rect.width <= 0.0 || rect.height <= 0.0 {
        return None;
    }
    let max_radius = 0.5 * rect.width.min(rect.height);
    let top_left = radius.top_left.clamp(0.0, max_radius);
    let top_right = radius.top_right.clamp(0.0, max_radius);
    let bottom_right = radius.bottom_right.clamp(0.0, max_radius);
    let bottom_left = radius.bottom_left.clamp(0.0, max_radius);
    if top_left <= 0.0 && top_right <= 0.0 && bottom_right <= 0.0 && bottom_left <= 0.0 {
        let mut builder = PathBuilder::new();
        builder.push_rect(tiny_skia::Rect::from_xywh(
            rect.x,
            rect.y,
            rect.width,
            rect.height,
        )?);
        return builder.finish();
    }
    let (x, y) = (rect.x, rect.y);
    let (right, bottom) = (rect.x + rect.width, rect.y + rect.height);
    let mut builder = PathBuilder::new();
    builder.move_to(x + top_left, y);
    builder.line_to(right - top_right, y);
    if top_right > 0.0 {
        builder.cubic_to(
            right - top_right + KAPPA * top_right,
            y,
            right,
            y + top_right - KAPPA * top_right,
            right,
            y + top_right,
        );
    }
    builder.line_to(right, bottom - bottom_right);
    if bottom_right > 0.0 {
        builder.cubic_to(
            right,
            bottom - bottom_right + KAPPA * bottom_right,
            right - bottom_right + KAPPA * bottom_right,
            bottom,
            right - bottom_right,
            bottom,
        );
    }
    builder.line_to(x + bottom_left, bottom);
    if bottom_left > 0.0 {
        builder.cubic_to(
            x + bottom_left - KAPPA * bottom_left,
            bottom,
            x,
            bottom - bottom_left + KAPPA * bottom_left,
            x,
            bottom - bottom_left,
        );
    }
    builder.line_to(x, y + top_left);
    if top_left > 0.0 {
        builder.cubic_to(
            x,
            y + top_left - KAPPA * top_left,
            x + top_left - KAPPA * top_left,
            y,
            x + top_left,
            y,
        );
    }
    builder.close();
    builder.finish()
}

fn elements_path(elements: &[PathElement]) -> Option<tiny_skia::Path> {
    let mut builder = PathBuilder::new();
    for element in elements {
        match element {
            PathElement::MoveTo(point) => builder.move_to(point.x, point.y),
            PathElement::LineTo(point) => builder.line_to(point.x, point.y),
            PathElement::QuadTo { ctrl, to } => builder.quad_to(ctrl.x, ctrl.y, to.x, to.y),
            PathElement::CubicTo { ctrl1, ctrl2, to } => {
                builder.cubic_to(ctrl1.x, ctrl1.y, ctrl2.x, ctrl2.y, to.x, to.y);
            }
            PathElement::Close => builder.close(),
        }
    }
    builder.finish()
}

fn zeno_path(outline: &swash::scale::outline::Outline) -> Option<tiny_skia::Path> {
    let mut builder = PathBuilder::new();
    for command in outline.path().commands() {
        match command {
            ZenoCommand::MoveTo(to) => builder.move_to(to.x, -to.y),
            ZenoCommand::LineTo(to) => builder.line_to(to.x, -to.y),
            ZenoCommand::QuadTo(ctrl, to) => builder.quad_to(ctrl.x, -ctrl.y, to.x, -to.y),
            ZenoCommand::CurveTo(ctrl1, ctrl2, to) => {
                builder.cubic_to(ctrl1.x, -ctrl1.y, ctrl2.x, -ctrl2.y, to.x, -to.y);
            }
            ZenoCommand::Close => builder.close(),
        }
    }
    builder.finish()
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn render_image_pixmap(image: &stern::render::RenderImage, tint: Option<Color>) -> Option<Pixmap> {
    use stern::render::{RenderImageAlpha, RenderImageFormat};

    let mut pixmap = Pixmap::new(image.width.max(1), image.height.max(1))?;
    let source = image.data.as_ref();
    let expected = (image.width as usize) * (image.height as usize) * 4;
    if source.len() != expected {
        return None;
    }
    let tint = tint.map(|color| {
        [
            color.r.clamp(0.0, 1.0),
            color.g.clamp(0.0, 1.0),
            color.b.clamp(0.0, 1.0),
            color.a.clamp(0.0, 1.0),
        ]
    });
    let data = pixmap.data_mut();
    for (index, chunk) in source.chunks_exact(4).enumerate() {
        let (red, green, blue, alpha) = match image.format {
            RenderImageFormat::Rgba8 => (chunk[0], chunk[1], chunk[2], chunk[3]),
            RenderImageFormat::Bgra8 => (chunk[2], chunk[1], chunk[0], chunk[3]),
        };
        let mut channels = [
            f32::from(red) / 255.0,
            f32::from(green) / 255.0,
            f32::from(blue) / 255.0,
            f32::from(alpha) / 255.0,
        ];
        if image.alpha == RenderImageAlpha::Premultiplied && channels[3] > 0.0 {
            channels[0] /= channels[3];
            channels[1] /= channels[3];
            channels[2] /= channels[3];
        }
        if let Some(tint) = tint {
            channels[0] *= tint[0];
            channels[1] *= tint[1];
            channels[2] *= tint[2];
            channels[3] *= tint[3];
        }
        // tiny-skia stores premultiplied RGBA.
        let alpha = channels[3].clamp(0.0, 1.0);
        let out = &mut data[index * 4..index * 4 + 4];
        out[0] = (channels[0].clamp(0.0, 1.0) * alpha * 255.0).round() as u8;
        out[1] = (channels[1].clamp(0.0, 1.0) * alpha * 255.0).round() as u8;
        out[2] = (channels[2].clamp(0.0, 1.0) * alpha * 255.0).round() as u8;
        out[3] = (alpha * 255.0).round() as u8;
    }
    Some(pixmap)
}

/// Approximate Gaussian blur with three box-blur passes (deterministic
/// integer image walk, f32 accumulation).
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn box_blur(pixmap: &mut Pixmap, sigma: f32) {
    let radius = (sigma * 1.5).round().max(1.0) as usize;
    let width = pixmap.width() as usize;
    let height = pixmap.height() as usize;
    for _ in 0..3 {
        blur_axis(pixmap.data_mut(), width, height, radius, true);
        blur_axis(pixmap.data_mut(), width, height, radius, false);
    }
}

#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
fn blur_axis(data: &mut [u8], width: usize, height: usize, radius: usize, horizontal: bool) {
    let (major, minor) = if horizontal {
        (height, width)
    } else {
        (width, height)
    };
    let mut line = vec![[0.0_f32; 4]; minor];
    for row in 0..major {
        for (position, sample) in line.iter_mut().enumerate() {
            let index = if horizontal {
                (row * width + position) * 4
            } else {
                (position * width + row) * 4
            };
            *sample = [
                f32::from(data[index]),
                f32::from(data[index + 1]),
                f32::from(data[index + 2]),
                f32::from(data[index + 3]),
            ];
        }
        let window = (2 * radius + 1) as f32;
        for position in 0..minor {
            let mut sum = [0.0_f32; 4];
            let window_start = position.saturating_sub(radius);
            let window_end = (position + radius + 1).min(minor);
            for sample in &line[window_start..window_end] {
                for (channel, value) in sum.iter_mut().enumerate() {
                    *value += sample[channel];
                }
            }
            let index = if horizontal {
                (row * width + position) * 4
            } else {
                (position * width + row) * 4
            };
            for channel in 0..4 {
                data[index + channel] = (sum[channel] / window).round().clamp(0.0, 255.0) as u8;
            }
        }
    }
}
