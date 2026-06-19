//! Vello renderer boundary for Kinetik UI render primitives.

use std::{collections::HashMap, sync::Arc};

use kinetik_ui_core::{
    Brush, ClipId, Color, CornerRadius, ImageId, LayerId, LinearGradient, PathElement, Point,
    Primitive, Rect, ShadowPrimitive, Size, Stroke, TextLayoutId, TextureId, Transform, Vec2,
    ViewportInfo,
};
pub use kinetik_ui_render::{
    ImageAtlasRegion, ImageResource, RenderDiagnostic, RenderFrameInput, RenderFrameOutput,
    RenderImage, RenderImageAlpha, RenderImageFormat, RenderImageSampling, RenderResources,
    RendererBackend, TextLayoutResource, TextureResource, Translation as RenderTranslation,
};
use kinetik_ui_text::{CosmicTextEngine, ShapedTextLayout, TextLayoutKey, TextStyle};
use vello::{
    Glyph, Scene,
    kurbo::{
        Affine, BezPath, Line as KurboLine, Rect as KurboRect, RoundedRect, RoundedRectRadii, Shape,
    },
    peniko::{
        Blob, Fill, Gradient as PenikoGradient, ImageAlphaType, ImageBrush, ImageData, ImageFormat,
        ImageQuality,
    },
};

/// Deterministic command produced before backend drawing.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderCommand {
    /// Layer used by the command.
    pub layer: LayerId,
    /// Clip stack active for the command, outermost to innermost.
    pub clips: Vec<RenderClip>,
    /// Transform used by the command.
    pub transform: Transform,
    /// Command kind.
    pub kind: RenderCommandKind,
}

/// Clip scope captured for a render command.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderClip {
    /// Clip rectangle.
    pub rect: Rect,
    /// Transform active when the clip scope began.
    pub transform: Transform,
}

/// Command kind produced by primitive translation.
#[derive(Debug, Clone, PartialEq)]
pub enum RenderCommandKind {
    /// Filled and/or stroked rectangle.
    Rect {
        /// Rectangle bounds.
        rect: Rect,
        /// Fill brush.
        fill: Option<Brush>,
        /// Stroke style.
        stroke: Option<Stroke>,
        /// Corner radii.
        radius: CornerRadius,
    },
    /// Stroked line.
    Line {
        /// Start x.
        x0: f32,
        /// Start y.
        y0: f32,
        /// End x.
        x1: f32,
        /// End y.
        y1: f32,
        /// Stroke style.
        stroke: Stroke,
    },
    /// Box shadow.
    Shadow {
        /// Source rectangle.
        rect: Rect,
        /// Shadow offset.
        offset: Vec2,
        /// Gaussian blur radius.
        blur_radius: f32,
        /// Spread amount.
        spread: f32,
        /// Uniform corner radius.
        radius: f32,
        /// Shadow color.
        color: Color,
    },
    /// Filled and/or stroked vector path.
    Path {
        /// Path elements in drawing order.
        elements: Vec<PathElement>,
        /// Fill brush.
        fill: Option<Brush>,
        /// Stroke style.
        stroke: Option<Stroke>,
    },
    /// Text command backed by a shaped layout resource or renderer fallback shaping.
    Text {
        /// Optional shaped layout resource.
        layout: Option<TextLayoutId>,
        /// Baseline origin.
        origin: kinetik_ui_core::Point,
        /// Text content.
        text: String,
        /// Font family name or logical family.
        family: String,
        /// Font size in logical units.
        size: f32,
        /// Line height in logical units.
        line_height: f32,
        /// Text color.
        color: Color,
    },
    /// Image resource draw command.
    Image {
        /// Image resource.
        image: ImageId,
        /// Destination rectangle.
        rect: Rect,
    },
    /// Texture resource draw command.
    Texture {
        /// Texture resource.
        texture: TextureId,
        /// Destination rectangle.
        rect: Rect,
        /// Source size in texture pixels.
        source_size: Size,
    },
}

/// Vello renderer boundary.
pub struct VelloRenderer {
    scene: Scene,
    text_engine: CosmicTextEngine,
    image_cache: ImageDataCache,
}

impl VelloRenderer {
    /// Creates a renderer boundary with an empty Vello scene.
    #[must_use]
    pub fn new() -> Self {
        Self {
            scene: Scene::new(),
            text_engine: CosmicTextEngine::new(),
            image_cache: ImageDataCache::default(),
        }
    }

    /// Returns the current Vello scene.
    #[must_use]
    pub const fn scene(&self) -> &Scene {
        &self.scene
    }

    #[cfg(test)]
    fn cached_image_count(&self) -> usize {
        self.image_cache.len()
    }

    /// Submits a frame for translation.
    pub fn submit_frame(&mut self, input: RenderFrameInput<'_>) -> RenderFrameOutput {
        let translated = translate_primitives(input.primitives, input.resources);
        self.scene.reset();
        encode_scene(
            &mut self.scene,
            &translated.commands,
            input.resources,
            &mut self.text_engine,
            &mut self.image_cache,
            viewport_device_scale(input.viewport),
        );
        RenderFrameOutput {
            primitive_count: input.primitives.len(),
            diagnostics: translated.diagnostics,
        }
    }
}

#[derive(Debug, Clone)]
struct CachedImageData {
    signature: ImageSignature,
    data: ImageData,
}

#[derive(Debug, Clone)]
struct ImageSignature {
    width: u32,
    height: u32,
    format: RenderImageFormat,
    alpha: RenderImageAlpha,
    data: Arc<[u8]>,
}

impl ImageSignature {
    fn matches(&self, image: &RenderImage) -> bool {
        self.width == image.width
            && self.height == image.height
            && self.format == image.format
            && self.alpha == image.alpha
            && Arc::ptr_eq(&self.data, &image.data)
    }
}

#[derive(Debug, Default)]
struct ImageDataCache {
    images: HashMap<ImageId, CachedImageData>,
}

impl ImageDataCache {
    fn image_data(&mut self, id: ImageId, image: &RenderImage) -> ImageData {
        let signature = image_signature(image);
        if let Some(cached) = self.images.get(&id)
            && cached.signature.matches(image)
        {
            return cached.data.clone();
        }

        let data = image_data_from_render_image(image);
        self.images.insert(
            id,
            CachedImageData {
                signature,
                data: data.clone(),
            },
        );
        data
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.images.len()
    }
}

fn image_signature(image: &RenderImage) -> ImageSignature {
    ImageSignature {
        width: image.width,
        height: image.height,
        format: image.format,
        alpha: image.alpha,
        data: Arc::clone(&image.data),
    }
}

fn image_data_from_render_image(image: &RenderImage) -> ImageData {
    ImageData {
        data: Blob::from(image.data.to_vec()),
        format: image_format(image.format),
        alpha_type: image_alpha(image.alpha),
        width: image.width,
        height: image.height,
    }
}

impl Default for VelloRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl RendererBackend for VelloRenderer {
    type Error = core::convert::Infallible;

    fn render_frame(
        &mut self,
        input: RenderFrameInput<'_>,
    ) -> Result<RenderFrameOutput, Self::Error> {
        Ok(self.submit_frame(input))
    }
}

/// Translation result used by tests and renderer internals.
pub type Translation = RenderTranslation<RenderCommand>;

/// Translates primitives into deterministic renderer commands.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn translate_primitives(primitives: &[Primitive], resources: &RenderResources) -> Translation {
    let mut commands = Vec::new();
    let mut diagnostics = Vec::new();
    let mut layers = vec![LayerId::from_raw(0)];
    let mut clips = Vec::<(ClipId, RenderClip)>::new();
    let mut transforms = Vec::<Transform>::new();
    let mut transform = Transform::IDENTITY;

    for primitive in primitives {
        match primitive {
            Primitive::Rect(rect) => {
                let Some(rect_bounds) = sanitize_rect(rect.rect, &mut diagnostics, "rect") else {
                    continue;
                };
                commands.push(render_command(
                    &layers,
                    &clips,
                    transform,
                    RenderCommandKind::Rect {
                        rect: rect_bounds,
                        fill: rect
                            .fill
                            .map(|brush| sanitize_brush(brush, &mut diagnostics, "rect_fill")),
                        stroke: rect.stroke.and_then(|stroke| {
                            sanitize_stroke(stroke, &mut diagnostics, "rect_stroke")
                        }),
                        radius: sanitize_radius(rect.radius, &mut diagnostics, "rect_radius"),
                    },
                ));
            }
            Primitive::Line(line) => {
                let Some(from) = sanitize_point(line.from, &mut diagnostics, "line") else {
                    continue;
                };
                let Some(to) = sanitize_point(line.to, &mut diagnostics, "line") else {
                    continue;
                };
                let Some(stroke) = sanitize_stroke(line.stroke, &mut diagnostics, "line_stroke")
                else {
                    continue;
                };
                commands.push(render_command(
                    &layers,
                    &clips,
                    transform,
                    RenderCommandKind::Line {
                        x0: from.x,
                        y0: from.y,
                        x1: to.x,
                        y1: to.y,
                        stroke,
                    },
                ));
            }
            Primitive::Shadow(shadow) => {
                let Some(shadow) = sanitize_shadow(*shadow, &mut diagnostics) else {
                    continue;
                };
                commands.push(render_command(
                    &layers,
                    &clips,
                    transform,
                    RenderCommandKind::Shadow {
                        rect: shadow.rect,
                        offset: shadow.offset,
                        blur_radius: shadow.blur_radius,
                        spread: shadow.spread,
                        radius: shadow.radius,
                        color: shadow.color,
                    },
                ));
            }
            Primitive::Path(path) => {
                let Some(elements) =
                    sanitize_path_elements(&path.elements, &mut diagnostics, "path")
                else {
                    continue;
                };
                commands.push(render_command(
                    &layers,
                    &clips,
                    transform,
                    RenderCommandKind::Path {
                        elements,
                        fill: path
                            .fill
                            .map(|brush| sanitize_brush(brush, &mut diagnostics, "path_fill")),
                        stroke: path.stroke.and_then(|stroke| {
                            sanitize_stroke(stroke, &mut diagnostics, "path_stroke")
                        }),
                    },
                ));
            }
            Primitive::Text(text) => {
                let Some(origin) = sanitize_point(text.origin, &mut diagnostics, "text") else {
                    continue;
                };
                let Some(size) = finite_positive(text.size) else {
                    diagnostics.push(RenderDiagnostic::InvalidGeometry("text_size"));
                    continue;
                };
                let Some(line_height) = finite_positive(text.line_height) else {
                    diagnostics.push(RenderDiagnostic::InvalidGeometry("text_line_height"));
                    continue;
                };
                match text.layout {
                    Some(layout) if !resources.has_text_layout(layout) => {
                        diagnostics.push(RenderDiagnostic::MissingTextLayout(layout));
                    }
                    Some(_) | None => {}
                }
                commands.push(render_command(
                    &layers,
                    &clips,
                    transform,
                    RenderCommandKind::Text {
                        layout: text.layout,
                        origin,
                        text: text.text.clone(),
                        family: text.family.clone(),
                        size,
                        line_height,
                        color: brush_fallback_color(&sanitize_brush(
                            text.brush,
                            &mut diagnostics,
                            "text_brush",
                        )),
                    },
                ));
            }
            Primitive::Image(image) => {
                let Some(rect) = sanitize_rect(image.rect, &mut diagnostics, "image") else {
                    continue;
                };
                validate_image_resource(resources, image.image, &mut diagnostics);
                commands.push(render_command(
                    &layers,
                    &clips,
                    transform,
                    RenderCommandKind::Image {
                        image: image.image,
                        rect,
                    },
                ));
            }
            Primitive::Texture(texture) => {
                let Some(rect) = sanitize_rect(texture.rect, &mut diagnostics, "texture") else {
                    continue;
                };
                let Some(source_size) = sanitize_size(texture.source_size) else {
                    diagnostics.push(RenderDiagnostic::InvalidGeometry("texture_source_size"));
                    continue;
                };
                match resources.texture(texture.texture) {
                    None => diagnostics.push(RenderDiagnostic::MissingTexture(texture.texture)),
                    Some(resource) if !logical_size_matches(source_size, resource.size) => {
                        diagnostics.push(RenderDiagnostic::InvalidGeometry("texture_source_size"));
                        continue;
                    }
                    Some(resource) if resource.snapshot.is_none() => {
                        diagnostics.push(RenderDiagnostic::MissingTextureSnapshot(texture.texture));
                    }
                    Some(resource)
                        if resource.snapshot.as_ref().is_some_and(|snapshot| {
                            !logical_size_matches_snapshot(resource.size, snapshot)
                        }) =>
                    {
                        diagnostics
                            .push(RenderDiagnostic::InvalidGeometry("texture_snapshot_size"));
                        continue;
                    }
                    Some(_) => {}
                }
                commands.push(render_command(
                    &layers,
                    &clips,
                    transform,
                    RenderCommandKind::Texture {
                        texture: texture.texture,
                        rect,
                        source_size,
                    },
                ));
            }
            Primitive::ClipBegin { id, rect } => {
                let Some(rect) = sanitize_rect(*rect, &mut diagnostics, "clip") else {
                    continue;
                };
                clips.push((*id, RenderClip { rect, transform }));
            }
            Primitive::ClipEnd { id } => {
                if clips.last().is_some_and(|(open_id, _)| open_id == id) {
                    clips.pop();
                } else {
                    diagnostics.push(RenderDiagnostic::InvalidGeometry("clip_stack"));
                }
            }
            Primitive::LayerBegin { id } => {
                layers.push(*id);
            }
            Primitive::LayerEnd { id } => {
                if layers.len() > 1 && layers.last() == Some(id) {
                    layers.pop();
                } else {
                    diagnostics.push(RenderDiagnostic::InvalidGeometry("layer_stack"));
                }
            }
            Primitive::TransformBegin(next_transform) => {
                let Some(next_transform) =
                    sanitize_transform(*next_transform, &mut diagnostics, "transform")
                else {
                    continue;
                };
                transforms.push(transform);
                let next = compose_transform(transform, next_transform);
                if transform_is_finite(next) {
                    transform = next;
                } else {
                    diagnostics.push(RenderDiagnostic::InvalidGeometry("transform"));
                }
            }
            Primitive::TransformEnd => {
                if let Some(previous) = transforms.pop() {
                    transform = previous;
                } else {
                    diagnostics.push(RenderDiagnostic::InvalidGeometry("transform_stack"));
                    transform = Transform::IDENTITY;
                }
            }
        }
    }
    if !clips.is_empty() {
        diagnostics.push(RenderDiagnostic::InvalidGeometry("clip_stack"));
    }
    if layers.len() > 1 {
        diagnostics.push(RenderDiagnostic::InvalidGeometry("layer_stack"));
    }
    if !transforms.is_empty() {
        diagnostics.push(RenderDiagnostic::InvalidGeometry("transform_stack"));
    }

    Translation {
        commands,
        diagnostics,
    }
}

fn validate_image_resource(
    resources: &RenderResources,
    image: ImageId,
    diagnostics: &mut Vec<RenderDiagnostic>,
) {
    let Some(resource) = resources.image(image) else {
        diagnostics.push(RenderDiagnostic::MissingImage(image));
        return;
    };
    if resource.pixels.is_some() {
        return;
    }
    let Some(region) = resource.atlas_region else {
        diagnostics.push(RenderDiagnostic::MissingImagePixels(image));
        return;
    };
    if !atlas_source_is_finite_positive(region.source) {
        diagnostics.push(RenderDiagnostic::InvalidGeometry("image_atlas_source"));
        return;
    }
    let Some(atlas) = resources.image(region.atlas) else {
        diagnostics.push(RenderDiagnostic::MissingImage(region.atlas));
        return;
    };
    let Some(pixels) = atlas.pixels.as_ref() else {
        diagnostics.push(RenderDiagnostic::MissingImagePixels(region.atlas));
        return;
    };
    if !atlas_source_fits_image(region.source, pixels) {
        diagnostics.push(RenderDiagnostic::InvalidGeometry("image_atlas_source"));
    }
}

/// Formats a translated command stream as stable line-oriented snapshot text.
#[must_use]
pub fn render_translation_snapshot(translation: &Translation) -> String {
    let mut lines = Vec::new();
    lines.push("commands:".to_owned());
    for (index, command) in translation.commands.iter().enumerate() {
        lines.push(format!(
            "  {index}: layer={} transform={} clips={} {}",
            command.layer.raw(),
            format_transform(command.transform),
            format_clips(&command.clips),
            format_command_kind(&command.kind),
        ));
    }
    lines.push("diagnostics:".to_owned());
    for diagnostic in &translation.diagnostics {
        lines.push(format!("  {}", format_diagnostic(diagnostic)));
    }
    lines.join("\n")
}

fn format_command_kind(kind: &RenderCommandKind) -> String {
    match kind {
        RenderCommandKind::Rect {
            rect,
            fill,
            stroke,
            radius,
        } => format!(
            "rect rect={} fill={} stroke={} radius={}",
            format_rect(*rect),
            format_optional_brush(fill.as_ref()),
            format_optional_stroke(stroke.as_ref()),
            format_radius(*radius),
        ),
        RenderCommandKind::Line {
            x0,
            y0,
            x1,
            y1,
            stroke,
        } => format!(
            "line from=({}, {}) to=({}, {}) stroke={}",
            format_f32(*x0),
            format_f32(*y0),
            format_f32(*x1),
            format_f32(*y1),
            format_stroke(*stroke),
        ),
        RenderCommandKind::Shadow {
            rect,
            offset,
            blur_radius,
            spread,
            radius,
            color,
        } => format!(
            "shadow rect={} offset=({}, {}) blur={} spread={} radius={} color={}",
            format_rect(*rect),
            format_f32(offset.x),
            format_f32(offset.y),
            format_f32(*blur_radius),
            format_f32(*spread),
            format_f32(*radius),
            format_color(*color),
        ),
        RenderCommandKind::Path {
            elements,
            fill,
            stroke,
        } => format!(
            "path elements={} fill={} stroke={}",
            format_path_elements(elements),
            format_optional_brush(fill.as_ref()),
            format_optional_stroke(stroke.as_ref()),
        ),
        RenderCommandKind::Text {
            layout,
            origin,
            text,
            family,
            size,
            line_height,
            color,
        } => format!(
            "text layout={} origin=({}, {}) family={:?} size={} line_height={} color={} text={:?}",
            layout.map_or_else(|| "none".to_owned(), |layout| layout.raw().to_string()),
            format_f32(origin.x),
            format_f32(origin.y),
            family,
            format_f32(*size),
            format_f32(*line_height),
            format_color(*color),
            text,
        ),
        RenderCommandKind::Image { image, rect } => {
            format!("image#{} rect={}", image.raw(), format_rect(*rect))
        }
        RenderCommandKind::Texture {
            texture,
            rect,
            source_size,
        } => {
            format!(
                "texture#{} rect={} source_size={}",
                texture.raw(),
                format_rect(*rect),
                format_size(*source_size)
            )
        }
    }
}

fn format_clips(clips: &[RenderClip]) -> String {
    if clips.is_empty() {
        return "[]".to_owned();
    }
    let clips = clips
        .iter()
        .map(|clip| {
            format!(
                "{{rect={} transform={}}}",
                format_rect(clip.rect),
                format_transform(clip.transform)
            )
        })
        .collect::<Vec<_>>();
    format!("[{}]", clips.join(", "))
}

fn format_path_elements(elements: &[PathElement]) -> String {
    let elements = elements
        .iter()
        .map(|element| match element {
            PathElement::MoveTo(point) => {
                format!("M({}, {})", format_f32(point.x), format_f32(point.y))
            }
            PathElement::LineTo(point) => {
                format!("L({}, {})", format_f32(point.x), format_f32(point.y))
            }
            PathElement::QuadTo { ctrl, to } => format!(
                "Q({}, {}; {}, {})",
                format_f32(ctrl.x),
                format_f32(ctrl.y),
                format_f32(to.x),
                format_f32(to.y),
            ),
            PathElement::CubicTo { ctrl1, ctrl2, to } => format!(
                "C({}, {}; {}, {}; {}, {})",
                format_f32(ctrl1.x),
                format_f32(ctrl1.y),
                format_f32(ctrl2.x),
                format_f32(ctrl2.y),
                format_f32(to.x),
                format_f32(to.y),
            ),
            PathElement::Close => "Z".to_owned(),
        })
        .collect::<Vec<_>>();
    format!("[{}]", elements.join(", "))
}

fn format_optional_brush(brush: Option<&Brush>) -> String {
    brush.map_or_else(|| "none".to_owned(), |brush| format_brush(*brush))
}

fn format_brush(brush: Brush) -> String {
    match brush {
        Brush::Solid(color) => format_color(color),
        Brush::LinearGradient(gradient) => {
            let stops = gradient
                .stops()
                .iter()
                .map(|stop| format!("{}@{}", format_color(stop.color), format_f32(stop.offset)))
                .collect::<Vec<_>>();
            format!(
                "linear({},{})-({},{})[{}]",
                format_f32(gradient.start().x),
                format_f32(gradient.start().y),
                format_f32(gradient.end().x),
                format_f32(gradient.end().y),
                stops.join(",")
            )
        }
    }
}

fn format_optional_stroke(stroke: Option<&Stroke>) -> String {
    stroke.map_or_else(|| "none".to_owned(), |stroke| format_stroke(*stroke))
}

fn format_stroke(stroke: Stroke) -> String {
    format!(
        "{} {}",
        format_f32(stroke.width),
        format_brush(stroke.brush)
    )
}

fn format_rect(rect: Rect) -> String {
    format!(
        "({}, {}, {}, {})",
        format_f32(rect.x),
        format_f32(rect.y),
        format_f32(rect.width),
        format_f32(rect.height),
    )
}

fn format_size(size: Size) -> String {
    format!("{}x{}", format_f32(size.width), format_f32(size.height))
}

fn format_radius(radius: CornerRadius) -> String {
    format!(
        "({}, {}, {}, {})",
        format_f32(radius.top_left),
        format_f32(radius.top_right),
        format_f32(radius.bottom_right),
        format_f32(radius.bottom_left),
    )
}

fn format_transform(transform: Transform) -> String {
    format!(
        "[{}, {}, {}, {}, {}, {}]",
        format_f32(transform.m11),
        format_f32(transform.m12),
        format_f32(transform.m21),
        format_f32(transform.m22),
        format_f32(transform.dx),
        format_f32(transform.dy),
    )
}

fn format_color(color: Color) -> String {
    format!(
        "rgba({}, {}, {}, {})",
        format_f32(color.r),
        format_f32(color.g),
        format_f32(color.b),
        format_f32(color.a),
    )
}

fn format_diagnostic(diagnostic: &RenderDiagnostic) -> String {
    match diagnostic {
        RenderDiagnostic::MissingTextLayout(id) => format!("missing_text_layout#{}", id.raw()),
        RenderDiagnostic::MissingImage(id) => format!("missing_image#{}", id.raw()),
        RenderDiagnostic::MissingImagePixels(id) => {
            format!("missing_image_pixels#{}", id.raw())
        }
        RenderDiagnostic::MissingTexture(id) => format!("missing_texture#{}", id.raw()),
        RenderDiagnostic::MissingTextureSnapshot(id) => {
            format!("missing_texture_snapshot#{}", id.raw())
        }
        RenderDiagnostic::UnsupportedPrimitive(kind) => format!("unsupported_primitive:{kind}"),
        RenderDiagnostic::InvalidGeometry(kind) => format!("invalid_geometry:{kind}"),
    }
}

fn format_f32(value: f32) -> String {
    let value = if value.is_finite() { value } else { 0.0 };
    let value = if value == 0.0 { 0.0 } else { value };
    format!("{value:.3}")
}

fn render_command(
    layers: &[LayerId],
    clips: &[(ClipId, RenderClip)],
    transform: Transform,
    kind: RenderCommandKind,
) -> RenderCommand {
    RenderCommand {
        layer: layers
            .last()
            .copied()
            .unwrap_or_else(|| LayerId::from_raw(0)),
        clips: clips.iter().map(|(_, clip)| *clip).collect(),
        transform,
        kind,
    }
}

fn brush_fallback_color(brush: &Brush) -> Color {
    match brush {
        Brush::Solid(color) => *color,
        Brush::LinearGradient(gradient) => gradient
            .stops()
            .first()
            .map_or(Color::TRANSPARENT, |stop| stop.color),
    }
}

fn finite_non_negative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn finite_or_zero(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

fn finite_positive(value: f32) -> Option<f32> {
    value
        .is_finite()
        .then_some(value)
        .filter(|value| *value > 0.0)
}

fn finite_unit(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn sanitize_point(
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

fn sanitize_vec2(
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

fn sanitize_size(size: Size) -> Option<Size> {
    Some(Size::new(
        finite_positive(size.width)?,
        finite_positive(size.height)?,
    ))
}

fn sanitize_rect(
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

fn sanitize_color(
    color: Color,
    diagnostics: &mut Vec<RenderDiagnostic>,
    context: &'static str,
) -> Color {
    if color.r.is_finite() && color.g.is_finite() && color.b.is_finite() && color.a.is_finite() {
        color
    } else {
        diagnostics.push(RenderDiagnostic::InvalidGeometry(context));
        Color::rgba(
            finite_unit(color.r),
            finite_unit(color.g),
            finite_unit(color.b),
            finite_unit(color.a),
        )
    }
}

fn sanitize_brush(
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

fn sanitize_linear_gradient(
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

fn sanitize_stroke(
    stroke: Stroke,
    diagnostics: &mut Vec<RenderDiagnostic>,
    context: &'static str,
) -> Option<Stroke> {
    let Some(width) = finite_positive(stroke.width) else {
        diagnostics.push(RenderDiagnostic::InvalidGeometry(context));
        return None;
    };
    Some(Stroke::new(
        width,
        sanitize_brush(stroke.brush, diagnostics, context),
    ))
}

fn sanitize_shadow(
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

fn sanitize_non_negative(
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

fn sanitize_finite(
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

fn sanitize_radius(
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

fn sanitize_path_elements(
    elements: &[PathElement],
    diagnostics: &mut Vec<RenderDiagnostic>,
    context: &'static str,
) -> Option<Vec<PathElement>> {
    if elements.is_empty() {
        diagnostics.push(RenderDiagnostic::InvalidGeometry(context));
        return None;
    }

    let mut sanitized = Vec::with_capacity(elements.len());
    let mut saw_point = false;
    for element in elements {
        match *element {
            PathElement::MoveTo(point) => {
                let point = sanitize_point(point, diagnostics, context)?;
                saw_point = true;
                sanitized.push(PathElement::MoveTo(point));
            }
            PathElement::LineTo(point) => {
                let point = sanitize_point(point, diagnostics, context)?;
                saw_point = true;
                sanitized.push(PathElement::LineTo(point));
            }
            PathElement::QuadTo { ctrl, to } => {
                let ctrl = sanitize_point(ctrl, diagnostics, context)?;
                let to = sanitize_point(to, diagnostics, context)?;
                saw_point = true;
                sanitized.push(PathElement::QuadTo { ctrl, to });
            }
            PathElement::CubicTo { ctrl1, ctrl2, to } => {
                let ctrl1 = sanitize_point(ctrl1, diagnostics, context)?;
                let ctrl2 = sanitize_point(ctrl2, diagnostics, context)?;
                let to = sanitize_point(to, diagnostics, context)?;
                saw_point = true;
                sanitized.push(PathElement::CubicTo { ctrl1, ctrl2, to });
            }
            PathElement::Close => sanitized.push(PathElement::Close),
        }
    }

    if saw_point {
        Some(sanitized)
    } else {
        diagnostics.push(RenderDiagnostic::InvalidGeometry(context));
        None
    }
}

fn transform_is_finite(transform: Transform) -> bool {
    transform.m11.is_finite()
        && transform.m12.is_finite()
        && transform.m21.is_finite()
        && transform.m22.is_finite()
        && transform.dx.is_finite()
        && transform.dy.is_finite()
}

fn sanitize_transform(
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

fn encode_scene(
    scene: &mut Scene,
    commands: &[RenderCommand],
    resources: &RenderResources,
    text_engine: &mut CosmicTextEngine,
    image_cache: &mut ImageDataCache,
    device_scale: f64,
) {
    let root_transform = root_transform(device_scale);
    for command in commands {
        for clip in &command.clips {
            scene.push_clip_layer(
                Fill::NonZero,
                snap_axis_aligned_translation(root_transform * transform_to_affine(clip.transform)),
                &kurbo_rect(snap_rect_to_device(clip.rect, device_scale)),
            );
        }

        encode_command(
            scene,
            command,
            resources,
            text_engine,
            image_cache,
            device_scale,
        );

        for _ in &command.clips {
            scene.pop_layer();
        }
    }
}

fn encode_command(
    scene: &mut Scene,
    command: &RenderCommand,
    resources: &RenderResources,
    text_engine: &mut CosmicTextEngine,
    image_cache: &mut ImageDataCache,
    device_scale: f64,
) {
    let transform = snap_axis_aligned_translation(
        root_transform(device_scale) * transform_to_affine(command.transform),
    );
    match &command.kind {
        RenderCommandKind::Rect {
            rect,
            fill,
            stroke,
            radius,
        } => encode_rect_command(
            scene,
            transform,
            *rect,
            *fill,
            *stroke,
            *radius,
            device_scale,
        ),
        RenderCommandKind::Line {
            x0,
            y0,
            x1,
            y1,
            stroke,
        } => encode_line_command(
            scene,
            transform,
            Point::new(*x0, *y0),
            Point::new(*x1, *y1),
            *stroke,
            device_scale,
        ),
        RenderCommandKind::Shadow {
            rect,
            offset,
            blur_radius,
            spread,
            radius,
            color,
        } => encode_shadow(
            scene,
            transform,
            ShadowPrimitive::new(*rect, *offset, *blur_radius, *spread, *radius, *color),
        ),
        RenderCommandKind::Path {
            elements,
            fill,
            stroke,
        } => encode_path(scene, transform, elements, *fill, *stroke, device_scale),
        RenderCommandKind::Text {
            layout,
            origin,
            text,
            family,
            size,
            line_height,
            color,
        } => encode_text_command(
            scene,
            transform,
            resources,
            text_engine,
            *layout,
            *origin,
            text,
            family,
            *size,
            *line_height,
            *color,
            device_scale,
        ),
        RenderCommandKind::Image { image, rect } => {
            encode_image_command(
                scene,
                transform,
                resources,
                image_cache,
                *image,
                *rect,
                device_scale,
            );
        }
        RenderCommandKind::Texture {
            texture,
            rect,
            source_size,
        } => {
            encode_texture_command(
                scene,
                transform,
                resources,
                *texture,
                *rect,
                *source_size,
                device_scale,
            );
        }
    }
}

fn encode_line_command(
    scene: &mut Scene,
    transform: Affine,
    from: Point,
    to: Point,
    stroke: Stroke,
    device_scale: f64,
) {
    let (from, to) = snap_stroked_line_to_device(from, to, stroke.width, device_scale);
    let line = KurboLine::new(
        (f64::from(from.x), f64::from(from.y)),
        (f64::from(to.x), f64::from(to.y)),
    );
    stroke_shape(scene, transform, &stroke, &line, device_scale);
}

fn encode_rect_command(
    scene: &mut Scene,
    transform: Affine,
    rect: Rect,
    fill: Option<Brush>,
    stroke: Option<Stroke>,
    radius: CornerRadius,
    device_scale: f64,
) {
    if let Some(fill) = fill {
        let shape = rounded_rect(snap_rect_to_device(rect, device_scale), radius);
        fill_shape(scene, transform, &fill, &shape);
    }
    if let Some(stroke) = stroke {
        let shape = rounded_rect(
            snap_stroked_rect_to_device(rect, stroke.width, device_scale),
            radius,
        );
        stroke_shape(scene, transform, &stroke, &shape, device_scale);
    }
}

#[allow(clippy::too_many_arguments)]
fn encode_text_command(
    scene: &mut Scene,
    transform: Affine,
    resources: &RenderResources,
    text_engine: &mut CosmicTextEngine,
    layout: Option<TextLayoutId>,
    origin: Point,
    text: &str,
    family: &str,
    size: f32,
    line_height: f32,
    color: Color,
    device_scale: f64,
) {
    if let Some(resource) = layout.and_then(|id| resources.text_layout_resource(id)) {
        let physical_layout = physical_text_layout_for_key(text_engine, transform, &resource.key);
        encode_text_layout(
            scene,
            transform,
            origin,
            &resource.layout,
            physical_layout.as_ref(),
            color,
            device_scale,
        );
    } else {
        let layout = shape_fallback_text(text_engine, text, family, size, line_height);
        let physical_layout =
            physical_text_layout(text_engine, transform, text, family, size, line_height);
        encode_text_layout(
            scene,
            transform,
            origin,
            &layout,
            physical_layout.as_ref(),
            color,
            device_scale,
        );
    }
}

fn encode_image_command(
    scene: &mut Scene,
    transform: Affine,
    resources: &RenderResources,
    image_cache: &mut ImageDataCache,
    image: ImageId,
    rect: Rect,
    device_scale: f64,
) {
    if let Some(draw) = resolve_image_draw(resources, image) {
        let rect = snap_image_rect_to_device(rect, draw.sampling, device_scale);
        encode_image_region(scene, transform, rect, image_cache, draw);
    } else {
        let rect = snap_rect_to_device(rect, device_scale);
        encode_resource_placeholder(
            scene,
            transform,
            rect,
            device_scale,
            Color::rgba(0.24, 0.32, 0.42, 0.35),
            Color::rgba(0.62, 0.72, 0.86, 0.75),
        );
    }
}

fn encode_texture_command(
    scene: &mut Scene,
    transform: Affine,
    resources: &RenderResources,
    texture: TextureId,
    rect: Rect,
    source_size: Size,
    device_scale: f64,
) {
    if let Some(resource) = resources.texture(texture)
        && let Some(snapshot) = resource.snapshot.as_ref()
        && source_size_matches_snapshot(source_size, snapshot)
    {
        let rect = snap_image_rect_to_device(rect, resource.sampling, device_scale);
        encode_image(scene, transform, rect, snapshot, resource.sampling);
    } else {
        let rect = snap_rect_to_device(rect, device_scale);
        encode_resource_placeholder(
            scene,
            transform,
            rect,
            device_scale,
            Color::rgba(0.20, 0.34, 0.24, 0.35),
            Color::rgba(0.60, 0.84, 0.62, 0.75),
        );
    }
}

#[derive(Debug, Clone, Copy)]
struct ResolvedImageDraw<'a> {
    payload: ImageId,
    pixels: &'a RenderImage,
    source: Rect,
    sampling: RenderImageSampling,
}

fn resolve_image_draw(
    resources: &RenderResources,
    image: ImageId,
) -> Option<ResolvedImageDraw<'_>> {
    let resource = resources.image(image)?;
    if let Some(pixels) = resource.pixels.as_ref() {
        return Some(ResolvedImageDraw {
            payload: image,
            pixels,
            source: full_image_source(pixels),
            sampling: resource.sampling,
        });
    }
    let region = resource.atlas_region?;
    let atlas = resources.image(region.atlas)?;
    let pixels = atlas.pixels.as_ref()?;
    atlas_source_fits_image(region.source, pixels).then_some(ResolvedImageDraw {
        payload: region.atlas,
        pixels,
        source: region.source,
        sampling: resource.sampling,
    })
}

fn encode_shadow(scene: &mut Scene, transform: Affine, shadow: ShadowPrimitive) {
    let shadow_rect = shadow
        .rect
        .translate(shadow.offset)
        .outset(shadow.spread)
        .max_zero();
    if shadow_rect.is_empty() {
        return;
    }
    let radius = (shadow.radius + shadow.spread).max(0.0);
    if shadow.blur_radius <= 0.0 {
        scene.fill(
            Fill::NonZero,
            transform,
            vello_color(shadow.color),
            None,
            &rounded_rect(shadow_rect, CornerRadius::all(radius)),
        );
        return;
    }
    scene.draw_blurred_rounded_rect(
        transform,
        kurbo_rect(shadow_rect),
        vello_color(shadow.color),
        f64::from(radius),
        f64::from(shadow.blur_radius),
    );
}

fn fill_shape(scene: &mut Scene, transform: Affine, brush: &Brush, shape: &impl Shape) {
    match brush {
        Brush::Solid(color) => {
            scene.fill(Fill::NonZero, transform, vello_color(*color), None, shape);
        }
        Brush::LinearGradient(gradient) => {
            let gradient = vello_gradient(gradient);
            scene.fill(Fill::NonZero, transform, &gradient, None, shape);
        }
    }
}

fn stroke_shape(
    scene: &mut Scene,
    transform: Affine,
    stroke: &Stroke,
    shape: &impl Shape,
    device_scale: f64,
) {
    let style = vello::kurbo::Stroke::new(f64::from(quantize_stroke_width_to_device(
        stroke.width,
        device_scale,
    )));
    match stroke.brush {
        Brush::Solid(color) => {
            scene.stroke(&style, transform, vello_color(color), None, shape);
        }
        Brush::LinearGradient(gradient) => {
            let gradient = vello_gradient(&gradient);
            scene.stroke(&style, transform, &gradient, None, shape);
        }
    }
}

fn encode_path(
    scene: &mut Scene,
    transform: Affine,
    elements: &[PathElement],
    fill: Option<Brush>,
    stroke: Option<Stroke>,
    device_scale: f64,
) {
    let path = bez_path(elements);
    if let Some(fill) = fill {
        fill_shape(scene, transform, &fill, &path);
    }
    if let Some(stroke) = stroke {
        stroke_shape(scene, transform, &stroke, &path, device_scale);
    }
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

fn shape_fallback_text(
    text_engine: &mut CosmicTextEngine,
    text: &str,
    family: &str,
    size: f32,
    line_height: f32,
) -> ShapedTextLayout {
    shape_text_with_key(
        text_engine,
        &TextLayoutKey::new(text, TextStyle::new(family, size, line_height), 0.0, false),
    )
}

fn shape_text_with_key(
    text_engine: &mut CosmicTextEngine,
    key: &TextLayoutKey,
) -> ShapedTextLayout {
    text_engine.shape_text(key)
}

fn shape_text_with_metrics(
    text_engine: &mut CosmicTextEngine,
    text: &str,
    family: &str,
    size: f32,
    line_height: f32,
) -> ShapedTextLayout {
    shape_text_with_key(
        text_engine,
        &TextLayoutKey::new(text, TextStyle::new(family, size, line_height), 0.0, false),
    )
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn physical_text_layout_for_key(
    text_engine: &mut CosmicTextEngine,
    transform: Affine,
    key: &TextLayoutKey,
) -> Option<ShapedTextLayout> {
    let scale = uniform_axis_aligned_scale(transform)?;
    let physical_size = (f64::from(key.style.size()) * scale) as f32;
    let physical_line_height = (f64::from(key.style.line_height()) * scale) as f32;
    let physical_width = (f64::from(key.width()) * scale) as f32;
    (physical_size.is_finite()
        && physical_size > 0.0
        && physical_line_height.is_finite()
        && physical_line_height > 0.0
        && physical_width.is_finite())
    .then(|| {
        shape_text_with_key(
            text_engine,
            &TextLayoutKey::new(
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
fn physical_text_layout(
    text_engine: &mut CosmicTextEngine,
    transform: Affine,
    text: &str,
    family: &str,
    size: f32,
    line_height: f32,
) -> Option<ShapedTextLayout> {
    let scale = uniform_axis_aligned_scale(transform)?;
    let physical_size = (f64::from(size) * scale) as f32;
    let physical_line_height = (f64::from(line_height) * scale) as f32;
    (physical_size.is_finite()
        && physical_size > 0.0
        && physical_line_height.is_finite()
        && physical_line_height > 0.0)
        .then(|| {
            shape_text_with_metrics(
                text_engine,
                text,
                family,
                physical_size,
                physical_line_height,
            )
        })
}

fn encode_text_layout(
    scene: &mut Scene,
    transform: Affine,
    origin: Point,
    layout: &ShapedTextLayout,
    physical_layout: Option<&ShapedTextLayout>,
    color: Color,
    device_scale: f64,
) {
    if let Some(scale) = uniform_axis_aligned_scale(transform) {
        let origin = transform_point(transform, origin);
        if let Some(physical_layout) = physical_layout {
            encode_shaped_text_device_space(scene, origin, physical_layout, color, 1.0);
        } else {
            encode_shaped_text_device_space(scene, origin, layout, color, scale);
        }
    } else {
        let origin = snap_point_to_device(origin, device_scale);
        encode_shaped_text(scene, transform, origin, layout, color);
    }
}

fn encode_shaped_text(
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
fn encode_shaped_text_device_space(
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
                    x: origin.x + glyph.x * scale as f32,
                    y: origin.y + glyph.y * scale as f32,
                }),
            );
    }
}

fn snap_text_origin_to_device(origin: Point) -> Point {
    Point::new(origin.x.round(), origin.y.round())
}

fn uniform_axis_aligned_scale(transform: Affine) -> Option<f64> {
    let coeffs = transform.as_coeffs();
    let scale_x = coeffs[0];
    let skew_y = coeffs[1];
    let skew_x = coeffs[2];
    let scale_y = coeffs[3];
    if skew_y.abs() <= f64::EPSILON
        && skew_x.abs() <= f64::EPSILON
        && scale_x.is_finite()
        && scale_y.is_finite()
        && scale_x > 0.0
        && (scale_x - scale_y).abs() <= f64::EPSILON
    {
        Some(scale_x)
    } else {
        None
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn transform_point(transform: Affine, point: Point) -> Point {
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

fn encode_image(
    scene: &mut Scene,
    transform: Affine,
    rect: Rect,
    image: &RenderImage,
    sampling: RenderImageSampling,
) {
    let source = full_image_source(image);
    if image.width == 0 || image.height == 0 || rect.width <= 0.0 || rect.height <= 0.0 {
        return;
    }
    if !atlas_source_is_finite_positive(source) || !atlas_source_fits_image(source, image) {
        return;
    }

    fill_image_region(
        scene,
        transform,
        rect,
        image_data_from_render_image(image),
        source,
        sampling,
    );
}

fn encode_image_region(
    scene: &mut Scene,
    transform: Affine,
    rect: Rect,
    image_cache: &mut ImageDataCache,
    draw: ResolvedImageDraw<'_>,
) {
    let image = draw.pixels;
    let source = draw.source;
    if image.width == 0 || image.height == 0 || rect.width <= 0.0 || rect.height <= 0.0 {
        return;
    }
    if !atlas_source_is_finite_positive(source) || !atlas_source_fits_image(source, image) {
        return;
    }

    fill_image_region(
        scene,
        transform,
        rect,
        image_cache.image_data(draw.payload, image),
        source,
        draw.sampling,
    );
}

fn fill_image_region(
    scene: &mut Scene,
    transform: Affine,
    rect: Rect,
    image_data: ImageData,
    source: Rect,
    sampling: RenderImageSampling,
) {
    let brush = ImageBrush::new(image_data).with_quality(image_quality(sampling));
    let scale_x = f64::from(rect.width) / f64::from(source.width);
    let scale_y = f64::from(rect.height) / f64::from(source.height);
    let image_transform = transform
        * Affine::translate((f64::from(rect.x), f64::from(rect.y)))
        * Affine::scale_non_uniform(scale_x, scale_y)
        * Affine::translate((-f64::from(source.x), -f64::from(source.y)));
    scene.fill(
        Fill::NonZero,
        image_transform,
        brush.as_ref(),
        None,
        &kurbo_rect(source),
    );
}

#[allow(clippy::cast_precision_loss)]
fn full_image_source(image: &RenderImage) -> Rect {
    Rect::new(0.0, 0.0, image.width as f32, image.height as f32)
}

fn atlas_source_is_finite_positive(source: Rect) -> bool {
    source.x.is_finite()
        && source.y.is_finite()
        && source.width.is_finite()
        && source.height.is_finite()
        && source.width > 0.0
        && source.height > 0.0
}

#[allow(clippy::cast_precision_loss)]
fn atlas_source_fits_image(source: Rect, image: &RenderImage) -> bool {
    atlas_source_is_finite_positive(source)
        && source.x >= 0.0
        && source.y >= 0.0
        && source.max_x() <= image.width as f32
        && source.max_y() <= image.height as f32
}

fn source_size_matches_snapshot(source_size: Size, image: &RenderImage) -> bool {
    (f64::from(source_size.width) - f64::from(image.width)).abs() <= f64::EPSILON
        && (f64::from(source_size.height) - f64::from(image.height)).abs() <= f64::EPSILON
}

#[allow(clippy::cast_precision_loss)]
fn logical_size_matches_snapshot(size: Size, image: &RenderImage) -> bool {
    logical_size_matches(size, Size::new(image.width as f32, image.height as f32))
}

fn image_quality(sampling: RenderImageSampling) -> ImageQuality {
    match sampling {
        RenderImageSampling::Pixelated | RenderImageSampling::UiIcon => ImageQuality::Low,
        RenderImageSampling::Smooth => ImageQuality::Medium,
        RenderImageSampling::HighQuality => ImageQuality::High,
    }
}

#[allow(clippy::cast_possible_truncation)]
fn quantize_stroke_width_to_device(width: f32, device_scale: f64) -> f32 {
    if width <= 0.0 || !width.is_finite() || !device_scale.is_finite() || device_scale <= 0.0 {
        return width;
    }

    let physical_width = (f64::from(width) * device_scale).round().max(1.0);
    (physical_width / device_scale) as f32
}

fn logical_size_matches(lhs: Size, rhs: Size) -> bool {
    (lhs.width - rhs.width).abs() <= f32::EPSILON && (lhs.height - rhs.height).abs() <= f32::EPSILON
}

fn image_format(format: RenderImageFormat) -> ImageFormat {
    match format {
        RenderImageFormat::Rgba8 => ImageFormat::Rgba8,
        RenderImageFormat::Bgra8 => ImageFormat::Bgra8,
    }
}

fn image_alpha(alpha: RenderImageAlpha) -> ImageAlphaType {
    match alpha {
        RenderImageAlpha::Alpha => ImageAlphaType::Alpha,
        RenderImageAlpha::Premultiplied => ImageAlphaType::AlphaPremultiplied,
    }
}

fn encode_resource_placeholder(
    scene: &mut Scene,
    transform: Affine,
    rect: Rect,
    device_scale: f64,
    fill: Color,
    stroke: Color,
) {
    let shape = rounded_rect(rect, CornerRadius::all(2.0));
    scene.fill(Fill::NonZero, transform, vello_color(fill), None, &shape);
    let stroke_style = vello::kurbo::Stroke::new(f64::from(quantize_stroke_width_to_device(
        1.0,
        device_scale,
    )));
    scene.stroke(&stroke_style, transform, vello_color(stroke), None, &shape);
    let first = KurboLine::new(
        (f64::from(rect.min_x()), f64::from(rect.min_y())),
        (f64::from(rect.max_x()), f64::from(rect.max_y())),
    );
    let second = KurboLine::new(
        (f64::from(rect.max_x()), f64::from(rect.min_y())),
        (f64::from(rect.min_x()), f64::from(rect.max_y())),
    );
    scene.stroke(
        &stroke_style,
        transform,
        vello_color(stroke.with_alpha(0.45)),
        None,
        &first,
    );
    scene.stroke(
        &stroke_style,
        transform,
        vello_color(stroke.with_alpha(0.45)),
        None,
        &second,
    );
}

fn transform_to_affine(transform: Transform) -> Affine {
    Affine::new([
        f64::from(transform.m11),
        f64::from(transform.m12),
        f64::from(transform.m21),
        f64::from(transform.m22),
        f64::from(transform.dx),
        f64::from(transform.dy),
    ])
}

fn viewport_device_scale(viewport: ViewportInfo) -> f64 {
    let scale = viewport.scale_factor.value();
    if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    }
}

fn root_transform(device_scale: f64) -> Affine {
    Affine::scale(device_scale.max(f64::EPSILON))
}

fn snap_axis_aligned_translation(transform: Affine) -> Affine {
    let mut coeffs = transform.as_coeffs();
    if coeffs[1].abs() <= f64::EPSILON && coeffs[2].abs() <= f64::EPSILON {
        coeffs[4] = coeffs[4].round();
        coeffs[5] = coeffs[5].round();
    }
    Affine::new(coeffs)
}

fn snap_rect_to_device(rect: Rect, device_scale: f64) -> Rect {
    let min = snap_point_to_device(Point::new(rect.x, rect.y), device_scale);
    let max = snap_point_to_device(Point::new(rect.max_x(), rect.max_y()), device_scale);
    Rect::new(
        min.x,
        min.y,
        (max.x - min.x).max(0.0),
        (max.y - min.y).max(0.0),
    )
}

fn snap_image_rect_to_device(rect: Rect, sampling: RenderImageSampling, device_scale: f64) -> Rect {
    match sampling {
        RenderImageSampling::Pixelated | RenderImageSampling::UiIcon => {
            snap_rect_to_device(rect, device_scale)
        }
        RenderImageSampling::Smooth | RenderImageSampling::HighQuality => {
            let origin = snap_point_to_device(rect.origin(), device_scale);
            Rect::new(origin.x, origin.y, rect.width, rect.height)
        }
    }
}

#[allow(clippy::cast_possible_truncation)]
fn snap_stroked_rect_to_device(rect: Rect, stroke_width: f32, device_scale: f64) -> Rect {
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

fn snap_stroked_line_to_device(
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
fn snap_stroke_center_to_device(value: f32, stroke_width: f32, device_scale: f64) -> f32 {
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

fn snap_point_to_device(point: Point, device_scale: f64) -> Point {
    Point::new(
        snap_scalar_to_device(point.x, device_scale),
        snap_scalar_to_device(point.y, device_scale),
    )
}

#[allow(clippy::cast_possible_truncation)]
fn snap_scalar_to_device(value: f32, device_scale: f64) -> f32 {
    if !value.is_finite() || !device_scale.is_finite() || device_scale <= 0.0 {
        return value;
    }
    ((f64::from(value) * device_scale).round() / device_scale) as f32
}

fn compose_transform(parent: Transform, child: Transform) -> Transform {
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

fn rounded_rect(rect: Rect, radius: CornerRadius) -> RoundedRect {
    RoundedRect::from_rect(kurbo_rect(rect), kurbo_radius(radius))
}

fn kurbo_rect(rect: Rect) -> KurboRect {
    KurboRect::new(
        f64::from(rect.min_x()),
        f64::from(rect.min_y()),
        f64::from(rect.max_x()),
        f64::from(rect.max_y()),
    )
}

fn kurbo_radius(radius: CornerRadius) -> RoundedRectRadii {
    RoundedRectRadii::new(
        f64::from(radius.top_left),
        f64::from(radius.top_right),
        f64::from(radius.bottom_right),
        f64::from(radius.bottom_left),
    )
}

fn vello_color(color: Color) -> vello::peniko::Color {
    vello::peniko::Color::new([
        finite_unit(color.r),
        finite_unit(color.g),
        finite_unit(color.b),
        finite_unit(color.a),
    ])
}

fn vello_gradient(gradient: &LinearGradient) -> PenikoGradient {
    let stops: Vec<(f32, vello::peniko::Color)> = gradient
        .stops()
        .iter()
        .map(|stop| (finite_unit(stop.offset), vello_color(stop.color)))
        .collect();
    PenikoGradient::new_linear(
        (f64::from(gradient.start().x), f64::from(gradient.start().y)),
        (f64::from(gradient.end().x), f64::from(gradient.end().y)),
    )
    .with_stops(stops.as_slice())
}

#[cfg(test)]
mod tests {
    use super::{
        ImageAtlasRegion, ImageDataCache, ImageResource, RenderCommand, RenderCommandKind,
        RenderDiagnostic, RenderFrameInput, RenderImage, RenderImageSampling, RenderResources,
        RendererBackend, TextLayoutResource, TextureResource, VelloRenderer, image_quality,
        physical_text_layout, physical_text_layout_for_key, quantize_stroke_width_to_device,
        render_translation_snapshot, root_transform, snap_axis_aligned_translation,
        snap_image_rect_to_device, snap_point_to_device, snap_rect_to_device,
        snap_stroke_center_to_device, snap_stroked_line_to_device, snap_stroked_rect_to_device,
        snap_text_origin_to_device, translate_primitives, viewport_device_scale,
    };
    use kinetik_ui_core::render::TexturePrimitive;
    use kinetik_ui_core::{
        Brush, ClipId, Color, CornerRadius, GradientStop, ImageId, ImagePrimitive, LayerId,
        LinePrimitive, LinearGradient, PathElement, PathPrimitive, Point, Primitive, Rect,
        RectPrimitive, ScaleFactor, ShadowPrimitive, Size, Stroke, TextLayoutId, TextPrimitive,
        TextureId, Transform, Vec2, ViewportInfo,
    };
    use kinetik_ui_text::{
        CosmicTextEngine, ShapedTextLayout, TextLayoutKey, TextLayoutStore, TextStyle,
    };
    use vello::{kurbo::Affine, peniko::ImageQuality};

    fn resources() -> RenderResources {
        let mut resources = RenderResources::new();
        resources.register_image(ImageResource {
            id: ImageId::from_raw(1),
            size: Size::new(64.0, 64.0),
            sampling: RenderImageSampling::default(),
            pixels: Some(tiny_image()),
            atlas_region: None,
        });
        resources.register_texture(TextureResource {
            id: TextureId::from_raw(2),
            size: Size::new(2.0, 2.0),
            sampling: RenderImageSampling::default(),
            snapshot: Some(tiny_image()),
        });
        resources
    }

    fn size_only_resources() -> RenderResources {
        let mut resources = RenderResources::new();
        resources.register_image(ImageResource {
            id: ImageId::from_raw(1),
            size: Size::new(64.0, 64.0),
            sampling: RenderImageSampling::default(),
            pixels: None,
            atlas_region: None,
        });
        resources.register_texture(TextureResource {
            id: TextureId::from_raw(2),
            size: Size::new(2.0, 2.0),
            sampling: RenderImageSampling::default(),
            snapshot: None,
        });
        resources
    }

    fn atlas_resources() -> RenderResources {
        let mut resources = RenderResources::new();
        resources.register_image(ImageResource {
            id: ImageId::from_raw(1),
            size: Size::new(2.0, 2.0),
            sampling: RenderImageSampling::default(),
            pixels: Some(tiny_image()),
            atlas_region: None,
        });
        resources.register_image(ImageResource {
            id: ImageId::from_raw(3),
            size: Size::new(1.0, 1.0),
            sampling: RenderImageSampling::default(),
            pixels: None,
            atlas_region: Some(ImageAtlasRegion {
                atlas: ImageId::from_raw(1),
                source: Rect::new(1.0, 0.0, 1.0, 1.0),
            }),
        });
        resources.register_image(ImageResource {
            id: ImageId::from_raw(4),
            size: Size::new(1.0, 1.0),
            sampling: RenderImageSampling::default(),
            pixels: None,
            atlas_region: Some(ImageAtlasRegion {
                atlas: ImageId::from_raw(1),
                source: Rect::new(0.0, 1.0, 1.0, 1.0),
            }),
        });
        resources
    }

    fn tiny_image() -> RenderImage {
        RenderImage::rgba8(
            2,
            2,
            vec![
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
            ],
        )
        .expect("valid tiny image")
    }

    fn one_pixel_image() -> RenderImage {
        RenderImage::rgba8(1, 1, vec![255, 255, 255, 255]).expect("valid one pixel image")
    }

    fn text_layout_resource(id: TextLayoutId, text: &str) -> TextLayoutResource {
        let mut engine = CosmicTextEngine::new();
        let key = TextLayoutKey::new(text, TextStyle::new("sans-serif", 12.0, 16.0), 200.0, false);
        let layout = engine.shape_text(&key);
        TextLayoutResource {
            id,
            key,
            layout: std::sync::Arc::new(layout),
        }
    }

    fn clip_rects(command: &RenderCommand) -> Vec<Rect> {
        command.clips.iter().map(|clip| clip.rect).collect()
    }

    fn clip_transforms(command: &RenderCommand) -> Vec<Transform> {
        command.clips.iter().map(|clip| clip.transform).collect()
    }

    fn assert_approx(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < f32::EPSILON,
            "expected {actual} to equal {expected}"
        );
    }

    fn assert_approx64(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < f64::EPSILON,
            "expected {actual} to equal {expected}"
        );
    }

    #[test]
    fn translates_rectangles_and_lines_in_order() {
        let primitives = vec![
            Primitive::Rect(RectPrimitive {
                rect: Rect::new(0.0, 0.0, 10.0, 10.0),
                fill: Some(Brush::Solid(Color::WHITE)),
                stroke: Some(Stroke::new(1.0, Brush::Solid(Color::BLACK))),
                radius: CornerRadius::all(0.0),
            }),
            Primitive::Line(LinePrimitive {
                from: Point::new(0.0, 0.0),
                to: Point::new(10.0, 10.0),
                stroke: Stroke::new(1.0, Brush::Solid(Color::WHITE)),
            }),
        ];

        let translation = translate_primitives(&primitives, &RenderResources::new());

        assert!(matches!(
            translation.commands[0].kind,
            RenderCommandKind::Rect { .. }
        ));
        assert!(matches!(
            translation.commands[1].kind,
            RenderCommandKind::Line { .. }
        ));
    }

    #[test]
    fn translates_paths_in_order() {
        let primitives = vec![Primitive::Path(PathPrimitive::new(
            vec![
                PathElement::MoveTo(Point::new(0.0, 0.0)),
                PathElement::LineTo(Point::new(10.0, 0.0)),
                PathElement::QuadTo {
                    ctrl: Point::new(12.0, 4.0),
                    to: Point::new(10.0, 8.0),
                },
                PathElement::CubicTo {
                    ctrl1: Point::new(8.0, 10.0),
                    ctrl2: Point::new(2.0, 10.0),
                    to: Point::new(0.0, 8.0),
                },
                PathElement::Close,
            ],
            Some(Brush::Solid(Color::WHITE)),
            Some(Stroke::new(1.0, Brush::Solid(Color::BLACK))),
        ))];

        let translation = translate_primitives(&primitives, &RenderResources::new());

        assert!(translation.diagnostics.is_empty());
        let RenderCommandKind::Path {
            elements,
            fill,
            stroke,
        } = &translation.commands[0].kind
        else {
            panic!("expected path command");
        };
        assert_eq!(elements.len(), 5);
        assert_eq!(*fill, Some(Brush::Solid(Color::WHITE)));
        assert_eq!(*stroke, Some(Stroke::new(1.0, Brush::Solid(Color::BLACK))));
    }

    #[test]
    fn translates_linear_gradient_brushes() {
        let gradient = LinearGradient::from_colors(
            Point::new(0.0, 0.0),
            Point::new(20.0, 0.0),
            &[Color::BLACK, Color::rgb(0.5, 0.5, 0.5), Color::WHITE],
        )
        .expect("valid gradient");
        let primitives = vec![Primitive::Rect(RectPrimitive {
            rect: Rect::new(0.0, 0.0, 20.0, 12.0),
            fill: Some(Brush::LinearGradient(gradient)),
            stroke: Some(Stroke::new(1.0, Brush::LinearGradient(gradient))),
            radius: CornerRadius::all(2.0),
        })];

        let translation = translate_primitives(&primitives, &RenderResources::new());

        assert!(translation.diagnostics.is_empty());
        let RenderCommandKind::Rect { fill, stroke, .. } = &translation.commands[0].kind else {
            panic!("expected rect command");
        };
        assert_eq!(*fill, Some(Brush::LinearGradient(gradient)));
        assert_eq!(
            *stroke,
            Some(Stroke::new(1.0, Brush::LinearGradient(gradient)))
        );
    }

    #[test]
    fn translates_shadows_in_order() {
        let shadow = ShadowPrimitive::new(
            Rect::new(2.0, 4.0, 20.0, 12.0),
            Vec2::new(1.0, 3.0),
            8.0,
            2.0,
            5.0,
            Color::rgba(0.0, 0.0, 0.0, 0.35),
        );
        let primitives = vec![Primitive::Shadow(shadow)];

        let translation = translate_primitives(&primitives, &RenderResources::new());

        assert!(translation.diagnostics.is_empty());
        let RenderCommandKind::Shadow {
            rect,
            offset,
            blur_radius,
            spread,
            radius,
            color,
        } = &translation.commands[0].kind
        else {
            panic!("expected shadow command");
        };
        assert_eq!(*rect, shadow.rect);
        assert_eq!(*offset, shadow.offset);
        assert_approx(*blur_radius, 8.0);
        assert_approx(*spread, 2.0);
        assert_approx(*radius, 5.0);
        assert_eq!(*color, shadow.color);
    }

    #[test]
    fn sanitizes_linear_gradient_stops_before_encoding() {
        let gradient = LinearGradient::new(
            Point::new(0.0, 0.0),
            Point::new(20.0, 0.0),
            &[
                GradientStop::new(1.0, Color::WHITE),
                GradientStop::new(f32::NAN, Color::rgba(f32::NAN, 0.25, 0.5, 1.0)),
                GradientStop::new(-0.25, Color::BLACK),
            ],
        )
        .expect("valid stop count");
        let primitives = vec![Primitive::Rect(RectPrimitive {
            rect: Rect::new(0.0, 0.0, 20.0, 12.0),
            fill: Some(Brush::LinearGradient(gradient)),
            stroke: None,
            radius: CornerRadius::all(0.0),
        })];

        let translation = translate_primitives(&primitives, &RenderResources::new());

        assert_eq!(
            translation.diagnostics,
            vec![
                RenderDiagnostic::InvalidGeometry("rect_fill"),
                RenderDiagnostic::InvalidGeometry("rect_fill"),
                RenderDiagnostic::InvalidGeometry("rect_fill"),
            ]
        );
        let RenderCommandKind::Rect {
            fill: Some(Brush::LinearGradient(gradient)),
            ..
        } = &translation.commands[0].kind
        else {
            panic!("expected sanitized gradient fill");
        };
        assert_approx(gradient.stops()[0].offset, 0.0);
        assert_approx(gradient.stops()[1].offset, 0.0);
        assert_approx(gradient.stops()[2].offset, 1.0);
        assert_eq!(gradient.stops()[0].color, Color::rgba(0.0, 0.25, 0.5, 1.0));
    }

    #[test]
    fn invalid_linear_gradient_endpoint_falls_back_to_solid_brush() {
        let gradient = LinearGradient::between(
            Point::new(f32::NAN, 0.0),
            Point::new(20.0, 0.0),
            Color::WHITE,
            Color::BLACK,
        );
        let primitives = vec![Primitive::Rect(RectPrimitive {
            rect: Rect::new(0.0, 0.0, 20.0, 12.0),
            fill: Some(Brush::LinearGradient(gradient)),
            stroke: None,
            radius: CornerRadius::all(0.0),
        })];

        let translation = translate_primitives(&primitives, &RenderResources::new());

        assert_eq!(
            translation.diagnostics,
            vec![RenderDiagnostic::InvalidGeometry("rect_fill")]
        );
        let RenderCommandKind::Rect {
            fill: Some(Brush::Solid(color)),
            ..
        } = &translation.commands[0].kind
        else {
            panic!("expected solid fallback");
        };
        assert_eq!(*color, Color::WHITE);
    }

    #[test]
    fn invalid_shadow_geometry_is_diagnosed_and_sanitized() {
        let primitives = vec![Primitive::Shadow(ShadowPrimitive::new(
            Rect::new(f32::NAN, 2.0, 20.0, 12.0),
            Vec2::new(f32::NAN, 3.0),
            -4.0,
            f32::NAN,
            -2.0,
            Color::rgba(f32::NAN, 0.0, 0.0, 0.25),
        ))];

        let translation = translate_primitives(&primitives, &RenderResources::new());

        assert_eq!(
            translation.diagnostics,
            vec![
                RenderDiagnostic::InvalidGeometry("shadow"),
                RenderDiagnostic::InvalidGeometry("shadow_offset"),
                RenderDiagnostic::InvalidGeometry("shadow_blur"),
                RenderDiagnostic::InvalidGeometry("shadow_spread"),
                RenderDiagnostic::InvalidGeometry("shadow_radius"),
                RenderDiagnostic::InvalidGeometry("shadow_color"),
            ]
        );
        let RenderCommandKind::Shadow {
            rect,
            offset,
            blur_radius,
            spread,
            radius,
            color,
        } = &translation.commands[0].kind
        else {
            panic!("expected sanitized shadow");
        };
        assert_approx(rect.x, 0.0);
        assert_eq!(*offset, Vec2::new(0.0, 3.0));
        assert_approx(*blur_radius, 0.0);
        assert_approx(*spread, 0.0);
        assert_approx(*radius, 0.0);
        assert_approx(color.r, 0.0);
    }

    #[test]
    fn shadow_spread_that_erases_rect_is_diagnosed_and_skipped() {
        let primitives = vec![Primitive::Shadow(ShadowPrimitive::new(
            Rect::new(0.0, 0.0, 10.0, 10.0),
            Vec2::ZERO,
            0.0,
            -6.0,
            0.0,
            Color::BLACK,
        ))];

        let translation = translate_primitives(&primitives, &RenderResources::new());

        assert_eq!(
            translation.diagnostics,
            vec![RenderDiagnostic::InvalidGeometry("shadow_spread")]
        );
        assert!(translation.commands.is_empty());
    }

    #[test]
    fn invalid_geometry_is_diagnosed_and_sanitized_before_encoding() {
        let primitives = vec![
            Primitive::Rect(RectPrimitive {
                rect: Rect::new(0.0, 0.0, -10.0, 10.0),
                fill: Some(Brush::Solid(Color::WHITE)),
                stroke: None,
                radius: CornerRadius::all(0.0),
            }),
            Primitive::Line(LinePrimitive {
                from: Point::new(f32::NAN, 0.0),
                to: Point::new(10.0, 10.0),
                stroke: Stroke::new(1.0, Brush::Solid(Color::WHITE)),
            }),
            Primitive::Path(PathPrimitive::new(
                vec![PathElement::MoveTo(Point::new(f32::NAN, 0.0))],
                Some(Brush::Solid(Color::WHITE)),
                None,
            )),
            Primitive::ClipBegin {
                id: ClipId::from_raw(9),
                rect: Rect::new(0.0, 0.0, f32::NAN, 10.0),
            },
            Primitive::TransformBegin(Transform {
                dx: f32::INFINITY,
                ..Transform::IDENTITY
            }),
            Primitive::Rect(RectPrimitive {
                rect: Rect::new(f32::NAN, 2.0, 10.0, 10.0),
                fill: Some(Brush::Solid(Color::rgba(f32::NAN, 0.5, 0.5, 1.0))),
                stroke: Some(Stroke::new(-1.0, Brush::Solid(Color::WHITE))),
                radius: CornerRadius::all(-3.0),
            }),
        ];

        let translation = translate_primitives(&primitives, &RenderResources::new());

        assert_eq!(
            translation.diagnostics,
            vec![
                RenderDiagnostic::InvalidGeometry("rect"),
                RenderDiagnostic::InvalidGeometry("line"),
                RenderDiagnostic::InvalidGeometry("path"),
                RenderDiagnostic::InvalidGeometry("clip"),
                RenderDiagnostic::InvalidGeometry("transform"),
                RenderDiagnostic::InvalidGeometry("rect"),
                RenderDiagnostic::InvalidGeometry("rect_fill"),
                RenderDiagnostic::InvalidGeometry("rect_stroke"),
                RenderDiagnostic::InvalidGeometry("rect_radius"),
            ]
        );
        assert_eq!(translation.commands.len(), 1);
        assert_eq!(translation.commands[0].transform, Transform::IDENTITY);
        assert!(translation.commands[0].clips.is_empty());
        let RenderCommandKind::Rect {
            rect,
            fill,
            stroke,
            radius,
        } = &translation.commands[0].kind
        else {
            panic!("expected sanitized rect command");
        };
        assert_approx(rect.x, 0.0);
        assert_approx(rect.y, 2.0);
        assert!(stroke.is_none());
        assert_approx(radius.top_left, 0.0);
        let Some(Brush::Solid(color)) = fill else {
            panic!("expected solid fill");
        };
        assert_approx(color.r, 0.0);
        assert_approx(color.g, 0.5);
    }

    #[test]
    fn invalid_empty_paths_are_diagnosed_and_skipped() {
        let primitives = vec![Primitive::Path(PathPrimitive::new(
            Vec::new(),
            Some(Brush::Solid(Color::WHITE)),
            None,
        ))];

        let translation = translate_primitives(&primitives, &RenderResources::new());

        assert_eq!(
            translation.diagnostics,
            vec![RenderDiagnostic::InvalidGeometry("path")]
        );
        assert!(translation.commands.is_empty());
    }

    #[test]
    fn invalid_texture_source_size_is_diagnosed_and_dropped() {
        let primitives = vec![Primitive::Texture(TexturePrimitive {
            texture: TextureId::from_raw(2),
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            source_size: Size::new(f32::NAN, 10.0),
        })];

        let translation = translate_primitives(&primitives, &resources());

        assert_eq!(
            translation.diagnostics,
            vec![RenderDiagnostic::InvalidGeometry("texture_source_size")]
        );
        assert!(translation.commands.is_empty());
    }

    #[test]
    fn applies_layer_clip_and_transform_to_following_commands() {
        let primitives = vec![
            Primitive::LayerBegin {
                id: LayerId::from_raw(3),
            },
            Primitive::ClipBegin {
                id: ClipId::from_raw(4),
                rect: Rect::new(0.0, 0.0, 20.0, 20.0),
            },
            Primitive::TransformBegin(Transform::translation(Vec2::new(2.0, 3.0))),
            Primitive::Rect(RectPrimitive {
                rect: Rect::new(1.0, 1.0, 4.0, 4.0),
                fill: None,
                stroke: None,
                radius: CornerRadius::all(0.0),
            }),
        ];

        let translation = translate_primitives(&primitives, &RenderResources::new());
        let command = &translation.commands[0];

        assert_eq!(command.layer, LayerId::from_raw(3));
        assert_eq!(clip_rects(command), vec![Rect::new(0.0, 0.0, 20.0, 20.0)]);
        assert_eq!(clip_transforms(command), vec![Transform::IDENTITY]);
        assert_eq!(
            command.transform,
            Transform::translation(Vec2::new(2.0, 3.0))
        );
    }

    #[test]
    fn restores_nested_layer_clip_and_transform_stacks() {
        let outer_clip = Rect::new(0.0, 0.0, 40.0, 40.0);
        let inner_clip = Rect::new(4.0, 4.0, 20.0, 20.0);
        let primitives = vec![
            Primitive::LayerBegin {
                id: LayerId::from_raw(1),
            },
            Primitive::ClipBegin {
                id: ClipId::from_raw(1),
                rect: outer_clip,
            },
            Primitive::TransformBegin(Transform::translation(Vec2::new(2.0, 3.0))),
            Primitive::LayerBegin {
                id: LayerId::from_raw(2),
            },
            Primitive::ClipBegin {
                id: ClipId::from_raw(2),
                rect: inner_clip,
            },
            Primitive::TransformBegin(Transform::translation(Vec2::new(5.0, 7.0))),
            Primitive::Rect(RectPrimitive {
                rect: Rect::new(1.0, 1.0, 4.0, 4.0),
                fill: None,
                stroke: None,
                radius: CornerRadius::all(0.0),
            }),
            Primitive::TransformEnd,
            Primitive::ClipEnd {
                id: ClipId::from_raw(2),
            },
            Primitive::LayerEnd {
                id: LayerId::from_raw(2),
            },
            Primitive::Rect(RectPrimitive {
                rect: Rect::new(2.0, 2.0, 4.0, 4.0),
                fill: None,
                stroke: None,
                radius: CornerRadius::all(0.0),
            }),
            Primitive::TransformEnd,
            Primitive::ClipEnd {
                id: ClipId::from_raw(1),
            },
            Primitive::LayerEnd {
                id: LayerId::from_raw(1),
            },
            Primitive::Rect(RectPrimitive {
                rect: Rect::new(3.0, 3.0, 4.0, 4.0),
                fill: None,
                stroke: None,
                radius: CornerRadius::all(0.0),
            }),
        ];

        let translation = translate_primitives(&primitives, &RenderResources::new());

        assert_eq!(translation.commands[0].layer, LayerId::from_raw(2));
        assert_eq!(
            clip_rects(&translation.commands[0]),
            vec![outer_clip, inner_clip]
        );
        assert_eq!(
            clip_transforms(&translation.commands[0]),
            vec![
                Transform::IDENTITY,
                Transform::translation(Vec2::new(2.0, 3.0))
            ]
        );
        assert_eq!(
            translation.commands[0].transform,
            Transform::translation(Vec2::new(7.0, 10.0))
        );
        assert_eq!(translation.commands[1].layer, LayerId::from_raw(1));
        assert_eq!(clip_rects(&translation.commands[1]), vec![outer_clip]);
        assert_eq!(
            clip_transforms(&translation.commands[1]),
            vec![Transform::IDENTITY]
        );
        assert_eq!(
            translation.commands[1].transform,
            Transform::translation(Vec2::new(2.0, 3.0))
        );
        assert_eq!(translation.commands[2].layer, LayerId::from_raw(0));
        assert!(translation.commands[2].clips.is_empty());
        assert_eq!(translation.commands[2].transform, Transform::IDENTITY);
    }

    #[test]
    fn reports_mismatched_scope_stack_end_primitives() {
        let primitives = vec![
            Primitive::ClipEnd {
                id: ClipId::from_raw(4),
            },
            Primitive::LayerEnd {
                id: LayerId::from_raw(3),
            },
            Primitive::TransformEnd,
        ];

        let translation = translate_primitives(&primitives, &resources());

        assert_eq!(
            translation.diagnostics,
            vec![
                RenderDiagnostic::InvalidGeometry("clip_stack"),
                RenderDiagnostic::InvalidGeometry("layer_stack"),
                RenderDiagnostic::InvalidGeometry("transform_stack"),
            ]
        );
    }

    #[test]
    fn reports_unclosed_scope_stacks_at_end_of_translation() {
        let primitives = vec![
            Primitive::LayerBegin {
                id: LayerId::from_raw(3),
            },
            Primitive::ClipBegin {
                id: ClipId::from_raw(4),
                rect: Rect::new(0.0, 0.0, 20.0, 20.0),
            },
            Primitive::TransformBegin(Transform::translation(Vec2::new(1.0, 2.0))),
        ];

        let translation = translate_primitives(&primitives, &resources());

        assert_eq!(
            translation.diagnostics,
            vec![
                RenderDiagnostic::InvalidGeometry("clip_stack"),
                RenderDiagnostic::InvalidGeometry("layer_stack"),
                RenderDiagnostic::InvalidGeometry("transform_stack"),
            ]
        );
    }

    #[test]
    fn render_translation_snapshot_covers_commands_resources_and_diagnostics() {
        let missing_layout = TextLayoutId::from_raw(7);
        let primitives = vec![
            Primitive::LayerBegin {
                id: LayerId::from_raw(3),
            },
            Primitive::ClipBegin {
                id: ClipId::from_raw(4),
                rect: Rect::new(0.0, 0.0, 20.0, 12.0),
            },
            Primitive::TransformBegin(Transform::translation(Vec2::new(2.0, 3.0))),
            Primitive::Rect(RectPrimitive {
                rect: Rect::new(1.0, 1.0, 8.0, 4.0),
                fill: Some(Brush::Solid(Color::WHITE)),
                stroke: None,
                radius: CornerRadius::all(2.0),
            }),
            Primitive::TransformEnd,
            Primitive::ClipEnd {
                id: ClipId::from_raw(4),
            },
            Primitive::LayerEnd {
                id: LayerId::from_raw(3),
            },
            Primitive::Text(TextPrimitive {
                layout: Some(missing_layout),
                origin: Point::new(4.0, 16.0),
                text: "Hi".to_owned(),
                family: "monospace".to_owned(),
                size: 12.0,
                line_height: 17.0,
                brush: Brush::Solid(Color::BLACK),
            }),
            Primitive::Image(ImagePrimitive {
                image: ImageId::from_raw(9),
                rect: Rect::new(0.0, 20.0, 16.0, 16.0),
            }),
            Primitive::Texture(TexturePrimitive {
                texture: TextureId::from_raw(2),
                rect: Rect::new(20.0, 20.0, 16.0, 16.0),
                source_size: Size::new(2.0, 2.0),
            }),
        ];

        let translation = translate_primitives(&primitives, &resources());

        assert_eq!(
            render_translation_snapshot(&translation),
            "commands:\n  0: layer=3 transform=[1.000, 0.000, 0.000, 1.000, 2.000, 3.000] clips=[{rect=(0.000, 0.000, 20.000, 12.000) transform=[1.000, 0.000, 0.000, 1.000, 0.000, 0.000]}] rect rect=(1.000, 1.000, 8.000, 4.000) fill=rgba(1.000, 1.000, 1.000, 1.000) stroke=none radius=(2.000, 2.000, 2.000, 2.000)\n  1: layer=0 transform=[1.000, 0.000, 0.000, 1.000, 0.000, 0.000] clips=[] text layout=7 origin=(4.000, 16.000) family=\"monospace\" size=12.000 line_height=17.000 color=rgba(0.000, 0.000, 0.000, 1.000) text=\"Hi\"\n  2: layer=0 transform=[1.000, 0.000, 0.000, 1.000, 0.000, 0.000] clips=[] image#9 rect=(0.000, 20.000, 16.000, 16.000)\n  3: layer=0 transform=[1.000, 0.000, 0.000, 1.000, 0.000, 0.000] clips=[] texture#2 rect=(20.000, 20.000, 16.000, 16.000) source_size=2.000x2.000\ndiagnostics:\n  missing_text_layout#7\n  missing_image#9"
        );
    }

    #[test]
    fn reports_missing_image_and_texture_resources() {
        let primitives = vec![
            Primitive::Image(ImagePrimitive {
                image: ImageId::from_raw(9),
                rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            }),
            Primitive::Texture(TexturePrimitive {
                texture: TextureId::from_raw(8),
                rect: Rect::new(0.0, 0.0, 10.0, 10.0),
                source_size: Size::new(10.0, 10.0),
            }),
        ];

        let translation = translate_primitives(&primitives, &resources());

        assert_eq!(
            translation.diagnostics,
            vec![
                RenderDiagnostic::MissingImage(ImageId::from_raw(9)),
                RenderDiagnostic::MissingTexture(TextureId::from_raw(8)),
            ]
        );
    }

    #[test]
    fn registered_resources_do_not_emit_missing_diagnostics() {
        let primitives = vec![
            Primitive::Image(ImagePrimitive {
                image: ImageId::from_raw(1),
                rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            }),
            Primitive::Texture(TexturePrimitive {
                texture: TextureId::from_raw(2),
                rect: Rect::new(0.0, 0.0, 10.0, 10.0),
                source_size: Size::new(2.0, 2.0),
            }),
        ];

        let translation = translate_primitives(&primitives, &resources());

        assert!(translation.diagnostics.is_empty());
    }

    #[test]
    fn atlas_backed_image_resources_do_not_emit_missing_diagnostics() {
        let primitives = vec![Primitive::Image(ImagePrimitive {
            image: ImageId::from_raw(3),
            rect: Rect::new(0.0, 0.0, 16.0, 16.0),
        })];

        let translation = translate_primitives(&primitives, &atlas_resources());

        assert!(translation.diagnostics.is_empty());
    }

    #[test]
    fn invalid_atlas_source_is_diagnosed() {
        let mut resources = atlas_resources();
        resources.register_image(ImageResource {
            id: ImageId::from_raw(5),
            size: Size::new(4.0, 4.0),
            sampling: RenderImageSampling::default(),
            pixels: None,
            atlas_region: Some(ImageAtlasRegion {
                atlas: ImageId::from_raw(1),
                source: Rect::new(1.0, 1.0, 4.0, 4.0),
            }),
        });
        let primitives = vec![Primitive::Image(ImagePrimitive {
            image: ImageId::from_raw(5),
            rect: Rect::new(0.0, 0.0, 16.0, 16.0),
        })];

        let translation = translate_primitives(&primitives, &resources);

        assert_eq!(
            translation.diagnostics,
            vec![RenderDiagnostic::InvalidGeometry("image_atlas_source")]
        );
    }

    #[test]
    fn texture_source_size_mismatch_is_diagnosed_and_dropped() {
        let primitives = vec![Primitive::Texture(TexturePrimitive {
            texture: TextureId::from_raw(2),
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            source_size: Size::new(3.0, 2.0),
        })];

        let translation = translate_primitives(&primitives, &resources());

        assert_eq!(
            translation.diagnostics,
            vec![RenderDiagnostic::InvalidGeometry("texture_source_size")]
        );
        assert!(translation.commands.is_empty());
    }

    #[test]
    fn texture_snapshot_size_mismatch_is_diagnosed_and_dropped() {
        let mut resources = RenderResources::new();
        resources.register_texture(TextureResource {
            id: TextureId::from_raw(8),
            size: Size::new(2.0, 2.0),
            sampling: RenderImageSampling::default(),
            snapshot: Some(one_pixel_image()),
        });
        let primitives = vec![Primitive::Texture(TexturePrimitive {
            texture: TextureId::from_raw(8),
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            source_size: Size::new(2.0, 2.0),
        })];

        let translation = translate_primitives(&primitives, &resources);

        assert_eq!(
            translation.diagnostics,
            vec![RenderDiagnostic::InvalidGeometry("texture_snapshot_size")]
        );
        assert!(translation.commands.is_empty());
    }

    #[test]
    fn registered_size_only_resources_emit_payload_diagnostics() {
        let primitives = vec![
            Primitive::Image(ImagePrimitive {
                image: ImageId::from_raw(1),
                rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            }),
            Primitive::Texture(TexturePrimitive {
                texture: TextureId::from_raw(2),
                rect: Rect::new(0.0, 0.0, 10.0, 10.0),
                source_size: Size::new(2.0, 2.0),
            }),
        ];

        let translation = translate_primitives(&primitives, &size_only_resources());

        assert_eq!(
            translation.diagnostics,
            vec![
                RenderDiagnostic::MissingImagePixels(ImageId::from_raw(1)),
                RenderDiagnostic::MissingTextureSnapshot(TextureId::from_raw(2)),
            ]
        );
    }

    #[test]
    fn render_image_validates_pixel_byte_length() {
        assert!(RenderImage::rgba8(2, 2, vec![0; 16]).is_some());
        assert!(RenderImage::rgba8(2, 2, vec![0; 15]).is_none());
    }

    #[test]
    fn text_translation_accepts_unshaped_text_for_renderer_fallback() {
        let primitives = vec![Primitive::Text(TextPrimitive {
            layout: None,
            origin: Point::new(0.0, 0.0),
            text: "Label".to_owned(),
            family: "sans-serif".to_owned(),
            size: 12.0,
            line_height: 16.0,
            brush: Brush::Solid(Color::WHITE),
        })];

        let translation = translate_primitives(&primitives, &RenderResources::new());

        assert!(translation.diagnostics.is_empty());
        assert!(matches!(
            translation.commands[0].kind,
            RenderCommandKind::Text { layout: None, .. }
        ));
    }

    #[test]
    fn text_translation_reports_missing_shaped_layout_resource() {
        let layout = TextLayoutId::from_raw(7);
        let primitives = vec![Primitive::Text(TextPrimitive {
            layout: Some(layout),
            origin: Point::new(0.0, 0.0),
            text: "Label".to_owned(),
            family: "sans-serif".to_owned(),
            size: 12.0,
            line_height: 16.0,
            brush: Brush::Solid(Color::WHITE),
        })];

        let translation = translate_primitives(&primitives, &RenderResources::new());

        assert_eq!(
            translation.diagnostics,
            vec![RenderDiagnostic::MissingTextLayout(layout)]
        );
    }

    #[test]
    fn frame_submission_reports_primitive_count_and_diagnostics() {
        let mut renderer = VelloRenderer::new();
        let primitives = vec![Primitive::Image(ImagePrimitive {
            image: ImageId::from_raw(9),
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
        })];
        let resources = RenderResources::new();
        let output = renderer.submit_frame(RenderFrameInput {
            viewport: ViewportInfo::new(
                Size::new(100.0, 100.0),
                kinetik_ui_core::PhysicalSize::new(100, 100),
                ScaleFactor::ONE,
            ),
            primitives: &primitives,
            resources: &resources,
        });

        assert_eq!(output.primitive_count, 1);
        assert_eq!(
            output.diagnostics,
            vec![RenderDiagnostic::MissingImage(ImageId::from_raw(9))]
        );
    }

    #[test]
    fn renderer_backend_trait_submits_vello_frames() {
        let mut renderer = VelloRenderer::new();
        let resources = RenderResources::new();

        let output = RendererBackend::render_frame(
            &mut renderer,
            RenderFrameInput {
                viewport: ViewportInfo::new(
                    Size::new(100.0, 100.0),
                    kinetik_ui_core::PhysicalSize::new(100, 100),
                    ScaleFactor::ONE,
                ),
                primitives: &[],
                resources: &resources,
            },
        )
        .expect("Vello frame submission is infallible before GPU presentation");

        assert_eq!(output.primitive_count, 0);
        assert!(output.diagnostics.is_empty());
        assert!(renderer.scene().encoding().is_empty());
    }

    #[test]
    fn viewport_device_scale_uses_frame_scale_factor() {
        let viewport = ViewportInfo::new(
            Size::new(800.0, 600.0),
            kinetik_ui_core::PhysicalSize::new(1200, 900),
            ScaleFactor::new(1.5),
        );

        assert!((viewport_device_scale(viewport) - 1.5).abs() < f64::EPSILON);
    }

    #[test]
    fn renderer_snaps_geometry_to_device_pixel_grid() {
        let point = snap_point_to_device(Point::new(10.2, 20.6), 2.0);
        let rect = snap_rect_to_device(Rect::new(1.2, 2.2, 9.1, 10.1), 2.0);

        assert_eq!(point, Point::new(10.0, 20.5));
        assert_eq!(rect, Rect::new(1.0, 2.0, 9.5, 10.5));
    }

    #[test]
    fn image_rect_snapping_respects_sampling_intent() {
        let rect = Rect::new(3.2, 4.2, 14.0, 14.0);
        let icon = snap_image_rect_to_device(rect, RenderImageSampling::UiIcon, 1.25);
        let high_quality = snap_image_rect_to_device(rect, RenderImageSampling::HighQuality, 1.25);

        assert_approx(icon.x, 3.2);
        assert_approx(icon.y, 4.0);
        assert!((icon.width - 14.4).abs() < 0.000_01);
        assert!((icon.height - 14.4).abs() < 0.000_01);
        assert_eq!(high_quality, Rect::new(3.2, 4.0, 14.0, 14.0));
        assert!((icon.width * 1.25 - 18.0).abs() < 0.000_01);
        assert_approx(high_quality.width, 14.0);
    }

    #[test]
    fn renderer_snaps_stroke_centers_to_physical_pixel_coverage() {
        let one_px = snap_stroke_center_to_device(10.0, 1.0, 1.0);
        let one_px_fractional_scale = snap_stroke_center_to_device(10.0, 1.0, 1.25);
        let two_px = snap_stroke_center_to_device(10.0, 1.0, 2.0);
        let horizontal =
            snap_stroked_line_to_device(Point::new(0.2, 10.0), Point::new(20.2, 10.0), 1.0, 1.0);
        let rect = snap_stroked_rect_to_device(Rect::new(0.1, 0.1, 20.2, 12.2), 1.0, 1.0);
        let fractional_rect =
            snap_stroked_rect_to_device(Rect::new(0.0, 0.0, 20.0, 12.0), 1.0, 1.25);

        assert_approx(one_px, 10.5);
        assert_approx(one_px_fractional_scale, 10.0);
        assert_approx(two_px, 10.0);
        assert_eq!(horizontal.0, Point::new(0.0, 10.5));
        assert_eq!(horizontal.1, Point::new(20.0, 10.5));
        assert_eq!(rect, Rect::new(0.5, 0.5, 19.0, 11.0));
        assert_eq!(fractional_rect, Rect::new(0.4, 0.4, 19.2, 11.2));
    }

    #[test]
    fn renderer_quantizes_stroke_widths_to_physical_pixels() {
        assert_approx(quantize_stroke_width_to_device(1.0, 1.0), 1.0);
        assert_approx(quantize_stroke_width_to_device(1.0, 1.25), 0.8);
        assert_approx(quantize_stroke_width_to_device(1.0, 1.5), 1.333_333_4);
        assert_approx(quantize_stroke_width_to_device(2.0, 1.25), 2.4);
    }

    #[test]
    fn renderer_snaps_axis_aligned_transform_translation_to_device_pixels() {
        let transform =
            snap_axis_aligned_translation(root_transform(2.0) * Affine::translate((0.25, 0.25)));

        let coeffs = transform.as_coeffs();
        assert_approx64(coeffs[0], 2.0);
        assert_approx64(coeffs[1], 0.0);
        assert_approx64(coeffs[2], 0.0);
        assert_approx64(coeffs[3], 2.0);
        assert_approx64(coeffs[4], 1.0);
        assert_approx64(coeffs[5], 1.0);
    }

    #[test]
    fn image_sampling_maps_to_vello_quality() {
        assert_eq!(
            image_quality(RenderImageSampling::Pixelated),
            ImageQuality::Low
        );
        assert_eq!(
            image_quality(RenderImageSampling::UiIcon),
            ImageQuality::Low
        );
        assert_eq!(
            image_quality(RenderImageSampling::Smooth),
            ImageQuality::Medium
        );
        assert_eq!(
            image_quality(RenderImageSampling::HighQuality),
            ImageQuality::High
        );
    }

    #[test]
    fn frame_submission_encodes_atlas_backed_image_resource() {
        let mut renderer = VelloRenderer::new();
        let resources = atlas_resources();
        let primitives = vec![Primitive::Image(ImagePrimitive {
            image: ImageId::from_raw(3),
            rect: Rect::new(4.0, 4.0, 16.0, 16.0),
        })];

        let output = renderer.submit_frame(RenderFrameInput {
            viewport: ViewportInfo::new(
                Size::new(100.0, 100.0),
                kinetik_ui_core::PhysicalSize::new(100, 100),
                ScaleFactor::ONE,
            ),
            primitives: &primitives,
            resources: &resources,
        });

        assert!(output.diagnostics.is_empty());
        assert!(!renderer.scene().encoding().is_empty());
        assert!(!renderer.scene().encoding().resources.patches.is_empty());
    }

    #[test]
    fn frame_submission_reuses_cached_atlas_payload_for_regions() {
        let mut renderer = VelloRenderer::new();
        let resources = atlas_resources();
        let primitives = vec![
            Primitive::Image(ImagePrimitive {
                image: ImageId::from_raw(3),
                rect: Rect::new(4.0, 4.0, 16.0, 16.0),
            }),
            Primitive::Image(ImagePrimitive {
                image: ImageId::from_raw(4),
                rect: Rect::new(24.0, 4.0, 16.0, 16.0),
            }),
        ];

        let output = renderer.submit_frame(RenderFrameInput {
            viewport: ViewportInfo::new(
                Size::new(100.0, 100.0),
                kinetik_ui_core::PhysicalSize::new(100, 100),
                ScaleFactor::ONE,
            ),
            primitives: &primitives,
            resources: &resources,
        });

        assert!(output.diagnostics.is_empty());
        assert_eq!(renderer.cached_image_count(), 1);
    }

    #[test]
    fn image_cache_uses_shared_payload_identity_for_hits() {
        let id = ImageId::from_raw(11);
        let image = RenderImage::rgba8(2, 2, vec![1; 16]).expect("valid image");
        let clone = image.clone();
        let replacement = RenderImage::rgba8(2, 2, vec![2; 16]).expect("valid image");
        let mut cache = ImageDataCache::default();

        cache.image_data(id, &image);
        let cached_payload = cache
            .images
            .get(&id)
            .expect("cache entry")
            .signature
            .data
            .clone();
        cache.image_data(id, &clone);
        assert!(std::sync::Arc::ptr_eq(
            &cached_payload,
            &cache.images.get(&id).expect("cache entry").signature.data
        ));

        cache.image_data(id, &replacement);
        let replaced_payload = &cache.images.get(&id).expect("cache entry").signature.data;
        assert!(std::sync::Arc::ptr_eq(replaced_payload, &replacement.data));
        assert!(!std::sync::Arc::ptr_eq(&cached_payload, replaced_payload));
    }

    #[test]
    fn frame_submission_encodes_vello_geometry() {
        let mut renderer = VelloRenderer::new();
        let primitives = vec![
            Primitive::Rect(RectPrimitive {
                rect: Rect::new(0.0, 0.0, 40.0, 24.0),
                fill: Some(Brush::Solid(Color::WHITE)),
                stroke: Some(Stroke::new(1.0, Brush::Solid(Color::BLACK))),
                radius: CornerRadius::all(4.0),
            }),
            Primitive::Line(LinePrimitive {
                from: Point::new(0.0, 0.0),
                to: Point::new(40.0, 24.0),
                stroke: Stroke::new(2.0, Brush::Solid(Color::WHITE)),
            }),
            Primitive::Path(PathPrimitive::new(
                vec![
                    PathElement::MoveTo(Point::new(6.0, 6.0)),
                    PathElement::LineTo(Point::new(30.0, 6.0)),
                    PathElement::LineTo(Point::new(18.0, 20.0)),
                    PathElement::Close,
                ],
                Some(Brush::Solid(Color::rgba(0.2, 0.6, 0.9, 1.0))),
                Some(Stroke::new(1.0, Brush::Solid(Color::BLACK))),
            )),
        ];
        let resources = RenderResources::new();

        let output = renderer.submit_frame(RenderFrameInput {
            viewport: ViewportInfo::new(
                Size::new(100.0, 100.0),
                kinetik_ui_core::PhysicalSize::new(100, 100),
                ScaleFactor::ONE,
            ),
            primitives: &primitives,
            resources: &resources,
        });

        assert!(output.diagnostics.is_empty());
        assert!(!renderer.scene().encoding().is_empty());
        assert!(renderer.scene().encoding().n_paths >= 2);
    }

    #[test]
    fn frame_submission_encodes_fallback_text_and_visible_resource_placeholders() {
        let mut renderer = VelloRenderer::new();
        let resources = resources();
        let primitives = vec![
            Primitive::Text(TextPrimitive {
                layout: None,
                origin: Point::new(4.0, 16.0),
                text: "Label".to_owned(),
                family: "sans-serif".to_owned(),
                size: 12.0,
                line_height: 16.0,
                brush: Brush::Solid(Color::WHITE),
            }),
            Primitive::Image(ImagePrimitive {
                image: ImageId::from_raw(1),
                rect: Rect::new(0.0, 24.0, 32.0, 24.0),
            }),
            Primitive::Texture(TexturePrimitive {
                texture: TextureId::from_raw(2),
                rect: Rect::new(40.0, 24.0, 32.0, 24.0),
                source_size: Size::new(2.0, 2.0),
            }),
        ];

        let output = renderer.submit_frame(RenderFrameInput {
            viewport: ViewportInfo::new(
                Size::new(100.0, 100.0),
                kinetik_ui_core::PhysicalSize::new(100, 100),
                ScaleFactor::ONE,
            ),
            primitives: &primitives,
            resources: &resources,
        });

        assert!(output.diagnostics.is_empty());
        assert!(!renderer.scene().encoding().is_empty());
        assert!(!renderer.scene().encoding().resources.glyph_runs.is_empty());
        assert!(!renderer.scene().encoding().resources.glyphs.is_empty());
        assert!(renderer.scene().encoding().resources.patches.len() >= 2);
    }

    #[test]
    fn frame_submission_encodes_axis_aligned_text_at_physical_font_size() {
        let mut renderer = VelloRenderer::new();
        let resources = RenderResources::new();
        let primitives = vec![Primitive::Text(TextPrimitive {
            layout: None,
            origin: Point::new(4.0, 16.0),
            text: "Label".to_owned(),
            family: "sans-serif".to_owned(),
            size: 12.0,
            line_height: 16.0,
            brush: Brush::Solid(Color::WHITE),
        })];

        let output = renderer.submit_frame(RenderFrameInput {
            viewport: ViewportInfo::new(
                Size::new(100.0, 100.0),
                kinetik_ui_core::PhysicalSize::new(200, 200),
                ScaleFactor::new(2.0),
            ),
            primitives: &primitives,
            resources: &resources,
        });

        let glyph_run = renderer
            .scene()
            .encoding()
            .resources
            .glyph_runs
            .first()
            .expect("glyph run");

        assert!(output.diagnostics.is_empty());
        assert_approx(glyph_run.font_size, 24.0);
        assert!(glyph_run.hint);
    }

    #[test]
    fn text_origin_snapping_rounds_x_and_baseline_y() {
        let origin = snap_text_origin_to_device(Point::new(5.375, 20.5));

        assert_approx(origin.x, 5.0);
        assert_approx(origin.y, 21.0);
    }

    #[test]
    fn physical_text_snaps_horizontal_origin_and_baseline() {
        let mut renderer = VelloRenderer::new();
        let resources = RenderResources::new();
        let primitives = vec![Primitive::Text(TextPrimitive {
            layout: None,
            origin: Point::new(4.3, 16.4),
            text: "Label".to_owned(),
            family: "sans-serif".to_owned(),
            size: 12.0,
            line_height: 16.0,
            brush: Brush::Solid(Color::WHITE),
        })];

        let output = renderer.submit_frame(RenderFrameInput {
            viewport: ViewportInfo::new(
                Size::new(100.0, 100.0),
                kinetik_ui_core::PhysicalSize::new(125, 125),
                ScaleFactor::new(1.25),
            ),
            primitives: &primitives,
            resources: &resources,
        });
        let glyph = renderer
            .scene()
            .encoding()
            .resources
            .glyphs
            .first()
            .expect("glyph");

        assert!(output.diagnostics.is_empty());
        assert_approx(glyph.x, 5.0);
        assert_approx(glyph.y, 21.0);
    }

    #[test]
    fn physical_text_layout_shapes_at_device_font_size() {
        let mut engine = CosmicTextEngine::new();
        let layout = physical_text_layout(
            &mut engine,
            root_transform(1.5),
            "Label",
            "monospace",
            12.0,
            17.0,
        )
        .expect("axis-aligned physical layout");

        assert!(
            layout
                .runs
                .iter()
                .all(|run| (run.font_size - 18.0).abs() < f32::EPSILON)
        );
        assert!(
            layout
                .lines
                .iter()
                .all(|line| (line.height - 25.5).abs() < f32::EPSILON)
        );
    }

    #[test]
    fn physical_text_layout_for_key_preserves_wrap_width_at_device_scale() {
        let key = TextLayoutKey::new(
            "alpha beta gamma delta epsilon",
            TextStyle::new("sans-serif", 12.0, 17.0),
            68.0,
            true,
        );
        let mut expected_engine = CosmicTextEngine::new();
        let expected = expected_engine.shape_text(&TextLayoutKey::new(
            key.text.clone(),
            TextStyle::new("sans-serif", 18.0, 25.5),
            102.0,
            true,
        ));
        let mut engine = CosmicTextEngine::new();

        let layout = physical_text_layout_for_key(&mut engine, root_transform(1.5), &key)
            .expect("axis-aligned physical layout");

        assert_eq!(layout.line_count, expected.line_count);
        assert_eq!(layout.lines.len(), expected.lines.len());
        assert!(layout.line_count > 1);
        assert_approx(layout.size.width, expected.size.width);
        assert!(
            layout
                .runs
                .iter()
                .all(|run| (run.font_size - 18.0).abs() < f32::EPSILON)
        );
        assert!(
            layout
                .lines
                .iter()
                .all(|line| (line.height - 25.5).abs() < f32::EPSILON)
        );
    }

    #[test]
    fn frame_submission_encodes_registered_shaped_text_layout() {
        let layout = TextLayoutId::from_raw(44);
        let mut resources = RenderResources::new();
        resources.register_text_layout(text_layout_resource(layout, "Label"));
        let primitives = vec![Primitive::Text(TextPrimitive {
            layout: Some(layout),
            origin: Point::new(4.0, 16.0),
            text: "Label".to_owned(),
            family: "sans-serif".to_owned(),
            size: 12.0,
            line_height: 16.0,
            brush: Brush::Solid(Color::WHITE),
        })];
        let mut renderer = VelloRenderer::new();

        let output = renderer.submit_frame(RenderFrameInput {
            viewport: ViewportInfo::new(
                Size::new(100.0, 100.0),
                kinetik_ui_core::PhysicalSize::new(100, 100),
                ScaleFactor::ONE,
            ),
            primitives: &primitives,
            resources: &resources,
        });

        assert!(output.diagnostics.is_empty());
        assert!(!renderer.scene().encoding().resources.glyph_runs.is_empty());
        assert!(!renderer.scene().encoding().resources.glyphs.is_empty());
    }

    #[test]
    fn registered_text_layout_renders_with_fractional_scale_physical_shape() {
        let layout = TextLayoutId::from_raw(45);
        let mut resources = RenderResources::new();
        resources.register_text_layout(text_layout_resource(layout, "Label"));
        let primitives = vec![Primitive::Text(TextPrimitive {
            layout: Some(layout),
            origin: Point::new(4.3, 16.4),
            text: "Label".to_owned(),
            family: "sans-serif".to_owned(),
            size: 12.0,
            line_height: 16.0,
            brush: Brush::Solid(Color::WHITE),
        })];
        let mut renderer = VelloRenderer::new();

        let output = renderer.submit_frame(RenderFrameInput {
            viewport: ViewportInfo::new(
                Size::new(100.0, 100.0),
                kinetik_ui_core::PhysicalSize::new(125, 125),
                ScaleFactor::new(1.25),
            ),
            primitives: &primitives,
            resources: &resources,
        });
        let glyph_run = renderer
            .scene()
            .encoding()
            .resources
            .glyph_runs
            .first()
            .expect("glyph run");
        let glyph = renderer
            .scene()
            .encoding()
            .resources
            .glyphs
            .first()
            .expect("glyph");

        assert!(output.diagnostics.is_empty());
        assert_approx(glyph_run.font_size, 15.0);
        assert!(glyph_run.hint);
        assert_approx(glyph.x, 5.0);
        assert_approx(glyph.y, 21.0);
    }

    #[test]
    fn render_resources_register_text_layout_store_entries() {
        let mut store = TextLayoutStore::new();
        let id = store.layout_id(TextLayoutKey::new(
            "Label",
            TextStyle::new("sans-serif", 12.0, 16.0),
            200.0,
            false,
        ));
        let mut resources = RenderResources::new();

        resources.register_text_layouts(store.layouts());

        assert!(resources.has_text_layout(id));
        assert_eq!(
            resources.text_layout(id).map(ShapedTextLayout::glyph_count),
            store.layout(id).map(ShapedTextLayout::glyph_count)
        );
    }

    #[test]
    fn frame_submission_resets_retained_scene() {
        let mut renderer = VelloRenderer::new();
        let resources = RenderResources::new();
        let primitives = vec![Primitive::Rect(RectPrimitive {
            rect: Rect::new(0.0, 0.0, 40.0, 24.0),
            fill: Some(Brush::Solid(Color::WHITE)),
            stroke: None,
            radius: CornerRadius::all(0.0),
        })];

        renderer.submit_frame(RenderFrameInput {
            viewport: ViewportInfo::new(
                Size::new(100.0, 100.0),
                kinetik_ui_core::PhysicalSize::new(100, 100),
                ScaleFactor::ONE,
            ),
            primitives: &primitives,
            resources: &resources,
        });
        assert!(!renderer.scene().encoding().is_empty());

        renderer.submit_frame(RenderFrameInput {
            viewport: ViewportInfo::new(
                Size::new(100.0, 100.0),
                kinetik_ui_core::PhysicalSize::new(100, 100),
                ScaleFactor::ONE,
            ),
            primitives: &[],
            resources: &resources,
        });

        assert!(renderer.scene().encoding().is_empty());
    }
}
